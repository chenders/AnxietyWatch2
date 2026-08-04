#!/usr/bin/env bash
# herdr-grid.sh — create a tab of N equally-sized panes.
#
# Why this exists: `herdr pane split --ratio 0.5` cascades. Splitting each new
# pane in half yields 1/2, 1/4, 1/8, 1/16 …, so five agents end up at 43/22/11/6/5
# rows. herdr has no "even-vertical" command, and `pane resize --amount` takes a
# ratio of the containing split rather than a cell count, so rebalancing after
# the fact converges slowly and lands a row or two off.
#
# The fix is arithmetic at split time: when a pane must become n equal parts,
# the first split takes 1/n, the next 1/(n-1), and so on. The last split is 1/2.
#
# It also builds a GRID rather than a single stack. Five agents in one column of
# a 172x43 terminal get 8 rows each, which is unreadable; 3x2 gives 57x21.
#
# Usage:
#   herdr-grid.sh <label> <n> [cwd] [name1 name2 ...]
#
# Example:
#   herdr-grid.sh spec-review 5 ~/Source/AnxietyWatch2 openai google xai deepseek moonshot
set -euo pipefail

LABEL="${1:?usage: herdr-grid.sh <label> <n> [cwd] [names...]}"
N="${2:?number of panes}"
CWD="${3:-$PWD}"
shift 3 2>/dev/null || shift 2
NAMES=("$@")

pane_id() { uv run python3 -c "import json,sys; d=json.load(sys.stdin)['result']; print(d.get('pane',d.get('root_pane'))['pane_id'])"; }

# Column layout: keep panes at least ~50 columns wide, which is the practical
# floor for reading agent output.
if   (( N <= 2 )); then COLS=$N
elif (( N <= 6 )); then COLS=$(( (N + 1) / 2 ))
else                    COLS=3
fi
ROWS_MAX=$(( (N + COLS - 1) / COLS ))

WS="$(herdr workspace list 2>/dev/null | uv run python3 -c "
import json,sys
w=json.load(sys.stdin)['result']['workspaces']
print(next((x['workspace_id'] for x in w if x.get('focused')), w[0]['workspace_id']))")"

ROOT="$(herdr tab create --workspace "$WS" --cwd "$CWD" --label "$LABEL" --focus | pane_id)"

# 1. Split the root into COLS equal columns: 1/COLS, 1/(COLS-1), … , 1/2.
COLUMNS=("$ROOT")
prev="$ROOT"
for (( c=0; c<COLS-1; c++ )); do
    remaining=$(( COLS - c ))
    ratio=$(uv run python3 -c "print(f'{1/$remaining:.4f}')")
    new="$(herdr pane split "$prev" --direction right --ratio "$ratio" --cwd "$CWD" --no-focus | pane_id)"
    COLUMNS+=("$new")
    prev="$new"
done

# 2. Split each column into its share of the rows, same 1/(n-k+1) progression.
ALL=()
idx=0
for (( c=0; c<COLS; c++ )); do
    # Distribute N panes over COLS columns as evenly as possible.
    rows=$(( N / COLS ))
    (( c < N % COLS )) && rows=$(( rows + 1 ))
    (( rows == 0 )) && continue
    col="${COLUMNS[$c]}"
    ALL+=("$col"); (( idx++ ))
    prev="$col"
    for (( r=0; r<rows-1; r++ )); do
        remaining=$(( rows - r ))
        ratio=$(uv run python3 -c "print(f'{1/$remaining:.4f}')")
        new="$(herdr pane split "$prev" --direction down --ratio "$ratio" --cwd "$CWD" --no-focus | pane_id)"
        ALL+=("$new"); (( idx++ ))
        prev="$new"
    done
done

for i in "${!ALL[@]}"; do
    name="${NAMES[$i]-}"; [[ -z "$name" ]] && name="pane-$((i+1))"
    herdr pane rename "${ALL[$i]}" "$name" >/dev/null 2>&1 || true
done

echo "tab '$LABEL': ${#ALL[@]} panes in ${COLS} columns (max ${ROWS_MAX} rows)"
herdr pane layout 2>/dev/null | uv run python3 -c "
import json,sys
ps=json.load(sys.stdin)['result']['layout']['panes']
for p in ps:
    r=p['rect']; print(f\"  {p['pane_id']:8} {r['width']:>3}w x {r['height']:>2}h\")
hs=[p['rect']['height'] for p in ps]; ws=[p['rect']['width'] for p in ps]
print(f'  spread: height {max(hs)-min(hs)}, width {max(ws)-min(ws)}')
"
printf '%s\n' "${ALL[@]}"
