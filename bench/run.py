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
from bench.clients import supports
from bench.report import render_markdown
from bench.results import BLOCKING_CLIENTS, require_publish
from docs.build import MAX_DATA_BYTES, decode_document

LATEST = ROOT / "bench/data/latest.json"
LATEST_BLOCKING = ROOT / "bench/data/latest-blocking.json"


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
        f"{config['requests']:,} timed requests/batch; "
        f"{config.get('warmup_requests', config['requests']):,} warm-up requests/batch.",
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


def publish(raw, target):
    """Atomically select a snapshot without reserializing its original bytes."""
    target.parent.mkdir(parents=True, exist_ok=True)
    handle = tempfile.NamedTemporaryFile(dir=target.parent, delete=False)
    temporary = Path(handle.name)
    try:
        with handle:
            handle.write(raw)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, target)
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
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument(
        "--publish",
        action="store_true",
        help="Build docs, then explicitly select this full matrix as latest",
    )
    selection.add_argument(
        "--publish-blocking",
        action="store_true",
        help="Build docs, then select the complete blocking matrix without changing async data",
    )
    parser.add_argument(
        "--docs-python",
        default=sys.executable,
        help="Python executable containing the docs dependencies",
    )
    options, forwarded = parser.parse_known_args(argv)
    try:
        selecting = options.publish or options.publish_blocking
        latest = LATEST_BLOCKING if options.publish_blocking else LATEST
        protected = {LATEST.resolve(), LATEST_BLOCKING.resolve()}
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
            if selecting:
                require_publish(config, blocking=options.publish_blocking)
            describe(config)
            stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
            name = f"{stamp}-{benchmark.source_info()['commit'][:12]}-{uuid.uuid4().hex[:8]}"
            output = (args.output or ROOT / "bench/data" / f"{name}.json").resolve()
            if output in protected or output.exists():
                raise FileExistsError(
                    f"Choose a new historical snapshot, not a latest file: {output}"
                )
        report = (options.report or output.with_suffix(".report.md")).resolve()
        logs = [
            output.with_name(f"{output.stem}-{stream}.log")
            for stream in ("stdout", "stderr")
        ]
        if report in {output, *protected, *logs}:
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
        if selecting:
            require_publish(
                document["configuration"], blocking=options.publish_blocking
            )
        if options.input is not None:
            describe(document["configuration"])
        preserve_report(report, render_markdown(document))
        print(f"Report: {report}", flush=True)
        if options.build_docs or selecting:
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
                command = [docs_python, str(ROOT / "docs/build.py")]
                if all(client in BLOCKING_CLIENTS for client in document["clients"]):
                    companion = LATEST
                    full = Path(directory) / "full.json"
                    with companion.open("rb") as handle:
                        companion_raw = handle.read(MAX_DATA_BYTES + 1)
                    decode_document(companion_raw)
                    full.write_bytes(companion_raw)
                    command.extend(
                        ["--data", str(full), "--blocking-data", str(frozen)]
                    )
                else:
                    command.extend(["--data", str(frozen)])
                    if options.publish and (
                        LATEST_BLOCKING.exists() or LATEST_BLOCKING.is_symlink()
                    ):
                        companion = Path(directory) / "blocking.json"
                        with LATEST_BLOCKING.open("rb") as handle:
                            companion_raw = handle.read(MAX_DATA_BYTES + 1)
                        companion_document = decode_document(companion_raw)
                        require_publish(
                            companion_document["configuration"],
                            blocking=True,
                            allow_legacy_snapshot=True,
                        )
                        companion.write_bytes(companion_raw)
                        command.extend(["--blocking-data", str(companion)])
                result = subprocess.run(
                    command,
                    cwd=ROOT,
                    check=False,
                )
            if result.returncode:
                raise RuntimeError(
                    f"Documentation build failed ({result.returncode}); {latest.name} was not changed"
                )
        if selecting:
            publish(raw, latest)
            print(f"Selected latest: {latest}", flush=True)
        return 0
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"Benchmark pipeline failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
