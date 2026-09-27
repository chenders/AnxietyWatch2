# Design — XE125 respiratory-rate sensing

**Status:** Accepted for implementation by explicit user instruction (auto-approved; no separate
approval gate).

## Context

`xe125/` contains an Acconeer XE125 PB41 V1.3 evaluation board with an XM125 module. The module
contains an A121 57–64 GHz pulsed-coherent radar and an STM32L4 MCU. Acconeer's
`acconeer-python-exploration` repository already provides a supported A121 breathing reference
application, but AnxietyWatch2 has no source/type, capture wrapper, quality contract, or durable
collector for it.

The existing project has a Rust `vitalsd` single-writer path and a generated registry. The accepted
amendments require canonical units at the decoder boundary, append-only provenance, stable source
identity, raw evidence for recoverability, and a strict distinction between unknown and healthy.
The first milestone must therefore be a reversible bench tool, not an unvalidated alarm source. Under
Amendment 01, the collector-to-spool path remains `collector -> libvitals -> vitalsd -> COPY spool`;
`vitalsd` remains the single writer and the collector must not open the spool independently.

## Goals / Non-Goals

**Goals:**

- Make the board usable immediately over USB serial through the vendor-supported A121 reference
  application.
- Emit state-preserving JSON Lines suitable for operator inspection and later replay.
- Add the registry foundation for a future `respiratory_rate` sample from `acconeer_xe125`.
- Define the boundary at which a future Rust collector can publish to `vitalsd` without using host
  arrival time as identity.
- Preserve raw recordings and validation metadata.

**Non-Goals:**

- Making Python part of the production or life-safety path.
- Reimplementing the Acconeer serial protocol before the module firmware and protocol version are
  verified on the Pi.
- Claiming apnea detection, ventilation, oxygenation, tidal volume, or clinical accuracy.
- Promoting radar output into CNS escalation or clearing in this change.
- Choosing a final bedroom enclosure, lens, or mechanical mounting design without bench evidence.

## Decisions

### 1. Use the upstream reference application for the first hardware milestone

The initial bench runner will call `acconeer-exptool` and its A121 `RefApp` rather than duplicating
Acconeer's module protocol. This is the shortest route to real frames and the documented breathing
algorithm. The runner is explicitly bench-only and produces evidence, not durable samples.

**Alternative rejected:** implement the serial protocol from scratch first. The protocol is tightly
coupled to module firmware and doing so before a live board session would create a large, untestable
surface.

### 2. Keep the vendor algorithm and the production pipeline separate

The first output records Acconeer's app state, range, presence scores, and rate. A future portable
Rust processing crate may consume recorded frames or a transport-neutral frame type. The eventual
Linux collector will publish through `vitalsd`; it will not write the spool directly. This is the
Amendment 01 single-writer path, not a return to SQLite: `libvitals` owns COPY encoding, `vitalsd`
owns spool writes, and the drainer owns PostgreSQL COPY ingestion.

**Alternative rejected:** treat one displayed floating-point number as a complete vital-sign sample.
That loses whether the algorithm saw a person, detected motion, was initializing, or had no valid
rate.

### 3. Add a generated registry type and source now, but do not promote them

`respiratory_rate` receives permanent type id 9, unit `breaths_per_minute`, engineering range
1.0–120.0. `acconeer_xe125` receives permanent source id 4 and transport `usb_serial`. These IDs
are provenance labels, not a claim that the source is already clinically accepted. The range is a
reject/quarantine boundary, not a normal-vitals threshold.

### 4. Bench output is JSONL; durable identity is deferred

Each line contains an observation timestamp for display, app state, presence context, range-point
selection, and an optional rate. The timestamp is not an identity. The production collector must use
a retransmission-stable module frame counter/tick or refuse durable ingestion until one is proven.
A process-local frame counter is diagnostic only and must not become `session_id`/`seq`.

### 5. Raw recording is optional but first-class

The runner exposes Acconeer's H5 recording facility. A requested recording is written in parallel
with processing and can be used to reproduce signal-processing decisions. Production raw journaling
will be designed separately for any source whose loss is unrecoverable.

### 6. Placement is a test matrix, not a hidden assumption

The first preset targets one stationary person at 0.5–1.0 m, with the A121 face aimed at the upper
chest/abdomen and the board rigidly mounted. Later validation must cover overhead and side mounts,
supine and side sleeping, bedding, partner/other motion, sensor obstruction, and loss of presence.
A missing result is always unknown.

## Risks / Trade-offs

- **Vendor dependency in the bench tool** → isolate it under `tools/`; do not import it from Rust or
  systemd services.
- **Firmware/protocol mismatch** → require explicit compatible firmware context and fail loudly on
  connection or sensor-version errors.
- **Spectral peak caused by movement or clutter** → preserve app states and raw recordings; gate
  durable ingest on state and validation.
- **No stable frame identity** → keep bench output non-durable until the module protocol exposes a
  retransmission-stable identity.
- **Multiple people or bedroom motion** → validate one-person placement first and report presence
  ambiguity rather than selecting a number silently.
- **Clinical overinterpretation** → keep radar outside the alarm path and state that it measures
  chest motion/frequency, not airflow or oxygenation.

## Migration Plan

1. Add the registry entries and generate the Rust/SQL/JSON artifacts.
2. Add and test the bench JSONL runner; use it with a compatible XM125 firmware on the Pi or laptop.
3. Collect raw sessions and placement/reference measurements.
4. After the protocol and identity are verified, add a transport-neutral Rust frame model and
   production collector in a separate task.
5. Add database/service wiring only after the source passes validation; rollback is removal/disable of
   the collector while preserving the raw evidence and registry IDs.

No database migration or alert-policy change is included in the initial milestone.

## Open Questions

- Which Acconeer module firmware/SDK version will be pinned for the Pi deployment?
- Does the selected XM125 firmware expose a frame counter or module tick that is stable across
  retransmission and restart, or must the production collector remain non-durable until a firmware
  change supplies one?
- Should production processing run on the Pi host or inside an XM125 embedded application after
  the reference algorithm is validated?
- What independent respiratory reference and acceptance error bounds will the project owner choose?
- Which overhead/side placement and radome/lens geometry works with the intended bed and bedding?
