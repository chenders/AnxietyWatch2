# Superseded sqitch changes

These four changes were removed from `db/sqitch.plan` by
[Amendment 19](../../docs/specs/2026-11-08-amendment-19-samples-storage-and-identity.md).
They are kept rather than deleted because the superseded design should stay readable beside the
amendment that explains why it could not deploy.

- `samples_hypertable` — `create_hypertable('samples', by_range('timestamp', …), migrate_data => TRUE)`
- `samples_compression` — columnar compression after 7 days
- `samples_1min` — a continuous aggregate over the hypertable
- `retention` — raw samples dropped at 2 years

**Why they could never deploy.** `create_hypertable` requires its partition column in every unique
constraint on the table. `samples`' primary key is `(source, type, session_id, seq)` and
deliberately excludes `timestamp`, per Amendment 05. Production `sqitch status` therefore listed all
four as Undeployed — and because the plan is linear, the 23 changes after them were unreachable too.

**They are not simply wrong.** Amendment 19 adopts their *goals*: `samples` does become a compressed
hypertable with an aggregate and retention. What changes is that the ingest identity moves to its own
`sample_identity` table so the hypertable can exist, and that the conversion happens as a verified
copy rather than the in-place `migrate_data => TRUE` here — which Amendment 01's own revert note had
to make *refuse* on a non-empty table, i.e. it had no way back.

When Amendment 19 Phase 3 is implemented, the replacements will be new changes with new names. These
files are history and must not be re-added to the plan.
