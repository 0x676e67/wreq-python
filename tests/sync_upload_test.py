import asyncio
import datetime
import threading
import time
from contextlib import asynccontextmanager

import pytest
import wreq
from wreq.runtime import Runtime, Scheduler

from upload_test import read_chunked


@asynccontextmanager
async def upload_server():
    bodies = asyncio.Queue()
    handlers = set()
    writers = []
    errors = []

    async def accept(reader, writer):
        if not server.is_serving():
            writer.close()
            return
        task = asyncio.current_task()
        handlers.add(task)
        writers.append(writer)
        try:
            header = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            assert b"transfer-encoding: chunked" in header.lower()
            body = bytearray()
            while True:
                line = await asyncio.wait_for(reader.readline(), 5)
                if not line:
                    return
                size = int(line, 16)
                if not size:
                    assert await reader.readexactly(2) == b"\r\n"
                    break
                body.extend(await asyncio.wait_for(reader.readexactly(size), 5))
                assert await reader.readexactly(2) == b"\r\n"
            bodies.put_nowait(bytes(body))
            writer.write(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nok"
            )
            await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError):
            # A rejected upload can close before any headers or terminal chunk.
            pass
        except Exception as error:
            errors.append(error)
        finally:
            writer.close()
            handlers.discard(task)

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    try:
        yield f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/", bodies
    finally:
        server.close()
        for writer in writers:
            writer.close()
        await asyncio.gather(
            *(writer.wait_closed() for writer in writers), return_exceptions=True
        )
        await asyncio.gather(*handlers, return_exceptions=True)
        await server.wait_closed()
        assert not errors, errors


def upload_client(blocking, current_thread):
    """A client whose iterators run on the blocking pool, or inline on a current-thread
    runtime's driving thread."""
    if current_thread:
        return wreq.blocking.Client(
            proxies=[], runtime=Runtime(scheduler=Scheduler.CURRENT_THREAD)
        )
    return (wreq.blocking.Client if blocking else wreq.Client)(proxies=[])


UPLOAD_CLIENTS = [(False, False), (True, False), (True, True)]


@pytest.mark.asyncio
@pytest.mark.parametrize(("blocking", "current_thread"), UPLOAD_CLIENTS)
@pytest.mark.parametrize("multipart", [False, True])
async def test_sync_upload_iterator_errors(blocking, current_thread, multipart):
    client = upload_client(blocking, current_thread)
    try:
        async with upload_server() as (url, bodies):
            for failure in (False, True):
                finalized = threading.Event()

                def chunks():
                    try:
                        yield b"hello"
                        if failure:
                            raise ValueError("upload exploded")
                        yield " world"
                    finally:
                        finalized.set()

                kwargs = (
                    {
                        "multipart": wreq.Multipart(
                            wreq.Part(name="file", value=chunks())
                        )
                    }
                    if multipart
                    else {"body": chunks()}
                )

                def send_blocking():
                    with client.post(url, **kwargs) as response:
                        return response.bytes()

                async def send():
                    if blocking:
                        return await asyncio.to_thread(send_blocking)
                    response = await client.post(url, **kwargs)
                    async with response:
                        return await response.bytes()

                if failure:
                    with pytest.raises(wreq.exceptions.RequestError) as caught:
                        await asyncio.wait_for(send(), 5)
                    assert "ValueError" in str(caught.value)
                    assert "upload exploded" in str(caught.value)
                else:
                    assert await asyncio.wait_for(send(), 5) == b"ok"
                    body = await asyncio.wait_for(bodies.get(), 5)
                    assert (
                        (b"hello world" in body)
                        if multipart
                        else (body == b"hello world")
                    )
                assert finalized.is_set()
    finally:
        client.close()


@pytest.mark.asyncio
@pytest.mark.parametrize("blocking", [False, True])
async def test_sync_upload_reads_ahead_within_the_budget(blocking):
    # A huge first chunk keeps the connection's write buffer full while the peer waits, so
    # only the budget bounds the read-ahead: 64 small chunks queue at the minimum charge
    # and the next parks with the pump. Once the peer reads, the parked chunk goes first.
    release, bodies, calls = threading.Event(), asyncio.Queue(), []
    parts = [b"\0" * (1 << 24)] + [bytes((index,)) * 4096 for index in range(100)]

    def chunks():
        for part in parts:
            calls.append(None)
            yield part

    async def accept(reader, writer):
        try:
            await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            await asyncio.to_thread(release.wait, 5)
            bodies.put_nowait(await asyncio.wait_for(read_chunked(reader), 10))
            writer.write(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nok"
            )
            await writer.drain()
        finally:
            writer.close()

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    client = upload_client(blocking, False)
    try:
        if blocking:

            def send_blocking():
                with client.post(url, body=chunks()) as response:
                    return response.bytes()

            task = asyncio.create_task(asyncio.to_thread(send_blocking))
        else:

            async def send():
                response = await client.post(url, body=chunks())
                async with response:
                    return await response.bytes()

            task = asyncio.create_task(send())
        seen, stable = -1, 0
        for _ in range(250):
            await asyncio.sleep(0.02)
            seen, stable = len(calls), stable + 1 if len(calls) == seen else 0
            if stable == 5:
                break
        assert len(calls) == 66
        release.set()
        assert await asyncio.wait_for(task, 10) == b"ok"
        assert await asyncio.wait_for(bodies.get(), 5) == b"".join(parts)
    finally:
        release.set()
        client.close()
        server.close()
        await server.wait_closed()


@pytest.mark.asyncio
async def test_blocking_upload_iterator_can_send_requests():
    # The iterator runs on a blocking thread, so it may itself block on requests.
    client = wreq.blocking.Client(proxies=[])
    try:
        async with upload_server() as (url, bodies):
            chunk = b"x" * 16384

            def chunks():
                for i in range(16):
                    if i == 8:
                        with client.post(url, body=iter([b"inner"])) as response:
                            assert response.bytes() == b"ok"
                    yield chunk

            def send():
                with client.post(url, body=chunks()) as response:
                    return response.bytes()

            assert await asyncio.wait_for(asyncio.to_thread(send), 5) == b"ok"
            assert await asyncio.wait_for(bodies.get(), 5) == b"inner"
            assert await asyncio.wait_for(bodies.get(), 5) == chunk * 16
    finally:
        client.close()


@pytest.mark.asyncio
@pytest.mark.parametrize(("blocking", "current_thread"), UPLOAD_CLIENTS)
async def test_sync_upload_sends_each_chunk_as_yielded(blocking, current_thread):
    # A chunk goes out once yielded, before the iterator produces the next one.
    first_seen = threading.Event()
    bodies = asyncio.Queue()

    async def accept(reader, writer):
        try:
            await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            body = bytearray()
            while size := int(await asyncio.wait_for(reader.readline(), 10), 16):
                body.extend(await reader.readexactly(size))
                await reader.readexactly(2)
                first_seen.set()
            await reader.readexactly(2)
            bodies.put_nowait(bytes(body))
            writer.write(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nok"
            )
            await writer.drain()
        finally:
            writer.close()

    def chunks():
        yield b"first"
        # Holding the first chunk back until a later one is ready would stall here.
        assert first_seen.wait(5), "the first chunk was not sent"
        yield b"second"

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    client = upload_client(blocking, current_thread)
    try:
        if blocking:

            def send():
                with client.post(url, body=chunks()) as response:
                    return response.bytes()

            assert await asyncio.wait_for(asyncio.to_thread(send), 10) == b"ok"
        else:
            response = await asyncio.wait_for(client.post(url, body=chunks()), 10)
            async with response:
                assert await response.bytes() == b"ok"
        assert await asyncio.wait_for(bodies.get(), 5) == b"firstsecond"
    finally:
        client.close()
        server.close()
        await server.wait_closed()


@pytest.mark.asyncio
async def test_blocking_upload_returns_on_early_response():
    # A response sent before the body is read returns while a slow iterator still uploads.
    finished = threading.Event()
    bodies = asyncio.Queue()

    async def accept(reader, writer):
        try:
            await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
            await writer.drain()
            body = bytearray()
            while size := int(await asyncio.wait_for(reader.readline(), 10), 16):
                body.extend(await reader.readexactly(size))
                await reader.readexactly(2)
            await reader.readexactly(2)
            bodies.put_nowait(bytes(body))
        finally:
            writer.close()

    def chunks():
        for _ in range(50):
            yield b"x"
            if finished.wait(0.1):
                return

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    client = wreq.blocking.Client(proxies=[])
    try:

        def send():
            started = time.monotonic()
            response = client.post(url, body=chunks())
            elapsed = time.monotonic() - started
            finished.set()
            with response:
                return elapsed, response.bytes()

        elapsed, body = await asyncio.wait_for(asyncio.to_thread(send), 10)
        assert body == b"ok"
        # The iterator alone takes 5 s; the response must not wait for it.
        assert elapsed < 2, elapsed
        assert set(await asyncio.wait_for(bodies.get(), 5)) == {ord("x")}
    finally:
        finished.set()
        client.close()
        server.close()
        await server.wait_closed()


@pytest.mark.asyncio
async def test_current_thread_upload_sees_an_early_response():
    # An inline iterator yields to the runtime before each item, so a response sent
    # before the body is read ends the call within an item or two.
    writers, calls = [], []

    async def accept(reader, writer):
        writers.append(writer)
        try:
            await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
            await writer.drain()
            await asyncio.wait_for(reader.read(), 10)
        except (asyncio.TimeoutError, ConnectionError):
            pass
        finally:
            writer.close()

    def chunks():
        for _ in range(400):
            calls.append(None)
            time.sleep(0.005)
            yield b"x"

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    client = upload_client(True, True)
    try:

        def send():
            timeout = datetime.timedelta(seconds=1)
            with client.post(url, body=chunks(), timeout=timeout) as response:
                return response.status

        assert await asyncio.wait_for(asyncio.to_thread(send), 5) == 200
        assert len(calls) < 20, len(calls)
    finally:
        client.close()
        server.close()
        for writer in writers:
            writer.close()
        await server.wait_closed()


@pytest.mark.asyncio
async def test_blocking_upload_does_not_wait_for_the_iterator():
    # A response or timeout must not wait for a `__next__` call that blocks.
    writers = []

    async def accept(reader, writer):
        if not server.is_serving():
            writer.close()
            return
        writers.append(writer)
        try:
            head = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            if not head.startswith(b"POST /slow "):
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                await writer.drain()
            await asyncio.wait_for(reader.read(), 10)
        except (asyncio.TimeoutError, ConnectionError):
            pass
        finally:
            writer.close()

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    client = wreq.blocking.Client(proxies=[])
    released = threading.Event()

    def chunks():
        yield b"first"
        released.wait(5)
        yield b"second"

    def send(path, **kwargs):
        start = time.monotonic()
        try:
            with client.post(url + path, body=chunks(), **kwargs) as response:
                outcome = response.status
        except wreq.exceptions.TimeoutError:
            outcome = "timeout"
        return outcome, time.monotonic() - start

    try:
        # Full duplex: the iterator waits until the caller has the response.
        assert await asyncio.to_thread(send, "") == (200, pytest.approx(0, abs=2))
        released.set()
        released.clear()
        # A request timeout fires while `__next__` blocks.
        timeout = datetime.timedelta(milliseconds=200)
        outcome = await asyncio.to_thread(send, "slow", timeout=timeout)
        assert outcome == ("timeout", pytest.approx(0, abs=2))
    finally:
        released.set()
        client.close()
        server.close()
        # Pooled connections outlive the client; close them so the server can stop.
        for writer in writers:
            writer.close()
        await server.wait_closed()


@pytest.mark.asyncio
async def test_stalled_uploads_hold_no_blocking_threads():
    # Uploads waiting on a server that stopped reading must not starve the blocking pool.
    finished = asyncio.Event()

    async def accept(reader, writer):
        try:
            head = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
            if head.startswith(b"POST /stall "):
                await asyncio.wait_for(finished.wait(), 10)
            else:
                while await asyncio.wait_for(reader.readline(), 5) not in (
                    b"0\r\n",
                    b"",
                ):
                    pass
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                await writer.drain()
        except (asyncio.IncompleteReadError, asyncio.TimeoutError, ConnectionError):
            pass
        finally:
            writer.close()

    def endless():
        while True:
            yield b"x" * (1 << 20)

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    runtime = wreq.Runtime(workers=1, max_blocking_threads=2)
    client = wreq.Client(proxies=[], runtime=runtime)
    stalled = [
        asyncio.create_task(client.post(url + "stall", body=endless()))
        for _ in range(4)
    ]
    try:
        await asyncio.sleep(0.5)
        response = await asyncio.wait_for(client.post(url, body=iter([b"hello"])), 3)
        async with response:
            assert await response.bytes() == b"ok"
    finally:
        finished.set()
        for task in stalled:
            task.cancel()
        await asyncio.gather(*stalled, return_exceptions=True)
        client.close()
        server.close()
        await server.wait_closed()
