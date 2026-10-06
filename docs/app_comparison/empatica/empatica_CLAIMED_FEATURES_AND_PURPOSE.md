# Empatica EpiMonitor — claimed features and purpose

Candidate 7 of 10 (`CANDIDATE_SHORTLIST.md` rank 7, score 14).
Research date **2026-10-06**. Prepared for `PROMPT1.md` Phase 1, and as the evidence base for
`PROMPT_2.md`.

**Evidence classes used throughout.** `[PUBLIC CLAIM]` the vendor's own published statement ·
`[REGULATOR RECORD]` an entry in an independent regulator's database or a regulator-published document
submitted by the vendor · `[THIRD PARTY]` an independent, non-regulator source · `[INFERENCE]` my
reasoning · `[UNKNOWN]` no evidence found.

**Nothing in this file is a runtime observation.** No Empatica app was installed or run. No account was
created. No Empatica API was called. No device was held. Every statement below is either the vendor's own
published words, an independent regulator record, or an explicitly-labelled inference.

**Artifact hashes.** Every fetched artifact is SHA-256 hashed in §14 so the pages this file quotes can be
identified exactly. Web pages change; a claim without a hash is not reproducible.

---

## 1. Executive summary

**Empatica's EpiMonitor is the only candidate in this package that is a regulated medical device with an
alert path whose failure mode is a person not receiving help.** That is why it is on the shortlist: it is
the closest existing comparator for AnxietyWatch2's `cns-engine` + supervised-alarm ambition, and the
clearest illustration of the regulatory, evidentiary and *contractual* burden that a clinically-serious
alerting product carries.

Five findings, in order of consequence for the comparison:

1. **The headline accuracy number is not the number in the regulatory record.** The vendor's page states
   the algorithm "detects possible seizures with 98% accuracy" and "a 98% detection accuracy against the
   gold standard of hospital-based EEG measurement" `[PUBLIC CLAIM]`. The FDA clearance document for the
   device reports **Positive Percentage Agreement and false-alarm rate per 24 hours**, not accuracy, and
   reports **"No clinical studies have been performed on the subject device"**; the performance of the
   shipped configuration is established by **bench testing for equivalence** to the predicate device
   (K181861) plus **retrospective re-analysis** of previously collected data. In the low-sensitivity mode
   that the EpiMonitor-specific submission was cleared for, adult EMU PPA is **1.000 uncorrected / 0.905
   corrected** in a subgroup of 12 patients and 17 seizures, and real-world PPA (from **Embrace2**
   hardware, not EmbracePlus) is **0.80–0.87** `[REGULATOR RECORD]`. The vendor's number is not thereby
   false; it is **a different statistic, from a different population, with no citation on the page**. This
   is the sharpest instance in the package of a headline metric that a reader cannot trace.

2. **The marketing promise and the licence are written by different hands.** The product page says the
   device is "designed to keep you and your loved ones safe". The EULA the same page links to says the
   vendor "**cannot guarantee that a call or text message will reach any Caregiver**", that the app "may
   fail to work with unexpected interruptions, including … loss of Internet connectivity, power outages,
   equipment failure or breakdown, telecommunication service provider failures", that the vendor "**DOES
   NOT REPRESENT OR WARRANT THAT THE APP, ITS USE, OR ANY RESULTS GENERATED THEREFROM ARE ACCURATE,
   COMPLETE, RELIABLE, CURRENT, OR ERROR-FREE**", that liability "**SHALL NOT EXCEED THE PURCHASE PRICE
   PAID**", and that the vendor is under no circumstance liable for "**PERSONAL INJURY OR DEATH**"
   `[PUBLIC CLAIM]`. Both are the vendor's own current documents. Recorded, not resolved.

3. **The regulator's definition of the device class states the limits the marketing pages do not.** The
   product code under which EpiMonitor is cleared is defined as a device that "**does not predict seizure
   onsets, and is not intended as a standalone seizure monitoring device**" `[REGULATOR RECORD]`. The
   vendor does not claim prediction (the forecasting work is labelled as a study, correctly). The
   standalone caveat is the one that matters for an alerting comparison, and it is **not** repeated on
   the marketing pages read: the pages state the detection scope ("not designed to detect seizures that do
   not generate movements") and the operational requirements (phone within 15 feet, Bluetooth, internet),
   but not "do not rely on this alone".

4. **The alert path is the product, and it has a dependency chain the user must assemble.** Seizure
   detection → companion app on a **dedicated** phone within **15 feet** → Bluetooth → internet → the
   vendor's servers → **a phone call and an SMS to each caregiver**. Caregiver count is **plan-gated**
   (1 / 3 / 10), as are settings and reports. US purchase requires a **prescription**. Device **$399** plus
   a subscription from **$17.40/mo** (Basic), $26.90 (Standard), $52.90 (Plus) `[PUBLIC CLAIM]`. There is no
   self-hosted, no-account, no-cloud variant: the alarm cannot exist without the vendor's infrastructure.

5. **The privacy notice for the device service reads like an advertising-technology notice.** The US
   privacy notice that expressly covers "the EmbracePlus watch, the associated App, and the associated
   EpiMonitor Account page" is dated **1 December 2024**, and in addition to health data it describes
   **Google Analytics**, **pixel tags**, **cross-device tracking**, **advertising networks**, and sharing
   "customer list information (such as your name, email address and other contact information)" with third
   parties for ad targeting `[PUBLIC CLAIM]`. Whether that language is intended to apply to the watch data
   or only to the web Portal is **ambiguous in the document itself**; the most conservative reading
   governs, and the document was last updated 22 months before this research date.

**Why this candidate matters to AnxietyWatch2 and not to v1.** EpiMonitor's physiology is seizure
detection, not anxiety; its user is a person with epilepsy and their caregivers. Almost nothing about its
*feature set* transfers. What transfers is that it is the only shortlisted product that has answered, in
public and with a regulator's involvement, the questions AnxietyWatch2's alarm layer will have to answer:
what a false alarm costs, what a missed alarm costs, what happens when the phone is out of range, what the
product is willing to promise in writing, and what it says instead when the promise fails.

---

## 2. Positioning and identity

| Field | Value |
|---|---|
| Product | EpiMonitor (watch product: **EmbracePlus**) |
| Vendor | Empatica Inc. (Cambridge MA, US) and Empatica S.r.l. (Milan, IT); vendor is the App Store seller |
| Category as the vendor frames it | "Next-Gen Epilepsy Monitoring"; "the only medical watch available for epilepsy monitoring in the world" |
| Core claim | Continuous wear + automated detection of generalised tonic-clonic seizures + caregiver alerting by phone call and SMS |
| Ancestry | Successor to **Embrace2** (2018 clearances); trade-in programme for Embrace2 owners |
| Regulatory identity | FDA-cleared (510(k)) and CE-certified as a Class IIa medical device under EU MDR 2017/745 |
| Adjacent product line | The Empatica Health Monitoring Platform — a research/clinical-trials wearable platform with a Cloud API, digital biomarkers, actigraphy and a Parkinson's monitoring line |
| Corporate footprint | Empatica Inc., 1 Broadway, 14th floor, Cambridge MA 02142; Empatica Srl, Via Stendhal 36, 20144 Milano; ISO 13485 certificate no. 39050072201 |
| Availability claim | US, UK, EU, AU, NZ and CA |

`[PUBLIC CLAIM]` unless noted. The dual `Inc.`/`S.r.l.` structure recurs throughout: the **FDA clearance
holder is Empatica S.r.l.** (the Italian entity) while the iOS seller is also **Empatica S.r.l.** and the
privacy notice is issued by **Empatica, Inc.** `[REGULATOR RECORD]`. For a data-protection question that
is not a formality — the contracting and data-controlling entities differ from the corporate face of the
marketing site.

## 3. Corporate, legal and regulatory identity

**Independently verified clearances.** The openFDA 510(k) database returns **nine** records for applicant
"Empatica" `[REGULATOR RECORD]`:

| K number | Decision date | Product code | Device name |
|---|---|---|---|
| K172935 | 2018-01-26 | POS | Embrace |
| K181861 | 2018-12-20 | POS | Embrace |
| K221282 | 2022-11-20 | DQA | Empatica Health Monitoring Platform |
| K230457 | 2023-10-30 | MWI | Empatica Health Monitoring Platform; EmbracePlus; Empatica Care; Care Portal |
| **K232915** | **2024-02-15** | **POS** | **EpiMonitor** |
| K242737 | 2025-06-06 | MWI | Empatica Health Monitoring Platform; EmbracePlus; Empatica Care; Care Portal |
| **K250515** | **2025-06-19** | **POS** | **EpiMonitor** |
| K252981 | 2025-12-22 | LEL | EmbraceMini |
| K260374 | 2026-08-14 | GYD | Empatica Health Monitoring Platform for Parkinson's Disease Monitoring |

Product-code definitions `[REGULATOR RECORD]`, from the openFDA classification database:

| Code | Class | 21 CFR | Name | Definition (excerpt) |
|---|---|---|---|---|
| **POS** | II | 882.1580 | Physiological Signal Based Seizure Monitoring System | "a prescription device that uses physiological signal to identify abnormal physiological activity that may be associated with a seizure. **The System does not predict seizure onsets, and is not intended as a standalone seizure monitoring device.**" |
| DQA | II | 870.2700 | Oximeter | — |
| MWI | II | 870.2300 | Monitor, Physiological, Patient (Without Arrhythmia Detection Or Alarm) | — |
| LEL | II | 882.5050 | Device, Sleep Assessment | — |
| GYD | II | 882.1950 | Transducer, Tremor | — |

**What the vendor says, and what reconciles.** `[PUBLIC CLAIM]`: "510(k) FDA-clearance as part of the
Empatica Health Monitoring Platform ( **K221282 - K230457** )" and "CE-certified as a class IIa medical
device". **Reconciled:** K221282 and K230457 exist and match the quoted numbers. But the clearance that
names **EpiMonitor** is **K232915 / K250515**, which the vendor page does not cite. The vendor's "FDA
Cleared" badge is therefore accurate in substance and **incomplete in citation** — a reader checking the
numbers the vendor supplies would find the *platform* clearances, not the *epilepsy* clearance.

**Two clearances the vendor does not advertise on the pages read:** EmbraceMini (K252981, sleep
assessment, 2025-12-22) and a Parkinson's monitoring platform (K260374, tremor transducer, 2026-08-14).
Both are new product lines; the first is named on the corporate homepage navigation ("EmbraceMini wearable
New"), the second as "Parkinson's Monitoring New".

**Business model structure.** Hardware sale + **mandatory subscription** + prescription (US) + insurance
reimbursement pathway + charity partnership programme + trade-in discount. `[PUBLIC CLAIM]` The
"IMPORTANT!" notice on the store page — that the $50 forecasting-study discount requires creating a user
account and signing a consent form first — is a **research-consent gate fused to a purchase discount**.
Recorded without characterisation; it is the vendor's own wording.

## 4. Availability, platforms and hardware

- **Watch: EmbracePlus.** Sensors, per the vendor `[PUBLIC CLAIM]`: ventral electrodermal activity (EDA);
  4-channel multi-wavelength PPG; 3D accelerometer and gyroscope; digital skin temperature. Dimensions
  40 × 38.5 × 11 mm, 40 g. Silicone band for 95–222 mm wrists. IP67 (water-resistant to 1 m/3 ft for 30
  minutes). Bluetooth 5 "with encrypted pairing and data transfer". E-ink display for status and clock.
  On-board flash; "non-stop data recording synced with the Care or Care Lab App".
- **Battery and charging (EpiMonitor page):** "up to 7 days of battery life", full charge in up to 90
  minutes `[PUBLIC CLAIM]`.
- **Removable pod** with body adapter, belt clip, or universal wristband adapter.
- **Strap variants** and three colours; smaller/larger sizing.
- **Apps.** iOS: **EpiMonitor by Empatica** `com.empatica.healthmonitor.alert`, v**1.78.1**, released
  **2026-03-24**, 210,617,344 bytes, iOS 16.0+, rating **4.26 / 263 ratings**, seller **Empatica S.r.l.**,
  genre Medical `[THIRD PARTY]` (Apple's own lookup API, not vendor copy). Also **Empatica Care**
  `com.empatica.healthmonitor`, v**5.100.6**, released **2026-09-24**, 214 MB, iOS 17.0+, **3.1 / 10
  ratings** `[THIRD PARTY]`.
- **Android.** The vendor page states the app is "Available on: App Store (iOS), Google Play Store
  (Android)" `[PUBLIC CLAIM]`. **No Play Store listing could be located or retrieved at the research
  date** — four candidate package identifiers returned HTTP 404, and Play's own 403 behaviour on some
  non-browser clients means this is **not** evidence of absence. Recorded as `[UNKNOWN]`: the vendor
  claims Android availability; the listing was not verified.
- **Languages.** App available in 8 languages (English, Spanish, French, Italian, German, Dutch, Danish,
  Swedish) `[PUBLIC CLAIM]`.
- **Shipping.** Region selector with currently-shipping lists for the Americas, Europe (12 named
  countries plus UK), and Oceania. "Other regions: check availability."

**Release cadence observation.** The EpiMonitor iOS app's most recent release on the App Store at this
research date is **2026-03-24**, roughly six and a half months earlier. The **Empatica Care** app was
updated **2026-09-24** `[THIRD PARTY]`. For an FDA-cleared alerting product, a patient-facing app that has
not shipped an update in over six months is an observation about maintenance tempo, not a defect; it is
recorded because the comparison will need it.

### 4.1 Documentation and platform lifecycle — a withdrawal that is verifiable in public

This candidate is the first in the package where the vendor's **developer documentation was deleted**, and
where the deletion is exactly dateable.

- **`developer.empatica.com`** was Empatica's public developer documentation host — the CNAME of the
  GitHub Pages repository `empatica/empatica.github.io`. Internet Archive's CDX index records the
  historical content at that host: `android-sdk-tutorial-200-legacy.html`, `android-sdk-tutorial-100.html`,
  `empatica-android-sdk-javadoc.zip`, `E4_streaming_server_scheme.jpg` and older pages back to **2016-01-16**
  `[THIRD PARTY]`.
- **On 2025-02-18 the vendor deleted it.** Three commits were made to the repository that day, in order:
  `c42b9fdf` "Redirect to sunset page", `5f22b82f` "**Remove all the pages**", `b8e0aa31` "Restore CNAME
  file" `[THIRD PARTY]` (GitHub API, `empatica/empatica.github.io`).
- **Today `developer.empatica.com` serves a 264-byte redirect** to
  `https://www.empatica.com/research/e4-sunset/` (verified: HTTP 200, 264 bytes, both the hostname and the
  GitHub Pages URL return the identical bytes). The CNAME file still reads `developer.empatica.com`.
- **The sunset notice states the software was disabled, not merely unsold** `[PUBLIC CLAIM]`:
  "As of February 14th, 2025, the E4 and its software suite have officially been sunset"; "The E4 software
  suite (E4 connect, E4 manager, E4 realtime app, E4 streaming server, and E4 SDK) **can no longer collect
  and store data, rendering services inoperative**."
- **The GitHub organisation itself still exists but publishes no product code** `[THIRD PARTY]`: `empatica`
  (created 2015-05-29, 5 public repositories, 27 followers) — one original repository, the now-emptied
  documentation site, and **four forks** (`steps-amazon-s3-upload`, `cyclonedx-gradle-plugin`,
  `fat-aar-android`, `io.appium.settings`). There is no Empatica SDK, client, or app source published.

**Why this is recorded as a lifecycle fact rather than as a criticism.** Withdrawing an end-of-life
research product is legitimate; the vendor published a sunset notice and a migration path to EmbracePlus.
What is recordable is the shape of it: **the canonical developer hostname was emptied within four days of
the sunset date, the documentation is no longer available from the vendor at all, and the hostname now
serves marketing content.** For a research platform sold to clinical-trials customers, the entire public
record of the previous generation's data interface survives only in third-party archives, which are not a
vendor-published contract. This is the same failure mode that `oscar` and `sleephq` illustrate from other
directions (`docs/app_comparison/oscar/`, `docs/app_comparison/sleephq/`): a pipeline's upstream contract
can change or vanish with no obligation to the pipeline.

## 5. Pricing, plans and commercial model

Store page, `[PUBLIC CLAIM]`:

| Item | Price |
|---|---|
| EmbracePlus (device) | **$399** |
| Forecast-study participation discount | −$50 (requires account creation and signed consent before purchase) |
| Basic subscription | **$17.40/mo** (≈ $0.58/day) — "seizure alerts with 1 caregiver and … an automated seizure diary" |
| Standard | **$26.90/mo** (≈ $0.90/day) — "All the benefits of Basic plus up to 3 caregivers, adjustable sensing precision settings and alert locations" |
| Plus | **$52.90/mo** (≈ $1.76/day) — "Up to 10 Caregivers, accurate sleep and activity tracking and shareable PDF reports" |
| Embrace2 trade-in | up to $175 off, US and EU only |

Terms of purchase stated on the same page `[PUBLIC CLAIM]`:

- "**EpiMonitor requires an active subscription** alongside the EmbracePlus device to work."
- "EpiMonitor is an FDA-cleared medical device and you will need to upload a prescription to your order to
  have it shipped."
- The subscription is charged "immediately after device pairing or 21 days after the shipping date,
  whichever comes first."
- Cancellation: full refund before prescription upload; full refund of device and subscription within 30
  days of pairing or 45 days of delivery, conditional on returning the device; cancelling the subscription
  alone disables auto-renewal and leaves access until the period ends.
- Reimbursement "depends on your plan and coverage"; the vendor publishes a predetermination form.
- Free shipping; taxes and shipping applied at checkout.
- Device-side compatibility notes keyed to purchase date (23 June 2025 boundary) and to specific Embrace2
  generations.

**Features are plan-gated, and the gates include the alert path itself.** The caregiver count — i.e. how
many people can be reached when a seizure is detected — is determined by the subscription tier. "Features
may vary depending on the plan you choose" `[PUBLIC CLAIM]`. For a product whose stated purpose is
caregiver alerting, the number of reachable caregivers is a **commercial** parameter. Recorded as an
observation about the business model; it is also the single most transferable contrast with
AnxietyWatch2's planned multi-modal alerting, which is not subscription-gated.

**The subscription plan support article could not be read.** `support.empatica.com` returned HTTP 403
(Cloudflare) for the "EpiMonitor Subscription Plans" article at the research date `[UNKNOWN]`. Plan
details above are therefore from the store page only, and any annual pricing or feature differences in
that article are not captured.

## 6. Sensors and measured signals

| Signal | EpiMonitor / EmbracePlus | Notes |
|---|---|---|
| Electrodermal activity (EDA) | Yes — "ventral" placement | The vendor's most distinctive sensor; third-party testimonials on the page emphasise it |
| PPG | Yes — 4-channel, multi-wavelength | Heart rate and derived measures |
| Accelerometer + gyroscope | Yes — 3D MEMS | Motion, sleep, activity, seizure detection |
| Skin temperature | Yes — "digital temperature sensor" | Peripheral skin temperature |
| Raw data access | Claimed: "raw data and an extensive range of digital measures"; "No black box algorithms … one of the few wearable devices that capture raw data" | See §12 for the conflict with "AI-powered" on the same page |
| Seizure detection | Algorithm **EpiAlgo** (v2.1 in the inspected clearance), on-watch | Detects generalised tonic-clonic seizures ≥ 20 s |
| Sensor modes | "predefined configurations … to focus on collecting specific parameters that are key to your study" | Available through the research platform; the public page says "Talk to our team" |

**Comparison note.** AnxietyWatch2 collects none of EDA, PPG or skin temperature. The overlap is
**architectural rather than physiological**: a body-worn sensor, a trained detection layer, and an
independent alert path to a third party. Nothing about EpiMonitor's sensor set transfers to
AnxietyWatch2's device list (AirSense 11 BLE, EMAY SleepO2, Polar H10, Acconeer XE125).

## 7. Derived measures and biomarkers

- **"Over 300 digital measures"** available through the platform `[PUBLIC CLAIM]`. The public pages do not
  enumerate them; the count is unverifiable from the surfaces read, and no measure definitions, units or
  ranges were published on the pages fetched. `[UNKNOWN]`
- **Named on the EpiMonitor page:** advanced sleep metrics (total sleep, effective sleep, efficiency,
  fragmentation); daily activity metrics (active time, steps, activity intensity); seizure diary
  (self-reported seizures, potential triggers, rescue medications, duration, mood); summary, daily,
  historical and trend views.
- **Named on the EmbracePlus page:** "digital biomarkers", "vitals, activity, sleep"; an actigraphy line
  sold as "the new standard to measure sleep and physical activity"; "validated PKG measures" for
  Parkinson's.
- **The alerting metric is not a derived biomarker but an event decision:** detection of "unusual patterns
  that may be associated with generalized tonic-clonic seizures lasting longer than 20 seconds"
  `[PUBLIC CLAIM]`.

**Undefined value domains.** As in `sleephq`, the vendor publishes names of derived quantities without
definitions, units, windows or thresholds on the pages read. This is an observation about the public
surface only: the research platform's Cloud API documentation (not fetched) may define them, and the
regulator documents define the alerting metrics precisely (§12). Nothing in this section should be read as
a claim that the measures are undefined *internally*.

## 8. The alert architecture — the comparison core

This is the part of the candidate that matters, so it is quoted in full.

**The vendor's own description of the full chain** `[PUBLIC CLAIM]`:

> "When the EpiMonitor algorithm detects unusual patterns that may be associated with generalized
> tonic-clonic seizures lasting longer than 20 seconds, it will communicate with the companion EpiMonitor
> App installed on the user's dedicated smartphone to alert active caregivers with a phone call and a text
> message. It is designed to expedite assistance when a seizure occurs."
>
> "For the EpiMonitor system to function as designed, watch wearers are required to have a compatible
> smartphone with Bluetooth and internet connectivity. **The phone needs to be kept within 15 feet of the
> EmbracePlus watch for alerts to be sent.**"

**Features listed on the product page** `[PUBLIC CLAIM]`:

- **Automated seizure alerts** — "sends a call and SMS to your caregivers when it detects a possible
  generalized tonic-clonic seizure".
- **Self-triggered alerts** — "Manually initiate an alert to contact your caregivers when you feel you
  might need help."
- **Easily cancel false alarms** — "If an activity is mistaken for a potential seizure, easily cancel
  false alerts before your caregivers are contacted." (i.e. there is a cancellation window between
  detection and caregiver contact.)
- **Adjustable sensing precision** — "Adjust the sensing precision or switch off the detection algorithm
  completely to fit your lifestyle, and reduce the chance of false alerts further."
- **Alert locations** — a Standard-tier feature.

**Detection scope, stated as a limit** `[PUBLIC CLAIM]`:

> "The EpiMonitor algorithm is designed to detect unusual patterns associated with generalized
> tonic-clonic seizures lasting 20 seconds or longer. **It is not designed to detect seizures that do not
> generate movements such as complex partial seizures, absence seizures or non-convulsive seizures.**
> However, you can manually add other types of seizures to the EpiMonitor App to keep track of all seizure
> events."

**Age range** `[PUBLIC CLAIM]`: "The EpiMonitor system is FDA-cleared for adults and children ages 6 and
up. Use in children younger than 6 should be evaluated by a clinician on a case-by-case basis."

**What the EULA adds, in the vendor's own words** `[PUBLIC CLAIM]`, from the agreement linked on the
product page:

- "You must have a Device that is compatible with the App … EMPATICA does not warrant that the App will be
  compatible with all mobile devices."
- "The User is solely responsible for compliance with applicable data protection laws when providing the
  Caregiver(s) contact information and for verifying that the Caregiver(s) are willing, able, and
  available to accept and to receive these calls and/or text messages."
- "EMPATICA cannot verify or influence and shall not be liable in respect of any conduct of the
  Caregiver(s). The User shall indemnify and hold harmless EMPATICA from any claim … of the Caregiver(s)."
- "You acknowledge and agree that, while EMPATICA intends for the App's features to be highly reliable and
  available, **EMPATICA cannot guarantee reliability. Empatica cannot guarantee that a call or text
  message will reach any Caregiver.** You acknowledge and agree that the App may fail to work with
  unexpected interruptions, including, without limitation, due to reasons such as loss of Internet
  connectivity, power outages, equipment failure or breakdown, telecommunication service provider
  failures, and third party software or hardware issues."
- Section 10, Limitation of Liability: "**EMPATICA'S MAXIMUM LIABILITY … SHALL NOT EXCEED THE PURCHASE
  PRICE PAID (IF ACTUALLY PAID) BY THE USER FOR THE APP'S LICENSE.** TO THE MAXIMUM EXTENT PERMITTED BY
  THE APPLICABLE LAW, UNDER NO CIRCUMSTANCE SHALL EMPATICA … BE LIABLE FOR **PERSONAL INJURY OR DEATH**,
  OR ANY INCIDENTAL, SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES WHATSOEVER."
- "EMPATICA DOES NOT REPRESENT OR WARRANT THAT THE APP, ITS USE, OR ANY RESULTS GENERATED THEREFROM ARE
  **ACCURATE, COMPLETE, RELIABLE, CURRENT, OR ERROR-FREE**."
- Registration: "You need not register with us to simply download and install the App. However, in order
  to access certain features of the App, **you must successfully register for an account**."
- Support: "EMPATICA shall not be obliged to support obsolete versions of the App or to update or provide
  versions of the App for use on any particular mobile device or operating system."
- Unilateral change: "EMPATICA also reserves the right, in its sole discretion, to modify or change the
  terms of this Agreement at any time, and without prior notice to User."

**Reading this fairly.** Every clause above is standard consumer-software practice, and a product cannot
warrant a phone network. The finding is not that the clauses exist; it is that **the marketing page and
the licence state opposite postures about the same capability, on the same day, and both are the vendor's
own current text**, and that a user who reads only the page receives the version with no caveat. The
package's rule applies: record both, resolve neither, and let the most conservative govern.

**No "do not rely on this alone" statement was found on the marketing pages read.** Compare Nightscout,
which states it explicitly and repeatedly (`docs/app_comparison/nightscout/`). The obvious place for such a
statement is the **EpiMonitor User Manual**, which the vendor publishes — and that search was **attempted
and blocked**, which is a stronger result than not looking:

- `https://www.empatica.com/manuals/` lists an **EpiMonitor User Manual** with a Download control; the
  `https://empatica.com/go/epimonitor-user-manual` link 302s to a 178-page PDF on `cdn.sanity.io`.
- The file was retrieved **in full — 112,765,579 bytes (112.8 MB), 178 pages**, created 2025-05-16 with
  Adobe InDesign, page size 424.677 × 424.677 pt, `Tagged: yes`, not encrypted.
- **It contains no text.** `pdffonts` returns an empty font table — the document has **no font
  resources at all**; `pdftotext -layout` and `pdftotext -raw` across all 178 pages return **178 bytes**
  (one form-feed per page, i.e. nothing); the only XObject subtypes present are `Image`, `XML` and `Form`,
  and the only filters are `FlateDecode` and `DCTDecode` (JPEG). Every page is a raster image.

**Consequences, all directly verifiable.** The published instructions for use of an FDA-cleared medical
device are (a) **not searchable**, (b) **not readable by a screen reader** — there is no text layer and no
font resource to expose one — and (c) **not machine-checkable**, which is precisely why this file carries
no rows in the reconciliation table in §12. It is also a 112.8 MB download for a document a caregiver may
want on a phone.

**And consequently: the question stands unanswered.** Whether the vendor tells users not to rely on the
alert alone remains `[UNKNOWN]`, and it is `[UNKNOWN]` *because the artifact exists and could not be read*,
not because no artifact exists. No OCR tool is installed in the research environment, and OCR of 178
image pages is a separate decision with its own error rate. This is recorded as the candidate's **top
evidence gap**.

## 9. Data model, API and integration

- **Account required** for the features that matter (`[PUBLIC CLAIM]`, EULA).
- **The research platform publishes a "Cloud API for Clinical Trials"** at `https://www.empatica.com/cloud-api/`
  (HTTP 200, 357,176 bytes, fetched). It is a **marketing and architecture page, not a reference**: **no
  OpenAPI document, endpoint list, model definition, authentication scheme, rate limit or error catalogue
  is published**, and the vendor states the contract is delivered commercially — "**All Cloud API clients
  will be provided with full developer documentation**" `[PUBLIC CLAIM]`. The same page directs enquiries to
  "Talk to our team". This is the sharpest difference from `exist` and `sleephq`, which publish their output
  contracts for anyone to read: Empatica's data interface is sold to research customers, not published, so
  for this candidate **the inspection mode cannot be `Performed`** on any API surface.
- **The published data path** `[PUBLIC CLAIM]`, quoted as the page diagrams it: **EmbracePlus** ("continuously
  measures and records physiological parameters, which are sent via Bluetooth® to a dedicated mobile app") →
  **Care App** ("uses the phone's internet connection to automatically transfer all data to the Empatica
  Cloud") → **Empatica Cloud** ("securely stores data and makes it available for transfer to your preferred
  system") → **Your Cloud** ("Track and manage participant data and activities using your existing clinical
  trial management software"). This is the same dependency chain as the consumer alert path in §8, and the
  vendor states it in its own architecture diagram: **no local, offline or self-hosted variant exists.**
- **The output contract, as far as it is published** `[PUBLIC CLAIM]`: "The Empatica Health Monitoring
  Platform uses two formats of files, **Avro and CSV**, which can be accessed and read by the world's most
  popular tools, libraries, and software. **Data will be available in S3.**" The transfer mechanism is
  **object storage holding Avro/CSV files**, not a REST/JSON API — a materially different integration shape
  from every other candidate in this package, and one that matters to the comparison: a consumer cannot poll
  a current value, they receive a dataset.
- **Named integration capabilities** `[PUBLIC CLAIM]`: "Use your existing participant IDs to log participants
  into EmbracePlus and Empatica's Care App"; "View and modify a participant's status, and **remotely end the
  data collection** with EmbracePlus"; "Monitor **wearing detection** information to understand your trial
  compliance and if intervention is needed". A remote stop-collection control and a wearing-detection signal
  are operational features no other candidate in this package publishes.
- **Named compliance claims on this page** `[PUBLIC CLAIM]`: "Advanced data security and privacy to protect
  sensitive health data and ensure compliance with industry standards including **HIPAA, GDPR and ISO**";
  "Cutting-edge FDA-cleared software and hardware, with access to all the latest features whilst ensuring
  **retrocompatibility**"; "40+ languages"; "160+ countries with over 100,000 devices deployed". Stated
  without qualification, certificate numbers or audit references on that page.
- **Portals named:** Care Portal, Care Lab Portal, Research Portal — login links on the public site.
  `[PUBLIC CLAIM]`
- **Export:** EpiMonitor "Plus" tier includes "shareable PDF reports"; the EpiMonitor diary can be
  "Export[ed] … into a PDF format, to discuss with your doctor" `[PUBLIC CLAIM]`. PDF export is
  **plan-gated** on the Plus tier — the same feature is described as generally available in the diary
  section and as a Plus benefit in the plan comparison. A minor internal surface conflict; the most
  conservative reading (plan-gated) governs.
- **No self-hosting and no open-source component.** The GitHub organisation `empatica` exists (created
  2015-05-29, 5 public repositories) but publishes **no product code**: one original repository — the
  documentation site, emptied on 2025-02-18 (§4.1) — and four forks of unrelated third-party projects
  `[THIRD PARTY]`. Searched 2026-10-06; the absence of an SDK or client source is therefore a *checked*
  absence from the vendor's named organisation, not an assumption.
- **The privacy instruments for the research platform are separate from the consumer ones**
  `[PUBLIC CLAIM]`: the platform has its own privacy policy, a **participant** policy, and a **Data Processing
  Agreement**; the US EpiMonitor notice says additional notices "will control to the extent there is a
  conflict with this Notice". A research participant's data and a patient's data are governed by different
  instruments from the same vendor.
- **Integration claims:** "effortless integrations for CROs" (contract research organisations); eCOA
  (electronic clinical outcome assessment); actigraphy; Parkinson's digital endpoints. All are
  `[PUBLIC CLAIM]` without published interfaces.

## 10. Privacy and data handling as published

**Document:** "Empatica, Inc. Privacy Notice", **last updated 1 December 2024**, US version, fetched from
`empatica.com/epimonitor/privacy-policy-us` `[PUBLIC CLAIM]`.

**Scope, in the notice's own words:** "this Notice applies to the personal information Empatica collects
and processes related to your use of **the EmbracePlus watch, the associated App, and the associated
EpiMonitor Account page ("Portal")**". So the ad-technology language below sits in a notice that
expressly covers the **medical device service**, not only a marketing website.

**Health data collected** (verbatim, excerpted): "physiological information, physical condition(s),
biometrical information (e.g., accelerometer, temperature, EDA data), computed biomarkers (e.g., sleep and
movement), and medications you take."

**Disclosures:**

- "Global affiliates, subsidiaries, branches, or associated offices."
- "**Vendors and service providers.** … service providers that host or operate the Services, payment
  processors, analytics providers, information technology service providers, communication service
  providers, customer service vendors, consultants, auditors, and legal counsel."
- "**With permission.** … where you have a designated caregiver, we may share information with that
  caregiver."
- "**Aggregate and De-Identified Information.** We may use and disclose aggregate and other
  non-identifiable data related to our business and the Services for quality control, analytics, research,
  development, and other purposes," with a commitment not to attempt re-identification except to test the
  de-identification.

**Purpose limitation, as written:** the notice lists purposes that include "**Marketing, advertising, and
public relations**", "**Research, surveys, and product development**", "Analytics and improvement", and
"internal quality control and training".

**Tracking technologies** (this is the finding):

- "We and our providers use cookies, **pixel tags**, and other similar tracking technologies …"
- "**Analytics.** We use analytics tools, such as **Google Analytics** …"
- "**Cross-Device Tracking.** We and ad networks we work with may use the information we collect about you
  within our Portal, and on other third-party websites and services, to help us and these third parties
  identify other devices that you use …"
- "**Advertising Networks.** … We **may share certain information with ad networks**, and we may each use
  cookies, pixel tags, and other tools to collect usage and browsing information within our Services, as
  well as on third-party websites, apps, and services. This information may include **IP address, location
  information, cookie and advertising IDs**, and other identifiers, as well as browsing information."
- "**Custom Lists and Matching.** We may share or make available certain customer list information (such
  as your **name, email address and other contact information**) with third parties (i) so that we can
  better target ads and content to you across third party sites, platforms and services, and (ii) in some
  cases, these third parties may help us to enhance our customer lists with additional demographic or
  other information …"

**Retention:** "We retain your personal information for as long as reasonably necessary to fulfill the
purposes for which it was collected or as otherwise necessary to comply with our legal obligations,
resolve disputes, maintain appropriate business records, and enforce our agreements." No period, no
category-specific schedule. Compare `sleephq`, which also publishes no retention period — this is the
second candidate in the package where retention is stated as a principle rather than a number.

**Children:** a dedicated section, relevant because the cleared age range starts at 6. It collects contact
info, health-related info and usage data from children, discloses to "third party vendors, service
providers, contractors or agents", shares "aggregate or de-identified information about users with third
parties for marketing, advertising, research or similar purposes", and points to a "Third-Party Operators
page" listing parties that collect persistent identifiers.

**Jurisdictional annexes:** Connecticut and Nevada sections in this document; a separate **Consumer Health
Data Privacy Policy** for Washington residents; separate EU (7 languages), UK, US and "WA-US" EpiMonitor
policies; separate research-participant and marketing notices; and a Health Monitoring Platform (PKG)
policy set. **The research-and-clinical-trials ecosystem has its own notice family**, which matters
because the `/pkg/` policies cover the platform that generates the "digital biomarkers" line.

**Second ambiguity, recorded not resolved.** The notice says "Additional Notices … will control to the
extent there is a conflict with this Notice". Whether the ad-network and cross-device language is intended
to reach *watch-derived health data* or only the web Portal's browsing activity is not determinable from
the document. The conservative reading — that health data collected on a device paired to an app in the
same service may be processed under a notice that permits advertising and list-matching disclosures —
governs. This is a **documentation** finding, not evidence about actual processing.

**Third observation, on the research sites:** a Google Tag Manager container
(`GTM-PBTLHFCB`) is present on the public pages, and one observed link carried
`gtm_preview=env-1`. A tag manager container is a standard marketing instrument; its presence does not
establish what it loads. Recorded as an observed artifact of the pages fetched, with no inference about
the app.

## 11. Security and access control as published

- "Bluetooth 5 with encrypted pairing and data transfer" `[PUBLIC CLAIM]`.
- Account required for the features that matter; user is "solely responsible" for password
  confidentiality; must notify the vendor of loss, theft, disclosure or breach (EULA) `[PUBLIC CLAIM]`.
- The page's regulatory blurb claims the clearance process includes "an assessment of testing related to
  usability and **cybersecurity**" `[PUBLIC CLAIM]`. That is a statement about the FDA process, and it is
  accurate as a general statement of what a 510(k) review covers.
- **No published security whitepaper, no bug-bounty programme, no `security.txt`, no penetration-test
  report and no incident history were located** at this research date. **This is a narrower gap than it
  looks, and §12 records the correction**: `/legal/` publishes an **ISO/IEC 27001:2022** information-security
  certification statement and an itemised medical-device standards list including IEC 62304 and IEC 62366-1,
  and links an ISO 13485 certificate. What Empatica does not publish is any *technical* security description
  of the app, the cloud or the Cloud API. `[UNKNOWN]` for those, and absence of evidence here is not
  evidence of absence.
- **No published incident or breach history was located.** `[UNKNOWN]`
- **No published `security.txt` was found at `developer.empatica.com`** — the Internet Archive CDX index
  shows a 2023-07-31 crawler probe of `/.well-known/security.txt` with status `-` (no archived response),
  which is the absence of a capture, not evidence about the file. `[UNKNOWN]`
- **No readable instructions-for-use text.** The EpiMonitor User Manual is a 178-page image-only PDF with no
  fonts and no text layer (§8). Recorded here as well as there because it is simultaneously an accessibility
  and a verifiability fact.

Compare `sleephq`, which at least names Oneleet and states it is "progressing towards SOC 2 Type II"; and
`nightscout`, which publishes a security page, a security audit and a formal authorisation requirements
document. Empatica publishes the least security detail of the three candidates that handle health data at
this level of sensitivity — a fact with an obvious explanation (a regulated medical device's security
evidence lives in the clearance, not on the marketing site) and a practical consequence (a prospective
user cannot read any of it).

## 12. Clinical and regulatory claims

**The accuracy claim.** `[PUBLIC CLAIM]`, on the EpiMonitor product page:

> "Unlike consumer devices offering seizure detection apps, EpiMonitor has undergone rigorous validation
> and testing, demonstrating a **98% detection accuracy against the gold standard of hospital-based EEG
> measurement**."
>
> "EpiMonitor is designed to keep you and your loved ones safe, with an advanced algorithm that detects
> possible seizures with **98% accuracy**."

No citation, no confidence interval, no population, no mode, and no date accompany the number on the page.

**What the regulator's document for the device reports** `[REGULATOR RECORD]` — FDA 510(k) **K250515**
summary (Empatica Srl, Special 510(k), EpiMonitor; fetched and hashed):

- "**No clinical studies have been performed on the subject device.** Clinical studies were performed on
  EpiAlgo ver 2.1 to support clearance of the predicate device (K181861)."
- "Clinical performance of the EpiAlgo ver 2.1 on the EmbracePlus device was confirmed **by bench testing
  to be equivalent** to the performance of the Embrace device presented in K181861, when using the
  standard alerting threshold during rest activities (High-Sensitivity)."
- To assess the **Low-Sensitivity** threshold introduced in EpiMonitor, "**two retrospective analyses of
  previously collected clinical data** were conducted."
- Metrics reported are **PPA (Positive Percentage Agreement)** and **FAR (false alarm rate per 24
  hours)** — not accuracy.

| Table | Data source | Subgroup | N patients | GTCS | PPA | corrected PPA | 95% CI (PPA) |
|---|---|---|---|---|---|---|---|
| 1 | EMU, low-sensitivity | 6–21 | 12 | 19 | 0.895 | 0.791 | 0.619–0.925 |
| 1 | EMU, low-sensitivity | >21 | 12 | 17 | **1.000** | 0.905 | 0.891–0.917 |
| 3 | Real-world, low-sensitivity | 6–21 | 601 | 1157 | 0.87 | 0.86 | 0.78–0.92 |
| 3 | Real-world, low-sensitivity | >21 | 843 | 3625 | **0.80** | 0.77 | 0.64–0.87 |

| Table | Data source | Subgroup | N patients | #FA | #days | Overall FAR | 95% CI | Mean FAR | 95% CI |
|---|---|---|---|---|---|---|---|---|---|
| 2 | EMU | 6–21 | 80 | 62 | 88.94 | 0.70 | 0.41–1.06 | 0.91 | 0.44–1.57 |
| 2 | EMU | >21 | 61 | 43 | 152.68 | 0.28 | 0.15–0.46 | 0.33 | 0.17–0.53 |
| 4 | Real-world | 6–21 | 601 | 12808 | 37594.2 | 0.34 | 0.23–0.50 | 0.35 | 0.28–0.45 |
| 4 | Real-world | >21 | 843 | 14308 | 56389.1 | 0.25 | 0.22–0.30 | 0.29 | 0.26–0.33 |

- "NOTE: The longitudinal analysis of real-world data was based on analysis of sensor data captured using
  the **Embrace2** wearable device."
- The **High-Sensitivity** (rest/sleep) performance on EmbracePlus is supported by **bench equivalence** to
  the predicate, not by a measurement on the subject device reported in this document.
- Conclusion: "EpiMonitor is considered to be substantially equivalent to the predicate devices on the
  basis of safety and effectiveness."

**Reconciliation, stated carefully.**

1. **The vendor's "98%" is not visible in the document that cleared the device.** The nearest figures are
   an **uncorrected** PPA of 1.000 in a **12-patient, 17-seizure** adult EMU subgroup, and a corrected PPA
   of 0.905 for the same subgroup and 0.791 for the paediatric subgroup. A reader cannot reconstruct 98%
   from the reported numbers.
2. **This does not make the vendor's claim false, and it must not be described as false.** The 98% may come
   from the predicate's clinical studies (K181861), from high-sensitivity-mode data reported elsewhere, or
   from a published paper not located in this pass. The finding is precisely that **the page states a
   number with no traceable source, while an independent regulator document for the same device reports
   different metrics and explicitly records that no clinical study was performed on the device itself.**
3. **"Corrected PPA" is used without definition** in the regulator document excerpt read here. That is an
   undefined term in a decisive table — the same finding class as `sleephq`'s undefined value domains, in a
   document of higher evidentiary weight.
4. **The real-world numbers come from different hardware** (Embrace2) than the product being sold
   (EmbracePlus), and the document says so.
5. **FAR is the number the page omits entirely.** At the paediatric real-world rate, 0.34 false alarms per
   24 hours implies roughly **one false caregiver alert every three days** for a child wearing the device —
   and the vendor's own optional low-sensitivity mode exists specifically "to reduce false alarm
   incidents", which is the vendor conceding that false alarms are a real burden.

**Other clinical/regulatory claims** `[PUBLIC CLAIM]`:

- "Empatica is running a **first-of-its-kind study** to develop a **seizure forecasting algorithm** using
  real-world data … Recruitment … available to all US-based EpiMonitor users." Correctly framed as a study,
  not a feature — and consistent with the cleared indication, which excludes prediction.
- "Developed as part of projects with the **U.S. Department of Health and Human Services, NASA Human
  Research Program, and U.S. Army Medical Research and Development Command**."
- "Notable partnerships" and a "Technology you can trust" section; client testimonials from
  University of Milano-Bicocca, Massachusetts General Hospital, University of Aberdeen, Thomas More
  University of Applied Sciences.
- **ISO 13485 certificate, published as a PDF** `[PUBLIC CLAIM]` / `[THIRD PARTY]`: certificate no.
  **39 05 0072201**, issued by **TÜV Rheinland Italia S.r.l.** to **Empatica S.r.l.**, standard
  **UNI CEI EN ISO 13485:2021**, scope "Design and development, manufacturing process management, and placing
  on the market of wearable medical devices intended to monitor vitals and non-vital physiological
  parameters" and "software intended to monitor vitals and non-vital physiological parameters"; valid
  **2025-05-08 to 2028-05-24**; first certification 2016-05-25; audit report no. 7993858010MB25. The
  certificate carries the standard TÜV disclaimer that it "does not represent proof that the statutory
  requirements of the Directives 93/42/EEC, 90/385/EEC, 98/79/EC or Regulations (UE) 2017/745, (UE)
  2017/746 have been fulfilled".

**Two findings attached to that certificate, recorded fairly.**

1. **The link is mislabelled.** On `https://www.empatica.com/legal/`, under the heading "TERMS &
   CONDITIONS / Read our Terms & Conditions", the hyperlink resolves to this **ISO 13485 certificate PDF**.
   A reader clicking "Terms & Conditions" receives a quality-management-system certificate. Verified by
   following the link target in the page source. This is a documentation defect of the same class already
   recorded three times in this package (`oscar`'s `Pressure_90th`/`Pressure_95th` header relabel;
   `sleephq`'s `time_over_leak_threshhold`; `nightscout`'s `cleanentriesdb.js:22` copy-paste description).
2. **The vendor's own compliance artifact disclaims what the marketing page asserts, and both are the
   vendor's own text.** The EmbracePlus page states "CE-certified as a class IIa medical device"; the
   certificate Empatica itself publishes and links states it is not proof that MDR 2017/745 requirements
   have been fulfilled. **Every notified body writes that sentence, and it is correct** — an ISO 13485
   certificate is not an MDR conformity certificate, and the vendor almost certainly does hold the separate
   CE marking the page claims. The observation is therefore about **placement, not impropriety**: the
   artifact the vendor offers a reader as its compliance evidence cannot bear the claim the adjacent page
   makes, and the vendor does not publish the MDR certificate or notified-body details that would.
   Recorded, unresolved, most conservative governs.

**A published standards list.** `https://www.empatica.com/legal/` sets out an itemised "Safety" section
listing the international standards the products were "developed and tested according to" `[PUBLIC CLAIM]`:
ISO 14971:2019 (risk management); IEC 60601-1:2005/A1:2012 (basic safety and essential performance);
IEC 60601-1-2:2014 (electromagnetic disturbances); **IEC 60601-1-6:2010+AMD1:2013 (usability)**;
**IEC 60601-1-11:2015 (home healthcare environment)**; FCC 47 CFR Part 15 Subpart C §15.247;
ETSI EN 301 489-17 V3.2.4 and ETSI EN 300 328 V2.2.2; **IEC 62304:2015 (medical device software life
cycle)**; **IEC 62366-1:2015 (usability engineering)**; ISO 10993-1:2018 and ISO 10993-10:2010 (biological
evaluation).

**Three observations about that list, each with its caveat.**

- **The list contains a duplicated line**: "ISO 10993-1:2018 — Biological evaluation of medical devices —
  Part 1: Evaluation and testing within a risk management process" appears **twice consecutively** in the
  rendered page. A copy-paste duplication in a regulatory compliance list; directly observable, no
  interpretation required.
- **IEC 60601-1-8 — the alarm-systems collateral standard — does not appear**, while the standards for
  usability (60601-1-6, 62366-1), home healthcare use (60601-1-11), risk management (14971) and software
  lifecycle (62304) all do. **The list does not claim to be exhaustive** ("the following international
  standards"), 60601-1-8 governs alarm *signals generated by the equipment* rather than a caregiver
  notification sent from a cloud service, and Empatica's alerting is regulation-adjacent to that standard
  rather than governed by it. Recorded as an observation about a published list, **not** as a finding of
  non-compliance and not as a suggestion that anything is missing from the clearance.
- **A "Validation" section states** `[PUBLIC CLAIM]`: "Empatica validates its regulated products to ensure
  they are compliant and fit for their intended use. We also work with researchers, scientists, data
  analysts, physicians, and clinical trial sponsors to assist them with leveraging our product
  documentation in their own validation objectives." That is the vendor handing validation responsibility
  for third-party use to the third party — the same boundary-statement class as Nightscout's "not officially
  approved or regulated" and SleepHQ's medical disclaimer, but stated as a division of labour rather than a
  disclaimer.

**Information security certification** `[PUBLIC CLAIM]`: `/legal/` publishes an "Information Security"
section stating **ISO/IEC 27001:2022** certification, describing an ISMS, systematic risk evaluation, and
independent third-party audit. **This corrects an earlier reading of this candidate**: the security surface
is not absent — it is published on the legal page rather than as a security whitepaper. What is still not
published is a bug-bounty programme, a `security.txt`, a penetration-test report, an incident history, or
any technical security description of the app or the cloud. The gap is narrower than §11 first recorded, and
this file corrects it rather than leaving the earlier version standing.
- "the world's most advanced smartwatch for continuous health monitoring" and "the only medical watch
  available for epilepsy monitoring in the world" — unverifiable superlatives, recorded as marketing.

**The internal AI conflict.** On the **same EmbracePlus page**: "**No black box algorithms.** EmbracePlus
is one of the few wearable devices that capture raw data, with no black box algorithms" and, in the
remote-patient-monitoring section, "your medical-grade, **AI-powered** remote monitoring" `[PUBLIC CLAIM]`.
Both statements are about the same platform. They are reconcilable (raw data is exposed *and* an AI model
exists to interpret it; the watch's own detection is a deterministic algorithm) — but they are not
reconciled *on the page*, and a reader takes away whichever sentence they read first. Recorded as a
surface conflict; the more conservative statement (there is an AI component) governs.

**All surface conflicts recorded for this candidate, in one place.** Two of the four marketing pages read
contain an explicit internal contradiction, which is itself the finding:

| # | Surface A | Surface B | Where |
|---|---|---|---|
| 1 | "No black box algorithms" | "medical-grade, AI-powered remote monitoring" | **same page** (EmbracePlus) |
| 2 | EmbraceMini battery "at least 7 days" | EmbraceMini battery "14+ days" | **same page** (EmbraceMini) |
| 3 | EmbracePlus battery "up to 7 days" | EmbracePlus battery "up to 28 days" | across two pages (EpiMonitor vs Cloud API) |
| 4 | "designed to keep you and your loved ones safe" | "cannot guarantee that a call or text message will reach any Caregiver"; no accuracy warranty; liability capped at purchase price; no liability for personal injury or death | EpiMonitor page vs the EULA it links to |
| 5 | "CE-certified as a class IIa medical device" | the certificate the vendor itself links states it is "not … proof that the statutory requirements of … Regulations (UE) 2017/745 … have been fulfilled" | EmbracePlus page vs `/legal/` artifact — both vendor text |
| 6 | "98% detection accuracy", no citation | PPA and FAR with confidence intervals; "No clinical studies have been performed on the subject device"; real-world data from **different hardware** | EpiMonitor page vs K250515 |
| 7 | PDF reports described as generally available (diary section) | PDF reports are a Plus-tier benefit (plan table) | same page (EpiMonitor) |
| 8 | "300+ digital measures" (EmbracePlus / Cloud API) | "200+ digital measures" (EmbraceMini) | across product pages — **not recorded as a conflict**; different products, and each page is explicit about which is which |

In every case the **most conservative reading governs** and the conflict is **recorded, not resolved** — the
package's standing rule. The pattern is worth stating honestly rather than flattened: items 1–3 and 7 are
internal inconsistency on one page, which is an editorial defect; items **4–6 are cross-instrument** tensions
between a marketing page and a contract or a regulator filing, which are a different and more consequential
class. Item 8 is listed only to show it was considered and deliberately not counted.

**What the vendor says is *not* covered** `[PUBLIC CLAIM]`: non-motor seizures (complex partial, absence,
non-convulsive) are explicitly outside the detection design; children under 6 require case-by-case
clinical evaluation; the AirMini-style hardware limit does not arise here, but the **phone within 15 feet**
requirement and the **compatible-device** caveat are stated limits that the vendor does publish.

## 13. What Empatica / EpiMonitor does not do

Recorded explicitly, because the comparison draws on it:

- **No respiratory, sleep-apnoea, CPAP or oxygen-saturation product** on the pages read. Product code
  **DQA (Oximeter)** exists in the platform's clearance (K221282) and **LEL (Sleep Assessment)** in
  EmbraceMini's (K252981), so the *capability* appears in the regulatory record — but no oximetry or sleep
  product is sold on the EpiMonitor or EmbracePlus pages, and no SpO₂ or respiratory claim appears there.
  `[REGULATOR RECORD]` + `[PUBLIC CLAIM]`.
- **No blood pressure product.** `[UNKNOWN]` for any BP claim; none found.
- **No anxiety, panic, mood or mental-health monitoring product.** The indication is seizure monitoring.
  EDA is collected and sold to researchers; the vendor's own digital-mental-health user testimonial
  (Thomas More University of Applied Sciences) is about *research* data collection, not a consumer feature.
- **No self-hosting. No offline mode.** The alert path requires the vendor's infrastructure
  `[INFERENCE]` from the alert chain (`[PUBLIC CLAIM]`) — the app communicates with the vendor's servers to
  place calls and send SMS, and coverage/reliability clauses name "third party software or hardware
  issues".
- **No free tier.** Subscription is mandatory for the device to function.
- **No open data export as a first-class feature** on the consumer tiers; the permitted export is PDF, and
  PDF is plan-gated.
- **No published API for consumers**; the Cloud API is a research-platform commercial offering.
- **No environmental/context data** (weather, noise, aircraft) of any kind.
- **No correlation engine** published as a consumer feature. "Data-driven insights" refers to diary
  views and trends.

## 14. Evidence table and artifact hashes

### 14.1 Claim ledger

| # | Claim | Class | Where |
|---|---|---|---|
| E-1 | "98% detection accuracy against the gold standard of hospital-based EEG measurement" | `[PUBLIC CLAIM]` | EpiMonitor page |
| E-2 | "an advanced algorithm that detects possible seizures with 98% accuracy" | `[PUBLIC CLAIM]` | EpiMonitor page |
| E-3 | Alerts are "a phone call and a text message" to caregivers | `[PUBLIC CLAIM]` | EpiMonitor page FAQ |
| E-4 | Phone must be within 15 feet; Bluetooth and internet required | `[PUBLIC CLAIM]` | EpiMonitor page FAQ |
| E-5 | Detects GTCS ≥ 20 s; not non-motor seizures | `[PUBLIC CLAIM]` | EpiMonitor page FAQ |
| E-6 | FDA-cleared for ages 6+ | `[PUBLIC CLAIM]` | EpiMonitor page FAQ |
| E-7 | Prescription required (US territories) | `[PUBLIC CLAIM]` | EpiMonitor page, store page |
| E-8 | $399 device; $17.40 / $26.90 / $52.90 per month | `[PUBLIC CLAIM]` | Store page |
| E-9 | Caregiver count gated by plan (1 / 3 / 10) | `[PUBLIC CLAIM]` | Store page |
| E-10 | Active subscription required for the device to work | `[PUBLIC CLAIM]` | Store page |
| E-11 | "510(k) FDA-clearance … ( K221282 - K230457 )" | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-12 | CE-certified as Class IIa under EU MDR 2017/745 | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-13 | Sensors: ventral EDA, 4-channel multi-wavelength PPG, 3D accel/gyro, digital temperature | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-14 | IP67; Bluetooth 5 "with encrypted pairing and data transfer" | `[PUBLIC CLAIM]` | EmbracePlus page, store page |
| E-15 | "No black box algorithms" | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-16 | "medical-grade, AI-powered remote monitoring" | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-17 | "Over 300 digital measures" | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-18 | Up to 7 days battery; 90-minute charge | `[PUBLIC CLAIM]` | EpiMonitor page |
| E-19 | App in 8 languages | `[PUBLIC CLAIM]` | EpiMonitor page |
| E-20 | Trade-in up to $175 (US, EU) | `[PUBLIC CLAIM]` | EpiMonitor page, store page |
| E-21 | Seizure-forecasting study, US users, $50 discount | `[PUBLIC CLAIM]` | EpiMonitor page, store page |
| E-22 | Privacy notice covers watch, app and Portal; last updated 2024-12-01 | `[PUBLIC CLAIM]` | US privacy notice |
| E-23 | Health data collected: physiological, biometrical (EDA, temperature), computed biomarkers, medications | `[PUBLIC CLAIM]` | US privacy notice §"Personal Information Collected" |
| E-24 | Disclosures to affiliates, service providers, caregivers, de-identified recipients | `[PUBLIC CLAIM]` | US privacy notice |
| E-25 | Google Analytics; pixel tags; cross-device tracking; advertising networks; custom-list matching with name/email | `[PUBLIC CLAIM]` | US privacy notice §"Cookies and Tracking" |
| E-26 | Purposes include marketing/advertising and research/product development | `[PUBLIC CLAIM]` | US privacy notice |
| E-27 | Retention: "as long as reasonably necessary…"; no period | `[PUBLIC CLAIM]` | US privacy notice §"Retention" |
| E-28 | Children's section: collects data from children, discloses to vendors, shares de-identified for advertising | `[PUBLIC CLAIM]` | US privacy notice §9 |
| E-29 | Separate WA consumer-health, EU (7 languages), UK, US, research-participant and marketing notices; additional notices control on conflict | `[PUBLIC CLAIM]` | Legal page link inventory |
| E-30 | EULA: cannot guarantee a call or SMS reaches any caregiver | `[PUBLIC CLAIM]` | EULA §"reliability" |
| E-31 | EULA: results not warranted accurate, complete, reliable, current or error-free | `[PUBLIC CLAIM]` | EULA (disclaimer) |
| E-32 | EULA: liability capped at purchase price; no liability for personal injury or death | `[PUBLIC CLAIM]` | EULA §10 |
| E-33 | EULA: account registration required for certain features | `[PUBLIC CLAIM]` | EULA |
| E-34 | EULA: vendor may change terms without prior notice; no duty to support obsolete versions | `[PUBLIC CLAIM]` | EULA |
| E-35 | ISO 13485 certificate 39050072201 | `[PUBLIC CLAIM]` | Site footer |
| E-36 | 510(k) clearance referenced as part of a platform assessment covering usability and cybersecurity | `[PUBLIC CLAIM]` | EpiMonitor page |
| E-37 | EpiMonitor clearances K232915 (2024-02-15) and K250515 (2025-06-19), product code POS | `[REGULATOR RECORD]` | openFDA 510(k) |
| E-38 | POS definition: "does not predict seizure onsets, and is not intended as a standalone seizure monitoring device" | `[REGULATOR RECORD]` | openFDA classification, 21 CFR 882.1580 |
| E-39 | "No clinical studies have been performed on the subject device" | `[REGULATOR RECORD]` | K250515 summary, p.9 |
| E-40 | High-sensitivity performance established by bench equivalence to K181861 | `[REGULATOR RECORD]` | K250515 summary |
| E-41 | Low-sensitivity assessed by two retrospective analyses of previously collected data | `[REGULATOR RECORD]` | K250515 summary |
| E-42 | EMU low-sensitivity PPA 0.895 / 1.000 (corrected 0.791 / 0.905), n=12 per subgroup | `[REGULATOR RECORD]` | K250515 Tables 1 |
| E-43 | Real-world low-sensitivity PPA 0.87 / 0.80 (corrected 0.86 / 0.77), n=601 / 843 | `[REGULATOR RECORD]` | K250515 Table 3 |
| E-44 | FAR 0.70 / 0.28 (EMU) and 0.34 / 0.25 (real-world) per 24 h | `[REGULATOR RECORD]` | K250515 Tables 2 and 4 |
| E-45 | Real-world analysis used **Embrace2** sensor data, not EmbracePlus | `[REGULATOR RECORD]` | K250515 note |
| E-46 | "corrected PPA" used without definition | `[REGULATOR RECORD]` | K250515 |
| E-47 | Nine Empatica 510(k) records; EmbraceMini (LEL, 2025-12-22) and Parkinson's (GYD, 2026-08-14) exist | `[REGULATOR RECORD]` | openFDA 510(k) |
| E-48 | iOS EpiMonitor v1.78.1, released 2026-03-24, 4.26★/263, 210 MB, iOS 16+, seller Empatica S.r.l. | `[THIRD PARTY]` | Apple lookup API |
| E-49 | iOS Empatica Care v5.100.6, released 2026-09-24, 3.1★/10, iOS 17+ | `[THIRD PARTY]` | Apple lookup API |
| E-50 | Play listing claimed as available; no listing located or retrieved | `[UNKNOWN]` | four candidate identifiers returned 404 |
| E-51 | Plan-comparison support article unreadable (HTTP 403) | `[UNKNOWN]` | `support.empatica.com` |
| E-52 | No public OpenAPI/SDK/endpoint documentation located | `[UNKNOWN]` | surfaces fetched |
| E-53 | No vendor security whitepaper, bug bounty, security.txt or SOC 2 report located | `[UNKNOWN]` | surfaces fetched |
| E-54 | Google Tag Manager container `GTM-PBTLHFCB` present on public pages | `[THIRD PARTY]`-observed | page source |
| E-55 | No respiratory/SpO₂/CPAP product on the pages read, though DQA (Oximeter) and LEL (Sleep Assessment) appear in the platform's clearance record | `[PUBLIC CLAIM]`+`[REGULATOR RECORD]` | EpiMonitor/EmbracePlus pages; openFDA |
| E-56 | "No black box algorithms" and "AI-powered" coexist on one page | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-57 | "designed to keep you and your loved ones safe" | `[PUBLIC CLAIM]` | EpiMonitor page |
| E-58 | EULA: user solely responsible for caregiver contact data and its lawfulness | `[PUBLIC CLAIM]` | EULA |
| E-59 | Charities and reimbursement pathways offered; predetermination form published | `[PUBLIC CLAIM]` | EpiMonitor page |
| E-60 | "the only medical watch available for epilepsy monitoring in the world" | `[PUBLIC CLAIM]` | EmbracePlus page |
| E-61 | Cloud API data path: EmbracePlus → BLE → Care App → phone internet → Empatica Cloud → "Your Cloud" (CTMS) | `[PUBLIC CLAIM]` | `/cloud-api/` |
| E-62 | "All Cloud API clients will be provided with full developer documentation" — i.e. not published | `[PUBLIC CLAIM]` | `/cloud-api/` |
| E-63 | Output formats Avro and CSV; "Data will be available in S3" | `[PUBLIC CLAIM]` | `/cloud-api/` |
| E-64 | Remote stop-collection; wearing-detection monitoring; participant-ID login | `[PUBLIC CLAIM]` | `/cloud-api/` |
| E-65 | "compliance with industry standards including HIPAA, GDPR and ISO"; "40+ languages"; "160+ countries"; "over 100,000 devices deployed" | `[PUBLIC CLAIM]` | `/cloud-api/` |
| E-66 | EmbracePlus "up to **28 days** of battery life" (platform page) versus EpiMonitor page's "up to **7 days**" | `[PUBLIC CLAIM]` | `/cloud-api/` and EpiMonitor page |
| E-67 | `developer.empatica.com` documentation pages deleted 2025-02-18 in three commits ("Remove all the pages") | `[THIRD PARTY]` | GitHub API, `empatica/empatica.github.io` |
| E-68 | `developer.empatica.com` now returns a 264-byte redirect to `/research/e4-sunset/`; CNAME unchanged | `[THIRD PARTY]` | direct fetch + raw CNAME |
| E-69 | E4 and its software suite sunset 2025-02-14; "can no longer collect and store data, rendering services inoperative" | `[PUBLIC CLAIM]` | E4 sunset notice |
| E-70 | Empatica GitHub org: 5 public repos, 1 original (emptied docs site) + 4 unrelated forks; no product source | `[THIRD PARTY]` | GitHub API |
| E-71 | EpiMonitor User Manual: 178-page PDF, 112,765,579 bytes, created 2025-05-16, 424.677 pt square pages | `[PUBLIC CLAIM]` | `cdn.sanity.io` via `/manuals/` |
| E-72 | That manual has **no font resources and no extractable text**: `pdffonts` empty, `pdftotext` returns 178 bytes for 178 pages, only Image/XML/Form XObjects and FlateDecode/DCTDecode filters | `[THIRD PARTY]` (measured locally) | `pdffonts`/`pdftotext`/object scan |
| E-73 | `/legal/` publishes **ISO/IEC 27001:2022** information-security certification | `[PUBLIC CLAIM]` | `/legal/` |
| E-74 | `/legal/` lists ISO 14971:2019, IEC 60601-1, -1-2, **-1-6**, **-1-11**, FCC Part 15.247, ETSI EN 301 489-17, ETSI EN 300 328, **IEC 62304:2015**, **IEC 62366-1:2015**, ISO 10993-1:2018, ISO 10993-10:2010 | `[PUBLIC CLAIM]` | `/legal/` |
| E-75 | The ISO 10993-1:2018 line appears **twice consecutively** in that list | `[PUBLIC CLAIM]` | `/legal/` |
| E-76 | IEC 60601-1-8 (alarm systems) is absent from that list; the list does not claim completeness | `[PUBLIC CLAIM]` | `/legal/` |
| E-77 | "Read our Terms & Conditions" links to the **ISO 13485 certificate PDF**, not terms and conditions | `[PUBLIC CLAIM]` (link target verified) | `/legal/` source |
| E-78 | ISO 13485 certificate no. 39 05 0072201, TÜV Rheinland Italia, UNI CEI EN ISO 13485:2021, valid 2025-05-08–2028-05-24, first cert. 2016-05-25 | `[THIRD PARTY]` | certificate PDF |
| E-79 | That certificate states it "does not represent proof that the statutory requirements of … Regulations (UE) 2017/745 … have been fulfilled" | `[THIRD PARTY]` | certificate PDF |
| E-80 | "Validation … we work with researchers … to assist them with leveraging our product documentation in their own validation objectives" | `[PUBLIC CLAIM]` | `/legal/` |
| E-81 | Privacy inventory on `/legal/`: EHMP policy, EHMP **participant** policy, EHMP **Data Processing Agreement**, EpiMonitor per jurisdiction (US, WA, GB, EU×7), Parkinson's per jurisdiction, Embrace2, website | `[PUBLIC CLAIM]` | `/legal/` |
| E-82 | EmbraceMini: 4 sensors (ambient light, 3-axis accelerometer, gyroscope, magnetometer), 15 g, 43×14×12 mm, BLE, "200+ digital measures", FDA CLEARED badge | `[PUBLIC CLAIM]` | `/embracemini/` |
| E-83 | EmbraceMini battery stated as both "at least 7 days" and "14+ days" **on the same page** | `[PUBLIC CLAIM]` | `/embracemini/` |

### 14.2 Artifact hashes (SHA-256)

Fetched 2026-10-06 into `/tmp/aw2research/empatica/`.

| Artifact | SHA-256 |
|---|---|
| `web/empatica_com_embraceplus.html` | `48f616432a4a2e3530104b6c15d05ef8ec4b242dbd9751181c6045c212562018` |
| `web/empatica_com_epimonitor.html` | `cb2f283af331887427086570d96c38a0cf1cf6c0f8cdc98595460fadcc264b91` |
| `web/empatica_com_store_epimonitor.html` | `bdb7a9b95f5295b40b5b800b8ef2338d6a1041c9e2994b89205d61e6603b4150` |
| `web/empatica_com_epimonitor_privacy-policy-us.html` | `6a7903e8d59c811912e9de6e1af45d3828fd361a4883f14882592bca8c459c6a` |
| `web/eula.html` | `66be3a0ab865b3a4ce60153c108f86732ef50c4306d094a3f6ee6be9cabb8567` |
| `web/empatica_com_legal.html` | `1fb744154d84eeaf890676f8381f558f78357bbeec3b07392a4d68a9c2ffc1b7` |
| `web/empatica_com_epimonitor_seizure-forecasting.html` | `48131208534ac56ea467580408c878b62de7500a6742da693bdfbd9f7cc00c50` |
| `web/empatica_com.html` | `20a8b24ccd81cafd7b7c087e4c5de04b00ba3d9e31162099da49d15141aef3e6` |
| `web/empatica_com_privacy-policy.html` (404 page) | `ed21a2cc7763d644973cbf17846635312354aa52e39aced55a19d69f01614193` |
| `web/empatica_com_terms-of-service.html` (404 page) | `5a1e433428b1e5f3d97e99e16f0d6314fc47c198825500c1c090d38e281ba5d8` |
| `support_...EpiMonitor-Subscription-Plans.html` (403 body) | `3b22d81f4a755833b87a31c8ace80312a09a799f63cb0f907700a8551ed37fdc` |
| `fda_K250515.pdf` | `595c5048b848b4dfc9a0cd3ca113e3f078d1afc6a3be7764934fcd6a18cbfd9e` |
| `fda_K250515.txt` | `a6b880654bbde8a8bed4dcd2da0c3ee058c941bee073f5ed2da6a36302a7eb55` |
| `fda_empatica_all.json` | `f35cd9bdcd5e0bd714c97c55da2cb0d9004b5766ba60ebd172bd6757c2df50c7` |
| `itunes_epimonitor.json` | `73c66f5b1a4fcb2b48abb70ec909d2a12b9dccc0fe1cf53c1cd81eef31648cbc` |
| `web/empatica_com_cloud-api.html` | `6158d0eeb7b23675ca34056305eb81a92544b6ecd193feb9673b6975741ba67b` |
| `web/empatica_com_embracemini.html` | `092ce72adf516b5441a2cb5b424fc048dfdfd6c47f6502c4289629d87b8643e7` |
| `web/empatica_com_manuals.html` | `630614edbbc96b0c19f7fabd4865ec06c746f92f17c829cf819e7304d6d79c1a` |
| `web/empatica_com_legal2.html` (byte-identical to `empatica_com_legal.html`, fetched twice) | `1fb744154d84eeaf890676f8381f558f78357bbeec3b07392a4d68a9c2ffc1b7` |
| `web/e4_sunset.html` | `cedc9f92a6567835d8b4df136625248653778399f4cb015d91c221fe5a85ad0b` |
| `web/developer_empatica.html` | `1e5e48cd6df1640e2219385f4096b54a45f0bbcb441d71cf01b1bd83c88d8845` |
| `web/gh_pages.html` (identical bytes to `developer_empatica.html`) | `1e5e48cd6df1640e2219385f4096b54a45f0bbcb441d71cf01b1bd83c88d8845` |
| `web/epimonitor_manual.pdf` (112,765,579 bytes, 178 pages) | `4bd5eedda64871096b29e4d7c1e96bd15b9b78cfaede1c4d8a1392489f6a5702` |
| `web/empatica_tc.pdf` (ISO 13485 certificate) | `cf38a846502333c759e6900b2067a49ace34f79e3c6839df7b1a23b54b214b` |
| `fda_K232915.pdf` (404 body, not the clearance summary) | `e49125b35874947bf03fcee5bfc055000b543c7c38d17077056582e3e8d336c6` |

**Two notes on this table.** (1) `web/gh_pages.html` and `web/developer_empatica.html` are byte-identical:
the GitHub Pages URL and the custom domain return the same 264-byte redirect, which is how the CNAME chain in
§4.1 was confirmed rather than assumed. (2) `web/empatica_com_legal.html` and `web/empatica_com_legal2.html`
were fetched in separate passes and are also byte-identical — an independent confirmation that the page was
stable across the research window, which matters because two of §12's findings rest on it.

Text extractions (`*.txt`) are derived artifacts and are hashed in `/tmp/aw2research/empatica/` but are not
listed here; they are reproducible from the HTML with the extraction described in `RESEARCH_METHOD.md`.

## 15. Relationship to AnxietyWatch v1 and AnxietyWatch2

**Versus v1 (`../AnxietyWatch`).** No meaningful product overlap. v1 is an iOS app for anxiety/panic
self-tracking with a CPAP CSV import and HealthKit; EpiMonitor is a prescription wearable for epilepsy.
The only transferable element is a **category observation**: a v1 user who also has epilepsy would use
EpiMonitor as a *separate* system, and v1 could never be the alerting path for it. Recorded for
completeness, not as a competitive finding.

**Versus AnxietyWatch2 (`./`).** The overlap is the **CNS/alert layer** (shortlist dimensions C22–C24), and
it is structural:

| Axis | Empatica EpiMonitor | AnxietyWatch2 |
|---|---|---|
| Detection layer | EpiAlgo on-watch; cleared as a Class II prescription device | `cns-engine` — `[V2 target]` |
| Alert path | vendor cloud → phone call + SMS to plan-gated caregivers | `[V2 target]` multi-modal (amendment 03), supervised alarm, hardware dead-man switch |
| Alert-path dependency | dedicated phone within 15 ft, Bluetooth, internet, vendor servers, active subscription | local box, local network target |
| Accuracy evidence | FDA 510(k) with bench-equivalence + retrospective analysis; PPA/FAR reported | none; no AHI, no percentile computation, no derived clinical statistic in the registry |
| False-alarm handling | user-facing cancellation window, adjustable sensitivity, optional low-sensitivity mode | `[V2 target]` |
| Regulatory posture | FDA-cleared, CE Class IIa, prescription-only in the US | explicitly not a medical device |
| Thresholds | not user-configurable in the sense the registry means; detection sensitivity is a mode | `vitals/registry.toml`, per type, one source of truth |
| Missing/absent data | not published; `[UNKNOWN]` — and this is the package's sharpest open question for this candidate | three states; "not measured" is never rendered as measured |
| Data location | vendor cloud, mandatory account | owner's hardware |

**The four things AnxietyWatch2 should read from this candidate, whether or not it ever competes with it:**

1. **The regulator's document is the only place where the accuracy numbers have denominators.** Every
   claim in this package that involves a performance figure should be checked against an independent
   record where one exists. `[REGULATOR RECORD]` checks are now a standing sub-check (`PROMPT_2.md`).
2. **"Not intended as a standalone device" is the regulated industry's own version of the three-state
   rule's failure mode.** A cleared Class II seizure monitor is *required by its definition* not to be
   relied on alone, and the product that fails to say so on its marketing pages is not thereby unlawful —
   but a user reading only the pages is worse off.
3. **False-alarm rate is a first-class product metric, not a quality footnote.** EpiMonitor's optional
   low-sensitivity mode exists because false alarms are the burden that makes users turn the feature off.
   AnxietyWatch2's supervised-alarm design has not yet stated what an acceptable false-alarm rate is for
   any of its alert types — a gap this comparison makes visible.
4. **A liability cap and a marketing promise can coexist, and the honest comparison records both.** This is
   the third candidate in the package with a vendor surface that undercuts its own headline (`oscar`,
   `sleephq`, now `empatica`), and the first where the undercutting document is a **contractual instrument**
   rather than a blog post or an internal review.

## 16. Weaknesses, omissions and unverified scope

**In this file's evidence base:**

- The **Android listing** was not verified; the vendor claims it exists. `[UNKNOWN]`
- The **subscription-plans support article** (403) was not read, so annual pricing and per-plan feature
  differences come from the store page alone. `[UNKNOWN]`
- **The EpiMonitor User Manual was retrieved and cannot be read**: 178 pages, 112.8 MB, image-only, no fonts,
  no text layer (§8). It is the highest-value unread artifact in this candidate's evidence base, because it
  is the surface most likely to state the limits in plain language to the person wearing the device.
  Closing it requires an OCR decision, not another fetch. **Top residual uncertainty.**
- **The `/cloud-api/` page is the whole published contract**: no OpenAPI, no endpoints, no auth scheme, no
  error catalogue. The integration surface cannot be inspected from public artifacts, so **this candidate's
  inspection mode cannot be `Performed`** unless a client-facing document is lawfully obtained.
- **`/research/` returns 404** and the product set is now organised under `/cloud-api/`, `/embracemini/`,
  `/embraceplus/`, `/embrace2/`, `/epimonitor/`, `/manuals/`, `/legal/` and per-use-case pages
  (`/clinical-trials/`, `/actigraphy/`, `/parkinsons-monitoring/`, `/academic-research/`, `/sleep/`). Only
  the pages listed in §14.2 were read; the **use-case, actigraphy, sleep, CRO, academic-research, eCOA,
  digital-biomarkers, publications, case-studies, compliance and resources-library pages were not
  enumerated.** Nothing here is a complete account of the vendor's claims.
- **The Publications page was not consulted**, so any peer-reviewed paper that could source the 98% figure is
  unexamined. That is the most likely route to resolving §12 and is the first check `PROMPT_2.md` asks for.
- **Only four marketing pages, one store page, one privacy notice and one EULA were read.** Empatica's
  site has a large surface (research, clinical trials, actigraphy, Parkinson's, per-jurisdiction privacy
  notices, the PKG platform policy family, a blog, a resource library, a support centre) that was not
  enumerated. Nothing here should be read as a complete account of the vendor's claims.
- **The K232915 summary PDF could not be retrieved** (the FDA path returned 404 at this date); K250515 was
  retrieved in full and is the basis for §12. The two clearances are for the same device name and product
  code, so the discrepancy is a retrieval failure, not an evidential gap about the clearance's existence.
- **No Empatica publication was located** to source the 98% claim. A search of the peer-reviewed
  literature and the FDA's own summary documents for K181861 is the obvious next step and was not
  performed.
- **"Corrected PPA" is undefined** in the excerpt read; a reader cannot tell what correction was applied.
- **No vendor source, SDK or API document was found**; the **Cloud API for Clinical Trials** is named in
  navigation but is not public. The inspection mode for this candidate therefore cannot be
  `Performed` unless a published artifact is found, and `PROMPT_2.md` makes locating one the first check.

**Known vendor-side limitations stated by the vendor itself** `[PUBLIC CLAIM]`: no detection of non-motor
seizures; children under 6 require clinical evaluation; a phone within 15 feet is required; a compatible
phone is required; the app may not work on all devices; no support obligation for obsolete versions.

**Not to be inferred from this file.** Absence of a security whitepaper or a published API from the pages
fetched is **not** evidence that they do not exist. Absence of a "do not rely on this alone" statement
from four marketing pages is **not** evidence that the product never says it. The 98% claim is **not**
characterised as false; it is characterised as **untraceable from the page that makes it**, against a
regulator document that reports different metrics.
