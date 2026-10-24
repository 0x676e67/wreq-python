"""Request extraction regressions; closed clients never poll a network future."""

import asyncio
import gc
import sys
import sysconfig
import threading
from datetime import timedelta

import pytest
import wreq
from wreq.runtime import Runtime


URL = "not a URL"


@pytest.fixture
def closed_clients():
    runtime = Runtime(workers=1)
    clients = (wreq.Client(runtime=runtime), wreq.blocking.Client(runtime=runtime))
    for client in clients:
        client.close()
    try:
        yield clients
    finally:
        for client in clients:
            client.close()


def captured_kwargs(value, fields):
    matches = [
        item
        for item in gc.get_referrers(value)
        if type(item) is dict
        and set(item) == set(fields)
        and any(entry is value for entry in item.values())
    ]
    assert len(matches) == 1
    return matches[0]


class IndexValue:
    def __init__(self):
        self.target = None
        self.calls = 0

    def __index__(self):
        self.calls += 1
        if self.target is not None:
            self.target["headers"] = object()
            self.target["body"] = object()
        return 3


@pytest.mark.asyncio
async def test_single_kwargs_conversion_and_error_timing(closed_clients):
    client, blocking = closed_clients
    for options in (
        {"body": None},
        {"body": b"data"},
        {"json": None},
        {"json": {"answer": 3}},
        {"headers": {"x-test": "value"}},
        {"timeout": timedelta(seconds=1)},
        {"zstd": False},
    ):
        with pytest.raises(asyncio.CancelledError):
            await client.post(URL, **options)
        with pytest.raises(asyncio.CancelledError):
            blocking.post(URL, **options)

    for route in (client, wreq):
        for name in ("complete_nonsense", "\ud800"):
            with pytest.raises(TypeError, match="unexpected keyword argument"):
                route.post(URL, **{name: object()})
        pending = route.post(URL, headers=object())
        with pytest.raises(TypeError):
            await pending
    for route in (blocking, wreq.blocking):
        with pytest.raises(TypeError, match="unexpected keyword argument"):
            route.post(URL, complete_nonsense=object())
        with pytest.raises(TypeError):
            route.post(URL, headers=object())


@pytest.mark.asyncio
async def test_kwargs_callback_mutation_keeps_extraction_order(closed_clients):
    client, _ = closed_clients
    for multiple in (False, True):
        value = IndexValue()
        pending = (
            client.post(URL, json=value, auth=None)
            if multiple
            else client.post(URL, json=value)
        )
        value.target = captured_kwargs(value, ("json", "auth") if multiple else ("json",))
        try:
            # With one field, remaining reaches zero after json conversion.
            # With two, the newly added invalid headers are encountered next.
            with pytest.raises(TypeError if multiple else asyncio.CancelledError):
                await pending
            assert value.calls == 1
        finally:
            value.target = None
            pending.close()


@pytest.mark.asyncio
async def test_captured_kwargs_are_rechecked_on_first_await(closed_clients):
    client, _ = closed_clients
    value = IndexValue()
    pending = client.post(URL, json=value)
    captured = captured_kwargs(value, ("json",))
    captured["complete_nonsense"] = None
    with pytest.raises(TypeError, match="unexpected keyword argument"):
        await pending
    assert value.calls == 0

    value = IndexValue()
    pending = client.post(URL, json=value)
    pending.close()
    assert value.calls == 0


@pytest.mark.asyncio
async def test_string_subclass_keyword_keeps_lookup_fallback(closed_clients):
    client, blocking = closed_clients

    class Keyword(str):
        def __new__(cls, value):
            instance = super().__new__(cls, value)
            instance.comparisons = 0
            return instance

        __hash__ = str.__hash__

        def __eq__(self, other):
            self.comparisons += 1
            return super().__eq__(other)

    key = Keyword("body")
    with pytest.raises(asyncio.CancelledError):
        await client.post(URL, **{key: b"data"})
    assert key.comparisons > 0
    key.comparisons = 0
    with pytest.raises(asyncio.CancelledError):
        blocking.post(URL, **{key: b"data"})
    assert key.comparisons > 0

    key = "".join(("bo", "dy"))
    with pytest.raises(asyncio.CancelledError):
        await client.post(URL, **{key: None})


@pytest.mark.asyncio
@pytest.mark.skipif(
    not sysconfig.get_config_var("Py_GIL_DISABLED"),
    reason="Concurrent native extraction requires free-threaded Python",
)
async def test_kwargs_value_replacement_is_safe_without_gil(closed_clients):
    assert not sys._is_gil_enabled()
    client, _ = closed_clients
    values = [IndexValue() for _ in range(12)]
    pending = [client.post(URL, json=value) for value in values]
    targets = [captured_kwargs(value, ("json",)) for value in values]
    stopped, started = threading.Event(), threading.Event()
    failures = []

    def replace_values():
        try:
            started.set()
            while not stopped.is_set():
                for target in targets:
                    # Preserve the key set: baseline dictionary iteration can
                    # reject size changes; values remain valid json options.
                    target["json"] = None
                    target["json"] = 3
        except BaseException as error:
            failures.append(error)

    thread = threading.Thread(target=replace_values, daemon=True)
    thread.start()
    try:
        assert started.wait(2)
        for operation in pending:
            with pytest.raises(asyncio.CancelledError):
                await operation
    finally:
        stopped.set()
        thread.join(3)
        for operation in pending:
            operation.close()
    assert not thread.is_alive()
    assert not failures
