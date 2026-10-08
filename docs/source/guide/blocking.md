# Blocking API

Use `wreq.blocking.Client` in scripts, synchronous applications or dedicated
worker threads. Request methods and body readers return their results directly.
Calling them on an asyncio event-loop thread blocks that loop; use the async
client there.

## Make a request

```python
from datetime import timedelta

from wreq.blocking import Client


with Client(timeout=timedelta(seconds=30)) as client:
    with client.get("https://httpbin.org/get") as response:
        response.raise_for_status()
        print(response.status.as_int())
        print(response.json())
```

Keep the client open across requests to reuse connections and settings. The
response context releases the body when you leave it; the client context closes
the client. Finish requests and body reads before leaving the client context.

## Configure requests

Client and request options match the async API. This example sets a browser
profile and connection timeout, then sends query parameters, JSON and form data:

```python
from datetime import timedelta

from wreq import Emulation, Method
from wreq.blocking import Client


with Client(
    emulation=Emulation.Chrome154,
    timeout=timedelta(seconds=30),
    connect_timeout=timedelta(seconds=5),
) as client:
    with client.request(
        Method.GET,
        "https://httpbin.org/get",
        query={"search": "wreq", "page": 1},
    ) as response:
        print(response.json())

    with client.post("https://httpbin.org/post", json={"name": "Ada"}) as response:
        print(response.json())

    with client.post("https://httpbin.org/post", form={"count": 3}) as response:
        print(response.json())
```

Import shared types such as `Method`, `Emulation` and `Proxy` from `wreq` or
their configuration modules. The `wreq.blocking` module supplies the blocking
client, response, WebSocket and request functions.

Use `headers` for request headers, `body` for raw bytes or text, and
`basic_auth` or `bearer_auth` for authentication. See
[basic usage](basic.md) for these options and timeout semantics. Replace
`async with` with `with` and omit `await` when adapting its request examples.

## Keep cookies

Enable `cookie_store=True` to retain cookies across requests. Without a cookie
store, response cookies are available but are not saved for later requests.

```python
from wreq.blocking import Client
from wreq.redirect import Policy


with Client(cookie_store=True, redirect=Policy.limited(5)) as client:
    with client.get("https://httpbin.org/cookies/set?session=example") as response:
        response.raise_for_status()
        print(response.json())

    with client.get("https://httpbin.org/cookies") as response:
        print(response.json())
```

`response.cookies` contains cookies from that response's `Set-Cookie` headers.
Use `client.cookie_jar` to inspect the persistent store. The
[cookie guide](cookie.md) covers a custom `Jar` and cookie attributes.

## Handle errors

The same exception classes apply to both APIs. HTTP 4xx and 5xx responses raise
`StatusError` when you call `response.raise_for_status()` or enable
`Client(raise_for_status=True)`:

```python
from wreq.blocking import Client
from wreq.exceptions import StatusError


with Client() as client:
    with client.get("https://httpbin.org/status/404") as response:
        try:
            response.raise_for_status()
        except StatusError:
            print("HTTP error:", response.status.as_int())
```

Closing a client cancels pending requests and rejects new ones with
`asyncio.CancelledError`, including in the blocking API.

## Streaming response

`response.bytes()` returns a read-only memoryview of the complete body. To
write a large response incrementally, use a streamer:

```python
from wreq import HeaderMap
from wreq.blocking import Client


with Client() as client:
    with client.get("https://httpbin.org/bytes/65536") as response:
        response.raise_for_status()
        with response.stream() as streamer, open("download.bin", "wb") as output:
            for frame in streamer:
                if isinstance(frame, memoryview):
                    output.write(frame)
                elif isinstance(frame, HeaderMap):
                    print("Trailers:", frame)
```

Data views remain valid after the stream closes. `output.write(frame)` accepts
the buffer directly; `bytes(frame)` makes a copy. Only an unread response can
be streamed. Complete body reads through `text()`, `json()` or `bytes()` are
cached and can be repeated sequentially until the response is released.

Do not close a response while another thread is reading its body.
`response.close()` discards the retained body and marks its connection as
non-reusable; it does not interrupt an active read or guarantee an immediate
socket shutdown. Leaving `with response:` keeps a fully read connection
reusable. A body transferred to a streamer needs its own context, as above.

## Custom runtime

Both APIs use a shared runtime with worker threads by default. Supply a
`Runtime` to choose a scheduler or worker count. Closing a client does not
shut down a runtime that other clients still use. See the
[runtime guide](runtime.md) for scheduler choices and ownership.

### Current-thread runtime

`CURRENT_THREAD` runs network I/O on the thread making the blocking call. It
can reduce the cost of handing work to a background worker, particularly at low
concurrency:

```python
from wreq.blocking import Client
from wreq.runtime import Runtime, Scheduler


with Client(runtime=Runtime(scheduler=Scheduler.CURRENT_THREAD)) as client:
    with client.get("https://httpbin.org/get") as response:
        print(response.json())
```

This scheduler only supports blocking clients and blocking stream iteration.
Connection tasks stop between calls, so keep-alive pings and idle cleanup do
not run continuously. Prefer the default worker scheduler when your application
needs background connection maintenance. See
[current-thread scheduling](runtime.md#current-thread) for shared-thread and
upload-iterator constraints.
