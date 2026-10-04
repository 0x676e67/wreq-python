import asyncio
import wreq


async def main():
    async with wreq.post(
        "https://httpbin.io/anything",
        json={"key": "value"},
    ) as resp:
        print(await resp.json())


if __name__ == "__main__":
    asyncio.run(main())
