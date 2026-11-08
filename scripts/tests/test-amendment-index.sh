#!/usr/bin/env bash
# Regressions for scripts/verify-amendment-index.sh.
#
# Written because that script failed on a correct index for two months without
# anyone noticing: nothing in CI or the pre-push hook ran it, and the only
# callers were three task Verify lines belonging to tasks nobody had reached.
# A verifier nothing verifies drifts.
#
# Each case builds a throwaway docs/specs tree and asserts the script's exit
# code and the reason it gives, so a future change that makes it pass or fail
# for the wrong reason is caught rather than merely noticed.
set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$REPO_ROOT/scripts/verify-amendment-index.sh"
PASSED=0
FAILED=0
TMPDIRS=()

cleanup() { for d in ${TMPDIRS+"${TMPDIRS[@]}"}; do rm -rf "$d"; done; }
trap cleanup EXIT

ok()   { printf '  ok    %s\n' "$1"; PASSED=$((PASSED + 1)); }
nope() { printf '  FAIL  %s\n        %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

HEADER='# Amendment index

Mandatory reading for every task.

## Accepted amendments

| # | Title | Supersedes | Affects |
|---|---|---|---|
'

# Build a repo whose docs/specs holds exactly the rows given on stdin, plus one
# file per row so the on-disk set matches unless a test says otherwise.
make_tree() {  # rows... ; echoes the directory
  local d
  d="$(mktemp -d)"
  TMPDIRS+=("$d")
  git -C "$d" init -q
  git -C "$d" config user.email test@example.com
  git -C "$d" config user.name 'Index Test'
  mkdir -p "$d/docs/specs" "$d/scripts"
  cp "$SCRIPT" "$d/scripts/verify-amendment-index.sh"
  chmod +x "$d/scripts/verify-amendment-index.sh"
  printf '%s' "$HEADER" > "$d/docs/specs/AMENDMENTS.md"
  printf '%s' "$d"
}

add_row() {  # dir, key, filename
  printf '| **%s** | [Title %s](%s) | x | y |\n' "$2" "$2" "$3" >> "$1/docs/specs/AMENDMENTS.md"
}

add_file() {  # dir, filename
  printf '# %s\n' "$2" > "$1/docs/specs/$2"
}

run_in() {  # dir -> prints output, returns exit code
  ( cd "$1" && bash scripts/verify-amendment-index.sh 2>&1 )
}

assert_pass() {  # dir, name
  local out rc
  out="$(run_in "$1")"; rc=$?
  if (( rc != 0 )); then
    nope "$2" "expected exit 0, got $rc: $(tr '\n' ' ' <<<"$out" | tail -c 200)"
  else
    ok "$2"
  fi
}

assert_fail() {  # dir, name, expected substring
  local out rc
  out="$(run_in "$1")"; rc=$?
  if (( rc == 0 )); then
    nope "$2" "expected a nonzero exit, got 0"
  elif ! grep -qF -- "$3" <<<"$out"; then
    nope "$2" "failed, but not for '$3'. Got: $(tr '\n' ' ' <<<"$out" | tail -c 220)"
  else
    ok "$2"
  fi
}

printf '\n── amendment-index verifier ──\n'

# 1. The shape that was failing on main: a suffixed parallel series whose files
#    share a leading number with the main series.
d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md';  add_file "$d" '2026-01-01-amendment-01-first.md'
add_row "$d" '02' '2026-01-02-amendment-02-second.md'; add_file "$d" '2026-01-02-amendment-02-second.md'
add_row "$d" '02-A11' '2026-01-03-amendment-02-parallel.md'
add_file "$d" '2026-01-03-amendment-02-parallel.md'
assert_pass "$d" 'a suffixed parallel series is accepted'

# 2. The main series must still be contiguous.
d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md'; add_file "$d" '2026-01-01-amendment-01-first.md'
add_row "$d" '03' '2026-01-03-amendment-03-third.md'; add_file "$d" '2026-01-03-amendment-03-third.md'
assert_fail "$d" 'a gap in the main series fails' 'must be contiguous'

# 3. A suffixed row must not be able to paper over a gap: 01, 03, 02-A11 still
#    leaves the main series missing 02.
d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md'; add_file "$d" '2026-01-01-amendment-01-first.md'
add_row "$d" '03' '2026-01-03-amendment-03-third.md'; add_file "$d" '2026-01-03-amendment-03-third.md'
add_row "$d" '02-A11' '2026-01-02-amendment-02-parallel.md'
add_file "$d" '2026-01-02-amendment-02-parallel.md'
assert_fail "$d" 'a suffixed row cannot fill a main-series gap' 'must be contiguous'

# 4. A file on disk that no row mentions.
d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md'; add_file "$d" '2026-01-01-amendment-01-first.md'
add_file "$d" '2026-01-09-amendment-09-orphan.md'
assert_fail "$d" 'an unindexed file fails and is named' 'amendment-09-orphan'

# 5. A row pointing at a file that is not there.
d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md'
assert_fail "$d" 'a row linking a missing file fails' 'links missing file'

# 6. A row whose number disagrees with its filename.
d="$(make_tree)"
add_row "$d" '01' '2026-01-07-amendment-07-mismatched.md'
add_file "$d" '2026-01-07-amendment-07-mismatched.md'
assert_fail "$d" 'a number/filename mismatch fails' 'whose number is 07, not 01'

# 7. Duplicate keys and duplicate links.
d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md'; add_file "$d" '2026-01-01-amendment-01-first.md'
add_row "$d" '01' '2026-01-02-amendment-01-again.md'; add_file "$d" '2026-01-02-amendment-01-again.md'
assert_fail "$d" 'a duplicate number fails' 'duplicate amendment number'

d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md'; add_file "$d" '2026-01-01-amendment-01-first.md'
add_row "$d" '02' '2026-01-01-amendment-01-first.md'
assert_fail "$d" 'a duplicate link fails' 'duplicate amendment link'

# 8. The mandatory-reading rule is what makes the index binding, so its absence
#    is a failure rather than a formatting nit.
d="$(make_tree)"
add_row "$d" '01' '2026-01-01-amendment-01-first.md'; add_file "$d" '2026-01-01-amendment-01-first.md'
sed -i.bak 's/^Mandatory reading for every task\.$//' "$d/docs/specs/AMENDMENTS.md"
assert_fail "$d" 'a missing mandatory-reading rule fails' 'mandatory-reading rule missing'

# 9. An index with no rows at all must fail rather than vacuously pass — the
#    empty-set trap this repository has hit in three other gates.
d="$(make_tree)"
assert_fail "$d" 'an index with no rows fails rather than passing vacuously' 'no numbered amendment rows'

# 10. And the real repository passes.
if ( cd "$REPO_ROOT" && bash scripts/verify-amendment-index.sh >/dev/null 2>&1 ); then
  ok 'the repository'"'"'s own index passes'
else
  nope 'the repository'"'"'s own index passes' "$(cd "$REPO_ROOT" && bash scripts/verify-amendment-index.sh 2>&1 | tail -1)"
fi

printf '\n%s passed, %s failed\n' "$PASSED" "$FAILED"
(( FAILED == 0 )) || exit 1
printf 'amendment-index verifier regressions passed\n'
