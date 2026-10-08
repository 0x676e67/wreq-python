# Cookies

Use `cookies=` for values you want to send with one request. Enable a cookie
jar when the server should maintain a session across requests. Cookie storage
is disabled by default. These options also work with the
[blocking client](blocking.md).

## Sending cookies

The `cookies` argument accepts a dictionary of strings or a raw `Cookie`
header value such as `"session=abc123; lang=en"`.

```python
import asyncio
from wreq import Client


async def main():
    async with Client() as client:
        async with client.get(
            "https://httpbin.org/cookies",
            cookies={"session": "abc123", "lang": "en"},
        ) as response:
            print(await response.json())


asyncio.run(main())
```

Passing `cookies=` does not add those values to a jar. If a jar is enabled,
an explicit `Cookie` header takes precedence over the jar's outgoing cookies;
the server's response cookies can still be stored.

## Reading response cookies

`response.cookies` contains the cookies from that response's `Set-Cookie`
headers. It does not list the client's stored cookies or cookies from earlier
responses in a redirect chain.

```python
import asyncio
from wreq import Client


async def main():
    async with Client() as client:
        async with client.get(
            "https://httpbin.org/cookies/set?session=abc123"
        ) as response:
            for cookie in response.cookies:
                print(cookie.name, cookie.value, cookie.domain, cookie.path)
            await response.bytes()


asyncio.run(main())
```

This endpoint returns a redirect with a `Set-Cookie` header. The example reads
that response directly; wreq does not follow redirects by default.

## Keeping a session

`cookie_store=True` creates a jar for the client. Cookies received from the
server are stored and sent on later requests when their domain, path, expiry,
and secure attributes allow it.

```python
import asyncio
from wreq import Client


async def main():
    async with Client(cookie_store=True) as client:
        async with client.get(
            "https://httpbin.org/cookies/set?token=abc"
        ) as response:
            await response.bytes()

        async with client.get("https://httpbin.org/cookies") as response:
            print(await response.json())


asyncio.run(main())
```

Access the store through `client.cookie_jar`. It is `None` when cookie handling
is disabled.

## Supplying a jar

Create a [Jar](../api/cookie.md#wreq.cookie.Jar) to load cookies before the first
request or share stored cookies between clients. The jar is safe to share
across threads and tasks.

```python
import asyncio
from wreq import Client, Cookie, Jar


async def main():
    jar = Jar()
    jar.add(
        Cookie("session", "abc123", domain="httpbin.org", path="/"),
        "https://httpbin.org/",
    )

    async with Client(cookie_provider=jar) as client:
        async with client.get("https://httpbin.org/cookies") as response:
            print(await response.json())

        for cookie in client.cookie_jar.get_all():
            print(cookie.name, cookie.value)


asyncio.run(main())
```

`client.cookie_jar` shares the supplied jar's storage. If both
`cookie_provider` and `cookie_store=True` are set, the supplied jar is used.
A request can also use `cookie_provider=jar` to select a store for that request.

## Managing stored cookies

`Jar.add` accepts a `Cookie` or a raw `Set-Cookie` value. The URL is the cookie's
origin and is used to validate its domain and determine its default path.

```python
from wreq import Cookie, Jar

jar = Jar()
url = "https://example.com/"
jar.add(Cookie("session", "abc123", domain="example.com", path="/"), url)
jar.add("language=en; Path=/; Secure", url)

cookie = jar.get("session", url)
if cookie is not None:
    print(cookie.name, cookie.value)

jar.add(Cookie("session", "new-value", domain="example.com", path="/"), url)

for cookie in jar.get_all():
    print(cookie.name, cookie.value, cookie.domain, cookie.path)

jar.remove("session", url)
jar.clear()
```

Adding a cookie with the same name, domain, path, and host-only scope replaces
the stored value. `get` and `remove` use an exact host and path lookup: a cookie
stored at `/` should be looked up with `https://example.com/`, even if it is
also sent on requests to `/account`. A subdomain URL does not look up a cookie
stored under its parent domain.

wreq formats outgoing cookie headers for the negotiated HTTP version; you can
use the same jar with HTTP/1.1 and HTTP/2.
