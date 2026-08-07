# Change: admin-console-m0 — foundations, no UI

**Scope:** SPEC-admin-console.md §3.1 row M0, exactly: per-source cadence in the registry ·
the attempt/observation/domain-record schema · the liveness evaluator as one pure function ·
the axum HTTP service consuming vitals-proto (amendment 07) · the §8b envelope carrying the
§2.3b domain-record class, with a crude test page · bind/auth deliberately deferred (§9c;
default 127.0.0.1).

**Method (operator-directed, 2026-08-07):** implementation by pi workers through the Pi's
LiteLLM gateway — deepseek / openai / gemini tiers per HERDR-GUIDE §3 — dispatched and
verified by the conductor. Acceptance is each task's Verify exit code, run by the conductor,
never a worker's report. Review tier remains claude.

**Gates:** none. M0 is gated on nothing (spec §3.1) and gates M1.

Task briefs live in tasks.md beside this file. Worker-facing copies were dispatched verbatim.
