# Cookies

This page covers how wreq handles HTTP cookies: reading cookies from responses,
sending cookies with requests, and managing a persistent cookie store across a session.

---

## The Cookie Object

A [Cookie](../api/cookie/#wreq.cookie.Cookie) represents a single HTTP cookie. You can create one manually to
pre-populate a jar or to send it with a specific request.

```python
import datetime
from wreq import Cookie, SameSite

cookie = Cookie(
    name="session",
    value="abc123",
    domain="example.com",
    path="/",
    max_age=datetime.timedelta(hours=1),
    http_only=True,
    secure=True,
    same_site=SameSite.Lax,
)

print(cookie.name)       # session
print(cookie.value)      # abc123
print(cookie.domain)     # example.com
print(cookie.http_only)  # True
print(cookie.secure)     # True
print(str(cookie))
# session=abc123; HttpOnly; Secure; SameSite=Lax; Path=/; Domain=example.com; Max-Age=3600
```

All constructor arguments except `name` and `value` are optional.

| Argument    | Type                        | Description                                      |
|-------------|-----------------------------|--------------------------------------------------|
| `name`      | `str`                       | The cookie name.                                 |
| `value`     | `str`                       | The cookie value.                                |
| `domain`    | `str \| None`               | Domain scope.                                    |
| `path`      | `str \| None`               | Path scope.                                      |
| `max_age`   | `timedelta \| None`         | Lifetime relative to the time of creation.       |
| `expires`   | `datetime \| None`          | Absolute expiry date.                            |
| `http_only` | `bool \| None`              | Prevents JavaScript access.                      |
| `secure`    | `bool \| None`              | Restricts the cookie to HTTPS connections.       |
| `same_site` | `SameSite \| None`          | Cross-site request policy (`Lax`, `Strict`, `Empty`). |

---

## Reading Cookies from a Response

Every response exposes the cookies set by the server as a sequence of `Cookie` objects:

```python
import asyncio
from wreq import Client

async def main():
    client = Client()
    async with client.get("https://httpbin.org/cookies/set?token=xyz") as response:
        for cookie in response.cookies:
            print(cookie.name, cookie.value)

asyncio.run(main())
```

`response.cookies` only contains the cookies set by that specific response.
To persist cookies automatically across multiple requests, use a `Jar`.

---

## Sending Cookies with a Request

To attach cookies to a single request without a persistent store, pass a
dictionary or a raw cookie string to the `cookies` argument:

```python
import asyncio
from wreq import Client

async def main():
    client = Client()

    async with client.get(
        "https://httpbin.org/cookies",
        cookies={"session": "abc123", "lang": "en"},
    ) as response:
        print(await response.json())
```

Cookies passed this way apply only to that request and are not persisted.

---

## Persistent Cookie Store with Jar

For sessions that span multiple requests - such as logging in and then
accessing protected pages - use a [Jar](../api/cookie/#wreq.cookie.Jar). The jar stores cookies received from
servers and sends them back automatically on subsequent requests.

### Enabling automatic cookie handling

Pass `cookie_store=True` to let wreq create and manage a jar internally:

```python
import asyncio
from wreq import Client

async def main():
    client = Client(cookie_store=True)

    # The server sets a cookie here
    await client.get("https://httpbin.org/cookies/set?token=abc")

    # The cookie is sent back automatically here
    async with client.get("https://httpbin.org/cookies") as response:
        print(await response.json())

asyncio.run(main())
```

You can access the internal jar at any time via `client.cookie_jar`.

### Using your own Jar

If you want to pre-populate the jar or inspect it directly, create a `Jar`
and pass it via `cookie_provider`:

```python
import asyncio
from wreq import Client, Jar, Cookie

async def main():
    jar = Jar()
    jar.add(
        Cookie("session", "abc123", domain="httpbin.org", path="/"),
        "https://httpbin.org",
    )

    client = Client(cookie_provider=jar)
    async with client.get("https://httpbin.org/cookies") as response:
        print(await response.json())

asyncio.run(main())
```

The `client.cookie_jar` property returns the same `Jar` instance in both cases,
so you can read and modify it while the client is active.

---

## Managing the Jar

### Adding a cookie

`Jar.add` accepts either a `Cookie` object or a raw `Set-Cookie` header string.
Both require a URL that determines the domain and path scope:

```python
from wreq import Jar, Cookie

jar = Jar()

# Using a Cookie object
jar.add(Cookie("session", "abc123", domain="example.com", path="/"), "https://example.com")

# Using a raw Set-Cookie string
jar.add("user=john; Path=/; HttpOnly", "https://example.com")
```

### Reading cookies

To retrieve a specific cookie by name and URL:

```python
cookie = jar.get("session", "https://example.com")
if cookie:
    print(cookie.name, cookie.value)
```

Domain matching in `get` is exact: only cookies whose domain exactly matches
the URL host are returned. Subdomains are not matched.

To retrieve all cookies currently in the jar:

```python
for cookie in jar.get_all():
    print(cookie.name, cookie.value, cookie.domain)
```

### Updating a cookie

There is no dedicated update method. Adding a cookie with the same name and
URL overwrites the existing entry:

```python
jar.add(Cookie("session", "new-value", domain="example.com", path="/"), "https://example.com")
```

### Removing a cookie

To remove a specific cookie by name and URL:

```python
jar.remove("session", "https://example.com")
```

### Clearing all cookies

```python
jar.clear()
```

---

## Protocol Behaviour

wreq adjusts how cookies are sent depending on the HTTP version in use:

- Over HTTP/1.1, all cookies are folded into a single `Cookie` header, as required by RFC 9112.
- Over HTTP/2 and above, each cookie is sent as an individual header field, as required by RFC 9113.

This is handled automatically. You do not need to change your code based on the protocol.

---

## Next Steps

- See the [Examples](../guide/basic.md) for more code samples
- Explore the [API Reference](../api/wreq.md) for detailed documentation
