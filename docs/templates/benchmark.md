# HTTPS benchmarks

This benchmark follows the Rust `wreq` benchmark's body contract: HTTPS over
HTTP/1.1 or HTTP/2, **Full** or **Stream** uploads, and responses streamed to EOF.
It compares the recorded `wreq` revision with pinned versions of `ry`,
`pyreqwest`, `httpx`, `aiohttp`, `niquests`, `curl_cffi`, `requests`, and `pycurl`
against a controlled local Rust TLS echo server. Async and blocking clients
have separate comparison tables.

## Latest measurements

These are local measurements retained with the source on `main`. The measured
revision below identifies the code tested; it may differ from this documentation.

{{BENCHMARK_RESULTS}}

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

The default suite tests all seven payloads below with Full and Stream uploads
at concurrency 10, 50, 100, and 150. Each batch has 300 requests, repeated in
three rounds. Earlier snapshots can have a smaller matrix; the result tables
and raw JSON always reflect what was actually measured.

| Upload / echo payload | Stream upload chunk |
| --- | --- |
| 1 KiB | 1 KiB |
| 10 KiB | 10 KiB |
| 64 KiB | 16 KiB |
| 128 KiB | 32 KiB |
| 1 MiB | 64 KiB |
| 2 MiB | 128 KiB |
| 4 MiB | 256 KiB |

The echo server collects the upload before returning the same payload. These
are end-to-end HTTPS throughput measurements, not isolated TLS read/write
measurements, handshake benchmarks, or first-byte latency measurements.
Payload sizes use binary units (KiB/MiB); JSON response throughput uses decimal
MB/s and counts response payload only, not both upload and download traffic.

## Runtime and client differences

`wreq` and `ry` use their default runtimes. `wreq (1 thread)` uses
`Runtime(workers=1, work_steal=False)`. The two `pyreqwest` variants explicitly
select single-thread and multi-thread runtimes. The configuration is part of
the comparison, rather than treating all runtime choices as equivalent.

Blocking clients use a persistent thread pool with independent worker clients
and connection pools. Their tables do not represent the same scheduling or
HTTP/2 multiplexing arrangement as the shared-client async tables. HTTP/2 and
true streamed-upload support also vary between libraries; unsupported
combinations are marked **N/A**, not replaced by HTTP/1.1 or a buffered upload.

Responses use each library's natural streaming interface: `response.stream()`
in `wreq`, `response.stream()` without a minimum chunk size in `ry`, and
`response.body_reader.read_chunk()` with initial read buffering disabled in
`pyreqwest`. Internal read-ahead, allocation, scheduling, and chunk boundaries
can still differ between clients.

Other adapters use natural chunks when the API provides them, or 64 KiB reads
when a read size is required. Read-buffer choices are part of the workload,
not a guarantee that every library processes identical chunks internally.

## Reading the results

Higher RPS means greater throughput for that workload. The result is calculated
from total measured requests divided by total measured time across every round;
it is not an average chosen from the fastest samples. Raw JSON preserves the
individual timings, per-round rates, runtime settings, interpreter information,
source revision, and native artifact hashes.

Other programs can compete for CPU and IO. Resource contention and performance
can vary between runs, so these results are not a
strict regression gate and small differences should not be treated as stable
rankings. Compare repeated measurements with matching configurations before
making performance claims.

Every dataset retains the CPU model, operating system, architecture, and
logical CPU count. Check these before comparing different machines or runs.
Incomplete or timed-out measurements do not create a result snapshot.

## Reproduce and follow updates

See the [benchmark instructions](https://github.com/0x676e67/wreq-python/tree/main/bench)
for local builds and smaller smoke runs. Benchmarks are run locally; GitHub
Actions does not run this suite.

Completed runs retain JSON under
[`bench/data`](https://github.com/0x676e67/wreq-python/tree/main/bench/data).
`latest.json` selects a reviewed full measurement for display. Documentation
builds validate that checked-in dataset and freeze it into the page and its
downloadable JSON, without fetching data from a separate branch. Missing or
invalid data fails the build rather than reusing an earlier generated page.
