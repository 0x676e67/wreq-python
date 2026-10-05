"""JSON-line worker protocol and owned subprocess lifetimes.

Workers import comparison libraries in their own processes; the coordinator
only sends cases and receives completed measurements.
"""

import asyncio
from contextlib import asynccontextmanager
from dataclasses import dataclass
import json
from pathlib import Path
import sys
from urllib.parse import urlsplit


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


@dataclass
class Worker:
    client: str
    process: asyncio.subprocess.Process
    metadata: dict

    async def measure(self, request):
        self.process.stdin.write((json.dumps(request) + "\n").encode())
        await self.process.stdin.drain()
        answer = await receive(self.process, request["case_timeout"] + 15, self.client)
        if not answer.get("ok"):
            raise RuntimeError(f"{self.client}: {answer.get('error')}")
        return answer["result"]


@asynccontextmanager
async def start_worker(script: Path, client: str):
    process = await asyncio.create_subprocess_exec(
        sys.executable,
        str(script),
        "--worker",
        client,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
    )
    try:
        answer = await receive(process, 30, client)
        if not answer.get("ok"):
            raise RuntimeError(f"{client}: {answer.get('error')}")
        yield Worker(client, process, answer["metadata"])
    finally:
        await stop_process(process, b'{"stop":true}\n')
