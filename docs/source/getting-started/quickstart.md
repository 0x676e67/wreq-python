# Quick start

Use `wreq.Client` in an asyncio application, or `wreq.blocking.Client` in
synchronous code. Keep a client open across requests to reuse connections and
configuration.

## Make a request

=== "Async"

    ```python
    import asyncio
    from datetime import timedelta

    from wreq import Client


    async def main():
        async with Client(timeout=timedelta(seconds=30)) as client:
            async with client.get("https://httpbin.org/get") as response:
                response.raise_for_status()
                print(response.status.as_int())
                print(await response.json())


    asyncio.run(main())
    ```

=== "Blocking"

    ```python
    from datetime import timedelta

    from wreq.blocking import Client


    with Client(timeout=timedelta(seconds=30)) as client:
        with client.get("https://httpbin.org/get") as response:
            response.raise_for_status()
            print(response.status.as_int())
            print(response.json())
    ```

The response context releases its body when you leave the block. Reading the
whole body lets the connection return to the pool. The outer client context
closes the client when all its requests are finished.

The snippets below go inside `main()`, within the `async with Client(...) as
client:` block above. For blocking code, use `with` and remove `await`.
`raise_for_status()` is synchronous in both APIs.

## Pass query parameters

Use `query` to add parameters to a URL:

```python
async with client.get(
    "https://httpbin.org/get",
    query={"search": "wreq", "page": 1},
) as response:
    print(response.url)
    print(await response.json())
```

For repeated keys, pass a list of pairs, such as
`query=[("tag", "python"), ("tag", "rust")]`.

## Send JSON or form data

`json` serializes a JSON body and sets its content type. `form` sends
URL-encoded form fields. Choose one body argument for each request.

```python
async with client.post(
    "https://httpbin.org/post",
    json={"name": "Ada", "active": True},
) as response:
    print(await response.json())

async with client.post(
    "https://httpbin.org/post",
    form={"username": "ada", "count": 3},
) as response:
    print(await response.json())
```

The client also has `put`, `patch`, `delete`, `head`, `options` and `trace`
methods. Use `body=b"..."` or `body="..."` to send raw data.

## Read a response

`response.status`, `response.url` and `response.headers` are available as soon
as the request returns. Reading the body uses `text()`, `json()` or `bytes()`:

```python
async with client.get("https://httpbin.org/json") as response:
    response.raise_for_status()
    print(await response.text())
    print(await response.json())
    content_type = response.headers.get("content-type")
    if content_type is not None:
        print(str(content_type, "ascii"))
```

A complete body read is cached, so the second read above uses the same bytes.
Finish each read before starting another. For large bodies, see
[streaming responses](../guide/basic.md#streaming-responses).

### Binary data

`bytes()` returns a read-only `memoryview` backed by Rust-owned data. Returning
the view is zero-copy; collecting a complete response can still allocate memory
to join body chunks.

```python
import hashlib

async with client.get("https://httpbin.org/bytes/16") as response:
    view = await response.bytes()

print(view.readonly)  # True
print(hashlib.sha256(view).hexdigest())
```

The view remains valid after the response closes. Use it directly with APIs
that accept buffers, such as `file.write(view)` or `hashlib.sha256(view)`.
For text, use `str(view, "utf-8")`; memoryviews have no `.decode()` method.

Use `bytes(view)` or `view.tobytes()` when an API needs a `bytes` object. Both
copy the data. This includes passing a view back to wreq's binary inputs such
as `body`, `Part`, `Message` constructors or `CertStore`. You can release your
reference early with `view.release()`; other views and slices stay valid.

The blocking API returns the same type. Stream data frames, binary WebSocket
fields, header names and values, and peer certificates also use read-only
memoryviews that retain their backing data.

## Set headers

Pass a dictionary for ordinary request headers:

```python
async with client.get(
    "https://httpbin.org/headers",
    headers={"User-Agent": "MyApp/1.0", "Accept": "application/json"},
) as response:
    print(await response.json())
```

Use [HeaderMap](../api/header.md#wreq.header.HeaderMap) when a header needs
multiple values. Set `headers` on the client to apply defaults to its requests.

## Handle HTTP errors

HTTP 4xx and 5xx responses are returned normally unless you enable
`Client(raise_for_status=True)`. To check a response yourself, call
`raise_for_status()` and catch `StatusError`:

```python
from wreq.exceptions import StatusError

async with client.get("https://httpbin.org/status/404") as response:
    try:
        response.raise_for_status()
    except StatusError:
        print("HTTP error:", response.status.as_int())
```

Network failures raise separate [exceptions](../api/exceptions.md). See
[timeouts and errors](../guide/basic.md#timeouts-and-errors) for an example.

## Browser profiles and proxies

Choose a profile when constructing the client, or for one request with
`emulation`. Profiles configure TLS, HTTP/2 and headers:

```python
from wreq import Emulation

async with client.get(
    "https://httpbin.org/get",
    emulation=Emulation.Chrome154,
) as response:
    print(await response.json())
```

They do not execute JavaScript or render pages. See
[browser emulation](../guide/emulation.md) for profile settings.

To use a proxy for one request, pass `proxy=Proxy.all(url)`. To configure a
client, use `Client(proxies=[Proxy.all(url)])`. The
[proxy guide](../guide/proxy.md) covers authentication and routing rules.

## Next steps

- [Basic usage](../guide/basic.md): client settings, authentication and streaming.
- [Blocking API](../guide/blocking.md): synchronous requests and downloads.
- [Runtimes](../guide/runtime.md): shared workers and current-thread scheduling.
- [API reference](../api/wreq.md): complete signatures and options.
