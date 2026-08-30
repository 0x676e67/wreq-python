import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest
import wreq
from wreq import Client, Proxy


class _Handler(BaseHTTPRequestHandler):
    r"""Answers every request with the label of the server that handled it.

    Used both as an origin server and as a forward proxy: a proxied request is
    answered by the proxy itself, so the label tells which one was reached.
    """

    protocol_version = "HTTP/1.1"

    def do_GET(self):
        body = json.dumps(
            {
                "server": self.server.label,
                "url": self.path,
                "headers": {
                    name.lower(): value for name, value in self.headers.items()
                },
            }
        ).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.send_header("set-cookie", "flavor=chocolate; Path=/")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        pass


@pytest.fixture
def servers():
    r"""An origin server and two proxies, all listening on the loopback interface."""
    servers = {}
    for label in ("origin", "first", "second"):
        server = ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        server.label = label
        threading.Thread(target=server.serve_forever, daemon=True).start()
        servers[label] = server

    yield {
        label: f"http://127.0.0.1:{server.server_address[1]}"
        for label, server in servers.items()
    }

    for server in servers.values():
        server.shutdown()
        server.server_close()


def test_client_proxies(servers):
    client = Client()
    assert client.proxies is None

    client = Client(proxies=[Proxy.all(servers["first"])])
    assert client.proxies is not None
    assert len(client.proxies) == 1

    client.proxies = [Proxy.all(servers["first"]), Proxy.all(servers["second"])]
    assert client.proxies is not None
    assert len(client.proxies) == 2

    client.proxies = None
    assert client.proxies is None


def test_client_proxies_accept_a_single_proxy(servers):
    # A lone `Proxy` is accepted wherever a sequence of them is, and is read back as a
    # single element list.
    client = Client(proxies=Proxy.all(servers["first"]))
    assert client.proxies is not None
    assert len(client.proxies) == 1

    client.proxies = Proxy.all(servers["second"])
    assert client.proxies is not None
    assert len(client.proxies) == 1


@pytest.mark.asyncio
async def test_client_switch_to_a_single_proxy(servers):
    client = Client(proxies=Proxy.all(servers["first"]))

    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "first"

    client.proxies = Proxy.all(servers["second"])
    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "second"


@pytest.mark.asyncio
async def test_client_switch_proxies(servers):
    client = Client(proxies=[Proxy.all(servers["first"])])

    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "first"

    client.proxies = [Proxy.all(servers["second"])]
    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "second"

    client.proxies = None
    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "origin"


@pytest.mark.asyncio
async def test_client_switch_proxies_without_config(servers):
    client = Client()
    client.proxies = [Proxy.all(servers["first"])]

    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "first"


@pytest.mark.asyncio
async def test_client_switch_scheme_specific_proxies(servers):
    client = Client(
        proxies=[Proxy.https(servers["first"]), Proxy.http(servers["second"])]
    )

    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "second"

    client.proxies = [Proxy.http(servers["first"]), Proxy.https(servers["second"])]
    resp = await client.get(servers["origin"])
    async with resp:
        assert (await resp.json())["server"] == "first"


@pytest.mark.asyncio
async def test_client_switch_proxies_keeps_config(servers):
    client = Client(user_agent="wreq-test", proxies=[Proxy.all(servers["first"])])
    client.proxies = [Proxy.all(servers["second"])]

    resp = await client.get(servers["origin"])
    async with resp:
        body = await resp.json()
        assert body["server"] == "second"
        assert body["headers"]["user-agent"] == "wreq-test"


@pytest.mark.asyncio
async def test_client_switch_proxies_keeps_cookie_jar(servers):
    client = Client(cookie_store=True, proxies=[Proxy.all(servers["first"])])

    resp = await client.get(servers["origin"])
    async with resp:
        assert resp.status.is_success()

    client.proxies = [Proxy.all(servers["second"])]
    assert client.cookie_jar is not None
    assert any(cookie.name == "flavor" for cookie in client.cookie_jar.get_all())

    # The cookie stored while going through the first proxy is still sent once the
    # client has been switched over to the second one.
    resp = await client.get(servers["origin"])
    async with resp:
        body = await resp.json()
        assert body["server"] == "second"
        assert body["headers"]["cookie"] == "flavor=chocolate"


@pytest.mark.asyncio
async def test_request_proxy_overrides_client_proxies(servers):
    client = Client(proxies=[Proxy.all(servers["first"])])

    resp = await client.get(servers["origin"], proxy=Proxy.all(servers["second"]))
    async with resp:
        assert (await resp.json())["server"] == "second"


def test_blocking_client_switch_proxies(servers):
    client = wreq.blocking.Client(proxies=[Proxy.all(servers["first"])])
    assert client.proxies is not None
    assert len(client.proxies) == 1

    with client.get(servers["origin"]) as resp:
        assert resp.json()["server"] == "first"

    client.proxies = [Proxy.all(servers["second"])]
    with client.get(servers["origin"]) as resp:
        assert resp.json()["server"] == "second"

    client.proxies = None
    with client.get(servers["origin"]) as resp:
        assert resp.json()["server"] == "origin"
