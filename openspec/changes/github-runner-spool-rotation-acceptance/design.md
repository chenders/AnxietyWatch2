# Design: Stable spool identity for activation

`vitals-deploy` currently fingerprints each pending `*.spool` file using its relative filename,
size, and contents. `vitalsd` creates a time-derived empty segment on every writer restart, so a
controlled activation can change only the filename while preserving the header bytes and pending
rows. The first real activation demonstrated this: preflight passed against a fresh gate, but the
post-restart health check rolled back with `spool_unchanged: false`.

The fingerprint will instead hash the multiset of each segment's size and content digest, sorted
before the aggregate digest. Segment names are excluded; file count and total bytes remain explicit
`SpoolState` fields. Therefore a renamed equivalent segment remains the same identity, while changed
bytes, a changed count, or changed total size remains a blocker.

The test uses a temporary spool fixture to prove filename changes are accepted and content changes
are rejected. The live recovery gate must still be recreated after the helper is installed, because
the fingerprint algorithm is part of the gate contract.
