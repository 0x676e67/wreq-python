# Local benchmark data

Completed HTTPS measurements are retained here on `main`. Each JSON records
the exact tested source revision, dirty state, interpreter, installed client
versions, native artifact hashes, CPU, OS, architecture, and measured workload.

- Timestamped JSON files are immutable run snapshots.
- `*-provenance.json` sidecars, when present, retain build flags, toolchains,
  dependency and harness hashes, and log checks. They are not measurement
  snapshots and must not be selected as `latest.json`.
- `*-stdout.log` and `*-stderr.log` retain the corresponding run's raw output.
  Passing request validation does not classify server connection diagnostics
  or prove the absence of transport retries.
- `smoke/` contains integration checks, not throughput rankings.
- `latest.json` is a copy of the full measurement selected for the documentation.

Git preserves these JSON and log files without newline conversion, so checking
them out on another platform does not change their raw-file hashes.

The first retained full run has three payload sizes and two concurrency levels.
The expanded suite uses seven payload sizes and four concurrency levels; do not
compare runs as if their workload matrices and upload chunks were identical.

Documentation builds use the local `latest.json`. There is no automatic
benchmark run, separate data branch, or network download during page generation.
