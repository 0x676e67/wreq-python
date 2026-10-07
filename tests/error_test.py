import asyncio
import builtins
import pickle
import socket
from datetime import timedelta

import pytest
import wreq
import wreq.exceptions as exceptions

from cancellation_test import local_server


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_proxy_connection_error():
    invalid_proxies = [
        "http://invalid.proxy:8080",
        "https://invalid.proxy:8080",
        "socks4://invalid.proxy:8080",
        "socks4a://invalid.proxy:8080",
        "socks5://invalid.proxy:8080",
        "socks5h://invalid.proxy:8080",
    ]
    target_urls = ["https://example.com", "http://example.com"]
    for proxy in invalid_proxies:
        for url in target_urls:
            with pytest.raises(exceptions.ProxyConnectionError):
                await wreq.get(url, proxy=wreq.Proxy.all(proxy))


@pytest.mark.asyncio
async def test_errors_share_one_hierarchy():
    # Every exception is exported from the package root and derives from `Error`.
    for name in exceptions.__all__:
        assert getattr(wreq, name) is getattr(exceptions, name)
        assert issubclass(getattr(exceptions, name), wreq.Error)

    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        refused = f"http://127.0.0.1:{sock.getsockname()[1]}/"

    async with wreq.Client(proxies=[]) as client:
        # A refused connection is also a builtin `ConnectionError`, and pickles.
        with pytest.raises(wreq.ConnectionError) as caught:
            await client.get(refused)
        assert isinstance(caught.value, wreq.RequestError)
        assert isinstance(caught.value, builtins.ConnectionError)
        copy = pickle.loads(pickle.dumps(caught.value))
        assert type(copy) is type(caught.value) and str(copy) == str(caught.value)

        # Invalid input fails as a `BuilderError`.
        with pytest.raises(wreq.BuilderError, match="Invalid header name"):
            await client.get(refused, headers={"bad name": "v"})

        # A timeout while reading the body is a `TimeoutError`, not a `BodyError`.
        async with local_server() as (url, connections):
            task = asyncio.create_task(
                client.get(url, read_timeout=timedelta(seconds=0.2))
            )
            _, writer = await asyncio.wait_for(connections.get(), 5)
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nhello")
            response = await asyncio.wait_for(task, 5)
            with pytest.raises(wreq.TimeoutError) as caught:
                await asyncio.wait_for(response.bytes(), 5)
            assert isinstance(caught.value, builtins.TimeoutError)
