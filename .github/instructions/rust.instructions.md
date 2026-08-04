---
applyTo: "**/*.rs"
---

# Rust review rules — AnxietyWatch2

This code runs unattended all night and is the only thing standing between a desaturation and an
alarm. Review it that way.

## Radio decoders — the highest-risk code here

Any function parsing bytes off Bluetooth (`0x2A35` blood pressure, `0x2A37` heart rate, EMAY frames,
Oura frames) sees **attacker-controlled input**. A malformed advertisement or truncated GATT
indication is not hypothetical — it is what BlueZ hands you when a device browns out mid-packet.

Required shape, flag any deviation:

```rust
pub fn decode_x(buf: &[u8]) -> Option<Measurement>;
```

- **Pure.** No allocation, no I/O, no global or static state, no logging.
- **`get()` / `get(a..b)`, never `buf[i]` or `buf[a..b]`.** `indexing_slicing` is denied; an index
  that panics in a collector takes the collector down and the alarm with it.
- **Length checks before each optional field**, not just once at the top. Optional-field layouts mean
  the required length depends on the flags byte.
- **Returns `None` on malformed input.** `Option`/`Result` rather than a `bool` plus an out-parameter
  is the whole reason this project is in Rust — a caller cannot read a value that is not there.
- **Has a `cargo-fuzz` target** in `fuzz/fuzz_targets/`. A new decoder without one is incomplete.
- Decoder crates are `#![no_std]` and `#![forbid(unsafe_code)]`. Flag any `std` dependency that
  creeps in, and any `unsafe` block anywhere in the workspace.

## Numeric hazards

**IEEE-11073 SFLOAT is not IEEE-754 half-float.** 4-bit signed exponent in the high nibble, 12-bit
signed mantissa below. Flag any `f16` cast, `from_bits`, `transmute`, or byte reinterpretation — it
produces plausible-looking wrong numbers, and 118/74 versus 141/92 are both believable.

**Sign-extension.** 4-bit and 12-bit signed fields must be explicitly sign-extended (`<< 4 >> 4` at
the right width, or an equivalent). Flag arithmetic on a raw masked field without it.

**Reserved sentinels are absent, not zero.** `0x07FF` NaN, `0x0800` NRes, `0x07FE`/`0x0802` ±INF,
`0x0801` Reserved. Flag any path where one becomes `Some(value)`.

**Never compare floats with `==`.** Tests use `(a - b).abs() < 1e-9`. Flag exact equality on `f64`.

**Integer overflow wraps in release** unless `overflow-checks` is on. Flag arithmetic on
wire-derived values that could overflow; prefer `checked_*` / `saturating_*` and say which you meant.

**`as` casts are lint-denied** (`cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`).
Flag an `as` added to silence a lint rather than to state an intent, and any
`#[allow(clippy::cast_*)]` with no comment justifying it.

## Panics are the memory-safety equivalent here

Rust removed the buffer overrun. The remaining way to kill a collector is to panic.

- Flag `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` in non-test code.
  `unwrap_used` and `panic` are denied — an `#[allow]` needs a written reason.
- Flag slicing, indexing, integer division and `%` on any value derived from the wire.
- `panic = "abort"` in release: a panic is not recoverable, it is a process death.
- Decoders should carry a `kani` proof harness for panic-freedom where practical (amendment 04 §11);
  fuzzing approximates what kani proves.

## Concurrency

`vitalsd` is a **single-writer, single-threaded `tokio` runtime**. That is the design, and it is why
there is no locking. Flag:

- `tokio::spawn` of work that touches store state (use `spawn_local` on the `LocalSet`).
- A multi-threaded runtime flavor in the daemon.
- Any shared mutable state introduced between collectors and the store.
- Any assumption that two collectors can write the database directly — they append through the
  socket; `vitalsd` alone touches PostgreSQL.
- `.await` while holding a lock or an open transaction.
- **Blocking calls in async context** — `std::fs`, `std::net`, a contended `std::sync::Mutex`. One
  blocking call on a current-thread runtime stalls every collector at once.

## Crate boundaries

| Crate | Contains | Constraint |
|---|---|---|
| `vitals-registry` | generated types and sources | `no_std`, no dependencies |
| `vitals-decode` | pure decoders | `no_std`, `forbid(unsafe_code)`, no I/O |
| `vitals-core` | shared value types | no BlueZ, no SQL, no systemd |
| `vitalsd`, collectors | Linux-only: BlueZ, tokio, postgres, systemd | Pi only |

Flag any BlueZ, systemd, or SQL dependency reaching `vitals-decode` or `vitals-registry`. Flag a
mocked BlueZ used as test evidence — a mocked radio tests the mock.

## Storage

- No `UPDATE` or `DELETE` against `samples`. Append-only; removal is a tombstone.
- `INSERT ... ON CONFLICT DO NOTHING` so append is idempotent on
  **`(source, type, session_id, seq)`** — not on `timestamp`. Flag any code or comment still keying
  identity on time (amendment 05 §1).
- Range validation against the registry happens **before** insert. Out-of-range goes to quarantine
  with a reason, never clamped, never dropped silently.
- Flag any SQL built by string concatenation. Use `sqlx` with bound parameters and compile-time
  checking.
- **Binary COPY freezes column count and order.** Flag any change to the spool row layout not paired
  with a schema-fingerprint bump and a drained-spool guard.

## Time

- `timestamp` is `TIMESTAMPTZ`, an absolute instant; the cluster is pinned to UTC. Flag
  `chrono::Local`, naive datetimes crossing a boundary, or any timezone conversion below
  presentation.
- Flag monotonic and wall clocks being mixed. Durations use `Instant`; event times use the wire value
  or a documented estimate.
- **`timestamp` is not identity.** It may be an estimate and may be revised. Flag any dedup, cursor,
  or primary-key logic depending on it.

## Error handling

- Flag `let _ = ...` on a fallible call, and any `.ok()` discarding an error silently.
- Flag a caught-and-logged error that then proceeds as though it succeeded.
- Flag `debug_assert!` validating external input — it compiles out in release. Validate and return
  an error.
- **A guard that blocks a user-visible action must name its blocker.** Flag a `bool` return where the
  caller cannot say *which* condition failed, and any `Result<_, ()>`.
- Prefer typed errors (`thiserror`) at library boundaries; `anyhow` is fine inside binaries.

## Style

`rustfmt` handles layout; do not flag it. Do flag:

- Comments explaining *what* rather than *why*.
- A doc comment that no longer matches the code — especially after a threshold, filter, or sort order
  changes.
- Magic numbers used in two or more places. They belong in `vitals/registry.toml` and reach Rust
  through `vitals-registry`'s generated enums.
- A source label as a string literal (`"polar_h10"`). Use the generated enum variant.
