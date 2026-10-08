# WebSockets

Open a connection with `client.websocket()`. Use its context manager to close
the connection when the exchange finishes, including when an exception occurs.

## Sending and receiving messages

This example sends a text message to a public echo service. The service may
send a greeting first, so it reads until the echoed text arrives. The overall
wait is limited to 10 seconds.

```python
import asyncio
from wreq import Client, Message


async def main():
    async with Client() as client:
        async with client.websocket("wss://echo.websocket.org") as ws:
            print(ws.status, ws.version, ws.protocol)
            async with asyncio.timeout(10):
                await ws.send(Message.from_text("Hello from wreq"))
                while True:
                    message = await ws.recv()
                    if message is None or message.close is not None:
                        break
                    if message.text == "Hello from wreq":
                        print(message.text)
                        break


asyncio.run(main())
```

`recv()` returns a `Message`, or `None` when the peer has closed the stream.
Inspect `message.text`, `message.binary`, or `message.close` to distinguish
the message type. When a close frame supplies a status, `.close` contains a
`(code, reason)` pair.

Use `Message.from_binary(data)` to send binary data. `send_all([...])` sends
a sequence of messages in order. To choose the closing status, call
`await ws.close(code=1000, reason="Finished")` before leaving the context.

Binary payload access is zero-copy: `Message.data`, `.binary`, `.ping`, and
`.pong` return read-only `memoryview` objects when present. The views remain
valid after the message is deleted or the connection closes. Use `bytes(view)`
when another API requires a `bytes` object; `Message.text` returns a string.

## Receive timeouts

For a timeout on a single receive, pass `datetime.timedelta`. It raises
`wreq.exceptions.TimeoutError`; you can receive again if your application
should keep waiting. The following fragment belongs inside an open WebSocket
context in an async function:

```python
from datetime import timedelta
from wreq import exceptions

try:
    message = await ws.recv(timeout=timedelta(seconds=5))
except exceptions.TimeoutError:
    print("No message arrived within five seconds")
```

`asyncio.timeout()` in the first example limits the whole exchange and raises
Python's built-in `TimeoutError` when that deadline expires.

## Blocking connections

Use `wreq.blocking.Client` with `with` and ordinary method calls. This example
also requires access to the public echo service.

```python
from datetime import timedelta

from wreq import Message
from wreq.blocking import Client

with Client() as client:
    with client.websocket("wss://echo.websocket.org") as ws:
        ws.send(Message.from_text("Hello from wreq"))
        for _ in range(10):
            message = ws.recv(timeout=timedelta(seconds=5))
            if message is None or message.close is not None:
                break
            if message.text == "Hello from wreq":
                print(message.text)
                break
```

Use the default runtime for connections that need background processing
between calls. A `CURRENT_THREAD` runtime only drives connection work while a
blocking call is running; see [Runtimes](runtime.md).

## Handshake options

`client.websocket()` accepts `headers`, `cookies`, `basic_auth`, `bearer_auth`,
`proxy`, and `emulation` much like an HTTP request. Use `protocols=["chat"]`
to offer subprotocols and inspect `ws.protocol` for the server's selection.

`max_message_size` and `max_frame_size` bound incoming data in bytes. They
default to 64 MiB per message and 16 MiB per frame. These options are passed
to `websocket()`, together with the handshake options.

## HTTP/2 connections

Pass `version=Version.HTTP_2` to use WebSockets over HTTP/2. The server must
support extended CONNECT and advertise that support; an ordinary HTTP/1.1
WebSocket endpoint is not sufficient. A successful HTTP/2 handshake returns
status 200, while an HTTP/1.1 upgrade returns 101.

The function below expects the URL of a compatible echo server with a trusted
TLS certificate. Call it from an async function with your server's URL. For a
private certificate authority, configure `Client(tls_verify="ca.pem")` with
its PEM bundle.

```python
from datetime import timedelta
from wreq import Client, Message, Version


async def echo_http2(url: str):
    async with Client() as client:
        async with client.websocket(url, version=Version.HTTP_2) as ws:
            print(ws.status, ws.version)
            await ws.send(Message.from_text("Hello over HTTP/2"))
            message = await ws.recv(timeout=timedelta(seconds=5))
            if message is not None:
                print(message.text)
```
