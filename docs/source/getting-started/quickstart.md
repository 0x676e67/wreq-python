# Quickstart

This page covers the basics of making HTTP requests with wreq. By the end, you will be able to send requests, read responses, pass headers, work with JSON, and route traffic through a proxy.

---

## Making a Request

wreq supports both async and blocking usage. The async client is the default and recommended approach for most use cases.

=== "Async"
    ```python
    import asyncio
    from wreq import Client

    async def main():
        client = Client()
        async with client.get("https://httpbin.org/get") as response:
            print(response.status)

    asyncio.run(main())
    ```

=== "Blocking"
    ```python
    from wreq.blocking import Client

    client = Client()
    response = client.get("https://httpbin.org/get")
    print(response.status)
    ```

The same interface works for all standard HTTP methods:

```python
response = await client.post("https://httpbin.org/post")
response = await client.put("https://httpbin.org/put")
response = await client.delete("https://httpbin.org/delete")
response = await client.head("https://httpbin.org/get")
```

---

## Passing Parameters in URLs

To append query parameters to a URL, pass a dictionary to the `query` argument:

```python
query = {"search": "wreq", "page": "1"}
async with client.get("https://httpbin.org/get", query=query) as response:
    print(response.url)
    # https://httpbin.org/get?search=wreq&page=1
```

---

## Reading the Response

### Status code

```python
async with client.get("https://httpbin.org/get") as response:
    print(response.status)
    # 200
```

### Text

```python
text = await response.text()
print(text)
```

### JSON

If the server returns a JSON body, parse it directly with `.json()`:

```python
async with client.get("https://httpbin.org/json") as response:
    data = await response.json()
    print(data)
```

### Binary data

`response.bytes()` returns a read-only `memoryview`, not a `bytes` object. The view shares Rust-owned data without a copy into Python bytes. Reading a complete response can still allocate memory to combine body chunks.

```python
import hashlib

async with response:
    view = await response.bytes()
print(view.readonly)  # True; releasing the response does not invalidate the view
print(hashlib.sha256(view).hexdigest())  # Reads the buffer directly
```

The blocking API returns the same type, without `await`. Stream data frames, WebSocket binary fields, header names and values, and peer certificates also return read-only memoryviews. Each view retains its backing data even after the source object is closed or deleted.

Use views directly with APIs that accept the buffer protocol, such as `file.write(view)` or `hashlib.sha256(view)`. For text, `str(view, "utf-8")` decodes into a string without an intermediate `bytes` object.

#### Copying data

Only convert when you need an independent `bytes` object or an API requires one:

```python
data = bytes(view)  # Copies the data; view.tobytes() also copies
view.release()
```

This changes the binary return type. `memoryview` has no `.decode()` method or byte-string concatenation. When you finish using a view, you can call `view.release()`; this does not release other views or slices sharing the data. Input types are unchanged.

Built-in `bytes` and `str` inputs can share their storage. Their subclasses are copied from the actual contents to avoid hidden reference cycles; deleting a view releases its ownership normally, without requiring an explicit `release()`.

When passing a view back to wreq's binary inputs (`body`, `Part`, `Message` constructors, or `CertStore`), convert it with `bytes(view)`. These inputs do not treat a memoryview as binary data.

### Response headers

Response headers are available as a [HeaderMap](../api/header/?h=HeaerMap#wreq.header.HeaderMap) object:

```python
content_type = response.headers.get("content-type")
if content_type is not None:
    print(str(content_type, "ascii"))
# application/json
```

---

## Sending Data

### JSON body

Pass a dictionary to the `json` argument and wreq will serialize it and set the correct `Content-Type` header automatically:

The [Proxy](../guide/proxy.md) object defines how your HTTP client routes traffic through a proxy server.
You create one using a named constructor that specifies the scope:

- `Proxy.all(url)` - routes all traffic through the given proxy
- `Proxy.http(url)` - only intercepts HTTP requests
- `Proxy.https(url)` - only intercepts HTTPS requests

Once created, you pass it to the `Client` and all requests will go through it automatically.

```python
payload = {"name": "John", "age": 30}
async with client.post("https://httpbin.org/post", json=payload) as response:
    result = await response.json()
    print(result)
```

### Form-encoded data

To send HTML form data, use the `form` argument instead:

```python
form = {"username": "john", "password": "secret"}
response = await client.post("https://httpbin.org/post", form=form)
```

---

## Custom Headers

Pass a [HeaderMap](../api/header.md?h=HeaderMap#wreq.header.HeaderMap) to attach additional headers to a request:

```python
from wreq.header import HeaderMap

headers = HeaderMap()
headers["User-Agent"] = "MyApp/1.0"
headers["Accept"] = "application/json"

async with client.get("https://httpbin.org/headers", headers=headers) as response:
    print(await response.text())
```

---

## Using Proxies

The [Proxy](../guide/proxy.md) object controls how the client routes traffic. You create one using a named constructor that defines its scope:

- `Proxy.all(url)` routes all traffic through the proxy.
- `Proxy.http(url)` only intercepts plain HTTP requests.
- `Proxy.https(url)` only intercepts HTTPS requests.

Pass the proxy to the `Client` and every subsequent request will use it:

```python
from wreq import Client, Proxy

client = Client(proxies=[Proxy.all("http://proxy.example.com:8080")])
async with client.get("https://httpbin.org/ip") as response:
    print(await response.text())
```

---

## Browser Emulation

wreq can emulate the TLS fingerprint and headers of real browsers, which is useful when connecting to servers that inspect these signals. Pass an [Emulation](../guide/emulation.md) preset to the `Client`:

```python
from wreq import Client, Emulation

client = Client(emulation=Emulation.Safari26)
async with client.get("https://tls.peet.ws/api/all") as response:
    print(await response.text())
```

Available presets are listed in the [Emulation reference](../getting-started/introduction.md#behavior).

---

## Error Handling

Check the status code manually, or call `raise_for_status()` to raise an exception on any 4xx or 5xx response:

```python
async with client.get("https://httpbin.org/status/404") as response:
    try:
        response.raise_for_status()
    except Exception as exc:
        print(f"Request failed with status {response.status}")
```

---

## Next Steps

- See the [Examples](../guide/basic.md) for more code samples
- Explore the [API Reference](../api/wreq.md) for detailed documentation
