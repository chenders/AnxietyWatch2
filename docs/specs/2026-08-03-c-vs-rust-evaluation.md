# C vs Rust — evaluation

**Date:** 2026-08-03
**Status:** Recommendation — decide before T-D3
**Scope:** every component currently specified as C11

Written to be checkable, not persuasive. Claims are either verifiable facts about the ecosystem or
statements about this codebase specifically. Where the answer is "no meaningful difference," it says
so.

---

## 1. Ecosystem — does every C dependency have a mature Rust equivalent?

Yes, and two are official.

| Need | C | Rust | Status |
|---|---|---|---|
| BLE / GATT | BlueZ over D-Bus | [`bluer`](https://github.com/bluez/bluer) | **Official** — lives under the `bluez` org |
| Postgres binary `COPY` | libpq `PQputCopyData` | [`postgres` / `tokio-postgres`](https://docs.rs/tokio-postgres/) | `binary_copy` is a documented feature of the crate |
| systemd notify / watchdog | libsystemd | [`libsystemd`](https://crates.io/crates/libsystemd), [`sd-notify`](https://docs.rs/sd-notify) | Pure Rust, no C library needed. Packaged in Debian and **Raspbian** |
| Fuzzing | libFuzzer | [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) | Wraps libFuzzer — the same engine, same corpus format |
| ALSA output | libasound | `alsa` crate | Bindings, mature |
| CBOR wire format | hand-rolled | `ciborium` / `minicbor` | `minicbor` is `no_std`-capable |

No component is blocked. The fuzzing point matters: switching languages does not mean abandoning the
existing fuzz corpus or harness style — `cargo-fuzz` drives the same libFuzzer.

---

## 2. Safety — measured against this project's actual defects

Abstract memory-safety arguments are weak. Here is the concrete record.

**The decoders are already correct in C**, bounds-checked by hand and verified at 30M executions
under ASan+UBSan. Rust would not have found a bug there, because there is not one. What Rust changes
is the *default* for every decoder written after this one: `&[u8]` slicing is bounds-checked by the
compiler rather than by reviewer attention. The current C decoder is safe because I wrote every
check deliberately and a fuzzer confirms it. That is a process guarantee, not a structural one, and
this project's whole thesis is preferring structural.

**Where C's risk is not covered by fuzzing:** the collectors. Long-lived connection state machines,
reconnect backoff, per-device buffers, D-Bus callback lifetimes. Bugs there are stateful and emerge
over hours, which is exactly what a pure-function fuzzer does not reach.
[CVE-2026-46056](https://windowsnews.ai/article/cve-2026-46056-critical-linux-bluetooth-use-after-free-flaw-demands-immediate-patching.420382)
is a current use-after-free in Linux Bluetooth, one of a long line, and
[BlueZ has itself been rewritten in Rust](https://blitzy.com/blog/the-blitzy-bluez-building-bluetooth-support)
(380K LOC) for precisely this reason.

**Verdict:** meaningful advantage to Rust, concentrated in the collectors, marginal in the decoders.

---

## 3. The port argument — the strongest project-specific point

`cns-engine` is a **port of existing Swift code** (`Services/CNSRisk/`), deliberately written free of
CoreBluetooth, SwiftData and SwiftUI so it could move.

That Swift uses: enums with associated values, `Optional`, `Result`, exhaustive pattern matching,
and a type system where "absent" is a distinct thing from "a value."

| Swift construct | Rust | C |
|---|---|---|
| `enum Tier { case watch(since: Date) }` | `enum` with payload — direct | hand-lowered tagged union |
| `Optional<Double>` | `Option<f64>` — direct | sentinel value or an out-param + bool |
| `Result<T, E>` | `Result<T, E>` — direct | errno convention |
| exhaustive `switch` | `match`, compiler-enforced | `switch` + hope |

**The C column is where this project's core invariant gets lost.** "Invalid data contributes nothing,
never coerce absent to a number" is enforced by the *type system* in Swift and in Rust. In C it is
enforced by convention — a sentinel is a value, and nothing stops it being read as one. Lowering
Swift `Optional` into a C sentinel is precisely the transformation the design forbids, performed by
hand across the whole detection engine.

Swift → Rust is close to mechanical. Swift → C requires hand-lowering the exact constructs that
carry the safety property.

**Verdict:** strong advantage to Rust, and it is about correctness of the port rather than taste.

---

## 4. Parallelism

The current design is single-threaded by necessity, not by preference: one process per collector,
because sharing state across threads in C is where the bugs are. The processes exist to provide
isolation the language cannot.

In Rust, `Send`/`Sync` make shared state safe by construction. The four collectors could be one
supervised process with a task per radio, sharing reconnect logic, backoff policy, and a single
`vitalsd` connection — one systemd unit instead of four, and one place where "how do we reconnect"
is written.

That is a real architectural simplification the language enables. It is also optional: if
process-per-collector is preferred for blast-radius isolation, Rust does that identically.

**Verdict:** advantage to Rust, though the current design does not depend on it.

---

## 5. Contributors

v1 is public and this will be too. Two effects, in opposite directions, both real:

- **For Rust:** larger pool of people who *want* to work in it, and a contributor can be confident a
  PR will not introduce a use-after-free in a health-monitoring daemon. Rust has led Stack
  Overflow's most-admired-language ranking for roughly nine consecutive years.
- **For C:** the embedded-Linux world is C. BlueZ, systemd, ALSA and libpq are all C, so anyone from
  that background is immediately at home with no FFI boundary to reason about.

**Verdict:** modest advantage to Rust for a project of this kind, not decisive either way.

---

## 6. Honest case for C

Stated properly rather than as a strawman.

**Distro-maintained dependencies.** BlueZ, libsystemd and libasound are shipped and patched by
Debian. `bluer` and `alsa` are crates — a supply-chain surface you own if a maintainer walks away.
This is a genuine long-term risk that Rust's ecosystem does not remove.

**Compile times.** Rust is substantially slower to build than C, and this project builds on a Pi 5.
Mitigable (cross-compile, `sccache`), but real friction on every iteration.

**Debugging.** `gdb` on C is direct. Rust debugging is good, but async stack traces through `tokio`
are worse than a C call stack — and the collectors are where you will be debugging at 3am.

**Not closer to the metal.** `bluer` wraps D-Bus, which is the same abstraction the C path uses.
Neither choice gets nearer the radio.

**Things that are NOT arguments, and should be dropped:**

- *Binary size* — hundreds of KB versus tens, on an 8 GB machine with NVMe. Irrelevant.
- *Runtime overhead* — neither language has a GC. With `panic = "abort"`, a Rust panic and a C crash
  are the same event. `klaxond` gains nothing from C.
- *Startup latency* — both are static binaries starting in milliseconds.

---

## 7. Recommendation

**Rust for everything currently specified as C11.**

The case for C rested on three things. Toolchain simplicity is explicitly discounted. "Lowest level"
turns out not to be true — both paths go through D-Bus and libpq-equivalents. Runtime overhead does
not exist. What remains is dependency provenance and compile times, against memory safety in the
stateful code, a near-mechanical port of the Swift detection engine instead of a hand-lowering that
discards its type-level invariants, and safe shared-state concurrency.

**Component by component:**

| Component | Recommendation | Confidence |
|---|---|---|
| BLE collectors | **Rust** | High — this is where the bugs are and where fuzzing does not reach |
| `cns-engine` | **Rust** | High — the Swift port preserves its invariants instead of lowering them away |
| `vitalsd`, spool, COPY encoder | **Rust** | Medium — C is adequate; Rust is better and consistency has value |
| Wire decoders | **Rust** | Medium — existing C is correct and fuzzed, but every future decoder gets safety by default |
| `klaxond` | **Rust** with `panic = "abort"` | Medium — no measurable advantage either way; consistency decides it |

**Cost of switching now:** approximately 100 lines of decoder and its tests, already reviewed and
fuzzed. That is the entire sunk cost. After T-D3/D4/D5 it becomes thousands of lines and a rewrite.
This is the cheapest moment this decision will ever have.

**One caveat that is about the project, not the language:** the C written so far is verified working.
Rust would be written fresh. That risk is small at 100 lines and grows daily.

---

## 8. If the answer is Rust, what changes

- `vitals_core` / `vitals_linux` become one cargo workspace with `no_std`-capable decoder crates.
  The portable/Linux split survives unchanged as crate features.
- CMake stays for nothing, or stays only if a C component remains. `cargo` + `cargo-fuzz` + `cargo
  test` replace CMake + CTest + the libFuzzer wiring.
- Task briefs T-D2 through T-D6 need rewriting. T-D1 is unaffected — the generator emits a C header
  today and would emit a `.rs` module instead, which is a change to one emitter function.
- `.github/instructions/c.instructions.md` becomes `rust.instructions.md`. Most rules survive
  verbatim; the bounds-checking rules become "no `unsafe` without a comment justifying it, and
  `#![forbid(unsafe_code)]` in the decoder crates."
- Coverage tooling changes from lcov to `cargo-llvm-cov`. Gates stay at 95/85.
