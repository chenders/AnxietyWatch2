# Amendment 07 — The HTTP API and admin console are Rust, not Python

**Date:** 2026-08-05
**Supersedes:** [`2026-08-03-language-choices.md`](2026-08-03-language-choices.md) §1 row
*"HTTP API, admin, analysis → Python 3.12 + FastAPI"*, and §3 in full.
**Affects:** the admin console (M0–M2 in [`../design/SPEC-admin-console.md`](../design/SPEC-admin-console.md) §3.1), the web dashboard's backend, any future client write API.

---

## What the earlier decision said

`language-choices.md` §3 chose Python 3.12 + FastAPI for the HTTP API, admin console, and analysis
jobs. Its reasoning was sound as far as it went:

> "none of this is on the life-safety path. It is I/O-bound, changes often…"

plus ASGI-native SSE for job status, and OpenAPI generated from the code for the iOS client and the
dashboard.

## Why it was wrong

**It missed what the console actually does.** The admin console's core job is reading `vitalsd`'s
health and subscribing to its stream. That transport is **length-prefixed CBOR over a Unix socket**,
defined by `vitals-proto` and already implemented in Rust — `vitals-cli` is a working client.

A Python backend means implementing that wire protocol **a second time, in a second language, kept
in sync by hand**. Every future change to `Request`, `Response`, `Health`, `SourceHealth` or
`Liveness` becomes a two-place edit with no compiler to catch the drift.

That is precisely the failure class the registry codegen exists to eliminate. The architecture doc's
own §5 table lists *"`\"polar_h10\"` string literals drifted from typed constants"* as a scar, fixed
structurally by generating one definition into every consumer. Hand-porting the protocol into Python
reintroduces exactly that hazard one layer up — and unlike a source label, a protocol mismatch fails
at runtime, in the dark, on the box that is supposed to be reporting whether things are working.

**One wire-protocol implementation, not two.** That is the whole argument.

## Secondary reasons, none decisive alone

- **No second runtime on the Pi.** The console is currently the only thing that would drag Python and
  a dependency tree onto the box outside batch tooling.
- **The registry already generates Rust.** Types, units, ranges and (per M0) cadences arrive as
  compiled constants rather than as JSON parsed at startup and validated by hand.
- **`serde` already models the protocol.** The types exist; the API layer reuses them instead of
  restating them.
- **Static binary, systemd unit, same shape as everything else** on the box.

## What Python keeps

**The batch parsers.** EDF/CPAP parsing and any analysis that is genuinely a data-processing job stay
Python, run out-of-process, and never touch the socket. They already work and there is no reason to
port them. This amendment is about the **HTTP and socket-facing layer**, not about removing Python.

## What this costs, honestly

- **Slower iteration on request/response shapes** than FastAPI. Real, and accepted: this API has one
  consumer to begin with, and the compiler catches what the iteration speed would have cost later.
- **OpenAPI is no longer free.** `utoipa` or an equivalent must be wired deliberately, and the
  §1-row promise of generated client documentation now needs an explicit task rather than arriving
  as a side effect.
- **SSE and long-running job status need doing by hand**, where FastAPI gave them for nothing. `axum`
  supports both; it is work rather than a gift.

## Decision

**`axum` + `tokio` + `sqlx` for the HTTP API and admin console.** Same workspace as the existing
crates, consuming `vitals-proto` directly.

`language-choices.md` §4's *"vanilla HTML/CSS/JS, no framework"* is **unchanged** by this amendment —
including its explicit escape hatch, which is still open and should be re-read when the console
designs land.
