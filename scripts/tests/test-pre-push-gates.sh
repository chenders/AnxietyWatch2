#!/usr/bin/env bash
# Regression tests for the .githooks/pre-push content-exposure gates.
#
# Why this exists: the secrets, capture-artifact and PHI/device gates keyed off
# `git diff "$remote..$local"` — a two-dot diff between the two endpoint trees.
# A branch that added a real device address in commit 12 and redacted it in
# commit 40 therefore netted to zero and sailed through, while the address
# stayed in the branch's history forever. Squash-merge was quietly containing
# that: the dirty commits never became ancestors of main. Once merge commits are
# allowed, they do, so the gates have to inspect every commit being pushed.
#
# Fixture values are assembled at runtime from fragments on purpose. A literal
# device address or credential in this file would be caught by the very gates it
# tests, blocking the commit that adds the test. Fragments keep the gate's blast
# radius unchanged — the alternative, a pathspec exemption for this file, would
# carve a permanent hole that a real secret could later hide in.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HOOK="$REPO_ROOT/.githooks/pre-push"
SCANNER="$REPO_ROOT/scripts/check-public-info.py"

PASSED=0
FAILED=0
TMPDIRS=()

cleanup() { for d in ${TMPDIRS+"${TMPDIRS[@]}"}; do rm -rf "$d"; done; }
trap cleanup EXIT

ok()   { printf '  ok    %s\n' "$1"; PASSED=$((PASSED + 1)); }
nope() { printf '  FAIL  %s\n        %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# A throwaway repo carrying the hook and the public-info scanner it shells out
# to. The root commit is the pre-existing state: the hook's no-origin/main
# fallback is ROOT_COMMIT..HEAD, so the root is excluded from the scanned range
# and everything committed after it is the "push" under test.
make_repo() {
  local d
  d="$(mktemp -d)"
  TMPDIRS+=("$d")
  git -C "$d" init -q
  # symbolic-ref rather than `init -b`: works on every git version CI might have
  git -C "$d" symbolic-ref HEAD refs/heads/feature
  git -C "$d" config user.email test@example.com
  git -C "$d" config user.name 'Gate Test'
  git -C "$d" config commit.gpgsign false
  mkdir -p "$d/scripts" "$d/.githooks"
  cp "$SCANNER" "$d/scripts/check-public-info.py"
  cp "$HOOK" "$d/.githooks/pre-push"
  chmod +x "$d/.githooks/pre-push"
  # The scanner is copied in, not stubbed, so the hook's own interpreter
  # resolution (`uv run python3` when uv is on PATH, bare python3 otherwise)
  # is what runs — some dev machines carry a shim that rejects bare python3,
  # and stubbing uv would hide that rather than exercise it.
  echo 'baseline' > "$d/README.md"
  git -C "$d" add README.md scripts/check-public-info.py .githooks/pre-push
  git -C "$d" commit -qm 'baseline'
  printf '%s' "$d"
}

commit_in() {  # repo, message, [files...]
  local d=$1 msg=$2
  shift 2
  git -C "$d" add -- "$@"
  git -C "$d" commit -qm "$msg"
}

# Feed the hook the stdin git would: "<local_ref> <local_sha> <remote_ref> <remote_sha>".
# remote_sha is the root commit, so the pushed range is everything after it.
run_hook() {
  local d=$1 base tip
  base="$(git -C "$d" rev-list --max-parents=0 HEAD | head -1)"
  tip="$(git -C "$d" rev-parse HEAD)"
  (
    cd "$d" || exit 1
    printf 'refs/heads/feature %s refs/heads/feature %s\n' "$tip" "$base" \
      | ./.githooks/pre-push origin 'git@example.com:owner/repo.git' 2>&1
  )
}

assert_blocked() {  # repo, test name, expected substring
  local d=$1 name=$2 want=$3 out rc
  out="$(run_hook "$d")"
  rc=$?
  if (( rc == 0 )); then
    nope "$name" "hook exited 0 — the commit-local exposure was not caught"
  elif ! grep -qF -- "$want" <<<"$out"; then
    nope "$name" "blocked, but not for '$want'. Got: $(tr '\n' ' ' <<<"$out" | tail -c 200)"
  else
    ok "$name"
  fi
}

assert_passed() {  # repo, test name
  local d=$1 name=$2 out rc
  out="$(run_hook "$d")"
  rc=$?
  if (( rc != 0 )); then
    nope "$name" "hook blocked a clean branch: $(tr '\n' ' ' <<<"$out" | tail -c 250)"
  else
    ok "$name"
  fi
}

printf '\n── pre-push gate regressions ──\n'

# 1. Credential introduced, then removed before the tip.
d="$(make_repo)"
printf 'API_%s = "%s"\n' 'KEY' 'abcdefghij0123456789' > "$d/service.conf"
commit_in "$d" 'add service config' service.conf
printf 'API_%s = ""\n' 'KEY' > "$d/service.conf"
commit_in "$d" 'drop the inline value' service.conf
assert_blocked "$d" 'credential added then removed is blocked' 'a credential value appears'

# 2. Full device address introduced, then redacted to the OUI before the tip.
#    This is the exact shape that produced the -sanitized branches in this repo.
d="$(make_repo)"
printf 'sensor %s:%s\n' '11:22:33' '44:55:66' > "$d/docs-device.md"
commit_in "$d" 'record the sensor' docs-device.md
printf 'sensor %s:XX:XX:XX\n' '11:22:33' > "$d/docs-device.md"
commit_in "$d" 'redact to the OUI' docs-device.md
assert_blocked "$d" 'device address added then redacted is blocked' 'a full device address appears'

# 3. Capture artifact added, then deleted before the tip.
d="$(make_repo)"
echo 'GET /session' > "$d/session.har"
commit_in "$d" 'stash a capture for debugging' session.har
git -C "$d" rm -q session.har
git -C "$d" commit -qm 'remove the capture'
assert_blocked "$d" 'capture artifact added then deleted is blocked' 'capture artifact staged'

# 4. A branch that never introduced anything sensitive must still pass. Without
#    this, a gate that simply always fails would look like a working fix.
d="$(make_repo)"
echo 'design notes' > "$d/notes.md"
commit_in "$d" 'add notes' notes.md
echo 'more notes' >> "$d/notes.md"
commit_in "$d" 'extend notes' notes.md
assert_passed "$d" 'clean branch still passes'

# 5. Existing behaviour: a secret still present at the tip stays blocked.
d="$(make_repo)"
printf 'API_%s = "%s"\n' 'KEY' 'abcdefghij0123456789' > "$d/service.conf"
commit_in "$d" 'add service config' service.conf
assert_blocked "$d" 'credential present at tip is still blocked' 'a credential value appears'

# 6. Merging origin/main into a branch must not re-report content that is
#    already on the remote. Per-commit scanning that diffed a merge against its
#    first parent would surface everything the merge brought in from main and
#    block the push over content this branch never introduced — the kind of
#    false positive that teaches people to reach for --no-verify.
d="$(make_repo)"
git -C "$d" branch -q main
git -C "$d" checkout -q main
printf 'API_%s = "%s"\n' 'KEY' 'grandfatheredsecret123' > "$d/legacy.conf"
commit_in "$d" 'legacy config already on the remote' legacy.conf
git -C "$d" update-ref refs/remotes/origin/main refs/heads/main
git -C "$d" checkout -q feature
echo 'feature work' > "$d/feature.md"
commit_in "$d" 'add feature notes' feature.md
git -C "$d" merge -q --no-edit main
assert_passed "$d" 'merging remote content does not re-report it'

# 7. Content introduced on a side branch and merged in is still caught: the
#    side commit is part of the pushed range even though the merge itself is
#    clean.
d="$(make_repo)"
echo 'feature work' > "$d/feature.md"
commit_in "$d" 'add feature notes' feature.md
git -C "$d" checkout -q -b side
printf 'sensor %s:%s\n' '11:22:33' '44:55:66' > "$d/side-device.md"
commit_in "$d" 'record the sensor on a side branch' side-device.md
git -C "$d" checkout -q feature
git -C "$d" merge -q --no-edit side
git -C "$d" rm -q side-device.md
git -C "$d" commit -qm 'remove the sensor note'
assert_blocked "$d" 'secret merged in from a side branch is caught' 'a full device address appears'

# 8. A guard that fails closed must say where. Under per-commit scanning the
#    offending content is usually absent from the working tree — commit 12 added
#    it and commit 40 took it out — so "appears in the diff" leaves the
#    developer with nothing to grep for and no commit to rewrite. The message
#    has to name the commit that introduced it.
d="$(make_repo)"
printf 'API_%s = "%s"\n' 'KEY' 'abcdefghij0123456789' > "$d/service.conf"
commit_in "$d" 'add service config' service.conf
offender="$(git -C "$d" rev-parse HEAD)"
printf 'API_%s = ""\n' 'KEY' > "$d/service.conf"
commit_in "$d" 'drop the inline value' service.conf
assert_blocked "$d" 'the blocked message names the offending commit' "${offender:0:12}"

# 9. One line carrying both a documented placeholder and a real address must
#    still fail on the real one. The extraction is per line for this reason: a
#    `grep -v` over the whole blob would discard the line for containing the
#    placeholder and let the real address through.
d="$(make_repo)"
printf 'example %s vs live %s:%s\n' 'AA:BB:CC:DD:EE:FF' '11:22:33' '44:55:66' > "$d/mixed.md"
commit_in "$d" 'document both addresses' mixed.md
assert_blocked "$d" 'a real address beside a placeholder still fails' 'a full device address appears'

printf '\n%s passed, %s failed\n' "$PASSED" "$FAILED"
(( FAILED == 0 )) || exit 1
printf 'pre-push gate regressions passed\n'
