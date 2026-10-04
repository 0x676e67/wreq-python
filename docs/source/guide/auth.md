# Authentication Guide

!!! tip "Supported authentication methods"
    - Basic Auth
    - Bearer Token
    - Simple Token

### Basic Authentication

```python
import asyncio
import wreq


async def main():
    resp = await wreq.get(
        "https://httpbin.io/anything",
        basic_auth=("username", "password"),
    )
    print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
```

### Bearer Token Authentication

```python
import asyncio
import wreq


async def main():
    resp = await wreq.get(
        "https://httpbin.io/anything",
        bearer_auth="token",
    )
    print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
```

### Simple Auth Token

```python
import asyncio
import wreq


async def main():
    resp = await wreq.get(
        "https://httpbin.io/anything",
        auth="token",
    )
    print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
```
