"""Artifact preservation and explicit publication without network requests."""

import json
from pathlib import Path
from types import SimpleNamespace

from bench import run
from bench.clients import CLIENTS
from bench.test_benchmark import make_document
from bench.workloads import BODY_CASES, CONCURRENCY_CASES


def candidate(tmp_path, full=False):
    config = (
        {
            "clients": list(CLIENTS),
            "protocols": ["h1", "h2"],
            "body_kinds": ["full", "stream"],
            "payload_bytes": list(BODY_CASES),
            "concurrency": list(CONCURRENCY_CASES),
            "requests": 300,
            "rounds": 3,
            "samples": 1,
            "warmup": 1,
        }
        if full
        else None
    )
    path = tmp_path / "history.json"
    # Non-default whitespace proves publication does not reserialize the JSON.
    raw = json.dumps(make_document(config), indent=1).encode() + b"\n\n"
    path.write_bytes(raw)
    return path, raw


def test_existing_input_and_identical_report_reuse(tmp_path, monkeypatch):
    source, raw = candidate(tmp_path)
    monkeypatch.setattr(
        run.subprocess,
        "run",
        lambda *a, **kw: (_ for _ in ()).throw(AssertionError("Must not measure")),
    )
    assert run.main(["--input", str(source)]) == 0
    report = source.with_suffix(".report.md")
    content = report.read_bytes()
    assert run.main(["--input", str(source)]) == 0
    assert report.read_bytes() == content and source.read_bytes() == raw
    report.write_bytes(b"Precious old report")
    assert run.main(["--input", str(source)]) == 1
    assert report.read_bytes() == b"Precious old report"


def test_incomplete_publish_rejected_before_measurement(tmp_path, monkeypatch):
    source, raw = candidate(tmp_path)
    latest = tmp_path / "latest.json"
    latest.write_bytes(b"old")
    monkeypatch.setattr(run, "LATEST", latest)
    monkeypatch.setattr(
        run.subprocess,
        "run",
        lambda *a, **kw: (_ for _ in ()).throw(AssertionError("Must not run")),
    )
    assert run.main(["--input", str(source), "--publish"]) == 1
    assert run.main(["--server", "unused", "--clients", "wreq", "--publish"]) == 1
    assert latest.read_bytes() == b"old" and source.read_bytes() == raw


def test_documentation_failure_preserves_latest_and_candidate(tmp_path, monkeypatch):
    source, raw = candidate(tmp_path, full=True)
    latest = tmp_path / "latest.json"
    latest.write_bytes(b"old")
    monkeypatch.setattr(run, "LATEST", latest)
    builds = []

    def failed(command, **kwargs):
        builds.append(command)
        frozen = Path(command[-1])
        assert frozen.read_bytes() == raw
        assert kwargs["cwd"] == run.ROOT
        return SimpleNamespace(returncode=2)

    monkeypatch.setattr(run.subprocess, "run", failed)
    assert (
        run.main(["--input", str(source), "--publish", "--docs-python", "docs-python"])
        == 1
    )
    assert len(builds) == 1 and builds[0][0] == "docs-python"
    assert latest.read_bytes() == b"old" and source.read_bytes() == raw
    assert source.with_suffix(".report.md").exists()


def test_successful_publish_uses_frozen_original_bytes(tmp_path, monkeypatch):
    source, raw = candidate(tmp_path, full=True)
    latest = tmp_path / "latest.json"
    latest.write_bytes(b"old")
    monkeypatch.setattr(run, "LATEST", latest)

    def built(command, **kwargs):
        assert Path(command[-1]).read_bytes() == raw
        # Editing the source while building cannot change the selected snapshot.
        source.write_bytes(b"changed outside the script")
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(run.subprocess, "run", built)
    assert run.main(["--input", str(source), "--publish"]) == 0
    assert latest.read_bytes() == raw


def test_measurement_uses_new_names_and_preserves_failure_logs(tmp_path, monkeypatch):
    latest = tmp_path / "latest.json"
    latest.write_bytes(b"old")
    output = tmp_path / "new.json"
    monkeypatch.setattr(run, "LATEST", latest)
    monkeypatch.setattr(run.benchmark, "source_info", lambda: {"commit": "a" * 40})

    def failed(command, **kwargs):
        assert command[-4:-2] == ["--server", str(Path("unused").resolve())]
        assert command[-2:] == ["--output", str(output)]
        kwargs["stdout"].write(b"progress\n")
        kwargs["stderr"].write(b"diagnostic\n")
        return SimpleNamespace(returncode=1)

    monkeypatch.setattr(run.subprocess, "run", failed)
    args = ["--server", "unused", "--clients", "wreq", "--output", str(output)]
    assert run.main(args) == 1
    assert output.with_name("new-stdout.log").read_bytes() == b"progress\n"
    assert output.with_name("new-stderr.log").read_bytes() == b"diagnostic\n"
    assert run.main(args) == 1
    assert latest.read_bytes() == b"old" and not output.exists()
    monkeypatch.setattr(run, "ROOT", tmp_path)
    occupied_report = tmp_path / "old.report.md"
    occupied_report.write_bytes(b"Precious report")
    with monkeypatch.context() as patch:
        patch.setattr(
            run.subprocess,
            "run",
            lambda *a, **kw: (_ for _ in ()).throw(
                AssertionError("Must reject before measuring")
            ),
        )
        assert (
            run.main(
                [
                    "--server",
                    "unused",
                    "--clients",
                    "wreq",
                    "--report",
                    str(occupied_report),
                ]
            )
            == 1
        )
    assert occupied_report.read_bytes() == b"Precious report"
    monkeypatch.chdir(tmp_path)
    generated = []

    def measured(command, **kwargs):
        assert command[-4:-2] == ["--server", str(tmp_path / "unused")]
        path = Path(command[-1])
        generated.append(path)
        assert "-aaaaaaaaaaaa-" in path.name and path.suffix == ".json"
        assert not path.exists()
        path.write_text(json.dumps(make_document()), encoding="utf-8")
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(run.subprocess, "run", measured)
    args = ["--server", "unused", "--clients", "wreq"]
    assert run.main(args) == run.main(args) == 0
    assert generated[0] != generated[1]
    assert all(path.with_suffix(".report.md").exists() for path in generated)
    assert latest.read_bytes() == b"old"
