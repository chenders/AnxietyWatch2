# App comparison — process state

**Last updated:** 2026-10-06
**Execution mode:** **full sequential mode** — the pilot (`bearable`) was reviewed and the owner
instructed the run to continue. Candidates are processed strictly one at a time to completion.
**Sequencing rule:** strictly sequential. **Exactly one active candidate at a time.** No cross-candidate
parallelism, no prefetching a later candidate's research while an earlier one is in progress.

---

## Status legend

| Status | Meaning |
|---|---|
| **Complete** | All four candidate deliverables written and verified present, and the candidate's comparison produced |
| **In progress** | This candidate is the single active candidate |
| **Not started** | No work done; no research files, no prefetching |
| **Blocked** | Cannot proceed without an owner decision or an external change |
| **Skipped** | Deliberately dropped, with a reason |

## Queue and per-candidate status

Shortlist order is from `CANDIDATE_SHORTLIST.md`. The ordering is a **review order**, not a quality
order — see `RESEARCH_METHOD.md` §2.4.

**Active candidate: `#3 exist`.** Candidate 2 (`welltory`) is complete; nothing else is in progress.

| # | Slug | Score | Status | Started | Completed | Blocker |
|---|---|---|---|---|---|---|
| 1 | `bearable` | 13 (rank 1 on dimension A) | **Complete with inspection limitation** | 2026-10-05 | 2026-10-06 | Phase 2A not performed — no lawfully authorized artifact. Public-claims research unaffected. |
| 2 | `welltory` | 14 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). Vendor's **public research repos and datasets were inspected** and their citations independently verified. |
| 3 | `exist` | 11 | **In progress** | 2026-10-06 | — | — |
| 4 | `oscar` | 11 | Not started | — | — | **Inspectable from published source** |
| 5 | `sleephq` | 12 | Not started | — | — | — |
| 6 | `nightscout` | 15 | Not started | — | — | **Inspectable from published source** |
| 7 | `empatica` | 14 | Not started | — | — | — |
| 8 | `oura` | 13 | Not started | — | — | — |
| 9 | `homeassistant` | 13 | Not started | — | — | **Inspectable from published source** |
| 10 | `mindshift` | 12 | Not started | — | — | — |
| — | `emoods` | — | Skipped | — | — | No DNS A record for `emoods.net` at research time |

**Sequencing rule in force.** Only one row may be **In progress**. A candidate is started only after the
previous one's four deliverables exist and its verification command has been run.

## Pilot status

| Step | Status | Evidence |
|---|---|---|
| Ground AnxietyWatch capability inventory | **Complete** | C1–C26 in `CANDIDATE_SHORTLIST.md`, each labelled V1 / V2 current / V2 specified target / Unresolved |
| Build full ten-candidate shortlist | **Complete** | `CANDIDATE_SHORTLIST.md` |
| Validate the free inspection toolchain | **Complete** | `bearable/bearable_IMPLEMENTATION_INSPECTION.md` §4.1 — mitmproxy 12.2.3, emulator proxy `10.0.2.2:8080`, flows captured, HTTPS opaque by design, proxy removed and verified `null` |
| **Candidate 1 of 10 — `bearable`** | **Complete (with an inspection limitation)** | See §Candidate 1 |
| **Candidate 2 of 10 — `welltory`** | **Complete (with a partial inspection)** | See §Candidate 2 |
| Candidate 3 — `exist` | **In progress** | Active |
| Candidates 4–10 | **Not started** | No files created. No research begun. |
| Pilot review by owner | **Completed** | Owner instructed the run to continue; mode switched to full sequential |
| Full sequential run of candidates 3–10 | **Authorized** | One candidate at a time |

## Candidate 1 — `bearable`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `bearable/bearable_CLAIMED_FEATURES_AND_PURPOSE.md` | Written |
| Authorized implementation inspection | `bearable/bearable_IMPLEMENTATION_INSPECTION.md` | Written — **inspection NOT PERFORMED**, blocker and limitation recorded |
| Candidate-specific Prompt 2 | `bearable/PROMPT_2.md` | Written — based on `PROMPT2-USER-VALUE.md` |
| Comparison report (Prompt 2 run) | `bearable/bearable_COMPARISON.md` | Written |

**Verification command:**

```sh
ls -l docs/app_comparison/bearable/
```

All four files must be present and non-empty. This is the check that marks the candidate complete —
not a model assertion.

**Outcome in one line.** Bearable is a **partial competitor to AnxietyWatch v1** (same daily
journal-and-correlate loop, no sensors, no CPAP, no PDF, no local-first storage, mandatory cloud
account) and **not a competitor to AnxietyWatch2** (no always-on collection, no provenance, no alarm).

## Candidate 2 — `welltory`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `welltory/welltory_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 19 sections, 26-row evidence table |
| Authorized implementation inspection | `welltory/welltory_IMPLEMENTATION_INSPECTION.md` | Written — **PARTIALLY PERFORMED**: vendor's 13 public GitHub repos and CC0 research datasets inspected; app binaries not |
| Candidate-specific Prompt 2 | `welltory/PROMPT_2.md` | Written — based on `PROMPT2-EVIDENCE-FIRST.md` |
| Comparison report (Prompt 2 run) | `welltory/welltory_COMPARISON.md` | Written — 10 sections |

**Verification command:**

```sh
ls -l docs/app_comparison/welltory/
```

All four files must be present and non-empty.

**Outcome in one line.** Welltory is a **partial competitor to AnxietyWatch v1** (shares the
personal-baseline HRV commitment, paired before/after measurement and automatic weather context; has no
CPAP, no medication feature, no overnight capture, no alarm, no self-hosting, and deletes free-tier data
at 30 days), **not a competitor to AnxietyWatch2**, and its most useful contribution is a
**vendor-documented case study** of a personal-baseline system whose fixed medical threshold overrides
the baseline and tells chronic-condition users that HRV "can't provide meaningful insights". Its PPG
accuracy work was validated against a **Polar H10** — the same reference sensor AnxietyWatch uses — with
the paired dataset published under CC0 and the method peer-reviewed, though the study is n=13 healthy
adults at rest and the store copy generalises past it.

## Residual uncertainty — `welltory`

Carried forward from `welltory/welltory_COMPARISON.md` §9. Rows 1, 2, 8, 9, 10 and 12 of the
reconciliation table remain unresolved because no product binary is inspectable.

- **W1** Store copy says Fitbit, Garmin, Oura and Mi Band sync for "heart health data"; the vendor's own
  support documentation says those devices **cannot** supply HRV. Which reading is operative?
- **W2** The fixed "medical threshold" — its value, scope, and whether it overrides the personal
  baseline. **This is the single most consequential open question in the report.**
- **W3** Whether any medication, supplement or dose-tracking feature exists at all. None was found.
- **W4** The export format, granularity, timezone, units, and whether it can be automated before the
  30-day free-tier deletion.
- **W5** Whether the 21 HRV metrics are computed on-device or server-side, and offline behaviour.
- **W6** Which "clinically validated tests" exist for anxiety; the instruments are not named.
- **W7** Which enumerated sleep/vital metrics are genuinely ingested per platform versus merely listed
  as categories in the privacy policy.
- **W8** `My patterns` reliability — the vendor's ~7-occurrence threshold against one six-month user
  report of zero connections.
- **W9** What the Play data-safety "3 others" (shared) and "3 others" (collected) resolve to.
- **W10** Whether any clinician-shareable output exists beyond the printable blood-pressure report.
- **W11** Whether the April 2022 athlete-validation study and the IEEE EMBC Berlin presentation are
  backed by retrievable evidence. Neither was located.
- **W12** Whether stress/recovery episode detection is preventive in any sense or purely retrospective.

**Most promising untried step.** Parsing the published `datatypes.md` schemas in the vendor's CC0
repositories. Those schemas would show which measurements Welltory actually models, and could close the
CPAP (W3) and medication questions on evidence rather than absence. This was not done.

**Process-level note.** Welltory was the first candidate to yield a *partial* inspection, because the
vendor publishes research artifacts even though it publishes no product source. That is a third
inspection mode between "binary inspected" and "nothing inspected", and it is worth applying to every
remaining candidate: check for a vendor GitHub organisation before concluding inspection is impossible.

## Residual uncertainty — `bearable` None of these can be closed without an
authorized artifact inspection.

- **Q1** Actual Premium price. The vendor does not publish one; Play Store lists only a US$4.99–$199.99
  IAP range.
- **Q2** Whether arbitrary imported metrics (HRV, SpO₂, respiratory rate) can enter the correlation
  engine as *outcomes* or only Bearable's fixed outcome set.
- **Q3** What Bearable does with a day whose imported metric is missing or stale — no data, zero, or a
  normal observation. This is the direct analogue of AnxietyWatch's three-state rule.
- **Q4** Whether blank/skipped entries are excluded from correlations or treated as low/zero. The
  support article exists; its content was not retrieved.
- **Q5** Whether any report shows significance, confidence or sample size, or only effect direction and
  magnitude.
- **Q6** The exact CSV export schema — raw events vs daily aggregates, timestamp format, timezone, units.
- **Q7** What the Android data-safety declaration's "2 others" / "5 others" resolve to, and whether any
  of it covers health data. **This is an apparent conflict with the vendor's "no third-party sharing"
  statement and is the most reputationally significant open question.**
- **Q8** Whether the account is an absolute gate or whether any logging works without one.
- **Q9** Whether the cited oncology systematic review (DOI 10.1016/j.adro.2020.09.016) rated Bearable
  favourably or merely included it.
- **Q10** How custom scales are stored and exported; any accuracy or accessibility documentation.

**Process-level uncertainty.** The toolchain is validated but has never been exercised against an
actual target app. A first real inspection may surface problems that emulator-system-component testing
could not.

## Owner decisions pending

1. **Pilot review.** **Resolved** — the owner instructed the run to continue; full sequential mode is
   now active.
2. **Inspection authorization policy.** Still open for commercial candidates with no published research
   surface. Options, in ascending order of owner involvement:
   - **a.** Inspect only open-source candidates (`oscar`, `nightscout`, `homeassistant`) from published
     source. **No owner decision needed**; this is the recommended default and is lawful today.
   - **b.** Authorize installing store apps on a disposable emulator under an owner store account,
     without creating any target-app account. Unlocks first-run permissions, bundled SDKs, manifest
     surface and pre-account network destinations.
   - **c.** Authorize creating a throwaway target-app account under a sheltering alias on the
     disposable emulator, to observe post-login sync behaviour. The brief currently forbids this; it
     would need an explicit recorded decision.
   - **d.** Accept public-claims-only for all commercial candidates, and mark Phase 2A *not performed*
     in each comparison report.
3. **Shortlist adjustment.** `emoods` was dropped (no DNS A record for `emoods.net`). Confirm whether a
   replacement tenth candidate is wanted. **Not blocking** — nine candidates remain and are being
   processed in order.

## Rules this file enforces

- A candidate is **Complete** only when all four deliverables exist and the verification command has
  been run. A model's assertion is not completion.
- No candidate's research begins while another is **In progress**.
- Inspection limitations are recorded as **evidence limitations**, never as proof that a feature does
  or does not exist.
- Phase 2A is never silently skipped for a mobile candidate: it is either performed, or recorded as not
  performed with a reason.
