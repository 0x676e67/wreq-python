# Proxies

Set `Client(proxies=[...])` to route a client's requests through a proxy, or
pass `proxy=...` to choose a proxy for one request. These options work with
both the asynchronous and [blocking API](blocking.md).

| Constructor | Requests it handles |
| --- | --- |
| `Proxy.all(url)` | HTTP and HTTPS destinations |
| `Proxy.http(url)` | HTTP destinations |
| `Proxy.https(url)` | HTTPS destinations |
| `Proxy.unix(path)` | A local service on a Unix socket |

`http` and `https` select the destination scheme. The proxy URL separately
specifies how to connect to the proxy: for example,
`Proxy.https("http://proxy.example.com:8080")` tunnels HTTPS requests through
an HTTP proxy.

## Configuring a client

Replace the example address with a running proxy before executing this example.

```python
import asyncio
from wreq import Client, Proxy


async def main():
    async with Client(
        proxies=[Proxy.all("http://proxy.example.com:8080")]
    ) as client:
        async with client.get("https://httpbin.org/ip") as response:
            print(await response.json())


asyncio.run(main())
```

By default, a client without `proxies` uses the system proxy: the
`HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` and `NO_PROXY` environment variables
(and their lowercase forms), plus the operating system proxy settings on macOS
and Windows.

For direct connections that ignore environment and system proxy settings, use
`Client(no_proxy=True)`. This also clears proxies configured on that client.
To exclude hosts from a particular proxy, pass an exclusion list when creating
it:

```python
from wreq import Proxy

proxy = Proxy.all(
    "http://proxy.example.com:8080",
    exclusion="localhost,127.0.0.1,internal.example.com",
)
```

## Proxy authentication

Supply both `username` and `password`, or include them in the proxy URL.

```python
from wreq import Proxy

proxy = Proxy.all(
    "http://proxy.example.com:8080",
    username="username",
    password="password",
)
```

For a proxy that expects another HTTP authorization scheme, use
`custom_http_auth="Bearer example-token"`. These options authenticate to the
proxy; [request authentication](auth.md) authenticates to the destination.

## SOCKS proxies

Use `socks5://` for local DNS resolution or `socks5h://` to resolve destination
hostnames at the proxy. SOCKS4 and SOCKS4a URLs are also supported.

```python
from wreq import Proxy

proxy = Proxy.all("socks5h://username:password@127.0.0.1:1080")
```

Pass this object to `Client(proxies=[proxy])` or a request's `proxy` argument.

## Selecting a proxy for one request

Use the singular `proxy` argument on requests. `custom_http_headers` configures
headers for the HTTP proxy connection, such as headers required by a provider
on a CONNECT request. Use the request's `headers` argument for destination
headers.

```python
import asyncio
from wreq import Client, Proxy


async def main():
    async with Client() as client:
        async with client.get(
            "https://httpbin.org/ip",
            proxy=Proxy.all(
                "http://proxy.example.com:8080",
                custom_http_headers={"X-Proxy-Region": "us"},
            ),
        ) as response:
            print(await response.json())


asyncio.run(main())
```

## Unix sockets

On Unix platforms, `Proxy.unix` connects to a local socket. The example below
requires access to a running Docker daemon at `/var/run/docker.sock`. Change
the path and endpoint for another service. This constructor is not supported
on Windows.

```python
import asyncio
from wreq import Client, Proxy


async def main():
    async with Client() as client:
        async with client.get(
            "http://localhost/containers/json",
            proxy=Proxy.unix("/var/run/docker.sock"),
        ) as response:
            print(await response.json())


asyncio.run(main())
```

The URL supplies the HTTP request path and host; the connection uses the socket
instead of a TCP port.
