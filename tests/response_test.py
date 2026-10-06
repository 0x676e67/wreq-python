import asyncio
import json
import socket
import threading
import time
from contextlib import asynccontextmanager
from datetime import timedelta

import pytest
import wreq
from pathlib import Path
from wreq import Version, Multipart, Part

from cancellation_test import local_server

client = wreq.Client(tls_info=True)


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_multiple_requests():
    async def file_to_bytes_stream(file_path):
        with open(file_path, "rb") as f:
            while chunk := f.read(1024):
                yield chunk

    resp = await client.post(
        "http://localhost:8080/anything",
        multipart=Multipart(
            Part(name="def", value="111", filename="def.txt", mime="text/plain"),
            Part(name="abc", value=b"000", filename="abc.txt", mime="text/plain"),
            Part(
                name="LICENSE",
                value=Path("./LICENSE"),
                filename="LICENSE",
                mime="text/plain",
            ),
            Part(
                name="Cargo.toml",
                value=file_to_bytes_stream("./Cargo.toml"),
                filename="Cargo.toml",
                mime="text/plain",
            ),
        ),
    )
    async with resp:
        assert resp.status.is_success() is True
        text = await resp.text()
        assert "111" in text
        assert "000" in text
        assert "wreq" in text


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_cookies():
    url = "http://localhost:8080/cookies/set?mycookie=testvalue"
    resp = await client.get(url)
    async with resp:
        assert any(cookie.name == "mycookie" for cookie in resp.cookies)


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_headers():
    url = "http://localhost:8080/headers"
    resp = await client.get(url)
    async with resp:
        assert resp.headers is not None


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_getters():
    url = "http://localhost:8080/anything"
    resp = await client.get(url, version=Version.HTTP_11)
    async with resp:
        assert resp.url == url
        assert resp.status.is_success()
        assert resp.version == Version.HTTP_11


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_json():
    url = "http://localhost:8080/json"
    resp = await client.get(url)
    async with resp:
        json = await resp.json()
        assert json is not None


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_text():
    url = "http://localhost:8080/html"
    resp = await client.get(url)
    async with resp:
        text = await resp.text()
        assert text is not None


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_bytes():
    url = "http://localhost:8080/image/png"
    resp = await client.get(url)
    async with resp:
        bytes = await resp.bytes()
        assert bytes is not None


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_get_stream():
    url = "http://localhost:8080/stream/1"
    resp = await client.get(url)
    async with resp:
        async with resp.stream() as streamer:
            async for bytes in streamer:
                assert bytes is not None


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_peer_certificate():
    resp = await client.get("https://www.google.com/anything")
    async with resp:
        assert resp.tls_info is not None
        certificate = resp.tls_info.peer_certificate()
        assert type(certificate) is memoryview
        assert certificate.readonly


@pytest.mark.asyncio
async def test_context_exit_keeps_connection_but_close_forbids_reuse():
    reply = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.get(url))
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(reply)
        async with await asyncio.wait_for(task, 5) as response:
            assert bytes(await response.bytes()) == b"ok"

        # Leaving `async with` after a full read returns the connection to the pool.
        task = asyncio.create_task(client.get(url))
        await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
        writer.write(reply)
        response = await asyncio.wait_for(task, 5)
        assert bytes(await response.bytes()) == b"ok"
        await response.close()

        # An explicit close() still forbids reuse, so the next request reconnects.
        task = asyncio.create_task(client.get(url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(reply)
        assert bytes(await (await asyncio.wait_for(task, 5)).bytes()) == b"ok"
        assert connections.empty()


# 2**63 overflows `isize`, so it decodes as a float.
JSON_ITEM = {"n": [0, -2, 1.5, 2**63, True, None], "s": "\u00e9\n", "o": {}}


@pytest.mark.asyncio
@pytest.mark.parametrize("count", [1, 200, 1200], ids=["inline", "runtime", "large"])
async def test_reads_decode_buffered_and_cached_bodies(count):
    payload = json.dumps([JSON_ITEM] * count).encode()
    reply = b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: %d\r\n\r\n%s" % (
        len(payload),
        payload,
    )
    # A read timeout starts a timer even when the body is read on the event loop.
    options = {"proxies": [], "read_timeout": timedelta(seconds=5)}

    def read_blocking(url):
        with wreq.blocking.Client(**options).get(url) as response:
            return response.json(), response.text(), bytes(response.bytes())

    async with local_server() as (url, connections):
        async with wreq.Client(**options) as client:
            task = asyncio.create_task(client.get(url))
            _, writer = await asyncio.wait_for(connections.get(), 5)
            writer.write(reply)
            response = await asyncio.wait_for(task, 5)
            value = await response.json()
            reads = value, await response.text(), bytes(await response.bytes())

        task = asyncio.create_task(asyncio.to_thread(read_blocking, url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(reply)
        assert await asyncio.wait_for(task, 5) == reads

    assert reads == ([JSON_ITEM] * count, payload.decode(), payload)
    assert isinstance(value[0]["n"][3], float)


@pytest.mark.asyncio
async def test_empty_body_and_unit_results():
    async with local_server() as (url, connections):
        client = wreq.Client(proxies=[])

        async def get():
            task = asyncio.create_task(client.get(url))
            _, writer = await asyncio.wait_for(connections.get(), 5)
            writer.write(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
            )
            return await asyncio.wait_for(task, 5)

        response = await get()
        assert bytes(await response.bytes()) == b""
        assert await response.text() == ""
        assert await response.__aexit__(None, None, None) is None

        response = await get()
        streamer = response.stream()
        assert [chunk async for chunk in streamer] == []
        assert await streamer.__aexit__(None, None, None) is None
        # Coroutines without a result return None, as sync methods do.
        with pytest.raises(StopIteration) as stop:
            response.close().__next__()
        assert stop.value.value is None
        assert await client.__aexit__(None, None, None) is None


@pytest.mark.asyncio
async def test_pending_read_holds_the_body_until_it_ends():
    head = b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n"
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.get(url))
        reader, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(head + b"hello")
        response = await asyncio.wait_for(task, 5)
        first = asyncio.create_task(response.text())
        await asyncio.sleep(0.1)
        with pytest.raises(RuntimeError):
            await response.bytes()
        writer.write(b"world")
        assert await asyncio.wait_for(first, 5) == "helloworld"
        assert bytes(await response.bytes()) == b"helloworld"

        # The fully read connection is reused; a truncated body fails, and so do later reads.
        task = asyncio.create_task(client.get(url))
        await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 5)
        writer.write(head + b"hello")
        response = await asyncio.wait_for(task, 5)
        writer.close()
        with pytest.raises(wreq.exceptions.DecodingError):
            await asyncio.wait_for(response.bytes(), 5)
        with pytest.raises(RuntimeError):
            await response.text()


async def steps(coroutine):
    """Drive `coroutine` by hand; return its result and how often it suspended."""
    suspended = 0
    while True:
        try:
            waiter = coroutine.send(None)
        except StopIteration as stop:
            return stop.value, suspended
        except StopAsyncIteration:
            return None, suspended
        suspended += 1
        if waiter is None:
            await asyncio.sleep(0)
        else:
            await asyncio.wait({waiter}, timeout=5)


@asynccontextmanager
async def body_server(http2, body):
    """Answer every request with `body` and its Content-Length, over HTTP/1 or h2c."""
    length = str(len(body)).encode()
    handlers, writers = set(), []

    def frame(kind, flags, stream, payload=b""):
        size = len(payload).to_bytes(3, "big")
        return size + bytes((kind, flags)) + stream.to_bytes(4, "big") + payload

    async def accept(reader, writer):
        handlers.add(asyncio.current_task())
        writers.append(writer)
        try:
            if not http2:
                await reader.readuntil(b"\r\n\r\n")
                head = b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: "
                writer.write(head + length + b"\r\n\r\n" + body)
                await writer.drain()
                await reader.read()
                return
            assert await reader.readexactly(24) == b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n"
            writer.write(frame(4, 0, 0))
            while True:
                header = await reader.readexactly(9)
                payload = await reader.readexactly(int.from_bytes(header[:3], "big"))
                kind, flags = header[3:5]
                stream = int.from_bytes(header[5:], "big") & 0x7FFFFFFF
                if kind == 4 and not flags & 1:
                    writer.write(frame(4, 1, 0))
                elif kind == 1:
                    # `:status: 200` and `content-length`, then the body ending the stream.
                    block = b"\x88\x0f\x0d" + bytes((len(length),)) + length
                    if body:
                        writer.write(
                            frame(1, 4, stream, block) + frame(0, 1, stream, body)
                        )
                    else:
                        writer.write(frame(1, 5, stream, block))
                await writer.drain()
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            handlers.discard(asyncio.current_task())
            writer.close()

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    try:
        yield f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    finally:
        server.close()
        for writer in writers:
            writer.close()
        await asyncio.gather(*handlers, return_exceptions=True)
        await server.wait_closed()


@pytest.mark.asyncio
@pytest.mark.parametrize("read", ["bytes", "stream"])
@pytest.mark.parametrize("body", [b"ok", b""], ids=["body", "empty"])
@pytest.mark.parametrize("http2", [False, True], ids=["http1", "http2"])
async def test_only_http1_bodies_are_read_on_the_event_loop(http2, body, read):
    # A buffered HTTP/1 body is read in the first step of the coroutine. An HTTP/2 body,
    # even an empty one, is always read on the runtime, so the coroutine suspends first.
    async with body_server(http2, body) as url:
        async with wreq.Client(http2_only=http2, proxies=[]) as client:
            response = await asyncio.wait_for(client.get(url), 5)
            assert response.version == (Version.HTTP_2 if http2 else Version.HTTP_11)
            # Let the whole body arrive before the read starts.
            await asyncio.sleep(0.05)
            if read == "bytes":
                result, suspended = await steps(response.bytes())
                assert bytes(result) == body
            else:
                result, suspended = await steps(response.stream().__anext__())
                assert (bytes(result) if result is not None else b"") == body
            assert (suspended > 0) == http2


@pytest.mark.asyncio
async def test_stream_read_ahead_yields_and_closes_waiting_readers():
    stalled = threading.Event()

    def serve(listener):
        for size in (64 << 20, None):
            try:
                conn, _ = listener.accept()
            except OSError:
                return
            with conn:
                head = b""
                while b"\r\n\r\n" not in head:
                    data = conn.recv(65536)
                    if not data:
                        return
                    head += data
                length = size or 1 << 20
                conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: %d\r\n\r\n" % length)
                try:
                    conn.sendall(b"x" * (size or 4096))
                except OSError:
                    pass
                if size is None:
                    stalled.wait(10)

    listener = socket.create_server(("127.0.0.1", 0))
    # Closing the listener does not wake a blocked accept after a failed test.
    listener.settimeout(10)
    url = f"http://127.0.0.1:{listener.getsockname()[1]}/"
    thread = threading.Thread(target=serve, args=(listener,), daemon=True)
    thread.start()
    try:
        async with wreq.Client(proxies=[]) as client:
            response = await client.get(url, read_timeout=timedelta(seconds=1))
            streamer = response.stream()
            # Reading starts on the first iteration, so this wait is not a read timeout.
            await asyncio.sleep(1.5)

            ticks = 0

            async def tick():
                nonlocal ticks
                while True:
                    ticks += 1
                    await asyncio.sleep(0)

            ticker = asyncio.create_task(tick())
            async with asyncio.timeout(5), streamer:
                frames = 0
                async for _ in streamer:
                    # A consumer slower than the network must still yield to the loop.
                    time.sleep(0.002)
                    frames += 1
                    if frames == 64:
                        break
            ticker.cancel()
            assert frames == 64 and ticks > 1

            response = await client.get(url)
            streamer = response.stream()
            assert len(await anext(streamer)) == 4096
            reader = asyncio.create_task(anext(streamer))
            await asyncio.sleep(0.1)
            assert not reader.done()
            # A synchronous exit must not wait for the pending read; it ends that read.
            streamer.__exit__(None, None, None)
            with pytest.raises(StopAsyncIteration):
                await asyncio.wait_for(reader, 5)
    finally:
        stalled.set()
        listener.close()
        thread.join(10)


@pytest.mark.asyncio
async def test_content_length_reports_the_received_length():
    heads = [
        (b"Content-Length: 5\r\n\r\nhello", 5),
        (b"Transfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n", None),
    ]
    async with local_server() as (url, connections):
        for head, length in heads:
            async with wreq.Client(proxies=[]) as client:
                task = asyncio.create_task(client.get(url))
                _, writer = await asyncio.wait_for(connections.get(), 5)
                writer.write(b"HTTP/1.1 200 OK\r\nConnection: close\r\n" + head)
                await writer.drain()
                response = await asyncio.wait_for(task, 5)
                async with response:
                    assert response.content_length == length
                    assert await response.bytes() == b"hello"
                    # The length describes the received body, not what is left to read.
                    assert response.content_length == length


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "head, frames",
    [
        (b"Content-Length: 5\r\n\r\nhello", [b"hello"]),
        (
            b"Transfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n5\r\nworld\r\n0\r\n\r\n",
            [b"hello", b"world"],
        ),
    ],
    ids=["length", "chunked"],
)
async def test_blocking_stream_survives_a_stalled_reader(head, frames):
    # A reader pausing past the read timeout after a frame must still see the body end.
    client = wreq.blocking.Client(proxies=[], read_timeout=timedelta(milliseconds=200))

    def read(url):
        with client.get(url) as response, response.stream() as streamer:
            chunks = [bytes(next(streamer))]
            time.sleep(0.5)
            return chunks + [bytes(chunk) for chunk in streamer]

    async with local_server() as (url, connections):
        task = asyncio.create_task(asyncio.to_thread(read, url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(b"HTTP/1.1 200 OK\r\n" + head)
        await writer.drain()
        assert await asyncio.wait_for(task, 5) == frames
