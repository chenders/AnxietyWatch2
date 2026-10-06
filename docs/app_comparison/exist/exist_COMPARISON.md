# Exist versus AnxietyWatch v1 and AnxietyWatch2

**Candidate:** Exist (Hello Code Pty Ltd)
**Slug:** `exist` — shortlist rank 3 of 10
**Report date:** 2026-10-06
**Base prompt:** `PROMPT_2.md`, derived from `../PROMPT2-TECHNICAL-SYSTEMS.md` with the operating-model
audit block from `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md`
**Implementation inspection:** partially performed — public API reference inspected; app binary not
(see `exist_IMPLEMENTATION_INSPECTION.md`)

AnxietyWatch status labels used throughout: **V1 documented/implemented** · **V2 current** ·
**V2 specified target** · **Unresolved**. Nothing fully specified in `openspec/` is presented as shipped.

---

## 1. Executive finding

Exist is the most **methodologically instructive** candidate on the shortlist and almost the least
competitive one on features. Both halves of that sentence are evidenced, and the comparison only makes
sense if both are held at once.

**What Exist is.** A twelve-year-old personal-analytics service that ingests data from roughly twenty
connected services and manual entry, and computes statistical correlations across it. Two founders, no
investors, ~940 paying users, published revenue. No sensor of its own, no alarm, no local store, no
clinical output.

**What Exist does that no other candidate does.** It publishes the correlation record it stores. The
public API reference ships the object itself:

```json
{ "period": 309, "offset": 0, "attribute": "sleep", "attribute2": "sleep_start",
  "value": -0.5577851552276848, "p": 1.1564227190131434e-26,
  "stars": 5, "strength_description": "Quite often go together",
  "stars_description": "Certain to be related",
  "second_person": "you spend more time asleep when you go to bed earlier." }
```

A coefficient, a **p-value**, a **sample size**, a lag offset, a 1–5 confidence rating, a plain-language
strength band, and the sentence fragments the natural-language claim is built from. The vendor also
publishes `strong` and `confident` filters, a three-week minimum-data gate, a weekly refresh cadence, and
a user-facing causation caveat that names confounding by name. `[STATIC — primary API documentation]`

**Three findings that matter to AnxietyWatch, in order of usefulness.**

1. **Exist independently solved missingness the way V2 specifies** — by *excluding* absent days, not
   coercing them. "At least three weeks' worth of data for an attribute, **excluding days with no data
   (shown as 'N/A')**", and the API returns sparse date/value pairs with **no zero-fill**. Same instinct,
   shipped in 2014. `[STATIC]` This is real external evidence that the approach works at consumer scale,
   from a system that has never had the sentinel-rejection problem V2 must solve.

2. **Exist's statistical transparency is a bar AnxietyWatch should meet or consciously refuse.** Strength
   and confidence are deliberately separate axes, and the vendor explains why in user-facing terms:
   "You might have a very strong relationship between two attributes, for example, with a **very low
   confidence if we don't have much data**." `[STATIC]` AnxietyWatch v1's correlation engine reports
   which metrics correlate with anxiety; it does not, on the documented evidence, expose sample size or a
   confidence measure to the user. That is a concrete, adoptable idea.

3. **Exist's pairwise ceiling is structural, and it is the honest shape of the problem.** The correlation
   schema has exactly two attribute slots and no covariate field. The vendor names confounding as
   something *the user* should consider. `[STATIC]` AnxietyWatch has the same conceptual exposure — a
   correlation between two metrics is not a confounder-controlled result — and the useful lesson is that
   Exist chose to be explicit about the limit rather than hide it behind a strength score.

**What Exist is not.** It is not a competitor to AnxietyWatch2 on any axis that matters — no measurement,
no provenance at sample level, no CPAP, no oximetry, no respiratory rate, no local operation, no alarm,
no overnight path, no three-state model, no clinical report. It is a **partial competitor to V1's
analysis-and-context layer**, and it is the only candidate on the shortlist that is plausibly a
**complement rather than a rival**.

**One concrete, non-speculative opportunity.** Exist publishes a supported **write** API with dedicated
"Attribute ownership", "Creating attributes" and "Updating values" sections, and its own help centre names
the API as the supported alternative to file import: "If you're a developer, you can also use our API to
add data to your Exist account." `[STATIC]` A nightly CPAP-derived or HRV-derived summary attribute could
reach a user's existing Exist account — where it would be correlated against their mood series — without
reverse-engineering anything. The counterweight is equally real: that inverts V2's local-first claim and
adds a cloud third party to the data path. Assessed in §8; not recommended.

**Verdict in one line.** Exist is a **partial competitor to V1**, **not a competitor to V2**, and the
**most plausible integration partner and methodological precedent** among the ten candidates — a
privacy-first, no-AI, no-analytics, bootstrapped product with published revenue that shipped the
missing-data behaviour AnxietyWatch2 specifies, and that is honest, in its API schema, about what its
statistics cannot do.

## 2. Product and availability

| Field | Exist | Source / class |
|---|---|---|
| Vendor | Hello Code Pty Ltd, Melbourne, Australia; founders Belle B. Cooper and Josh Sharp | vendor site `[CLAIM]` |
| Operating since | 2014 (privacy-policy update history begins 2014-01-28) | `[STATIC]` |
| Platforms | iOS, Android, **web** | `[CLAIM]` |
| Android | `co.hellocode.exist` — 10K+ downloads, 4.5★, ~187 ratings, updated 2026-09-23 | Play `[STATIC]` |
| iOS | `co.hellocode.Exist`, trackId 986201088 — 4.02★ from 47 ratings, v4.3, 14.7 MB, iOS 18.0+, released 2026-09-25 | iTunes Lookup `[STATIC]` |
| **Paying users** | **940** (vendor-published, generated from its payment processor) | `[STATIC]` |
| 4-week revenue | $6,420.85 (Exist); $6,648.45 across all Hello Code products | `[STATIC]` |
| Model | One plan, no tiers; 30-day trial with credit card up front | `[CLAIM]` |
| **Price** | **$6.99/mo USD** or **$62.90/yr (25% off** = $5.24/mo). Play copy says "US$6/month" | `[CLAIM]` + conflict `[STATIC]` |
| Hardware | None of its own; requires ≥1 connected service or manual entry | `[STATIC]` |
| Manual entry | Mobile apps only (iOS/Android); web is for review and analysis | `[STATIC]` |
| Roadmap | Public and votable | `[CLAIM]` |
| Status | Active — Android and iOS both updated late September 2026 | `[STATIC]` |

**Scale context, stated fairly.** At 940 paying users Exist is two to three orders of magnitude smaller
than Bearable (Play 500K+ downloads, "over 2M patients and doctors" claimed) or Welltory (Play 5M+
downloads, "17M users" claimed). It is also the only candidate whose scale figure is instrument-derived
rather than a download count or a marketing number, and the only one where the figure is *unflattering*
to marketing — which is exactly why it is credible. For this comparison the small scale is an asset: it
makes Exist the candidate whose **business situation** most resembles an independent project's, so its
choices are evidence about what that shape actually sustains after twelve years.

## 3. Claim-versus-implementation reconciliation

Findings are static, from authorized published artifacts. Full detail and the complete reconciliation
table are in `exist_IMPLEMENTATION_INSPECTION.md` §6. The material rows:

| # | Claim | Finding | Verdict |
|---|---|---|---|
| X1 | "Correlations are the most powerful part of Exist" | Record carries coefficient, p-value, sample size, offset, 1–5 stars, strength band, compositional sentence | **Corroborated and exceeded** — the implementation is more rigorous than the claim requires |
| X2 | "Each correlation includes indications of strength and confidence" | Separate stored fields; `strong` and `confident` filters exist | **Corroborated precisely** |
| X3 | "At least three weeks… excluding days with no data (shown as 'N/A')" | API returns sparse date/value pairs; no zero-fill, no sentinel, no null row | **Corroborated by schema** |
| X4 | "Exist will check for correlations… across two consecutive days" | An `offset` field exists (`0` in the example); **how many lags are tested is not documented** | **Partially corroborated** → E10 |
| X5 | "A third variable might affect both attributes" | Schema has exactly **two** attribute slots, no covariate, no model field | **Corroborated, and the limitation is structural** — the caveat honestly describes what the model cannot do |
| X6 | "**No AI** — we won't be stuffing LLMs into Exist to hallucinate what your data means" | Insights are **typed** (`dow_sleep`), carry a period and priority, and are composed of clause fragments, not free prose | **Consistent** — the generator is unpublished, so consistency, not proof → E8 |
| X7 | "We finally turned off Google Analytics… no third-parties watching how you use the site" | Policy update history records the 2024-09-19 removal; no analytics or ad partner named; **Play declares no data shared** | **Corroborated by two independent sources** |
| X8 | "We don't, and won't ever, sell your data" | Policy: never sold "as part of the everyday process… **but it may be transferred as part of the sale, merger, or dissolution**", with notice and opt-out | **Qualified** — absolute marketing claim, disclosed legal carve-out → E1 |
| X9 | "Permanent deletion — everything… permanently wiped" | Policy: kept "for a commercially reasonable period, for backup and archival purposes"; payment records and support emails retained | **Qualified** — reconcilable, not identical |
| X10 | "Not building features to let your boss, colleagues, or coach see your data" | No sharing, coach or clinician surface anywhere in the API, catalogue or help centre | **Consistent** |
| X11 | "You can export it at any time" | **PDF, CSV and JSON**, plus programmatic read/write via the API | **Corroborated** |
| X12 | "No pricing tiers, one plan gets you everything" | Structure confirmed; **price conflicts** (website $6.99 vs Play "US$6") | **Corroborated for structure, conflicting on price** |
| X16 | Mood 1–9, optional note, "a year ago" view | Help article matches exactly; the 1–9 scale is **rationalised** (5 as exact midpoint; range for symptom severity, energy, mood) | **Corroborated, with a documented rationale** |
| X17 | "Run experiments on your behaviour, and Exist will confirm or rule out your hunches" | **No experiment object, endpoint or record exists in the API.** It is the correlation engine applied to a user-chosen pair | **Marketing term without a distinct implementation surface** — the capability is real; the framing is presentation |
| X18 | "Exist can even uncover what happens when you don't track a tag" | No tag-absence field, endpoint or mechanism found | **Unverified** → E3 |
| X20 | Published scale | 940 paying users, $6,420.85/4 weeks | **Corroborated by vendor's own instrumentation** |

**The one claim that is stronger than the implementation.** X17. "Experiments" is presented as a
capability; the API has no experiment record, and the help centre describes an experiment only as the
correlation engine run against a chosen pair. The distinction is not trivial: a *designed* experiment has
a pre-registered hypothesis and a defined window, whereas a retrospective correlation mined from
arbitrary data has neither. Exist's marketing implies the former and its system provides the latter. This
is a **mild but real overstatement**, and it is worth naming precisely because the rest of Exist's public
material is unusually disciplined.

**The claim that is notably under-sold.** X1. "Correlations are the most powerful part of Exist" is
marketing language, but the shipped implementation computes a p-value, stores the sample size, and
separates confidence from strength — and then exposes all of it to clients rather than keeping it
internal. That is a more sophisticated posture than the prose suggests, and it is the single most
transferable finding in this report.

## 4. Evidence ledger

| Class | Count | Notes |
|---|---|---|
| `[CLAIM]` public vendor statements | — | Homepage, values page, Play/iOS listings, help centre prose |
| `[STATIC]` authorized published artifacts | **the bulk** | API reference (primary), attribute catalogue, help articles, privacy policy, published revenue |
| `[RUNTIME]` runtime observations | **0** | No app installed, no traffic intercepted, no API endpoint called |
| `[INFERENCE]` | few, labelled | `period` as sample size; `offset` as a day lag; the insight generator being template-based |
| `[UNKNOWN]` | E1–E12 | See §9 |

Strongest evidence, in order: the **correlation record** in the API reference; the **sparse value model**
in the attributes reference; the **insight record** showing baseline-relative typed text; the **published
revenue**; the **privacy policy** with its own update history; the **Play data-safety declaration**.

Weakest, and treated as unverified: everything about the Android and iOS implementations, the internal
statistical method, the insight generator, the live behaviour of any endpoint, and the hosting
jurisdiction.

## 5. Feature matrix

| Capability | V1 documented/implemented | V2 current | V2 specified target | **Exist** | Evidence / maturity |
|---|---|---|---|---|---|
| **Measures a signal itself** | Yes — Polar H10 RR, CPAP SD/bridge, EMAY oximetry, barometer, environmental sound | Collection crates exist | Continuous, multi-source | **No** — aggregator only | Exist `[STATIC]`; V1 docs |
| Raw samples retained | Yes — beat-to-beat, per-sample rows | Append-only store | Provenance per sample | **No** — daily summaries only | Exist: HRV/sleep as numbers |
| Multi-source provenance | Yes — `sourceBundleID`, `sourceName`, device model | Provenance model | Per-sample, registry-driven | **Attribute-level only** — `service` + `available_services` | Exist `[STATIC]` |
| **CPAP / AHI / leak / adherence** | Yes — AirSense 11 SD import (OSCAR CSV), server EDF leak parsing, **live 25 Hz flow/pressure via the AirSense 11 Bridge** | `openspec` ingest change | First-class domain | **Nothing** | Absence of evidence in the catalogue (E-24) |
| Oximetry / SpO₂ | Yes — EMAY CSV import + live BLE streaming | — | Registry source | **Nothing listed** | Absence of evidence |
| Respiratory rate | Via CPAP/oximeter | `xe125` proposal | Registry source | **Nothing listed** | Absence of evidence |
| HRV | Yes — Polar H10 chest strap, RR fidelity | Collected | Registry source | **Imported daily number** | Exist `[STATIC]` |
| Sleep stages | Yes — HealthKit | Collected | Registry | **Imported** | Exist `[STATIC]` |
| Barometric pressure | Yes — Core Motion | Collected | Environment source | **Imported, automatic** | Exist `[STATIC]` |
| Weather context | Yes | Collected | Environment | **9 fields, automatic** (temp, precipitation, pressure, cloud, humidity, wind, day length, summary) | Exist `[STATIC]` — broadest on the shortlist |
| Environmental sound / noise | Yes — environmental sound levels | — | Noise tracking | **Nothing** | Absence of evidence |
| **Aircraft overflight** | Specified | — | Environment | **Nothing** | Absence of evidence — fourth candidate in a row |
| Mood / subjective | Yes — anxiety severity instrument | Client UI | — | **1–9 daily + note, anniversary view** | Exist `[STATIC]` |
| Anxiety-specific instrument | Yes | — | — | **None** — anxiety is an example tag | Exist `[STATIC]` |
| Panic-time capture | Yes — watchOS quick log | — | — | **None** — one rating per day, retrospective | Exist `[STATIC]` |
| Medication model | Yes — doses with paired follow-up | — | — | **Free-form tag/number only** | Exist `[STATIC]` |
| Correlation engine | Yes — live, per-metric breakdowns, scatter plots | Implemented | Extended | **Yes — coefficient + p-value + n + stars** | Exist `[STATIC]` |
| Confidence / sample size exposed to user | Not evidenced | Unresolved | — | **Yes** (`stars`; `period`/`p` in API) | **Exist leads** |
| **Missing-data model** | Three-state intent in the app's own framing | Three-state | **Three states + quarantine + no clamping** | **Sparse; absent days excluded and shown "N/A"** | Exist `[STATIC]` — correct behaviour, no named third state |
| Sentinel rejection (SFLOAT 0x07FF/0x0800) | — | — | **Specified** | **N/A** — never ingests a sensor protocol | Structural |
| Out-of-range rejection / naming a reason | — | — | **Specified** | **Not documented anywhere** | E5 |
| Live alert on a personal baseline | Yes — HRV-below-baseline dashboard alert | — | Supervised alarm | **No alert of any kind** | Exist `[STATIC]` |
| CNS / respiratory-depression watch | Phase 1–2 merged and unit-tested; **no user-facing alarm (Phase 3 not shipped)** | — | Supervised alarm + dead-man switch | **None** | V1 docs, explicitly not shipped |
| Alarm / watchdog / dead-man switch | Partial (dashboard alert) | — | **Specified** | **None** | Exist `[STATIC]` |
| Overnight capture | Yes | Pi collectors | Continuous | **None of its own** | Exist `[STATIC]` |
| Real-time path | Yes — 25 Hz CPAP bridge, 1 Hz leak/SpO₂/HR | Streams | Near-real-time | **None** — hourly-to-daily pull, **weekly** correlations | Exist `[STATIC]` |
| Local-only operation possible | Yes — HealthKit source of truth; sync server optional and self-hosted | Local console | Local-first | **No** — mandatory account, vendor cloud | Exist `[STATIC]` |
| Export | Yes — JSON/CSV; **clinical PDF** report | — | Reports | **PDF + CSV + JSON** | Exist `[STATIC]` — broader formats; not clinical |
| Clinical report | Yes — psychiatrist-oriented PDF | — | — | **No** — generic personal report | Exist `[STATIC]` |
| Manual file import | Yes — CPAP CSV, EMAY CSV | — | — | **No** — unsupported; API is the documented workaround | E-16 |
| **Write API for third parties** | — | — | — | **Yes** — documented create/update | **Exist leads; integration path** |
| AI / LLM analysis | Yes — opt-in, self-hosted server calls the Claude API with *your* key | — | — | **Explicitly refused** | Both `[STATIC]`/`[CLAIM]` |
| Telemetry / analytics / ads | None claimed; no third-party SDKs | — | — | **None** (Google Analytics removed 2024-09-19); Sentry for crash/error only | Exist `[STATIC]` |
| Self-hosting | Sync server optional, self-hosted | Pi | Pi-first | **No** | Exist `[STATIC]` |
| Third-party review | — | — | — | No clinical validation; no clinical claims at all | Exist `[STATIC]` |
| Hardware cost | Polar H10, CPAP, oximeter, Apple Watch | Pi 5 + sensors | Pi + sensors | **$0 of its own** | — |
| Price | one-time app + optional hardware | — | — | **$6.99/mo or $62.90/yr** | Cheapest on the shortlist |

## 6. Evidence-backed workflows

Where verifiable, each workflow states what the evidence actually supports.

### 6.1 Panic logging at the moment it happens

**V1:** watchOS quick log — the physiological and subjective record are captured at the moment of the
event, which is what makes a *panic-time* rating different from a retrospective one.

**Exist:** no equivalent, and the difference is architectural rather than a missing screen. Exist's mood
rating is **once per day**, delivered by a notification or an email the user can reply to, and its
purpose is a daily aggregate for correlation. There is no event model, no timestamp finer than a date
(`date: "2022-05-16"` throughout the API), and no way to attach a mood to a moment. `[STATIC]`

**Consequence.** Exist cannot answer "what was happening when this got worse" — only "on days like this,
the day's rating tended to be lower". For the comparison this is the single cleanest illustration of the
boundary: Exist's unit of analysis is **the day**, and AnxietyWatch's is **the event**. Every downstream
difference in this report follows from that one design choice.

### 6.2 Sleep review the morning after

**Exist:** genuinely useful and evidence-backed. Sleep arrives as stages and durations from Oura, Fitbit,
Garmin, Apple Health or Health Connect; weekly averages are stored per attribute with a **day-of-week
breakdown** (`monday` … `sunday`, `overall`); and a documented insight type (`dow_sleep`) produces text
such as "Last night's sleep was surprisingly long. / Monday is usually your shortest sleep of the week."
`[STATIC]`

**V1:** richer source (CPAP AHI/leak/usage, oximetry, HRV) and a baseline comparison, plus a PDF.

**Where Exist is instructive:** the day-of-week pattern insight. It is cheap, baseline-relative, and
states something the user can act on. AnxietyWatch's correlation engine could express the same and, on the
documented evidence, does not present it in that form. Recorded as an idea, not a gap claim.

### 6.3 CPAP / respiratory investigation

**V1:** AirSense 11 SD import with AHI, leak, usage, pressure and event breakdowns; server-side EDF parsing
for leak percentiles; a live AirSense 11 Bridge at 25 Hz flow/pressure and 1 Hz leak/SpO₂/HR for real-time
fault detection.

**Exist:** nothing. No CPAP, PAP, AHI, leak or adherence attribute appears anywhere in the published
attribute catalogue, and no respiratory-rate or SpO₂ attribute does either. `[absence of evidence]`

**Consequence.** The entire respiratory and therapy domain — which is the reason AnxietyWatch v1 exists
in its current form — is outside Exist's data model. This is a **scope boundary**, not a defect: Exist
never claimed it. It is, however, decisive for the competition verdict.

### 6.4 Medication follow-up

**Exist:** a user-invented tag or manual attribute. The custom-tracking page suggests tracking
"medications" and asks "how medications affect your productivity". So medication *data* is supported;
medication *semantics* are not. There is no dose, no schedule, no reminder, no drug-class concept, and
no event to trigger anything.

**V1:** dose events with paired follow-up, so a dose can arm a window and an outcome can be attached to it.
**V2 specified:** CNS respiratory-depression watch armed by a logged benzodiazepine or opioid dose; Phase 1–2
merged and unit-tested, **no user-facing alarm shipped** (Phase 3).

**Consequence.** Diagnostically this is the most important scope difference after CPAP. Inside a generic
attribute model, a medication is a *label*; in AnxietyWatch it is a *causal anchor with a time window*.
Exist's architecture cannot express the latter — there is no per-event record to hang a window on.

### 6.5 Environmental interruption analysis

**Exist** has the broadest weather coverage on the shortlist: temperature (max/min), precipitation, **air
pressure**, cloud cover, humidity, wind speed, day length and a weather summary, automatic via Apple
Weather. Because everything in Exist is an attribute, each of these is correlatable against mood, sleep
and HRV with no extra modelling. `[STATIC]`

**V1** collects barometric pressure and environmental sound levels. **Neither candidate addresses aircraft
overflight or environmental noise as a sleep interrupter** — Exist has nothing, and V1 has sound levels
without an identifiable-source model. This is the fourth candidate examined with no aircraft-overflight
equivalent, which continues to support the view that the feature is unclaimed territory rather than an
oversight shared by a market.

**The transferable architectural point.** Exist's marginal cost for a new environmental variable is
effectively zero, because the variable is just another attribute. AnxietyWatch2's registry-driven design
achieves the same property more strictly (typed, ranged, unit-canonical) but at the cost of a
regeneration step. Both are right for their contexts; the comparison is worth recording because it is the
one place where Exist's generic model is a genuine engineering advantage rather than a convenience.

### 6.6 Clinical sharing

**Exist:** a **PDF report, CSV spreadsheet, or full JSON export**, available from the account page. `[STATIC]`
Three formats is more than Bearable (CSV only) or Welltory (a printable blood-pressure report only).

**But it is not a clinical artifact.** There is no medical framing, no reference ranges, no
psychiatrist-oriented structure, no adherence summary and no provenance or quality context beyond the
attribute's source service. An AnxietyWatch V1 clinical PDF carries anxiety-severity distribution,
medication adherence per drug, sleep stages, HRV trend with baseline status, CPAP compliance, blood
pressure and labs with reference ranges. Exist's PDF is a personal-analytics report.

**Consequence.** Exist leads on **format breadth** and loses completely on **clinical content** — and this
is a case where breadth is genuinely valuable to AnxietyWatch, because a portable JSON export is the
cheapest possible bridge between the two systems.

### 6.7 Offline / phone-independent operation

**Exist:** total dependency. If the internet or the vendor's cloud is unavailable, no new data arrives at
all — every input is either a connected service (fetched server-side) or a manual entry stored in the
vendor's cloud. The app is a client to a service, not a local store. `[STATIC, from architecture]`

**V1:** HealthKit as source of truth on-device; the sync server is optional and self-hosted. **V2
specified:** local-first, Pi-resident, degrade rather than fail.

**Consequence.** This is the sharpest operating-model difference in the report and the one where Exist
cannot be defended by scope. A privacy-first product with a mandatory account and a single-vendor cloud is
privacy-first in *policy* and cloud-dependent in *architecture*. It is the clearest single illustration of
why AnxietyWatch's local-first position is a structural distinction and not a slogan.

### 6.8 Longitudinal self-study over a year

**Exist:** strong, and this is where twelve years of operation shows. Weekly averages are **stored and
queryable** with `include_historical=1`, day-of-week patterns are first-class, the three-week gate means
correlations are not reported until they are statistically defensible, lags are checked across consecutive
days, and the mood feature surfaces the same day a year ago. `[STATIC]`

**V1/V2:** longer raw retention, richer signals, immediate baselines — but a younger system with no
comparable published discipline around statistical confidence.

**Consequence.** For a user asking "what actually affects me, over a year", Exist's *statistical* posture
is currently more mature than AnxietyWatch's published posture, and it is worth saying so plainly. The
difference is that Exist can say "this relationship is strong but our confidence is low because we only
have 40 days" and AnxietyWatch's documented engine does not expose that distinction.

## 7. Data and analysis audit

### 7.1 The analytical contract

Exist stores and exposes, per correlation: `value` (coefficient), `p` (**p-value**), `percentage`
(presentation transform of `value`), `period` (**sample size**, per the confidence explanation in the
help centre — labelled `[INFERENCE]` because the reference does not define it), `offset` (lag; semantics
undocumented → E10), `stars` (1–5 confidence), `strength_description` and `stars_description`
(plain-language bands), and `second_person` **with** `second_person_elements`.

**The composition detail deserves emphasis.** The natural-language claim is stored alongside the fragments
it is built from — `["you spend more time asleep", "when", "you go to bed earlier"]`. The claim is
therefore assembled from identified parts, not generated. That is consistent with the vendor's explicit
"no AI" position, and it is a design pattern AnxietyWatch could adopt for its own generated summaries:
store the reasoning parts, not just the sentence.

### 7.2 What the model provably does not do

- **No multivariate analysis.** Two attribute slots, no covariate, no model identifier. Confounder control
  is *named as the user's job*. `[STATIC — structural]`
- **No published statistic identity.** Whether `value` is Pearson, Spearman or something else is not
  documented; neither is any multiple-comparison correction, despite the vendor's own example returning
  `count: 479`. With hundreds of pairwise tests, an uncorrected p-value threshold would produce false
  positives — and **whether Exist corrects for this is unknown** → **E6**. This is the most important open
  technical question about Exist's statistics, and it is the question AnxietyWatch should ask of itself
  too.
- **No published thresholds** behind `strong` and `confident` beyond the words "above a certain
  relationship strength" and "five-star confidence".
- **No published validation.** No benchmark, no accuracy study, no third-party evaluation was found.
- **No clinical claim** of any kind, which is consistent and honest.

### 7.3 Missingness, staleness and the three-state rule

| Dimension | V2 specified | Exist |
|---|---|---|
| Absent observation | Third state, explicitly modelled and named | **Sparse — no row exists** |
| Rendered as | Never good or bad | "N/A" |
| In correlation | Contributes nothing | **Excluded pairwise** `[STATIC]` |
| Zero-fill | Forbidden | **None present** `[STATIC]` |
| Sentinel coercion | Forbidden (SFLOAT `0x07FF`/`0x0800` rejected) | **N/A** — no sensor protocol ingested |
| Out-of-range value | Rejected, reason named, quarantined | **Not documented anywhere** |
| Clamping | Forbidden | **Not documented** |
| **Stale value** | Distinguished from absent | **Unknown** — and this is the sharpest test → **E12** |

**The honest reading.** Exist has the **right behaviour** (absent data is excluded, never zero-filled)
without an explicit **third state** (a distinguishable, queryable "not measured" marker). It has not
needed one, because it has never ingested a sentinel-bearing sensor protocol — a physiological device
that reports `0x07FF` for "no reading" is a problem Exist's data model has never met.

**Do not assume the omission model generalises.** A sparse-list model plus "exclude absent" handles
*absence* well. It says nothing about *invalidity*: a value that is present but wrong, out of range, or
stale. V2's three-state model exists precisely because a device can report a number that must not be
treated as a measurement. Exist's documentation shows no mechanism for this, and question **E12** — does
an attribute freeze at its last value when its source stops reporting? — is the test that settles whether
Exist's discipline is principled or merely convenient.

### 7.4 Latency as a design budget

Exist accepts up to **seven days** of analysis latency (weekly Monday refresh) and up to a day of ingest
latency, in exchange for a three-week minimum-data gate and a confidence measure. That is a coherent
trade: the product is a weekly retrospective, and the latency is what buys statistical defensibility.

AnxietyWatch's real-time path (a 25 Hz CPAP bridge, 1 Hz leak/SpO₂/HR, beat-to-beat HRV) buys the
opposite: immediate, actionable, and — with the CNS watch — safety-relevant. The two are not comparable
on a single axis, and the correct conclusion is not "one is better". It is that **AnxietyWatch's
real-time capability creates an obligation Exist does not have**: a system that reports within seconds
must be certain what it is reporting, which is exactly why V2 specifies rejection, named reasons and
quarantine. Exist, reporting weekly, can afford to wait three weeks.

### 7.5 Provenance and correction

Exist's model, from published rules:

- **One designated `service` per attribute**, with an `available_services` fallback list; on disconnect,
  another connected service takes over or the attribute is disabled; history is retained. `[STATIC]`
- **Attribute-level provenance only** — no per-sample source, device identity, quality flag or timestamp.
- **Correction is delegated to the source service** for synced data. For manual data, a value for a date
  **cannot be removed**, only replaced ("track 0 to change the value"). `[STATIC]`
- **Vendor-defined and template attributes cannot be deleted**, only deactivated; user-defined attributes
  and tags can be permanently deleted with all their data. `[STATIC]`

**Comparison with append-only.** Exist's manual rule — the date slot, once filled, stays filled — is
*closer* to AnxietyWatch's tombstone discipline than Bearable's editable factors or Welltory's ordinary
CRUD. But the mechanism is **substitution, not tombstone**: there is no record of what the value was
before, and no audit trail. AnxietyWatch2's append-only store with tombstone rows preserves the fact that
a value existed and was withdrawn; Exist preserves only the latest value. The similarity is directional,
not equivalent.

## 8. Local/cloud/privacy and operational cost

### Privacy and data governance

| Aspect | Exist | Class |
|---|---|---|
| Account required | **Yes** — mandatory; no local-only mode | `[STATIC]` |
| Processing | Vendor cloud; the app is a client | `[STATIC]` |
| **No data sale** | "We don't, and won't ever, sell your data. You pay for Exist. You (and your data) are not the product." | `[CLAIM]` |
| Acquisition carve-out | "…but it may be transferred as part of the sale, merger, or dissolution of Exist or Hello Code", with notice and opt-out; clarified in the 2018 update | `[STATIC]` → E1 |
| **No analytics** | Google Analytics removed; policy update history records 2024-09-19 | `[STATIC]` |
| **No advertising** | No ad partner, pixel or audience named anywhere | `[STATIC]` |
| Sub-processors | Help Scout (support), Stripe (billing), Mapbox (suburb from coordinates), Apple Weather, Campaign Monitor (product email), **Sentry (crash/error, incl. username, phone model, OS version, and possibly attribute-related data in error logs)** | `[STATIC]` |
| Hosting | Deliberately **outside the United States**; jurisdiction unnamed | `[STATIC]` → E9 |
| Retention | Backups for "a commercially reasonable period"; payment records and support emails retained | `[STATIC]` |
| Deletion | Account deletion wipes personal data; backups and billing records excepted | `[STATIC]` |
| Disconnecting a service | Does **not** delete history | `[STATIC]` |
| Health Connect | Explicit Limited Use statement in the policy | `[STATIC]` |
| **Play data safety** | **No data shared**; collects **Location** + **App info and performance**; encrypted in transit; deletion requestable | `[STATIC]` |
| "Your data is for you" | "We're not building features to let your boss, colleagues, or coach see your data" | `[CLAIM]` |
| **No AI** | "We don't chase trends and won't be stuffing large language models into Exist to hallucinate what your data means. We stick with old-fashioned statistical methods." | `[CLAIM]` |

**Two tensions, both disclosed.** (1) "Won't ever sell your data" versus the acquisition carve-out — the
marketing is absolute, the legal text is qualified, and the policy's own update history shows the
qualification was deliberate rather than buried. (2) "Permanent deletion" versus backup retention for "a
commercially reasonable period". Both are reconcilable; neither is identical. Neither is a
misrepresentation.

**One unresolved observation, without accusation.** The Play declaration does not list **Health and
fitness** among collected types, despite the app's entire function being to ingest health data. The most
likely explanation is that Health Connect data is governed by a separate declaration regime — the policy
carries an explicit Limited Use statement — but this was **not verified**, and it is carried as **E4**,
not asserted as a discrepancy.

**The Sentry exception deserves emphasis.** "No telemetry" is not literally true: Sentry receives crash
and error reports including username, phone model, OS version, and — per the policy's own wording —
possibly attribute-related data in error logs. That is a narrow, ordinary, defensible use, and it is
disclosed. It is still an exception, and the honest summary is **"no analytics or advertising; error
reporting to Sentry"** rather than "no third parties".

### The AI refusal as a position

Exist's "no AI" statement is the most categorical public position on generative AI found on this
shortlist, and it sits between two opposing approaches: **Welltory** ships an OpenAI-GPT "AI Coach" whose
Beta carries an explicit hallucination warning and which requires both a Premium subscription and a
ChatGPT Plus account, and **Bearable** recommends that free-tier users export a CSV and analyse it with
ChatGPT. **V1** takes a third path: an opt-in, self-hosted admin page that calls the Claude API using the
user's own key against a chosen date range, with data leaving the device only when the user presses the
button.

**Assessing the refusal honestly.** Exist's position preserves traceability — every statement in the
product is a stored, typed record with a computed basis. That is a *real* engineering benefit and it
matches AnxietyWatch2's general insistence that a conclusion be derivable from the data. But the refusal
is also a genuine capability limit: Exist cannot do the plain-language synthesis over a complex,
multi-signal night that V1's Claude path and V2's specified analysis are meant to provide. The correct
reading is that **Exist and AnxietyWatch made opposite bets on the same question** — Exist that
statistics alone suffice and generation cannot be trusted, AnxietyWatch that generation is worth having
when it is opt-in, keyed to the user's own account, and grounded in stored data. Both bets are defensible
and the comparison should record the disagreement rather than resolve it.

### Cost and lock-in

| | V1 | V2 specified | **Exist** |
|---|---|---|---|
| Software | one-time app | self-hosted | **$6.99/mo or $62.90/yr** |
| Hardware | Polar H10, CPAP, oximeter, Apple Watch | Pi 5, storage, sensors | **$0 of its own** (but useless without other services the user already pays for) |
| Electricity / hosting | optional self-hosted server | Pi ~continuous | **included** |
| Export | JSON/CSV + clinical PDF | reports | **PDF + CSV + JSON** |
| Programmatic export | — | — | **Read/write API** |
| Exit cost | low | lowest | **low, and documented** |

**Exist is the cheapest paid candidate on the shortlist and has the lowest lock-in.** $62.90/year against
Welltory's $99/year or $599 lifetime, with no mandatory hardware, three export formats on the base plan,
and a documented API for programmatic exit. Contrast with Welltory, which **deletes free-tier data older
than 30 days** and blocks account deletion while a subscription is active, and with Bearable, whose
correlation features are subscription-gated and whose export is CSV-only with no PDF.

**"Free" discipline, applied to both sides.** Exist is not free — $62.90/year, and it is close to useless
without a connected service, so the true cost includes whatever tracker the user already pays for.
AnxietyWatch2's Pi + sensors + electricity is also not free. Neither product should be described as free
without naming those costs.

### The integration path, assessed

Exist publishes a documented **write** API — "Attribute ownership", "Creating attributes", "Updating
values", a "Sharing your integration" guide — and its help centre names the API as the supported
alternative to file import: "If you're a developer, you can also use our API to add data to your Exist
account." `[STATIC]`

**What it would enable.** A CPAP-derived or HRV-derived nightly summary could become a user-defined Exist
attribute, and would then be correlated against that user's existing mood series by Exist's own engine —
with a p-value, a sample size and a confidence rating. For a user who already keeps a mood log in Exist
and a CPAP in the bedroom, that is a real capability neither product has alone.

**What it would cost.** It inverts V2's local-first claim: AnxietyWatch-derived data would leave the
device and land in a third-party cloud, under a mandatory account, with the acquisition carve-out in the
privacy policy applying to it. It also adds a maintenance dependency on a two-person company with 940
users.

**Assessment: do not recommend; assess.** The finding's real value is not "integrate". It is that the
cheapest interoperability path between two health systems on this shortlist is a **documented API rather
than a reverse-engineered one** — which is worth knowing when judging how AnxietyWatch should present its
own data to the outside world. AnxietyWatch's advantage here is the reverse: its JSON/CSV export and
clinical PDF require no vendor cooperation at all.

## 9. What cannot be concluded

**Unknowns, with the artifact that would resolve each**

| # | Unknown | Would be resolved by |
|---|---|---|
| E1 | How the absolute no-sale claim and the acquisition carve-out reconcile in practice; what notice and opt-out a user actually receives | The full terms of service, and the 2018 policy-update diff |
| E2 | Whether absent days are dropped pairwise in the correlation, and how intermittent coverage affects `period` and `stars` | The statistics documentation, or a controlled test against a live account |
| **E3** | The mechanism behind "uncover what happens when you don't track a tag" — is tag absence a categorical level? | Help-centre detail or the custom-tags implementation |
| E4 | Why Health and fitness is not in the Play data-safety declaration despite the app's function | Google's Health Connect declaration guidance, or the vendor |
| **E5** | The full `value_type` enumeration, and **whether one canonical unit per attribute is enforced** — can two services supply different units to the same attribute? | `reference/object-types-and-terminology` (404 at access) |
| **E6** | The numeric thresholds behind `strong` and `confident`, whether `p < 0.05` is implied, and **whether multiple comparisons are corrected** given hundreds of pairwise tests | Vendor documentation or a published method note |
| E7 | Whether the PDF export carries correlations with strength and confidence, or only charts and averages | Generating an export from a live account |
| E8 | Whether insight texts come from a documented personal-baseline rule | Vendor documentation or source |
| E9 | The actual hosting jurisdiction | The vendor's sub-processor list or DPA |
| E10 | What `offset` means and how many lags are tested | The correlations reference or a live response |
| E11 | Refund policy and the interaction of the up-front card with cooling-off rights | The terms of service |
| **E12** | Whether an attribute freezes at its last value, goes to "N/A", or is disabled when its source stops reporting | A controlled test; **the decisive test of the missing-data discipline** |

**Claims this report deliberately does not make**

- That Exist's correlations are correct, or more correct than AnxietyWatch's. A published p-value is
  evidence of engineering discipline, not of accuracy, and the statistic used and any correction applied
  are unpublished.
- That Exist is "private" in an architectural sense. It is private *in policy* and **cloud-dependent in
  architecture** — a mandatory account and a single-vendor cloud. Those are different things and only the
  first is evidenced.
- That Exist cannot support CPAP, oximetry or respiratory rate. No attribute is *listed*; that is absence
  of evidence.
- That Exist's insight generator is not AI, or that its "no AI" claim is verified. Its typed, compositional
  insight records are **consistent** with the claim; the generator is unpublished.
- That Exist's missing-data handling would generalise to sensor protocols with sentinels, invalid ranges
  or stale values. It has never needed to handle them.
- Any medical, clinical, diagnostic or emergency-monitoring status for either product. Exist makes no
  health claim at all; V1's CNS watch is explicitly an early-warning aid with no user-facing alarm shipped.
- Any characterisation of the "Experiments" feature as a designed experiment. No experiment record exists.

**Inspection limitations.** The Android and iOS binaries were not inspected — no APK surface, no network
endpoints, no local storage, no bundled SDKs, no permissions beyond the Play declaration. **No API endpoint
was called**, so every API finding is a documented contract, not observed behaviour. **There are no runtime
observations for this candidate at all.** The mitmproxy toolchain validated in the Bearable pilot was
available and deliberately not applied, for the same authorization reason recorded there.

## 10. Balanced conclusion

**Against V1: partial competitor, and a genuine methodological peer.**
Exist overlaps V1 where V1 does its analysis rather than its measurement: subjective ratings alongside
objective data, personal baselines rather than population norms, correlation as the central claim, weather
and air pressure as context, medication tracked, an explicit refusal to imply causation. On **statistical
transparency Exist is currently ahead** — coefficient, p-value, sample size and a separate confidence axis,
with a categorical exclusion rule for missing days — and its three export formats beat V1's JSON/CSV plus
clinical PDF on breadth if not on clinical content.

Against that, Exist **measures nothing**, has no CPAP, oximetry, respiratory-rate or blood-pressure
attribute, no panic-time capture, no medication semantics, no alarm, no local operation and no clinical
report. Its analysis latency is a week. Its missing-data model handles absence but has never met
invalidity, staleness or a sentinel.

**Against V2: not a competitor.** No continuous collection, no provenance at sample level, no append-only
store, no local console, no alarm, no dead-man switch, no CPAP or oximetry domain, no three-state model.
The only genuine architectural affinities are that Exist independently excludes absent data rather than
coercing it, and that it separates strength from confidence — both instincts V2 formalises further.

**The three findings worth carrying forward, in priority order**

1. **Expose confidence separately from strength.** Exist's strongest transferable idea, explained in its
   own words: a relationship can be strong with very low confidence when there is little data. AnxietyWatch's
   documented correlation engine reports which metrics influence anxiety without, on the published evidence,
   exposing sample size or confidence. Adopting the split — and being willing to say "we cannot tell yet" —
   is the single cheapest improvement this comparison suggests, and it is one Exist has validated for
   twelve years at consumer scale.
2. **Keep the reasoning parts, not just the sentence.** Exist stores `second_person_elements` alongside the
   rendered claim. Generated summaries in AnxietyWatch should be traceable to the components that produced
   them; that is the same discipline as provenance, applied to text.
3. **Ask the multiple-comparisons question of AnxietyWatch's own engine.** Exist returns hundreds of
   pairwise correlations and does not publish whether it corrects for multiple comparisons. The question
   is uncomfortable and applies equally to AnxietyWatch's correlation engine, and it is better asked by the
   project than by a user.

**And one thing to refuse, or at least to face.** Exist's refusal of generative AI is coherent, tested over
twelve years and matched by a traceable implementation. AnxietyWatch V1's opt-in Claude path and V2's
specified analysis make the opposite bet. Exist's position is evidence that a statistics-only product can
be honest, useful and durable — and a reminder that "we do not use AI" is a position a project can hold
deliberately rather than defensively.

**Complementary use.** This is the strongest complementarity case on the shortlist. Exist is where a user's
*existing* data lives and where it can be correlated with statistical discipline; AnxietyWatch is what
actually measures the night, with provenance and a therapy domain Exist has no model for. The two could
coexist for the same user with no overlap in function — and Exist is the only candidate where a documented,
vendor-sanctioned API makes the connection possible without reverse engineering, even though the
local-first trade-off means this comparison should record the option rather than recommend it.

**Final verdict.** Exist is a **partial competitor to V1's analysis layer**, **not a competitor to V2**, and
the **most plausible integration partner and the most instructive methodological precedent** among the ten
candidates — a privacy-first, no-AI, no-analytics, bootstrapped product with published revenue that shipped
the missing-data behaviour AnxietyWatch2 specifies, and that is honest, in the schema of its own API, about
what its statistics cannot do.

---

## Related files

- Claims and purpose: `exist_CLAIMED_FEATURES_AND_PURPOSE.md`
- Implementation inspection: `exist_IMPLEMENTATION_INSPECTION.md`
- Candidate-specific prompt: `PROMPT_2.md`
- Shortlist: `../CANDIDATE_SHORTLIST.md`
- Method and limits: `../RESEARCH_METHOD.md`
