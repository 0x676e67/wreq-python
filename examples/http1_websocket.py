import asyncio
import datetime
import signal
import wreq
from wreq import Message, WebSocket
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
    async with client.websocket("wss://echo.websocket.org") as ws:
        print("Status Code: ", ws.status)
        print("Version: ", ws.version)
        print("Headers: ", ws.headers)
        print("Remote Address: ", ws.remote_addr)

        if ws.status == 101:
            print("WebSocket connection established successfully.")
            send_task = asyncio.create_task(send_message(ws))
            receive_task = asyncio.create_task(recv_message(ws))

            async def close():
                send_task.cancel()
                receive_task.cancel()

            loop = asyncio.get_running_loop()
            for sig in (signal.SIGINT, signal.SIGTERM):
                loop.add_signal_handler(sig, lambda: asyncio.create_task(close()))

            await asyncio.gather(send_task, receive_task)


if __name__ == "__main__":
    asyncio.run(main())
