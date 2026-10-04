# Benchmark revision comparison

| Snapshot | Source revision | Dirty checkout | Collected (UTC) | wreq version | wreq native SHA-256 |
| --- | --- | --- | --- | --- | --- |
| Before | [59d47eec74f06311b4b81eff434ff6cb9d1cae3f](https://github.com/0x676e67/wreq-python/commit/59d47eec74f06311b4b81eff434ff6cb9d1cae3f) | No | 2026&#45;10&#45;04T11:09:04&#46;322643&#43;00:00 | 0&#46;12&#46;3 | ef00bb006e6c6db54f7dd1154627686ecac14c3bbf87cdf25e100b2500b99368 |
| After | [80b48db8b2741b60b41ba6f0966bd5f52c3f5185](https://github.com/0x676e67/wreq-python/commit/80b48db8b2741b60b41ba6f0966bd5f52c3f5185) | No | 2026&#45;10&#45;04T11:14:51&#46;742882&#43;00:00 | 0&#46;12&#46;3 | 84dcf6b05a38abdb9b39c61184e00eac324df7f8722953017a59cf1ebfdf437f |

Each snapshot contains 48 supported cells, with 900 timed requests per cell across 3 rounds. Counts come from the cases recorded in these snapshots.

### Matching measurement environment

| Item | Value |
| --- | --- |
| python | 3&#46;14&#46;6 &#40;main, Jul 23 2026, 14:45:24&#41; &#91;Clang 22&#46;1&#46;3 &#93; |
| implementation | CPython |
| platform | Linux&#45;6&#46;18&#46;33&#46;2&#45;microsoft&#45;standard&#45;WSL2&#45;x86&#95;64&#45;with&#45;glibc2&#46;39 |
| machine | x86&#95;64 |
| cpu | AMD Ryzen 9 9950X 16&#45;Core Processor |
| cpu&#95;count | 32 |
| event&#95;loop | asyncio |
| affinity | &#91;0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31&#93; |
| Server SHA-256 | 52201b771a5ae474fc3d52249b922b04a1d077508ac71621b256d7c4d597f550 |

Unit: requests/s (RPS, requests per second). RPS is total timed requests divided by total timed seconds across all rounds; warmup and validation requests are excluded. Change = 100 × (After RPS / Before RPS − 1); positive values mean higher throughput.

### Blocking clients

#### HTTP/1.1: Full upload

| Payload | Concurrency | Client | Before RPS | After RPS | Change % | Before CV % | After CV % |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | wreq &#40;blocking&#41; | 3,996.0 | 19,198.4 | +380.4 | 0.66 | 0.82 |
| 1 KiB | 10 | ry &#40;blocking&#41; | 8,631.0 | 8,838.6 | +2.4 | 1.43 | 1.10 |
| 1 KiB | 10 | PycURL | 13,333.4 | 12,998.5 | -2.5 | 0.68 | 1.92 |
| 1 KiB | 50 | wreq &#40;blocking&#41; | 3,799.6 | 15,383.0 | +304.9 | 0.70 | 0.75 |
| 1 KiB | 50 | ry &#40;blocking&#41; | 7,806.6 | 7,879.9 | +0.9 | 2.07 | 2.86 |
| 1 KiB | 50 | PycURL | 11,437.6 | 11,600.2 | +1.4 | 0.97 | 0.95 |
| 4 MiB | 10 | wreq &#40;blocking&#41; | 560.0 | 959.7 | +71.4 | 0.86 | 6.66 |
| 4 MiB | 10 | ry &#40;blocking&#41; | 909.4 | 869.7 | -4.4 | 5.72 | 7.65 |
| 4 MiB | 10 | PycURL | 108.3 | 106.8 | -1.3 | 0.44 | 0.30 |
| 4 MiB | 50 | wreq &#40;blocking&#41; | 529.8 | 920.7 | +73.8 | 1.40 | 1.47 |
| 4 MiB | 50 | ry &#40;blocking&#41; | 911.0 | 903.6 | -0.8 | 1.51 | 2.46 |
| 4 MiB | 50 | PycURL | 112.8 | 111.6 | -1.1 | 1.63 | 2.01 |

#### HTTP/1.1: Stream upload

| Payload | Concurrency | Client | Before RPS | After RPS | Change % | Before CV % | After CV % |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | wreq &#40;blocking&#41; | 559.0 | 8,050.9 | +1340.1 | 0.34 | 0.39 |
| 1 KiB | 10 | ry &#40;blocking&#41; | 5,402.7 | 5,328.7 | -1.4 | 0.55 | 0.27 |
| 1 KiB | 10 | PycURL | 6,789.8 | 6,781.5 | -0.1 | 1.65 | 0.87 |
| 1 KiB | 50 | wreq &#40;blocking&#41; | 561.4 | 7,399.2 | +1218.1 | 0.64 | 1.12 |
| 1 KiB | 50 | ry &#40;blocking&#41; | 4,963.9 | 4,988.9 | +0.5 | 1.27 | 2.46 |
| 1 KiB | 50 | PycURL | 6,277.0 | 6,370.0 | +1.5 | 0.68 | 1.58 |
| 4 MiB | 10 | wreq &#40;blocking&#41; | 259.0 | 924.0 | +256.7 | 0.68 | 4.93 |
| 4 MiB | 10 | ry &#40;blocking&#41; | 967.1 | 946.2 | -2.2 | 2.26 | 2.47 |
| 4 MiB | 10 | PycURL | 80.1 | 80.3 | +0.3 | 0.51 | 0.27 |
| 4 MiB | 50 | wreq &#40;blocking&#41; | 254.9 | 896.4 | +251.6 | 0.35 | 0.38 |
| 4 MiB | 50 | ry &#40;blocking&#41; | 735.2 | 748.0 | +1.7 | 0.33 | 0.40 |
| 4 MiB | 50 | PycURL | 82.1 | 81.7 | -0.4 | 0.29 | 0.07 |

#### HTTP/2: Full upload

| Payload | Concurrency | Client | Before RPS | After RPS | Change % | Before CV % | After CV % |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | wreq &#40;blocking&#41; | 3,890.4 | 18,395.9 | +372.9 | 0.64 | 3.26 |
| 1 KiB | 10 | ry &#40;blocking&#41; | 8,829.8 | 8,803.8 | -0.3 | 1.03 | 1.68 |
| 1 KiB | 10 | PycURL | 12,651.9 | 12,777.2 | +1.0 | 1.50 | 1.58 |
| 1 KiB | 50 | wreq &#40;blocking&#41; | 3,791.8 | 14,685.9 | +287.3 | 0.10 | 2.03 |
| 1 KiB | 50 | ry &#40;blocking&#41; | 7,838.6 | 7,930.7 | +1.2 | 1.16 | 1.27 |
| 1 KiB | 50 | PycURL | 11,633.2 | 11,336.3 | -2.6 | 0.24 | 0.67 |
| 4 MiB | 10 | wreq &#40;blocking&#41; | 689.5 | 751.8 | +9.0 | 0.84 | 4.31 |
| 4 MiB | 10 | ry &#40;blocking&#41; | 105.0 | 104.9 | -0.0 | 0.39 | 0.56 |
| 4 MiB | 10 | PycURL | 53.1 | 52.7 | -0.8 | 0.47 | 0.32 |
| 4 MiB | 50 | wreq &#40;blocking&#41; | 652.3 | 683.7 | +4.8 | 1.97 | 2.62 |
| 4 MiB | 50 | ry &#40;blocking&#41; | 108.4 | 107.7 | -0.6 | 0.81 | 0.50 |
| 4 MiB | 50 | PycURL | 55.4 | 55.2 | -0.3 | 0.61 | 0.04 |

#### HTTP/2: Stream upload

| Payload | Concurrency | Client | Before RPS | After RPS | Change % | Before CV % | After CV % |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 10 | wreq &#40;blocking&#41; | 561.5 | 7,989.0 | +1322.8 | 0.31 | 1.21 |
| 1 KiB | 10 | ry &#40;blocking&#41; | 5,360.4 | 5,312.8 | -0.9 | 0.81 | 0.18 |
| 1 KiB | 10 | PycURL | 6,630.8 | 6,735.2 | +1.6 | 1.71 | 0.34 |
| 1 KiB | 50 | wreq &#40;blocking&#41; | 564.3 | 7,283.8 | +1190.8 | 0.11 | 0.29 |
| 1 KiB | 50 | ry &#40;blocking&#41; | 4,996.5 | 4,924.2 | -1.4 | 1.08 | 1.34 |
| 1 KiB | 50 | PycURL | 6,271.5 | 6,370.6 | +1.6 | 2.44 | 1.17 |
| 4 MiB | 10 | wreq &#40;blocking&#41; | 278.7 | 528.8 | +89.7 | 0.47 | 0.20 |
| 4 MiB | 10 | ry &#40;blocking&#41; | 98.5 | 97.6 | -0.9 | 0.61 | 0.23 |
| 4 MiB | 10 | PycURL | 46.7 | 46.8 | +0.2 | 0.15 | 0.38 |
| 4 MiB | 50 | wreq &#40;blocking&#41; | 277.5 | 518.7 | +86.9 | 0.65 | 0.22 |
| 4 MiB | 50 | ry &#40;blocking&#41; | 100.1 | 99.8 | -0.3 | 0.65 | 0.31 |
| 4 MiB | 50 | PycURL | 48.5 | 48.5 | +0.1 | 0.14 | 0.18 |

### Reading the comparison

CV is the variation in per-round RPS: population standard deviation divided by the mean. It is not a confidence interval. Sequential runs are not a paired experiment, and other programs can affect performance. If peer clients also change, the environment may have changed between runs; their results alone can't tell you why.

The timer covers uploads, TLS and HTTP processing, and streamed response consumption. A throughput change doesn't tell you how much came from TLS I/O, how long an individual request took, or which client will be fastest in a different workload.

The comparison checks that recorded configurations, environments, server hashes and client metadata match. It allows wreq versions and native hashes to change, and artifact paths to move. Check the build and source records yourself for transitive dependencies, toolchains and flags, allocators, changes to the timed harness and uncommitted source changes.
