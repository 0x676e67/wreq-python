# Redirects and errors

## Following redirects

wreq returns redirect responses without following them by default. Set
`redirect=Policy.limited(5)` to follow a bounded chain, or `Policy.none()` to
stop following redirects. The option can be set on the client or on one request.

```python
import asyncio
from wreq import Client
from wreq.redirect import Policy


async def main():
    async with Client(redirect=Policy.limited(5)) as client:
        async with client.get("https://httpbin.org/redirect/3") as response:
            print("Final URL:", response.url)
            for entry in response.history:
                print(entry.status, entry.previous, entry.url)
            await response.bytes()

        async with client.get(
            "https://httpbin.org/redirect/1", redirect=Policy.none()
        ) as response:
            print(response.status)
            location = response.headers.get("location")
            if location is not None:
                print("Location:", bytes(location).decode())
            await response.bytes()


asyncio.run(main())
```

`Policy.limited()` without an argument uses a limit of 10. This only takes
effect when you pass the policy to the client or request.

## Custom redirect policies

A callback receives the next URL, the redirect response, and the previous
URLs. Return `attempt.follow()`, `attempt.stop()`, or `attempt.error(message)`.
Stopping returns the redirect response; returning an error raises
`wreq.exceptions.RedirectError`.

```python
import asyncio
from urllib.parse import urlsplit

from wreq import Client
from wreq.redirect import Action, Attempt, Policy


def follow_allowed_host(attempt: Attempt) -> Action:
    if urlsplit(attempt.next).hostname != "httpbin.org":
        return attempt.stop()
    if len(attempt.previous) > 5:
        return attempt.error("Redirect limit reached")
    return attempt.follow()


async def main():
    async with Client(redirect=Policy.custom(follow_allowed_host)) as client:
        async with client.get("https://httpbin.org/redirect/3") as response:
            print(response.url, response.status)
            await response.bytes()


asyncio.run(main())
```

The callback is a regular `def`, runs on a background thread, and must return
an `Action`. A custom policy must enforce its own redirect limit;
`attempt.previous` includes the original request URL. Compare the
parsed hostname when restricting destinations; a substring match can also
match a hostname or URL path you did not intend to allow.

## HTTP status errors

By default, a 4xx or 5xx response is returned normally, so you can inspect
`response.status` and read its body. `Client(raise_for_status=True)` instead
raises `StatusError` when such a response arrives.

```python
import asyncio
from wreq import Client, exceptions


async def main():
    async with Client(raise_for_status=True) as client:
        try:
            async with client.get("https://httpbin.org/status/404") as response:
                print(await response.text())
        except exceptions.StatusError as error:
            print("HTTP error:", error)


asyncio.run(main())
```

Use the default behavior if you need an API's error response body.

## Timeouts and transport errors

Timeout values use `datetime.timedelta`. `timeout` covers the request through
reading its response body. `connect_timeout` limits connection establishment;
`read_timeout` limits the wait for a read and resets after successful reads.
Requests can override `timeout` and `read_timeout`.

```python
import asyncio
from datetime import timedelta

from wreq import Client, exceptions


async def main():
    async with Client(
        timeout=timedelta(seconds=10),
        connect_timeout=timedelta(seconds=3),
    ) as client:
        try:
            async with client.get(
                "https://httpbin.org/delay/5",
                timeout=timedelta(seconds=1),
            ) as response:
                print(await response.text())
        except exceptions.TimeoutError as error:
            print("Request timed out:", error)
        except (exceptions.ConnectionError, exceptions.TlsError) as error:
            print("Connection failed:", error)


asyncio.run(main())
```

Catch the failures your application can handle. Other exception types include
`ProxyConnectionError`, `ConnectionResetError`, `BodyError`, `DecodingError`,
`BuilderError`, and `WebSocketError`; see the
[exception reference](../api/exceptions.md). `RequestError` is not a common
base class for all wreq errors.

Asynchronous cancellation uses `asyncio.CancelledError`. Allow it to propagate
when a caller cancels the task. The same request options and wreq exception
classes apply to the [blocking API](blocking.md).
