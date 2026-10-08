# HTTPS benchmarks

We compare Python HTTP clients against a local Rust TLS echo server, using
HTTPS over HTTP/1.1 and HTTP/2. Each request uploads a body and reads the echoed
response to EOF. Async and blocking clients have separate charts.

The results include `wreq`, `ry`, `pyreqwest`, `httpx`, `aiohttp`, `niquests`,
`curl_cffi`, `requests`, and `pycurl`, at the versions recorded with each run.

## Latest measurements

Choose a workload in the chart below. The revision and date beside each chart
identify the code measured, which may differ from the docs revision.

??? note "Measurement environment"

    Hardware, Python and test settings used for these measurements.

{{BENCHMARK_RESULTS}}

{{BENCHMARK_CHARTS}}

## Reading the results

RPS is the total number of completed requests divided by the total timed seconds
across all rounds. Higher RPS means greater throughput for the selected workload.
All timed samples contribute to the result; warm-up requests are excluded.

Full and Stream describe the upload. Every response is streamed to EOF.
N/A means a client API does not support that combination; the benchmark does
not substitute HTTP/1.1 or a buffered upload.

Compare clients within the same workload. Async clients share one client and
connection pool, while blocking clients use one per worker. Read buffering and
chunk boundaries also differ between libraries, so these results reflect the
[configured adapters](#runtime-and-client-differences), not every library's
fastest possible configuration. Larger read chunks can substantially affect
large-body HTTP/2 throughput.

When comparing runs, check the CPU, operating system, architecture, logical CPU
count and test settings. Other programs can affect CPU and I/O availability.
Short fixed batches can also include worker startup and scheduling overhead.
Small differences may be noise; repeat measurements under matching conditions
before treating a change as a regression.

The raw JSON includes individual timings, per-round rates, runtime settings,
interpreter details, the source revision and native artifact hashes. Payload
sizes use binary KiB/MiB. Response throughput uses decimal MB/s and counts only
the response payload, although the timer covers both upload and download.

## What is measured

The echo server collects the upload before returning the same payload. The
timer covers the complete HTTPS exchange, including response consumption.
It does not isolate TLS reads or writes, handshakes, or first-byte latency.

| Setting | Behavior |
| --- | --- |
| Full upload | One complete in-memory request body |
| Stream upload | An iterator or upload read callback over prepared chunks; chunk sizes are recorded in the JSON |
| Response | Streamed to EOF, without complete-body aggregation |
| Concurrency | A fixed number of workers; each starts its next request after EOF |
| Client lifetime | One isolated process per variant; clients are reused within each case and round |
| Timing | Upload, server handling and complete response consumption; payload preparation is outside the timer |
| Validation | HTTP status, actual protocol and echoed body length are checked |
| Repetition | Case order is shuffled; warm-up batches are excluded from reported rates |

The default suite tests seven body sizes from 1 KiB to 4 MiB, with Full and
Stream uploads at concurrency 2, 10, 50, and 100. Each case runs for three rounds,
with 200 warm-up requests and 300 timed requests per round. Historical snapshots
may use different budgets or cover fewer cases; the charts show the cases
actually recorded.

## Runtime and client differences

### wreq runtimes

| Label | Runtime used in the benchmark |
| --- | --- |
| MT | Shared multithreaded runtime, with workers set by available CPU parallelism |
| ST | `Runtime(scheduler=Scheduler.PER_WORKER, workers=1)`; blocking clients share this runtime within a case |
| CT (blocking only) | Each client has its own current-thread runtime, driven by the calling thread |

These labels describe the Rust runtime. Blocking calls still run concurrently
in a persistent Python thread pool, with one client and connection pool per
logical worker. Async cases reuse one shared client. This changes scheduling
and HTTP/2 multiplexing between the two APIs.

`ry` uses its default runtime. The `pyreqwest` variants use single-threaded and
multithreaded runtimes. Historical charts only show variants present in their
recorded data.

### Response reading

`wreq` uses `response.stream()`. `ry` uses `response.stream()` without a minimum
chunk size, and `pyreqwest` uses `response.body_reader.read_chunk()` with initial
read buffering disabled. The remaining adapters use natural chunks where
available, or 64 KiB reads where the API requires a size.

Internal read-ahead, allocation, scheduling and chunk boundaries can differ
even when libraries use the same requested read size.

## Run the benchmarks yourself

Follow the [benchmark instructions](https://github.com/0x676e67/wreq-python/tree/main/bench)
to build locally and choose a full suite or a smaller smoke run. The suite runs
locally, outside GitHub Actions.

The [runner](https://github.com/0x676e67/wreq-python/blob/main/bench/run.py)
saves logs, raw JSON and an English report. It saves a result snapshot only
after every case finishes and passes validation; incomplete or timed-out runs
do not produce one.

??? note "Publishing results to the docs"

    Completed runs keep their JSON in
    [`bench/data`](https://github.com/0x676e67/wreq-python/tree/main/bench/data).
    After reviewing a complete suite, use `--input RUN.json --publish`.
    For a complete blocking run, use `--input RUN.json --publish-blocking`.
    Both build the docs before selecting the saved snapshot, without measuring again.

    Docs builds validate the checked-in data and generate charts, metadata and
    frozen JSON copies. A separate `latest-blocking.json` supplies the blocking
    charts while async charts keep their own measurements. Each group shows its
    revision and collection date. Builds read local data, without fetching from
    another branch, and stop if the data is invalid.
