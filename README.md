# AnxietyWatch2

**A local-first Raspberry Pi physiological monitor that can keep watch all night—and eventually sound an alarm when a phone cannot.**

AnxietyWatch2 is a local-first physiological monitoring project in Rust. A Raspberry Pi 5 on the nightstand captures body-worn Bluetooth Low Energy (BLE) sensor data, preserves where every sample came from, and is being built toward the detection of nocturnal central-nervous-system (CNS) depression and an alarm that does not depend on a phone battery or the house internet. 

Between a medication warning label about breathing and a clinical sleep study months away, American healthcare often leaves a gap: nobody is watching. This project explores how to close that gap at home, in the open, with the receipts. It is designed around the concept of continuous individual monitoring rather than population-scale data collection.

> [!WARNING]
> **Not a medical device.** This is an experimental open-source project. It has not been clinically validated and must not be relied on to diagnose, prevent, detect, or treat any medical condition. It is not a substitute for professional care, approved monitoring equipment, or emergency services. Physiological decoding and alarm logic may be incomplete, wrong, or absent.
>
> **The alarm does not exist yet.** If overnight breathing or medication safety concerns you, talk to a clinician and use validated monitoring—not this repository.

## Should I use this?

**Not as a working monitor today.** There is no supported installer, the production collectors are incomplete, and neither the detection engine nor the physical alarm exists. If you are looking for a system to install and trust tonight, this is not it. 

Instead: **Read it, audit it, or reuse its engineering.** 
The most transferable materials are the [accepted architecture amendments](docs/specs/AMENDMENTS.md), the [alarm-timing reassessment](docs/research/klaxon-alarm-timing-reassessment.md), the hardware protocol notes for the [Polar H10](docs/hardware/POLAR_H10.md) and [EMAY SleepO2](docs/hardware/EMAY_OXIMETER.md), and the project's rigorous approach to safety invariants.

## Status: what is real, and what is still a design

**Under active development. Eleven of the fifteen Stage 0–3 tasks are complete.** The next software task is `vitals-spool-drain` (T-D5). Follow the [task index](openspec/changes/pi5-vitals-core/tasks.md) for acceptance evidence and the [amendment index](docs/specs/AMENDMENTS.md) for current decisions.

### Built and tested
- Generated, strongly typed registries for eight vital types and five sources.
- Pure `no_std` decoders for IEEE-11073 SFLOAT, BLE heart-rate/RR, and EMAY SleepO2, with rigorous fuzz targets.
- PostgreSQL binary `COPY` encoding, an append-only spool with schema fingerprints, and restart-safe session identity.
- `vitalsd`, its Unix-socket CBOR protocol, health surface, and single-writer ingest path.
- Sqitch schema definitions and encrypted pgBackRest backups with scheduled restore verification.

### Present, but pending deployment reconciliation
- The TimescaleDB hypertable, compression, one-minute aggregates, and raw-data retention are defined in migrations, but this chain is **not claimed as verified-deployed** while primary-key reconciliation remains open.

### Designed, not built
- `vitals-spool-drain` (T-D5): single-drainer locking, transactional `COPY` staging, live-schema fingerprint verification, disk-watermark degradation, and drain-liveness reporting. Blocked by G3.
- `bp-collector` and the production collector fleet.
- `cns-engine`: quality gate, personal baseline, fusion, and hysteretic tier machine.
- `klaxond`: supervised alarm channels and the hardware dead-man switch.
- The Rust HTTP API, admin console, and web-dashboard backend. (A bedside panel exists as a display-only hardware-validation stopgap, but it is not a detector and cannot wake anyone).

## The rule everything else serves

**Unknown is not healthy.** There are always three states: measured-and-good, measured-and-bad, and *not measured*. Missing or invalid data contributes nothing. It never reassures, never escalates by itself, and is never joined by an invented line.

![Illustrated overnight trace with measured-in-range values in teal, measured-out-of-range values in orange, and a hatched gap labelled not measured; a dashed line across the gap is crossed out because it would invent data.](docs/images/readme/three-states.png)

That rule reaches from bytes to bedside:
- **Reject; never clamp.** An out-of-range value is rejected with a named reason. A clamped value is indistinguishable from a real one downstream.
- **Absence is not zero.** Radio sentinels return no measurement, not a reassuring number.
- **Identity is not arrival time.** Several RR intervals can arrive in one notification, so samples are identified by `(source, type, session_id, seq)` rather than host timing.
- **Gaps remain gaps.** Physiological series are never interpolated or carried forward.
- **A viewer is never a dependency.** The target alarm path behaves the same with every screen unplugged.

## Clickable Trust: A decoder bug that waits for sleep

The repository distrusts plausible success. As an example, the EMAY SleepO2 checksum uses `sum(payload) & 0x7F`, not `& 0xFF`. Across the decoder's documented grid (pulse 25–250 bpm and SpO₂ 50–100), the masks disagree for **4,998 of 11,526 combinations (43%)**. 

![Pulse-rate by oxygen-saturation grid. Hatched red cells show valid EMAY frames rejected by an incorrect 0xFF checksum mask; the sleeping region at pulse 40–48 and SpO₂ 95–100 is entirely rejected while the awake captures fall where both masks agree.](docs/images/readme/checksum-mask-map.png)

A decoder using `0xFF` accepts the reference frame and the 27-frame awake capture. It then silently rejects every frame in the illustrated sleeping region as the wearer's pulse falls into the 40s. The correct mask came from decompiling the vendor app; no awake capture could have proved it. 

This is why this project demands receipts. Its CI mutation-tests gates that once reported green without compiling Rust. Every radio decoder gets a [fuzz target](fuzz/fuzz_targets/fuzz_emay.rs). A backup is not trusted merely because it was written: the project schedules a restore into a scratch cluster and checks the result.

## The Signal Path

The labels below are part of the diagram: the honesty boundary is implementation state, not visual polish.

```text
Polar H10 · EMAY SleepO2 · A&D BP cuff       CPAP · Oura · HealthKit
                │                                  planned imports
                ▼
collectors → pure decoders                  bp-collector and fleet: not built
                ▼
vitalsd → append-only COPY-binary spool     built
                ▼
vitals-spool-drain → PostgreSQL 16          drainer built; DB deployment reconciliation open
                ▼
cns-engine                                  designed, not built
quality gate → personal baseline → max-not-average fusion
→ clear · can't assess · watch · confirm · klaxon
                ▼
klaxond                                     designed, not built
GPIO piezo · bed shaker · speaker · Zigbee · push · haptic · hardware dead-man

viewers: bedside panel · web dashboard · admin console · phone
         never dependencies in the target alert path
```

The target radio-packet-to-tier-evaluation latency is p99 under two seconds. The alert path is native services under systemd: no network and no container between a sample and an alarm output. 

## Engineering Invariants

- **Storage has a spool:** Collectors do not write the database directly. They send samples to the single-writer `vitalsd`, which appends rows already encoded in PostgreSQL binary `COPY` format. `vitals-spool-drain` copies them transactionally.
- **Safety-path hardware:** Every alarm output must be provably OFF from power-on until `klaxond` takes control. GPIO floats before configuration, so a 10 kΩ gate-to-source pulldown is mandatory on every MOSFET.
- **OOM adjustments:** `OOMScoreAdjust=-1000` on the daemon and collectors. Without this, the kernel kills the alarm to save the database.

## Repository map

| Path | Purpose |
|---|---|
| [`crates/`](crates/) | Registry, decoders, spool core, socket protocol, `vitalsd`, drainer, capture tools |
| [`vitals/`](vitals/) | Registry source plus generated Rust, SQL, and JSON |
| [`db/`](db/) | Sqitch deploy, revert, and verify migrations |
| [`fuzz/`](fuzz/) | `cargo-fuzz` targets for hostile radio input |
| [`firmware/`](firmware/) | Zephyr/nRF52840 sensor emulators and scripted desaturation |
| [`deploy/`](deploy/) | Native service definitions, restore verification, stopgap panel |
| [`docs/specs/`](docs/specs/) | Accepted architecture amendments |
| [`docs/design/`](docs/design/) | Theory of operation and interface research |
| [`docs/research/`](docs/research/) | Baseline, alarm timing, and protocol evidence |
| [`openspec/changes/pi5-vitals-core/`](openspec/changes/pi5-vitals-core/) | Current task contracts and exact verification commands |

## Documentation

- [Theory of operation](docs/design/theory-of-operation/theory-of-operation.html) — the completed design, end to end.
- [Current task contracts](openspec/changes/pi5-vitals-core/tasks.md) — acceptance is each command's exit code.
- [Accepted amendments](docs/specs/AMENDMENTS.md) — current decisions; these override older prose.
- [Contributor and agent rules](AGENTS.md) — safety invariants, scope, tests, and workflow.

## Licence

[MIT](LICENSE)
