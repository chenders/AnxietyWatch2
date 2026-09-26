#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
ledger="$root/docs/specs/2026-09-17-gen1-capability-ledger.md"
[[ -s "$ledger" ]] || { echo "ledger: missing or empty $ledger" >&2; exit 1; }

python3 - "$ledger" <<'PY'
from pathlib import Path
import sys

ledger = Path(sys.argv[1]).read_text(encoding="utf-8")
required = (
    "Redundant push alert channel",
    "Clinical PDF report generation",
    "JSON/CSV export",
    "Correlation recompute",
    "Timezone setting",
    "Schema revision + pending migrations",
    "Deployed build identity",
    "Log sanitisation",
    "Phone has gone dark",
    "Phone-side health-access state",
    "Device↔server reconciliation",
    "Replay window uses server receipt time",
    "Non-finite values are rejected explicitly",
    "Nil-AHI nights are excluded",
    "Baseline windows are unfiltered",
    "Third-party token revocation",
    "Mobile credential token/key revocation",
    "Mobile key rotation",
    "Database read-only enforcement",
)
missing = [item for item in required if item.casefold() not in ledger.casefold()]
if missing:
    raise SystemExit(f"ledger: missing capability rows: {', '.join(missing)}")

rows = [line for line in ledger.splitlines() if line.startswith("|") and not line.startswith("|---")]
if len(rows) < 20:
    raise SystemExit(f"ledger: expected at least 20 assigned rows, found {len(rows)}")
for row in rows:
    columns = [column.strip() for column in row.strip("|").split("|")]
    if len(columns) != 4:
        raise SystemExit(f"ledger: malformed row: {row}")
    if columns[0] == "Capability":
        continue
    if not columns[1] or not columns[2] or not columns[3]:
        raise SystemExit(f"ledger: unassigned row: {row}")

if "Generation 1 remains in service." not in ledger:
    raise SystemExit("ledger: decommission state is not explicit")
print("gen1-capability-ledger: PASS — all migration-risk and named correctness rows are assigned")
PY
