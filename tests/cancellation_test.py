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
@pytest.mark.parametrize("operation", ["request", "request_error", "bytes", "text", "json"])
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
            coroutine = getattr(response, operation)()
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
@pytest.mark.parametrize("action", ["cancel", "close_coroutine", "close_client"])
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
            if action == "cancel":
                task.cancel("caller cancellation message")
            else:
                client.close()
            done, _ = await asyncio.wait({task}, timeout=5)
            assert task in done, "Cancellation did not finish"
            with pytest.raises(asyncio.CancelledError) as caught:
                await task
            expected = (
                "caller cancellation message"
                if action == "cancel"
                else "Operation was cancelled: client has been closed"
            )
            assert caught.value.args == (expected,)

        # The cancelled operation must release its pending network request.
        assert await asyncio.wait_for(reader.read(), 5) == b""
