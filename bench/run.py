"""Run local benchmarks, preserve artifacts, and optionally publish built docs."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import uuid

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from bench import benchmark
from bench.clients import CLIENTS, supports
from bench.report import render_markdown
from bench.workloads import BODY_CASES, CONCURRENCY_CASES
from docs.build import MAX_DATA_BYTES, decode_document

LATEST = ROOT / "bench/data/latest.json"


def require_publish(config):
    """Only the complete default matrix can become the public snapshot."""
    axes = {
        "clients": CLIENTS,
        "protocols": ("h1", "h2"),
        "body_kinds": ("full", "stream"),
        "payload_bytes": BODY_CASES,
        "concurrency": CONCURRENCY_CASES,
    }
    if any(set(config[key]) != set(values) for key, values in axes.items()) or any(
        config[key] < minimum
        for key, minimum in (
            ("requests", 300),
            ("rounds", 3),
            ("warmup", 1),
            ("samples", 1),
        )
    ):
        raise ValueError(
            "Publishing requires the full default matrix, >=300 requests, >=3 rounds, >=1 warm-up and timed sample"
        )


def describe(config):
    cells = (
        sum(
            supports(client, protocol, kind)
            for client in config["clients"]
            for protocol in config["protocols"]
            for kind in config["body_kinds"]
        )
        * len(config["payload_bytes"])
        * len(config["concurrency"])
    )
    print(
        f"Matrix: {cells:,} supported cells; "
        f"{cells * config['rounds'] * config['samples']:,} timed batches; "
        f"{cells * config['rounds'] * config['warmup']:,} warm-up batches; "
        f"{config['requests']:,} requests/batch.",
        flush=True,
    )


def preserve_report(path, content):
    raw = content.encode("utf-8")
    if path.exists():
        if path.read_bytes() != raw:
            raise FileExistsError(f"Refusing to replace a different report: {path}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("xb") as handle:
        handle.write(raw)


def publish(raw):
    """Replace only latest.json, preserving the candidate's original bytes."""
    LATEST.parent.mkdir(parents=True, exist_ok=True)
    handle = tempfile.NamedTemporaryFile(dir=LATEST.parent, delete=False)
    temporary = Path(handle.name)
    try:
        with handle:
            handle.write(raw)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, LATEST)
    finally:
        temporary.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__,
        allow_abbrev=False,
        epilog="Other options are passed to benchmark.py; see python bench/benchmark.py --help.",
    )
    parser.add_argument(
        "--input", type=Path, help="Post-process existing JSON without measuring"
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="New Markdown report; identical existing content is reusable",
    )
    parser.add_argument(
        "--build-docs", action="store_true", help="Preview docs using this candidate"
    )
    parser.add_argument(
        "--publish",
        action="store_true",
        help="Build docs, then explicitly select this full matrix as latest",
    )
    parser.add_argument(
        "--docs-python",
        default=sys.executable,
        help="Python executable containing the docs dependencies",
    )
    options, forwarded = parser.parse_known_args(argv)
    try:
        if options.input is not None:
            if forwarded:
                raise ValueError(
                    "--input cannot be combined with measurement arguments"
                )
            output = options.input.resolve()
        else:
            args = benchmark.parse_args(forwarded)
            if args.worker is not None:
                raise ValueError("Worker mode is internal to benchmark.py")
            config = {**vars(args), "payload_bytes": args.sizes}
            if options.publish:
                require_publish(config)
            describe(config)
            stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
            name = f"{stamp}-{benchmark.source_info()['commit'][:12]}-{uuid.uuid4().hex[:8]}"
            output = (args.output or ROOT / "bench/data" / f"{name}.json").resolve()
            if output == LATEST.resolve() or output.exists():
                raise FileExistsError(
                    f"Choose a new historical snapshot, not latest.json: {output}"
                )
        report = (options.report or output.with_suffix(".report.md")).resolve()
        logs = [
            output.with_name(f"{output.stem}-{stream}.log")
            for stream in ("stdout", "stderr")
        ]
        if report in {output, LATEST.resolve(), *logs}:
            raise ValueError("Report path must be separate from JSON and logs")
        if options.input is None:
            if report.exists():
                raise FileExistsError(
                    f"Choose a new report path before measuring: {report}"
                )
            output.parent.mkdir(parents=True, exist_ok=True)
            print(f"Raw JSON: {output}\nLogs: {logs[0]}\n      {logs[1]}", flush=True)
            with logs[0].open("xb") as stdout, logs[1].open("xb") as stderr:
                result = subprocess.run(
                    [
                        sys.executable,
                        str(ROOT / "bench/benchmark.py"),
                        *forwarded,
                        "--server",
                        str(args.server.resolve()),
                        "--output",
                        str(output),
                    ],
                    cwd=ROOT,
                    stdout=stdout,
                    stderr=stderr,
                    check=False,
                )
            if result.returncode:
                raise RuntimeError(
                    f"Benchmark failed ({result.returncode}); logs were preserved"
                )
        with output.open("rb") as handle:
            raw = handle.read(MAX_DATA_BYTES + 1)
        document = decode_document(raw)
        if options.publish:
            require_publish(document["configuration"])
        if options.input is not None:
            describe(document["configuration"])
        preserve_report(report, render_markdown(document))
        print(f"Report: {report}", flush=True)
        if options.build_docs or options.publish:
            docs_python = Path(options.docs_python)
            docs_python = (
                str(docs_python.resolve())
                if docs_python.is_file()
                else options.docs_python
            )
            with tempfile.TemporaryDirectory(
                prefix="wreq-benchmark-docs-"
            ) as directory:
                frozen = Path(directory) / "candidate.json"
                frozen.write_bytes(raw)
                result = subprocess.run(
                    [
                        docs_python,
                        str(ROOT / "docs/build.py"),
                        "--data",
                        str(frozen),
                    ],
                    cwd=ROOT,
                    check=False,
                )
            if result.returncode:
                raise RuntimeError(
                    f"Documentation build failed ({result.returncode}); latest.json was not changed"
                )
        if options.publish:
            publish(raw)
            print(f"Selected latest: {LATEST}", flush=True)
        return 0
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"Benchmark pipeline failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
