"""Prepare a frozen benchmark page, then build or serve the documentation."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from bench.charts import write_charts
from bench.report import render_markdown
from bench.results import validate_document, write_atomic
from docs.benchmark_view import render_explorer

MAX_DATA_BYTES = 8 * 1024 * 1024
PLACEHOLDER = "{{BENCHMARK_RESULTS}}"
CHART_PLACEHOLDER = "{{BENCHMARK_CHARTS}}"


def clear_generated(root: Path) -> None:
    """Remove only this build's page, download and flat generated chart assets."""
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


def decode_document(raw: bytes) -> dict:
    if len(raw) > MAX_DATA_BYTES:
        raise ValueError("Benchmark JSON exceeds the 8 MiB file limit")
    document = json.loads(raw)
    validate_document(document)
    return document


def prepare(
    *,
    data: Path | None = None,
    root: Path = ROOT,
) -> dict:
    """Read checked-in data or a local override; never reuse stale generated output."""
    root = Path(root)
    page = root / "docs/source/benchmark.md"
    snapshot = root / "docs/source/assets/benchmark/latest.json"
    source = Path(data) if data is not None else root / "bench/data/latest.json"
    try:
        template = (root / "docs/templates/benchmark.md").read_text(encoding="utf-8")
        # --data may point to the previously generated snapshot itself.
        with source.open("rb") as handle:
            raw = handle.read(MAX_DATA_BYTES + 1)
    finally:
        clear_generated(root)
    if template.count(PLACEHOLDER) != 1 or template.count(CHART_PLACEHOLDER) != 1:
        raise ValueError(
            "The benchmark template must contain exactly one results and charts placeholder"
        )
    document = decode_document(raw)
    try:
        content = render_markdown(document, data_link="assets/benchmark/latest.json")
        catalog = write_charts(document, root / "docs/source/assets/benchmark/charts")
        charts = render_explorer(document, catalog)
        write_atomic(snapshot, document)
        page.parent.mkdir(parents=True, exist_ok=True)
        page.write_text(
            template.replace(PLACEHOLDER, content).replace(CHART_PLACEHOLDER, charts),
            encoding="utf-8",
        )
    except Exception:
        clear_generated(root)
        raise
    return document


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "action", nargs="?", choices=("build", "serve"), default="build"
    )
    parser.add_argument(
        "--data", type=Path, help="Use a local JSON instead of bench/data/latest.json"
    )
    args = parser.parse_args(argv)
    try:
        prepare(data=args.data)
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
