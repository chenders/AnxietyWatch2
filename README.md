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
  source        INTEGER NOT NULL,   -- from the generated registry
  type          INTEGER NOT NULL,   -- from the generated registry
  timestamp     REAL    NOT NULL,   -- UTC epoch seconds. Never local time.
  value         REAL    NOT NULL,   -- canonical unit for `type`. Always.
  extra         BLOB,
  hlc_physical  INTEGER NOT NULL,   -- hybrid logical clock: multi-writer merge
  hlc_logical   INTEGER NOT NULL,
  node_id       BLOB    NOT NULL,
  PRIMARY KEY (source, type, timestamp)
) WITHOUT ROWID, STRICT;
```

HealthKit's real contribution was never storage — it was a type registry, a provenance model, and an
observer API. This keeps those three, drops the permission model, and adds the one HealthKit never
had: a total order across independent writers.

`vitals/registry.toml` is the single source of truth for every source, type, canonical unit, and
valid range. It generates the C header, the SQL, the Swift enums, and the JSON. A source label is
never typed by hand.

## Architecture

The organizing rule: **the life-safety path never runs in Docker and never depends on the network.**

```
systemd — static C binaries, hardware watchdog
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
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build
ctest --test-dir build --output-on-failure
```

`vitals_core` (decoders, HLC) is pure C11 and builds on macOS and Linux — Apple Silicon and the Pi 5
are both arm64 little-endian, so results are identical. `vitals_linux` (SQLite store, `vitalsd`,
epoll, BlueZ) is Linux-only and is skipped on macOS rather than mocked.

Anything Linux-specific, **including debugging**, happens on the Pi:

```sh
scripts/sync-to-pi.sh --test
```

## Testing

```sh
scripts/coverage.sh --check
```

Gates are differentiated, because one flat number would be wrong in both directions:

| Layer | Gate |
|---|---|
| Decoders, HLC | **95%** — pure functions, and hostile input lands here |
| Store, daemon logic | **85%** — testable against `:memory:` SQLite |
| BLE / systemd glue | measured, not gated — needs real radios |

Coverage is a floor, not the goal. Every byte-parsing function also gets a libFuzzer target, because
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
