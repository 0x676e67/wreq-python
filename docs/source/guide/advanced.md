# Advanced Features

!!! info "On this page"
    - Header order
    - Other advanced usage

### Streaming Request Body

Send data using async generators for streaming uploads:

Async upload generators run on the caller's running event loop with its context variables.
Their exceptions fail the request. When an upload ends, the generator is closed;
cancelling or dropping the upload schedules producer cancellation and cleanup on that loop.
Keep the loop running until generator cleanup has finished. This also applies to async multipart parts.
Construct async-generator `Part` objects inside a running event loop; their producers
start at construction. A producer runs at most 256 KiB, or 64 small chunks, ahead of the
upload.
Use synchronous iterators for blocking uploads. They run on the runtime's blocking pool,
also at most 256 KiB or 64 small chunks ahead, except on a current-thread runtime. A
blocking call on the producer's event-loop thread prevents async generators from
progressing.

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
    async with wreq.post(
        "https://httpbin.io/anything",
        body=gen(),
    ) as resp:
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
    async with wreq.post(
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
    ) as resp:
        print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
```

### Custom runtimes

Clients share a global multi-thread runtime when `runtime` is omitted or `None`.
It starts on first use. Construct a `Runtime` to
start a separate worker pool for an async or blocking client:

```python
from datetime import timedelta

from wreq import Client
from wreq.runtime import Runtime, Scheduler

runtime = Runtime(
    scheduler=Scheduler.PER_WORKER,
    workers=1,
    thread_name="http-client",
    max_blocking_threads=8,
    thread_keep_alive=timedelta(seconds=10),
)
client = Client(runtime=runtime)
```

`scheduler` selects how the runtime runs client work:

- `Scheduler.WORK_STEALING` (the default) uses one multi-thread pool whose
  workers steal work from each other.
- `Scheduler.PER_WORKER` gives each worker its own single-thread Tokio runtime.
- `Scheduler.CURRENT_THREAD` has no workers: blocking calls drive its IO, which
  is fastest with a client and runtime per thread. It serves only blocking
  clients; see the [blocking guide](blocking.md#current-thread-runtime).

With `Scheduler.PER_WORKER`, each client is assigned one worker for its
lifetime; requests, response reads, streams and WebSocket operations use that
worker. Async reads of HTTP/1 data that has already arrived, and is not
content-encoded, finish on the event loop thread instead: `bytes()` and `text()`
up to 64 KiB, `json()` up to 8 KiB, and the first frame of a stream whose length
is known; later frames are read on the worker. With multiple workers, newly
created clients select a worker randomly and keep that selection. This is not CPU
pinning. Sharing the same `Runtime` between clients is supported, and
`client.runtime` returns the shared runtime object.

`workers=None` uses the available CPU parallelism, or 1 if it cannot be determined;
`Scheduler.CURRENT_THREAD` requires it.
Custom runtimes start their threads during construction, before any client is
bound or request is sent.

`thread_name=None` uses the package name, `wreq-python`, as the thread name.

`thread_keep_alive` accepts a nonnegative `datetime.timedelta`.
`max_blocking_threads` and `thread_keep_alive` default to Tokio's settings
(512 and 10 seconds). With
`Scheduler.PER_WORKER` these limits apply to **each worker's** blocking pool, not
the pool as a whole. Python async upload generators still run on the caller's event loop.
Multipart files open on the client's runtime when the request is built; only
upload cleanup that runs outside any runtime uses the shared runtime. A dedicated
client runtime does not isolate Python's GIL or every process resource. DNS resolvers are owned by individual clients so their
connections are not shared across runtimes.

Closing a client cancels pending requests and rejects new requests with
`asyncio.CancelledError`, for both async and blocking APIs. It does not shut down
the runtime or invalidate existing responses and WebSockets.
Clients, responses, streams and active tasks share ownership. Dropping the last
owner automatically releases a custom runtime without synchronously waiting for
its workers; already running blocking work may finish later. The default runtime
is shared for the process lifetime. Zero thread counts, NUL characters in thread
names and negative durations raise `ValueError`.

### TLS Key Logging

Capture TLS keys for debugging with tools like Wireshark:

```python
import asyncio
from wreq import Client
from wreq.tls import KeyLog


async def main():
    client = Client(keylog=KeyLog.file("keylog.log"))
    async with client.get("https://www.google.com") as resp:
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
    async with wreq.websocket(
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
    ) as ws:
        msg = await ws.recv()
        if msg is not None:
            print(msg.json())
        await ws.close()


if __name__ == "__main__":
    asyncio.run(main())
```
