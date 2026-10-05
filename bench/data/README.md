# Local benchmark data

We keep completed HTTPS measurements here on `main`. Each JSON identifies the
source revision tested and whether the checkout had uncommitted changes. It
also records the interpreter, installed client versions, native artifact hashes,
CPU, OS, architecture and workload.

- Timestamped JSON files are immutable run snapshots.
- `*.provenance.json` and `*-provenance.json` sidecars retain build flags, toolchains,
  dependency and harness hashes, and log checks. They are not measurement
  snapshots and must not be selected as `latest.json` or `latest-blocking.json`.
- `*-stdout.log` and `*-stderr.log` retain the corresponding run's raw output.
  Successful request validation doesn't explain server connection messages
  or prove that no transport retries occurred.
- `smoke/` contains integration checks, not throughput rankings.
- `latest.json` is a copy of the full measurement selected for the documentation.
- `latest-blocking.json`, when present, selects a complete blocking-only rerun.
  It replaces the page's blocking results without changing the async source in
  `latest.json`.

Git preserves raw JSON, logs, markers and `.report.md` files without newline
conversion, so checking them out on another platform does not change their hashes.

The first saved full run has three payload sizes and two concurrency levels.
The expanded suite uses seven payload sizes and four concurrency levels. Check
the workload matrix and upload chunks before comparing runs; they can differ.

Docs builds read these local selections as separate sources. They don't run
benchmarks, use a separate data branch or download measurements during page generation.

The [2026-10-04 blocking run](20261004-pr634-blocking/20261004T111716Z-80b48db8b274-wsl-blocking-full.json)
measured clean revision `80b48db8b274` on WSL with a release + jemalloc extension.
It contains 728 supported cells and took 2 h 22 min 39 s. The directory keeps
both short before/after runs, raw logs, build logs and finalized provenance with
their original filenames. The [short comparison](20261004-pr634-blocking/short-comparison.report.md)
uses the same session's main and PR builds; the [full comparison](20261004-pr634-blocking/full-comparison.report.md)
compares common blocking cases with the earlier full WSL snapshot.

All 3,442 server stderr lines in the full rerun are the generic message
`connection error: connection error`. No benchmark failure or traceback was
recorded, and all requests passed the harness's status, protocol and length
checks. The logs don't establish the connection error's cause or whether
internal retries occurred. Gains describe the whole PR, not isolated TLS I/O.

The [2026-10-05 blocking run](20261005-wsl-blocking-mt-st/20261005T063752Z-82b372d0006a-wsl-blocking-full.json)
measured clean revision `82b372d0006a` on the same WSL machine with a release +
jemalloc extension. It contains 840 supported cells across eight blocking clients,
including separate wreq MT and ST runtimes, and took 2 h 24 min 4 s. It uses 300
warmup and 300 timed requests per batch and three rounds. The
[comparison](20261005-wsl-blocking-mt-st/review-comparison.report.md) covers all
728 cases shared with the preceding blocking run and all 112 new MT/ST pairs.
The async selection and older snapshots remain unchanged.

All 3,398 server stderr lines are the same generic connection message described
above; request validation passed and the benchmark exited successfully. The
outer launcher returned an error after completion because its final line had a
Windows line ending. Raw outputs, the original launcher and finalized provenance
retain this distinction. Peer clients also improved by roughly 2.4–3.6% between
sessions, so the comparison does not isolate a wreq code change.
