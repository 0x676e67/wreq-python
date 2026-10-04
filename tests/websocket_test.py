import asyncio
import base64
import hashlib
import struct
import threading

import pytest
import wreq
import wreq.blocking

GUID = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


async def read_frame(reader):
    """Read one masked client frame and return its opcode and payload."""
    first, size = await reader.readexactly(2)
    mask = await reader.readexactly(4)
    payload = await reader.readexactly(size & 0x7F)
    return first & 0x0F, bytes(b ^ mask[i % 4] for i, b in enumerate(payload))


async def read_close(reader):
    """Read one masked client close frame and return its code and reason."""
    opcode, payload = await read_frame(reader)
    assert opcode == 0x8
    if not payload:
        return None, ""
    return struct.unpack("!H", payload[:2])[0], payload[2:].decode()


async def handshake(reader, writer):
    head = await reader.readuntil(b"\r\n\r\n")
    key = next(
        line.split(b":", 1)[1].strip()
        for line in head.split(b"\r\n")
        if line.lower().startswith(b"sec-websocket-key:")
    )
    accept = base64.b64encode(hashlib.sha1(key + GUID).digest())
    writer.write(
        b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n"
        b"Connection: Upgrade\r\nSec-WebSocket-Accept: " + accept + b"\r\n\r\n"
    )


async def serve(reader, writer, frames):
    try:
        await handshake(reader, writer)
        frames.put_nowait(await read_close(reader))
        writer.write(b"\x88\x00")
        await writer.drain()
    finally:
        # A failed handshake must not leave `wait_closed` waiting.
        writer.close()


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "code, reason, expected",
    [
        (None, None, (None, "")),
        (4000, None, (4000, "")),
        (4000, "bye", (4000, "bye")),
        (None, "bye", (1000, "bye")),
    ],
)
async def test_websocket_close_frame(code, reason, expected):
    frames = asyncio.Queue()
    server = await asyncio.start_server(
        lambda r, w: serve(r, w, frames), "127.0.0.1", 0
    )
    url = f"ws://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    try:
        async with wreq.Client(proxies=[]) as client:
            # Leaving the block after an explicit close must not fail.
            async with client.websocket(url) as ws:
                await ws.close(code, reason)
            assert await asyncio.wait_for(frames.get(), 5) == expected
    finally:
        server.close()
        await server.wait_closed()


def test_blocking_websocket_exit_after_close():
    frames = asyncio.Queue()
    loop = asyncio.new_event_loop()
    server = loop.run_until_complete(
        asyncio.start_server(lambda r, w: serve(r, w, frames), "127.0.0.1", 0)
    )
    url = f"ws://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    thread = threading.Thread(target=loop.run_forever, daemon=True)
    thread.start()
    try:
        with wreq.blocking.Client(proxies=[]).websocket(url) as ws:
            ws.close(4001)
        frame = asyncio.run_coroutine_threadsafe(frames.get(), loop).result(5)
        assert frame == (4001, "")
    finally:
        loop.call_soon_threadsafe(loop.stop)
        thread.join(5)
        server.close()
        loop.run_until_complete(asyncio.wait_for(server.wait_closed(), 5))
        loop.close()


@pytest.mark.asyncio
async def test_websocket_reads_and_writes_do_not_block_each_other():
    received = asyncio.Queue()
    release = asyncio.Event()

    async def serve_messages(reader, writer):
        async def send_later():
            await release.wait()
            for text in (b"first", b"second"):
                writer.write(bytes([0x81, len(text)]) + text)
            await writer.drain()

        sender = None
        try:
            await handshake(reader, writer)
            sender = asyncio.ensure_future(send_later())
            while True:
                received.put_nowait(await read_frame(reader))
        except (asyncio.IncompleteReadError, ConnectionError):
            pass
        finally:
            if sender is not None:
                sender.cancel()
            writer.close()

    server = await asyncio.start_server(serve_messages, "127.0.0.1", 0)
    url = f"ws://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    try:
        async with wreq.Client(proxies=[]) as client:
            async with client.websocket(url) as ws:
                # A receive cancelled by its caller stops waiting and loses nothing.
                with pytest.raises(asyncio.TimeoutError):
                    await asyncio.wait_for(ws.recv(), 0.2)
                # A pending receive does not hold up a send.
                pending = asyncio.ensure_future(ws.recv())
                await asyncio.wait_for(ws.send(wreq.Message.from_text("ping")), 5)
                assert await asyncio.wait_for(received.get(), 5) == (0x1, b"ping")
                release.set()
                assert (await asyncio.wait_for(pending, 5)).text == "first"
                assert (await asyncio.wait_for(ws.recv(), 5)).text == "second"
    finally:
        release.set()
        server.close()
        await server.wait_closed()
