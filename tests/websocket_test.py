import asyncio
import base64
import hashlib
import struct
import threading

import pytest
import wreq
import wreq.blocking

GUID = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"


async def read_close(reader):
    """Read one masked client frame and return its close code and reason."""
    opcode, size = await reader.readexactly(2)
    assert opcode == 0x88
    mask = await reader.readexactly(4)
    payload = bytes(
        b ^ mask[i % 4] for i, b in enumerate(await reader.readexactly(size & 0x7F))
    )
    if not payload:
        return None, ""
    return struct.unpack("!H", payload[:2])[0], payload[2:].decode()


async def serve(reader, writer, frames):
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
    frames.put_nowait(await read_close(reader))
    writer.write(b"\x88\x00")
    await writer.drain()
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
        loop.run_until_complete(server.wait_closed())
        loop.close()
