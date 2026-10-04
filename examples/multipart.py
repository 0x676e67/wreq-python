from pathlib import Path
import asyncio
import aiofiles
import wreq
from wreq import Multipart, Part


async def file_to_bytes_stream(file_path):
    async with aiofiles.open(file_path, "rb") as f:
        while chunk := await f.read(1024):
            yield chunk


async def main():
    async with wreq.post(
        "https://httpbin.io/anything",
        multipart=Multipart(
            # Upload text data
            Part(name="def", value="111", filename="def.txt", mime="text/plain"),
            # Upload binary data
            Part(name="abc", value=b"000", filename="abc.txt", mime="text/plain"),
            # Unload file data
            Part(
                name="LICENSE",
                value=Path("LICENSE"),
                filename="LICENSE",
                mime="text/plain",
            ),
            # Upload bytes stream file data
            Part(
                name="README",
                value=file_to_bytes_stream("README.md"),
                filename="README.md",
                mime="text/plain",
            ),
        ),
    ) as resp:
        print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
