# Design

## Startup schema contract

After connecting, `silence-detector` probes the exact columns and table used by its load/save
queries. A missing object is a configuration failure with exit status 70; the process must not stay
`active` while every persistence operation fails.

## Operator-controlled schema activation

`scripts/apply-silence-detector-schema.sh` requires root, an explicit confirmation variable, and the
root-owned off-Pi backup gate. It refuses partial schemas, runs the checked-in transactional
`db/deploy/silence_detector.sql` as the `postgres` user, verifies every resulting object, and writes
a root-owned receipt. It never invokes Sqitch to replay unrelated pending changes and never touches
`samples`.

## Root helper refresh

`scripts/install-deploy-helper.sh` requires explicit confirmation, a regular non-symlink ARM64 ELF,
and an expected SHA-256. It atomically backs up and replaces `/usr/local/bin/vitals-deploy`, stores a
root-only receipt, and leaves the GitHub sudoers boundary unchanged. The workflow remains unable to
install arbitrary root code.

## Acceptance

After schema and helper receipts pass, start the disabled service manually. Observe at least three
30-second cycles, verify status and notification rows persist, verify `NRestarts` is unchanged and
all structured service fields are successful, then enable the unit only as the final operator step.
A failure stops the service and leaves it disabled.
