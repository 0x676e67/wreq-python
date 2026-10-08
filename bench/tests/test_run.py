"""Artifact preservation and explicit publication without network requests."""

import copy
import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from bench import run
from bench import benchmark
from bench.clients import CLIENTS
from bench.results import BLOCKING_CLIENTS, require_publish
from bench.tests.test_benchmark import make_document
from bench.workloads import BODY_CASES, CONCURRENCY_CASES


@pytest.fixture(autouse=True)
def isolated_blocking_selection(tmp_path, monkeypatch):
    monkeypatch.setattr(run, "LATEST_BLOCKING", tmp_path / "latest-blocking.json")


def publication_config(blocking=False):
    return {
        "clients": list(BLOCKING_CLIENTS if blocking else CLIENTS),
        "protocols": ["h1", "h2"],
        "body_kinds": ["full", "stream"],
        "payload_bytes": list(BODY_CASES),
        "concurrency": list(CONCURRENCY_CASES),
        "requests": 300,
        "warmup_requests": 200,
        "rounds": 3,
        "samples": 1,
        "warmup": 1,
    }


def candidate(tmp_path, full=False, blocking=False):
    config = publication_config(blocking) if full or blocking else None
    path = tmp_path / "history.json"
    # Non-default whitespace proves publication does not reserialize the JSON.
    raw = json.dumps(make_document(config), indent=1).encode() + b"\n\n"
    path.write_bytes(raw)
    return path, raw


def test_separate_publication_gates_reject_subsets_and_weak_batches():
    assert len(BLOCKING_CLIENTS) == 9 and len(CLIENTS) == 18
    for blocking in (False, True):
        config = publication_config(blocking)
        require_publish(config, blocking=blocking)
        with pytest.raises(ValueError):
            require_publish({**config, "warmup_requests": 199}, blocking=blocking)
        with pytest.raises(ValueError):
            require_publish(config, blocking=not blocking)
        for key in (
            "clients",
            "protocols",
            "body_kinds",
            "payload_bytes",
            "concurrency",
        ):
            invalid = copy.deepcopy(config)
            invalid[key].pop()
            with pytest.raises(ValueError):
                require_publish(invalid, blocking=blocking)
            invalid[key] = [*config[key], config[key][0]]
            with pytest.raises(ValueError):
                require_publish(invalid, blocking=blocking)
        for key in ("requests", "rounds", "warmup", "samples"):
            invalid = {**config, key: config[key] - 1}
            with pytest.raises(ValueError):
                require_publish(invalid, blocking=blocking)
        with pytest.raises(ValueError):
            require_publish({**config, "warmup": True}, blocking=blocking)


def test_default_suite_includes_wreq_mt_st_and_reads_legacy_snapshots():
    args = benchmark.parse_args(["--server", "server"])
    assert {"wreq", "wreq_st", "wreq_blocking", "wreq_blocking_st", "wreq_blocking_ct"} <= set(args.clients)
    assert len(args.clients) == 18
    assert args.requests == 300 and args.warmup_requests == 200
    assert args.concurrency == [2, 10, 50, 100]
    for blocking in (False, True):
        previous = {**publication_config(blocking), "concurrency": [10, 50, 100]}
        require_publish(previous, blocking=blocking, allow_legacy_snapshot=True)
        with pytest.raises(ValueError):
            require_publish(previous, blocking=blocking)
        legacy = publication_config(blocking)
        for client in ("wreq_blocking_ct", "wreq_blocking_st"):
            legacy["clients"].remove(client)
            require_publish(legacy, blocking=blocking, allow_legacy_snapshot=True)
            with pytest.raises(ValueError):
                require_publish(legacy, blocking=blocking)
        historical = {
            **publication_config(blocking),
            "concurrency": [150, 10, 100, 50],
            "requests": 300,
            "warmup_requests": 300,
        }
        require_publish(historical, blocking=blocking, allow_legacy_snapshot=True)
        with pytest.raises(ValueError):
            require_publish(historical, blocking=blocking)
        with pytest.raises(ValueError):
            require_publish(
                {**historical, "warmup_requests": 100},
                blocking=blocking,
                allow_legacy_snapshot=True,
            )
        legacy["clients"].remove("wreq_blocking")
        with pytest.raises(ValueError):
            require_publish(legacy, blocking=blocking, allow_legacy_snapshot=True)


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
    assert run.main(["--input", str(source), "--publish-blocking"]) == 1
    assert run.main(["--server", "unused", "--publish-blocking"]) == 1
    with pytest.raises(SystemExit):
        run.main(["--input", str(source), "--publish", "--publish-blocking"])
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


@pytest.mark.parametrize("option", ["--publish-blocking", "--build-docs"])
def test_blocking_publish_freezes_both_sources_and_preserves_async(
    tmp_path, monkeypatch, option
):
    source, raw = candidate(tmp_path, blocking=True)
    latest = tmp_path / "latest.json"
    latest_raw = json.dumps(make_document(publication_config()), indent=3).encode()
    latest.write_bytes(latest_raw)
    latest_blocking = tmp_path / "latest-blocking.json"
    latest_blocking.write_bytes(b"previous blocking selection")
    monkeypatch.setattr(run, "LATEST", latest)
    monkeypatch.setattr(run, "LATEST_BLOCKING", latest_blocking)
    calls = []

    def built(command, **kwargs):
        calls.append(command)
        full = Path(command[command.index("--data") + 1])
        blocking = Path(command[command.index("--blocking-data") + 1])
        assert full.read_bytes() == latest_raw and blocking.read_bytes() == raw
        assert full != latest and blocking != source and kwargs["cwd"] == run.ROOT
        if len(calls) == 1:
            return SimpleNamespace(returncode=2)
        source.write_bytes(b"changed outside the script")
        assert blocking.read_bytes() == raw
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(run.subprocess, "run", built)
    args = ["--input", str(source), option]
    assert run.main(args) == 1
    assert latest.read_bytes() == latest_raw
    assert latest_blocking.read_bytes() == b"previous blocking selection"
    assert source.read_bytes() == raw
    assert run.main(args) == 0
    assert latest.read_bytes() == latest_raw
    assert latest_blocking.read_bytes() == (
        raw if option == "--publish-blocking" else b"previous blocking selection"
    )
    assert len(calls) == 2


@pytest.mark.parametrize("option", ["--publish", "--build-docs"])
def test_full_publish_freezes_existing_blocking_selection(
    tmp_path, monkeypatch, option
):
    source, raw = candidate(tmp_path, full=True)
    latest = tmp_path / "latest.json"
    latest.write_bytes(b"previous full selection")
    latest_blocking = tmp_path / "latest-blocking.json"
    legacy_configuration = publication_config(True)
    for client in ("wreq_blocking_st", "wreq_blocking_ct"):
        legacy_configuration["clients"].remove(client)
    blocking_raw = json.dumps(make_document(legacy_configuration), indent=3).encode()
    latest_blocking.write_bytes(blocking_raw)
    monkeypatch.setattr(run, "LATEST", latest)
    monkeypatch.setattr(run, "LATEST_BLOCKING", latest_blocking)

    def built(command, **kwargs):
        assert Path(command[command.index("--data") + 1]).read_bytes() == raw
        if option == "--publish":
            overlay = Path(command[command.index("--blocking-data") + 1])
            assert overlay != latest_blocking and overlay.read_bytes() == blocking_raw
        else:
            assert "--blocking-data" not in command
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(run.subprocess, "run", built)
    assert run.main(["--input", str(source), option]) == 0
    assert latest.read_bytes() == (
        raw if option == "--publish" else b"previous full selection"
    )
    assert latest_blocking.read_bytes() == blocking_raw
    if option == "--publish":
        latest_blocking.write_bytes(b"invalid overlay")
        assert run.main(["--input", str(source), option]) == 1
        assert latest.read_bytes() == raw


def test_blocking_publish_rejects_invalid_base_and_reserved_paths(
    tmp_path, monkeypatch
):
    source, raw = candidate(tmp_path, blocking=True)
    latest = tmp_path / "latest.json"
    latest.write_bytes(b"invalid base")
    latest_blocking = tmp_path / "latest-blocking.json"
    monkeypatch.setattr(run, "LATEST", latest)
    monkeypatch.setattr(run, "LATEST_BLOCKING", latest_blocking)
    monkeypatch.setattr(run.benchmark, "source_info", lambda: {"commit": "a" * 40})
    monkeypatch.setattr(
        run.subprocess,
        "run",
        lambda *a, **kw: (_ for _ in ()).throw(AssertionError("Must not build")),
    )
    assert run.main(["--input", str(source), "--publish-blocking"]) == 1
    assert not latest_blocking.exists() and latest.read_bytes() == b"invalid base"
    assert source.read_bytes() == raw
    assert run.main(["--input", str(source), "--report", str(latest_blocking)]) == 1
    assert run.main(["--server", "unused", "--output", str(latest_blocking)]) == 1
    assert not latest_blocking.exists()


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
