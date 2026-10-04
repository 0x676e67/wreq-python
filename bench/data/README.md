# Local benchmark data

We keep completed HTTPS measurements here on `main`. Each JSON identifies the
source revision tested and whether the checkout had uncommitted changes. It
also records the interpreter, installed client versions, native artifact hashes,
CPU, OS, architecture and workload.

- Timestamped JSON files are immutable run snapshots.
- `*-provenance.json` sidecars, when present, retain build flags, toolchains,
  dependency and harness hashes, and log checks. They are not measurement
  snapshots and must not be selected as `latest.json`.
- `*-stdout.log` and `*-stderr.log` retain the corresponding run's raw output.
  Successful request validation doesn't explain server connection messages
  or prove that no transport retries occurred.
- `smoke/` contains integration checks, not throughput rankings.
- `latest.json` is a copy of the full measurement selected for the documentation.

Git preserves these JSON and log files without newline conversion, so checking
them out on another platform does not change their raw-file hashes.

The first saved full run has three payload sizes and two concurrency levels.
The expanded suite uses seven payload sizes and four concurrency levels. Check
the workload matrix and upload chunks before comparing runs; they can differ.

Docs builds read the local `latest.json`. They don't run benchmarks, use a
separate data branch or download measurements during page generation.
