#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

python3 - <<'PY'
import re
from pathlib import Path

specs = Path("docs/specs")
index_path = specs / "AMENDMENTS.md"
text = index_path.read_text(encoding="utf-8")
row_pattern = re.compile(r"^\| \*\*(\d{2})\*\* \| \[[^]]+\]\(([^)]+)\) \|", re.MULTILINE)
rows = row_pattern.findall(text)
if not rows:
    raise SystemExit("amendment-index: FAIL — no numbered amendment rows found")

numbers = [number for number, _ in rows]
links = [link for _, link in rows]
if len(numbers) != len(set(numbers)):
    raise SystemExit("amendment-index: FAIL — duplicate amendment number")
if len(links) != len(set(links)):
    raise SystemExit("amendment-index: FAIL — duplicate amendment link")
expected = [f"{number:02d}" for number in range(1, max(map(int, numbers)) + 1)]
if numbers != expected:
    raise SystemExit(f"amendment-index: FAIL — expected contiguous rows {expected}, got {numbers}")

linked_paths = set()
for number, link in rows:
    path = specs / link
    if not path.is_file():
        raise SystemExit(f"amendment-index: FAIL — missing linked file {path}")
    match = re.search(r"amendment-(\d{2})-", path.name)
    if match is None or match.group(1) != number:
        raise SystemExit(
            f"amendment-index: FAIL — row {number} points to mismatched filename {path.name}"
        )
    linked_paths.add(path.name)

actual_paths = {
    path.name
    for path in specs.glob("*amendment-[0-9][0-9]-*.md")
    if not path.name.startswith("DRAFT-") and re.search(r"amendment-\d{2}-", path.name)
}
if actual_paths != linked_paths:
    raise SystemExit(
        "amendment-index: FAIL — index/file mismatch; "
        f"unindexed={sorted(actual_paths - linked_paths)}, missing={sorted(linked_paths - actual_paths)}"
    )
if "Mandatory reading for every task" not in text:
    raise SystemExit("amendment-index: FAIL — mandatory-reading rule missing")
print(f"amendment-index: PASS — {len(rows)} contiguous amendments indexed")
PY
