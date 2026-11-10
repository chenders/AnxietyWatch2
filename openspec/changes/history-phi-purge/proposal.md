# Proposal: Remove the owner's PII and PHI from every commit, not just the tip

PR #105 cleaned the current tree. It did not clean history, and history is what a clone
delivers: anyone who clones the repository receives every blob ever reachable from a pushed
ref, deleted files included. This change finishes the job.

## What is in history

Measured on 2026-11-09 over every blob reachable from `refs/remotes/origin/*` (2,338 blobs),
with `scripts/check-public-info.py`'s rules plus a read-only semantic audit:

- **Health data:** 33 screenshots (Health and HealthKit screens among them) and a screen
  recording; a 5.7 MB repo-snapshot ZIP that re-bundles them; the owner's real blood-pressure
  and oximeter readings in decoder fixtures and docs; clinical specifics in a research note;
  a real reading plotted in a README figure; hard-coded medications in the iOS client.
- **Identifiers:** 36 full Bluetooth MAC addresses of the owner's devices, the Pi's private
  hostname (57 hits), Linux and macOS home paths with the owner's account name (57), the
  account name in deployment configuration (20), personalised device names and serial
  suffixes (16), the owner's iPhone name and server user id, the Pi's LAN address.
- **Withheld code:** `firmware/`, pending the owner's review, is in history from PR #22.
- **Location risk:** deleted ADS-B / flight-tracking panel tooling under `deploy/stopgap/`
  whose configuration may place the Pi, and so the owner's home.

The exact list — paths to remove, and text to replace — is the purge manifest. It is kept
**outside the repository** (`$HOME/Source/aw2-history-purge/`), because writing the values
here would put them back into history. This document describes them; it never quotes them.

## Why not leave it

The repository is private, so today's exposure is everyone with clone access. But it is
permanent: any future collaborator, any fork, any backup of a clone, receives all of it, and
nothing in the tree reveals that it is there.

## Approach

Rewrite once, deterministically, with `git filter-repo` over a single mirror holding every
ref — local branches with unpushed work, remote-tracking branches, tags — so rewritten local
work lines up with rewritten GitHub history and nothing is lost. Prove the result before
pushing: no scanner finding in any blob, no removed path in any commit, and `main`'s final
tree byte-identical to before. Then force-push, with the owner lifting the `main` ruleset's
force-push block for that one push only.

Two things a rewrite cannot reach are handled separately: GitHub's `refs/pull/*` and cached
views (a GitHub Support purge request), and existing clones (re-cloned, including the Pi's).

## Recurrence

Merge commits keep every branch commit forever, so a PR that adds a file and deletes it two
commits later still lands the file in `main`'s history — exactly how most of the above got
there. CI gates check the tip. The last task makes the path and content gates check every
commit a PR introduces.
