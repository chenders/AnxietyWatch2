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

**Active candidate: none — the shortlist is exhausted.** Candidates 1–10 are complete. No further candidate
may be started without an owner decision; see **Owner decisions pending** §3.

| # | Slug | Score | Status | Started | Completed | Blocker |
|---|---|---|---|---|---|---|
| 1 | `bearable` | 13 (rank 1 on dimension A) | **Complete with inspection limitation** | 2026-10-05 | 2026-10-06 | Phase 2A not performed — no lawfully authorized artifact. Public-claims research unaffected. |
| 2 | `welltory` | 14 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). Vendor's **public research repos and datasets were inspected** and their citations independently verified. |
| 3 | `exist` | 11 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). The vendor's **public API reference was inspected in full** — a stronger artifact than Welltory's, because it is the analysis layer's own output contract. |
| 4 | `oscar` | 11 | **Complete with full implementation inspection** | 2026-10-06 | 2026-10-06 | None. GPLv3 source inspected at a pinned commit; no runtime observation (nothing was executed). |
| 5 | `sleephq` | 12 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binaries not inspected (closed source, store-only, account-gated; no lawful route to the APK). No vendor source exists. The vendor's **published OpenAPI specification was inspected in full** — 21 paths, 11 models, and a documented hash algorithm. **No endpoint was called.** |
| 6 | `nightscout` | 15 | **Complete with full implementation inspection** | 2026-10-06 | 2026-10-06 | None. AGPL-3.0 source inspected at a pinned commit; no runtime observation (nothing was executed, no instance built or contacted). |
| 7 | `empatica` | 14 | **Complete with artifact-and-regulator inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, **prescription-gated**; no lawful route to the binary, and no Android artifact located at all). No vendor source exists. **No API was inspected** — the Cloud API documentation is supplied to clients only. The inspection mode is a new one: **the vendor's own published artifacts plus an independent regulator's record of the vendor's own submission.** |
| 8 | `oura` | 13 | **Complete with documented-contract inspection** | 2026-10-06 | 2026-10-06 | No binary and no source inspected. The Terms **prohibit** reverse engineering and circumventing subscription restrictions, so Phase 2A was closed by contract and not by accident. The vendor's **published OpenAPI 3.1 contract was inspected in full** — 72 paths, 75 schemas, 274 resolvable `$ref`s — together with its legal, trust and research surfaces. **No endpoint was called, including the sandbox.** |
| 9 | `homeassistant` | 13 | **Complete with full published-source inspection** | 2026-10-06 | 2026-10-06 | Apache-2.0, so Phase 2A ran on **published source** at tag `2026.9.4` (`9212531f…`) with no owner decision. The state machine has first-class `unknown`/`unavailable`; the **alert path has no branch for either** and ends the alert — sending the resolution message — when the watched entity goes absent. Raw history is **hard-deleted** after `purge_keep_days` (default 10) with no tombstone; hourly statistics are **upserted**. `components/sensor/recorder.py:239` **does not parse** under Python 3, verified four ways, and the module is imported by its own test file. **No binary, no running instance, zero runtime observations.** |
| 10 | `mindshift` | 12 | **Complete with published-client and public-records inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only; no lawful route to the binary). No vendor source and **no API contract** exists. The inspected surface is the vendor's **published web client** (741,896-byte served bundle, read as text) plus public records: two storefront listings, the App Store privacy panel, the independent registration **NCT04562948**, the peer-reviewed paper **PMID 40656972** and its **corrigendum PMID 40655914**. **No endpoint was called and no runtime was observed.** |
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
| **Candidate 5 of 10 — `sleephq`** | **Complete (with a partial inspection)** | See §Candidate 5 |
| **Candidate 6 of 10 — `nightscout`** | **Complete (full implementation inspection)** | See §Candidate 6 |
| **Candidate 7 of 10 — `empatica`** | **Complete (artifact-and-regulator inspection)** | See §Candidate 7 |
| **Candidate 8 of 10 — `oura`** | **Complete (documented-contract inspection)** | See §Candidate 8 |
| **Candidate 9 of 10 — `homeassistant`** | **Complete (full published-source inspection)** | See §Candidate 9 |
| **Candidate 10 of 10 — `mindshift`** | **Complete (published-client and public-records inspection)** | See §Candidate 10 |
| Pilot review by owner | **Completed** | Owner instructed the run to continue; mode switched to full sequential |
| Full sequential run of candidates 3–10 | **Authorized and completed** | One candidate at a time; shortlist exhausted |

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

## Candidate 5 — `sleephq`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `sleephq/sleephq_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 18 sections, evidence table **S-1…S-60** |
| Authorized implementation inspection | `sleephq/sleephq_IMPLEMENTATION_INSPECTION.md` | Written — **documented-contract mode**: the published OpenAPI specification read in full. No source, no binary, no endpoint called |
| Candidate-specific Prompt 2 | `sleephq/PROMPT_2.md` | Written — `PROMPT2-TECHNICAL-SYSTEMS.md` base plus the `PROMPT2-PRIVACY-AND-OPERATING-MODE` supplementary block |
| Comparison report (Prompt 2 run) | `sleephq/sleephq_COMPARISON.md` | Written — 14 sections |

**Verification command:**

```sh
ls -l docs/app_comparison/sleephq/
```

**Inspection mode:** **documented-contract inspection — performed for the specification, not performed for
the binaries or any running service.** The vendor's OpenAPI document (21 paths, 11 models, OpenAPI 2.0,
SHA-256 `9a4305e6…536a4`) and its rendered reference were read in full, together with the marketing site,
shop pages, support/FAQ pages, privacy policy, terms, supported-devices page, App Store metadata, Play
listing and two founder-authored articles. **Nothing was executed. No endpoint was called. No account,
no OAuth client, no trial. The runtime-observation count for this candidate is zero.**

This is the **second candidate to reach the documented-contract mode** (`exist` was the first), and the
second for which no vendor source exists: no SleepHQ-authored code repository was found, so the
source-inspection mode available for `oscar` is unavailable here. The third-party projects that exist (a
Raspberry Pi uploader, an API client) are unaffiliated user work and were **not** treated as vendor
evidence.

**Five findings, in order of consequence.**

1. **SleepHQ publishes the derived values AnxietyWatch2 refuses to compute — and its founder publicly
   argues its own headline number is unreliable.** The session model carries `ahi`, `oa_count`,
   `ca_count`, `h_count`, `ua_count`, `pressure_set_95`/`ipap_set_95`/`epap_set_95`/`flow_limit_95`,
   `duration_90`, `drop_90`, `drop_3`, `drop_4` and `o2_score` `[DOCUMENTED CONTRACT]`. Simultaneously, the
   founder's two articles state that CPAP machine AHI under-counts hypopnoeas, that "if the device scored
   an AHI of zero, it could actually be closer to a 10", and give a worked case of machine AHI 6.32
   versus a five-tech manual review of 14.3. **Those articles cite no study, no author, no journal and no
   link** — they are unsourced vendor claims, in the vendor's own interest, and are **not** independent
   validation of anything. That both statements coexist is the candidate's most interesting fact, and it
   is recorded without resolving it in AnxietyWatch's favour. Separately, `_95` percentiles, the `drop_*`
   family and `o2_score` are published with **no documented estimator, units or threshold** — an
   undefined number that looks like a measurement.

2. **The transport is the SD card, not the radio — and the automated path costs real money.** AirSense 11
   support is explicit (8 variants), but ingestion is manual card copy, a phone OTG reader, or a
   **AUD $399** Magic Uploader Pro whose mechanism is a **Wi-Fi SD card** and which **requires a Pro
   subscription**. Oximetry is single-sourced by the vendor's own statement: "**only SleepHQ branded
   devices are compatible**" — a **AUD $299** O2 Ring Pro. This is the clearest product-philosophy
   divergence from AnxietyWatch2 in the package: V2 streams over BLE with no vendor hardware and targets
   a replaceable EMAY `SleepO2`.

3. **SleepHQ has shipped the two features V2 has only specified, and its release notes price them.** A
   "sleep alarm" is shipped, with "**Fixed three crashes in the sleep alarm**" and "You're now told when a
   night's recording fails to save" in the Android release notes for 2026-09-13 `[STATIC]`. Clinician
   sharing is shipped (share links, clinical accounts, an enterprise tier). V2's multi-modal alerting,
   supervised alarm, hardware dead-man switch, PDF reports and third-party read access are all
   `[V2 target]`. **Recorded as a clear competitor advantage in production**, with the engineering cost
   evidenced by the vendor's own defect history.

4. **The documented contract has undocumented value domains and an undocumented AI feature.** `status`,
   `progress`, `usage`, `omitted` and `teams_type` are typed and **undefined**; `o2_score` has no method;
   `duration_90`/`drop_90`/`drop_3`/`drop_4` have no units or equations; `fingerprint_base64` has no
   published algorithm; `active_energy_joules` is accepted by both journal write operations but **does not
   appear in the journal serializer**; and `time_over_leak_threshhold` is misspelled in the vendor's own
   schema, so the misspelling is the contract. Pro includes "Custom PDF reports with **AI Insights**", a
   four-word description with **no model, provider, processing location or opt-in status published
   anywhere** — the largest privacy unknown in the candidate.

5. **Two vendor surfaces conflict, and one makes an absolute security claim its own policy contradicts.**
   The FAQ says data is "encrypted both at rest and in flight ensuring your data is **completely secure at
   all times**"; the privacy policy says the vendor is "**progressing towards** SOC 2 Type II" with HIPAA
   controls "**designed to meet**"; the Play panel says only "encrypted in transit". The most
   conservative statement governs: SleepHQ is recorded as **not claiming SOC 2 Type II certification**.
   Separately, the pricing page lists the Magic Uploader on the **free** tier while the shop states "you
   need to have a SleepHQ Pro subscription" — the **second unreconciled conflict between two current vendor
   surfaces** in this package (after Bearable).

**Where SleepHQ is ahead of both AnxietyWatch versions** (stated without hedging): device breadth (~19
families, with the unsupported machines listed too); an SD card as a store-and-forward buffer, which
survives a dropped radio link in a way V2's BLE stream does not; shipped alerting; clinician sharing;
an integration-friendly API included in the Pro tier with a precisely documented hash algorithm; and an
unusually candid rate-limit disclosure that documents its own fixed-bucket edge case.

**Where v2 is ahead, and where it is not.** v2's oximeter is replaceable and its data never leaves the
owner's hardware; its store is append-only with tombstones and a versioned clock transform; its
missingness rule (amendment 01: never `locf()`/`interpolate()`, gaps return as explicit NULLs) is
**documented**, where SleepHQ's is not documented at all. But v2 is **not ahead on the analysis layer**:
it computes no AHI and no percentile anywhere, and it decodes and classifies the device's own scored
events (`events.rs`, `EventKind`) while **reading that classification nowhere** — the collector passes
only the raw event name to the session lifecycle (`resmed_air11_collector.rs:536`,
`as11_session.rs:156,168`). That is **implemented and unwired**. The one occurrence of the string "AHI" in
the Rust sources is a doc comment recording the opposite — `EventKind::Rera` "contributes to RDI, not
AHI". SleepHQ publishes an index V2 does not, and V2's refusal has no shipped replacement yet.

**Additional v2 gap recorded for symmetry.** V2 itself drops raw `samples` after **2 years** (amendment
01) while keeping a `samples_1min` aggregate, so SleepHQ Pro's "unlimited data" claim is a broader
retention claim than V2's own rule. V2's file-level provenance is also weaker than SleepHQ's: the attempt
ledger is `[V2 target]` (`cpap-ezshare-ingest` invariant 7, **0 of 6 tasks built**), and rejection reasons
are currently printed to stderr and not persisted (`crates/vitals-collectors/src/bin/h10_collector.rs:330–332`).

**Verification method for this candidate.** Every documented-contract claim was read from
`https://sleephq.com/api/swagger.json` and re-checked by enumerating paths and serializer attributes
(reproduction commands in `sleephq_IMPLEMENTATION_INSPECTION.md` §9). Every v2-side claim was re-verified
against this tree before being written — including the `EventKind` variant list, the session lifecycle
match arms, the 17-type `resmed_air11` registry entry, the absence of any AHI or percentile computation,
the `Rera`/AHI doc comment, the amendment-01 retention and gap rules, and the change-status counts. No
claim rests on a model statement alone.

## Residual uncertainty — `sleephq`

- **S1** Whether the session `ahi` is **relayed from the machine** or **re-derived** by SleepHQ. Not
  documented. Decisive for whether the founder's criticism of device AHI applies to the platform's own
  number, and therefore for finding 1.
- **S2** The contents of the eleven `jsonb` summary blocks (`usage_summary`, `ahi_summary`,
  `pressure_summary`, `leak_rate_summary`, `flow_limit_summary`, `resp_rate_summary`, `epap_summary`,
  `machine_settings`, `pulse_rate_summary`, `spo2_summary`, `movement_summary`). The per-night analysis
  depth is unknowable without them.
- **S3** The definitions, units and equations behind `o2_score`, `duration_90`, `drop_90`, `drop_3` and
  `drop_4` — five oximetry-derived quantities published as bare names.
- **S4** What `_95` actually estimates: the estimator, the window, the interpolation, and whether it is
  per session or per night.
- **S5** Whether the free tier **deletes or merely hides** data older than 30 days. The vendor states only
  a "view" limit. The comparison to Welltory's deleting free tier turns on this.
- **S6** What "AI Insights" is: model, provider, whether data leaves SleepHQ's infrastructure, and whether
  it is opt-in. A Pro-tier feature name with no published description, in a platform holding respiratory
  and oximetry data, and alongside a policy permitting "anonymised statistical analysis and research".
- **S7** Whether the "sleep alarm" is **supervised**, and whether it distinguishes an unmeasured or
  failed state from measured-and-good. No SleepHQ source states either.
- **S8** What a share link actually contains. "No personally identifiable information is shared" is
  unfalsifiable from outside; no share link was created.
- **S9** The meaning of `omitted` — the one field that might carry a three-state "this session was
  excluded" meaning — and of the `ImportSerializer.status`/`progress` domains.
- **S10** The night-boundary convention: whether a "machine date" is midnight- or noon-derived, and
  whether it follows the device clock, the user's `time_zone` or the team's `time_zone`.
- **S11** The `fingerprint_base64` algorithm, and why `content_hash` is documented as MD5.
- **S12** Whether `active_energy_joules` is returned by any endpoint, or is genuinely write-only as
  documented.
- **S13** The clinic/enterprise API surface, documented as existing and explicitly unpublished.
- **S14** Whether the platform's own position on AHI is coherent with advertising "track your AHI". Not
  resolvable from outside; recorded as a tension between two vendor statements, not as an inconsistency
  proven in behaviour.
- **S15** All hardware properties — 1-second sampling, 24-hour battery, 12-month warranty, the vibration
  alert, the Wi-Fi SD card mechanism, the 10-minute sync cadence — are vendor statements about devices
  nobody in this exercise has held.
- **S16** The claimed scale (100,000 patients / 160 countries) against Play's 10K+ Android installs. The
  gap is reconcilable by platform mix but that reconciliation is an **inference**, not a measurement.
- **S17** Whether the O2 Ring Pro holds any regulatory registration. No registry was searched; the
  "medical grade" description is uncited on the page fetched. **Unknown, not absent.**

**Most promising untried step.** A single authorised API session — one OAuth client, one read of
`GET /v1/machines/{id}/machine_dates` and one `GET /v1/machines/sessions/{id}` — would close S2, S4, S9
and S12 at once, and would convert the largest block of `unknown` in this candidate into documented
output. It requires an account, an OAuth client registration and an explicit authorization decision. It
was **not** performed and must not be performed without that decision.

**Process-level rules added by this candidate.** Two, both recorded in §"Rules this file enforces": the
documented-value-domain check, and the rule for handling a vendor's own material that undercuts the
product's headline claim.

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

## Candidate 6 — `nightscout`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `nightscout/nightscout_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 16 sections, evidence table **N-1…N-60** |
| Authorized implementation inspection | `nightscout/nightscout_IMPLEMENTATION_INSPECTION.md` | Written — **source-inspection mode, performed**: the AGPL-3.0 tree read at a pinned commit. No binary, no deployed instance, **nothing executed** |
| Candidate-specific Prompt 2 | `nightscout/PROMPT_2.md` | Written — `PROMPT2-TECHNICAL-SYSTEMS.md` base plus the `PROMPT2-PRIVACY-AND-OPERATING-MODE` supplementary block |
| Comparison report (Prompt 2 run) | `nightscout/nightscout_COMPARISON.md` | Written — 14 sections |

**Verification command:**

```sh
ls -l docs/app_comparison/nightscout/
```

**Inspection mode:** **source inspection — performed.**
`nightscout/cgm-remote-monitor` pinned at **`92d0834219aa771b5837dbcbf1baeb839a200cf6`**, which is the
release commit of **v15.0.8** (2026-09-04) — so every documentation-versus-code finding below is
*v15.0.8-docs versus v15.0.8-code*, not a stale-doc artefact. **AGPL-3.0**, Node `>=20.x`, Express +
MongoDB, 65 dependencies, ~15 MB / 764 files, 141 files under `tests/`. The public documentation site
(landing, security, Pushover, Admin Tools, reports) and the Nightscout Foundation site were fetched and
SHA-256 hashed. **Nothing was executed. No instance was built, deployed or contacted. The
runtime-observation count for this candidate is zero.**

This is the **second candidate to reach the source-inspection mode** (`oscar` was the first) and the
second to be a published-source project rather than a published-contract one. The distinction that
matters: for `sleephq` and `exist` the strongest available artifact was a *description* of the output; for
`nightscout` it is the *implementation*, which can be checked against its own description.

**Note on the fork count.** GitHub reports **73,382 forks**. The project's own deployment instructions
tell users to deploy from a fork, so most forks are installations, not contributions. It is recorded as a
**deployment artifact**, not as a community-size signal. `[INFERENCE]`

**Five findings, in order of consequence.**

1. **The shipped default publishes the data, and the mitigation is a notice only its own admin page
   shows.** `lib/settings.js:39` sets `authDefaultRoles: 'readable'`; `lib/authorization/storage.js:144`
   maps `readable` to `['*:*:read']`. The project's security documentation states it outright: "In a
default setup, anyone with your Nightscout URL can view your site." The project's own security audit
   recommended defaulting to `denied`; the recommendation was **not applied**. The one mitigation that
   exists is a persistent boot-time notice — "Nightscout readable by world" — injected at
   `lib/server/bootevent.js:149–152` with `persistent: true`, visible **only on that instance's own admin
   page**. `ALLOW_UNRESTRICTED_FRAME_EMBEDDING` also defaults to `true` (`lib/server/env.js:85`).
   Recorded as a **disclosed default**, not as a hidden one — the disclosure is real and is the reason
   this is finding 1 rather than a scandal.
2. **The unit rule, refuted in production, in a shipped clinical number.** `lib/report/reportclient.js:847`
   builds the report series in the user's **display** unit; `lib/report_plugins/dailystats.js:111` then
   feeds it to `(average * consts.MMOL_TO_MGDL + 46.7) / 28.7`, a formula written for mmol/L. At 150 mg/dL
   that is `(150 × 18.01559 + 46.7) / 28.7` = **95.8**, which the project's own 15.0.8 report documentation
   confirms: "the Daily stats estimated A1c is wrong when glucose is displayed in mg/dL (for example 95.8%
   instead of 6.9%)… The value shown in mmol/L is correct." Recorded with both halves: a real defect in a
   real derived value, **and** the correct disclosure of it. Time-in-Range in the same report is
   unit-consistent because its thresholds are scaled to match the series — the mechanism is narrow.
3. **"Not measured" renders as *current*, twice, in the project's own words.** `lib/plugins/timeago.js:103–104`
   carries `//assume current`; `lib/api/entries/index.js:66` carries `// bless absence with known type`.
   `lib/levels.js:39–48` collapses four of six severity levels into `'current'`. The stale-data alarm is
   **enabled by default** (15/30 min, `lib/settings.js:27–30`) — but it cannot fire when nothing has
   arrived, because `timeago.js:26` returns early when there is no entry, and `simplealarms.js:21`
   requires a current reading. A sensor that stops sending is therefore quieter than a sensor sending an
   old value. This is the failure mode AnxietyWatch2's three-state rule exists for, stated by the code
   rather than by a reviewer.
4. **Nothing in the write path validates physiology, and two alarm engines disagree about the bottom of
   the scale.** `lib/api3/generic/create/validate.js:15–24` checks the `identifier`; `lib/api3/shared/operationTools.js:56–75`
   checks only `date`, `utcOffset` and `app`. No range check on a glucose value, anywhere. Separately,
   `mgdl === 39` (Dexcom "LOW") is excluded from glucose alarms by `simplealarms.js:21` while the
   predictive engine uses `BG_MIN = 36` with the comment `//Not 39, but why?`, and the **report layer
   includes 39** (`reportclient.js:861–866` filters only `d.y < 39`). The same sentinel is special-cased
   differently in four places, and the alarm engine clamps its own forecast into `[36,400]`
   (`ar2.js:308`) rather than rejecting it.
5. **Three of the project's own *Complete* audited subsystems are described wrongly by their audits —
   while the requirements document beside them is right, and the code is right.** The January 2026
   security audit gives a token-derivation rule, a JWT signing key (`API_SECRET`) and a JWT lifetime (1 h)
   that the code does not implement (`lib/authorization/storage.js:187–193`, `lib/server/enclave.js:30,59`
   — 8 h, key from a generated `randomString` file, and `setJWTKey` is never called anywhere, so a
   redeploy that clears `node_modules` silently invalidates every issued JWT). The same audit's role table
   omits a role and misnames another, and it describes the brute-force delay list as auto-clearing after
   60 s when `lib/authorization/delaylist.js:43` is a single one-shot `setTimeout(clearList, 30000)` with
   no interval. Not an argument that the audit programme is worthless — an argument that **"the vendor
   audited it" is not a substitute for reading the code**, which is the load-bearing control in this
   repository too.

**Also recorded (secondary).** The **documented-default drift**: the published API3 OpenAPI description
says autopruning defaults to `DEVICESTATUS`=60, and the code has no default at all
(`lib/api3/generic/collection.js:32` reads the env var with no fallback). When pruning *is* enabled, its
`$or` filter (`srvCreated` OR `created_at` OR `date`) can delete a recently modified document, and deletes
tombstones too. **Two delete semantics coexist in one process**: API3 soft-deletes and keeps `history`,
while API v1/v2 and the admin tools hard-delete. **Any valid token can silence any alarm**: the alarm
socket has no rooms and `socket.join` appears nowhere in the tree, so every subscriber receives every
alarm, and the token-path `ack` handler performs no permission check (`lib/api3/alarmSocket.js:83–90`) —
which matches the README's own prose that alarms "can be cleared by any watcher of the data". **Alert
delivery needs four cloud services and three accounts** on the recommended path, with the project
documenting its own alarm-fatigue drawback. **An alarm plugin that throws is logged and dropped**
(`lib/plugins/index.js:196–202`) — no admin notice, no persisted state, although an admin-notification
channel exists and is used for security warnings. **There is no built-in backup and no point-in-time
recovery**, by the project's own audit. **There is no first-party mobile app**, and therefore no
first-party push channel. **The changelog is behind the release it ships with.**

**Positives recorded as positives.** The project's residual-risk statements are unusually honest and are
quoted as written: "**not officially approved or regulated for diabetes therapy and/or treatment in any
way**" and "**Do not rely only on Nightscout as the only way you have of knowing your blood glucose values
and trends**". The **sanitisation claim was checked and holds** — all six collections are purified on the
way in. A hypothesis that a `TODO` marked a broken fallback was **checked and refuted**: the code works,
and the comment is stale. The **`history` delta-sync surface** and the **published machine-readable API
description** are things AnxietyWatch2 does not have.

**Two hypotheses refuted per finding, and five hypotheses refuted for the candidate as a whole** — all
recorded in `nightscout_IMPLEMENTATION_INSPECTION.md` §5.20 and *not* carried into the findings. Had the
stale `TODO` been trusted, the package would have recorded a false defect **and missed the real one** (an
entry with no type is stamped with the caller's type rather than left unknown). The check-before-recording
rule is now applied per finding, not per candidate.

**Claim reconciliation:** 20 checkable claims — **14 match, 5 do not, 1 forward-looking**. Four of the five
failures are in the project's own security audit or its OpenAPI description, not in its user-facing
marketing.

**One internal contradiction inside one vendor's own documentation site, left unresolved:** the security
page states the system "has not been designed for scenarios where a third party operates a Nightscout
installation on behalf of others", while the landing page advertises **ten** commercial managed-hosting
providers with pricing links. The most conservative statement governs. A vendor that publishes no privacy
policy because it holds no data has delegated its entire privacy surface to those ten providers, and what
those providers commit to in writing is `[UNKNOWN]`.

**Process-level rules added by this candidate.** Four: the **documented-default-versus-code** check, the
**intra-docs-site-conflict** rule, the **display-unit-reaches-a-derived-statistic** finding class, and the
rule that an inspection mode is not an observation (already present; now exercised on a second candidate).

## Residual uncertainty — `nightscout`

- **N1** Whether the `readable` default is actually in effect for an anonymous caller in a default 15.0.8
  deployment, and whether the "readable by world" notice actually renders. Both are code readings.
  **Decisive for finding 1.** One unauthenticated request and one admin-page load against a throwaway
  local instance would settle it; that is a separate authorization decision and was not taken.
- **N2** Whether the A1c defect reproduces at this exact commit, and whether the fix has landed. The
  arithmetic is reproduced here from the code and matches the vendor's published figures; it was not run.
- **N3** Whether the stale-data alarm fires when data has flowed and then stopped (the code path suggests
  it can) as distinct from never having arrived (the code path returns early). This distinction is easy to
  misread and is the practical form of finding 3.
- **N4** Whether the three audit-vs-code discrepancies are known to the maintainers, tracked, or fixed on
  the default branch. Changes the weight of finding 5; public issue search would resolve it.
- **N5** Whether alarms actually cross group boundaries over the socket, and whether an arbitrary token
  actually silences an alarm. Both are code readings, both stated in prose by the README.
- **N6** What the ten advertised hosting providers commit to — location, encryption, retention, breach
  notification, sub-processors and governing law. No vendor endpoint or instance was contacted.
- **N7** What the storage purifier actually strips, and whether `activity` is treated identically to
  `entries`. The sanitisation claim was verified at the *import* level only.
- **N8** Whether one instance can serve more than one person, and what happens if it does. The documented
  model is one site per person; the ten hosting providers imply shared infrastructure.
- **N9** The end-to-end behaviour of the `39` sentinel across the alarm engine, reports and UI — the code
  shows four different special cases and the patient-safety consequence cannot be stated from source
  alone.
- **N10** What actually removes data in practice if the operator does nothing, given three surfaces with
  two answers. The remaining `lib/storage` index definitions (TTL indexes, migrations) were not read.
- **N11** Whether anything in the surrounding ecosystem (uploaders, followers, Loop/Trio/AAPS and their
  analytics) overlaps AnxietyWatch2's sleep, SpO₂, respiratory or context surfaces. The ecosystem was not
  surveyed, so the shortlist's premise that Nightscout is a *platform* analogue and not a *physiology*
  competitor is verified for the core project and **unverified for the ecosystem around it**.
- **N12** Whether `lib/plugins/{pushover,maker,speech,alexa,googlehome}.js`, `lib/server/{server,app,pushnotify}.js`,
  the `lib/reports` bodies, `lib/times.js` and the 141 test files change any conclusion above. They were
  not read; the report says so in its limitations section rather than implying coverage.
- **N13** Who governs the repository and the trademark, and the bus factor. `CONTRIBUTING.md` and the
  commit-authorship history were not analysed.
- **N14** Whether the January 2026 "agentic control plane" RFC in `docs/proposals/` is under active
  consideration. It is marked **Draft (2026 Proposal)** and is recorded once, in §15.3 of the claimed
  features, explicitly as an unshipped document and not as a feature.

**Most promising untried step.** A single authorised local instance on disposable infrastructure — one
unauthenticated HTTP request, one admin-page load, and one synthetic feed — would close N1, N2, N3 and N5
at once, converting the four decisive code readings of the most architecturally relevant candidate into
demonstrated behaviour. It requires building and running a Node + MongoDB application, which is a separate
authorization decision from reading a repository. It was **not performed.**

**Not to be mistaken for a feature.** Nightscout's `docs/proposals/agent-control-plane-rfc.md` (RFC v1.0,
January 2026, status *Draft*) proposes AI agents in insulin therapy management. It is a **proposal
document**, not an accepted plan and not shipped code, and neither this file nor the comparison treats it
as part of the product.

## Candidate 7 — `empatica`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `empatica/empatica_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 16 sections, evidence ledger **E-1…E-83**, consolidated conflict table, artifact SHA-256 table |
| Authorized implementation inspection | `empatica/empatica_IMPLEMENTATION_INSPECTION.md` | Written — **artifact-and-regulator inspection, performed**: vendor artifacts and the FDA record read. No binary, no source, **no API**, **nothing executed** |
| Candidate-specific Prompt 2 | `empatica/PROMPT_2.md` | Written — `PROMPT2-EVIDENCE-FIRST.md` base plus the `PROMPT2-PRIVACY-AND-OPERATING-MODE` supplementary block |
| Comparison report (Prompt 2 run) | `empatica/empatica_COMPARISON.md` | Written — 14 sections |

**Verification command:**

```sh
ls -l docs/app_comparison/empatica/
```

**Inspection mode:** **artifact and regulator inspection — performed.** This is a **new mode instance**,
the first in the package with no binary, no source and no API. What was read: the vendor's own published
surfaces (product, store, `/legal/`, `/cloud-api/`, `/manuals/`, privacy notice, **EULA**, E4 sunset notice),
plus an **independent regulator's record of the vendor's own submission** — openFDA 510(k) records and
product-code classification, and the **K250515** 510(k) summary text (2,661,556 bytes → 602 lines). All
artifacts are SHA-256 hashed; every quoted page, the EULA, the certificate, the manual and the FDA PDF were
retrieved directly rather than read through a secondary source. **The runtime-observation count for this
candidate is zero, and no vendor endpoint was called.**

**Why the mode changed.** The three prior modes assumed an implementation surface (source, published API, or
runnable binary). Empatica publishes **none of the three**: the repository organisation holds 5 public repos
and only one is original (a documentation site, since emptied); the Cloud API documentation is supplied to
clients only; the app cannot lawfully be obtained and is iOS-only in practice. What it *does* publish is a
**regulated product's paperwork** — and, independently, the regulator's own record of it. That is a
stronger evidence class than marketing, so it became the mode rather than a fallback.

**Six findings, in order of consequence.**

1. **The headline performance figure cannot be reached from the page that makes it, and the metric the
   regulator considered decisive is absent from the page.** The product page claims "detects possible
   seizures with **98% accuracy**" and "demonstrating a **98% detection accuracy against the gold standard
   of hospital-based EEG measurement**" — uncited. K250515 reports **positive percentage agreement** and
   **false-alarm rate per 24 h**, states "**No clinical studies have been performed on the subject
   device**", establishes the shipped configuration "by bench testing to be equivalent" to the 2018
   predicate K181861, and derives its real-world figures from **Embrace2** hardware. Recorded precisely: the
   page's number is **untraceable**, not shown to be false. This is separately verified as a **fact about
   the page**, not an inference about the product.
2. **The regime answers the question AnxietyWatch2's three-state rule answers, by regulation.** The cleared
   product code **POS** (class 2, 21 CFR **882.1580**) is defined in the regulator's own words as a device that
   "**does not predict seizure onsets, and is not intended as a standalone seizure monitoring device**".
   The vendor's own scope statement is consistent and precise — GTC seizures ≥ 20 s; "**not designed to
   detect** seizures that do not generate movements such as complex partial seizures, absence seizures or
   non-convulsive seizures"; ages 6+. It is the first candidate whose **independently published scope limit**
   states, in the regulator's language, the same principle V2 enforces structurally: an absent signal is not
   a reassurance.
3. **A four-hop cloud alert path with a physical precondition, a commercial gate and a contract that declines
   to warrant delivery.** Watch → BLE (**phone within 15 ft**) → phone internet → Empatica Cloud → **call +
   SMS** to 1 / 3 / 10 caregivers by plan, with a **mandatory subscription**. The EULA states the vendor
   "**cannot guarantee that a call or text message will reach any Caregiver**", that results are not
   warranted "**ACCURATE, COMPLETE, RELIABLE, CURRENT, OR ERROR-FREE**", that liability "**SHALL NOT EXCEED
   THE PURCHASE PRICE PAID**", and that there is no liability for "**PERSONAL INJURY OR DEATH**". Every
   clause is ordinary consumer-software practice; the finding is the **coexistence, unresolved, on the same
   page** — "designed to keep you and your loved ones safe". This is the first candidate where a **contract**
   is a primary evidence surface, and it is recorded as a counter-surface, not as a scandal.
4. **False alarms are a first-class design problem, with numbers, and AnxietyWatch2 has stated no number.**
   FAR per 24 h from K250515: **0.70 / 0.28** (EMU, 6–21 / >21) and **0.34 / 0.25** (real-world) —
   paediatric ≈ **one false caregiver alert every three days**. The vendor ships three mitigations: a
   **cancellation window before caregivers are contacted** (length unpublished), **adjustable sensing
   precision** (high / low), and an optional **low-sensitivity mode** described as existing to reduce false
   alarms. V2's supervised-alarm specification states no acceptable false-alarm rate for any alert type, and
   that gap is visible only because this candidate is in the shortlist.
5. **The vendor's own published artifact undercuts the claim printed beside it, and its instructions for use
   cannot be read.** The EmbracePlus page claims "CE-certified as a class IIa medical device" while the
   compliance document the vendor links is an **ISO 13485** certificate (cert. 39 05 0072201, TÜV Rheinland
   Italia, UNI CEI EN ISO 13485:2021, valid 2025-05-08 → 2028-05-24) carrying the standard statement that it
   "does not represent proof that the statutory requirements of … Regulations (UE) 2017/745 … have been
   fulfilled" — **recorded as a placement tension, not impropriety**; every notified body writes that
   sentence. Separately, the **EpiMonitor User Manual** (178 pages, 112.8 MB) has **no fonts and no text
   layer** — `pdffonts` empty, `pdftotext` returning 178 bytes across 178 pages — so it is not searchable,
   not screen-reader accessible, not machine-checkable, and blocks claim reconciliation on the single
   question the comparison most wants answered.
6. **The FDA citation on the platform page names the wrong clearances.** The page cites "( K221282 -
   K230457 )", which are the *platform* clearances and do not name EpiMonitor; the EpiMonitor clearances
   (**K232915**, **K250515**) are uncited. Accurate in substance, **incomplete in citation** — a reader
   checking the supplied numbers is shown the wrong record.

**Also recorded (secondary).** **Eight consolidated surface conflicts** — four of them the same shape (a
vendor's own artifact failing to support the vendor's own text). Two are intra-page: "No black box
   algorithms" beside "AI-powered remote monitoring", and EmbraceMini's battery stated as both "at least 7
   days" and "14+ days" on one page. One is cross-page: EmbracePlus "up to 7 days" versus "up to 28 days".
   One is cross-instrument. A **ninth** — "300+" versus "200+" digital measures — was considered and
   **deliberately not counted**, because the two numbers describe different products; recorded only to show
   it was examined. The **privacy notice** (US, last updated **2024-12-01**, ~22 months stale) explicitly
   covers the watch, the app and the Portal, and describes **Google Analytics**, **pixel tags**,
   **cross-device tracking**, **advertising networks** and "Custom Lists and Matching" sharing name and
   email for ad targeting — with **two ambiguities recorded and left unresolved**: whether the advertising
   language reaches watch-derived health data or only Portal browsing, and that additional notices control
   on conflict. The **`/legal/` page** publishes an **ISO/IEC 27001:2022** statement and a **Safety standards
   list**; two direct observations: **ISO 10993-1:2018 appears twice consecutively**, and **IEC 60601-1-8
   (alarm systems) is absent** — the latter recorded as an observation, **not** a non-compliance finding,
   because the list does not claim to be exhaustive and the standard governs equipment-generated alarm
   signals rather than a cloud caregiver notification. A **mislabelled link**: "Read our Terms & Conditions"
   resolves to the ISO 13485 PDF. A **documentation lifecycle fact with an exact date**: `developer.empatica.com`
   was emptied by three commits on **2025-02-18** — four days after the **2025-02-14** E4 sunset notice, whose
   own words are that the E4 software suite "**can no longer collect and store data, rendering services
   inoperative**". Recorded as a lifecycle fact, not a criticism. **No security.txt, penetration test or
   disclosure programme was found** — `[UNKNOWN]`; the Internet Archive's CDX index holds only a 2023-07-31
   crawler *probe* with no archived response, which is the absence of a capture and not evidence about the
   file.

**Positives recorded as positives.** Empatica **publishes false-alarm rates with confidence intervals and
   denominators**; it ships **wearing detection** as a platform signal, which is the quantity a caregiver
   actually wants; it publishes a **standards list** and an ISO/IEC 27001:2022 statement; it states its
   scope limits precisely; and it publishes a **contract that says what it will not promise**, which is more
   than most consumer health products do. A correction was made in the other direction during this candidate:
   an earlier reading that the vendor published **no security surface** was **wrong** — the detail is on
   `/legal/`, and it was corrected in the same file. Two further self-corrections were made: a **fabricated
   placeholder hash** written into the artifact table was replaced with computed SHA-256 values (the rule
   "an unverified hash is worse than no hash" is now recorded in the file), and a **failure-mode hypothesis
   about the app's Bluetooth-drop behaviour was dropped** rather than recorded, because the app cannot be
   observed.

**Claim reconciliation:** 27 checkable claims — **12 match, 7 documented-only or unverifiable, 2 partial,
   6 do not match as stated**.

**Evidence gaps:** **G1–G10**, headed by the unreadable User Manual. The decisive gap is that **no runtime
   behaviour of any kind was observed** — alert latency, cancellation-window length, out-of-range handling,
   lapsed-subscription behaviour and sensor-failure behaviour are all `[UNKNOWN]`, and this inspection cannot
   answer any of them.

**Mode-record note.** The manual is the first artifact in this package that **exists, was retrieved in full,
   and cannot be read.** It is recorded simultaneously as an **accessibility** finding and a **verifiability**
   finding, and it is the reason gap G1 is not closed by another fetch.

## Residual uncertainty — `empatica`

- **R1** **The User Manual's content** — 178 pages, 112.8 MB, no text layer, retrieved and unreadable. The
  surface most likely to state limits in plain language to the wearer, and the blocker on the standalone-
  reliance question. Closing it needs an **OCR decision**, not a fetch. **Decisive.**
- **R2** **The 98% figure's provenance.** Not on the page; not reconstructible from K250515. The vendor's
  **Publications** page was **not** consulted in this pass. A source may exist. **Decisive.**
- **R3** **No runtime behaviour of any kind.** iOS-only in practice, prescription-gated, no lawful binary.
- **R4** **The Cloud API contract** — no OpenAPI, endpoint, auth scheme, rate limit or error catalogue
  published; "provided" to commercial clients only.
- **R5** **Android presence** — four candidate identifiers returned 404; Play returns 403 to some non-browser
  clients, so this is **not evidence of absence**. `[UNKNOWN]`.
- **R6** **Plan detail** — the support-centre article returned HTTP 403 (Cloudflare).
- **R7** **K232915's summary** returned 404 at this date; K250515 covers the same device name and product code,
  so this is a retrieval failure, not an evidential gap.
- **R8** **HIPAA and GDPR artifacts.** "Compliance with industry standards including HIPAA, GDPR and ISO" is
  asserted on a marketing page; ISO/IEC 27001:2022 and ISO 13485 are published; no BAA, DPA or GDPR artifact
  was fetched, though an EHMP **Data Processing Agreement** is listed on `/legal/`.
- **R9** **"corrected PPA" is undefined** in K250515, and the corrected values are materially lower than the
  raw ones (1.000 → 0.905; 0.87 → 0.86; 0.80 → 0.77).
- **R10** **No security.txt, penetration test, disclosure programme or incident history found.** `[UNKNOWN]`.
- **R11** **Only nine surfaces were read.** The use-case, actigraphy, sleep, CRO, academic-research, eCOA,
  digital-biomarkers, publications, case-studies, compliance and resources-library pages were not enumerated.
  Nothing in this comparison is a complete account of the vendor's claims.

**Most promising untried step.** Two, and both are **read-only**: fetch the vendor's **Publications** page and
follow any citation that reports the 98% figure (**R2**), and obtain an **OCR** of the manual or a text-format
substitute (**R1**). Neither is performed here. **The step that would change the most is the one that is least
available** — observing the app — and it is correctly out of reach without an owner decision.

**Not to be mistaken for a finding.** The 98% claim is characterised as **untraceable from the page that makes
it**, never as false. A 510(k) resting on bench equivalence plus retrospective analyses is a **lawful and
common route**; the observation is that the marketing page does not describe the route it took. No medical or
safety judgement is offered, and no claim is made that a clearance is improper.

## Candidate 8 — `oura`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `oura/oura_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 15 sections, claim ledger **O-1…O-95**, consolidated conflict table, 23-row artifact SHA-256 table, gaps **G1–G13**, second-pass record (§14.8) and quote-verification pass (§14.9) |
| Authorized implementation inspection | `oura/oura_IMPLEMENTATION_INSPECTION.md` | Written — **documented-contract inspection, partially performed**: the vendor's published specification and legal/trust surfaces read. No binary, no source, **no endpoint called, nothing executed** |
| Candidate-specific Prompt 2 | `oura/PROMPT_2.md` | Written — `PROMPT2-TECHNICAL-SYSTEMS.md` base plus the `PROMPT2-EVIDENCE-FIRST.md` supplementary block |
| Comparison report (Prompt 2 run) | `oura/oura_COMPARISON.md` | Written — 14 sections |
| Final findings, PDF | `docs/oura-final-findings.pdf` | Written — `oura_COMPARISON.md` at `be74b06` rendered with pandoc + weasyprint; extracted with `pdftotext -layout` and grepped for private identifiers (the public-info scanner skips binaries) |

**Verification command:**

```sh
ls -l docs/app_comparison/oura/
```

**Inspection mode:** **partially performed — documented contract only.** No new mode was created; this is
the strongest instance of the **implementing-artifact** strength already established by `sleephq`, and the
first case in the package where **the vendor's own terms closed the stronger mode by contract** rather than
by unavailability. What was read: the complete **OpenAPI 3.1 contract** (`openapi-1.41.json`, 454,152 bytes,
SHA-256 `a144a343…`, pinned), the **Terms and Conditions** and **Privacy Policy** in full, the **Trust
Center**, the science/validation articles and their author bylines, the membership, health-radar and B2B
pages, the **Google Play** listing, and the **iTunes Lookup** metadata. All 23 artifacts are SHA-256 hashed.
**The runtime-observation count for this candidate is zero.**

**Authorization basis, quoted verbatim from the Terms §II:** reverse-engineering prohibition on (ii)
"reverse engineer, disassemble, decompile, decode, adapt, or otherwise attempt to derive or gain access to
any **Oura Apps** component of the Services, in whole or in part"; (vi) "attempt to probe, scan, or test the
vulnerability of any **ŌURA** system or network or to breach security or authentication measures **without
express authorization**"; (ix) "circumvent, bypass, disable, degrade, or otherwise interfere with any access
controls, usage limits, security features, authentication measures, **subscription restrictions**,
technological protection measures, or other technical or contractual restrictions that ŌURA applies to the
Services…". **No call was issued to any Oura endpoint, including the sandbox.**

**Seven findings, in order of consequence.**

1. **The contract independently reaches the same design conclusions V2 states — and documents them.**
   `PublicSleepType.deleted` (a user-deleted record is a **tombstone**, not a removal); `PublicSleepType`
   value `rest` = "Falsely detected sleep / nap, rejected in confirm prompt by user" (a **rejected
detection retained with its reason**); `PublicSleepAlgorithmVersion` ∈ `{v1, v2}` (per-record **algorithm
provenance**); `PublicSleepAnalysisReason` (mutation provenance); `class_5_min` `'0'` = non wear;
   `non_wear_time` (a measured duration, not a sentinel); a nullable `score` with **all-nullable contributor
lists**; and **four write operations in the entire API, all webhook-subscription management** — i.e. the API
is **read-only for health data**. The honest framing, recorded in the comparison: *both systems have these
rules; V2 enforces them where Oura documents them.* **This is not a V2 advantage and must not be written as
one.**
2. **Missingness is structural and cannot state its cause.** Oura can represent *not measured* and *not
worn*; it cannot say **why**. V2's three-state rule names the states. The sharpest form of this finding is
the **three disagreeing character-encoded alphabets** in one activity/sleep model: `class_5_min` (`'0'`–`'5'`,
where only `'0'` can mean non-wear), the sleep-phase strings (`'1'`–`'4'`), and `movement_30_sec` (`'1'`–`'4'`
with **no `'0'` at all**, so movement cannot express absence). The same digit means different things in
fields that appear side by side.
3. **There is no alert, escalation, acknowledgement, delivery-receipt or caregiver concept in 72 paths and
75 schemas**, and the Terms state the services are "NOT INTENDED FOR USE IN EMERGENCY OR LIFE-THREATENING
SITUATIONS". Recorded as a **complete inventory**, not as an absence of a search hit. The comparison states
the consequence plainly: V2's specified alarm layer is **measured against nothing**, which is weaker
differentiation than beating a competitor — and it is the same candidate pairing in which V2's still-unstated
**acceptable false-alarm rate** becomes visible.
4. **Four date/time schemas are bare `type: string` with no `format`, pattern, example or description**
   (`ISODate`, `LocalDateTime`, `LocalizedDateTime`, `UtcDateTime`) on the field that determines *when a
measurement happened*; exactly **one field violates the document's own ISO 8601 rule**
(`PublicHeartRateRow.timestamp_unix`, milliseconds); and only **one model** anywhere carries a UTC offset
(`PublicSleepTimeWindow.day_tz`). Six fields state **no unit at all**. This is the comparison's largest
reproducibility difference and the clearest place where V2's "UTC epoch seconds and nothing else" and the
registry's one-canonical-unit rule differ from a published contract.
5. **A contract ambiguity and an intra-document range conflict.** The `heartrate` 200 response is
   `anyOf[TimeSeriesResponse_PublicHeartRateRow_ | TimeSeriesResponseDict]` with the second branch
`additionalProperties: true` and **no documented criterion** for choosing between them. Separately, the same
document states a score range **three ways** (global rule `0-100`; `[1, 100]` on the activity score and most
contributors; `[0, 100]` on resilience contributors; unstated on sleep/readiness) — **whether `0` is valid is
contradictory within the vendor's own document.** `ResilienceContributors` is also the one contributor model
that **requires** all three fields and forbids null, against three sibling models where every contributor is
nullable and none is required.
6. **The evidence surfaces do not match the contract's quality.** "99% Heart Rate Accuracy" is an r² of
   **0.996 measured on 49 healthy adults**; "Independently validated sensors" is a gallery containing a study
   whose lead author the vendor itself identifies as its **Chief Scientist at the time**; "over 30 biometrics"
   (Play) contradicts "over 50 health metrics" (home); a **2025 independent study reporting 53% agreement** and
a **pending class action** are reported by the vendor against itself in a 2026 article, alongside its own
79% figure. Recorded verbatim, unresolved, with the mitigation that the **basis is disclosed** in the
accompanying artifact — the defect is in the surface that omits it.
7. **Documentation defects, recorded as such and not as product defects.** The Quick Start claims "Each
endpoint includes example requests and responses" while the contract contains **0 `example`/`examples`**
keywords; "IDs … follow a consistent format" is asserted where almost no `format` is declared; the FAQ's flat
"5000 requests per 5-minute period" sits beside a **two-layer** model whose ceilings are unpublished; and
`PublicRestModePeriod.start_time` is described as "…rest mode ended.", identical to `end_time`. **A
documentation defect is not a product defect**, and a retrieval failure is not an absence: `/legal`,
`/science` and `/for-organizations` returned JavaScript error shells, not 404 pages, and are recorded
`[UNKNOWN]`.

**Claim reconciliation:** 50 checkable claims — **24 match, 9 documented-only, 6 partial, 7 do not match**
(items 10, 11, 12, 14, 24, 25, 36), with four items outside those classes named individually. The tally was
**wrong when first written** and was machine-counted from the table and corrected in place.

**Evidence gaps:** **I1–I11** (inspection) and **G1–G13** (claims). The decisive gap is unchanged from every
prior candidate: **no runtime behaviour of any kind was observed**, and here that was reinforced by a
contractual prohibition rather than by an unavailable artifact.

**Mode-record note — two firsts for the package's own method.**

- **A verification pass that reversed the draft's direction.** A **field-class sweep** performed after the
  first pass reversed **five verdicts**, softened a sixth, and produced **nine new findings, five of them
  positive for the vendor**. That is recorded against the **research process**, not the vendor, and the
  standing remedy — *sweep the structured classes before drafting a conclusion* — is now a sub-check in
  `RESEARCH_METHOD.md`.
- **Nine quotations survived two value-checking passes and were still not verbatim**, including a Terms
  clause cited as the authorization basis whose paraphrase had **invented verbs**. All nine were found by
  **machine comparing every quoted span against the fetched bytes, with the specification parsed as JSON
  string leaves** so `\u`-escapes could not defeat matching. That sweep is now a **required pre-commit
  step**; both earlier passes had checked **values** (hashes, counts, field types) and neither had checked
  **words**. §14.9 of the claimed-features file carries the before/after table.

## Residual uncertainty — `oura`

- **R1** **Null semantics per field** — what a null means (not worn / sensor failed / rejected / not yet
  processed) is not distinguishable in the contract. Answerable from the document for *representability*;
  not answerable for *cause*. **Decisive for the missingness comparison.**
- **R2** **The four bare-string date/time schemas.** The difference is carried only by the type name. Would
  need a live sample; **not authorized**. **Decisive.**
- **R3** **Webhook reliability in practice.** Counts and a window are documented ("10 retries" per failure
  class; "about an hour"; "you might miss some events"); the **backoff schedule is not**, the signature
  scheme has **no documented replay window**, and the vendor's own reference implementation reads
  `x-oura-timestamp`, feeds it into the HMAC, and **never compares it to the current time**.
- **R4** **The two-layer rate-limit ceilings.** `info.description` describes a per-token and a per-app limit
  without publishing either ceiling; discoverable only from a `429` response header. `[UNKNOWN]`.
- **R5** **`sleep_phase_30_sec` vs the vendor's blog.** The blog says the public API reports sleep stages "in
  five-minute blocks"; the contract publishes a 30-second field. Vendor prose against vendor contract,
  unresolved. `sleep_algorithm_version` resolves the *related* timeline question even though the prose does
  not.
- **R6** **`temperature_deviation` vs `temperature_trend_deviation`** — near-identical descriptions, no
  stated difference, and no absolute-temperature field anywhere. `[UNKNOWN]`.
- **R7** **The BLE layer, firmware, ring-side buffering and off-finger behaviour** — wholly unpublished, and
  the app/firmware are covered by the Terms' reverse-engineering prohibition. `[UNKNOWN]`.
- **R8** **`meal` / glucose.** In `ExtApiV2DataType` but with no REST path and no sandbox twin; Reachable
  only, if at all, through the Enterprise API. `[UNKNOWN]`.
- **R9** **Health Radar rates.** Blood Pressure Signals, Nighttime Breathing and Symptom Radar publish no
  sensitivity, specificity or false-alarm figure on any read page.
- **R10** **The 53% study and the rebuttal letter were not fetched.** The package holds the vendor's
  characterisation of both, written by the vendor's own lead sleep-algorithm scientist.
- **R11** **Three surfaces `[UNKNOWN]`.** `/legal`, `/science` and `/for-organizations` returned JavaScript
  error shells. Not 404s; not evidence of absence.
- **R12** **V2 counterpart to `sleep_analysis_reason`** (gap G12). The claimed-features file records it as
  having no identified counterpart; confirming or refuting that needs a read of the V2 provenance code, and
  it is `[UNKNOWN]` until then. **This is the one place where Oura appears ahead of V2 on a documented
  capability.**

**Most promising untried step.** Two, both **read-only**: fetch the 2025 *Scientific Reports* study and the
*Sleep Advances* letter directly (R10), and read the V2 provenance/versioning code to settle R12 — the latter
is in-tree, requires no authorization, and is the only residual item that could move a *conclusion*. **The
step that would change the most remains the one that is unavailable**: a live API sample (R2, R4), which
needs an owner decision that has not been taken and that the Terms separately discourage.

**Not to be mistaken for a finding.** A pending class action is **not** a finding of liability. An r²
rendered as "Accuracy %" is **not** an accusation of fraud — the basis is published in the adjacent
artifact. An intra-vendor count discrepancy is **not** deception. A documentation defect is **not** a product
defect. A retrieval failure is **not** an absence. And the convergence of Oura's record design with V2's
rules is **not** a V2 advantage: it is evidence that two independent teams reached the same conclusions,
which is a finding about the conclusions.

## Candidate 9 — `homeassistant`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `homeassistant/homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 15 sections, claim ledger **H-1…H-35**, 15-row artifact SHA-256 table, gaps **G1–G11**, shortlist correction (§14.4) |
| Authorized implementation inspection | `homeassistant/homeassistant_IMPLEMENTATION_INSPECTION.md` | Written — **performed: published source.** Pinned tag, 13-row file/hash table, the four-way syntax-error verification chain, 50-claim reconciliation, gaps **G1–G15** |
| Candidate-specific Prompt 2 | `homeassistant/PROMPT_2.md` | Written — `PROMPT2-TECHNICAL-SYSTEMS.md` base plus the `PROMPT2-PRIVACY-AND-OPERATING-MODE.md` supplementary block |
| Comparison report (Prompt 2 run) | `homeassistant/homeassistant_COMPARISON.md` | Written — 14 sections |

**Verification command:**

```sh
ls -l docs/app_comparison/homeassistant/
```

**Inspection mode:** **performed — published source.** The strongest mode in the package, and the only one
that required no authorization question of any kind, because the code is **Apache-2.0**. Sparse clone of
`home-assistant/core` at tag **2026.9.4** (commit `9212531f40a0b7b23229a90d688dd79d9dfccff4`, 2026-09-27),
110 Python files present, **eleven read** in whole or in part, every read file SHA-256 hashed. The project's
documentation was read alongside it, and the site displays **2026.9.4**, so the docs and the source are the
same release. **The runtime-observation count for this candidate is zero.**

**Authorization basis.** Apache-2.0 (`LICENSE.md`, SHA-256 `c71d239d…`, text confirmed). Reading and quoting
a published Apache-2.0 repository is expressly permitted by its licence: **nothing was bypassed, no
authentication, DRM, licence check or protection was circumvented, no account was created, no instance —
public, demo or private — was contacted, and no endpoint was called.** The third-party `habluetooth`
package was deliberately **not** fetched.

**Five findings, in order of consequence.**

1. **The alert path has the exact failure mode V2 forbids.** Home Assistant's state machine has first-class
   `STATE_UNKNOWN`/`STATE_UNAVAILABLE` (`const.py:303,313`), and its most mature alarm primitive — the
   `alert` integration, read in full — has **no branch for either**. The alert ends whenever the watched
   state differs from the configured alarm state (`entity.py:98`) and, on that transition, sends the user's
   own `done_message` (`:115-126,148`). **A sensor going offline reads as "the problem is over."** The
   published state table documents `idle`/`on`/`off` and never mentions the absent case, so the
   documentation is *accurate* about a rule that is wrong for a missing signal. Both docstrings state
   "DEVELOPMENT OF THE ALERT INTEGRATION IS FROZEN." (`:3,33`). **This is the sharpest external vindication
   in the package of V2's "a missing signal never reassures" rule** — and the sharpest warning, because the
   correct three-state model is defeated by one equality test downstream.
2. **The three-state model exists and is lost twice** — at the alarm layer (above) and at storage:
   `unknown`/`unavailable` are written into `states.state` as those literal strings, a **removed** state is
   written as the **empty string `""`**, and the column is a nullable `String(255)` with **no unit and no
   provenance** (`db_schema.py:422,478-490`). Three distinct absences collapse onto three string values in
   one untyped column. **This refutes the shortlist's hypothesis** that a general-purpose hub has no
   three-state honesty model (`CANDIDATE_SHORTLIST.md:317`); the correction is recorded in the
   claimed-features file §14.4 and in this file's own rules.
3. **Retention is a scheduled hard delete for history and an upsert for aggregates.** Raw `states`/`events`
   rows are deleted past `purge_keep_days`, **default 10** (`purge.py:55,498-511`; `recorder/__init__.py:105`),
   with **no tombstone and no record of removal**; short-term statistics go with them. Hourly `statistics`
   are the opposite — a unique index on `(metadata_id, start_ts)` makes each entity-hour an **upserted row**
   (`db_schema.py:696-722`) that the scheduled purge never deletes, so long-term aggregates persist. The
   aggregate layer carries two things V2 lacks and they are recorded as **positives**: a **coverage weight**
   (`mean_weight`) stored beside the mean, and **circular-mean** SQL helpers (`statistics.py:144-162`).
4. **A delivery-confirmation mechanism exists and stops after one hop.** Local push requires confirmation
   within `PUSH_CONFIRM_TIMEOUT = 10` seconds; on timeout the channel is torn down and a single
   `fallback_send` runs (`push_notification.py:10,33-62`). Receipts are not persisted, there is no second
   fallback and no escalation, and `ServiceNotFound` is merely logged and retried (`entity.py:183-193`).
   **V2's supervised alarm and hardware dead-man switch are V2 specified target and not built**; the report
   says so on both sides rather than scoring a win.
5. **One file in the inspected release does not parse.** `components/sensor/recorder.py:239` reads
   `except ValueError, TypeError:`; `ast.parse` fails with "multiple exception types must be parenthesized",
   and the bytes are identical in the checkout, the git object database, `raw.githubusercontent.com` at the
   tag, and on `dev` (`ea8cf79e…`; `dev` is `2792ec63…`). Three further verified facts: the module is
   **imported by its own test file** (`:48-52`, 56 tests); the **same idiom is written correctly five lines
   below** (`:244-249`, `with suppress(ValueError, TypeError):`); and ruff's `select` includes `E` with **no
   per-file ignore** for the path while the only `"syntax-error"` entry lives in the separate **pylint**
   section (`pyproject.toml:658,647,862-879` vs `:214`). **Why CI appears not to have caught a non-parsing
   file is unexplained, and the tension is recorded as unresolved (gap G14) rather than guessed at.** The
   function containing the error is referenced by **no test** (gap G15). It is a static fact about a file;
   its runtime consequence is inferred, labelled, and not used rhetorically.

**Claim reconciliation:** 50 checkable claims — **31 match, 10 documented-only, 2 partial, 7 outside
classes.** Machine-counted from the table, not written from impression. The two `partial` rows (items 32
and 34) are exactly the two places where the static fact is verified but the *consequence* rests on a file
the inspection did not read.

**Evidence gaps:** **G1–G15.** The decisive one is unchanged: **no runtime behaviour of any kind was
observed**, so every behavioural statement is a reading or an inference.

**Positive comparators recorded for the vendor, not against it** (a comparison that lists only differences
is not a comparison): substituting an **explicit absent state** rather than clamping or truncating an
over-long value (`core.py:2500-2507`); an acknowledgement path that **raises with a named blocker** when
acknowledgement is disabled (`entity.py:201-206`); a completion message **armed only after a message was
actually sent** (`:148`); an explicit `False` for a receipt that matches nothing
(`push_notification.py:65-73`); `mean_weight` coverage weighting; circular means; opt-in-only telemetry with
published retention; and a privacy policy that **names its own scope** and carves the separately-governed
cloud out in the open.

**Mode-record note — a first for the package.** This is the **only candidate inspected where the mode
itself required no judgement call**: three of ten candidates were inspected from published source and this
one closed the question of whether the strongest artifact can lawfully be inspected completely, by licence. It also
produced the package's **first case where a mature *unrelated* project's code vindicates a V2 rule** rather
than merely differing from it — which is stronger evidence for the rule than any competitor's marketing,
because the project had no stake in it.

## Residual uncertainty — `homeassistant`

- **R1** **Is the alert false-all-clear intended or unhandled?** The code path is verified; the intent is
  not stated anywhere read. The frozen-development docstring and the three-state documentation are the only
  contextual clues. **Decisive for the alarm comparison.**
- **R2** **What a running instance does with the non-parsing module.** Whether the statistics sensor path
  loads, and under which import path, was not tested. **Decisive for the severity of finding 5.**
- **R3** **Why CI appears not to have caught it** (gap G14). Requires the CI configuration and a CI run,
  neither of which was retrieved.
- **R4** **`alert/reproduce_state.py`** was listed but not read, so the restart conclusion rests on entity
  inheritance and `cancel_on_shutdown=True` alone. **This is the report's weakest source-derived claim.**
- **R5** **The `habluetooth` watchdog values** (`SCANNER_WATCHDOG_TIMEOUT`, `SCANNER_WATCHDOG_INTERVAL`) are
  outside the inspected tree.
- **R6** **The REST and WebSocket API contracts** were not read, so the external interface is not compared.
- **R7** **Deployment, backup, upgrade and onboarding** were not inspected, despite the shortlist naming
  Home Assistant the benchmark for them. Only the documentation's story exists.
- **R8** **Staleness for the health sensors.** Only `last_reboot` documents an unavailable-on-unknown-
  timestamp rule; whether the other 25 apply it uniformly is unstated.
- **R9** **Android and Play metadata** were not retrieved; only iOS store metadata exists.
- **R10** **Nabu Casa cloud pricing and its separate privacy notice** were not retrieved, and the push/crash
  third parties are unnamed on the policy page.
- **R11** **"1500+ integrations"** is single-sourced to the home page, and "top open source project by
  contributors in 2025" names no ranking source.
- **R12** **The shortlist correction is recorded here but not written back into `CANDIDATE_SHORTLIST.md`** —
  a reader of that file alone still sees the refuted hypothesis.

**Most promising untried step.** Two, both cheap and read-only: read `alert/reproduce_state.py` to close R4
(the only remaining source-level gap inside the inspected tree), and read the `alert` integration's
`automation` counterpart to test whether the same absent-state hole exists on the trigger side. **The step
that would change the most is R2** — running the project's own test file — but it needs a working
`homeassistant` test environment, is not required for any verdict here, and would still be a *test result*
rather than a runtime observation of a real instance.

**Not to be mistaken for a finding.** A file that does not parse is **not** evidence about the maintainers'
intent. A frozen integration is **not** negligence; it is a lifecycle fact the project publishes about
itself. Refuting the shortlist's hypothesis is **not** a criticism of the shortlist — it is the hypothesis
being tested, which is what the entry invited. And the fact that a mature project shares V2's conclusions is
**not** a V2 advantage: it is evidence that the conclusions are right, which is a weaker claim and the one
the report actually makes.

## Candidate 10 — `mindshift`

| Deliverable | Path | Status |
|---|---|---|
| Claimed features and purpose | `mindshift/mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` | Written — 15 sections, claim ledger **M-1…M-58**, artifact SHA-256 table, gaps **G-1–G-12**, shortlist correction (§14.4) |
| Authorized implementation inspection | `mindshift/mindshift_IMPLEMENTATION_INSPECTION.md` | Written — **performed: published web client and public records.** Five-way privacy-URL verification chain, static architecture findings, **56-row** claim reconciliation, **zero** runtime observations |
| Candidate-specific Prompt 2 | `mindshift/PROMPT_2.md` | Written — `PROMPT2-EVIDENCE-FIRST.md` base plus the `PROMPT2-PRIVACY-AND-OPERATING-MODE.md` supplementary block (the same pair chosen for `empatica`) |
| Comparison report (Prompt 2 run) | `mindshift/mindshift_COMPARISON.md` | Written — 14 sections |

**Verification command:**

```sh
ls -l docs/app_comparison/mindshift/
```

**Inspection mode:** **performed — published web client and public records.** The app binary was **not**
obtained (closed source, store-only, no lawful route), there is no vendor source, and there is **no published
API contract** — the third candidate in the package with none. What was available instead is the vendor's
**published client**: `index-BogFfh2L.js`, 741,896 bytes, SHA-256 `1bb16811…`, read as text. That is
`[FOUND IN PUBLISHED CLIENT]` — a **static** class, not a runtime and not a binary. **The runtime-observation
count for this candidate is zero.**

**Authorization basis.** Documents served to any unauthenticated visitor plus public records: the site's HTML
and JavaScript, two storefront listings and the iTunes Lookup record, the App Store App Privacy panel (which
Apple labels "**This information has not been verified by Apple.**"), the ClinicalTrials.gov v2 record
`NCT04562948`, and the paper and corrigendum from Europe PMC / PMC. **No DRM, licence or protection was
circumvented; no certificate pinning was touched; no account, credential or token was used; no endpoint was
called**, including `/api/analytics/session`, `/api/analytics/events`, `/api/admin/*` and
`/api/resources/*` — those appear in this package only as strings read from published JavaScript.

**Five findings, in order of consequence.**

1. **The privacy policy both storefronts declare does not exist.** `https://mindshiftcbt.com/privacy/` returns
a genuine **HTTP 404** (3,362 B, `<title>Page not found | MindShift CBT</title>`), verified **five independent
ways**: the direct probe (with three control paths returning 200), the site's own **9**-URL sitemap, the
**13**-path SPA router table, a fixed-string sweep of the 741,896-byte bundle (`Privacy Policy` → 0,
`Terms of Use` → 0, `Terms` → 0), and a same-day re-fetch returning **byte-identical** content (SHA-256
`4a9ba979…` unchanged). **This is the strongest verified absence in the package.** It is recorded as a
documentation-lifecycle and discoverability defect — **not** as a data-handling finding and **not** as an
allegation of intent. The vendor's own release notes for **v3.0.1** and **v3.0.3** say the documents were
updated, so they existed; the URL still published does not resolve.
2. **The vendor's own funded study is the counter-surface to its marketing.** The site renders "Clinically
proven", "Proven, not promised." and "Most anxiety apps are guesswork." The paper it cites reports that
"**Improvements were unrelated to the frequency with which participants used the MindShift app**", that the
result was consistent with response "**but not remission**", and that "**the lack of a comparison group who
abstained from using the MindShift app limits the causal inferences**". **Recorded as selective citation with
both sides quoted, and left standing — not a misconduct allegation.**
3. **An independent registry outranks the vendor's own noun, twice.** Registration says
`studyType: **OBSERVATIONAL**`, `observationalModel: ECOLOGIC_OR_COMMUNITY`, **no `armsInterventionsModule`**,
and **one** primary outcome plus three secondary. The paper's title says "trial" and its text says "**four
primary outcomes**"; the site says "open-label clinical trial" and "**all four primary outcomes**". **Two
conflicts — design class and primary-outcome count — both quoted, both unresolved.** The outcome count is the
sharper of the two.
4. **The site ships its own visitor telemetry and publishes no notice for it.** The client mints or reads a
UUIDv4 in `sessionStorage`, `POST`s it to `/api/analytics/session` with `credentials:"include"`, and posts
four event categories (`page_view`, `client_error`, `resource_opened`, `features_explored`) to
`/api/analytics/events`, both ending `.catch(()=>{})`. In the same bundle: `consent` → **0**, `banner` → 0,
`GDPR` → 0, and all six `cookie` hits are inside the bundled Sentry SDK. **The finding is the absence of a
notice, on a site that has no privacy page to carry one — and it is static**: it establishes what the code is
written to do, not that any endpoint is live or that any event was stored.
5. **Two disclosure asymmetries a claims-only reading cannot see.** (a) The paper's competing-interest
statement reads "**Lance M. Rappaport reports financial support was provided by Anxiety Canada**" — Anxiety
Canada being the app's original co-developer — and the marketing surface that cites the paper carries no
funding disclosure. (b) The paper's methods restrict the analysis to "**the 156 participants for whom
entrance of their study identification code into the MindShift smartphone app verified that they downloaded
the smartphone app**", so **224 of 380 never demonstrably installed the product**, while every headline
surface carries 380.

**Claim reconciliation:** 56 rows — **20 match, 12 documented-only, 12 do not match, 10 partial, 2 outside
classes.** Machine-counted from the table, not written from impression. **Eight rows are `DOES NOT MATCH` for
vendor-surface conflicts within the vendor's own material** (the dead privacy URL on both storefronts, the
"clinical trial" class, the four-outcome claim, its internal three-versus-four inconsistency, "Clinically
proven", "Proven, not promised.", the "4.8 rating", and the FAQ's privacy meta-description, which promises to
explain "how it protects your privacy" on a site with no privacy document).

**Evidence gaps:** **G-1–G-12.** The decisive one is **G-1**: whether a privacy policy or terms document
exists **inside the app** is `[UNKNOWN]`, because the binary was not inspected — and the release notes say the
documents were updated. **G-3** is the second: `anxietycanada.com` returned a **byte-identical 9,321-byte SPA
shell for every path**, including `/robots.txt` and `/sitemap.xml`, which is a **retrieval failure** and not an
absence of content. **A 404 and an error shell are different findings and are never merged in this file.**

**Shortlist correction carried in.** The shortlist entry attributes MindShift to **Anxiety Canada** and
describes it as non-commercial. Both are superseded: the App Store `sellerName` and the Play developer are
**247 Labs Inc** (a for-profit Toronto consultancy), the site says the app was "**originally developed in
partnership with** Anxiety Canada" — past tense — and a B2B surface (`/for-clinics`, `/institutions`) with
**unpublished** pricing runs beside a "Always Free" consumer app, with Clerk organizations and Stripe billing
visible in the published client. **This materially alters the shortlist premise, not merely its wording**, and
it is recorded in the claimed-features file §14.4 and in **Owner decisions** §3 below.

**Positive comparators recorded for the vendor** (a comparison that lists only differences is not a
comparison): three named Canadian crisis lines at the point of use — **988**, **Kids Help Phone**, **Talk
Suicide Canada** — with `tel:` links and qualifiers; the scope boundary stated in the user's language ("not a
medical device, and not a replacement for therapy or crisis care", "mild-to-moderate anxiety", "Adolescents
(13+)"); an explicit refusal of a clinician portal in favour of patient-controlled export; per-page "Last
reviewed and updated" timestamps on content; and a published corrigendum — the process working.

**Mode-record note.** Two firsts. (a) This is the package's **first candidate where the strongest artifact is
the vendor's own peer-reviewed paper**, which makes the counter-surface to a vendor's marketing **another
surface of the same vendor** — a *within-vendor* finding rather than a third-party one. (b) It is the **first candidate
where a vendor's own JavaScript is a primary evidence surface**, and the discipline that made it usable is
recorded in `RESEARCH_METHOD.md`: a served client is a **published artifact** (`[FOUND IN PUBLISHED CLIENT]`,
static), a **zero must be counted with its match mode stated** (`Sentry` capital 20 vs case-insensitive 129;
`Clerk` capital 44 vs case-insensitive 505), and a string in the **HTML shell** is not a string in the
**bundle** (`i.replit.com/script.js` is in every served document including the 404s, and **0** times in the
741,896-byte client — attribute the shell, not the client).

## Residual uncertainty — `mindshift`

- **R1** **Do the privacy policy and terms exist inside the app?** The release notes say they were updated;
the web surface publishes neither and the declared URL 404s. **Decisive for the severity of finding 1 and
for gap G-1.** Requires the binary, or a vendor statement.
- **R2** **Does a controlled or comparative study of MindShift exist?** It decides whether "Clinically
proven" is a present-tense claim with a source. The paper cites prior MindShift work as "[35], [36]"; read
those. **Decisive for finding 2.**
- **R3** **Was the quality-of-life effect size restated after the corrigendum?** The corrigendum reports
"an unfortunate error in the calculated total score for quality-of-life from the Q-LES-Q-SF", corrects the
baseline (M 30.76 → 47.83, SD 9.77 → 15.39) and says the primary conclusions are unchanged, but does **not**
restate d = 0.31. `[UNKNOWN]` — gap **G-11**.
- **R4** **Is the site's analytics disclosed anywhere at all?** No notice was found in the client. The Play
data-safety block describes the **app**, not the website, and is a developer self-declaration Apple labels
unverified. **Decisive for the scope of finding 4.** Requires a vendor statement; the endpoints are **not** to
be called.
- **R5** **Which surface reflects the shipped UI language set?** Site FAQ "English and French", App Store
`['EN','FR','ES']`, client switch `en`/`fr`, and a **v2.0.9** release note describing the app as bilingual.
Four surfaces, two answers — gap **G-4**.
- **R6** **Is there a moderation policy a user can read?** The bundle contains **0** occurrences of
`guidelines`, and no terms exist on the web; the v4.1.0 notes imply an in-app guidelines surface. Publication
placement, not absence — gap **G-7**.
- **R7** **Does the app implement AODA/WCAG?** The site's own role description claims it; the App Store
accessibility section is unpopulated. A design standard is not a declared feature; the two objects are kept
apart.
- **R8** **What are the B2B terms and prices?** `/institutions` says "Talk to us about early access for your
organization"; the waitlist was **not** joined. Gap **G-6**.
- **R9** **Was the funding source visible anywhere on the marketing surface?** Not found. A negative search
result on the surfaces read, not proof of absence.
- **R10** **Anxiety Canada's current statements.** Unreadable — byte-identical SPA shell for every path.
Gap **G-3**.
- **R11** **Does the site's telemetry actually transmit?** `[UNKNOWN]`; the endpoints were never called. The
finding is written to the level of the code and no further.
- **R12** **The shortlist correction is recorded here but not written back into
`CANDIDATE_SHORTLIST.md`** — a reader of that file alone still sees the superseded operator attribution and
the superseded commercial-model description.

**Most promising untried step, and it is the only one that needs an owner decision.** Every remaining
high-value question (R1, R3, R6, R7) is behind the **app binary**, which cannot lawfully be obtained for this
candidate. The cheap read-only steps that do not need it — reading the paper's references [35] and [36], and
checking the registration's `history`/version fields for an amendment — are recorded in `PROMPT_2.md` as
unresolved questions **M2**, **M10** and **M11**, and they are the ones this file recommends trying first.

**Not to be mistaken for a finding.** A dead declared URL is **not** evidence of ungoverned data handling. A
funded study is **normal**; the finding is a **disclosure asymmetry between two surfaces by the same author**,
and the paper's candour is a point in the vendor's favour. First-party analytics with no notice on a site with
no privacy page is a **recordable tension**, not an accusation, and it is **static** — it says what the code is
written to do. Two hypotheses — a Segment CDP integration and a Google `gtag` tag — were tested and
**refuted**; neither is recorded as a finding anywhere in this package. And the fact that a free CBT app has
better crisis routing than this project is a **gap in this project**, not a defect in the candidate, and it is
recorded as such in the comparison's §11 and §14.

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
   precisely because they are small. Across five candidates the blocker for store-only apps has been
   consistent and is not expected to change: the binary cannot be lawfully obtained without an owner
   decision, and no candidate so far has needed it to reach a defensible verdict. **`oscar` resolved the
   question from the other direction** — as a GPLv3 project it is fully inspectable with no owner decision
   at all, which removes any argument that an authorization decision is needed *in order to* inspect
   source when source is already published. **`sleephq` tested the mode from the opposite side**: its
   published API is the strongest artifact available from a closed commercial vendor, and it was enough
   to reach a verdict on architecture, lock-in and provenance — but the four questions that would decide
   the *analysis* comparison (what the session `ahi` is derived from, what is in the `jsonb` summaries, what `_95`
   estimates, and what `o2_score` means) are all behind an account. **This is the first candidate where a
   single authorised read-only API session would materially change the verdict**, which is the concrete
   case option (b)/(c) must now be weighed against option (a). **`oura` then closed the question from a
   third direction**: the vendor's Terms **prohibit** reverse engineering and circumventing subscription
   restrictions, so the stronger mode is barred by **contract**, not by availability — the first such case
   in the package, and one that does not depend on an owner decision to stay closed. The published contract
   was nonetheless enough to reach verdicts on architecture, provenance, missingness, units and time
   encodings; what it cannot reach is any statement about runtime behaviour, and the two items that would
   need a live call (null semantics on the wire, and the two-layer rate-limit ceilings) are recorded as
   `[UNKNOWN]` rather than estimated. **`homeassistant` then closed the question from a fourth direction, and
   the cleanest one: it is Apache-2.0**, so the strongest mode available in the package ran with **no owner
   decision at all** — and it produced the sharpest finding in the whole set, precisely because a mature and
   unrelated project could be read rather than merely described. Combined with `oscar` and `nightscout`, this
   means **every candidate that publishes its source has now been inspected**, and the option (b)/(c)
   question is confined to store-only commercial apps with no published surface, where it has never yet
   been the difference between a verdict and no verdict. **`mindshift` closed it from a fifth direction, and
   the one that should settle it: the strongest artifact was not source, not a binary and not an API
   contract, but the vendor's *own published client* plus its *own peer-reviewed paper* and its *own
   independent registry record*.** The 741,896-byte served JavaScript yielded the package's strongest
   verified absence (a storefront-declared privacy URL that 404s, confirmed five ways) and its first
   within-vendor evidence conflict — a funded paper whose reported limitations contradict the marketing
   surface that cites it. **No owner decision was needed and no endpoint was called.** The remaining
   high-value questions for that candidate **are** behind the binary (residual uncertainty R1, R3, R6, R7),
   so `mindshift` is now the one candidate where option **(b)/(c) would plausibly change a verdict** rather
   than merely deepen one — and it is also the candidate where the contract case for staying closed is
   weakest, since nothing in the App Store or Play terms was found prohibiting local inspection of the
   shipped app. **That decision is placed before the owner in §3 and is not taken here.**
3. **Shortlist adjustment.** `emoods` was dropped (no DNS A record for `emoods.net`). **This decision is now
due:** the ten-candidate shortlist is **exhausted** (10 of 10 complete). Confirm whether a replacement
eleventh candidate is wanted, and confirm the two **shortlist corrections** that the run has accumulated:

   - **`homeassistant`** — the shortlist hypothesis that a general-purpose hub has no three-state honesty
     model was **refuted**; `STATE_UNKNOWN`/`STATE_UNAVAILABLE` are first-class (`const.py:303,313`). Recorded
     in gap **R12** of that candidate's residual uncertainty.
   - **`mindshift`** — the shortlist attributes the app to **Anxiety Canada** and describes it as
     non-commercial. Both are superseded: the operator is **247 Labs Inc** (for-profit; App Store
     `sellerName` and Play developer), the site says the app was "**originally developed in partnership with**
     Anxiety Canada" in the past tense, and a B2B surface with **unpublished** pricing runs beside a "Always
     Free" consumer app, with Clerk organizations and Stripe billing in the published client. Recorded in gap
     **G-8** and in the claimed-features file §14.4.

   Neither correction is a criticism of the shortlist — both are hypotheses the entries invited a run to
test, which is what the entries were for. **Both are recorded in the candidate deliverables and in this file,
and neither has been written back into `CANDIDATE_SHORTLIST.md`**, which still carries the old wording until
it is next edited. **Not blocking.**

## Rules this file enforces

- A candidate is **Complete** only when all four deliverables exist and the verification command has
  been run. A model's assertion is not completion.
- No candidate's research begins while another is **In progress**.
- Inspection limitations are recorded as **evidence limitations**, never as proof that a feature does
  or does not exist.
- Phase 2A is never silently skipped for a mobile candidate: it is either performed, or recorded as not
  performed with a reason.
- A shortlist hypothesis that the inspection **refutes** is recorded as a correction — in the candidate
  deliverables, in this file, and (when the shortlist is next edited) in `CANDIDATE_SHORTLIST.md` itself —
  because an untested hypothesis left standing in the shortlist is a claim the package has already
  disproved.
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
- **A published interface specification has a value domain as well as a type.** A field typed `string` or
  `integer` with no enumeration, description or range is **unusable**, and that is a finding in its own
  right, not a detail. Record every undocumented domain explicitly (`sleephq`: `status`, `progress`,
  `usage`, `omitted`, `teams_type`, `o2_score`, the `drop_*` family; `exist`: checked and found complete).
  Likewise record a documented field that is **write-only** (`active_energy_joules`) and a documented
  hash whose algorithm is **not** published (`fingerprint_base64`).
- **A vendor's own material that undercuts the product's headline claim is a finding, and is never
  resolved in the vendor's favour or against it.** Record both statements, note which surface each comes
  from, and hold the conclusion open. This has occurred twice: `oscar`'s internal leak-percentile review
  is more conservative than its own report presentation, and `sleephq`'s founder publishes that device AHI
  under-counts while the platform publishes an AHI on every session. Do not use either as independent
  validation of an AnxietyWatch position.
- **When two current vendor surfaces disagree, record the conflict and let the most conservative
  statement govern.** Second occurrence (`sleephq`: the pricing page lists the Magic Uploader on the free
  tier, the shop requires Pro; after `bearable`'s policy-versus-data-safety conflict). A conflict between
  two surfaces of one vendor is evidence about the vendor's documentation, not about its behaviour.
- **A specification's `example` blocks may be generated rather than captured.** `sleephq`'s machine
  example contains filler values and timestamps equal to the fetch time. Never read an example value as a
  record, and check the example's provenance before quoting it.
- **Record a vendor's honest boundary statement as a positive.** `sleephq` states that the AirMini is
  unsupported because it has no SD card slot, and lists the machines it cannot read; that is a product
  limit stated as a limit, and it was recorded as such rather than as a missing feature. `nightscout` goes
  further: "not officially approved or regulated for diabetes therapy and/or treatment in any way", "Do
  not rely only on Nightscout as the only way you have of knowing your blood glucose values and trends",
  and "expect unexpected failure" are recorded as positives, quoted verbatim.
- **A documented default that the code does not implement is a finding, independent of which of the two
  is safer.** `nightscout`'s published API3 description says autopruning defaults to `DEVICESTATUS`=60;
  `lib/api3/generic/collection.js:32` reads the env var with no default argument, so nothing prunes unless
  the operator configures it. Record the divergence, name both surfaces, and let the conservative one
  govern. Do **not** read the divergence as either surface being correct.
- **Two current surfaces of one vendor can conflict within a single documentation site, not only across
  sites.** `nightscout`'s security page states the system "has not been designed for scenarios where a
  third party operates a Nightscout installation on behalf of others" while its own landing page
  advertises ten commercial managed-hosting providers with pricing links. Recorded unresolved; the most
  conservative statement governs. This is the third occurrence of the conflicting-surfaces rule across
  the package, and the first that is internal to one site.
- **A shipped derived statistic that depends on a user display preference is the sharpest form of the
  unit finding.** `nightscout` 15.0.8 renders A1c as **95.8%** for mg/dL display and **6.9%** for mmol/L
  display from the same underlying data (`dailystats.js:111` over a series scaled at `reportclient.js:847`),
  and the vendor documents it. Record the mechanism, not just the number, and record the disclosure
  alongside the defect. This is what AnxietyWatch2's "converted once, at the decoder" rule protects
  against.
- **A documented default that exposes the user is mitigated to the extent that the product says so where
  the user can see it.** `nightscout`'s `readable` default is announced by a persistent boot-time notice
  (`lib/server/bootevent.js:149–152`, `persistent: true`) that appears on the instance's own admin page.
  Record the disclosure as a disclosure and state what it does not reach: a user who never opens the admin
  page is unaffected by it.
- **An "implemented and unwired" finding class applies to candidates as well as to AnxietyWatch2.**
  Present and tested, read by no consumer, is a distinct status from absent and from working.
- **A source tree can contain two answers to the same question, and that is itself the finding.** Two
  delete semantics coexist in one `nightscout` process — API3 soft-deletes with a retained history, while
  API v1/v2 and the admin tools hard-delete the same collection. Do not resolve which is "the" behaviour;
  record that the product has both, and say which path a given caller takes.
- **An independent regulator's record of the vendor's own submission is a first-class evidence source, and it
  is queried directly, never inferred.** For a regulated candidate, read openFDA's 510(k) records and the
  **product-code classification** before characterising any regulatory claim, and quote the regulation text
  verbatim. `empatica` was cited as cleared, so the record was read: the vendor page cites the *platform*
  clearances and omits the device's own; and the product code's definition — "does not predict seizure onsets,
  and is not intended as a standalone seizure monitoring device" — narrows the claim in the regulator's
  language. **A product code is a scope fact distinct from the fact of clearance**: never interpret one
  without querying the classification API.
- **A vendor's contractual instrument is a distinct evidence class from its marketing, and both are recordable.**
  `empatica`'s EULA declines to warrant that "a call or text message will reach any Caregiver" on a page
  promising it will "keep you and your loved ones safe". Record the sentence, name the instrument, and **do
  not imply bad faith** — the finding is the coexistence, unresolved, not a scandal. Mark the class difference
  explicitly: a contract is not a marketing page, and that matters more than the disagreement.
- **A vendor's published compliance artifact can disclaim the claim printed beside it.** `empatica`'s
  EmbracePlus page claims CE Class IIa while the document it links is an ISO 13485 certificate stating it is
  not proof of conformity with Regulation (UE) 2017/745. **Record placement, not impropriety** — every
  notified body writes that sentence on every certificate. The same check found a **mislabelled link**
  ("Read our Terms & Conditions" → the ISO 13485 PDF), which is a documentation defect and is recorded as one.
- **A vendor can delete its own developer documentation, and the date is the finding.** `empatica` emptied
  `developer.empatica.com` with three commits on **2025-02-18**, four days after the **2025-02-14** E4 sunset
  notice. Record the lifecycle fact with its exact date and its own words ("can no longer collect and store
  data, rendering services inoperative") as **a lifecycle fact, not a criticism** — it is the most concrete
  available demonstration of what a vendor's interface withdrawal costs a dependent product.
- **An artifact that exists and cannot be read is a finding in two classes at once.** `empatica`'s 178-page,
  112.8 MB User Manual has no fonts and no text layer (`pdffonts` empty; `pdftotext` returns 178 bytes across
  178 pages). It is simultaneously an **accessibility** finding and a **verifiability** finding, it **blocks
  claim reconciliation**, and a further fetch cannot close it. Record it as an evidence gap with the blocker
  named, and if OCR is ever attempted, record the tool, its version, the pages read, and mark the output as
  **OCR text, not the vendor's verbatim text**.
- **An undefined term in a decisive table is a finding, and this generalises the undocumented-domain rule
  from interfaces to regulated documents.** `empatica`'s K250515 reports a "**corrected PPA**" without
  defining it, and the corrected values are materially lower than the raw ones (1.000 → 0.905; 0.87 → 0.86;
  0.80 → 0.77). `sleephq`'s undefined `status`/`progress`/`o2_score` domains and this are the same class at
  different evidentiary weights.
- **A documented defect in a published list is reported as an observation, and the absence of a standard from
  a non-exhaustive list is not a non-compliance finding.** `empatica`'s Safety list duplicates
  **ISO 10993-1:2018** on consecutive lines (directly observable) and omits **IEC 60601-1-8** (governs
  equipment-generated alarm signals, not a cloud caregiver notification; the list does not claim to be
  exhaustive). Record the duplicate; record the omission **as an observation with the reason it is not a
  finding**.
- **An artifact a vendor offers as evidence of one claim is tested against the claim it is placed beside.**
  This is the standing form of the `empatica` findings: the linked certificate is placed beside a Class IIa
  claim and bears an ISO 13485 scope; the cited 510(k) numbers are placed beside a device claim and name the
  platform; the instruction manual is placed beside a standalone-monitoring product and cannot be read.
  Whenever a vendor supplies a document *as* the support for a claim, read the document and check whether it
  supports **that** claim.
- **Vendor-published business figures and vendor-authored analyses are never independent evidence.**
  `empatica`'s EpiAlgo analyses were submitted by the vendor and published by a regulator: the **venue** is
  independent, the **work** is the vendor's. Say it that way. Likewise a vendor's internal document is the
  vendor's *statement*, quoted with dates, and its numbers were not re-measured.
- **Never fill a hash table row with a plausible value.** An unverified hash is worse than no hash. During
  `empatica` a fabricated placeholder SHA-256 was written into the artifact table and then replaced with
  computed values; the rule is now recorded inside that file. **Compute the hash or mark the row "not
  hashed".**
- **An absence is verified by a conjunction of methods, not by one fetch.** When a vendor declares an
  artifact and the artifact is not there, close it with the direct probe (plus a control path that returns
  200), the vendor's own sitemap, the client's own route table, a whole-artifact string sweep, and a
  byte-identical re-fetch. `mindshift`'s storefront-declared privacy URL is the package's strongest verified
  absence on this basis: **no single method would have sufficed**.
- **A published client is a static evidence class, and a reference count is meaningless without its match
  mode.** Findings read from served JavaScript are labelled `[FOUND IN PUBLISHED CLIENT]` and state only what
  the code is *written to do*. A string in the served HTML **shell** is not a string in the **bundle**
  (`i.replit.com/script.js`: every document including the 404s; **0** in the 741,896-byte bundle). Counts are
  reported with their mode, because `sentry` is 129 case-insensitive and 20 as `Sentry`.
- **A vendor's own funded study is the counter-surface to the vendor's own marketing.** Quote the funder's
  role from the paper's competing-interest disclosure, never infer it; never upgrade a preprint to
  peer-reviewed; and treat an independent **registry** record (`studyType`, outcome counts) as outranking the
  vendor's noun. Record a headline sample size against the paper's verified-install subsample when they
  differ (`mindshift`: 380 recruited, 156 code-verified). Read a corrigendum as its own artifact, and where it
  does not restate an affected effect size, record `[UNKNOWN]` rather than carrying the old value forward.
- **A quote-verification sweep is a required pre-commit step, and the checker must never be circular.** The
  target file is excluded from its own corpus; the corpus is the package docs plus the V2 and V1 trees;
  pairing resets at every blank line and table row; HTML tags are stripped and JS `\uXXXX` escapes unescaped;
  a markdown link inside a quotation is split out. **A MISS list is triaged** into checker artifact, author
  paraphrase, or real defect — never auto-accepted. A paraphrase wearing quotation marks is the specific
  failure this package exists to avoid.

**The full rule register, with the sub-check table and the candidate that first produced each rule, lives in
`RESEARCH_METHOD.md` §6.1.** This list records the rules that changed a verdict or a commit; §6.1 is
authoritative when the two disagree.
