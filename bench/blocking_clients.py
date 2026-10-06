"""HTTPS adapters owned by individual closed-loop blocking workers."""

from contextlib import contextmanager
import ssl
import warnings

if __package__:
    from .workloads import prepare_chunks
    from .registry import (
        CAPABILITIES as ALL_CAPABILITIES,
        SPECS,
        adapter_clients,
        RESPONSE_CHUNK_BYTES,
    )
else:
    from workloads import prepare_chunks
    from registry import (
        CAPABILITIES as ALL_CAPABILITIES,
        SPECS,
        adapter_clients,
        RESPONSE_CHUNK_BYTES,
    )

CLIENTS = adapter_clients("blocking")
PACKAGES = {client: SPECS[client].package for client in CLIENTS}
CAPABILITIES = {client: ALL_CAPABILITIES[client] for client in CLIENTS}
RESPONSE_READ = {client: SPECS[client].response_read for client in CLIENTS}


def tls_context():
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    context.check_hostname = False
    context.verify_mode = ssl.CERT_NONE
    context.minimum_version = context.maximum_version = ssl.TLSVersion.TLSv1_3
    return context


def check_response(status, actual, protocol, total, expected):
    if status != 200 or actual != protocol or total != expected:
        raise RuntimeError(
            f"Expected 200/{protocol}/{expected} bytes, received {status}/{actual}/{total} bytes"
        )
    return {"status": status, "protocol": actual, "response_bytes": total}


def native_protocol(version):
    value = str(version).upper()
    if value in {"HTTP/1.1", "VERSION.HTTP_11", "HTTP_11"}:
        return "h1"
    if value in {"HTTP/2", "HTTP/2.0", "VERSION.HTTP_2", "HTTP_2"}:
        return "h2"
    return value


class ChunkReader:
    """Feed libcurl a bounded piece without collecting the upload iterator."""

    def __init__(self, chunks):
        self.chunks = iter(chunks)
        self.chunk = memoryview(b"")
        self.offset = 0

    def read(self, amount):
        while self.offset == len(self.chunk):
            chunk = next(self.chunks, None)
            if chunk is None:
                return b""
            self.chunk = memoryview(chunk)
            self.offset = 0
        end = min(self.offset + amount, len(self.chunk))
        result = self.chunk[self.offset : end]
        self.offset = end
        return result


@contextmanager
def operations(client_id, protocol, url, body, body_kind, *, chunks=None, runtime=None):
    """Yield one post callable; its session must never be used concurrently.

    The caller may transfer this worker-owned adapter between threads after a
    previous batch has finished. Shared upload chunks are prepared outside timing.
    """
    capability = CAPABILITIES[client_id]
    if (
        protocol not in capability["protocols"]
        or body_kind not in capability["body_kinds"]
    ):
        raise ValueError(
            f"Unsupported benchmark case: {client_id}/{protocol}/{body_kind}"
        )
    if chunks is None:
        chunks = prepare_chunks(body, body_kind)
    expected = len(body)
    # Verification is deliberately disabled for the controlled self-signed server.
    warnings.filterwarnings(
        "ignore", message="Unverified HTTPS request is being made.*"
    )

    if SPECS[client_id].package == "wreq":
        from wreq.blocking import Client
        from wreq.tls import TlsVersion

        kind = SPECS[client_id].runtime["kind"]
        if kind == "custom" and runtime is None:
            raise ValueError("Blocking ST requires one runtime shared by the case")
        if kind == "current_thread":
            from wreq.runtime import Runtime

            try:
                from wreq.runtime import Scheduler
            except ImportError as error:
                raise ValueError("Blocking CT requires wreq with Scheduler") from error
            # Each logical worker drives its own runtime, like one curl handle each.
            runtime = Runtime(scheduler=Scheduler.CURRENT_THREAD)
        with Client(
            runtime=runtime,
            tls_verify=False,
            https_only=True,
            no_proxy=True,
            http1_only=protocol == "h1",
            http2_only=protocol == "h2",
            tls_min_version=TlsVersion.TLS_1_3,
            tls_max_version=TlsVersion.TLS_1_3,
        ) as client:

            def post():
                response = client.post(
                    url, body=iter(chunks) if body_kind == "stream" else body
                )
                status, version = (
                    response.status.as_int(),
                    native_protocol(response.version),
                )
                total = 0
                with response.stream() as stream:
                    for chunk in stream:
                        total += len(chunk)
                return check_response(status, version, protocol, total, expected)

            yield post
    elif client_id == "ry_blocking":
        import ry

        client = ry.BlockingClient(
            https_only=True,
            tls_danger_accept_invalid_certs=True,
            tls_danger_accept_invalid_hostnames=True,
            http1_only=protocol == "h1",
            http2_prior_knowledge=protocol == "h2",
            tls_version_min="1.3",
            tls_version_max="1.3",
        )
        parsed = ry.URL(url)

        def post():
            response = client.post(
                parsed, body=iter(chunks) if body_kind == "stream" else body
            )
            status, version = response.status, native_protocol(response.version)
            total = sum(len(chunk) for chunk in response.stream())
            return check_response(status, version, protocol, total, expected)

        try:
            yield post
        finally:
            del client
    elif client_id == "requests":
        import requests

        class TLSAdapter(requests.adapters.HTTPAdapter):
            def init_poolmanager(self, *args, **kwargs):
                super().init_poolmanager(*args, ssl_context=tls_context(), **kwargs)

        with requests.Session() as client:
            client.trust_env = False
            client.mount("https://", TLSAdapter())

            def post():
                with client.post(
                    url,
                    data=iter(chunks) if body_kind == "stream" else body,
                    stream=True,
                    verify=False,
                    allow_redirects=False,
                    timeout=60,
                ) as response:
                    total = sum(
                        len(chunk)
                        for chunk in response.iter_content(RESPONSE_CHUNK_BYTES)
                    )
                    version = {11: "h1", 20: "h2"}.get(response.raw.version)
                    return check_response(
                        response.status_code, version, protocol, total, expected
                    )

            yield post
    elif client_id == "httpx_blocking":
        import httpx

        with httpx.Client(
            verify=tls_context(),
            trust_env=False,
            http1=protocol == "h1",
            http2=protocol == "h2",
            timeout=60,
        ) as client:

            def post():
                with client.stream(
                    "POST", url, content=iter(chunks) if body_kind == "stream" else body
                ) as response:
                    total = sum(len(chunk) for chunk in response.iter_raw())
                    return check_response(
                        response.status_code,
                        native_protocol(response.http_version),
                        protocol,
                        total,
                        expected,
                    )

            yield post
    elif client_id == "niquests_blocking":
        import niquests
        from niquests.extensions.tls import TLSConfiguration

        tls = TLSConfiguration(
            min_version=ssl.TLSVersion.TLSv1_3,
            max_version=ssl.TLSVersion.TLSv1_3,
            assert_hostname=False,
        )
        with niquests.Session(
            disable_http1=protocol == "h2",
            disable_http2=protocol == "h1",
            disable_http3=True,
            tls_configuration=tls,
            revocation_configuration=None,
        ) as client:
            client.trust_env = False

            def post():
                with client.post(
                    url,
                    data=iter(chunks) if body_kind == "stream" else body,
                    stream=True,
                    verify=False,
                    allow_redirects=False,
                    timeout=60,
                ) as response:
                    total = sum(
                        len(chunk)
                        for chunk in response.iter_content(RESPONSE_CHUNK_BYTES)
                    )
                    version = {11: "h1", 20: "h2"}.get(response.http_version)
                    return check_response(
                        response.status_code, version, protocol, total, expected
                    )

            yield post
    elif client_id == "curl_cffi_blocking":
        from curl_cffi import CurlHttpVersion, CurlOpt, CurlSslVersion
        from curl_cffi.requests import Session

        version = (
            CurlHttpVersion.V1_1
            if protocol == "h1"
            else CurlHttpVersion.V2_PRIOR_KNOWLEDGE
        )
        # libcurl stores the maximum TLS version in the upper 16 bits.
        tls = int(CurlSslVersion.TLSv1_3)
        with Session(
            verify=False,
            trust_env=False,
            use_thread_local_curl=False,
            http_version=version,
            timeout=60,
            curl_options={
                CurlOpt.SSLVERSION: tls | (tls << 16),
                CurlOpt.PROXY: b"",
                CurlOpt.NOPROXY: b"*",
            },
        ) as client:

            def post():
                total = 0

                def consume(chunk):
                    nonlocal total
                    total += len(chunk)
                    return len(chunk)

                response = client.post(
                    url,
                    content=iter(chunks) if body_kind == "stream" else body,
                    content_callback=consume,
                    allow_redirects=False,
                    accept_encoding="identity",
                )
                actual = {CurlHttpVersion.V1_1: "h1", CurlHttpVersion.V2_0: "h2"}.get(
                    response.http_version
                )
                return check_response(
                    response.status_code, actual, protocol, total, expected
                )

            yield post
    elif client_id == "pycurl":
        import pycurl

        client = pycurl.Curl()
        client.setopt(pycurl.URL, url)
        client.setopt(pycurl.PROXY, "")
        client.setopt(pycurl.NOPROXY, "*")
        client.setopt(pycurl.SSL_VERIFYPEER, 0)
        client.setopt(pycurl.SSL_VERIFYHOST, 0)
        client.setopt(
            pycurl.SSLVERSION, pycurl.SSLVERSION_TLSv1_3 | pycurl.SSLVERSION_MAX_TLSv1_3
        )
        client.setopt(
            pycurl.HTTP_VERSION,
            (
                pycurl.CURL_HTTP_VERSION_1_1
                if protocol == "h1"
                else pycurl.CURL_HTTP_VERSION_2_PRIOR_KNOWLEDGE
            ),
        )
        client.setopt(pycurl.FOLLOWLOCATION, 0)
        client.setopt(pycurl.TIMEOUT, 60)
        client.setopt(
            pycurl.HTTPHEADER, ["Content-Type: application/octet-stream", "Expect:"]
        )
        client.setopt(pycurl.POST, 1)
        if body_kind == "full":
            client.setopt(pycurl.POSTFIELDS, body)
        else:
            client.setopt(pycurl.POSTFIELDSIZE_LARGE, -1)

        def post():
            total = 0

            def consume(chunk):
                nonlocal total
                total += len(chunk)
                return len(chunk)

            client.setopt(pycurl.WRITEFUNCTION, consume)
            if body_kind == "stream":
                reader = ChunkReader(chunks)
                client.setopt(pycurl.READFUNCTION, reader.read)
            client.perform()
            actual = {
                pycurl.CURL_HTTP_VERSION_1_1: "h1",
                pycurl.CURL_HTTP_VERSION_2_0: "h2",
            }.get(client.getinfo(pycurl.INFO_HTTP_VERSION))
            return check_response(
                client.getinfo(pycurl.RESPONSE_CODE), actual, protocol, total, expected
            )

        try:
            yield post
        finally:
            client.close()
