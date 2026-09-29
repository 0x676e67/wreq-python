# :star2: Advanced Features

!!! info "On this page"
    - Header order
    - Other advanced usage

### Streaming Request Body

Send data using async generators for streaming uploads:

Async upload generators run on the caller's running event loop with its context variables.
Their exceptions fail the request. When an upload ends, the generator is closed;
cancelling or dropping the upload schedules producer cancellation and cleanup on that loop.
Keep the loop running until generator cleanup has finished. This also applies to async multipart parts.

```python
import asyncio
import wreq


async def gen():
    for i in range(10):
        await asyncio.sleep(0.1)

        if i <= 5:
            # bytes chunk
            yield bytes(f"Hello {i}\n", "utf-8")
        else:
            # str chunk
            yield str("Hello {}\n".format(i)).encode("utf-8")


async def main():
    resp = await wreq.post(
        "https://httpbin.io/anything",
        body=gen(),
    )
    print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
```

### Multipart File Upload

Upload multiple files and data parts:

```python
from pathlib import Path
import asyncio
import aiofiles
import wreq
from wreq import Multipart, Part


async def file_to_bytes_stream(file_path):
    async with aiofiles.open(file_path, "rb") as f:
        while chunk := await f.read(1024):
            yield chunk


async def main():
    resp = await wreq.post(
        "https://httpbin.io/anything",
        multipart=Multipart(
            # Upload text data
            Part(name="def", value="111", filename="def.txt", mime="text/plain"),
            # Upload binary data
            Part(name="abc", value=b"000", filename="abc.txt", mime="text/plain"),
            # Upload file data
            Part(
                name="LICENSE",
                value=Path("LICENSE"),
                filename="LICENSE",
                mime="text/plain",
            ),
            # Upload bytes stream file data
            Part(
                name="README",
                value=file_to_bytes_stream("README.md"),
                filename="README.md",
                mime="text/plain",
            ),
        ),
    )

    print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
```

### Custom runtimes

Clients share a lazily started global multi-thread runtime when `runtime` is
omitted or `None`. Pass a `Runtime` to choose a separate worker pool for an
async or blocking client:

```python
from wreq import Client
from wreq.runtime import Runtime

runtime = Runtime(
    workers=1,
    work_steal=False,
    thread_name="http-client",
    max_blocking_threads=8,
    thread_keep_alive=10.0,
)
client = Client(runtime=runtime)
```

With `work_steal=False`, workers use independent single-thread Tokio runtimes.
Each client is assigned one worker for its lifetime; requests, response reads,
streams and WebSocket operations use that worker. With multiple workers, newly
created clients are assigned round-robin. This is not CPU pinning. Sharing the
same `Runtime` between clients is supported, and `client.runtime` returns its
runtime configuration and owner.

`workers=None` uses `TOKIO_WORKER_THREADS` when it contains a positive integer,
otherwise the available parallelism. The global runtime's configuration is
chosen when first accessed. Both default and custom runtimes start their
threads on first use.

`thread_keep_alive` is in seconds. `max_blocking_threads` and
`thread_keep_alive` default to Tokio's settings (512 and 10 seconds). In
no-steal mode these limits apply to **each worker's** blocking pool, not the pool
as a whole. Python async upload generators still run on the caller's event loop.
Standalone multipart file preparation and upload-task cleanup can use the
shared runtime; a dedicated client runtime does not isolate Python's GIL or
every process resource. DNS resolvers are owned by individual clients so their
connections are not shared across runtimes.

Closing a client cancels its requests but does not shut down its runtime.
Responses and streams retain the runtime independently. For explicit shutdown,
release all clients (including closed ones), responses, streams and in-flight
tasks, then call `runtime.shutdown_timeout(1.0)`. This is a blocking call which
releases the GIL. The timeout is in seconds per worker; already running blocking
tasks may outlive it. Shutdown raises `RuntimeError` while the runtime is in use
or if it is the shared default. Repeated shutdown of a custom runtime is harmless,
and `runtime.closed` reports whether it has been explicitly shut down.

Dropping all owners automatically releases a custom runtime without synchronously
waiting for its worker threads. Invalid configuration is rejected with Python
exceptions.

### TLS Key Logging

Capture TLS keys for debugging with tools like Wireshark:

```python
import asyncio
from wreq import Client
from wreq.tls import KeyLog


async def main():
    client = Client(keylog=KeyLog.file("keylog.log"))
    resp = await client.get("https://www.google.com")
    async with resp:
        print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
```

### Original Header Order Preservation

Preserve header case and order for specific sites:

```python
import asyncio
import wreq
from wreq.emulation import Emulation


async def main():
    ws = await wreq.websocket(
        "wss://gateway.discord.gg/",
        emulation=Emulation.Chrome137,
        headers={"Origin": "https://discord.com"},
        # Preserve HTTP/1 case and header order
        orig_headers=[
            "User-Agent",
            "Origin",
            "Host",
            "Accept",
            "Accept-Encoding",
            "Accept-Language",
        ],
    )

    msg = await ws.recv()
    if msg is not None:
        print(msg.json())
    await ws.close()


if __name__ == "__main__":
    asyncio.run(main())
```
