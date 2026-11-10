# Proposal: Finish the 2026-11-09 repository tidy-up

The `task/repo-tidy` branch removed what could be removed safely in one reviewed PR: root
screenshots, a repo-snapshot archive, session prompts, a PDF duplicate, five unowned
experiments, and the owner's health data from fixtures, research notes, an evidence ledger,
and an iOS placeholder. It added `scripts/check-tracked-paths.py` so the same class of file
cannot return unnoticed.

This change records what that branch deliberately did **not** do, because each item either
needs a decision only the owner can make, changes what runs on the Pi, or is a separate defect
found along the way. None of it is urgent except T-RH-1, and T-RH-1 is urgent only if the
repository's audience is about to widen.

## What was found

**1. The owner's health data is still in git history.** Deleting a file at the tip does not
remove it from history. Every commit on `origin/main` before the tidy-up still holds: 33 root
screenshots (including Health and HealthKit screens) and a screen recording; a 5.7 MB
whole-repo archive re-bundling them; four real blood-pressure frames and two oximeter frames
in test fixtures; clinical specifics in a research note; the owner's iPhone name and server
user id. The repository is private, so the exposure is everyone with clone access — but it is
permanent. AGENTS.md forbids rewriting `main` without the owner. GitHub also retains
`refs/pull/*` and cached views of every merged PR, which a history rewrite does not touch.

**2. Small drifts found by the inventory.** The log follower's hardcoded unit list misses
four collectors; `docs/HERDR-GUIDE.md` still documents the C/CMake build and the CI guard
meant to catch that misses `cmake --build`; two test files exist that nothing runs; the
restore-verification files exist in two diverged copies; the console's bind address is a
real LAN address committed in a tracked unit; the Apple Team ID is committed; the local
personal-term denylist the pre-push PHI gate reads was absent on the development machine;
and one Swift test failed intermittently during verification.

## What does not belong here

Anything that makes stored data *wrong* (a misattributed type, a bound that reads healthy on
silence) is raised immediately, not filed — see `prototype-hardening-backlog`'s dividing line.
Nothing below corrupts the record.
