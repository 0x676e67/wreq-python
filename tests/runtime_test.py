import asyncio
import base64
import hashlib
import threading
from contextlib import asynccontextmanager
from datetime import timedelta

import pytest
import wreq
from wreq.runtime import Runtime, Scheduler

from cancellation_test import local_server
from upload_test import read_chunked


def custom_runtime(scheduler, **options):
    """A one-worker runtime, or a current-thread one, which has no workers."""
    if scheduler != Scheduler.CURRENT_THREAD:
        options["workers"] = 1
    return Runtime(scheduler=scheduler, **options)


def test_runtime_configuration():
    assert Runtime is wreq.Runtime
    assert Runtime is wreq.runtime.Runtime
    assert Scheduler is wreq.Scheduler
    assert {"Runtime", "Scheduler"} <= set(wreq.__all__)
    for kwargs in (
        {"workers": 0},
        {"max_blocking_threads": 0},
        {"thread_name": "bad\0name"},
        {"thread_keep_alive": timedelta(microseconds=-1)},
        {"scheduler": Scheduler.CURRENT_THREAD, "workers": 1},
    ):
        with pytest.raises(ValueError):
            Runtime(**kwargs)
    with pytest.raises(TypeError):
        Runtime(thread_keep_alive=0.25)
    with pytest.raises(TypeError):
        wreq.Client(runtime=object())
    for duration in (None, timedelta(), timedelta(microseconds=250001)):
        runtime = Runtime(
            scheduler=Scheduler.PER_WORKER,
            workers=1,
            thread_name=None if duration is None else "isolated",
            max_blocking_threads=3,
            thread_keep_alive=duration,
        )
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
        client = factory(runtime=None)
        assert isinstance(client.runtime, Runtime)
        client.close()


@pytest.mark.asyncio
@pytest.mark.parametrize("scheduler", [Scheduler.PER_WORKER, Scheduler.WORK_STEALING])
async def test_response_and_stream_keep_runtime_alive(scheduler):
    runtime = custom_runtime(scheduler)
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
async def test_shared_runtime_cancellation_and_upload():
    runtime = Runtime(scheduler=Scheduler.PER_WORKER, workers=2, max_blocking_threads=2)
    async with local_server() as (url, connections):
        first = wreq.Client(runtime=runtime, proxies=[])
        second = wreq.Client(runtime=runtime, proxies=[])
        del runtime
        pending = asyncio.create_task(first.get(url))
        await asyncio.wait_for(connections.get(), 5)
        first.close()
        with pytest.raises(asyncio.CancelledError):
            await asyncio.wait_for(pending, 5)
        with pytest.raises(asyncio.CancelledError):
            await first.get("invalid URL")
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
@pytest.mark.parametrize(
    "scheduler",
    [None, Scheduler.PER_WORKER, Scheduler.WORK_STEALING, Scheduler.CURRENT_THREAD],
)
async def test_blocking_client_runtime(scheduler):
    runtime = (
        None if scheduler is None else custom_runtime(scheduler, max_blocking_threads=2)
    )

    def request(url):
        with wreq.blocking.Client(runtime=runtime, proxies=[]) as client:
            assert isinstance(client.runtime, Runtime)
            with client.post(url, body=iter((b"sync", b" upload"))) as response:
                assert response.bytes() == b"{}"
                assert response.json() == {}
            with client.post(url, body=iter((b"sync", b" upload"))) as response:
                with response.stream() as stream:
                    return b"".join(stream)

    async with local_server() as (url, connections):
        task = asyncio.create_task(asyncio.to_thread(request, url))
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        for index in range(2):
            if index:
                # Leaving `with` after a full read keeps the connection reusable.
                await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            assert await asyncio.wait_for(read_chunked(reader), 5) == b"sync upload"
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            await writer.drain()
        assert await asyncio.wait_for(task, 5) == b"{}"
        assert connections.empty()
        del task


async def run_blocking(func, *args, timeout=5):
    """Run a blocking call on a daemon thread, so a call that never returns fails the
    test instead of hanging executor shutdown."""
    loop = asyncio.get_running_loop()
    future = loop.create_future()

    def run():
        try:
            outcome = (func(*args), None)
        except BaseException as error:
            outcome = (None, error)

        def settle():
            if not future.done():
                if outcome[1] is None:
                    future.set_result(outcome[0])
                else:
                    future.set_exception(outcome[1])

        loop.call_soon_threadsafe(settle)

    threading.Thread(target=run, daemon=True).start()
    return await asyncio.wait_for(future, timeout)


@asynccontextmanager
async def path_server():
    """Answer each request with its path, sending the body after the head so reading it
    must drive the runtime. `/hold` waits for `release`; `arrived` records paths."""
    arrived, release, uploads, writers = asyncio.Queue(), asyncio.Event(), [], []

    async def accept(reader, writer):
        writers.append(writer)
        try:
            head = await reader.readuntil(b"\r\n\r\n")
            path = head.split(b" ", 2)[1]
            arrived.put_nowait(path.decode())
            if b"transfer-encoding: chunked" in head.lower():
                uploads.append(await read_chunked(reader))
            if path == b"/hold":
                await release.wait()
            writer.write(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\n"
                b"Content-Length: %d\r\n\r\n" % len(path)
            )
            await writer.drain()
            await asyncio.sleep(0.05)
            writer.write(path)
            await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError, ValueError):
            # Interrupted uploads end mid-body.
            pass
        finally:
            writer.close()

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    try:
        url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}"
        yield url, arrived, release, uploads
    finally:
        release.set()
        server.close()
        for writer in writers:
            writer.close()
        await server.wait_closed()


@pytest.mark.asyncio
async def test_current_thread_runtime():
    runtime = Runtime(scheduler=Scheduler.CURRENT_THREAD)
    with pytest.raises(ValueError):
        wreq.Client(runtime=runtime)
    client = wreq.blocking.Client(runtime=runtime, proxies=[])

    class Interrupt(BaseException):
        pass

    def fetch(path):
        with client.get(url + path) as response:
            return response.text()

    def interrupted(error):
        yield b"x"
        raise error

    def upload_interrupted(error=Interrupt):
        client.post(url + "/interrupted", body=interrupted(error))

    def run():
        caller, readers = threading.get_ident(), []
        pending = client.get(url + "/pending")
        streamed = client.get(url + "/streamed").stream()

        def chunks():
            # Read on the thread driving the runtime, which refuses blocking calls
            # before they consume anything.
            readers.append(threading.get_ident())
            yield b"sync"
            for read in (lambda: fetch("/nested"), pending.text, pending.bytes):
                with pytest.raises(RuntimeError, match="CURRENT_THREAD"):
                    read()
            with pytest.raises(RuntimeError, match="CURRENT_THREAD"):
                next(streamed)
            yield b" upload"

        with client.post(url + "/upload", body=chunks()) as response:
            uploaded = response.text()
        # Nothing would drive an async read, so it is refused up front.
        for entry in (streamed.__aiter__, streamed.__anext__, streamed.__aenter__):
            with pytest.raises(RuntimeError):
                entry()

        # A KeyboardInterrupt-like error from the caller's own iterator surfaces as
        # itself.
        with pytest.raises(Interrupt):
            upload_interrupted()
        return readers == [caller], uploaded, pending.text(), b"".join(streamed)

    try:
        async with path_server() as (url, arrived, release, uploads):
            assert await run_blocking(run) == (
                True,
                "/upload",
                "/pending",
                b"/streamed",
            )
            assert uploads == [b"sync upload"]
            seen = []
            while not seen or seen[-1] != "/interrupted":
                seen.append(await asyncio.wait_for(arrived.get(), 5))
            assert "/nested" not in seen

            # Threads sharing the runtime take turns driving it, so a request waiting
            # on the server does not hold up another thread's request.
            held = asyncio.create_task(run_blocking(fetch, "/hold"))
            assert await asyncio.wait_for(arrived.get(), 5) == "/hold"
            assert await run_blocking(fetch, "/other") == "/other"
            # This upload's iterator runs on the held thread, which keeps its own
            # result; the uploading call gets the wrapped error.
            with pytest.raises(wreq.exceptions.RequestError, match="Interrupt"):
                await run_blocking(upload_interrupted)
            assert not held.done()
            release.set()
            assert await held == "/hold"

            # Ctrl+C lands on whichever thread runs Python, so it ends the driving thread's
            # own call, while the uploading call gets the wrapped error.
            def fetch_interrupted():
                # Caught here: a KeyboardInterrupt reaching an asyncio task stops the loop.
                with pytest.raises(KeyboardInterrupt):
                    fetch("/hold")

            release.clear()
            held = asyncio.create_task(run_blocking(fetch_interrupted))
            while await asyncio.wait_for(arrived.get(), 5) != "/hold":
                pass
            with pytest.raises(wreq.exceptions.RequestError, match="KeyboardInterrupt"):
                await run_blocking(upload_interrupted, KeyboardInterrupt)
            assert not held.done()
            release.set()
            await held

            # A multipart iterator belongs to the thread sending it, not the one that built
            # its part, so the held thread that built it keeps its own result.
            built = []

            def build_and_fetch():
                built.append(wreq.Part(name="file", value=interrupted(Interrupt)))
                return fetch("/hold")

            def upload_part():
                client.post(url + "/interrupted", multipart=wreq.Multipart(built[0]))

            release.clear()
            held = asyncio.create_task(run_blocking(build_and_fetch))
            while await asyncio.wait_for(arrived.get(), 5) != "/hold":
                pass
            with pytest.raises(wreq.exceptions.RequestError, match="Interrupt"):
                await run_blocking(upload_part)
            assert not held.done()
            release.set()
            assert await held == "/hold"

            # Closing the client from another thread ends a call driving the runtime.
            release.clear()
            held = asyncio.create_task(run_blocking(fetch, "/hold"))
            while await asyncio.wait_for(arrived.get(), 5) != "/hold":
                pass
            client.close()
            with pytest.raises(asyncio.CancelledError):
                await held
    finally:
        client.close()


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("blocking", "operation"),
    [(True, "get"), (False, "websocket"), (True, "websocket")],
)
async def test_client_close_cancels_requests(blocking, operation):
    factory = wreq.blocking.Client if blocking else wreq.Client
    client = factory(
        runtime=custom_runtime(Scheduler.PER_WORKER),
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
            with pytest.raises(asyncio.CancelledError) as caught:
                await task
            assert caught.value.args == (
                "Operation was cancelled: client has been closed",
            )
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
@pytest.mark.parametrize(
    ("blocking", "scheduler"),
    [
        (False, Scheduler.PER_WORKER),
        (True, Scheduler.PER_WORKER),
        (True, Scheduler.CURRENT_THREAD),
    ],
)
async def test_websocket_outlives_client(blocking, scheduler):
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
    runtime = custom_runtime(scheduler)
    writer = None
    try:
        url = f"ws://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
        client = (wreq.blocking.Client if blocking else wreq.Client)(
            runtime=runtime, proxies=[]
        )
        ws = (
            await run_blocking(client.websocket, url)
            if blocking
            else await client.websocket(url)
        )
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        client.close()
        del client, runtime
        writer.write(b"\x81\x04pong")
        await writer.drain()
        message = await run_blocking(ws.recv) if blocking else await ws.recv()
        assert message.text == "pong"
        outgoing = wreq.Message.from_text("ping")
        if blocking:
            await run_blocking(ws.send, outgoing)
        else:
            await ws.send(outgoing)
        frame = await asyncio.wait_for(reader.readexactly(10), 5)
        assert frame[:2] == b"\x81\x84"
        assert (
            bytes(byte ^ frame[2 + i % 4] for i, byte in enumerate(frame[6:]))
            == b"ping"
        )
        if blocking:
            await run_blocking(ws.close)
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
@pytest.mark.parametrize("scheduler", [Scheduler.PER_WORKER, Scheduler.WORK_STEALING])
async def test_http2_multiplexing_on_custom_runtime(scheduler):
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

    runtime = Runtime(scheduler=scheduler, workers=2)
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
