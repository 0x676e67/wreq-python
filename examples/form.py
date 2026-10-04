import asyncio
import wreq


async def main():
    client = wreq.Client()

    # use a list of tuples
    async with client.post(
        "https://httpbin.io/anything",
        form=[
            ("key1", "value1"),
            ("key2", "value2"),
            ("number", 123),
            ("flag", True),
            ("float", 45.67),
        ],
    ) as resp:
        print(await resp.text())

    # OR use a dictionary
    async with client.post(
        "https://httpbin.io/anything",
        form={
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
