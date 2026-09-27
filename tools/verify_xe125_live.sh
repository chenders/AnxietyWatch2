#!/usr/bin/env bash
# Verify the bounded XE125 bench evidence directory.
set -euo pipefail

root=${1:?usage: verify_xe125_live.sh ARTIFACT_DIR}
port_file="$root/enhanced-port.txt"
firmware_file="$root/firmware.txt"
jsonl="$root/capture.jsonl"
h5="$root/capture.h5"

for file in "$port_file" "$firmware_file" "$jsonl" "$h5"; do
  test -s "$file" || { echo "XE125 verify: missing or empty $file" >&2; exit 1; }
done
grep -qi 'enhanced' "$port_file" || {
  echo "XE125 verify: enhanced serial port is not identified in $port_file" >&2
  exit 1
}
port=$(sed -n 's/^port=//p' "$port_file" | head -n 1)
test -n "$port" || { echo "XE125 verify: $port_file has no port= entry" >&2; exit 1; }
python3 - "$jsonl" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
count = 0
observations = []
for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
    if not line.strip():
        continue
    try:
        item = json.loads(line)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"XE125 verify: invalid JSONL at line {line_number}: {exc}")
    if not isinstance(item, dict) or not item.get("state"):
        raise SystemExit(f"XE125 verify: line {line_number} lacks an explicit state")
    observed = item.get("observed_at_unix_micros")
    if not isinstance(observed, int) or observed < 0:
        raise SystemExit(f"XE125 verify: line {line_number} lacks an integer observation time")
    observations.append(observed)
    count += 1
if count == 0:
    raise SystemExit("XE125 verify: JSONL contains no frames")
duration_s = (max(observations) - min(observations)) / 1_000_000
if duration_s > 600:
    raise SystemExit(f"XE125 verify: capture duration {duration_s:.3f}s exceeds ten minutes")
print(f"XE125 verify: {count} state-preserving frames over {duration_s:.3f}s")
PY
printf 'XE125 verify: firmware and H5 evidence present\n'
