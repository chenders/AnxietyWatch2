# Oura — claimed features and purpose

Candidate 8 of 10 (`CANDIDATE_SHORTLIST.md` rank 8, score 13).
Research date **2026-10-06**. Prepared for `PROMPT1.md` Phase 1, and as the evidence base for
`PROMPT_2.md`.

**Evidence classes used throughout.** `[PUBLIC CLAIM]` the vendor's own published statement ·
`[DOCUMENTED CONTRACT]` the vendor's own published interface specification, read statically ·
`[THIRD PARTY]` an independent, non-vendor source · `[INFERENCE]` my reasoning · `[UNKNOWN]` no evidence
found.

**Nothing in this file is a runtime observation.** No Oura app was installed or run. No Oura account was
created. **No Oura endpoint was called** — the sandbox, the production API and the OAuth endpoints were
all left alone. No ring was worn. Every statement below is either the vendor's own published words, the
vendor's own published specification read statically, an independent third-party statement, or an
explicitly-labelled inference.

**Artifact hashes.** Every fetched artifact is SHA-256 hashed in §14.5 so the pages and the specification
this file quotes can be identified exactly. Web pages and API contracts change; a claim without a hash is
not reproducible.

**Why this candidate is on the shortlist.** Oura is the only candidate that publishes a **complete,
machine-readable output contract for a consumer wearable** — a 454,152-byte OpenAPI 3.1 document
(`openapi-1.41.json`) — *and* a **contested validation record**. It is therefore simultaneously the best
available comparator for AnxietyWatch2's **data model, missingness handling, unit discipline and
provenance rules** and the clearest available example of how a wearable vendor's **evidence claims are
constructed**. Both halves of that are in scope for the comparison, and this file keeps them separate.

---

## 1. Executive summary

**Oura is the strongest technical comparator in the package and the weakest evidential one.** Its
published interface is detailed enough to read real design decisions at field level — including several
that AnxietyWatch2's rules were written to enforce, and several that violate Oura's own stated contract.
At the same time, the numbers on its science page are rendered differently from the quantities they
measure, and the studies behind two of them were authored by Oura staff.

Seven findings, in order of consequence for the comparison:

1. **The published contract states V2's missingness principle and then hands the problem to the
   integrator.** The API's own "Best Practices" section says to "**Check for null values in responses**"
   and to "**Consider displaying gaps in visualizations rather than connecting across missing data**".
   The time-series container (`PublicSample`) carries an explicit `interval` in seconds alongside an
   `items` array whose element type is the nullable-float array `Array_Union_float__NoneType__Bits64_`
   (title `ArrayNullableFloatBits64`) — so a hole is representable **in band**
   and is not interpolated. That is the same position AnxietyWatch2 takes structurally
   (`AMENDMENTS.md`:61, "Never use `locf()` or `interpolate()`… Use `time_bucket_gapfill`"). It is
   recorded as a **positive**, and it is the only such statement in the package so far.
2. **Four named timestamp types exist, all defined as a bare `string`, and the API's date filters are
   interpreted in the end user's local timezone.** `ISODate`, `LocalDateTime`, `LocalizedDateTime` and
   `UtcDateTime` are declared as `{"type": "string"}` with no `format`, no pattern and no example —
   they are the complete set of schemas in the document with that exact definition; the semantic
   difference is carried entirely by the name, and the difference between `LocalDateTime` and
   `LocalizedDateTime` is never stated. `LocalizedDateTime` is used in ten models, `UtcDateTime` in three
   (the two time-series row models and ring configuration). Separately, the FAQ states that "**the dates
   are interpreted in the user's local timezone**". A consumer cannot determine from the contract whether
   a given timestamp is UTC or local, cannot machine-check it, and cannot know the boundary its own query
   was evaluated against. This is the sharpest form of the standard's undocumented-domain finding, and it
   lands on the one field AnxietyWatch2 stores as "**UTC epoch seconds and nothing else**". One field
   excepts it: `PublicSleepTimeWindow.day_tz` carries a UTC offset in seconds — the only timezone the API
   reports, in one model, for one purpose.
3. **The contract states one global timestamp rule and its own schemas break it, in one place.** "Common
   Data Structures" says "**Timestamps**: All time-based data uses ISO 8601 format". `PublicHeartRateRow`
   carries `timestamp_unix`, described as "Timestamp of the discrete sample as **unix time in
   milliseconds**". The rate-limit reset header is unix **seconds**; the webhook payload carries an ISO
   `event_time`; and the webhook subscription's `expiration_time` is an ISO `date-time` string. So the
   document uses ISO 8601, unix seconds (one header) and unix milliseconds (one field), and **exactly one
   field violates the document's own rule** — the one on the real-time heart-rate row. Both violations
   were originally mis-stated in an earlier draft as "two", counting a field that is in fact ISO 8601;
   the corrected inventory is in §8.4.
4. **Three of the four headline accuracy figures are `r²` or an agreement statistic rendered as a
   percentage, and the two that matter most come from vendor-authored studies presented under
   "Independently validated sensors".** The science page publishes "**99% Heart Rate Accuracy** / r²
   compared to ECG" and "**98% Heart Rate Variability Accuracy**" — an r² value headlined as an accuracy
   percentage. The linked source is a 2020 study of 49 adults whose "**Hannu Kinnunen, lead study author
   and Oura's Chief Scientist at the time**" was the vendor's own chief scientist. The sleep figure, "**79% Sleep Tracking Accuracy**", derives from an
   algorithm article whose "**Lead author, Marco Altini, PhD**" is credited to "the Oura Research Team".
   **In both cases the vendor's own page supplies the disclosure and the gallery heading contradicts
   it.** Recorded with both surfaces quoted; the egregiousness is in the label, not in the number.
5. **The strongest independent check on the sleep-staging claim is a study the vendor is actively
   disputing — and the dispute is being conducted in the peer-reviewed literature.** An independent 2025
   study in *Scientific Reports* reports "**53% agreement for four-stage classification with the Oura Ring
   Gen3**"; Oura responds in a 2026 article by the scientist who built the algorithm, citing a
   2025 peer-reviewed letter to the editor in *Sleep Advances*, and states that "**Oura-funded and
   independent**" studies report "**70 to 79% range in healthy adults**". **A proposed class action is
   documented on the vendor's own blog.** This is the first candidate in the package where the vendor's
   headline number has a serious, independently published counter-number, and where the vendor's rebuttal
   is authored by the person whose work is being challenged. Recorded unresolved, with the vendor's
   methodological objections quoted and its *stated willingness to distinguish funded from independent
   research* noted as a positive.
6. **The vendor's own blog contradicts the vendor's published API on how sleep stages are reported.**
   The rebuttal article states that "**Oura's public API reports sleep stages in five-minute blocks**",
   while the specification it publishes exposes `sleep_phase_30_sec` as well as `sleep_phase_5_min` — plus
   a third, separately named `app_sleep_phase_5_min` described as "aligned with what is shown in the app".
   Three names for sleep phase, one of them explicitly needed because the API value and the app value
   differ. Together with `average_heart_rate`'s own note that "**this is the average calculated by ecore
   (based on 30-second samples) which is different from what is shown in the app**", this is a documented
   **pattern**: Oura ships parallel implementations of the same named metric and the contract discloses
   which one a field carries. Nothing in AnxietyWatch2 is permitted to do that, and the reason is the same
   in both systems — a derived value must not be confusable with a measured one.
7. **Oura independently publishes three of the four rules AnxietyWatch2 treats as non-negotiable — and
   publishes two of them as *data*.** (§8.12, added on the second verification pass.) A sleep period the
   user deletes **remains addressable as a tombstone** (`PublicSleepType` = `"deleted"`, "deleted sleep by
   user") rather than disappearing; a machine detection the user rejects is **kept and labelled with the
   rejection reason** (`"rest"` = "Falsely detected sleep / nap, **rejected in confirm prompt by
   user**"); and the record carries the **algorithm version that produced it**
   (`sleep_algorithm_version` ∈ `{v1, v2}`) plus **why it was created or updated**
   (`sleep_analysis_reason` ∈ {foreground pass, bedtime edit, background pass, background-then-foreground}).
   This is the fourth independent arrival at V2's patterns in this package (after `oscar`'s session
   provenance, `nightscout`'s treatments grain and `empatica`'s regulatory record) and it is the closest
   match yet: **a consumer can compute a false-positive rate from a rejected detection because the rejected
   detection was not discarded.** The honest framing is not *V2 has these properties and Oura does not*;
   it is **both have them, and V2 enforces them where Oura documents them** — Oura's tombstone is a
   value in a record that could have been omitted, and V2's is a schema-level rule with no alternative.

**Also recorded:** a **free-to-the-developer but membership-gated-for-the-user** API (`403` means "the
user's Oura subscription has expired"; renewal "restore[s] API access"), against a home page promising
"you can export all of your Oura data in CSV format through the Membership Hub" **without** a membership —
so manual export and programmatic access have deliberately different commercial conditions; a
**webhook path the vendor itself documents as lossy** ("if your server is down for an extended period, you
might miss some events… implement a reconciliation process"), with ten retries and **no published backoff
schedule**; a rate limit published as a flat "**5000 requests per 5-minute period**" in the FAQ against a
**two-layer** model in the overview whose ceilings are **not published** and are only discoverable from a
`429` response header; a **sandbox** dataset that is explicitly labelled fake ("The data is not real…
generated by Oura and is not based on any real user data") — the correct handling of the problem SleepHQ's
unlabelled synthetic examples had; a **privacy policy that discloses LLM-provider integration and Oura's
own model training** beside a Trust Center claim that "we do not share or sell your personal data. That
includes third-party AI models (LLMs)", left unresolved; and **four different Oura legal entities** named
across five surfaces.

---

## 2. Positioning and identity

| | |
|---|---|
| Product | Oura Ring (smart ring) + Oura App + Oura Membership + Oura Advisor + Health Radar + Oura Enterprise Platform |
| Current devices | **Oura Ring 5**; Oura Ring 4; Oura Ring 4 Ceramic; charging cases |
| Platforms | iOS (`com.ouraring.oura`, trackId 1043837948) and Android (`com.ouraring.oura`) |
| Company (site) | Ōura Health Oy, Elektroniikkatie 10, 90590 Oulu, Finland |
| Home | `https://ouraring.com/` |
| Business model | Hardware purchase + **mandatory-feeling membership** for insights; B2B/enterprise platform |
| Category, vendor's own | Health & Fitness (both stores) |
| Regulated surface | **One**: the Fertile Window feature is CE-marked under MDR (EU) 2017/745 (see §11.6) |

The vendor's stated purpose is unambiguous on the consumer side and is repeated almost verbatim on both
store listings:

> "The Oura Ring is **not a medical device** and is not intended to diagnose, treat, cure, **monitor**, or
> prevent medical conditions or illnesses. The Oura Ring is only designed for **general fitness and
> wellness purposes**." `[PUBLIC CLAIM]` — Google Play listing, `com.ouraring.oura`

That sentence is the single most important one in this file, because the **same listing** carries a
feature heading in capitals, **ILLNESS DETECTION**, above the sentence "Oura Ring **monitors** shifts in
your body temperature and heart rate so you can tell when you may be getting sick." Both are on one page.
The record is the coexistence, not a judgement about either.

`[PUBLIC CLAIM]` — Play listing, same page, both quoted verbatim.

---

## 3. Corporate, legal and regulatory identity

**Five surfaces name four different Oura legal entities.** This is a provenance fact that matters for a
data-driven product: which entity is the controller, the seller, the counterparty and the app publisher
differs by surface, and a user reconciling them has to read four documents.

| Surface | Entity named | Role as stated on that surface |
|---|---|---|
| `ouraring.com` home / about | **Ōura Health Oy** | Company |
| Apple App Store (iOS) | **Oura Health Oy** | Seller, `com.ouraring.oura` |
| Google Play (Android) | **Oura Health Ltd** | Developer of `com.ouraring.oura` |
| Privacy Policy | **Oura Inc.**, **Oura Health Oy**, **Ouraring Inc.** | Oura Health Oy controls all purposes **except marketing** (Elektroniikkatie 10, 90590 Oulu, Finland); Ouraring Inc. (415 Kearny Street, San Francisco, CA 94108) controls **marketing**; Oura Inc. is named a controller collectively |
| Terms (contracting party) | **Ouraring Inc.** (US) / **Oura Health Oy** (non-US) | Contracting counterparty; governing law California (US) / Finnish law (non-US) |
| Terms (regulated feature) | **Oura Health Oy** | Legal manufacturer of the CE-marked Fertile Window feature |
| `organizations.ouraring.com` | **Oura** (unspecified entity) | Oura for Organizations / Oura Enterprise Platform |

`[PUBLIC CLAIM]` for every row. `[INFERENCE]` that "Oura Health Ltd" and "Oura Health Oy" are distinct
registered bodies — the Play developer name is what the store displays and no incorporation number is
published on the listing. Recorded as an observed surface difference, **not** as a finding about corporate
structure.

**The Apple listing and the Google listing disagree on the developer entity for the same app.** Both
publish the app for the same product. Neither store surface cross-references the other.

---

## 4. Availability, platforms and hardware

### 4.1 Store metadata

| | iOS | Android |
|---|---|---|
| Identifier | `com.ouraring.oura` | `com.ouraring.oura` |
| Store id | trackId **1043837948** | — |
| Version | **7.26.0** | not stated in the listing text |
| Released / updated | currentVersionReleaseDate **2026-10-06T05:28:51Z** (the research date) | "Updated on **Sep 18, 2026**" |
| First release | 2016-01-14 | — |
| Size | **476,589,056 bytes (476 MB)** | — |
| Rating | **4.8633★** over **308,643** ratings | **4.7★** over **40.8K** reviews |
| Installs | — | **1M+** |
| Min OS | 16.0 | — |
| Price | Free (membership sold in-app/online) | Free |
| Age rating | 4+ | Everyone |
| Seller / developer | **Oura Health Oy** | **Oura Health Ltd** |

`[THIRD PARTY]` — Apple iTunes Lookup API (`bundleId=com.ouraring.oura`) and the Google Play listing.

`com.ouraring.ouraapp` returned **0 results** — there is no second Oura bundle under that identifier.

**A second app exists and is not treated as the product here.** Oura also ships the **Oura Research App**
(a separate application for research participants; the privacy policy describes its blinding behaviour,
§10.4). It was not enumerated and is not compared.

### 4.2 An unresolved ratings-versus-reviews ambiguity — recorded as unresolved, not as a discrepancy

The membership page publishes "**4.9 App Store Rating based on 280k reviews**"; the live store metadata
returns **4.8633** over **308,643** ratings. Ratings and written reviews are different counts, the page's
figure is rounded and older, and the API does not expose the split. **Both numbers are recorded; the
difference is not called a discrepancy.** `[INFERENCE]` that the two counts measure different things;
`[UNKNOWN]` which population the 280k refers to.

### 4.3 Hardware signals

`[PUBLIC CLAIM]` — science-and-research page and Play listing. The ring's sensor set as published:

- **Red and infrared LEDs** — SpO₂ (and, per the science page, additional signals)
- **Green and infrared LEDs** — heart rate and HRV
- **NTC temperature sensor**
- **3D accelerometer**
- **Not present as published: no gyroscope, no EDA/electrodermal sensor.**

The finger-placement argument is a vendor claim on the B2B site: "The finger's arteries provide a
**50-100x stronger pulse signal**" than wrist wearables. `[PUBLIC CLAIM]`, unsourced.

**Sensor-count claim varies by surface.** The home page says "Track **over 50 health metrics**"; the Play
listing says "the most accurate reading for **over 30 biometrics**". Both are vague, both are the vendor's
own, and they are **quoted rather than reconciled**. `[PUBLIC CLAIM]` — both surfaces.

### 4.4 Data capture requires the user to open the app

This is an operating constraint, published by the vendor, that conditions every downstream claim:

> "**Sleep Data**: Only syncs when the user opens the Oura app and actively syncs their ring"
> "**Daily Activity**: Updates periodically in the background"
> `[DOCUMENTED CONTRACT]` — `openapi-1.41.json`, tag "Data Model & Concepts"

> "**Data Delay**: Different data types sync at different times - sleep data requires users to open the
> Oura app, while daily activity and stress may sync in the background"
> `[DOCUMENTED CONTRACT]` — same document, `info.description`

So the flagship derived quantities — sleep score, readiness, sleep time, sleep staging — are
**user-initiated**, while activity, stress and heart rate are background. `[INFERENCE]` the practical
consequence: overnight monitoring is retrospective and depends on a morning action; nothing Oura publishes
describes a real-time overnight alert, and §11.7 records that the product expressly disclaims one.

---

## 5. Pricing, plans and commercial model

### 5.1 Membership

`[PUBLIC CLAIM]` — membership page and home page:

- **$5.99 USD/month or $69.99 USD/annually**, before tax, US members (regional pricing applies)
- **"FSA or HSA eligible"**
- Free first month
- Without a membership: "your Oura Ring and Oura App will still function, but the insights, personal
  health data, and benefits you receive will be much more limited"
  and benefits … much more limited"

### 5.2 The API is free to the developer and gated for the user — two different statements, both published

| Statement | Surface |
|---|---|
| "Is the Oura API free to use? **Yes, the Oura API is free to use for both personal and commercial applications.** However, API applications accessing more than **10** users require approval from Oura." | FAQ tag, specification |
| "API Applications are limited to **10** users before requiring approval from Oura. There is no limit once an application is approved." | `info.description` |
| `403 Forbidden` — "The requested resource requires additional permissions **or the user's Oura subscription has expired**." | HTTP response table |
| "**Expired Oura Membership** … The user needs to renew their Oura membership to **restore API access**." | Authentication Troubleshooting tag |
| "Even **without a membership**, you can export all of your Oura data in CSV format through the Membership Hub" | home page |

`[DOCUMENTED CONTRACT]` for the first four; `[PUBLIC CLAIM]` for the last.

**The two are consistent and precisely stated, which is worth recording as such.** The developer pays
nothing; the **member** must hold an active subscription or the API returns `403`. And the fallback path
without a membership is a **manual CSV export from the Membership Hub** — not the API. A consumer reading
only the home page would conclude that data access survives a lapsed membership; a consumer reading only
the API contract would conclude it does not. **Both surfaces are the vendor's.** No price for API use is
documented anywhere in the specification — `[INFERENCE]` the API has no separate price.

### 5.3 Enterprise

`[PUBLIC CLAIM]` — `organizations.ouraring.com`. **Oura Enterprise Platform**: dashboards; adoption,
attrition and engagement tracking; de-identification options and role-based permissions; "Enterprise API
to send data to external systems"; "**Flexible data export of raw data to CSV or JSON file**"; and
"Complies with stringent data protection laws like GDPR and **processes PHI in accordance with HIPAA**".
No price is published. Audiences listed: healthcare providers, health plans, research, government &
defense, employers, corporate gifting, wellness & coaching, athlete performance, partners & resellers.

### 5.4 Scale claims, all vendor-published

- "**5+ Million Oura Members**" `[PUBLIC CLAIM]` — home page
- "**86% of Oura Members see their health improve**" — footnote: "Based on a **2026 survey with 3,501 Oura
  members** after using Oura Ring for 30 days" `[PUBLIC CLAIM]`, **vendor survey, self-reported
  perception, no control group**. Recorded as the vendor's own survey and nothing more.
- "**12+ Years**", "**130+ peer-reviewed publications**", "**14 active collaborations**" — science page
  (see §11.5 for what the publications count actually counts)

---

## 6. Measured signals as published

What the API exposes, read from the specification's schema set. **Signal list, not feature list** — this
is what a consumer of Oura data can actually obtain:

| Signal class | Fields / models | Notes |
|---|---|---|
| Heart rate | `PublicHeartRateRow.bpm`, `source` ∈ {awake, workout, rest, sleep, live, session} | Time-series, one row per discrete sample; `timestamp` + `timestamp_unix` (**ms**) |
| Heart rate during sleep | `PublicModifiedSleepModel.heart_rate` (`PublicSample`) | Container form, interval + nullable array |
| HRV during sleep | `PublicModifiedSleepModel.hrv` (`PublicSample`), `average_hrv` (integer, no unit) | `average_hrv` has **no unit and no method** stated |
| Sleep phase | `sleep_phase_5_min`, `sleep_phase_30_sec`, `app_sleep_phase_5_min` | Three fields; see §8.3 |
| Movement | `movement_30_sec` ('1' no motion, '2' restless, '3' tossing and turning, '4' active) | Digit-encoded string; **no code for non-wear** |
| Activity class | `class_5_min` ('0' non wear, '1' rest … '5' high activity) | Digit-encoded string; **non-wear is a first-class code** |
| Breathing rate | `average_breath` ("as breaths/minute") | Unit stated |
| SpO₂ | `PublicDailySpO2.spo2_percentage.average`, `breathing_disturbance_index` | See §7.2 for both |
| Body temperature | `temperature_deviation`, `temperature_trend_deviation` (°C) and `body_temperature` (score contribution) | **Exposed as a deviation, not an absolute**; two near-identical field names (§7.2) |
| Ring battery | `PublicRingBatteryLevelRow` (UtcDateTime model); `low_battery_alert` on the sleep record | No webhook type (§8.2) |
| Workout / session / tags | `PublicWorkout`, `PublicSession`, `TagModel`, `EnhancedTagModel` | `PublicSession.mood` enumerated |
| Cardiovascular | `PublicDailyCardiovascularAge` (vascular age; `pulse_wave_velocity`), `PublicVO2Max` | See §7.2 |
| Stress / resilience | `PublicDailyStress` (`stress_high`, `recovery_high`), `DailyResilienceModel` | See §7.2 |
| **Record provenance** | `sleep_algorithm_version` ∈ {v1, v2}, `sleep_analysis_reason`, `PublicSleepType` (incl. `'deleted'`), `ring_id` (deprecated, null) | **See §8.12 — the strongest structural finding in this file** |
| Score contributors | `PublicSleepContributors` (7), `PublicActivityContributors` (6), `PublicReadinessContributors` (9), `ResilienceContributors` (3) | Decomposition published; **weights not** (§7.1) |
| Meals | Webhook data type `meal` only | **No REST read path** (§8.2) |

**No raw/ungraded sensor stream is exposed.** `[INFERENCE]`: every field above is either a discrete
sample (heart rate, battery) or an aggregate or classification produced by Oura's own processing. The
"Data Processing Timeline" section states the pipeline explicitly: "**Processing**: Algorithms analyze raw
data to generate insights and scores" then "**API Availability**: **Processed** data becomes available
through API endpoints". `[DOCUMENTED CONTRACT]`. There is no field in the specification through which a
consumer can obtain pre-algorithm sensor data.

---

## 7. Derived measures and biomarkers

### 7.1 Scores

- Daily **Sleep**, **Activity** and **Readiness** scores; the contract's global rule states "**Scores**: Range
  from 0-100, with higher values indicating better performance" `[DOCUMENTED CONTRACT]`
- **The score range is stated three different ways in one document.** The global rule says **0-100**;
  `PublicDailyActivity.score` says "Activity score in range **[1, 100]**"; `ReadinessContributors` says
  "Range: **[0, 100]**"; `PublicReadinessContributors` / `PublicSleepContributors` / sleep `efficiency`
  say "in range **[1, 100]**"; and `PublicDailySleep.score` and `PublicDailyReadiness.score` state
  **no range at all** ("Daily sleep score." / "Daily readiness score."). **Whether 0 is a valid score is
  therefore contradictory within the vendor's own contract.** Recorded as an intra-document conflict —
  V2's counterpart is that "The unit **and valid range** come from `vitals/registry.toml`. Never hardcode
  either", so the range is data here and prose there.
- `sleep_score_delta` and `readiness_score_delta` on a sleep period — the **contribution of that period to
  the score**, an unusual field to publish: it exposes a decomposition of a composite rather than only the
  composite. Recorded as a positive structural detail.
- **The contributor decomposition is published in full, and the weights are not.** `PublicSleepContributors`
  (7 fields), `PublicActivityContributors` (6), `PublicReadinessContributors` (9) and
  `ResilienceContributors` (3) are all returned, each a named contribution in a stated range. **No
  weighting, formula or aggregation rule for any composite score is published anywhere in the contract.**
  Recorded as a split verdict: a consumer can see **what feeds** each score and cannot reproduce it.
- `PublicSleepTimeStatus` is a **fully enumerated** status: `not_enough_nights`, `not_enough_recent_nights`,
  `bad_sleep_quality`, `only_recommended_found`, `optimal_found`. `[INFERENCE]` this is the clearest
  example of the vendor's "not measured" vocabulary surviving into the public contract — three of the five
  values describe a state in which no recommendation was produced.
- **A headline score can be absent while its record exists.** `PublicDailySleep.required` is
  `['id', 'contributors', 'day', 'timestamp']` — **`score` is not required and is typed nullable**. The
  same is true of `PublicDailyReadiness.score` and `PublicDailyActivity.score`. So the contract can say
  *today's sleep record exists and today's sleep score is null*. Recorded as a **positive**: the composite
  is not forced to a number when it was not computed.
- **Every contributor field is nullable except in the resilience model.** `PublicSleepContributors`,
  `PublicActivityContributors` and `PublicReadinessContributors` declare **no `required` list** and every
  field is nullable — so a contributor that was not computed is absent rather than zero. `ResilienceContributors`
  is the exception: all three fields are `required` and non-nullable (`number`). Recorded as **an
  inconsistency between sibling models**, and the nullable majority is recorded as a positive.

### 7.2 Derived values whose method is disclosed — the pattern that matters most

These are the fields where Oura's own descriptions say something a consumer needs and most vendors omit:

| Field | Vendor's own description, verbatim `[DOCUMENTED CONTRACT]` |
|---|---|
| `average_heart_rate` | "Average heart rate during sleep as beats/minute. NOTE: this is the average calculated by **ecore (based on 30-second samples)** which is **different from what is shown in the app**. The app shows the average of **aggregated 5-minute heart rate samples**." |
| `lowest_heart_rate` | "Lowest heart rate during sleep. NOTE: this is the value calculated by **ecore** (based on 30-second samples) which is **different from what is shown in the app**." |
| `breathing_disturbance_index` | "Breathing Disturbance Index (BDI) calculated using **detected SpO2 drops from timeseries**. Values should be in range **[0, 100]**" |
| `spo2_percentage.average` | "Average of spo2." — **no unit, no range, no method** |
| `pulse_wave_velocity` | "Pulse wave velocity (m/s), **derived from vascular age, with possible offset added**." |
| `stress_high` | "Time spent in a high stress zone (**top quartile of data**) in seconds." |
| `recovery_high` | "Time spent in a high recovery zone (**bottom quartile data**) in seconds." |
| `vo2_max` | "VO2 max value." — **no unit**; typed `integer` |
| `average_hrv` | "Average heart rate variability during sleep." — **no unit, no method** |
| `temperature_deviation` | "Temperature deviation in degrees Celsius." |
| `temperature_trend_deviation` | "**Temperature trend deviation** in degrees Celsius." — **the difference from `temperature_deviation` is not stated** |
| `body_temperature` | "Contribution of body temperature in range [1, 100]." — a **score input, not a temperature** |
| `efficiency` | "Sleep efficiency rating in range [1, 100]." |
| `latency` | "Sleep latency in seconds. This is the time it took for the user to fall asleep after going to bed." |

**Two classes are mixed together here and must be separated.**

*Disclosed-method fields — recorded as positives.* `average_heart_rate` and `lowest_heart_rate` are the
only fields in the package where a vendor's published contract **tells the consumer that the API value and
the app value for the same named quantity differ**, and names the window each uses. `stress_high` /
`recovery_high` disclose their **quartile basis**. `breathing_disturbance_index` discloses its derivation
and its range. `pulse_wave_velocity` discloses that an **offset may have been added** — which is an
admission that the published number is not the raw computed quantity.

*Undocumented-domain fields — recorded as findings.* `spo2_percentage.average`, `vo2_max`, `average_hrv`
and `PersonalInfoResponse.weight`/`height` publish **no unit at all**. For a value typed `number` or
`integer` with no unit, a consumer cannot render it correctly or compare it to anything. This is the
standing undocumented-value-domain sub-check, and Oura produces **six instances** of it.

**A seventh instance is a near-duplicate pair.** `temperature_deviation` ("Temperature deviation in
degrees Celsius.") and `temperature_trend_deviation` ("Temperature trend deviation in degrees Celsius.")
are two `number` fields, in the **same model** (`PublicDailyReadiness` and `PublicReadiness`), whose
descriptions differ by the word "trend" and whose semantics are **never stated**. `[INFERENCE]` one is
today's deviation from baseline and the other is the deviation's own direction or smoothed trend — **the
contract does not say**, and a consumer choosing between them has two plausible readings and no criterion.
This is the same shape as Oura's `average_heart_rate` / "what the app shows" divergence, in a worse form:
there, the vendor names the difference; here it does not.

**Body temperature is exposed as a deviation and never as an absolute.** `[INFERENCE]` from the complete
schema inventory: the only temperature values in the contract are `temperature_deviation`,
`temperature_trend_deviation` and the `body_temperature` **score contribution** (`[1, 100]`). There is no
field carrying an absolute body temperature. Recorded because the science page's illness-detection copy
("shifts in your body temperature", §2) is delivered to a consumer of this API **only as a deviation**, so
a third party cannot reconstruct a temperature or compare across users.

### 7.3 `pulse_wave_velocity`: a derived value with an undisclosed offset

"**derived from vascular age, with possible offset added**" `[DOCUMENTED CONTRACT]`. The field is named for
a physical measurement (m/s) and is documented as being **derived from a different quantity** (vascular
age) with a **possibly-added offset of unpublished magnitude**. `[INFERENCE]` a consumer that reads this
field as a measured pulse-wave velocity is reading a value that has been transformed twice, and the second
transformation is not quantified. The vendor **discloses the transformation's existence and not its
size** — which is materially better than disclosing nothing and still insufficient to reproduce the
number.

### 7.4 Metabolic and cardiovascular surfaces

- The Trust Center's health-metrics list includes "**Metabolic Health: Glucose Levels and Meals**"
  `[PUBLIC CLAIM]`, and the specification's webhook data-type enumeration includes `meal`
  `[DOCUMENTED CONTRACT]` — but **no REST endpoint returns meal or glucose data** (§8.2). `[INFERENCE]`
  the surface is push-only.
- `PublicDailyCardiovascularAge` provides **Cardiovascular Age** and `pulse_wave_velocity`; the OAuth scope
  list includes `heart_health` for "cardiovascular age / VO2 max".
- **Health Radar** (§11.7) is the app-side carrier of these signals and is not exposed through the API as
  a distinct model.

---

## 8. The published API — the comparison core

All of §8 is `[DOCUMENTED CONTRACT]`, read statically from `openapi-1.41.json`
(SHA-256 `a144a343…b557`, 454,152 bytes, `openapi 3.1.0`, `info.version 2.0`, server
`https://api.ouraring.com`). **No request was made to any Oura endpoint.** The document was fetched from
the vendor's public static path that its own documentation page points at
(`/v2/static/json/openapi`, referenced by `<redoc spec-url="/v2/static/json/openapi">`).

### 8.1 Structure

- **274 `$ref` values, every one resolvable; zero broken references; zero non-schema references.**
  *Checked, not assumed.* The document is internally referentially consistent.
- **75 schemas.** 72 paths in total: **19 `/v2/usercollection/*` data endpoints** (all `GET`), **18
  `/v2/sandbox/*` mirrors**, and **three webhook subscription paths carrying five operations**. Plus
  OAuth and the document's informational tags.
- **The API is read-only with respect to health data.** *Checked:* every operation across the 19
  `/v2/usercollection/*` endpoints is a `GET`. The **only** write operations in the entire document are
  four webhook-management calls — `POST /v2/webhook/subscription`,
  `PUT /v2/webhook/subscription/{id}`, `DELETE /v2/webhook/subscription/{id}` and
  `PUT /v2/webhook/subscription/renew/{id}`. Recorded as a clear **boundary** and as a positive: there is
  no published path by which a third-party application can write to a user's health record.
- **Security schemes:** `BearerAuth` (http bearer), `OAuth2` (authorizationCode,
  `https://cloud.ouraring.com/oauth/authorize`, token `https://api.ouraring.com/oauth/token`),
  `ClientIdAuth` and `ClientSecretAuth` (`x-client-id` / `x-client-secret` headers, used on the webhook
  subscription routes). All four are declared in `components.securitySchemes` **and** referenced by
  operations — no dangling references. *Checked; an earlier expectation of dangling `ClientIdAuth`
  references was refuted and is **not** recorded as a finding.*
- **OAuth scopes** include `daily`, `heartrate`, `personal`, `workout`, `session`, `tag`, `email`, and
  `heart_health`.

### 8.2 Endpoint, webhook and sandbox coverage disagree — three different sets

| Data type | REST endpoint | Webhook data type | Sandbox route |
|---|---|---|---|
| sleep, daily_sleep, daily_readiness, daily_activity, daily_spo2, sleep_time, rest_mode_period, ring_configuration, daily_stress, daily_cardiovascular_age, daily_resilience, **vO2_max**, workout, session, tag, enhanced_tag | ✅ | ✅ | ✅ |
| `heartrate` | ✅ | ❌ **no webhook type** | ✅ |
| `personal_info` | ✅ | ❌ **no webhook type** | ❌ **no sandbox route** |
| `ring_battery_level` | ✅ | ❌ **no webhook type** | ✅ |
| **`meal`** | ❌ **no endpoint** | ✅ **in the enum** | ❌ |

`[DOCUMENTED CONTRACT]`, verified by comparing the path list, the `ExtApiV2DataType` enum (17 values) and
the `/v2/sandbox/*` path list.

Three findings from this table:

1. **Three data types are pull-only.** `heartrate`, `personal_info` and `ring_battery_level` have REST
   endpoints but **no webhook data type**. `heartrate` is the field most likely to be wanted in near
   real-time, and the contract's own "preferred way to consume Oura data" cannot carry it. `[INFERENCE]`
   a consumer wanting current heart rate must poll, and polling is what the FAQ's rate-limit guidance
   discourages.
2. **One webhook type has no read path.** `meal` is a valid `data_type` for a subscription and appears in
   `ExtApiV2DataType`, but **no `/v2/usercollection/meal` path exists**. A subscriber can be notified that
   a meal record changed and has no documented way to fetch it. Recorded as observed; `[UNKNOWN]` whether
   the notification is intended to be consumed without a fetch.
3. **`personal_info` is the only data type with no sandbox twin**, and the reason is not stated. `[INFERENCE]`
   a fabricated identity record would be meaningless to test against; recorded as an inference, not a
   finding.
4. **Subscriptions are per (event\_type, data\_type) combination, so an integration scaling to a full user
   set scales in subscriptions as well as users.** The webhook documentation states: "You need to create
   separate subscriptions for each combination of: **event_type** … **data_type** …". With three
   `WebhookOperation` values (`create`, `update`, `delete`) and 17 `ExtApiV2DataType` values, a consumer
   wanting everything needs **up to 51 subscriptions per user**. Both enums are published; the multiplication
   is not stated. Recorded as an **operational-cost** fact, and as a contrast with V2's single authorized
   collector per source — Oura's fan-out is per combination **and** per end user.

**A case-sensitivity trap.** The REST path is `/v2/usercollection/**vO2_max**`; the webhook enumeration
value is `**vo2_max**`. A consumer building a webhook subscription or comparing types across the two lists
has to know which list it is in. Recorded as a directly observable naming inconsistency, not an error
claim — the two are different namespaces.

### 8.3 Parallel implementations of the same named quantity

This is the single most consequential technical pattern in the contract, and it appears in four places:

| Concept | Field 1 | Field 2 | Field 3 | Vendor's own statement of the difference |
|---|---|---|---|---|
| Sleep phase (5-min) | `sleep_phase_5_min` | `app_sleep_phase_5_min` | — | `app_sleep_phase_5_min` is "aligned with what is shown in the app"; the primary field is separately documented and `app_sleep_phase_5_min` carries a note that it will be "removed in the future after a transition period" |
| Sleep phase (resolution) | `sleep_phase_5_min` | `sleep_phase_30_sec` | — | Contradicted by the vendor's own blog — §8.6 |
| Average heart rate | `average_heart_rate` (ecore, 30-second samples) | the value shown in the app (aggregated 5-minute samples) | — | Stated in the field description |
| Lowest heart rate | `lowest_heart_rate` (ecore, 30-second samples) | the app's minimum of aggregated 5-minute samples | — | Stated in the field description |
| Sleep phase (legacy) | `app_sleep_phase_5_min` | — | — | "removed in the future after a transition period" |
| Body temperature deviation | `temperature_deviation` | `temperature_trend_deviation` | — | **Nothing — the difference is not stated** (§7.2) |

`[DOCUMENTED CONTRACT]` — every field description quoted above is read from the schema set.

**Why this is the comparison's central technical finding.** AnxietyWatch2's rules forbid this shape:
"One canonical unit per type, converted **once**, at the decoder. Never at a later layer" (`AGENTS.md`),
and "The unit and valid range come from `vitals/registry.toml`. Never hardcode either." The reason is not
tidiness — it is that when two implementations of the same named quantity coexist, **a downstream consumer
cannot tell which one it holds**, and a trend line can be assembled from two different algorithms without
anything flagging it. Oura's mitigation is to **name the difference in the field description and publish
both**. That is a legitimate design choice, honestly documented, and it is the opposite of V2's approach:
Oura resolves the ambiguity *in the contract*; V2 resolves it *by having one implementation*. Both are
defensible. Only one of them makes the guarantee machine-checkable, and Oura's does not — the difference
lives in prose, so `average_heart_rate` and "what the app shows" are the same type at the type level.

**The `app_sleep_phase_5_min` field is the strongest single artifact in this file.** A vendor adding a
field *specifically because the API value and the app value for the same named concept diverge* is
publishing, in the interface, that it maintains two implementations. It is recorded as a disclosed
divergence.

### 8.4 Time: one global rule, one violation, four indistinguishable timestamp types

*The global rule.*

> "**Common Data Structures** … **Timestamps**: All time-based data uses ISO 8601 format … **Durations**:
> Provided in seconds unless otherwise specified" `[DOCUMENTED CONTRACT]`

*Violation 1 — milliseconds.*

> `PublicHeartRateRow.timestamp_unix`: "Timestamp of the discrete sample as **unix time in milliseconds**."
> `[DOCUMENTED CONTRACT]`

The document's other encodings, *checked one by one*: `X-RateLimit-Reset` is "**Unix epoch (seconds)**";
the webhook payload example carries `"event_time": "2023-01-01T08:00:00+00:00"` in **ISO 8601**; the
webhook subscription's `expiration_time` is `type: string, format: date-time` — **ISO 8601, not a unix
integer**; `PublicSample.interval` is a `number` in **seconds**; `PublicSleepTimeWindow.day_tz` is an
**integer offset in seconds from GMT**; and every `*_duration`, `*_time` and `latency` field is stated
**in seconds**.

So the document uses **ISO 8601** (the stated rule and the large majority of fields), **unix seconds**
(one header), and **unix milliseconds** (one field). **Exactly one violates the document's own global rule,
and it is the one on the real-time heart-rate row** — the field a consumer building a live view would reach
for first. An earlier draft of this file listed the webhook `expiration_time` as a unix-seconds field; that
was **wrong and has been corrected here** — it is an ISO 8601 `date-time` string. The corrected inventory
is the one above.

*A fourth encoding worth naming.* `PublicSleepTimeWindow.day_tz` — "Timezone offset in second from GMT of
the day" — is the **only** place in the contract where a UTC offset is carried as data. Every row timestamp
in `LocalizedDateTime` is a bare string with no offset field beside it. So the API **does** report a
timezone, in **one** model, in **seconds**, and does **not** report one for any measurement row.

*The sharper problem — four timestamp types that are the same type.*

```json
"ISODate":           { "type": "string" }
"LocalDateTime":     { "type": "string" }
"LocalizedDateTime": { "type": "string" }
"UtcDateTime":       { "type": "string" }
```

**All four are bare strings** — the complete set of schemas in the document whose definition is exactly
`{"type": "string"}`. For each: **no `format`, no pattern, no example, no description.** The semantic
difference between "this timestamp is localised" and "this timestamp is UTC" is carried **entirely by the
type name** and is not machine-checkable, and the contract never explains the difference between
`LocalDateTime` and `LocalizedDateTime` at all. Usage splits cleanly and the split is not documented:

- `LocalizedDateTime` — `PublicDailyActivity`, `PublicDailyReadiness`, `PublicDailySleep`,
  `PublicModifiedSleepModel`, `PublicRestModeEpisode`, `PublicRestModePeriod`, `PublicSample`,
  `PublicSession`, `PublicVO2Max`, `PublicWorkout` (**10 models**)
- `UtcDateTime` — `PublicHeartRateRow`, `PublicRingBatteryLevelRow`, `PublicRingConfiguration`
  (**3 models**)

`[INFERENCE]` the pattern is coherent — point samples are UTC, daily/nightly aggregates are localised —
but **the contract does not state it**, so a consumer must infer the rule from which model it holds.

*The third leg — query boundaries are evaluated in a timezone the consumer does not control.*

> "All timestamps in the Oura API use ISO 8601 format with timezone information. When querying data with
> date parameters, **the dates are interpreted in the user's local timezone**. Make sure to account for
> this when processing data across different timezones." `[DOCUMENTED CONTRACT]` — FAQ tag

So: the **row** timestamps are either UTC or localised depending on the model and cannot be
distinguished programmatically; and the **query boundary** is interpreted in a per-user timezone that can
change (travel, relocation, device settings) and that the API does not report. `[INFERENCE]` the same
query re-issued later can return a different set of rows with no field changing. **This is the most
reproducibility-hostile aspect of the contract**, and it is disclosed — the vendor tells the integrator to
account for it and does not provide the means. `PublicSleepTimeWindow.day_tz` (seconds from GMT) is the
single exception and covers only the sleep-time recommendation.

AnxietyWatch2's counterpart rule is absolute: "The store holds **UTC epoch seconds and nothing else**.
Local time exists only in presentation." Oura stores and transports a localised string and resolves the
conversion per query.

### 8.5 Missingness — representable, documented, and the integrator's problem

*The container.*

```json
"PublicSample": {
  "interval":  { "type": "number", "description": "Interval in seconds between the sampled items." },
  "items":     { "$ref": "#/components/schemas/Array_Union_float__NoneType__Bits64_" },
  "timestamp": { "$ref": "#/components/schemas/LocalizedDateTime" }
}
"Array_Union_float__NoneType__Bits64_": {
  "title": "ArrayNullableFloatBits64",
  "type": "array",
  "items": { "anyOf": [ { "type": "number" }, { "type": "null" } ] }
}
```

Three things are right here and are recorded as positives:

1. **The sampling interval travels with the data.** `interval` is a required field on the sample object,
   so the consumer does not have to assume a rate. AnxietyWatch2's spool and TimescaleDB path needs the
   same information and derives it from the source's declared rate; Oura publishes it per object.
2. **A hole is representable in band.** The element type is explicitly nullable, so "no value here" is
   distinct from "the value zero" and from "the array is shorter". The element type's name,
   `ArrayNullableFloatBits64`, says so.
3. **`null` is not a sentinel.** There is no `-1`, no `0x07FF`, no `-9999`. This is exactly what
   AnxietyWatch2's rules demand ("Invalid or absent data contributes nothing. Never coerce a sentinel to a
   number", with the IEEE-11073 SFLOAT `0x07FF`/`0x0800` case named). **A nullable array element is the
   correct structural answer, and Oura ships it.**

*The guidance.*

> "How should I handle missing data? Some users may have gaps in their data if they didn't wear their ring
> or haven't synced it recently. Your application should:
> - **Check for null values in responses**
> - Have **fallback logic** for missing data points
> - Consider **displaying gaps in visualizations rather than connecting across missing data**
> - Use webhooks to ensure you're promptly notified when new data becomes available"
> `[DOCUMENTED CONTRACT]` — FAQ tag

> "**Error Handling**: Be prepared for **occasional gaps in data** if users don't regularly sync their
> rings" `[DOCUMENTED CONTRACT]` — Data Model & Concepts tag

**"displaying gaps in visualizations rather than connecting across missing data" is the same instruction as
`AMENDMENTS.md`:61** ("Never use `locf()` or `interpolate()`… Use `time_bucket_gapfill`"), arrived at
independently, addressed to integrators rather than to implementers. It is recorded as the strongest
positive in this file and as the closest published analogue to V2's missingness rule found in the whole
package.

*What is not documented.* `[UNKNOWN]` **what a null element means.** The contract distinguishes absent
from zero and does not distinguish *not worn* from *sensor failed* from *rejected by the algorithm*. So
the structural decision is correct and the **semantics of the hole are undocumented** — the consumer is
told to render a gap and not told what the gap is evidence of. AnxietyWatch2's three-state rule
("measured-and-good, measured-and-bad, and **not measured**") collapses these into "not measured" for
storage and keeps the reason in the rejection path (`h10_collector.rs:330–332` records rejections to
stderr). Oura documents neither the reason nor a place to look for it.

*One more in-band encoding, and the contract is inconsistent about it.* Three fields carry a **string of
digits, one character per block**: `sleep_phase_5_min`, `sleep_phase_30_sec`, `app_sleep_phase_5_min`
('1' deep, '2' light, '3' REM, '4' awake), `movement_30_sec` ('1' no motion, '2' restless, '3' tossing and
turning, '4' active) and `class_5_min` ('**0 = non wear**', '1' rest … '5' high activity). The vendor
supplies an example string for each.

- **In the two sleep-phase strings and `movement_30_sec`, non-wear and non-classification are not
  representable.** There is no `'0'`, no null, no skip token and no per-character timestamp. `[INFERENCE]`
  a period with no classification is either omitted from the string or encoded as one of the real stages;
  **the contract states neither.**
- **In `class_5_min`, non-wear *is* a first-class code** — `'0' = non wear` — and the vendor's own example
  string begins `"001233…"`, i.e. it demonstrates the non-wear code in use.
- **The three alphabets disagree with each other and no shared convention is stated.** `class_5_min` runs
  **0–5**; the sleep-phase and movement strings run **1–4**; and the same digit means different things
  across them (`'1'` = *rest* in `class_5_min`, but *no motion* in `movement_30_sec`, and *deep sleep* in
the phase strings).

**This is the one place where the API's missingness model does not hold**, and the finding is sharper than
*the encoding cannot express a hole*: Oura solved exactly this problem **in one of the three fields and not
the other two**, in the same contract, with no stated reason. Recorded as an intra-document inconsistency,
and the `class_5_min` treatment is recorded as the correct pattern.

The same model pair also shows the missingness design working correctly at the field level:
`PublicDailyActivity.non_wear_time` ("Ring non-wear time in seconds") is a **measured quantity** rather
than an absent row, so a day the ring was not worn is recorded as non-wear **duration** while the
classification string carries `'0'` and `score` is nullable. Three different, mutually consistent ways of
saying "not measured", none of them a sentinel. Recorded as a positive.

### 8.6 The vendor's blog contradicts the published contract on sleep-phase resolution

> "Five-minute device output was compared against 30-second reference scoring. **Oura's public API reports
> sleep stages in five-minute blocks**, and the authors assigned each five-minute label to all ten
> underlying 30-second epochs scored by polysomnography."
> `[PUBLIC CLAIM]` — Oura blog, 2026-08-28, Raphael Vallat, PhD

> `sleep_phase_30_sec`: "30-second sleep phase classification for the period where every character
> corresponds to: '1' = deep sleep, '2' = light sleep, '3' = REM sleep '4' = awake."
> `[DOCUMENTED CONTRACT]` — `openapi-1.41.json`

Both are the vendor's own, both are current, and the specification exposes a 30-second field the article
says the API does not report. `[INFERENCE]` the most likely explanations are that the 30-second field was
added after the 2025 evaluation the article discusses, or that the article describes the field of record
for researchers rather than the full contract. **`[UNKNOWN]` which is correct; both statements are
recorded and neither is preferred.** The comparison does not depend on the resolution: the finding is that
a vendor's public scientific argument and its public interface specification disagree about the resolution
at which the product reports the quantity under dispute, and a reader of either document alone would not
know.

### 8.7 Webhooks — the documented delivery model

`[DOCUMENTED CONTRACT]` — "Webhook Subscription Routes" tag, 8,370 characters, quoted below.

**Management.** `GET` / `POST` / `PUT` / `DELETE /v2/webhook/subscription[/{id}]` plus
`PUT /v2/webhook/subscription/renew/{id}`, authenticated with the `x-client-id` / `x-client-secret`
header scheme. Subscriptions carry an **`expiration_time`** — so a subscription **expires** — and there is a
dedicated **renew** operation, meaning **renewal is the consumer's responsibility**.

**Verification.** On creation Oura sends `GET <callback>?verification_token=…&challenge=…` and the endpoint
must return `{"challenge": "<value>"}`.

**Payload.** `{event_type, data_type, object_id, event_time, user_id}` with headers `x-oura-signature` and
`x-oura-timestamp`, delivered as a POST. The consumer must then **fetch the full record** using
`object_id` — the webhook is a notification, not a payload.

**Signature.** HMAC-SHA256 over `timestamp + JSON.stringify(body)`, keyed on the OAuth **client secret**,
hex-encoded **uppercase**, compared to `x-oura-signature`.

**Timing.** "Webhook notifications arrive **approximately 30 seconds after data syncs from the mobile
app**"; the endpoint must respond "**within 10 seconds with a 2xx or 3xx status code**".

**Retries.** "For 4xx responses: **10** retries; For 5xx responses: **10** retries; For timeouts: **10**
retries." **No backoff schedule, interval or total window is published.**

**Lossiness — stated by the vendor.**

> "What if my server goes down? Oura will retry webhook deliveries for **about an hour** if your server
> doesn't respond properly. However, **if your server is down for an extended period, you might miss some
> events.** It's a good practice to **implement a reconciliation process** that can fetch data for periods
> when your webhook might have been unavailable." `[DOCUMENTED CONTRACT]`

Five things follow, and the first is a positive:

1. **The vendor documents that its own delivery path is lossy and tells the integrator to reconcile.**
   Recorded as an honest boundary statement. AnxietyWatch2's spool exists to make the equivalent problem
   *internal* — `cpap-ezshare-ingest` invariant (7) is "**Attempts precede work**" (`design.md`:26) — so
   that a failed ingest is a recorded attempt rather than a silent gap. Oura has no attempts concept; it
   has a **notification channel with a documented drop rate described in prose** and an instruction to the
   consumer to build the reconciliation. Both approaches admit the same failure; one of them keeps the
   ledger.
2. **"About an hour" and ten retries with no schedule is not an at-least-once guarantee.** A consumer
   cannot compute how many events it should expect to recover, or when it is safe to conclude that no
   more are coming. `[INFERENCE]` the reconciliation window is unknowable from the contract.
3. **No replay window is documented for the signature.** `x-oura-timestamp` is supplied and is used as
   HMAC input; the specification **states no validity period, does not require the receiver to compare it
   to the current time, and the vendor's own reference implementation** (`Code Examples` tag) **does not
   check it** — it computes the HMAC over `timestamp + JSON.stringify(body)` and compares signatures, with
   no freshness test. `[UNKNOWN]` whether a window is enforced server-side. Recorded as a documented
   mechanism with an undocumented freshness guarantee, **not** as a vulnerability claim.
4. **The documented verification procedure requires re-serialising the parsed body.** The HMAC is defined
   over `JSON.stringify(body)`, so a receiver that verifies a parsed object depends on its JSON
   serialiser reproducing the sender's bytes — key order, whitespace, unicode escaping and number
   formatting included. The specification **does not state that the raw request body must be used**. This
   is a real integration hazard and it is exactly the sort of thing V2's COPY-binary spool and versioned
   metadata design avoids: the stored artifact is the bytes, not a re-encoding of them.
5. **A subscription that expires stops delivering silently unless the consumer renews.** Expiry is
   machine-readable in the subscription model (`expiration_time`) and **no duration, warning, or
   notification-before-expiry is documented**. Combined with the lossiness statement, the failure mode of
   a consumer that forgets to renew is **absence of events, indistinguishable from absence of data**.

### 8.8 Rate limits and pagination — a published number that does not fit the published model

*Two statements, both the vendor's, in one document.*

> "The API enforces rate limits at **two layers** … a **per-access-token** limit … and a **per-application**
> limit … A request that trips either layer receives a `429` … The **`X-RateLimit-Tier`** response header
> identifies which layer fired." `[DOCUMENTED CONTRACT]` — `info.description`

> "How do I handle rate limits? The Oura API is limited to **5000 requests per 5-minute period**."
> `[DOCUMENTED CONTRACT]` — FAQ tag

`[INFERENCE]` both can be true if 5,000/5 min is one of the two ceilings, but **the document does not say
which**, and the two-layer model's own header set (`X-RateLimit-Limit`, `X-RateLimit-Window`,
`X-RateLimit-Tier`) exists precisely because the ceiling is not a fixed global constant. The consequence
is concrete: **a consumer cannot know its ceiling before receiving a `429`.** There is no published
per-application figure at all. Recorded as an **intra-document conflict** — a single flat number in the FAQ
against a two-layer model with unpublished ceilings in the overview.

What the document does get right, and it is unusual: `Retry-After` (RFC 7231), `X-RateLimit-Limit`,
`X-RateLimit-Window` (rolling window, in seconds), `X-RateLimit-Reset` (epoch seconds) and
`X-RateLimit-Tier` are all specified, with the instruction "**Prefer these over fixed-interval backoff**".
That is a better-specified rate-limit contract than any other candidate in the package.

*Pagination.* `next_token` is the only pagination mechanism. **No page size, no maximum `next_token`
lifetime, and no maximum date range are documented for any endpoint.** The FAQ offers prose guidance
("Use date range queries with reasonable time spans (e.g., **1-3 months at a time**)") — which is a
recommendation, not a contract, and is not machine-checkable. `[INFERENCE]` an integrator's paging loop
must be written defensively against an unknown page size; compare Nightscout's published
`API3_MAX_LIMIT 1000` (`lib/api3/const.json:2–6`), which is a documented ceiling for an equivalent
operation.

*Field selection.* `fields` is described as: "Comma-separated list of fields to include in the response,
**in addition to the always returned fields**. Defaults to all fields if not provided." Two observations:
the parameter **cannot exclude** (it is additive), and **the set of "always returned fields" is not
published**. `[INFERENCE]` a response's minimum size is therefore unknowable from the contract.

### 8.9 Undocumented value domains in the contract

Applying the standing sub-check to every `string`-typed field: **an undocumented value domain is a
finding**, because a `string` with no enumeration is an unconstrained value at the type level.

| Field | Type as published | What is missing |
|---|---|---|
| `PublicWorkout.activity` | `string`, "Type of the workout activity." | **No enumeration, no example.** The set of activity names is unpublished |
| `PersonalInfoResponse.biological_sex` | `string` \| null, **no description** | No enumeration |
| `PersonalInfoResponse.weight`, `.height` | `number` \| null, **no description** | **No unit** |
| `PersonalInfoResponse.id`, `.age`, `.email` | `string`/`integer`, **no description** | No statement of the identifier's meaning or stability |
| `PublicVO2Max.vo2_max` | `integer` | **No unit**; integer type cannot carry fractional VO₂max |
| `PublicSpo2AggregatedValues.average` | `number` | **No unit, no range** |
| `PublicModifiedSleepModel.average_hrv` | `integer` | **No unit, no method** |
| `PublicSample.interval` element semantics | `ArrayNullableFloatBits64` | Element `null` meaning undefined |
| `sleep_phase_5_min` / `_30_sec` / `app_…` character equivalence | `string` | Whether the three encodings use the same alphabet is stated for each separately and never compared |
| `LocalizedDateTime`, `UtcDateTime` | `string` | No `format`, no pattern, no description |
| `PublicDailyActivity`, `PublicModifiedSleepModel` aggregate fields | mixed | See §7.2 for the six unit-less fields |
| `next_token` | `string` | No page size, no lifetime |
| `PublicRingConfiguration.size` | (US ring size) — the one dimension field in the API whose **unit is stated** | Recorded as a positive contrast to `weight`/`height` |

That is **eleven undocumented-domain instances in one published contract**, against a document whose
`PersonalInfoResponse` object has six properties and **zero descriptions**.

### 8.10 Two contract-quality positives

1. **The sandbox is labelled, addressable and honestly described.** `[DOCUMENTED CONTRACT]` — "Sandbox
   Routes" tag, verbatim: "**Fake user data that you can access without an Oura account.** There is a
   corresponding sandbox endpoint to each available data type. This is useful for testing and development
   purposes. **The data is not real and should not be used for any production purposes. The data is
   generated by Oura and is not based on any real user data.** The data is not updated in real-time and is
   not guaranteed to be accurate. **The rate limit for the sandbox endpoints is shared with your rate
   limit on other data endpoints.**"
   Contrast `sleephq`, whose specification's `example` blocks were **synthetic without being labelled**
   (candidate 5's standing finding). Oura's synthetic data is at a **separate path**
   (`/v2/sandbox/…`), **labelled fake in the contract**, and the label states it is generated and not real.
   That is the correct handling of the same problem.
2. **The document states what it will not do.** `info.description` records the V1 sunset ("the previous
   V1 API has been sunset"), the deprecation of personal access tokens ("**deprecated in December 2025 and
   are no longer available for use**"), and the approval gate for >10 users. `[INFERENCE]` this is a
   vendor retiring an interface with a stated boundary rather than leaving callers to discover it.

   Against that: **personal access tokens were the simplest possible integration path**, and their removal
   leaves OAuth2 as the only route for every use case, including a personal script. `[PUBLIC CLAIM]` — the
   document's own statement. Recorded as a lifecycle fact, and as a directly relevant precedent for
   AnxietyWatch2's mobile-enrollment and network-trust proposals (amendments 09–11).

### 8.11 A published claim the contract does not satisfy

> "Browse the available endpoints in this documentation to discover what data you can access. **Each
> endpoint includes example requests and responses**." `[DOCUMENTED CONTRACT]` — `info.description`

*Checked.* The specification contains **75 `x-codeSamples`** blocks (request-side code samples in cURL,
Python and JavaScript) and **zero `example` or `examples` keywords anywhere in the document** — no
response examples at all. So the claim is half-satisfied: **requests have examples, responses do not.**
Recorded as a checkable documentation defect, of the same shape as `nightscout`'s published `DEVICESTATUS`
default the code does not implement and `empatica`'s cited clearances that name the wrong device: **a
statement in a vendor's own document that the vendor's own artifact does not bear out.**

*Also observed, recorded for completeness and not as a finding:* the published `x-codeSamples` blocks
contain a concrete subscription UUID in a URL path (e.g. a `PUT /v2/webhook/subscription/renew/<uuid>`
sample). Its provenance is not disclosed. No significance is attached to it and no claim is made about
what it is.

### 8.12 Record provenance, deletion and falsified detections — the strongest structural finding in this file

This section was added after a second verification pass over the schema set, because the fields it
contains were missed on the first pass and they are the most directly comparable material in the
candidate.

**A user-deleted sleep period is a tombstone, not an absent row.** `PublicModifiedSleepModel.type`
references `PublicSleepType`, whose **enumeration and vendor-written description** are, verbatim:

```
"deleted"     = deleted sleep by user.
"sleep"       = user confirmed sleep / nap, min 15 minutes, max 3 hours, contributes to daily scores
"long_sleep"  = sleep that is long enough (>3h) to automatically contribute to daily scores
"late_nap"    = user confirmed sleep / nap, min 15 minutes, ended after sleep day change (6 pm),
                contributes to next days daily scores
"rest"        = Falsely detected sleep / nap, rejected in confirm prompt by user
```

`[DOCUMENTED CONTRACT]` — `PublicSleepType`, quoted in full.

Three things in that block are findings, and they are the reason this file's conclusion is more positive
about Oura than its first draft was:

1. **Deletion is a value, not a row removal.** `'deleted' = deleted sleep by user` means the record the
   user deleted **remains addressable** and carries the reason for its state. This is precisely
   AnxietyWatch2's rule: "No `UPDATE`, no `DELETE` against `samples`. It is append-only; removal is a
   **tombstone row**." **Oura arrived at the tombstone independently, for a consumer wearable, and
   publishes it in the contract.** With §8.1's read-only finding, the shape is complete: a consumer can
   see *that* a period was deleted, and cannot delete it themselves.
2. **A falsified detection is stored and labelled as falsified.** `'rest' = Falsely detected sleep / nap,
   **rejected in confirm prompt by user**` keeps the machine's original detection **and the reason it was
   rejected**, rather than discarding it. Compare `AGENTS.md`: "Never clamp an out-of-range value.
   **Reject it, name the reason, quarantine it.**" This is the same rule, applied to a false positive
   rather than an out-of-range value, and it is the second independent arrival at the pattern in this
   candidate (the first is §8.5's nullable container). **A rejected detection is therefore still
   available to a third party** — which is what makes a false-positive rate computable downstream at all.
3. **The vendor's own day boundary is published.** The descriptions state the nap window (**min 15
   minutes, max 3 hours**), the **sleep-day change at 6 pm**, and the >3 h threshold above which a sleep
   contributes to the daily score automatically. So "which day a sleep belongs to" is a **stated rule with
   stated thresholds**, not an implementation detail. `[INFERENCE]` this is directly relevant to V2's
   `time_bucket_gapfill` day bucketing and to the `xe125-respiratory-rate` change: V2 has no published
   day-boundary rule of this kind, because it has no published interface — recorded as a **documentation
   comparator**, not as a V2 defect.

**The algorithm that produced a record is versioned in the record.**

> `sleep_algorithm_version`: "Version of the sleep algorithm used to calculate the sleep data."
> `PublicSleepAlgorithmVersion` enum: **`['v1', 'v2']`**
> `[DOCUMENTED CONTRACT]`

**This materially corrects §11.10.** Oura's *blog* dates are inconsistent about which release was called
what, and the *API* carries a per-record **`sleep_algorithm_version`** whose enumerated values are `v1`
and `v2`. So the question a downstream analyst actually needs answered — *which algorithm produced this
night's staging* — **is answerable from the data**, while the marketing and scientific articles use
inconsistent labels for the same releases. §11.10 is therefore restated below as a **documentation-naming
inconsistency**, and this field is recorded as a **strong provenance positive**: a derived classification
carries the identifier of the model that produced it. AnxietyWatch2 requires the same property in a
different place — "the unit and valid range come from `vitals/registry.toml`", and `cpap-ezshare-ingest`
invariant (6) is that "clock correction is **versioned metadata**" — so both systems version what
produced a value; Oura versions the *algorithm*, V2 versions the *unit and the clock*.

**Why a record exists at all is also versioned.**

> `sleep_analysis_reason`: "The reason for the creation or update of the latest version of this sleep."
> `PublicSleepAnalysisReason` enum:
> `['foreground_sleep_analysis', 'bedtime_edit', 'background_sleep_analysis', 'background_created_foreground_updated']`
> `[DOCUMENTED CONTRACT]`

This is a **mutation-provenance field**: it distinguishes a record produced by a background pass from one
created in the foreground, from one produced by the user **editing their bedtime**, from one where the
background created it and the foreground later updated it. So the contract states **that a sleep record
can be revised, and why**. Recorded as a positive and as an unusually precise disclosure — a consumer that
sees two versions of the same night has a field explaining the difference. V2's append-only `samples` table
has **no counterpart field**, and the analogue is the tombstone-plus-attempts machinery rather than a
reason code (see §14.6 G12).

**Two smaller provenance facts in the same model:**

- `low_battery_alert: boolean` — "Flag indicating if a low battery alert occurred." So a **device-state
  condition is attached to the biological record**. `[INFERENCE]` this is the field that would let an
  analyst exclude or annotate a night whose data quality was compromised by power state, and it is the
  only device-health field in the contract. Recorded as a positive comparator to V2's `xe125`
  respiratory-rate work, where a sensor-state condition and a physiological value are the two things a
  consumer must not confuse.
- `ring_id` — "Encrypted identifier of the ring that produced this sleep data. **Deprecated, returns
  null.**" A field that is **present in the contract, required to be understood, and guaranteed to carry
  no information**. Recorded as a **low-severity** finding: the documentation *states* it returns null, so
  a consumer reading the description is correctly warned, and the field's continued presence is a
  compatibility measure rather than an omission. `[INFERENCE]` the shape is nevertheless worth naming
  because a consumer that treats field presence as evidence of measurement will read `ring_id: null` as
  "the ring identity is unknown" when it means "this field no longer exists". That is the **same hazard as
  a `not measured` value being read as a measurement**, with the documentation on the correct side.

**What §8.12 changes about this file's conclusions.** Three of the four rules AnxietyWatch2 treats as
non-negotiable — append-only with tombstones, reject-and-name-rather-than-clamp, and no-interpolation
missingness — are **independently present in Oura's published contract**, and Oura's version of two of them
is expressed in the *data* rather than in *guidance*. The comparison's honest framing is therefore not
*V2 has these properties and Oura does not*; it is **both systems have them, and V2 enforces them where
Oura documents them"** — Oura's tombstone is a value in a record that could also have been omitted, and
V2's is a schema-level requirement that has no alternative. `[INFERENCE]`.

---

## 9. Privacy and data handling as published

`[PUBLIC CLAIM]` — Privacy Policy, dated **June 15, 2026**, unless stated otherwise.

### 9.1 What the policy says it will not do

- Does not sell personal data; explicitly does not sell it for **cross-context behavioural advertising**
- "We do **not sell personal data for the purpose of training third party Large Language Models**"
- Does not process location data without consent ("does not process such location data without first
  obtaining your consent")
- "In the preceding twelve (12) months, Oura has **not sold personal information** … as it is against our
  policies."
- No automated decision-making with legal effect
- Data-subject rights, EU–US / UK-Extension / Swiss–US **Data Privacy Framework** participation under FTC
  oversight

### 9.2 What it discloses that a reader should notice

- **Oura Advisor is an AI assistant**, and Oura trains its **own** models on user data. The prohibition is
  scoped to **third-party** LLM training and to **selling** — a narrow, precisely drafted limit. This is
  the correct reading of the sentence, and it is narrower than "we do not train on your data".
- **FHIR / EHR import.** "**the records we receive no longer fall under HIPAA while they are in Oura's
  possession**", and Oura is **not a Business Associate** of the importing entity. `[PUBLIC CLAIM]`
- **Oura Research App blinding.** "During a blinded period, you may not be able to access certain personal
  data, scores, or insights … **even though your Oura Ring continues to collect sensor data**."
- **Oura Enterprise Platform.** The Data Recipient becomes the controller and "**Oura is not responsible
  for the Data Recipient's processing**."
- **Storage tied to account lifetime**: "Your measurement data … is stored only so long as your Oura
  account is active."
- **Posthumous access** described ("right to privacy endures beyond their lifetime").
- **No transparency report yet**: Oura says it is "actively evaluating … like through a transparency
  report". `[PUBLIC CLAIM]` — recorded as the vendor's own statement of an absent artifact.
- Third-party AI/LLM providers are named as recipients rather than excluded.

### 9.3 The privacy policy and the Trust Center disagree about third-party AI — recorded unresolved

| Surface | Statement |
|---|---|
| Privacy Policy | Names LLM providers as recipients; the prohibition is limited to **selling** data for **third-party** LLM training and to **selling** data generally |
| Trust Center (`/security`) | "we **do not share or sell** your personal data. **That includes third-party AI models (LLMs)**." |

`[PUBLIC CLAIM]` — both. Under the Trust Center's wording, sharing with a third-party LLM provider is
excluded; under the Privacy Policy's wording, such sharing is disclosed and only the *sale for training*
is excluded. `[INFERENCE]` the two sentences can be reconciled if "share" is read narrowly (onward
disclosure for the provider's own purposes) versus processing on Oura's behalf — **the policy does not
define "share" in the way the reconciliation would require**, so the ambiguity stays open. **Recorded as a
conflicting pair with the more conservative statement governing**, which is the Privacy Policy: it names
recipients, so a reader should assume third-party processing occurs.

### 9.4 Three HIPAA statements that are scoped and mutually consistent — a discoverability asymmetry, not a conflict

This one is recorded carefully because an earlier pass through similar material (`empatica`) found a real
conflict, and this one is not:

| Surface | Statement |
|---|---|
| Privacy Policy | "Oura is generally **not a Covered Entity under HIPAA** for our direct-to-consumer Services" |
| Privacy Policy | Imported health records "**no longer fall under HIPAA while they are in Oura's possession**" |
| Trust Center | "We also meet **HIPAA requirements where we handle protected health information for a healthcare partner**" |
| `organizations.ouraring.com` | "Complies with stringent data protection laws like GDPR and **processes PHI in accordance with HIPAA**" |

`[PUBLIC CLAIM]` — all four. Read with their scope clauses attached, they are **consistent**: consumer
services are outside HIPAA; imported records leave HIPAA protection on transfer; partner PHI is handled
under HIPAA. The finding is not a contradiction — it is that **the consumer-facing surfaces never state
the third sentence's precondition**. A reader of the Trust Center's "we meet HIPAA requirements" and of the
B2B page's "processes PHI in accordance with HIPAA" has no way to learn from either surface that the
direct-to-consumer service is **not** covered and that importing records **loses** the protection they
had. That clause is in the Privacy Policy only. Recorded as a **discoverability asymmetry in the vendor's
disclosures**, not as a misleading claim.

### 9.5 Data Safety declarations

`[THIRD PARTY]` — Google Play. "**No data shared with third parties**"; "This app **may collect** these
data types Location, Personal info and 4 others"; "Data is encrypted in transit"; "You can request that
data be deleted".

**These differ in kind from the Privacy Policy and are not put in opposition to it.** Google's Data Safety
form has its own definitions, in which processing by service providers acting on the developer's behalf
is generally not "sharing". `[INFERENCE]` and `[UNKNOWN]` — Google's current definition text was **not**
retrieved for this comparison, so no contradiction is asserted. The finding recorded is an
**interpretation hazard**: a user reading the Play badge "No data shared with third parties" beside a
privacy policy listing named AI providers, analytics vendors and payment processors will draw a
conclusion the badge does not make, and neither surface cross-references the other's definition.

### 9.6 Terms — contract clauses relevant to a data-dependent product

`[PUBLIC CLAIM]` — Terms, **Last Updated 2026-10-02**, **Effective Date 2026-11-04**
(see §14.1 for the lifecycle point). The clauses that bear on integrating with Oura:

- **Liability cap**: "IN NO EVENT SHALL ŌURA BE LIABLE … IN EXCESS OF THE AMOUNT PAID TO OURA IN THE
  PRIOR TWELVE (12) MONTHS."
- **Unilateral modification**: Oura may modify the terms "at any time and for any reason in its sole
  discretion by posting updated terms".
- **Revocability of the service**: "**ŌURA reserves the right … to remove, modify, suspend, or disable
  access to all or any portion of the Services.**"
- **Mandatory binding arbitration** with a class-action waiver and mass-arbitration procedures.
- **Governing law**: California (US), Finnish law (non-US).
- **Inactivity**: an account-inactivity clause conditioned on a free trial.
- **Disclaimed liability for user-supplied health records**: "**ŌURA disclaims all liability that arises
  out of or in any way relates to any information that you provide or direct to be provided to ŌURA, even
  if that information constitutes electronic patient health records or similar information.**"
- **Emergency use**: "**NOT INTENDED FOR USE IN EMERGENCY OR LIFE-THREATENING SITUATIONS**."
- **Population-average caveat**, quoted in full in §11.8.

**An API dependency is revocable at will under clause 4**, against a FAQ stating the API "is free to use
for both personal and commercial applications". Both are Oura's; the terms are the instrument that governs.
Recorded as a contract-versus-marketing pair, unresolved and without any implication of bad faith — this
is ordinary consumer-software practice, and it is the same shape as `empatica`'s EULA finding.

---

## 10. Security and access control as published

`[PUBLIC CLAIM]` — Trust Center, `ouraring.com/security`.

**Claims:**

- **SOC 2** — "independent auditors … verify … designed and operating as effectively as intended".
  **No Type I or Type II is stated**, and no report is published. `[UNKNOWN]` which. Recorded as an
  **undefined term in a certification claim**, milder than an unsupported claim but the same class as
  `empatica`'s "corrected PPA".
- **HITRUST certification** — "Oura obtains a HITRUST certification". **No level, no scope, no date, no
  certificate identifier.**
- Penetration testing; **NIST CSF** alignment; a **bug bounty** programme.
- **Encryption chain, stated at algorithm level** — the most specific security claim on the surface:
  "**Bluetooth data is encrypted between Oura Ring and your phone with AES-128, with TLS 1.2+ while in
  transit, and with AES-256 at rest.**"
- **HIPAA** where handling partner PHI (§9.4).
- A **Security Resource Center** with a whitepaper described as "available to view here … **upon request**"
  — `[INFERENCE]` a gated document, so it is not a published artifact and cannot be reconciled.

**Two claims that conflict with the Privacy Policy, recorded unresolved:**

1. **"We don't sell or share your data to train other AI models"** / "we do not share or sell your
   personal data. That includes third-party AI models (LLMs)". The Privacy Policy names LLM providers as
   recipients. See §9.3 — the more conservative statement governs.
2. **"deleted data is fully removed from your account"** against the Privacy Policy's treatment of
   retention, which conditions storage on account activity and carves out legal retention. `[INFERENCE]`
   the two can coexist (account-scope removal versus system-scope retention), and **the Trust Center
   sentence does not carry that qualifier**. Recorded as a conflict with the policy governing.

**What is absent.** `[UNKNOWN]`: no `security.txt`, no incident history, no disclosure policy text, no
sub-processor list, no SOC 2 report, no penetration-test summary, no bug-bounty scope or payout figures
were located. This is **absence of evidence and is not evidence of absence**; the security page is
substantive enough that its omissions are worth naming rather than its content being discounted.

**Where security detail lives — a placement observation.** Oura's **most specific** security statement is
an end-to-end encryption chain with named algorithms, and it appears on the **marketing Trust Center**
rather than in the API documentation. The API documentation contains no security section beyond
authorisation: no statement about token storage, secret handling, or the protection of webhook client
secrets beyond "never store access tokens in client-side code". `[INFERENCE]` a developer following only
the specification has less security guidance than a reader of the marketing page.

---

## 11. Clinical, regulatory and validation claims

### 11.1 The regulatory position, stated by the vendor

The Play listing's disclaimer (§2) is the vendor's general position: **not a medical device, general
fitness and wellness only, and expressly not intended to "monitor"**. Health Radar's own footnote repeats
it (§11.7). Against that, §11.6 records the one feature the vendor says **is** a medical device.

### 11.2 The four headline figures on the science page

`[PUBLIC CLAIM]` — `ouraring.com/science-and-research`, headline statistics, quoted verbatim:

| As published | Qualifier as published | Source the page footnotes |
|---|---|---|
| "**99% Heart Rate Accuracy**" | "**r² compared to ECG**" | `blog/how-accurate-is-oura/` |
| "**98% Heart Rate Variability Accuracy**" | "r² compared to ECG" | `blog/how-accurate-is-oura/` |
| "**94% Ovulation Detection Accuracy**" | compared to ovulation prediction kits | (not footnoted) |
| "**79% Sleep Tracking Accuracy** — Compared to clinical polysomnography at 83%" | — | `blog/new-sleep-staging-algorithm/` |

**Finding — a category mismatch rendered as a headline.** An **r²** is a coefficient of determination: the
proportion of variance in one signal explained by another. It is **not** a percentage of measurements
classified correctly, and it is not an "accuracy". The page's own qualifier says so, and the headline noun
does not. The correct reading of "99% Heart Rate Accuracy / r² compared to ECG" is *"r² = 0.99 against
ECG"* — a strong correlation between two signals, which is not the same claim as *99% of heart-rate
readings are correct*. **Both surfaces are quoted; the discrepancy is in the noun, and it is left
unresolved.** This is the sharpest available instance of the standing rule that a derived statistic must
not be confusable with the quantity it summarises.

**Finding — a paired comparison that is fairer than it looks.** "**79% … compared to clinical
polysomnography at 83%**" places the device's own result beside the **reference standard's own
inter-scorer agreement**, and footnote ² leads to the article that explains the 83% figure. A vendor
publishing its device's number next to the human benchmark's imperfection is doing something almost no
comparable product does. Recorded as a **positive**, with the caveat that the qualifier is only reachable
by following the footnote.

### 11.3 The studies behind 99% and 98% were written by Oura staff

`[PUBLIC CLAIM]` — `blog/how-accurate-is-oura`, the page the science statistics footnote:

- A **2020 study of 49 healthy adults**
- Reports **r² = 0.996** for resting heart rate and **r² = 0.980** for HRV
- Also reports **5-minute segment** values: **r² = 0.918** (resting HR) and **r² = 0.827** (HRV)
- States that "**Hannu Kinnunen, lead study author and Oura's Chief Scientist at the time**" — the
  vendor's own words
- Describes r² as "**correlation**"
- The gallery entry title on the science page reads: "Heart Rate and HRV: Oura Ring's Average Nighttime
  Values **Correspond Near-Perfect with ECG**"

**Finding — the evidence-classification defect.** The science page presents these studies under the
heading "**Independently validated sensors**". The page that supplies the number says the lead author was
Oura's Chief Scientist. **Both statements are the vendor's own, on two pages of the same site, and the
gallery heading is contradicted by the source it links to.** Recorded verbatim, with no speculation about
the paper's independence beyond what the vendor states. Note also that the headline figure (r² = 0.996,
whole-night average) is materially higher than the same study's per-segment figure (r² = 0.918) — the page
that publishes both uses the first for the headline.

### 11.4 The study behind 79% was also written by the Oura research team

`[PUBLIC CLAIM]` — `blog/new-sleep-staging-algorithm`, "**Sleep Staging Algorithm 2.0**", published
**2022-11-16**, credited to the **Oura Team**:

- 79% agreement with polysomnography on four-stage classification
- "most commercial wrist-based wearables are limited to **60-65%** agreement"
- "**Lead author, Marco Altini, PhD**" — attributed to "**From the Oura Research Team**"
- Sensitivity/specificity **74–98%**
- An illustrative hypnogram labelled "**accuracy = 84%**" — a **single representative night**, not the
  study result
- Endorsements quoted from **Michael Chee** (NUS) and **Ian Colrain** (SRI Biosciences / University of
  Melbourne)
- "one third of the study participants did not tolerate the headband" (the Dreem reference device)

**Finding — the same evidence-classification defect, and an undisclosed advisory relationship.** The lead
author is credited to the Oura research team, and the **science page presents the resulting figure in its
validation block**. Separately, **Michael Chee is named on the science page as an Oura Medical Advisor**
("Meet the Oura Medical Advisors" — `Michael WL Chee, MBBS, FRCP (Edin)`), and the blog quotes his
endorsement **without disclosing the advisory relationship at the point of quotation**. Recorded as a
**directly observable** juxtaposition of two vendor pages; **no claim is made about the independence or
quality of the underlying research**, and the relationship is not characterised as concealed — it is
published on the vendor's own site, one page away.

### 11.5 What "130+ peer-reviewed publications" counts

`[PUBLIC CLAIM]` — science page: "**130+ peer-reviewed publications**", "**14 active collaborations**",
"**12+ Years**". The page's own framing establishes that this is a count of publications **using** Oura
data, not of studies validating the product — it sits beside "Oura in Research" material and a call to
"**Oura Research Database**". Recorded with that distinction attached, because the number is
frequently read as a validation count and the page's own structure separates the two. **"The most
scientifically validated smart ring"** is the page's own superlative and is unverifiable by any means
available here.

### 11.6 The one CE-marked feature, and the boundary the vendor draws around it

`[PUBLIC CLAIM]` — Terms, section "**Regulated Features**" (the earlier draft cited a section title,
`Regulated Feature: Fertile Window`, which the Terms do not contain; corrected
here):

- **CE marked under MDR (EU) 2017/745**; legal manufacturer **Oura Health Oy**
- "**NOT INTENDED FOR USE AS CONTRACEPTION AND MUST NOT BE RELIED UPON TO PREVENT PREGNANCY.**"
- Predictions and estimates; may be inaccurate or unavailable; the **User Manual controls on conflict**
- The science page's "**94% Ovulation Detection Accuracy**" is a separate, uncited figure

**Recorded as a positive** on the disclosure axis: a vendor of a consumer wearable marking **one** feature
and stating its limit in capitals in the contract is doing the thing the package's other regulated
candidate (`empatica`) needed a regulator's own product-code definition to establish. The "must not be
relied upon" sentence is the Fertile Window's counterpart of `empatica`'s POS definition ("not intended as
a standalone seizure monitoring device"), stated by the vendor rather than found in a regulator's record.

### 11.7 Health Radar — the closest thing to a clinical surface, and its disclaimer

`[PUBLIC CLAIM]` — `ouraring.com/health-radar`:

- **Blood Pressure Signals** — "appears only when Health Radar identifies patterns that may indicate
  cardiovascular strain"; 30-day view; guidance
- **Nighttime Breathing** — 30-day view of sleep-related breathing patterns; prompts further evaluation;
  **explicitly not a sleep-apnea diagnosis**
- **Symptom Radar** — early alerts for respiratory illness, "sometimes before symptoms … are even felt"
- **Counsel Health** — "Care is provided by **Counsel's licensed physicians, not by Oura**"; prescriptions;
  live in **43 US states** (not AR, DC, KS, MS, RI, WA, WV, WI); "**not for emergency care**"
- The suite-level footnote: Health Radar is **general wellness only**, "not intended to diagnose, treat,
  cure, mitigate, monitor, or prevent any disease, illness, or medical condition", and "**Oura Ring and
  Advisor are not medical devices**"

`[INFERENCE]` **Symptom Radar is the closest published analogue to AnxietyWatch2's CNS risk scoring** —
an illness-onset signal derived from physiology, surfaced before the user would have noticed, with a
stated "sometimes" rather than a guaranteed lead time. It is recorded because the comparison needs a real
example of how a consumer wearable handles a *predictive* claim: Oura wraps it in a general-wellness
footnote and reduces the claim to "sometimes before symptoms … are even felt". **G7 in §14.6 records that no
sensitivity, specificity or false-alarm rate is published for Symptom Radar.**

### 11.8 The vendor's own caveat about population averages — an honest boundary statement

> "The information in the Services may be **misleading if your physiological functions and responses differ
> significantly from population averages**." `[PUBLIC CLAIM]` — Terms

Recorded as a **positive and as an unusually precise statement of the fundamental limitation of
population-trained algorithms**. It is the counterpart, in V2's terms, of the reason
`vitals/registry.toml` carries per-type valid ranges rather than a global one: a value that is normal on
average can be wrong for a person. Oura states the limitation and does not state what it does about it.

### 11.9 The claim is being litigated, and the vendor's rebuttal is on its own blog

`[PUBLIC CLAIM]` — `blog/sleep-staging-is-not-guesswork`, **2026-08-28**, author
**Raphael Vallat, PhD**, self-described as:

> "Since 2020, I have been the **lead algorithm scientist for sleep tracking at Oura**. That work included
> **building the sleep staging algorithm the ring uses today**."

What the article states, quoted:

- "**A proposed class action filed against Oura last week** alleges that our models are 'doing guesswork'
  and that our sleep staging has 'a coin flip's chance of being correct.'"
- "These claims are made on the basis of a **2025 study, published in *Scientific Reports***, that
  reported a **53% agreement for four-stage classification with the Oura Ring Gen3**."
- Oura's team "addressed [it] in **2025 in a peer-reviewed letter to the editor in *Sleep Advances***,
  'Toward better evaluation of consumer sleep technologies: a call for rigor, context, and collaboration'".
- "**Recent studies, both Oura-funded and independent, report four-stage agreement with clinical
  polysomnography in the 70 to 79% range in healthy adults**, and sleep versus wake agreement of ≥90%."
- "**An imperfect reference standard is not an excuse for an imperfect algorithm.**"
- "**Performance in older adults and in clinical populations is one area my team and I are actively
  working to improve**."
- Oura's detailed methodological objections to the 2025 study: ring fit ("nearly a third of the
  participants produced no usable Oura data at all … 14 out of 45"); 5-minute device output compared
  against 30-second reference scoring; missing device epochs "discarded rather than scored as wake"; no
  consensus scoring; no time-shift correction; algorithm and device version not reported; data collected
  in 2022 and the manuscript submitted in January 2025, "nineteen months after the algorithm release";
  Oura Ring 4 launched October 2024 and absent from the discussion.
- Inter-scorer agreement restated: "agreement between scorers is around **83%** in healthy adults, and
  lower in people with sleep disorders."
- The 2021 peer-reviewed algorithm paper: "Oura became one of the first wearable companies to
  transparently publish the inner workings of our algorithm in a peer-reviewed journal … cited more than
  **250 times**."

**Findings from this article, in order of consequence:**

1. **An independently published study reporting 53% four-stage agreement exists, and the vendor disputes
   it in the literature rather than ignoring it.** This is the **first candidate in the package where the
   headline performance figure has a serious, independently published counter-number.** Recorded
   unresolved, with the vendor's objections quoted and their specificity noted: these are falsifiable
   methodological claims, not a dismissal.
2. **The rebuttal is authored by the scientist who built the algorithm under challenge**, and his role is
   **disclosed in the article's first paragraph**. Recorded as a disclosed interest, exactly as the vendor
   disclosed it. **The disclosure is the mitigating fact and it is present.**
3. **This is the one Oura surface that distinguishes funded from independent research.** "both Oura-funded
   and independent" appears here. That distinction is **available to the vendor and is not used** on the
   science page's "Independently validated sensors" gallery or in the validation article's citation list
   (§11.3, §11.4). Recorded as a **documentation inconsistency within one vendor's own communications**,
   and it is the reason the §11.3 finding is stated as strongly as it is: the vendor demonstrably knows the
   difference.
4. **A vendor-stated performance limitation.** "Performance in older adults and in clinical populations is
   one area my team and I are actively working to improve" is a limitation **named by the vendor**, and it
   is recorded as a positive (§11.8's caveat is the same class). **`[UNKNOWN]` by how much performance
   differs in those populations** — no figure is given.
5. **The vendor's own article corroborates the inter-scorer-agreement figure used for the 83% benchmark**
   and thereby strengthens the "79% versus 83%" pairing's fairness (§11.2).

### 11.10 Release-naming inconsistency in the validation article — and how the API resolves it

`[PUBLIC CLAIM]` — `blog/how-oura-measures-sleep-and-validates-accuracy` (2026-08-23), which presents a
list of studies said to have evaluated "**Oura's latest sleep staging algorithm , released in 2023**":

| Date | Source | Statement |
|---|---|---|
| 2021 | the study cited in that list | The **MDPI *Sensors*** paper, "The Promise of Sleep: A Multi-Sensor Approach for Accurate Sleep Stage Detection Using Oura Ring" |
| **2022-11-16** | `blog/new-sleep-staging-algorithm` | "**Sleep Staging Algorithm 2.0**" announced; the same blog describes the algorithm as based on "a 2021 research study" |
| **~June 2023** (inferred from "nineteen months" before a January 2025 submission) | `blog/sleep-staging-is-not-guesswork` | "a major upgrade to its sleep staging algorithm" |
| 2026-08-23 | validation article | "Oura's latest sleep staging algorithm, **released in 2023**" |

`[INFERENCE]` the 2023 upgrade and the 2022 Algorithm 2.0 may be different releases, which would make the
2022 blog's "2.0" and the 2026 article's "2023" both correct for their dates. **Under either reading, a
2021 study cannot have evaluated a 2023 algorithm**, so the citation list contains at least one entry that
predates the thing it is offered as evaluating.

**This is restated as a naming and dating inconsistency rather than an unexplained defect, because the**
**published contract resolves the substantive question.** `sleep_algorithm_version` ∈ `{'v1', 'v2'}` on
`PublicModifiedSleepModel` (§8.12) means **the API states, per record, which of exactly two algorithm
generations produced that night's staging** — while the marketing and scientific surfaces use
inconsistent release labels for the same generations. So a downstream analyst is **not** dependent on the
blog dates: the data answers the question the prose confuses. `[INFERENCE]` this is arguably the single
most useful field Oura publishes for a third-party analyst, and it is published **quietly, as an enum on a
sleep record**, with no corresponding statement on any marketing or science page.

Recorded with all three data points quoted; **not** recorded as evidence about the study's quality, and an
earlier draft's framing of this as a straightforward defect has been **softened in the same file** — the
field that resolves it was missed on the first pass.

### 11.11 Other validation-surface observations

- The article states "**WE REMAIN CONFIDENT IN OUR SCIENCE, PROUD OF OUR ENGINEERING, AND COMMITTED TO
  DEFENDING OUR TECHNOLOGY IN COURT.**" `[PUBLIC CLAIM]` — recorded as a statement about the vendor's
  posture toward litigation, quoted because it is unusual to see on a product blog.
- The article cites "**Robbins et al., 2024**". **Dr. Rebecca Robbins, PhD is named on the science page as
  an Oura Medical Advisor.** `[PUBLIC CLAIM]` — the same advisory-relationship juxtaposition as §11.4.
  Recorded as observed; **no claim is made about the paper.**
- The article states "**more than 50 MDs and PhDs**" on staff and "**trained on … over 1,200 nights of
  clinical PSG data**" `[PUBLIC CLAIM]`.
- "**inter-scorer agreement at around 83%**" and "**the ring correctly detected sleep 94–98% of the time,
  with overall sleep/wake agreement of roughly 90–96%**" are the *unpacked* numbers behind the "95% Sleep Staging Accuracy" phrasing the article
  criticises in others — the article describes its own headline's basis in a footnote rather than in the
  headline. Recorded as consistent with §11.2's finding.
- "**While Oura is not a medical device intended to diagnose medical conditions like sleep apnea or
  insomnia**" — repeated in the validation article. `[PUBLIC CLAIM]`, and consistent with Health Radar's
  Nighttime Breathing disclaimer.

---

## 12. What Oura does not do — as published

Recorded because the comparison needs the negative space, and because every entry below is a vendor
statement or a directly observed absence in the vendor's own contract.

| Not done | Basis |
|---|---|
| **Not a medical device** for the consumer product; "not intended to diagnose, treat, cure, **monitor**, or prevent" | Play listing disclaimer |
| **Not for emergency or life-threatening situations** | Terms |
| **No real-time alerting or alarm path** of any kind is published | `[INFERENCE]` from the contract and the Trust Center; **no alarm, notification, escalation or caregiver mechanism appears in the API, the specification, or the security page**. Health Radar's outputs are surfaced in the app. |
| **Three data types have no webhook** (`heartrate`, `personal_info`, `ring_battery_level`) and **one webhook type has no read path** (`meal`) | §8.2 |
| **No raw sensor stream** — the API exposes processed data only | §6 |
| **No CPAP, PAP, respiratory-event, AHI, leak or therapy data** — not a field, not an endpoint, not a metric | `[INFERENCE]` from the complete schema inventory of `openapi-1.41.json` |
| **No self-hosting, no local-only mode, no on-premise deployment.** All data flows to Oura's cloud; the enterprise product is Oura's cloud with dashboards | `[INFERENCE]` from §5.3 and §8; the "Your Cloud"/CTMS-hosting pattern that `empatica` offers does not exist here |
| **No API for the analysis layer's reasoning** — scores and derived values are returned as values; no contributor decomposition beyond `sleep_score_delta`/`readiness_score_delta` | §7.1 |
| **No published false-alarm or error rate for any alert-like surface** (Symptom Radar, Blood Pressure Signals, Nighttime Breathing) | §14.6 G7 |
| **No DPA, SOC 2 report, transparency report or sub-processor list published** | §10; the vendor states the transparency report is being "evaluated" |
| **No OCR-independent instructions for use published for the consumer product** — no equivalent of a User Manual PDF was located for the ring/app | `[UNKNOWN]` — one was not searched for exhaustively; the Terms refer to a User Manual controlling on the Fertile Window |
| **No published Android data-safety scope matching the Privacy Policy's recipient list** | §9.5 |

**One item is recorded as a positive absence.** Oura does **not** publish an "accuracy" figure for its
scores. Sleep, Readiness and Activity scores are documented only as "Range from 0-100, with higher values
indicating better performance" `[DOCUMENTED CONTRACT]`. The vendor publishes a validation percentage for
heart rate and sleep staging — the quantities that have reference standards — and **none** for its
composite scores, which do not. Recorded as a correct restraint.

---

## 13. Relationship to AnxietyWatch v1 and AnxietyWatch2

| AnxietyWatch capability | Oura's position | Class |
|---|---|---|
| Real-time CPAP / PAP therapy streaming (V2 `resmed-air11-ble-stream`) | **Absent entirely.** No PAP concept in the product or the API | Gap in Oura's favour of V2 |
| AHI, leak, respiratory-event detection (`cpap-ezshare-ingest`, OSCAR ingest) | **Absent** | Gap in V2's favour |
| SpO₂ | Present as a **daily aggregate and a BDI**, not as a time series | Partial overlap |
| Respiratory rate | Present as `average_breath` for sleep | Overlap |
| Resting heart rate and HRV | Present; **two implementations of each (§8.3)** | Overlap, with a caveat V2 does not have |
| Nightly trend and correlation analysis | Present through scores and the app; **no correlation engine published in the API** | Gap in V2's favour for the API consumer |
| Local-first / offline operation | **None.** Cloud-mandatory, subscription-gated for API access | Gap in V2's favour |
| Missingness as "not measured" | **Structurally correct** (nullable arrays, `interval`, nullable score, nullable contributors, `class_5_min` non-wear code), **semantically undocumented** (§8.5, §8.12) | **The strongest positive found in the package** |
| One canonical unit per type | **Violated**: three time encodings, six unit-less fields, four timestamp types that are the same type | Gap in V2's favour |
| Provenance and rejections | **Partially present.** No attempts concept and no rejection *store*, but a **tombstone** (`"deleted"`), a **retained false positive with its rejection reason** (`"rest"`), an **algorithm version** (`sleep_algorithm_version`) and a **mutation reason** (`sleep_analysis_reason`) are all published (§8.12) | **Largely overlap — the gap is narrower than this file's first draft stated** |
| Supervised alarm / caregiver notification (`cns-engine`, `klaxond`) | **Absent.** Terms forbid reliance in emergencies; no alert path is published | Gap in V2's favour |
| Predictive illness signal | **Present** as Symptom Radar, "sometimes before symptoms … are even felt", with **no published rate** | Overlap with V2's CNS ambition; **Oura's disclosure is weaker than V2's specification on the alarm side and stronger on the marketing-caveat side** |
| Environmental and aircraft-noise tracking | **Absent** | Gap in V2's favour |
| Weather and context data | **Absent** from the API against V2's specified context feed | Gap in V2's favour |
| Open-source / self-hosted | **Not open source**; Terms prohibit reverse engineering and circumventing subscription restrictions (§14.3) | Gap in V2's favour |
| Camera-based PPG / phone-camera capture | Present on the Oura Ring as an extension; the ring is the primary sensor | Different form factor |

**The comparison's most useful inversion.** For the first time in this package, the candidate's
**missingness model is at least as disciplined as AnxietyWatch2's**, and in one respect better: Oura
publishes the sampling interval with the data and makes the hole representable in the element type. V2's
advantage is not that it has a missingness rule and Oura does not — it is that V2's rule is **enforced by
the schema** (`time_bucket_gapfill`, no `locf`, three-state rendering) rather than **advised in a FAQ**.
The difference is between a product that cannot interpolate and one that tells integrators not to.

---

## 14. Evidence posture and known limits of this file

### 14.1 A lifecycle fact with an exact date

The **Terms are published but not yet in effect** at the research date: "Last Updated: **October 2,
2026**", "Effective Date: **November 4, 2026**", research date **2026-10-06**. A "Previous Terms and
Conditions" link is provided, and the document carries the trailing control string `42200170v1`.
Recorded as a **lifecycle fact, not a defect** — a terms revision announced ahead of its effective date
is normal practice. The consequence for this file is stated: **the quotes in §9.6 are from a document that
had not taken effect**, and the document Oura itself links as "Previous Terms and Conditions" was **not**
fetched, so this file does not compare the two.

### 14.2 Deployment continuity

The iOS app's `currentVersionReleaseDate` is **2026-10-06T05:28:51Z** — the same date as this research. The
Android listing shows "Updated on Sep 18, 2026". Recorded as an **observation about cadence**, not as a
finding: the product was actively shipping on the research date, on at least one platform.

### 14.3 The contract's own prohibition — the authorization basis for Phase 2A

`[PUBLIC CLAIM]` — Terms §II prohibits, among other acts, verified verbatim against the fetched page:

> "(ii) reverse engineer, disassemble, decompile, decode, adapt, or otherwise attempt to derive or gain
> access to any **Oura Apps** component of the Services, in whole or in part"

> "(vi) attempt to probe, scan, or test the vulnerability of any ŌURA system or network or to breach
> security or authentication measures **without express authorization**"

> "(ix) circumvent, bypass, disable, degrade, or otherwise interfere with any access controls, usage
> limits, security features, authentication measures, **subscription restrictions**, technological
> protection measures, or other technical or contractual restrictions that ŌURA applies to the Services,
> including by using **multiple accounts, Credentials, devices, IP addresses, automated tools**, or other
> methods to avoid or exceed those restrictions"

**These three clauses were mis-quoted in an earlier draft of this file** — the paraphrase invented verbs
("remove, deactivate, impair … defeat") that do not appear in the contract and dropped the "Oura Apps"
qualifier from (ii). They are corrected here against the fetched text, because **an inexact quote of a
contract used as an authorization basis is worse than no quote**: a reader cannot tell which of the two
versions the vendor actually wrote, and the whole point of quoting a contract is that its wording is
adjudicable.

**This is quoted as the basis for the Phase 2A decision in
`oura_IMPLEMENTATION_INSPECTION.md` §2.** Binary inspection and any attempt to defeat a subscription gate
are contractually prohibited in the vendor's own words, and no such act was performed.

### 14.4 Claim ledger

Every checkable claim in this file, with the surface it comes from and its reconciliation status. **Status
is about whether the vendor's artifacts support the vendor's statement** — never about whether a
scientific or medical claim is true.

| # | Claim | Surface | Status |
|---|---|---|---|
| O-1 | Ring measures heart rate, HRV, temperature, SpO₂, movement | Play, science page | Documented in contract (`PublicHeartRateRow`, `PublicSample`, `PublicDailySpO2`, `movement_30_sec`) |
| O-2 | "over 30 biometrics" / "over 50 health metrics" | Play / home page | **Two surfaces, two counts**; neither enumerated |
| O-3 | No gyroscope, no EDA | science page (sensor list) | Consistent with contract inventory |
| O-4 | Sleep score, Activity score, Readiness score | home, contract | Match |
| O-5 | "Scores: Range from 0-100" | contract | Match (`PublicDailySleep.score` etc.) |
| O-6 | "99% Heart Rate Accuracy" | science page | **r² = 0.996 headline; noun is not the metric** (§11.2) |
| O-7 | "98% Heart Rate Variability Accuracy" | science page | **r² = 0.980; same noun problem** (§11.2) |
| O-8 | r² values from 2020 study of 49 adults | linked blog | Match — 0.996 RHR, 0.980 HRV; **per-segment 0.918 / 0.827** |
| O-9 | Study under "Independently validated sensors" | science page | **Lead author was Oura's Chief Scientist at the time — vendor's own words** (§11.3) |
| O-10 | "94% Ovulation Detection Accuracy" | science page | **Uncited**; no source located |
| O-11 | "79% Sleep Tracking Accuracy" | science page | Derives from Oura-team-authored algorithm article (§11.4) |
| O-12 | "Compared to clinical polysomnography at 83%" | science page | **Match and a positive** — 83% is the inter-scorer benchmark, restated by the vendor |
| O-13 | "Sleep Staging Algorithm 2.0", 79% agreement | 2022 blog | Match within that document |
| O-14 | Algorithm "released in 2023" | 2026 validation article | **Conflicts with the 2022-11-16 announcement; 2021 study in the same list predates it** (§11.10). **Partly resolved:** the contract's `sleep_algorithm_version` ∈ {v1, v2} answers the substantive question even though the prose labels conflict |
| O-15 | "130+ peer-reviewed publications" | science page | Count of publications **using** Oura — the page's own framing; not a validation count |
| O-16 | "The most scientifically validated smart ring" | science page | **Superlative; unverifiable** |
| O-17 | "5+ Million Oura Members" | home page | Vendor-published; not independently verified |
| O-18 | "86% of Oura Members see their health improve" | home page | **Vendor survey, 3,501 self-reported responses, no control**; footnote discloses |
| O-19 | "4.9 App Store Rating based on 280k reviews" | membership page | **Not recorded as a discrepancy** — the live store shows 4.8633 over 308,643 *ratings* while the vendor says 280k *reviews*; ratings and written reviews are different quantities and the page does not define which it means. **Both quoted, marked unresolved** (§4.2) |
| O-20 | Ring works without membership but with "much more limited" insights | membership page | Match with the API's `403`-on-lapsed-subscription behaviour |
| O-21 | "export all of your Oura data in CSV … without a membership" | home page | **Consistent with the contract** — export and API are different paths (§5.2) |
| O-22 | API "free to use for both personal and commercial applications" | FAQ | Consistent; **but end-user membership required for data** (`403`) |
| O-23 | API applications limited to 10 users before approval | FAQ, `info.description` | Match — stated twice, identically |
| O-24 | Personal access tokens no longer available | `info.description` | Lifecycle fact, stated by the vendor |
| O-25 | Webhook notifications "approximately 30 seconds after data syncs" | FAQ, webhook tag | Match across both statements |
| O-26 | Retries: 10 for 4xx, 5xx and timeouts | webhook tag | Documented; **no backoff schedule published** |
| O-27 | "you might miss some events … implement a reconciliation process" | webhook tag | **Documented lossiness — recorded as an honest boundary** |
| O-28 | Signature: HMAC-SHA256 over timestamp + body, uppercase hex | webhook tag, code sample | Match; **no replay window documented and the vendor's sample does not check the timestamp** |
| O-29 | "5000 requests per 5-minute period" | FAQ | **Conflicts with the two-layer model in `info.description`** (§8.8) |
| O-30 | `Retry-After`, `X-RateLimit-*` headers published | `info.description` | Match — **stronger than any other candidate's rate-limit contract** |
| O-31 | "Each endpoint includes example requests and responses" | `info.description` | **Response examples do not exist**: 0 `example` keywords, 75 request code samples (§8.11) |
| O-32 | Sandbox data is fake, generated, not real | sandbox tag | **Match — labelled and separately addressable; recorded as a positive** |
| O-33 | "Timestamps: All time-based data uses ISO 8601 format" | `info.description` | **Violated by `timestamp_unix` (ms)** (§8.4) |
| O-34 | "Durations: Provided in seconds unless otherwise specified" | `info.description` | Match |
| O-35 | Dates interpreted in the user's local timezone | FAQ | Documented; **the timezone is not reported** (§8.4) |
| O-36 | `LocalizedDateTime` / `UtcDateTime` distinguish local from UTC | schemas | **All four date/time schemas are bare `string`** (`ISODate`, `LocalDateTime`, `LocalizedDateTime`, `UtcDateTime`); the distinctions are not machine-checkable and `LocalDateTime` vs `LocalizedDateTime` is never explained (§8.4) |
| O-37 | Missing data should be rendered as gaps, not connected | FAQ | **Match — the package's strongest positive; same rule as `AMENDMENTS.md`:61** |
| O-38 | `PublicSample` carries `interval` and nullable `items` | schemas | Match — holes representable, not sentinels |
| O-39 | `average_heart_rate` differs from the app's value | field description | **Disclosed by the vendor** — recorded as a positive |
| O-40 | `app_sleep_phase_5_min` is app-aligned and will be removed | field description | Match — a disclosed parallel implementation |
| O-41 | `pulse_wave_velocity` "derived from vascular age, with possible offset added" | field description | **Offset magnitude not published** (§7.3) |
| O-42 | BDI is "calculated using detected SpO2 drops", range [0, 100] | field description | Match |
| O-43 | `stress_high` / `recovery_high` are top / bottom quartile | field descriptions | Match — and a positive for method disclosure |
| O-44 | `vo2_max`, `spo2_percentage.average`, `average_hrv` have units | fields | **No units published** (§7.2) |
| O-45 | `weight` and `height` have units | `PersonalInfoResponse` | **No units, no descriptions on any of the six properties** |
| O-46 | `PublicWorkout.activity` value domain | schema | **`string`, no enum, no example** |
| O-47 | Sleep data requires the user to open the app | two contract sections | Match across both |
| O-48 | Data types sync at different times | `info.description` | Match |
| O-49 | Oura stores measurement data only while the account is active | Privacy Policy | Match with the deletion-rights statement |
| O-50 | No sale of personal data; none sold in the preceding 12 months | Privacy Policy | Stated; not independently verifiable |
| O-51 | No sale for third-party LLM training | Privacy Policy | Match as stated; **scope is narrower than "no training"** |
| O-52 | "we do not share or sell your personal data. That includes third-party AI models" | Trust Center | **Conflicts with the Privacy Policy's named LLM providers** (§9.3) |
| O-53 | Health Radar is general wellness, not for monitoring | Health Radar footnote | Match with the Play listing's "monitor" exclusion |
| O-54 | "Oura Ring and Advisor are not medical devices" | Health Radar footnote | Consistent with the Play listing |
| O-55 | Fertile Window is CE-marked under MDR; **must not be relied on to prevent pregnancy** | Terms | Match — **recorded as a positive** |
| O-56 | Oura is not a Covered Entity under HIPAA for DTC services | Privacy Policy | Match with the three other scoped HIPAA statements |
| O-57 | "We also meet HIPAA requirements where we handle protected health information for a healthcare partner" | Trust Center | Consistent **with scope**; the DTC exclusion is on a different surface (§9.4) |
| O-58 | Bluetooth AES-128; TLS 1.2+ in transit; AES-256 at rest | Trust Center | **Match as a stated chain** — the most specific security claim published |
| O-59 | SOC 2 certification | Trust Center | **Type I / Type II not stated; no report published** |
| O-60 | HITRUST certification | Trust Center | **No level, scope or date** |
| O-61 | "deleted data is fully removed from your account" | Trust Center | **Conflicts with the policy's retention treatment; policy governs** (§10) |
| O-62 | 130+ publications, 14 collaborations, 12+ years | science page | Vendor-published |
| O-63 | "more than 50 MDs and PhDs" | validation article | Vendor-published |
| O-64 | "trained on … over 1,200 nights of clinical PSG data" | validation article | Vendor-published; no dataset published |
| O-65 | Inter-scorer agreement ~83% | validation article, guesswork article | **Stated twice, consistently — supports O-12** |
| O-66 | "Performance in older adults and in clinical populations" needs improvement | guesswork article | **Vendor-stated limitation — recorded as a positive** |
| O-67 | Independent 2025 study reports 53% four-stage agreement | guesswork article | **Reported by the vendor as the basis of litigation; Oura disputes it in the literature** (§11.9) |
| O-68 | Oura published a peer-reviewed letter to the editor (2025) | guesswork article | Stated; **the letter itself was not fetched** |
| O-69 | "A proposed class action filed against Oura last week" | guesswork article | Stated by the vendor; **no court record consulted** |
| O-70 | "Not intended for use in emergency or life-threatening situations" | Terms | Match with the absence of any alert path |
| O-71 | Population averages may not apply to a given user | Terms | **Vendor-stated limitation — recorded as a positive** |
| O-72 | "This is not a competitor" — no PAP/AHI/leak anywhere | product, API | **Match by complete schema inventory — a genuine V2 gap in V2's favour** |
| O-73 | No raw sensor stream exposed | Data Processing Timeline + schema inventory | Match |
| O-74 | Terms prohibit reverse engineering and defeating subscription restrictions | Terms §II | **Match — quoted as the Phase 2A authorization basis** |
| O-75 | Terms published 2026-10-02, effective 2026-11-04 | Terms | **Lifecycle fact — not yet in effect at the research date** |
| O-76 | Oura Enterprise Platform: GDPR, HIPAA PHI, de-identification, CSV/JSON export | B2B site | Stated; no artifact published to verify |
| O-77 | Counsel Health care is delivered by Counsel's physicians, not Oura | Health Radar page | Match — a clear boundary statement |
| O-78 | Counsel Health available in 43 US states | Health Radar page | Stated as an enumerated exclusion list; consistent internally |
| O-79 | Symptom Radar alerts "sometimes before symptoms … are even felt" | Health Radar page | Stated; **hedged, and no rate published** |
| O-80 | Four legal entities named across five surfaces | §3 table | **Directly observed** |
| O-81 | **Oura's public API reports sleep stages in five-minute blocks** | guesswork article | **Contradicted by the specification's `sleep_phase_30_sec`** (§8.6) |
| O-82 | Internal referential consistency of the specification | `openapi-1.41.json` | **Checked: 274 `$ref`s, 0 unresolved** — a positive |
| O-83 | `ClientIdAuth`/`ClientSecretAuth` referenced but undeclared | hypothesis | **REFUTED — both are declared and referenced; not recorded as a finding** |

*Added on the second verification pass (O-84…O-95). These rows were missed on the first pass and they change
this file's conclusion — the positive ones especially.*

| # | Claim or property | Surface | Verdict |
|---|---|---|---|
| O-84 | A deleted sleep period disappears from the API | `PublicSleepType` | **Does not match — `"deleted"` = "deleted sleep by user"; the record remains addressable as a tombstone** (§8.12). **Recorded as a major positive: the same rule as V2's append-only/tombstone requirement** |
| O-85 | A falsely detected sleep or nap is discarded | `PublicSleepType` | **Does not match — `"rest"` = "Falsely detected sleep / nap, rejected in confirm prompt by user"; the detection and the rejection reason are both retained** (§8.12). **Positive: a false-positive rate is computable downstream** |
| O-86 | Nothing identifies which algorithm produced a record | `PublicModifiedSleepModel` | **Does not match — `sleep_algorithm_version` ∈ {'v1','v2'}** (§8.12). **Positive; also the resolution for O-14** |
| O-87 | Staging is reported at one resolution | `sleep_phase_5_min` / `sleep_phase_30_sec` / blog | **Conflicts with the vendor's own blog** (§8.6) — **still not reconciled** |
| O-88 | A record's revision history is unobservable | `PublicSleepAnalysisReason` | **Does not match — `sleep_analysis_reason` ∈ {foreground_sleep_analysis, bedtime_edit, background_sleep_analysis, background_created_foreground_updated}** (§8.12) |
| O-89 | Third parties can write to a user's health record | API operation inventory | **Does not match — 19 `/v2/usercollection/*` roots are all GET; the only 4 write operations are webhook-subscription management** (§8.1). **Recorded as a boundary** |
| O-90 | Scores are always present when a daily record is | `PublicDailySleep` / `PublicDailyReadiness` / `PublicDailyActivity` `required` | **Does not match — `score` is nullable and not required in all three** (§7.1). **Positive** |
| O-91 | Contributors are always present | 4 contributor models | **Mostly does not match — 3 of the 4 declare no `required` list and every field is nullable; `ResilienceContributors` is the exception** (§7.1) |
| O-92 | A missed classification is representable | `movement_30_sec`, sleep-phase strings | **Does not match — no `'0'`, no null, no skip token**; but **`class_5_min` has `'0' = non wear`** (§8.5). **Intra-document inconsistency** |
| O-93 | Score range is 0-100 | global rule vs field descriptions | **Conflicts internally — global rule says 0-100; `PublicDailyActivity.score` and most contributors say `[1, 100]`; `ResilienceContributors` says `[0, 100]`; sleep and readiness scores state none** (§7.1) |
| O-94 | A webhook `expiration_time` is a unix timestamp | `WebhookSubscriptionModel` | **REFUTED — it is `type: string, format: date-time` (ISO 8601). An earlier draft of this file recorded it as unix seconds; corrected** (§8.4) |
| O-95 | `PublicDailyResilience` is the resilience schema | schema inventory | **REFUTED — the schema is `DailyResilienceModel`. An earlier draft named it wrongly; corrected** (§6, §7.2) |

**Summary of O-1…O-95.** Every row was classified, and **the rows recorded as *not matching or not
reconcilable* are enumerated in full rather than tallied** — the intermediate classes ("documented-only",
"partial", *carries a qualifier the headline does not*) admit judgement that a single number would
overstate. The rows that do not match or cannot be reconciled are **O-2, O-6, O-7, O-14, O-29, O-31,
O-33, O-36, O-52, O-61, O-81, O-84, O-85, O-86, O-88, O-89, O-90, O-91, O-92, O-93** — **20 rows** — of
which **five (O-84, O-85, O-86, O-88, O-89) are recorded as positives** for how Oura handles deletion,
falsified detections, algorithm provenance, revision reasons and write access. **O-19 is deliberately
excluded**: the ratings-versus-reviews difference is an unresolved *ambiguity of definition*, not a
surface conflict, and the package's rule is that such an ambiguity is not recorded as a discrepancy.
**Two hypotheses were
refuted** (O-83, O-94) and **one schema name was corrected** (O-95); a refuted hypothesis and a corrected
proof-reading error are **not findings**, and they appear here so that a reader can see the second pass
happened.

### 14.5 Artifact hashes (SHA-256)

Every artifact this file quotes, computed from the retrieved bytes.

| Artifact | SHA-256 |
|---|---|
| `openapi-1.41.json` (the published contract) | `a144a3436e9b73da568999fc713257e1d73e12cd3f7024eef45035c9d8f6b557` |
| `ouraring.com/` | `4a8690452c33e09506ef15f834137ea3f2a01ac220881914808ea2457fd86450` |
| `/membership` | `f74dc7a1422b4222e0a6f65da0907d91183c736e74ab1aedaa8196befa06f4d5` |
| `/privacy-policy` | `de77fff164ca9ac8bfc842b66fdc74e31922b383da241cc366048c773bfe9aca` |
| `/terms-and-conditions` | `891c51c238ac8104eb6e8a35007b26335d8ae07ef39fce1998851dd6056509b5` |
| `/security` | `41706e303aef9ff204ce3695466a532dd54aa1bd80eca322b52caa7c84818b2d` |
| `/science-and-research` | `388af90948153be8de72e8017c98586c7f8b3d216208aa6b8f4c5b5c46bd2662` |
| `/health-radar` | `ae42a191f253e0466f87fd70dc163de3967c0e4be57fbee87b46763bcf262abb` |
| `/blog/how-oura-measures-sleep-and-validates-accuracy/` | `1ad377d76f4b0db99cc9cec2a5521bee105771b97c85d263ff1c7f9f4851aae7` |
| `/blog/how-accurate-is-oura/` | `604f9e344176e3ea96082f21cd7f61f0879b350e42a09c851eec4e87e80cb074` |
| `/blog/new-sleep-staging-algorithm/` | `529bf111d0425bae72c478c6a6a206d83b658c001d49429fba432fd9d6bb0f8d` |
| `/blog/sleep-staging-is-not-guesswork/` | `51adcbf9eda140a130023cbc070079ac9eaf23f2798a19e76d62ea899bddf22b` |
| `/blog/` | `f3b679ba768c15a1bb1f78a40e5390b0e43520d3ef1cc754ae253b20e33c9430` |
| `/why-oura` | `264d1e242f685d95d35b81fa11fa84db5741f687f69613919ef1e9bcd14395cb` |
| Google Play listing (`com.ouraring.oura`) | `5f23b4ffc616f848258fe8ae64ac0118b577dbd55bea78f7a14016aa8c846673` |
| `organizations.ouraring.com/` | `25876b61d68294504d2af8b51231b35e5efc39969d18d05697cd884476ede602` |
| Apple lookup, `bundleId=com.ouraring.oura` | `a492b7cec36d17323e0f41e518b0223286d7e8f640ebc519c159fa2f77f07ec3` |
| Apple lookup, `bundleId=com.ouraring.ouraapp` (empty) | `7f5ce663b726607eae2fde2a6dc438052ac0d1681e636de7cc3377c0bb77e047` |
| `cloud.ouraring.com/v2/docs` (JS shell, 2,008 B) | `1029c1a654483a221c8bec090fcc57c9f1e72ca6bc9bbc125862bc71f8132567` |
| **Retrieval failures — error shell, not a 404** — `/science` | `472d2e26f9465ef9bb942d710a02c1d1f1f0a559fb2ff620bc8fdbb7b56e6068` |
| **Error shell, not a 404** — `/legal`, `/legal/privacy-policy`, `/legal/terms-of-service` | `a46d675d2b639278b4f11b9de89f6f14ec0e925a5aa093e9d15df70a0e2aff54` |
| **Error shell, not a 404** — `/for-organizations` (on `ouraring.com`) | `80e1d8d88efd608cae25f4ae0f57340fc44240115801dc2634b131068e5f3ca0` |
| **A readable 404 page** — `/blog/white-papers/` **and** `/blog/inside-the-ring-developing-ouras-latest-sleep-staging-algorithm/` | `f015cc7650b24f41fa49b41494e9a2d5db09c36a61fb31fef236594d7e4ab052` (byte-identical to each other) |

All three error-shell artifacts contain `<html id="__next_error__">` and approximately one character of
text; none contains a vendor-authored "not found" message. **The distinction is the finding:** an error
shell is a *retrieval failure*, a 404 page is a *vendor statement*. Only the latter can be cited as the
vendor saying a resource does not exist.

**The three shells are recorded as retrieval failures, not as 404 pages and not as evidence of absence.**
`/legal`, `/science` and `/for-organizations` on `ouraring.com` each return a Next.js **error shell** —
`<html id="__next_error__">` with roughly one character of rendered text, and **no readable
"Page not found" body**. `[UNKNOWN] across all three.` An earlier draft of this file described them as
**404s**, which was **wrong**: a 404 page is a readable statement by the vendor, and an error shell is a
failed retrieval. The correction matters because "the page does not exist" and *the content could not be
retrieved* support different conclusions, and only the second is supported by the bytes. The only genuine
**404** rendering in the set is `/blog/white-papers/`, which returns a readable `404 / Page not found`
page — and that page is **byte-identical** to `/blog/inside-the-ring-developing-ouras-latest-sleep-staging-algorithm/`
(`f015cc76…`), i.e. two different URLs render the same 404 document. The searched-for "white papers" page
does not exist at that path; whether Oura publishes white papers elsewhere is `[UNKNOWN]` (§14.6 G6).

**Reading the hash table correctly.** The three error-shell rows and the 404 row are **retrieval facts about
this research session**, not properties of Oura's website, and they are the only rows in the table that can
change between two fetches of the same URL when nothing about the content has changed.

### 14.6 Evidence gaps

- **G1 — No runtime behaviour of any kind.** No app, no account, no ring, no endpoint. Notification
  latency, sync reliability, the sandbox's actual output, the real response to a lapsed membership, and
  the behaviour of the app when the ring is off-finger are all unobserved. **This inspection cannot answer
  any of them**, and no hypothesis about them is recorded as a finding.
- **G2 — No endpoint was called, including the sandbox.** The sandbox exists specifically to make a lawful
  read possible without an account; **calling it would still be an explicit authorization decision under
  this package's rules**, and it was not taken.
- **G3 — The 2025 *Sleep Advances* letter was not fetched.** Oura states it exists and describes its
  argument. `[UNKNOWN]` whether it is peer-reviewed correspondence, its authorship, and whether the
  *Scientific Reports* authors replied. **The strongest counter-evidence in this candidate has a
  rebuttal this file does not read.**
- **G4 — The 2025 *Scientific Reports* study was not fetched.** Only Oura's characterisation of it is
  recorded. `[UNKNOWN]` the study's own stated limitations, its sample, and whether it acknowledges the
  version-drift objection.
- **G5 — No court record consulted.** The proposed class action is known only from the vendor's blog post
  about it.
- **G6 — White papers not located, and three pages could not be read at all.** `/blog/white-papers/`
  returns a **readable 404 page**, so that path does not exist. `/legal`, `/science` and
  `/for-organizations` each return a **JavaScript error shell with no readable body** — a *retrieval
  failure*, from which **nothing can be concluded**, not even that the page is absent. `[UNKNOWN]` whether
  Oura publishes validation white papers, legal notices or a science landing page at other paths. **An
  error shell is not evidence of absence**, and an earlier draft of this file reported these three as 404s;
  that was wrong and is corrected in §14.5.
- **G7 — No false-alarm, sensitivity or specificity rate is published for Symptom Radar, Blood Pressure
  Signals or Nighttime Breathing.** The closest published rates in the whole candidate set come from
  `empatica`'s 510(k) summary.
- **G8 — SOC 2 type and HITRUST level are not stated**, and neither report is published.
- **G9 — "Previous Terms and Conditions" was not fetched**, so the terms revision was not diffed.
- **G10 — Only the surfaces listed in §14.5 were read.** The blog index was fetched but its individual
  articles were not enumerated; the Oura Research App was identified and not inspected; the Oura
  Enterprise Platform documentation was not searched for; `cloud.ouraring.com/docs/authentication` was
  referenced by the specification and not fetched. **Nothing in this file is a complete account of Oura's
  claims.**
- **G11 — The `meal`/glucose surface was not resolved.** The Trust Center lists "Glucose Levels and
  Meals"; the webhook enum carries `meal`; no read path exists. `[UNKNOWN]` how a third party is expected
  to obtain meal or glucose data.
- **G12 — No counterpart field to `sleep_analysis_reason` was found in V2.** Oura publishes *why a record
  was created or revised*; V2's append-only `samples` table records *that* a reading was written and
  `cpap-ezshare-ingest` invariant (6) versions the **clock correction**, but no V2 field states a reason
  for a revision. `[UNKNOWN]` whether V2 has such a field on a path not read for this candidate; **this is
  recorded as a comparison gap to be resolved in Phase 2A of candidate 9, not as a V2 defect.**
- **G13 — The second verification pass was not a re-read of the whole contract.** It queried the fetched
  `openapi-1.41.json` for enums, `required` lists, nullable unions, bare-string schemas, time formats, and
  character-encoded fields — the classes where the first pass had already erred. **Nine new findings and
  five corrections came out of it (O-84…O-95)**, which means the first pass was not exhaustive and **a
  third class of field may still be unexamined.** `[UNKNOWN]` what else a targeted third pass would find.
  This gap is recorded rather than closed because the existence of the gap is itself the finding: **the
  first pass produced a materially more negative picture of Oura than the second one did.**

### 14.7 Not to be mistaken for a finding

- The r²/"accuracy" mismatch is recorded as a **naming and disclosure** finding. **No claim is made that
  the underlying measurements are wrong**, and r² = 0.996 against ECG is a strong result.
- The vendor-authored studies are recorded as an **evidence-classification** finding against the
  "Independently validated" heading. **No claim is made about the quality, methods or conclusions of
  those studies**, and the vendor discloses the authorship on the page it links.
- The 53% figure is recorded as **reported by the vendor as the basis of a legal claim Oura disputes**. It
  is **not** recorded as the true agreement rate, and Oura's methodological objections are quoted at
  length precisely so the reader can weigh them.
- The API being free while data access requires a membership is **recorded as two consistent vendor
  statements**, not as a bait-and-switch.
- The Terms being published ahead of their effective date is a **lifecycle fact**.
- `ClientIdAuth`/`ClientSecretAuth` was **checked and found correctly declared**; the hypothesis that it
  was a dangling reference is recorded as refuted and is not a finding.
- The Play "No data shared with third parties" badge is recorded as an **interpretation hazard**, with
  Google's definition explicitly **not** verified. It is **not** recorded as a contradiction.
- **The tombstone, retained false positive, algorithm version and mutation reason are recorded as
  positives, not as evidence that Oura's data model is equivalent to V2's.** Oura publishes *values* in a
  contract; V2 *enforces* a schema and a storage rule (`No UPDATE, no DELETE against samples`). A value can
  be omitted by a future release; a denied SQL statement cannot. The distinction is stated in §8.12 and is
  not to be collapsed into "Oura does the same thing".
- **The `class_5_min` non-wear code is recorded as a correct pattern in one field**, not as evidence that
  the API's missingness model is coherent — the sleep-phase and movement strings in the same document
  cannot represent the same condition (§8.5).

### 14.8 What a second verification pass changed, and why it is recorded here

This file was written, and then **every hash and every quantitative or structural claim in it was
re-checked against the retrieved bytes** in a second pass. That pass produced **nine new findings and five
corrections** (ledger rows O-84…O-95), and the directions were not symmetric:

| Direction | Effect |
|---|---|
| **Corrections against this file's accuracy** | `expiration_time` was mis-recorded as a unix timestamp (it is ISO 8601); the resilience schema was mis-named (`PublicDailyResilience` → `DailyResilienceModel`); three pages were mis-described as 404s (they are error shells); two bare-string timestamp types became **four**; "two violations" of the ISO 8601 rule became **one** |
| **New findings against Oura** | The score-range conflict (0-100 / `[1, 100]` / `[0, 100]` / unstated); the three disagreeing digit-string alphabets; the near-duplicate temperature pair |
| **New findings in Oura's favour** | Tombstone deletion; retained false positives with a named rejection reason; per-record algorithm version; mutation reason; read-only-for-health-data API; nullable scores and contributors; `low_battery_alert` on the biological record |

**The asymmetry is the finding, and it is recorded against this research process rather than against the
vendor:** the first pass produced a **materially more negative** picture of Oura than the second one did,
and the material it missed was material it had already fetched. A verification pass should be expected to
*firm up* a draft, not to **reverse five of its verdicts and soften a sixth**. The standing consequence,
recorded in `../RESEARCH_METHOD.md`, is that **the field-class sweep is part of the first pass, not an
audit of it** — enums, `required` lists, nullable unions, bare-type schemas, time formats and
character-encoded fields are to be enumerated **before** a conclusions section is written.

No claim in this file was left standing on a value that failed re-computation, and no hash in §14.5 is a
placeholder: **all 23 table rows were computed from the retrieved bytes** on the same day, and the rows
that could not be verified as *content* are labelled as retrieval failures rather than as pages.

### 14.9 The quote-verification pass — nine inexact quotations found and corrected

A third check was run before commit: **every quoted span in this file and in
`oura_IMPLEMENTATION_INSPECTION.md` was machine-compared against the text of the retrieved artifacts**,
with the specification compared as parsed JSON string leaves (so its `\u`-escaped text could not silently
fail to match) and the HTML compared both raw and tag-stripped. It found **nine quotations that were not
verbatim** and they are corrected in place:

| # | Was quoted as | The vendor's actual words |
|---|---|---|
| 1 | `(ii) reverse engineer, disassemble, decompile, decode, or adapt` | "(ii) reverse engineer, disassemble, decompile, decode, **adapt, or otherwise attempt to derive or gain access to any Oura Apps component** of the Services, in whole or in part" |
| 2 | `(ix) circumvent, bypass, remove, deactivate, impair or otherwise defeat … or other protections` | "(ix) circumvent, bypass, **disable, degrade, or otherwise interfere with** any access controls … or other **technical or contractual restrictions**" |
| 3 | `(vi) probe, scan or test the vulnerability of the Services or any system or network` | "(vi) **attempt to** probe, scan, or test the vulnerability of any **ŌURA system or network or to breach security or authentication measures without express authorization**" |
| 4 | `most commercial wrist-based wearables limited to 60-65% agreement` | "most commercial wrist-based wearables **are** limited to 60-65% agreement" |
| 5 | `the latest sleep staging algorithm, released in 2023` | "**Oura's** latest sleep staging algorithm , released in 2023" |
| 6 | `we meet HIPAA requirements where we handle PHI for a healthcare partner` | "**We also** meet HIPAA requirements where we handle **protected health information** for a healthcare partner" |
| 7 | `lead study author … was Oura's Chief Scientist at the time` | "**Hannu Kinnunen,** lead study author **and** Oura's Chief Scientist at the time" |
| 8 | `IDs: follow a consistent format` | "**IDs**: **Unique identifiers for specific data objects** follow a consistent format" |
| 9 | `Terms, Regulated Feature: Fertile Window (cited as a section title)` | The Terms contain **no such string**; the section is titled "**Regulated Features**" and the feature is the "**Fertile Window**" *within Cycle Insights* |

The first column records what this package itself previously wrote, in code spans, because those words are
**not** the vendor's and the table's whole purpose is to keep them out of quotation marks.

**Items 1–3 are the serious ones.** They are quotations of a **contract clause cited as the authorization
basis for the Phase 2A decision**, and item 2 is worse than a truncation: it invented a verb sequence
(`remove, deactivate, impair or otherwise defeat`) that appears nowhere in the Terms. A paraphrase wearing
quotation marks is the specific failure mode the rest of this package is built to avoid, and it survived
the first two passes because both passes checked **values** (hashes, counts, field types) and neither
checked **words**.

**The standing remedy, now in `../RESEARCH_METHOD.md`:** a quote-verification sweep is a **separate,
required** step before commit, performed against the fetched bytes with the specification parsed as JSON —
because the artifact that supplied three of these nine quotes was a JSON document whose escapes defeat
naive text matching. **Nine of nine were found by machine comparison; none was found by re-reading.**

**Retrospective sweep (recorded 2026-10-06).** The package-wide quote-verification sweep re-ran the check
against every oura deliverable. Its first pass reported **29 non-verifying fragments** across the four
files. **Seven of them were a defect in the checker, not in this file**, and fixing it resolved them
outright: an OpenAPI description is prose stored with `\n` and `\uXXXX` escapes, the check decoded those
escapes for `.js` but not for `.json`, and so a vendor sentence spanning two JSON lines could never match.
**A quote checker's corpus and its escape handling are part of the evidence, not a convenience.**

The remaining **22** fragments are counted and classified here, and every one is accounted for:

| Class | Count | Treatment |
|---|---|---|
| Inexact quotation of a vendor surface | **5** | Corrected in place (membership sentence; blood-pressure trigger; Health Radar footnote; sleep-detection figure; the "wearables **are** limited" line) |
| This package's own paraphrase wearing quotation marks | **9** | Converted to italics or plain prose |
| This package's own superseded wording, quoted as part of a correction record | **8** | Left verbatim but moved out of quotation marks into code spans (§14.9) |

The five corrected quotations are: the membership sentence ("your Oura Ring and Oura App will still
function…" — the earlier text said "the ring and app"); the blood-pressure trigger (the page says "appears
only when Health Radar identifies patterns that may indicate cardiovascular strain"); the Health Radar
footnote ("**not intended** to diagnose, treat, cure, mitigate, monitor, or prevent any disease, illness, or
medical condition" — the earlier text used a bracketed insertion and dropped the objects); the sleep
detection figure ("the ring correctly detected sleep 94–98% of the time, with overall sleep/wake agreement
of roughly 90–96%"); and "wrist-based wearables **are** limited to 60-65% agreement".

The eight superseded-wording records are the rows of the §14.9 table together with the verb sequence in the
prose beneath it; that table's first column is now rendered in code spans precisely because it is **not**
the vendor's text and must not look like a quotation.

---

## 15. Weaknesses, omissions and unverified scope

**In Oura's published material, as observed:**

1. Two headline accuracy figures use **r²** as "accuracy".
2. Two of the figures' source studies are **vendor-authored** and presented under "Independently validated
   sensors"; the vendor demonstrably knows the distinction (it uses it in a different article).
3. A **peer-reviewed advisory relationship** (Medical Advisors) is adjacent to quoted validation without
   disclosure at the point of citation, on two pages.
4. The validation article's citation list contains a **2021 study offered as evaluating a 2023 algorithm**.
5. The vendor's blog says the API reports sleep stages in **five-minute blocks** while the specification
   publishes a **30-second** field.
6. **Six unit-less fields**, an internal **score-range conflict** (0-100 vs `[1, 100]` vs `[0, 100]` vs
   unstated), and a **near-duplicate temperature pair** whose difference is never described, in a contract
   that is otherwise detailed enough to publish `x-owner` squad names.
7. **Four timestamp types that are the same type**, with machine-invisible semantic differences, on the
   field that determines when a measurement happened.
8. **Three time encodings**, one of them (`timestamp_unix`, ms) violating the document's own global ISO
   8601 rule.
9. **Three digit-string encodings with three disagreeing alphabets**, in which non-wear is representable
   in `class_5_min` (`'0'`) and **not** in the sleep-phase or movement strings.
10. A **flat rate-limit figure** in the FAQ against a **two-layer model** with unpublished ceilings.
11. **"Each endpoint includes example requests and responses"** against zero response examples.
12. **Three data types with no webhook** and **one webhook type with no read path**.
13. A webhook path documented as **lossy** with no at-least-once guarantee and no backoff schedule.
14. **No replay window** documented for webhook signatures and a **reference implementation that does not
    check the timestamp**.
15. A **verification procedure that requires re-serialising a parsed body** without saying the raw body
    must be used.
16. **Subscription expiry with no documented duration and no warning.**
17. **SOC 2** without a type and **HITRUST** without a level or scope; neither report published.
18. **Policy-versus-Trust-Center conflicts** on third-party AI and on deletion.
19. An **ILLNESS DETECTION** heading above a "not intended to … monitor" disclaimer on the same page.
20. **"over 30 biometrics"** against **"over 50 health metrics"**.
21. **Four legal entities** across five surfaces with no cross-reference.
22. **A published claim of assistance that no published artifact implements**: the Quick Start promises
   "example requests and responses" and the contract carries 75 request samples and **zero** response
   examples.
23. **A rate limit stated as one flat number** in the FAQ beside a two-layer model whose ceilings are not
   published at all.

**In Oura's published material, recorded as strengths — added on the second verification pass, because the
first pass understated them:**

24. **Deletion is a tombstone, not a row removal** (`PublicSleepType` = `"deleted"`) — V2's append-only
   rule, arrived at independently (§8.12).
25. **A rejected false-positive detection is retained with its rejection reason** (`"rest"` = "Falsely
   detected sleep / nap, rejected in confirm prompt by user") — V2's "reject it, name the reason,
   quarantine it", arrived at independently, and it makes a downstream false-positive rate computable.
26. **Per-record algorithm provenance** (`sleep_algorithm_version` ∈ {v1, v2}) and **mutation provenance**
   (`sleep_analysis_reason`, four enumerated reasons).
27. **The API is read-only for health data** — 19 GET roots, 4 write operations all for webhook subscriptions.
28. **A headline score can be null while its record exists** (score nullable and not `required` in all three
   daily models), and **contributors are nullable with no `required` list** in three of the four contributor
   models — absent, not zero.
29. **`class_5_min` encodes non-wear as `'0'`** — the correct treatment of the exact problem the other two
   digit-string encodings leave unsolved.
30. **A dead field is documented as dead** (`ring_id`: "Deprecated, returns null."), so the trap is disarmed
   in the description rather than left to be discovered.

**In AnxietyWatch2, corrected against the tree because this candidate makes the gaps visible:**

31. **V2's missingness rules are enforced in the storage path but not stated as an integrator contract.**
    Oura publishes "displaying gaps in visualizations rather than connecting across missing data" as guidance to third parties.
    V2 has no equivalent published integration guidance, because it has no published integration surface —
    which is the point of the exercise and is recorded as such, not as a defect.
32. **`events.rs` produces `EventKind` values that nothing consumes.** `MAX_EVENTS = 16` at `events.rs:8`,
    the `EventKind` enum at `:19–51`, and the only occurrence of the string "AHI" in the Rust tree is a
    `Rera` doc comment. **Oura is the second candidate in the package to make this visible** (after
    `oscar`): a respiratory-event vocabulary exists with no downstream reader. Oura's position is the
    inverse — **no event vocabulary at all** — and neither is a working event surface.
33. **V2's three-state rule has no published counterpart.** Oura publishes `PublicSleepTimeStatus` with
    three "not measured"-flavoured values and no equivalent for any physiological signal. V2's registry
    carries per-type valid ranges; the API exposes a `breathing_disturbance_index` range and no others.
    **Partially corrected in Oura's favour on the second pass:** Oura also publishes a **nullable score**, a
    **nullable contributor decomposition** and a **non-wear code**, which are three-state handling expressed
    structurally rather than as a status enum (§8.5, §8.12).
34. **No V2 comparison is offered on alarm rates or false alarms**, because Oura publishes none for its
    predictive surfaces. §14.6 G7 records this as Oura's gap, and `empatica`'s FAR figures remain the only
    published error rates in the package.

---

## Related files

- `../CANDIDATE_SHORTLIST.md` — why Oura is candidate 8 (`C16`)
- `../RESEARCH_METHOD.md` — evidence classes, authorization rules, standing sub-checks
- `../PROCESS_STATE.md` — candidate status and residual uncertainty
- `oura_IMPLEMENTATION_INSPECTION.md` — Phase 2A: what was inspected, and what was not
- `PROMPT_2.md` — the candidate-specific second-stage brief
- `oura_COMPARISON.md` — the comparison report
