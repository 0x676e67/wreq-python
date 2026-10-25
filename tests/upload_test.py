import asyncio
import contextvars
import gc
import inspect
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
@pytest.mark.parametrize("hook", ["instance", "base", "factory"])
@pytest.mark.parametrize("keywords", [False, True], ids=["legacy", "kwargs"])
async def test_upload_task_creation_hooks(hook, keywords, monkeypatch):
    loop = asyncio.get_running_loop()
    original = loop.create_task
    context = contextvars.ContextVar("task_hook", default="missing")
    created = []

    def tracked(coro, **kwargs):
        # The loop also closes collected async generators here, which PyPy does late.
        if getattr(coro, "__qualname__", None) == "forward":
            created.append(coro)
            task_context = contextvars.copy_context()
            task_context.run(context.set, "wrapped")
            kwargs["context"] = task_context
        if hook == "factory":
            return asyncio.Task(coro, loop=loop, **kwargs)
        return original(coro, **kwargs)

    def legacy(coro, *, name=None, context=None):
        return tracked(coro, name=name, context=context)

    create = tracked if keywords else legacy
    if hook == "instance":
        monkeypatch.setattr(loop, "create_task", create)
    else:
        if keywords:

            def create_on_loop(self, coro, **kwargs):
                return tracked(coro, **kwargs)
        else:

            def create_on_loop(self, coro, *, name=None, context=None):
                return tracked(coro, name=name, context=context)
        if hook == "base":
            monkeypatch.setattr(asyncio.BaseEventLoop, "create_task", create_on_loop)
        else:
            factory = loop.get_task_factory()
            loop.set_task_factory(create_on_loop)

    async def chunks():
        assert context.get() == "wrapped"
        yield b"body"

    try:
        async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
            request = asyncio.create_task(client.post(url, body=chunks()))
            try:
                await asyncio.sleep(0)
                assert len(created) == 1
                reader, writer = await asyncio.wait_for(connections.get(), 5)
                assert await asyncio.wait_for(read_chunked(reader), 5) == b"body"
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                await writer.drain()
                await (await asyncio.wait_for(request, 5)).close()
                assert context.get() == "missing"
            finally:
                request.cancel()
                await asyncio.gather(request, return_exceptions=True)
    finally:
        if hook == "factory":
            loop.set_task_factory(factory)


@pytest.mark.asyncio
@pytest.mark.skipif(
    inspect.getfullargspec(asyncio.BaseEventLoop.create_task).varkw is None,
    reason="loop create_task does not pass eager_start",
)
async def test_upload_starts_eagerly_on_standard_loop():
    started = False

    async def chunks():
        nonlocal started
        started = True
        yield b"body"

    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        request = asyncio.create_task(client.post(url, body=chunks()))
        # Let the request's first poll extract the body, before a separately queued
        # forwarding task could run on the following loop iteration.
        await asyncio.sleep(0)
        assert started
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        assert await asyncio.wait_for(read_chunked(reader), 5) == b"body"
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
        await writer.drain()
        await (await asyncio.wait_for(request, 5)).close()


@pytest.mark.asyncio
@pytest.mark.parametrize("size", [0, 1 << 24], ids=["empty", "queued_tail"])
async def test_finish_delivers_queued_chunks(size):
    # The peer reads nothing until forwarding has finished. A first chunk far above netty's
    # ~408 KiB write buffer stalls the upload, so b"tail" is still queued when `finish`
    # drops the sender; it and the end chunk must still be sent.
    parts = (b"x" * size, b"tail") if size else ()
    yielded = asyncio.Event()

    async def chunks():
        for part in parts:
            yield part
        yielded.set()

    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.post(url, body=chunks()))
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        await asyncio.wait_for(yielded.wait(), 5)
        forwarding = [
            t for t in asyncio.all_tasks() if t.get_coro().__qualname__ == "forward"
        ]
        # `finish` does not wait for room, so forwarding ends before the peer reads.
        await asyncio.wait_for(asyncio.gather(*forwarding), 5)
        assert await asyncio.wait_for(read_chunked(reader), 10) == b"".join(parts)
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
        await writer.drain()
        await (await asyncio.wait_for(task, 5)).close()


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
@pytest.mark.parametrize("options", [{"zstd": 1}, {"complete_nonsense": 1}])
async def test_invalid_option_does_not_start_the_body(options):
    started = asyncio.Event()

    async def chunks():
        started.set()
        yield b"first"

    form = wreq.Multipart(wreq.Part("file", iter([b"x"])))
    async with local_server() as (url, _), wreq.Client(proxies=[]) as client:
        # Unknown keys and invalid values are rejected before taking the body or form.
        with pytest.raises(TypeError):
            await client.post(url, body=chunks(), **options)
        for _ in range(2):
            with pytest.raises(TypeError):
                await client.post(url, multipart=form, **options)
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


@pytest.mark.asyncio
async def test_upload_task_code_replacement_keeps_hook_arguments(monkeypatch):
    loop = asyncio.get_running_loop()
    original = loop.create_task
    calls = []

    def create(coro, **kwargs):
        if getattr(coro, "__qualname__", None) == "forward":
            calls.append(dict(kwargs))
        return original(coro, **kwargs)

    def legacy(coro, *, name=None, context=None):
        if getattr(coro, "__qualname__", None) == "forward":
            calls.append({})
        return original(coro, name=name, context=context)

    monkeypatch.setattr(loop, "create_task", create)
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        for code in (create.__code__, legacy.__code__):
            create.__code__ = code
            create.__kwdefaults__ = legacy.__kwdefaults__

            async def chunks():
                yield b"body"

            task = original(client.post(url, body=chunks()))
            reader, writer = await asyncio.wait_for(connections.get(), 5)
            assert await asyncio.wait_for(read_chunked(reader), 5) == b"body"
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            await writer.drain()
            await (await asyncio.wait_for(task, 5)).close()
    assert len(calls) == 2
    assert calls[-1] == {}
