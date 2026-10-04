"""Render validated benchmark JSON as a fixed Markdown comparison."""

from datetime import datetime, timezone
import argparse
import html
import json
from pathlib import Path
import re
import sys
from urllib.parse import urlsplit

if __package__:
    from .clients import CAPABILITIES
    from .results import validate_document
else:
    from clients import CAPABILITIES
    from results import validate_document


REPOSITORY = "https://github.com/0x676e67/wreq-python"
LABELS = {
    "wreq": "wreq (MT)",
    "wreq_st": "wreq (ST)",
    "pyreqwest_st": "pyreqwest (ST)",
    "pyreqwest_mt": "pyreqwest (MT)",
    "ry": "ry (default)",
    "httpx": "httpx",
    "aiohttp": "aiohttp",
    "niquests": "niquests",
    "curl_cffi": "curl_cffi",
    "wreq_blocking": "wreq (blocking)",
    "ry_blocking": "ry (blocking)",
    "requests": "requests",
    "httpx_blocking": "httpx (blocking)",
    "niquests_blocking": "niquests (blocking)",
    "curl_cffi_blocking": "curl_cffi (blocking)",
    "pycurl": "PycURL",
}


def escape(value) -> str:
    """Keep metadata as plain text, including inside Markdown tables."""
    text = html.escape(" ".join(str(value).split()), quote=False)
    return re.sub(r"[\\`*_{}\[\]()#+.!|>\-]", lambda match: f"&#{ord(match[0])};", text)


def payload_label(size: int) -> str:
    if size % (1024 * 1024) == 0:
        return f"{size // (1024 * 1024)} MiB"
    if size % 1024 == 0:
        return f"{size // 1024} KiB"
    return f"{size:,} B"


def runtime_label(runtime: dict) -> str:
    kind = runtime.get("kind")
    if kind == "default":
        return "Library default"
    if kind == "single_thread":
        return "Single-thread runtime"
    if kind == "multi_thread":
        return "Multi-thread runtime"
    if kind == "custom":
        workers = runtime.get("workers", "not recorded")
        steal = runtime.get("work_steal", "not recorded")
        return f"Custom: workers={workers}, work_steal={steal}"
    if kind == "thread_pool":
        return "Thread pool; one client per logical worker"
    return json.dumps(runtime, sort_keys=True, ensure_ascii=False)


def safe_data_link(link: str) -> str:
    parsed = urlsplit(link)
    if re.search(r"[\s\\<>\[\]()\x00-\x1f]", link):
        raise ValueError("Invalid raw-data link")
    if parsed.scheme:
        if (
            parsed.scheme != "https"
            or not parsed.hostname
            or parsed.username is not None
        ):
            raise ValueError("Raw-data links must use HTTPS")
    elif parsed.netloc or link.startswith("/") or ".." in parsed.path.split("/"):
        raise ValueError("Raw-data links must be safe relative paths")
    if not link:
        raise ValueError("Raw-data link cannot be empty")
    return link


def render_markdown(
    document: dict,
    data_link: str | None = None,
    *,
    environment_only: bool = False,
) -> str:
    validate_document(document)
    config = document["configuration"]
    source = document["source"]
    environment = document["environment"]
    if not isinstance(environment, dict):
        raise ValueError("Benchmark environment metadata must be an object")
    commit = source["commit"]
    generated = datetime.fromisoformat(document["generated_at"]).astimezone(
        timezone.utc
    )
    lines = [
        f"Measured revision: [{commit[:12]}]({REPOSITORY}/commit/{commit}). "
        f"Collected {generated:%Y-%m-%d %H:%M UTC}.",
        "",
    ]
    if source["dirty"]:
        lines += [
            "**Local measurement: the source checkout had uncommitted changes.**",
            "",
        ]
    if data_link is not None:
        lines += [f"[Download this build's raw JSON]({safe_data_link(data_link)}).", ""]
    lines += [
        "### Measurement environment",
        "",
        "| Item | Value |",
        "| --- | --- |",
        f"| Python | {escape(environment.get('implementation', 'Not recorded'))} "
        f"{escape(environment.get('python', 'Not recorded'))} |",
        f"| OS | {escape(environment['platform'])} |",
        f"| Architecture | {escape(environment['machine'])} |",
        f"| CPU | {escape(environment.get('cpu', 'Not recorded'))}; "
        f"{escape(environment.get('cpu_count', 'Not recorded'))} logical CPUs |",
        f"| Python event loop | {escape(environment.get('event_loop', 'Not recorded'))} |",
        f"| Server | Controlled Rust TLS echo server; {config['server_workers']} workers |",
        "| TLS | TLS 1.3; certificate verification disabled for the local test server |",
        f"| Repeats | Rounds: {config['rounds']}; timed batches/round: {config['samples']}; "
        f"requests/batch: {config['requests']}; warm-up batches/round: {config['warmup']} |",
        "| Stream upload chunk | "
        + (
            (
                "Varies by payload; recorded in raw JSON"
                if environment_only
                else "Varies by payload; see body cases below"
            )
            if "stream_chunk_bytes_by_payload" in config
            else payload_label(config["stream_chunk_bytes"])
        )
        + " |",
        "",
    ]
    if environment_only:
        return "\n".join(lines)
    if "stream_chunk_bytes_by_payload" in config:
        lines += [
            "### Body cases",
            "",
            "Each payload uses the upload modes and concurrency levels listed in this run.",
            "",
            "| Upload / echo payload | Stream upload chunk |",
            "| --- | --- |",
        ]
        for size in sorted(config["payload_bytes"]):
            lines.append(
                f"| {payload_label(size)} | "
                f"{payload_label(config['stream_chunk_bytes_by_payload'][str(size)])} |"
            )
    lines += [
        "",
        "### Client versions and runtimes",
        "",
        "| Client | API | Package | Version | Runtime | Protocols | Uploads |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    ordered_clients = [client for client in LABELS if client in config["clients"]]
    ordered_clients += [client for client in config["clients"] if client not in LABELS]
    for client in ordered_clients:
        metadata = document["clients"][client]
        capability = CAPABILITIES[client]
        lines.append(
            f"| {escape(LABELS.get(client, client))} | {capability['api']} | "
            f"{escape(metadata.get('package', client))} | {escape(metadata['version'])} | "
            f"{escape(runtime_label(metadata['runtime']))} | "
            f"{', '.join('HTTP/1.1' if p == 'h1' else 'HTTP/2' for p in capability['protocols'])} | "
            f"{', '.join(k.title() for k in capability['body_kinds'])} |"
        )
    lines += [
        "",
        "### Throughput comparison",
        "",
        "Each value is total measured requests divided by total measured time "
        "across all rounds, in requests per second (RPS). All timed samples count. "
        "The raw JSON keeps individual timings, per-round results, response-payload "
        "MB/s and native artifact SHA-256 hashes. Read chunking, buffering and "
        "connection pools differ between adapters; their configurations are "
        "recorded in the raw JSON.",
        "",
    ]
    results = {
        (
            r["client"],
            r["protocol"],
            r["body_kind"],
            r["payload_bytes"],
            r["concurrency"],
        ): r
        for r in document["results"]
    }
    for api, heading in (
        ("async", "Asynchronous clients"),
        ("blocking", "Blocking clients"),
    ):
        group = [
            client for client in ordered_clients if CAPABILITIES[client]["api"] == api
        ]
        if not group:
            continue
        lines += [
            f"### {heading}",
            "",
            "N/A means the client API doesn't support that protocol or upload mode. "
            "It doesn't indicate a failed request or zero throughput.",
            "",
        ]
        for protocol, name in (("h1", "HTTP/1.1"), ("h2", "HTTP/2")):
            if protocol not in config["protocols"]:
                continue
            for kind in ("full", "stream"):
                if kind not in config["body_kinds"]:
                    continue
                headers = ["Upload / echo payload", "Concurrency"] + [
                    escape(LABELS.get(client, client)) for client in group
                ]
                lines += [
                    f"#### {name}: {kind.title()} upload",
                    "",
                    "Unit: requests/s (RPS, requests per second). Higher is better.",
                    "",
                    "| " + " | ".join(headers) + " |",
                    "| --- | ---: |" + " ---: |" * len(group),
                ]
                for size in sorted(config["payload_bytes"]):
                    for concurrency in sorted(config["concurrency"]):
                        values = [payload_label(size), str(concurrency)] + [
                            (
                                f"{cell['rps']:,.1f}"
                                if (
                                    cell := results.get(
                                        (client, protocol, kind, size, concurrency)
                                    )
                                )
                                is not None
                                else "N/A"
                            )
                            for client in group
                        ]
                        lines.append("| " + " | ".join(values) + " |")
                lines.append("")
    return "\n".join(lines)


def main(argv=None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--markdown", action="store_true", required=True)
    parser.add_argument("--output", type=Path, help="Write a new UTF-8 Markdown file")
    args = parser.parse_args(argv)
    content = render_markdown(json.loads(args.input.read_text(encoding="utf-8")))
    if args.output is not None:
        with args.output.open("x", encoding="utf-8", newline="\n") as handle:
            handle.write(content)
    else:
        if hasattr(sys.stdout, "reconfigure"):
            sys.stdout.reconfigure(encoding="utf-8")
        print(content, end="")


if __name__ == "__main__":
    main()
