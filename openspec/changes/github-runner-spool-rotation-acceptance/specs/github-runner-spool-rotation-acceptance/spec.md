# Specification: Stable spool identity for activation

## ADDED Requirements

### Requirement: Restart-stable pending spool identity

The deployment helper SHALL fingerprint pending spool contents independently of time-derived segment
filenames while retaining file count, total bytes, per-file size, and per-file content. The helper
MUST continue to reject a backup gate whose pre-activation spool identity differs from the current
pending spool.

#### Scenario: Controlled writer restart creates an equivalent empty segment

- **GIVEN** the backup gate matches the pending spool before activation
- **WHEN** activation restarts the spool writer and the equivalent segment receives a new filename
- **THEN** the post-activation spool identity remains unchanged when segment count, total bytes,
and segment contents are unchanged
- **AND** activation may continue to its other health gates

#### Scenario: Pending spool content changes during activation

- **GIVEN** the backup gate matches the pending spool before activation
- **WHEN** a segment's contents, total bytes, or file count changes during activation
- **THEN** post-activation health fails and the existing rollback path runs
- **AND** the receipt records `spool_unchanged: false`

#### Scenario: Stale backup gate remains blocked

- **GIVEN** the current pending spool differs from the fingerprint recorded in the backup gate
- **WHEN** activation begins
- **THEN** preflight fails closed with the differing values named
- **AND** no release or service mutation occurs
