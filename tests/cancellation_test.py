import asyncio
import gc
import weakref
from contextlib import asynccontextmanager

import pytest

import wreq


class Cancellation(asyncio.CancelledError):
    pass


@asynccontextmanager
async def local_server():
    connections = asyncio.Queue()
    writers = []

    async def accept(reader, writer):
        writers.append(writer)
        await reader.readuntil(b"\r\n\r\n")
        connections.put_nowait((reader, writer))

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    port = server.sockets[0].getsockname()[1]
    try:
        yield f"http://127.0.0.1:{port}/", connections
    finally:
        server.close()
        for writer in writers:
            writer.close()
        await asyncio.gather(*(writer.wait_closed() for writer in writers))
        await server.wait_closed()


@pytest.mark.asyncio
@pytest.mark.parametrize("operation", ["request", "request_error", "stream"])
async def test_cancellation_after_rust_completion(operation):
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        response = None
        if operation.startswith("request"):
            coroutine = client.get(url)
            waiter = coroutine.send(None)
            _, writer = await asyncio.wait_for(connections.get(), 5)
        else:
            task = asyncio.create_task(client.get(url))
            _, writer = await asyncio.wait_for(connections.get(), 5)
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n")
            await writer.drain()
            response = await asyncio.wait_for(task, 5)
            coroutine = anext(response.stream())
            waiter = coroutine.send(None)

        try:
            # Complete the Rust work without resuming its Python coroutine.
            assert isinstance(waiter, asyncio.Future)
            if operation == "request_error":
                writer.write(b"invalid HTTP response\r\n\r\n")
                writer.close()
            elif operation == "request":
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            else:
                writer.write(b"{}")
            done, _ = await asyncio.wait({waiter}, timeout=5)
            assert waiter in done, "Rust operation did not finish"

            error = Cancellation("cancelled after Rust completion")
            error_ref = weakref.ref(error)
            with pytest.raises(asyncio.CancelledError) as caught:
                coroutine.throw(error)
            assert caught.value is error
            # Keeping the finished coroutine alive must not retain its exception.
            del caught, error
            gc.collect()
            assert error_ref() is None
        finally:
            coroutine.close()
            if response is not None:
                await response.close()


@pytest.mark.asyncio
@pytest.mark.parametrize("action", ["cancel", "close_coroutine"])
async def test_pending_request_cancellation(action):
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        coroutine = client.get(url)
        if action == "close_coroutine":
            coroutine.send(None)
        else:
            task = asyncio.create_task(coroutine)
        reader, _ = await asyncio.wait_for(connections.get(), 5)

        if action == "close_coroutine":
            coroutine.close()
        else:
            task.cancel("caller cancellation message")
            done, _ = await asyncio.wait({task}, timeout=5)
            assert task in done, "Cancellation did not finish"
            with pytest.raises(asyncio.CancelledError) as caught:
                await task
            assert caught.value.args == ("caller cancellation message",)

        # The cancelled operation must release its pending network request.
        assert await asyncio.wait_for(reader.read(), 5) == b""


@pytest.mark.asyncio
@pytest.mark.parametrize("action", ["cancel", "close_coroutine"])
async def test_pending_stream_cancellation(action):
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.get(url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n")
        await writer.drain()
        response = await asyncio.wait_for(task, 5)
        stream = response.stream()
        coroutine = anext(stream)

        if action == "close_coroutine":
            assert isinstance(coroutine.send(None), asyncio.Future)
            coroutine.close()
        else:
            started = asyncio.Event()

            async def read():
                started.set()
                return await coroutine

            task = asyncio.create_task(read())
            await started.wait()
            task.cancel("cancel stream read")
            with pytest.raises(asyncio.CancelledError, match="cancel stream read"):
                await asyncio.wait_for(task, 5)

        # Closing the stream must acquire the lock held by the pending read.
        # Do not send a body: that would let a leaked read release it naturally.
        await asyncio.wait_for(stream.__aexit__(None, None, None), 5)
        with pytest.raises(StopAsyncIteration):
            await anext(stream)
        await response.close()


@pytest.mark.asyncio
async def test_stream_coroutine_iteration():
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.get(url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n"
            b"Trailer: x-check\r\n\r\n1\r\na\r\n"
        )
        await writer.drain()
        response = await asyncio.wait_for(task, 5)
        async with response.stream() as stream:
            deferred = stream.__anext__()
            assert asyncio.iscoroutine(deferred)
            assert not isinstance(deferred, asyncio.Future)
            assert deferred.__qualname__ == "Streamer.__anext__"
            assert not hasattr(stream, "_anext")
            try:
                # An unawaited __anext__ must not consume the first frame.
                assert await asyncio.wait_for(anext(stream), 5) == b"a"
                writer.write(b"1\r\nb\r\n0\r\nx-check: done\r\n\r\n")
                await writer.drain()
                assert await asyncio.wait_for(deferred, 5) == b"b"
                with pytest.raises(
                    RuntimeError, match="cannot reuse already awaited coroutine"
                ):
                    await deferred
            finally:
                deferred.close()

            async with asyncio.timeout(5):
                frames = [frame async for frame in stream]
            assert len(frames) == 1
            assert isinstance(frames[0], wreq.HeaderMap)
            assert frames[0]["x-check"] == b"done"
            with pytest.raises(StopAsyncIteration):
                await stream.__anext__()
            assert await anext(stream, None) is None
        await response.close()
