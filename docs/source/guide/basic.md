# :rocket: Basic Usage

!!! info "On this page"
    - GET/POST requests
    - Form and JSON
    - Custom headers, and the headers of a client
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
    response = await wreq.get("https://httpbin.org/get")
    print(response.status)
 
asyncio.run(main())
```
 
If you need access to the full response metadata, all of it is available on the response object:
 
```python
async def main():
    response = await wreq.get("https://httpbin.org/get")
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
    response = await wreq.post(
        "https://httpbin.org/post",
        json={"key": "value"},
    )
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
    response = await client.post(
        "https://httpbin.org/post",
        form=[
            ("key1", "value1"),
            ("key2", "value2"),
            ("count", 3),
        ],
    )
    print(await response.text())
 
    # Dictionary — simpler when keys are unique
    response = await client.post(
        "https://httpbin.org/post",
        form={
            "key1": "value1",
            "key2": "value2",
            "count": 3,
        },
    )
    print(await response.text())
```
 
Non-string values such as integers, booleans, and floats are accepted and will be serialized automatically.
 
---
 
## Query Parameters
 
Use the `query` argument to append parameters to the URL. Like `form`, it accepts both a list of tuples and a dictionary:
 
```python
async def main():
    # List of tuples
    response = await wreq.get(
        "https://httpbin.org/get",
        query=[
            ("search", "wreq"),
            ("page", 1),
            ("active", True),
        ],
    )
    print(await response.text())
 
    # Dictionary
    response = await wreq.get(
        "https://httpbin.org/get",
        query={
            "search": "wreq",
            "page": 1,
            "active": True,
        },
    )
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
print(headers.get("Content-Type"))
# application/json
 
# Retrieve all values for a multi-value header
print(list(headers.get_all("Accept")))
# ['application/json', 'text/html']
```
 
Pass the `HeaderMap` to any request method via the `headers` argument:
 
```python
response = await wreq.get("https://httpbin.org/headers", headers=headers)
```
 
### Headers of a client
 
A client sends its own headers with every request. They are given through the `headers`
argument of the constructor, and can be changed at any time afterwards through the
`headers` attribute:
 
```python
from wreq import Client
 
client = Client(headers={"x-api-key": "secret"})
 
# Add a header, or replace the value of one that is already there
client.headers.update({"x-request-id": "1"})
 
# Set, or remove, a single header
client.headers["x-request-id"] = "2"
del client.headers["x-request-id"]
 
# Replace the headers of the client altogether
client.headers = {"x-api-key": "another-secret"}
```
 
Reading the attribute gives back a `HeaderMap` bound to the client, so all of the usual
lookups work on it too:
 
```python
print(client.headers["x-api-key"])
# b'another-secret'
```
 
Every change rebuilds the client, keeping the rest of its configuration and its cookie
jar: requests made afterwards carry the new headers, while the ones already in flight
keep using the previous ones. Only the headers of the client are replaced, so those
coming from an [emulation](emulation.md) are kept.
 
---
 
## Streaming Responses
 
For large responses, you can read the body incrementally instead of loading it all into memory at once. Use `resp.stream()` as an async iterator:
 
```python
from wreq import Client
 
async def main():
    client = Client()
    response = await client.get("https://httpbin.org/stream/10")
 
    async for chunk in response.stream():
        print(chunk.decode("utf-8"))
```
 
Each `chunk` is a `bytes` object. Decode it to a string only if you know the response body is text.
 
---
