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

**They are not simply wrong, and the alternative was measured before being declined.** Amendment 19
built a compressed hypertable with the identity in a separate `sample_identity` table, loaded it with
1,043,564 real rows, and priced it: **44 GB → 13 GB, a 31 GB saving** — but the identity table costs
125 bytes/row (11.9 GB at scale, *more* than the 7.4 GB index it replaces), and the hypertable
accepts a replayed row **silently**, where today's `samples` rejects it unconditionally. On a disk
with 3.3 TB free, that trade was declined.

So `samples` remains a plain, uncompressed table **knowingly**: 23.9 GB of `extra` stays
uncompressed because the only mechanism that would compress it requires giving up unconditional
duplicate rejection on the table holding the data.

That verdict could change. If identity can be stored as `(source, type, session_id, seq_lo, seq_hi)`
ranges rather than one row per sample, the identity cost collapses and the option is worth
re-costing — see Amendment 19 §3.2.

These files are history. Any future replacement will be new changes with new names; nothing here
should be re-added to the plan.
