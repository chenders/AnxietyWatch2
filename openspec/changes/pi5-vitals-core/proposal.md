# Proposal — Pi 5 Vitals Core (Stages 0–3)

## Why

The iPhone is the wrong hardware for two jobs this system depends on: holding BLE connections
continuously, and sounding a reliable alarm. Both fail for the same reason — it runs on a battery,
and that battery is at 71.5% health. The current branch `fix/battery-drain-remediation-all` is the
symptom of the first; speaker throttling under voltage sag is the second.

A Raspberry Pi 5 on mains power with an external antenna and a real speaker has neither problem.

## What

Relocate the **always-on radio** and the **always-on alarm** onto the Pi, and make the Pi the single
system of record — replacing gen1-host. The phone keeps HealthKit access, journaling, and viewing.

This change covers **Stages 0–3 only**. It is complete when a blood pressure reading taken on the
A&D UA-651BLE lands in Postgres on the Pi with correct provenance, recoverable from a verified
backup.

### In scope

| Stage | Deliverable |
|---|---|
| 0 | Hardware floor: NVMe root, audio out, Sena dongle, watchdog, governor |
| 1 | Thesis spike: 8-hour uninterrupted Polar H10 capture with a measured gap histogram |
| 2 | Storage floor: Postgres 16 bare metal on NVMe, pgBackRest, verified restore |
| 3 | `vitals` core: registry codegen, decoder crates, `vitalsd`, spool drain, `bp-collector` |

### Out of scope (follow-on changes)

Stage 4 BLE fleet · Stage 5 detector + klaxon · Stage 6 gen1-host migration ·
Stage 7 thin the phone · Stage 8 bedside panel

## Design reference

Full architecture, rationale, and the measured hardware baseline:
`docs/superpowers/specs/2026-08-03-pi5-vitals-core-design.md`

Two documents it amends:

- `docs/redesign_spec.md` §1.7 — inverted. `samples` may now hold HR/HRV/respiratory rate. The Pi
  reads RR intervals off the Polar H10 directly; HealthKit becomes one source among many.
- `CLAUDE.md` Design Principle #1 ("HealthKit is the source of truth") — demoted accordingly.

## Success criteria

1. `/` runs from NVMe; microSD holds `/boot/firmware` only.
2. A Polar H10 capture runs 8 hours with a recorded drop count and gap histogram.
3. `pgbackrest check` passes and the monthly restore-verification timer has completed once green.
4. A BP measurement taken on the cuff appears in `samples` within 60 s of the cuff being in range,
   with `source = ad_ua651ble` and values in canonical units.
5. `cargo test --workspace` passes, including the `0x2A35` decoder fuzz target at 60 s with zero crashes.

## Non-goals

- Rewriting the iOS app. It is untouched by this change.
- Any alarm behavior. `klaxond` is Stage 5.
- Migrating gen1-host's forty tables. Stage 6.
