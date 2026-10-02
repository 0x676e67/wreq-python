# :hourglass: Blocking/Sync API

!!! info "On this page"
    - Blocking GET
    - Configuration
    - Cookie/Auth/Streaming

The blocking API provides synchronous methods for environments where async/await is not needed.

### Simple GET Request

```python
import datetime
from wreq.blocking import Client
from wreq.emulation import Emulation


def main():
    client = Client()
    resp = client.get(
        "https://tls.peet.ws/api/all",
        timeout=datetime.timedelta(seconds=10),
        emulation=Emulation.Firefox139,
    )
    print(resp.text())


if __name__ == "__main__":
    main()
```

### Client Configuration

```python
from wreq import Proxy
from wreq.blocking import Client
from wreq.emulation import Emulation


def main():
    client = Client(
        emulation=Emulation.Firefox133,
        user_agent="wreq",
        proxies=[
            Proxy.http("socks5h://abc:def@127.0.0.1:1080"),
            Proxy.https(url="socks5h://127.0.0.1:1080", username="abc", password="def"),
            Proxy.http(url="http://abc:def@127.0.0.1:1080", custom_http_auth="abcedf"),
            Proxy.all(
                url="socks5h://abc:def@127.0.0.1:1080",
                exclusion="google.com, facebook.com, twitter.com",
            ),
        ],
    )
    resp = client.get("https://api.ip.sb/ip")
    print("Status Code: ", resp.status)
    print("Version: ", resp.version)
    print("Response URL: ", resp.url)
    print("Headers: ", resp.headers)
    print("Content-Length: ", resp.content_length)
    print("Remote Address: ", resp.remote_addr)
    print("Text: ", resp.text())


if __name__ == "__main__":
    main()
```

### Custom Runtime

The blocking client accepts the same `Runtime` as the async client. Without one,
it uses the shared global multi-thread runtime.

```python
from wreq.blocking import Client
from wreq.runtime import Runtime

runtime = Runtime(workers=1, work_steal=False)
with Client(runtime=runtime) as client:
    with client.get("https://httpbin.io/get") as response:
        print(response.text())
```

Network work runs on the selected worker while the calling thread waits.
`client.runtime` is read-only. Closing the client does not shut down a shared
runtime; it cancels pending requests and rejects new ones with
`asyncio.CancelledError`. See [custom runtimes](advanced.md#custom-runtimes) for
configuration and lifetime details.

### Cookies

```python
from wreq.blocking import Client, Method


def main():
    client = Client()
    resp = client.request(Method.GET, "https://www.google.com/")
    for resp in resp.cookies:
        print(f"{resp.name}: {resp.value}")


if __name__ == "__main__":
    main()
```

### Authentication

```python
from wreq.blocking import Client


def main():
    # Basic auth
    resp = Client().get(
        "https://httpbin.io/anything",
        basic_auth=("username", "password"),
    )
    print(resp.text())

    # Bearer token
    resp = Client().get(
        "https://httpbin.io/anything",
        bearer_auth="token",
    )
    print(resp.text())


if __name__ == "__main__":
    main()
```

### JSON and Form Data

```python
from wreq.blocking import Client


def main():
    client = Client()

    # JSON request
    resp = client.post(
        "https://httpbin.io/anything",
        json={"key": "value"},
    )
    print(resp.json())

    # Form data
    resp = client.post(
        "https://httpbin.io/anything",
        form={
            "keyA": "valueA",
            "keyB": "valueB",
            "number": 789,
        },
    )
    print(resp.text())


if __name__ == "__main__":
    main()
```

### Query Parameters

```python
from wreq.blocking import Client


def main():
    client = Client()
    resp = client.get(
        "https://httpbin.io/anything",
        query={
            "keyA": "valueA",
            "keyB": "valueB",
            "number": 789,
        },
    )
    print(resp.text())


if __name__ == "__main__":
    main()
```

### Streaming Response

```python
import sys

from wreq.blocking import Client


def main():
    client = Client()
    resp = client.get("https://httpbin.io/stream/20")
    with resp:
        with resp.stream() as streamer:
            for chunk in streamer:
                if isinstance(chunk, memoryview):
                    sys.stdout.buffer.write(chunk)
                else:
                    print("Trailers:", chunk)


if __name__ == "__main__":
    main()
```

Data chunks are read-only `memoryview` objects that stay valid after the stream is closed. `resp.bytes()` returns the same type. Pass views directly to APIs that accept the buffer protocol; use `bytes(view)` or `view.tobytes()` only when you need a copy.

Do not close a response while another thread is reading its body. `resp.close()` discards the retained body and marks its connection as non-reusable, but does not guarantee an immediate socket shutdown or interrupt an active read. A body transferred by `resp.stream()` belongs to the streamer and needs its own context manager, as shown above.
