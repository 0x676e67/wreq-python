"""HTTPS adapters for the original Python asyncio benchmark clients."""

from contextlib import asynccontextmanager
import ssl
from urllib.parse import urlsplit

if __package__:
    from .workloads import prepare_chunks
else:
    from workloads import prepare_chunks

CLIENTS = ("httpx", "aiohttp", "niquests", "curl_cffi")
PACKAGE = {client: client for client in CLIENTS}
CAPABILITIES = {
    client: {
        "api": "async",
        "protocols": ["h1"] if client == "aiohttp" else ["h1", "h2"],
        "body_kinds": ["full", "stream"],
    }
    for client in CLIENTS
}
RESPONSE_READ = {
    "httpx": "aiter_raw(): native transport chunks",
    "aiohttp": "iter_any(): available response chunks",
    "niquests": "iter_raw(65536): reads of at most 64 KiB",
    "curl_cffi": "aiter_content(): libcurl callback chunks",
}
RESPONSE_CHUNK_BYTES = 65536
# Match niquests' connection pool and curl_cffi's handle pool to concurrency 150.
POOL_LIMIT = 150


def validate_target(url):
    target = urlsplit(url)
    if (
        target.scheme != "https"
        or target.hostname not in {"127.0.0.1", "localhost", "::1"}
        or target.username is not None
    ):
        raise ValueError("Unverified benchmark TLS is restricted to HTTPS loopback")


def tls_context():
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    context.check_hostname = False
    context.verify_mode = ssl.CERT_NONE
    context.minimum_version = context.maximum_version = ssl.TLSVersion.TLSv1_3
    context.set_alpn_protocols(["http/1.1"])
    return context


def validation(status, actual, protocol, total, expected):
    if status != 200 or actual != protocol or total != expected:
        raise RuntimeError(
            f"Expected 200/{protocol}/{expected} bytes, received "
            f"{status}/{actual}/{total} bytes"
        )
    return {"status": status, "protocol": actual, "response_bytes": total}


async def parts(chunks):
    for chunk in chunks:
        yield chunk


@asynccontextmanager
async def operations(client_id, protocol, url, body, body_kind):
    validate_target(url)
    capability = CAPABILITIES[client_id]
    if (
        protocol not in capability["protocols"]
        or body_kind not in capability["body_kinds"]
    ):
        raise ValueError(f"Unsupported benchmark: {client_id}/{protocol}/{body_kind}")
    # Construct chunks outside timed batches; each request gets a fresh iterator.
    chunks = prepare_chunks(body, body_kind)

    if client_id == "httpx":
        import httpx

        async with httpx.AsyncClient(
            verify=tls_context(),
            trust_env=False,
            http1=protocol == "h1",
            http2=protocol == "h2",
            timeout=None,
            limits=httpx.Limits(max_connections=None, max_keepalive_connections=None),
        ) as client:

            async def post():
                async with client.stream(
                    "POST", url, content=parts(chunks) if chunks else body
                ) as response:
                    total = 0
                    async for chunk in response.aiter_raw():
                        total += len(chunk)
                    actual = {"HTTP/1.1": "h1", "HTTP/2": "h2"}.get(
                        response.http_version
                    )
                    return validation(
                        response.status_code, actual, protocol, total, len(body)
                    )

            yield post

    elif client_id == "aiohttp":
        import aiohttp

        async with aiohttp.ClientSession(
            connector=aiohttp.TCPConnector(ssl=tls_context(), limit=0),
            trust_env=False,
            auto_decompress=False,
            timeout=aiohttp.ClientTimeout(total=None),
        ) as client:

            async def post():
                async with client.post(
                    url, data=parts(chunks) if chunks else body, allow_redirects=False
                ) as response:
                    total = 0
                    async for chunk in response.content.iter_any():
                        total += len(chunk)
                    actual = "h1" if response.version == aiohttp.HttpVersion11 else None
                    return validation(
                        response.status, actual, protocol, total, len(body)
                    )

            yield post

    elif client_id == "niquests":
        import niquests
        from niquests.packages.urllib3 import disable_warnings
        from niquests.packages.urllib3.exceptions import InsecureRequestWarning

        disable_warnings(InsecureRequestWarning)

        async with niquests.AsyncSession(
            verify=False,
            retries=0,
            disable_http1=protocol == "h2",
            disable_http2=protocol == "h1",
            disable_http3=True,
            pool_maxsize=POOL_LIMIT,
            revocation_configuration=None,
            tls_configuration=niquests.TLSConfiguration(
                backend="ssl",
                min_version=ssl.TLSVersion.TLSv1_3,
                max_version=ssl.TLSVersion.TLSv1_3,
            ),
        ) as client:
            client.trust_env = False

            async def post():
                async with await client.post(
                    url,
                    data=parts(chunks) if chunks else body,
                    stream=True,
                    allow_redirects=False,
                ) as response:
                    total = 0
                    async for chunk in await response.iter_raw(RESPONSE_CHUNK_BYTES):
                        total += len(chunk)
                    actual = {11: "h1", 20: "h2"}.get(response.http_version)
                    return validation(
                        response.status_code, actual, protocol, total, len(body)
                    )

            yield post

    elif client_id == "curl_cffi":
        from curl_cffi import CurlHttpVersion, CurlOpt, CurlSslVersion
        from curl_cffi.requests import AsyncSession

        version = (
            CurlHttpVersion.V1_1
            if protocol == "h1"
            else CurlHttpVersion.V2_PRIOR_KNOWLEDGE
        )
        async with AsyncSession(
            max_clients=POOL_LIMIT,
            verify=False,
            trust_env=False,
            allow_redirects=False,
            timeout=None,
            http_version=version,
            curl_options={
                CurlOpt.PROXY: "",
                # libcurl encodes the maximum TLS version in the high 16 bits.
                CurlOpt.SSLVERSION: int(CurlSslVersion.TLSv1_3)
                | (int(CurlSslVersion.TLSv1_3) << 16),
            },
        ) as client:

            async def post():
                async with client.stream(
                    "POST", url, content=parts(chunks) if chunks else body
                ) as response:
                    total = 0
                    async for chunk in response.aiter_content():
                        total += len(chunk)
                    actual = {
                        CurlHttpVersion.V1_1: "h1",
                        CurlHttpVersion.V2_0: "h2",
                    }.get(response.http_version)
                    return validation(
                        response.status_code, actual, protocol, total, len(body)
                    )

            yield post
