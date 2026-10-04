import asyncio
import wreq


async def main():
    async with wreq.get(
        "https://httpbin.io/anything",
        bearer_auth="token",
    ) as resp:
        print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
