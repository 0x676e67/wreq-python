import asyncio
import platform
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from ipaddress import IPv4Address

import pytest

import wreq
from wreq import blocking
from wreq.dns import DnsOptions, LookupIpStrategy


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
                async with client.get(url) as response:
                    body = await response.text()
        assert body == "DNS resolved"

    # Exercise actual hostname resolution, including the default resolver.
    await fetch("localhost", None)
    await fetch("localhost", DnsOptions(system_dns=True))
    await fetch("localhost", DnsOptions(system_dns=False, lookup_ip_strategy=LookupIpStrategy.IPV4_ONLY))

    # Every strategy must initialize and preserve explicit address overrides.
    for strategy in (
        LookupIpStrategy.IPV4_ONLY,
        LookupIpStrategy.IPV6_ONLY,
        LookupIpStrategy.IPV4_AND_IPV6,
        LookupIpStrategy.IPV6_THEN_IPV4,
        LookupIpStrategy.IPV4_THEN_IPV6,
    ):
        options = DnsOptions(system_dns=False, lookup_ip_strategy=strategy)
        options.add_resolve("dns-test.invalid", [IPv4Address("127.0.0.1")])
        await fetch("dns-test.invalid", options)


@pytest.mark.asyncio
async def test_dns_resolve_override(dns_http_server):
    # An override wins over the real address of a resolvable host.
    dns_options = DnsOptions(system_dns=False, lookup_ip_strategy=LookupIpStrategy.IPV4_ONLY)
    dns_options.add_resolve("www.google.com", [IPv4Address("127.0.0.1")])
    async with wreq.Client(dns_options=dns_options, no_proxy=True) as client:
        url = f"http://www.google.com:{dns_http_server}/"
        async with await asyncio.wait_for(client.get(url), 5) as response:
            assert await response.text() == "DNS resolved"


async def invalid_name_resolve_error(client: wreq.Client) -> str:
    # A 64-byte label is not a valid DNS name, so every resolver fails without
    # sending a query, each with its own error.
    async with client:
        with pytest.raises(wreq.exceptions.ConnectionError) as exc_info:
            async with await asyncio.wait_for(client.get(f"http://{'a' * 64}.invalid/"), 5):
                pass
    return str(exc_info.value)


@pytest.mark.asyncio
@pytest.mark.parametrize(
    "use_dns_options", [False, True], ids=["no_kwargs", "dns_options"]
)
async def test_default_dns_resolver_is_getaddrinfo(use_dns_options: bool):
    # Control: hickory rejects the name while parsing it. If this stops holding,
    # the assertions below no longer tell the resolvers apart.
    hickory_client = wreq.Client(dns_options=DnsOptions(system_dns=False))
    assert "LabelBytesTooLong" in await invalid_name_resolve_error(hickory_client)

    client = wreq.Client(dns_options=DnsOptions()) if use_dns_options else wreq.Client()
    error = await invalid_name_resolve_error(client)
    assert "LabelBytesTooLong" not in error
    if platform.system() != "Windows":
        # std's getaddrinfo wrapper prefixes every EAI_* failure with this;
        # Windows reports a localized WSA error instead.
        assert "failed to lookup address information" in error
