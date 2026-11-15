import asyncio
import sys
import wreq


async def main():
    async with wreq.get("https://httpbin.io/stream/20") as resp:
        async with resp.stream() as streamer:
            async for chunk in streamer:
                if isinstance(chunk, wreq.HeaderMap):
                    print("Trailers:", chunk)
                else:
                    sys.stdout.buffer.write(chunk)
                await asyncio.sleep(0.1)


if __name__ == "__main__":
    asyncio.run(main())
