# AnxietyWatch2

A Raspberry Pi 5 that collects physiological data from body-worn sensors all night, detects
CNS-depression events, and sounds an alarm.

This is the second-generation system. The [first](https://github.com/chenders/AnxietyWatch) is an
iOS + watchOS app; it stays, and keeps the jobs it is good at. What moves here are the two jobs an
iPhone is structurally wrong for:

- **The always-on radio.** iOS backgrounding, CoreBluetooth state restoration, and battery cost make
  continuous BLE capture unreliable.
- **The always-on alarm.** A speaker driven from a degraded battery throttles under voltage sag.

A Pi on mains power with an external antenna and a real amplifier has neither problem. The phone
stays as a HealthKit bridge and a viewer.

## The core

One narrow table, append-only, and everything else is a view over it:

```sql
CREATE TABLE samples (
  source       INTEGER          NOT NULL REFERENCES vitals_source(id),
  type         INTEGER          NOT NULL REFERENCES vitals_type(id),
  session_id   BIGINT           NOT NULL,
  seq          BIGINT           NOT NULL,   -- from the sensor's own data
  timestamp    TIMESTAMPTZ      NOT NULL,   -- best-estimate event time, UTC
  value        DOUBLE PRECISION NOT NULL,   -- canonical unit for `type`. Always.
  extra        BYTEA,
  node_id      BYTEA            NOT NULL,
  batch_id     BIGINT,                      -- NULL = did not arrive signed
  PRIMARY KEY (source, type, session_id, seq)
);
```

**`timestamp` is not the key.** Several RR intervals arrive in one BLE notification, so keying on
arrival time makes them collide and `ON CONFLICT DO NOTHING` silently discards real measurements
while every health metric reads normal. `seq` is a monotonic per-session index derived from the
sensor's own data, so a retransmission dedupes correctly and a reconnect mints a new `session_id`
rather than faking continuity. `max(seq) - count(*)` per session is the lost-beat count, which makes
packet loss a measurable quantity instead of an invisible one.

HealthKit's real contribution was never storage — it was a type registry, a provenance model, and an
observer API. This keeps those three, drops the permission model, and adds the one HealthKit never
had: an identity model that makes packet loss measurable rather than invisible.

`vitals/registry.toml` is the single source of truth for every source, type, canonical unit, and
valid range. It generates the Rust enums, the SQL reference data, and the JSON. A source label is
never typed by hand.

## Architecture

The organizing rule: **the life-safety path never runs in Docker and never depends on the network.**

```
systemd — static Rust binaries, hardware watchdog
  collectors ──▶ vitalsd ──▶ cns-engine ──▶ klaxond
                    │
                    ▼
  PostgreSQL — bare metal, NVMe, pgBackRest
                    ▲
  Docker — api · admin · analysis   (restartable conveniences)
```

A container runtime is a dependency: an upgrade, a daemon, a storage driver. None of that belongs
between a desaturation and a siren.

## Build

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

`vitals-decode` and `vitals-registry` are `no_std` and build anywhere — Apple Silicon and the Pi 5
are both arm64 little-endian, so results are identical. `vitalsd` and the collectors (Postgres,
BlueZ, systemd) are Linux-only and are skipped on macOS rather than mocked.

Anything Linux-specific, **including debugging**, happens on the Pi:

```sh
scripts/sync-to-pi.sh --test
```

## Testing

```sh
cargo llvm-cov --workspace --summary-only
```

Gates are differentiated, because one flat number would be wrong in both directions:

| Layer | Gate |
|---|---|
| Decoders | **90%** — pure functions, and hostile input lands here |
| Store, daemon logic | **85%** — testable against a scratch Postgres |
| BLE / systemd glue | measured, not gated — needs real radios |

Coverage is a floor, not the goal. Every byte-parsing function also gets a `cargo-fuzz` target, because
coverage tells you a line ran, not that it survived a truncated frame.

## Documentation

| | |
|---|---|
| [`docs/HERDR-GUIDE.md`](docs/HERDR-GUIDE.md) | **Start here.** Build, dispatch, debug, recommendations |
| [`docs/STAGE-DELIVERABLES.md`](docs/STAGE-DELIVERABLES.md) | What each stage produces that you can look at |
| [`docs/specs/`](docs/specs/) | Architecture and rationale |
| [`openspec/changes/pi5-vitals-core/`](openspec/changes/pi5-vitals-core/) | The current change: proposal, design, 15 tasks, runbook |
| [`docs/design/`](docs/design/) | UI design prompts and the bedside panel |

## Status

Stage 0 of 8. The toolchain builds and tests on both hosts; no task has landed yet.

Progress is visible rather than asserted — each stage produces an artifact you open in a browser,
and those artifacts are the panels of the admin console. See
[`docs/STAGE-DELIVERABLES.md`](docs/STAGE-DELIVERABLES.md).

## Hardware

Raspberry Pi 5 (8 GB) · M.2 HAT + NVMe · Hosyond 7" DSI touchscreen · Sena UD100-G03 long-range BLE ·
Polar H10 · EMAY SleepO2 · A&D UA-651BLE blood pressure monitor · USB DAC + amplifier

## License

MIT
