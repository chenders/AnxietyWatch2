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

**Active candidate: none.** Candidates 1–5 are complete. The next candidate is `#6 nightscout`, which is not
started and must not be begun until this file is committed.

| # | Slug | Score | Status | Started | Completed | Blocker |
|---|---|---|---|---|---|---|
| 1 | `bearable` | 13 (rank 1 on dimension A) | **Complete with inspection limitation** | 2026-10-05 | 2026-10-06 | Phase 2A not performed — no lawfully authorized artifact. Public-claims research unaffected. |
| 2 | `welltory` | 14 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). Vendor's **public research repos and datasets were inspected** and their citations independently verified. |
| 3 | `exist` | 11 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binary not inspected (closed source, store-only, account-gated). The vendor's **public API reference was inspected in full** — a stronger artifact than Welltory's, because it is the analysis layer's own output contract. |
| 4 | `oscar` | 11 | **Complete with full implementation inspection** | 2026-10-06 | 2026-10-06 | None. GPLv3 source inspected at a pinned commit; no runtime observation (nothing was executed). |
| 5 | `sleephq` | 12 | **Complete with partial inspection** | 2026-10-06 | 2026-10-06 | App binaries not inspected (closed source, store-only, account-gated; no lawful route to the APK). No vendor source exists. The vendor's **published OpenAPI specification was inspected in full** — 21 paths, 11 models, and a documented hash algorithm. **No endpoint was called.** |
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
| **Candidate 5 of 10 — `sleephq`** | **Complete (with a partial inspection)** | See §Candidate 5 |
| Candidates 6–10 | **Not started** | No files created. No research begun. |
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
   Separately, the pricing page lists the Magic Uploader on the **free** tier while the shop states it
   "requires a SleepHQ Pro subscription" — the **second unreconciled conflict between two current vendor
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
   case option (b)/(c) must now be weighed against option (a).
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
  limit stated as a limit, and it was recorded as such rather than as a missing feature.
