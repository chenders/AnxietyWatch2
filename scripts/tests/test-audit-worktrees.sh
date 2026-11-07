#!/usr/bin/env bash
# Exit-code contract tests for scripts/audit-worktrees.sh.
#
# The contract is the whole point of the script, so these tests assert exit
# codes, not output. The discriminating case is test 5: an unpushed, unmerged
# branch must NOT fail the gate. With ~101 local branches that is the normal
# state, and a gate that fires on it would be disabled within a day.
set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/audit-worktrees.sh"
[ -x "$SCRIPT" ] || { echo "not executable: $SCRIPT"; exit 1; }

PASSED=0; FAILED=0; TMPDIRS=()
cleanup() { for d in ${TMPDIRS+"${TMPDIRS[@]}"}; do
  git -C "$d" worktree prune 2>/dev/null
  rm -rf "$d" "$d-wt" "$d-p1" "$d-p2" 2>/dev/null; done; }
trap cleanup EXIT

ok()   { printf '  ok    %s\n' "$1"; PASSED=$((PASSED+1)); }
nope() { printf '  FAIL  %s\n        %s\n' "$1" "$2"; FAILED=$((FAILED+1)); }

# A repo with a main branch, an origin/main ref, and one linked worktree.
make_repo() {
  local d; d="$(mktemp -d)"; TMPDIRS+=("$d")
  git -C "$d" init -q
  git -C "$d" symbolic-ref HEAD refs/heads/main
  git -C "$d" config user.email t@example.com
  git -C "$d" config user.name 'Audit Test'
  git -C "$d" config commit.gpgsign false
  echo base > "$d/README.md"
  git -C "$d" add README.md
  git -C "$d" commit -qm base
  git -C "$d" update-ref refs/remotes/origin/main refs/heads/main
  git -C "$d" worktree add -q -b feature "$d-wt" main 2>/dev/null
  printf '%s' "$d"
}

run() { ( cd "$1" && "$SCRIPT" >/dev/null 2>&1 ); }

assert_exit() {  # repo, expected, name
  local d=$1 want=$2 name=$3
  run "$d"; local got=$?
  [ "$got" = "$want" ] && ok "$name (exit $got)" \
    || nope "$name" "expected exit $want, got $got"
}

printf '\n── audit-worktrees exit-code contract ──\n'

# 1. Everything committed -> safe.
d="$(make_repo)"
assert_exit "$d" 0 'clean worktrees pass'

# 2. A tracked file modified but not committed -> unsafe.
d="$(make_repo)"
echo changed >> "$d-wt/README.md"
assert_exit "$d" 1 'uncommitted modification fails'

# 3. An untracked file -> unsafe. This is the case that nearly lost 55 files
#    of governance specs on 2026-11-05.
d="$(make_repo)"
echo notes > "$d-wt/scratch.md"
assert_exit "$d" 1 'untracked file fails'

# 4. A paused merge -> unsafe. Conflict resolution lives in the working tree
#    and the index; no ref points at it.
d="$(make_repo)"
git -C "$d-wt" checkout -q -b side
echo side > "$d-wt/conflict.txt"; git -C "$d-wt" add conflict.txt
git -C "$d-wt" commit -qm side
git -C "$d-wt" checkout -q feature
echo feature > "$d-wt/conflict.txt"; git -C "$d-wt" add conflict.txt
git -C "$d-wt" commit -qm feature
git -C "$d-wt" merge side -q 2>/dev/null
assert_exit "$d" 1 'in-progress merge fails'

# 5. DISCRIMINATING: an unpushed, unmerged branch with everything committed
#    must PASS. If this fails, the gate fires on the normal state of ~101
#    branches and will be ignored.
d="$(make_repo)"
echo work > "$d-wt/feature.md"
git -C "$d-wt" add feature.md
git -C "$d-wt" commit -qm 'committed feature work'
assert_exit "$d" 0 'unpushed+unmerged but committed passes'

# 6. FAIL-OPEN REGRESSION: a paused rebase in a worktree whose directory
#    basename collides with another worktree's. git appends a numeric suffix to
#    the second admin directory (`worktrees/aw2-dup` and `worktrees/aw2-dup1`),
#    so deriving the admin path from the basename inspects the wrong worktree.
#    `rebase --exec false` stops with a clean working tree and index, which
#    makes the state marker the only evidence — so a wrong lookup printed
#    "all worktrees are safe to remove" and exited 0 over two commits of work
#    plus the rebase's partially-applied, branch-unreachable HEAD.
d="$(make_repo)"
git -C "$d" worktree add -q -b dup1 "$d-p1/aw2-dup" main
git -C "$d" worktree add -q -b dup2 "$d-p2/aw2-dup" main
echo one > "$d-p2/aw2-dup/one.txt"
git -C "$d-p2/aw2-dup" add one.txt; git -C "$d-p2/aw2-dup" commit -qm one
echo two > "$d-p2/aw2-dup/two.txt"
git -C "$d-p2/aw2-dup" add two.txt; git -C "$d-p2/aw2-dup" commit -qm two
git -C "$d-p2/aw2-dup" rebase main --exec false >/dev/null 2>&1
[ -z "$(git -C "$d-p2/aw2-dup" status --porcelain=v1 -uall)" ] \
  || nope 'fixture invalid' 'paused rebase left a dirty tree; test 6 proves nothing'
assert_exit "$d" 1 'paused rebase under a colliding worktree basename fails'

# 7. A bad base ref is a usage error (2), not a content failure (1).
d="$(make_repo)"
( cd "$d" && "$SCRIPT" origin/nonexistent >/dev/null 2>&1 )
rc=$?
[ "$rc" = 2 ] && ok "missing base ref exits 2 (exit $rc)" \
  || nope 'missing base ref exits 2' "expected 2, got $rc"

printf '\n%s passed, %s failed\n' "$PASSED" "$FAILED"
(( FAILED == 0 )) || exit 1
printf 'audit-worktrees contract holds\n'
