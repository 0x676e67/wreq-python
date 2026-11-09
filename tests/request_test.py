import asyncio

import pytest
import wreq
from wreq import Version
from wreq.header import HeaderMap

client = wreq.Client(tls_info=True)


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_gzip():
    url = "http://localhost:8080/gzip"
    resp = await client.get(url)
    async with resp:
        text = await resp.text()
        assert text is not None
        assert "gzipped" in text


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_deflate():
    url = "http://localhost:8080/deflate"
    resp = await client.get(url)
    async with resp:
        text = await resp.text()
        assert text is not None
        assert "deflated" in text


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_brotli():
    url = "http://localhost:8080/brotli"
    resp = await client.get(url)
    async with resp:
        text = await resp.text()
        assert text is not None
        assert "brotli" in text


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_auth():
    resp = await client.get(
        "http://localhost:8080/anything",
        auth="token",
    )
    async with resp:
        json = await resp.json()
        authorization = json["headers"]["Authorization"]
        assert authorization == "token"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_bearer_auth():
    resp = await client.get(
        "http://localhost:8080/anything",
        bearer_auth="token",
    )
    async with resp:
        json = await resp.json()
        authorization = json["headers"]["Authorization"]
        assert authorization == "Bearer token"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_basic_auth():
    resp = await client.get(
        "http://localhost:8080/anything",
        basic_auth=("user", "pass"),
    )
    async with resp:
        json = await resp.json()
        authorization = json["headers"]["Authorization"]
        assert authorization == "Basic dXNlcjpwYXNz"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_with_version():
    url = "https://www.google.com/anything"
    resp = await client.get(url, version=Version.HTTP_11)
    async with resp:
        assert resp.version == Version.HTTP_11

    resp = await client.get(url, version=Version.HTTP_2)
    async with resp:
        assert resp.version == Version.HTTP_2


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_headers():
    url = "http://localhost:8080/headers"
    headers = {"foo": "bar"}
    resp = await client.get(url, headers=headers)
    async with resp:
        json = await resp.json()
        assert json["headers"]["Foo"] == "bar"

    resp = await client.get(url, headers=HeaderMap(headers))
    async with resp:
        json = await resp.json()
        assert json["headers"]["Foo"] == "bar"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_disable_default_headers():
    url = "http://localhost:8080/headers"
    headers = {"foo": "bar"}
    client = wreq.Client(headers=headers)
    resp = await client.get(url)
    async with resp:
        json = await resp.json()
        assert json["headers"]["Foo"] == "bar"

    resp = await client.get(url, default_headers=False)
    async with resp:
        json = await resp.json()
        assert "Foo" not in json["headers"]


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_cookies():
    url = "http://localhost:8080/cookies"
    resp = await client.get(url, cookies={"foo": "bar"})
    async with resp:
        json = await resp.json()
        assert json["cookies"] == {"foo": "bar"}


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_form():
    url = "http://localhost:8080/post"
    resp = await client.post(url, form=[("foo", "bar")])
    async with resp:
        json = await resp.json()
        assert json["form"] == {"foo": "bar"}


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_json():
    url = "http://localhost:8080/post"
    resp = await client.post(url, json={"foo": "bar"})
    async with resp:
        json = await resp.json()
        assert json["json"] == {"foo": "bar"}


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_text():
    url = "http://localhost:8080/post"
    resp = await client.post(url, body="hello")
    async with resp:
        json = await resp.json()
        assert json["data"] == "hello"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_bytes():
    url = "http://localhost:8080/post"
    resp = await client.post(url, body=b"hello")
    async with resp:
        json = await resp.json()
        assert json["data"] == "hello"


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_async_bytes_stream():
    async def file_bytes_stream():
        with open("README.md", "rb") as f:
            while True:
                chunk = f.read(1024)
                if not chunk:
                    break
                yield chunk

    url = "http://localhost:8080/post"
    resp = await client.post(url, body=file_bytes_stream())
    async with resp:
        json = await resp.json()
        assert json["data"] in open("README.md").read()


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_send_sync_bytes_stream():
    def file_to_bytes_stream(file_path):
        with open(file_path, "rb") as f:
            while chunk := f.read(1024):
                yield chunk

    url = "http://localhost:8080/post"
    resp = await client.post(url, body=file_to_bytes_stream("README.md"))
    async with resp:
        json = await resp.json()
        assert json["data"] in open("README.md").read()


@pytest.mark.asyncio
async def test_large_ints_are_not_sent_as_floats():
    requests = asyncio.Queue()

    async def handle(reader, writer):
        head = await reader.readuntil(b"\r\n\r\n")
        length = next(
            (
                int(line.split(b":")[1])
                for line in head.split(b"\r\n")
                if line.lower().startswith(b"content-length:")
            ),
            0,
        )
        requests.put_nowait(
            head.split(b" ")[1] + b" " + await reader.readexactly(length)
        )
        writer.write(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        await writer.drain()
        writer.close()

    server = await asyncio.start_server(handle, "127.0.0.1", 0)
    url = f"http://127.0.0.1:{server.sockets[0].getsockname()[1]}/"
    try:
        await (await client.post(url, json={"a": 2**63, "b": -(2**63)})).close()
        await (await client.get(url, query={"a": 2**64 - 1})).close()
        sent = [await asyncio.wait_for(requests.get(), 5) for _ in range(2)]
    finally:
        server.close()
        await server.wait_closed()

    assert sent == [
        b'/ {"a":9223372036854775808,"b":-9223372036854775808}',
        b"/?a=18446744073709551615 ",
    ]
