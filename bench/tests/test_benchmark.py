"""Focused checks for benchmark scheduling and complete-result storage."""

import asyncio
import copy
from contextlib import asynccontextmanager, contextmanager
from itertools import product
import json
import time
import sys
from types import ModuleType

import pytest

from bench import benchmark, clients, results
from bench.workloads import BODY_CASES, prepare_chunks, upload_chunk_bytes


def make_document(configuration=None, commit="a" * 40):
    """Small valid document, or a complete matrix for a supplied configuration."""
    config = {
        "clients": ["wreq"],
        "protocols": ["h1", "h2"],
        "body_kinds": ["full", "stream"],
        "payload_bytes": [10240],
        "concurrency": [2],
        "rounds": 2,
        "samples": 2,
        "warmup": 1,
        "requests": 100,
        "stream_chunk_bytes": 65536,
        "server_workers": 4,
        "seed": 160130,
        "tls_version": "1.3",
        "tls_verification": False,
        "response_consumption": "native chunks to EOF",
        "throughput_unit": "MB/s (decimal, response payload only)",
    }
    if configuration is not None:
        config.update(configuration)
    rows = []
    for client, protocol, kind, size, concurrency in product(
        config["clients"],
        config["protocols"],
        config["body_kinds"],
        config["payload_bytes"],
        config["concurrency"],
    ):
        if not clients.supports(client, protocol, kind):
            continue
        for round_id in range(1, config["rounds"] + 1):
            rows.append(
                {
                    "client": client,
                    "protocol": protocol,
                    "body_kind": kind,
                    "payload_bytes": size,
                    "concurrency": concurrency,
                    "round": round_id,
                    "samples": [
                        {"seconds": float(round_id * (i + 1)), "cpu_seconds": 0.1}
                        for i in range(config["samples"])
                    ],
                    "warmup": [
                        {"seconds": 1.0, "cpu_seconds": 0.1}
                        for _ in range(config["warmup"])
                    ],
                    "validation": {
                        "status": 200,
                        "protocol": protocol,
                        "response_bytes": size,
                    },
                }
            )
    return {
        "schema_version": 1,
        "generated_at": "2026-10-03T00:00:00+00:00",
        "source": {
            "repository": "https://github.com/0x676e67/wreq-python",
            "commit": commit,
            "dirty": False,
        },
        "environment": {
            "python": "3.14.6",
            "implementation": "CPython",
            "platform": "Linux",
            "machine": "x86_64",
            "cpu": "test CPU",
            "cpu_count": 4,
            "event_loop": "asyncio",
        },
        "configuration": config,
        "clients": {
            client: {
                "package": clients.PACKAGES[client],
                "version": "1.0",
                "runtime": {"kind": "default"},
                "native": [
                    {"module": client, "path": f"/tmp/{client}.so", "sha256": "b" * 64}
                ],
                **clients.CAPABILITIES[client],
            }
            for client in config["clients"]
        },
        "server": {
            "kind": "rust-tls-echo",
            "executable": "/tmp/benchmark-server",
            "sha256": "c" * 64,
            "workers": config["server_workers"],
            "protocols": [
                {
                    "protocol": protocol,
                    "url": "https://127.0.0.1:443",
                    "workers": config["server_workers"],
                }
                for protocol in config["protocols"]
            ],
        },
        "results": results.aggregate(rows, config["requests"]),
    }


def test_configuration_and_aggregation():
    defaults = benchmark.parse_args(["--server", "server"])
    assert defaults.sizes == list(BODY_CASES)
    assert defaults.concurrency == [10, 50, 100, 150] and defaults.requests == 300
    for size, chunk_bytes in {**BODY_CASES, 123: 123, 70000: 65536}.items():
        body = b"a" * size
        chunks = prepare_chunks(body, "stream")
        assert upload_chunk_bytes(size) == chunk_bytes
        assert len(chunks[0]) == chunk_bytes and max(map(len, chunks)) <= chunk_bytes
        assert b"".join(chunks) == body and prepare_chunks(body, "full") == ()
    args = benchmark.parse_args(
        [
            "--server",
            "server",
            "--clients",
            "wreq,ry",
            "--sizes",
            "10240,4194304",
            "--warmup",
            "0",
        ]
    )
    assert args.clients == ["wreq", "ry"] and args.sizes == [10240, 4194304]
    assert args.protocols == ["h1", "h2"] and args.warmup == 0
    with pytest.raises(SystemExit):
        benchmark.parse_args(
            ["--server", "server", "--clients", "requests,aiohttp", "--protocols", "h2"]
        )
    document = make_document()
    results.validate_document(document)
    cell = document["results"][0]
    assert cell["total_requests"] == 400
    assert cell["total_seconds"] == 9
    assert cell["rps"] == pytest.approx(400 / 9)
    assert cell["rps"] != pytest.approx(sum(r["rps"] for r in cell["rounds"]) / 2)
    assert cell["mbps"] == pytest.approx(10240 * cell["rps"] / 1_000_000)
    document["configuration"]["stream_chunk_bytes_by_payload"] = {"10240": 10240}
    results.validate_document(document)
    for bad in ({}, {"10240": 0}, {"10240": 10240, "1024": 1024}):
        document["configuration"]["stream_chunk_bytes_by_payload"] = bad
        with pytest.raises(ValueError, match="chunk mapping"):
            results.validate_document(document)


@pytest.mark.parametrize(
    "option,value",
    [
        ("--protocols", "http"),
        ("--clients", "unknown"),
        ("--body-kinds", "buffered"),
        ("--sizes", "0"),
        ("--concurrency", "10,10"),
        ("--rounds", "0"),
        ("--warmup", "-1"),
        ("--requests", "1"),
    ],
)
def test_invalid_cli(option, value):
    with pytest.raises(SystemExit):
        benchmark.parse_args(["--server", "server", option, value])


def test_closed_loop_and_cancellation():
    async def check():
        active = peak = completed = 0
        per_worker = {}

        async def post():
            nonlocal active, peak, completed
            task = asyncio.current_task()
            per_worker[task] = per_worker.get(task, 0) + 1
            active += 1
            peak = max(peak, active)
            await asyncio.sleep(0)
            active -= 1
            completed += 1

        await benchmark.closed_loop(post, 17, 3)
        assert completed == 17 and peak == 3 and active == 0
        assert list(per_worker.values()) == [6, 6, 5]
        started = cancelled = 0

        async def fail():
            nonlocal started, cancelled
            started += 1
            if started == 1:
                await asyncio.sleep(0)
                raise RuntimeError("request failure")
            try:
                await asyncio.Event().wait()
            finally:
                cancelled += 1

        with pytest.raises(ExceptionGroup):
            await benchmark.closed_loop(fail, 100, 3)
        assert started == 3 and cancelled == 2

    asyncio.run(check())


def test_blocking_workers_and_failure_cleanup(monkeypatch):
    calls = []
    closed = []
    shared_chunks = []
    failing = False

    @contextmanager
    def operation(*args, chunks):
        index = len(calls)
        calls.append(0)
        shared_chunks.append(chunks)
        active = False

        def post():
            nonlocal active
            assert not active
            active = True
            try:
                calls[index] += 1
                time.sleep(0.001)
                if failing and index == 0 and calls[index] > 1:
                    raise RuntimeError("blocking request failure")
                return {"status": 200, "protocol": "h1", "response_bytes": 10240}
            finally:
                active = False

        try:
            yield post
        finally:
            assert not active
            closed.append(index)

    monkeypatch.setattr(benchmark, "blocking_operations", operation)
    request = {
        "payload_bytes": 10240,
        "body_kind": "stream",
        "protocol": "h1",
        "url": "https://127.0.0.1:1",
        "requests": 17,
        "concurrency": 3,
        "warmup": 0,
        "samples": 1,
    }
    result = benchmark.run_blocking_case("requests", request)
    assert calls == [7, 7, 6]  # One untimed request, then the 6/6/5 batch.
    assert closed == [2, 1, 0]
    assert all(chunks is shared_chunks[0] for chunks in shared_chunks)
    assert result["samples"][0]["seconds"] > 0
    calls.clear()
    closed.clear()
    failing = True
    with pytest.raises(RuntimeError, match="blocking request failure"):
        benchmark.run_blocking_case("requests", request)
    assert closed == [2, 1, 0]


def test_independent_warmup_budget(monkeypatch):
    args = benchmark.parse_args(["--server", "server", "--warmup-requests", "150"])
    assert args.requests == 300 and args.warmup_requests == 150
    assert benchmark.parse_args(["--server", "server"]).warmup_requests == 300
    with pytest.raises(SystemExit):
        benchmark.parse_args(["--server", "server", "--warmup-requests", "149"])
    calls = []

    @contextmanager
    def blocking(*args, **kwargs):
        index = len(calls)
        calls.append(0)

        def post():
            calls[index] += 1
            return {"status": 200, "protocol": "h1", "response_bytes": 1024}

        yield post

    @asynccontextmanager
    async def asynchronous(*args):
        calls.append(0)

        async def post():
            calls[0] += 1
            return {"status": 200, "protocol": "h1", "response_bytes": 1024}

        yield post

    request = {
        "payload_bytes": 1024,
        "body_kind": "full",
        "protocol": "h1",
        "url": "https://127.0.0.1:1",
        "requests": 6,
        "warmup_requests": 3,
        "concurrency": 3,
        "warmup": 1,
        "samples": 2,
    }
    monkeypatch.setattr(benchmark, "blocking_operations", blocking)
    result = benchmark.run_blocking_case("requests", request)
    assert calls == [
        6,
        6,
        6,
    ]  # Validation + one warm-up + four timed requests per worker.
    assert len(result["warmup"]) == 1 and len(result["samples"]) == 2
    calls.clear()
    monkeypatch.setattr(benchmark, "operations", asynchronous)
    asyncio.run(benchmark.run_case("wreq", request))
    assert calls == [16]  # Validation + three warm-up + twelve timed requests.
    document = make_document({"warmup_requests": 2})
    results.validate_document(document)
    document["configuration"]["warmup_requests"] = 1
    with pytest.raises(ValueError, match="warm-up request budget"):
        results.validate_document(document)


def test_reject_incomplete_or_corrupt_results():
    base = make_document()
    invalid = []
    document = copy.deepcopy(base)
    document["results"].pop()
    invalid.append(document)
    document = copy.deepcopy(base)
    document["results"].append(copy.deepcopy(document["results"][0]))
    invalid.append(document)
    for seconds in (0, -1, float("nan"), float("inf")):
        document = copy.deepcopy(base)
        document["results"][0]["samples"][0]["seconds"] = seconds
        invalid.append(document)
    for field, value in (("payload_bytes", 1), ("rps", 1), ("total_requests", 1)):
        document = copy.deepcopy(base)
        document["results"][0][field] = value
        invalid.append(document)
    document = copy.deepcopy(base)
    document["results"][0]["validation"]["protocol"] = "h2"
    invalid.append(document)
    document = copy.deepcopy(base)
    document["results"][0]["rounds"].pop()
    invalid.append(document)
    document = copy.deepcopy(base)
    document["clients"]["wreq"]["native"] = []
    invalid.append(document)
    for document in invalid:
        with pytest.raises(ValueError):
            results.validate_document(document)


def test_blocking_st_shares_one_network_runtime(monkeypatch):
    runtimes, received = [], []
    runtime_module = ModuleType("wreq.runtime")

    def create_runtime(**options):
        assert options == {"workers": 1, "work_steal": False}
        runtime = object()
        runtimes.append(runtime)
        return runtime

    runtime_module.Runtime = create_runtime
    monkeypatch.setitem(sys.modules, "wreq.runtime", runtime_module)

    @contextmanager
    def operation(client, protocol, url, body, kind, **options):
        received.append(options.get("runtime"))
        yield lambda: {"status": 200, "protocol": protocol, "response_bytes": len(body)}

    monkeypatch.setattr(benchmark, "blocking_operations", operation)
    request = {
        "payload_bytes": 1024,
        "body_kind": "full",
        "protocol": "h1",
        "url": "https://localhost",
        "concurrency": 3,
        "requests": 6,
        "warmup_requests": 3,
        "warmup": 1,
        "samples": 1,
    }
    benchmark.run_blocking_case("wreq_blocking_st", request)
    assert len(runtimes) == 1 and received == [runtimes[0]] * 3
    received.clear()
    benchmark.run_blocking_case("wreq_blocking", request)
    assert len(runtimes) == 1 and received == [None] * 3


def test_native_chunk_and_response_validation():
    async def check():
        chunks = (b"a", b"b")
        assert [chunk async for chunk in clients.parts(chunks)] == list(chunks)

    asyncio.run(check())
    assert clients.normalize_version("Version.HTTP_11") == "h1"
    assert clients.normalize_version("HTTP/2.0") == "h2"
    with pytest.raises(ValueError):
        clients.normalize_version("HTTP/3")
    assert (
        clients.validate_length({"status": 200, "protocol": "h1"}, 5, 5)[
            "response_bytes"
        ]
        == 5
    )
    with pytest.raises(RuntimeError):
        clients.validate_length({}, 4, 5)


def test_atomic_output_preserves_previous_result(tmp_path):
    output = tmp_path / "latest.json"
    document = make_document()
    results.write_atomic(output, document, overwrite=False)
    previous = output.read_bytes()
    assert json.loads(previous) == document
    with pytest.raises(FileExistsError):
        results.write_atomic(output, document, overwrite=False)
    assert output.read_bytes() == previous and list(tmp_path.iterdir()) == [output]
    args = benchmark.parse_args(["--server", str(output), "--output", str(output)])
    with pytest.raises(ValueError, match="Refusing to overwrite"):
        asyncio.run(benchmark.orchestrate(args))
    assert output.read_bytes() == previous
    document["results"][0]["rps"] = float("nan")
    with pytest.raises(ValueError):
        results.write_atomic(output, document)
    assert output.read_bytes() == previous
    assert list(tmp_path.iterdir()) == [output]


def test_environment_cpu_model_and_platform(monkeypatch):
    for system, model in (
        ("Linux", "AMD Ryzen 9 9950X"),
        ("Darwin", "Apple M3 Max"),
        ("Windows", "Intel Core Ultra"),
    ):
        with monkeypatch.context() as patch:
            patch.setattr(benchmark.platform, "system", lambda: system)
            patch.setattr(benchmark.platform, "processor", lambda: "generic processor")
            patch.setattr(benchmark.platform, "machine", lambda: "test-arch")
            patch.setattr(benchmark.platform, "platform", lambda: f"{system}-test")
            patch.setattr(benchmark.os, "cpu_count", lambda: 16)
            patch.setattr(benchmark.Path, "exists", lambda _: True)
            patch.setattr(
                benchmark.Path, "read_text", lambda _: f"model name\t: {model}\n"
            )
            commands = []

            def output(command, **kwargs):
                commands.append(command)
                assert kwargs["timeout"] == 5
                return model + "\n"

            patch.setattr(benchmark.subprocess, "check_output", output)
            info = benchmark.environment_info()
            assert info["cpu"] == model and info["cpu_count"] == 16
            assert (
                info["platform"] == f"{system}-test" and info["machine"] == "test-arch"
            )
            assert (
                not commands
                if system == "Linux"
                else commands[0][0]
                == ("sysctl" if system == "Darwin" else "powershell.exe")
            )
            if system != "Linux":

                def denied(*args, **kwargs):
                    raise OSError("unavailable")

                patch.setattr(benchmark.subprocess, "check_output", denied)
                assert benchmark.environment_info()["cpu"] == "generic processor"
