import asyncio
import wreq
from wreq import Method


async def main():
    async with wreq.request(Method.GET, url="https://www.google.com/") as resp:
        print("Status Code: ", resp.status)
        print("Version: ", resp.version)
        print("Response URL: ", resp.url)
        print("Headers: ", resp.headers)
        print("Cookies: ", resp.cookies)
        print("Content-Length: ", resp.content_length)
        print("Remote Address: ", resp.remote_addr)
        set_cookie = resp.headers["set-cookie"]
        if set_cookie is not None:
            print("Headers set-cookie: ", str(set_cookie, "latin-1"))

        for key in resp.headers.keys():
            print(str(key, "ascii"))

        for key, value in resp.headers:
            print(f"{str(key, 'ascii')}: {str(value, 'latin-1')}")

        for cookie in resp.cookies:
            print(cookie)


if __name__ == "__main__":
    asyncio.run(main())
