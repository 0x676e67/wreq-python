import platform
import subprocess
import sys

import pytest

# https://github.com/0x676e67/wreq-python/discussions/305: a Tokio worker that attaches
# to Python during interpreter shutdown used to hit PyO3's not-initialized panic. wreq
# now detects the unavailable interpreter and reports a wreq error without panicking.

# A request is left pending on a listener that never accepts, then an uncaught error
# shuts the interpreter down. Module teardown runs after Py_IsInitialized() drops to 0;
# `HoldTeardown.__del__` closes the listener there, failing the request so the coroutine
# waker attaches from a Tokio worker, and stalls teardown until that happens.
SCRIPT = """
import asyncio
import socket
import time

import wreq

listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(8)
url = f"http://127.0.0.1:{listener.getsockname()[1]}/"


class HoldTeardown:
    def __init__(self, listener, task):
        self.listener = listener
        self.task = task

    def __del__(self):
        self.listener.close()
        time.sleep(1)


loop = asyncio.new_event_loop()
task = loop.create_task(wreq.Client().get(url))
loop.run_until_complete(asyncio.sleep(0.05))
hold = HoldTeardown(listener, task)
del listener, task
raise RuntimeError("uncaught error while a request is in flight")
"""

# PyPy doesn't guarantee `__del__` runs during interpreter exit, so the trigger may not fire.
pytestmark = pytest.mark.skipif(
    platform.python_implementation() != "CPython",
    reason="relies on CPython running __del__ during module teardown",
)


def test_shutdown_wake_reports_error_without_panic():
    proc = subprocess.run(
        [sys.executable, "-c", SCRIPT],
        capture_output=True,
        text=True,
        timeout=60,
    )
    # Exits through the uncaught RuntimeError, not an abort.
    assert proc.returncode == 1, proc.stderr
    assert "panicked" not in proc.stderr
    assert (
        "wreq: failed to wake a Python coroutine: "
        "The Python interpreter is not available" in proc.stderr
    )
