"""Client descriptions shared by scheduling, adapters, validation and reports.

Keep library imports in adapters: reading this registry must not load clients
or start runtimes in the coordinator process.
"""

from copy import deepcopy
from dataclasses import dataclass, field
from typing import Literal

POOL_LIMIT = 150
RESPONSE_CHUNK_BYTES = 65536


@dataclass(frozen=True)
class ClientSpec:
    package: str
    label: str
    adapter: Literal["core", "async", "blocking"] = "core"
    protocols: tuple[str, ...] = ("h1", "h2")
    body_kinds: tuple[str, ...] = ("full", "stream")
    response_read: str = "Client streaming API to EOF"
    runtime: dict = field(default_factory=lambda: {"kind": "default"})
    pool: dict = field(default_factory=lambda: {"kind": "library default"})
    native_required: bool = True
    color: str = "peer"

    @property
    def api(self):
        return "blocking" if self.adapter == "blocking" else "async"

    def capabilities(self):
        return {
            "api": self.api,
            "protocols": list(self.protocols),
            "body_kinds": list(self.body_kinds),
        }

    def settings(self):
        runtime, pool = deepcopy(self.runtime), deepcopy(self.pool)
        if self.api == "blocking":
            runtime = {
                "kind": "thread_pool",
                "session": "one client per logical worker",
            }
            pool = {"kind": "one client per logical worker"}
            if self.package == "wreq":
                runtime["network"] = deepcopy(self.runtime)
        return {
            "runtime": runtime,
            "pool": pool,
            "response_read": self.response_read,
            **self.capabilities(),
        }


SPECS = {
    "wreq": ClientSpec("wreq", "wreq (MT)", color="wreq"),
    "wreq_st": ClientSpec(
        "wreq",
        "wreq (ST)",
        runtime={"kind": "custom", "workers": 1, "scheduler": "PER_WORKER"},
        color="wreq_st",
    ),
    "pyreqwest_st": ClientSpec(
        "pyreqwest", "pyreqwest (ST)", runtime={"kind": "single_thread"}
    ),
    "pyreqwest_mt": ClientSpec(
        "pyreqwest", "pyreqwest (MT)", runtime={"kind": "multi_thread"}
    ),
    "ry": ClientSpec("ry", "ry (default)"),
    "httpx": ClientSpec(
        "httpx",
        "httpx",
        "async",
        response_read="aiter_raw(): native transport chunks",
        pool={"connection_limit": None},
        native_required=False,
    ),
    "aiohttp": ClientSpec(
        "aiohttp",
        "aiohttp",
        "async",
        protocols=("h1",),
        response_read="iter_any(): available response chunks",
        pool={"connection_limit": None},
        native_required=False,
    ),
    "niquests": ClientSpec(
        "niquests",
        "niquests",
        "async",
        response_read="iter_raw(65536): reads of at most 64 KiB",
        pool={"pool_maxsize": POOL_LIMIT},
        native_required=False,
    ),
    "curl_cffi": ClientSpec(
        "curl_cffi",
        "curl_cffi",
        "async",
        response_read="aiter_content(): libcurl callback chunks",
        pool={"max_clients": POOL_LIMIT},
    ),
    "wreq_blocking": ClientSpec(
        "wreq",
        "wreq (blocking MT)",
        "blocking",
        response_read="stream(): native transport chunks",
        runtime={"kind": "default", "scope": "shared per process"},
        color="wreq",
    ),
    "wreq_blocking_st": ClientSpec(
        "wreq",
        "wreq (blocking ST)",
        "blocking",
        response_read="stream(): native transport chunks",
        runtime={
            "kind": "custom",
            "workers": 1,
            "scheduler": "PER_WORKER",
            "scope": "shared per case",
        },
        color="wreq_st",
    ),
    "wreq_blocking_ct": ClientSpec(
        "wreq",
        "wreq (blocking CT)",
        "blocking",
        response_read="stream(): native transport chunks",
        runtime={"kind": "current_thread", "scope": "one per client"},
        color="wreq_ct",
    ),
    "ry_blocking": ClientSpec(
        "ry",
        "ry (blocking)",
        "blocking",
        response_read="stream(): native transport chunks",
    ),
    "requests": ClientSpec(
        "requests",
        "requests",
        "blocking",
        protocols=("h1",),
        response_read="iter_content(65536): reads of at most 64 KiB",
        native_required=False,
    ),
    "httpx_blocking": ClientSpec(
        "httpx",
        "httpx (blocking)",
        "blocking",
        response_read="iter_raw(): native transport chunks",
        native_required=False,
    ),
    "niquests_blocking": ClientSpec(
        "niquests",
        "niquests (blocking)",
        "blocking",
        response_read="iter_content(65536): reads of at most 64 KiB",
        native_required=False,
    ),
    "curl_cffi_blocking": ClientSpec(
        "curl_cffi",
        "curl_cffi (blocking)",
        "blocking",
        response_read="content_callback: libcurl callback chunks",
    ),
    "pycurl": ClientSpec(
        "pycurl",
        "PycURL",
        "blocking",
        response_read="WRITEFUNCTION: libcurl callback chunks",
    ),
}


def adapter_clients(adapter):
    return tuple(client for client, spec in SPECS.items() if spec.adapter == adapter)


CLIENTS = tuple(SPECS)
CAPABILITIES = {client: spec.capabilities() for client, spec in SPECS.items()}
PACKAGES = {client: spec.package for client, spec in SPECS.items()}
LABELS = {client: spec.label for client, spec in SPECS.items()}
CORE_PACKAGES = {client: SPECS[client].package for client in adapter_clients("core")}
NATIVE_PACKAGES = {spec.package for spec in SPECS.values() if spec.native_required}


def supports(client, protocol, body_kind):
    capability = CAPABILITIES[client]
    return protocol in capability["protocols"] and body_kind in capability["body_kinds"]
