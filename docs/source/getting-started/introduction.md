# Introduction

wreq is a Python HTTP client with a native [Rust engine](https://github.com/0x676e67/wreq).
It provides async and blocking APIs, reusable connection pools, streaming
transfers, and control over TLS and HTTP/2 behavior.

Use it for everyday HTTP requests, or when a server expects network behavior
that a conventional Python client does not expose. Browser emulation is a
configuration of the network stack, not a browser process: it does not execute
JavaScript, render pages, or guarantee access to a protected website.

## Start with a reusable client

```python
import asyncio

from wreq import Client, Emulation


async def main():
    async with Client(emulation=Emulation.Chrome154) as client:
        async with await client.get("https://example.com") as response:
            print(response.status)
            print(await response.text())


asyncio.run(main())
```

The [blocking client](../guide/blocking.md) offers the same HTTP building
blocks for synchronous applications. Reuse a client across requests to reuse
connections, and use context managers to close resources.

## Choose the control you need

<div class="grid cards" markdown>

- **Browser profiles**

    Configure TLS handshakes, HTTP/2 settings, and headers with Chrome, Firefox,
    Safari, Edge, Opera, or OkHttp profiles. Platform options cover Windows,
    macOS, Linux, Android, and iOS.

    [Use emulation](../guide/emulation.md) · [Available profiles](../api/emulation.md)

- **HTTP essentials**

    Send JSON, forms, and multipart bodies. Configure cookies, redirects,
    authentication, proxies, timeouts, and connection pooling.

    [Basic usage](../guide/basic.md)

- **Streaming and buffers**

    Stream request and response bodies, or read a complete body. Rust-backed
    binary outputs are read-only `memoryview` objects; convert to `bytes` only
    when another API requires a copy.

    [Advanced features](../guide/advanced.md)

- **Runtime configuration**

    Clients use the shared runtime unless you supply a custom `Runtime`.
    Select worker settings for your workload, including a single-worker runtime.

    [Runtime API](../api/runtime.md)

- **Protocol settings**

    Tune HTTP/1, HTTP/2, TLS, certificate verification, and client certificates
    when the default configuration does not fit your service.

    [HTTP/1](../api/http1.md) · [HTTP/2](../api/http2.md) · [TLS](../api/tls.md)

- **WebSockets**

    Upgrade a connection and exchange text or binary frames through the client.

    [WebSocket guide](../guide/websocket.md)

</div>

## Measure your workload

Performance depends on payload size, concurrency, protocol, runtime settings,
and how the application consumes responses. Our [HTTPS benchmarks](../benchmark.md)
publish those settings, repeated measurements, and raw data alongside the
tested source commit. Treat them as reproducible workloads, not a promise that
one client is fastest in every application.

## Next steps

[Install wreq](installation.md), follow the [quick start](quickstart.md), or
browse the [API reference](../api/wreq.md). The project is licensed under
[Apache-2.0](https://github.com/0x676e67/wreq-python/blob/main/LICENSE).
