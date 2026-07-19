from wreq.blocking import Client
from wreq.emulation import Emulation


def main():
    client = Client(emulation=Emulation.Firefox135)
    with client.get("https://pingly.us.kg/api/all") as resp:
        print("Status Code: ", resp.status)
        print("Version: ", resp.version)
        print("Response URL: ", resp.url)
        print("Headers: ", resp.headers)
        print("Content-Length: ", resp.content_length)
        print("Remote Address: ", resp.remote_addr)
        print("Text: ", resp.text())


if __name__ == "__main__":
    main()
