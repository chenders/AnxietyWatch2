# FINDINGS — the cross-candidate summary

**Status:** empty by design. This run has produced no candidate findings yet.
**Research date:** 2026-10-07 · **Shortlist:** `CANDIDATE_SHORTLIST.md` · **State:** `PROCESS_STATE.md`

This is the file a reader opens first. It is updated when a candidate is closed by
`scripts/compare-one-app.sh finish`, and never left stale. The per-candidate detail lives in
`{slug}/{slug}_COMPARISON.md`; this file carries only what survives across candidates.

---

## 1. Where the run stands

| Candidate | Tier | Status | Strongest finding | Where to read it |
|---|---|---|---|---|
| — | — | — | — | — |

---

## 2. Feature matrix — AnxietyWatch v1 / v2 versus the field

Filled in as candidates close. v2 columns always distinguish **current** from **specified**.

| Capability | v1 | v2 | Candidate(s) matching | Notes |
|---|---|---|---|---|
| Subjective anxiety logging (C1–C2) | shipped | current | — | |
| Medication dose + paired efficacy (C3–C4) | shipped | current | — | |
| Multi-source provenance-preserving ingestion (C5, C18) | partial | current | — | |
| Personal rolling baseline (C6) | shipped | current | — | |
| Correlation engine (C7) | shipped | current | — | |
| Raw beat-to-beat HRV capture (C8) | shipped | current | — | |
| CPAP import, retrospective (C9) | shipped | current | — | |
| CPAP live stream (C20) | no | current | — | |
| Overnight pulse oximetry (C10) | shipped | current | — | |
| Always-on BLE collection (C17) | no | current | — | |
| Clinical report / export (C11) | shipped | current | — | |
| Self-hosted, no third-party cloud (C12) | shipped | current | — | |
| Three-state honesty, not-measured explicit (C21) | partial | current | — | |
| CNS early warning + independent alarm (C22–C24) | no | specified | — | |
| Environmental / aircraft-noise context (C25) | partial | idea | — | |
| Radar respiratory rate (C26) | no | current | — | |

---

## 3. Patterns worth watching

Recurring shapes to check for in every candidate. A pattern becomes a finding only when two or more
candidates exhibit it; until then it is a question.

- **The gated morning score** — a single derived number that (a) needs N days of data before it appears,
  (b) silently carries the last value or vanishes when a night is missed, and (c) is the paywall's
  centrepiece. Check: is the not-measured state rendered as *not measured*, or as a stale value, or as
  nothing at all?
- **Export asymmetry** — the derived score is exportable, the raw signal it is derived from is not.
- **The unversioned algorithm change** — a scoring change shipped after the validation that is used to
  market it, with no version identifier in the app or the data.
- **Three-surface privacy disagreement** — the policy, the shipped client and the storefront data-safety
  panel giving three different answers, with the most conservative statement governing.
- **Vendor-authored validation** — the study is real, peer-reviewed, and authored by the vendor's own
  staff; it validates a different tier or a different population than the marketing implies.
- **Alert-path degradation by documented confounder** — a real-time alert that a documented, common
  condition (medication, altitude, sensor placement) silently desensitises, with the only mitigation being
  a manual user action.
- **The absent-signal branch** — what the alert path does when the watched signal goes missing: hold,
  escalate, or silently resolve.

---

## 4. Open questions this run has not answered

Recorded as they arise; each is closed with a finding, a named coverage limit, or an owner decision.

1. Has any consumer product in this set shipped an alarm path that survives loss of the phone *and* loss of
   the network? (v2 specifies one; is it unoccupied in practice?)
2. Does any candidate treat "no measurement tonight" as a first-class state rather than a gap to be filled?
3. What does the cheapest paid tier in each candidate actually unlock, priced per month, at the time of
   reading?

---

## 5. How this file is maintained

- One row per closed candidate in §1, added in the same commit as that candidate's four deliverables.
- §2 rows are filled with `matching candidate (evidence class)`, not with prose.
- §3 gains a pattern only on the second sighting; the first sighting is recorded in the candidate's own
  comparison file under §13.
- §4 loses a question only when it is answered, and the answer links to the candidate file that answers it.
