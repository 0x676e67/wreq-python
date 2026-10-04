"""HTTPS echo throughput with Full/Stream uploads and streamed responses."""

from __future__ import annotations

import argparse
import asyncio
from concurrent.futures import ThreadPoolExecutor
from contextlib import ExitStack, asynccontextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import subprocess
import sys
import threading
import time
import traceback
from urllib.parse import urlsplit

if __package__:
    from .clients import (
        CAPABILITIES,
        CLIENTS,
        blocking_operations,
        metadata,
        operations,
        supports,
    )
    from .results import aggregate, validate_document, write_atomic
    from .workloads import (
        BODY_CASES,
        CONCURRENCY_CASES,
        STREAM_CHUNK_BYTES,
        prepare_chunks,
        upload_chunk_bytes,
    )
else:
    from clients import (
        CAPABILITIES,
        CLIENTS,
        blocking_operations,
        metadata,
        operations,
        supports,
    )
    from results import aggregate, validate_document, write_atomic
    from workloads import (
        BODY_CASES,
        CONCURRENCY_CASES,
        STREAM_CHUNK_BYTES,
        prepare_chunks,
        upload_chunk_bytes,
    )


def csv_choices(value, choices):
    values = value.split(",")
    if (
        not values
        or len(values) != len(set(values))
        or any(v not in choices for v in values)
    ):
        raise argparse.ArgumentTypeError(
            f"Use unique comma-separated values from {', '.join(choices)}"
        )
    return values


def csv_positive(value):
    try:
        values = [int(v) for v in value.split(",")]
    except ValueError as exc:
        raise argparse.ArgumentTypeError(
            "Use comma-separated positive integers"
        ) from exc
    if not values or min(values) < 1 or len(values) != len(set(values)):
        raise argparse.ArgumentTypeError("Use unique positive integers")
    return values


def positive(value):
    value = int(value)
    if value < 1:
        raise argparse.ArgumentTypeError("Must be positive")
    return value


def nonnegative(value):
    value = int(value)
    if value < 0:
        raise argparse.ArgumentTypeError("Must not be negative")
    return value


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--server", type=Path, help="Built bench/server TLS echo executable"
    )
    parser.add_argument(
        "--clients", type=lambda v: csv_choices(v, CLIENTS), default=list(CLIENTS)
    )
    parser.add_argument(
        "--protocols", type=lambda v: csv_choices(v, ("h1", "h2")), default=["h1", "h2"]
    )
    parser.add_argument(
        "--sizes",
        type=csv_positive,
        default=list(BODY_CASES),
        metavar="BYTES,...",
    )
    parser.add_argument("--concurrency", type=csv_positive, default=CONCURRENCY_CASES)
    parser.add_argument(
        "--body-kinds",
        type=lambda v: csv_choices(v, ("full", "stream")),
        default=["full", "stream"],
    )
    parser.add_argument("--rounds", type=positive, default=3)
    parser.add_argument("--warmup", type=nonnegative, default=1)
    parser.add_argument("--samples", type=positive, default=1)
    parser.add_argument("--requests", type=positive, default=300)
    parser.add_argument("--server-workers", type=positive, default=4)
    parser.add_argument(
        "--case-timeout",
        type=positive,
        default=120,
        help="Seconds allowed for each client case",
    )
    parser.add_argument("--seed", type=int, default=160130)
    parser.add_argument(
        "--output", type=Path, help="New JSON snapshot path; must not exist"
    )
    parser.add_argument("--worker", choices=CLIENTS, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)
    if args.worker is None:
        if args.server is None:
            parser.error("--server is required")
        if args.requests < max(args.concurrency):
            parser.error("--requests must be at least the largest --concurrency")
        if not any(
            supports(client, protocol, kind)
            for client in args.clients
            for protocol in args.protocols
            for kind in args.body_kinds
        ):
            parser.error("Selected clients do not support the requested workload")
    return args


async def closed_loop(operation, requests, concurrency):
    """Keep a fixed number of workers; each starts its next request after EOF."""
    worker_count = min(concurrency, requests)
    per_worker, remainder = divmod(requests, worker_count)

    async def run(count):
        for _ in range(count):
            await operation()

    async with asyncio.TaskGroup() as group:
        for index in range(worker_count):
            group.create_task(run(per_worker + (index < remainder)))


async def run_case(client, request):
    if CAPABILITIES[client]["api"] == "blocking":
        return await asyncio.to_thread(run_blocking_case, client, request)
    body = b"x" * request["payload_bytes"]
    async with operations(
        client, request["protocol"], request["url"], body, request["body_kind"]
    ) as post:
        # The untimed request validates status, protocol, EOF and echoed length.
        validation = await post()

        async def batch():
            cpu_start = time.process_time()
            start = time.perf_counter()
            await closed_loop(post, request["requests"], request["concurrency"])
            return {
                "seconds": time.perf_counter() - start,
                "cpu_seconds": time.process_time() - cpu_start,
            }

        warmup = [await batch() for _ in range(request["warmup"])]
        samples = [await batch() for _ in range(request["samples"])]
    return {"warmup": warmup, "samples": samples, "validation": validation}


def run_blocking_case(client, request):
    """Reuse one client per logical worker, submitting whole closed-loop batches."""
    body = b"x" * request["payload_bytes"]
    chunks = prepare_chunks(body, request["body_kind"])
    workers = min(request["concurrency"], request["requests"])
    quotient, remainder = divmod(request["requests"], workers)
    with ExitStack() as stack:
        posts = [
            stack.enter_context(
                blocking_operations(
                    client,
                    request["protocol"],
                    request["url"],
                    body,
                    request["body_kind"],
                    chunks=chunks,
                )
            )
            for _ in range(workers)
        ]
        # Establish each worker's reusable connection before any timing.
        validation = posts[0]()
        if any(post() != validation for post in posts[1:]):
            raise RuntimeError("Blocking workers returned inconsistent validations")
        with ThreadPoolExecutor(max_workers=workers) as executor:

            def batch():
                stopped = threading.Event()

                def run(post, count):
                    try:
                        for _ in range(count):
                            if stopped.is_set():
                                break
                            post()
                    except BaseException:
                        stopped.set()
                        raise

                cpu_start = time.process_time()
                start = time.perf_counter()
                futures = [
                    executor.submit(run, post, quotient + (i < remainder))
                    for i, post in enumerate(posts)
                ]
                try:
                    for future in futures:
                        future.result()
                finally:
                    stopped.set()
                    # No client is closed while another thread is still using it.
                    for future in futures:
                        future.cancel()
                    for future in futures:
                        if not future.cancelled():
                            try:
                                future.result()
                            except BaseException:
                                pass
                return {
                    "seconds": time.perf_counter() - start,
                    "cpu_seconds": time.process_time() - cpu_start,
                }

            warmup = [batch() for _ in range(request["warmup"])]
            samples = [batch() for _ in range(request["samples"])]
    return {"warmup": warmup, "samples": samples, "validation": validation}


async def worker(client):
    # This happens before importing libraries, including those without no_proxy().
    for name in list(os.environ):
        if name.lower() in {"http_proxy", "https_proxy", "all_proxy", "no_proxy"}:
            del os.environ[name]
    os.environ["NO_PROXY"] = "*"
    os.environ["no_proxy"] = "*"
    try:
        print(json.dumps({"ok": True, "metadata": metadata(client)}), flush=True)
        while line := await asyncio.to_thread(sys.stdin.readline):
            request = json.loads(line)
            if request.get("stop"):
                return
            async with asyncio.timeout(request["case_timeout"]):
                result = await run_case(client, request)
            print(json.dumps({"ok": True, "result": result}), flush=True)
    except Exception:
        print(json.dumps({"ok": False, "error": traceback.format_exc()}), flush=True)
        raise SystemExit(1)


async def stop_process(process, command=None):
    if process.returncode is not None:
        return
    try:
        if command is not None:
            process.stdin.write(command)
            await process.stdin.drain()
        process.stdin.close()
        async with asyncio.timeout(10):
            await process.wait()
        return
    except (TimeoutError, OSError, ConnectionError):
        pass
    if process.returncode is None:
        try:
            process.terminate()
        except ProcessLookupError:
            return
        try:
            async with asyncio.timeout(5):
                await process.wait()
        except TimeoutError:
            if process.returncode is None:
                try:
                    process.kill()
                except ProcessLookupError:
                    pass
                await process.wait()


async def receive(process, timeout, label):
    async with asyncio.timeout(timeout):
        line = await process.stdout.readline()
    if not line:
        raise RuntimeError(
            f"{label} closed stdout before returning a result (exit {process.returncode})"
        )
    try:
        answer = json.loads(line)
    except (ValueError, UnicodeDecodeError) as exc:
        raise RuntimeError(f"Invalid JSON from {label}: {line!r}") from exc
    if not isinstance(answer, dict):
        raise RuntimeError(f"Invalid response from {label}: {answer!r}")
    return answer


@asynccontextmanager
async def benchmark_server(executable, protocol, workers):
    process = await asyncio.create_subprocess_exec(
        str(executable),
        "--protocol",
        protocol,
        "--workers",
        str(workers),
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
    )
    try:
        info = await receive(process, 30, "TLS server")
        url = urlsplit(info.get("url", ""))
        if (
            url.scheme != "https"
            or url.hostname not in {"127.0.0.1", "localhost", "::1"}
            or url.port is None
            or url.path not in {"", "/"}
            or url.query
            or url.fragment
            or url.username is not None
            or info.get("protocol") != protocol
            or info.get("workers") != workers
        ):
            raise RuntimeError(f"Unexpected TLS server configuration: {info!r}")
        yield info
    finally:
        await stop_process(process)


def source_info():
    root = Path(__file__).resolve().parent.parent

    def git(*args):
        return subprocess.check_output(
            ["git", "-C", str(root), *args], text=True
        ).strip()

    return {
        "repository": git("remote", "get-url", "origin"),
        "commit": git("rev-parse", "HEAD"),
        "dirty": bool(git("status", "--porcelain")),
    }


def environment_info():
    cpu = platform.processor() or platform.machine()
    system = platform.system()
    try:
        if system == "Linux":
            cpuinfo = Path("/proc/cpuinfo")
            if cpuinfo.exists():
                for line in cpuinfo.read_text().splitlines():
                    if line.startswith("model name"):
                        cpu = line.split(":", 1)[1].strip() or cpu
                        break
        elif system == "Darwin":
            cpu = (
                subprocess.check_output(
                    ["sysctl", "-n", "machdep.cpu.brand_string"], text=True, timeout=5
                ).strip()
                or cpu
            )
        elif system == "Windows":
            cpu = (
                subprocess.check_output(
                    [
                        "powershell.exe",
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "(Get-CimInstance -ClassName Win32_Processor | Select-Object -First 1).Name",
                    ],
                    text=True,
                    timeout=5,
                    creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
                ).strip()
                or cpu
            )
    except (OSError, subprocess.SubprocessError):
        # Restricted containers and hosts without these utilities keep the
        # platform-provided processor/architecture fallback.
        pass
    info = {
        "python": sys.version,
        "implementation": platform.python_implementation(),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "cpu": cpu,
        "cpu_count": os.cpu_count(),
        "event_loop": "asyncio",
    }
    if hasattr(os, "sched_getaffinity"):
        info["affinity"] = sorted(os.sched_getaffinity(0))
    if hasattr(os, "getloadavg"):
        info["load_average"] = list(os.getloadavg())
    return info


async def orchestrate(args):
    executable = args.server.resolve(strict=True)
    source = source_info()
    if args.output is None:
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        args.output = (
            Path(__file__).resolve().parent
            / "data"
            / f"{stamp}-{source['commit'][:12]}.json"
        )
    if args.output.exists():
        raise ValueError(f"Refusing to overwrite benchmark snapshot: {args.output}")
    configuration = {
        "clients": args.clients,
        "protocols": args.protocols,
        "payload_bytes": args.sizes,
        "concurrency": args.concurrency,
        "body_kinds": args.body_kinds,
        "rounds": args.rounds,
        "warmup": args.warmup,
        "samples": args.samples,
        "requests": args.requests,
        "stream_chunk_bytes": STREAM_CHUNK_BYTES,
        "stream_chunk_bytes_by_payload": {
            str(size): upload_chunk_bytes(size) for size in args.sizes
        },
        "server_workers": args.server_workers,
        "seed": args.seed,
        "case_timeout": args.case_timeout,
        "tls_version": "1.3",
        "tls_verification": False,
        "response_consumption": "streamed chunks to EOF; adapter read sizes recorded per client",
        "throughput_unit": "MB/s (decimal, response payload only)",
        "timing": "upload and complete streamed response consumption; preparation excluded",
        "concurrency_model": "evenly preallocated closed-loop workers",
        "client_scope": {
            "async": "one shared client per case",
            "blocking": "one client per logical worker; persistent thread pool per case",
        },
    }
    document = {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "source": source,
        "environment": environment_info(),
        "configuration": configuration,
        "clients": {},
        "server": {
            "kind": "rust-tls-echo",
            "executable": str(executable),
            "sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
            "workers": args.server_workers,
            "protocols": [],
        },
        "results": [],
    }
    rng = random.Random(args.seed)
    workers, rows = {}, []
    try:
        for client in args.clients:
            process = await asyncio.create_subprocess_exec(
                sys.executable,
                str(Path(__file__).resolve()),
                "--worker",
                client,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
            )
            workers[client] = process
            answer = await receive(process, 30, client)
            if not answer.get("ok"):
                raise RuntimeError(f"{client}: {answer.get('error')}")
            document["clients"][client] = answer["metadata"]
        for protocol in args.protocols:
            async with benchmark_server(
                executable, protocol, args.server_workers
            ) as server:
                document["server"]["protocols"].append(server)
                cases = [
                    (client, kind, size, concurrency)
                    for client in args.clients
                    for kind in args.body_kinds
                    for size in args.sizes
                    for concurrency in args.concurrency
                    if supports(client, protocol, kind)
                ]
                for round_id in range(1, args.rounds + 1):
                    order = list(cases)
                    rng.shuffle(order)
                    for client, kind, size, concurrency in order:
                        request = {
                            "url": server["url"].rstrip("/") + "/echo",
                            "protocol": protocol,
                            "body_kind": kind,
                            "payload_bytes": size,
                            "concurrency": concurrency,
                            "requests": args.requests,
                            "warmup": args.warmup,
                            "samples": args.samples,
                            "case_timeout": args.case_timeout,
                        }
                        process = workers[client]
                        process.stdin.write((json.dumps(request) + "\n").encode())
                        await process.stdin.drain()
                        answer = await receive(process, args.case_timeout + 15, client)
                        if not answer.get("ok"):
                            raise RuntimeError(f"{client}: {answer.get('error')}")
                        result = answer["result"]
                        rows.append(
                            {
                                "client": client,
                                "protocol": protocol,
                                "body_kind": kind,
                                "payload_bytes": size,
                                "concurrency": concurrency,
                                "round": round_id,
                                **result,
                            }
                        )
                        rps = (
                            args.requests
                            * len(result["samples"])
                            / sum(s["seconds"] for s in result["samples"])
                        )
                        print(
                            f"{protocol} {kind:6s} {client:14s} {size:8d} B c={concurrency:3d} "
                            f"round={round_id}/{args.rounds} {rps:9.1f} req/s",
                            flush=True,
                        )
    finally:
        for process in workers.values():
            await stop_process(process, b'{"stop":true}\n')
    document["results"] = aggregate(rows, args.requests)
    document["generated_at"] = datetime.now(timezone.utc).isoformat()
    validate_document(document)
    write_atomic(args.output, document, overwrite=False)
    print(f"Saved complete benchmark: {args.output}")


def main(argv=None):
    args = parse_args(argv)
    try:
        asyncio.run(worker(args.worker) if args.worker else orchestrate(args))
    except (Exception, KeyboardInterrupt) as exc:
        print(f"Benchmark failed; no complete result written: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
