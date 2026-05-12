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
        response = await client.get("https://httpbin.org/get")
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
response = await client.get("https://httpbin.org/get", query=query)
print(response.url)
# https://httpbin.org/get?search=wreq&page=1
```

---

## Reading the Response

### Status code

```python
response = await client.get("https://httpbin.org/get")
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
response = await client.get("https://httpbin.org/json")
data = await response.json()
print(data)
```

### Response headers

Response headers are available as a [HeaderMap](../api/header/?h=HeaerMap#wreq.header.HeaderMap) object:

```python
print(response.headers.get("content-type"))
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
response = await client.post("https://httpbin.org/post", json=payload)
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

response = await client.get("https://httpbin.org/headers", headers=headers)
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
response = await client.get("https://httpbin.org/ip")
print(await response.text())
```

---

## Browser Emulation

wreq can emulate the TLS fingerprint and headers of real browsers, which is useful when connecting to servers that inspect these signals. Pass an [Emulation](../guide/emulation.md) preset to the `Client`:

```python
from wreq import Client, Emulation

client = Client(emulation=Emulation.Safari26)
response = await client.get("https://tls.peet.ws/api/all")
print(await response.text())
```

Available presets are listed in the [Emulation reference](../getting-started/introduction.md#behavior).

---

## Error Handling

Check the status code manually, or call `raise_for_status()` to raise an exception on any 4xx or 5xx response:

```python
response = await client.get("https://httpbin.org/status/404")

try:
    response.raise_for_status()
except Exception as exc:
    print(f"Request failed with status {response.status}")
```

---

## Next Steps

- See the [Examples](../guide/basic.md) for more code samples
- Explore the [API Reference](../api/wreq.md) for detailed documentation
