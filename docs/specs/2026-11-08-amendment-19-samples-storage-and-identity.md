# Amendment 19 — `samples` keeps its identity; Timescale moves to a rollup

**Status:** proposed, awaiting operator acceptance. No implementation before acceptance (T-M3a).

**Supersedes:** Amendment 01 §§3.1–3.4 — `samples` becoming a hypertable, its columnar compression
policy, the `samples_1min` continuous aggregate built on it, and the `add_retention_policy` on it.
Amendment 01's *goals* are retained and re-sited; only the mechanism changes.

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

Two of these change the shape of the decision:

- **Compression is proven on this hardware**, not hypothetical. A01 claimed "90–95% reduction";
  the same-box measurement is 4.0× (75%) on a different table. Applying 4× to the 34 GB heap
  suggests roughly **25 GB recoverable** — real, and a conservative floor since `samples` segmented
  by `(source, type)` and ordered by `timestamp` is more compressible than aircraft tracks.
- **There is no space emergency.** 3.3 TB free at 4% use. This can be decided on correctness and
  operational simplicity rather than under pressure. 44 GB is not a problem today; 44 GB growing
  with no retention mechanism is a problem eventually.

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

### Option D — `samples` stays plain; the rollup becomes the hypertable · **recommended**

Keep `samples` exactly as deployed. Add `samples_1min` as a **standalone hypertable** — not a
continuous aggregate, because a CAGG requires its source to be a hypertable — populated
incrementally from `samples` by a `pg_cron` job, compressed, with its own retention policy.

- **Identity is untouched.** No migration of 99M rows, no lock, no one-way conversion, no risk to
  the measurement history.
- **The 23 blocked sqitch changes unblock**, because changes 4–7 are replaced by changes that can
  actually deploy.
- **Compression and retention land where the data volume is**, on the rollup, and the rollup is what
  dashboard and chart reads actually want. A01 §3.2 already observed that continuous aggregates
  "survive after the raw data is dropped, which is the retention story" — Option D keeps that
  property; the aggregate simply is not a CAGG.
- **Raw retention stays an open question**, honestly. Option D does not make `DELETE` on the raw
  table cheap. It does make dropping raw data *safe*, because the rollup retains the shape of the
  history. Whether raw samples should age out at all is T-M3b's decision, not this one.

The cost is one more moving part: an incremental refresh job whose watermark and idempotency have to
be right. That is a real cost and it is why this is a recommendation rather than an assertion.

### Option E — native declarative partitioning

**Rejected.** PostgreSQL's own partitioning has the same requirement: the partition key must be in
every unique constraint. Identical conflict, fewer features.

---

## 4. Decision

1. **Identity is final and wins.** `(source, type, session_id, seq)` stands. No amendment, migration
   or convenience may add `timestamp` to it. A05's reasoning is accepted as load-bearing: a key that
   moves under retransmission is not an identity.
2. **`samples` is not a hypertable, and never becomes one in place.** Amendment 01 §3.1 is
   superseded. Any future conversion requires a new table and a copy, not `migrate_data => TRUE` on
   99M live rows.
3. **Timescale's benefits are re-sited to a rollup**, per Option D. `samples_1min` becomes a
   standalone compressed hypertable refreshed from `samples`.
4. **The sqitch plan is unblocked first and separately.** Changes 4–7 are removed from the plan;
   8–30 are re-parented onto `samples`. This is mechanical, carries no storage decision, and should
   land before the rollup is built.
5. **Raw retention is deferred to T-M3b** with the constraint that it may not rely on chunk drops on
   `samples`.

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

**Phase 2 — the rollup (separate change, after Phase 1 is deployed).**

A new sqitch change creates `samples_1min` as a hypertable on its own `bucket` column, with
`compress_segmentby = 'source, type'`, `compress_orderby = 'bucket DESC'`, a compression policy, and
a `pg_cron` refresh that advances a watermark and upserts complete minute buckets only. Incomplete
trailing buckets are never written, so a refresh interrupted mid-minute cannot publish a partial
aggregate as a complete one.

### 5.2 Rollback

**Phase 1 is fully reversible.** It touches only the sqitch plan and registry; `sqitch revert` to
change 3 restores the previous state, and no row in `samples` is read or written. Rolling back the
*plan edit* is a git revert.

**Phase 2 is reversible by construction**, which is the main reason to prefer a rollup over an
in-place conversion: `samples_1min` is derived data. Dropping it loses nothing that cannot be
recomputed from `samples`, so its revert is `DROP TABLE` plus removing the cron entry — unlike
`samples_hypertable`, whose revert A01 itself had to make *refuse* in order to avoid destroying data.

### 5.3 Verification

Phase 1:

- `sqitch status` reports change 26 deployed and no undeployed changes before it
- `sqitch verify` passes for every change from `samples` forward
- `SELECT count(*) FROM samples` is unchanged across the deploy, asserted before and after
- `SELECT pg_get_constraintdef` on `samples_pkey` still reads
  `PRIMARY KEY (source, type, session_id, seq)`
- the four superseded scripts are absent from the plan and present under `db/superseded/`

Phase 2:

- `samples_1min` appears in `timescaledb_information.hypertables`
- for a fixed historical hour, the rollup's `avg`/`min`/`max` per `(source, type)` match a direct
  aggregate over `samples` within a float tolerance
- a second refresh over the same window inserts zero rows — the refresh is idempotent
- `hypertable_compression_stats('samples_1min')` reports compressed chunks after the policy interval
- killing the refresh mid-run leaves the watermark unadvanced and publishes no partial bucket

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
