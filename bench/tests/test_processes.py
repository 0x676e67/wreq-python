"""Exercise worker protocol failures and owned-child cleanup."""

import asyncio
import pytest

from bench.processes import start_worker


@pytest.mark.parametrize("failure", ["malformed", "error", "startup_error"])
def test_worker_protocol_and_cleanup(tmp_path, failure):
    script = tmp_path / "worker.py"
    script.write_text(
        """import json
from pathlib import Path
import sys

startup_error = sys.argv[-1] == "startup_error"
print(json.dumps({"ok": not startup_error, "metadata": {"client": "test"}, "error": "startup failed"}), flush=True)
for line in sys.stdin:
    request = json.loads(line)
    if request.get("stop"):
        break
    if request.get("failure") == "malformed":
        print("not JSON", flush=True)
    elif request.get("failure") == "error":
        print(json.dumps({"ok": False, "error": "measurement failed"}), flush=True)
    else:
        print(json.dumps({"ok": True, "result": {"echo": request["value"]}}), flush=True)
Path(__file__).with_suffix(".closed").write_text("closed")
""",
        encoding="utf-8",
    )

    async def check():
        if failure == "startup_error":
            with pytest.raises(RuntimeError, match="startup failed"):
                async with start_worker(script, failure):
                    pytest.fail("An unready worker must not be yielded")
        else:
            with pytest.raises(RuntimeError, match="Invalid JSON|measurement failed"):
                async with start_worker(script, "test") as worker:
                    assert worker.metadata == {"client": "test"}
                    assert await worker.measure({"value": 42, "case_timeout": 1}) == {
                        "echo": 42
                    }
                    await worker.measure({"failure": failure, "case_timeout": 1})
            assert worker.process.returncode == 0
        assert script.with_suffix(".closed").read_text() == "closed"

    asyncio.run(check())
