# Runbook — macOS storage, Time Machine, and external disks

Operator knowledge about the machine this project is developed on. Every item below
cost real time to learn and several were learned by shipping the bug first. Paths use
`$HOME` and placeholders; substitute your own.

## Time Machine

### 1. `tmutil latestbackup` returns a synthetic path, never the mount point

```
$HOME → tmutil latestbackup
/Volumes/.timemachine/<DESTINATION-UUID>/2026-11-06-200440.backup/2026-11-06-200440.backup
```

Not `/Volumes/<volume name>/…`. Any script that prefix-tests the result against the
user-visible mount point rejects **every real backup**.

This was shipped as a "hardening" improvement and ran for eight hours, rejecting a
successful backup 235 consecutive times before hitting its own deadline. The machine
it was supposed to power off stayed on all day. Cost: two scripts, both discarded.

### 2. Assert the outcome, not the absence of activity

A *failed* Time Machine backup **deletes its own `.inprogress` bundle**. So this test —

> `Running = 0` AND no `.inprogress` AND some backup exists

— is satisfied *exactly* as well by failure as by success. A real log window of 695
seconds existed between a `BACKUP_FAILED_DISCONNECTED_DESTINATION` and the next
attempt, during which a 240-second version of that test would have passed.

The authoritative signal is in `/Library/Preferences/com.apple.TimeMachine.plist`:

```sh
sudo defaults read /Library/Preferences/com.apple.TimeMachine Destinations \
  | awk '/RESULT/{gsub(/[^0-9]/,"",$3); print $3; exit}'
```

`0` = success, `11` = copy-stage failure, `27` = disconnected destination. Also require
the backup's own timestamp to be **newer than when your watcher armed**, or any
pre-existing backup satisfies the test.

### 3. A destination's Time Machine ID is not its APFS volume UUID

`tmutil destinationinfo` reports one identifier; `diskutil info` reports a different one
for the same disk. Resolve by whichever the API you are calling indexes on.

**Never identify a disk by BSD node.** This machine mounts simulator runtimes and
installer images at nodes up to `disk17`, and the external disk's own node moved
`disk5`→`disk6`→`disk7` across reconnects. A script that hardcodes `disk5` and runs
`diskutil unmountDisk` can unmount a completely different volume. Resolve from a UUID:

```sh
diskutil info "<VOLUME-UUID>" | awk -F': *' '/Part of Whole/{print $2}'
```

### 4. Verify the *effect* of an exclusion, not the return code

`tmutil addexclusion` exiting 0 does not mean the **running** backup session honours it.
Sessions snapshot their exclusion set at start. Read it back from inside the in-progress
bundle:

```sh
sudo plutil -p "/Volumes/<tm volume>/<date>.inprogress/.exclusions.plist" \
  | awk '/^  "/{s=$1;gsub(/"/,"",s);next} /=> "/{c[s]++} END{for(k in c) print k, c[k]}'
```

Sections are `userExclusionPaths` (yours), `stickyExclusionPaths` (app-declared, found
via Spotlight), `standardExclusionPaths` (macOS defaults). To have exclusions apply from
byte zero, set them, **then** start the backup — editing them mid-session wastes the pass.

### 5. `-p` decides whether an exclusion survives the file

- `tmutil addexclusion -p <path>` → recorded in `SkipPaths` in the TM preference plist.
  Tied to the path; survives the file being deleted and recreated. Needs `sudo`.
- `tmutil addexclusion <path>` → an extended attribute on the item itself. Travels with
  the file if moved; **dies with the inode** if the file is recreated.

For a stable policy, use `-p`.

### 6. Time Machine is exclude-only, hierarchical, with no override

You cannot exclude a parent and re-include a child. Include-only semantics must be
emulated by inverting — excluding everything except an allow-list. Two honest caveats:

- An inversion **with a size threshold is still allow-by-default below it**. Be explicit
  about which you built.
- Maintaining the deny-list by hand drifts. Generate it from a declarative allow-list and
  gate it with a check that exits nonzero when a new unlisted entry appears.

A maintained third-party deny-list manager exists (`qveys/tm-exclusions`) — check for
prior art before writing one.

### 7. Quotas and encryption are creation-time only

```
$ diskutil apfs setQuota
diskutil: did not recognize APFS verb "setQuota"
```

`-quota` and `-reserve` exist **only** as `diskutil apfs addVolume` flags. There is no
setter in `diskutil` or `apfs.util`. The only way to add a quota later is
`deleteVolume` + recreate — i.e. destroying the history you wanted to bound.

Consequence for a Time Machine volume sharing an APFS container with a data volume:
`backupd` thins against *container* free space, so it will grow until the sibling volume
can no longer be written. Decide the quota when you create the volume, because "later"
costs the whole backup history, and that cost rises every day.

`encryptVolume`, by contrast, **does** work in place.

## Shell guards

### 8. `for e in $(ls -A dir)` is a fail-open defect in any guard

Word-splitting turns one space-named entry into several tokens. Each token then fails
`du` and reports 0 KB — so a size-threshold guard classifies it as *small* and lets it
through. On this machine that silently exempted 15 entries totalling ~3 GB from a backup
exclusion policy, including a 2.1 GB VM directory.

```sh
while IFS= read -r e; do ... done < <(ls -A "$dir")     # correct
find "$dir" -maxdepth 1 -print0 | while IFS= read -r -d '' p; do ... done   # better
```

The failure mode matters more than the bug: it fails **open**, and reports success.

### 9. Guards go stale between check and use

Re-resolve and re-assert immediately before the destructive step, not once at the top.
For a disk erase that means: re-resolve the target by identity, re-confirm it is External
and is not the boot disk, and re-run the content verification — all inside the final
guard. Identifiers change on replug; another process can touch staging in between.

Other guards worth copying:

- A path outside the allowlisted roots is a **reported bug**, not a silent skip.
- Delete simulator devices via `xcrun simctl delete`, so CoreSimulator's index stays
  consistent instead of being healed later.
- Refuse to touch Docker's `Data/vms` when the daemon isn't running — you cannot
  enumerate named volumes, and they are not re-creatable the way images are.
- Verify an archive **before** destroying what it archives, with a count assertion
  (tarball entries must equal the untracked-file count), and HOLD on mismatch.

### 10. `df` before/after is ground truth; summed `du` is an upper bound

APFS clones share physical blocks, and `du` counts each clone's full logical size. Deleting
343 GB of `du`-measured paths freed **175 GB** of real space — Xcode creates simulator
device sets as clones of the runtime images. Reconcile the two and report the delta; a
disagreement is the useful signal.

Related: macOS `du` reports **512-byte blocks** by default, not KB. Use `du -k`.

## Diagnosing external-disk disconnects

Classify by kernel string before forming a hypothesis — they mean different things:

| Log signature | Meaning |
|---|---|
| `simulated or deferred connection lost` | sleep/wake power cycle |
| `hardware connection lost` | the port electrically saw the device leave |
| re-enumeration `at 480 Mbps` | SuperSpeed pairs failed to train — cable or contact |
| `status 0xe00002ed (transaction error)` | UAS/protocol error, often on bulk-OUT under write load |

Two further traps:

- **`PreventUserIdleSystemSleep` does not prevent clamshell sleep.** `backupd` holds that
  assertion during a backup and the machine still slept on lid close. The sleep reason
  names `'Clamshell Sleep'`. `caffeinate -dimsu` does not cover it either
  (`AppleClamshellCausesSleep = Yes` with the assertion held). Only
  `sudo pmset -a disablesleep 1` suppresses it.
- **Diagnose across the full time span, not the window you are watching.** A disconnect
  adjacent to one sleep event produced a confident "it's sleep" diagnosis; checking all
  ten events showed nine happened while the machine was provably awake. The real cause
  was a partially-seated connector, and the evidence that settled it was eleven
  consecutive clean hours — not any amount of log analysis during the bad window.

## Measuring a backup's speed

Time Machine's throughput is **bimodal** and any single short sample is meaningless. The
same first backup measured 72 MB/s in a small-file phase and 954 MB/s sustained in a
large-file phase. Sample both, or state the phase alongside the number.

`tmutil status`'s `totalBytes` and `totalFiles` are **whole-source-volume totals with no
exclusions applied** — they match `df -i` exactly. Its `Percent` and `TimeRemaining` are
therefore wrong by whatever your exclusions remove. Track bytes landed on the destination
instead.
