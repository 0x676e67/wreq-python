import asyncio
import base64
import hashlib
from datetime import timedelta

import pytest
import wreq
from wreq.runtime import Runtime

from cancellation_test import local_server
from upload_test import read_chunked


def test_runtime_configuration():
    assert Runtime is wreq.Runtime
    assert Runtime is wreq.runtime.Runtime
    assert "Runtime" in wreq.__all__
    for kwargs in (
        {"workers": 0},
        {"max_blocking_threads": 0},
        {"thread_name": "bad\0name"},
        {"thread_keep_alive": timedelta(microseconds=-1)},
    ):
        with pytest.raises(ValueError):
            Runtime(**kwargs)
    for invalid in (0.25, "1s"):
        with pytest.raises(TypeError):
            Runtime(thread_keep_alive=invalid)
    with pytest.raises(TypeError):
        wreq.Client(runtime=object())
    for duration in (None, timedelta(), timedelta(microseconds=250001)):
        runtime = Runtime(
            workers=1,
            work_steal=False,
            thread_name=None if duration is None else "isolated",
            max_blocking_threads=3,
            thread_keep_alive=duration,
        )
        assert not hasattr(runtime, "default")
        assert not hasattr(runtime, "closed")
        assert not hasattr(runtime, "shutdown_timeout")
        for factory in (wreq.Client, wreq.blocking.Client):
            client = factory(runtime=runtime)
            alias = client.runtime
            with pytest.raises(AttributeError):
                client.runtime = runtime
            client.close()
            del client
            # Releasing a client does not close a shared runtime.
            other = factory(runtime=alias)
            other.close()
    for factory in (wreq.Client, wreq.blocking.Client):
        for kwargs in ({}, {"runtime": None}):
            client = factory(**kwargs)
            assert isinstance(client.runtime, Runtime)
            client.close()


@pytest.mark.asyncio
@pytest.mark.parametrize("steal", [False, True])
async def test_response_and_stream_keep_runtime_alive(steal):
    runtime = wreq.Runtime(workers=1, work_steal=steal)
    async with local_server() as (url, connections):
        client = wreq.Client(runtime=runtime, proxies=[])
        task = asyncio.create_task(client.get(url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n")
        await writer.drain()
        response = await asyncio.wait_for(task, 5)
        del task
        client.close()
        del client, runtime
        stream = response.stream()
        del response
        writer.write(b"body")
        await writer.drain()
        assert await asyncio.wait_for(anext(stream), 5) == b"body"
        with pytest.raises(StopAsyncIteration):
            await anext(stream)
        del stream


@pytest.mark.asyncio
@pytest.mark.parametrize("steal", [False, True])
async def test_shared_runtime_cancellation_and_upload(steal):
    runtime = wreq.Runtime(workers=2, work_steal=steal, max_blocking_threads=2)
    async with local_server() as (url, connections):
        first = wreq.Client(runtime=runtime, proxies=[])
        second = wreq.Client(runtime=runtime, proxies=[])
        del runtime
        pending = asyncio.create_task(first.get(url))
        await asyncio.wait_for(connections.get(), 5)
        first.close()
        with pytest.raises(asyncio.CancelledError):
            await asyncio.wait_for(pending, 5)
        del pending, first
        finalized = asyncio.Event()

        async def chunks():
            try:
                yield b"custom "
                await asyncio.sleep(0)
                yield b"runtime"
            finally:
                finalized.set()

        task = asyncio.create_task(second.post(url, body=chunks()))
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        assert await asyncio.wait_for(read_chunked(reader), 5) == b"custom runtime"
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
        await writer.drain()
        response = await asyncio.wait_for(task, 5)
        assert await response.json() == {}
        await asyncio.wait_for(finalized.wait(), 5)
        await response.close()
        second.close()
        del response, task, second


@pytest.mark.asyncio
@pytest.mark.parametrize("steal", [False, True])
async def test_blocking_client_uses_custom_runtime(steal):
    runtime = wreq.Runtime(workers=1, work_steal=steal, max_blocking_threads=2)

    def request(url):
        with wreq.blocking.Client(runtime=runtime, proxies=[]) as client:
            assert isinstance(client.runtime, Runtime)
            with client.post(url, body=iter((b"blocking",))) as response:
                with response.stream() as stream:
                    return b"".join(stream)

    async with local_server() as (url, connections):
        task = asyncio.create_task(asyncio.to_thread(request, url))
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        assert await asyncio.wait_for(read_chunked(reader), 5) == b"blocking"
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
        await writer.drain()
        assert await asyncio.wait_for(task, 5) == b"ok"
        del task


@pytest.mark.asyncio
@pytest.mark.parametrize("blocking", [False, True])
@pytest.mark.parametrize("operation", ["get", "websocket"])
async def test_client_close_cancels_requests(blocking, operation):
    factory = wreq.blocking.Client if blocking else wreq.Client
    client = factory(
        runtime=Runtime(workers=1, work_steal=False),
        proxies=[],
        timeout=timedelta(seconds=10),
    )
    task = None
    try:
        async with local_server() as (url, connections):
            if operation == "websocket":
                url = url.replace("http://", "ws://", 1)
            request = getattr(client, operation)

            def start():
                return asyncio.create_task(
                    asyncio.to_thread(request, url) if blocking else request(url)
                )

            task = start()
            reader, _ = await asyncio.wait_for(connections.get(), 5)
            client.close()
            done, _ = await asyncio.wait({task}, timeout=5)
            assert task in done, "close() did not cancel the pending request"
            with pytest.raises(asyncio.CancelledError):
                await task
            assert await asyncio.wait_for(reader.read(), 5) == b""

            task = start()
            done, _ = await asyncio.wait({task}, timeout=5)
            assert task in done, "a closed client started another request"
            with pytest.raises(asyncio.CancelledError):
                await task
            assert connections.empty()
    finally:
        client.close()
        if task is not None:
            await asyncio.gather(task, return_exceptions=True)


@pytest.mark.asyncio
@pytest.mark.parametrize("blocking", [False, True])
async def test_websocket_outlives_client(blocking):
    connections = asyncio.Queue()

    async def accept(reader, writer):
        header = await reader.readuntil(b"\r\n\r\n")
        key = next(
            line.split(b":", 1)[1].strip()
            for line in header.split(b"\r\n")
            if line.lower().startswith(b"sec-websocket-key:")
        )
        digest = base64.b64encode(
            hashlib.sha1(key + b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11").digest()
        )
        writer.write(
            b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
            b"Connection: Upgrade\r\nSec-WebSocket-Accept: " + digest + b"\r\n\r\n"
        )
        await writer.drain()
        connections.put_nowait((reader, writer))

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    runtime = wreq.Runtime(workers=1, work_steal=False)
    writer = None
    try:
        url = f"ws://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
        client = (wreq.blocking.Client if blocking else wreq.Client)(
            runtime=runtime, proxies=[]
        )
        ws = (
            await asyncio.to_thread(client.websocket, url)
            if blocking
            else await client.websocket(url)
        )
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        client.close()
        del client, runtime
        writer.write(b"\x81\x04pong")
        await writer.drain()
        message = await asyncio.to_thread(ws.recv) if blocking else await ws.recv()
        assert message.text == "pong"
        outgoing = wreq.Message.from_text("ping")
        if blocking:
            await asyncio.to_thread(ws.send, outgoing)
        else:
            await ws.send(outgoing)
        frame = await asyncio.wait_for(reader.readexactly(10), 5)
        assert frame[:2] == b"\x81\x84"
        assert (
            bytes(byte ^ frame[2 + i % 4] for i, byte in enumerate(frame[6:]))
            == b"ping"
        )
        if blocking:
            await asyncio.to_thread(ws.close)
        else:
            await ws.close()
        del ws
    finally:
        if writer is not None:
            writer.close()
            await writer.wait_closed()
        server.close()
        await server.wait_closed()


@pytest.mark.asyncio
@pytest.mark.parametrize("steal", [False, True])
async def test_http2_multiplexing_on_custom_runtime(steal):
    # Minimal h2c responder: indexed :status=200, then a two-byte DATA frame.
    # No HPACK decoder is needed because the test does not inspect request headers.
    def frame(kind, flags, stream, payload=b""):
        return (
            len(payload).to_bytes(3, "big")
            + bytes((kind, flags))
            + stream.to_bytes(4, "big")
            + payload
        )

    connections = []
    handlers = set()
    errors = []

    async def accept(reader, writer):
        handlers.add(asyncio.current_task())
        connections.append(writer)
        try:
            assert await reader.readexactly(24) == b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"
            writer.write(frame(4, 0, 0))
            await writer.drain()
            while True:
                header = await reader.readexactly(9)
                payload = await reader.readexactly(int.from_bytes(header[:3], "big"))
                kind, flags = header[3:5]
                stream = int.from_bytes(header[5:], "big") & 0x7FFFFFFF
                if kind == 4 and not flags & 1:
                    writer.write(frame(4, 1, 0))
                elif kind == 6 and not flags & 1:
                    writer.write(frame(6, 1, 0, payload))
                elif kind == 1:
                    assert flags & 4  # These small requests fit in one header block.
                    writer.write(
                        frame(1, 4, stream, b"\x88") + frame(0, 1, stream, b"ok")
                    )
                await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        except Exception as error:
            errors.append(error)
        finally:
            handlers.discard(asyncio.current_task())

    runtime = wreq.Runtime(workers=2, work_steal=steal)
    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    try:
        url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
        client = wreq.Client(runtime=runtime, http2_only=True, proxies=[])

        async def request():
            response = await client.get(url)
            assert response.version == wreq.Version.HTTP_2
            # Consume without close(), which intentionally forbids connection reuse.
            return await response.bytes()

        assert await asyncio.wait_for(request(), 5) == b"ok"
        assert (
            await asyncio.wait_for(asyncio.gather(*(request() for _ in range(16))), 5)
            == [b"ok"] * 16
        )
        assert len(connections) == 1
        assert not errors
        client.close()
        del client, runtime
    finally:
        server.close()
        for writer in connections:
            writer.close()
        await asyncio.gather(*(writer.wait_closed() for writer in connections))
        await asyncio.gather(*handlers)
        await server.wait_closed()
