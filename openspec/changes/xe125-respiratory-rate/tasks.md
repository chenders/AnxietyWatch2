# Tasks — XE125 respiratory-rate sensing

**Status:** Spec accepted for implementation by explicit user instruction. The first two tasks are
being started immediately; no radar sample is promoted to the alarm path by this change.

**Amends:** `docs/specs/AMENDMENTS.md` (mandatory), amendment 11's ten-minute hardware-check ceiling; no accepted amendment is changed.

## Task index

- [x] 1.1 Registry type and source
- [x] 1.2 Bench JSONL capture runner
- [ ] 2.1 Live Pi acquisition and firmware pin
- [ ] 2.2 Placement/reference capture matrix
- [ ] 3.1 Transport-neutral Rust frame model
- [ ] 3.2 Rust production collector and `vitalsd` publication
- [ ] 4.1 Validation gate and optional alert promotion

## 1. Bench foundation

### T-X1 — Registry type and source

**Tier:** implement-standard · **Host:** either · **Context budget:** ~3k · **Depends:** none
**Review required:** no

**Reads:** `docs/specs/AMENDMENTS.md`, `vitals/registry.toml`, `vitals/tools/gen_registry.py`,
`crates/vitals-registry/src/lib.rs`

**Creates:** generated registry artifacts through the existing generator.

**Modifies:** `vitals/registry.toml`, `crates/vitals-registry/src/lib.rs`

**Do:**

1. Add permanent type id 9: `respiratory_rate`, canonical unit `breaths_per_minute`, engineering
   range 1.0–120.0, precision 0.1.
2. Add permanent source id 4: `acconeer_xe125`, transport `usb_serial`, providing the new type.
3. Regenerate; never hand-edit generated files.
4. Pin the new IDs in registry tests.

**Verify:** `python3 vitals/tools/gen_registry.py --check && cargo test -p vitals-registry && cargo build -p vitals-registry`

**Expect:** exit 0; generated artifacts contain `RespiratoryRate` and `AcconeerXe125`.

**Commit:** `git add vitals/registry.toml crates/vitals-registry/src/lib.rs crates/vitals-registry/src/generated.rs vitals/generated && git commit -m "feat(T-X1): register XE125 respiratory rate"`

### T-X2 — Bench JSONL capture runner

**Tier:** implement-standard · **Host:** either for tests, Pi for live use · **Context budget:** ~5k
**Depends:** T-X1 · **Review required:** no

**Reads:** `docs/specs/AMENDMENTS.md`, `xe125/README.md`, `pyproject.toml`

**Creates:** `tools/xe125_breathing.py`, `tools/tests/test_xe125_breathing.py`

**Modifies:** none

**Do:**

1. Use the upstream `acconeer-exptool` A121 `RefApp`; do not duplicate its serial protocol.
2. Require an explicit serial port, configure the documented breathing reference defaults, and
   optionally attach an H5 recorder.
3. Emit one flushed JSON object per frame containing observation time, app state, presence context,
   selected range points, and an optional rate. Preserve absent rate as JSON `null`.
4. Exit non-zero for connection/setup failures and make the runtime dependency error actionable.
5. Keep the tool bench-only; it MUST NOT append directly to the spool or claim durable identity.
6. Unit-test serialization and absent/non-finite values without importing the vendor package.

**Verify:** `ruff check tools/xe125_breathing.py tools/tests/test_xe125_breathing.py && python3 -m pytest tools/tests/test_xe125_breathing.py && python3 -m py_compile tools/xe125_breathing.py`

**Expect:** exit 0; tests cover measured, absent, motion, and invalid-rate output.

**Commit:** `git add tools/xe125_breathing.py tools/tests/test_xe125_breathing.py && git commit -m "feat(T-X2): add XE125 breathing bench runner"`

## 2. Hardware and evidence

### T-X3 — Live Pi acquisition and firmware pin

**Tier:** human/implement-standard · **Host:** pi · **Context budget:** ~3k · **Depends:** T-X2
**Review required:** no

**Reads:** `docs/specs/AMENDMENTS.md`, `xe125/README.md`,
`openspec/changes/xe125-respiratory-rate/design.md`

**Creates:** a recorded firmware/version note and raw session outside the repository's source tree.

**Modifies:** the implementation notes for this change when evidence exists.

**Do:** connect the board over USB1, identify the Enhanced port, flash and record a compatible
XM125/A121 module firmware version, run a capture of no more than ten minutes, and preserve at least
one H5 recording.

**Verify:** a Pi command that proves the Enhanced port exists, the session produced JSONL frames, and
the requested raw recording is non-empty.

**Expect:** exit 0; a failed connection names the blocker instead of being accepted as no breathing.

### T-X4 — Placement and reference capture matrix

**Tier:** human · **Host:** pi · **Context budget:** ~4k · **Depends:** T-X3
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `xe125/README.md`,
`openspec/changes/xe125-respiratory-rate/specs/xe125-respiratory-rate/spec.md`

**Creates:** raw captures and a measured placement/reference report.

**Modifies:** the implementation notes for this change when evidence exists.

**Do:** measure one stationary person at 0.5–1 m using overhead and side placements, include supine
and side positions, bedding, deliberate movement, no-presence, and a respiratory reference. Bound
each individual hardware capture to ten minutes. Record which observations are valid and which are
unknown.

**Verify:** a report-check command that requires every matrix case, a reference value, and explicit
unknown/motion counts.

**Expect:** exit 0 only when the matrix is complete; no numeric result is accepted for a no-presence
or motion case.

## 3. Production Rust path

### T-X5 — Transport-neutral Rust frame model

**Tier:** implement-complex · **Host:** either · **Context budget:** ~7k · **Depends:** T-X3
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`,
`openspec/changes/xe125-respiratory-rate/specs/xe125-respiratory-rate/spec.md`,
`openspec/changes/xe125-respiratory-rate/design.md`, `crates/vitals-core/src/lib.rs`,
`crates/vitals-proto/src/lib.rs`

**Creates:** a tested Rust module/crate for A121 frame/state normalization and quality decisions;
add a fuzz target if it parses raw bytes.

**Modifies:** workspace membership only if the design proves a new crate is necessary.

**Do:** define owned I/Q/frame metadata without host-time identity, represent the four app states,
reject non-finite/out-of-range rates, and keep the module independent of BlueZ, SQL, and systemd.

**Verify:** the crate's unit, clippy, and fuzz/Kani commands, with zero warnings and no panic paths.

**Expect:** exit 0; all invalid and unknown states are tested.

### T-X6 — Rust production collector and `vitalsd` publication

**Tier:** implement-complex · **Host:** pi · **Context budget:** ~8k · **Depends:** T-X5 and a
stable module identity · **Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`,
`openspec/changes/xe125-respiratory-rate/specs/xe125-respiratory-rate/spec.md`,
`openspec/changes/xe125-respiratory-rate/design.md`, `crates/vitalsd/src/main.rs`,
`crates/vitals-proto/src/lib.rs`, `crates/vitals-core/src/lib.rs`, `vitals/registry.toml`

**Creates:** a Linux-only Rust collector, tests, raw-frame journal wiring, and a service definition.

**Modifies:** `vitalsd` primary-source configuration only in a separately reviewed promotion step.

**Do:** use the verified module transport, obtain a retransmission-stable frame identity or refuse
durable ingestion, publish only valid measured rates through `vitalsd`, retain quality/provenance,
and leave the alarm path disabled.

**Verify:** Pi build/test/clippy plus a live capture command proving valid samples and explicit
unknown states reach the socket without using arrival time as identity.

**Expect:** exit 0; a missing identity or invalid rate is visibly quarantined.

## 4. Promotion

### T-X7 — Validation gate and optional alert promotion

**Tier:** review/human · **Host:** either/pi · **Context budget:** ~5k · **Depends:** T-X4 and T-X6
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`,
`openspec/changes/xe125-respiratory-rate/specs/xe125-respiratory-rate/spec.md`,
`openspec/changes/xe125-respiratory-rate/design.md`, validation reports

**Creates:** an accepted validation report or a permanent decision to keep the source experimental.

**Modifies:** alerting configuration only after explicit acceptance.

**Do:** review accuracy, gaps, false positives/negatives, motion behavior, placement, and failure
states. Promote the source only with a separate explicit alert-policy decision.

**Verify:** a report gate that fails unless all required evidence and owner acceptance are present.

**Expect:** exit 0 only for an explicit decision; otherwise the radar remains record-only.

## Implementation notes

### T-X1 — passed 2026-09-01

- Added `VitalsType::RespiratoryRate` as type id 9 with canonical unit
  `breaths_per_minute` and engineering range 1.0–120.0.
- Added `VitalsSource::AcconeerXe125` as source id 4 with `usb_serial` transport.
- Regenerated all registry artifacts; generated files remain source-controlled outputs only.
- Verify passed: `python3 vitals/tools/gen_registry.py --check && cargo test -p vitals-registry &&
  cargo build -p vitals-registry`.

### T-X2 — passed 2026-09-01

- Added `tools/xe125_breathing.py`, a bench-only wrapper around Acconeer's A121 `RefApp`.
- The runner emits flushed JSONL with state, presence/range context, optional rate, and no durable
  sample identity; it can optionally save an H5 raw session.
- Added four vendor-independent serialization tests. The optional Acconeer dependency is imported
  only when a live capture starts.
- Verify passed: `ruff check tools/xe125_breathing.py tools/tests/test_xe125_breathing.py &&
  python3 -m pytest tools/tests/test_xe125_breathing.py && python3 -m py_compile
  tools/xe125_breathing.py`.

Live board acquisition remains T-X3 and has not been claimed or simulated.
