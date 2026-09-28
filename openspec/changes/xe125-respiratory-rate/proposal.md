# Proposal — XE125 respiratory-rate sensing

**Status:** Accepted for implementation by explicit user instruction (auto-approved; no separate
approval gate).

## Why

An Acconeer XE125/XM125 evaluation board is now available and contains an A121 60 GHz radar with
an official breathing reference application. AnxietyWatch2 needs a software path that can evaluate
that signal safely, preserve the distinction between a real estimate and missing evidence, and only
promote it into the physiological pipeline after validation against a respiratory reference.

## What Changes

- Add a registry entry for the `respiratory_rate` measurement type and the `acconeer_xe125` source.
- Add a bench-only host runner around Acconeer's supported A121/XM125 Exploration Tool path so the
  board can be exercised immediately over USB serial and its raw sessions can be replayed.
- Preserve the Acconeer breathing application state, presence state, range, and quality context
  alongside every emitted estimate; never turn no-presence or motion states into a numeric zero.
- Define a portable Rust processing/ingestion boundary for a future production collector. The
  production path must not depend on Python or host arrival time for sample identity.
- Add raw evidence, validation, placement, failure, and promotion gates before radar output can
  affect the CNS detector or an alarm.

## Capabilities

### New Capabilities

- `xe125-respiratory-rate`: Bench capture, provenance-aware respiratory-rate estimates, validation,
  and the eventual Rust collector path for the Acconeer XE125/XM125.

### Modified Capabilities

None. Existing alerting and detector requirements are not changed by this proposal; radar remains
non-primary until the validation gate is explicitly passed.

## Impact

- `vitals/registry.toml` and generated registry artifacts gain one type and one source.
- New bench tooling lives under `tools/` and uses the upstream `acconeer-exptool` package only when
  invoked; it is not placed on the safety path.
- Future Linux integration will add an XE125 collector and service that publishes through
  `vitalsd`, plus raw-frame journaling and validation metadata.
- No database schema or alarm policy changes are made in the initial bench milestone.
