# AnxietyWatch v2 — Pi-Resident Vitals Core

**Date:** 2026-08-03
**Status:** Design, pending approval
**Supersedes (partially):** `docs/redesign_spec.md` §1.7, CLAUDE.md Design Principle #1
**Hardware:** Raspberry Pi 5 (BCM2712, 8 GB LPDDR4X), NVMe via PCIe, Sena UD100-G03 BLE, Hosyond 7" DSI
**Host:** `$PI_HOST` (hostname `pi-host`), passwordless sudo, Claude Code 2.1.220 installed

---

## 0. Measured baseline (2026-08-03)

Surveyed over SSH rather than assumed. Facts below supersede any estimate elsewhere in this document.

| | Measured |
|---|---|
| Board / kernel | Pi 5 Model B Rev 1.1, Debian 12 bookworm, `6.12.96+rpt-rpi-2712` aarch64 |
| Memory | 7.9 GiB (8 GB confirmed), zram/swap 199 MiB, `swappiness=60` |
| Storage | **microSD only** — 119.4 GB, `/` ext4 with 104 GB free. **No NVMe present.** |
| PCIe | `BCM2712 PCIe Bridge` live at `0002:00:00.0`. Lane free for an M.2 HAT. |
| EEPROM | `BOOT_ORDER=0xf461` → SD → NVMe → USB. **Already NVMe-boot capable; no EEPROM change needed.** |
| Bluetooth | `hci0` only (UART, onboard, `2C:CF:67:XX:XX:XX`). **Sena dongle not yet attached** — `lsusb` shows root hubs only. |
| Network | `wlan0` on **5 GHz** (ch 48, 5240 MHz, −63 dBm, 270 Mbit/s). `eth0` down. |
| Display | **`card1-DSI-2: connected`, mode `800x480`.** The 7" panel is attached and driving. |
| Desktop | `graphical.target`, **wayfire** compositor running (no Xorg). |
| Audio | HDMI only (`vc4hdmi0`, `vc4hdmi1`). **No analog and no USB DAC.** |
| Thermal | 47.2 °C idle, `throttled=0x0`, governor `ondemand` @ 2.4 GHz, load 0.00 |
| Toolchain | `gcc`, `clang`, `cmake`, `meson`, `python3` present. **No** docker, postgres, sqlite3, pgbackrest, node. |

Four consequences:

1. **The 5 GHz association already resolves BT coexistence.** The Pi 5 shares one antenna between WiFi
   and Bluetooth, and 2.4 GHz WiFi contends directly with BLE. This box is already on 5 GHz, so
   ethernet is an availability preference, not an interference fix.
2. **NVMe boot needs no EEPROM work** — `0xf461` already tries NVMe second. Stage 0 is physical
   installation only.
3. **The display stage is de-risked before it starts.** DSI-2 is connected at native resolution with
   a compositor already driving it. Stage 8 becomes a client for an existing session rather than a
   bring-up.
4. **Everything else is a clean slate**, with a full C toolchain already present.

---

## 1. Thesis

The iPhone is the wrong hardware for two jobs this system depends on, and the right hardware for
everything else.

**Wrong for:** holding BLE connections continuously (iOS backgrounding, CoreBluetooth state
restoration, and the battery cost — the drain work on `fix/battery-drain-remediation-all` is the
symptom), and sounding a reliable alarm (a speaker driven from a battery at 71.5% health throttles
under voltage sag).

**Right for:** HealthKit access, journaling, charts, reports. Those work today and should not be
rewritten.

So v2 is not a port. It is a **relocation of two responsibilities** — the always-on radio and the
always-on alarm — onto hardware that is mains-powered, has a real OS, an external antenna, and a
speaker that does not brown out. Everything else stays where it is and gets thinner over time.

The Pi replaces gen1-host as the single system of record.

---

## 2. The core: `vitals`

> *"A central core similar to HealthKit, but with only the restrictions our own system imposes."*

HealthKit's value was never storage. It was four things: a **type registry**, a **provenance model**,
an **observer API**, and a permission model. v2 keeps the first three, drops the fourth, and adds the
one HealthKit never had: **a total order across writers.**

### 2.1 It already exists

`docs/redesign_spec.md` §1.2 defines it and `AnxietyWatchKit/Storage/` implements ~40% of it:

```sql
CREATE TABLE samples (
  source        INTEGER NOT NULL,   -- from generated registry
  type          INTEGER NOT NULL,   -- from generated registry
  timestamp     REAL    NOT NULL,   -- UTC epoch seconds. Never local time.
  value         REAL    NOT NULL,   -- canonical unit for `type`. Always.
  extra         BLOB,               -- typed per (source,type); schema in registry
  hlc_physical  INTEGER NOT NULL,
  hlc_logical   INTEGER NOT NULL,
  node_id       BLOB    NOT NULL,   -- 16 bytes
  PRIMARY KEY (source, type, timestamp)
) WITHOUT ROWID, STRICT;
```

Those last three columns are a Hybrid Logical Clock. An HLC exists to merge writes from multiple
independent nodes without a coordinator. It was over-engineering for a single phone. It is exactly
right for a Pi, a phone, and a watch that each observe reality independently and reconcile later.

**v2 promotes this from one table among forty to the substrate everything else is a view over.**

### 2.2 The inversion

Spec §1.7 currently forbids HealthKit-owned types (HR, HRV, resting HR, respiratory rate) from
entering `samples`, because HealthKit was authoritative.

**This inverts.** The Pi reads HR and RR intervals from the Polar H10 directly over BLE, continuously,
on mains power. That is a better measurement than HealthKit's, available when the phone is asleep, in
another room, or dead. HealthKit becomes **source #2 among many** — an enrichment stream carrying
what only Apple can see (sleep stages, wrist temperature, the Watch's own resting HR), pushed
opportunistically by the phone, absent without consequence.

`SamplesStore.healthKitOwnedTypes` and its `preconditionFailure` are deleted. Provenance is carried
by `source`, which is what `source` is for.

### 2.3 What `vitals` is, concretely

Three artifacts:

| Artifact | Language | Role |
|---|---|---|
| `libvitals` | C11 | Append, query, subscribe. Links into every collector. No network, no SQL. |
| `vitalsd` | C11 | Owns the store. Single writer. Stamps HLC. Enforces the registry. Fans out to subscribers. |
| `vitals-registry` | TOML → codegen | The one place a source, type, or unit is ever declared. |

**The registry is the keystone.** One TOML file generates:

- `vitals_types.h` — C enums and canonical-unit constants
- `registry.sql` — Postgres enum tables + `CHECK` constraints
- `VitalsRegistry.swift` — Swift enums for the phone
- `registry.json` — for the API layer and any Python

```toml
[type.spo2]
id = 2
unit = "percent"
range = [50.0, 100.0]           # hard reject outside; quarantine, never clamp
precision = 0.1

[type.rr_interval]
id = 5
unit = "seconds"
range = [0.2, 3.0]

[source.polar_h10]
id = 1
transport = "ble"
service = "0x180D"              # SIG Heart Rate
provides = ["hr", "rr_interval"]

[source.ad_ua651ble]
id = 6
transport = "ble"
service = "0x1810"              # SIG Blood Pressure
provides = ["bp_systolic", "bp_diastolic", "bp_map", "hr"]
indication = true               # CCCD 0x0002, not 0x0001

[source.healthkit]
id = 2
transport = "push"
provides = ["sleep_stage", "wrist_temp", "resting_hr", "respiratory_rate"]
```

A source label can never drift, because no source label is ever typed by hand — `"polar_h10"` exists
in exactly one file and is an integer everywhere else.

### 2.4 The API — HealthKit's observer, without the wait

`vitalsd` listens on a Unix domain socket. Three verbs, length-prefixed CBOR:

```c
// Append. Idempotent on (source, type, timestamp). Caller does not supply HLC.
int  vitals_append(vitals_conn *c, const vitals_sample *s, size_t n);

// Query. Half-open [from, to). Zero-copy into caller's buffer.
int  vitals_query(vitals_conn *c, vitals_selector sel, vitals_sample *out, size_t cap);

// Subscribe. Live fan-out. This is HKObserverQuery without the 30-second latency
// and without background-delivery budget.
int  vitals_subscribe(vitals_conn *c, vitals_selector sel, vitals_cb cb, void *ctx);
```

Unix sockets mean filesystem permissions are the access-control model. That is the whole permission
system, and it is sufficient for a single-user appliance.

### 2.5 Append-only

`samples` takes `INSERT` only. No `UPDATE`, no `DELETE`. Removal is a tombstone row covering a range.
Retention and downsampling emit tombstones plus rollups; they never mutate history in place.

This is the design decision the most other properties fall out of: replay is exact, backup is
incremental by construction, merge is commutative, and a bug in a derived value is fixed by
recomputing the view rather than by a migration that rewrites data.

---

## 3. Process topology

The organizing rule: **the life-safety path never runs in Docker and never depends on the network.**

```
┌──────────────────────── Raspberry Pi 5 ─────────────────────────┐
│                                                                  │
│  systemd — life-safety path, static C binaries, hw watchdog      │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  polar-collector ─┐                                        │ │
│  │  emay-collector  ─┤                                        │ │
│  │  bp-collector    ─┼──▶ vitalsd ──▶ cns-engine ──▶ klaxond  │ │
│  │  hk-receiver     ─┤     (single      (pure          (amp + │ │
│  │  cpap-collector  ─┘      writer)      fold)          horn) │ │
│  └───────────────────────────┬────────────────────────────────┘ │
│                              │ WAL-tail replication              │
│                              ▼                                   │
│  PostgreSQL 16 — bare metal, NVMe, pgBackRest ──▶ backups        │
│                              ▲                                   │
│  ┌───────────────────────────┴────────────────────────────────┐ │
│  │  Docker — conveniences, restartable, read-mostly           │ │
│  │  api · admin · analysis · grafana · backup-runner          │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                    Cloudflare tunnel ──▶ phone, anywhere         │
└──────────────────────────────────────────────────────────────────┘
```

**Why the split.** A container runtime is a dependency: an upgrade, a daemon, a network namespace, a
storage driver. None of that belongs between a desaturation and a siren. The collectors, the store,
the detector, and the alarm are static binaries under systemd with `Restart=always` and the BCM2712
hardware watchdog armed — if the kernel hangs, the board reboots and the alarm path comes back
without anyone present. The API, admin UI, and analysis are genuinely fine to restart, upgrade, and
occasionally break, so they live in Docker where that is cheap.

### 3.1 Language assignment

C where the process is always-on and must be small, dependency-free, and predictable. Not C where it
buys nothing.

| Component | Language | Rationale |
|---|---|---|
| `libvitals`, `vitalsd` | C11 | Always-on, single writer, no runtime, no GC. |
| BLE collectors | C11 | Long-lived, one radio each, restartable in isolation. |
| `cns-engine` | C11 | Pure fold, `(state, sample) → (state, [cmd])`. Trivially testable, trivially replayable. |
| `klaxond` | C11 | Must work when everything else does not. |
| API / admin / analysis | Python 3.12 | Ports from `server/`. Not on the safety path. |
| EDF / CPAP parsing | Python | Already written, already tested, batch-mode. |
| Phone | Swift | Unchanged. |

**Wire decoders are fuzzed, not trusted.** Every function parsing bytes off a radio —
`0x2A35` blood pressure, `0x2A37` heart rate, EMAY frames, Oura frames — is a pure
`(const uint8_t*, size_t) → result` with no allocation, reached by a libFuzzer harness in CI with a
seed corpus of real captures. The repo already carries the fuzzing skills for this. Decoders are the
only C in the system that sees hostile input, and they are the easiest possible shape to prove
correct: no state, no I/O, one buffer in, one struct out.

---

## 4. Storage and durability

### 4.1 Postgres on bare metal

Not in Docker. Not on the microSD. On an NVMe SSD via the PCIe lane, with the 1 TB drive when it
arrives as the archive tier.

- `PGDATA` on NVMe, `ext4` with `noatime`
- microSD is boot + rootfs only, mounted `ro` where practical
- Tuned for 8 GB: `shared_buffers=2GB`, `effective_cache_size=4GB`, `work_mem=32MB`,
  `maintenance_work_mem=512MB`, `max_connections=40`
- `wal_level=replica`, WAL archived continuously

`vitalsd` writes SQLite (WAL, `synchronous=NORMAL`) as its hot path — a single-writer append at
sensor rates should not cross a socket to a server process — and tails its own WAL into Postgres.
SQLite is the durable ingest buffer; Postgres is the queryable system of record. If Postgres is down,
ingest does not stop, and nothing is lost.

### 4.2 Backups

Three tiers, each independently sufficient:

1. **pgBackRest** — full weekly, incremental nightly, continuous WAL archive to the 1 TB drive.
   Point-in-time recovery to any second.
2. **Off-box** — nightly `pgbackrest` push to a second location. Encrypted at rest.
3. **Cold export** — monthly, the whole store as newline-delimited CBOR + the registry TOML. Readable
   with no Postgres, no Docker, and no code from this repo. The format is documented in one page.

**Restore is tested on a schedule, not on faith.** A monthly systemd timer restores the latest backup
into a scratch cluster, runs a row-count and checksum comparison against production, and writes the
result to the health endpoint. A backup that has never been restored is a hypothesis.

---

## 5. What v2 makes structurally impossible

The CLAUDE.md "Common pitfalls" list is a scar catalogue — every entry cost real debugging. A fresh
start should not re-document them. It should make them unrepresentable.

| Hard-earned lesson | v2 structural fix |
|---|---|
| Sync cursor advanced to `.now` after I/O opened a data hole | Cursors are HLC positions **derived from data already written**. There is no wall clock in the sync path to be wrong. |
| A table synced UP but had no importer DOWN; 224k rows unrecoverable | One narrow table with generic replication. No per-table importer exists to forget to write. |
| `"polar_h10"` string literals drifted from typed constants | `source` is an integer from generated code. The string exists in one TOML file and never on the wire. |
| Nil discriminators on recently-added columns broke predicates | `STRICT` tables, every column `NOT NULL`. A nullable discriminator cannot be declared. |
| First-run-only paths rotted invisibly for months | Bootstrap is idempotent and runs identically on every start. There is no first-run branch. |
| A write before a gate resolved permanently bricked restore | Append-only + HLC merge. Restore is a merge, not a guarded import. "Is the store empty?" is never asked. |
| A guard failed closed but would not say why | Every daemon serves structured health on its socket. Any blocked state names its blocker and the value that blocked it. |
| HealthKit auth froze; aggregation overwrote real data with nils | The Pi's primary path does not read HealthKit. HealthKit absence is an absent source, not a null value. |
| Unit mismatches between ingest and aggregation | Canonical unit per type in the registry. Conversion happens once, at the decoder. The store has one unit per type, always. |
| Timezone and DST bugs in date arithmetic | The store holds UTC epoch seconds and nothing else. Local time exists only in the presentation layer. |
| Float equality in tests | Property-based tests over generated inputs; exact equality only on replayed fixtures. |
| Magic numbers duplicated across files | Thresholds live in the registry TOML, generated into C, SQL, and Swift. |
| Main-thread hangs from `@Query` + compound predicates | The phone stops querying a live database. It renders a projection the Pi computes and pushes. |
| Battery drain from continuous BLE | Mains power. |

### 5.1 Practices adopted fresh

**Deterministic replay is a first-class feature.** `cns-engine` is a pure fold. Any night can be
replayed from the sample log through the detector and must produce byte-identical output. This makes
threshold tuning empirical instead of theoretical: change a constant, replay six months, count what
would have fired. It also makes every past incident a permanent regression test.

**The alarm path proves itself daily.** `klaxond` runs a self-test on a timer: it drives a brief tone
through the amp and confirms via ALSA that the device accepted the write and the expected sample
count was consumed. A failure is itself an alert — routed to the phone via APNs, which is the one
notification path that does not depend on the thing being tested. A speaker that died in the night
is discovered the next morning, not on the night it mattered.

**Silence is an alert condition.** During a monitoring window, `vitalsd` expects samples at a known
cadence per source. Absence past a threshold raises a distinct "sensor lost" tier — never confused
with "readings are normal." This is the existing CNSRisk asymmetry invariant (indeterminate ≠ safe),
enforced at the store rather than only in the scorer.

**One binary, one job, restartable independently.** A wedged Polar collector must not take down the
EMAY collector, and neither must touch the alarm. Each is a separate systemd unit against the same
`vitalsd` socket.

---

## 6. Stages

Each stage ends with something that works and is worth keeping. No stage depends on a later one.

### Stage 0 — Hardware floor (~1 hr)

Smaller than planned: 5 GHz WiFi, NVMe boot order, and the DSI panel are already correct (§0).

**Physical:** install M.2 HAT + NVMe SSD; attach USB DAC/amp + speaker; plug in the Sena dongle.

**Configuration:**
- Migrate `/` to NVMe; microSD keeps `/boot/firmware` only. `PGDATA` and the SQLite store on NVMe.
- Sena binds as `hci1`. Disable onboard Bluetooth with `dtoverlay=disable-bt` so `hci0` cannot be
  selected by accident — collectors then have exactly one adapter and cannot bind the wrong one.
- Arm the BCM2712 hardware watchdog (`dtparam=watchdog=on`, `RuntimeWatchdogSec=15` in
  `systemd/system.conf`).
- Governor `ondemand` → `performance`. A collector that must not miss a connection interval should
  not be waiting on a frequency ramp, and the thermal headroom is there (47 °C idle, `throttled=0x0`).
- Disable unattended-upgrades. This box does not reboot itself for a kernel patch mid-night; updates
  are a deliberate act.
- Keep `graphical.target` and wayfire — the panel is already working and Stage 8 uses it. No
  life-safety daemon may depend on it; each is `WantedBy=multi-user.target` and must survive the
  compositor dying.

### Stage 1 — Thesis spike (~1 evening)

**Before any infrastructure**, prove the claim the project rests on: a Pi holding a Polar H10 for
eight uninterrupted hours. `bluetoothctl` + a 100-line C GATT client appending RR intervals to a flat
file. Success is a drop count and a gap histogram over one night. Everything below assumes this
number is good; measure it before building on it.

### Stage 2 — Storage floor

Postgres 16 bare metal on NVMe, tuned. pgBackRest with all three backup tiers. The monthly
restore-verification timer. Nothing writes to it yet. **Durability before data worth keeping.**

### Stage 3 — `vitals` core

The registry TOML and its codegen. `libvitals` + `vitalsd`. The SQLite→Postgres WAL tail.
`bp-collector` as the first real producer — the A&D cuff speaks standard `0x1810`, so it is the
cleanest possible end-to-end proof. Fuzz harness for the `0x2A35` decoder, including the IEEE-11073
SFLOAT sentinels (`0x07FF` NaN, `0x0800` NRes) which map to *absent*, never to a number.

Deliverable: take a blood pressure reading, watch it land in Postgres with correct provenance.

### Stage 4 — The BLE fleet

`polar-collector`, `emay-collector`, then Oura. Multi-adapter discipline (explicit `hci` binding),
reconnect with backoff, per-source liveness into the silence detector. This is the stage that retires
the phone's battery drain.

### Stage 5 — Detector and klaxon

Port `Services/CNSRisk/` to C as a pure fold — it was written free of CoreBluetooth, SwiftData, and
SwiftUI precisely so it could move. `CNSThresholds` becomes registry entries. The replay harness
lands here, and every historical night becomes a fixture. `klaxond` with the daily self-test.

**At the end of Stage 5 the Pi independently detects and alarms with the phone powered off.** That is
the project's actual goal; everything after is consolidation.

### Stage 6 — Migrate gen1-host

Move the remaining domain tables (journal, medications, prescriptions, pharmacies, labs, songs,
CPAP/AS11, Oura cloud) with their Flask API and admin UI into Docker on the Pi. Cloudflare tunnel for
remote access. Decommission gen1-host.

### Stage 7 — Thin the phone

The iOS app keeps HealthKit reads (pushing to `hk-receiver`), journaling, and viewing. It stops
computing baselines, stops running detection, stops holding BLE sessions. `Services/` drops from
~21k lines to roughly 3k.

### Stage 8 — Bedside panel

The panel is already connected and driving at 800×480 under wayfire (§0), so this is a client, not a
bring-up. A single fullscreen app subscribing to `vitalsd`: current tier, live SpO₂/HR, last BP,
per-sensor liveness, alarm self-test status, backup freshness.

At 800×480 it is a status light, not a dashboard — designed as such: large type, high contrast, one
screen, readable at arm's length in a dark room without becoming a light source. Since a compositor
is already running and the touchscreen already works, a native Wayland client is the pragmatic path;
raw framebuffer would be fighting a working stack for nothing.

The panel is a **viewer, never a dependency**. It subscribes over the `vitalsd` socket and holds no
state. Killing it must not perturb collection, detection, or the alarm — asserted by a test that
kills it mid-session and diffs the sample stream.

---

## 7. Bill of materials

| Item | Status | Note |
|---|---|---|
| Pi 5 8 GB + CanaKit PRO | **Verified running** | 47 °C idle, `throttled=0x0` at 2.4 GHz |
| 128 GB microSD | **Verified** | 104 GB free. Becomes `/boot/firmware` only after Stage 0 |
| Hosyond 7" DSI | **Verified connected** | `card1-DSI-2` at 800×480. Ribbon is correct; nothing to check |
| Sena UD100-G03 | Arriving — **not yet attached** | `lsusb` shows root hubs only. Will bind as `hci1` under `btusb` |
| A&D UA-651BLE | Arriving | Standard `0x1810`. Bond to the Pi, not the phone |
| **M.2 HAT + NVMe SSD** | **Needed** | ~$45–70. PCIe bridge is live and `BOOT_ORDER` already tries NVMe |
| **USB DAC + amp + speaker** | **Needed** | Only HDMI audio exists today. This is the alarm's output path |
| 1 TB drive | Planned | Archive tier + pgBackRest repository |

### 7.1 Build coordination

The Pi runs Claude Code 2.1.220 at `~/.local/bin/claude` with passwordless sudo. Pi-resident work —
daemon builds, systemd units, Postgres tuning, BlueZ bring-up, overnight capture runs — is driven
there, where the hardware, the radios, and the logs are local. This repo remains authoritative for
specs, plans, and the registry TOML; generated headers are committed here and consumed there.

---

## 8. Open decisions

Two items are deliberately deferred to when data exists to decide them with:

1. **Rollup cadence.** `samples_1min` is specified; whether raw RR intervals are retained
   indefinitely or downsampled past N months should be decided once a real growth rate is measured,
   not estimated. The 1 TB drive makes "keep everything" plausible.

2. **Overnight BP.** The A&D cuff is manual-trigger and cannot produce a nocturnal series. If a
   nocturnal series is wanted later, that is an ABPM cuff — separate hardware, and the `0x1810`
   collector written in Stage 3 works with it unchanged.

---

## 9. Acceptance

v2 Stage 5 is complete when, with the iPhone powered off and the house internet disconnected:

1. The Pi holds Polar H10 and EMAY connections for a full night with a measured drop count.
2. A replayed historical desaturation drives the tier machine to `klaxon`.
3. `klaxond` sounds the alarm through the amp.
4. Every sample from that night is in Postgres with correct provenance, recoverable from backup
   into a scratch cluster, and byte-identical on replay.
