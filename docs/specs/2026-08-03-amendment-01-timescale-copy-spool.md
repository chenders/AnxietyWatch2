# Amendment 01 — Drop SQLite; COPY-binary spool + TimescaleDB

**Date:** 2026-08-03
**Status:** Accepted
**Amends:** `2026-08-03-pi5-vitals-core-design.md` §2.1, §2.5, §4.1
**Supersedes:** the SQLite-as-ingest-buffer decision, and the hand-rolled `samples_1min` rollup
**Reworks tasks:** T-C1, T-D3, T-D5 (none written yet — this is cheap now and expensive after T-D3)

---

## 1. What was wrong

The original design had `vitalsd` write SQLite as the hot ingest path and tail its own WAL into
Postgres. SQLite was chosen because `AnxietyWatchKit/Storage/` on the phone already uses it, so the
two stores would share a schema.

That reasoning does not survive contact with the actual requirement. **SQLite is not
Postgres-compatible**: different SQL dialect, different type system (`STRICT` only approximates real
types), different DDL, different NULL and collation semantics. Every row therefore crosses a
translation layer on the way to the system of record — and that translation is code, on the
durability path, that can be wrong in ways no test on either side would catch.

The schema-identity argument was also weaker than it looked. The phone does not share a *file* with
the Pi; it replicates over HTTP, and the HLC merge happens server-side. Identity of the on-disk
engine was never load-bearing.

## 2. The buffer: an append-only COPY-binary spool

Collectors append rows **already encoded in PostgreSQL's binary `COPY` wire format**. The drainer
streams those bytes verbatim into `COPY … FROM STDIN (FORMAT binary)`.

```
collector ──▶ libvitals ──▶ vitalsd ──▶ spool.copybin (append + fsync)
                                              │
                                              ▼  vitals-spool-drain
                                    COPY … FROM STDIN (FORMAT binary)
                                              │
                                              ▼
                                        PostgreSQL
```

**Why this rather than any embedded database:**

- **No dialect mismatch.** The buffer format *is* Postgres's format. There is nothing to translate,
  so there is no translation to get wrong.
- **No second query engine, schema, or type system** to keep in sync with the first.
- **Fastest ingest path Postgres has.**
- **The buffer never needs to be queried.** `cns-engine` holds its own ~60 s ring buffers
  (design §4.1); nothing reads the spool except the drainer. This is the property that makes a
  database unnecessary here, and it was true in the original design too — SQLite's query engine was
  never used on this path.

**Rejected alternative — `libpglite`.** Real Postgres as a native library with C bindings
(ElectricSQL, v0.4, March 2026). Genuine dialect compliance, and the right answer if the buffer ever
needed to be queryable. The bindings are marked work-in-progress, and WIP is not acceptable on the
path between a desaturation and an alarm. Revisit if that changes.

### 2.1 Spool format

```
spool/                       (on NVMe, StateDirectory=vitals)
  cur.copybin                active segment, opened O_APPEND
  cur.offset                 last durably-drained byte offset, fsynced
  seg-<epoch>.copybin        rotated segments awaiting drain
```

- A segment opens with the 19-byte Postgres binary COPY header (`PGCOPY\n\377\r\n\0` + flags +
  header extension length) and is terminated by the 16-bit trailer `0xFFFF` at drain time, not at
  write time — an unterminated segment is the normal state of a live spool.
- Each row is `int16 field_count` followed by per-field `int32 length` + raw bytes, in the column
  order fixed by `vitals/registry.toml`. Length `-1` is SQL NULL.
- `fsync` after each batch, or every 250 ms, whichever comes first. A collector must never block on
  disk to deliver a sample; batching bounds loss at 250 ms of a single source.
- Rotate at 64 MB or hourly.

### 2.2 Durability and idempotency

`COPY` has no `ON CONFLICT`, so the drainer is two steps in one transaction:

```sql
BEGIN;
CREATE TEMP TABLE staging (LIKE samples INCLUDING DEFAULTS) ON COMMIT DROP;
COPY staging FROM STDIN (FORMAT binary);
INSERT INTO samples SELECT * FROM staging ON CONFLICT (source, type, timestamp) DO NOTHING;
COMMIT;
```

The byte offset advances **only after that transaction commits**, and it is fsynced before the
segment is unlinked. This is the same discipline as the sync-cursor rule in the parent design: the
cursor advance and the write that carried the rows commit together, or neither does. A crash
mid-drain replays from the last committed offset and the `ON CONFLICT` makes the replay a no-op.

### 2.3 Constraints this format imposes

Binary `COPY` is **not portable across architectures or major Postgres versions**. Therefore:

- The spool is a **transient queue, never an archive**. Cold export (`docs/STAGE-DELIVERABLES.md`
  tier 3) stays newline-delimited JSON.
- **Drain to empty before any major Postgres upgrade.** The upgrade runbook must assert
  `spool/` is empty and the offset file is at EOF, and refuse to proceed otherwise.
- A schema change to `samples` requires draining first, because field count and order are baked into
  every buffered row. `vitals-spool-drain --verify` checks the spool header against the current
  table shape and refuses to drain a mismatch rather than corrupting data.

## 3. PostgreSQL extensions

Confirmed available for arm64 / Debian bookworm / PG16.

| Extension | Use | Replaces |
|---|---|---|
| **timescaledb** | `samples` becomes a hypertable | manual partitioning |
| **timescaledb_toolkit** | LTTB downsampling for charts | hand-written decimation |
| **pg_stat_statements** | query observability | — |
| **pg_cron** | retention + aggregate refresh scheduled in-database | two systemd timers |

Not adopted: PostGIS, pgvector, pgaudit — no use here.

> **§§3.1–3.4 are superseded by [Amendment 19](2026-11-08-amendment-19-samples-storage-and-identity.md).**
> `samples` does not become a hypertable, and the compression policy, the `samples_1min` continuous
> aggregate and the retention policy move off it onto a standalone rollup.
>
> Not because this section was wrong when written — it was consistent with Amendment 04 §1, which
> kept `timestamp` in the primary key. **Amendment 05 removed it**, making the identity stable across
> retransmission, and `create_hypertable` requires its partition column in every unique constraint.
> A05's `Invalidates:` list named A04 and not A01, so the conflict went unrecorded and the last four
> sqitch changes have been undeployable ever since — which also blocked the 23 changes behind them in
> the linear plan. The goals below are retained and re-sited; only the mechanism changes.

### 3.1 `samples` becomes a hypertable

```sql
CREATE EXTENSION IF NOT EXISTS timescaledb;

CREATE TABLE samples (
  source        INTEGER          NOT NULL REFERENCES vitals_source(id),
  type          INTEGER          NOT NULL REFERENCES vitals_type(id),
  timestamp     TIMESTAMPTZ      NOT NULL,
  value         DOUBLE PRECISION NOT NULL,
  extra         BYTEA,
  hlc_physical  BIGINT           NOT NULL,
  hlc_logical   INTEGER          NOT NULL,
  node_id       BYTEA            NOT NULL,
  PRIMARY KEY (source, type, timestamp)
);

SELECT create_hypertable('samples', by_range('timestamp', INTERVAL '1 day'));

ALTER TABLE samples SET (
  timescaledb.compress,
  timescaledb.compress_segmentby = 'source, type',
  timescaledb.compress_orderby   = 'timestamp DESC'
);
SELECT add_compression_policy('samples', INTERVAL '7 days');
```

Columnar compression gives **90–95% reduction on cold chunks**. Years of RR intervals stop being a
storage question, and the 1 TB archive tier becomes generous rather than necessary.

**Note the type change:** `timestamp` becomes `TIMESTAMPTZ`, not the `REAL` epoch-seconds of the
parent design, because hypertable partitioning and every hyperfunction expect a real timestamp type.
This does **not** weaken the UTC-only rule — `TIMESTAMPTZ` stores an absolute instant with no
timezone attached; the session `TimeZone` is fixed to `UTC` in `postgresql.conf` so rendering is
unambiguous. The C side still holds `double` epoch seconds and converts once, in the COPY encoder.

### 3.2 Continuous aggregates replace `samples_1min`

The parent design specified a hand-maintained `samples_1min` rollup table. Delete it.

```sql
CREATE MATERIALIZED VIEW samples_1min
WITH (timescaledb.continuous) AS
SELECT source, type,
       time_bucket(INTERVAL '1 minute', timestamp) AS bucket,
       avg(value) AS value, count(*) AS sample_count,
       min(value) AS min_value, max(value) AS max_value
FROM samples
GROUP BY source, type, bucket;

SELECT add_continuous_aggregate_policy('samples_1min',
  start_offset => INTERVAL '3 hours', end_offset => INTERVAL '1 minute',
  schedule_interval => INTERVAL '1 minute');
```

Continuous aggregates **survive after the raw data is dropped**, which is the retention story the
parent design left to a future decision. Raw samples can age out while the minute rollups persist
indefinitely.

`min_value`/`max_value` are carried because an average hides a desaturation nadir, and the nadir is
the clinically interesting number.

### 3.3 Gap-fill enforces "unknown ≠ OK" in SQL

The design's hardest invariant is that missing data must never render as a healthy flat line. The
parent design enforced it twice by hand — once in the sparkline drawing code, once in the CNS
scorer — in two languages, by convention.

`time_bucket_gapfill` enforces it once, at the source of every query:

```sql
SELECT time_bucket_gapfill(INTERVAL '1 minute', timestamp) AS bucket,
       avg(value) AS value            -- NULL in buckets with no data
FROM samples
WHERE source = $1 AND type = $2 AND timestamp >= $3 AND timestamp < $4
GROUP BY bucket ORDER BY bucket;
```

Buckets with no data come back as explicit `NULL` rows rather than being silently absent. **A
consumer that forgets to handle gaps now gets a NULL it must deal with, instead of a shorter array
that interpolates cleanly and lies.** Every read path for chart or scoring data uses gapfill; a query
that omits it is a review finding.

**`locf()` and `interpolate()` are banned on physiological series.** They exist to fill gaps
plausibly, which is precisely the failure this system cannot tolerate. Permitted only for genuinely
step-valued config series, and never for SpO₂, HR, RR, or blood pressure.

### 3.4 Retention and scheduling via pg_cron

```sql
CREATE EXTENSION IF NOT EXISTS pg_cron;
-- Raw samples age out at 2 years; the 1-minute continuous aggregate is kept forever.
SELECT add_retention_policy('samples', INTERVAL '2 years');
```

Retention and aggregate refresh live in the database rather than in systemd timers: fewer moving
parts on the box, and a policy that cannot silently stop because a unit file was mistyped.

## 4. Task rework

### T-C1 — PostgreSQL 16 bare metal (**amended**)

Adds, after the base install:

```sh
curl -s https://packagecloud.io/install/repositories/timescale/timescaledb/script.deb.sh | sudo bash
sudo apt-get install -y timescaledb-2-postgresql-16 timescaledb-toolkit-postgresql-16 postgresql-16-cron
sudo timescaledb-tune --quiet --yes    # then hand-check against the 8 GB budget
```

`postgresql.conf` additions:

```
shared_preload_libraries = 'timescaledb,pg_stat_statements,pg_cron'
cron.database_name = 'vitals'
timezone = 'UTC'
timescaledb.max_background_workers = 4
```

`timescaledb-tune` assumes it owns the machine. It does not — `vitalsd`, the collectors, and the
klaxon share this box. Re-assert `shared_buffers = 2GB` after running it.

**Verify:** `psql -tAc "SELECT extname FROM pg_extension" | sort | tr '\n' ' '`
**Expect:** includes `timescaledb`, `timescaledb_toolkit`, `pg_cron`, `pg_stat_statements`.

### T-D3 — `libvitals` (**reworked: SQLite removed entirely**)

No longer a SQLite store. Now the COPY-binary encoder plus the spool writer.

> **Superseded in part.** This amendment predates the Rust decision, so the signatures below were
> written in C and the clock fields were later removed (amendment 04 §8). The *storage* argument —
> COPY-binary spool, TimescaleDB, portable encoder — stands unchanged and is what this amendment is
> for. **T-D3 in `tasks.md` is authoritative for the interface.** Do not implement from this section.

**Creates:** the COPY-binary encoder and spool writer in `crates/vitals-core`, and tests for each.

The encoder is **pure and portable** — no database handle, no I/O — so it unit-tests on macOS as
well as on the Pi. That is a real gain over the SQLite design, where the store could only be tested
on Linux:

```rust
/// Encodes one sample as a Postgres binary COPY row into `out`.
/// Returns the number of bytes written, or `None` if `out` is too small.
/// No allocation, no I/O.
pub fn encode_sample(out: &mut [u8], s: &Sample) -> Option<usize>;

/// The 19-byte file header and the 2-byte trailer.
pub fn header(out: &mut [u8; 19]) -> usize;
pub fn trailer(out: &mut [u8; 2]) -> usize;
```

Byte-order note: all COPY binary integers are **network byte order**. Both hosts are little-endian
arm64, so every field needs an explicit `htonl`/`htobe64` — a memcpy of a native integer produces a
silently wrong value that Postgres will accept. Tests assert against byte arrays captured from
`COPY … TO STDOUT (FORMAT binary)` on a real server, not against the encoder's own output.

The spool writer is Linux-side: `O_APPEND` writes, batched `fsync`, rotation, offset file.

**Verify:** `cargo test -p vitals-core` and a round-trip test that encodes 1000 samples, drains
them into a scratch database, and asserts every value matches within 1e-9.

### T-D5 — WAL tail → **spool drainer** (**reworked**)

**Creates:** the spool drainer binary, `/etc/systemd/system/vitals-spool-drain.service`

Replaces the SQLite-WAL-tailing design with the staging-table drain in §2.2. Retains the parent
design's cursor discipline unchanged: the offset advances only in the transaction that committed the
rows.

Adds `--verify`, which compares the spool header against the live table shape and refuses to drain a
mismatch. A schema change with a non-empty spool must fail loudly, not corrupt.

**Verify:** encode 10 000 samples to a spool, drain, assert the row count and that the offset equals
EOF. Then `SIGKILL` the drainer mid-batch, restart, and assert the final count is still exactly
10 000 with no duplicates.

### Registry codegen (T-D1) — follow-up

`registry.sql` must now emit the hypertable creation, the compression and retention policies, and
the continuous aggregate. Filed as a follow-up on the T-D1 branch rather than folded in, so the
generator fix already in review is not entangled with this amendment.

## 5. What does not change

- `samples` stays append-only. No `UPDATE`, no `DELETE`; removal is a tombstone.
- HLC columns, node ids, and the multi-writer merge story are unchanged.
- The registry remains the single source of truth for sources, types, units, and ranges.
- The life-safety path still runs as static C binaries under systemd, outside Docker.
- The phone keeps its own SQLite store. It replicates over HTTP; the engines were never required to
  match, and this amendment makes that explicit rather than accidental.

---

## 7. Migrations — Sqitch, and the two-owner split

**Not Alembic**, despite v1 using it. Alembic's value is autogenerating revisions from SQLAlchemy
models, and there are none here — the C/Rust daemon writes via `COPY` and the API reads. Without
models every revision is `op.execute("raw sql")`, which is Sqitch with a Python dependency attached.

**Sqitch**, for one reason that outweighs the rest: **every change carries a verify script.** Other
tools record *that* a migration ran; Sqitch makes *proving it produced the intended schema* a
first-class artifact that runs on every deploy. That is the same discipline as the rest of this
project — acceptance is a command that exits 0, not an assertion.

Secondary: it rides directly on `psql`, so `create_hypertable`, `add_compression_policy` and
continuous aggregates work with no tool-side schema model to choke on them. Dependency ordering
rather than a linear counter matters here because `retention` must not deploy before `samples_1min`
exists — dropping raw data before the rollup exists loses it permanently, and that constraint is
expressed in the plan rather than in a comment.

Rejected: **dbmate** (simpler, no verify scripts), **Atlas** (declarative diffing fights generated
DDL), **Flyway/Liquibase** (JVM).

### 7.1 Two owners, split explicitly

The schema has two sources of truth. Left implicit, they drift.

| Owner | Owns | Applied |
|---|---|---|
| `db/` (Sqitch) | **Structure** — `samples`, hypertable, compression, aggregates, retention, domain tables, indexes | Versioned changes with deploy/revert/verify |
| `registry.sql` (generated) | **Reference data** — the `vitals_type` / `vitals_source` rows | Idempotent `ON CONFLICT DO UPDATE`, reconciled on service start |

Structure is *migrated*; reference data is *generated and reconciled*. Sqitch creates the lookup
tables empty; the generator fills them and keeps them current as `registry.toml` evolves. Neither
can silently claim the other's objects.

**Follow-up on T-D1:** the generator currently emits both structure and rows. It must be trimmed to
rows only.

### 7.2 The initial change set

Seven changes in `db/`, each with deploy, revert and verify:

```
extensions            timescaledb, toolkit, pg_cron, pg_stat_statements
registry_tables       [extensions]           lookup table structure only
samples               [registry_tables]      the table + unlogged staging table
samples_hypertable    [samples]              1-day chunks
samples_compression   [samples_hypertable]   columnar, after 7 days
samples_1min          [samples_hypertable]   continuous aggregate, carries min/max
retention             [samples_1min]         raw drops at 2 years
```

Two reverts deliberately refuse rather than proceed:

- `extensions` does **not** drop `timescaledb` — that would destroy every hypertable in the database.
  That is data loss, not a schema revert.
- `samples_hypertable` raises if `samples` has rows, because a hypertable cannot be converted back in
  place and reverting would discard them.

A revert that silently destroys data is worse than one that refuses. Both say why.
