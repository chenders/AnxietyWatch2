# AnxietyWatch2

AnxietyWatch2 is an experimental Rust system for investigating reliable overnight collection of
physiological sensor data on a Raspberry Pi 5. The target design combines local Bluetooth ingestion,
provenance-aware storage, and a network-independent alert path.

The [first generation](https://github.com/chenders/AnxietyWatch) is an iOS and watchOS app. This
repository explores moving continuous radio capture and local alerting onto always-on, mains-powered
hardware while keeping the phone as a future HealthKit bridge and viewer.

> [!WARNING]
> **Not a medical device.** This is an experimental personal project. It has not been clinically
> validated and must not be relied on to diagnose, prevent, detect, or treat any medical condition.
> It is not a substitute for professional care, approved monitoring equipment, or emergency
> services. Physiological decoding and alarm logic may be incomplete, wrong, or absent.

## Status

**Under active development.** The end-to-end sensor-to-database-to-detector-to-alarm system is not
complete. The repository currently contains:

- generated, strongly typed source and vital-sign registries;
- pure `no_std` BLE decoders with fuzz targets;
- PostgreSQL binary `COPY` encoding, spool primitives, and restart-safe session identity;
- the `vitalsd` Unix-socket protocol and single-writer daemon;
- Polar H10 and EMAY capture spikes for hardware validation; and
- Sqitch definitions for the accepted PostgreSQL/TimescaleDB schema.

Not yet complete are the production database deployment and verified restore, spool drain, production
collector fleet, accepted overnight capture, CNS detector, klaxon path, and admin console. The
[task index](openspec/changes/pi5-vitals-core/tasks.md) is the source of truth for implementation
status; the [amendment index](docs/specs/AMENDMENTS.md) is the source of truth for decisions that
supersede older design prose.

## Design invariants

These are requirements of the target system, not claims that every component is implemented:

- **Unknown is not healthy.** Missing, stale, or invalid data never reassures or escalates by itself.
- **Reject; never clamp.** Out-of-range physiological values are rejected with a named reason.
- **One canonical unit per type.** Conversion happens once at the decoder boundary.
- **Append-only samples.** Corrections add records or tombstones; they do not rewrite history.
- **Stable ingest identity.** `(source, type, session_id, seq)` identifies a sample; arrival time does
  not.
- **No network or container dependency in the alert path.** Convenience services may restart without
  sitting between a sample and an alarm output.
- **Hardware must fail visibly.** The target alert design includes supervised native services,
  current-sensed outputs, and a conditional hardware dead-man rather than trusting a successful
  software write.

`vitals/registry.toml` is the single source of truth for sources, types, canonical units, and valid
ranges. It generates Rust enums, SQL reference data, and JSON; source and type labels are not
duplicated by hand.

## Target architecture

The target topology deliberately separates the native safety path from restartable conveniences:

```text
systemd — native Rust services, hardware watchdog
  collectors ──▶ vitalsd ──▶ cns-engine ──▶ klaxond
                    │
                    ▼
  PostgreSQL 16 + TimescaleDB — bare metal, NVMe, pgBackRest
                    ▲
  containers — analysis and other non-critical conveniences
```

Only `vitalsd`, the protocol, storage primitives, and hardware-validation spikes shown above are
currently landed. See the [stage deliverables](docs/STAGE-DELIVERABLES.md) for the intended sequence.

## Why sample identity is not time

Several RR intervals can arrive in one BLE notification. Using host arrival time as a key can either
collapse distinct measurements or admit retransmissions as new data. The accepted schema instead
uses a source-specific sequence that is stable across retransmission, plus a session identifier that
changes across discontinuities and process restarts. Event time remains UTC data, not identity.

The committed schema and its deploy/revert/verify scripts live in [`db/`](db/). Storage is
append-only: collectors send samples to the single-writer `vitalsd`, which spools rows in PostgreSQL
binary `COPY` format before the database drain handles them.

## Validate the repository

The same core checks run in CI:

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python3 vitals/tools/gen_registry.py --check
cargo llvm-cov --workspace --all-features \
  --fail-under-file-lines 90 \
  --fail-under-lines 95 --summary-only
```

Coverage is a per-source contract: every major Rust module/source target reported by
`cargo llvm-cov`, including binaries and hardware-boundary seams, must reach at least 90% line
coverage, while the workspace aggregate must reach at least 95%. CI must not exclude binaries or
hide code with coverage exclusions to satisfy that floor. Every decoder that parses radio bytes
also gets a `cargo-fuzz` target. Linux-specific integration work runs against real BlueZ,
PostgreSQL, systemd, and radios on the Pi rather than macOS mocks; hardware acceptance supplements,
not waives, the source-file coverage contract.

## Repository map

| Path | Purpose |
|---|---|
| [`crates/`](crates/) | Registry types, pure decoders, COPY/spool core, socket protocol, daemon, and capture spikes |
| [`vitals/`](vitals/) | Registry source plus generated Rust, SQL, and JSON artifacts |
| [`db/`](db/) | Sqitch deploy, revert, and verify migrations |
| [`fuzz/`](fuzz/) | `cargo-fuzz` targets for hostile radio input |
| [`deploy/`](deploy/) | Native service definitions |
| [`scripts/`](scripts/) | Host synchronization, hooks, and development helpers |
| [`docs/specs/`](docs/specs/) | Architecture decisions and accepted amendments |
| [`docs/design/`](docs/design/) | Console, panel, and architecture research |
| [`openspec/changes/pi5-vitals-core/`](openspec/changes/pi5-vitals-core/) | Current proposal, design, task contracts, and runbook |

## Target hardware

Raspberry Pi 5 (8 GB) · NVMe storage · 7-inch DSI display · Sena UD100-G03 external Bluetooth
adapter · Polar H10 · EMAY SleepO2 · A&D UA-651BLE blood-pressure monitor · supervised audio and
physical alert outputs

Hardware acquisition and validation are ongoing; this list is the target/test setup, not a claim
that every path is operational. See the [hardware recommendations](docs/hardware/HARDWARE_RECOMMENDATIONS.md)
and [purchase plan](docs/hardware/PURCHASE-PLAN.md).

## Documentation

- [Accepted amendments](docs/specs/AMENDMENTS.md) — current decisions; these override older prose.
- [Stage deliverables](docs/STAGE-DELIVERABLES.md) — what each stage is intended to produce.
- [Current task contracts](openspec/changes/pi5-vitals-core/tasks.md) — checkboxes, reads, and exact
  verification commands.
- [Admin-console specification](docs/design/SPEC-admin-console.md) — liveness, operations, and
  implementation milestones.
- [Contributor/agent rules](AGENTS.md) — scope, safety invariants, Git workflow, and acceptance rules.

## Licence

[MIT](LICENSE)
