# Handoff — admin console spec work

**Written:** 2026-08-05 · **Branch:** `docs/admin-console-spec` (unmerged, ~15 commits)
**Delete this file when the work lands.** It is session state, not project documentation.

---

## Where this got to

Four agents audited the gen-1 Flask console (26 routes, 17 templates, 40 tables) plus a live session
against the running deployment. That produced a functional spec, a judgement pass on what to carry
over from gen-1, a rewritten design brief, and one amendment. Six model reviews were folded in
(Opus, Sonnet, Fable via the Agent tool; GPT-5.6 Sol, Gemini 3.1 Pro, Grok 4.5 via `pi`).

### What is on the branch

| File | What it is |
|---|---|
| `docs/design/SPEC-admin-console.md` | The functional spec. What the console must do, and why |
| `docs/design/PROMPT-admin.md` | The design brief (rewritten — it previously described a superseded architecture) |
| `docs/design/GEN1-REASSESSMENT.md` | Every gen-1 concept judged ADOPT / REPLACE / EXTRACT / DROP / DEFER against gen-2's goals |
| `docs/specs/2026-08-05-amendment-07-api-in-rust.md` | Supersedes the Python/FastAPI decision for the HTTP layer |
| `docs/hardware/PURCHASE-PLAN.md` | Already merged as PR #17 |

### The spec's central claim, in one paragraph

Gen-1 has one defect wearing a dozen costumes: **it reports the last success and calls it status.**
Observed live — a 5-day sync gap shown as plain text; both CPAP paths dark with never-synced and
silently-stopped rendering identically; a poller that ran every 10 minutes re-upserting the same nine
rows and then died unnoticed. The fix is liveness as a computed property with a declared cadence,
degrading to `UNKNOWN` automatically, with no status rendered without the age of its observation.

---

## Still pending

### 1. Home Assistant research — two agents were running when this was written

- **Architecture**: HA's integration model, entity/device model, `device_class`/`state_class`/units,
  the `DataUpdateCoordinator` pattern, config flow, the Bluetooth stack, and its
  `unavailable` vs `unknown` distinction (which is the same distinction this spec is built on —
  worth citing as precedent).
- **Integration landscape**: which HA integrations map to this project's sources (Oura, Withings,
  Fitbit, the HA Companion app's HealthKit/Health Connect push, BLE health profiles, ESPHome for
  alarm output hardware), with PORT / READ-AND-REIMPLEMENT / USE-AS-IS / IGNORE verdicts and license
  notes.

**To do:** fold the findings into the spec — a section on which HA patterns to adopt, and a table of
which integrations to translate. Flag licensing: HA core is Apache 2.0, so porting code needs
attribution; reading protocol knowledge and reimplementing does not.

### 2. Final multi-model review, then merge

Chris's instruction: run one more `pi` review pass once everything above is done, **and merge the
branch when it passes.**

Review all four documents *together* — they now cross-reference each other, and the earlier rounds
reviewed them individually.

### 3. Two questions that genuinely need Chris

Everything else was answered from the code. These were not:

1. **Does the watch become a peer node** (own store, own `node_id`, reconciling with the Pi
   directly) **or stay a thin sender with a durable local outbox?** The pi5 design doc argues peer.
   Either fixes the real defect below; doing neither carries it forward.
2. **Bind and auth posture for the HTTP service** — LAN-only, Tailscale/WireGuard-only, or TLS with a
   pinned cert. Blocks M0 item 5.

---

## Operational recipes

### `pi` multi-model reviews

The gateway is LiteLLM on the Pi at `http://192.168.1.48:4000/v1`. Model catalog:
`~/.pi/agent/models.json`.

**The key in your shell is dead** — the LiteLLM DB was recreated 2026-08-04 and lost its token table,
so the old `LITELLM_WORKER_KEY` 401s. Mint a replacement (budget-capped $5/24h, $25/30d):

```sh
cd ~/Source/ai-harness-stack/litellm
env -u LITELLM_MASTER_KEY LITELLM_HOST=http://192.168.1.48:4000 ./create-worker-key.sh 5 25
```

`env -u` matters: it forces the script to read the master key from its own `.env` rather than the
stale one in the environment. Put the returned `key` into the shell profile as
`LITELLM_WORKER_KEY` so this stops recurring.

Then:

```sh
pi --model openai/gpt-5.6-sol -p "<review brief>"
```

Frontier models available: `openai/gpt-5.6-sol` · `openai/gpt-5-pro` ·
`gemini/gemini-3.1-pro-preview` · `xai/grok-4.5` · `deepseek/deepseek-v4-pro`.

**What worked in the review briefs:** give each model a *different lens* rather than the same prompt —
one on unbuildable requirements and contradictions, one grounded against the actual code, one on
whether the argument is sound and what to cut. Identical prompts produce overlapping findings.

**One warning learned the hard way:** an early review brief described the product as an alarm
appliance, and every finding from that round skewed toward the alarm path. The product is anxiety
tracking; the CNS feature is one of twenty and is built **last**. Brief accordingly.

---

## Then: build M0

Chris's priority as of the last message — *"the client is currently the least important part; getting
the backend right is the most important thing."* Design briefs are parked.

Recommended first step: write `openspec/changes/admin-console-foundation/` following the repo
convention (proposal, design, tasks; every task carries tier, host, `Reads:`, and a `Verify:` command
that exits 0). **Nothing in `openspec/` currently references the admin console**, which is why none
of this is wired into a gate.

M0, in build order — none of it exists today:

1. **The observation/attempt schema.** The canonical record everything reads. A row written *before*
   the work with an expected duration, updated with the outcome, distinguishing
   **created / updated / replayed / rejected**.
2. **Per-source cadence in `registry.toml`** plus codegen. A source without one is a build-time error.
3. **The liveness evaluator** — one pure function, injectable clock, consumed by both the console and
   the silence detector so they cannot drift.
4. **An `axum` service** exposing health and attempt history. Nothing serves HTTP today, and
   `vitalsd`'s unit is `AF_UNIX`-restricted.
5. **The bind/auth decision** (question 2 above).
6. **A crude test page** for posting journal entries, severity and tags. No design needed; it exercises
   the idempotency rule end to end.

---

## Things a fresh session should not have to rediscover

- **`quantity_health_samples` should not exist in gen-2.** The phone *does* push HealthKit data and
  that is a real source — but it belongs in `samples` with `source = healthkit`, not in a parallel
  mirror table with its own restore path (which nobody wrote, hence gen-1's 224k-row hole).
- **`health_snapshots` should not be a table.** It is a materialized view maintained by application
  code, and every bug it produced follows from that. A continuous aggregate cannot hold a value its
  inputs no longer support.
- **Domain records use natural keys today** — `anxiety_entries` PK is bare `timestamp`, and
  `song_occurrences.anxiety_entry_id` is a `TIMESTAMPTZ`. The Swift `id: UUID` exists but never
  reaches the wire and never survives a round-trip. There is **no conflict resolution anywhere**; the
  HLC on `samples` has no `WHERE EXCLUDED.hlc > stored` guard, so it is decorative.
- **The gen-1 watch path has a real defect (F-037).** The watch keeps no local copy of a quick-log
  entry, plays the success haptic before any delivery signal, and can double-deliver — surviving only
  because a natural-key upsert three hops downstream collapses the pair. Client-generated IDs remove
  that accidental protection unless **the watch mints the ID**.
- **Five `AnxietyEntry` fields and `MedicationDose.isPRN` are never synced** and are silently zeroed
  by a restore.
- **`heartbeat_pulsing` reads healthy unconditionally today** — its inputs are public fields with no
  request verb that can set them. Do not render a status from a field whose producer is not deployed.
- **The spec is amendable.** See its §9b. Measurement contradicting an estimate warrants an amendment;
  inconvenience does not.
