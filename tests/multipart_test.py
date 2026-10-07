from pathlib import Path

import pytest
import wreq
from wreq import Multipart, Part
from wreq.runtime import Runtime, Scheduler

client = wreq.Client(tls_info=True)


def assert_form_value(data, key, expected):
    value = data["form"][key]
    if isinstance(value, list):
        assert expected in value
    else:
        assert value == expected


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_reuse_multipart_with_clonable_parts():
    form = Multipart(
        Part(name="a", value="1"),
        Part(name="b", value=b"2"),
        Part(
            name="c", value=Path("./README.md"), filename="README.md", mime="text/plain"
        ),
    )

    for _ in range(3):
        resp = await client.post("https://httpbin.io/post", multipart=form)
        async with resp:
            assert resp.status.is_success()
            data = await resp.json()
            assert_form_value(data, "a", "1")
            assert_form_value(data, "b", "2")
            assert "c" in data["files"]


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_stream_part_is_one_shot_when_reusing_multipart():
    def file_stream(path):
        with open(path, "rb") as f:
            while chunk := f.read(1024):
                yield chunk

    form = Multipart(
        Part(
            name="stream",
            value=file_stream("./README.md"),
            filename="README.md",
            mime="text/plain",
        ),
    )

    resp = await client.post("https://httpbin.io/post", multipart=form)
    async with resp:
        assert resp.status.is_success()

    with pytest.raises(RuntimeError):
        resp = await client.post("https://httpbin.io/post", multipart=form)
        async with resp:
            pass


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_reuse_same_part_without_copy_for_clonable_value():
    part = Part(name="a", value="1")

    form1 = Multipart(part)
    form2 = Multipart(part)

    for form in (form1, form2):
        resp = await client.post("https://httpbin.io/post", multipart=form)
        async with resp:
            assert resp.status.is_success()
            data = await resp.json()
            assert_form_value(data, "a", "1")


@pytest.mark.asyncio
@pytest.mark.flaky(reruns=3, reruns_delay=2)
async def test_reuse_same_part_without_copy_fails_for_stream_value():
    def bytes_stream():
        yield b"hello"

    part = Part(name="stream", value=bytes_stream())
    Multipart(part)

    with pytest.raises(RuntimeError):
        Multipart(part)


@pytest.mark.asyncio
async def test_file_parts_open_when_the_request_is_built(tmp_path):
    url = "http://localhost:8080/post"
    path = tmp_path / "part.txt"
    path.write_bytes(b"file part")
    blocking = wreq.blocking.Client(runtime=Runtime(scheduler=Scheduler.CURRENT_THREAD))

    # A current-thread client opens the file on its own runtime, driven by the caller.
    with blocking.post(url, multipart=Multipart(Part(name="f", value=path))) as resp:
        assert resp.json()["files"]["f"] == "file part"

    # A missing file fails the request rather than building the form.
    form = Multipart(Part(name="f", value=tmp_path / "missing.txt"))
    with pytest.raises(RuntimeError, match="IO error"):
        blocking.post(url, multipart=form)
    with pytest.raises(RuntimeError, match="IO error"):
        await client.post(url, multipart=form)
