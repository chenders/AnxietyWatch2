# Visible — claimed features and purpose

**Candidate:** C11 — Visible (makevisible.com), Visible Health Inc.
**Phase:** 1 (public-claims research). Phase 2A (app-binary inspection) is **not performed** — see
`visible_IMPLEMENTATION_INSPECTION.md` §1.
**Research date:** 2026-10-06/07 (all "as of" statements refer to this date).
**Method:** `../RESEARCH_METHOD.md`, brief `../PROMPT1.md`, sequencing `../PROCESS_STATE.md`.

Every claim below carries an evidence class. A **claim** is what the vendor or another publisher says.
A **finding** is what an artifact demonstrably contains. The two are never merged.

| Class | Meaning in this file |
|---|---|
| `[PUBLIC CLAIM]` | Vendor marketing, store listing, or price page statement |
| `[DOCUMENTED]` | Vendor-authored help-centre article, privacy policy, terms, or specification |
| `[FOUND IN PUBLISHED CLIENT]` | String served to the browser by the vendor (HTML/JS), static |
| `[PUBLISHED RESEARCH]` | Peer-reviewed publication or its corrigendum |
| `[REGISTRY]` | Independent clinical-trial registry record |
| `[STATIC VENDOR ARTIFACT]` | Vendor GitHub organisation/repository metadata |
| `[OBSERVATION]` | Measured property of a retrieved artifact (HTTP status, byte length, digest) |
| `[INFERRED]` | Reasoning from observations, explicitly flagged |
| `[UNKNOWN]` | Not established from any retrieved artifact |
| `[ABSENT FROM CORPUS]` | Searched, not found. **Absence of a string is not proof of absence of a capability** |

Retrieved-artifact digests (full SHA-256) are in §20.

---

## 1. Executive summary

Visible is a **commercial, cloud-based, subscription symptom-and-pacing app** for energy-limiting
illness (Long Covid, ME/CFS, POTS, fibromyalgia, EDS), sold by Visible Health Inc. with a UK
subsidiary. It ships in two tiers: a free app that derives a daily **Stability score** from a 60-second
**phone-camera PPG** reading, and a paid **Membership** that pairs a Polar-manufactured **Visible
Band 2.0** for all-day heart rate, **PacePoints** energy budgeting, and real-time pacing notifications.

Its central and most instructive property, for this project, is that it is the **nearest published
analogue to AnxietyWatch2's stated posture** while differing from it at almost every architectural
axis. Visible draws the same three boundaries V2 draws — it says it is *not a medical device*, it
separates a *derived score* from the *stored signal*, and it has an explicit rule for a missing
measurement. Its answers to the questions V2 answers by construction are different:

- **Missing signal.** Visible's rule is *withhold the derived score*: "If you choose to get your
  Stability Score from overnight readings and forget to wear your band to bed, you will not receive a
  Stability Score that day." `[DOCUMENTED]` This is a genuine third state at the score layer — the
  closest published equivalent the package has found to V2's "a missing signal holds state; it never
  reassures and never escalates". It is not carried through to Visible's clinician surface, which
  "only displays sections when there is information to share" `[DOCUMENTED]` — i.e. missing data is
  **omitted**, not shown as unknown.
- **Derived value versus stored signal.** Visible **exports the derived score and withholds the raw
  signal**: "the data export does not include your live continuous heart rate data or Heart Rate
  Zones" `[DOCUMENTED]`. That is the inverse of V2's posture, where the decoder-level signal is the
  record of record and presentation is downstream.
- **Evidence class.** Visible's paid features (PacePoints, real-time alerts) rest on all-day
  continuous monitoring. The vendor's flagship peer-reviewed paper validates the **once-daily morning
  reading**, not the continuous path, and its own blog says continuous monitoring is what comes
  "Next". `[PUBLISHED RESEARCH]` `[PUBLIC CLAIM]` No published artifact ties the validated algorithm
  version to the shipped one.

Three further findings are recordable without any inference: a **four-way age contradiction** across
the Terms, the help centre and the vendor's own paper; an **absolute help-centre privacy sentence**
contradicted by the vendor's own privacy policy and its own Play Data Safety declaration; and an
**algorithm change shipped 2026-09-23 with no version identifier**, six months after the validation
that the vendor's marketing rests on, despite the vendor's own documented practice of labelling
Stability versions v1.1 and v1.2.

**Purpose of the product, in the vendor's own words:** "Get more from the energy you have. Pacing
(balancing activity and rest) is one of the best ways to live well with your condition and reduce the
risk of crashes or PEM." `[PUBLIC CLAIM]`

---

## 2. Entity, naming and availability

| Item | Value | Class |
|---|---|---|
| Product name | "Visible: Pacing for illness" (App Store title) | `[PUBLIC CLAIM]` |
| Operator | "Visible, a trading name of Visible Health Inc., a company registered at 251 Little Falls Drive, Wilmington, Delaware 19808 USA." | `[DOCUMENTED]` |
| UK subsidiary | "In the UK, we operate through Visible Health UK Ltd, a wholly owned subsidiary company of Visible Health Inc. registered at 7 Savoy Court, London, United Kingdom, WC2R 0EX." | `[DOCUMENTED]` |
| Support contact | `info@makevisible.com`; Play developer contact lists "+1 415-685-3058" | `[PUBLIC CLAIM]` |
| iOS bundle | `com.makevisible.visible`, track id 1624474919, seller "Visible Health Inc.", free, v26.9.3, first released 2022-08-29, minimum iOS 15.0, 160,522,240 bytes, `contentAdvisoryRating` "12+" | `[PUBLIC CLAIM]` |
| Android package | `com.makevisible.visible`, developer "Visible Health Inc.", **100K+ Downloads**, Play content rating "Everyone" | `[PUBLIC CLAIM]` |
| Geographic limit | "Pair the Visible band for real-time pacing notifications, PacePoints, all-day energy budgeting, and more. Only available in US and UK." (store listing, under the heading "GET ALL DAY DATA") | `[PUBLIC CLAIM]` |
| Membership markets | US and UK pricing columns only; no other market appears on the price page | `[OBSERVATION]` |
| Site | `https://www.makevisible.com` (bare host redirects); `sitemap.xml` lists 73 URLs | `[OBSERVATION]` |
| Vendor GitHub org | `github.com/makevisible`, created 2022-01-11, description "Making invisible illness visible.", location United Kingdom | `[STATIC VENDOR ARTIFACT]` |
| Help centre | `help.makevisible.com`, Intercom-hosted, 95 articles reachable from `sitemap.xml` | `[OBSERVATION]` |

**Distinct audience counts, never to be averaged.** The vendor publishes at least four member/scale
figures on different surfaces and dates: "Join over 250,000+ people with Long Covid, ME/CFS, POTS,
Fibromyalgia, chronic fatigue and more who are improving their pacing with Visible."
`[PUBLIC CLAIM]` (store listing, both stores); Play "100K+ Downloads" `[PUBLIC CLAIM]`; the study blog
"5.5M data points" (2023) and "58M data points" (2023-11) `[PUBLIC CLAIM]`; and the paper's `n = 4244`
research cohort with "an average of 125 biometric observations per participant" `[PUBLISHED RESEARCH]`.
A store-install counter, a marketing member count and a consented research cohort are **three
different quantities measured three different ways**; the package records them side by side and
performs no arithmetic on them.

### 2.1 Documentation lifecycle facts

| Surface | Date carried by the artifact | Class |
|---|---|---|
| Terms of Use | "Last updated: 14th July 2022" | `[DOCUMENTED]` |
| Privacy Policy | "Last updated: 27 Feb 2025"; footer "© 2025 Visible Health Inc" | `[DOCUMENTED]` |
| Help article "What happens to my data?" | "Oct 1, 2025" | `[DOCUMENTED]` |
| Help article "How is my data stored?" | "Nov 4, 2025" | `[DOCUMENTED]` |
| Help article "Age requirements for Visible" | 2025-10-01 (per §11) | `[DOCUMENTED]` |
| Stability-algorithm change | blog "An even more accurate Stability Score", 2026-09-23 | `[PUBLIC CLAIM]` |
| Flagship publication | npj Digital Medicine, published 2026-03-24, version of record 2026-03-27 | `[PUBLISHED RESEARCH]` |

The Terms are ~4¼ years old at the research date while the product has shipped an all-day wearable
tier, a clinician dashboard and an overnight measurement mode in the interim. Recorded as a
**lifecycle fact**, not a compliance finding: the vendor has published newer privacy documentation
(2025) alongside older terms (2022).

---

## 3. Purpose and target user, as claimed

- "Do you live with an energy-limiting health condition?" `[PUBLIC CLAIM]`
- "Visible is designed for people living with Long Covid, ME/CFS, POTS, fibromyalgia and
  Ehlers-Danlos syndrome (EDS). It supports pacing, a widely recommended approach for these
  conditions." `[PUBLIC CLAIM]`
- "Unlike fitness tracking apps, Visible is designed to support rest and pacing, not workouts and
  exercise." `[PUBLIC CLAIM]` (App Store and Play descriptions, verbatim, both stores)
- "Pacing (balancing activity and rest) is one of the best ways to live well with your condition and
  reduce the risk of crashes or PEM." `[PUBLIC CLAIM]`
- "Get more from the energy you have." `[PUBLIC CLAIM]`
- Play short description: "Activity and HRV tracking for Long Covid, ME/CFS, POTS, Fibro & more"
  `[PUBLIC CLAIM]`
- Positioning against wearables: "Can I use my Apple Watch, Garmin or Fitbit with Visible? …
  Most wearables, like Apple Watch, Garmin and Fitbit, are designed with fitness in mind… Visible is
  designed for people living with…" `[DOCUMENTED]`
- Business model: "At Visible, we are different from fitness tracker companies like Apple, Garmin and
  Fitbit. We don't make money by selling devices for a profit. Instead, we make hardware devices
  available to you as cheaply as we possibly can. Because we don't make money from selling armbands,
  it's your membership fee that powers everything we do." `[PUBLIC CLAIM]` — recorded as
  vendor-published. Hardware margin is **`[UNKNOWN]`**; nothing public verifies or refutes it.

The purpose is **chronic-illness energy management with paced activity**, not anxiety measurement,
not sleep-apnea treatment and not respiratory monitoring. That distinction governs every comparison
in `visible_COMPARISON.md`.

---

## 4. User-facing workflows (as documented)

1. **Morning check-in (free tier, no band).** 60-second phone-camera reading of heart rate and HRV,
   plus self-reported sleep and symptoms; produces the day's Stability score.
2. **Evening check-in.** Symptom scoring once a day, with a stated design rationale (§7).
3. **All-day measurement (paid, band).** Continuous heart rate in zones; PacePoints accrual;
   real-time pacing notifications.
4. **Overnight mode (band).** Stability computed from band readings during sleep instead of a
   morning reading (§6).
5. **Tagging and notes.** "Log your symptoms, medication, and daily exertion to spot patterns over
   time." `[PUBLIC CLAIM]`
6. **Health Report PDF.** Long-term trend report for a clinician.
7. **Clinician Dashboard.** Revocable live link so a clinician can see current data.
8. **CSV export.** Structured export of selected data classes.
9. **Research participation.** Per-study opt-in from within the app.

Sources: store descriptions, `/how-it-works`, and the help-centre articles cited per item below.
Workflow *mechanics* (what the screens do) are documented; **no screen was run** — see §19.

---

## 5. Data sources and supported hardware

| Source | Documented capability | Class |
|---|---|---|
| Phone camera (PPG) | "Use your phone's camera to measure your biometrics, including heart rate variability (HRV) and resting Heart Rate each morning." | `[PUBLIC CLAIM]` |
| Visible Band 2.0 | Screenless, waterproof, 5-day battery, 2-hour charge, wrist fit 6.1–8 in; "manufactured by Polar, a world leader in heart-rate monitoring since 1977." | `[PUBLIC CLAIM]` |
| Polar Verity Sense | Previous-generation band, a retail Polar product, supported and upgradable to Band 2.0 | `[DOCUMENTED]` |
| Apple Watch / Garmin / Fitbit | **Not supported.** "We don't currently support HRV data from Apple Health or Garmin and this is something a lot of people have asked for." | `[DOCUMENTED]` |
| Apple Health / Health Connect | **No import or export documented.** Neither string appears in the 95-article help corpus or the marketing site | `[ABSENT FROM CORPUS]` |
| CPAP / respiratory therapy devices | **No support documented.** "CPAP", "apnoea", "sleep apnea", "SpO2", "oxygen", "sleep staging" do not appear in the retrieved corpus | `[ABSENT FROM CORPUS]` |
| Weather / aircraft / noise sensors | **No ingestion documented.** Temperature and weather appear only as *user-interpretable confounders* and as a *research-study subject* (§5.2) | `[ABSENT FROM CORPUS]` |
| Public API / webhooks | **None documented.** No developer API, no webhook, no documented third-party integration endpoint | `[ABSENT FROM CORPUS]` |

The hardware being a **Polar OEM device** is directly relevant to V2's own Polar support: the
population-scale pacing product in this space reaches consumers through a Polar armband, and states
so on its own page.

### 5.1 The closed hardware ecosystem is a stated product decision

The FAQ answers the interoperability question with a refusal, and the refusal is the strategy: a
fitness-first wearable is rejected as the wrong sensor model for pacing. Recorded as a **stated
boundary**, not a deficiency. Its consequence for a user is that Visible data cannot be
cross-checked against, or fed by, any device the user already owns unless it is a Visible band or a
Polar Verity Sense.

### 5.2 Environmental and atmospheric context — the closest published analogue to V2's weather work

Two `[DOCUMENTED]` surfaces mention environment:

- **PacePoints variation list** advises the user to attribute part of a day's change to environment:
  "You are sensitive to changes in the temperature, e.g., there has been recent hot weather."
- **A research study the vendor is enrolling for**, listed on `/research` under "Effect of atmospheric
  conditions on complex chronic illness", attributed to "David Putrino Professor of Rehabilitation and
  Human Performance": "Many people notice their symptoms change with the weather. This study helps us
  investigate how environmental conditions and symptom patterns relate to one another." — with an
  "Enroll now" action.

So the vendor **documents the environmental confounder but does not measure it**, and is *at the
research date recruiting* to study it. Visible ingests no atmospheric data itself. This is the
sharpest single-point comparison to AnxietyWatch2's specified weather/aircraft/environmental-noise
context ingestion: the same hypothesis, one product leaving it to the user's judgement and a
research study, the other specified as an ingested, stored signal.

---

## 6. Measurement, derived scores and their semantics

### 6.1 Stability score

| Property | Vendor's statement | Class |
|---|---|---|
| Scale | Presented as a 1–5 scale; "Higher score (4–5)… Lower score (1–3)…" | `[DOCUMENTED]` |
| Inputs | Combines resting heart rate, HRV, and self-reported sleep and symptoms | `[DOCUMENTED]` |
| Minimum data | "4 days" of data before the first score | `[DOCUMENTED]` |
| Baseline window | Up to "90 days" rolling | `[DOCUMENTED]` |
| Meaning | Explicitly distinguished from wellness: a *stability* score, not a wellness score | `[DOCUMENTED]` |
| Honest boundary | "A stable score doesn't guarantee you'll feel good, and an unstable score doesn't necessarily mean something is wrong." | `[DOCUMENTED]` |
| Missing-signal rule | "If you choose to get your Stability Score from overnight readings and forget to wear your band to bed, you will not receive a Stability Score that day. There is no option to take a morning reading instead." | `[DOCUMENTED]` |
| Modality switching | "It's not possible to accurately switch between the two [modalities]." Switching modality **creates a new baseline** | `[DOCUMENTED]` |

The vendor publishes a scale, a minimum-data rule, a baseline window, and an explicit statement of
what the score does *not* mean. That is a more disciplined score contract than most consumer
products in this package, and it is the reason Visible is a useful comparator rather than a foil.

The **missing-signal sentence** is the single most package-relevant finding here. It is a **positive
comparator** to V2's three-state rule: the vendor refuses to produce the derived value when the input
is absent. Two limits are as important as the positive: the rule applies to the **score**, not to the
underlying record; and the clinician surface does **not** show a third state (§9 of the comparison).

### 6.2 PacePoints

| Property | Vendor's statement | Class |
|---|---|---|
| Derivation | From time spent in heart-rate zones, weighted by an exponential curve, accruing roughly four times faster in the Exertion zone | `[DOCUMENTED]` |
| Budget | A daily budget, available after 4 days of data | `[DOCUMENTED]` |
| Alerting | **PaceSetter** notifications when pacing ahead of budget | `[DOCUMENTED]` |
| Day boundary | PaceSetter day end adjustable between 6pm and 4am | `[DOCUMENTED]` |
| Design rationale | A bounded daily energy budget rather than an activity target | `[DOCUMENTED]` |

### 6.3 Heart-rate-monitored pacing, and its citation

Visible's pacing-alert feature cites external clinical guidance and then disclaims equivalence to it:
the help centre references the Workwell Foundation heart-rate monitoring sheet "(With the caveat that
we don't use the same zones and thresholds as the Workwell Foundation)". `[DOCUMENTED]`

This is a **correctly-bounded citation** — the vendor names the external source *and* states that its
own thresholds differ. It is recorded as a positive comparator: V2's own discipline is to refuse to
claim a clinical basis it does not have.

### 6.4 Uncited physiological claims

Two `[DOCUMENTED]` statements carry no citation:

- "Studies suggest the early hours of sleep are more sensitive to reflect recovery…" — no study named.
- "Increasingly, researchers are questioning the idea of a 'good' heart rate variability (HRV)." — no source named.

Recorded as **uncited claims**, not as errors. The second is notable because it argues *against* a
simplification the vendor itself uses elsewhere; the vendor is willing to publish a caveat about its
own headline input.

---

## 7. Symptom, medication and subjective input

- **Cadence.** Symptom check-in is once daily. The vendor states the design reason: "Focusing on your
  symptoms multiple times a day can sometimes lead to unnecessary stress or worry." `[DOCUMENTED]`
  This is an **anti-anxiety rationale for a measurement-frequency decision** — directly comparable to
  AnxietyWatch2's own tension between continuous monitoring and anxiety amplification, and the first
  time in this package a competitor has published that trade-off as a product decision.
- **Evening → next morning coupling.** "Your evening symptom scores directly inform your Stability
  Score the next morning, which is designed to help you start your day with a clear understanding of
  how stable your heart rate variability (HRV), resting heart rate, sleep, and symptoms are." `[DOCUMENTED]`
- **Scale.** A 0–3 symptom severity scale.
- **Medication.** Medication can be logged; the help centre has a dedicated article on heart-rate-
  altering medications (§8).
- **Experience trackers.** The Health Report includes periods but not other experience trackers — a
  stated partial coverage, not a silent omission. `[DOCUMENTED]`
- **Functional capacity.** FUNCAP answers are collected; the export includes the answers but "not
  [the] final average" `[DOCUMENTED]` — one derived value deliberately withheld from the export while
  its inputs are included.

---

## 8. The medication confounder, and a desensitised alert

The help article "Using Visible with heart rate-altering medications" documents a confounder that
**degrades the paid alert path**:

- With beta-blockers, "you won't generate as many PacePoints or receive exertion notifications, even
  if it feels like you are exerting yourself."
- In the other direction, "you might generate PacePoints even when at complete rest".

The documented mitigation is a **manual user action** — the user is told to retune heart-rate zones.
`[DOCUMENTED]`

This is recorded verbatim and left unresolved. It is not a defect claim: the vendor names the
mechanism, names the consequence and offers a remedy. It *is* a documented case of a **real-time
safety-relevant alert silently desensitising** for a common drug class, with detection delegated to
the user's own awareness. AnxietyWatch2's CNS alarm path is specified to hold state rather than
reassure when a signal is untrustworthy; Visible's equivalent path degrades toward *under-alerting*
and says so in a help article.

---

## 9. Real-time, overnight and failure behaviour

**Real-time alerting (paid).** Two notification types: **Exertion** (crossing into the Exertion zone)
and **PaceSetter** (pacing ahead of the day's budget). "Get a phone notification when you are
exerting or pacing ahead for the day." `[DOCUMENTED]` The band is the sensor; the phone is the alert
surface.

**Overnight measurement (paid).** A 4-hour window beginning 30 minutes after sleep onset; moving
periods are ignored; good-signal readings are averaged. The vendor states a mechanism for the
measurement window but publishes no signal-quality threshold with a number. `[DOCUMENTED]`

**Connectivity as a stated dependency of measurement.** "If you don't [have a strong and stable
internet connection], the heart rate reading may not start or complete." `[DOCUMENTED]` This is the
vendor's own statement that a *measurement* — not merely a sync — is gated on network availability.
It is recorded as a **documented dependency**, not as an observed failure: the package ran no
network-condition test. It is nevertheless the clearest published contrast with V2's offline-capable
spool-and-sync design.

**Device coverage limit, stated by the vendor.** "We have not been able to test Visible with every
smartphone (there are thousands of them!)". `[DOCUMENTED]`

**Offline mode.** No offline operation is documented anywhere in the retrieved corpus.
`[ABSENT FROM CORPUS]`

---

## 10. AI, modelling and personalisation

The vendor describes its own model as AI:

- "Over the past year, we've been diving into this data to develop a cutting-edge AI algorithm that
  predicts your susceptibility to crashes." `[PUBLIC CLAIM]` (2025-plans blog, referring to a
  preprint of the 2026 paper)

The published companion artifact reports a **within-person predictive model** with walk-forward
cross-validation: "(AUC = 0.82–0.85 vs. 0.73–0.83)" `[PUBLISHED RESEARCH]` (§13.2). Both the vendor's
term ("AI algorithm") and the paper's reported method (walk-forward cross-validated prediction of
within-person symptom change) are recorded; the package does not adjudicate the label. What the
artifact supports is a **group-level discrimination statistic (AUC)** for a next-day, within-person
prediction — not a per-user accuracy, and not a validation of the continuous-monitoring features.

**Personalisation** is documented at the level of a rolling personal baseline (up to 90 days, §6.1)
and a per-user PacePoints budget — i.e. personal baselines rather than population thresholds. That is
the same design choice V2 makes, and it is a similarity, not a difference.

**No generative or conversational assistant** is documented. `[ABSENT FROM CORPUS]` (For the
package's comparator set, that distinguishes Visible from Welltory's external-LLM AI Coach.)

---

## 11. Age: four statements that cannot all govern one rule

Four `[DOCUMENTED]`/`[PUBLISHED RESEARCH]` statements about the same account-creation rule:

| # | Surface | Statement |
|---|---|---|
| 1 | Terms of Use (2022-07-14) | "You must be at least 16 years old (the “Minimum Age”) to create an account." |
| 2 | Help centre, "Age requirements for Visible" | "You must be at least 16 years old to create a Visible account." |
| 3 | Same help-centre article | "Under-16s can use Visible, but a parent or guardian must accept the Agreement on their behalf and take full responsibility for their use of the service." and "Visible hasn't been officially tested with members under 18." |
| 4 | npj Digital Medicine paper | Eligibility: "People aged ≥18 years … were eligible for inclusion." Observed cohort: "Participants ranged in age from 13 to 83 years (M = 46.3, SD = 12.4)." |

Statements 1 and 2 agree with each other; statement 3 permits what 1 and 2 forbid; statement 4
excludes under-18s from eligibility while reporting a 13-year-old participant.

**The vendor's own onboarding mechanism is a documented, plausible explanation, and is quoted as
such, not as a verdict:** the same help article tells the guardian to "Create an account using your
own age." If the *guardian's* year of birth is entered for the child's account, then the account's
age field records the guardian, not the user — which is simultaneously a consistent reading of
statements 1–3 and a **documented provenance hazard for the age field**. The paper's own range
(13–83, with eligibility ≥18) is consistent with research consent having been given under a
guardian's account.

Recorded as: **four surfaces, unresolved**; the account age field's provenance is **`[UNKNOWN]`**.

---

## 12. Privacy, hosting, sharing and account deletion

### 12.1 What the vendor says it stores and why

The Privacy Policy (2025-02-27) documents categories including access data (username, year of birth,
email, approximate location), wellness data (symptoms, check-ins, heart rate; consent basis),
wearable data (resting heart rate, HRV, gyroscope, accelerometer), and states that on Android
geolocation is required for BLE scanning while "We do not store your nearby devices data… stored
locally on your device only." `[DOCUMENTED]`

### 12.2 Marketing and third-party sharing, in the vendor's own words

- "For website visitors and purchasers, we collect metadata and event details through Meta and
  Google's conversion tracking tools… Event data is retained by Meta and Google for up to 180 days and
  processed in accordance with a Data Processing Agreement." `[DOCUMENTED]`
- "For users of our mobile app, if you have explicitly provided marketing consent, we may share
  personally identifiable information with Meta for advertising targeting purposes. In such cases,
  your data is hashed on ingestion by Meta for enhanced security." `[DOCUMENTED]`
- Cookies/consent: "In accordance with relevant legislation including GDPR…" — the policy specifies
  consent collection for UK/EU. `[DOCUMENTED]`
- Retention: "We will only retain your personal data for as long as necessary…" — **no fixed period
  stated.** `[DOCUMENTED]`
- Processing locations: "both within the EEA and the USA", with SCCs. `[DOCUMENTED]`
- Deletion: in-app account deletion erases access, wellness and wearable data from servers, **but**
  "In certain cases, such as where you have opted into share your data with third parties for research
  purposes, data has been anonymised and cannot be deleted as it cannot be linked back to you."
  `[DOCUMENTED]`

### 12.3 The absolute claim, and the surfaces that contradict it

The FAQ and the `/research` page state, without qualification:

> "All your Visible data is stored in an encrypted database and isn't shared with any third parties
> unless you choose to opt in to a research study." `[PUBLIC CLAIM]`

Two help-centre articles make the same claim in **different wording** — a variant, not a copy — dated
2025-10-01 ("What happens to my data?") and 2025-11-04 ("How is my data stored?"):

> "All Visible data is stored in an encrypted database and is not shared with any third parties unless
> you explicitly opt in to a research study." `[DOCUMENTED]`

The vendor's most absolute privacy claim therefore appears on **four pages in two wordings**: the
possessive ("your Visible data" vs "Visible data") and the consent verb ("choose to opt in" vs
"explicitly opt in to") both vary. Two consequences, both recorded and neither resolved: four pages
carrying one claim are **not four independent sources**, and because the wording is not identical they
are **not one quotation either**. The drift is a wording-drift observation on the vendor's strongest
claim; it is not evidence about data handling, and no intent is inferred from it.

Against it, on the vendor's own surfaces:

| Counter-surface | What it says |
|---|---|
| Privacy Policy (2025-02-27) | Collects website/purchase metadata through "Meta and Google's conversion tracking tools" and, with marketing consent, shares PII with Meta for advertising targeting; names service providers and due-diligence recipients as additional recipients |
| Play Data Safety declaration | **Data shared** for this app is one entry: category **App activity**, data type **App interactions**, purpose **Analytics** (declared unconditionally, no "optional" qualifier) |
| Help centre, "What data is shared when I enroll" | Describes data shared with each study the user opts into |
| Terms of Use | "By using Visible, you give us permission to use your anonymous information to improve the Service for all users." |

The four counter-surfaces are recorded verbatim with dates. The vendor's sentence is about "your
Visible data" — an **undefined term** — and a narrow reading of it (health/wellness records
specifically, excluding app-interaction analytics and website conversion events) is available. That
reading is **not stated by the vendor anywhere in the retrieved corpus**, and the package does not
supply it. Recorded as: **absolute claim, four counter-surfaces, unresolved.**

This is a sharper conflict class than the package has previously recorded, because one of the
counter-surfaces (Play Data Safety) is a **platform-declared data-flow statement adjacent to the very
store listing that carries the marketing**, not a distant document.

### 12.4 Google Consent Mode, as served to the browser

`[FOUND IN PUBLISHED CLIENT]` The marketing pages ship a Google Consent Mode v2 configuration with a
comment stating the intent:

- "// EU/EEA + UK + CH → opt-in": all storage signals `"denied"` by default except
  `security_storage: "granted"`, `wait_for_update: 2000`, across a 33-entry `region` list
  (`["AT","BE","BG","HR","CY","CZ","DK","EE","FI","FR","DE","GR","HU","IE","IT","LV","LT","LU","MT","NL","PL","PT","RO","SK","SI","ES","SE","GB","IS","LI","NO","CH"]`).
- "// Everyone else (US incl. CCPA states, Canada, etc.) → opt-out": every storage signal
  `"granted"` by default.
- `gtag("set", "ads_data_redaction", true)` and `gtag("set", "url_passthrough", true)` are enabled.
- A Global Privacy Control branch runs
  `gtag("consent", "update", { ad_storage: "denied", ad_user_data: "denied", ad_personalization: "denied" })`.
  **Note the scope:** the GPC update denies the three advertising signals and leaves
  `analytics_storage` untouched.
- Google Ads tag `AW-11364136202`; container `GTM-T2JXK5ZD`.

The vendor's own source comments therefore document an **opt-out (granted-by-default) posture for US
and Canadian visitors**, including named US states. Recorded as a static observation of the served
client; no cookie banner behaviour was exercised (no runtime observation in this package).

### 12.5 Research participation boundary, drawn correctly

- "Participation in our studies is entirely optional, and if you choose to contribute your data, it
  will be done anonymously. You can withdraw from a study at any time, immediately ending any future
  data collection." `[PUBLIC CLAIM]`
- Residual limit, stated by the vendor: already-anonymised data "cannot be deleted as it cannot be
  linked back to you." `[DOCUMENTED]`

The boundary is drawn explicitly — withdrawal stops **future** collection, and the irreversibility of
anonymised prior data is stated rather than hidden. Recorded as a positive comparator to V2's
append-only/tombstone discipline: both refuse to pretend a record can be un-happened.

### 12.6 Not claimed

No statement of HIPAA compliance, SOC 2, ISO 27001, FDA clearance, CE marking, gzip, or
"research-grade" appears in the retrieved corpus. Data protection is claimed as "an encrypted
database" and encryption in transit. `[ABSENT FROM CORPUS]` The package records the **absence of a
compliance claim**, not non-compliance.

---

## 13. Evidence, research and regulatory posture

### 13.1 Regulatory posture

- "Visible is not a medical device, and does not provide medical services such as diagnosis, cure,
  mitigation, prevention or treatment in relation to any disease or medical condition."
  `[DOCUMENTED]`
- No CE mark, FDA clearance or notified-body reference appears anywhere in the retrieved corpus.
  `[ABSENT FROM CORPUS]`
- The store listing's health claims (understanding, pacing confidence, risk reduction) sit alongside
  the in-app disclaimer. Recorded as the package's familiar **marketing-versus-disclaimer tension**,
  unresolved.

### 13.2 The flagship publication, read as its own artifact

**Citation (verified from the publisher page).** *Digital physiological biomarkers predict
within-person symptom changes in complex chronic illness.* **npj Digital Medicine**, volume 9, Article
number 257 (2026). DOI `10.1038/s41746-026-02543-3`. PMID 41876851; Europe PMC `PMC13022203`.
"Published: 24 March 2026"; "Version of record: 27 March 2026"; "Received: 09 November 2024";
"Accepted: 03 March 2026"; open access. `[OBSERVATION]`

**Authors and competing interests (verbatim).** "A.A. received consulting fees from Visible Health
Inc. not related to the production of this manuscript. R.P., P.C., and H.L. are current or former
employees of Visible Health Inc. and did not play a role in the study design or decision to publish
the manuscript." `[PUBLISHED RESEARCH]`

**Funding (verbatim).** "The authors did not receive specific funding for this study. A.A. and D.P.
received research support from the Steven & Alexandra Cohen Foundation." `[PUBLISHED RESEARCH]`

**Ethics (verbatim).** "No personal identifiable information was collected from this dataset. Thus,
the dataset was determined to be exempt from human subject research by the Mount Sinai Program for
the Protection of Human Subjects." — an **exempt determination**, not IRB approval. Recorded as
stated.

**Data and code availability (verbatim).** "De-identified data may be made available upon reasonable
request, provided that appropriate institutional and ethical approvals are obtained." / "Data
analysis was completed in R. The analysis code is available at upon reasonable request."

**Design and results as reported.** `n = 4244`; "an average of 125 biometric observations per
participant"; "Within-person increases in HR and decreases in HRV in the morning were associated with
worsening symptom reports in the evening."; "walk-forward cross-validation showed a statistically
significant improvement in model performance when morning biometrics were added to prior-day symptom
reports (AUC = 0.82–0.85 vs. 0.73–0.83)"; and, for the input set, "We examined whether within-person
fluctuations in HR, HRV, and respiratory rate predicted daily changes in crash, fatigue, and brain
fog symptoms and secondarily evaluated model predictive performance." `[PUBLISHED RESEARCH]`

**Respiratory rate is an input to the published model, derived from the same 60-second PPG capture.**
That is a research-artifact fact about the paper, not a documented consumer feature: no
respiratory-rate display, value or trend appears in the 95-article help corpus or on the marketing
site (§5). It is recorded because AnxietyWatch2 has a specified respiratory-rate workstream.

**Two verified in-artifact defects.**

1. **Age contradiction inside one Participants section.** Eligibility states "People aged ≥18 years …
   were eligible for inclusion" and the next sentence reports "Participants ranged in age from 13 to
   83 years (M = 46.3, SD = 12.4)." Both strings verified in the publisher and PMC copies.
2. **A dangling preposition in the Code availability statement**: "The analysis code is available at
   upon reasonable request." Verified verbatim in both copies. The statement therefore does not
   identify where the code is available, and the paper's reproducibility claim is **`[UNKNOWN]`** on
   that sentence alone.

**Attribution limits, recorded as such.** The paper is vendor-affiliated (a co-author is a person
named on the vendor's own site as its founder) and the vendor co-founds the cohort. It is therefore
**not independent validation**, and the package does not present it as such.

**The vendor's own blog states the honest boundary.** "Importantly, this doesn't mean a single reading
can tell you exactly when a crash is coming…" `[PUBLIC CLAIM]`

**The paper's own limitation.** "Due to the retrospective study design, limited information was
collected on demographics, making the generalizability of the findings uncertain." `[PUBLISHED
RESEARCH]`

### 13.3 What the paper does *not* validate

The paper's biometric input is a **60-second morning PPG assessment** paired with an **evening**
symptom report: "The current study integrates a 60-s morning PPG assessment with evening symptom
severity reports". `[PUBLISHED RESEARCH]`

The vendor's paid tier is built on **all-day continuous monitoring** and real-time alerts (§9). The
vendor's own blog frames continuous monitoring as future work: "Next, we'll build on these findings by
bringing in continuous monitoring…" `[PUBLIC CLAIM]`

So the strongest published evidence behind Visible validates the **free tier's daily reading**, not
the paid tier's headline features. Recorded as an **evidence-scope finding**, quoted from both
surfaces.

### 13.4 Algorithm provenance: versioning history and the unlabelled change

The vendor's own blog history documents labelled Stability versions:

| Artifact | Date | What it does |
|---|---|---|
| "Introducing: Morning Stability" | 2023-01-26 | introduces "Morning Pace" |
| "Introducing: Morning stability v1.1" | 2023-04-11 | v1.1 |
| "Introducing: Morning stability v1.2" | 2023-05-30 | v1.2 |
| "A new way to get your Stability score" / overnight mode | 2026-07-14 | overnight/continuous mode introduced as an Early Access Hub beta; "Morning Stability" renamed "Stability score" |
| "An even more accurate Stability Score" | 2026-09-23 | changes existing scores — "a day that previously gave you a 4, might now give you a 3" — **with no version identifier** |

The first three lines are a **refutation of the initial hypothesis** (that the vendor ships algorithm
changes with no versioning at all): the vendor demonstrably labelled v1.1 and v1.2. The last line is
the finding that survives: **the 2026-09-23 change alters already-recorded daily scores and carries no
version label**, six months after the 2026-03-24 validation the marketing rests on, and no published
artifact links the validated version to the shipped one.

Contrast surfaces that make this a *precise* finding rather than a generic complaint:

- the vendor's own precedent of labelling v1.1/v1.2;
- Oura's published `PublicSleepAlgorithmVersion` enum in its documented API (package candidate 8);
- AnxietyWatch2's registry-driven schema-version discipline.

Recorded as a **provenance finding**, not misconduct.

---

## 14. Reports, clinician sharing and export

| Artifact | Documented content and limits | Class |
|---|---|---|
| **CSV export** (Profile → Export data) | Stability (resting HR, HRV, score), morning/evening/monthly check-ins, notes, medications, FUNCAP answers (not the final average), breathing minutes, PacePoints and budget. "the data export does not include your live continuous heart rate data or Heart Rate Zones" | `[DOCUMENTED]` |
| **Health Report PDF** | Requires **30 days** of morning check-in data; rolling averages; includes symptom prevalence and burden, periods (not other experience trackers), medications, resting HR, HRV | `[DOCUMENTED]` |
| **Clinician Dashboard** | Live, revocable link; membership-only; desktop/tablet view. "only displays sections when there is information to share"; shows trackers "logged at least once" | `[DOCUMENTED]` |

**The export asymmetry is the finding.** "This continuous, all-day heart rate can't be exported or
downloaded. To let a clinician view it, share a live link to your Clinician Dashboard instead."
`[DOCUMENTED]` Visible's derived score leaves the platform; the raw continuous signal does not. A
clinician sees the live view only through the vendor's own revocable link.

**The clinician surface has no third state.** The rule that omits empty sections, "to keep the
dashboard clean", means an absent month is **not** distinguishable from a month with no data on that
dashboard. The vendor has implemented V2's third state at the *score* layer and not at the
*presentation* layer. Quoted, not inferred.

---

## 15. Pricing, tiers and geography

From `/how-it-works`, verbatim table content `[PUBLIC CLAIM]`:

**United States**

| Tier | Membership | Device | Total upfront cost |
|---|---|---|---|
| Annual ("Save 25%") | $179.88 ($14.99/month) | Armband 2.0 $79.70 | $259.58 |
| Monthly | $19.99/month | Visible Band 2.0 $79.70 | $99.69 |

**United Kingdom**

| Tier | Membership | Device | Total upfront cost |
|---|---|---|---|
| Annual ("Save 31%") | Annual membership (25% off) £131.88 (£10.99/month) | Visible Band 2.0 (41% off) £49.70 | £181.58 |
| Monthly | Monthly membership £14.99/month | Visible Band 2.0 £84.70 | £99.69 |

`[OBSERVATION]` All four stated totals are arithmetically consistent with the stated components, and
the row-level discount labels are consistent with the listed prices (14.99 vs 19.99 = 25% off;
£49.70 vs £84.70 = 41% off). An earlier hypothesis of an arithmetic defect was **tested and
refuted**, and is therefore **not** recorded as a finding.

`[OBSERVATION]` The two tables' headline "Save N%" labels are computed on **different bases** and the
page does not say so: the US header matches the membership-only discount, while the UK header (31%)
corresponds to neither listed row label and matches a combined first-year saving against the monthly
tier. Recorded as a **presentation observation** — the reader must infer which base each headline
uses — not as a defect in the prices.

Other commercially relevant facts:

- Free tier: camera-PPG morning measurement, Stability score, symptom tracking, research
  participation.
- Paid tier (Membership + band): all-day heart rate, PacePoints, real-time alerts, clinician
  dashboard.
- Refunds are limited for the device: "we're unable to provide refunds for the device cost unless
  your armband is faulty." `[DOCUMENTED]`
- Platform floors: Android 9+, iOS 15+; Huawei and Oppo explicitly excluded. `[DOCUMENTED]`
- No free/OSS tier or self-hosted option is documented. `[ABSENT FROM CORPUS]`

---

## 16. Claim ledger

`Verdict` column meaning: **match** = a documented or static surface supports the claim;
**documented-only** = the claim exists only as a vendor statement with no verifiable artifact;
**partial** = supported but narrower or qualified elsewhere by the vendor; **does not match** = a
vendor surface contradicts or scopes it out; **unresolved** = conflicting vendor surfaces, recorded
without adjudication; **unknown** = not established.

| # | Claim | Surface | Verdict | Note |
|---|---|---|---|---|
| V-1 | "Join over 250,000+ people… improving their pacing with Visible." | Store listing (both stores) | documented-only | No independent count; Play installs "100K+" is a different quantity |
| V-2 | "Unlike fitness tracking apps, Visible is designed to support rest and pacing, not workouts and exercise." | Store listing | match | Consistent with FAQ refusal of fitness wearables |
| V-3 | "reduce the risk of crashes or PEM" | Store listing + website | unresolved | No controlled efficacy evidence published; see V-44…V-48 |
| V-4 | 60-second phone-camera PPG measurement of HR and HRV | Store listing, help centre | documented-only | Measurement mechanism described; no validation artifact for the camera path |
| V-5 | Band "manufactured by Polar, a world leader in heart-rate monitoring since 1977." | `/how-it-works` | match | Consistent with the supported Polar Verity Sense lineage |
| V-6 | Real-time pacing notifications (Exertion, PaceSetter) | Store listing, `/how-it-works`, help centre | documented-only | Feature documented; latency/algorithm unspecified |
| V-7 | PacePoints all-day energy budgeting | Store listing, help centre | documented-only | Derivation described qualitatively (exponential curve, ~4× in Exertion zone); no formula published |
| V-8 | "Only available in US and UK" (all-day data) | Store listing | match | Price page lists US/UK columns only |
| V-9 | Stability score 1–5 from RHR, HRV, sleep and symptoms | Help centre | documented-only | Inputs named; weights not published |
| V-10 | First score requires 4 days of data | Help centre | documented-only | — |
| V-11 | Baseline up to a 90-day rolling window | Help centre | documented-only | — |
| V-12 | Missing overnight reading ⇒ no score that day, no substitute morning reading | Help centre | match | Positive comparator: derived value withheld when input absent |
| V-13 | Switching measurement modality creates a new baseline | Help centre | match | Consistent with the missing-signal rule |
| V-14 | "A stable score doesn't guarantee you'll feel good…" | Help centre | match | Vendor's own honest boundary |
| V-15 | Workwell-sheet citation with explicit non-equivalence caveat | Help centre | match | Correctly bounded external citation |
| V-16 | "Studies suggest the early hours of sleep are more sensitive to reflect recovery…" | Help centre | documented-only | No study named |
| V-17 | "Increasingly, researchers are questioning the idea of a 'good' heart rate variability (HRV)." | Help centre | documented-only | No source named |
| V-18 | Symptom check-in limited to once daily to avoid anxiety | Help centre | documented-only | Stated anti-anxiety rationale |
| V-19 | "Your evening symptom scores directly inform your Stability Score the next morning, which is designed to help you start your day with a clear understanding of how stable your heart rate variability (HRV), resting heart rate, sleep, and symptoms are." | Help centre | documented-only | Coupling stated; method not published |
| V-20 | Beta-blockers reduce PacePoints and exertion notifications | Help centre | match | Documented degradation of the alert path; manual mitigation |
| V-21 | Beta-blockers may generate PacePoints "even when at complete rest" | Help centre | match | Same article, opposite direction |
| V-22 | Overnight window = 4 h from 30 min after sleep onset | Help centre | documented-only | No numeric signal-quality threshold published |
| V-23 | "If you don't [have a strong and stable internet connection], the heart rate reading may not start or complete." | Help centre | documented-only | Measurement gated on connectivity per the vendor; not runtime-tested |
| V-24 | "We have not been able to test Visible with every smartphone" | Help centre | match | Vendor states its own device-coverage limit |
| V-25 | "cutting-edge AI algorithm that predicts your susceptibility to crashes" | Blog (2025 plans) | partial | Paper reports a walk-forward cross-validated within-person model; "AI" is the vendor's label |
| V-26 | CSV export includes Stability, check-ins, notes, medications, FUNCAP answers, breathing minutes, PacePoints | Help centre | documented-only | — |
| V-27 | Export excludes live continuous heart rate and Heart Rate Zones | Help centre | match | Derived exported; raw withheld — inverse of V2 |
| V-28 | Health Report requires 30 days of morning check-in data | Help centre | documented-only | — |
| V-29 | Clinician Dashboard "only displays sections when there is information to share" | Help centre | match | Missingness omitted, not marked unknown |
| V-30 | Research participation optional, anonymous, withdrawable | `/research` | match | Withdrawal stops future collection |
| V-31 | Anonymised data cannot be deleted | Privacy policy | match | Consistent with `/research` |
| V-32 | "All your Visible data… isn't shared with any third parties unless you choose to opt in to a research study." (help-centre variant: "All Visible data… is not shared… unless you explicitly opt in to a research study.") | FAQ + `/research` + two help articles — two wordings, four pages | **unresolved** | Contradicted by privacy policy (Meta conversion + marketing PII sharing), Play Data Safety (App interactions → Analytics), and the Terms' anonymous-improvement permission |
| V-33 | Data encrypted in transit | Play Data Safety; privacy policy | match | Play: "Data is encrypted in transit" |
| V-34 | Data deletable on request | Play Data Safety | match | "You can request that data be deleted" |
| V-35 | Retention "only… as long as necessary" | Privacy policy | partial | No fixed period stated |
| V-36 | Processing in EEA and USA with SCCs | Privacy policy | match | Help centre says "generally… within the EEA" with third-party exceptions — informational tension |
| V-37 | Site cookies consent handled for UK/EU | Published client (Consent Mode) | partial | EU/EEA+UK+CH default-denied; rest of world default-granted opt-out |
| V-38 | Minimum age 16 to create an account | Terms (2022) + help centre | **unresolved** | Contradicted by the same help article's under-16 guardian path |
| V-39 | "Visible hasn't been officially tested with members under 18." | Help centre | match | Consistent with the paper's ≥18 eligibility |
| V-40 | "Visible is not a medical device…" | Privacy policy | match | No CE/FDA claim anywhere in corpus |
| V-41 | Paper: eligibility "People aged ≥18 years" | Paper | **does not match** | Paper also reports ages 13–83 |
| V-42 | Paper: "The analysis code is available at upon reasonable request." | Paper | **does not match** | Dangling phrase; code location not identified |
| V-43 | Paper: no specific funding | Paper | match | A.A. and D.P. had unrelated foundation support |
| V-44 | Vendor term "pre-print" / blog "breakthrough research paper" | Blog (2025) | match | Paper subsequently published 2026-03-24 (peer-reviewed) |
| V-45 | Paper validates the free tier's daily reading | Paper | match | Input is "a 60-s morning PPG assessment with evening symptom severity reports" |
| V-46 | Paper validates the paid tier's continuous monitoring | — | **does not match** | Blog: "Next, we'll build on these findings by bringing in continuous monitoring…" |
| V-47 | Vendor affiliation of the paper's authors | Paper competing interests | match | "H.L." is a founder named on the vendor's About page |
| V-48 | Ethics: "determined to be exempt from human subject research" | Paper | match | Exempt determination, not IRB approval |
| V-49 | Stability algorithm versions v1.1 and v1.2 exist | Blog history | match | Refutes an "unversioned algorithm" hypothesis |
| V-50 | The 2026-09-23 change carries a version label | Blog | **does not match** | Changes prior scores; no version identifier; postdates validation |
| V-51 | "We don't make money by selling devices for a profit." | Blog | documented-only | Hardware margin `[UNKNOWN]` from public data |
| V-52 | Membership is the monetisation route | Blog + price page | match | Price page: membership + band, US/UK |
| V-53 | Refunds for device only if faulty | Help centre | match | — |
| V-54 | Android 9+, iOS 15+; Huawei/Oppo excluded | Help centre | match | iTunes `minimumOsVersion` 15.0 agrees |
| V-55 | No public API, webhook or third-party integration | Corpus | match | Absent from all 95 help articles and the site |
| V-56 | No CPAP/respiratory/SpO₂/sleep-staging support | Corpus | match | Absent from corpus; recorded as absence of a documented capability |
| V-57 | No weather/aircraft/noise ingestion | Corpus | match | Environment appears only as a user-interpreted confounder and a research-study subject |
| V-58 | Environmental conditions study is open for enrolment | `/research` | match | "Effect of atmospheric conditions on complex chronic illness" |
| V-59 | No offline mode | Corpus | match | "offline" does not appear in the corpus |
| V-60 | Apple Health / Health Connect integration | Help centre | **does not match** | "We don't currently support HRV data from Apple Health or Garmin…" |
| V-61 | "Get a phone notification when you are exerting or pacing ahead for the day." | `/how-it-works` | documented-only | Alert types documented; delivery/escalation semantics unspecified |
| V-62 | "Visible is designed for people living with Long Covid, ME/CFS, POTS, fibromyalgia and Ehlers-Danlos syndrome (EDS)." | Website | documented-only | No clinical scope artifact published |
| V-63 | "It supports pacing, a widely recommended approach for these conditions" | Website | documented-only | External guidance not cited on this page |
| V-64 | 12+ App Store advisory | Store listing | match | Play content rating "Everyone"; neither equals the Terms' 16 minimum |

---

## 17. Inconsistencies and reconciliation items (recorded, not adjudicated)

1. **Age (V-38 / V-41).** Four surfaces, §11. Plausible documented mechanism named by the vendor; not
   resolved.
2. **Third-party sharing (V-32).** Absolute help-centre sentence versus privacy policy, Play Data
   Safety and Terms. §12.3.
3. **Compliance-withdrawal incompleteness.** "You can withdraw from a study at any time, immediately
   ending any future data collection" sits beside the privacy policy's statement that anonymised
   research data cannot be deleted. Both are true and both are published; they answer different
   questions and the help centre does not link them.
4. **Validation scope (V-45 / V-46).** Published validation covers the daily reading; the paid tier's
   headline features are continuous. Blog's "Next…" framing is consistent with the gap.
5. **Algorithm provenance (V-49 / V-50).** The vendor labels some versions and not the change that
   moved existing users' scores.
6. **Retention.** "As long as necessary" with no period; research data explicitly undeletable. Not a
   contradiction, but the two statements are on different surfaces and are not reconciled for the
   reader.
7. **Pricing-headline basis.** "Save 25%" (US) and "Save 31%" (UK) are computed on different bases
   without stating the base. Prices themselves are arithmetically consistent.
8. **Terms staleness.** Terms dated 2022-07-14 govern a 2026 product tier (band, clinician dashboard,
   overnight mode) not described in them.
9. **Documentation defects (verbatim).** Terms: "The Agreement controls the relationship between you
   and and does not establish…" (repeated conjunction); the phrase "Privacy and Policy"; use of both
   "Terms of Use" and "Terms of Service" for one document. The paper: §13.2 items 1 and 2.
10. **Site routing.** `/pricing` and `/science` return HTTP 404; `/research` and `/join-research`
    return byte-identical documents (same SHA-256); `/study` and `/product-demo` return ~13 KB
    JavaScript shells with no server-rendered content. Consumers of the site's own navigation see a
    pricing page only at `/how-it-works`.

---

## 18. Versus AnxietyWatch

### 18.1 Versus AnxietyWatch v1 (implemented)

| Axis | AnxietyWatch v1 | Visible | Reading |
|---|---|---|---|
| Primary input | iPhone/HealthKit + CPAP/OSCAR import + manual entries | Phone camera PPG ± visible band | Different sensor models; v1 is device-agnostic and imports, Visible is a closed sensor pair |
| Crisis handling | `CNSRisk/` alert stack; crisis notice in `README.md:57` | No crisis/suicide surface documented anywhere in the corpus | Visible is **out of scope** for the crisis use-case entirely — a category difference, not a feature gap |
| Correlation analysis | Local correlation engine (specified 2026-04-04) | No user-facing correlation engine documented | v1 specifies cross-signal analysis; Visible publishes none |
| Real-time alerts | No continuous hardware alert path | Exertion + PaceSetter notifications, band-driven | Visible leads on a real-time consumer alert; v1 leads on breadth of ingested signals |
| Export | Local data ownership posture | Server-side CSV/PDF export with raw continuous HR excluded | Opposite postures |
| CPAP | OSCAR/CPAP import path with a known column-position coupling | None | v1 only |

### 18.2 Versus AnxietyWatch2 (current + specified target)

| Axis | AnxietyWatch2 | Visible | Reading |
|---|---|---|---|
| Storage model | Append-only, no UPDATE/DELETE, tombstone removal, UTC epoch seconds | Server-side; anonymised research data explicitly undeletable | Both refuse retroactive erasure; V2 by schema, Visible by anonymisation |
| Missing data | Three states everywhere: measured-good, measured-bad, not measured | Third state implemented **at the score** ("no Stability Score that day"), **absent at the clinician dashboard** | Visible is the closest published comparator to V2's rule, and shows where V2's rule is stricter |
| Units/derivation | Unit converted once at the decoder; values rejected, named, quarantined when out of range | Derived score computed server-side; no published formula, no published range contract for raw values | V2's discipline is a documented absence in Visible |
| Algorithm provenance | Registry-driven types; schema/version discipline | v1.1/v1.2 labelled historically; 2026-09-23 change unlabelled and it moved existing scores | V2's strongest structural difference in this comparison |
| Sensor plane | Polar H10, BlueZ broker, ResMed Air11 BLE, OSCAR/XE125 decoders | Polar-OEM armband only; no third-party device ingestion | Direct overlap (Polar), opposite openness |
| Environmental context | Weather/aircraft/noise context **specified as ingested data** | Confounder documented, not measured; a study on atmospheric conditions is recruiting | Same hypothesis; Visible treats it as user judgement + research, V2 as a signal |
| Offline operation | Spool-and-sync, degraded-mode design | Measurement itself is documented as connectivity-dependent; no offline mode | V2's design is the direct answer to Visible's documented limit |
| Alarm path | Supervised alarm (specified, unshipped) with a hold-state rule | Real-time pacing alerts that under-alert on beta-blockers, with manual mitigation | V2's specification is explicit about untrusted-signal handling; Visible's published behaviour is not |
| Regulatory claim | "explicitly not a medical device" (admin-console spec) | "Visible is not a medical device…" | Aligned; neither claims clearance |
| Research model | N-of-1 trial framing (v1 `docs/FEATURES.md:10`) | Vendor-run cohort studies with opt-in data donation; peer-reviewed paper | Different evidence models; V2's N-of-1 framing is single-subject by design |
| Export posture | Signals are the record; presentation downstream | Derived exported, raw continuous withheld | Inverse |
| Hardware | Third-party sensors; no vendor hardware | Vendor-branded Polar OEM band; "we don't make money by selling devices for a profit" | Visible's hardware is the paid tier's gate on the alert features |

### 18.3 Where Visible is genuinely stronger than V2 as currently shipped

1. A **shipped real-time consumer alert path** on continuous heart rate; V2's supervised alarm is
   specified and unshipped.
2. **Published labelled algorithm versions** (v1.1, v1.2) — V2 has schema discipline but no
   user-visible algorithm-version history.
3. A **peer-reviewed publication with a full ethics, competing-interest and limitations section** —
   V2 has no comparable published evidence artifact.
4. A **published anti-anxiety rationale for measurement cadence** — V2 has no equivalent documented
   trade-off on the anxiety-amplification axis.
5. A **clinician-shareable live surface** — V2's iOS client and admin console do not ship an
   equivalent clinician view at this date.

### 18.4 Where V2 is genuinely stronger

1. **Third state at every layer**, including presentation; Visible drops empty sections.
2. **Signal-level record of record with raw export**; Visible withholds the raw continuous signal.
3. **Append-only/tombstone semantics** versus server-side mutable records with anonymised data that
   can never be deleted.
4. **Open sensor plane and device-agnostic ingestion**; Visible is closed to Apple Health, Garmin,
   Fitbit and CPAP devices.
5. **Offline-capable design**; Visible documents connectivity-gated measurement.
6. **Decoded, typed, registry-driven signals** with unit and range contracts; Visible publishes no
   such contract.
7. **Algorithm/model versioning discipline** at the schema layer and an explicit registry source of
   truth; Visible's latest score change is unlabelled.

---

## 19. Evidence gaps and limits of this file

- **No runtime observation.** No Visible build was installed or exercised; the runtime-observation
  count for this package remains zero. Every functional statement is `[PUBLIC CLAIM]` or
  `[DOCUMENTED]`.
- **No app-binary inspection.** The Terms prohibit it (§1 of the inspection file). Nothing here is a
  static finding from the app binary.
- **Screens, latency and notification semantics are `[UNKNOWN]`.** Whether an alert fires, how fast,
  and what it says cannot be established from documentation.
- **Measurement quality is `[UNKNOWN]`.** No validation artifact for the camera PPG path, no
  published signal-quality threshold, no error model.
- **The Stability formula is `[UNKNOWN]`.** Inputs are named; weights, thresholds and the version
  actually running on any given date are not published.
- **Hardware margin and unit economics are `[UNKNOWN]`.**
- **Server architecture, database, hosting region and sub-processors beyond those the privacy policy
  names are `[UNKNOWN]`.**
- **No Play/App Store version was installed**, so the store listing's version numbers cannot be tied
  to any behavior.
- **Retrieval limits.** `/study` and `/product-demo` returned ~13 KB JavaScript shells (no
  server-rendered content): recorded as `[UNKNOWN]`, not as absence. `/pricing` and `/science` return
  404: recorded as verifiable absence of those URLs.
- **Absences in §5 and §9 are corpus absences**, established by searching the 95-article help corpus
  and the marketing site for the corresponding strings; a capability could exist undocumented.

---

## 20. Artifact record

Digests are SHA-256 of the retrieved bytes; the manifest is the retrieval record. Retrieval used a
desktop HTTP client with a browser user-agent; the help centre `sitemap.xml` supplied article URLs.

### 20.1 Primary surfaces

| Artifact | URL | Bytes | SHA-256 |
|---|---|---|---|
| Marketing home | `https://www.makevisible.com/` | 123486 | `0867c1b5f684925e55d9ad19928c2d832c8dbc17562b45fe4292ef00c28a4cbe` |
| How it works (product + pricing) | `https://www.makevisible.com/how-it-works` | 111726 | `9b9355ff23dd002099ad60e55f91d6628739e6973d62a79b4468435d6a1d2621` |
| Pacing | `https://www.makevisible.com/pacing` | 68800 | `4c4de76e60ef1d19482c1b1e53257068d93bc894f53bb976263d807bf7b0253d` |
| FAQs | `https://www.makevisible.com/faqs` | 79145 | `2ecbf9dba70bceb2417342231b1e597a8f4f9ee85cd743d66586ad00f3471f43` |
| Research / join-research (byte-identical) | `https://www.makevisible.com/join-research` | 68637 | `f0e17c9c974a27f8663bc4c70b6de3f2d76909261b592e5b0ea0b0c6428d18e6` |
| Privacy policy | `https://www.makevisible.com/privacy` | 68066 | `d0dc0ce9cac10412f721efccffc3d9745a141ed129460b4ec9477f08740ddacb` |
| Terms of Use | `https://www.makevisible.com/terms` | 63704 | `591dec4e7fe422efc50abcad9df9d3f91e7ef40dcdba2dba21dbfad94b143f48` |
| About | `https://www.makevisible.com/about` | 58032 | `1a32b32bca3e9cc8cea926f357a45b8468782ad37b220195e5753d11d78cde26` |
| Sitemap | `https://www.makevisible.com/sitemap.xml` | 11511 | `ff3d33a549da84492bd982d2c9d74d274b68bbf71934ba1a3eb1ffd139b0987b` |

### 20.2 Stores

| Artifact | URL | Bytes | SHA-256 |
|---|---|---|---|
| App Store lookup (US) | `https://itunes.apple.com/lookup?id=1624474919&country=us` | 10745 | `fb05abe9cb5b8fdf49e73236906a4064e54613e9af5ff30428d6ac28285ac861` |
| App Store lookup (GB) | `https://itunes.apple.com/lookup?id=1624474919&country=gb` | 10737 | `ca0a5f3370591a40ec0f009911c849a719d2d4e0deb758054ef73f6fcd809c3b` |
| Google Play listing | `https://play.google.com/store/apps/details?id=com.makevisible.visible` | 1185607 | `e15b3b1db5ca8b3384b00ccc6b80018c78b0f80d42b9c4f6fbfab11f5e2ac73f` |
| Google Play Data Safety | `.../datasafety` | 636860 | `795b8e9646724fcec75e563677b9ae7b2c2d02fa0e8f237e636babba06170940` |

### 20.3 Publication and registry

| Artifact | URL | Bytes | SHA-256 |
|---|---|---|---|
| Publisher article | `https://www.nature.com/articles/s41746-026-02543-3` | 464712 | `a4c29b000f0481dec0ce6449970242e4bed51275879a693bfc1a452fffe2a176` |
| PMC full text | `https://pmc.ncbi.nlm.nih.gov/articles/PMC13022203/` | 248692 | `b9de160bcdbc8490145d8b38b8b042c2586fade42fb57199bb21880cc4d5c0b0` |
| Europe PMC API record | Europe PMC REST, `PMC13022203` | 6792 | `53408e066d998ad08c4110752c86c5887dde5de5bea177df1d6b277dc31db551` |
| ClinicalTrials.gov query (sponsor=Visible) | API v2 `studies` | 11273 | `67c8deb8cd5c6f81f7a0e40771f9750d9536138beed88b3024c5a73385462ad2` |

The registry query returned no registered trial sponsored by the vendor; the flagship evidence is an
analysis of app-collected observational data, not a registered interventional trial. Recorded as a
`[REGISTRY]` observation.

### 20.4 Vendor GitHub organisation

| Artifact | URL | Bytes | SHA-256 |
|---|---|---|---|
| Org metadata | `https://api.github.com/orgs/makevisible` | 1191 | `d79aaad6c1950fad31fdb88b5a95d18228bcd311c43ac2a806446bf4c9615d7f` |
| Repo list | `https://api.github.com/orgs/makevisible/repos` | 12450 | `98dae4c2a77323283b197c36e296b2c101f86b73177181d1e8b4a7f6f94fa73a` |
| `polar-ble-sdk` fork | `https://api.github.com/repos/makevisible/polar-ble-sdk` | 19782 | `45f7834b26c4b424eb604212d183e21e6dc2cc4d46077a1b25072038e3f7d3e1` |
| `async-zip` fork | `https://api.github.com/repos/makevisible/async-zip` | 18764 | `0be7ebc480ed5c3b6749967cc39eb71695ab730c0fa95615ef27269bf4aae799` |
| Compare `polar-ble-sdk` | `.../compare/polarofficial:master...makevisible:master` | 10276 | `2105975ddcae8abde00e25d1f305e010245a7afe5de6f4a1eb470ff1e020ece1` |
| Compare `async-zip` | `.../compare/korzonkiee:master...makevisible:master` | 8785 | `8c0b3414b8c697599ba3aeccb8c918db121ee9aab7550e4415fe027af3597417` |

Both repositories are **forks**: `polar-ble-sdk` is a Swift fork of `polarofficial/polar-ble-sdk`
(created 2025-01-30, **23 commits behind upstream, 0 ahead**) and `async-zip` is a "Flutter plugin for
asynchronous reading and writing of Zip files" fork of `korzonkiee/async-zip` (created 2025-04-11,
identical to upstream). The organisation publishes **no first-party source**. This is a
`[STATIC VENDOR ARTIFACT]` fact about a *published mirror*, not about the app binary.

### 20.5 Help-centre articles cited

All retrieved from `help.makevisible.com` (URLs discovered via `sitemap.xml`); all HTTP 200.

| Article | Bytes | SHA-256 |
|---|---|---|
| `15519170-age-requirements-for-visible` | 80140 | `64f233c9728cdfe0894eedf3fbf0ae6fba9bb77604e7dc60e21e6e682b6f1c16` |
| `10125371-an-introduction-to-the-stability-score` | 86627 | `a3e1fbfb89690aa6a8d003132c0f78602aac53939132d0ac18bd635589277d5b` |
| `17115224-understanding-your-stability-score` | 100681 | `b4f9397e3b2a7ccc7d5cd95e6e8b1ccc8730bab0108794a86f6235c1fb42b3ed` |
| `14982301-getting-your-stability-from-overnight-data` | 109027 | `1627ee629fd0d3c82a69b981db459210ffd06ff729e4dad9fd7a0336440b5a8e` |
| `14979704-how-stability-overnight-data-measures-your-hr-and-hrv` | 76513 | `c152ab887122ac71c8ac23258d5b7f0748f6695070dcfbf1a7cd1f111f770366` |
| `15074973-taking-consistent-morning-measurements` | 103761 | `00c889dd54e582282ebe631f85aee0674bc397f900837a4ff5dacdeb062836c3` |
| `8643760-morning-check-in-with-smartphone-camera-help-and-tips` | 76474 | `312a435031d2e3d835e8f2fb76182216efd41f3bac7e06a0962e7c4f0879b390` |
| `9562114-pacepoints-and-the-pacesetter` | 108771 | `7e3368cfb28cc62894679ee1d4413730f3ee82a408987553d3e31db4db30c962` |
| `9562198-pacing-notifications` | 77088 | `05bc32bb6823e4831cefbc62d399c2719a081d3ac510ab355c4009a797fd22cc` |
| `10584552-heart-rate-monitored-pacing` | 78554 | `1b89449b101c96b861842d0fbfb9ab89a6874c1ba80c8e7a39a598be28c4a03a` |
| `10632785-using-visible-with-heart-rate-altering-medications` | 86289 | `c372733dbd36ddd067a14bf9553e374fbf2dca2544a408e486aef28cd69eb869` |
| `10125165-why-do-we-only-ask-you-to-record-your-symptoms-once-a-day` | 65710 | `b3ee08a5f9bf6948f6fa847302158817b81e425d0ac372609ac516b3bafd9f76` |
| `9583077-data-export` | 91477 | `fdf957918f694da601fe2f30eb2ef094d2539103935a52a3b9f51b9ced2ccc7c` |
| `9583075-health-report-pdf` | 97931 | `118f1dbb5242d5952c9c69556f45f7cce0e790abfe844aa5e153178d61aecb59` |
| `17297503-share-your-data-with-a-clinician` | 120050 | `58d4b0d1dd4a1aed541c1e2290de25f43353fcd337c594cf9f09d1fe0d444713` |
| `10632816-what-happens-to-my-data` | 63193 | `300c0ceafeff878df679df655b9d0bce7d6408d0472e783248237bbe63ef39fb` |
| `10632819-how-is-my-data-stored` | 63321 | `57e0bb60c0a9459b3a9059d45354ab29829569a6a70ce80887c7080c03fb19fd` |
| `9583638-what-data-is-shared-when-i-enroll` | 64414 | `ace8920f63ad16c0032482aca74baa632b224faa7806d7c766b712a8cbffb413` |
| `9561734-polar-verity-sense-faqs` | 94822 | `36f869fe6d9435ecf5fee177e2b2b67fc48e2ffdb750d17a655c82c3c3704753` |
| `10125529-what-is-my-normal-range` | 70463 | `b916d30fc1e2e07e8fbede7e495f2c2e425a6d991985379f6710ff1923356d16` |
| `10125500-what-is-heart-rate-variability-hrv` | 91429 | `e3859ece7d0822da5b3b6a0987de5a86e97d1b66a13bd48c8d70f89808eaa89d` |
| `14767334-early-access-hub` | 86067 | `dc459f85c817ecf98f68e3e417d11bb2f7cfc9e84ef615e8757c0e207cdd9d91` |
| `13600620-your-pacing-day` | 105669 | `11d872c08dd9530e4b9aa2de1c51b92454c5266004e7f31ac6f73f13e95b61a7` |
| `10632792-what-conditions-does-visible-support` | 63214 | `808d704e19c1e6fe1923c7416feeebfcc09f086c99778ef090ac3c8e13594deb` |

### 20.6 Blog posts cited

| Post | Bytes | SHA-256 |
|---|---|---|
| `a-more-accurate-stability-score` (2026-09-23) | 59528 | `918dba03b250176dd8e21f5fa16d2e08a1620ead8bb98118e6ffd56ff60168eb` |
| `a-new-way-to-get-your-stability-score` | 63711 | `9aa3622892cb401d86c4ce9a8b1a6237392ffe77e2ece00440b1204d9cea76ac` |
| `introducing-the-morning-stability-score` | 62079 | `e9b3b99be9be96d696f5c41a257481ef884146e79c957fae62453b1f3630d237` |
| `introducing-the-morning-stability-score-1-1` | 60545 | `f2cba821318fbbf39d2f69007ec48f2e8b4d7a7da6e3847ec4b3816e3af897ee` |
| `introducing-the-morning-stability-score-v1-2` | 59483 | `0788ec18390c063e3d1d4effc6925631c9c588bcb592d5abbb339c7ad5b9d81d` |
| `introducing-visible-armband-2-0` | 61435 | `2a9bb0cc5dbc8625f6384528874458820d32581f9e7f6ed7544a217179153aca` |
| `our-new-research-morning-biometrics-nature` | 63387 | `c499d7407cce433262ca16c63b5fab58fcabf2c9e15efc1089ad311c0ab4562a` |
| `first-two-visible-studies-published-building-the-science-of-wearables-and-pacing-together` | 64285 | `2243fb2d6b2a6a1a17489f869d7b5dad16106d8435e7ed2441ca10d13615f8e7` |
| `visible-whats-happening-in-2025` | 61010 | `f99b4e48b05a00b54f09c8965f3c1f6eb051dca547fd59ba1e5029b9990ecdbe` |
| `how-visible-members-are-helping-build-the-science-of-complex-chronic-illness` | 63175 | `362de52a66471bdecceb45cd3b852507ff461ef140e04eed56dea7110fe3d235` |
| `introducing-david-putrinos-research-study` | 60668 | `d4393a1f601b5111123311524be1231b57d89b51398980e4e23ff813c2b6ec05` |
| `making-visible-more-widely-available` | 62581 | `5eac73645c5444a0f68c28678de08ac5ff7224c8d72d681bad82adabdb7a8dc3` |

---

## 21. Related files

- `visible_IMPLEMENTATION_INSPECTION.md` — inspection mode, static findings, claim reconciliation
- `PROMPT_2.md` — the candidate-specific second-stage prompt
- `visible_COMPARISON.md` — the 14-section comparison against v1 and V2
- `../PROCESS_STATE.md` — sequencing, authorization decisions, machine-lifecycle facts
- `../RESEARCH_METHOD.md` — evidence classes, rule register, recovery procedure
- `../CANDIDATE_SHORTLIST.md` — why Visible was selected as candidate 11
