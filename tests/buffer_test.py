import array
import asyncio
import collections.abc
import copy
import datetime
import gc
import operator
import pickle
import sys
import threading
import weakref
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

import wreq
from wreq import Message, Multipart, Part, Version, blocking
from wreq.header import HeaderMap, OrigHeaderMap


def assert_shared_bytes(data, expected):
    assert type(data) is wreq.Bytes
    view = memoryview(data)
    assert view.readonly
    assert view.format == "B"
    assert view.ndim == 1
    assert view.obj is data
    assert data == expected
    assert expected == data
    assert hash(data) == hash(expected)
    assert {expected: "value"}[data] == "value"


def test_header_and_message_values():
    headers = HeaderMap({"X-Buffer": "value"})
    headers.append("X-Buffer", "other")
    original = OrigHeaderMap(["X-Buffer"])

    # Short values are plain bytes copies.
    shorts = [
        headers.get("X-Buffer"),
        headers["X-Buffer"],
        headers.get("missing", b"default"),
        headers.get("missing", wreq.Bytes(b"default")),
        *headers.get_all("X-Buffer"),
        *headers.keys(),
        *headers.values(),
        *(value for pair in headers for value in pair),
        *(value for pair in original for value in pair),
        Message.from_ping(b"ping").ping,
        Message.from_pong(b"pong").pong,
    ]
    assert all(type(value) is bytes for value in shorts)
    assert shorts[:4] == [b"value", b"value", b"default", b"default"]
    assert shorts[-2:] == [b"ping", b"pong"]
    assert headers.get("missing") is None

    # Payloads share their data as wreq.Bytes.
    messages = [
        Message.from_text("text"),
        Message.from_binary(b"binary"),
        Message.from_ping(b"ping"),
        Message.from_pong(b"pong"),
        Message.from_binary(b""),
    ]
    assert messages[0].text == "text"
    assert type(messages[0].text) is str
    payloads = [*(message.data for message in messages), messages[1].binary]
    expected = [b"text", b"binary", b"ping", b"pong", b"", b"binary"]
    for data, value in zip(payloads, expected):
        assert_shared_bytes(data, value)
    assert Message.from_binary(payloads[1]).binary == b"binary"

    with pytest.raises(TypeError):
        payloads[1][0] = 0

    del headers, original, messages
    gc.collect()
    for data, value in zip(payloads, expected):
        assert data == value

    parent = payloads.pop(1)
    child = parent[1:]
    assert type(child) is wreq.Bytes
    with pytest.warns(DeprecationWarning):
        parent.release()
    del parent
    gc.collect()
    assert child == b"inary"


class Index:
    def __init__(self, value):
        self.value = value

    def __index__(self):
        if isinstance(self.value, Exception):
            raise self.value
        return self.value


class IndexFailure(Exception):
    pass


def outcome(method, args):
    try:
        return "ok", method(*args)
    except Exception as err:
        return "error", type(err)


def test_bytes_matches_bytes():
    raw = b"hello world"
    data = Message.from_binary(raw).binary
    huge = 2**70
    failing = Index(IndexFailure())
    cases = [
        *(
            ("__getitem__", key)
            for key in (0, -1, 11, -12, huge, Index(1), failing, 1.0, "0")
        ),
        *(
            ("__getitem__", slice(*args))
            for args in [
                (6, None),
                (None, None, 2),
                (None, None, -1),
                (8, 2, -1),
                (2, 8, -1),
                (None, None, -3),
                (huge, None),
            ]
        ),
        *(
            ("__contains__", item)
            for item in (b"wor", 104, b"xyz", 256, -1, huge, "w")
        ),
        *(
            ("__contains__", item)
            for item in (bytearray(b"lo"), memoryview(b"lo"), array.array("B", b"lo"))
        ),
        ("find", b"o"),
        ("find", b"o", 5),
        ("find", 111, 5, 7),
        ("find", b"", 11),
        ("find", b"", 12),
        ("find", b"", 2, 1),
        ("find", b"l", -huge, huge),
        ("find", b"l", Index(3)),
        ("find", b"l", failing),
        ("find", b"o", 1.0),
        ("find", "x", 12),
        ("find", 256, 20),
        ("find", array.array("B", b"wor")),
        ("index", b"wor"),
        ("index", b"z"),
        ("index", b"o", 5, 6),
        ("count", b"l"),
        ("count", 108),
        ("count", b""),
        ("count", b"", 5, 4),
        ("count", b"", 12),
        ("count", b"l", 3, huge),
        ("count", "l"),
        ("startswith", b"hello"),
        ("startswith", (b"x", b"he")),
        ("startswith", b"world", 6),
        ("startswith", b"", 11),
        ("startswith", b"", 12),
        ("startswith", b"", 2, 1),
        ("startswith", b"h", -huge),
        ("startswith", memoryview(b"he")),
        ("startswith", "x"),
        ("startswith", ("x",)),
        ("startswith", "x", 20),
        ("endswith", b"hell", 0, 4),
        ("endswith", b"", 5, 4),
        ("endswith", array.array("B", b"ld")),
        ("hex",),
        ("hex", ":", 2),
        ("decode",),
        ("decode", "latin-1"),
        *(
            ("__add__", tail)
            for tail in (b"!", b"", bytearray(b"!"), array.array("B", b"!"))
        ),
    ]
    for method, *args in cases:
        expected = outcome(getattr(raw, method), args)
        assert outcome(getattr(data, method), args) == expected, (method, args)

    # wreq follows CPython, whose bytes takes `__index__` needles; PyPy's rejects them.
    assert Index(104) in data
    assert data.find(Index(119)) == 6
    for result in (data[6:], data[::2], data[::-1], data + b"", data + b"!"):
        assert type(result) is wreq.Bytes
    assert b">" + data == b">hello world"
    with pytest.raises(TypeError):
        data + 1
    assert wreq.Bytes(b"aaaa").count(b"aa") == 2
    assert wreq.Bytes(b"\xffab").decode("utf-8", "replace") == "\ufffdab"
    with pytest.raises(UnicodeDecodeError):
        wreq.Bytes(b"\xff").decode()


def test_bytes_api():
    data = Message.from_binary(b"hello world").binary
    assert wreq.Bytes(data) is data
    assert wreq.Bytes() == b""
    assert wreq.Bytes(bytearray(b"ab")) == b"ab"
    numbers = array.array("d", [1.5])
    assert wreq.Bytes(numbers) == bytes(numbers)
    for invalid in (5, "text", [1]):
        with pytest.raises(TypeError):
            wreq.Bytes(invalid)

    assert len(data) == 11
    assert list(data[:3]) == [104, 101, 108]
    assert list(reversed(data[:3])) == [108, 101, 104]
    items = iter(data)
    next(items)
    assert operator.length_hint(items) == 10
    assert isinstance(data, collections.abc.Sequence)
    if hasattr(collections.abc, "Buffer"):
        assert isinstance(data, collections.abc.Buffer)
    assert sorted([b"z", data, b"a"]) == [b"a", b"hello world", b"z"]
    # Other types reflect, so bytearray and memoryview compare by content.
    assert data == bytearray(b"hello world") and data == memoryview(b"hello world")
    assert data != bytearray(b"x") and data < bytearray(b"z")
    assert data != "hello world"
    assert hash(wreq.Bytes()) == hash(b"")
    assert type(data.tobytes()) is bytes
    assert data.tobytes() == bytes(data) == b"hello world"
    assert repr(data) == "Bytes(b'hello world')"

    assert copy.copy(data) is data
    assert copy.deepcopy(data) is data
    restored = pickle.loads(pickle.dumps(data))
    assert type(restored) is wreq.Bytes
    assert restored == data
    assert weakref.ref(data)() is data
    with pytest.warns(DeprecationWarning):
        with data as same:
            assert same is data


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
    # PyPy's cpyext keeps the source of any memoryview passed to an extension alive.
    if sys.implementation.name == "cpython":
        cases.append(
            (
                Binary,
                b"payload",
                lambda source: Message.from_binary(memoryview(source)).binary,
            )
        )
    for input_type, payload, make_view in cases:
        source = input_type(payload)
        source.marker = Marker()
        marker = weakref.ref(source.marker)
        view = make_view(source)
        expected = payload.encode() if isinstance(payload, str) else payload
        assert view == expected
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

        def do_POST(self):
            if self.headers.get("Transfer-Encoding") == "chunked":
                body = b""
                while size := int(self.rfile.readline(), 16):
                    body += self.rfile.read(size)
                    self.rfile.readline()
                self.rfile.readline()
            else:
                body = self.rfile.read(int(self.headers["Content-Length"]))
            self.send_response(200)
            self.send_header("Connection", "close")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

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


def test_bytes_like_inputs(buffer_http_server):
    sources = [
        Message.from_binary(b"<payload>").binary[1:-1],
        memoryview(b"<payload>")[1:-1],
        memoryview(bytearray(b"payload")),
        memoryview(b"ppaayyllooaadd")[::2],
        memoryview(array.array("B", b"payload")),
        wreq.Bytes(b"payload"),
    ]
    client = blocking.Client(no_proxy=True, timeout=datetime.timedelta(seconds=3))
    try:
        # Other exporters keep their JSON meaning unless wrapped in a memoryview.
        numbers = array.array("d", [1.5])
        assert Message.from_binary(numbers).binary == b"[1.5]"
        assert client.post(buffer_http_server, body=numbers).bytes() == b"[1.5]"
        assert Message.from_binary({"a": 1}).binary == b'{"a":1}'
        message = r"'dict' object is not an instance of 'bytes \| bytearray \| memoryview \| Bytes'"
        with pytest.raises(TypeError, match=message):
            Message.from_ping({})
        for source in sources:
            assert Message.from_binary(source).binary == b"payload"
            assert Message.from_ping(source).ping == b"payload"
            assert Message.from_pong(source).pong == b"payload"
            assert HeaderMap().get("missing", source) == b"payload"
            for body in (source, iter([source])):
                response = client.post(buffer_http_server, body=body)
                assert response.bytes() == b"payload", source
            form = Multipart(Part("field", source))
            response = client.post(buffer_http_server, multipart=form)
            assert b"\r\n\r\npayload\r\n" in bytes(response.bytes()), source
    finally:
        client.close()


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
        data = await call(response.bytes)
        repeated = await call(response.bytes)
        assert_shared_bytes(data, b"hello world")
        assert_shared_bytes(repeated, b"hello world")
        child = data[6:]
        await call(response.close)
        del response, repeated
        gc.collect()
        assert data == b"hello world"
        del data
        gc.collect()
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
        data = [frame for frame in frames if not isinstance(frame, HeaderMap)]
        trailers = [frame for frame in frames if isinstance(frame, HeaderMap)]
        assert data
        assert b"".join(data) == b"hello world"
        assert len(trailers) == 1
        assert trailers[0]["X-Buffer"] == b"complete"
        for frame in data:
            assert_shared_bytes(frame, frame.tobytes())
    finally:
        client.close()
