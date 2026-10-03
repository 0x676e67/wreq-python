"""Library adapters; imports and runtimes stay inside isolated workers."""

from contextlib import asynccontextmanager
import hashlib
import importlib
import importlib.machinery
import importlib.metadata
from pathlib import Path
import sys

if __package__:
    from . import async_clients, blocking_clients
    from .workloads import prepare_chunks
else:
    import async_clients
    import blocking_clients
    from workloads import prepare_chunks

CORE_PACKAGES = {
    "wreq": "wreq",
    "wreq_st": "wreq",
    "pyreqwest_st": "pyreqwest",
    "pyreqwest_mt": "pyreqwest",
    "ry": "ry",
}
NATIVE_PACKAGES = {"wreq", "pyreqwest", "ry", "curl_cffi", "pycurl"}
CAPABILITIES = {
    **{
        client: {
            "api": "async",
            "protocols": ["h1", "h2"],
            "body_kinds": ["full", "stream"],
        }
        for client in CORE_PACKAGES
    },
    **async_clients.CAPABILITIES,
    **blocking_clients.CAPABILITIES,
}
CLIENTS = tuple(CAPABILITIES)
PACKAGES = {**CORE_PACKAGES, **async_clients.PACKAGE, **blocking_clients.PACKAGES}


def supports(client, protocol, body_kind):
    capability = CAPABILITIES[client]
    return protocol in capability["protocols"] and body_kind in capability["body_kinds"]


def normalize_version(version):
    value = str(version).upper()
    if value in {"HTTP/1.1", "1.1", "VERSION.HTTP_11", "HTTP_11"}:
        return "h1"
    if value in {"HTTP/2", "HTTP/2.0", "2", "2.0", "VERSION.HTTP_2", "HTTP_2"}:
        return "h2"
    raise ValueError(f"Unsupported response version: {version!r}")


def validate_response(response, protocol):
    status = response.status
    status = status.as_int() if hasattr(status, "as_int") else int(status)
    actual = normalize_version(response.version)
    if status != 200 or actual != protocol:
        raise RuntimeError(f"Expected 200/{protocol}, received {status}/{actual}")
    return {"status": status, "protocol": actual}


def validate_length(validation, total, expected):
    if total != expected:
        raise RuntimeError(
            f"Incomplete echo: expected {expected} bytes, received {total}"
        )
    return {**validation, "response_bytes": total}


def metadata(client):
    package = PACKAGES[client]
    importlib.import_module(package)
    native = []
    for name, imported in list(sys.modules.items()):
        path = getattr(imported, "__file__", None)
        if (
            name.split(".")[0] == package
            and path
            and any(path.endswith(s) for s in importlib.machinery.EXTENSION_SUFFIXES)
        ):
            path = Path(path).resolve()
            native.append(
                {
                    "module": name,
                    "path": str(path),
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                }
            )
    if package in NATIVE_PACKAGES and not native:
        raise RuntimeError(
            f"{package} did not load a native extension; build/install the actual client first"
        )
    try:
        version = importlib.metadata.version(package)
    except importlib.metadata.PackageNotFoundError as exc:
        raise RuntimeError(
            f"Cannot identify the installed {package} distribution"
        ) from exc
    if not version:
        raise RuntimeError(f"Cannot identify the installed {package} version")
    runtime = (
        {"kind": "thread_pool", "session": "one client per logical worker"}
        if CAPABILITIES[client]["api"] == "blocking"
        else (
            {"kind": "custom", "workers": 1, "work_steal": False}
            if client == "wreq_st"
            else (
                {"kind": "single_thread"}
                if client == "pyreqwest_st"
                else (
                    {"kind": "multi_thread"}
                    if client == "pyreqwest_mt"
                    else {"kind": "default"}
                )
            )
        )
    )
    response_read = "Client streaming API to EOF"
    pool = {"kind": "library default"}
    if client in async_clients.CLIENTS:
        response_read = async_clients.RESPONSE_READ[client]
        pool = (
            {"max_clients": async_clients.POOL_LIMIT}
            if client == "curl_cffi"
            else (
                {"pool_maxsize": async_clients.POOL_LIMIT}
                if client == "niquests"
                else {"connection_limit": None}
            )
        )
    elif CAPABILITIES[client]["api"] == "blocking":
        response_read = blocking_clients.RESPONSE_READ[client]
        pool = {"kind": "one client per logical worker"}
    return {
        "package": package,
        "version": version,
        "runtime": runtime,
        "native": native,
        "response_read": response_read,
        "pool": pool,
        **CAPABILITIES[client],
    }


async def parts(chunks):
    for chunk in chunks:
        yield chunk


@asynccontextmanager
async def operations(client_id, protocol, url, body, body_kind):
    if CAPABILITIES[client_id]["api"] != "async" or not supports(
        client_id, protocol, body_kind
    ):
        raise ValueError(
            f"Unsupported async benchmark combination: {client_id}/{protocol}/{body_kind}"
        )
    if client_id in async_clients.CLIENTS:
        async with async_clients.operations(
            client_id, protocol, url, body, body_kind
        ) as post:
            yield post
        return
    # Payloads/chunks are prepared once, outside timed batches. Each upload gets
    # its own iterator; no artificial delay is inserted between chunks.
    chunks = prepare_chunks(body, body_kind)
    if client_id.startswith("wreq"):
        import wreq
        from wreq.runtime import Runtime
        from wreq.tls import TlsVersion

        runtime = (
            Runtime(workers=1, work_steal=False) if client_id == "wreq_st" else None
        )
        client = wreq.Client(
            tls_verify=False,
            https_only=True,
            no_proxy=True,
            http1_only=protocol == "h1",
            http2_only=protocol == "h2",
            tls_min_version=TlsVersion.TLS_1_3,
            tls_max_version=TlsVersion.TLS_1_3,
            **({"runtime": runtime} if runtime is not None else {}),
        )

        async def post():
            response = await client.post(url, body=parts(chunks) if chunks else body)
            validation = validate_response(response, protocol)
            total = 0
            async with response.stream() as stream:
                async for chunk in stream:
                    total += len(chunk)
            return validate_length(validation, total, len(body))

        try:
            yield post
        finally:
            client.close()
    elif client_id.startswith("pyreqwest"):
        from pyreqwest.client import ClientBuilder
        from pyreqwest.http import Url

        builder = (
            ClientBuilder()
            .danger_accept_invalid_certs(True)
            .https_only(True)
            .no_proxy()
            .runtime_multithreaded(client_id == "pyreqwest_mt")
            .min_tls_version("TLSv1.3")
            .max_tls_version("TLSv1.3")
        )
        builder = (
            builder.http1_only()
            if protocol == "h1"
            else builder.http2_prior_knowledge()
        )
        async with builder.build() as client:
            parsed = Url(url)

            async def post():
                request = client.post(parsed)
                request = (
                    request.body_stream(parts(chunks))
                    if chunks
                    else request.body_bytes(body)
                )
                # Natural chunks, not a full-body send() or minimum-size read.
                async with request.streamed_read_buffer_limit(
                    0
                ).build_streamed() as response:
                    validation = validate_response(response, protocol)
                    total = 0
                    while (
                        chunk := await response.body_reader.read_chunk()
                    ) is not None:
                        total += len(chunk)
                    return validate_length(validation, total, len(body))

            yield post
    elif client_id == "ry":
        import ry

        client = ry.Client(
            https_only=True,
            tls_danger_accept_invalid_certs=True,
            tls_danger_accept_invalid_hostnames=True,
            http1_only=protocol == "h1",
            http2_prior_knowledge=protocol == "h2",
            tls_version_min="1.3",
            tls_version_max="1.3",
        )
        parsed = ry.URL(url)

        async def post():
            response = await client.post(parsed, body=parts(chunks) if chunks else body)
            validation = validate_response(response, protocol)
            total = 0
            async for chunk in response.stream():
                total += len(chunk)
            return validate_length(validation, total, len(body))

        try:
            yield post
        finally:
            # ry has no Client.close(); release this case's client and pool.
            del client
    else:
        raise ValueError(f"Unknown client: {client_id}")


def blocking_operations(client_id, protocol, url, body, body_kind, *, chunks=None):
    if CAPABILITIES[client_id]["api"] != "blocking" or not supports(
        client_id, protocol, body_kind
    ):
        raise ValueError(
            f"Unsupported blocking benchmark combination: {client_id}/{protocol}/{body_kind}"
        )
    return blocking_clients.operations(
        client_id, protocol, url, body, body_kind, chunks=chunks
    )
