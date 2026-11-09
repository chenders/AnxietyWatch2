# Tasks: Repository hygiene follow-ups

Independent unless a task says otherwise; no required order. Items marked **owner call** carry
a decision only the owner can make. Each Verify exits 0 only when the task is done.

- [ ] T-RH-1 — Purge the owner's health data from history · ops · host · **owner call**
- T-RH-2 — withdrawn (2026-11-09; owner decision, not tracked here)
- [ ] T-RH-3 — Follow every deployed unit in the log follower · implement-simple · host
- [ ] T-RH-4 — Remove the C/CMake build from HERDR-GUIDE and close the guard gap · implement-simple · host
- [ ] T-RH-5 — Run the two tests nothing runs · implement-simple · host
- [ ] T-RH-6 — Collapse the two restore-verification copies into one · implement · pi
- [ ] T-RH-7 — Move the console bind address out of the tracked unit · implement · pi
- [ ] T-RH-8 — Move the Apple Team ID out of the tracked xcconfig · implement-simple · host · **owner call**
- [ ] T-RH-9 — Install the personal-term denylist on each development machine · ops · host
- [ ] T-RH-10 — Find the intermittent AWTransport test failure · debug · host

---

### T-RH-1 — Purge the owner's health data from history

**Depends:** none · **Review required:** yes · **Owner call**

**Amends:** none.

**Reads:** `AGENTS.md` (Git: history-rewrite rule).

**Do:** Every commit on `origin/main` before the 2026-11-09 tidy-up still holds the material
that branch removed from the tip: 33 root screenshots (Health and HealthKit screens among
them), a screen recording, `aw2-ios-thin-client-chatgpt-context.zip` (which re-bundles them),
real blood-pressure and oximeter frames in decoder fixtures, clinical specifics in
`docs/research/klaxon-alarm-timing-reassessment.md`, and the owner's iPhone name and server user
id in the APNs evidence ledger. The repository is private; the exposure is everyone with clone
access, permanently.

**Owner call:** whether to rewrite `main`. AGENTS.md forbids it without the owner, and it is
not reversible for anyone who has already cloned. If yes: `git filter-repo` with a path list
for the deleted files and `--replace-text` for the fixture values (build the replacement list
from the pre-tidy blobs; do not commit it), force-push every branch, re-clone every worktree
and the Pi's deploy checkout, and ask GitHub Support to purge `refs/pull/*` and cached views —
a rewrite does not reach those. If no: record that decision here and close the task.

**Verify:** run against a fresh clone after the rewrite (exits 0 only when the paths and the
fixture provenance note are gone from every reachable commit):
```sh
test -z "$(git rev-list --all -- health.jpg healthkit.jpg h10_vitals.mp4 aw2-ios-thin-client-chatgpt-context.zip)" \
  && test -z "$(git log --all --format=%h -G 'readings are real, not syntheti[c]')"
```

**Context budget:** 1 file; the work is a history operation, not an edit.

---

### T-RH-3 — Follow every deployed unit in the log follower

**Depends:** none · **Review required:** no

**Amends:** none.

**Reads:** `scripts/vitals-log-follow.sh`, `deploy/README.md` (unit table).

**Do:** The follower's hardcoded `-u` list has eight units and omits `vitals-air11`,
`vitals-ble-broker` (both instances), `silence-detector` and `vitals-emay`, so an operator
following logs during an incident does not see four collectors. Derive the list from the
enabled units in `deploy/README.md`'s table, or from `deploy/*.service`, rather than a second
hand-maintained copy.

**Verify:**
```sh
for u in vitals-air11 vitals-ble-broker silence-detector vitals-emay; do grep -q -- "$u" scripts/vitals-log-follow.sh || { echo "missing $u"; exit 1; }; done
```

**Context budget:** 2 files.

---

### T-RH-4 — Remove the C/CMake build from HERDR-GUIDE and close the guard gap

**Depends:** none · **Review required:** no

**Amends:** none.

**Reads:** `docs/HERDR-GUIDE.md`, `.github/workflows/ci.yml` (the superseded-language guard,
`C_PATTERN`).

**Do:** `docs/HERDR-GUIDE.md` still tells a worker to build and fuzz the C tree
(`cmake --build build-fuzz && build-fuzz/vitals/fuzz_sfloat`) and lists CLion's CMake
toolchain. The CI guard that exists to stop exactly this matches `cmake -B` but not
`cmake --build`, so it passes on the document it polices. Replace the text with the cargo
equivalents, add a `cmake --build` clause to `C_PATTERN`, and add its `probe_clause` line so
the self-test proves the clause matches.

**Verify:**
```sh
! grep -nE 'cmake|build-fuzz|fuzz_sfloat' docs/HERDR-GUIDE.md && grep -qF "probe_clause 'cmake --build'" .github/workflows/ci.yml
```

**Context budget:** 2 files.

---

### T-RH-5 — Run the two tests nothing runs

**Depends:** none · **Review required:** no

**Amends:** none.

**Reads:** `pyproject.toml` (pytest `testpaths`), `.github/workflows/ci.yml`,
`tools/tests/test_gap_histogram.py`, `tools/tests/test_bedside_vitals_panel.mjs`.

**Do:** pytest's `testpaths` covers only `vitals/tools/tests`, and no CI step names either
file, so `tools/gap_histogram.py` (named in a T-B1 Verify line) and the bedside panel's
contract check are untested in CI. Add both to a CI job; fix whatever they reveal.

**Verify:**
```sh
grep -q 'test_gap_histogram' .github/workflows/ci.yml && grep -q 'test_bedside_vitals_panel' .github/workflows/ci.yml
```

**Context budget:** 4 files.

---

### T-RH-6 — Collapse the two restore-verification copies into one

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `deploy/README.md`, `deploy/restore-verification/README.md`, both copies of each
unit, `scripts/pi/verify-restore.sh`, `deploy/restore-verification/verify-restore.sh`.

**Do:** `deploy/` holds the units captured from the Pi on 2026-11-09;
`deploy/restore-verification/` holds the T-C3 install artifacts. Four of five units are
identical, but `verify-restore.service` differs (Group, RuntimeDirectory, ReadWritePaths) and
the two `verify-restore.sh` copies differ in `work_parent`. Openspec `Reads:` lines in
`pi-github-actions-deployment` and `pi-prototype-storage-degraded-mode` point at the copies
that do *not* match what is deployed. Establish on the Pi which is correct, keep one copy,
and repoint those `Reads:` lines (never a `Verify:` line).

**Verify:**
```sh
for f in verify-restore.service verify-restore.timer verify-restore-alert.service verify-restore-staleness.service verify-restore-staleness.timer; do
  test ! -e "deploy/restore-verification/$f" || cmp -s "deploy/$f" "deploy/restore-verification/$f" || { echo "diverged: $f"; exit 1; }
done
```

**Context budget:** 8 files.

---

### T-RH-7 — Move the console bind address out of the tracked unit

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `deploy/vitals-console.service`, `scripts/check-air11-public.py`,
`scripts/install-air11-pi-artifact.sh`.

**Do:** `VITALS_CONSOLE_BIND` and `VITALS_CONSOLE_ADMIN_HOST` are set to the Pi's real LAN
address in the tracked unit, and `check-air11-public.py` requires that exact line. Move both
into the unit's existing `EnvironmentFile=` and make the checker assert the variable comes
from there. The file is currently optional (`-` prefix): decide whether a missing file should
stop the unit or fall back to loopback — never to an all-interfaces bind. Verify on the Pi
that the console still binds where it did.

**Verify:**
```sh
! git grep -nP '192\.168\.\d+\.\d+:\d+' -- deploy scripts/check-air11-public.py && uv run python3 scripts/check-air11-public.py
```

**Context budget:** 3 files.

---

### T-RH-8 — Move the Apple Team ID out of the tracked xcconfig

**Depends:** none · **Review required:** no · **Owner call**

**Amends:** none.

**Reads:** `clients/apple/Config/Shared.xcconfig`, `.github/workflows/ios-ci.yml`.

**Do:** `DEVELOPMENT_TEAM` is the owner's real Team ID. It ships inside any signed build, so
this is low severity. Move it to an `#include?`'d, gitignored local xcconfig, with a
`.example`. CI builds with signing disabled and is unaffected.

**Owner call:** whether it is worth doing. The bundle prefix also identifies the developer
account but is load-bearing (Keychain service, background-task identifiers) — leave it.

**Verify:**
```sh
! git grep -nE 'DEVELOPMENT_TEAM *= *[A-Z0-9]{10}' -- clients/apple
```

**Context budget:** 2 files.

---

### T-RH-9 — Install the personal-term denylist on each development machine

**Depends:** none · **Review required:** no

**Amends:** none.

**Reads:** `.githooks/pii-denylist.local.txt.example`.

**Do:** The pre-push PHI gate reads `.githooks/pii-denylist.local.txt` and, when it is absent,
prints "personal-term checking is off" and continues. It was absent on the development machine
used for the 2026-11-09 tidy-up. Create it from the example on every machine that pushes
(names, address, clinicians, device names, account identifiers — the example lists what
belongs). It is gitignored by design; never commit it.

**Verify:** on each pushing machine:
```sh
test -s .githooks/pii-denylist.local.txt && git check-ignore -q .githooks/pii-denylist.local.txt
```

**Context budget:** 1 file.

---

### T-RH-10 — Find the intermittent AWTransport test failure

**Depends:** none · **Review required:** no

**Amends:** none.

**Reads:** `clients/apple/Packages/AWTransport/Tests/AWTransportTests/`.

**Do:** During the tidy-up, one run of `scripts/verify-ios-apns.sh` (which runs the
AWTransport Swift tests) failed in `AWTransportTests` and passed on rerun with identical
inputs. Which test failed was not captured. Reproduce under repetition, identify the test,
and fix the race or shared state; a test that passes on retry still fails CI at random.

**Verify:**
```sh
for i in $(seq 20); do swift test --package-path clients/apple/Packages/AWTransport >/dev/null 2>&1 || { echo "failed on run $i"; exit 1; }; done
```

**Context budget:** 1 directory.
