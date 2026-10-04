"""Payload and upload chunk cases shared with the Rust wreq benchmark."""

BODY_CASES = {
    1024: 1024,
    10240: 10240,
    65536: 16384,
    131072: 32768,
    1048576: 65536,
    2097152: 131072,
    4194304: 262144,
}
CONCURRENCY_CASES = [10, 50, 100, 150]
STREAM_CHUNK_BYTES = 65536


def upload_chunk_bytes(size):
    """Use the Rust preset, or at most 64 KiB for a custom payload size."""
    return BODY_CASES.get(size, min(size, STREAM_CHUNK_BYTES))


def prepare_chunks(body, body_kind):
    if body_kind != "stream":
        return ()
    chunk_bytes = upload_chunk_bytes(len(body))
    return tuple(
        body[index : index + chunk_bytes] for index in range(0, len(body), chunk_bytes)
    )
