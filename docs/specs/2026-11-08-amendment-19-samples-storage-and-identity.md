# Amendment 19 — `samples` keeps its identity; the hypertable is costed and declined

**Status:** proposed, awaiting operator acceptance. Phase 1 below is already executed on operator
direction; nothing else is implemented.

**Supersedes:** Amendment 01 §3.1's mechanism — `create_hypertable('samples', …,
migrate_data => TRUE)` on the live table, whose primary key lacks the partition column — and the
§§3.2–3.4 ordering that depends on it. Those four sqitch changes have never been deployable and are
now out of the plan.

**Preserves, unchanged:** Amendment 04 §1 (packet arrival time is not beat time), Amendment 05's
ingest identity `(source, type, session_id, seq)` stable across retransmission, and Amendment 06's
BP identity and durability rules. Nothing here adds `timestamp` to any identity, and the identity
stays enforced by PostgreSQL **on the table that holds the data**.

**Affects:** T-M3a; `db/sqitch.plan`; the four changes now under `db/superseded/`;
`docs/specs/2026-08-03-amendment-01-timescale-copy-spool.md` §§3.1–3.4; the recorded M0b finding in
`openspec/changes/admin-console-m0/tasks.md`.

**Two earlier drafts of this amendment were wrong, and both are recorded here rather than quietly
replaced.** The first recommended a rollup and claimed ~25 GB of savings; the second recommended
making `samples` a hypertable with identity in a side table, and claimed that side table would be
cheaper than the index it replaced. Measurement refuted both. The pattern is the point: each draft
was plausible, and each was corrected by going and looking.

---

## 1. What is actually wrong

`create_hypertable('samples', by_range('timestamp', …))` requires the partition column to appear in
**every** unique constraint on the table. `samples`' primary key is `(source, type, session_id,
seq)` and deliberately excludes `timestamp`. The two cannot both hold.

### 1.1 How the contradiction was introduced

Not a mistake in Amendment 01. It was created by Amendment 05 and never propagated:

| | |
|---|---|
| **A01 §3.1** | `samples` becomes a hypertable partitioned on `timestamp`, compressed after 7 days, with a `samples_1min` continuous aggregate and 2-year retention. |
| **A04 §1** | `PRIMARY KEY (source, type, timestamp)` destroys RR intervals — "silent data loss dressed as idempotency". Its fix: *"The primary key stays as it is; the timestamp becomes correct."* So A04 left `timestamp` **in** the key. A01 and A04 were consistent. |
| **A05** | *"A04 §1 said `PRIMARY KEY (source, type, timestamp)` could stay. That was wrong"* — identity becomes `(source, type, session_id, seq)`, **stable across retransmission**. |

A05 is right, and its reasoning is the strongest in the set: a retransmitted payload anchored to a
later arrival gets new timestamps and inserts again as a duplicate, so a timestamp-bearing key is
unstable precisely when the network is unreliable. But the moment `timestamp` left the key, A01
§3.1's `create_hypertable` stopped being deployable and **nobody noticed**, because A05's
`Invalidates:` list named A04 and not A01.

### 1.2 The cost was never compression. It was the migration pipeline.

Measured on the running Pi, 2026-11-08: the sqitch plan held **30 changes and 3 were deployed**
(`extensions`, `registry_tables`, `samples`, all 2026-08-06).

sqitch is a **linear** plan and `samples_hypertable` was change 4, so changes **8–30 were
unreachable through the intended tool** — `attempts`, `status_observations`, `domain_records`,
`domain_schema`, `envelope_clients`, `silence_detector`, `mobile_credentials`, every `apns_*`,
`air11_admin`: 23 changes. That is why T-M0b had to verify its four changes "by direct script
application on an isolated scratch DB" rather than by deploying them.

**This was the urgent half, it is independent of every storage question, and it is done** — see
Phase 1 in §5.

### 1.3 The original change is also irreversible

Amendment 01's own revert note states the rule: *"`samples_hypertable` raises if `samples` has rows,
because a hypertable cannot be converted back in place and reverting would discard them."*

`samples` holds **99,020,648 rows / 44 GB**. Converting in place is therefore a one-way operation on
the entire measurement history, with no revert path. That is an independent reason to reject A01
§3.1's mechanism regardless of the constraint conflict.

---

## 2. Measured state

Everything here is from the running system, not from the design documents.

| | |
|---|---|
| `samples` | **99,020,648 rows, 44 GB** — heap 34 GB, `samples_pkey` 7.4 GB, `samples_time_idx` 2.7 GB |
| Is it a hypertable? | **No.** Plain table, `PRIMARY KEY (source, type, session_id, seq)` as declared. |
| Read path | `samples_time_idx (source, type, timestamp DESC)`, carrying every time-ranged query |
| `timestamp` span | **2026-09-01 → 2026-10-29**, ≈58 days, ≈20 rows/second sustained |
| TimescaleDB | **2.29.1, installed and working**, plus `timescaledb_toolkit` 1.24.0 and `pg_cron` 1.6 |
| Existing hypertables | 2 — `aircraft_pass` (8 chunks), `aircraft_track_point` (35 chunks, compressed 4.0×) |
| Disk | 3.6 TB, 125 GB used, **3.3 TB free, 4%** |

**There is no space emergency.** That single fact governs §4: 44 GB on a 3.3 TB disk is a question of
tidiness and query speed, not survival, so the decision can be made on correctness.

### 2.1 Where the 44 GB is

| | |
|---|---|
| `extra` present on | **100%** of rows |
| `extra` size | 28–429 bytes, **avg 241** — ≈ **23.9 GB of the 34 GB heap**, about 70% |
| `extra` cardinality | 54,707 distinct values in 55,242 sampled rows — effectively unique per row |
| `extra` content | JSON, e.g. `{"clock_quality":"untrusted","device_start_time":"2008-01-03T…"}`; a minority are not JSON at all, one beginning `test_origin` |
| Source mix | **`resmed_air11` is 99.2%** of all rows; `polar_h10` is 0.7% |
| `samples` reloptions | defaults, so `toast_tuple_target` is 2048 and a ~331-byte row **never reaches the TOAST threshold** — `extra` is stored inline, raw, uncompressed |

Compressibility of `extra`, measured on 4,000 real values:

```
per-value compression    0.99x   — no reduction at all
column-wise compression  37.8x
```

**That pair rules out the obvious cheap fix.** A 241-byte value is too short for a compressor to find
redundancy *within itself*, so TOAST — which compresses each value independently — achieves nothing;
lowering `toast_tuple_target` would spend a full table rewrite for 1%. The redundancy is **between**
rows: repeated JSON keys, a near-constant `device_start_time` within a session. Only columnar
compression, which compresses a column across many rows at once, reaches it.

So the one mechanism that recovers this space is columnar compression on `samples` itself — which is
exactly what the identity constraint forbids. §3 is the attempt to get both.

### 2.2 The device clock is untrusted; the stored timestamps are not garbage

| | |
|---|---|
| `clock_quality: untrusted` | **98.8%** of sampled rows |
| `device_start_time` in 2008 | **98.8%** — the same rows; 2008 is the AirSense's power-on epoch |
| `test_origin` marker | 0.9% |
| Distinct sessions | 2,173 in the sample |

An earlier draft read this as "the time axis is untrusted for 98.8% of the history", and that was an
overstatement worth correcting. `samples.timestamp` spans **2026-09-01 to 2026-10-29 at a steady ≈20
rows/second** — it is real host-derived wall-clock, not the device's 2008 clock. The flag records
that the *device's* own clock could not corroborate it, which is provenance, and is precisely the
situation A04 §1 described and A05 made non-load-bearing by removing `timestamp` from the key.

The practical consequences are narrower than that draft claimed: queries over `timestamp` are
meaningful today, and the 0.9% carrying `test_origin` is a far better case for targeted deletion than
anything about the clock.

---

## 3. Options

**Rejected on identity.** *Option A*, adding `timestamp` to the primary key, reintroduces exactly
what A05 forbade and what T-M3a's brief forbids: a retransmitted payload with a re-estimated
timestamp inserts again. *Option E*, native declarative partitioning, has the identical requirement
that the partition key appear in every unique constraint. *Option C*, partitioning on an
identity-derived column, has no candidate: the useful partition key is time and the identity
deliberately excludes time.

**Rejected on measurement.** *Option D* kept `samples` plain and moved compression, an aggregate and
retention onto a rollup. It was the first draft's recommendation. §2.1 shows why it fails its own
purpose: a rollup aggregates numeric values and carries no `extra` column, so it recovers **none** of
the 23.9 GB. It remains worth building later for read speed and retention — judged on those merits,
not on space.

### 3.1 Option H — compressed hypertable plus a separate identity table

The constraint forbids a unique constraint on the hypertable. It says nothing about one on a
*different* table:

```
samples           hypertable on timestamp, compressed, NO unique constraint
sample_identity   PRIMARY KEY (source, type, session_id, seq) — and nothing else
```

The drainer writes both in one transaction; a replayed batch violates that key and rolls back.

**This was built and measured rather than predicted.** A scratch database was loaded with
**1,043,564 real rows** — six hours of 2026-10-23, the densest day. `vitals` was read only, and the
scratch database was dropped afterwards.

| | as deployed today | Option H |
|---|---|---|
| data | heap 341 MB + `pkey` 73 MB | hypertable **351 MB → 13 MB, 26.4×** |
| identity | (inside the primary key) | `sample_identity` **125 MB** |
| **total** | **428 MB** | **138 MB** |

Extrapolating by the 94.9× row ratio puts today's shape at 40.6 GB against 44 GB measured — close
enough to trust. So at full scale Option H is roughly **44 GB → 13 GB: a 31 GB saving, 3.4×.**

**Three things the measurement corrected.**

1. The second draft said the 34 GB heap would land "near 3–4 GB". The data half does compress, 26.4×
   measured — but the identity table takes 11.9 GB back, so the end state is ≈13 GB.
2. It said `sample_identity` would be "≈40 bytes/row ≈ 4 GB, cheaper than the 7.4 GB `samples_pkey`
   it replaces". Measured at **125.2 bytes/row** — a 52 MB heap plus a 73 MB primary-key index per
   1.04M rows, so **11.9 GB at scale: 60% more expensive than the index it replaces, not cheaper.**
   The estimate ignored the tuple header and the fact that a four-column btree duplicates most of the
   row.
3. **The guarantee is not equivalent, and the spike demonstrated it rather than arguing it.**
   Inserting a replayed row into `sample_identity` raises `duplicate key value violates unique
   constraint`, as designed. Inserting the same row into the **hypertable alone succeeds silently** —
   the table went to 1,043,565 rows. Today's `samples` rejects it unconditionally, because the
   constraint sits on the table holding the data.

Point 3 is the real cost, and it is not about space. Today no drainer bug can produce a duplicate
sample: PostgreSQL refuses it. Under Option H the data table has no protection of its own and the
whole guarantee rests on the drainer writing the identity row in the same transaction — a safety
property moved from *impossible by construction* to *correct if the code is correct*, on an
append-only medical store whose silent-duplication failure mode is the subject of A04 §1.

### 3.2 The judgement

**31 GB, on a disk with 3.3 TB free at 4% use, in exchange for making identity enforcement depend on
application transaction discipline — plus a 99M-row copy migration and a drainer rewrite.**

**Declined for now.** The spike did its job: it converted a plausible design into three corrected
numbers and one demonstrated weakening, and at the real price the answer is no. Space is not scarce;
unconditional identity enforcement on a medical record is not something to trade for tidiness.

**What would change the answer.** `seq` is monotonic within a session, and on the EDF path it is
derived from time — so identity is dense and sequential, and a session collapses into a handful of
**ranges** rather than one row per sample:

```
sample_identity_range (source, type, session_id, seq_lo, seq_hi)
```

If that holds for the live sources, 11.9 GB becomes megabytes and Option H lands near **44 GB → 1–2
GB**, which is a materially different trade. It needs an exclusion constraint rather than a primary
key, and careful handling of out-of-order arrival splitting a range. That is design work and belongs
in its own task, not in an assertion here.

---

## 4. Decision

1. **Identity is final, and stays on the data table.** `(source, type, session_id, seq)`, enforced by
   PostgreSQL on `samples` itself. No amendment, migration or convenience may add `timestamp` to it
   or move its enforcement off the table holding the rows without a new amendment that prices the
   loss explicitly.
2. **`samples` is never converted in place.** A01 §3.1's `migrate_data => TRUE` is superseded
   outright: it cannot deploy, and A01's own revert note makes it irreversible if it did.
3. **The plan is unblocked — done.** Changes 4–7 are out, `attempts` re-parents onto `samples`, and
   26 changes remain with every dependency resolving. 23 changes stop being unreachable. See §5.
4. **`samples` stays a plain table for now**, which is Option B in effect. Option H is costed in §3.1
   and declined in §3.2 — not rejected on principle, declined on price.
5. **Compression remains unavailable, knowingly.** 23.9 GB of `extra` stays uncompressed because the
   only mechanism that would compress it requires giving up unconditional identity enforcement. That
   is a deliberate trade recorded here, not an oversight to be rediscovered.
6. **Retention is deferred to T-M3b**, constrained: it may not rely on chunk drops on `samples`, and
   it must weigh that raw samples are the only artefact that could be re-read if the device-clock
   provenance is ever reconciled.
7. **Follow-up task: range-based identity.** If `sample_identity_range` proves out, Option H should be
   re-costed, because at 1–2 GB it is a different proposition.

This resolves the recorded M0b finding: `samples_hypertable` **resolved** by supersession rather than
by deployment, and the Timescale/identity decision is recorded here.

---

## 5. Migration, rollback and verification

### 5.1 Phase 1 — unblock the plan · **executed 2026-11-08**

`samples_hypertable`, `samples_compression`, `samples_1min` and `retention` are removed from
`db/sqitch.plan`; `attempts` is re-parented onto `samples`. Their twelve deploy/revert/verify scripts
moved to `db/superseded/` with a README, rather than being deleted, so the superseded design stays
readable beside this amendment.

Verified: **26 changes, every dependency resolving to an earlier change, no duplicates** — checked
explicitly, since sqitch is not installed on the development host. No row in `samples` is read or
written.

**Not yet deployed to the Pi.** Those 23 changes have never run against production, so deployment
wants a window, a backup, and `sqitch verify` after each change rather than a single bulk deploy.

**Rollback:** `sqitch revert` to change 3 restores the registry; the plan edit is a git revert. No
data is involved either way.

### 5.2 If Option H is ever accepted

Recorded so a future implementer does not have to rediscover the shape, and explicitly **not**
authorised by this amendment.

A copy and a rename, never an in-place conversion: build `sample_identity` and a constraint-free
hypertable alongside; backfill in time-bounded chunks **with the spool drain stopped**, because A04
§7 requires a migration touching `samples` to guard on an empty spool and binary COPY freezes column
count and order; verify; switch the drainer to the two-table write; rename last. The original table
is kept until a second clean night of ingest has landed.

Verification would have to include, at minimum: equal row counts across all three tables; an ordered
digest over `(source, type, session_id, seq, timestamp, value)` agreeing between old and new; a
re-run of one backfill chunk **raising** a unique violation and rolling back, proving the duplicate is
rejected rather than merely absent; the compression prediction in §3.1 confirmed or the amendment
revisited; and a chart query whose plan shows chunk exclusion rather than a full scan.

---

## 6. What this does not decide

- **Raw retention** — T-M3b, under the constraints in §4.6.
- **Range-based identity** — the §3.2 follow-up, which is the only thing that would make Option H
  worth its price.
- **The 0.9% of rows carrying `test_origin`** — a candidate for targeted deletion, and a better one
  than anything in §2.2.
- **`pg_cron`'s `cron.database_name = vitals` binding**, M0b's second finding: the plan cannot replay
  onto a sibling scratch database. Physical restores are unaffected; dev tooling is.
