# HTTPS benchmarks

This suite compares Python HTTP clients over HTTPS HTTP/1.1 and HTTP/2. It
follows the workload model of the Rust wreq benchmark: Full or Stream request
bodies, with every echoed response streamed to EOF. Async and blocking results
are reported separately.

## Workload

- Async: one reused client per case, with 10, 50, 100, or 150 closed-loop workers.
- Blocking: one reused client per logical worker, using a persistent thread
  pool. Submit a whole worker batch, not one executor task per request.
- Seven payload and upload-chunk cases matching the Rust benchmark, listed below.
- A local Rust TLS 1.3 echo server, restricted to the selected protocol by ALPN.
- Async clients: wreq default runtime, wreq single worker, pyreqwest
  single-threaded and multithreaded runtimes, ry, httpx, aiohttp, niquests, and
  curl_cffi.
- Blocking variants: wreq MT/ST, ry, requests, httpx, niquests, curl_cffi, and pycurl.
- Three rounds, each with one warmup and one timed batch of 300 requests.
  Case order is shuffled with a recorded seed.
- Status, negotiated HTTP version, and total response length are checked for
  every request. Unsupported protocol or upload combinations are reported as
  N/A, not downgraded or silently substituted. TLS verification is disabled
  only for the local self-signed
  benchmark certificate. Proxy environment variables are not used.

Client construction is outside the timer. Timed batches include uploads, TLS
and HTTP processing, and response iteration; they are not isolated TLS IO
measurements. Untimed requests establish reusable connections. Client variants
run in separate processes; async clients use standard asyncio rather than
uvloop. Blocking results include thread-pool scheduling, and their independent
worker clients have separate connection pools.

| Upload / echo payload | Stream upload chunk |
| --- | --- |
| 1 KiB | 1 KiB |
| 10 KiB | 10 KiB |
| 64 KiB | 16 KiB |
| 128 KiB | 32 KiB |
| 1 MiB | 64 KiB |
| 2 MiB | 128 KiB |
| 4 MiB | 256 KiB |

Both Full and Stream uploads are tested for every payload and concurrency.
The complete matrix contains 1,680 supported cells across 16 client variants;
each cell has three measured rounds. The Rust benchmark uses Criterion and
600 requests per iteration; this suite uses fixed batches of 300 requests.
Custom payload sizes use upload chunks of at most 64 KiB.

## Run locally

Use CPython 3.14 and a Rust toolchain compatible with the manifests. BoringSSL
requires CMake, Clang, and the usual native build tools.

```bash
uv venv --python 3.14
uv pip install -r bench/requirements.txt
uv run --no-sync maturin develop --release --uv --locked --features jemalloc
cargo build --release --locked --manifest-path bench/server/Cargo.toml
uv run --no-sync python bench/run.py \
  --server bench/server/target/release/wreq-benchmark-server
```

The build command above uses jemalloc on macOS and Linux. On Windows, use the
platform allocator or the supported `mimalloc` feature instead; do not enable
both allocator features. The server executable ends in `.exe` on Windows. If
Cargo uses a custom target directory, pass the actual executable path to `--server`.

`bench/run.py` passes workload options to the measurement runner. Before it
starts, it prints the number of cases and batches. It saves stdout/stderr logs
beside the raw JSON and generates an English `.report.md` once validation passes.

The default suite has 1,680 supported cases, 5,040 timed batches and the same
number of warm-up batches. Allow several hours for a full run. The setup commands
above prepare the dependencies and native binaries; the runner won't install or
build them for you. Publishing the results to the docs is optional.

For a warm-up experiment, pass `--warmup-requests 150`. This keeps the default
300 timed requests and three rounds, but sends 150 requests per warm-up batch.
The budget must reach every concurrent worker. Compare against the default on
the same machine before using these results; reduced warm-up runs can be
previewed with `docs/build.py --data RUN.json` but cannot replace published data.

Use the same allocator and release flags for both wreq revisions in a comparison,
and record them in the run's build provenance. jemalloc controls Rust allocations
in the wreq extension, not Python's allocator or those of the other clients.
The saved WSL measurements also used `--features jemalloc`.

For an integration check, use the same clients and protocols with fewer
requests. This smoke run checks behavior; it isn't enough to rank performance:

```bash
uv run --no-sync python bench/run.py \
  --server bench/server/target/release/wreq-benchmark-server \
  --sizes 10240,1048576 --concurrency 2 --requests 4 \
  --rounds 1 --warmup 0 --samples 1 --output bench/data/smoke/RUN.json
uv run --no-sync python -m pytest bench/tests
```

Default `pytest` runs only `tests/`. The explicit command above checks the
benchmark tools without running a performance measurement.

## Results and interpretation

The JSON records the source SHA and dirty state, Python and package versions,
CPU model, operating system, machine architecture, logical CPU count, runtime
configurations, native module hashes, server hash, and individual timings.
Results are written atomically only after all supported combinations in the
requested matrix pass validation.

Requests/s is total completed requests divided by total elapsed seconds, not an
arithmetic mean of rates. MB/s counts the response payload only, in decimal MB;
the timer includes both upload and download. Responses are streamed, never
collected into a complete body. Native streaming interfaces are used where
available; adapters requiring a read size use documented 64 KiB reads.

Other programs on the machine compete for CPU and I/O. Repeat measurements and
check their environments before treating a difference as stable. A client that
wins here may perform differently in your application. These tests also don't
measure browser-emulation compatibility.

Compare two snapshots with identical recorded workloads and compatible metadata:

```bash
uv run --no-sync python bench/compare.py \
  --before bench/data/BEFORE.json --after bench/data/AFTER.json \
  --output bench/data/COMPARISON.md
```

The English report shows each case's RPS change and variation between rounds.
That variation isn't a confidence interval. Running one snapshot after another
also doesn't isolate TLS I/O improvements; check the build and harness records
before attributing a change. Omit `--output` to print UTF-8 Markdown to stdout.
File exports refuse to overwrite any existing path, including the input JSON.

Add `--api blocking` to compare a full snapshot with a blocking-only rerun.
Both original snapshots are validated; only the report is filtered. Workload,
environment, server and the selected client metadata must still match.

## Stored data and documentation

We run the benchmarks locally, outside GitHub Actions, and keep JSON snapshots
in [`bench/data`](data/) on `main`. Without `--output`, the runner creates a
timestamped filename containing the measured source SHA. If you supply an output
path, it must be new. Completed runs never overwrite an earlier snapshot.

Keep every completed snapshot. After reviewing a full measurement, select it
with:

```bash
uv run --no-sync python bench/run.py --input bench/data/RUN.json --publish
```

`--input` reads a saved run and generates its report, or reuses the report if its
contents match. It never starts a measurement. With `--publish`, the runner also
freezes the candidate JSON and any selected blocking snapshot, then builds the docs.
After a successful build, it atomically selects the original bytes as
`bench/data/latest.json`. Historical JSON, logs and reports stay untouched.

To update only the blocking results without rerunning the async clients:

```bash
uv run --no-sync python bench/run.py --input bench/data/BLOCKING.json --publish-blocking
```

This requires all eight blocking variants and the same complete workload and
batch minimums. It freezes the candidate and existing `latest.json` for the docs
build, then selects `bench/data/latest-blocking.json` only if the build succeeds.
The async source and all historical files stay unchanged. The two publication
options cannot be combined.

Smoke runs belong in `bench/data/smoke/` and cannot be published. Publication
requires the complete default or blocking matrix, at least 300 requests per
batch and three rounds, with at least one warm-up and timed sample per round.
These checks confirm coverage; you still need to review the quality of the measurements.

Use `--build-docs` instead of `--publish` to preview a complete recorded matrix
without changing `latest.json`. If your docs dependencies are in a separate
virtual environment, add `--docs-python PATH/TO/python`. You can also pass
`--publish` to a new measurement to run those steps after it finishes. The
runner doesn't commit, push or enable benchmarks in CI.

`python docs/build.py` reads and validates the checked-in `bench/data/latest.json`
and, when present, `latest-blocking.json`. The page keeps each source's revision
and environment separate; no combined measurement JSON is created.
It generates responsive light/dark SVG charts and fills
`docs/templates/benchmark.md` with body-size controls and measurement details.
The built site includes a frozen raw JSON copy. The build won't fetch
measurements or start a benchmark, and missing or invalid data stops it.
Read the Docs uses this same entry point.

Use `python docs/build.py --data bench/data/RUN.json` to preview another run.
This standalone preview does not load the selected blocking overlay.
For blocking-only data, use `--blocking-data bench/data/BLOCKING.json` instead.
Each measurement shows its source revision and whether the checkout had
uncommitted changes. Saving data on `main` doesn't change which code was tested.

Explicit `--blocking-data` overrides may preview a blocking subset or a reduced
warm-up run, provided their workload axes match the async snapshot. Publishing
still requires the complete current matrix and full publication budget.
Combined preview JSON may record `measurement_sources`; each entry must assign
distinct clients, and together they must cover the configured clients. The page
shows each run's revision and collection time separately.

## Blocking wreq runtimes

`wreq_blocking` uses the default shared multi-thread network runtime (MT).
`wreq_blocking_st` shares one
`Runtime(scheduler=Scheduler.PER_WORKER, workers=1)` across all logical workers
in each case (ST). `wreq_blocking_ct` gives each logical worker its own
`Runtime(scheduler=Scheduler.CURRENT_THREAD)`, driven by the calling thread
(CT). All three retain one client per logical worker and the same Python thread
pool for concurrent blocking calls.

The default full run includes async wreq MT/ST and blocking wreq MT/ST/CT, for
18 variants and 1,904 supported cells. No `--clients` option is needed.
Historical published snapshots remain readable; new full/blocking publication
requires ST and CT. CT needs a wreq release with `Scheduler`.

## Maintaining the suite

- `registry.py`: one `ClientSpec` per variant, with package, label, capabilities,
  runtime, pool and response-reading settings. Default selection, validation,
  metadata and chart labels derive from this registry.
- `clients.py`, `async_clients.py`, `blocking_clients.py`: library-specific
  construction and request/response adapters. Imports stay inside worker
  processes. Custom wreq runtimes use the registered configuration.
- `benchmark.py`: workload scheduling and the timed async/blocking loops.
- `processes.py`: worker/server startup, JSON-line communication and cleanup.
  `start_worker()` owns a subprocess; `Worker.measure()` submits one case.
- `results.py`: aggregation and snapshot validation; `run.py`: artifact
  preservation, reporting and explicit publication.

To add a client, register its description in `SPECS` and implement its adapter.
To add another wreq runtime variant, register its runtime settings and label.
Keep payload preparation, runtime creation and protocol communication outside
timed batches. Verify descriptions against the actual library configuration,
then run the tool tests and an all-client local HTTPS smoke test.
