import asyncio
import wreq
from wreq.emulation import Emulation


async def main():
    async with wreq.websocket(
        "wss://gateway.discord.gg/",
        emulation=Emulation.Chrome137,
        headers={"Origin": "https://discord.com"},
        # Preserve HTTP/1 case and header order
        orig_headers=[
            "User-Agent",
            "Origin",
            "Host",
            "Accept",
            "Accept-Encoding",
            "Accept-Language",
        ],
    ) as ws:
        msg = await ws.recv()
        if msg is not None:
            print(msg.json())
        await ws.close()


if __name__ == "__main__":
    asyncio.run(main())
