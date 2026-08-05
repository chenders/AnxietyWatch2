# Handoff — admin console spec work

**Written:** 2026-08-05, updated same day at 00:35 PT for herdr/pi takeover ·
**Branch:** `docs/admin-console-spec` (unmerged, draft PR #19)
**Delete this file when the work lands.** It is session state, not project documentation.
**Start at [Immediate next steps](#immediate-next-steps-in-order).**

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
| `docs/design/HOMEASSISTANT-ARCHITECTURE.md` | HA research: patterns to adopt, machinery to avoid (reference) |
| `docs/design/HOMEASSISTANT-PRIOR-ART.md` | HA research: integrations to translate, licences (reference) |
| `docs/hardware/PURCHASE-PLAN.md` | Already merged as PR #17 |

### The spec's central claim, in one paragraph

Gen-1 has one defect wearing a dozen costumes: **it reports the last success and calls it status.**
Observed live — a 5-day sync gap shown as plain text; both CPAP paths dark with never-synced and
silently-stopped rendering identically; a poller that ran every 10 minutes re-upserting the same nine
rows and then died unnoticed. The fix is liveness as a computed property with a declared cadence,
degrading to `UNKNOWN` automatically, with no status rendered without the age of its observation.

---

## Still pending

### 1. Home Assistant research — DONE (2026-08-05)

Both agents completed. Findings are written out and folded in:

- [`HOMEASSISTANT-ARCHITECTURE.md`](HOMEASSISTANT-ARCHITECTURE.md) — how HA is built; verified
  against a clone of `home-assistant/core` @ `9c8ae126`. Headline: HA has **no automatic staleness
  detection** (2 of 1,481 integrations implement expiry; a failed poll writes nothing and the stale
  value looks healthy forever) — independent confirmation of this spec's governing law.
- [`HOMEASSISTANT-PRIOR-ART.md`](HOMEASSISTANT-PRIOR-ART.md) — which integrations to translate.
- Spec **§8b** — the normative subset: eight adopted patterns, the rejected machinery, and the
  integration-translation commitment table with licence obligations.

### 2. Final multi-model review — TWO ROUNDS DONE, one verdict outstanding

Chris's instruction: **merge the branch when the final review passes.**

**Round 1** (fresh eyes, all six documents together, three lenses — Sol/coherence,
Gemini/buildability, Grok/soundness): 6 blockers, 13 shoulds, 4 nits. All fixed or declined with a
recorded reason — commits `20499d9` and `376dc79` carry the full disposition lists in their
messages. **Two findings were declined deliberately; do not re-litigate them:** the §8b
integrations table stays in the spec (Chris's explicit direction), and §1.3's rationale overlap
with `PROMPT-admin.md` stays (different audiences).

**Round 2** (verification): Gemini **PASS**. Sol **FAIL on exactly one residual** — `PROMPT-admin.md`
still counted "four" status vocabularies against the spec's five — **fixed in the same commit as
this handoff update** (five vocabularies, Configuration row added, deliverables count updated).
Grok's round-2 verdict had not returned when this session stopped; its output lands at
`/private/tmp/claude-501/-Users-chris-Source-AnxietyWatch/8b7fa9a8-929a-496d-8ae0-c9f9dc6cea5c/scratchpad/verify2-out-grok.md`
(local, ephemeral — alongside `verify2-out-{sol,gem}.md` and the briefs `verify2-*.txt`).

### Immediate next steps, in order

1. **Read Grok's round-2 verdict** at the path above (an empty file means still running or lost;
   re-run is cheap — `verify2-grok.txt` in that directory is the exact brief, model
   `xai/grok-4.5`). Round 2's only known open item was the PROMPT four→five count, already fixed.
2. Fix anything real that Grok adds; ignore anything that re-litigates the declined items.
3. **Confirm the PROMPT count fix** — either judge it directly (it is mechanical: PROMPT now says
   "five separate visual vocabularies" with a Configuration row, and "all five status
   vocabularies" in deliverables) or re-run Sol with the round-2 brief.
4. **Merge PR #19**: update its body first (it still says two HA research agents are outstanding —
   they are done and folded in), `gh pr ready 19`, then merge. Delete this handoff file in the
   merge or immediately after.
5. **Post-merge queued work** (Chris's standing instruction from this session): update `AGENTS.md`,
   `CLAUDE.md`, `.github/copilot-instructions.md`, the Rust instructions file, `ci.yml`,
   `.githooks/pre-push`, and `README.md` to encode the newly-settled rules — the status law
   (no status without observation age), the per-source-cadence build gate, never render status
   from an undeployed producer, no `UPDATE`/`DELETE` on samples, the telemetry privacy boundary
   (counters and timings leave, measurements and words do not), and reconcile the README's
   no-telemetry claim with §4.9c's opt-in New Relic.
6. **Then start M0** — see "Then: build M0" below. Bind/auth and the watch peer-vs-outbox
   questions are **deferred by spec §9c**: ask Chris at the point of use, do not block on them,
   and do not resolve them early.

---

## Operational recipes

### `pi` multi-model reviews

The gateway is LiteLLM on the Pi at `http://192.168.1.48:4000/v1`. Model catalog:
`~/.pi/agent/models.json`.

**Any pre-2026-08-04 key is dead** — the LiteLLM DB was recreated that day and lost its token
table, so old `LITELLM_WORKER_KEY` values 401 (Chris hit this too; the recipe below is confirmed
working). Mint a fresh worker key (budget-capped $5/24h, $25/30d):

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
5. **The bind/auth decision** — deferred by spec §9c: ask Chris when the service is about to
   listen, not before.
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
