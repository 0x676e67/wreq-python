import asyncio
import hashlib
import json

import wreq


async def main():
    async with wreq.Client() as client:
        async with client.get("https://httpbin.io/json") as resp:
            # An immutable wreq.Bytes that shares the body without copying.
            data = await resp.bytes()

        # Buffer APIs read it in place, and slices share it too.
        print(hashlib.sha256(data).hexdigest(), data[:16])

        # json.loads() takes only str, bytes or bytearray.
        print(json.loads(data.decode())["slideshow"]["title"])

        # Passing it back as a request body does not copy it.
        async with client.post("https://httpbin.io/anything", body=data) as resp:
            print(resp.status)


if __name__ == "__main__":
    asyncio.run(main())
