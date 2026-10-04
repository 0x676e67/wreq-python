import asyncio
import gc
import socket
import sys
import time
import weakref

import pytest
import wreq


class NoReaderLoop(asyncio.SelectorEventLoop):
    """A loop that cannot watch the wake socket, so wakes are scheduled thread-safely."""

    ports = 0

    def add_reader(self, *args):
        # Each wake port first tries to register its socket.
        self.ports += 1
        raise NotImplementedError


LOOPS = [
    pytest.param(asyncio.new_event_loop, id="default"),
    pytest.param(NoReaderLoop, id="no-reader"),
]
if sys.platform == "win32":
    LOOPS.append(pytest.param(asyncio.SelectorEventLoop, id="selector"))
try:
    import uvloop
except ImportError:
    pass
else:
    LOOPS.append(pytest.param(uvloop.new_event_loop, id="uvloop"))


async def exchange():
    hanging = asyncio.Event()
    abandoned = asyncio.Event()
    handlers = set()
    stopping = False

    async def serve(reader, writer):
        # A pooled connection accepted during teardown would otherwise idle until
        # `wait_closed` times out.
        if stopping:
            writer.close()
            return
        handlers.add(asyncio.current_task())
        try:
            while True:
                head = await reader.readuntil(b"\r\n\r\n")
                path = head.split(b" ", 2)[1]
                if path == b"/chunks":
                    writer.write(
                        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n"
                    )
                    writer.write(b"1\r\na\r\n1\r\nb\r\n0\r\n\r\n")
                    await writer.drain()
                    continue
                if path == b"/hang":
                    hanging.set()
                    await reader.read()
                    abandoned.set()
                    break
                body = path.lstrip(b"/") * 4096
                writer.write(
                    b"HTTP/1.1 200 OK\r\nContent-Length: %d\r\n\r\n" % len(body)
                )
                writer.write(body)
                await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            writer.close()

    # The default backlog of 100 could overflow with 300 concurrent stream connections.
    server = await asyncio.start_server(serve, "127.0.0.1", 0, backlog=512)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}"
    try:
        async with wreq.Client(proxies=[]) as client:

            async def fetch(i):
                async with client.get(f"{url}/{i}") as response:
                    return bytes(await response.bytes())

            # Concurrent completions resume their tasks through the loop's wake port.
            bodies = await asyncio.gather(*(fetch(i) for i in range(64)))
            assert bodies == [str(i).encode() * 4096 for i in range(64)]

            response = await client.get(f"{url}/stream")
            async with response.stream() as stream:
                frames = [bytes(chunk) async for chunk in stream]
                assert b"".join(frames) == b"stream" * 4096

            # Short streams end right after their last frame; readers must see the end.
            async def drain_chunks():
                async with client.get(f"{url}/chunks") as response:
                    return b"".join([bytes(c) async for c in response.stream()])

            assert (
                await asyncio.gather(*(drain_chunks() for _ in range(300)))
                == [b"ab"] * 300
            )

            # Cancelling a pending request resumes the task and aborts its Tokio task,
            # which drops the connection.
            hang = client.get(f"{url}/hang")
            pending = asyncio.ensure_future(hang)
            await asyncio.wait_for(hanging.wait(), 5)
            # A second task awaiting the same coroutine fails without stranding the first.
            with pytest.raises(RuntimeError, match="awaited already"):
                await hang
            pending.cancel()
            with pytest.raises(asyncio.CancelledError):
                await pending
            await asyncio.wait_for(abandoned.wait(), 5)
    finally:
        stopping = True
        server.close()
        for handler in handlers:
            handler.cancel()
        await asyncio.gather(*handlers, return_exceptions=True)
        await server.wait_closed()


@pytest.mark.parametrize("new_loop", LOOPS)
def test_wakes_resume_tasks_on_event_loops(new_loop):
    # Each new loop gets its own wake port.
    for _ in range(3):
        loop = new_loop()
        try:
            loop.run_until_complete(asyncio.wait_for(exchange(), 30))
            # A loop keeps one wake port across all of its awaits.
            assert getattr(loop, "ports", 1) == 1
        finally:
            loop.close()


@pytest.mark.skipif(
    sys.implementation.name != "cpython",
    reason="PyPy's cpyext does not collect cycles through extension objects",
)
@pytest.mark.parametrize("new_loop", LOOPS)
@pytest.mark.parametrize("woken", [False, True], ids=["pending", "woken"])
def test_closed_loop_releases_pending_requests(new_loop, woken):
    # The listener accepts connections into its backlog and answers only when woken.
    with socket.create_server(("127.0.0.1", 0)) as server:
        url = f"http://127.0.0.1:{server.getsockname()[1]}/"
        client = wreq.Client(proxies=[])
        loop = new_loop()
        task = loop.create_task(client.get(url))
        loop.run_until_complete(asyncio.sleep(0.2))
        if woken:
            # Answer while the loop is stopped, so its wake is queued but never runs.
            conn, _ = server.accept()
            conn.recv(65536)
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
            time.sleep(0.2)
            conn.close()
        loop.close()
        # Once its loop is closed, a task left pending can be collected with its request.
        ref = weakref.ref(task)
        del task
        gc.collect()
        assert ref() is None


@pytest.mark.asyncio
async def test_request_coroutine_enters_its_result():
    writers = []

    async def serve(reader, writer):
        if not server.is_serving():
            writer.close()
            return
        writers.append(writer)
        try:
            while True:
                await reader.readuntil(b"\r\n\r\n")
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            writer.close()

    server = await asyncio.start_server(serve, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    try:
        async with wreq.Client(proxies=[]) as client:
            # Entering awaits the request; exiting releases the response body.
            async with client.get(url) as response:
                assert isinstance(response, wreq.Response)
                assert await response.text() == "ok"
            with pytest.raises(RuntimeError, match="consumed"):
                await response.text()

            # Entering also awaits the response's own `__aenter__`, as `async with await` does.
            entered = []
            original = wreq.Response.__aenter__

            async def aenter(self):
                await asyncio.sleep(0.01)
                entered.append(self)
                return "entered"

            wreq.Response.__aenter__ = aenter
            try:
                async with client.get(url) as value:
                    assert value == "entered"
            finally:
                wreq.Response.__aenter__ = original
            assert len(entered) == 1
            with pytest.raises(RuntimeError, match="consumed"):
                await entered[0].text()

            # The response exits even when the block fails.
            with pytest.raises(KeyError):
                async with wreq.get(url, proxies=[]) as response:
                    raise KeyError
            with pytest.raises(RuntimeError, match="consumed"):
                await response.text()

            # A request failure propagates from entering, before any exit.
            with pytest.raises(wreq.exceptions.BuilderError):
                async with client.get("http://"):
                    pass

            # The coroutine is entered or awaited once; plain coroutines cannot be entered.
            coroutine = client.get(url)
            assert await coroutine is not None
            with pytest.raises(RuntimeError, match="already awaited"):
                async with coroutine:
                    pass
            text = response.text()
            with pytest.raises(RuntimeError, match="not entered"):
                text.__aexit__(None, None, None)
            with pytest.raises(TypeError, match="context manager"):
                async with text:
                    pass
    finally:
        server.close()
        # Pooled connections outlive the client; close them so the server can stop.
        for writer in writers:
            writer.close()
        await server.wait_closed()


@pytest.mark.asyncio
async def test_entering_rejects_a_second_driver():
    gate = asyncio.Event()
    writers = []

    async def serve(reader, writer):
        if not server.is_serving():
            writer.close()
            return
        writers.append(writer)
        try:
            while True:
                head = await reader.readuntil(b"\r\n\r\n")
                if head.startswith(b"GET /slow "):
                    await gate.wait()
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            writer.close()

    server = await asyncio.start_server(serve, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    original = wreq.Response.__aenter__
    try:
        async with wreq.Client(proxies=[]) as client:
            # A second task awaiting a coroutine suspended in `__aenter__` fails, while
            # the first one still enters.
            opening = asyncio.Event()

            async def aenter(self):
                opening.set()
                await gate.wait()
                return self

            wreq.Response.__aenter__ = aenter

            async def enter(coroutine):
                async with coroutine as response:
                    return await response.text()

            coroutine = client.get(url)
            first = asyncio.create_task(enter(coroutine))
            await asyncio.wait_for(opening.wait(), 5)
            with pytest.raises(RuntimeError, match="awaited already"):
                await coroutine
            gate.set()
            assert await asyncio.wait_for(first, 5) == "ok"

            # An `__aenter__` returning a coroutine another task awaits fails, and that
            # task still completes.
            gate.clear()
            awaited = client.get(url + "slow")
            other = asyncio.ensure_future(awaited)
            await asyncio.sleep(0.05)
            wreq.Response.__aenter__ = lambda self: awaited
            with pytest.raises(RuntimeError, match="awaited already"):
                async with client.get(url):
                    pass
            wreq.Response.__aenter__ = original
            gate.set()
            assert await (await asyncio.wait_for(other, 5)).text() == "ok"

            # So does a Python coroutine another task is suspended in.
            gate.clear()

            async def slow():
                await gate.wait()
                return "slow"

            shared = slow()
            other = asyncio.ensure_future(shared)
            await asyncio.sleep(0)
            wreq.Response.__aenter__ = lambda self: shared
            with pytest.raises(RuntimeError, match="awaited already"):
                async with client.get(url):
                    pass
            wreq.Response.__aenter__ = original
            gate.set()
            assert await asyncio.wait_for(other, 5) == "slow"

            # An `__aenter__` result must be awaitable.
            wreq.Response.__aenter__ = lambda self: 1
            with pytest.raises(TypeError, match="does not implement __await__"):
                async with client.get(url):
                    pass
    finally:
        wreq.Response.__aenter__ = original
        gate.set()
        server.close()
        for writer in writers:
            writer.close()
        await server.wait_closed()
