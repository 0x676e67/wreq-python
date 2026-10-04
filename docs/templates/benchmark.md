# HTTPS benchmarks

We test Python HTTP clients against a controlled local Rust TLS echo server.
The workloads follow the Rust `wreq` benchmark: HTTPS over HTTP/1.1 or HTTP/2,
with Full or Stream uploads and every response streamed to EOF.

The results compare the recorded `wreq` revision with pinned versions of `ry`,
`pyreqwest`, `httpx`, `aiohttp`, `niquests`, `curl_cffi`, `requests`, and `pycurl`.
Async and blocking clients have separate charts.

## Latest measurements

We keep the local measurements in the repository. Check the revision beside
the chart: it identifies the code tested, which may differ from the docs revision.

{{BENCHMARK_RESULTS}}

{{BENCHMARK_CHARTS}}

!!! note "Reading settings affect throughput"

    Larger read chunks and buffering can make a big difference to large-body
    HTTP/2 throughput. These charts use the configurations saved in the raw JSON;
    they don't show every library's fastest possible configuration. Check the
    [reading settings](#runtime-and-client-differences) when comparing clients.

## What is measured

| Setting | Behavior |
| --- | --- |
| Full upload | One complete in-memory request body |
| Stream upload | An iterator or upload read callback over prepared chunks; sizes are recorded with the results |
| Response | Streamed to EOF; no complete-body aggregation |
| Concurrency | A fixed number of closed-loop workers: each starts its next request after EOF |
| Client lifetime | One isolated process per variant; one reused async client or one blocking client per logical worker, per case and round |
| Timing | Upload, request handling, and complete response consumption; preparation is outside the timer |
| Validation | HTTP status, actual HTTP protocol, and echoed body length are checked |
| Repetition | Case order is shuffled; warm-up batches are excluded from all reported rates |

The default suite tests seven body sizes from 1 KiB to 4 MiB, with Full and
Stream uploads at concurrency 10, 50, 100, and 150. Each batch has 300 requests,
and each case runs for three rounds. Earlier snapshots may cover fewer cases.
The charts and raw JSON show the cases actually measured, including the upload
chunk sizes recorded in the JSON.

The echo server collects the upload before returning the same payload. The
timer therefore measures the complete HTTPS exchange, including response
consumption. It doesn't isolate TLS read/write performance, handshakes or
first-byte latency.

Payload sizes use binary units (KiB/MiB). Response throughput in the JSON uses
decimal MB/s and counts only the response payload, although the timer includes
both the upload and download.

## Runtime and client differences

`wreq (MT)` uses the shared multithreaded runtime, with its worker count set by
available CPU parallelism. `wreq (ST)` uses
`Runtime(workers=1, work_steal=False)`. The ST and MT labels refer to the Rust
runtime, not Python threads. `ry` uses its default runtime; the two `pyreqwest`
variants use single-threaded and multithreaded runtimes.

Blocking clients run in a persistent thread pool. Each worker has its own client
and connection pool, so scheduling and HTTP/2 multiplexing differ from the
shared-client async tests. Support for HTTP/2 and streamed uploads varies by
library. N/A means the API doesn't support that combination; we don't substitute
HTTP/1.1 or a buffered upload.

Responses use each library's natural streaming interface: `response.stream()`
in `wreq`, `response.stream()` without a minimum chunk size in `ry`, and
`response.body_reader.read_chunk()` with initial read buffering disabled in
`pyreqwest`. Internal read-ahead, allocation, scheduling, and chunk boundaries
can still differ between clients.

The remaining adapters use natural chunks where the API provides them, or
64 KiB reads where a size is required. Even with the same read size, libraries
can process different chunk sizes internally.

## Reading the results

RPS is total completed requests divided by total timed seconds across all
rounds. Higher RPS means greater throughput for that workload. We include every
timed sample, not just the fastest ones. The raw JSON keeps individual timings
and per-round rates, along with runtime settings, interpreter information,
the source revision and native artifact hashes.

Other programs on the machine compete for CPU and I/O, and that can change
results between runs. Small differences may be noise. Repeat a measurement with
matching configurations before drawing conclusions; this suite isn't a strict
regression check.

Check the recorded CPU model, operating system, architecture and logical CPU
count when comparing runs or machines. A run only saves a result snapshot after
all its cases finish and pass validation; incomplete or timed-out runs don't
produce one.

## Run the benchmarks yourself

See the [benchmark instructions](https://github.com/0x676e67/wreq-python/tree/main/bench)
for local builds and smaller smoke runs. We run this suite locally, outside
GitHub Actions.

The [local runner](https://github.com/0x676e67/wreq-python/blob/main/bench/run.py)
automatically saves raw JSON, logs and an English report. Completed runs keep
their JSON under
[`bench/data`](https://github.com/0x676e67/wreq-python/tree/main/bench/data).
After reviewing a complete suite, use `--input RUN.json --publish`. For a
complete blocking run, use `--input RUN.json --publish-blocking`. Both build
the docs before selecting the saved snapshot, without measuring again.

Docs builds validate the checked-in dataset and generate the charts, metadata
and frozen JSON copies from it. When a separate `latest-blocking.json` exists,
blocking charts use that run while async charts keep their original measurements.
Each group shows its own revision and collection date. The build doesn't fetch
data from another branch; invalid data stops it rather than reusing an old page.
