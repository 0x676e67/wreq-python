Measured revision: [59d47eec74f0](https://github.com/0x676e67/wreq-python/commit/59d47eec74f06311b4b81eff434ff6cb9d1cae3f). Collected 2026-10-04 11:09 UTC.

### Measurement environment

| Item | Value |
| --- | --- |
| Python | CPython 3&#46;14&#46;6 &#40;main, Jul 23 2026, 14:45:24&#41; &#91;Clang 22&#46;1&#46;3 &#93; |
| OS | Linux&#45;6&#46;18&#46;33&#46;2&#45;microsoft&#45;standard&#45;WSL2&#45;x86&#95;64&#45;with&#45;glibc2&#46;39 |
| Architecture | x86&#95;64 |
| CPU | AMD Ryzen 9 9950X 16&#45;Core Processor; 32 logical CPUs |
| Python event loop | asyncio |
| Server | Controlled Rust TLS echo server; 4 workers |
| TLS | TLS 1.3; certificate verification disabled for the local test server |
| Repeats | Rounds: 3; timed batches/round: 1; requests/batch: 300; warm-up batches/round: 1 |
| Stream upload chunk | Varies by payload; see body cases below |

### Body cases

Each payload uses the upload modes and concurrency levels listed in this run.

| Upload / echo payload | Stream upload chunk |
| --- | --- |
| 1 KiB | 1 KiB |
| 4 MiB | 256 KiB |

### Client versions and runtimes

| Client | API | Package | Version | Runtime | Protocols | Uploads |
| --- | --- | --- | --- | --- | --- | --- |
| wreq &#40;blocking&#41; | blocking | wreq | 0&#46;12&#46;3 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| ry &#40;blocking&#41; | blocking | ry | 0&#46;0&#46;101 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| PycURL | blocking | pycurl | 7&#46;48&#46;0 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |

### Throughput comparison

Each value is total measured requests divided by total measured time across all rounds, in requests per second (RPS). All timed samples count. The raw JSON keeps individual timings, per-round results, response-payload MB/s and native artifact SHA-256 hashes. Read chunking, buffering and connection pools differ between adapters; their configurations are recorded in the raw JSON.

### Blocking clients

N/A means the client API doesn't support that protocol or upload mode. It doesn't indicate a failed request or zero throughput.

#### HTTP/1.1: Full upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking&#41; | ry &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | 3,996.0 | 8,631.0 | 13,333.4 |
| 1 KiB | 50 | 3,799.6 | 7,806.6 | 11,437.6 |
| 4 MiB | 10 | 560.0 | 909.4 | 108.3 |
| 4 MiB | 50 | 529.8 | 911.0 | 112.8 |

#### HTTP/1.1: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking&#41; | ry &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | 559.0 | 5,402.7 | 6,789.8 |
| 1 KiB | 50 | 561.4 | 4,963.9 | 6,277.0 |
| 4 MiB | 10 | 259.0 | 967.1 | 80.1 |
| 4 MiB | 50 | 254.9 | 735.2 | 82.1 |

#### HTTP/2: Full upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking&#41; | ry &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | 3,890.4 | 8,829.8 | 12,651.9 |
| 1 KiB | 50 | 3,791.8 | 7,838.6 | 11,633.2 |
| 4 MiB | 10 | 689.5 | 105.0 | 53.1 |
| 4 MiB | 50 | 652.3 | 108.4 | 55.4 |

#### HTTP/2: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking&#41; | ry &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | 561.5 | 5,360.4 | 6,630.8 |
| 1 KiB | 50 | 564.3 | 4,996.5 | 6,271.5 |
| 4 MiB | 10 | 278.7 | 98.5 | 46.7 |
| 4 MiB | 50 | 277.5 | 100.1 | 48.5 |
