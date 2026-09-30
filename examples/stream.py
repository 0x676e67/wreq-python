import asyncio
import sys
import wreq
from wreq import Response


async def main():
    resp: Response = await wreq.get("https://httpbin.io/stream/20")
    async with resp:
        async with resp.stream() as streamer:
            async for chunk in streamer:
                if isinstance(chunk, memoryview):
                    sys.stdout.buffer.write(chunk)
                else:
                    print("Trailers:", chunk)
                await asyncio.sleep(0.1)


if __name__ == "__main__":
    asyncio.run(main())
