Measured revision: [80b48db8b274](https://github.com/0x676e67/wreq-python/commit/80b48db8b2741b60b41ba6f0966bd5f52c3f5185). Collected 2026-10-04 11:14 UTC.

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
| 1 KiB | 10 | 19,198.4 | 8,838.6 | 12,998.5 |
| 1 KiB | 50 | 15,383.0 | 7,879.9 | 11,600.2 |
| 4 MiB | 10 | 959.7 | 869.7 | 106.8 |
| 4 MiB | 50 | 920.7 | 903.6 | 111.6 |

#### HTTP/1.1: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking&#41; | ry &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | 8,050.9 | 5,328.7 | 6,781.5 |
| 1 KiB | 50 | 7,399.2 | 4,988.9 | 6,370.0 |
| 4 MiB | 10 | 924.0 | 946.2 | 80.3 |
| 4 MiB | 50 | 896.4 | 748.0 | 81.7 |

#### HTTP/2: Full upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking&#41; | ry &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | 18,395.9 | 8,803.8 | 12,777.2 |
| 1 KiB | 50 | 14,685.9 | 7,930.7 | 11,336.3 |
| 4 MiB | 10 | 751.8 | 104.9 | 52.7 |
| 4 MiB | 50 | 683.7 | 107.7 | 55.2 |

#### HTTP/2: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking&#41; | ry &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | 7,989.0 | 5,312.8 | 6,735.2 |
| 1 KiB | 50 | 7,283.8 | 4,924.2 | 6,370.6 |
| 4 MiB | 10 | 528.8 | 97.6 | 46.8 |
| 4 MiB | 50 | 518.7 | 99.8 | 48.5 |
