"""Versioned benchmark aggregation and complete-result validation."""

from datetime import datetime
from itertools import product
import json
import math
import os
from pathlib import Path
import re
import statistics
import tempfile
from urllib.parse import urlsplit

if __package__:
    from .clients import (
        CAPABILITIES,
        CLIENTS,
        CORE_PACKAGES,
        NATIVE_PACKAGES,
        PACKAGES,
        supports,
    )
    from .workloads import BODY_CASES, CONCURRENCY_CASES
else:
    from clients import (
        CAPABILITIES,
        CLIENTS,
        CORE_PACKAGES,
        NATIVE_PACKAGES,
        PACKAGES,
        supports,
    )
    from workloads import BODY_CASES, CONCURRENCY_CASES

DIMENSIONS = ("client", "protocol", "body_kind", "payload_bytes", "concurrency")
BLOCKING_CLIENTS = tuple(
    client
    for client, capability in CAPABILITIES.items()
    if capability["api"] == "blocking"
)


def require_publish(config, *, blocking=False):
    """Require the complete default suite or its independent blocking matrix."""
    axes = {
        "clients": BLOCKING_CLIENTS if blocking else CLIENTS,
        "protocols": ("h1", "h2"),
        "body_kinds": ("full", "stream"),
        "payload_bytes": BODY_CASES,
        "concurrency": CONCURRENCY_CASES,
    }
    message = (
        f"Publishing requires the complete {'blocking' if blocking else 'default'} matrix, "
        ">=300 requests, >=3 rounds, >=1 warm-up and timed sample"
    )
    try:
        complete = all(
            len(config[key]) == len(values) and set(config[key]) == set(values)
            for key, values in axes.items()
        ) and all(
            type(config[key]) is int and config[key] >= minimum
            for key, minimum in (
                ("requests", 300),
                ("rounds", 3),
                ("warmup", 1),
                ("samples", 1),
            )
        )
    except (KeyError, TypeError) as exc:
        raise ValueError(message) from exc
    if not complete:
        raise ValueError(message)


def rates(requests, seconds, size):
    rps = requests / seconds
    return {
        "total_requests": requests,
        "total_seconds": seconds,
        "rps": rps,
        "mbps": size * rps / 1_000_000,
    }


def aggregate(rows, requests):
    cells = {}
    for row in rows:
        key = tuple(row[k] for k in DIMENSIONS)
        cell = cells.setdefault(
            key,
            {
                **dict(zip(DIMENSIONS, key)),
                "samples": [],
                "warmup": [],
                "rounds": [],
                "validation": row["validation"],
            },
        )
        if any(r["round"] == row["round"] for r in cell["rounds"]):
            raise ValueError(f"Duplicate round {row['round']} for {key}")
        if cell["validation"] != row["validation"]:
            raise ValueError(f"Inconsistent response validation for {key}")
        for name in ("samples", "warmup"):
            cell[name].extend(
                {"round": row["round"], "sample": i, **sample}
                for i, sample in enumerate(row[name], 1)
            )
        seconds = sum(sample["seconds"] for sample in row["samples"])
        cell["rounds"].append(
            {
                "round": row["round"],
                **rates(requests * len(row["samples"]), seconds, row["payload_bytes"]),
            }
        )
    result = []
    for key, cell in sorted(cells.items()):
        cell["samples"].sort(key=lambda s: (s["round"], s["sample"]))
        cell["warmup"].sort(key=lambda s: (s["round"], s["sample"]))
        cell["rounds"].sort(key=lambda s: s["round"])
        cell.update(
            rates(
                requests * len(cell["samples"]),
                sum(s["seconds"] for s in cell["samples"]),
                key[3],
            )
        )
        round_rps = [r["rps"] for r in cell["rounds"]]
        cell["round_rps_cv_pct"] = (
            100 * statistics.pstdev(round_rps) / statistics.mean(round_rps)
        )
        result.append(cell)
    return result


def validate_document(document):
    """Reject incomplete matrices and invalid or inconsistent measured rates."""

    def require(condition, message):
        if not condition:
            raise ValueError(message)

    def integer(value, minimum=1):
        return type(value) is int and value >= minimum

    def finite(value, positive=True):
        return (
            type(value) in (int, float)
            and math.isfinite(value)
            and (value > 0 if positive else value >= 0)
        )

    def check_rates(value, count, seconds, size):
        require(
            value.get("total_requests") == count
            and integer(value.get("total_requests")),
            "Incorrect total_requests",
        )
        for name, expected in rates(count, seconds, size).items():
            require(
                finite(value.get(name))
                and math.isclose(value[name], expected, rel_tol=1e-9),
                f"Incorrect {name}",
            )

    try:
        require(
            type(document["schema_version"]) is int and document["schema_version"] == 1,
            "Unsupported schema_version",
        )
        generated = datetime.fromisoformat(document["generated_at"])
        require(
            generated.utcoffset() is not None
            and generated.utcoffset().total_seconds() == 0,
            "generated_at must use UTC",
        )
        source = document["source"]
        require(
            bool(re.fullmatch(r"[0-9a-f]{40}", source["commit"]))
            and type(source["dirty"]) is bool,
            "Invalid source revision",
        )
        config = document["configuration"]
        axes = (
            config["clients"],
            config["protocols"],
            config["body_kinds"],
            config["payload_bytes"],
            config["concurrency"],
        )
        for axis in axes:
            require(
                type(axis) is list and bool(axis) and len(axis) == len(set(axis)),
                "Invalid or duplicate matrix dimensions",
            )
        require(
            set(axes[1]) <= {"h1", "h2"} and set(axes[2]) <= {"full", "stream"},
            "Unsupported protocol or body kind",
        )
        require(set(axes[0]) <= set(CAPABILITIES), "Unknown client configuration")
        require(
            all(integer(v) for axis in axes[3:] for v in axis),
            "Invalid payload size or concurrency",
        )
        require(
            all(
                integer(config[k])
                for k in (
                    "rounds",
                    "samples",
                    "requests",
                    "server_workers",
                    "stream_chunk_bytes",
                )
            )
            and integer(config["warmup"], 0),
            "Invalid batch configuration",
        )
        require(
            config["requests"] >= max(config["concurrency"]),
            "Requests cannot be below concurrency",
        )
        if "stream_chunk_bytes_by_payload" in config:
            chunks = config["stream_chunk_bytes_by_payload"]
            require(
                type(chunks) is dict
                and set(chunks) == {str(size) for size in config["payload_bytes"]}
                and all(integer(chunk) for chunk in chunks.values()),
                "Invalid stream upload chunk mapping",
            )
        require(
            config["tls_version"] == "1.3" and config["tls_verification"] is False,
            "Unexpected TLS configuration",
        )
        require(
            set(document["clients"]) == set(config["clients"]),
            "Missing client metadata",
        )
        environment = document["environment"]
        require(
            all(
                isinstance(environment[k], str) and environment[k]
                for k in ("python", "implementation", "platform", "machine", "cpu")
            )
            and environment["event_loop"] == "asyncio",
            "Missing benchmark environment",
        )
        require(integer(environment["cpu_count"]), "Invalid CPU count")
        for client_id, client in document["clients"].items():
            require(
                isinstance(client["version"], str)
                and bool(client["version"])
                and isinstance(client["package"], str)
                and client["package"] == PACKAGES[client_id]
                and isinstance(client["native"], list)
                and (bool(client["native"]) or client["package"] not in NATIVE_PACKAGES)
                and isinstance(client["runtime"], dict),
                "Missing native client metadata",
            )
            capability = CAPABILITIES[client_id]
            # The original five-client schema did not include additive API metadata.
            legacy = client_id in CORE_PACKAGES
            require(
                all(
                    client.get(field, value if legacy else None) == value
                    for field, value in capability.items()
                ),
                "Incorrect client API or capability metadata",
            )
            for native in client["native"]:
                require(
                    bool(native["path"])
                    and bool(re.fullmatch(r"[0-9a-f]{64}", native["sha256"])),
                    "Invalid native artifact metadata",
                )
        server = document["server"]
        require(
            server["workers"] == config["server_workers"]
            and bool(re.fullmatch(r"[0-9a-f]{64}", server["sha256"])),
            "Invalid server metadata",
        )
        require(
            len(server["protocols"]) == len(config["protocols"])
            and {s["protocol"] for s in server["protocols"]}
            == set(config["protocols"]),
            "Missing server protocol",
        )
        for info in server["protocols"]:
            url = urlsplit(info["url"])
            require(
                info["workers"] == config["server_workers"]
                and url.scheme == "https"
                and url.hostname in {"localhost", "127.0.0.1", "::1"}
                and url.port is not None
                and url.username is None
                and url.path in {"", "/"}
                and not url.query
                and not url.fragment,
                "Invalid native HTTPS server configuration",
            )
        expected = {key for key in product(*axes) if supports(key[0], key[1], key[2])}
        require(bool(expected), "No supported benchmark combinations")
        seen = set()
        for cell in document["results"]:
            key = tuple(cell[k] for k in DIMENSIONS)
            require(
                key in expected and key not in seen, "Unknown or duplicate result cell"
            )
            require(
                integer(cell["payload_bytes"]) and integer(cell["concurrency"]),
                "Invalid result dimensions",
            )
            seen.add(key)
            require(
                cell["validation"]
                == {
                    "status": 200,
                    "protocol": cell["protocol"],
                    "response_bytes": cell["payload_bytes"],
                },
                "Response validation failed",
            )
            for name, per_round in (
                ("samples", config["samples"]),
                ("warmup", config["warmup"]),
            ):
                entries = cell[name]
                require(len(entries) == config["rounds"] * per_round, f"Missing {name}")
                require(
                    all(integer(s["round"]) and integer(s["sample"]) for s in entries)
                    and {(s["round"], s["sample"]) for s in entries}
                    == set(
                        product(range(1, config["rounds"] + 1), range(1, per_round + 1))
                    ),
                    f"Invalid {name} indices",
                )
                require(
                    all(
                        finite(s["seconds"]) and finite(s["cpu_seconds"], False)
                        for s in entries
                    ),
                    "Invalid measured duration",
                )
            require(
                len(cell["rounds"]) == config["rounds"]
                and all(integer(r["round"]) for r in cell["rounds"])
                and {r["round"] for r in cell["rounds"]}
                == set(range(1, config["rounds"] + 1)),
                "Missing round summaries",
            )
            for summary in cell["rounds"]:
                seconds = sum(
                    s["seconds"]
                    for s in cell["samples"]
                    if s["round"] == summary["round"]
                )
                check_rates(
                    summary,
                    config["requests"] * config["samples"],
                    seconds,
                    cell["payload_bytes"],
                )
            check_rates(
                cell,
                config["requests"] * config["samples"] * config["rounds"],
                sum(s["seconds"] for s in cell["samples"]),
                cell["payload_bytes"],
            )
            round_rps = [r["rps"] for r in cell["rounds"]]
            expected_cv = (
                100 * statistics.pstdev(round_rps) / statistics.mean(round_rps)
            )
            require(
                finite(cell["round_rps_cv_pct"], False)
                and math.isclose(
                    cell["round_rps_cv_pct"], expected_cv, rel_tol=1e-9, abs_tol=1e-12
                ),
                "Incorrect round variation",
            )
        require(seen == expected, "Incomplete benchmark matrix")
    except (KeyError, TypeError, OverflowError, AttributeError, IndexError) as exc:
        raise ValueError(f"Malformed benchmark document: {exc}") from exc


def write_atomic(output, document, *, overwrite=True):
    """Write a complete document; immutable snapshots reject an existing path."""
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            "w",
            encoding="utf-8",
            dir=output.parent,
            prefix=output.name + ".",
            suffix=".tmp",
            delete=False,
        ) as handle:
            temporary = Path(handle.name)
            json.dump(document, handle, indent=2, allow_nan=False)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        if overwrite:
            os.replace(temporary, output)
        else:
            # A link publishes the complete file without replacing a racing writer.
            os.link(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
