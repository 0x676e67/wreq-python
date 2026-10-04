# :rocket: Basic Usage

!!! info "On this page"
    - GET/POST requests
    - Form and JSON
    - Custom headers
    - Query parameters & streaming

This page covers the most common request patterns in wreq: sending GET and POST requests, working with form data and JSON, customizing headers, passing query parameters, and reading streaming responses.
 
---
 
## Making Requests
 
### GET
 
The simplest way to make a request is through the top-level `wreq.get()` shortcut:
 
```python
import asyncio
import wreq
 
async def main():
    async with wreq.get("https://httpbin.org/get") as response:
        print(response.status)
 
asyncio.run(main())
```
 
If you need access to the full response metadata, all of it is available on the response object:
 
```python
async def main():
    async with wreq.get("https://httpbin.org/get") as response:
        print(response.status)
        print(response.version)
        print(response.url)
        print(response.headers)
        print(response.cookies)
        print(response.content_length)
        print(response.remote_addr)
```
 
### POST with JSON
 
Pass a dictionary to the `json` argument. wreq will serialize it and set the `Content-Type` header to `application/json` automatically:
 
```python
async def main():
    async with wreq.post(
        "https://httpbin.org/post",
        json={"key": "value"},
    ) as response:
        print(await response.json())
```
 
---
 
## Form Data
 
To send form-encoded data, use the `form` argument. You can pass either a list of tuples or a dictionary. The list of tuples is useful when you need to send the same key more than once:
 
```python
from wreq import Client
 
async def main():
    client = Client()
 
    # List of tuples — preserves key order and allows duplicate keys
    async with client.post(
        "https://httpbin.org/post",
        form=[
            ("key1", "value1"),
            ("key2", "value2"),
            ("count", 3),
        ],
    ) as response:
        print(await response.text())
 
    # Dictionary — simpler when keys are unique
    async with client.post(
        "https://httpbin.org/post",
        form={
            "key1": "value1",
            "key2": "value2",
            "count": 3,
        },
    ) as response:
        print(await response.text())
```
 
Non-string values such as integers, booleans, and floats are accepted and will be serialized automatically.
 
---
 
## Query Parameters
 
Use the `query` argument to append parameters to the URL. Like `form`, it accepts both a list of tuples and a dictionary:
 
```python
async def main():
    # List of tuples
    async with wreq.get(
        "https://httpbin.org/get",
        query=[
            ("search", "wreq"),
            ("page", 1),
            ("active", True),
        ],
    ) as response:
        print(await response.text())
 
    # Dictionary
    async with wreq.get(
        "https://httpbin.org/get",
        query={
            "search": "wreq",
            "page": 1,
            "active": True,
        },
    ) as response:
        print(await response.text())
```
 
---
 
## Custom Headers
 
wreq uses a [HeaderMap](../api/header.md?h=HeaerMap#wreq.header.HeaderMap) object to represent request headers. Unlike a regular dictionary, `HeaderMap` supports multiple values per key, which some headers such as `Accept` require:
 
```python
from wreq.header import HeaderMap
 
headers = HeaderMap()
 
headers.insert("Content-Type", "application/json")
 
# A header can hold multiple values
headers.append("Accept", "application/json")
headers.append("Accept", "text/html")
 
# Retrieve a single value
content_type = headers.get("Content-Type")
if content_type is not None:
    print(str(content_type, "ascii"))
# application/json
 
# Retrieve all values for a multi-value header
print([str(value, "ascii") for value in headers.get_all("Accept")])
# ['application/json', 'text/html']
```

Header names and values are read-only `memoryview` objects. Decode text with `str(view, encoding)` without first copying it into `bytes`.
 
Pass the `HeaderMap` to any request method via the `headers` argument:
 
```python
response = await wreq.get("https://httpbin.org/headers", headers=headers)
```
 
---
 
## Streaming Responses
 
For large responses, you can read the body incrementally instead of loading it all into memory at once. Use `resp.stream()` as an async iterator:
 
```python
import sys

from wreq import Client, HeaderMap
 
async def main():
    client = Client()
    async with client.get("https://httpbin.org/stream/10") as response:
        async with response.stream() as streamer:
            async for chunk in streamer:
                if isinstance(chunk, memoryview):
                    sys.stdout.buffer.write(chunk)
                elif isinstance(chunk, HeaderMap):
                    print("Trailers:", chunk)
```
 
Data chunks are read-only `memoryview` objects; trailer frames are `HeaderMap` objects. The example writes data directly through the buffer protocol. Each view stays valid after the stream is closed. See [Binary data](../getting-started/quickstart.md#binary-data) for copying and releasing views.

Finish a body read before closing the response. If a read task is still running, cancel it and await its cancellation first: `response.close()` does not cancel active reads or guarantee an immediate socket shutdown. It also marks the connection as non-reusable; leaving `async with response:` instead releases the body and keeps a fully read connection reusable. `response.stream()` transfers the body to the streamer, so manage the streamer with its own context manager as shown above.
 
---
