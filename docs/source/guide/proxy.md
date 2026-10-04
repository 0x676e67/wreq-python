# :globe_with_meridians: Proxy Usage

!!! info "On this page"
    - HTTP/HTTPS proxy
    - Proxy with Authentication
    - Per-request proxy with custom headers
    - Unix Socket proxy for local services (Docker, Podman)
    - Constructor reference and choosing the right one

The `Proxy` class provides several constructors depending on what you want to intercept:

| Constructor | What it proxies |
|---|---|
| `Proxy.all(url)` | All requests (HTTP + HTTPS) |
| `Proxy.http(url)` | HTTP requests only |
| `Proxy.https(url)` | HTTPS requests only |
| `Proxy.unix(path)` | Requests via a Unix socket |

You can pass a proxy in two ways:
- **Per-client** via `Client(proxies=[...])` — applies to every request made by that client.
- **Per-request** via `wreq.get(..., proxy=...)` — overrides or sets a proxy for a single request.

---

## HTTP / HTTPS Proxy

### Basic usage with a client

```python
import asyncio
from wreq import Client, Proxy

async def main():
    client = Client(
        proxies=[Proxy.all("http://proxy.example.com:8080")]
    )

    async with client.get("https://httpbin.io/ip") as resp:
        print(await resp.text())

asyncio.run(main())
```

All requests made through this `client` will be routed via the proxy.

---

### Proxy with authentication

If your proxy requires credentials, include them directly in the URL using the `username:password@host:port` syntax.

**HTTP / HTTPS proxy with credentials:**

```python
proxy = Proxy.all("http://username:password@proxy.example.com:8080")

client = Client(proxies=[proxy])
```

Or using the credentials separately

```python
proxy = Proxy.all(
    url="http://proxy.example.com:8080",
    username="username",
    password="password"
)

client = Client(proxies=[proxy])
```

**SOCKS5 proxy with credentials:**

```python
# SOCKS5
client = Client(
    proxies=[Proxy.all("socks5://username:password@127.0.0.1:1080")]
)


# SOCKS5h (DNS also resolved by the proxy)
client = Client(
    proxies=[Proxy.all("socks5h://username:password@127.0.0.1:6152")]
)
```

> The difference between `socks5://` and `socks5h://` is that `socks5h` delegates DNS resolution to the proxy server, which helps avoid DNS leaks.

---

### Per-request proxy with custom headers

You can configure a proxy for a single request and attach custom headers sent to the proxy server:

```python
import asyncio
import wreq
from wreq import Proxy

async def main():
    async with wreq.get(
        "https://httpbin.io/anything",
        proxy=Proxy.all(
                url="http://127.0.0.1:6152",
                custom_http_headers={
                    "user-agent": "wreq",
                    "accept": "*/*",
                    "accept-encoding": "gzip, deflate, br",
                    "x-proxy": "wreq",
                },
            )
    ) as resp:
        print(await resp.text())

asyncio.run(main())
```

> **Note:** `custom_http_headers` are headers sent *to the proxy itself*, not to the final destination server.

---

## Unix Socket Proxy

Unix sockets allow communication with local services without going through a network port. This is common when working with Docker, Podman, or other daemons that expose a socket file.

```python
import asyncio
import wreq
from wreq import Proxy

async def main():
    async with wreq.get(
        "http://localhost/v1.41/containers/json",
        proxies=[Proxy.unix("/var/run/docker.sock")],
    ) as resp:
        print(await resp.text())

asyncio.run(main())
```

Even though the URL says `http://localhost`, the request never touches the network. It goes directly through the socket file at the given path.
