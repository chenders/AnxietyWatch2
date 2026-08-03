# Amendment index

**Mandatory reading for every task**, regardless of that task's `Reads:` list — see `AGENTS.md`.

Amendments supersede the original design and, where they disagree with a task brief, **the amendment
wins**. If you find a contradiction, report it rather than implementing either version.

---

## Accepted amendments

| # | Title | Supersedes | Affects |
|---|---|---|---|
| **01** | [Drop SQLite; COPY-binary spool + TimescaleDB](2026-08-03-amendment-01-timescale-copy-spool.md) | design §2.1, §2.5, §4.1 | T-C1, T-D3, T-D5 |
| **02** | [Integrity, provenance, encryption at rest](2026-08-03-amendment-02-integrity-and-encryption.md) | design §2 | T-C1, T-D3, T-D4, + T-C4/C5/C6, T-D7, T-H1 |
| **03** | [Multi-modal alerting](2026-08-03-amendment-03-multimodal-alerting.md) | design §5, T-A2 | Stage 5, + T-K1…T-K6 |
| **04** | [Multi-model review findings](2026-08-03-amendment-04-review-findings.md) | design §3–§5, amendments 01–03 | most of Group D |
| — | [C vs Rust evaluation](2026-08-03-c-vs-rust-evaluation.md) | the C11 decision | all code tasks |
| — | [Review synthesis and fix tracker](../reviews/SYNTHESIS.md) | — | status of every finding |

---

## The short version — what changed since the original design

Read this before any implementation task. Each line replaces something the original design or an
early task brief still says.

### Language

**Rust, not C11.** Cargo workspace, not meson. `#![forbid(unsafe_code)]` in decoder crates. Clippy
lints at `deny`: `indexing_slicing`, `unwrap_used`, `panic`, and the cast lints. `panic = "abort"` in
release. Toolchain pinned in `rust-toolchain.toml`.

### Storage

**SQLite is gone.** There is no on-Pi SQLite store, no GRDB, no WAL tail.

Collectors append rows **already encoded in PostgreSQL binary `COPY` format** to a spool file.
`vitals-spool-drain` streams those bytes into `COPY … FROM STDIN (FORMAT binary)` via a **private
`TEMP` staging table** — never a shared one — then `INSERT … ON CONFLICT DO NOTHING`.

PostgreSQL 16 + **TimescaleDB**: `samples` is a hypertable, compressed after 7 days, with a
`samples_1min` continuous aggregate carrying min/max. Retention drops raw at 2 years.

**Never use `locf()` or `interpolate()` on a physiological series.** Use `time_bucket_gapfill` so
gaps come back as explicit NULLs.

### Schema

```sql
PRIMARY KEY (source, type, session_id, seq)   -- stable ingest identity
timestamp   TIMESTAMPTZ NOT NULL              -- best-estimate event time, NOT the key
batch_id    BIGINT REFERENCES ingest_batch(id) -- NULL = did not arrive signed
```

- **`hlc_physical` and `hlc_logical` are removed.** `node_id` stays, as provenance.
- `seq` is a monotonic per-session index. For RR intervals it comes from the cumulative RR sum, so
  it is unaffected by packet arrival jitter or retransmission.
- **Never key on packet arrival time.** Several RR intervals arrive in one notification; anchoring
  them to arrival either collides (dropping real data) or shifts on retransmit (admitting
  duplicates). See amendment 04 §1 and `SYNTHESIS.md` A2.

### Migrations

**Sqitch** in `db/`, three scripts per change: deploy, revert, **verify**. A change touching
`samples` must guard on the spool being empty — binary COPY freezes column count and order, so
migrating with a non-empty spool orphans buffered rows unrecoverably.

### Safety-path invariants

- **`OOMScoreAdjust=-1000`** plus `MemoryMin`/`MemoryHigh` on `vitalsd`, `klaxond`, `cns-engine` and
  every collector. Postgres and Docker get *positive* adjustments. Without this the kernel kills the
  alarm to save the database.
- **Every alarm output must be provably OFF from power-on until `klaxond` takes control** — in
  hardware, not by software timing. GPIO floats before configuration, so a 10 kΩ gate-to-source
  pulldown is mandatory on every MOSFET. This rule exists because false-alarm-on-boot has now been
  designed in three separate times, at three different layers.
- **Never let a storage or infrastructure fault fire the klaxon tone.** A failed LUKS unlock uses a
  distinct signal and escalates only after 15 minutes inside a monitoring window. An alarm that
  cries wolf is a broken alarm.
- **Self-tests must be able to fail.** ALSA accepting a write is not proof of sound; enroll the DAC
  by stable identity, disable USB autosuspend, assert a volume floor, and current-sense the siren.

### Alerting

Channels are a trait with declared dependencies. **Independence is reported in tiers** — peripheral,
process, kernel, host — because everything shares one CPU and one `klaxond`. Amendment 03's flat
"N independent domains" was false confidence.

A **555 hardware dead-man's switch** provides the only tier-2 independence, but its heartbeat must be
**conditional on end-to-end liveness** (klaxond pulses only if it has seen a fresh sample), and it
needs a shutdown inhibit so a deliberate reboot is silent.

### Tools adopted

`kani` (prove decoders panic-free) · `rppal` (GPIO) · `fs4` (spool locking) · `sqlx`
(compile-time-checked queries) · `metrics` (Prometheus) · `tokio` · `bluer` (BlueZ) ·
`tokio-postgres` with the `binary-copy` feature.

---

## Still open

Tracked in [`../reviews/SYNTHESIS.md`](../reviews/SYNTHESIS.md) §C. Do not assume these are handled:
drain-progress liveness, disk-full behaviour, BLE MAC randomisation (use IRK, not static MAC), spool
format version detection, stuck-`cns-engine` detection, gap-fill enforcement, latency SLO, warm
replay after a `vitalsd` restart, subscriber backpressure, off-box backup.
