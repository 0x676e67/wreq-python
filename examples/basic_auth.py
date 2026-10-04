import asyncio
import wreq


async def main():
    async with wreq.get(
        "https://httpbin.io/anything",
        basic_auth=("username", "password"),
    ) as resp:
        print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
