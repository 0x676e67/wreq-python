import asyncio
import gc
import weakref
from contextlib import asynccontextmanager

import pytest

import wreq


class Cancellation(asyncio.CancelledError):
    pass


def test_legacy_coroutine_throw():
    try:
        raise RuntimeError("traceback origin")
    except RuntimeError as error:
        origin = error.__traceback__

    def contains(traceback):
        while traceback is not None:
            if traceback is origin:
                return True
            traceback = traceback.tb_next
        return False

    def invoke(*args, **kwargs):
        coroutine = wreq.get("")
        try:
            coroutine.throw(*args, **kwargs)
        except BaseException as error:
            return error
        finally:
            coroutine.close()
        pytest.fail("throw did not raise")

    class ExceptionValue(ValueError):
        def with_traceback(self, *_):
            raise AssertionError("overridden method must not run")

    for instance_first in (False, True):
        for traceback in (None, origin):
            error = ExceptionValue("identity")
            BaseException.with_traceback(error, origin)
            args = (
                (error, None, traceback)
                if instance_first
                else (ExceptionValue, error, traceback)
            )
            caught = invoke(*args)
            assert caught is error
            assert caught.args == ("identity",)
            assert contains(caught.__traceback__) is (
                instance_first or traceback is not None
            )

    assert invoke(ValueError, ("tuple", 7)).args == ("tuple", 7)
    error = ValueError("keyword")
    assert invoke(exc=error) is error

    class PretendException:
        @property
        def __class__(self):
            return ValueError

    value = PretendException()
    assert invoke(ValueError, value).args == (value,)

    class ExceptionMeta(type):
        def __subclasscheck__(cls, subclass):
            return True

    class CustomException(Exception, metaclass=ExceptionMeta):
        pass

    assert type(invoke(CustomException, value)) is CustomException
    assert invoke(CustomException, value).args == (value,)

    class HiddenTraceback(RuntimeError):
        def __getattribute__(self, name):
            if name == "__traceback__":
                return None
            return super().__getattribute__(name)

    class BadConstructor(Exception):
        def __new__(cls):
            raise HiddenTraceback("constructor failure")

    class NotAnException(Exception):
        def __new__(cls):
            return PretendException()

    caught = invoke(BadConstructor, None, origin)
    assert type(caught) is HiddenTraceback
    assert caught.args == ("constructor failure",)
    assert not contains(BaseException.__traceback__.__get__(caught))
    # A failure without a traceback of its own takes the given one.
    for args in ((NotAnException, None), (UnicodeDecodeError, ("a",))):
        caught = invoke(*args, origin)
        assert type(caught) is TypeError
        assert contains(caught.__traceback__)

    unrelated = KeyError("unrelated")
    caught = invoke(ValueError, unrelated, origin)
    assert type(caught) is ValueError
    assert caught.args == (unrelated,)
    assert contains(caught.__traceback__)
    assert invoke(ValueError, value=("keyword",), traceback=origin).args == ("keyword",)

    class RefusingMeta(type):
        def __subclasscheck__(cls, subclass):
            raise ZeroDivisionError

    class Refusing(Exception, metaclass=RefusingMeta):
        pass

    assert type(invoke(Refusing, ValueError())) is ZeroDivisionError

    constructed = []

    class NotRaisable:
        def __init__(self):
            constructed.append(self)

    assert type(invoke(NotRaisable)) is TypeError
    assert not constructed

    coroutine = wreq.get("")

    class UsesCoroutine(Exception):
        def __init__(self):
            super().__init__(coroutine.__qualname__)

    try:
        for args in (
            (object(),),
            (ValueError, None, object()),
            (error, "value"),
            (ValueError,) * 4,
        ):
            with pytest.raises(TypeError):
                coroutine.throw(*args)
        # A rejected throw leaves the coroutine running, as a generator's does.
        with pytest.raises(Exception) as caught:
            coroutine.send(None)
        assert "cannot reuse" not in str(caught.value)
        # The exception is built before the throw borrows the coroutine.
        with pytest.raises(UsesCoroutine):
            coroutine.throw(UsesCoroutine)
        with pytest.raises(ValueError) as caught:
            coroutine.throw(error)
        assert caught.value is error
    finally:
        coroutine.close()


@asynccontextmanager
async def local_server():
    connections = asyncio.Queue()
    writers = []

    async def accept(reader, writer):
        # A connection accepted as teardown starts would otherwise keep
        # `wait_closed` waiting on Python 3.12.1+.
        if not server.is_serving():
            writer.close()
            return
        writers.append(writer)
        await reader.readuntil(b"\r\n\r\n")
        connections.put_nowait((reader, writer))

    server = await asyncio.start_server(accept, "127.0.0.1", 0)
    port = server.sockets[0].getsockname()[1]
    try:
        yield f"http://127.0.0.1:{port}/", connections
    finally:
        server.close()
        for writer in writers:
            writer.close()
        await asyncio.gather(*(writer.wait_closed() for writer in writers))
        await server.wait_closed()


def throw_cancellation(coroutine):
    """Throw a cancellation into `coroutine` and return a weak reference to it, so no
    caller frame still holds the exception."""
    error = Cancellation("cancelled after Rust completion")
    with pytest.raises(asyncio.CancelledError) as caught:
        coroutine.throw(error)
    assert caught.value is error
    return weakref.ref(error)


@pytest.mark.asyncio
@pytest.mark.parametrize("operation", ["request", "request_error", "stream"])
async def test_cancellation_after_rust_completion(operation):
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        response = None
        if operation.startswith("request"):
            coroutine = client.get(url)
            waiter = coroutine.send(None)
            _, writer = await asyncio.wait_for(connections.get(), 5)
        else:
            task = asyncio.create_task(client.get(url))
            _, writer = await asyncio.wait_for(connections.get(), 5)
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n")
            await writer.drain()
            response = await asyncio.wait_for(task, 5)
            coroutine = anext(response.stream())
            waiter = coroutine.send(None)

        try:
            # Complete the Rust work without resuming its Python coroutine.
            assert isinstance(waiter, asyncio.Future)
            if operation == "request_error":
                writer.write(b"invalid HTTP response\r\n\r\n")
                writer.close()
            elif operation == "request":
                writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
            else:
                writer.write(b"{}")
            done, _ = await asyncio.wait({waiter}, timeout=5)
            assert waiter in done, "Rust operation did not finish"

            error_ref = throw_cancellation(coroutine)
            # Keeping the finished coroutine alive must not retain its exception.
            # PyPy may need several passes to release it.
            for _ in range(10):
                if error_ref() is None:
                    break
                gc.collect()
            assert error_ref() is None
        finally:
            coroutine.close()
            if response is not None:
                await response.close()


@pytest.mark.asyncio
@pytest.mark.parametrize("action", ["cancel", "close_coroutine"])
async def test_pending_request_cancellation(action):
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        coroutine = client.get(url)
        if action == "close_coroutine":
            coroutine.send(None)
        else:
            task = asyncio.create_task(coroutine)
        reader, _ = await asyncio.wait_for(connections.get(), 5)

        if action == "close_coroutine":
            coroutine.close()
        else:
            task.cancel("caller cancellation message")
            done, _ = await asyncio.wait({task}, timeout=5)
            assert task in done, "Cancellation did not finish"
            with pytest.raises(asyncio.CancelledError) as caught:
                await task
            assert caught.value.args == ("caller cancellation message",)

        # The cancelled operation must release its pending network request.
        assert await asyncio.wait_for(reader.read(), 5) == b""


@pytest.mark.asyncio
@pytest.mark.parametrize("action", ["cancel", "close_coroutine"])
async def test_pending_stream_cancellation(action):
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.get(url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n")
        await writer.drain()
        response = await asyncio.wait_for(task, 5)
        stream = response.stream()
        coroutine = anext(stream)

        if action == "close_coroutine":
            assert isinstance(coroutine.send(None), asyncio.Future)
            coroutine.close()
        else:
            started = asyncio.Event()

            async def read():
                started.set()
                return await coroutine

            task = asyncio.create_task(read())
            await started.wait()
            task.cancel("cancel stream read")
            with pytest.raises(asyncio.CancelledError, match="cancel stream read"):
                await asyncio.wait_for(task, 5)

        # Closing the stream must acquire the lock held by the pending read.
        # Do not send a body: that would let a leaked read release it naturally.
        await asyncio.wait_for(stream.__aexit__(None, None, None), 5)
        with pytest.raises(StopAsyncIteration):
            await anext(stream)
        await response.close()


@pytest.mark.asyncio
async def test_stream_coroutine_iteration():
    async with local_server() as (url, connections), wreq.Client(proxies=[]) as client:
        task = asyncio.create_task(client.get(url))
        _, writer = await asyncio.wait_for(connections.get(), 5)
        writer.write(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n"
            b"Trailer: x-check\r\n\r\n1\r\na\r\n"
        )
        await writer.drain()
        response = await asyncio.wait_for(task, 5)
        async with response.stream() as stream:
            deferred = stream.__anext__()
            assert asyncio.iscoroutine(deferred)
            assert not isinstance(deferred, asyncio.Future)
            assert deferred.__qualname__ == "Streamer.__anext__"
            assert not hasattr(stream, "_anext")
            try:
                # An unawaited __anext__ must not consume the first frame.
                assert await asyncio.wait_for(anext(stream), 5) == b"a"
                writer.write(b"1\r\nb\r\n0\r\nx-check: done\r\n\r\n")
                await writer.drain()
                assert await asyncio.wait_for(deferred, 5) == b"b"
                with pytest.raises(
                    RuntimeError, match="cannot reuse already awaited coroutine"
                ):
                    await deferred
            finally:
                deferred.close()

            async with asyncio.timeout(5):
                frames = [frame async for frame in stream]
            assert len(frames) == 1
            assert isinstance(frames[0], wreq.HeaderMap)
            assert frames[0]["x-check"] == b"done"
            with pytest.raises(StopAsyncIteration):
                await stream.__anext__()
            assert await anext(stream, None) is None
        await response.close()
