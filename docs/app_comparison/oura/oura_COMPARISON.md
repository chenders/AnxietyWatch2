# Oura — comparison against AnxietyWatch v1 and AnxietyWatch2

**Candidate:** Ōura (Oura Ring 5 / 4 / 4 Ceramic, Oura App, Oura Membership, published OpenAPI v2, Oura Enterprise Platform)
**Candidate slug:** `oura`
**Research date:** 2026-10-06
**Inspection mode:** *Partially performed — documented contract only.* No binary inspected. No source inspected.
**No endpoint called.**
**Primary artifact:** `openapi-1.41.json`, 454,152 B, SHA-256
`a144a3436e9b73da568999fc713257e1d73e12cd3f7024eef45035c9d8f6b557`, OpenAPI 3.1.0, `info.version` 2.0,
server `https://api.ouraring.com`.
**Companion documents:** `oura_CLAIMED_FEATURES_AND_PURPOSE.md` (claim ledger O-1…O-95, artifact hashes,
evidence gaps G1–G13) and `oura_IMPLEMENTATION_INSPECTION.md` (Phase 2A; contract findings, reconciliation
points R1–R13, 50-row claim reconciliation, gaps I1–I11).

**Evidence classes used in this report** — `[PUBLIC CLAIM]`, `[DOCUMENTED CONTRACT]`, `[REGULATOR RECORD]`,
`[THIRD PARTY]`, `[INFERENCE]`, `[UNKNOWN]`, and `[runtime observation]`, of which this candidate has
**zero**. Nothing below is a runtime observation and no sentence should be read as one.

---

## 1. Executive finding

**Oura is the strongest technical comparator in this package and one of the weaker evidential ones, and the two
facts are independent.**

On the technical side, Oura publishes a contract that is large enough to be checked and good enough to
survive the check. The specification resolves 274 of 274 `$ref`s; declares four security schemes and
references all four; documents a **tombstone deletion value** (`PublicSleepType.deleted`), a **retained
rejection with a named reason** (`rest` — documented as "Falsely detected sleep / nap, rejected in confirm
prompt by user"), a **per-record algorithm version** (`PublicSleepAlgorithmVersion` ∈ `{v1, v2}`), a
**mutation-reason enumeration** (`PublicSleepAnalysisReason`), an explicit **non-wear code** in a
character-encoded activity field (`class_5_min` `'0'` = non wear), a **measured** non-wear duration
(`non_wear_time`), a **nullable score with an all-nullable contributor list**, and a **read-only-for-health-data
API** — four write operations exist, all of them webhook-subscription management.
`[DOCUMENTED CONTRACT]`

Those are, item for item, the same rules AnxietyWatch2's own documentation states. `docs/specs/AMENDMENTS.md`
grants append-only storage where removal is a tombstone; the decoder contract rejects an out-of-range value
rather than clamping it and names the reason; and `AMENDMENTS.md:61` reads — "Never
use `locf()` or `interpolate()` on a physiological series. Use `time_bucket_gapfill` so gaps come back as
explicit NULLs" — and that instruction is Oura's own, in different words: its FAQ tells developers to check
for null values and to display "gaps in visualizations rather than connecting across missing data."
`[DOCUMENTED CONTRACT]`

**The honest framing of that convergence is not that V2 has features Oura lacks. It is: both systems have these
rules; V2 enforces them where Oura documents them.** V2's advantage is *enforcement and stated states* — an
append-only grant applied by the database, a decoder that cannot return a value it does not have, a
three-state rule that names not-measured as a first-class outcome. Oura's advantage is *public
inspectability*: its rules are written down where a third party can read them. Neither cancels the other, and
a report that claims either one does has failed at its job.

On the evidential side the picture inverts. The same vendor's public surfaces state that a `r²` of 0.996
measured on 49 healthy adults is "**99% Heart Rate Accuracy**"; place a heading "**Independently validated
sensors**" above a gallery whose lead-author study was written by the vendor's own Chief Scientist at the
time; state "over 30 biometrics" on one surface and "over 50 health metrics" on another; state a score range
**four different ways inside one document**; and render a **53%** independent agreement figure and a pending
class action alongside a **79%** vendor figure, in the vendor's own voice, on a page whose URL claims the
question is settled. `[PUBLIC CLAIM]`

**The finding that matters most for AnxietyWatch, though, is neither.** It is that **Oura's contract has no
alert, no escalation, no acknowledgement, no delivery receipt and no caregiver concept anywhere in 72 paths
and 75 schemas**, and the Terms state the services are "NOT INTENDED FOR USE IN EMERGENCY OR
LIFE-THREATENING SITUATIONS." `[DOCUMENTED CONTRACT]` AnxietyWatch2's multi-modal alerting and supervised
alarm layers (amendment 03, specified target) are therefore differentiated **by the absence of a competitor
on that axis** — which is a weaker form of differentiation than beating one, and the report should say so.

**The most important unknown.** Nothing about Oura's runtime behaviour was observed and **no request was
issued**, including to the sandbox. The contract can say that `LocalizedDateTime` and `UtcDateTime` differ; it
cannot say what arrives on the wire. Every contract finding in this report is a statement about a document.

---

## 2. Product and availability

| Item | Oura | Evidence |
|---|---|---|
| Vendor identity | **Ōura Health Oy** (CE legal manufacturer, iOS seller, non-US terms) · **Ouraring Inc.** (US contracting, marketing controller) · **Oura Inc.** (privacy policy) · **Oura Health Ltd** (Google Play developer) | `[PUBLIC CLAIM]` — four entities across five surfaces |
| Hardware | Oura Ring 5; Ring 4; Ring 4 Ceramic; charging cases | `[PUBLIC CLAIM]` |
| App | `com.ouraring.oura`, version **7.26.0**, released **2026-10-06T05:28:51Z**, 476,589,056 B, minOS 16.0, **4.8633★ / 308,643 ratings** | `[THIRD PARTY]` — iTunes Lookup, US storefront |
| App (second bundle) | `com.ouraring.ouraapp` — **resultCount 0** | `[THIRD PARTY]` |
| Android listing | Developer **Oura Health Ltd**; **4.7★ / 40.8K reviews**; **1M+ downloads**; updated 2026-09-18 | `[PUBLIC CLAIM]` |
| Membership | **$5.99/month** or **$69.99/year**; HSA/FSA eligible; **required for API access**; lapsed membership → `403` | `[PUBLIC CLAIM]` + `[DOCUMENTED CONTRACT]` |
| App download age | iOS build 7.26.0 published the same day as the research date | `[THIRD PARTY]` |
| Sensors, as the vendor describes them | Red + infrared LEDs (SpO₂); green + infrared (HR/HRV); NTC thermistor; 3-D accelerometer. **No gyroscope, no EDA.** | `[PUBLIC CLAIM]` |
| Regulated surface | Exactly one: the **Fertile Window** within Cycle Insights, CE-marked under MDR (EU) 2017/745; Terms add "**NOT INTENDED FOR USE AS CONTRACEPTION AND MUST NOT BE RELIED UPON TO PREVENT PREGNANCY**" | `[PUBLIC CLAIM]` |
| Clinical partnership | Counsel Health — licensed physicians, prescriptions, **43 US states**; the vendor states the experience "is not intended for emergency care" | `[PUBLIC CLAIM]` |
| Enterprise | Enterprise Platform, Enterprise API, adoption/attrition dashboards, CSV/JSON export | `[PUBLIC CLAIM]` |

**Availability is not the differentiator and the comparison should not pretend otherwise.** Oura is a mature,
widely distributed consumer product; AnxietyWatch v1 is a personal iOS application and AnxietyWatch2 is a
Raspberry-Pi-based multi-source collector specified in `openspec/`. They are not in the same distribution
category and no purchasable substitute exists for what V2 is being built to do.

**Two availability facts are user-facing and belong on the record.** `[PUBLIC CLAIM]` The API requires an
active membership, so the export and integration surface is **subscription-gated in a way the app itself is
not** — a lapsed payment removes the user's programmatic access to their own measurements. And the app was
published on the research date, which tells us the client is actively maintained while telling us nothing
about firm- or ring-side behaviour.

---

## 3. System boundary and architecture

The architecture below is reconstructed **from the published contract and the vendor's own processing-timeline
description only**. Everything in dashed boxes is `[UNKNOWN]` — unpublished and not inferable from the
contract, and the app and firmware are additionally covered by the Terms' reverse-engineering prohibition.

```text
        ANXIETYWATCH2 (V2, specified)                    OURA (documented contract only)
        ────────────────────────────                     ──────────────────────────────

  ┌──────────────────────┐                        ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
  │ AirSense 11 (BLE)    │  registry.toml:317     │   Oura Ring 5            │
  │ 17 provided types    │  17 types, conv.       │   sensors (vendor claim) │
  ├──────────────────────┤  leak ×60, TV ×1000    │   BLE: [UNKNOWN]         │
  │ xe125 / polar_h10    │                        │   firmware: [UNKNOWN]    │
  │  (registry sources)  │                        │   ring-side buffer:      │
  └──────────┬───────────┘                        │   [UNKNOWN]              │
             │ pure decoders, no I/O              └ ─ ─ ─ ─ ─ ─┬ ─ ─ ─ ─ ─ ─ ─ ┘
  ┌──────────▼───────────┐                                     │ BLE, unpublished
  │ vitals-decode        │  reject, never clamp                 ▼
  │ no_std, forbid(unsafe)│  Option/Result, no bool+out     ┌──────────────────────┐
  └──────────┬───────────┘                                │  Oura App (7.26.0)   │
             │ COPY binary                                │  background sync:    │
  ┌──────────▼───────────┐                                │  activity, stress,   │
  │ TEMP staging table   │  ON CONFLICT DO NOTHING        │  heart rate          │
  │  (private, per-spool)│                                │  sleep: app must be  │
  └──────────┬───────────┘                                │  opened  [PUBLIC]    │
             │                                            └ ─ ─ ─ ─ ─ ─┬ ─ ─ ─ ─ ┘
  ┌──────────▼───────────┐                                             │ HTTPS
  │ PostgreSQL 16 +      │  samples hypertable    ┌────────────────────▼─────────┐
  │ TimescaleDB          │  7-day compression     │  Oura Cloud                  │
  │ append-only +        │  2-year raw retention  │  Data Collection →           │
  │ tombstone, NEVER     │                        │  Processing → API            │
  │ UPDATE/DELETE,       │                        │  Availability  [DOCUMENTED]  │
  │ NEVER locf/interp    │                        └────────────────────┬─────────┘
  └──────────┬───────────┘                                             │
             │  time_bucket_gapfill                                       │
             │  → explicit NULLs                    ┌────────────────────▼─────────┐
  ┌──────────▼───────────┐   ┌──────────────┐       │  api.ouraring.com/v2         │
  │ alert path: local +  │   │ iOS client   │       │  72 paths · 75 schemas       │
  │ push + supervised    │   │ (v1 lineage) │       │  OAuth2 + BearerAuth         │
  │ alarm (SPECIFIED)    │   └──────────────┘       │  READ-ONLY for health data   │
  └──────────────────────┘                          │  4 writes: webhook mgmt only │
                                                    │  webhooks: no `heartrate`    │
                                                    │  documented lossy            │
                                                    └──────────────────────────────┘
```

**The two boundaries differ in kind.** V2's boundary is a **device-collection boundary** — the interesting
engineering is at the BLE layer, in the pure decoders, in the registry-driven unit conversion, and in the
spool that survives an interruption. Oura's boundary, as far as anything published shows, is an **API
boundary** — the interesting engineering is upstream of it, invisible, and in the contract, visible. V2
publishes its decode layer to a reader; Oura publishes its output contract to a reader. Neither publishes the
layer the other does.

**A structural consequence worth recording.** `[DOCUMENTED CONTRACT]` Oura's cloud performs the analysis, and
the contract's own timeline names the stages *Data Collection → Processing → API Availability*. There is no
documented local-processing path and no documented offline mode. V2's decoders are `no_std` with
`forbid(unsafe_code)` and no I/O by construction, and the conversion rule — "One canonical unit per type,
converted **once**, at the decoder" — is enforced by the crate boundary rather than by policy. That is a real
architectural difference, and it is a difference in *where the analysis runs*, not in whether it is done well.

---

## 4. Data path and source comparison

| Property | AnxietyWatch v1 | AnxietyWatch2 (specified) | Oura (documented contract) |
|---|---|---|---|
| Primary sources | HealthKit, CPAP import, manual entry, Polar H10 | `vitals/registry.toml` sources; AirSense 11 over BLE gives 17 types (`registry.toml:317`); decoders in `vitals-decode` | The ring, via Oura Cloud, via the published API |
| Raw samples | Not retained as a series | Retained in `samples` (append-only hypertable) | **No raw stream is exposed.** Every published field is a discrete sample, a vendor aggregate or a vendor classification `[DOCUMENTED CONTRACT]` |
| Time encoding | Store UTC epoch seconds | UTC epoch seconds and nothing else | **Four bare `type: string` date/time schemas** — `ISODate`, `LocalDateTime`, `LocalizedDateTime`, `UtcDateTime` — with no `format`, no pattern, no example, no description |
| Units | One canonical unit per type | One canonical unit per type; conversion declared in the registry (`leak = 60.0`, `tidal_volume = 1000.0`) | Six fields with **no unit stated anywhere** — `vo2_max` is documented only as "VO2 max value.", `PublicSpo2AggregatedValues.average` only as "Average of spo2." |
| Live stream | Real-time HealthKit path (v1 design doc) | Specified; `expected_cadence_s = 1` for `resmed_air11` | `heartrate` is pull-only: `/v2/usercollection/heartrate`, **no webhook, no sandbox twin** |
| Overnight | v1 CNS pipeline reads overnight data | Specified overnight collection | Sleep data recorded overnight, **not synced until the user opens the app** `[PUBLIC CLAIM]` |
| Device count | Several sources | Several sources, registry-governed | **One device.** No device-disagreement concept exists in the contract |
| Import | OSCAR by-session report (v1) | `cpap-ezshare-ingest` with the OSCAR boundary invariant (no GPL code copied, `design.md:39`) | FHIR/EHR import on the clinical side — **the record leaves HIPAA once in Oura's possession**; Oura is not a Business Associate |
| Export | v1 reports | COMP encoder + provenance | CSV export "through the Membership Hub"; API returns JSON per the contract |
| Provenance | Limited | Versioned, clock-corrected (amendment 02) | `PublicSleepAnalysisReason`; `sleep_algorithm_version` ∈ `{v1, v2}`; `low_battery_alert` |

**Reading the table.** The comparison does not favour one column uniformly. Oura is **ahead** on two rows and
it is worth being precise about why:

- **Record mutation provenance.** `sleep_analysis_reason` distinguishes `foreground_sleep_analysis`,
  `bedtime_edit`, `background_sleep_analysis`, and `background_created_foreground_updated`. That last value
  records a record that was *created in the background and then revised in the foreground* — the origin and
  the revision are both preserved. The claimed-features file records that V2 has the versioned
  clock-corrected metadata from amendment 02 but **no identified counterpart** to this field (gap G12).
  `[DOCUMENTED CONTRACT]` vs `[UNKNOWN]` for V2 until the tree is read and the counterpart cited or the gap
  confirmed.
- **Which algorithm produced this record.** `sleep_algorithm_version` is a per-record field, so an integrator
  can separate v1-scored from v2-scored nights in the data, and Oura's own narrative about "the 2023
  algorithm" cannot silently overwrite history. This is a genuinely good design and the comparison should say
  so without hedging.

V2 is **ahead** where the difference is structural rather than documentary:

- **Units.** Oura's contract leaves six fields unit-less, including a physiological measurement whose meaning
  is an open question in the same document (`temperature_deviation` vs `temperature_trend_deviation` —
  near-identical descriptions, no stated difference). V2's registry makes a unit-less field
  unrepresentable, and the conversion factors live beside the source that needs them.
- **Time.** Four indistinguishable string types on the field that says *when a measurement happened* is a
  reproducibility problem. V2 stores UTC epoch seconds and nothing else, and local time exists only in
  presentation. Oura's own `PublicSleepTimeWindow.day_tz` — "Timezone offset in second from GMT of the day",
  the only UTC offset anywhere in the contract — shows the vendor knows the distinction matters and chose to
  carry it on exactly one model.
- **Device disagreement.** Not a fair row: Oura has one device, so it has nothing to disagree with itself
  about. Recording it as a V2 advantage would be dishonest; recording it as an Oura weakness would be too.

---

## 5. Claim-versus-documented-contract reconciliation

Fifty checkable claims are reconciled row by row in `oura_IMPLEMENTATION_INSPECTION.md` §7:
**24 match · 9 documented-only · 6 partial · 7 do not match**, with four items outside those classes named
individually (item 38, a superlative with nothing to test; item 40, published accuracy rates for predictive
surfaces that are absent from every surface; item 41, the class action and the 53% figure; item 50, a
hypothesis that was refuted). The seven that do not match are items **10, 11, 12, 14, 24, 25, 36**.

This section carries the subset that changes how the product should be compared.

| # | Public claim | Documented contract / counter-surface | Disposition |
|---|---|---|---|
| 1 | "Check for null values" and "displaying gaps in visualizations rather than connecting across missing data" (developer FAQ) | Matches V2's own rule (`AMENDMENTS.md:61`), in different words | **Match.** Two independently designed systems reached the same instruction. Report as convergence, not as a V2 advantage. |
| 2 | "Each endpoint includes example requests and responses" (`info.description`) | **0 `example` / `examples` keywords**; 75 `x-codeSamples` (requests only, and one of them contains a concrete subscription UUID) | **Do not match.** Half-satisfied at best; the claim is about both halves and only the request half exists. Documentation defect. |
| 3 | "Scores: Range from 0-100" (Data Model & Concepts) | Same document states `[1, 100]` on the activity score and most contributors, `[0, 100]` on resilience contributors, and leaves sleep and readiness scores unstated | **Do not match — intra-document.** Whether `0` is valid is contradictory within the vendor's own text. Record; do not resolve. |
| 4 | "IDs: Unique identifiers for specific data objects follow a consistent format" | Almost no `format` declarations exist: only `DailyResilienceModel.day` / `TagModel.day` / `EnhancedTagModel.start_day` (`format=date`), `WebhookSubscriptionModel.id` (`uuid4`) and `expiration_time` (`date-time`) | **Do not match.** An undocumented claim, not a wrong one — the sentence asserts consistency the contract does not state. |
| 5 | "All time-based data uses ISO 8601 format" | `PublicHeartRateRow.timestamp_unix` is an **integer in milliseconds**; the field coexists with an ISO `timestamp` on the same row | **Exactly one self-violation**, and it is documented rather than hidden. Name it as one field, not a pattern. |
| 6 | "5000 requests per 5-minute period" (FAQ) | `info.description` describes a **two-layer** model whose ceilings it does not publish, discoverable from a `429` response header | **Conflict.** The flat figure and the layered model are both the vendor's. The conservative statement is the layered one; the ceiling is `[UNKNOWN]`. |
| 7 | Oura's public API reports sleep stages "in five-minute blocks" (vendor blog, 2026-08-28) | The contract publishes `sleep_phase_30_sec`, `sleep_phase_5_min`, and a deprecated app-aligned `app_sleep_phase_5_min` | **Conflict between the vendor's narrative and its own contract.** Record; note that `sleep_algorithm_version` resolves the *related* timeline question even though the prose does not. |
| 8 | "Independently validated sensors" (science page, gallery heading) | The gallery includes a study whose lead author the vendor elsewhere identifies as **"Hannu Kinnunen, lead study author and Oura's Chief Scientist at the time"** | **Evidence-classification defect.** Record both surfaces verbatim. The defect is the classification, not the study. |
| 9 | "99% Heart Rate Accuracy" (science page) | The supporting 2020 article reports **r² = 0.996** on **49 healthy adults**, resting | **Category mismatch.** The basis is disclosed in one place and the headline uses a different noun in another. Record verbatim; the word "accuracy" has no single technical meaning — which is precisely why the *basis* is the finding. |
| 10 | "79% Sleep Tracking Accuracy — Compared to clinical polysomnography at 83%" | The 2022 post announcing the algorithm renders an illustrative hypnogram captioned "accuracy = 84%"; the 2026 post reports a **2025 independent study at 53% agreement** for four-stage classification on Ring Gen3 and a rebuttal letter in *Sleep Advances* | **Reported by the vendor against itself.** All three figures come from vendor-authored surfaces; the study and letter were not fetched. Record as the vendor's account of a disputed number, unresolved. |
| 11 | "86% of Oura Members see their health improve" | 2026 survey of **3,501 members, self-reported, 30 days, no control group** | **Vendor-published, method disclosed.** Not independent; the disclosure is the mitigating fact and should be named. |
| 12 | "Track over 50 health metrics" (home) vs "over 30 biometrics" (Play) | Both vendor surfaces, no reconciliation | **Intra-vendor count discrepancy.** For the vendor's total addressable surface (79%), the claimed-features file records the contradiction and leaves it open rather than averaging. |
| 13 | Sensors listed on `/security`/science page | Red + infrared (SpO₂), green + infrared (HR/HRV), NTC thermistor, 3-D accelerometer; **no gyroscope, no EDA** | **Match with a boundary.** The positive finding is a sensor claim that does *not* overreach. |
| 14 | "the most scientifically validated smart ring" | "130+ peer-reviewed publications", "14 active collaborations", "12+ Years", four named Medical Advisors | **Superlative (item 38, outside the checkable classes).** Nothing to test. Record, do not rate. |
| 15 | Health Radar: Blood Pressure Signals, Nighttime Breathing, Symptom Radar ("before symptoms of a respiratory illness like a cold or the flu are even felt"), Counsel Health | **No sensitivity, specificity or false-alarm rate is published on any of these pages.** Footnoted as general wellness only | **Gap in the vendor's material**, not evidence the features fail. Compare with `empatica`, the only candidate with published FAR figures — and those are adverse. |
| 16 | "we do not share or sell your personal data. That includes third-party AI models (LLMs)" (Trust Center) | The privacy policy names LLM providers and permits training Oura's own models while prohibiting third-party LLM training | **Cross-surface conflict. The policy governs.** Record both; do not imply bad faith. |
| 17 | "deleted data is fully removed" (Trust Center) | The privacy policy ties retention to account lifetime and account state | **Cross-surface conflict. The policy governs.** Note that the *contract-level* deletion mechanism is the API tombstone, which is a different fact from the marketing sentence. |
| 18 | "We also meet HIPAA requirements where we handle protected health information for a healthcare partner." | Privacy policy: FHIR/EHR records leave HIPAA once in Oura's possession; Oura is not a Business Associate | **Contractual boundary, correctly stated by the vendor on the surface where it matters.** Record as a positive. |

**What this table is not.** It is not a scoreboard. Nine of these rows are documentation defects; two are
disputes the vendor raises against itself; three are honest boundaries; one is a real intra-document math
conflict. **A documentation defect is not a product defect**, and the report would be misleading if a reader
came away thinking Oura's ring works less well because its specification has a copy-paste error in
`PublicRestModePeriod.start_time` (described as "…rest mode ended.", identical to `end_time`).

---

## 6. Feature matrix

Legend: **✔** present · **◐** partial or conditional · **—** absent · **?** unknown · *(S)* specified target
in V2, not shipped.

| Capability | v1 | V2 (current / *specified*) | Oura | Evidence for the Oura column |
|---|---|---|---|---|
| BLE device collection | ✔ HealthKit/H10 | ✔ decoders + registry | ? BLE layer unpublished | `[UNKNOWN]` |
| Raw sample retention | — | ✔ append-only hypertable | — no raw stream exposed | `[DOCUMENTED CONTRACT]` |
| Live/real-time stream | ◐ | ✔ *(S)* | ◐ `heartrate` poll-only, no webhook, no sandbox twin | `[DOCUMENTED CONTRACT]` |
| Overnight collection | ✔ | ✔ | ✔ recorded, **synced on app open** | `[PUBLIC CLAIM]` |
| Explicit missingness | ◐ | ✔ three-state rule + `time_bucket_gapfill` | ◐ nullable score/contributors, non-wear code, measured `non_wear_time`; **causes not distinguished** | `[DOCUMENTED CONTRACT]` |
| Rejection with a named reason | — | ✔ reject, name, quarantine | ◐ `rest` = rejected *detection*, not rejected measurement | `[DOCUMENTED CONTRACT]` |
| Tombstone deletion | — | ✔ (amendments) | ✔ `PublicSleepType.deleted` | `[DOCUMENTED CONTRACT]` |
| Per-record algorithm version | — | ? (gap G12) | ✔ `sleep_algorithm_version` ∈ {v1, v2} | `[DOCUMENTED CONTRACT]` |
| Mutation provenance | — | ◐ amendment 02 metadata | ✔ `sleep_analysis_reason` (4 values) | `[DOCUMENTED CONTRACT]` |
| Unit discipline | ◐ | ✔ registry, convert once | ◐ six unit-less fields | `[DOCUMENTED CONTRACT]` |
| UTC-only storage | ✔ | ✔ | — four bare-string time types; one offset field | `[DOCUMENTED CONTRACT]` |
| Device disagreement | — | ◐ multi-source | — one device | structural |
| Personal baselines | ✔ | ✔ | ✔ contributors published, **weights not** | `[DOCUMENTED CONTRACT]` |
| Correlations / derived analysis | ✔ | ✔ | ✔ scores, contributors, resilience decomposition | `[PUBLIC CLAIM]` |
| AI analysis | ✔ broad | ✔ | ✔ Oura Advisor, Health Radar, Symptom Radar | `[PUBLIC CLAIM]` |
| Explainability of composites | ◐ | ✔ | ◐ resilience fully decomposed; weights unpublished; score range contradictory | `[DOCUMENTED CONTRACT]` |
| Local alarm | ✔ | ✔ *(S)* supervised | — **no alert/escalation/ack concept anywhere** | `[DOCUMENTED CONTRACT]` |
| Hardware dead-man switch | — | ✔ *(S)* | — | `[DOCUMENTED CONTRACT]` |
| Offline operation | ◐ | ✔ degraded mode *(S)* | ? app requires sync; no offline mode documented | `[UNKNOWN]` |
| Open API | — | ✔ Rust API (amendment 07) | ✔ read-only for health data, membership-gated | `[DOCUMENTED CONTRACT]` |
| Export with provenance | ◐ | ✔ | ◐ CSV + JSON; **four indistinguishable timestamp types** | `[DOCUMENTED CONTRACT]` |
| Multi-source CPAP integration | ✔ | ✔ *(S)* AirSense 11, 17 types | — | `[DOCUMENTED CONTRACT]` |
| Weather / environmental context | — | ✔ *(S)* | — | — |
| EMA / symptom logging | ✔ | ✔ | ◐ tags, `PublicMomentMood` ∈ {bad, worse, same, good, great} | `[DOCUMENTED CONTRACT]` |
| Subscription required | — | — | ✔ membership gates API and features | `[PUBLIC CLAIM]` |

**How to read the matrix.** Three rows carry most of the weight and are easy to misread:

- **Explicit missingness.** Oura is not missing this. It is missing *one third* of it: Oura can represent
  "not measured" structurally, and cannot say *why*. V2 names the states. That is a real difference and a
  narrower one than the ✔/◐ suggests.
- **Rejection with a named reason.** This is the row where the two systems are doing genuinely different
  things. V2 rejects a **measurement**; Oura records a rejected **detection**. A rejected detection is a
  valid sleep record the user disowned; a quarantined measurement is a number the decoder would not vouch
  for. Both are good practice. They are not the same practice.
- **Alarm rows.** Oura is — not ◐. State the inventory: 72 paths, 75 schemas, and no alert, escalation,
  acknowledgement, delivery-receipt or caregiver concept in any of them. This is a *complete absence*, which
  is a stronger statement than a missing search hit, and it is the one place where V2's specified alarm layer
  has no competitor to be measured against.

---

## 7. Derived values versus stored signals

This is the section where the two systems are most nearly symmetric, and the symmetry is instructive.

**Oura stores or returns:** scores (`daily_sleep`, `daily_readiness`, `daily_activity`, `daily_stress`,
`daily_resilience`, `daily_cardiovascular_age`, `daily_spo2`), contributor decompositions (sleep 7,
activity 6, readiness 9, resilience 3), a `class_5_min` activity classification, phase strings, aggregates
(`PublicSpo2AggregatedValues.average`), and deviations (`temperature_deviation`,
`temperature_trend_deviation`). `[DOCUMENTED CONTRACT]`

**V2 stores:** measurements, in canonical units, and derivatives are computed downstream. The registry
declares 17 provided types for `resmed_air11` and **no AHI type** — the therapy index is deliberately not
among them, so the summary the clinician expects is a derived value over stored signals rather than a stored
manufacturer number.

**The shared principle.** Both systems separate a measured signal from a vendor-produced index, and both
make the index decomposable rather than opaque. Oura publishes contributor lists — nullable, with no
`required` on any of the three sibling models — and V2's registry-driven design makes the transform from raw
to canonical arithmetic and inspectable.

**Three differences, all documentary:**

1. **Weights.** Oura decomposes the composite into named contributors and stops there. A contributing
   component's *weight* is not published anywhere in the contract, so a reader can see that `sleep_recovery`
   contributed without learning how much it moved the score. V2's derivatives are arithmetic over the stored
   series.
2. **The one required composite.** `PublicDailySleep`, `PublicActivityContributors` and
   `PublicReadinessContributors` all make every contributor nullable with no `required` list.
   `ResilienceContributors` — the newest — requires all three (`sleep_recovery`, `daytime_recovery`,
   `stress`) and does not allow null on any of them. `[DOCUMENTED CONTRACT]` **That inconsistency is a
   finding, not a nitpick**: the newest composite is the one that cannot represent "this contributor was not
   measured", which is exactly the state the older models can express. Record it as an observation about the
   contract; do not infer the reason.
3. **`temperature_deviation` vs `temperature_trend_deviation`.** Two number fields, both documented in
   degrees Celsius, with near-identical descriptions and **no stated difference** — and no absolute
   temperature exposed anywhere. An undocumented value domain on a physiological measure. `[DOCUMENTED
   CONTRACT]`; `[UNKNOWN]` whether the distinction is meaningful, and nothing published resolves it.

**On "AI analysis".** Both products offer it and the comparison should not treat it as a differentiator on
either side. The relevant difference is evidential: V2's `openspec/` fixes what the analysis is specified to
consume and produce; Oura's Advisor is an assistant with no published context contract, and the vendor's own
Trust Center sentence about not sharing data with third-party LLMs is contradicted by its own privacy policy,
which names them.

---

## 8. Real-time, overnight and failure-mode comparison

| Scenario | v1 | V2 (specified) | Oura (documented) |
|---|---|---|---|
| Sensor stream drops overnight | v1 CNS pipeline reads what HealthKit has | Decoders return `Option`; absence contributes nothing; gap renders as explicit NULL | `sleep_phase_*` strings have **no non-wear code**; `non_wear_time` is a measured duration; the score is nullable. **Ring-side buffering: `[UNKNOWN]`** |
| Phone unavailable | v1 sync retries | Spool + degraded mode survive interruption (`pi-prototype-storage-degraded-mode`) | **Sleep data is not synced until the app is opened**; ring-side behaviour `[UNKNOWN]` |
| Internet/cloud unavailable | v1 degrades | Degraded mode specified | Webhook delivery is **documented as lossy** — "you might miss some events"; "implement a reconciliation process" — and re-sync behaviour is not documented |
| Reading stale/malformed/out of range/absent | v1 limited | **Reject, name the reason, quarantine** — never clamp | `rest` = rejected detection; `'0'` = non-wear; nullable score; **no stated rule for a malformed or contradictory reading** |
| Clinician needs an export with source/quality context | v1 reports | COMP + provenance (amendment 02) | CSV/JSON; **no quality flags**, six unit-less fields, four indistinguishable timestamp types |
| Alert path failure | v1 local notification only | Multi-modal alerting + supervised alarm + dead-man switch *(S)* | **No alert path exists to fail** |

**Two statements in this table must not be collapsed into one.**

- **The lossiness finding cuts both ways, and both halves belong on the record.** The vendor writes, in its own
  contract documentation: "you might miss some events" and "implement a reconciliation process that can fetch
  data for periods when your webhook might have been unavailable." A vendor that documents its own
  lossiness is doing the right thing; a system whose *only* documented push mechanism is lossy is still
  lossy. Both sentences are true and the report should carry both.
- **The retry path documents its counts and not its timing.** `[DOCUMENTED CONTRACT]` Ten retries are
  documented for each of three failure classes (4xx, 5xx, timeout), and a delivery window of "about an hour" is
  stated — so the retry path is better specified than a first glance suggests. What is **not** documented is
  the backoff schedule between those retries; the signature scheme has **no documented replay window**; and the
  vendor's own reference implementation reads `x-oura-timestamp`, feeds it into the HMAC, and **never compares
  it against the current time**. Against V2's invariant — "Attempts precede work" (`cpap-ezshare-ingest`
  `design.md:26`) — the difference is that V2 makes the attempt the unit of record.

**The overnight row is where a consumer product and a monitoring product genuinely part.** Oura's timeline
states that sleep requires the user to open the app. That is a reasonable design for a consumer ring and a
disqualifying one for a monitor that must survive an unattended night — and the vendor does not present it as
a monitoring product, so the observation is a boundary fact, not a defect.

---

## 9. Provenance, missingness and reproducibility

**Provenance — Oura is the strongest surface in the package, and it is worth stating exactly what it
achieves.** `[DOCUMENTED CONTRACT]`

- `sleep_analysis_reason` ∈ {`foreground_sleep_analysis`, `bedtime_edit`, `background_sleep_analysis`,
  `background_created_foreground_updated`} — records *why* a sleep record exists or was revised, including
  the case where a background record was later revised in the foreground.
- `sleep_algorithm_version` ∈ {`v1`, `v2`} — a per-record field that resolves which algorithm produced a
  record at the data layer, where the vendor's prose cannot.
- `PublicSleepType.deleted` — deletion as a tombstone type value, not row removal.
- `low_battery_alert` on the sleep record.
- `PublicSleepType.rest` — a false-positive detection retained rather than discarded, with the reason named.

Those five mechanisms are, in substance, the rules V2 states. The comparison's honest conclusion is that
**Oura reached the same design conclusions independently and published them; V2 reached them and made the
database enforce them.** The one mechanism the claimed-features file records as having **no identified V2
counterpart** is `sleep_analysis_reason` (gap G12) — and that record is `[UNKNOWN]` for V2 until the tree is
read and the counterpart cited or the gap confirmed.

**Missingness — Oura's model is structural and incomplete in a specific, nameable way.**

| Representation | Oura mechanism | What V2 has |
|---|---|---|
| Not worn | `class_5_min` `'0'` = non wear (the only digit alphabet that can express it); `non_wear_time` measured | Three-state rule; not-measured is a first-class outcome |
| Not measured / no score | Nullable `score` on `PublicDailySleep`, `PublicDailyActivity`; all-nullable contributor lists | Explicit NULLs via `time_bucket_gapfill` |
| Rejected | `rest` (a rejected detection, not a measurement) | Quarantine with a named reason |
| **Why missing** | **Not expressible** | Named state |

**The three disagreeing digit alphabets are the sharpest missingness finding.** `[DOCUMENTED CONTRACT]`
`class_5_min` uses `'0'`–`'5'` where `'0'` means non-wear; the sleep-phase strings use `'1'`–`'4'`; and
`movement_30_sec` uses `'1'`–`'4'` **with no `'0'` code at all**, so movement cannot express non-wear while
activity can. The same character means different things in fields that appear beside each other, and only
one of the three can represent absence. That is a representational hazard for any integrator, and it is
recorded as a finding with the alphabets quoted verbatim.

**Reproducibility — two contract properties bite.** The four bare `type: string` date/time schemas mean an
integrator cannot know from the contract whether a returned timestamp is localised, UTC, or naive; and the
`heartrate` 200 response is declared `anyOf[TimeSeriesResponse_PublicHeartRateRow_ |
TimeSeriesResponseDict]`, where the second branch has `additionalProperties: true` — **two possible shapes
with no documented criterion for choosing between them.** `[DOCUMENTED CONTRACT]` Neither is a defect that
can be resolved without a call, and no call was authorized.

**A negative result that belongs here.** The suspected dangling-reference hypothesis was checked and
**refuted**: 274 `$ref`s, 0 broken; four security schemes declared and all four referenced. The refuted
hypothesis is recorded as refuted and **not** as a finding. A verification that produces a positive is as
much a result as one that produces a defect, and a package that only reports defects is not measuring.

---

## 10. Local/cloud/privacy and operational cost

| Dimension | v1 | V2 (specified) | Oura |
|---|---|---|---|
| Where analysis runs | On device | On the Pi / in Rust services; local-first | **In Oura Cloud**, per the contract's own timeline |
| Account required | No | No (self-hosted) | **Yes**, and an active membership for API access |
| Offline mode | Partial | Specified degraded mode | **None documented** |
| Storage location | Local | Local PostgreSQL 16 + TimescaleDB | Vendor cloud |
| Encryption | Platform | Amendment 02 | **Documented:** "AES-128, with TLS 1.2+ while in transit, and with AES-256 at rest" |
| Compliance artifacts | — | — | **SOC 2** (no Type I/II stated); **HITRUST** (no level or scope stated); NIST CSF; bug bounty; **no transparency report yet** (vendor's own statement) |
| Health-data boundary | — | — | Policy: FHIR/EHR records leave HIPAA once in Oura's possession; **Oura is not a Business Associate** |
| Data sale | — | — | "we do not share or sell your personal data" — with a policy that names advertising-adjacent processors; **the policy governs** |
| Cost to the user | Free (own hardware) | Own hardware | Ring hardware + **$5.99/mo or $69.99/yr**; API access is membership-gated |
| Cost to run | Owner's time | Pi + storage + energy | None to the user beyond the subscription |

**Reading this table without inflation.** Oura's compliance posture is stated at a level of specificity that
its own marketing does not exceed — SOC 2 with no type, HITRUST with no level — and the report should record
that the *absence* of a Type or level in the Trust Center is a documentation gap, not evidence that the
attestation is weak. `[PUBLIC CLAIM]`

**Three cost facts are genuinely user-facing and should be in the report.** The premium hardware has a
recurring subscription attached; **the API — which is the surface that makes the ring's data portable — is
gated on that subscription while the ring itself is not**; and a lapsed membership returns `403` on API
access. A user who has bought the hardware can lose programmatic access to their own measurements. Contrast
with V2's self-hosted model, where portability is a property of owning the database, and with Oura's own
CSV export, which remains available "through the Membership Hub" — i.e. also membership-scoped.

**Where V2 is genuinely better and it is not the pitch.** V2's privacy story is structural (local storage,
no account, no cloud) rather than contractual. That is a real difference. It is also a difference most users
will never be able to evaluate, and it does not make V2's *validation* better, its *sensors* better, or its
*analysis* better.

---

## 11. Alerting and the alarm path

**This section has one finding and it is a complete inventory, not an absence of evidence.**

`[DOCUMENTED CONTRACT]` In 72 paths and 75 schemas:

- no alert, alarm or notification concept;
- no escalation or severity concept;
- no acknowledgement or delivery-receipt concept;
- no caregiver, contact or on-call concept;
- no watchdog, heartbeat or dead-man concept;
- four write operations in the entire API, all webhook-subscription management.

And in the Terms: the services are "**NOT INTENDED FOR USE IN EMERGENCY OR LIFE-THREATENING SITUATIONS**",
with an accompanying population-average caveat.

**The correct comparison is therefore not Oura-versus-V2 on alerting — it is V2's specified alarm path
against nothing.** That is a weaker form of differentiation than beating a competitor, and the report should
say so plainly rather than scoring it as a V2 win. Two consequences follow:

1. **V2's alarm layer has no external reference point in this package.** Of the eight candidates compared so
   far, only `empatica` publishes an alert path at all, and its published false-alarm figures are adverse
   (paediatric FAR 0.34/24 h — roughly one false alert every three days). The claimed-features deliverable
   records that **AnxietyWatch2 has not stated an acceptable false-alarm rate for any alert type**. When a
   system is compared against a product that does not compete on the axis, that gap stays invisible, and it
   should not.
2. **What Oura does instead of alerting is a design choice, not an omission.** It surfaces upstream signal
   ("Blood Pressure Signals", "Symptom Radar") for the user to interpret, and it disclaims emergency use. It
   is a wellness product and it says so. The comparison should record the boundary without treating it as a
   shortfall against a specification Oura never claimed.

---

## 12. User-facing consequences

| For a user who wants… | Oura | AnxietyWatch (v1 / V2) |
|---|---|---|
| A sleep and readiness score with an explanation of contributors | ✔ good, with unexplained weights | v1 partial; V2 specified |
| To see the raw signal behind the score | **Not available** — no raw stream is exposed | V2 stores and exposes the series |
| An alarm when something is wrong overnight | **Not offered**, and expressly outside the terms | v1 local; V2 specified supervised path |
| Their own data on their own disk | CSV export; API (membership-gated); **no self-hosting** | Native property |
| To know *why* a night is missing | **Not expressible** in the data model | V2's three-state rule names it |
| To know which algorithm scored a night | ✔ `sleep_algorithm_version` per record | V2: `[UNKNOWN]`, gap G12 |
| To integrate with a CPAP workflow | — | V2 specified, AirSense 11, 17 types |
| To run it without a subscription | — | No subscription exists |
| An accurate published accuracy figure | Present, with the basis disclosed in a different artifact and disputed by an independent study | Not applicable |

**Three consequences that are easy to get wrong.**

- **"No raw stream" is not a deficiency for Oura's user.** A consumer ring that returns a score is the
  product; expecting raw photoplethysmography would be expecting a different product. It is a decisive
  difference for a *researcher* or for a user comparing a CPAP leak curve against a sleep stage, and that is
  the honest way to frame it.
- **The membership gate is the most concrete user-facing asymmetry in the whole comparison.** Oura's API is
  the only way to get the ring's data out programmatically, and it is subscription-gated, while the ring is
  sold as a purchase. V2 has no equivalent failure mode.
- **A user comparing "99% Heart Rate Accuracy" with the 2025 independent 53% sleep-staging figure is comparing
  two different quantities.** The comparison must resist the temptation to present that juxtaposition as a
  contradiction. The vendor's own 2026 article holds both, and holds them in the correct frames — the finding
  is that the vendor's *marketing* surface does not carry the qualification its *blog* does.

---

## 13. Limitations, unverified claims and evidence gaps

**Limits of this comparison.**

1. **No runtime observation of any kind.** No endpoint was called, including the documented sandbox. Zero
   rows in the inspection file's §6. Every `[DOCUMENTED CONTRACT]` finding is a statement about a document.
2. **No binary or source inspection, and the reason is contractual.** The Terms prohibit reverse engineering,
   decompilation and circumventing subscription restrictions, quoted verbatim in
   `oura_IMPLEMENTATION_INSPECTION.md` §2.1. This is an authorization decision, not an availability accident.
3. **Three surfaces are `[UNKNOWN]` because retrieval failed, not because they are absent.** `/legal`,
   `/science` and `/for-organizations` returned JavaScript error shells (`<html id="__next_error__">`, ~1
   character of text). A retrieval failure is not an absence, and the one genuine 404 in the artifact set is
   a different document. `[UNKNOWN]`
4. **The 53% study and the rebuttal letter were not fetched.** The package holds the vendor's characterisation
   of both, in a post written by the vendor's own lead sleep-algorithm scientist.
5. **The class action is recorded as a vendor-reported fact about litigation**, not as a finding of liability
   or of product defect.
6. **Four legal entities across five surfaces** are mapped and the mapping is left unexplained — because the
   artifacts do not explain it, and inferring a corporate intent from a page footer would be exactly the kind
   of inference this package forbids.
7. **Draft-level completeness.** The verdict tally in the inspection file's §7 was wrong when first written
   and was machine-counted from the table and corrected. It is recorded here because an unreproducible tally
   is worse than no tally, and because the correction is part of the evidence trail.
8. **The first pass was materially more negative than the second.** A field-class sweep reversed five
   verdicts, softened a sixth, and produced nine new findings, five of them positive for the vendor. That is
   recorded against the **research process**, not against Oura, in the claimed-features file §14.8 and as a
   standing method rule in `../RESEARCH_METHOD.md`.
9. **Nine quotations were not verbatim** when first written — including a Terms clause cited as an
   authorization basis, whose paraphrase had invented verbs. All nine were found by machine comparison against
   the fetched bytes (with the specification parsed as JSON string leaves) and corrected; the sweep is now a
   required pre-commit step. §14.9 of the claimed-features file carries the before/after table. **This is the
   failure mode the package exists to avoid, and two earlier passes missed it because both checked values and
   neither checked words.**
10. **An unverified hash is worse than no hash.** The artifact table carries 23 computed SHA-256 values, all
    verified against the pinned files. The rule is recorded in the file itself, having been learned the hard
    way in `empatica`.

**Open evidence gaps carried forward** (inspection file §8, I1–I11, and claimed-features file §14.6,
G1–G13). The ones that would most change a conclusion:

| Gap | Would resolve |
|---|---|
| I2 / Q1 — null semantics per field | Whether Oura can distinguish not-worn from sensor-failure from rejection, at field level |
| I1 / Q2 — four bare-string time schemas | The single largest reproducibility difference; needs a live sample, not authorized |
| I4 / Q3 — webhook backoff window and replay window | Whether the push path is usable for a monitor |
| I5 / Q4 — the two-layer rate-limit ceilings | Whether polling `heartrate` is viable at all |
| I7 — BLE layer, firmware, ring-side buffering | Everything about the layer V2 actually engineers at |
| I10 — first-pass non-exhaustiveness | Recorded as a process defect, not a vendor finding |
| G12 — V2 counterpart to `sleep_analysis_reason` | Read the V2 provenance code; cite the counterpart or confirm the gap |

**What cannot be concluded.** That Oura's sensors are or are not accurate (published figures exist, with
disclosed and disputed bases, and nothing was measured). That its webhooks work as documented. That its app
behaves as the contract implies. That the Enterprise Platform does what its page claims. That V2 is superior
on any axis except stated missingness states, unit and time discipline, offline operation and the alert path —
and on the alert path, superior to nothing.

---

## 14. Balanced conclusion

**Closest technical overlap.** Between the contract and the tree, the overlap is **record-level design
discipline**: append-only-with-tombstone, rejection retained with a reason, per-record algorithm version,
explicit nullability, and the instruction not to interpolate across a gap. Two systems arrived at the same
rules from different starting points, and neither appears to have copied the other.

**Where Oura is genuinely ahead of AnxietyWatch2.** (1) `sleep_analysis_reason` — mutation provenance at the
field level, with no identified V2 counterpart (gap G12, `[UNKNOWN]` for V2 pending a tree read). (2)
`sleep_algorithm_version` — per-record algorithm provenance. (3) Published inspectability: the contract is
public, checkable and internally resolvable, and V2's equivalent surface (`openspec/` plus the Rust API)
serves a different audience. (4) Maturity: a shipped product with a shipped app and a shipped API, against a
specified target.

**Where AnxietyWatch2 is genuinely ahead.** (1) **Stated missingness states** — Oura can represent absence and
cannot say why. (2) **Unit and time discipline** — six unit-less fields and four indistinguishable timestamp
types against a registry that makes both unrepresentable. (3) **Offline and local-first operation**, with no
account and no subscription. (4) **The alert path** — against nothing, which is why this is ranked last and
stated as such. (5) **Multi-source collection**, where V2 is the only candidate in the package that ingests a
CPAP device's 17 native types.

**Direct competition: none.** Oura is a consumer wellness ring with a cloud analysis pipeline; AnxietyWatch2
is a self-hosted multi-source collector with a specified alarm path. They overlap in "sleep and physiology
tracking" and diverge on nearly every architectural axis. **Partial competition** on the analysis layer.
**Complementarity** is real: a user could run both, and Oura's API is the kind of source V2's registry is
built to accept.

**The transferable lesson, which is the point of the comparison.** Oura's contract is documentation of a
system that has not been inspected; V2's tree is an implementation whose public description is a set of
amendment documents. **The strongest thing in this package's Oura dossier is not a V2 advantage — it is the
demonstration that a vendor can publish enough of its data model for an outside party to check it, and that
the check finds real defects and real quality in the same document.** AnxietyWatch2 has the same opportunity.
Whether its `openspec/` and its registry are as checkable, at the same level of field detail, is a question
this comparison cannot answer — because nothing in this package has yet read the V2 tree the way it read
`openapi-1.41.json`.

**Best-fit users.** Oura: a consumer who wants a low-friction daily score and will pay a subscription for it.
AnxietyWatch2: an owner who wants their own measurements on their own disk, with the source and the quality
context attached, and who needs the alarm path. **The products are not substitutes and the comparison should
not imply that a user must choose.**

---

## Related files

- `oura_CLAIMED_FEATURES_AND_PURPOSE.md` — public-claims research, claim ledger O-1…O-95, 23-row SHA-256
  artifact table, evidence gaps G1–G13, quote-verification pass (§14.9).
- `oura_IMPLEMENTATION_INSPECTION.md` — Phase 2A; authorization basis, contract findings, reconciliation
  points R1–R13, 50-row claim reconciliation (§7), gaps I1–I11.
- `PROMPT_2.md` — the candidate-specific brief this report satisfies.
- `../PROCESS_STATE.md` — candidate status and authorization decisions.
- `../RESEARCH_METHOD.md` — inspection modes, evidence classes, standing sub-checks.
- `../PROMPT1.md` — the package brief.
- `../CANDIDATE_SHORTLIST.md` — how this candidate was selected (rank 8, score 13).
