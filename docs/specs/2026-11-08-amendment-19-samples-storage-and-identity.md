# Amendment 19 — Identity moves to its own table so `samples` can be a compressed hypertable

**Status:** proposed, awaiting operator acceptance. No implementation before acceptance (T-M3a).

**Supersedes:** Amendment 01 §3.1's **mechanism** — `create_hypertable('samples', …,
migrate_data => TRUE)` on the live table, whose primary key lacks the partition column — and the
§§3.2–3.4 ordering that depends on it.

**Amendment 01's goals are not superseded; they are delivered.** `samples` does become a compressed
hypertable with an aggregate and a retention policy. What changes is that the ingest identity moves
to its own table so the hypertable can exist at all, and that the conversion happens as a verified
copy rather than in place. An earlier draft of this amendment proposed abandoning the hypertable and
re-siting its benefits onto a rollup; measurement in §2.1 showed that would have recovered none of
the space, so that draft is withdrawn.

**Preserves, unchanged:** Amendment 04 §1 (packet arrival time is not beat time), Amendment 05's
ingest identity `(source, type, session_id, seq)` stable across retransmission, and Amendment 06's
BP identity and durability rules. Nothing here adds `timestamp` to any identity.

**Affects:** T-M3a (this decision), then T-M3c, T-M3d, T-M3g, T-M3i and T-X5 which are blocked on
it; `db/sqitch.plan` and the four changes `samples_hypertable`, `samples_compression`,
`samples_1min`, `retention`; `docs/specs/2026-08-03-amendment-01-timescale-copy-spool.md` §§3.1–3.4.

---

## 1. What is actually wrong

`create_hypertable('samples', by_range('timestamp', …))` requires the partition column to appear in
**every** unique constraint on the table. `samples`' primary key is `(source, type, session_id,
seq)` and deliberately excludes `timestamp`. The two cannot both hold. One of them has to go, and
this amendment says which.

### 1.1 How the contradiction was introduced

It was not a mistake in Amendment 01. It was created by Amendment 05 and never propagated:

| | |
|---|---|
| **A01 §3.1** | `samples` becomes a hypertable partitioned on `timestamp`, compressed after 7 days, with a `samples_1min` continuous aggregate and a 2-year retention policy. |
| **A04 §1** | `PRIMARY KEY (source, type, timestamp)` destroys RR intervals — "silent data loss dressed as idempotency". Its fix: *"The primary key stays as it is; the timestamp becomes correct."* So A04 left `timestamp` **in** the key. A01 and A04 were consistent. |
| **A05** | *"A04 §1 said `PRIMARY KEY (source, type, timestamp)` could stay. That was wrong"* — identity becomes `(source, type, session_id, seq)`, **stable across retransmission**. |

A05 is right, and the reasoning is the strongest in the set: a retransmitted payload anchored to a
later arrival gets new timestamps and inserts again as a duplicate, so a timestamp-bearing key is
unstable precisely when the network is unreliable. But the moment `timestamp` left the key, A01
§3.1's `create_hypertable` call stopped being deployable, and **nobody noticed**, because A05's
`Invalidates:` list named A04 and not A01.

This amendment is that missed propagation, performed deliberately.

### 1.2 The cost is not compression. It is the whole migration pipeline.

Measured on the running Pi, 2026-11-08:

```
sqitch plan      30 changes
deployed          3  (extensions, registry_tables, samples — all 2026-08-06)
```

sqitch is a **linear** plan. `samples_hypertable` is change 4 and cannot deploy, so changes **8–30
are unreachable through the intended tool** — `attempts`, `status_observations`, `domain_records`,
`domain_schema`, `envelope_clients`, `silence_detector`, `mobile_credentials`, every `apns_*`,
`air11_admin`: 23 changes. That is why T-M0b had to verify its four changes "by direct script
application on an isolated scratch DB" rather than by deploying them.

**This is the urgent half, and it is independent of the storage decision.** Whatever is chosen for
compression, the plan must stop being blocked at change 4.

### 1.3 The change is now also irreversible if it ever did deploy

Amendment 01 §(revert) already states the rule: *"`samples_hypertable` raises if `samples` has rows,
because a hypertable cannot be converted back in place and reverting would discard them."*

`samples` currently holds **99,020,648 rows / 44 GB**. So even setting the constraint conflict aside,
converting now would be a long, locking, one-way operation on the system's entire measurement
history. A revert path does not exist. That is a second, independent reason the original plan cannot
be executed as written.

---

## 2. Measured state

All figures from the running system on 2026-11-08, not from the design documents.

| | |
|---|---|
| `samples` | 99,020,648 rows, 44 GB total — heap 34 GB, `samples_pkey` 7.4 GB, `samples_time_idx` 2.7 GB |
| Is it a hypertable? | **No.** Plain table. `PRIMARY KEY (source, type, session_id, seq)` exactly as `db/deploy/samples.sql` declares. |
| Read path | `samples_time_idx (source, type, timestamp DESC)` — present and carrying every time-ranged query |
| TimescaleDB | **2.29.1 installed and working**, plus `timescaledb_toolkit` 1.24.0 and `pg_cron` 1.6 |
| Existing hypertables | 2 — `aircraft_pass` (8 chunks) and `aircraft_track_point` (35 chunks, **compressed**) |
| Compression, measured on this box | `aircraft_track_point`: 244 MB → 61 MB, **4.0×**, 35 of 35 chunks |
| Disk | 3.6 TB, 125 GB used, **3.3 TB free, 4%** |
| `spool_status` | 479,966 rows / 103 MB |

**There is no space emergency.** 3.3 TB free at 4% use. This can be decided on correctness and
operational simplicity rather than under pressure. 44 GB is not a problem today; 44 GB growing with
no retention mechanism is a problem eventually.

### 2.1 Where the 44 GB actually is, and what can compress it

Measured by sampling the table, because the answer determines which options are worth their
machinery:

| | |
|---|---|
| `extra` present on | **100%** of rows (53,644 of 53,644 sampled) |
| `extra` size | 28–429 bytes, **avg 241** — so ≈ **23.9 GB of the 34 GB heap**, about 70% |
| `extra` cardinality | 54,707 distinct values in 55,242 rows — effectively unique per row |
| `extra` content | JSON, e.g. `{"clock_quality":"untrusted","device_start_time":"2008-01-01T…"` — and a minority are not JSON at all, one beginning `test_origin` |
| Source mix | **source 5 (`resmed_air11`) is 99.2%** of all rows; `polar_h10` is 0.7% |
| `samples` reloptions | defaults — so `toast_tuple_target` is 2048 and a ~331-byte row **never reaches the TOAST threshold**. `extra` is stored inline, raw, uncompressed. |

Compressibility of `extra`, measured on 4,000 real values on the Pi:

```
per-value compression    0.99x   — no reduction at all
column-wise compression  37.8x
```

**That pair of numbers decides the storage question, and it rules out the obvious cheap fix.** A
241-byte value is too short for a compressor to find redundancy *within itself*, so TOAST — which
compresses each value independently — achieves nothing. Lowering `toast_tuple_target` to force TOAST
on these rows would spend a full table rewrite for a 1% gain.

The redundancy is **between** rows: the JSON keys repeat on every row and `device_start_time` is
near-constant within a session. Columnar compression, which compresses one column across many rows
at once, therefore gets 37.8× on this column alone. Applied to `samples`, ≈23.9 GB of `extra` would
become ≈0.6 GB, and the numeric columns compress well too — the whole 34 GB heap plausibly lands
near 3–4 GB.

So: **the only mechanism that recovers the space is columnar compression on `samples` itself, which
is exactly what the identity constraint forbids.** That is the real shape of this trade-off, and §3
is rewritten around it.

### 2.2 A finding that outranks the storage question

| | |
|---|---|
| `clock_quality: untrusted` | **98.8%** of sampled rows (52,274 of 52,884) |
| `device_start_time` in 2008 | **98.8%** — the same rows |
| `test_origin` marker | 0.9% (472 rows) |
| Distinct sessions | 2,173 in the sample |

Nearly every row in the table was recorded by a device whose clock was never set; 2008 is the
AirSense's power-on epoch. The measurements are real, but **their time axis is not**, and
`samples.timestamp` is the column every read path and every chart uses.

This does not change the identity decision — identity deliberately excludes `timestamp`, which is
precisely why it survives this. It does three other things:

1. It raises the value of Amendment 19's clock-transform requirement from housekeeping to the
   precondition for using this data at all. A per-session offset derived from a trusted reference is
   what makes 98.8% of the history interpretable.
2. It lowers the urgency of recovering 23 GB, because space spent on data whose timestamps are
   untrusted is not the first problem to solve.
3. It is a finding for T-M3b and for whatever decides retention: ageing out raw samples is a
   different proposition when the raw samples are the only artefact that could later be
   re-interpreted under a corrected clock.

---

## 3. Options considered

### Option A — add `timestamp` to the primary key

`PRIMARY KEY (source, type, session_id, seq, timestamp)`.

**Rejected.** It reintroduces exactly what A05 forbade. A retransmitted payload whose timestamp is
re-estimated inserts again as a new row, so the key stops being stable across retransmission — the
one property it was changed to obtain. T-M3a's own brief forbids it ("without adding timestamp to
ingest identity"), and it would grow the already-7.4 GB PK index by another 8 bytes per row.

### Option B — drop the hypertable requirement entirely

Keep `samples` a plain table; delete changes 4–7 from the plan.

**Viable, and the fallback.** Identity is preserved, the 23 blocked changes unblock immediately, and
nothing touches the 44 GB table. What is lost: compression (~25 GB), chunk pruning, the continuous
aggregate, and — most importantly — a cheap retention mechanism. Without partitioning, ageing out
old samples is a `DELETE` over tens of millions of rows that bloats the heap rather than dropping a
chunk. A01's retention story becomes unavailable rather than merely re-sited.

### Option C — partition on an identity-derived column

Partition on something that *is* a function of the identity, so including it in the key adds no new
identity.

**Rejected as unworkable.** The useful partition key for this workload is time, and the identity
deliberately excludes time. `session_id` is in the key but does not order by time, so chunks would
not align with any query. There is no column that is both identity-derived and time-ordered.

### Option D — `samples` stays plain; the rollup becomes the hypertable

Keep `samples` exactly as deployed. Add `samples_1min` as a **standalone hypertable** — not a
continuous aggregate, because a CAGG requires its source to be a hypertable — populated
incrementally from `samples` by a `pg_cron` job, compressed, with its own retention policy.

- **Identity is untouched.** No migration of 99M rows, no lock, no one-way conversion, no risk to
  the measurement history.
- **The 23 blocked sqitch changes unblock**, because changes 4–7 are replaced by changes that can
  actually deploy.
- **Retention and read speed land on the rollup**, which is what dashboard and chart reads actually
  want. A01 §3.2 already observed that continuous aggregates "survive after the raw data is dropped,
  which is the retention story" — Option D keeps that property; the aggregate simply is not a CAGG.

**It recovers essentially no space, and an earlier draft of this amendment claimed otherwise.** That
draft applied the 4.0× aircraft figure to the 34 GB heap and said roughly 25 GB was recoverable, in a
context implying Option D delivered it. It does not: the rollup aggregates numeric values and carries
no `extra` column at all, so the 23.9 GB described in §2.1 is untouched. Option D should be judged on
read speed and retention, which are real, and not on space, which it does not provide.

The other cost is one more moving part: an incremental refresh job whose watermark and idempotency
have to be right.

### Option H — `samples` becomes a compressed hypertable; identity moves to its own table · **recommended**

The constraint is that a hypertable may not carry a unique constraint lacking its partition column.
It says nothing about a unique constraint on a *different* table.

```
samples           hypertable on timestamp, compressed, NO unique constraint
sample_identity   PRIMARY KEY (source, type, session_id, seq) — and nothing else
```

The drainer inserts into `sample_identity` first, in the same transaction as the `samples` rows it
accounts for. A replayed batch violates that primary key, the transaction rolls back, and no row is
written to either table. The guarantee stays **hard and database-enforced on the exact 4-tuple
Amendment 05 specifies** — it is not moved into application logic, and there is no `ON CONFLICT DO
NOTHING` anywhere, so neither a silent duplicate nor a silent drop is reachable.

- **Identity is unchanged**, enforced by PostgreSQL, on the same four columns.
- **The 23.9 GB becomes ≈0.6 GB**, because `samples` can now be compressed columnar — the one
  mechanism §2.1 shows actually works on this data.
- **`sample_identity` is cheaper than the index it replaces**: four fixed-width columns at ≈40
  bytes/row is ≈4 GB over 99M rows, against the 7.4 GB `samples_pkey` that exists today.
- The rollup from Option D remains available on top, and becomes a true continuous aggregate once
  `samples` is a hypertable — so Option H is a superset, not an alternative.

Costs, stated plainly:

- **A copy migration of 99M rows.** Not `migrate_data => TRUE` on the live table: a new hypertable,
  a chunked copy, a verification pass, then a rename. This is the only option here that moves data.
- **A drainer change.** The COPY-binary path must populate both tables in one transaction. That is
  the part that needs care, because a partial write that lands rows without their identity row would
  defeat the whole mechanism.
- **Two inserts per row on ingest**, against a 1 Hz-per-signal workload. Negligible in principle;
  worth measuring against the spool drain rate before committing.

Option H is recommended **after** Phase 1, and only once the clock finding in §2.2 is understood —
there is no value in a fast, compressed, well-identified copy of 99M rows whose timestamps are still
untrusted.

### Option E — native declarative partitioning

**Rejected.** PostgreSQL's own partitioning has the same requirement: the partition key must be in
every unique constraint. Identical conflict, fewer features.

---

## 4. Decision

1. **Identity is final and wins.** `(source, type, session_id, seq)` stands, enforced by PostgreSQL
   on exactly those four columns. No amendment, migration or convenience may add `timestamp` to it.
   A05's reasoning is accepted as load-bearing: a key that moves under retransmission is not an
   identity.
2. **`samples` is never converted in place.** Amendment 01 §3.1's `create_hypertable(…,
   migrate_data => TRUE)` on the live table is superseded outright. It cannot deploy, and A01's own
   revert note makes it irreversible if it did — on 99M rows that is not a migration, it is a
   one-way bet.
3. **The sqitch plan is unblocked first, separately, and immediately.** Changes 4–7 leave the plan;
   8–30 are re-parented onto `samples`. This is Phase 1: mechanical, carries no storage decision,
   touches no row, and is fully reversible. 23 changes stop being unreachable. **Do this regardless
   of what is chosen below.**
4. **The storage shape is Option H**, adopted as the target rather than executed now: `samples`
   becomes a compressed hypertable and identity moves to its own `sample_identity` table. It is the
   only option that recovers the 23.9 GB, and it does so without weakening the identity guarantee
   or moving it into application code.
5. **Option H is gated on the clock finding, not on effort.** §2.2 shows 98.8% of rows carry an
   untrusted device clock and a 2008 `device_start_time`. Copying 99M rows into a better-shaped
   table before their timestamps mean anything would be well-executed work on data that cannot yet
   be read. The clock transform comes first.
6. **Option D's rollup is deferred and re-justified.** It becomes a true continuous aggregate once
   Option H lands, so building it beforehand as a hand-rolled job is work that would be thrown away.
   It was also mis-sold in an earlier draft as a space saving, which it is not.
7. **Raw retention is deferred to T-M3b**, now with a second constraint: raw samples are the only
   artefact that could be re-interpreted under a corrected clock, so retention must not discard them
   while 98.8% of the history is still untrusted.

This resolves the recorded M0b finding: `samples_hypertable` **resolved** by supersession rather
than by deployment, and the Timescale/identity decision is recorded here.

---

## 5. Migration, rollback and verification

### 5.1 Migration

**Phase 1 — unblock the plan (no data movement).**

Remove `samples_hypertable`, `samples_compression`, `samples_1min` and `retention` from
`db/sqitch.plan` and re-parent `attempts` onto `samples`. Their deploy/revert/verify scripts move to
`db/superseded/` rather than being deleted, so the superseded design stays readable beside this
amendment. `sqitch deploy` then advances from change 3 to change 26 on a database that already holds
`samples`.

Nothing is written to `samples`. The risk is confined to sqitch's own registry.

**Phase 2 — the clock transform.** Not specified here, but it precedes Phase 3 by §2.2: copying 99M
rows into a better-shaped table while 98.8% of their timestamps are untrusted is work done on data
that cannot yet be read.

**Phase 3 — Option H, as a copy and a rename, never an in-place conversion.**

1. Create `sample_identity (source, type, session_id, seq)` with that primary key, and
   `samples_next` as a hypertable on `timestamp` with **no** unique constraint, compression
   configured `segmentby = 'source, type'`, `orderby = 'timestamp DESC'`.
2. Backfill both in chunks, bounded by time, with the spool drain **stopped** — A04 §7's rule that a
   migration touching `samples` must guard on an empty spool applies here with force, because
   binary COPY freezes column count and order and a mid-migration drain would write to the old
   shape.
3. Verify (below) while both tables exist and the old one is still authoritative.
4. Switch the drainer to the two-table write, then rename. The old table is kept, not dropped, until
   a second night of ingest has landed cleanly.

The copy is restartable: it is keyed by time range, and a range already present in
`sample_identity` is skipped, so an interrupted backfill resumes rather than duplicating.

### 5.2 Rollback

**Phase 1 is fully reversible.** It touches only the sqitch plan and registry; `sqitch revert` to
change 3 restores the previous state, and no row in `samples` is read or written. Rolling back the
*plan edit* is a git revert.

**Phase 3 is reversible because the original table is never modified.** That is the whole reason it
is a copy: `samples` is untouched throughout, so rollback is "point the drainer back and drop
`samples_next`". Contrast A01 §3.1's `migrate_data => TRUE`, whose revert A01 itself had to make
*refuse* in order to avoid destroying data — that is a migration with no way back, and this
amendment rejects it on exactly that ground.

The irreversible moment is the rename, and it is deliberately the last step, after verification and
after a second clean night of ingest.

### 5.3 Verification

Phase 1:

- `sqitch status` reports change 26 deployed and no undeployed changes before it
- `sqitch verify` passes for every change from `samples` forward
- `SELECT count(*) FROM samples` is unchanged across the deploy, asserted before and after
- `SELECT pg_get_constraintdef` on `samples_pkey` still reads
  `PRIMARY KEY (source, type, session_id, seq)`
- the four superseded scripts are absent from the plan and present under `db/superseded/`

Phase 3, all of it while `samples` is still authoritative:

- `samples_next` appears in `timescaledb_information.hypertables`; `samples` is unchanged and still
  reports `PRIMARY KEY (source, type, session_id, seq)`
- row counts match exactly: `count(*)` over `samples`, `samples_next` and `sample_identity` are
  equal, and `count(DISTINCT (source, type, session_id, seq))` equals that count — so the copy
  neither dropped nor duplicated a row
- a full-table checksum agrees: the same ordered digest over `(source, type, session_id, seq,
  timestamp, value)` computed on both tables
- **the identity guarantee is demonstrated, not assumed**: re-running one chunk of the backfill
  raises a unique violation on `sample_identity` and the transaction rolls back, leaving both tables
  byte-identical. A test that proves the duplicate is *rejected* rather than merely absent.
- `hypertable_compression_stats('samples_next')` reports the compressed size; the §2.1 prediction
  (≈23.9 GB of `extra` → under 1 GB) is confirmed or the amendment is wrong and should be revisited
- the spool is empty before the backfill and `spool_status` is unchanged by it
- a representative chart query against `samples_next` returns the same rows as against `samples`, and
  its plan shows chunk exclusion rather than a full scan

---

## 6. What this does not decide

- **Raw retention** — T-M3b, constrained as above.
- **Whether `samples` is ever partitioned.** If raw volume later justifies it, the route is a new
  table and a copy, with identity preserved and `timestamp` still outside the key. Not this
  amendment.
- **`pg_cron`'s `cron.database_name = vitals` binding**, which is the second M0b finding: the plan
  cannot replay onto a sibling scratch database. Physical restores are unaffected; dev tooling is.
  Separate defect, separate fix.
- **`scripts/verify-amendment-index.sh` currently fails** on `main`, independently of this
  amendment: its row pattern matches `**NN**` only, so the A11-series files indexed as `**10-A11**`
  and `**11-A11**` are treated as unindexed. T-M3a's Verify invokes that script, so T-M3a cannot pass
  until it is fixed. This is the same two-series numbering collision recorded in
  `openspec/changes/amendment-reference-integrity`. It needs its own change; teaching the verifier
  the suffixed convention is preferable to renaming files that twenty citations point at.
