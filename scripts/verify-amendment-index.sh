#!/usr/bin/env bash
# Check that docs/specs/AMENDMENTS.md and the amendment files agree.
#
# `AGENTS.md` makes the index mandatory reading for every task, so an index that
# disagrees with the files on disk is a correctness problem and not a tidiness
# one: a worker obeying the scoping rule reads the index and nothing else.
#
# The index carries TWO numbering series, and that is the thing this script got
# wrong until 2026-11-08. The main series is `01`..`NN`. The AirSense series is
# suffixed -- `10-A11` and `11-A11` -- because those two amendments were written
# on 2026-09-13 as plain 10 and 11, and the main series then reused both numbers
# two days later; the index disambiguated them with a suffix while the filenames
# still read `amendment-10-` and `amendment-11-`.
#
# The old row pattern matched `**NN**` only, so the two suffixed rows were
# invisible to it while their files still matched the on-disk glob. The script
# therefore reported them as unindexed and exited nonzero on a correct index --
# failing on `main` continuously, and blocking the three task Verify lines that
# invoke it (T-M3a, admin-console-roadmap:648, ios-client-stage7:88).
#
# Teaching the script the real convention is preferable to renaming the files:
# twenty-odd citations point at those names, and the suffix is already how the
# index expresses the distinction.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
cd "$root"

PY=python3
command -v uv >/dev/null && PY="uv run python3"

$PY - <<'PY'
import re
from pathlib import Path

specs = Path("docs/specs")
index_path = specs / "AMENDMENTS.md"
text = index_path.read_text(encoding="utf-8")

# A key is `NN` for the main series or `NN-SUFFIX` for a parallel one.
row_pattern = re.compile(
    r"^\| \*\*(\d{2}(?:-[A-Za-z0-9]+)?)\*\* \| \[[^]]+\]\(([^)]+)\) \|", re.MULTILINE
)
rows = row_pattern.findall(text)
if not rows:
    raise SystemExit("amendment-index: FAIL — no numbered amendment rows found")

keys = [key for key, _ in rows]
links = [link for _, link in rows]

duplicate_keys = sorted({k for k in keys if keys.count(k) > 1})
if duplicate_keys:
    raise SystemExit(f"amendment-index: FAIL — duplicate amendment number(s) {duplicate_keys}")
duplicate_links = sorted({l for l in links if links.count(l) > 1})
if duplicate_links:
    raise SystemExit(f"amendment-index: FAIL — duplicate amendment link(s) {duplicate_links}")

# Contiguity applies to the main series only. A suffixed row is a parallel
# decision that shares a leading number on purpose, so counting it would demand
# a gap in the main sequence that must not exist.
main = [key for key in keys if "-" not in key]
if not main:
    raise SystemExit("amendment-index: FAIL — no unsuffixed amendment rows found")
expected = [f"{number:02d}" for number in range(1, max(map(int, main)) + 1)]
if main != expected:
    raise SystemExit(
        f"amendment-index: FAIL — main series must be contiguous {expected}, got {main}"
    )

linked_paths = set()
for key, link in rows:
    path = specs / link
    if not path.is_file():
        raise SystemExit(f"amendment-index: FAIL — row {key} links missing file {path}")
    match = re.search(r"amendment-(\d{2})-", path.name)
    if match is None:
        raise SystemExit(
            f"amendment-index: FAIL — row {key} points to {path.name}, which carries no "
            "amendment-NN- element"
        )
    # The filename carries the leading number; the suffix lives only in the
    # index, which is how `10` and `10-A11` can both exist on disk as
    # `amendment-10-`.
    leading = key.split("-", 1)[0]
    if match.group(1) != leading:
        raise SystemExit(
            f"amendment-index: FAIL — row {key} points to {path.name}, whose number is "
            f"{match.group(1)}, not {leading}"
        )
    linked_paths.add(path.name)

actual_paths = {
    path.name
    for path in specs.glob("*amendment-[0-9][0-9]-*.md")
    if not path.name.startswith("DRAFT-") and re.search(r"amendment-\d{2}-", path.name)
}
unindexed = sorted(actual_paths - linked_paths)
missing = sorted(linked_paths - actual_paths)
if unindexed or missing:
    raise SystemExit(
        "amendment-index: FAIL — index and files disagree; "
        f"on disk but unindexed={unindexed}, indexed but absent={missing}"
    )

if "Mandatory reading for every task" not in text:
    raise SystemExit("amendment-index: FAIL — mandatory-reading rule missing")

suffixed = sorted(key for key in keys if "-" in key)
print(
    f"amendment-index: PASS — {len(main)} contiguous main amendments"
    + (f", {len(suffixed)} suffixed ({', '.join(suffixed)})" if suffixed else "")
)
PY
