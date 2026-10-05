set -euo pipefail
root="$HOME/.cache/wreq-python/bench-21"
cd "$root/source"
export RUSTUP_TOOLCHAIN=1.98.0
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
clients=$(.venv/bin/python -c 'from bench.results import BLOCKING_CLIENTS; print(",".join(BLOCKING_CLIENTS))')
stamp=$(date -u +%Y%m%dT%H%M%SZ)
prefix="$root/results/$stamp-82b372d0006a-wsl-blocking-full"
printf '%s\n' "$prefix" > "$root/full-run-location.txt"
.venv/bin/python - "$prefix" <<'PY'
from pathlib import Path
import json, subprocess, time, sys
from datetime import datetime, timezone
p=Path(sys.argv[1])
record={'started_at':datetime.now(timezone.utc).isoformat(),'source':'82b372d0006a5ef71e82b1bcde9df80acba01bb0','uptime':Path('/proc/uptime').read_text(),'load':Path('/proc/loadavg').read_text(),'processes':subprocess.check_output(['ps','-eo','pid,pcpu,pmem,comm','--sort=-pcpu'],text=True)}
p.with_suffix('.started.json').write_text(json.dumps(record,indent=2)+'\n')
PY
set +e
/usr/bin/time -p -o "$prefix.elapsed" .venv/bin/python -u bench/run.py --server "$HOME/.cache/wreq-python/bench-16/wreq-benchmark-server" --clients "$clients" --requests 300 --warmup-requests 300 --rounds 3 --warmup 1 --samples 1 --output "$prefix.json" > "$prefix.runner.log" 2>&1
status=$?
set -e
printf '%s\n' "$status" > "$prefix.exit-code"
date -u +%Y-%m-%dT%H:%M:%SZ > "$prefix.finished"
tail -n 8 "$prefix.runner.log"
exit "$status"
