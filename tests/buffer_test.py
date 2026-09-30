import asyncio
import datetime
import gc
import threading
import weakref
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

import wreq
from wreq import Message, Multipart, Part, Version, blocking
from wreq.header import HeaderMap, OrigHeaderMap


def assert_readonly_view(view, expected):
    assert type(view) is memoryview
    assert view.readonly
    assert view.format == "B"
    assert view.ndim == 1
    assert view.itemsize == 1
    assert view.shape == (len(expected),)
    assert view == expected


def test_header_and_message_views():
    headers = HeaderMap({"X-Buffer": "value"})
    headers.append("X-Buffer", "other")
    original = OrigHeaderMap(["X-Buffer"])
    messages = [
        Message.from_text("text"),
        Message.from_binary(b"binary"),
        Message.from_ping(b"ping"),
        Message.from_pong(b"pong"),
        Message.from_binary(b""),
    ]
    assert messages[0].text == "text"
    assert type(messages[0].text) is str
    round_trip = Message.from_binary(bytes(messages[1].binary))
    assert round_trip.binary == b"binary"
    assert headers.get("missing") is None
    assert headers.get("missing", b"default") == b"default"

    views = [
        headers.get("X-Buffer"),
        headers["X-Buffer"],
        headers.get("missing", b"default"),
        *headers.get_all("X-Buffer"),
        *headers.keys(),
        *headers.values(),
        *(view for pair in headers for view in pair),
        *(view for pair in original for view in pair),
        *(message.data for message in messages),
        messages[1].binary,
        messages[2].ping,
        messages[3].pong,
        round_trip.binary,
    ]
    expected = [bytes(view) for view in views]
    for view, data in zip(views, expected):
        assert_readonly_view(view, data)
        assert hash(view) == hash(data)
        assert {view: "value"}[data] == "value"

    with pytest.raises(TypeError):
        views[0][0] = 0

    headers.clear()
    del headers, original, messages, round_trip
    gc.collect()
    for view, data in zip(views, expected):
        assert view == data

    parent = views[0]
    child = parent[1:]
    parent.release()
    assert child == b"alue"
    child.release()


def test_subclass_input_cycles_are_collected():
    class Text(str):
        def __str__(self):
            raise AssertionError("subclass conversion must not be called")

    class Binary(bytes):
        def __bytes__(self):
            raise AssertionError("subclass conversion must not be called")

    class Marker:
        pass

    def collect_garbage():
        # PyPy may need several GC cycles to finalize C-extension buffers.
        for _ in range(3):
            gc.collect()

    def header_value(source, method):
        headers = HeaderMap()
        getattr(headers, method)("X-Buffer", source)
        return headers["X-Buffer"]

    def original_name(source):
        headers = OrigHeaderMap()
        headers.insert(source)
        return next(iter(headers))[1]

    cases = [
        (Text, "payload", lambda source: Message.from_text(source).data),
        (Binary, b"payload", lambda source: Message.from_binary(source).binary),
        (Binary, b"payload", lambda source: Message.from_ping(source).ping),
        (Binary, b"payload", lambda source: Message.from_pong(source).pong),
        (Text, "payload", lambda source: HeaderMap({"X-Buffer": source})["X-Buffer"]),
        (Text, "payload", lambda source: header_value(source, "insert")),
        (Text, "payload", lambda source: header_value(source, "append")),
        (Text, "payload", lambda source: header_value(source, "__setitem__")),
        (Binary, b"payload", lambda source: HeaderMap().get("missing", source)),
        (Text, "X-Buffer", lambda source: next(iter(OrigHeaderMap([source])))[1]),
        (Text, "X-Buffer", original_name),
    ]
    for input_type, payload, make_view in cases:
        source = input_type(payload)
        source.marker = Marker()
        marker = weakref.ref(source.marker)
        view = make_view(source)
        expected = payload.encode() if isinstance(payload, str) else payload
        assert_readonly_view(view, expected)
        source.view = view
        del source, view
        collect_garbage()
        assert marker() is None, make_view

    for input_type, payload, make_owner in [
        (Text, "payload", lambda source: Part("field", source)),
        (Binary, b"payload", lambda source: Multipart(Part("field", source))),
        (Text, "payload", lambda source: Message.from_close(1000, source)),
    ]:
        source = input_type(payload)
        source.marker = Marker()
        marker = weakref.ref(source.marker)
        owner = make_owner(source)
        source.owner = owner
        del source, owner
        collect_garbage()
        assert marker() is None, make_owner


@pytest.fixture
def buffer_http_server():
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def setup(self):
            super().setup()
            self.connection.settimeout(3)

        def do_GET(self):
            self.send_response(200)
            self.send_header("Connection", "close")
            if self.path == "/stream":
                self.send_header("Transfer-Encoding", "chunked")
                self.send_header("Trailer", "X-Buffer")
                self.end_headers()
                self.wfile.write(
                    b"5\r\nhello\r\n6\r\n world\r\n0\r\nX-Buffer: complete\r\n\r\n"
                )
            else:
                self.send_header("Content-Length", "11")
                self.end_headers()
                self.wfile.write(b"hello world")

        def log_message(self, *_):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(
        target=server.serve_forever, kwargs={"poll_interval": 0.05}, daemon=True
    )
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
        assert not thread.is_alive()


@pytest.mark.asyncio
@pytest.mark.parametrize("blocking_api", [False, True])
async def test_response_and_stream_views(buffer_http_server, blocking_api):
    client_type = blocking.Client if blocking_api else wreq.Client
    client = client_type(no_proxy=True, timeout=datetime.timedelta(seconds=3))

    async def call(method, *args, **kwargs):
        if blocking_api:
            return await asyncio.wait_for(
                asyncio.to_thread(method, *args, **kwargs), timeout=5
            )
        return await asyncio.wait_for(method(*args, **kwargs), timeout=5)

    def collect_blocking(response):
        with response.stream() as stream:
            return list(stream)

    async def collect_async(response):
        async with response.stream() as stream:
            return [frame async for frame in stream]

    try:
        response = await call(
            client.get, f"{buffer_http_server}/body", version=Version.HTTP_11
        )
        view = await call(response.bytes)
        repeated = await call(response.bytes)
        assert_readonly_view(view, b"hello world")
        assert_readonly_view(repeated, b"hello world")
        repeated.release()
        child = view[6:]
        await call(response.close)
        del response, repeated
        gc.collect()
        assert view == b"hello world"
        view.release()
        assert child == b"world"

        response = await call(
            client.get, f"{buffer_http_server}/stream", version=Version.HTTP_11
        )
        if blocking_api:
            frames = await asyncio.wait_for(
                asyncio.to_thread(collect_blocking, response), timeout=5
            )
        else:
            frames = await asyncio.wait_for(collect_async(response), timeout=5)
        await call(response.close)
        del response
        gc.collect()
        data = [frame for frame in frames if isinstance(frame, memoryview)]
        trailers = [frame for frame in frames if isinstance(frame, HeaderMap)]
        assert data
        assert b"".join(data) == b"hello world"
        assert len(trailers) == 1
        assert_readonly_view(trailers[0]["X-Buffer"], b"complete")
        for frame in data:
            expected = bytes(frame)
            assert_readonly_view(frame, expected)
            child = frame[:]
            frame.release()
            assert child == expected
    finally:
        client.close()
