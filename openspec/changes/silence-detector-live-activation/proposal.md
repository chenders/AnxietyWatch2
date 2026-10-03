# Change: complete silence-detector production activation

## Why

The corrected daemon starts with the SQLx URL, but direct Pi observation found that the live
`status_observations` table lacks the detector columns and `notification_events` is absent. The
GitHub workflow also invokes a root-owned deployment helper installed before the latest health
changes, so the new per-service receipt evidence was not active.

## What changes

- fail closed at daemon startup when the required detector schema is absent;
- provide an explicit, backup-gated operator script to apply the existing `silence_detector` Sqitch
  deploy SQL without touching `samples`;
- provide an explicit operator-only root-helper replacement procedure with hash and architecture
  checks;
- verify the helper, schema, persistence, restart count, and several 30-second cycles before
  enabling the unit.

## Non-goals

- no automatic schema migration from the GitHub activation workflow;
- no writes, updates, or deletes against `samples`;
- no automatic enabling of `silence-detector`;
- no change to notification policy or the emergency `scripts/sync-to-pi.sh` path.
