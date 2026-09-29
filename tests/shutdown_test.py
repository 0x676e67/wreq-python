import platform
import subprocess
import sys

import pytest

# Keep a pending request alive into CPython module teardown, then fail its I/O.
# Upload mode also wakes the Python producer waiting for channel capacity.
SCRIPT = """
import asyncio
import os
import socket
import sys
import threading
from types import FunctionType

import wreq

listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(8)
listener.settimeout(5)
url = f"http://127.0.0.1:{listener.getsockname()[1]}/"


wait_lock = threading.Lock()
wait_lock.acquire()


class HoldTeardown:
    def __init__(self, peer, task):
        self.peer = peer
        self.task = task

    def __del__(self, write=os.write, wait=wait_lock.acquire):
        self.peer.close()
        write(2, b"teardown: connection closed\\n")
        wait(timeout=1)
        write(2, b"teardown: wait complete\\n")


async def chunks():
    chunk = b"x" * (1024 * 1024)
    while True:
        yield chunk


# Do not let the suspended generator retain this module's teardown sentinel.
chunks = FunctionType(chunks.__code__, {})
loop = asyncio.new_event_loop()
client = wreq.Client(proxies=[])
upload = sys.argv[1] == "upload"
task = loop.create_task(
    client.post(url, body=chunks()) if upload else client.get(url)
)
loop.run_until_complete(asyncio.sleep(0.05))
peer, _ = listener.accept()
listener.close()
peer.settimeout(5)
assert peer.recv(4096), "request did not reach the server"
loop.run_until_complete(asyncio.sleep(0.1))
assert not task.done(), "request must remain pending"
if upload:
    assert any(
        getattr(getattr(t.get_coro(), "cr_await", None), "__name__", None) == "send"
        for t in asyncio.all_tasks(loop)
    ), "upload producer must be waiting for channel capacity"
hold = HoldTeardown(peer, task)
del peer, task
raise RuntimeError("uncaught error while a request is in flight")
"""


@pytest.mark.skipif(
    platform.python_implementation() != "CPython",
    reason="requires CPython module teardown to run __del__",
)
@pytest.mark.parametrize("operation", ["request", "upload"])
def test_shutdown_wake_without_panic(operation):
    proc = subprocess.run(
        [sys.executable, "-c", SCRIPT, operation],
        capture_output=True,
        text=True,
        timeout=15,
    )
    assert proc.returncode == 1, proc.stderr
    assert "uncaught error while a request is in flight" in proc.stderr
    assert "teardown: connection closed" in proc.stderr
    assert "teardown: wait complete" in proc.stderr
    assert "panicked" not in proc.stderr
    assert "Exception ignored" not in proc.stderr
