import asyncio
import threading
from contextlib import asynccontextmanager

import pytest
import wreq


@asynccontextmanager
async def upload_server():
    bodies = asyncio.Queue()
    handlers = set()
    writers = []
    errors = []

    async def accept(reader, writer):
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


@pytest.mark.asyncio
@pytest.mark.parametrize("blocking", [False, True])
@pytest.mark.parametrize("multipart", [False, True])
async def test_sync_upload_iterator_errors(blocking, multipart):
    factory = wreq.blocking.Client if blocking else wreq.Client
    client = factory(proxies=[])
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
async def test_blocking_upload_iterator_can_send_requests():
    # Past the read-ahead, the waiting caller pulls the iterator, which may itself block on requests.
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
