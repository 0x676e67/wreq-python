# Basic usage

Use a client for a group of requests that share headers, cookies, timeouts or
connections. This guide uses the async API; see the [blocking guide](blocking.md)
for synchronous examples.

## Reuse a client

```python
import asyncio
from datetime import timedelta

from wreq import Client


async def main():
    async with Client(
        headers={"User-Agent": "MyApp/1.0", "Accept": "application/json"},
        timeout=timedelta(seconds=30),
        connect_timeout=timedelta(seconds=5),
    ) as client:
        async with client.get("https://httpbin.org/get") as response:
            response.raise_for_status()
            print(await response.json())


asyncio.run(main())
```

The following request snippets belong inside that client's context in `main()`.
Keep the client open until all requests and response reads finish. Client
context exit calls `close()`, cancelling pending requests and making new
requests fail with `asyncio.CancelledError`. `client.close()` itself is
synchronous, including on the async client.

For a single request, the top-level functions offer the same request options:

```python
import asyncio
import wreq


async def main():
    async with wreq.get("https://httpbin.org/get") as response:
        print(await response.json())


asyncio.run(main())
```

## Request methods and parameters

Use `get`, `post`, `put`, `patch`, `delete`, `head`, `options` or `trace`.
For a method selected at runtime, use `request` with a `Method` enum:

```python
from wreq import Method

async with client.request(
    Method.GET,
    "https://httpbin.org/get",
    query=[("tag", "python"), ("tag", "rust"), ("page", 1)],
) as response:
    print(await response.json())
```

`query` and `form` accept dictionaries or lists of pairs. Pairs preserve
repeated keys. Values can be strings, integers, floats or booleans.

## Request bodies

Use `json` for JSON, `form` for URL-encoded fields, and `body` for raw text or
bytes. `json` and `form` set their content type automatically. Set it explicitly
for a raw body when the server needs it. Choose one body argument per request.

```python
async with client.post(
    "https://httpbin.org/post", json={"name": "Ada", "active": True}
) as response:
    print(await response.json())

async with client.post(
    "https://httpbin.org/post", form=[("tag", "python"), ("tag", "rust")]
) as response:
    print(await response.json())

async with client.post(
    "https://httpbin.org/post",
    body=b"hello",
    headers={"Content-Type": "application/octet-stream"},
) as response:
    print(await response.json())
```

For file uploads or generated data, see [multipart and streaming uploads](advanced.md).

## Headers

Client headers supply defaults; request headers override matching names for
that request. Dictionaries work for ordinary headers. Use
[HeaderMap](../api/header.md#wreq.header.HeaderMap) to retain multiple values:

```python
from wreq.header import HeaderMap

headers = HeaderMap({"Accept": "application/json"})
headers.append("Accept", "text/html")
print([str(value, "ascii") for value in headers.get_all("accept")])

async with client.get("https://httpbin.org/headers", headers=headers) as response:
    content_type = response.headers.get("content-type")
    if content_type is not None:
        print(str(content_type, "ascii"))
    print(await response.json())
```

Header names are case insensitive. Header names and values returned by
`HeaderMap` are read-only memoryviews; `str(view, encoding)` decodes them
without an intermediate `bytes` copy.

## Authentication

Use `basic_auth=(username, password)` or `bearer_auth=token` on a request.
For a different scheme, `auth` supplies the complete `Authorization` value.

```python
async with client.get(
    "https://httpbin.org/basic-auth/ada/password",
    basic_auth=("ada", "password"),
) as response:
    response.raise_for_status()
    print(await response.json())

async with client.get(
    "https://httpbin.org/bearer", bearer_auth="example-token"
) as response:
    response.raise_for_status()
    print(await response.json())
```

The credentials above are example values. Read application credentials from
your configuration instead of putting them in source code.

## Timeouts and errors

Timeout values use `datetime.timedelta`:

| Option | Scope |
| --- | --- |
| `timeout` | Overall request, from connection setup through reading the body. No timeout by default. |
| `connect_timeout` | Connection setup; set this on the client. |
| `read_timeout` | Each read operation; the timer resets after a successful read. |

A request can override `timeout` and `read_timeout` for that call. HTTP error
statuses do not raise automatically unless the client has
`raise_for_status=True`. Otherwise call the synchronous `raise_for_status()`
method on the response:

```python
from datetime import timedelta

from wreq.exceptions import ConnectionError, StatusError, TimeoutError

try:
    async with client.get(
        "https://httpbin.org/get",
        timeout=timedelta(seconds=10),
        read_timeout=timedelta(seconds=5),
    ) as response:
        response.raise_for_status()
        print(await response.json())
except StatusError as exc:
    print("HTTP error:", exc)
except (ConnectionError, TimeoutError) as exc:
    print("Connection or timeout error:", exc)
```

Other failures, including TLS and decoding errors, have their own
[exception classes](../api/exceptions.md). All of them derive from
`wreq.exceptions.Error`, so catch it to handle any other failure.

## Read response metadata and bodies

```python
async with client.get("https://httpbin.org/json") as response:
    print(response.status.as_int())
    print(response.version, response.url)
    print(response.headers, response.cookies)
    print(response.content_length, response.remote_addr)
    print(await response.text())
    print(await response.json())
```

`text()`, `json()` and `bytes()` cache the complete body after reading it, so
sequential calls can read the same content. Complete one read before starting
another. `stream()` only accepts an unread body and transfers it to a streamer;
after that, use the streamer to read it. Reading a released body or starting an
overlapping read raises `RuntimeError`.

`bytes()` returns a zero-copy, read-only memoryview of the collected Rust
buffer. Collecting the body may allocate to join chunks. Views stay valid after
the response closes; see [binary data](../getting-started/quickstart.md#binary-data)
for decoding, copying and releasing them.

## Concurrent requests

Reuse one async client across tasks. Limit the number of active requests when
the input can grow large; a runtime's worker count does not limit Python tasks.
This example allows up to four complete exchanges at once:

```python
import asyncio

from wreq import Client


async def main():
    limit = asyncio.Semaphore(4)
    async with Client() as client:
        async def fetch(page):
            async with limit:
                async with client.get(
                    "https://httpbin.org/get", query={"page": page}
                ) as response:
                    response.raise_for_status()
                    return await response.json()

        async with asyncio.TaskGroup() as group:
            tasks = [group.create_task(fetch(page)) for page in range(10)]
        print([task.result() for task in tasks])


asyncio.run(main())
```

`TaskGroup` waits for its tasks before the client closes and cancels sibling
tasks if one fails. For very large inputs, use a fixed number of tasks reading
from a queue to avoid creating one task per item. Let `asyncio.CancelledError`
propagate when a caller cancels your task.

## Streaming responses

Read large bodies incrementally with `response.stream()`. Data frames are
read-only memoryviews, and trailing HTTP headers are `HeaderMap` objects.
This complete example counts the bytes without retaining the whole body:

```python
import asyncio

from wreq import Client, HeaderMap


async def main():
    total = 0
    async with Client() as client:
        async with client.get("https://httpbin.org/stream/10") as response:
            response.raise_for_status()
            async with response.stream() as streamer:
                async for frame in streamer:
                    if isinstance(frame, memoryview):
                        total += len(frame)
                    elif isinstance(frame, HeaderMap):
                        print("Trailers:", frame)
    print("Received bytes:", total)


asyncio.run(main())
```

A stream chunk is not necessarily a complete text line or JSON object. Use an
incremental decoder and retain incomplete records when parsing a streamed format.

Manage the streamer with its own context, including when you stop early.
Response context exit releases the body and keeps a fully read connection
reusable. Explicit `await response.close()` also marks the connection as
non-reusable; it does not cancel an active read or guarantee an immediate socket
shutdown. Cancel and await any body-read task before closing its response.

Continue with [cookies](cookie.md), [redirects](redirect-errors.md), or
[streaming and multipart uploads](advanced.md). The [runtime guide](runtime.md)
shows concurrent requests and explains how to configure the workers that drive
network I/O.
