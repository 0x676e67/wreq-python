import asyncio
import wreq
from wreq import Client, Proxy


async def main():
    # Create a client with multiple proxies
    client = Client(
        proxies=[Proxy.http("socks5h://abc:def@127.0.0.1:6152")],
    )

    # Send request via the client proxy
    async with client.get("https://httpbin.io/anything") as resp:
        print(await resp.text())

    # Send request via custom proxy
    async with wreq.get(
        "https://httpbin.io/anything",
        proxy=Proxy.all(
            url="http://127.0.0.1:6152",
            custom_http_headers={
                "user-agent": "wreq",
                "accept": "*/*",
                "accept-encoding": "gzip, deflate, br",
                "x-proxy": "wreq",
            },
        ),
    ) as resp:
        print(await resp.text())

    # Send request via Unix socket proxy
    async with wreq.get(
        "http://localhost/v1.41/containers/json",
        proxy=Proxy.unix("/var/run/docker.sock"),
    ) as resp:
        print(await resp.text())


if __name__ == "__main__":
    asyncio.run(main())
