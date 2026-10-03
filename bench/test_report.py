"""Static reports and documentation preparation, without network or site builds."""

import copy
import importlib.metadata
import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from bench import clients, compare, report, results
from bench import async_clients, blocking_clients
from bench.test_benchmark import make_document
from bench.workloads import BODY_CASES
from docs import build


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
            assert f"#### {protocol} — {kind} upload" in text
    assert "10 KiB" in text and "1 MiB" in text and "4 MiB" in text
    assert text.count("| Upload / echo payload") == 8
    assert text.count("Unit: **requests/s (RPS, requests per second)**") == 8
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
    assert (
        '<details markdown="1">' in text
        and "<summary>Adapter settings</summary>" in text
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
    assert text.count("Unit: **requests/s (RPS, requests per second)**") == 8
    full = make_document({**dynamic["configuration"], "body_kinds": ["full"]})
    text = report.render_markdown(full)
    assert "both Full and Stream" not in text and "— Stream upload" not in text


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
    assert text.count("#### HTTP/") == 8
    assert text.count("| +100.0 |") == len(before["results"])
    assert "16 supported cells" in text and "400 timed requests per cell" in text
    assert "d" * 40 in text and "| Yes |" in text
    assert "e" * 64 in text and "| 2&#46;0 |" in text
    assert "<script>" not in text and "&lt;script&gt;" in text
    assert "not a confidence interval" in text and "not a paired experiment" in text
    for path, value in (
        (("configuration", "seed"), 1),
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


def test_markdown_cli_exports_preserve_existing_files(tmp_path, capsys):
    document = make_document()
    source = tmp_path / "source.json"
    source.write_text(json.dumps(document), encoding="utf-8")
    for main, args, name, expected, character in (
        (
            report.main,
            ["--input", str(source), "--markdown"],
            "report.md",
            report.render_markdown(document),
            "—",
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
    assert "{{BENCHMARK_RESULTS}}" not in page.read_text(encoding="utf-8")
    assert "assets/benchmark/latest.json" in page.read_text(encoding="utf-8")
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
    for failure in ("missing", "json", "schema", "oversized", "override"):
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
            with pytest.raises((FileNotFoundError, ValueError)):
                build.prepare(
                    data=root / "absent.json" if failure == "override" else None,
                    root=root,
                )
        assert not page.exists() and not snapshot.exists()
