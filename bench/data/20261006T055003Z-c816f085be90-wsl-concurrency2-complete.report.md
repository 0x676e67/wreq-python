Measured revision: [c816f085be90](https://github.com/0x676e67/wreq-python/commit/c816f085be9064f366716b268b4264ca9050982b). Collected 2026-10-06 06:23 UTC.

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
| Repeats | Rounds: 3; timed batches/round: 1; timed requests/batch: 300; warm-up batches/round: 1; warm-up requests/batch: 200 |
| Stream upload chunk | Varies by payload; see body cases below |

### Body cases

Each payload uses the upload modes and concurrency levels listed in this run.

| Upload / echo payload | Stream upload chunk |
| --- | --- |
| 1 KiB | 1 KiB |
| 10 KiB | 10 KiB |
| 64 KiB | 16 KiB |
| 128 KiB | 32 KiB |
| 1 MiB | 64 KiB |
| 2 MiB | 128 KiB |
| 4 MiB | 256 KiB |

### Client versions and runtimes

| Client | API | Package | Version | Runtime | Protocols | Uploads |
| --- | --- | --- | --- | --- | --- | --- |
| wreq &#40;MT&#41; | async | wreq | 0&#46;13&#46;0 | Library default | HTTP/1.1, HTTP/2 | Full, Stream |
| wreq &#40;ST&#41; | async | wreq | 0&#46;13&#46;0 | Custom: workers=1, work&#95;steal=False | HTTP/1.1, HTTP/2 | Full, Stream |
| pyreqwest &#40;ST&#41; | async | pyreqwest | 0&#46;14&#46;0 | Single&#45;thread runtime | HTTP/1.1, HTTP/2 | Full, Stream |
| pyreqwest &#40;MT&#41; | async | pyreqwest | 0&#46;14&#46;0 | Multi&#45;thread runtime | HTTP/1.1, HTTP/2 | Full, Stream |
| ry &#40;default&#41; | async | ry | 0&#46;0&#46;101 | Library default | HTTP/1.1, HTTP/2 | Full, Stream |
| httpx | async | httpx | 0&#46;28&#46;1 | Library default | HTTP/1.1, HTTP/2 | Full, Stream |
| aiohttp | async | aiohttp | 3&#46;14&#46;3 | Library default | HTTP/1.1 | Full, Stream |
| niquests | async | niquests | 3&#46;21&#46;2 | Library default | HTTP/1.1, HTTP/2 | Full, Stream |
| curl&#95;cffi | async | curl&#95;cffi | 0&#46;16&#46;3 | Library default | HTTP/1.1, HTTP/2 | Full, Stream |
| wreq &#40;blocking MT&#41; | blocking | wreq | 0&#46;13&#46;0 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| wreq &#40;blocking ST&#41; | blocking | wreq | 0&#46;13&#46;0 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| ry &#40;blocking&#41; | blocking | ry | 0&#46;0&#46;101 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| requests | blocking | requests | 2&#46;34&#46;2 | Thread pool; one client per logical worker | HTTP/1.1 | Full, Stream |
| httpx &#40;blocking&#41; | blocking | httpx | 0&#46;28&#46;1 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| niquests &#40;blocking&#41; | blocking | niquests | 3&#46;21&#46;2 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| curl&#95;cffi &#40;blocking&#41; | blocking | curl&#95;cffi | 0&#46;16&#46;3 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |
| PycURL | blocking | pycurl | 7&#46;48&#46;0 | Thread pool; one client per logical worker | HTTP/1.1, HTTP/2 | Full, Stream |

### Throughput comparison

Each value is total measured requests divided by total measured time across all rounds, in requests per second (RPS). All timed samples count. The raw JSON keeps individual timings, per-round results, response-payload MB/s and native artifact SHA-256 hashes. Read chunking, buffering and connection pools differ between adapters; their configurations are recorded in the raw JSON.

### Asynchronous clients

N/A means the client API doesn't support that protocol or upload mode. It doesn't indicate a failed request or zero throughput.

#### HTTP/1.1: Full upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;MT&#41; | wreq &#40;ST&#41; | pyreqwest &#40;ST&#41; | pyreqwest &#40;MT&#41; | ry &#40;default&#41; | httpx | aiohttp | niquests | curl&#95;cffi |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 8,495.3 | 9,911.2 | 5,793.7 | 6,230.4 | 2,479.5 | 1,879.3 | 7,222.9 | 2,375.6 | 5,059.6 |
| 10 KiB | 2 | 8,233.7 | 9,493.8 | 5,998.8 | 6,103.5 | 2,500.0 | 1,833.5 | 7,023.3 | 2,332.1 | 5,002.7 |
| 64 KiB | 2 | 6,217.0 | 6,583.7 | 4,836.0 | 5,263.4 | 2,312.2 | 1,544.1 | 5,631.3 | 2,122.0 | 3,358.6 |
| 128 KiB | 2 | 4,455.4 | 4,843.4 | 4,303.9 | 4,541.8 | 2,193.4 | 1,380.1 | 5,251.4 | 1,811.0 | 2,480.9 |
| 1 MiB | 2 | 1,357.6 | 1,251.6 | 1,244.6 | 1,283.2 | 989.8 | 572.9 | 1,401.8 | 770.7 | 621.1 |
| 2 MiB | 2 | 760.3 | 760.5 | 688.7 | 682.2 | 566.1 | 321.1 | 726.9 | 444.7 | 353.1 |
| 4 MiB | 2 | 362.3 | 366.7 | 338.1 | 339.8 | 294.9 | 171.3 | 331.7 | 226.3 | 171.6 |

#### HTTP/1.1: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;MT&#41; | wreq &#40;ST&#41; | pyreqwest &#40;ST&#41; | pyreqwest &#40;MT&#41; | ry &#40;default&#41; | httpx | aiohttp | niquests | curl&#95;cffi |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 7,597.8 | 8,249.7 | 2,669.4 | 2,504.2 | 1,466.9 | 1,608.3 | 6,416.5 | 2,198.8 | 3,783.0 |
| 10 KiB | 2 | 7,449.3 | 7,983.7 | 2,650.7 | 2,492.3 | 1,470.1 | 1,614.9 | 6,116.4 | 2,232.5 | 3,654.2 |
| 64 KiB | 2 | 4,401.6 | 4,383.9 | 1,510.7 | 1,308.9 | 890.7 | 1,195.6 | 4,589.4 | 1,808.5 | 2,189.9 |
| 128 KiB | 2 | 3,654.5 | 3,457.4 | 1,461.7 | 1,302.9 | 899.6 | 1,048.9 | 4,584.7 | 1,434.1 | 1,808.3 |
| 1 MiB | 2 | 1,176.1 | 823.9 | 439.1 | 423.4 | 314.0 | 332.7 | 1,107.3 | 536.0 | 397.8 |
| 2 MiB | 2 | 696.7 | 476.2 | 329.5 | 367.8 | 280.9 | 243.6 | 759.7 | 407.0 | 238.0 |
| 4 MiB | 2 | 361.4 | 339.2 | 201.9 | 270.8 | 227.2 | 149.6 | 370.2 | 211.1 | 132.5 |

#### HTTP/2: Full upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;MT&#41; | wreq &#40;ST&#41; | pyreqwest &#40;ST&#41; | pyreqwest &#40;MT&#41; | ry &#40;default&#41; | httpx | aiohttp | niquests | curl&#95;cffi |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 4,552.6 | 7,545.0 | 5,217.4 | 5,138.6 | 2,405.3 | 1,599.6 | N/A | 2,047.3 | 4,376.2 |
| 10 KiB | 2 | 4,394.9 | 7,409.2 | 5,588.5 | 5,178.2 | 2,323.6 | 1,528.7 | N/A | 2,015.7 | 4,229.6 |
| 64 KiB | 2 | 3,487.2 | 4,931.2 | 2,941.5 | 2,601.6 | 1,210.2 | 822.2 | N/A | 1,352.5 | 2,821.4 |
| 128 KiB | 2 | 2,792.8 | 4,324.4 | 1,688.5 | 1,731.3 | 738.0 | 543.0 | N/A | 937.2 | 1,788.0 |
| 1 MiB | 2 | 618.8 | 822.4 | 235.9 | 286.5 | 114.2 | 99.3 | N/A | 186.4 | 417.9 |
| 2 MiB | 2 | 327.7 | 414.5 | 116.1 | 149.7 | 59.1 | 48.0 | N/A | 95.0 | 211.8 |
| 4 MiB | 2 | 154.0 | 211.3 | 66.4 | 78.5 | 30.3 | 21.6 | N/A | 48.9 | 118.3 |

#### HTTP/2: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;MT&#41; | wreq &#40;ST&#41; | pyreqwest &#40;ST&#41; | pyreqwest &#40;MT&#41; | ry &#40;default&#41; | httpx | aiohttp | niquests | curl&#95;cffi |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 4,238.5 | 7,226.0 | 2,643.2 | 2,455.4 | 1,462.8 | 1,614.6 | N/A | 2,018.3 | 3,201.4 |
| 10 KiB | 2 | 4,149.2 | 6,762.2 | 2,593.5 | 2,452.4 | 1,472.1 | 1,549.7 | N/A | 1,988.0 | 3,252.3 |
| 64 KiB | 2 | 2,601.9 | 3,327.2 | 1,232.7 | 1,142.2 | 676.8 | 798.4 | N/A | 1,342.4 | 1,581.4 |
| 128 KiB | 2 | 2,118.1 | 2,666.0 | 937.8 | 925.3 | 505.3 | 545.6 | N/A | 895.3 | 1,256.7 |
| 1 MiB | 2 | 458.7 | 536.3 | 173.4 | 193.5 | 92.7 | 103.8 | N/A | 174.6 | 270.8 |
| 2 MiB | 2 | 276.2 | 328.2 | 99.5 | 121.2 | 52.7 | 54.6 | N/A | 94.3 | 177.9 |
| 4 MiB | 2 | 146.1 | 194.8 | 60.4 | 70.8 | 28.7 | 27.5 | N/A | 46.8 | 100.0 |

### Blocking clients

N/A means the client API doesn't support that protocol or upload mode. It doesn't indicate a failed request or zero throughput.

#### HTTP/1.1: Full upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking MT&#41; | wreq &#40;blocking ST&#41; | ry &#40;blocking&#41; | requests | httpx &#40;blocking&#41; | niquests &#40;blocking&#41; | curl&#95;cffi &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 10,292.0 | 10,534.9 | 9,907.1 | 1,961.4 | 2,768.8 | 2,165.3 | 2,951.7 | 13,414.9 |
| 10 KiB | 2 | 9,996.4 | 10,130.4 | 9,668.7 | 1,984.9 | 2,775.8 | 2,186.1 | 2,981.4 | 13,408.4 |
| 64 KiB | 2 | 6,759.6 | 7,053.8 | 6,569.8 | 1,285.9 | 1,907.1 | 1,358.8 | 2,781.9 | 8,708.1 |
| 128 KiB | 2 | 5,662.0 | 6,085.4 | 5,724.7 | 975.2 | 1,491.1 | 1,011.8 | 3,098.1 | 6,911.7 |
| 1 MiB | 2 | 1,457.7 | 1,280.8 | 1,426.6 | 330.1 | 589.0 | 318.0 | 1,303.7 | 1,541.0 |
| 2 MiB | 2 | 705.0 | 738.7 | 716.9 | 213.8 | 340.6 | 206.0 | 699.9 | 794.5 |
| 4 MiB | 2 | 369.3 | 364.5 | 333.4 | 91.6 | 159.7 | 98.8 | 346.9 | 374.5 |

#### HTTP/1.1: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking MT&#41; | wreq &#40;blocking ST&#41; | ry &#40;blocking&#41; | requests | httpx &#40;blocking&#41; | niquests &#40;blocking&#41; | curl&#95;cffi &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 6,834.2 | 7,124.2 | 7,869.2 | 1,923.9 | 2,488.1 | 2,027.3 | 2,504.6 | 11,041.3 |
| 10 KiB | 2 | 6,922.6 | 6,991.3 | 7,695.6 | 1,918.1 | 2,513.1 | 2,062.2 | 2,433.2 | 10,476.3 |
| 64 KiB | 2 | 4,323.8 | 3,461.6 | 6,115.3 | 1,087.8 | 1,504.2 | 1,139.4 | 1,984.1 | 6,910.3 |
| 128 KiB | 2 | 3,939.3 | 3,029.5 | 5,281.1 | 844.2 | 1,263.4 | 877.4 | 2,073.2 | 5,671.2 |
| 1 MiB | 2 | 1,098.0 | 727.2 | 1,355.3 | 195.4 | 327.9 | 189.2 | 761.4 | 1,269.7 |
| 2 MiB | 2 | 682.8 | 490.3 | 697.2 | 112.9 | 225.4 | 111.8 | 459.5 | 691.1 |
| 4 MiB | 2 | 338.1 | 345.5 | 319.2 | 73.7 | 178.0 | 75.6 | 247.5 | 356.8 |

#### HTTP/2: Full upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking MT&#41; | wreq &#40;blocking ST&#41; | ry &#40;blocking&#41; | requests | httpx &#40;blocking&#41; | niquests &#40;blocking&#41; | curl&#95;cffi &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 7,176.9 | 10,635.4 | 7,173.1 | N/A | 2,489.5 | 1,938.6 | 3,187.6 | 13,194.5 |
| 10 KiB | 2 | 7,152.6 | 11,215.6 | 7,141.4 | N/A | 2,478.3 | 1,973.0 | 2,867.0 | 12,290.0 |
| 64 KiB | 2 | 5,032.0 | 5,133.0 | 6,137.7 | N/A | 1,096.5 | 934.7 | 3,631.9 | 8,870.5 |
| 128 KiB | 2 | 4,310.9 | 4,021.9 | 5,253.6 | N/A | 650.0 | 583.6 | 2,397.3 | 6,153.0 |
| 1 MiB | 2 | 1,144.3 | 825.2 | 1,179.8 | N/A | 94.8 | 94.8 | 929.7 | 1,298.9 |
| 2 MiB | 2 | 601.5 | 423.0 | 636.7 | N/A | 45.6 | 48.4 | 527.2 | 651.3 |
| 4 MiB | 2 | 314.7 | 210.7 | 287.2 | N/A | 20.9 | 24.6 | 286.8 | 335.3 |

#### HTTP/2: Stream upload

Unit: requests/s (RPS, requests per second). Higher is better.

| Upload / echo payload | Concurrency | wreq &#40;blocking MT&#41; | wreq &#40;blocking ST&#41; | ry &#40;blocking&#41; | requests | httpx &#40;blocking&#41; | niquests &#40;blocking&#41; | curl&#95;cffi &#40;blocking&#41; | PycURL |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 KiB | 2 | 5,329.7 | 6,347.2 | 6,762.3 | N/A | 2,538.6 | 1,900.8 | 2,543.3 | 10,040.0 |
| 10 KiB | 2 | 5,135.4 | 5,514.6 | 6,466.2 | N/A | 2,452.5 | 1,941.3 | 2,472.4 | 9,658.3 |
| 64 KiB | 2 | 2,952.4 | 2,702.9 | 5,525.7 | N/A | 1,078.9 | 940.1 | 1,922.2 | 6,094.8 |
| 128 KiB | 2 | 2,524.4 | 2,236.2 | 4,818.0 | N/A | 649.1 | 584.4 | 1,891.9 | 5,040.6 |
| 1 MiB | 2 | 662.3 | 511.2 | 1,135.2 | N/A | 100.4 | 92.8 | 666.5 | 1,168.2 |
| 2 MiB | 2 | 438.1 | 326.8 | 609.6 | N/A | 50.2 | 48.2 | 371.4 | 629.7 |
| 4 MiB | 2 | 271.5 | 185.8 | 295.2 | N/A | 25.6 | 24.9 | 195.5 | 311.6 |
