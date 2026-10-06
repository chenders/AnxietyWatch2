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

**Active candidate: none.** Candidates 1–3 are complete. The next candidate is `#4 oscar`, which is not
started and must not be begun until this file is committed.

| # | Slug | Score | Status | Started | Completed | Blocker |
|---|---|---|---|---|---|---|
| 1 | `bearable` | 13 (rank 1 on dimension A) | **Complete with inspection limitation** | 2026-10-05 | 2026-10-06 | Phase 2A not performed — no lawfully authorized artifact. Public-claims research unaffected. |
| 2 | `welltory` | 14 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). Vendor's **public research repos and datasets were inspected** and their citations independently verified. |
| 3 | `exist` | 11 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). The vendor's **public API reference was inspected in full** — a stronger artifact than Welltory's, because it is the analysis layer's own output contract. |
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
| **Candidate 3 of 10 — `exist`** | **Complete (with a partial inspection)** | See §Candidate 3 |
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

## Candidate 3 — `exist`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `exist/exist_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 19 sections, 37-row evidence table (E-1…E-37) |
| Authorized implementation inspection | `exist/exist_IMPLEMENTATION_INSPECTION.md` | Written — **PARTIALLY PERFORMED**: the public API reference was inspected as the analysis layer's output contract; app binary not inspected; no runtime observation |
| Candidate-specific Prompt 2 | `exist/PROMPT_2.md` | Written — based on `PROMPT2-TECHNICAL-SYSTEMS.md`, with the operating-model audit block from `PROMPT2-PRIVACY-AND-OPERATING-MODE.md` |
| Comparison report (Prompt 2 run) | `exist/exist_COMPARISON.md` | Written — 10 sections |

**Verification command:**

```sh
ls -l docs/app_comparison/exist/
```

All four files must be present and non-empty.

**Outcome in one line.** Exist is a **partial competitor to AnxietyWatch v1** (same analysis-layer idea —
subjective ratings beside objective data, personal baselines, correlation as the central claim, automatic
weather and air pressure, medications tracked as data; but no sensor of its own, no CPAP, no SpO₂, no
respiratory rate, no panic-time capture, no medication semantics, no alarm, no local operation, weekly
analysis latency), **not a competitor to AnxietyWatch2**, and the **most plausible integration partner and
methodological precedent** on the shortlist: a privacy-first, no-AI, no-analytics, bootstrapped product
with published revenue (~940 paying users) that independently shipped the missing-data behaviour v2
specifies.

**What made this candidate different.** Exist is the first candidate whose **implementation surface is
the product's analytical contract**. The public API reference ships real response objects, and the
correlation record carries a coefficient, a **p-value**, a **sample size** (`period`, 309 in the vendor's
example), a lag `offset`, a 1–5 `stars` confidence, plain-language strength and confidence bands, and the
sentence fragments (`second_person_elements`) the rendered claim is built from. This is genuine static
evidence about the analysis layer — not marketing, and not a dataset that merely corroborates marketing.

**Three findings worth carrying forward**

1. **Exist excludes absent data instead of coercing it** — "at least three weeks' worth of data for an
   attribute, **excluding days with no data (shown as 'N/A')"**, and the API returns sparse date/value
   pairs with **no zero-fill and no sentinel**. Independently arrived at and shipped since 2014. It is
   external evidence that v2's direction works at consumer scale.
2. **Exist separates confidence from strength**, and says why in user-facing terms: "a very strong
   relationship … with a **very low confidence if we don't have much data**." The documented AnxietyWatch
   v1 correlation engine reports which metrics influence anxiety without exposing sample size or a
   confidence axis. This is the cheapest adoptable idea in the report.
3. **The pairwise ceiling is structural.** The correlation schema has exactly two attribute slots and no
   covariate field; the vendor names confounding as *the user's* job. AnxietyWatch has the same conceptual
   exposure and should be as explicit about it.

**The one claim that is stronger than the implementation.** "Experiments" is presented as a capability,
but **no experiment object, endpoint or record exists in the API** — it is the correlation engine run
against a user-chosen pair. A designed experiment has a pre-registered hypothesis and a defined window; a
mined retrospective correlation has neither.

**The one concrete technical opportunity (assessed, not recommended).** Exist publishes a supported
**write** API ("Attribute ownership", "Creating attributes", "Updating values"), and its own help centre
names it as the supported alternative to file import. A nightly CPAP-derived or HRV-derived summary could
reach a user's existing Exist account and be correlated against their mood series with no reverse
engineering. It also inverts v2's local-first claim and adds a cloud third party to the data path.
Recorded as an option with its cost, not as a recommendation.

## Residual uncertainty — `exist`

Carried forward from `exist/exist_COMPARISON.md` §9 and `exist_IMPLEMENTATION_INSPECTION.md` §7. The app
binary was not inspected and **no API endpoint was called**, so every API finding is a documented contract
rather than observed behaviour.

- **E1** How the absolute "won't ever sell your data" claim and the privacy policy's acquisition
  carve-out ("may be transferred as part of the sale, merger, or dissolution") reconcile in practice, and
  what notice and opt-out a user actually receives.
- **E2** Whether absent days are dropped pairwise inside the correlation, and how intermittent coverage
  affects `period` and `stars`. The exclusion rule is documented; its statistical consequence is not.
- **E3** The mechanism behind "Exist can even uncover what happens when you don't track a tag" — is tag
  absence a categorical level, and is it reported with the same strength/confidence treatment?
- **E4** Why the Play data-safety declaration does not list **Health and fitness** among collected types
  despite the app's entire function. Most likely a separate Health Connect declaration regime (the policy
  carries an explicit Limited Use statement); **not verified**, and recorded without accusation.
- **E5** The full `value_type` enumeration, and **whether one canonical unit per attribute is enforced** —
  can two services supply different units to the same attribute? The terminology reference page 404'd.
  This is the sharpest technical question against AnxietyWatch2's one-unit-per-type rule.
- **E6** The numeric thresholds behind `strong=1` and `confident=1`, whether `p < 0.05` is implied, and
  **whether multiple comparisons are corrected** given the vendor's own example returns `count: 479`.
- **E7** Whether the PDF export carries correlations with strength and confidence, or only charts and
  averages.
- **E8** Whether insight texts ("Last night's sleep was surprisingly long") come from a documented
  personal-baseline rule. The baseline mechanism is inferred from phrasing, not published.
- **E9** The actual hosting jurisdiction — only "outside the United States" is stated.
- **E10** What `offset` represents and how many lags are tested. Marketing says "two consecutive days";
  the API exposes an offset field.
- **E11** Refund policy, and how the up-front credit-card requirement interacts with the 30-day trial in
  jurisdictions with cooling-off rights.
- **E12** Whether an attribute **freezes at its last value**, goes to "N/A", or is disabled when its source
  stops reporting. **This is the decisive test of whether Exist's missing-data discipline is principled or
  merely convenient** — a freeze would silently manufacture stale data, the exact failure v2's three-state
  rule exists to prevent.

**Most promising untried step.** Retrieve `developer.exist.io/reference/object-types-and-terminology/`
(404 at access, possibly moved) and the public roadmap. The former would close E5 — the unit model — which
is the one place Exist's generic attribute design is directly comparable to AnxietyWatch2's registry.

**Process-level note.** The vendor-published-artifact inspection mode now has two distinguishable
strengths, and both should be checked for every remaining candidate:

- **Corroborating artifacts** (Welltory: CC0 datasets, research repos) — evidence *about* the vendor's
  claims, from outside the product.
- **Implementing artifacts** (Exist: the API reference) — the product's own output contract, i.e. genuine
  static evidence *of* the implementation.

Exist also supplies a **fourth** authorizable surface worth checking on every candidate: a public,
self-generated business disclosure (revenue and paid-user counts). It is the only scale figure on the
shortlist that is not a download count or a marketing claim.

## Residual uncertainty — `bearable`

None of these can be closed without an authorized artifact inspection.

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
2. **Inspection authorization policy.** Still open for commercial candidates with no published surface
   of any kind. Options, in ascending order of owner involvement:
   - **a.** Inspect only open-source candidates (`oscar`, `nightscout`, `homeassistant`) from published
     source, **plus** whatever a vendor publishes itself (public API references, research repos and
     datasets, public roadmaps, self-published business figures). **No owner decision needed**; this is
     the default actually in force, it is lawful today, and it has already produced substantive static
     findings for two of the three candidates processed (Welltory's CC0 datasets, Exist's analysis-layer
     contract).
   - **b.** Authorize installing store apps on a disposable emulator under an owner store account,
     without creating any target-app account. Unlocks first-run permissions, bundled SDKs, manifest
     surface and pre-account network destinations.
   - **c.** Authorize creating a throwaway target-app account under a sheltering alias on the
     disposable emulator, to observe post-login sync behaviour. The brief currently forbids this; it
     would need an explicit recorded decision.
   - **d.** Accept public-claims-only for commercial candidates with no published surface, and mark
     Phase 2A *not performed* in each comparison report.

   **Running assessment.** Options **(b) and (c) would each have to justify themselves against option
   (a)**, which is now producing more than expected: because the strongest candidates are
   privacy-conscious independent products, they tend to publish API references and business figures
   precisely because they are small. Across three candidates the blocker for store-only apps has been
   consistent and is not expected to change: the binary cannot be lawfully obtained without an owner
   decision, and no candidate so far has needed it to reach a defensible verdict.
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
- Before concluding that a candidate is uninspectable, **check every vendor-published surface**: a
  GitHub organisation, an open-source component, a public API reference, published datasets or research
  artifacts, a public roadmap, and self-published business figures. Two of the three candidates
  processed so far yielded substantive static findings this way.
- **A documented contract is not an observed behaviour.** API-reference findings are labelled as
  documented, never described as verified or working, and no vendor endpoint is called without an
  explicit authorization decision.
- **A vendor publishing its own numbers does not make those numbers independent.** Self-published
  revenue and scale figures are recorded as vendor-published; vendor-authored research is never
  presented as third-party validation.
