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
        # A refused connection is also a builtin `ConnectionError`. Its URL stays out
        # of the message, and the predicates and URL survive pickling.
        with pytest.raises(wreq.ConnectionError) as caught:
            await client.get(refused)
        error = caught.value
        assert isinstance(error, wreq.RequestError)
        assert isinstance(error, builtins.ConnectionError)
        assert error.is_connect() and error.is_request() and not error.is_timeout()
        assert error.url == refused and refused not in str(error)
        copy = pickle.loads(pickle.dumps(error))
        assert type(copy) is type(error) and str(copy) == str(error)
        assert copy.is_connect() and copy.url == refused

        # An error the binding raises matches the predicate of its class.
        with pytest.raises(wreq.BuilderError, match="Invalid header name") as caught:
            await client.get(refused, headers={"bad name": "v"})
        assert caught.value.is_builder() and caught.value.url is None

        async with local_server() as (url, connections):
            # A timeout while reading the body is a `TimeoutError` that is also
            # `is_body()`.
            task = asyncio.create_task(
                client.get(url, read_timeout=timedelta(seconds=0.2))
            )
            _, writer = await asyncio.wait_for(connections.get(), 5)
            writer.write(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nhello")
            response = await asyncio.wait_for(task, 5)
            with pytest.raises(wreq.TimeoutError) as caught:
                await asyncio.wait_for(response.bytes(), 5)
            assert isinstance(caught.value, builtins.TimeoutError)
            assert caught.value.is_timeout() and caught.value.is_body()

            # A `StatusError` carries the response status, which pickles too.
            task = asyncio.create_task(client.get(url))
            _, writer = await asyncio.wait_for(connections.get(), 5)
            writer.write(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
            response = await asyncio.wait_for(task, 5)
            with pytest.raises(wreq.StatusError) as caught:
                response.raise_for_status()
            copy = pickle.loads(pickle.dumps(caught.value))
            assert copy.status == 404 and copy.status.is_client_error()
            assert copy.is_status() and copy.url == url
