"""Static reports and documentation preparation, without network or site builds."""

import copy
import importlib.metadata
import json
from pathlib import Path
import re
from textwrap import indent
from types import SimpleNamespace

import pytest

from bench import clients, compare, report, results
from bench import async_clients, blocking_clients
from bench.tests.test_benchmark import make_document
from bench.workloads import BODY_CASES, CONCURRENCY_CASES
from docs import build
from docs.benchmark_view import provenance, render_explorer


def test_combined_preview_shows_each_measurement_source():
    document = make_document()
    document["measurement_sources"] = [
        {
            "label": "Original run",
            "source": document["source"],
            "generated_at": "2026-10-05T03:24:00+00:00",
        },
        {
            "label": "Blocking MT/ST rerun",
            "source": document["source"],
            "generated_at": "2026-10-05T05:26:00+00:00",
        },
    ]
    text = provenance(document)
    assert "Combined preview" in text
    assert "Original run" in text and "2026-10-05 03:24 UTC" in text
    assert "Blocking MT/ST rerun" in text and "2026-10-05 05:26 UTC" in text


def test_combined_source_validation_and_explicit_blocking_preview(tmp_path):
    baseline, _ = blocking_documents()
    partial = make_document(
        {
            **baseline["configuration"],
            "clients": ["wreq_blocking", "wreq_blocking_st"],
            "warmup_requests": 150,
        }
    )
    root = docs_root(tmp_path)
    data = root / "full.json"
    overlay = root / "blocking.json"
    data.write_text(json.dumps(baseline))
    overlay.write_text(json.dumps(partial))
    build.prepare(root=root, data=data, blocking_data=overlay)
    content = (root / "docs/source/benchmark.md").read_text(encoding="utf-8")
    assert "wreq (blocking MT)" in content and "wreq (blocking ST)" in content
    assert "Experimental warm-up budget" in content

    document = make_document({"clients": ["wreq", "wreq_st"]})
    document["measurement_sources"] = [
        {
            "label": "Original",
            "clients": ["wreq"],
            "source": document["source"],
            "generated_at": document["generated_at"],
        },
        {
            "label": "Rerun",
            "clients": ["wreq_st"],
            "source": document["source"],
            "generated_at": document["generated_at"],
        },
    ]
    results.validate_document(document)
    for mutation in ("revision", "clients", "nested", "timestamp"):
        invalid = copy.deepcopy(document)
        source = invalid["measurement_sources"][1]
        if mutation == "revision":
            source["source"]["commit"] = '"><script>alert(1)</script>'
        elif mutation == "clients":
            source["clients"] = ["wreq"]
        elif mutation == "nested":
            source["measurement_sources"] = [source.copy()]
        else:
            source["generated_at"] = "2026-10-05T00:00:00"
        with pytest.raises(ValueError):
            results.validate_document(invalid)


def docs_root(tmp_path: Path) -> Path:
    template = tmp_path / "docs/templates/benchmark.md"
    template.parent.mkdir(parents=True)
    template.write_text(
        (build.ROOT / "docs/templates/benchmark.md").read_text(encoding="utf-8"),
        encoding="utf-8",
    )
    return tmp_path


def test_report_aggregation_and_metadata_escaping():
    document = make_document(
        {"clients": list(report.LABELS), "payload_bytes": [10240, 1048576, 4194304]}
    )
    text = report.render_markdown(document, "assets/benchmark/latest.json")
    for protocol in ("HTTP/1.1", "HTTP/2"):
        for kind in ("Full", "Stream"):
            assert f"#### {protocol}: {kind} upload" in text
    assert "10 KiB" in text and "1 MiB" in text and "4 MiB" in text
    assert text.count("| Upload / echo payload") == 8
    assert text.count("Unit: requests/s (RPS, requests per second)") == 8
    assert "### Asynchronous clients" in text and "### Blocking clients" in text
    assert "N/A" in text
    assert f"{document['results'][0]['rps']:,.1f}" in text
    assert "total measured requests divided by total measured time" in text
    assert "assets/benchmark/latest.json" in text
    document["environment"][
        "cpu"
    ] = "<script>alert(1)</script>|[click](javascript:bad)\n**fake**"
    document["clients"]["wreq"]["version"] = "`x`|[fake](bad)"
    document["clients"]["wreq"]["response_read"] = "<script>untrusted</script>"
    document["source"]["repository"] = "javascript:source-repository"
    text = report.render_markdown(document)
    assert "<script>" not in text and "&lt;script&gt;" in text
    assert "[click]" not in text and "**fake**" not in text
    assert "`x`" not in text and "&#124;" in text
    assert report.escape("CPU's name") == "CPU's name"
    assert "javascript:source-repository" not in text
    assert "Adapter settings" not in text and "<details" not in text
    assert (
        "Read chunking, buffering and connection pools differ between adapters" in text
    )
    assert f"{report.REPOSITORY}/commit/{document['source']['commit']}" in text
    for link in (
        "javascript:alert",
        "//example.com/data",
        "../data.json",
        "data)\n<script>",
    ):
        with pytest.raises(ValueError):
            report.render_markdown(document, link)
    incomplete = copy.deepcopy(document)
    incomplete["results"].pop()
    with pytest.raises(ValueError):
        report.render_markdown(incomplete)
    dynamic = make_document(
        {
            "clients": list(report.LABELS),
            "payload_bytes": list(BODY_CASES),
            "stream_chunk_bytes_by_payload": {
                str(size): chunk for size, chunk in BODY_CASES.items()
            },
        }
    )
    text = report.render_markdown(dynamic)
    assert "### Body cases" in text
    for size, chunk in BODY_CASES.items():
        assert (
            f"| {report.payload_label(size)} | {report.payload_label(chunk)} |" in text
        )
    assert "| 4 MiB | 256 KiB |" in text
    assert text.count("Unit: requests/s (RPS, requests per second)") == 8
    full = make_document({**dynamic["configuration"], "body_kinds": ["full"]})
    text = report.render_markdown(full)
    assert "both Full and Stream" not in text and ": Stream upload" not in text


def test_experimental_docs_preview_labels_its_warmup_budget(tmp_path):
    source = tmp_path / "experiment.json"
    source.write_text(
        json.dumps(make_document({"warmup_requests": 2})), encoding="utf-8"
    )
    (tmp_path / "docs/templates").mkdir(parents=True)
    (tmp_path / "docs/templates/benchmark.md").write_text(
        "{{BENCHMARK_RESULTS}}\n\n{{BENCHMARK_CHARTS}}", encoding="utf-8"
    )
    build.prepare(root=tmp_path, data=source)
    text = (tmp_path / "docs/source/benchmark.md").read_text(encoding="utf-8")
    assert "Experimental warm-up budget" in text
    assert "2 warm-up requests and 100 timed requests" in text
    assert "have not replaced" in text


def test_revision_comparison_and_mismatch_rejection():
    before = make_document({"clients": ["wreq", "wreq_st", "wreq_blocking", "ry"]})
    before["environment"]["affinity"] = [0, 1, 2, 3]
    before["environment"]["cpu"] = "<script>CPU</script>|[fake]"
    after = copy.deepcopy(before)
    after["source"].update(commit="d" * 40, dirty=True)
    after["generated_at"] = "2026-10-04T00:00:00+00:00"
    for metadata in after["clients"].values():
        for native in metadata["native"]:
            native["path"] = "/relocated/extension.so"
            if metadata["package"] == "wreq":
                native["sha256"] = "e" * 64
                metadata["version"] = "2.0"
    for cell in after["results"]:
        for sample in cell["samples"]:
            sample["seconds"] /= 2
        for rate in [cell, *cell["rounds"]]:
            rate["total_seconds"] /= 2
            rate["rps"] *= 2
            rate["mbps"] *= 2
    text = compare.render_comparison(before, after)
    after["configuration"]["warmup_requests"] = after["configuration"]["requests"]
    assert compare.render_comparison(before, after) == text
    assert text.count("#### HTTP/") == 8
    assert text.count("| +100.0 |") == len(before["results"])
    assert "16 supported cells" in text and "400 timed requests per cell" in text
    assert "d" * 40 in text and "| Yes |" in text
    assert "e" * 64 in text and "| 2&#46;0 |" in text
    assert "<script>" not in text and "&lt;script&gt;" in text
    assert "not a confidence interval" in text and "not a paired experiment" in text
    for path, value in (
        (("configuration", "seed"), 1),
        (("configuration", "warmup_requests"), 50),
        (("environment", "affinity"), [0]),
        (("environment", "python"), "3.13"),
        (("server", "sha256"), "f" * 64),
        (("clients", "ry", "version"), "2.0"),
        (("clients", "ry", "native", 0, "sha256"), "f" * 64),
        (("clients", "wreq", "runtime", "kind"), "single_thread"),
    ):
        mismatch = copy.deepcopy(after)
        target = mismatch
        for key in path[:-1]:
            target = target[key]
        target[path[-1]] = value
        with pytest.raises(ValueError, match="do(?:es)? not match"):
            compare.render_comparison(before, mismatch)
    incomplete = copy.deepcopy(after)
    incomplete["results"].pop()
    with pytest.raises(ValueError, match="Incomplete benchmark matrix"):
        compare.render_comparison(before, incomplete)
    blocking = copy.deepcopy(after)
    blocking["configuration"]["clients"] = ["wreq_blocking"]
    blocking["clients"] = {"wreq_blocking": blocking["clients"]["wreq_blocking"]}
    blocking["results"] = [
        cell for cell in blocking["results"] if cell["client"] == "wreq_blocking"
    ]
    original = copy.deepcopy(before)
    with pytest.raises(ValueError, match="configurations do not match"):
        compare.render_comparison(before, blocking)
    text = compare.render_comparison(before, blocking, api="blocking")
    assert text.count("| +100.0 |") == 4 and "Asynchronous clients" not in text
    assert "Comparison scope: blocking clients only" in text and before == original
    with pytest.raises(ValueError, match="contains no async clients"):
        compare.render_comparison(before, blocking, api="async")
    with pytest.raises(ValueError, match="Incomplete benchmark matrix"):
        compare.render_comparison(incomplete, blocking, api="blocking")
    blocking["configuration"]["seed"] += 1
    with pytest.raises(ValueError, match="configurations do not match"):
        compare.render_comparison(before, blocking, api="blocking")


def test_markdown_cli_exports_preserve_existing_files(tmp_path, capsys):
    document = make_document()
    document["environment"]["cpu"] = "Café CPU"
    source = tmp_path / "source.json"
    source.write_text(json.dumps(document), encoding="utf-8")
    for main, args, name, expected, character in (
        (
            report.main,
            ["--input", str(source), "--markdown"],
            "report.md",
            report.render_markdown(document),
            "é",
        ),
        (
            compare.main,
            ["--before", str(source), "--after", str(source)],
            "comparison.md",
            compare.render_comparison(document, document),
            "−",
        ),
    ):
        output = tmp_path / name
        main([*args, "--output", str(output)])
        assert output.read_bytes() == expected.encode("utf-8")
        assert character in output.read_text(encoding="utf-8")
        for existing in (output, source):
            previous = existing.read_bytes()
            with pytest.raises(FileExistsError):
                main([*args, "--output", str(existing)])
            assert existing.read_bytes() == previous
    assert capsys.readouterr().out == ""


def test_capability_matrix_environment_and_legacy_metadata():
    for adapter in (async_clients, blocking_clients):
        assert adapter.CAPABILITIES == {
            client: clients.CAPABILITIES[client] for client in adapter.CLIENTS
        }
    document = make_document({"clients": ["wreq", "aiohttp", "requests"]})
    for client in ("aiohttp", "requests"):
        document["clients"][client]["native"] = []
    for field in ("api", "protocols", "body_kinds"):
        document["clients"]["wreq"].pop(field)
    results.validate_document(document)
    assert not any(
        cell["client"] in {"aiohttp", "requests"} and cell["protocol"] == "h2"
        for cell in document["results"]
    )
    assert all(
        clients.supports(cell["client"], cell["protocol"], cell["body_kind"])
        for cell in document["results"]
    )
    for field in ("cpu", "platform", "machine", "cpu_count"):
        missing = copy.deepcopy(document)
        del missing["environment"][field]
        with pytest.raises(ValueError):
            results.validate_document(missing)
    unexpected = copy.deepcopy(document)
    extra = copy.deepcopy(unexpected["results"][0])
    extra["client"] = "requests"
    extra["protocol"] = "h2"
    extra["validation"]["protocol"] = "h2"
    unexpected["results"].append(extra)
    with pytest.raises(ValueError):
        results.validate_document(unexpected)
    empty = make_document({"clients": ["requests"], "protocols": ["h2"]})
    with pytest.raises(ValueError, match="supported benchmark combinations"):
        results.validate_document(empty)


def test_pure_python_metadata_requires_installed_distribution(monkeypatch):
    monkeypatch.setattr(
        clients.importlib,
        "import_module",
        lambda package: SimpleNamespace(__version__="not-an-installed-version"),
    )
    monkeypatch.setattr(clients.importlib.metadata, "version", lambda package: "2.34.2")
    metadata = clients.metadata("requests")
    assert metadata["version"] == "2.34.2" and metadata["native"] == []
    assert metadata["api"] == "blocking" and metadata["protocols"] == ["h1"]
    assert metadata["response_read"] == "iter_content(65536): reads of at most 64 KiB"

    def absent(package):
        raise importlib.metadata.PackageNotFoundError(package)

    monkeypatch.setattr(clients.importlib.metadata, "version", absent)
    with pytest.raises(RuntimeError, match="installed requests distribution"):
        clients.metadata("requests")


def test_prepare_tracked_input_and_local_snapshot_override(tmp_path, monkeypatch):
    root = docs_root(tmp_path)
    document = make_document()
    local = root / "bench/data/latest.json"
    local.parent.mkdir(parents=True)
    local.write_text(json.dumps(document), encoding="utf-8")
    original = local.read_bytes()

    def no_network(*args, **kwargs):
        raise AssertionError("Documentation data must never access the network")

    monkeypatch.setattr("urllib.request.urlopen", no_network)
    assert build.prepare(root=root) == document
    snapshot = root / "docs/source/assets/benchmark/latest.json"
    page = root / "docs/source/benchmark.md"
    assert json.loads(snapshot.read_text(encoding="utf-8")) == document
    content = page.read_text(encoding="utf-8")
    assert "{{BENCHMARK_RESULTS}}" not in content
    assert "{{BENCHMARK_CHARTS}}" not in content
    assert "<details" not in content and "<summary>" not in content
    assert '??? note "Measurement environment"' in content
    assert '???+ note "Measurement environment"' not in content
    assert (
        indent(
            report.render_markdown(
                document, environment_only=True, environment_heading=None
            ),
            "    ",
        )
        in content
    )
    assert "    | Item | Value |" in content
    assert "### Throughput comparison" not in content
    assert "| Upload / echo payload | Concurrency" not in content
    assert "### Measurement environment" not in content
    assert "### Client versions and runtimes" not in content
    assert "### Body cases" not in content
    assert "## Runtime and client differences" in content
    charts = root / "docs/source/assets/benchmark/charts"
    assert len(list(charts.glob("*.svg"))) == 16
    assert "Download this build's raw JSON" not in content
    assert "Raw JSON</a>" not in content and "Download SVG" not in content
    assert build.prepare(data=snapshot, root=root) == document
    override = root / "override.json"
    alternate = make_document(commit="d" * 40)
    override.write_text(json.dumps(alternate), encoding="utf-8")
    assert build.prepare(data=override, root=root) == alternate
    assert json.loads(snapshot.read_text(encoding="utf-8")) == alternate
    assert build.prepare(root=root) == document
    assert local.read_bytes() == original


def test_prepare_rejects_missing_invalid_and_oversized_local_data(
    tmp_path, monkeypatch
):
    root = docs_root(tmp_path)
    document = make_document()
    local = root / "bench/data/latest.json"
    local.parent.mkdir(parents=True)
    page = root / "docs/source/benchmark.md"
    snapshot = root / "docs/source/assets/benchmark/latest.json"
    charts = root / "docs/source/assets/benchmark/charts"
    for failure in ("missing", "json", "schema", "oversized", "override", "render"):
        local.write_text(json.dumps(document), encoding="utf-8")
        assert build.prepare(root=root) == document
        with monkeypatch.context() as patch:
            if failure == "missing":
                local.unlink()
            elif failure == "json":
                local.write_text("{broken", encoding="utf-8")
            elif failure == "schema":
                invalid = copy.deepcopy(document)
                invalid["results"].pop()
                local.write_text(json.dumps(invalid), encoding="utf-8")
            elif failure == "oversized":
                patch.setattr(build, "MAX_DATA_BYTES", 128)
                local.write_bytes(b" " * 129)
            elif failure == "render":

                def broken_renderer(*args):
                    raise ValueError("Rendering failed after chart generation")

                patch.setattr(build, "render_explorer", broken_renderer)
            with pytest.raises((FileNotFoundError, ValueError)):
                build.prepare(
                    data=root / "absent.json" if failure == "override" else None,
                    root=root,
                )
        assert not page.exists() and not snapshot.exists()
        assert not list(charts.glob("*.svg"))


def test_chart_explorer_controls_fallback_and_inert_metadata(tmp_path):
    document = make_document(
        {"clients": ["requests"], "protocols": ["h1"], "payload_bytes": [10240]}
    )
    document["environment"]["cpu"] = "</script><script>alert(1)</script>"
    catalog = build.write_charts(document, tmp_path)
    catalog["cases"][0]["rows"][0]["label"] = '<img src=x onerror="alert(1)">'
    catalog["description"] = document["environment"]["cpu"]
    content = render_explorer(document, catalog)
    assert 'value="blocking" selected' in content and 'value="async"' not in content
    assert 'value="h1" selected' in content and 'value="h2"' not in content
    assert content.count("data-chart-payload=") == 1
    assert content.count("data-chart-controls hidden") == 2
    assert "data-chart-previous" not in content and "data-chart-next" not in content
    assert "<noscript>" in content and "measurement details remain available" in content
    assert "Values for this chart" not in content and "data-chart-values" not in content
    assert "<table" not in content and "<details" not in content
    assert "data-chart-download" not in content and "Download SVG" not in content
    assert "Raw JSON</a>" not in content
    assets = catalog["cases"][0]["assets"]["dark"]
    assert f'src="assets/benchmark/charts/{assets["desktop"]}"' in content
    assert f'srcset="assets/benchmark/charts/{assets["mobile"]}"' in content
    assert "zero-based linear scale" in content and "requests/s (RPS)" in content
    alt = re.search(r'<img data-chart-image[^>]* alt="([^"]*)"', content).group(1)
    assert f': {catalog["cases"][0]["rows"][0]["rps"]:,.1f}' in alt
    assert '<img src=x onerror="alert(1)">' not in content
    assert "&lt;img src=x" in content
    data = re.search(
        r'<script type="application/json" data-chart-catalog>(.*?)</script>', content
    ).group(1)
    assert "</script>" not in data
    assert json.loads(data) == catalog


def blocking_documents():
    configuration = {
        "payload_bytes": list(BODY_CASES),
        "concurrency": list(CONCURRENCY_CASES),
        "rounds": 3,
        "samples": 1,
        "requests": 300,
        "stream_chunk_bytes_by_payload": {
            str(size): chunk for size, chunk in BODY_CASES.items()
        },
    }
    baseline = make_document({**configuration, "clients": ["wreq", "wreq_blocking"]})
    blocking = make_document(
        {**configuration, "clients": list(blocking_clients.CLIENTS)}, commit="d" * 40
    )
    blocking["generated_at"] = "2026-10-05T12:34:00+00:00"
    for cell in blocking["results"]:
        for sample in cell["samples"]:
            sample["seconds"] /= 2
        for rate in (cell, *cell["rounds"]):
            rate["total_seconds"] /= 2
            rate["rps"] *= 2
            rate["mbps"] *= 2
    return baseline, blocking


def test_prepare_independent_blocking_source_and_shared_environment(tmp_path):
    root = docs_root(tmp_path)
    baseline, blocking = blocking_documents()
    # Different load samples must not duplicate an otherwise identical table.
    baseline["environment"]["load_average"] = [1, 2, 3]
    blocking["environment"]["load_average"] = [4, 5, 6]
    latest = root / "bench/data/latest.json"
    latest.parent.mkdir(parents=True)
    latest_blocking = latest.with_name("latest-blocking.json")
    baseline_raw = json.dumps(baseline, indent=1).encode() + b"\n\n"
    blocking_raw = json.dumps(blocking, indent=3).encode() + b"\n"
    latest.write_bytes(baseline_raw)
    latest_blocking.write_bytes(blocking_raw)

    assert build.prepare(root=root) == baseline
    snapshot = root / "docs/source/assets/benchmark/latest.json"
    blocking_snapshot = snapshot.with_name("latest-blocking.json")
    assert snapshot.read_bytes() == latest.read_bytes() == baseline_raw
    assert (
        blocking_snapshot.read_bytes() == latest_blocking.read_bytes() == blocking_raw
    )
    content = (root / "docs/source/benchmark.md").read_text(encoding="utf-8")
    assert content.count('class="wreq-bench-provenance"') == 2
    assert "Async clients · Measured" in content
    assert "Blocking clients · Measured" in content
    assert "2026-10-03 00:00 UTC" in content and "2026-10-05 12:34 UTC" in content
    assert content.count('??? note "Measurement environment"') == 1
    assert "### Measurement environment" not in content
    catalog = json.loads(
        re.search(r"data-chart-catalog>(.*?)</script>", content).group(1)
    )
    assert "source" not in catalog and "generated_at" not in catalog
    assert catalog["measurements"]["async"]["source"] == baseline["source"]
    assert catalog["measurements"]["blocking"]["source"] == blocking["source"]
    charts = root / "docs/source/assets/benchmark/charts"
    for api, document in (("async", baseline), ("blocking", blocking)):
        case = next(case for case in catalog["cases"] if case["api"] == api)
        client = "wreq" if api == "async" else "wreq_blocking"
        row = next(row for row in case["rows"] if row["client"] == client)
        measured = next(
            cell
            for cell in document["results"]
            if cell["client"] == client
            and all(
                cell[key] == case[key]
                for key in (
                    "protocol",
                    "body_kind",
                    "payload_bytes",
                    "concurrency",
                )
            )
        )
        assert row["rps"] == measured["rps"]
        svg = (charts / case["assets"]["dark"]["desktop"]).read_text(encoding="utf-8")
        assert f"Revision {document['source']['commit'][:12]}" in svg
        assert document["generated_at"][:10] in svg
    # Generated copies can themselves be inputs; both must be read before cleanup.
    assert (
        build.prepare(data=snapshot, blocking_data=blocking_snapshot, root=root)
        == baseline
    )
    assert snapshot.read_bytes() == baseline_raw
    assert blocking_snapshot.read_bytes() == blocking_raw
    blocking["environment"]["cpu"] = "another CPU"
    distinct = build.measurement_environments(baseline, blocking)
    assert "### Async measurement environment" in distinct
    assert "### Blocking measurement environment" in distinct
    assert "another CPU" in distinct and "test CPU" in distinct


def test_prepare_rejects_invalid_blocking_overlay_and_clears_both_snapshots(tmp_path):
    root = docs_root(tmp_path)
    baseline, blocking = blocking_documents()
    latest = root / "bench/data/latest.json"
    latest.parent.mkdir(parents=True)
    overlay = latest.with_name("latest-blocking.json")
    page = root / "docs/source/benchmark.md"
    snapshot = root / "docs/source/assets/benchmark/latest.json"
    blocking_snapshot = snapshot.with_name("latest-blocking.json")
    charts = snapshot.parent / "charts"
    charts.mkdir(parents=True)
    for failure in ("json", "subset", "explicit_missing", "axes"):
        latest.write_text(json.dumps(baseline))
        overlay.write_text(json.dumps(blocking))
        page.write_text("stale page")
        snapshot.write_bytes(b"stale async data")
        blocking_snapshot.write_bytes(b"stale blocking data")
        (charts / "stale.svg").write_text("stale chart")
        candidate = None
        if failure == "json":
            overlay.write_text("{broken")
        elif failure == "subset":
            overlay.write_text(
                json.dumps(
                    make_document(
                        {
                            **blocking["configuration"],
                            "payload_bytes": [10240],
                        }
                    )
                )
            )
        elif failure == "explicit_missing":
            candidate = root / "missing.json"
        else:
            latest.write_text(
                json.dumps(
                    make_document(
                        {
                            **baseline["configuration"],
                            "protocols": ["h1"],
                        }
                    )
                )
            )
        with pytest.raises((FileNotFoundError, ValueError)):
            build.prepare(root=root, blocking_data=candidate)
        assert not page.exists() and not snapshot.exists()
        assert not blocking_snapshot.exists() and not list(charts.glob("*.svg"))


def test_explicit_data_preview_does_not_select_default_blocking_overlay(tmp_path):
    root = docs_root(tmp_path)
    baseline, blocking = blocking_documents()
    latest = root / "bench/data/latest.json"
    latest.parent.mkdir(parents=True)
    overlay = latest.with_name("latest-blocking.json")
    baseline_raw = json.dumps(baseline).encode()
    blocking_raw = json.dumps(blocking).encode()
    latest.write_bytes(baseline_raw)
    overlay.write_bytes(blocking_raw)
    smaller = make_document({"protocols": ["h1"], "body_kinds": ["full"]})
    candidate = root / "smaller.json"
    candidate_raw = json.dumps(smaller).encode()
    candidate.write_bytes(candidate_raw)

    assert build.prepare(data=candidate, root=root) == smaller
    page = root / "docs/source/benchmark.md"
    snapshot = root / "docs/source/assets/benchmark/latest.json"
    blocking_snapshot = snapshot.with_name("latest-blocking.json")
    charts = snapshot.parent / "charts"
    assert snapshot.read_bytes() == candidate_raw and not blocking_snapshot.exists()
    content = page.read_text(encoding="utf-8")
    assert content.count('class="wreq-bench-provenance"') == 1
    assert len(list(charts.glob("*.svg"))) == 4

    with pytest.raises(ValueError, match="Async and blocking chart axes do not match"):
        build.prepare(data=candidate, blocking_data=overlay, root=root)
    assert (
        not page.exists() and not snapshot.exists() and not blocking_snapshot.exists()
    )
    assert not list(charts.glob("*.svg"))
    assert latest.read_bytes() == baseline_raw and overlay.read_bytes() == blocking_raw


def test_measurement_sources_can_partition_concurrency_without_overlap():
    document = make_document({"clients": ["wreq", "wreq_st"], "concurrency": [2, 10]})
    document["measurement_sources"] = [
        {
            "label": f"Concurrency {concurrency}",
            "clients": list(document["configuration"]["clients"]),
            "concurrency": [concurrency],
            "source": copy.deepcopy(document["source"]),
            "generated_at": document["generated_at"],
        }
        for concurrency in (2, 10)
    ]
    results.validate_document(document)
    for axis in ([2], [2, 10], [10, 10], [50], [True], []):
        invalid = copy.deepcopy(document)
        invalid["measurement_sources"][1]["concurrency"] = axis
        with pytest.raises(ValueError):
            results.validate_document(invalid)
    text = provenance(document, "Async clients")
    assert "Async clients · Combined preview" in text
    assert "Concurrency 2" in text and "Concurrency 10" in text
    text = report.render_markdown(document)
    assert "Combined results from separate measured runs" in text
    assert "Concurrency 2" in text and "Concurrency 10" in text
