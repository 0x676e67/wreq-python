from wreq.blocking import Client
from wreq.proxy import Proxy


def main():
    client = Client()
    resp = client.post(
        "https://httpbin.io/anything",
        proxy=Proxy.all("http://127.0.0.1:6152"),
    )
    print(resp.text())

    # Set the proxy of the client, so every request that follows goes through it
    client.proxies = Proxy.all("http://127.0.0.1:6152")
    resp = client.post("https://httpbin.io/anything")
    print(resp.text())


if __name__ == "__main__":
    main()
