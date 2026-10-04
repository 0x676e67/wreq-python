"""pyperf-based HTTP client benchmark suite.

Each benchmark measures *concurrent* HTTP throughput: the reported value is
wall-clock time per request (after normalising by --http-requests), so lower
is better.  req/s ≈ 1 / mean_seconds.

Usage::

    python benchmark.py                          # 3 processes, 3 values, 1 warmup
    python benchmark.py --fast                   # quick estimate (~1 process)
    python benchmark.py --rigorous               # more accurate (6 processes)
    python benchmark.py -o results.json          # save JSON for later comparison
    python benchmark.py --http-requests=200 --http-workers=16
    python benchmark.py -v                       # verbose: show each value as it lands
"""

from __future__ import annotations

import asyncio
import inspect
import sys
import threading
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from importlib.metadata import PackageNotFoundError, version
from io import BytesIO
from typing import Any, Awaitable, Callable

import pyperf

# Import all HTTP clients
import pycurl
import aiohttp
import httpx
import niquests
import requests
import curl_cffi
import curl_cffi.requests
import wreq
import wreq.blocking
import ry

try:
    import uvloop  # type: ignore
except ImportError:
    uvloop = None
else:
    uvloop.install()

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

PAYLOAD_SIZES = ("20k", "50k", "200k")


# ---------------------------------------------------------------------------
# Dataclasses
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class BenchmarkCase:
    id: str
    runner: Callable[..., None]


@dataclass(frozen=True)
class AsyncBenchmarkCase:
    id: str
    runner: Callable[[str, int], Awaitable[None]]


class PycurlSession:
    def __init__(self):
        self.c = pycurl.Curl()
        self.content = None

    def close(self):
        self.c.close()

    def __del__(self):
        self.close()

    def get(self, url):
        buffer = BytesIO()
        self.c.setopt(pycurl.URL, url)
        self.c.setopt(pycurl.WRITEDATA, buffer)
        self.c.perform()
        self.content = buffer.getvalue()
        return self

    @property
    def text(self) -> bytes | None:
        return self.content


# ---------------------------------------------------------------------------
# Utility helpers
# ---------------------------------------------------------------------------


def add_package_version(packages: list[tuple[str, Any]]) -> list[tuple[str, Any]]:
    results = []
    for name, target in packages:
        try:
            label = f"{name} {version(name)}"
        except PackageNotFoundError:
            label = name
        results.append((label, target))
    return results


def maybe_close(resource: Any) -> None:
    close = getattr(resource, "close", None)
    if callable(close):
        close()


async def maybe_aclose(resource: Any) -> None:
    close = getattr(resource, "close", None)
    if not callable(close):
        return
    result = close()
    if inspect.isawaitable(result):
        await result


# ---------------------------------------------------------------------------
# Concurrency helpers
# ---------------------------------------------------------------------------


def _run_concurrent_requests(
    fetch_fn: Callable[[], None], count: int, workers: int
) -> None:
    """Run *fetch_fn* concurrently *count* times using at most *workers* threads."""
    with ThreadPoolExecutor(max_workers=min(workers, count)) as executor:
        futures = [executor.submit(fetch_fn) for _ in range(count)]
        for future in as_completed(futures):
            future.result()


def run_parallel_non_session_case(
    runner_fn: Callable[[str], None], url: str, count: int, workers: int
) -> None:
    """Parallel non-session: each worker creates its own client."""
    _run_concurrent_requests(lambda: runner_fn(url), count, workers)


# ---------------------------------------------------------------------------
# Sync session benchmarks  (shared client, concurrent requests)
# ---------------------------------------------------------------------------


def requests_session_test(url: str, count: int, workers: int) -> None:
    with requests.Session() as s:
        _run_concurrent_requests(lambda: s.get(url).content, count, workers)
        s.close()


def httpx_session_test(url: str, count: int, workers: int) -> None:
    with httpx.Client() as s:
        _run_concurrent_requests(lambda: s.get(url).content, count, workers)
        s.close()


def niquests_session_test(url: str, count: int, workers: int) -> None:
    with niquests.Session() as s:
        _run_concurrent_requests(lambda: s.get(url).content, count, workers)
        s.close()


def curl_cffi_session_test(url: str, count: int, workers: int) -> None:
    with curl_cffi.requests.Session() as s:
        _run_concurrent_requests(lambda: s.get(url).content, count, workers)
        s.close()


def wreq_blocking_session_test(url: str, count: int, workers: int) -> None:
    with wreq.blocking.Client() as s:
        _run_concurrent_requests(lambda: s.get(url).bytes(), count, workers)
        s.close()


def pycurl_session_test(url: str, count: int, workers: int) -> None:
    # pycurl Curl handles are not thread-safe; use thread-local storage so each
    # worker thread gets its own handle that persists across its assigned requests.
    _local = threading.local()

    def _fetch() -> None:
        if not hasattr(_local, "curl"):
            _local.curl = pycurl.Curl()
        buf = BytesIO()
        _local.curl.setopt(pycurl.URL, url)
        _local.curl.setopt(pycurl.WRITEDATA, buf)
        _local.curl.perform()

    _run_concurrent_requests(_fetch, count, workers)


def ry_blocking_session_test(url: str, count: int, workers: int) -> None:
    s = ry.BlockingClient()
    try:
        _run_concurrent_requests(lambda: s.get(url).bytes(), count, workers)
    finally:
        maybe_close(s)


# ---------------------------------------------------------------------------
# Sync non-session benchmarks  (one request per call; parallelism is external)
# ---------------------------------------------------------------------------


def requests_non_session_test(url: str) -> None:
    with requests.get(url) as resp:
        resp.content
        resp.close()


def httpx_non_session_test(url: str) -> None:
    resp = httpx.get(url)
    _ = resp.content
    resp.close()


def niquests_non_session_test(url: str) -> None:
    with niquests.get(url) as resp:
        _ = resp.content
        resp.close()


def curl_cffi_non_session_test(url: str) -> None:
    resp = curl_cffi.requests.get(url)
    _ = resp.content
    resp.close()


def wreq_blocking_non_session_test(url: str) -> None:
    with wreq.blocking.get(url) as resp:
        resp.bytes()
        resp.close()


def pycurl_non_session_test(url: str) -> None:
    s = PycurlSession()
    try:
        s.get(url).content
    finally:
        maybe_close(s)


def ry_blocking_non_session_test(url: str) -> None:
    s = ry.BlockingClient()
    try:
        s.get(url).bytes()
    finally:
        maybe_close(s)


# ---------------------------------------------------------------------------
# Async session benchmarks  (shared client, asyncio.gather)
# ---------------------------------------------------------------------------


async def httpx_async_session_test(url: str, count: int) -> None:
    async with httpx.AsyncClient() as s:
        await asyncio.gather(*[s.get(url) for _ in range(count)])


async def aiohttp_async_session_test(url: str, count: int) -> None:
    async with aiohttp.ClientSession() as s:

        async def _fetch() -> None:
            async with s.get(url) as resp:
                await resp.read()

        await asyncio.gather(*[_fetch() for _ in range(count)])


async def niquests_async_session_test(url: str, count: int) -> None:
    s = niquests.AsyncSession()
    try:

        async def _fetch() -> None:
            resp = await s.get(url)
            _ = resp.content

        await asyncio.gather(*[_fetch() for _ in range(count)])
    finally:
        await maybe_aclose(s)


async def wreq_async_session_test(url: str, count: int) -> None:
    s = wreq.Client()
    try:

        async def _fetch() -> None:
            resp = await s.get(url)
            await resp.bytes()

        await asyncio.gather(*[_fetch() for _ in range(count)])
    finally:
        await maybe_aclose(s)


async def curl_cffi_async_session_test(url: str, count: int) -> None:
    s = curl_cffi.requests.AsyncSession()
    try:

        async def _fetch() -> None:
            resp = await s.get(url)
            _ = resp.text

        await asyncio.gather(*[_fetch() for _ in range(count)])
    finally:
        await s.close()


async def ry_async_session_test(url: str, count: int) -> None:
    s = ry.HttpClient()
    try:

        async def _fetch():
            resp = await s.get(url)
            return await resp.bytes()

        await asyncio.gather(*[_fetch() for _ in range(count)])
    finally:
        await maybe_aclose(s)


# ---------------------------------------------------------------------------
# Async non-session benchmarks  (one request per call; parallelism is external)
# ---------------------------------------------------------------------------


async def httpx_async_non_session_test(url: str, count: int) -> None:
    async def _fetch() -> None:
        async with httpx.AsyncClient() as s:
            await s.get(url)

    await asyncio.gather(*[_fetch() for _ in range(count)])


async def aiohttp_async_non_session_test(url: str, count: int) -> None:
    async def _fetch() -> None:
        async with aiohttp.ClientSession() as s:
            async with s.get(url) as resp:
                await resp.read()

    await asyncio.gather(*[_fetch() for _ in range(count)])


async def niquests_async_non_session_test(url: str, count: int) -> None:
    async def _fetch() -> None:
        async with niquests.AsyncSession() as s:
            resp = await s.get(url)
            _ = resp.content

    await asyncio.gather(*[_fetch() for _ in range(count)])


async def wreq_async_non_session_test(url: str, count: int) -> None:
    async def _fetch() -> None:
        async with wreq.Client() as s:
            resp = await s.get(url)
            await resp.bytes()

    await asyncio.gather(*[_fetch() for _ in range(count)])


async def curl_cffi_async_non_session_test(url: str, count: int) -> None:
    async def _fetch() -> None:
        async with curl_cffi.requests.AsyncSession() as s:
            resp = await s.get(url)
            _ = resp.text

    await asyncio.gather(*[_fetch() for _ in range(count)])


async def ry_async_non_session_test(url: str, count: int) -> None:
    async def _fetch() -> None:
        s = ry.HttpClient()
        try:
            resp = await s.get(url)
            await resp.bytes()
        finally:
            await maybe_aclose(s)

    await asyncio.gather(*[_fetch() for _ in range(count)])


# ---------------------------------------------------------------------------
# Benchmark case builders
# ---------------------------------------------------------------------------


def build_sync_session_cases() -> list[BenchmarkCase]:
    cases = []
    if httpx is not None:
        cases.append(("httpx", httpx_session_test))
    if requests is not None:
        cases.append(("requests", requests_session_test))
    if niquests is not None:
        cases.append(("niquests", niquests_session_test))
    if curl_cffi.requests is not None:
        cases.append(("curl_cffi", curl_cffi_session_test))
    if pycurl is not None:
        cases.append(("pycurl", pycurl_session_test))
    if ry is not None:
        cases.append(("ry", ry_blocking_session_test))
    if wreq.blocking is not None:
        cases.append(("wreq", wreq_blocking_session_test))
    return [BenchmarkCase(name, runner) for name, runner in add_package_version(cases)]


def build_sync_non_session_cases() -> list[BenchmarkCase]:
    cases = []
    if httpx is not None:
        cases.append(("httpx", httpx_non_session_test))
    if requests is not None:
        cases.append(("requests", requests_non_session_test))
    if niquests is not None:
        cases.append(("niquests", niquests_non_session_test))
    if curl_cffi.requests is not None:
        cases.append(("curl_cffi", curl_cffi_non_session_test))
    if pycurl is not None:
        cases.append(("pycurl", pycurl_non_session_test))
    if ry is not None:
        cases.append(("ry", ry_blocking_non_session_test))
    if wreq.blocking is not None:
        cases.append(("wreq", wreq_blocking_non_session_test))
    return [BenchmarkCase(name, runner) for name, runner in add_package_version(cases)]


def build_async_session_cases() -> list[AsyncBenchmarkCase]:
    cases = []
    if httpx is not None:
        cases.append(("httpx", httpx_async_session_test))
    if niquests is not None:
        cases.append(("niquests", niquests_async_session_test))
    if curl_cffi.requests is not None:
        cases.append(("curl_cffi", curl_cffi_async_session_test))
    if aiohttp is not None:
        cases.append(("aiohttp", aiohttp_async_session_test))
    if ry is not None:
        cases.append(("ry", ry_async_session_test))
    if wreq is not None:
        cases.append(("wreq", wreq_async_session_test))
    return [
        AsyncBenchmarkCase(name, runner) for name, runner in add_package_version(cases)
    ]


def build_async_non_session_cases() -> list[AsyncBenchmarkCase]:
    cases = []
    if httpx is not None:
        cases.append(("httpx", httpx_async_non_session_test))
    if niquests is not None:
        cases.append(("niquests", niquests_async_non_session_test))
    if curl_cffi.requests is not None:
        cases.append(("curl_cffi", curl_cffi_async_non_session_test))
    if aiohttp is not None:
        cases.append(("aiohttp", aiohttp_async_non_session_test))
    if ry is not None:
        cases.append(("ry", ry_async_non_session_test))
    if wreq is not None:
        cases.append(("wreq", wreq_async_non_session_test))
    return [
        AsyncBenchmarkCase(name, runner) for name, runner in add_package_version(cases)
    ]


SYNC_SESSION_CASES = build_sync_session_cases()
SYNC_NON_SESSION_CASES = build_sync_non_session_cases()
ASYNC_SESSION_CASES = build_async_session_cases()
ASYNC_NON_SESSION_CASES = build_async_non_session_cases()


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> int:
    # Quick server probe before spawning any subprocesses.
    if "--worker" not in sys.argv:
        base_url_probe = "http://127.0.0.1:8000"
        for arg in sys.argv[1:]:
            if arg.startswith("--http-base-url="):
                base_url_probe = arg.split("=", 1)[1]
                break
        probe_url = f"{base_url_probe.rstrip('/')}/20k"
        try:
            with urllib.request.urlopen(probe_url, timeout=2) as resp:
                resp.read(1)
        except (OSError, urllib.error.URLError) as exc:
            print(
                f"ERROR: benchmark server unavailable at {probe_url}: {exc}",
                file=sys.stderr,
            )
            print("Start bench/server.py before running benchmarks.", file=sys.stderr)
            return 1

    # 3 processes × 3 values × 1 warmup = 9 measurements per benchmark.
    # Pass --fast for a quick estimate or --rigorous for higher confidence.
    runner = pyperf.Runner(processes=3, values=3, warmups=1)
    runner.argparser.add_argument(
        "--http-base-url",
        default="http://127.0.0.1:8000",
        metavar="URL",
        help="Base URL for the benchmark server. (default: %(default)s)",
    )
    runner.argparser.add_argument(
        "--http-requests",
        type=int,
        default=100,
        metavar="N",
        help="Concurrent HTTP requests per benchmark call. (default: %(default)s)",
    )
    runner.argparser.add_argument(
        "--http-workers",
        type=int,
        default=32,
        metavar="N",
        help="Thread pool size for sync benchmarks. (default: %(default)s)",
    )

    args = runner.parse_args()
    base_url: str = args.http_base_url.rstrip("/")
    http_requests: int = args.http_requests
    workers: int = args.http_workers

    # Register all benchmarks.  pyperf assigns each a task ID in order;
    # worker subprocesses run only the specific task they are assigned.
    # inner_loops=http_requests normalises the reported time to per-request,
    # so the reported mean is time/request and req/s = http_requests / mean.

    for size in PAYLOAD_SIZES:
        for case in SYNC_NON_SESSION_CASES:
            runner.bench_func(
                f"sync-non-session/{size}/{case.id}",
                run_parallel_non_session_case,
                case.runner,
                f"{base_url}/{size}",
                http_requests,
                workers,
                inner_loops=http_requests,
            )

    for size in PAYLOAD_SIZES:
        for case in SYNC_SESSION_CASES:
            runner.bench_func(
                f"sync-session/{size}/{case.id}",
                case.runner,
                f"{base_url}/{size}",
                http_requests,
                workers,
                inner_loops=http_requests,
            )

    for size in PAYLOAD_SIZES:
        for case in ASYNC_NON_SESSION_CASES:
            runner.bench_async_func(
                f"async-non-session/{size}/{case.id}",
                case.runner,
                f"{base_url}/{size}",
                http_requests,
                inner_loops=http_requests,
            )

    for size in PAYLOAD_SIZES:
        for case in ASYNC_SESSION_CASES:
            runner.bench_async_func(
                f"async-session/{size}/{case.id}",
                case.runner,
                f"{base_url}/{size}",
                http_requests,
                inner_loops=http_requests,
            )

    return 0


if __name__ == "__main__":
    sys.exit(main())
