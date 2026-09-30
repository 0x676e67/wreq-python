import asyncio
import contextvars
import threading

import pytest
import wreq

from cancellation_test import local_server


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
async def test_async_upload(multipart):
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
