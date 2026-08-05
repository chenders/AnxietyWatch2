# Prompt for pi — independent three-model spec review

Paste the block below to pi verbatim. Session state, deliberately untracked — delete once
consumed. Context: the spec set already closed a two-round three-model review on 2026-08-05
(triple-PASS after fixes), so this run is an independent confirmation under pi's own harness,
run **before** the PR #19 merge in `PI-KICKOFF.md`.

---

You are in `~/Source/AnxietyWatch2` on branch `docs/admin-console-spec`. Run an independent
three-model review of the admin-console spec set through the LiteLLM gateway, invoking each
reviewer from your shell as `pi --model <id> -p "<brief>"` with `LITELLM_WORKER_KEY` set (mint a
fresh key via `~/Source/ai-harness-stack/litellm/create-worker-key.sh 5 25` if yours 401s — every
pre-2026-08-04 key is dead).

Bundle all six documents into every brief, in this order, with clear headers:

1. `docs/design/SPEC-admin-console.md` (the normative spec)
2. `docs/design/GEN1-REASSESSMENT.md`
3. `docs/design/PROMPT-admin.md`
4. `docs/specs/2026-08-05-amendment-07-api-in-rust.md`
5. `docs/design/HOMEASSISTANT-ARCHITECTURE.md`
6. `docs/design/HOMEASSISTANT-PRIOR-ART.md`

Shared framing in every brief: AnxietyWatch2 is a single-user, open-source anxiety-tracking
system — a Raspberry Pi 5 collects and stores physiological data, Rust services,
PostgreSQL+TimescaleDB. It is NOT a medical device and NOT primarily an alarm; the CNS alarm is
one feature of ~20 and is built LAST (M6). Do not let findings skew toward alarm/safety framing.
Document 1 is normative; documents 2–6 are companions it references.

Give each of the three models a **different lens** — identical prompts produce overlapping
findings:

- `openai/gpt-5.6-sol` — **coherence**: cross-document contradictions, the same requirement
  stated twice at different strength, §-references that land on the wrong heading, terminology
  drift (console `LIVE`/`FAILING`/`UNKNOWN` vs wire `Live`/`Stale`/`NeverSeen`; M-milestones vs
  Stage-N vs the M.2 HAT).
- `gemini/gemini-3.1-pro-preview` — **buildability**: every normative requirement must be owned
  by a milestone (§3.1), deferred (§9c), or blocked (§7b) — find orphans; acceptance criteria
  (§9) that gate nothing; anything unbuildable at proportionate cost for a one-operator
  appliance.
- `xai/grok-4.5` — **soundness**: do the arguments actually follow from the
  `[observed]`/`[verified]`/`[computed]` claims; are the licence statements
  (Apache-2.0/MIT/GPL-3/CC-BY-NC and their consequences) correct; does anything still read
  alarm-first; what should be cut.

Required report format, stated in each brief: numbered findings, each with severity
BLOCKER / SHOULD / NIT, the document + section, and a one-line proposed fix; end with exactly
`VERDICT: PASS` or `VERDICT: FAIL` plus blocker numbers; be selective — a finding that cannot be
grounded in the text is not a finding.

Context to include in each brief: this set passed a two-round three-model review on 2026-08-05.
Two findings were deliberately declined by the operator and must NOT be re-litigated:
(1) the §8b integrations table stays in the spec at the operator's direction;
(2) §1.3's rationale overlap with `PROMPT-admin.md` stays.

When the three reports return: verify every finding against the actual document text before
accepting it (models misread), fix genuine BLOCKERs and cheap SHOULDs on the branch in focused
commits (stage files by name, never `git add -A`), record a reason for anything you decline, and
give me a consolidated summary — per-model verdict, what you fixed, what you declined and why.
Do not push, and do not merge PR #19, without telling me first.
