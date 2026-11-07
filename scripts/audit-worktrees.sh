#!/usr/bin/env bash
# Audit this repo's git worktrees for work that would be lost if the worktree
# directory were removed.
#
# Exit code is the contract:
#   0  every worktree is safe to remove (its content is committed to a branch)
#   1  at least one worktree holds work that exists ONLY in its working directory
#
# Why this exists: on 2026-11-05 this repo had 57 `aw2-*` worktrees totalling
# 83 GB. Collapsing them to plain branches is safe — worktrees share one object
# database, so removing a directory never deletes a commit — but twelve of them
# held uncommitted edits or untracked files, and one held a paused merge with 16
# unresolved conflicts. Those are the only states that lose data.
#
# What this script deliberately does NOT treat as a failure: a branch being
# unpushed, or unmerged relative to main. With ~101 local branches that is the
# normal state, and a gate that fires on normal states gets ignored.
#
# The mergedness column uses `git cherry`, not a rev-list ahead-count. This repo
# squash-merged until 2026-11-06, and a squash rewrites commits — so after a
# merge the ahead-count still reports every commit on the branch as unmerged.
# Believing that number means deleting work you think is unmerged, or keeping
# worktrees you think are live. `git cherry` compares patch-ids, which at least
# detects cherry-picks and rebases; it still cannot see through a squash, which
# is why the column is labelled as advisory and never gates the exit code.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "not inside a git repository" >&2; exit 2; }
cd "$ROOT" || exit 2

BASE="${1:-origin/main}"
git rev-parse --verify -q "$BASE" >/dev/null || {
  echo "base ref '$BASE' does not exist" >&2; exit 2; }

GC="$(git rev-parse --git-common-dir)"
unsafe=0

printf '%-34s %-38s %6s %6s %-9s %s\n' \
  WORKTREE BRANCH MOD UNTRACKED STATE "UNMERGED(advisory)"

while IFS= read -r wt; do
  [ -n "$wt" ] || continue
  name=$(basename "$wt")

  # A worktree whose directory is gone is already effectively just a branch.
  if [ ! -d "$wt" ]; then
    printf '%-34s %-38s %6s %6s %-9s %s\n' "$name" "-" "-" "-" "PRUNABLE" "-"
    continue
  fi

  br=$(git -C "$wt" symbolic-ref --quiet --short HEAD 2>/dev/null || echo DETACHED)
  mod=$(git -C "$wt" status --porcelain=v1 --untracked-files=no 2>/dev/null | wc -l | tr -d ' ')
  unt=$(git -C "$wt" status --porcelain=v1 --untracked-files=all 2>/dev/null | grep -c '^??')

  # An in-progress merge/rebase/cherry-pick is the one state that removal
  # destroys irrecoverably: conflict resolution lives in the working tree and
  # the staged index, and no ref points at either.
  wtdir="$GC/worktrees/$name"
  state=clean
  for marker in MERGE_HEAD CHERRY_PICK_HEAD REVERT_HEAD rebase-merge rebase-apply; do
    if [ -e "$wtdir/$marker" ] || { [ "$wt" = "$ROOT" ] && [ -e "$GC/$marker" ]; }; then
      state="$marker"; break
    fi
  done

  if [ "$br" = DETACHED ]; then
    unmerged="n/a"
  else
    unmerged=$(git cherry "$BASE" "$br" 2>/dev/null | grep -c '^+')
  fi

  printf '%-34s %-38s %6s %6s %-9s %s\n' "$name" "$br" "$mod" "$unt" "$state" "$unmerged"

  if [ "$mod" -gt 0 ] || [ "$unt" -gt 0 ] || [ "$state" != clean ]; then
    unsafe=$((unsafe + 1))
  fi
done < <(git worktree list --porcelain | awk '/^worktree /{print substr($0,10)}')

echo
if [ "$unsafe" -eq 0 ]; then
  echo "all worktrees are safe to remove: every change is committed to a branch"
  exit 0
fi
echo "$unsafe worktree(s) hold work that exists only in the working directory." >&2
echo "Removing those directories would lose it. Commit, stash, or archive first." >&2
exit 1
