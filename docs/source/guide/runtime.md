# Runtimes

`wreq` uses a Rust runtime to schedule connections and network I/O. An async
client also needs a running Python event loop for its coroutines and async
upload generators. Changing the Rust runtime does not replace that event loop.

Most applications can use `Client()` with the default runtime. Create a
`Runtime` when you need a separate worker pool, a specific worker count, or
caller-driven blocking I/O.

## Choose a scheduler

| Scheduler | Behavior and use |
| --- | --- |
| `WORK_STEALING` | Shared worker pool for async services and concurrent blocking requests; the default |
| `PER_WORKER` | Each client stays on one worker's single-thread runtime; `workers=1` is benchmark ST |
| `CURRENT_THREAD` | The calling thread drives I/O to reduce handoffs; consider one client and runtime per calling thread |

Both worker schedulers support async and blocking clients. `CURRENT_THREAD`
supports only `wreq.blocking.Client`; passing it to `wreq.Client` raises
`ValueError`.

Runtime worker count and request concurrency are different settings. For
example, one async client can have many requests in flight with `workers=1`.
With `PER_WORKER`, increasing the worker count distributes newly created clients
across workers; a single client keeps its selected worker. This does not pin
work to a CPU core.

The [benchmarks](../benchmark.md) compare these configurations. Choose using
your own workload, including its concurrency, body sizes and response reads.

## Share a worker runtime

Omitting `runtime`, or passing `None`, uses a shared process-wide runtime that
starts on first use. `Runtime()` creates a separate `WORK_STEALING` runtime.
Its worker threads start when the runtime is constructed.

Pass the same runtime to clients that should share its workers:

```python
import asyncio
from datetime import timedelta

from wreq import Client
from wreq.runtime import Runtime, Scheduler


async def fetch(client, url):
    async with client.get(url) as response:
        response.raise_for_status()
        return await response.json()


async def main():
    runtime = Runtime(scheduler=Scheduler.WORK_STEALING, workers=2)
    async with (
        Client(runtime=runtime, timeout=timedelta(seconds=10)) as first,
        Client(runtime=runtime, timeout=timedelta(seconds=10)) as second,
    ):
        results = await asyncio.gather(
            fetch(first, "https://httpbin.io/get?client=first"),
            fetch(second, "https://httpbin.io/get?client=second"),
        )
        print(results)


asyncio.run(main())
```

Clients keep separate connection pools and DNS resolvers even when they share
a runtime. `client.runtime` provides access to that shared underlying runtime;
the attribute is read-only. A dedicated runtime does not isolate Python's GIL
or all process resources.

## Use the calling thread for blocking I/O {#current-thread}

`CURRENT_THREAD` avoids dispatching each request to a background I/O worker.
Create the client and runtime together and reuse them for subsequent calls:

```python
from datetime import timedelta

from wreq.blocking import Client
from wreq.runtime import Runtime, Scheduler


with Client(
    runtime=Runtime(scheduler=Scheduler.CURRENT_THREAD),
    timeout=timedelta(seconds=10),
) as client:
    for page in range(3):
        with client.get("https://httpbin.io/get", query={"page": page}) as response:
            response.raise_for_status()
            print(response.json())
```

Leave `workers` unset with this scheduler. Even `workers=1` raises `ValueError`.
It still has a blocking pool for work such as multipart file reads, redirect
callbacks and system DNS lookups.

Use it when your program can make progress through explicit blocking calls.
It has several limits:

- Between calls, connection tasks and timers do not progress. HTTP/2 keep-alive
  pings, idle-connection cleanup and WebSocket control frames may wait for the
  next call. Use a worker scheduler when connections need background activity.
- Threads sharing a CT runtime take turns driving its I/O. For concurrent
  blocking workloads, consider one client and runtime per calling thread, or
  use a worker scheduler for a shared client.
- Synchronous upload iterators run on whichever thread drives the runtime.
  A slow `__next__` also delays timeouts and other work on that runtime. It must
  not wait for another call on the same runtime. Calling a blocking wreq method
  from such an iterator raises `RuntimeError`, including reads on another response.
- Response streams from CT clients support blocking iteration only. Async
  iteration or entering an async context raises `RuntimeError`.

Do not call the blocking API on an asyncio event-loop thread. Use the async
client there, or move the whole blocking operation to a separate Python thread.

## Configure workers and blocking tasks

These options apply when constructing `Runtime`:

| Option | Behavior |
| --- | --- |
| `scheduler` | Defaults to `Scheduler.WORK_STEALING` |
| `workers` | A positive worker count; `None` uses available CPU parallelism, falling back to 1. Must be `None` for CT |
| `thread_name` | Defaults to `"wreq-python"`; names cannot contain NUL characters |
| `max_blocking_threads` | A positive limit for the blocking pool; `None` keeps Tokio's default of 512 |
| `thread_keep_alive` | A nonnegative `datetime.timedelta` for idle blocking threads; `None` keeps Tokio's default of 10 seconds |

With `PER_WORKER`, blocking-pool limits apply to each worker's runtime. The
blocking pool is separate from the I/O worker count. Async upload generators
always run on their Python event loop. Synchronous iterators use blocking tasks
with a worker scheduler and are read directly by the driving thread with CT.

The older `work_steal` argument has been replaced by `scheduler`.
Use `Scheduler.PER_WORKER` where you previously used `work_steal=False`.

## Closing clients and runtime lifetime

Use `async with Client(...)` or `with blocking.Client(...)` to close clients.
You can also call `client.close()` directly; it is synchronous on both APIs.
Closing cancels pending requests and rejects new ones with
`asyncio.CancelledError`. Existing responses, streams and WebSockets remain
usable, and other clients sharing the runtime remain open.

`Runtime` has no `close()` method or context manager. Clients, responses,
streams and active work keep a custom runtime alive. Dropping the last owner
releases it without synchronously waiting for its workers; blocking work
already running may finish later. The default runtime lives for the process
lifetime.

See the [Runtime API reference](../api/runtime.md) for the full signature and
the [streaming guide](advanced.md) for upload cleanup and response ownership.
