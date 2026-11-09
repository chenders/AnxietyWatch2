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
| **05** | [Corrections and runtime safety](2026-08-03-amendment-05-corrections-and-runtime-safety.md) | **amendment 04 §1 and §3.2 — both were wrong** | T-B1, T-D3…D6, T-K2, T-K6 |
| **06** | [BP identity and durability](2026-08-03-amendment-06-bp-identity-and-durability.md) | **amendment 05 §5 (T-D6 row) — the hardware disproved it**, and **§1.1's identity rule, which it narrows**; extends §4.3; restates A01 §5's durability rule | T-B1, T-D3, T-D4, T-D6 |
| **07** | [The HTTP API and admin console are Rust, not Python](2026-08-05-amendment-07-api-in-rust.md) | **`language-choices.md` §1 row "HTTP API, admin, analysis → Python 3.12 + FastAPI", and §3 in full** | admin console M0–M2, web dashboard backend, client write API |
| **08** | [Layout selection by live evaluation, not upfront choice](2026-08-06-amendment-08-layout-evaluation.md) | the implicit "pick a design direction before M1" in `SPEC-admin-console.md` §3.1 | admin console M1 (all three directions behind a runtime switcher), web dashboard (gains a plan home: port B/C onto the `model()` seam + switcher, gated on the vitalsd query path) |
| **09** | [Production mobile enrollment, request authentication, and TLS topology](2026-08-30-amendment-09-mobile-enrollment-and-tls.md) | Amendment 02 mobile key custody; Amendment 07 client API; admin §9c bind/auth deferral; iOS §6.1 enrollment gate | T-I1, T-I2, T-I4, T-I5, T-I7, T-I17, T-I18, T-M2a, T-M2h |
| **10** | [First-draft private-LAN transport](2026-09-15-amendment-10-first-draft-lan-http.md) | Amendment 09 §§2, 3.2, and 7 for the first-draft iPhone release; iOS §§6.1 and 7.2 | first-draft T-I1, T-I2, T-I4, T-I5, T-I7, T-M2a |
| **11** | [Optional direct lyrics retrieval on the iPhone](2026-09-16-amendment-11-optional-direct-lyrics.md) | first-draft Pi-only Genius lyrics decision and iOS records/transport behavior | first-draft T-I4, T-I7, T-I11, T-I13, Pi lyrics/admin work |
| **12** | [Provider-aware song search and provider-independent identity](2026-09-16-amendment-12-provider-aware-song-search.md) | first-draft song-search provider choice and song-catalog identity behavior | first-draft T-I11, T-I13, song metadata/Pi lyrics work |
| **13** | [Platform-sample envelope and sub-provenance](2026-09-17-amendment-13-platform-sample-provenance.md) | platform-store identity and provenance decisions pending T-X3 | T-X3, T-X4, T-I6, T-I8, T-I19 |
| **14** | [Phone-silence escalation](2026-09-17-amendment-14-phone-silence-escalation.md) | open question 1 and undefined phone-silence escalation | T-X1, T-M2e, T-M2f |
| **15** | [HealthKit is a first-release phone capability](2026-09-17-amendment-15-healthkit-first-release.md) | Amendment 10's first-draft HealthKit deferral | T-I8, T-I9, T-I11, T-I18, T-I19, T-I20 |
| **16** | [Operator-directed APNs messages](2026-09-18-amendment-16-operator-directed-apns.md) | first-draft remote-notification deferral and the deferred Stage 7 alarm-push client follow-up; preserves Amendment 14's no-loop phone-silence rule | T-M6a–T-M6e, T-I22 |
| **17** | [Opt-in iPhone BLE mobility relay](2026-09-22-amendment-17-iphone-ble-mobility-relay.md) | Stage 7's blanket exclusion of iPhone BLE sessions and the ordinary-sync socket/keep-alive wording, only for a bounded mobility relay | T-I3, T-I18, T-I20, T-I21, T-I23 |
| **18** | [Deployment acceptance is evidence-based, not time-based](2026-09-28-amendment-18-deployment-evidence-not-time.md) | the undefined Pi deployment observation-period gate | Pi GitHub Actions deployment T-R8 |
| **19** | [`samples` keeps its identity; the hypertable is costed and declined](2026-11-08-amendment-19-samples-storage-and-identity.md) | **Amendment 01 §§3.1–3.4 — `samples` as a hypertable, its compression policy, the `samples_1min` continuous aggregate on it, and its retention policy** | T-M3a, then T-M3c/T-M3d/T-M3g/T-M3i/T-X5; `db/sqitch.plan` changes 4–7 |
| **20** | [Liveness has three evidence channels, and ids are permanent](2026-11-09-amendment-20-liveness-evidence-channels.md) | §2.5's single-quantity liveness comparison, its unbounded UNKNOWN, its on-demand rule, and the reuse of source id 4 | silence detector, console health view, `vitals/registry.toml` |
| **10-A11** | [AirSense acceptance scope](2026-09-13-amendment-10-air11-acceptance-scope.md) | T-A11-6's unrelated host-health gates | T-A11-6 |
| **11-A11** | [H10 live gate](2026-09-13-amendment-11-h10-live-gate.md) | Amendment 10's treatment of H10 as optional for T-A11-6 | T-A11-6 |
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
- `seq` is a monotonic per-session index derived from whatever the source makes **stable across
  retransmission** — for a *streaming* source that is the sensor's own data, never host timing
  (amendment 05 §1.1). For a **confirmed store-and-forward** source it is the delivery index
  (amendment 06 §1.2). A source that is neither cannot be given a correct identity and must not be
  ingested. A retransmission reuses the same `seq` and dedupes correctly; a reconnect mints a **new
  `session_id`**, so there is no false dedup across a discontinuity.
- **Platform-store samples use an opaque, lossless identity split** (amendment 13): the HealthKit
  sample UUID's first and last eight bytes become `session_id` and `seq`. The pair is stable across
  retries but is not chronological; sub-provenance remains in the versioned `extra` object.
- **`session_id` must be unique across process restarts.** A per-process counter from 0 makes a
  restarted collector's first rows collide with a previous run's and `ON CONFLICT DO NOTHING` drops
  them silently. Near-total for BP, where `seq` is 0–3 every time.
- `max(seq) - count(*)` per session is the **lost-beat count** — packet loss becomes measurable.
- **Never key on packet arrival time.** Several RR intervals arrive in one notification; anchoring
  them to arrival either collides (dropping real data) or shifts on retransmit (admitting
  duplicates). See amendment 04 §1 and `SYNTHESIS.md` A2.
- **`seq` derives from whatever the source makes stable across retransmission** (amendment 06 §1.2).
  For a streaming source that is the sensor's own data. For a **confirmed store-and-forward** source
  — the A&D cuff, which sends no timestamp, user id or counter — it is the delivery index, because
  the ATT confirmation makes redelivery impossible. A source that is neither confirmed nor
  payload-identifiable cannot be given a correct identity and **must not be ingested**.

### Durability

**`append { sync: true }` fsyncs before `vitalsd` replies** (amendment 06 §2). Used by
`bp-collector` and nothing else: BP frames are deleted by the device on the kernel's ATT
confirmation and are reconstructible from nothing, while a night of RR is ~250 MB and would be
destroyed by per-append fsync. Durability cost is paid only where loss is unrecoverable.

Every synced source also keeps a **raw-frame journal** written before decode, so a decoder bug is
replayable. A lost frame never is.

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

A **555 hardware dead-man's switch** provides the only tier-2 independence. Its heartbeat is
**conditional on the whole chain being alive** — a monitoring window active, a fresh primary sample,
a recent tier evaluation, and drain progress — because a healthy `klaxond` pulsing while every
collector is dead is the worst false negative available. A shutdown inhibit keeps deliberate reboots
silent, and it fails safe (armed by default). Its test injects upstream faults; stopping the
heartbeat only tests the 555. See amendment 05 §2.

**Amendment 16 adds operator-directed APNs messages** to one active device, one user's active
phones, or every active non-revoked phone registered with the current Pi. Recipient resolution,
confirmation, audit, token invalidation, minimal payloads, and separate APNs acceptance/display/
acknowledgement evidence are mandatory. This is never the phone-silence path and never replaces the
local klaxon.

**Amendment 17 permits an opt-in iPhone BLE mobility relay.** A supported BLE sensor may reach the
phone, which forwards an authenticated bounded stream over the private LAN to the Pi. The Pi remains
authoritative and the local detector/alarm remains independent. A NeoPixel bracelet is a convenience
indicator only; measured foreground/background behavior, stale/unknown handling, and physical-device
latency evidence are required before the relay is represented as more than an experiment.

### Mobile enrollment and network trust

**The eventual public phone API is HTTPS-only and the admin console remains private** (amendment 09),
but Amendment 10 permits the first draft to use HTTP only to a manually configured private-LAN Pi
address. The first draft has no away-from-home endpoint. Device enrollment, Secure Enclave keys,
RFC 9421 signatures, durable replay IDs, operator approval, and legacy-route exclusion remain in force.
The production HTTPS/Caddy topology and no-cleartext gate return before public or away-from-home use.

Amendment 11 additionally permits explicit, encrypted provisioning of the Pi-held Genius credential to
an opted-in iPhone for direct off-LAN lyrics retrieval. This does not permit silent provisioning or
move the Pi's authoritative record ownership.

### UI direction

**No layout is chosen from mockups** (amendment 08). The admin console's M1 renders every screen
through all three design directions (Service Mode / Preflight / Flowpath) behind a `?dir=` +
cookie switcher; the web dashboard ports Night Report and Terminal onto the existing `model()`
seam with the same switcher, gated on the vitalsd query path. One view-model per screen, three
renderings, no direction-specific data logic. The direction determination is recorded after real
use, then the losing template sets are deleted. The bedside panel's shipped eight-view swipe
switcher is the precedent and needs no change.

### Tools adopted

`kani` (prove decoders panic-free) · `rppal` (GPIO) · `fs4` (spool locking) · `sqlx`
(compile-time-checked queries) · `metrics` (Prometheus) · `tokio` · `bluer` (BlueZ) ·
`tokio-postgres` with the `binary-copy` feature.

### Liveness evidence

**Amendment 20 splits liveness into three evidence channels and makes source ids permanent.**
Attempt cadence, sample arrival and transport contact are separate physical quantities; comparing a
sample age against `expected_cadence_s` — a sample *rate* for the 1 Hz sources — is what let a
ten-day total-data-loss outage render identically to healthy operation while the detector emitted
15,628 events saying nothing had changed. An undeclared bound now contributes **nothing** rather
than falling back to the cadence, silence counts only while a declared activity gate says data
should be arriving, and `UNKNOWN` gains an upper bound so sustained silence is evidence of absence.
Source id 4, reassigned while holding 156,201 rows, is restored to its original device; the
reuse guard is in the generated SQL, because only the database knows whether an id has rows.

### Deployment acceptance

**Amendment 18 removes the undefined elapsed-time observation period from the Pi delivery gate.**
T-R8 now closes on a finite post-deploy evidence set: a green activation receipt, a read-only idle
snapshot, a manual rollback rehearsal with its exit result, and an explicit operator decision about
retaining or retiring the emergency sync path. No arbitrary wait or delivery count substitutes for
those facts.

---

## Still open

All of `SYNTHESIS.md` §C is now specified in **amendment 05 §4** — drain liveness, disk-full, BLE
IRK pairing, off-box backup, spool fingerprint, spool locking, stuck-detector detection, gap-fill
enforcement, the 2 s p99 latency SLO, warm replay after restart, and subscriber backpressure.

**Specified is not implemented.** Track build status in `SYNTHESIS.md` §H.
