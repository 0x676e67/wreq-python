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
- Blocking clients: wreq, ry, requests, httpx, niquests, curl_cffi, and pycurl.
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
uv run --no-sync maturin develop --release --uv --locked
cargo build --release --locked --manifest-path bench/server/Cargo.toml
uv run --no-sync python bench/benchmark.py \
  --server bench/server/target/release/wreq-benchmark-server
uv run --no-sync python bench/report.py --input bench/data/RUN.json --markdown \
  --output bench/data/RUN.md
```

On Windows, the server executable ends in `.exe`. If Cargo uses a custom target
directory, pass the actual executable path to `--server`.

The retained WSL measurements built wreq with `--features jemalloc` in addition
to the release flags above. Use a run's build records to reproduce its allocator
and toolchain; the generic command above uses the platform's default allocator.

A quick integration check uses the same clients and protocols, with fewer
requests. Smoke results validate behavior, not performance rankings:

```bash
uv run --no-sync python bench/benchmark.py \
  --server bench/server/target/release/wreq-benchmark-server \
  --sizes 10240,1048576 --concurrency 2 --requests 4 \
  --rounds 1 --warmup 0 --samples 1 --output bench/data/smoke/RUN.json
uv run --no-sync python -m pytest bench/test_*.py
```

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

Other programs can compete for CPU and IO. Compare repeated results and their
environment, not one ranking. These benchmarks do not establish universal
client performance or measure browser-emulation compatibility.

Compare two snapshots with identical recorded workloads and compatible metadata:

```bash
uv run --no-sync python bench/compare.py \
  --before bench/data/BEFORE.json --after bench/data/AFTER.json \
  --output bench/data/COMPARISON.md
```

The English report shows each cell's RPS change and round variation. Three-round
variation is not a confidence interval, and sequential runs do not isolate TLS
IO improvements; review build and harness records before attributing changes.
Omit `--output` to print UTF-8 Markdown to stdout. File exports refuse to
overwrite an existing path, including the input JSON.

## Stored data and documentation

Benchmarks run locally, not in GitHub Actions. JSON snapshots are retained in
[`bench/data`](data/) on `main`. Without `--output`, a run creates a timestamped
filename containing the measured source SHA. An explicit output path must not
already exist; completed runs never overwrite an earlier snapshot.

Keep every completed snapshot. After reviewing a full measurement, copy that
JSON to `bench/data/latest.json` to select it for the documentation. Smoke runs
belong in `bench/data/smoke/` and must not replace `latest.json`.

`python docs/build.py` reads the checked-in `bench/data/latest.json`, validates
the complete recorded matrix, renders `docs/templates/benchmark.md`, and
includes a frozen raw JSON download in the built site. It does not download
measurements or start a benchmark. Missing or invalid data fails the build.

Use `python docs/build.py --data bench/data/RUN.json` to preview another run.
The page labels every measurement with its actual source revision and dirty
state; storing data on `main` does not change which code was measured.
