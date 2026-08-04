# Review synthesis and fix tracker

**Date:** 2026-08-03
**Rounds:** 3 · **Reviewers:** grok-4.5, deepseek-v4-pro, kimi-k2, gemini-3.1-pro, gpt-5.6-sol
**Transcripts:** `docs/reviews/2026-08-03-*.txt`

Every finding, its status, and what happens next. **Two of my own Amendment 04 fixes were wrong** and
are marked as such.

---

## Status summary

| | Count |
|---|---|
| Fixed in amendments 01–04 | 9 |
| **Wrong fix — corrected in A05** | **2** |
| Open — now specified in A05 §4 | 11 |
| Rejected with reason | 2 |
| Deferred with reason | 2 |

---

## A. Critical — blocking implementation

### A1. The repo describes three different systems · **FIXED** (commit 9604aeb)

*Found independently by gpt-5.6-sol and deepseek-v4-pro.*

> "`tasks.md` still mandates C11, SQLite, HLC replication, `samples_staging`, and
> `(source,type,timestamp)` idempotency, while **AGENTS.md orders workers to read only each task's
> `Reads:` list — which does not include amendments 01–04.**" — gpt-5.6-sol

> "`grep -c SQLite openspec/changes/pi5-vitals-core/tasks.md` returns 7. `grep -c batch_id
> db/deploy/samples.sql` returns 0. **These numbers must converge.**" — deepseek

Four amendments changed the language, storage engine, schema and key. No task brief was updated, and
`db/deploy/samples.sql` still declares `hlc_physical`, `hlc_logical` and `samples_staging`.

**The mechanism is my own rule.** `AGENTS.md` says *"Read **only** the files in that task's `Reads:`
list"* — correct, and it prevents context blowout. But no `Reads:` list references an amendment, so a
worker obeying the rules **cannot see them**. The safety mechanism became the delivery vector for the
defect. Same shape as the CLAUDE.md pitfall about a conditional-skip optimisation breaking an
invariant the unconditional version held.

**Fix:**
1. Rewrite all 15 task briefs against the amended architecture.
2. Regenerate the sqitch change set: drop `hlc_*` and `samples_staging`; add `batch_id`,
   `ingest_batch`, `vitals_producer`, `audit_log`.
3. Add a mandatory `**Amends:**` field to every task header, and change `AGENTS.md` so the amendment
   index is always in scope.
4. CI check: `grep -c SQLite tasks.md` must be 0.

---

### A2. Amendment 04 §1's RR fix is wrong · **CORRECTED IN A05 §1**

*gpt-5.6-sol, and it is right.*

> "Reconstructing RR times backward from packet arrival makes intervals within one packet distinct,
> but **it does not make packet arrival a stable event identity**… retransmission creates a second set
> at new times."

I conflated three separate things:

| Concept | What it is |
|---|---|
| **Event time** | when the beat occurred |
| **Observation time** | when the host received the packet |
| **Ingest identity** | whether this is the same observation replayed |

Backward accumulation estimates event time. It says nothing about identity, and the failure inverts
the original bug — instead of dropping real data, **the PK now accepts duplicates**:

- **Retransmission after reconnect** — the same RR payload anchored to a later arrival gets entirely
  new timestamps and inserts again.
- **Connection-interval batching** — adjacent notifications have different queuing delays, so every
  packet boundary introduces a discontinuity.
- **Host scheduling stalls** — a delayed callback shifts a whole packet of valid intervals.

**Corrected design — separate the three concepts in the schema:**

```sql
PRIMARY KEY (source, type, session_id, seq)   -- stable ingest identity
timestamp   TIMESTAMPTZ NOT NULL              -- best-estimate event time, a regular column
```

`seq` is a monotonic per-session beat index maintained from the cumulative RR sum, continuous across
packets and unaffected by arrival jitter. `timestamp` remains the queryable event time for gapfill,
charting and detection, and is allowed to be an estimate — because it is no longer load-bearing for
identity.

**Detection signals** gpt-5.6-sol gave: negative inter-packet timing residuals; duplicate interval
sequences after reconnect; RR counts *exceeding* expected beat count rather than falling below it.

---

### A3. The dead-man switch monitors the wrong thing · **CORRECTED IN A05 §2**

*Two independent failures, from gpt-5.6-sol and deepseek.*

**False negative** — gpt-5.6-sol:

> "A healthy `klaxond` can continue pulsing while `vitalsd`, `cns-engine`, every BLE collector, or the
> live subscription path is dead. **That is the worst false negative available**: the hardware
> watchdog sees a healthy heartbeat while no physiological event can reach the alarm."

**False positive** — deepseek:

> "During a deliberate `sudo reboot`, systemd sends SIGTERM to `klaxond`, which stops pulsing… the
> 555 times out mid-shutdown and the siren blares. **Every clean reboot produces a false alarm.**
> This is the exact class of failure amendment 04 §4 was written to prevent."

I wrote a false-positive generator two sections after complaining about false positives, and the
switch proves only that one process is scheduled.

**Corrected design:**

1. **The heartbeat is conditional on end-to-end liveness.** `klaxond` pulses the pin *only if* it has
   received a fresh sample from `vitalsd` within N seconds during a monitoring window. The heartbeat
   then attests the whole chain — collector → `vitalsd` → subscription → `klaxond` — not one process.
2. **Shutdown inhibit.** A systemd `ExecStop` asserts a hardware disable line before the pulse stops,
   released on next boot. A deliberate reboot is silent; a crash is not.
3. **The test must inject upstream faults.** Stopping the heartbeat only tests the 555. The real test
   kills or wedges `vitalsd`, `cns-engine`, and each collector **while leaving `klaxond` running**,
   and asserts the siren fires.

---

## B. Fixed in amendments 01–04

| Finding | Source | Where |
|---|---|---|
| `PRIMARY KEY` drops RR intervals | grok | A04 §1 — **but the fix is wrong, see A2** |
| OOM killer terminates alarm daemons | gemini | A04 §2 |
| "Independent failure domains" is false confidence | gemini | A04 §3 |
| LUKS-unlock-fires-alarm causes alarm fatigue | gemini | A04 §4 |
| Alarm self-test proves ALSA write, not sound | grok + deepseek | A04 §5 |
| Staging table `TEMP` vs `UNLOGGED` contradiction | deepseek | A04 §6 |
| Schema change orphans buffered spool rows | deepseek + kimi | A04 §7 |
| HLC vestigial after the spool amendment | grok + deepseek | A04 §8 |
| RTC absence also breaks TLS → kills out-of-band alerts | gemini | A04 §8 |
| Tactile-primary claim over-read the evidence | gemini | A04 §9 |
| Hash chain achieves little against root | gemini | A04 §10 |

---

## C. Open — accepted, not yet written

All eleven are specified in amendment 05 §4. Specified is not implemented.

| # | Finding | Source | Spec |
|---|---|---|---|
| C1 | Drain *progress* liveness — `systemctl is-active` lies | all three r1 | A05 §4.1 |
| C2 | Disk-full entirely unspecified; ~250 MB/night of RR | deepseek | A05 §4.2 |
| C3 | BLE MAC randomisation breaks static-MAC trust; use IRK | gemini | A05 §4.3 |
| C4 | Off-box backup deferred — all tiers on one disk | grok | A05 §4.4 |
| C5 | Spool format version detection and hard failure | kimi | A05 §4.5 |
| C6 | Advisory locking on the spool offset file | kimi + deepseek | A05 §4.6 |
| C7 | No mechanism to detect a stuck `cns-engine` | kimi | A05 §4.7 |
| C8 | Gap-fill policy not enforced at the query layer | kimi | A05 §4.8 |
| C9 | No end-to-end latency SLO | kimi | A05 §4.9 |
| C10 | `vitalsd` restart leaves detection blind with no warm replay | grok | A05 §4.10 |
| C11 | Subscriber backpressure undefined — slow consumer drops or blocks | grok | A05 §4.11 |

---

## D. Tools to adopt

| Tool | For | Status |
|---|---|---|
| `kani` | prove `decode_sfloat` / `bp_decode_2a35` panic-free | accepted, A04 §11 |
| `rppal` | GPIO for siren, shaker, heartbeat pin | accepted |
| `fs4` | `flock` on spool offset and segments | accepted |
| `sqlx` | compile-time-checked queries | accepted |
| `metrics` | Prometheus for lag, dedup counts, self-test age | accepted |

---

## E. Rejected, with reasons

**pgMemento for the audit trail.** Transaction-based logging with schema versioning and selective
rollback. Disproportionate for a single-user appliance where the requirement is "prove nothing
changed silently," not "roll back transaction 41,203."

**Blanket `REVOKE UPDATE, DELETE` on `samples`.** Forcing a correction to a superuser shell produces
*less* auditability, because the fix then happens outside any recorded path. Replaced by an audit
trigger with a hash chain.

---

## F. Deferred, with reasons

**Wayfire on a life-safety device** (gemini). A full Wayland compositor is genuine risk surface and
direct DRM/KMS would remove it. Deferred to Stage 8: the panel is not on the safety path and the
compositor already works. Revisit if it proves unstable.

**"TimescaleDB is bypassed by the safety path"** (gemini). True and intentional — the database serves
history, admin and analysis while detection runs on in-memory ring buffers. A04 §2's memory limits
address the resource competition. The reviewer is right that it must be justified rather than
assumed, which is what this entry does.

---

## G. What reviewers endorsed

Recorded so it is not re-litigated.

- **Rust over C11**, specifically not hand-lowering Swift's `Option`/`Result` into C sentinels —
  "an excellent, concrete architectural argument" (gemini)
- Quarantine out-of-range rather than clamp (grok)
- The `0x2A35` CCCD `0x0002` indication note — "the kind of scar that deserves to be structural" (grok)
- `WITH NO DATA` on the continuous aggregate (deepseek)
- Arming watchdog and governor in Stage 0 before daemons depend on them — "this ordering discipline
  is the right instinct and it's applied consistently" (deepseek)
- Stage deliverables progressing from static artifacts to a live console (kimi)
- The panel as a pure subscriber — "right dependency direction" (grok)
- The restore-verification timer as a concept (grok, with the caveat to add checksums and off-box)

---

## H. Fix order

1. **A1** — rewrite tasks + sqitch + `AGENTS.md`. Nothing else can be implemented correctly until
   the repo describes one system.
2. **A2** — redesign the RR identity model. Changes the schema, so it must land with A1.
3. **A3** — redesign the dead-man switch. Design-only until hardware arrives.
4. **C1, C2, C6, C10, C11** — the runtime-safety cluster; these determine whether failures are
   visible.
5. **C3, C5, C7, C8, C9** — correctness and observability.
6. **C4** — off-box backup, required before Stage 5 puts the klaxon into service.

---

## I. Notes on the review process itself

Worth recording, because the process nearly failed silently.

- **Round 1**: 3 of 5 produced real reviews. gemini emitted ~300 tokens and stopped; gpt-5-pro was
  not yet dispatched.
- **Round 2**: both retries died. First on a **request timeout** — gpt-5-pro read every file then
  failed with `Retry failed after 3 attempts` having generated nothing. Raised
  `litellm_settings.request_timeout` to 900 s.
- Then both hit **`budget_exceeded`**: the worker key was capped at $5/day and round 1 consumed it. A
  `/key/update` reported success but did not persist; minting a fresh key at $50/day fixed it.
- gpt-5-pro timed out three times even at 900 s, so the limit is not LiteLLM's. Swapped to
  `gpt-5.6-sol`, which completed immediately — and produced the sharpest findings of the exercise.
- **Round 3**: all five produced 4.4k–16k tokens.

**The lesson: agent status is not evidence of output.** All the failed runs sat at `idle` or `done`
with plausible-looking panes. What caught them was checking *generated token counts* — ↓893 and ↓314
are not reviews. Any automated dispatch loop must gate on "did it produce a review," not "did the
agent finish." That is the same rule the project already applies to workers, applied one level up.
