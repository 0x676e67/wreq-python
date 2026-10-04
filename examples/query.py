import asyncio
import wreq


async def main():
    # Send list of tuples as query parameters
    async with wreq.get(
        "https://httpbin.io/anything",
        query=[
            ("key1", "value1"),
            ("key2", "value2"),
            ("number", 123),
            ("flag", True),
            ("float", 45.67),
        ],
    ) as resp:
        print(await resp.text())

    # OR send dictionary as query parameters
    async with wreq.get(
        "https://httpbin.io/anything",
        query={
            "keyA": "valueA",
            "keyB": "valueB",
            "number": 789,
            "flag": False,
            "float": 12.34,
        },
    ) as resp:
        print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
