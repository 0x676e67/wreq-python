# WebSocket

!!! info "On this page"
    - HTTP/1.1 WebSocket
    - HTTP/2 WebSocket

`Message.data`, `.binary`, `.ping`, and `.pong` return read-only `memoryview` objects when present. The views remain valid after the message is deleted or the connection is closed. Use `bytes(view)` when a consumer requires a `bytes` object; `Message.text` still returns a string.

### HTTP/1.1 WebSocket Connection

```python
import asyncio
import datetime
import wreq
from wreq import Message, WebSocket, Version
from wreq import exceptions


async def send_message(ws: WebSocket):
    print("Starting to send messages...")
    for i in range(20):
        print(f"Sending: Message {i + 1}")
        await ws.send(Message.from_text(f"Message {i + 1}"))
        await asyncio.sleep(0.1)


async def recv_message(ws: WebSocket):
    print("Starting to receive messages...")
    while True:
        try:
            message = await ws.recv(timeout=datetime.timedelta(milliseconds=10))
            print("Received: ", message)
            if message is None:
                print("Connection closed by server.")
                break

            if message.data == b"Message 20":
                print("Closing connection...")
                break
        except exceptions.TimeoutError:
            continue


async def main():
    client = wreq.Client()
    ws: WebSocket = await client.websocket("wss://echo.websocket.org")
    async with ws:
        print("Status Code: ", ws.status)
        print("Version: ", ws.version)
        print("Headers: ", ws.headers)
        print("Remote Address: ", ws.remote_addr)

        if ws.status == 101:
            print("WebSocket connection established successfully.")
            send_task = asyncio.create_task(send_message(ws))
            receive_task = asyncio.create_task(recv_message(ws))
            await asyncio.gather(send_task, receive_task)


if __name__ == "__main__":
    asyncio.run(main())
```

### HTTP/2 WebSocket Connection

```python
import asyncio
import datetime
import wreq
from wreq import Message, WebSocket
from wreq import exceptions


async def send_message(ws):
    print("Starting to send messages...")
    for i in range(20):
        print(f"Sending: Message {i + 1}")
        await ws.send(Message.from_text(f"Message {i + 1}"))
        await asyncio.sleep(0.1)


async def recv_message(ws):
    print("Starting to receive messages...")
    while True:
        try:
            message = await ws.recv(timeout=datetime.timedelta(milliseconds=10))
            print("Received: ", message)
            if message is None:
                print("Connection closed by server.")
                break

            if message.data == b"Message 20":
                print("Closing connection...")
                break
        except exceptions.TimeoutError:
            continue


async def main():
    # Connect to HTTP/2 WebSocket server
    client = wreq.Client(tls_verify=False)
    ws: WebSocket = await client.websocket(
        "wss://127.0.0.1:3000/ws",
        version=Version.HTTP_2
    )
    async with ws:
        print("Status Code: ", ws.status)
        print("Version: ", ws.version)
        print("Headers: ", ws.headers)
        print("Remote Address: ", ws.remote_addr)

        if ws.status == 200:
            print("WebSocket connection established successfully.")
            send_task = asyncio.create_task(send_message(ws))
            receive_task = asyncio.create_task(recv_message(ws))
            await asyncio.gather(send_task, receive_task)


if __name__ == "__main__":
    asyncio.run(main())
```

!!! note "HTTP/2 WebSocket Testing"
    To test HTTP/2 WebSocket, you need to run a local server:
    
    ```bash
    git clone https://github.com/tokio-rs/axum && cd axum
    cargo run -p example-websockets-http2
    ```
    
    Then run the Python script to connect to the websocket server.
