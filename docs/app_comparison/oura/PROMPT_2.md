# Prompt 2 — Oura Ring Web API and consumer platform

**Candidate:** Ōura (Oura Ring 5 / 4 / 4 Ceramic, the Oura App, Oura Membership, the published OpenAPI v2 contract, and the B2B Enterprise Platform)
**Base variant:** `../PROMPT2-TECHNICAL-SYSTEMS.md`
**Supplementary block:** `../PROMPT2-EVIDENCE-FIRST.md`
**Chosen because:** Oura is the only candidate where a **complete, machine-readable interface contract** is
available (72 paths, 75 schemas, OpenAPI 3.1) while **no binary, no source and no live call** is available.
That combination makes the technical-systems questions answerable at field level — encodings, nullability,
provenance, envelope shape, webhook semantics — and makes the evidence-first questions *mandatory*, because a
documented contract that is internally pristine can still describe behaviour nothing was observed to perform.
`PROMPT2-TECHNICAL-SYSTEMS.md` supplies the systems questions; `PROMPT2-EVIDENCE-FIRST.md` supplies the
discipline that keeps a specification from being read as a working system.

**Why not the other variants.** `PROMPT2-USER-VALUE.md` is a poor fit: the user value is not in dispute and
the interesting differences are all below the UI. `PROMPT2-PRIVACY-AND-OPERATING-MODE.md` would be plausible —
Oura has real privacy and enterprise complexity — but it is secondary, and the technical-systems block already
carries a local/cloud/cost comparison. `PROMPT2.md` (general) is a strict subset of what the two chosen
variants cover.

**The single most important constraint on this candidate.** The vendor's Terms prohibit reverse engineering,
decompilation and circumventing subscription restrictions (quoted verbatim in
`oura_IMPLEMENTATION_INSPECTION.md` §2.1). **No endpoint — including the sandbox — was called.** That was not
an oversight and it is not to be corrected by this prompt: it is an explicit authorization decision, and the
next person to revisit it must make it explicitly and record it in `PROCESS_STATE.md` before any request is
issued.

---

## Assignment

Produce a second-stage comparison of **Oura** against AnxietyWatch v1 (`../AnxietyWatch`) and AnxietyWatch2
(`./`), using the published contract as the primary artifact and the vendor's legal, trust and research
surfaces as counter-surfaces.

`oura_CLAIMED_FEATURES_AND_PURPOSE.md` and `oura_IMPLEMENTATION_INSPECTION.md` are complete and are the
starting point, not drafts to redo. Do **not** re-fetch the specification; read the pinned artifact by SHA-256
(`a144a3436e9b73da568999fc713257e1d73e12cd3f7024eef45035c9d8f6b557`) and re-verify any `file`/field/line
reference against it rather than carrying a citation over from the claimed-features file.

**The comparison's central question is not "what can Oura do".** It is: **where the published contract
resolves a question that AnxietyWatch2's own rules were written to answer, and where it leaves it open** —
and in each case, whether that is a *documented* resolution or an *enforced* one. Oura documents a tombstone,
a named rejection reason, a per-record algorithm version and a read-only health-data API. V2 enforces an
append-only grant, a quarantine path and a three-state rule. **The honest finding is "both systems have these
rules; V2 enforces them where Oura documents them"** — and any sentence that claims more than that is a defect
in the report.

**Hard limits, unchanged from PROMPT1.** This is an authorized research brief, not permission to act.
**No DRM, authentication, licensing, encryption, certificate pinning, anti-debugging or platform protection
may be bypassed.** No credentials or private data may be extracted. Do not create an Oura account, start or
renew a membership, register an OAuth application, stand up a webhook receiver, or issue a request to any
Oura endpoint without an explicit authorization decision recorded in `PROCESS_STATE.md`. Do not decompile,
decompile-adjacent, or otherwise derive from the app or firmware — the Terms prohibit it by name.

---

## Candidate identity

- **Vendor:** **Ōura Health Oy** (Oulu/Finland) is the CE legal manufacturer and the iOS seller; **Ouraring
  Inc.** contracts with US users and controls the marketing site; **Oura Inc.** is named in the privacy
  policy; **Oura Health Ltd** is the Google Play developer. Four entities across five surfaces — record the
  mapping, do not infer intent from it.
- **Product under comparison:** **Oura Ring 5** (and 4 / 4 Ceramic), **Oura App** (`com.ouraring.oura`,
  7.26.0, released 2026-10-06, 476 MB, 4.8633★ / 308,643 ratings, minOS 16.0), **Oura Membership**
  (**$5.99/mo** or **$69.99/yr**, HSA/FSA eligible) — **membership is required for API data access**, and a
  lapsed membership returns `403`.
- **Artifact under comparison:** **`openapi-1.41.json`** (454,152 B, OpenAPI 3.1.0, `info.version` 2.0,
  server `https://api.ouraring.com`). 72 paths, 75 schemas, 274 resolvable `$ref`s, 4 security schemes,
  75 `x-codeSamples`, **0 `example`/`examples`**.
- **Adjacent, not the subject:** **Oura Advisor**, **Health Radar** (Blood Pressure Signals, Nighttime
  Breathing, Symptom Radar), **Counsel Health** (third-party physicians, 43 US states), the **Oura Research
  App** and **Research Database**, the **Oura Enterprise Platform** (`organizations.ouraring.com`).
- **Regulated surface:** exactly one — the **Fertile Window** feature within Cycle Insights, CE-marked under
  MDR (EU) 2017/745; the Terms add "**NOT INTENDED FOR USE AS CONTRACEPTION AND MUST NOT BE RELIED UPON TO
  PREVENT PREGNANCY.**"
- **Primary comparator in this package:** `empatica` for evidence posture (regulator artifact, contract as
  counter-surface), `sleephq` for the inspection mode (published interface contract), `nightscout` for the
  append-only/tombstone rule, `oscar` for the dependency framing.

---

## Read first

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/RESEARCH_METHOD.md
./docs/app_comparison/oura/oura_CLAIMED_FEATURES_AND_PURPOSE.md
./docs/app_comparison/oura/oura_IMPLEMENTATION_INSPECTION.md
./docs/specs/AMENDMENTS.md
```

`AMENDMENTS.md` is mandatory and governs over any brief that contradicts it. Read the V2 tree for the
storage, missingness and alert rules you intend to compare against — **`vitals/registry.toml`,
`crates/vitals-decode/src/events.rs`, and the `pi-prototype-storage-degraded-mode` design** — and cite file
and line. Do not compare Oura against a remembered version of V2.

---

## AnxietyWatch status labels (use these exactly)

| Label | Means |
|---|---|
| **shipped** | code in the tree, with a test, and a consumer |
| **implemented and unwired** | code and tests present, read by nothing (the `EventKind` case) |
| **specified** (V2 target) | fully specified in `openspec/` and/or `vitals/registry.toml`, not shipped |
| **not present** | absent from the tree, verified by reading, naming the file or search that establishes it |
| **unknown** | not checked, or checked and inconclusive — say which |

AnxietyWatch2's CNS engine, supervised alarm and hardware dead-man switch are **specified (V2 target)**.
Never write them as shipped. Never write a V2 "does not X" claim without checking the tree first — **this
candidate's first pass got two such claims wrong and both are already corrected in the claimed-features file;
do not reintroduce them.**

---

## Evidence rules

1. **Keep the classes apart and never merge them:** `[PUBLIC CLAIM]` · `[DOCUMENTED CONTRACT]` ·
   `[REGULATOR RECORD]` · `[THIRD PARTY]` · `[INFERENCE]` · `[UNKNOWN]`. Plus `[runtime observation]` —
   **of which this candidate has none.** There is no request to point at, including the sandbox.
2. **`[DOCUMENTED CONTRACT]` is a distinct class, and "documented" never means "verified" or "working".**
   Every one of the inspection file's §4 findings is a statement about what a document says.
3. **A retrieval failure is not an absence.** `/legal`, `/science` and `/for-organizations` returned
   JavaScript error shells, not 404 pages. They are `[UNKNOWN]`; the genuine 404 page is a different artifact
   and can be identified as such.
4. **Quote the more conservative surface when the vendor disagrees with itself**, and record the conflict
   without resolving it. The Trust Center's "we do not share or sell your personal data … third-party AI
   models (LLMs)" is contradicted by the privacy policy's named LLM providers; **the policy governs**.
5. **Never upgrade a vendor-authored study to independent validation.** The gallery heading "Independently
   validated sensors" contains a study whose lead author was Oura's Chief Scientist at the time, in the
   vendor's own words. Record both surfaces.
6. **A statistic published as a headline in a different category is recorded verbatim, both surfaces, and
   left unresolved.** `r² = 0.996` on 49 adults rendered as "99% Heart Rate Accuracy" is the pattern; do not
   declare fraud and do not call it accuracy.
7. **Absence of evidence is `[UNKNOWN]`.** The unpublished BLE layer, the unstated retry windows and the
   unpublished rate-limit ceilings are unknown, not absent.
8. **Never convert correlation into causation**, and never state float equality — use tolerances.
9. **Do not reproduce internal org metadata as impropriety.** The specification's `x-owner` squad names are
   observable and recordable; they are not a finding against the vendor.
10. **Reproduce no machine paths, device identifiers or credentials**, from any artifact.
11. **An inspection mode is not an observation.** "Partially performed — documented contract only" means
    *read*, never *ran*.
12. **Verify every quote against the pinned bytes before it goes in the report.** Parse the specification as
    **JSON string leaves**, not as text — its `\u` escapes defeat naive matching. Nine quotations survived
    two value-checking passes in this candidate and were still not verbatim; the method rule is now in
    `../RESEARCH_METHOD.md`.
13. **Count every tally.** Every verdict count is reproducible from the table it summarises. A tally
    written from impression is worse than none.
14. **Verify a failure-mode hypothesis against the artifact before recording it.** If you cannot observe the
    system, the hypothesis is an **open question**, not a finding. Falsify your own hypothesis and drop it
    silently — the dangling-`ClientIdAuth` check is recorded as refuted for exactly this reason.
15. **Standing sub-checks — apply each, and record negative results too:** field-class sweep **before**
    drafting · quote verification against fetched bytes · counted tallies · undocumented value domain ·
    intra-document range conflict · disagreeing character-encoded alphabets · retrieval failure ≠ absence ·
    record-provenance trio · documented dead field · envelope ambiguity · lifecycle fact with an exact date ·
    statistic in the wrong category · evidence-classification defect · intra-vendor count discrepancy ·
    multi-entity surface mapping · a falsified hypothesis (never recorded as a finding).

---

## Candidate-specific unresolved questions

Answer in order. **Q1–Q3 are the assignment**; the rest are lower value, and several are unresolvable without
an authorization decision that has not been taken.

| # | Question | Why it matters | What would resolve it |
|---|---|---|---|
| **Q1** | For each field the contract exposes, **what does a null mean** — not worn, sensor failed, rejected by the algorithm, or not yet processed? | This is the comparison's sharpest finding in both directions. Oura *structurally* distinguishes absent from zero (nullable `score`, all-nullable contributor lists, `class_5_min` `'0'` = non-wear, a measured `non_wear_time`), and its own guidance says to display gaps rather than connect across them — the same instruction as `AMENDMENTS.md`:61. What it does **not** do is distinguish the *causes*. V2's three-state rule names the states; Oura names one of them. Establish per field, from `description` text only, and record `[UNKNOWN]` where the contract is silent. | Nothing further — this is answerable from the pinned specification. Do not call the API to test it. |
| **Q2** | Why are there **four date/time schemas that are all bare `type: string`** (`ISODate`, `LocalDateTime`, `LocalizedDateTime`, `UtcDateTime`), and what does an integrator do about it? | The difference between "localised" and "UTC" is carried entirely by the type name, with no `format`, no pattern, no example and no description, on the field that determines when a measurement happened. Only `PublicSleepTimeWindow.day_tz` carries a UTC offset anywhere in the contract, and only `PublicHeartRateRow.timestamp_unix` violates the document's own ISO 8601 rule (milliseconds). V2 stores UTC epoch seconds and nothing else. This is the single largest reproducibility difference in the comparison. | Nothing further from documents. A live sample would settle it and **is not authorized** — record it as the reason the question stays open. |
| **Q3** | Is the webhook path usable for a real-time monitor, and what does the vendor itself say about its reliability? | `heartrate` — the most real-time-relevant type — **has no webhook**, so a live view must poll, and polling is what the FAQ's rate-limit guidance discourages. For everything else the vendor documents lossiness in its own words ("you might miss some events"; "implement a reconciliation process"), **10 retries** per failure class (4xx, 5xx, timeout) inside a documented window of **about an hour**, but with **no documented backoff schedule**; a signature scheme with **no documented replay window**; and a **reference implementation that reads the timestamp header, feeds it into the HMAC, and never compares it to the current time**. Compare against V2's "attempts precede work" invariant. | The backoff schedule and the signature replay window are unstated in the contract. Record as `[UNKNOWN]`, and quote the retry counts, the "about an hour" window and the lossiness statement, all of which *are* documented. |
| Q4 | What is the two-layer rate-limit model's actual ceiling? | The FAQ states a flat "5000 requests per 5-minute period" while `info.description` describes a two-layer model whose ceilings it does not publish, discoverable only from a `429` response header. A flat figure that contradicts a layered model is the comparison's clearest case of marketing-shaped documentation inside a technical artifact. | A `429` response, which requires a call — **not authorized**. Record the conflict as documented and the ceiling as `[UNKNOWN]`. |
| Q5 | Is `sleep_phase_30_sec` or the vendor's blog correct about the API's sleep-stage resolution? | The blog states the public API reports sleep stages "in five-minute blocks"; the specification publishes a 30-second field and a five-minute field and a deprecated app-aligned field. A conflict between the vendor's own narrative and its own contract, with no resolution stated. | Nothing further from documents. Record the conflict; note that `sleep_algorithm_version` resolves the *related* timeline question even though the prose does not. |
| Q6 | What is the difference between `temperature_deviation` and `temperature_trend_deviation`? | Two number fields on the same models, both described in degrees Celsius, with near-identical descriptions and **no stated difference**. An undocumented value domain in a physiological measure. Temperature is exposed **only as a deviation** — there is no absolute-temperature field anywhere. | Nothing further from documents; record `[UNKNOWN]`. |
| Q7 | Does Oura expose any pre-alert, alert or escalation concept at all? | The complete schema and path inventory contains **no alert, no escalation, no caregiver, no acknowledgement and no delivery-receipt concept** — and the Terms state the services are "NOT INTENDED FOR USE IN EMERGENCY OR LIFE-THREATENING SITUATIONS". This is where V2's specified alarm path is genuinely differentiated, and it must be established by complete inventory rather than by absence of a search hit. | Already answerable from the pinned specification; state the inventory method explicitly. |
| Q8 | What is the BLE layer, the sampling configuration, the off-finger behaviour and the firmware version? | Entirely unpublished. V2's `resmed-air11-ble-stream` and `xe125-respiratory-rate` work sits precisely at this layer, and **nothing published by Oura is comparable to it**. Do not infer a design from the ring's sensors. | Nothing published; `[UNKNOWN]`, and the app/firmware are covered by the Terms' reverse-engineering prohibition. |
| Q9 | Do Blood Pressure Signals and Symptom Radar publish any sensitivity, specificity or false-alarm figure? | The pages make early-detection claims ("before symptoms of a respiratory illness like a cold or the flu are even felt") with **no rate published anywhere**, and the Terms disclaim population-average applicability. Compare with `empatica`, the package's only candidate with published FAR figures — which are themselves adverse. | Nothing further; record as a gap in Oura's material and note that the absence is a documentation gap, not evidence the feature fails. |
| Q10 | Is `meal` / glucose data reachable through the free API? | `meal` is in `ExtApiV2DataType` (the webhook enum) but has **no REST path and no sandbox twin**, and the Trust Center lists "Metabolic Health: Glucose Levels and Meals" as a health metric. A documented-but-unreachable surface for a headline new feature. | Check the Enterprise API's published material; if it is client-only, record `[UNKNOWN]` and say why. |
| Q11 | Does AnxietyWatch2 have anything corresponding to `sleep_analysis_reason`? | Oura records **why** a record was created or revised (`foreground_sleep_analysis`, `bedtime_edit`, `background_sleep_analysis`, `background_created_foreground_updated`) at the field level. V2 has versioned clock-correct metadata (amendment 02) but the claimed-features file records `sleep_analysis_reason` as having **no V2 counterpart** (§14.6 G12). Read the tree and either cite the counterpart or confirm the gap by naming the search. | Read the V2 provenance/versioning code and the amendment text; cite file and line. |
| Q12 | How should the pending class action and the disputed 53% figure be handled in the report? | Both are reported **by the vendor, about itself**, in a 2026-08-28 post by its own lead sleep-algorithm scientist; the counter-study and the rebuttal letter were not fetched, so the package holds the vendor's characterisation of both. The risk is not bias toward Oura — it is presenting a vendor's account of litigation as a finding. | Fetch the 2025 *Scientific Reports* study and the *Sleep Advances* letter, if they are openly available, and quote them directly. If they are not, say so and keep the vendor's framing labelled as the vendor's. |

---

## Comparison taxonomy for this candidate

Use these dimensions, in this order, so the report lines up with the other candidates' comparisons.

1. **Contract as architecture** — what the published interface reveals about the system, and what it cannot
   reveal. The primary axis of this comparison.
2. **Provenance and record lifecycle** — tombstone deletion, retained rejections with a named reason,
   per-record algorithm version, mutation reason, `low_battery_alert`; V2's append-only grant and
   quarantine path. The comparison's strongest two-way finding.
3. **Missingness and the three-state rule** — nullable score and contributors, the `class_5_min` non-wear
   code, the three disagreeing digit-string alphabets, `non_wear_time`, and the four indistinguishable
   timestamp types. Where V2 is *structurally* comparable and where it states more.
4. **Time, units and encodings** — one global rule, one self-violation (ms), four bare-string date types,
   one UTC offset in a window model, six unit-less fields, an undocumented score range stated four ways.
5. **Real-time, overnight and failure modes** — no `heartrate` webhook, sleep sync requiring the app to be
   opened, background sync for activity/stress/heart rate, a documented-lossy webhook path, no replay window.
6. **Alerting** — the complete absence of an alert or escalation concept, against V2's specified path. State
   the inventory method, not an impression.
7. **Evidence quality per claim** — for each headline figure, which artifact carries it, whether the basis is
   disclosed, and whether the study behind it is vendor-authored; the r²-as-accuracy and
   "Independently validated" findings belong here.
8. **Contract versus promise** — the Terms and privacy policy as counter-surfaces to the marketing pages:
   arbitration and class-action waiver, unilateral modification, retention versus "deleted data is fully
   removed", the fertility disclaimer, the emergency disclaimer.
9. **Local/cloud/privacy and operating cost** — membership-gated API access, `403` on expiry, read-only for
   health data, FHIR/EHR import and the HIPAA boundary, SOC 2 and HITRUST without a level, four legal
   entities.
10. **What transfers and what does not** — a consumer ring is not a Pi-based multi-source collector. Be
    explicit about which findings are architectural, which are evidential, and which are simply different
    products.

---

## Technical-systems supplementary block

Answer the base variant's technical questions with these Oura-specific substitutions. **Every answer is a
statement about the document unless marked otherwise.**

- **Sensors, devices, protocols, imports.** Ring sensors as the vendor describes them (red + infrared LEDs for
  SpO₂; green + infrared for HR/HRV; an NTC thermistor; a 3-D accelerometer — **no gyroscope, no EDA**);
  the app; the API; **FHIR/EHR import** on the clinical side. The BLE protocol and firmware are
  **`[UNKNOWN]`**. Record the sensor set as the vendor's claim, not as an inspected design.
- **Raw samples versus derived summaries.** Establish by complete schema inventory (not by sampling) that
  **no raw stream is exposed** — every field is a discrete sample or a vendor-produced aggregate or
  classification. Then distinguish the two kinds explicitly: `PublicSample` (raw-ish, with a self-describing
  `interval`) versus scores, contributors and phase strings.
- **Live streams, background collection, overnight operation.** `heartrate` is pull-only via
  `/v2/usercollection/heartrate` with no webhook and no sandbox twin — it is the clearest real-time gap.
  Oura's own Data Processing Timeline: sleep requires the user to open the app; activity, stress and heart
  rate sync in the background.
- **Timestamps, sample identity, ordering, retransmission, deduplication, provenance.** Time encodings as in
  taxonomy 4; `id` fields with almost no declared `format`; the webhook payload's
  `{event_type, data_type, object_id, event_time, user_id}`; **no documented deduplication or ordering**
  guarantee; provenance via `sleep_analysis_reason` and `sleep_algorithm_version`.
- **Measured-good / measured-bad / stale / missing / inferred / rejected.** Map each Oura mechanism to one of
  V2's three states and record where Oura leaves the state unstated. Include the freshness guidance (data
  available ~30 s after sync) and the `403`-on-expiry behaviour.
- **Calibration, validation, ranges, units, quality flags, device disagreement.** The six unit-less fields;
  the score-range conflict; `PublicDailySpO2.breathing_disturbance_index`'s stated `[0, 100]`; the
  undocumented `temperature_deviation` pair; **no device-disagreement concept at all**, because there is one
  device.
- **Local / phone / cloud processing.** Oura's cloud performs the processing — the contract's own Data
  Processing Timeline stages are Data Collection → Processing → API Availability. Compare against V2's
  local-first Pi collector and the decoder rule that conversion happens once, at the decoder.
- **Storage, retention, backups, export, deletion, recovery.** CSV export through the Membership Hub;
  retention tied to account lifetime; deletion as a **tombstone value in the API** versus "deleted data is
  fully removed from your account" in the Trust Center. Compare with V2's append-only + tombstone rule and
  cite it.
- **Baselines, correlations, AI context, explainability, human review.** Contributor decompositions are
  published but **weights are not**; Oura Advisor is an AI assistant; there is no human-review concept in the
  contract. The resilience contributors are the one place where a composite is fully decomposed — and the one
  place where all three contributors are `required` and non-nullable, against three sibling models where
  every contributor is nullable and none is required. Record that inconsistency.
- **Alarms, notifications, watchdogs, dead-man behaviour, technical faults.** Complete absence, as in
  taxonomy 6. Do not soften this into *no published alarm API* — state the inventory result.
- **Behaviour when the phone, internet, cloud, database or a sensor is unavailable.** The contract documents
  lossiness, a retry count with no schedule, and a sync model that requires app interaction for sleep. It
  documents **nothing** about ring-side buffering capacity or off-finger handling. `[UNKNOWN]` for all of it,
  and the app/firmware cannot be inspected.
- **Access control, enrollment, encryption, network trust, auditability.** OAuth 2.0 authorization-code with
  scopes; four security schemes all declared and referenced; the Trust Center's AES-128 / TLS 1.2+ / AES-256
  claims as **documented-only**; SOC 2 with no type, HITRUST with no level or scope; no transparency report
  yet, by the vendor's own statement.

---

## Failure-mode scenarios to walk through

Walk each, for **Oura** and for **V2**, and label every Oura answer `documented` or `[UNKNOWN]`:

1. **A sensor stream drops during an overnight monitoring window.** What does Oura's contract imply
   (`sleep_phase_*` strings with no non-wear code; `non_wear_time` as a measured duration; a nullable score)?
   What does V2's three-state rule require? Do not assert ring-side behaviour.
2. **The phone is unavailable for a night.** Sleep data requires the app to be opened; the contract is silent
   on buffering. Contrast with V2's local collector and the spool in `pi-prototype-storage-degraded-mode`.
3. **Internet or cloud services are unavailable.** Webhook lossiness is documented; re-sync behaviour is not.
   Compare with V2's degraded-mode specification.
4. **A reading is stale, malformed, out of range or absent.** Apply V2's "reject it, name the reason,
   quarantine it" and Oura's `PublicSleepType` value `rest` (documented as "rejected in confirm prompt by
   user") side by side.
   Name the difference: **V2 rejects measurements, Oura records a rejected *detection***. Those are not the
   same object and the report must not conflate them.
5. **A clinician needs an export with source and quality context.** CSV/JSON export; `x-codeSamples` with no
   response examples; four indistinguishable timestamp types; unit-less fields. Compare with what V2's
   COMP encoder and provenance amendments are specified to carry.
6. **An alert path has a technical failure.** Oura has no alert path; the comparison is V2's specified path
   against nothing, and the honest framing is that **V2 is compared here against a product that does not
   compete on this axis** — not that V2 wins.

---

## Required report structure

`oura_COMPARISON.md`, in this order — the fourteen sections used by every other candidate in this package:

1. Executive finding
2. Product and availability
3. System boundary and architecture (ASCII)
4. Data path and source comparison
5. Claim-versus-documented-contract reconciliation
6. Feature matrix
7. Derived values versus stored signals
8. Real-time / overnight / failure-mode comparison
9. Provenance, missingness and reproducibility
10. Local/cloud/privacy and operational cost
11. Alerting and the alarm path
12. User-facing consequences
13. Limitations, unverified claims and evidence gaps
14. Balanced conclusion

Section 5 is the one that must be exact: every row is a **public claim**, what the **documented contract**
says, and a disposition. Where the claim and the contract diverge, say which governs and why.

---

## Tone and claim discipline

- **Oura is a strong technical comparator and a weak evidential one.** Both halves are findings and neither
  cancels the other. Do not let the contract's quality launder the science page's headlines, and do not let
  the science page's headlines discount the contract's genuine quality.
- **Do not overstate the gap in V2's favour.** V2's advantage is *enforcement and stated states*, not the
  existence of the rules. Oura independently arrived at tombstone deletion, named rejection reasons, nullable
  composites and per-record algorithm provenance.
- **Do not overstate the gap against Oura either.** Where the package previously recorded something as absent
  and it is in fact published elsewhere, the earlier version is corrected in the same file — and that has
  already happened twice in this candidate.
- **Do not catastrophise.** An intra-vendor count discrepancy is not deception. A documentation defect is not
  a product defect. A pending class action is not a finding of liability.
- **Never present a reverse-engineering observation as a public claim** — there are none here, and the contract
  is not one.
- **Never present a V2 specified target as shipped.**
- **Distinguish "not documented" from "does not exist" from "not measured"**, in every sentence where it
  matters.

---

## Sources to use

Priority order, highest first:

1. **`openapi-1.41.json`**, pinned by SHA-256, parsed as JSON — the primary artifact.
2. **The Terms and Conditions** (Last Updated 2026-10-02, Effective 2026-11-04 — **not yet in effect at the
   research date**) and the **Privacy Policy** (June 15, 2026), quoted verbatim.
3. **The Trust Center** (`/security`) as a counter-surface to the policy.
4. **The vendor's research and validation articles**, with their authors' affiliations and dates.
5. **The science, health-radar, membership, home and organizations pages**, and the **Google Play** and
   **iTunes Lookup** metadata.
6. **Third-party and independent sources** — for the class action and the disputed agreement figure, fetched
   directly if openly available, and labelled as the vendor's account if not.
7. **The V2 tree and `openspec/`** for every AnxietyWatch claim, cited by file and line.

**Do not re-fetch the specification** unless the pinned hash cannot be reproduced; if you do, record the new
hash and treat the old findings as re-verified or superseded.

---

## Output

Write `./docs/app_comparison/oura/oura_COMPARISON.md`.

Length and density: match `empatica/empatica_COMPARISON.md` and `nightscout/nightscout_COMPARISON.md`. Every
comparative claim carries a source and an evidence class. Every V2 claim carries a file and line. Every
disagreement is recorded, not resolved, unless the more conservative surface plainly governs — in which case
say so and name it.

Finish with a **Related files** footer linking the claimed-features file, the inspection file, this brief,
`../PROCESS_STATE.md`, `../RESEARCH_METHOD.md` and `../PROMPT1.md`.
