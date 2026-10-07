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
    from .registry import (
        CAPABILITIES,
        CLIENTS,
        CORE_PACKAGES,
        NATIVE_PACKAGES,
        PACKAGES,
        SPECS,
        supports,
    )
else:
    import async_clients
    import blocking_clients
    from workloads import prepare_chunks
    from registry import (
        CAPABILITIES,
        CLIENTS,
        CORE_PACKAGES,
        NATIVE_PACKAGES,
        PACKAGES,
        SPECS,
        supports,
    )


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
    return {
        "package": package,
        "version": version,
        "native": native,
        **SPECS[client].settings(),
    }


async def parts(chunks):
    for chunk in chunks:
        yield chunk


def network_runtime(client):
    """Create a custom wreq runtime before validation or timing starts."""
    spec = SPECS[client]
    if spec.package != "wreq" or spec.runtime["kind"] != "custom":
        return None
    from wreq.runtime import Runtime

    workers, scheduler = spec.runtime["workers"], spec.runtime["scheduler"]
    try:
        from wreq.runtime import Scheduler
    except ImportError:
        # Releases before Scheduler choose it with `work_steal`.
        return Runtime(workers=workers, work_steal=scheduler == "WORK_STEALING")
    return Runtime(scheduler=getattr(Scheduler, scheduler), workers=workers)


@asynccontextmanager
async def operations(client_id, protocol, url, body, body_kind):
    if CAPABILITIES[client_id]["api"] != "async" or not supports(
        client_id, protocol, body_kind
    ):
        raise ValueError(
            f"Unsupported async benchmark combination: {client_id}/{protocol}/{body_kind}"
        )
    if SPECS[client_id].adapter == "async":
        async with async_clients.operations(
            client_id, protocol, url, body, body_kind
        ) as post:
            yield post
        return
    # Payloads/chunks are prepared once, outside timed batches. Each upload gets
    # its own iterator; no artificial delay is inserted between chunks.
    chunks = prepare_chunks(body, body_kind)
    if SPECS[client_id].package == "wreq":
        import wreq
        from wreq.tls import TlsVersion

        runtime = network_runtime(client_id)
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
    elif SPECS[client_id].package == "pyreqwest":
        from pyreqwest.client import ClientBuilder
        from pyreqwest.http import Url

        builder = (
            ClientBuilder()
            .danger_accept_invalid_certs(True)
            .https_only(True)
            .no_proxy()
            .runtime_multithreaded(SPECS[client_id].runtime["kind"] == "multi_thread")
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


def blocking_operations(
    client_id, protocol, url, body, body_kind, *, chunks=None, runtime=None
):
    if CAPABILITIES[client_id]["api"] != "blocking" or not supports(
        client_id, protocol, body_kind
    ):
        raise ValueError(
            f"Unsupported blocking benchmark combination: {client_id}/{protocol}/{body_kind}"
        )
    return blocking_clients.operations(
        client_id, protocol, url, body, body_kind, chunks=chunks, runtime=runtime
    )
