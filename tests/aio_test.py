import asyncio
import sys
import threading

import pytest
import wreq

LOOPS = [pytest.param(asyncio.new_event_loop, id="default")]
if sys.platform == "win32":
    LOOPS.append(pytest.param(asyncio.SelectorEventLoop, id="selector"))
try:
    import uvloop
except ImportError:
    pass
else:
    LOOPS.append(pytest.param(uvloop.new_event_loop, id="uvloop"))


async def exchange():
    abandoned = asyncio.Event()
    handlers = set()

    async def serve(reader, writer):
        handlers.add(asyncio.current_task())
        try:
            while head := await reader.readuntil(b"\r\n\r\n"):
                path = head.split(b" ", 2)[1]
                if path == b"/chunks":
                    writer.write(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                    writer.write(b"1\r\na\r\n1\r\nb\r\n0\r\n\r\n")
                    await writer.drain()
                    continue
                if path == b"/hang":
                    await reader.read()
                    abandoned.set()
                    break
                body = path.lstrip(b"/") * 4096
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: %d\r\n\r\n" % len(body))
                writer.write(body)
                await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            writer.close()

    server = await asyncio.start_server(serve, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}"
    try:
        async with wreq.Client(proxies=[]) as client:

            async def fetch(i):
                async with await client.get(f"{url}/{i}") as response:
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
                async with await client.get(f"{url}/chunks") as response:
                    return b"".join([bytes(c) async for c in response.stream()])

            assert await asyncio.gather(*(drain_chunks() for _ in range(300))) == [b"ab"] * 300

            # Cancelling a pending request resumes the task and aborts its Tokio task,
            # which drops the connection.
            pending = asyncio.ensure_future(client.get(f"{url}/hang"))
            await asyncio.sleep(0.1)
            pending.cancel()
            with pytest.raises(asyncio.CancelledError):
                await pending
            await asyncio.wait_for(abandoned.wait(), 5)
    finally:
        server.close()
        for handler in handlers:
            handler.cancel()
        await asyncio.gather(*handlers, return_exceptions=True)
        await server.wait_closed()


@pytest.mark.parametrize("new_loop", LOOPS)
def test_wakes_resume_tasks_on_event_loops(new_loop):
    # Each new loop gets its own wake port; no thread relays wakes.
    for _ in range(3):
        loop = new_loop()
        try:
            loop.run_until_complete(exchange())
        finally:
            loop.close()
    assert all(t.name != "wreq-python-waker" for t in threading.enumerate())
