# Introduction

wreq is a Python HTTP client built on [Rust](https://github.com/0x676e67/wreq).
You can make async or blocking requests, reuse connections and stream bodies.
It also lets you configure TLS and HTTP/2 behavior.

Use it for everyday HTTP calls or for services that expect browser-like network
behavior. Browser profiles configure the network stack; they don't run a browser,
execute JavaScript or render pages. A profile won't guarantee access to a
protected website.

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

For synchronous code, use the [blocking client](../guide/blocking.md).
It has the same HTTP capabilities. Keep a client open across requests to reuse
connections, and use context managers to close resources when you're done.

## What you can configure

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
    binary outputs are read-only `memoryview` objects. Convert to `bytes` when
    an API requires that type; the conversion copies the data.

    [Advanced features](../guide/advanced.md)

- **Runtime configuration**

    Clients share a runtime by default. Supply your own `Runtime` to choose
    worker settings for a workload, including a single-worker runtime.

    [Runtime API](../api/runtime.md)

- **Protocol settings**

    Change HTTP/1, HTTP/2 or TLS settings when the defaults don't fit your
    service. You can also configure certificate verification and client certificates.

    [HTTP/1](../api/http1.md) · [HTTP/2](../api/http2.md) · [TLS](../api/tls.md)

- **WebSockets**

    Open a WebSocket connection through the client to send and receive text or
    binary frames.

    [WebSocket guide](../guide/websocket.md)

</div>

## Measure your workload

Payload size, concurrency and protocol affect performance. So do runtime
settings and the way you read responses. Our [HTTPS benchmarks](../benchmark.md)
include those settings, repeated measurements and raw data, with the source
commit tested. Use them to reproduce a workload and check how close it is to
your application. Performance rankings can change with the workload.

## Next steps

[Install wreq](installation.md), follow the [quick start](quickstart.md), or
browse the [API reference](../api/wreq.md). The project is licensed under
[Apache-2.0](https://github.com/0x676e67/wreq-python/blob/main/LICENSE).
