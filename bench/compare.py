"""Compare complete benchmark snapshots with matching workload and metadata."""

from datetime import datetime, timezone
import argparse
import json
from pathlib import Path
import sys

if __package__:
    from .clients import CAPABILITIES
    from .report import LABELS, REPOSITORY, escape, payload_label
    from .results import DIMENSIONS, validate_document
else:
    from clients import CAPABILITIES
    from report import LABELS, REPOSITORY, escape, payload_label
    from results import DIMENSIONS, validate_document


def comparable_client(metadata):
    """Ignore relocated artifacts and allow only wreq's build/version to change."""
    result = dict(metadata)
    ignored = {"path"}
    if metadata["package"] == "wreq":
        result.pop("version")
        ignored.add("sha256")
    result["native"] = sorted(
        (
            {key: value for key, value in artifact.items() if key not in ignored}
            for artifact in metadata["native"]
        ),
        key=lambda artifact: json.dumps(artifact, sort_keys=True),
    )
    return result


def render_comparison(before: dict, after: dict, *, api: str | None = None) -> str:
    for document in (before, after):
        validate_document(document)
    if api not in (None, "async", "blocking"):
        raise ValueError("Comparison API must be async or blocking")
    configurations = [dict(document["configuration"]) for document in (before, after)]
    if api is not None:
        for config in configurations:
            config["clients"] = sorted(
                client
                for client in config["clients"]
                if CAPABILITIES[client]["api"] == api
            )
            if not config["clients"]:
                raise ValueError(f"Snapshot contains no {api} clients")
    config = configurations[0]
    if config != configurations[1]:
        raise ValueError("Benchmark configurations do not match")
    fields = (
        "python",
        "implementation",
        "platform",
        "machine",
        "cpu",
        "cpu_count",
        "event_loop",
        "affinity",
    )
    for field in fields:
        if before["environment"].get(field) != after["environment"].get(field):
            raise ValueError(f"Benchmark environment does not match: {field}")
    if before["server"]["sha256"] != after["server"]["sha256"]:
        raise ValueError("Benchmark server artifacts do not match")
    for client in config["clients"]:
        if comparable_client(before["clients"][client]) != comparable_client(
            after["clients"][client]
        ):
            raise ValueError(f"Benchmark client metadata does not match: {client}")
    cells = [
        {
            tuple(cell[key] for key in DIMENSIONS): cell
            for cell in document["results"]
            if cell["client"] in config["clients"]
        }
        for document in (before, after)
    ]
    if cells[0].keys() != cells[1].keys():
        raise ValueError("Benchmark result matrices do not match")
    lines = [
        "# Benchmark revision comparison",
        "",
        "| Snapshot | Source revision | Dirty checkout | Collected (UTC) | wreq version | wreq native SHA-256 |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for name, document in (("Before", before), ("After", after)):
        source = document["source"]
        date = datetime.fromisoformat(document["generated_at"]).astimezone(timezone.utc)
        wreq = [
            metadata
            for client, metadata in document["clients"].items()
            if client in config["clients"] and metadata["package"] == "wreq"
        ]
        versions = ", ".join(sorted({metadata["version"] for metadata in wreq}))
        hashes = ", ".join(
            sorted(
                {
                    artifact["sha256"]
                    for metadata in wreq
                    for artifact in metadata["native"]
                }
            )
        )
        lines.append(
            f"| {name} | [{escape(source['commit'])}]"
            f"({REPOSITORY}/commit/{source['commit']}) | "
            f"{'Yes' if source['dirty'] else 'No'} | {escape(date.isoformat())} | "
            f"{escape(versions or 'Not recorded')} | {escape(hashes or 'Not recorded')} |"
        )
    if api is not None:
        lines += [
            "",
            f"Comparison scope: {api} clients only; original snapshots are unchanged.",
        ]
    requests = config["requests"] * config["samples"] * config["rounds"]
    lines += [
        "",
        f"Each snapshot {'contributes' if api is not None else 'contains'} "
        f"{len(cells[0]):,} supported cells{' to this comparison' if api is not None else ''}, with "
        f"{requests:,} timed requests per cell across {config['rounds']} rounds. "
        "Counts come from the cases recorded in these snapshots.",
        "",
        "### Matching measurement environment",
        "",
        "| Item | Value |",
        "| --- | --- |",
    ]
    for field in fields:
        value = before["environment"].get(field, "Not recorded")
        lines.append(f"| {escape(field)} | {escape(value)} |")
    lines += [
        f"| Server SHA-256 | {escape(before['server']['sha256'])} |",
        "",
        "Unit: requests/s (RPS, requests per second). RPS is total timed requests "
        "divided by total timed seconds across all rounds; warmup and validation "
        "requests are excluded. Change = 100 × (After RPS / Before RPS − 1); "
        "positive values mean higher throughput.",
        "",
    ]
    order = {client: index for index, client in enumerate(LABELS)}
    for api, heading in (
        ("async", "Asynchronous clients"),
        ("blocking", "Blocking clients"),
    ):
        group = [key for key in cells[0] if CAPABILITIES[key[0]]["api"] == api]
        if not group:
            continue
        lines += [f"### {heading}", ""]
        for protocol, name in (("h1", "HTTP/1.1"), ("h2", "HTTP/2")):
            for kind in ("full", "stream"):
                keys = [key for key in group if key[1:3] == (protocol, kind)]
                if not keys:
                    continue
                lines += [
                    f"#### {name}: {kind.title()} upload",
                    "",
                    "| Payload | Concurrency | Client | Before RPS | After RPS | Change % | Before CV % | After CV % |",
                    "| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |",
                ]
                for key in sorted(
                    keys,
                    key=lambda key: (
                        key[3],
                        key[4],
                        order.get(key[0], len(order)),
                        key[0],
                    ),
                ):
                    old, new = cells[0][key], cells[1][key]
                    change = 100 * (new["rps"] / old["rps"] - 1)
                    lines.append(
                        f"| {payload_label(key[3])} | {key[4]} | "
                        f"{escape(LABELS.get(key[0], key[0]))} | "
                        f"{old['rps']:,.1f} | {new['rps']:,.1f} | {change:+.1f} | "
                        f"{old['round_rps_cv_pct']:.2f} | {new['round_rps_cv_pct']:.2f} |"
                    )
                lines.append("")
    lines += [
        "### Reading the comparison",
        "",
        "CV is the variation in per-round RPS: population standard deviation "
        "divided by the mean. It is not a confidence interval. Sequential runs "
        "are not a paired experiment, and other programs can affect performance. "
        "If peer clients also change, the environment may have changed between "
        "runs; their results alone can't tell you why.",
        "",
        "The timer covers uploads, TLS and HTTP processing, and streamed response "
        "consumption. A throughput change doesn't tell you how much came from "
        "TLS I/O, how long an individual request took, or which client will be "
        "fastest in a different workload.",
        "",
        "The comparison checks that recorded configurations, environments, "
        "server hashes and client metadata match. It allows wreq versions and "
        "native hashes to change, and artifact paths to move. Check the build "
        "and source records yourself for transitive dependencies, toolchains "
        "and flags, allocators, changes to the timed harness and uncommitted "
        "source changes.",
        "",
    ]
    return "\n".join(lines)


def main(argv=None) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", type=Path, required=True)
    parser.add_argument("--after", type=Path, required=True)
    parser.add_argument(
        "--api", choices=("async", "blocking"), help="Compare only this API group"
    )
    parser.add_argument("--output", type=Path, help="Write a new UTF-8 Markdown file")
    args = parser.parse_args(argv)
    documents = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in (args.before, args.after)
    ]
    content = render_comparison(*documents, api=args.api)
    if args.output is not None:
        with args.output.open("x", encoding="utf-8", newline="\n") as handle:
            handle.write(content)
    else:
        if hasattr(sys.stdout, "reconfigure"):
            sys.stdout.reconfigure(encoding="utf-8")
        print(content, end="")


if __name__ == "__main__":
    main()
