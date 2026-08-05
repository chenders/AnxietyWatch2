# Language choices, and the candidates they beat

**Date:** 2026-08-03
**Status:** Proposed — the Rust recommendation in §2 needs a decision before T-D4

Every row states what it is used for, why, and what it was chosen *over*. Nothing here is inherited
from AnxietyWatch v1; where v1 made a choice, it is re-argued from scratch or reversed.

---

## 1. Summary

| Component | Language | Chosen over |
|---|---|---|
| Wire decoders (`0x2A35`, `0x2A37`, EMAY, Oura) | **C11** | Rust, Zig |
| `vitalsd`, `libvitals` COPY encoder, spool | **C11** | Rust, Go |
| `cns-engine` (detection fold) | **C11** | Rust, Swift |
| `klaxond` | **C11** | anything with a runtime |
| **BLE collectors** | **Rust** *(recommended change)* | C11 |
| Codegen, tooling | **Python 3.12** | C, Go, shell |
| HTTP API and admin | **Rust + axum** — superseded by [Amendment 07](2026-08-05-amendment-07-api-in-rust.md) | Python + FastAPI, Go |
| Analysis and batch importers | **Python 3.12** | Go, Rust |
| Bedside panel + web UI | **Vanilla HTML/CSS/JS** | React, Svelte |
| Database | **PostgreSQL 16 + TimescaleDB** | SQLite, InfluxDB, DuckDB |
| Build | **CMake** | meson, Bazel, plain Make |
| iOS bridge | **Swift** | nothing — HealthKit has no alternative |

---

## 2. C vs Rust — the one that is genuinely contested

The brief asked for "the lowest language possible (like C) when feasible, but use other technologies
as appropriate when it's pragmatic." Applying that honestly splits the C code in two.

### Where C is the right answer

**The decoders.** Highest-risk code in the project — they parse attacker-controlled radio bytes —
but also the easiest to make safe in any language, because they are pure
`(const uint8_t *, size_t) -> result` with no allocation, no I/O, and no state. The entire attack
surface is "does every read check `len` first," which is mechanically verifiable and continuously
fuzzed. `fuzz_sfloat` already executes 30M inputs in 60s under ASan+UBSan. Rust would remove a class
of bug that fuzzing already covers here, at the cost of a second toolchain.

**`vitalsd`, the spool, `klaxond`.** Single-threaded, no concurrency, tiny surface, and `klaxond`
specifically must work when everything else has failed. No allocator pressure, no runtime, no GC
pause, a static binary that starts in milliseconds under systemd.

**`cns-engine`.** A pure fold, `(state, sample) -> (state, [cmd])`. No I/O, no allocation, fully
deterministic and replayable. The language barely matters; C keeps it linkable into the same binary
as everything else on the safety path.

### Where C is the wrong answer

**The BLE collectors.** These are long-lived processes holding connection state machines, reconnect
backoff, per-device buffers, and D-Bus callback lifetimes — exactly the shape where use-after-free
and buffer-lifetime bugs live, and exactly where fuzzing does *not* reach, because the bugs are in
stateful interaction over hours rather than in a pure function over bytes.

The evidence is not theoretical: [CVE-2026-46056](https://windowsnews.ai/article/cve-2026-46056-critical-linux-bluetooth-use-after-free-flaw-demands-immediate-patching.420382)
is a use-after-free in Linux Bluetooth, and it is one of a long line. [BlueR](https://github.com/bluez/bluer)
is the official BlueZ Rust binding, and [BlueZ itself has been rewritten in Rust](https://blitzy.com/blog/the-blitzy-bluez-building-bluetooth-support)
(380K LOC) specifically for memory safety.

**Recommendation: write the collectors in Rust, keep everything else C11.**

The boundary is clean because it already exists: collectors talk to `vitalsd` over a Unix socket
with a length-prefixed CBOR protocol. They share no memory and no headers with the C side. A Rust
collector is a drop-in replacement for a C one — the protocol is the interface, not an ABI.

Cost: cargo alongside CMake, and a second toolchain on the Pi (`rustup`, ~400 MB). That is the whole
price, and it buys memory safety in the one place where the bugs are stateful, long-lived, and
unfuzzed.

**Zig** was considered and rejected: better than C here, but pre-1.0 with breaking releases, and a
life-safety daemon should not track a moving language.

**If the answer is "C everywhere,"** that is defensible — but then the collectors need bounded arena
allocation, no raw `malloc`/`free` in connection handling, `-fsanitize=address` in a long-running
soak build, and an explicit weekly ASan soak test as a gate. Say which and I will write it that way.

---

## 3. Python — analysis and tooling only

> **Superseded in part by [Amendment 07](2026-08-05-amendment-07-api-in-rust.md).** The HTTP API and
> admin backend are Rust with axum. The FastAPI rationale below is retained as the record of the
> superseded decision, not current guidance.

**Still used for:** codegen (`gen_registry.py`), analysis jobs, and the carved-out batch importers
such as EDF/CPAP parsing.

**Why Python at all:** none of this is on the life-safety path. It is I/O-bound, changes often, and
benefits from a large ecosystem. Rewriting the EDF parser in C would be weeks of work to make an
already-tested batch process marginally faster at something that runs once a day.

**FastAPI over Flask** — v1 used Flask, and that is precisely why it deserves re-argument:

- **Native async.** The API serves Server-Sent Events for live `vitalsd` subscriptions and long-poll
  job status. Flask needs an ASGI shim for this; FastAPI is ASGI natively.
- **Pydantic validation at the boundary.** Request and response models are declared once and
  validated automatically. That is the same "reject at the boundary, name the blocker" discipline
  the C side enforces by hand — a malformed request fails with a field-level error rather than
  reaching business logic as a partially-populated dict.
- **OpenAPI generated from the code.** The iOS client and the web dashboard both consume this API;
  a generated schema means a client and server cannot silently disagree.

**Rejected:** Go (a second compiled toolchain for something that is not performance-critical);
Rust (same, plus slower iteration on the part of the system that changes most).

**Modern Python practice, applied — not inherited:**

| | |
|---|---|
| `pathlib` | not `os.path.join` |
| `argparse` | not `sys.argv[1] == "--check"` |
| Type hints + `from __future__ import annotations` | not untyped dicts |
| Frozen `@dataclass(slots=True)` | not raw dicts threaded through functions |
| Exceptions raised, exit codes only at `main()` | not `sys.exit()` buried in helpers |
| `ruff` for lint + format | not flake8 + black + isort |
| `pytest` | not unittest |
| `uv` | not pip + venv by hand |

`gen_registry.py` was rewritten to all of the above after its first version shipped with a loop that
silently matched nothing.

---

## 4. Frontend — vanilla, deliberately

**Used for:** the 800×480 bedside panel, the web dashboard, the admin console.

**Why no framework:** the panel is a fixed-function status display that must start instantly on a Pi
and never fail to render. The dashboard is read-mostly. React or Svelte would add a build step, a
dependency tree with a supply-chain surface, and a bundle to load before anything appears — in
exchange for component ergonomics this UI does not need.

The bedside panel is already built and proves the point: one HTML file, no build, no dependencies,
opens in any browser.

**Reconsider if** the admin console's four operational patterns (long-running jobs, generate→review,
upload with validation, credential lifecycle) turn out to need real client state. That is a genuine
possibility and the decision should be revisited when those designs land — not assumed either way
now.

---

## 5. Database

**PostgreSQL 16 + TimescaleDB.** Argued in full in amendment 01. Briefly: SQLite was inherited from
the phone and is not Postgres-compatible, so it put a translation layer on the durability path;
InfluxDB and DuckDB are single-purpose engines that would sit *beside* the relational store rather
than replace it, leaving two systems anyway; TimescaleDB is Postgres, so there is one system, one
backup story, one query language.

---

## 6. Build — CMake

Chosen over the meson the original spec named, for one reason: JetBrains tooling has first-class
CMake support and only plugin-grade meson support, and this project is driven from an IntelliJ
project with a remote toolchain on the Pi. CTest gives the same exit-code acceptance the task Verify
steps rely on.

Bazel and plain Make were not seriously considered — Bazel is disproportionate for ~10 C targets,
and Make lacks the toolchain and test integration.

**Modern CMake practice:** target-scoped `target_include_directories` / `target_compile_options`,
not directory-scoped `include_directories()` / `add_compile_options()`. The current root
`CMakeLists.txt` uses the directory-scoped forms and should be migrated — noted as a follow-up.

---

## 7. Swift

**Used for:** the iOS app only, as a HealthKit bridge and viewer.

Not a choice: HealthKit is reachable only from an app on the device. There is no server-side API and
no alternative language.

**What changed:** the phone no longer shares a storage engine, a schema, or a generated enum with
the Pi. It reads `registry.json` over HTTP like any other client. The Swift codegen artifact was
removed for exactly this reason — it existed only because of the assumption that the phone links
into the same schema, which is the same assumption that put SQLite on the Pi.
