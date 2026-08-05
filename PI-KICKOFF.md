# Kickoff prompt for pi — admin console continuation

Paste the block below to pi verbatim. This file is session state, not project documentation —
delete it once pi has consumed it (it is deliberately untracked).

---

You are continuing admin-console spec work in `~/Source/AnxietyWatch2`, on branch
`docs/admin-console-spec` (draft PR #19). The source of truth is
`docs/design/HANDOFF-admin-console.md` — read it fully, then execute its "Immediate next steps, in
order" section. In short:

1. **Confirm the last review fix** — `docs/design/PROMPT-admin.md` must say "five separate visual
   vocabularies" with a Configuration row, and "all five status vocabularies" in deliverables. With
   that confirmed, the multi-model review is a triple-PASS and the merge condition is met.
2. **Update PR #19's body** (it stale-claims two research agents are outstanding — they are done
   and folded in), mark it ready, **merge it**, and delete the handoff file.
3. **Do the post-merge instruction-file work** the handoff lists: `CLAUDE.md`, `AGENTS.md`, the
   Copilot instructions, `ci.yml`, the pre-push hook, and `README.md` — encoding the status law
   (no status without observation age), the per-source-cadence build gate, never render status
   from an undeployed producer, no `UPDATE`/`DELETE` on samples, and the telemetry privacy
   boundary (counters and timings leave; measurements and words do not — reconcile the README's
   no-telemetry claim with spec §4.9c's opt-in New Relic).
4. **Then start M0**: author `openspec/changes/admin-console-foundation/` per the handoff's
   "Then: build M0" section and the spec's §3.1 and §9 — proposal, design, tasks; every task
   carries tier, host, `Reads:`, and a `Verify:` command that exits 0.

Constraints:

- Do **not** re-litigate the two review findings the handoff marks as deliberately declined.
- The bind/auth and watch peer-vs-outbox decisions are **deferred by spec §9c** — ask Chris at the
  point of use; never block on them and never resolve them early.
- The alarm path is M6, built **last**.

Housekeeping:

- The untracked `firmware/` directory (including the new `emay-sleepo2-emu/`) is **not** part of
  this PR — stage files by name, never `git add -A`.
- If you need the LiteLLM gateway, mint a fresh worker key per the handoff's "Operational recipes"
  section — every pre-2026-08-04 key is dead.
