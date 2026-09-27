import asyncio
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from ipaddress import IPv4Address

import pytest

import wreq
from wreq import Client, blocking
from wreq.dns import DnsOptions, LookupIpStrategy
from wreq.exceptions import ConnectionError


@pytest.fixture
def dns_http_server():
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            body = b"DNS resolved"
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield server.server_port
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


@pytest.mark.asyncio
@pytest.mark.parametrize("blocking_api", [False, True])
async def test_dns_options(dns_http_server, blocking_api):
    async def fetch(host, options):
        url = f"http://{host}:{dns_http_server}/"
        kwargs = {"no_proxy": True}
        if options is not None:
            kwargs["dns_options"] = options

        if blocking_api:
            def request():
                with blocking.Client(**kwargs) as client:
                    with client.get(url) as response:
                        return response.text()

            body = await asyncio.to_thread(request)
        else:
            async with wreq.Client(**kwargs) as client:
                async with await client.get(url) as response:
                    body = await response.text()
        assert body == "DNS resolved"

    # Exercise actual hostname resolution, including the default resolver.
    await fetch("localhost", None)
    await fetch("localhost", DnsOptions(system_dns=True))
    await fetch("localhost", DnsOptions(lookup_ip_strategy=LookupIpStrategy.IPV4_ONLY))

    # Every strategy must initialize and preserve explicit address overrides.
    for strategy in (
        LookupIpStrategy.IPV4_ONLY,
        LookupIpStrategy.IPV6_ONLY,
        LookupIpStrategy.IPV4_AND_IPV6,
        LookupIpStrategy.IPV6_THEN_IPV4,
        LookupIpStrategy.IPV4_THEN_IPV6,
    ):
        options = DnsOptions(lookup_ip_strategy=strategy)
        options.add_resolve("dns-test.invalid", [IPv4Address("127.0.0.1")])
        await fetch("dns-test.invalid", options)


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_dns_resolve_override():
    dns_options = DnsOptions(lookup_ip_strategy=LookupIpStrategy.IPV4_ONLY)
    dns_options.add_resolve("www.google.com", [IPv4Address("192.168.1.1")])
    client = Client(
        dns_options=dns_options,
    )

    try:
        await client.get("https://www.google.com")
        assert False, "ConnectionError was expected"
    except ConnectionError:
        pass
