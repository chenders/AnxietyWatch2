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

**Active candidate: none.** Candidates 1–4 are complete. The next candidate is `#5 sleephq`, which is not
started and must not be begun until this file is committed.

| # | Slug | Score | Status | Started | Completed | Blocker |
|---|---|---|---|---|---|---|
| 1 | `bearable` | 13 (rank 1 on dimension A) | **Complete with inspection limitation** | 2026-10-05 | 2026-10-06 | Phase 2A not performed — no lawfully authorized artifact. Public-claims research unaffected. |
| 2 | `welltory` | 14 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). Vendor's **public research repos and datasets were inspected** and their citations independently verified. |
| 3 | `exist` | 11 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). The vendor's **public API reference was inspected in full** — a stronger artifact than Welltory's, because it is the analysis layer's own output contract. |
| 4 | `oscar` | 11 | **Complete with full implementation inspection** | 2026-10-06 | 2026-10-06 | None. GPLv3 source inspected at a pinned commit; no runtime observation (nothing was executed). |
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
| **Candidate 4 of 10 — `oscar`** | **Complete (full implementation inspection)** | See §Candidate 4 |
| Candidates 5–10 | **Not started** | No files created. No research begun. |
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

## Candidate 4 — `oscar`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `oscar/oscar_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 17 sections, evidence table **O-1…O-50h** |
| Authorized implementation inspection | `oscar/oscar_IMPLEMENTATION_INSPECTION.md` | Written — **inspection PERFORMED**, full GPLv3 source read at a pinned commit |
| Candidate-specific Prompt 2 | `oscar/PROMPT_2.md` | Written — `PROMPT2-EVIDENCE-FIRST.md` base plus a `PROMPT2-TECHNICAL-SYSTEMS` supplementary block |
| Comparison report (Prompt 2 run) | `oscar/oscar_COMPARISON.md` | Written |

**Verification command:**

```sh
ls -l docs/app_comparison/oscar/
```

**Inspection mode:** **Performed** — the first candidate on the shortlist to reach this mode. The entire
product source was obtained from a public GPLv3 repository and read at a pinned commit. **Nothing was
executed**: no binary installed, no data imported, no report generated, no CSV inspected as output. The
runtime-observation count for this candidate is **zero**.

| Artifact | Value |
|---|---|
| Repository | `https://gitlab.com/CrimsonNape/oscar-sql.git` (the 2.0 SQL line) |
| Commit | `28df2e46e35e6260e186fced997edadcf001cc24`, 2026-10-04, branch `master`, shallow clone (59 MB) |
| Licence | GNU GPL v3 (`COPYING`), confirmed in the tree |
| Ancillary (metadata only) | `gitlab.com/pholy/OSCAR-code` — the 1.x line, not inspected |

**Four findings, in order of consequence.**

1. **v1's CPAP importer is coupled to OSCAR's report vocabulary by column position, and that vocabulary
   has already changed.** v1 parses the by-session export at hardcoded indices 0–20 behind a
   `count >= 21` guard and detects the format with the six-token prefix `"date,start,ahi,rdi,oa"`. The
   report v1 was written against — 22 columns, matching OSCAR 2.0.1 token for token, per the vendor's own
   design record for the release note *"Add more fields to Session/by Session and Daily/by Day CSV
   reports"* — is **27 columns** in the inspected source, with `OAHI`, `CAHI`, `A`, `OH` and `CH`
   inserted mid-header. The detector still fires (because `OAHI` begins with `OA`) and the guard still
   passes (because `27 >= 21`), so every field from index 4 onward is silently misassigned — including
   index 20, where **`Leak_95th` is read as `Hours_Used`**, turning a 6.5-hour night with a leak 95th of
   12.4 into 12.4 hours of therapy. v1 is correct against 2.0.1; the break arrives with the next OSCAR
   release. That the 27-column form is unreleased is an **inference** from the 2026-08-06 specification
   date and the absence of any related entry in the 2.0.1 notes, not from running the 2.0.1 binary. The
   latent-break conclusion holds either way (§3.2 of the comparison).

2. **OSCAR answers the invalid-data question three different ways in one program.** The ResMed EDF loader
   substitutes the *previous sample* for an end-of-data sentinel (`c = last`, `resmed_loader.cpp:4361–4367`,
   comment: "this change stops false error values from deing displayed"). The newer Apple Health loader
   rejects an out-of-range value, **names the reason and the value**, and drops it
   (`applehealthDataParsing.cpp:337–446`; "0 and negatives are no-reading sentinels, not measurements"),
   with parse-time unit conversion and rejection of unknown units. The summary store writes the number
   **`0.0`** for absent SpO2/pulse and masks it only at export with `NULLIF(col, 0)`. The newest path
   matches this project's rules almost exactly; the oldest violates them in the precise way the rules
   forbid; the store's violation is invisible to anything reading the published database.

3. **v1's leak number is not the machine's number.** OSCAR parses the device's `Leak.95`
   (`resmed_loader.cpp:1681–1691`) and then does not persist it (`:2540–2544`) — percentiles are
   recomputed from the time-series. The vendor's own method review
   (`Notes/Developer Notes/LEAK_PERCENTILE_METHOD_REVIEW_2026-04-16.md`) reproduces a **2.0×**
   disagreement with the device on one night and names a gain-handling defect in `Day::percentile` plus
   a time over-accumulation risk in `updateCountSummary()`. The review's own neutral framing is adopted:
   *"a product-behavior mismatch, not necessarily a pure math bug."* **Scope limit:** v1 also has an EDF
   leak path, so this qualification applies to the OSCAR-imported values, not to every leak number in v1.

4. **OSCAR is a dependency and a reference, not a competitor, and v2 inherits none of its semantics.**
   v2's AirSense 11 registry source declares 17 instantaneous signal types, there is **no AHI type and no
   percentile computation anywhere in the v2 tree**, so the leak-percentile question cannot propagate.
   One precision, to avoid overstating the gap in v2's favour: v2 *does* decode and classify the device's
   own scored events (`events.rs:19–51`, incl. hypopnea/central/obstructive/RERA/CSR), but the
   classification is read by nothing — only the raw event name feeds the session lifecycle
   (`as11_session.rs:156,168`). That is **implemented and unwired**, not a shipped analysis capability.
   The overlap with OSCAR is the *concept* of leak, not the implementation, the column, or the definition.

**Where OSCAR is ahead of both AnxietyWatch versions** (stated without hedging): portable exit and
interoperability (published schema, Python reader, three CSV modes); per-session fidelity markers
(`summary_only`); an implemented typed/reasoned/drift-modelled/reversible device-clock correction ledger;
~19 device loaders; and an unambiguously free, licence-enforced, zero-telemetry operating model.

**Where v2 is ahead, and where it is not.** v2's rules and implemented rejection behaviour are better:
rejections name their blocker **and the value**, uncapped, with a test asserting that **absent is not
invalid** (`bp_build.rs:127`), and the store cannot fabricate a value. But v2 is **not yet better on
durability of the rejection record** — rejections are `eprintln!`-ed by the collector, exactly as OSCAR's
are `qWarning`-ed, and v2's durable attempt ledger is specified in `cpap-ezshare-ingest` (invariant 7) at
**0 of 6 tasks built**. Likewise v2's versioned, append-only clock-correction rule is **better designed
than OSCAR's mutable ledger and is specified, not built**. These are recorded as live v2 gaps relative to
OSCAR, not as recommendations.

**Verification method for this candidate.** Every finding above was traced to a named file and line in the
pinned commit, and the reproduction commands are in
`oscar/oscar_IMPLEMENTATION_INSPECTION.md` §10. The v1-side claims were read from the v1 tree at the paths
named in §3.1 of the comparison. No claim rests on a model statement alone.

## Residual uncertainty — `oscar`

- **O1** Whether v1's OSCAR-CSV import has ever been exercised against a real OSCAR export. The header
  correspondence is verified from both source trees; no end-to-end run was performed and no evidence of
  one was found.
- **O2** Which header OSCAR **2.0.1** actually emits. Established from the vendor's design record for the
  report that release shipped; **not** confirmed by running the binary. The 27-column form is read from the
  inspected commit. The "unreleased" conclusion is an inference.
- **O3** Whether v1's detector matches the 27-column header in practice. The prefix match follows from
  string comparison; it was not executed. A 27-column fixture would settle it empirically.
- **O4** Whether the summary-store `0.0` sentinel reaches v1 in any form. The `NULLIF` masking is read
  from the report SQL; no real import was performed.
- **O5** The invalid-data policy of the **other seventeen loaders**. The three-policy finding rests on the
  ResMed EDF, Apple Health and summary-store paths — the three that touch v1's data — and is **not** a
  claim about OSCAR as a whole.
- **O6** Whether the specific leak defects named in the 2026-04-16 review are still present. The *method*
  (percentiles recomputed, not read) was confirmed in the current tree; the gain-handling and
  over-accumulation defects were not re-confirmed.
- **O7** Whether the localised-header fragility reproduces. Read from `tr()` in the dialog exporter; no
  localised build was run. It does not affect the ORF report's fixed aliases.
- **O8** The status of the five unmigrated loaders (`mseries`, `dreem`, `somnopose`, `zeo`,
  `serialoximeter`) in 2.0, and the 1.7.2 line in full.
- **O9** Whether the wider OSCAR Team reviews the 2.0 line, which is a short-history fork under an
  individual namespace linked from the official download page.
- **O10** Any scale figure at all. No download, install or active-user number exists; star and fork counts
  are not user counts and were not used as such.
- **O11** Whether "quarantine", as this project's rules state it, is implementable in v2's store. What is
  established: v2 returns named rejections and logs them; it does not persist them.
- **O12** Whether emitting a CSV that OSCAR can read would create a GPL obligation for v2. Assessed as
  feasible in principle and already bounded by v2's stated clean-room position
  (`cpap-ezshare-ingest/design.md` §11: "no GPL implementation code is copied"); **not legally reviewed**,
  and no recommendation is made.

**Most promising untried step.** Compile OSCAR at the pinned commit, run the Apple Health test suite, and
feed v1's importer a 27-column fixture. That single sequence would move the two decisive findings — the
invalid-data policy contrast and the silent column misassignment — from *static* to *observed*. It was not
executed, and nothing in the `oscar` package should be read as though it was.

**Process-level rule added by this candidate.** When a shortlisted product is a **dependency** of
AnxietyWatch v1 rather than merely a competitor, two checks become mandatory and were not previously part
of the method: (a) test the **format contract for drift** — compare the header or schema v1 was written
against with the header or schema the upstream currently emits, token by token; and (b) state whether the
inheritance is **vocabulary** (breaks when upstream changes output) or **semantic** (silently becomes a
frozen definition of the upstream's older understanding). A dependency's defects are v1's defects, and
their scope must be bounded explicitly in the comparison report.

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
   precisely because they are small. Across four candidates the blocker for store-only apps has been
   consistent and is not expected to change: the binary cannot be lawfully obtained without an owner
   decision, and no candidate so far has needed it to reach a defensible verdict. **`oscar` resolved the
   question from the other direction** — as a GPLv3 project it is fully inspectable with no owner decision
   at all, which removes any argument that an authorization decision is needed *in order to* inspect
   source when source is already published.
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
  GitHub/GitLab organisation, an open-source component, a public API reference, published datasets or
  research artifacts, a public roadmap, and self-published business figures. **Three of the four
  candidates processed so far yielded substantive static findings this way**, and one (`oscar`) reached a
  full implementation inspection from a GPLv3 repository.
- **When a candidate is a dependency of AnxietyWatch v1, not merely a competitor, two extra checks are
  mandatory:** test the **format contract for drift** (compare the header or schema v1 was written
  against with what the upstream currently emits, token by token — a length check is not a schema check),
  and state whether the inheritance is **vocabulary** (breaks when upstream changes output) or
  **semantic** (silently freezes the upstream's older understanding). A dependency's defects are v1's
  defects, and their scope is bounded explicitly rather than left implicit.
- **An inspection mode is not an observation.** *Performed* means the implementation was read. Running
  the software requires a separate, explicit authorization decision, and static findings and runtime
  observations are never merged into one evidence class.
- **A documented contract is not an observed behaviour.** API-reference findings are labelled as
  documented, never described as verified or working, and no vendor endpoint is called without an
  explicit authorization decision.
- **A vendor publishing its own numbers does not make those numbers independent.** Self-published
  revenue and scale figures are recorded as vendor-published; vendor-authored research is never
  presented as third-party validation.
