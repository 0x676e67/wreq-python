# Authentication

Pass authentication options to a request. The same options work with
`wreq.Client`, the module-level request functions, and the
[blocking API](blocking.md).

## Basic authentication

`basic_auth=(username, password)` builds an `Authorization: Basic ...` header.
Use `None` for a missing password.

```python
import asyncio
import wreq


async def main():
    async with wreq.Client() as client:
        async with client.get(
            "https://httpbin.org/basic-auth/username/password",
            basic_auth=("username", "password"),
        ) as response:
            print(await response.json())


asyncio.run(main())
```

## Bearer tokens and other authorization schemes

`bearer_auth` adds the `Bearer ` prefix. Use `auth` when you need to supply
the complete `Authorization` header value, including any scheme name.

```python
import asyncio
import wreq


async def main():
    async with wreq.Client() as client:
        async with client.get(
            "https://httpbin.org/anything", bearer_auth="example-token"
        ) as response:
            print(await response.json())

        async with client.get(
            "https://httpbin.org/anything", auth="Token example-token"
        ) as response:
            print(await response.json())


asyncio.run(main())
```

Choose one authentication option per request. Basic authentication encodes
credentials; use HTTPS to protect them in transit.

## Reusing a token

For an API that uses the same token on every request, set the client's default
headers. Keep that client dedicated to the API that should receive the token.
APIs that use a custom header such as `X-API-Key` can be configured the same way.

```python
import asyncio
import wreq


async def main():
    async with wreq.Client(
        headers={"Authorization": "Bearer example-token"}
    ) as client:
        async with client.get("https://httpbin.org/anything") as response:
            print(await response.json())


asyncio.run(main())
```

For proxy credentials, see [Proxy authentication](proxy.md#proxy-authentication).
