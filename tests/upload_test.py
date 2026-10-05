import asyncio
import contextvars
import gc
import threading

import pytest
import wreq

from cancellation_test import local_server


@pytest.fixture
def no_automatic_gc():
    enabled = gc.isenabled()
    gc.disable()
    try:
        yield
    finally:
        if enabled:
            gc.enable()


async def read_chunked(reader):
    body = bytearray()
    while True:
        size = int(await reader.readline(), 16)
        if not size:
            assert await reader.readline() == b"\r\n"
            return bytes(body)
        body.extend(await reader.readexactly(size))
        assert await reader.readexactly(2) == b"\r\n"


@pytest.mark.asyncio
@pytest.mark.parametrize("multipart", [False, True])
async def test_async_upload(multipart, no_automatic_gc):
    context = contextvars.ContextVar("upload_context", default="missing")
    context.set("caller")
    thread = threading.get_ident()
    closed = asyncio.Event()

    async def chunks():
        try:
            for item in (b"hello ", "world"):
                await asyncio.sleep(0)
                assert context.get() == "caller"
                assert threading.get_ident() == thread
                yield item
        finally:
            closed.set()

    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        kwds = (
            {"multipart": wreq.Multipart(wreq.Part(name="file", value=chunks()))}
            if multipart
            else {"body": chunks()}
        )
        task = asyncio.create_task(client.post(url, **kwds))
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        body = await asyncio.wait_for(read_chunked(reader), 5)
        assert (b"hello world" in body) if multipart else (body == b"hello world")
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
        await writer.drain()
        response = await asyncio.wait_for(task, 5)
        await response.close()
        assert closed.is_set()


@pytest.mark.asyncio
@pytest.mark.parametrize("failure", ["exception", "type", "cancelled"])
async def test_upload_errors(failure):
    closed = asyncio.Event()

    async def chunks():
        try:
            yield b"first"
            if failure == "exception":
                raise ValueError("upload exploded")
            if failure == "cancelled":
                raise asyncio.CancelledError("generator cancelled")
            yield object()
        finally:
            closed.set()

    async with local_server() as (url, _), wreq.Client(proxies=[]) as client:
        with pytest.raises(wreq.exceptions.RequestError):
            await asyncio.wait_for(client.post(url, body=chunks()), 5)
        await asyncio.wait_for(closed.wait(), 5)


@pytest.mark.asyncio
async def test_invalid_option_does_not_start_the_body():
    started = asyncio.Event()

    async def chunks():
        started.set()
        yield b"first"

    async with local_server() as (url, _), wreq.Client(proxies=[]) as client:
        # Options are validated before the body generator is consumed.
        with pytest.raises(TypeError):
            await client.post(url, body=chunks(), timeout=5)
        await asyncio.sleep(0.1)
        assert not started.is_set()


@pytest.mark.asyncio
@pytest.mark.parametrize("action", ["cancel", "early_response"])
async def test_abandoned_upload_to_stalled_peer(action):
    # The peer reads only the head, so the upload stalls with the socket buffers full.
    closed = asyncio.Event()

    async def chunks():
        try:
            while True:
                yield b"x" * 65536
        finally:
            closed.set()

    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.post(url, body=chunks()))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        await asyncio.sleep(0.5)
        if action == "cancel":
            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task
        else:
            # Close an early response whose body never completes, mid-upload.
            writer.write(
                b"HTTP/1.1 413 Payload Too Large\r\nContent-Length: 10\r\n\r\nab"
            )
            await writer.drain()
            response = await asyncio.wait_for(task, 5)
            await response.close()
        await asyncio.wait_for(closed.wait(), 5)


@pytest.mark.asyncio
@pytest.mark.parametrize("action", ["cancel", "close_client"])
async def test_upload_cancellation(action):
    started = asyncio.Event()
    closed = asyncio.Event()

    async def chunks():
        try:
            yield b"first"
            started.set()
            await asyncio.Event().wait()
        finally:
            await asyncio.sleep(0)
            closed.set()

    async with local_server() as (url, _), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.post(url, body=chunks()))
        await asyncio.wait_for(started.wait(), 5)
        if action == "cancel":
            task.cancel("stop upload")
        else:
            client.close()
        with pytest.raises(asyncio.CancelledError):
            await asyncio.wait_for(task, 5)
        await asyncio.wait_for(closed.wait(), 5)


@pytest.mark.asyncio
async def test_cancelled_forwarding_fails_the_upload():
    # Cancelling the task that forwards an async generator body must fail the request,
    # not leave it waiting or send the partial body as complete.
    first = asyncio.Event()
    complete = asyncio.Queue()

    async def serve(reader, writer):
        if not server.is_serving():
            writer.close()
            return
        ended = False
        try:
            await reader.readuntil(b"\r\n\r\n")
            while size := int((await reader.readuntil(b"\r\n")).strip(), 16):
                await reader.readexactly(size + 2)
                first.set()
            await reader.readuntil(b"\r\n")
            ended = True
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            complete.put_nowait(ended)
            writer.close()

    async def body():
        yield b"part"
        await asyncio.Event().wait()
        yield b"rest"

    server = await asyncio.start_server(serve, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    try:
        async with wreq.Client(proxies=[]) as client:
            request = asyncio.ensure_future(client.post(url, body=body()))
            await asyncio.wait_for(first.wait(), 5)
            (forward,) = [
                task
                for task in asyncio.all_tasks()
                if task.get_coro().__qualname__ == "forward"
            ]
            forward.cancel()
            with pytest.raises(wreq.exceptions.RequestError):
                await asyncio.wait_for(request, 5)
            assert await asyncio.wait_for(complete.get(), 5) is False
    finally:
        server.close()
        await server.wait_closed()
