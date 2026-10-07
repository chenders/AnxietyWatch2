#!/usr/bin/env bash
# compare-one-app.sh — run the app-comparison pipeline on exactly one candidate at a time.
#
#   compare-one-app.sh start  "<app name or URL>" [slug]   admit a candidate and open its row
#   compare-one-app.sh status                              in-progress row, queue, derived file counts
#   compare-one-app.sh finish <slug>                       close a candidate after verifying deliverables
#   compare-one-app.sh lab    <slug>                       start the inspection lab (proxy + emulator)
#
# Design rules:
#   * One candidate at a time. `start` refuses while another row is open; there is no --force.
#   * The script never commits and never touches main. It prints the commit command instead.
#   * Every refusal names its blocker and the value that blocked it.
set -Eeuo pipefail

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
STATE="$ROOT_DIR/docs/app_comparison/PROCESS_STATE.md"
PKG_DIR="$ROOT_DIR/docs/app_comparison"
RESEARCH_DIR="${AW2_RESEARCH_DIR:-$HOME/aw2research}"
DELIVERABLES=(
  "CLAIMED_FEATURES_AND_PURPOSE"
  "IMPLEMENTATION_INSPECTION"
  "COMPARISON"
)

die() { printf 'compare-one-app: %s\n' "$*" >&2; exit 1; }
note() { printf '%s\n' "$*"; }

# The package is local-only: gitignored, untracked, and refused at the push boundary in
# the parent repository. If it is tracked here, this worktree predates that rule, and
# starting a candidate would produce deliverables that could be committed by accident.
require_local_only() {
  local tracked
  tracked=$(git -C "$ROOT_DIR" ls-files docs/app_comparison 2>/dev/null | head -5)
  if [[ -n "$tracked" ]]; then
    die "docs/app_comparison is tracked in this worktree, so it is not local-only:
$tracked
     It must never be committed or pushed (docs/app_comparison/LOCAL-ONLY.md).
     Fix this worktree once with:  git rm -r --cached docs/app_comparison
     and confirm:                  git ls-files docs/app_comparison   # must print nothing"
  fi
}

usage() {
  cat >&2 <<'EOF'
Usage:
  compare-one-app.sh start  "<app name or URL>" [slug]
  compare-one-app.sh status
  compare-one-app.sh finish <slug>
  compare-one-app.sh lab    <slug>
EOF
  exit 2
}

[[ $# -ge 1 ]] || usage

# ---------------------------------------------------------------- state helpers
# The queue is the table under "## 2. Queue". Rows look like:
#   | 1 | bearable | T1 | not-started | - | - | 0/4 |
py_read() {  # py_read <slug-or-empty>  -> prints matching rows (tab separated)
  python3 - "$STATE" "${1:-}" <<'PY'
import re, sys
path, want = sys.argv[1], sys.argv[2]
rows = []
for line in open(path, encoding="utf-8"):
    m = re.match(r"^\|\s*(\d+)\s*\|\s*([a-z0-9-]+)\s*\|\s*(T\d)\s*\|\s*([a-z-]+)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*$", line)
    if m:
        rows.append(m.groups())
if want:
    rows = [r for r in rows if r[1] == want]
for r in rows:
    print("\t".join(x.strip() for x in r))
PY
}

py_write() {  # py_write <slug> <status> <started> <finished>
  python3 - "$STATE" "$1" "$2" "$3" "$4" <<'PY'
import re, sys
path, slug, status, started, finished = sys.argv[1:6]
out, hit = [], False
for line in open(path, encoding="utf-8"):
    m = re.match(r"^\|\s*(\d+)\s*\|\s*([a-z0-9-]+)\s*\|\s*(T\d)\s*\|\s*([a-z-]+)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*$", line)
    if m and m.group(2) == slug:
        hit = True
        line = f"| {m.group(1)} | {slug} | {m.group(3)} | {status} | {started} | {finished} | {m.group(7).strip()} |\n"
    out.append(line)
if not hit:
    sys.exit(f"compare-one-app: slug '{slug}' is not a row in the queue")
open(path, "w", encoding="utf-8").writelines(out)
PY
}

py_set_files() {  # py_set_files <slug>
  local slug=$1 n=0 f
  for f in "${DELIVERABLES[@]}"; do [[ -s "$PKG_DIR/$slug/${slug}_${f}.md" ]] && n=$((n + 1)); done
  [[ -s "$PKG_DIR/$slug/PROMPT_2.md" ]] && n=$((n + 1))
  python3 - "$STATE" "$slug" "$n" <<'PY'
import re, sys
path, slug, n = sys.argv[1], sys.argv[2], sys.argv[3]
out = []
for line in open(path, encoding="utf-8"):
    m = re.match(r"^\|\s*(\d+)\s*\|\s*([a-z0-9-]+)\s*\|\s*(T\d)\s*\|\s*([a-z-]+)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*([^|]*?)\s*\|\s*$", line)
    if m and m.group(2) == slug:
        line = f"| {m.group(1)} | {slug} | {m.group(3)} | {m.group(4)} | {m.group(5).strip()} | {m.group(6).strip()} | {n}/4 |\n"
    out.append(line)
open(path, "w", encoding="utf-8").writelines(out)
PY
  printf '%s' "$n"
}

slugify() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]' | sed -e 's|https\?://||' -e 's|/.*$||' -e 's/^www\.//' -e 's/\..*$//' -e 's/[^a-z0-9]/-/g' -e 's/--*/-/g' -e 's/^-//' -e 's/-$//'; }

open_row() { py_read | awk -F'\t' '$4 == "in-progress" { print $2 }'; }

# ---------------------------------------------------------------- commands
cmd_start() {
  require_local_only
  local app=${1:-} slug=${2:-} existing
  [[ -n "$app" ]] || usage
  [[ -n "$slug" ]] || slug=$(slugify "$app")
  [[ -n "$slug" ]] || die "could not derive a slug from '$app'"

  existing=$(open_row)
  if [[ -n "$existing" ]]; then
    die "refusing to start '$slug': candidate '$existing' is still in-progress.
     One candidate at a time (PROMPT1.md section 3). Finish it first:
       scripts/compare-one-app.sh finish $existing"
  fi

  py_read "$slug" >/dev/null
  [[ -n "$(py_read "$slug")" ]] || die "slug '$slug' is not a row in $STATE.
     Add it to the queue first (or use a reserve from CANDIDATE_SHORTLIST.md section 4.2)."

  mkdir -p "$PKG_DIR/$slug" "$RESEARCH_DIR/$slug"
  py_write "$slug" "in-progress" "$(date -u +%Y-%m-%d)" "-"
  py_set_files "$slug" >/dev/null

  note "admitted: $slug"
  note "  deliverable dir : docs/app_comparison/$slug/"
  note "  scratch dir     : $RESEARCH_DIR/$slug/   (never committed — see RESEARCH_METHOD.md section 7)"
  note ""
  note "Next, in this order:"
  note "  1. public claims      -> docs/app_comparison/$slug/${slug}_CLAIMED_FEATURES_AND_PURPOSE.md"
  note "  2. decompile + read   -> docs/app_comparison/$slug/${slug}_IMPLEMENTATION_INSPECTION.md"
  note "     scripts/app-decompile.sh android $slug      # or: ios"
  note "  3. runtime, if a device exists:  scripts/compare-one-app.sh lab $slug"
  note "  4. reconciliation table in the inspection file, every V-n claim with a verdict"
  note "  5. write docs/app_comparison/$slug/PROMPT_2.md from PROMPT2.md (record the lens variant)"
  note "  6. write docs/app_comparison/$slug/${slug}_COMPARISON.md (14 sections)"
  note "  7. verify quotes in both modes (RESEARCH_METHOD.md section 5), update FINDINGS.md, then:"
  note "     scripts/compare-one-app.sh finish $slug"
}

cmd_status() {
  local open slug row n
  open=$(open_row)
  if [[ -z "$open" ]]; then
    note "in progress: none"
  else
    note "in progress: $open"
  fi
  note ""
  printf '%-3s %-14s %-4s %-12s %-11s %-11s %s\n' "#" "SLUG" "TIER" "STATUS" "STARTED" "FINISHED" "FILES"
  while IFS=$'\t' read -r num slug tier status started finished files; do
    n=0
    for f in "${DELIVERABLES[@]}"; do [[ -s "$PKG_DIR/$slug/${slug}_${f}.md" ]] && n=$((n + 1)); done
    [[ -s "$PKG_DIR/$slug/PROMPT_2.md" ]] && n=$((n + 1))
    printf '%-3s %-14s %-4s %-12s %-11s %-11s %s/4\n' "$num" "$slug" "$tier" "$status" "$started" "$finished" "$n"
  done < <(py_read)
}

cmd_finish() {
  require_local_only
  local slug=${1:-} missing=() f row status
  [[ -n "$slug" ]] || usage
  row=$(py_read "$slug")
  [[ -n "$row" ]] || die "slug '$slug' is not a row in the queue"
  status=$(printf '%s' "$row" | cut -f4)
  [[ "$status" == "in-progress" ]] || die "refusing to finish '$slug': its status is '$status', not 'in-progress'"

  for f in "${DELIVERABLES[@]}"; do
    [[ -s "$PKG_DIR/$slug/${slug}_${f}.md" ]] || missing+=("${slug}_${f}.md")
  done
  [[ -s "$PKG_DIR/$slug/PROMPT_2.md" ]] || missing+=("PROMPT_2.md")
  if (( ${#missing[@]} > 0 )); then
    printf 'compare-one-app: refusing to finish %s — %d of 4 deliverables missing:\n' "$slug" "${#missing[@]}" >&2
    printf '    %s\n' "${missing[@]}" >&2
    exit 1
  fi

  grep -q -- "$slug" "$PKG_DIR/FINDINGS.md" || die "refusing to finish '$slug':
     FINDINGS.md does not mention it. The summary and the deliverables ship in the same commit
     (PROCESS_STATE.md section 6). Add its row to FINDINGS.md first."

  py_write "$slug" "complete" "$(printf '%s' "$row" | cut -f5)" "$(date -u +%Y-%m-%d)"
  local n; n=$(py_set_files "$slug")
  note "closed: $slug  ($n/4 deliverables, FINDINGS.md updated)"
  note ""
  note "Commit it in the LOCAL-ONLY repository — never in the AnxietyWatch2 repo (LOCAL-ONLY.md):"
  note "  git -C docs/app_comparison add -A"
  note "  git -C docs/app_comparison commit -m 'research(app-comparison): candidate <N> $slug — <mode> inspection'"
}

cmd_lab() {
  local slug=${1:-}
  [[ -n "$slug" ]] || usage
  command -v mitmdump >/dev/null || die "mitmdump not found. Install: brew install mitmproxy"
  command -v adb >/dev/null || die "adb not found. Install the Android SDK command-line tools."

  local confdir="$RESEARCH_DIR/$slug/mitm" flows="$RESEARCH_DIR/$slug/flows.mitm"
  mkdir -p "$confdir"
  note "starting mitmdump on 0.0.0.0:8080  (flows -> $flows)"
  mitmdump --listen-host 0.0.0.0 --listen-port 8080 --set "confdir=$confdir" -w "$flows" &
  local pid=$!
  trap 'note "stopping mitmdump"; kill '"$pid"' 2>/dev/null || true' EXIT
  sleep 2

  if adb get-state >/dev/null 2>&1; then
    adb shell settings put global http_proxy 10.0.2.2:8080
    note "emulator proxy set: $(adb shell settings get global http_proxy | tr -d '\r')"
    note "install the proxy CA in this disposable emulator only, then install and launch the target."
  else
    note "no device/emulator attached — proxy is up, nothing configured. Connect one and re-run, or set it by hand:"
    note "  adb shell settings put global http_proxy 10.0.2.2:8080"
  fi
  note ""
  note "Synthetic data only. No real user data, no production accounts (PROMPT1.md section 2)."
  note "Press Ctrl-C to stop and clear the proxy."
  wait "$pid" || true
  if adb get-state >/dev/null 2>&1; then
    adb shell settings put global http_proxy :0 >/dev/null 2>&1 || true
    note "emulator proxy cleared"
  fi
}

case "$1" in
  start)  shift; cmd_start  "$@" ;;
  status) shift; cmd_status ;;
  finish) shift; cmd_finish "$@" ;;
  lab)    shift; cmd_lab    "$@" ;;
  -h|--help) usage ;;
  *) usage ;;
esac
