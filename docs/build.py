"""Prepare a frozen benchmark page, then build or serve the documentation."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
from textwrap import indent

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from bench.charts import write_charts
from bench.clients import CAPABILITIES
from bench.report import render_markdown
from bench.results import require_publish, validate_document
from docs.benchmark_view import render_explorer

MAX_DATA_BYTES = 8 * 1024 * 1024
PLACEHOLDER = "{{BENCHMARK_RESULTS}}"
CHART_PLACEHOLDER = "{{BENCHMARK_CHARTS}}"


def clear_generated(root: Path) -> None:
    """Remove only this build's generated page, snapshots and chart assets."""
    root = root.resolve()
    charts = root / "docs/source/assets/benchmark/charts"
    if not charts.resolve().is_relative_to(root):
        raise ValueError(
            "Generated chart directory must stay inside the docs workspace"
        )
    for path in charts.glob("*.svg"):
        if not path.resolve().is_relative_to(charts.resolve()):
            raise ValueError("Generated chart asset points outside its directory")
        path.unlink()
    (root / "docs/source/benchmark.md").unlink(missing_ok=True)
    (root / "docs/source/assets/benchmark/latest.json").unlink(missing_ok=True)
    (root / "docs/source/assets/benchmark/latest-blocking.json").unlink(missing_ok=True)


def decode_document(raw: bytes) -> dict:
    if len(raw) > MAX_DATA_BYTES:
        raise ValueError("Benchmark JSON exceeds the 8 MiB file limit")
    document = json.loads(raw)
    validate_document(document)
    return document


def prepare(
    *,
    data: Path | None = None,
    blocking_data: Path | None = None,
    root: Path = ROOT,
) -> dict:
    """Read checked-in data or a local override; never reuse stale generated output."""
    root = Path(root)
    page = root / "docs/source/benchmark.md"
    snapshot = root / "docs/source/assets/benchmark/latest.json"
    blocking_snapshot = root / "docs/source/assets/benchmark/latest-blocking.json"
    source = Path(data) if data is not None else root / "bench/data/latest.json"
    blocking_source = (
        Path(blocking_data)
        if blocking_data is not None
        else root / "bench/data/latest-blocking.json"
    )
    try:
        template = (root / "docs/templates/benchmark.md").read_text(encoding="utf-8")
        # --data may point to the previously generated snapshot itself.
        with source.open("rb") as handle:
            raw = handle.read(MAX_DATA_BYTES + 1)
        blocking_raw = None
        # An explicit --data previews only that run unless an overlay is selected.
        if data is None or blocking_data is not None:
            try:
                with blocking_source.open("rb") as handle:
                    blocking_raw = handle.read(MAX_DATA_BYTES + 1)
            except FileNotFoundError:
                if blocking_data is not None or blocking_source.is_symlink():
                    raise
    finally:
        clear_generated(root)
    if template.count(PLACEHOLDER) != 1 or template.count(CHART_PLACEHOLDER) != 1:
        raise ValueError(
            "The benchmark template must contain exactly one results and charts placeholder"
        )
    document = decode_document(raw)
    blocking = decode_document(blocking_raw) if blocking_raw is not None else None
    if blocking is not None:
        require_publish(blocking["configuration"], blocking=True)
        for axis in ("protocols", "body_kinds", "payload_bytes", "concurrency"):
            if set(document["configuration"][axis]) != set(
                blocking["configuration"][axis]
            ):
                raise ValueError(f"Async and blocking chart axes do not match: {axis}")
        if not any(
            CAPABILITIES[client]["api"] == "async" for client in document["clients"]
        ):
            raise ValueError("The baseline snapshot must contain async clients")
    try:
        directory = root / "docs/source/assets/benchmark/charts"
        if blocking is None:
            content = render_markdown(
                document, environment_only=True, environment_heading=None
            )
            catalog = write_charts(document, directory)
        else:
            content = measurement_environments(document, blocking)
            catalog = {
                "schema_version": 1,
                "cases": (
                    write_charts(document, directory, api="async")["cases"]
                    + write_charts(blocking, directory, api="blocking")["cases"]
                ),
                "measurements": {
                    api: {
                        "source": value["source"],
                        "generated_at": value["generated_at"],
                    }
                    for api, value in (("async", document), ("blocking", blocking))
                },
            }
        charts = render_explorer(document, catalog)
        snapshot.parent.mkdir(parents=True, exist_ok=True)
        snapshot.write_bytes(raw)
        if blocking_raw is not None:
            blocking_snapshot.write_bytes(blocking_raw)
        page.parent.mkdir(parents=True, exist_ok=True)
        page.write_text(
            template.replace(PLACEHOLDER, indent(content, "    ")).replace(
                CHART_PLACEHOLDER, charts
            ),
            encoding="utf-8",
        )
    except Exception:
        clear_generated(root)
        raise
    return document


def measurement_environments(async_document: dict, blocking_document: dict) -> str:
    """Share one table only when all displayed environment/settings match."""
    shared = [
        render_markdown(
            document,
            environment_only=True,
            environment_heading=None,
            include_source=False,
        )
        for document in (async_document, blocking_document)
    ]
    if shared[0] == shared[1]:
        return shared[0]
    return "\n".join(
        render_markdown(
            document,
            environment_only=True,
            include_source=False,
            environment_heading=f"{label} measurement environment",
        )
        for label, document in (
            ("Async", async_document),
            ("Blocking", blocking_document),
        )
    )


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "action", nargs="?", choices=("build", "serve"), default="build"
    )
    parser.add_argument(
        "--data",
        type=Path,
        help="Preview a local JSON independently unless --blocking-data is also set",
    )
    parser.add_argument(
        "--blocking-data",
        type=Path,
        help="Use a complete blocking JSON instead of bench/data/latest-blocking.json",
    )
    args = parser.parse_args(argv)
    try:
        prepare(data=args.data, blocking_data=args.blocking_data)
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"Documentation preparation failed: {exc}", file=sys.stderr)
        return 1
    return subprocess.run(
        [sys.executable, "-m", "zensical", args.action, "-f", "docs/mkdocs.yml"],
        cwd=ROOT,
        check=False,
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
