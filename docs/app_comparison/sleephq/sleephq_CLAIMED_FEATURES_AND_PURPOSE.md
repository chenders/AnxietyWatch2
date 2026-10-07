# SleepHQ — claimed features and purpose

Candidate 5 of 10 in `docs/app_comparison/CANDIDATE_SHORTLIST.md`.
Research date: **2026-10-06**. All findings below are from public sources fetched on that date.

**Evidence classes used throughout.** Each claim is labelled with exactly one of:

- **public claim** — the vendor's own published statement (website, store listing, terms, privacy policy)
- **documented contract** — a machine-readable or structured interface specification the vendor publishes
  (here: the OpenAPI document). *A documented contract is not an observed behaviour.*
- **static-only finding** — read out of a published artifact, not from running anything
- **runtime observation** — measured by running software. **There are none in this file.**
- **inference** — my reasoning from evidence, explicitly labelled
- **unknown** — no evidence found in either direction

**A note on the two evidence classes that dominate this file.** SleepHQ publishes an OpenAPI document for
its public REST API. That document is unusually specific — it names fields, types and one hash algorithm.
It is therefore strong evidence about **what SleepHQ says its API returns**, and it is *not* evidence about
what the running service returns, what the analysis layer computes internally, or what the app does. No
SleepHQ endpoint was called. Every API-derived statement in this file is labelled **documented contract**.

---

## 1. Executive summary

SleepHQ is a **cloud CPAP-data platform**: upload your machine's SD card data (manually, over Wi-Fi, or
from a phone card reader), see it analysed on a dashboard, and share it with clinicians or a community.
It is made by **SleepHQ Pty Ltd** in Melbourne and Phillip Island, Australia, founded by **Nick Dunn** (a
sleep technologist) and **Adam** (a web developer), and it describes itself as the first cloud-based CPAP
data analytics platform.

It is the **closest commercial analogue on the shortlist to the combination AnxietyWatch actually cares
about**: AirSense 11 data plus overnight SpO₂ plus longitudinal statistics. It supports the AirSense 11
explicitly, and it has solved the *get the data off the machine without pulling the card every morning*
problem — with hardware rather than with Bluetooth.

Four findings shape everything that follows:

1. **It is a cloud platform with an account, a subscription and proprietary hardware.** Free tier, Pro at
   **US $14.90/month or US $149.00/year**, a **AUD $399** Magic Uploader, and a **AUD $299** O2 Ring Pro
   that is the only oximeter it supports. AnxietyWatch2's stated position is the opposite: private LAN,
   self-hosted, no account, no vendor hardware.
2. **Its data path from the AirSense 11 is the SD card, not the radio.** SleepHQ reads the card manually,
   from a phone via an On-The-Go card reader, or automatically through a Wi-Fi SD card in a dongle. It does
   not stream from the device. This is the single clearest architectural contrast with AnxietyWatch2's
   BLE ingestion, and it is a contrast of *method*, not of capability.
3. **The founder publicly argues that the CPAP machine's own AHI is unreliable** — that it under-counts
   hypopneas, and that in one worked example a machine-flagged AHI of 6.32 was 14.3 on manual review. This
   is the same doubt about device-scored AHI that underlies AnxietyWatch2's decision not to compute or
   publish an AHI at all. It is a **vendor statement in the vendor's own interest**, and its cited research
   is not linked (see §10.4, §16).
4. **SleepHQ publishes a documented API with an OAuth2 contract and a documented hash algorithm.** That
   makes it, like Exist, an integration candidate — and it makes its data model inspectable as a
   *documented contract* rather than as behaviour. It is the second candidate to reach this inspection mode.

SleepHQ is a **partial competitor to AnxietyWatch v1's CPAP/SpO₂ subsystem** and, for the AirSense 11
specifically, a **partial competitor to AnxietyWatch2's ingestion path by a different mechanism**. It is
not a competitor in the anxiety, panic, medication or self-hosted dimensions — it has no panic capture, no
medication tracking, no weather or aircraft context, and no on-premises deployment. Whether it can inform
anything about anxiety is **unknown**: nothing found on any SleepHQ property links CPAP data to anxiety,
mood or panic.

---

## 2. Positioning

| Field | Value |
|---|---|
| Product | SleepHQ |
| Vendor | **SleepHQ Pty Ltd** (Australia) |
| Founders | **Nick Dunn** (sleep technologist, biomedical science degree) and **Adam** (web developer), Melbourne and Phillip Island |
| Website | `https://sleephq.com/` (marketing at `www.sleephq.com`) |
| Shop | `https://shop.sleephq.com/` |
| Category | Health & Fitness / Lifestyle (App Store) |
| One-line self-description | "Your all-in-one platform for optimizing your CPAP journey" |
| Stated first-of-kind claim | "The first ever cloud based, cpap data analytics platform" |
| Self-reported scale | "Over 100k+ happy pappers"; privacy policy states **"Over 100,000 patients use SleepHQ in over 160 countries"** |
| Explicit non-claim | Play listing: **"SleepHQ is not a medical device and does not diagnose, treat, cure or prevent any medical condition."** |

The two scale figures are consistent with each other (100k users) and are **vendor-published**; neither is
independently audited. The Android install band is 10K+ (`com.sleephq.android`), which is two orders of
magnitude below the claimed 100k users — the plausible reconciliation is that most users are on web and
iOS, but that is an **inference**, not a measurement. The vendor publishes no user count per platform.

---

## 3. Identity, legal and governance

| Item | Finding | Class |
|---|---|---|
| Legal entity | SleepHQ Pty Ltd | public claim (App Store seller name; site footer) |
| Billing processor | **Stripe** — "SleepHQ subscriptions are processed securely by our billing partner Stripe" | public claim (Terms) |
| Security programme | "We are **working with Oneleet** to implement a formal security program, and are **progressing towards SOC 2 Type II certification**" | public claim (Privacy Policy) |
| Clinic-facing compliance posture | "Where we handle data on behalf of clinics, our controls are **designed to meet** HIPAA requirements" | public claim (Privacy Policy) |
| Penetration testing | "We engage independent security firms to perform regular penetration testing" | public claim (Privacy Policy) |
| Data-centre controls | "our data centres are protected by both digital and physical controls" | public claim (Privacy Policy) |
| Refunds | **"No Refunds on Subscriptions or products."** Credits at vendor discretion only | public claim (Terms) |
| Renewal | Auto-renewing; cancellation takes effect at end of current term; "we may still send you promotional communications... unless you opt-out" | public claim (Terms) |
| Price changes | "We reserve the right to adjust pricing" with notice | public claim (Terms) |

**The certification language matters and is recorded verbatim.** "Working with", "progressing towards" and
"designed to meet" are *not* certification claims, and the vendor does not make one. SleepHQ is therefore
recorded as **not claiming SOC 2 Type II certification** at the research date. A vendor stating that it is
*progressing towards* an attestation is the conservative form of the claim, and it should be read that way.

---

## 4. Availability and platforms

| Platform | Finding | Class |
|---|---|---|
| Web | Yes — "can be used on any device with a browser" | public claim |
| iOS | Yes — `com.sleephq.ios`, App Store id **6451459781**, "iPhone and iPad" | public claim |
| Android | Yes — `com.sleephq.android`, "all Android devices" | public claim |
| Desktop app | None found | unknown |
| Self-hosted / on-premises option | **None found.** The product is a hosted cloud service | public claim (by omission: no self-host option appears anywhere on the site, shop, terms or API reference) |
| API | Yes — public REST API with published OAuth2 reference | documented contract |

### 4.1 App store metadata

| Field | iOS (`com.sleephq.ios`) | Android (`com.sleephq.android`) |
|---|---|---|
| Version / updated | 3.0.34, **2026-10-05** | (not shown), updated **2026-09-13** |
| Initial release | **2023-11-09** | — |
| Minimum OS | iOS **18.1** | — |
| Size | 17,401,856 bytes (~16.6 MB) | — |
| Rating | **4.73★** from **204** ratings | **4.4★** from **76** reviews |
| Installs | — | **10K+** |
| Content rating | **17+** | "Everyone" (with "Users Interact") |
| Price | Free (account required) | Free (account required) |
| Declared data collection | — | **Personal info and Health and fitness** |
| Declared sharing | — | **"No data shared with third parties"** |
| Transit encryption | — | **"Data is encrypted in transit"** |
| Deletion | — | **"You can request that data be deleted"** |

The 17+ iOS rating is recorded as published; the store does not state its reason on the page fetched.

### 4.2 Release-note findings

The Android listing's "What's new" for 2026-09-13 is unusually candid and is quoted because it is evidence
about the product's maturity and about a feature category this comparison cares about:

> • Large SD card imports no longer run out of memory
> • **Fixed three crashes in the sleep alarm**
> • Health Connect now uploads to the account the data was recorded for, and Cancel works on the first-sync dialog
> • **You're now told when a night's recording fails to save**
> • The ring screen no longer stays lit all night during a sleep test
> • The O2 Ring S is recognised by name

Three things follow. **SleepHQ ships a "sleep alarm"** — an alerting feature with a real crash history in
the shipped release notes. That is direct evidence of a category (real-time alerting) that AnxietyWatch v2
*specifies as a target* and that SleepHQ *has shipped and is still stabilising*. And "You're now told when
a night's recording fails to save" is the vendor adding an explicit failure notification — the same
*unknown is never rendered as good* concern this project treats as a rule, being retrofitted.

---

## 5. Pricing, hardware and the free tier

### 5.1 Subscription

| | Free ("Basic") | Pro |
|---|---|---|
| Price | US $0, "free to use forever" | **US $14.90/month**, or **US $149.00/year** (shown as "17% off", effective US $12.42/month) |
| Trial | — | **14-day free trial** |
| Devices tracked | **1** CPAP or O2 device | **Unlimited** |
| Account members | **1** | **Unlimited** |
| Data history | **"View your most recent 30 days of data"** | **"Unlimited data"** |
| Breath-by-breath + event flags | Included | Included |
| Share links | Included | Included |
| Connect with registered CPAP professionals | Included | Included |
| Sleep journal | — | "with notes, flags and advanced trends" |
| Custom PDF reports | — | "**with AI Insights**" |
| Apple Health / Google Health sync | **Not included** | Included |
| SleepHQ Community (Pro community) | **Not included** | Included |
| SleepHQ Academy (therapy training course) | — | Included |
| Exclusive discounts/offers | — | Included |
| **API access** | — | Included |
| Magic Uploader integration | Listed on both tiers — see §5.3 | Listed on both tiers — see §5.3 |

Prices are displayed in a selectable currency (AUD, CAD, EUR, GBP, USD); the page shows USD by default for
the fetch used here, and states "Prices shown in US dollars (USD)". A **GBP/AUD/EUR price is not the same
number** in the vendor's own selector, so regional pricing exists; the individual regional values were not
recorded. This is an **evidence gap**, not a finding.

### 5.2 Hardware

| Product | Price | Notes | Class |
|---|---|---|---|
| **Magic Uploader Pro** | **AUD $399.00** | Plugs into a ResMed Series 10/11 machine, connects to home Wi-Fi, uploads overnight | public claim |
| **O2 Ring Pro** | **AUD $299.00** | The only oximeter SleepHQ supports | public claim |
| Bundle | "save 40% off the cost of the O2 Ring" when both are bought together | public claim |

**Provenance note on those two figures.** The shop pages render prices through JavaScript, so the values
were read from each product's embedded JSON in the fetched page: **29900** for `o2-ring-pro` and **39900**
for `magic-uploader-pro`. Both are single-variant list prices in AUD; USD equivalents, bundle pricing and
any account-specific discounts were not captured.

### 5.3 An unreconciled conflict in SleepHQ's own pricing copy

The pricing block lists **"Magic Uploader Integration (requires separate purchase)"** under the **free**
tier, and lists the same item, without qualification, under Pro. The shop's own product copy states the
opposite of the free-tier reading: *"To use the Magic Uploader Pro, you need to have a SleepHQ Pro
subscription."*

Two vendor properties therefore disagree about whether the uploader functions on the free plan. Following
the package's standing rule, the **more conservative vendor statement governs**: the uploader requires Pro.
The conflict is recorded rather than resolved because both statements are public and current, and because
this is the second candidate (after Bearable's data-safety conflict) where two sleeping vendor surfaces
disagree — see `PROCESS_STATE.md` §process-level rules.

### 5.4 The free tier is a funnel, and its history limit is stated as a *view* limit

The free tier's constraints are **1 device, 1 member, 30 days of history**, no Health sync, no community,
no API. Every one of those is the top of the funnel for the Pro upgrade, and the hardware requires Pro.
The 30-day limit is stated as **"View your most recent 30 days of data"**. The vendor does **not** state
whether data older than 30 days is retained-but-hidden or deleted. **This is `unknown`, and the difference
matters**: Welltory's free tier *deletes* older data (candidate 2), and a free tier that hides rather than
deletes is a materially different privacy and portability proposition. No statement found in any SleepHQ
source resolves it.

---

## 6. Supported CPAP machines

From the vendor's supported-devices page, which is unusually complete and lists unsupported machines too.

| Brand | Families read | Notable |
|---|---|---|
| **ResMed** | AirSense 11 / AirCurve 11; AirSense 10 / AirCurve 10; S9 series and Lumis | Includes **AirSense 11 AutoSet, Elite, CPAP, Respond**, and AirCurve 11 VAuto/ASV/S/ST |
| **Philips Respironics** | DreamStation 2; DreamStation | Includes DreamStation Go and Go Auto |
| **Fisher & Paykel** | SleepStyle; ICON | |
| **Löwenstein** | prisma line (SMART, SOFT, 20A, 20C, 25S, 30ST, CR, LAB) | |
| **BMC** | G3 (incl. React Health Luna G3 APAP); G2S | |

**Explicitly not supported:** ResMed **AirMini** — and the vendor states the reason in general terms rather
than promising a future fix: *"It has no SD card slot, so there is no data for us to read. That is a
property of the machine rather than something a future SleepHQ release changes."* That is a notably honest
boundary statement.

**Not yet supported:** Resvent iBreeze 20A/20C/25A/30STA and iDisc 20A; Yuwell; DeVilbiss; Hoffrichter;
Weinmann SOMNOsoft2; Löwenstein prisma VENT30/40/50.

**The AirSense 11 overlap is the material one.** SleepHQ reads the same device AnxietyWatch2 targets. The
difference is the transport, not the device: SleepHQ reads the SD card, AnxietyWatch2 streams over BLE.

---

## 7. How data arrives

| Path | Detail | Class |
|---|---|---|
| Manual SD-card upload | Copy the card's contents and upload the files | public claim |
| **On-The-Go card reader** | Connect the SD card to a phone/tablet with an OTG reader and select files in the app | public claim (App Store copy) |
| **Magic Uploader Pro** | Plugs into a ResMed Series 10/11 machine; connects to home Wi-Fi; "uploads your ResMed data overnight"; "analysed and waiting on your dashboard by the time you take your mask off" | public claim |
| Magic Uploader — mechanism | **Reads the machine's SD card over Wi-Fi, using the Wi-Fi SD card it ships with** | public claim |
| Magic Uploader — travel | Wi-Fi SD card "stores your data until you return to your home network, then syncs automatically" | public claim |
| Magic Uploader — updates | Over-the-air updates | public claim |
| Magic Uploader — oximetry | Syncs the O2 Ring Pro **over built-in Bluetooth** | public claim |
| Magic Uploader — power | "requires its own power supply"; ships with power plugs for major countries | public claim |
| Upload cadence | App Store copy: "automatically syncs your complete CPAP data with SleepHQ **every 10 minutes**" | public claim |
| O2 Ring Pro | Over Bluetooth to the phone app or to the Magic Uploader | public claim |
| Apple Health / Google Health | Sleep, weight, steps, activity from any compatible wearable; Pro only; via the iOS/Android apps | public claim |

### 7.1 The documented upload contract, and a documented hash

The API reference documents the file-upload path precisely enough to be read as a contract:

| Item | Finding | Class |
|---|---|---|
| Upload a file | `POST /v1/imports/{import_id}/files` with `name`, `path` (**relative to the SD-card root**), optional `file`, and **required** `content_hash` | documented contract |
| Content hash algorithm | **"The content_hash is a MD5 hash of the file's content and name joined together."** Worked example given: file `file.txt` with content `Hello, World!` → MD5 of the string `Hello, World!file.txt`. **"Do not include the file's path when calculating the content hash."** | documented contract |
| Hash test endpoint | `POST /v1/imports/files/calculate_content_hash` — explicitly **"do not use this end point to calculate the content in your finished application"**; for validating one's own implementation | documented contract |
| File-level dedupe | Files are stored with `content_hash` **and** a `fingerprint_base64` | documented contract |
| Import deletion | `DELETE /v1/imports/{id}` with optional `purge_files=true` to delete associated files | documented contract |
| Rate limit | `POST /v1/teams/{team_id}/imports` only: **20 requests/hour and 100 per 24 hours per team**; `429` with `retry-after`; **fixed buckets**, 24-hour bucket reopens at UTC midnight; the cap counts *requests*, not imports — a large card is split server-side into child imports, one per calendar month (or per Philips P-Series folder) | documented contract |

Two observations, both labelled: **(a)** MD5 is documented as the dedupe/integrity hash. MD5 is broken for
collision resistance; for content-addressing among a single user's own SD-card files the collision risk is
not the interesting property, but the choice is recorded because it is a documented hash algorithm and
because this project treats provenance as load-bearing. **(b)** The rate-limit note is unusually precise
and *self-limiting* — the vendor documents its own window behaviour, including the fixed-bucket edge case
("requests that straddle a boundary can be allowed up to twice a limit in quick succession"), which is the
kind of honesty about limits that this package records positively.

---

## 8. Analysis and reporting features

### 8.1 Claimed capabilities

| Capability | Claim | Class |
|---|---|---|
| AHI tracking | "Track your AHI (apnea-hypopnea index), mask fit, usage compliance" | public claim |
| Breath-by-breath | "Pan through the night and discover how your breathing changes during different sleep stages. Zoom to analyse airflow limitation and respiratory arousals" | public claim |
| Sleep trends | "Track your CPAP Therapy changes and their impact on your AHI & sleep quality" | public claim |
| Sleep journal | Record sleep experiences, habits, lifestyle factors; "Track your feelings with fun emojis & **create custom flags** to track improvements and trends" | public claim |
| Advanced statistics | "detailed statistics on your therapy for every night. See how your pressure changes over time" | public claim |
| PDF reports | Pro: "Custom PDF reports **with AI Insights**" | public claim |
| Sleep stages | "See your CPAP data right alongside oxygen and sleep stage data" | public claim |
| Homepage capability areas | "Complete Dashboard", "Advanced Statistics", "Full Health Data", "The Community", "SleepHQ Academy" | public claim |

### 8.2 The documented analysis output — the per-night record

This is the most useful static evidence in the file, because it shows the granularity SleepHQ's analysis
layer actually publishes. Per **machine date** (one night), the documented attributes are:

| Attribute | Type | Reading |
|---|---|---|
| `date` | date | the night, as a calendar date |
| `time_offset` | integer | an explicit offset, editable via `PUT /v1/machine_dates/{id}` |
| `time_zone` | string | a time zone on the record |
| `usage` | string | — |
| `usage_summary`, `ahi_summary`, `pressure_summary`, `leak_rate_summary`, `flow_limit_summary`, `resp_rate_summary`, `epap_summary`, `machine_settings`, `pulse_rate_summary`, `spo2_summary`, `movement_summary` | **jsonb** | eleven summary blocks, published as opaque JSON |
| `large_leak` | integer | a flag, not a measurement |
| `created_at`, `updated_at` | datetime | **mutable** — the record is updated in place |

Per **session** (mask-on/mask-off), the documented attributes are richer still: `total_time`,
`summary_calculated` (boolean), `started_at`, `ended_at`, `length`, **`ahi`**, **`oa_count`, `ca_count`,
`h_count`, `ua_count`**, `pressure_set` and `pressure_set_95`, `ipap_set` and `ipap_set_95`, `epap_set` and
`epap_set_95`, `flow_limit` and `flow_limit_95`, `time_over_leak_threshhold`, `duration_90`, `drop_90`,
`drop_3`, `drop_4`, **`o2_score`**, `archived_at`, `omitted`, `moved_from_date`,
`moved_from_machine_date_id`, `device_session_key`, `machine_session_id`.

Four static observations from that list, each held to the label:

1. **SleepHQ models device-scored apnoea events and an AHI per session.** `oa_count`/`ca_count`/`h_count`/
   `ua_count` are the machine's obstructive/central/hypopnoea/unclassified counts, and `ahi` is an index.
   This is the exact category of derived value AnxietyWatch2 does not produce. *(`documented contract`)*
2. **There is a session-move mechanism.** `moved_from_machine_date_id` and `moved_from_date` exist
   alongside `PUT /v1/machine_dates/{id}` accepting `time_offset`. The night boundary and the device clock
   are correctable, and a moved session records where it came from. That is a provenance-preserving
   correction, and it is the same problem AnxietyWatch2 addresses with a versioned clock transform —
   reached by a different route. *(`documented contract`)*
3. **`omitted` is a string.** No enumeration or description is published, so what may be omitted, and
   whether "omitted" is a three-state marker for "this session was excluded", is **unknown**.
4. **`time_over_leak_threshhold` is misspelled in the vendor's own schema** ("threshhold"). Recorded
   because it means the field name a consumer must match is the misspelled one — a real integration
   hazard, and the same class of hazard as OSCAR's report header (candidate 4): a published interface
   name that does not read the way a consumer would guess.

The `jsonb` summary blocks are the limit of what the reference says. **What is inside `ahi_summary`,
`spo2_summary` or `machine_settings` is not specified anywhere in the public reference** — this is the
single largest evidence gap in this file, and it is why the per-night analysis depth is recorded as
`unknown` rather than as a finding.

### 8.3 "AI Insights" — named, not described

The Pro tier includes "Custom PDF reports with AI Insights". Beyond that four-word phrase, **nothing is
published**: no model named, no provider named, no statement of whether data leaves SleepHQ's
infrastructure, no statement of whether it is a language model or a statistical rule engine, and no
statement of whether it is on by default. Searches of the marketing site, articles, FAQ and API reference
found no further description.

This is recorded as `unknown` and it is a **material** unknown for two reasons: the privacy policy permits
"anonymised statistical analysis and research to develop new products and services", and AnxietyWatch2
specifies a local-first analysis posture. A feature that sends CPAP records to an unnamed AI is a different
proposition from one that does not, and SleepHQ's public materials do not let a reader tell which it is.

---

## 9. Sharing, community and clinicians

| Item | Finding | Class |
|---|---|---|
| Share links | "share your data with anyone you choose via sharelinks. **No personally identifiable information is shared** making sharelinks perfect for forums or social media groups" | public claim |
| Share-link lifetime | "Easily deactivate sharelinks when not in use" | public claim |
| Clinician sharing | If the clinician has a **SleepHQ clinical account**, they give the patient a custom link that connects directly; otherwise the patient creates a share link | public claim |
| Clinician account tiers | "SleepHQ Consult and Clinical API" — clinic and enterprise accounts have **additional API endpoints not in the public reference** | documented contract |
| Registered professionals | Free tier includes "Connect with registered CPAP professionals" | public claim |
| Community | "SleepHQ Community" — Pro members only; free tier's "Access to the SleepHQ **Pro** Community" is "not included" | public claim |
| Enterprise | "Screen, test, monitor, report, and manage patients across CPAP, oral appliances, oxygen therapy, and more"; "Book My Enterprise Demo" | public claim (Enterprise page) |
| Enterprise evidence | Four clinic case articles (Rise Precision Medicine, CPAP Direct, ISD Health Solutions, one independent clinic), all authored by the founder, dated 2024-09-07 | public claim |

**The PII claim is a public claim and is not verifiable from outside.** "No personally identifiable
information is shared" is the vendor's description of a share-link payload that was not retrieved: no
share link was created or fetched. Whether the payload is genuinely de-identified — and against which
identifier set — is `unknown`.

**Note on the enterprise evidence.** All four clinic case studies are vendor-authored and one is about a
clinic the vendor names as a partner. They are the vendor's statements, not independent validation.

---

## 10. Oximetry — the O2 Ring Pro

| Item | Finding | Class |
|---|---|---|
| Compatibility | **"Please note only SleepHQ branded devices are compatible."** The only supported oximeter is the SleepHQ O2 Ring Pro | public claim (FAQ) |
| Measures | SpO₂, pulse rate, body movement | public claim |
| Sample rate | **"1 second sample rate"** | public claim |
| Accuracy claim | "The O2 Ring Pro is a **medical grade device with high levels of accuracy**" | public claim |
| Battery | Up to **24 hours**; USB-C, empty to full in three hours | public claim |
| Alerts | **"Discreet Vibration Alerts — Receive immediate notifications when your blood oxygen or heart rate falls below preset thresholds"** | public claim |
| App alerts | App Store: "**set personalised alerts** to receive notifications for specific oxygen & pulse rate level events" | public claim |
| Display | Play release notes refer to "the ring screen", implying an on-device display | static-only finding (inference from a release note) |
| Health sync | Syncs to the SleepHQ app, and "also connecting to Apple Health (iOS) and Health Connect (Android)" | public claim |
| Warranty | 12-month replacement warranty | public claim |
| Price | AUD $299.00 | public claim |
| Related model | An "**O2 Ring S**" exists and is recognised by name in the Android app | static-only finding (release note) |

**The vendor-lock is the finding.** SleepHQ's own FAQ states that **only SleepHQ-branded devices** are
compatible. A user with an existing EMAY, Contec, Viatom or Wellue oximeter cannot bring it. AnxietyWatch2
targets an EMAY `SleepO2` and treats the oximeter as a replaceable sensor. This is the clearest
product-philosophy divergence in the whole candidate, and it is stated by the vendor rather than inferred.

**The desaturation alert is a shipped real-time-path feature.** "Immediate notifications when your blood
oxygen or heart rate falls below preset thresholds" is the vendor describing an alert — in the same
category as AnxietyWatch2's multi-modal alerting (amendment 03). The Android release notes' "Fixed three
crashes in the sleep alarm" is evidence that the alerting path is real, shipped, and was unstable.
Whether the SleepHQ alert is *supervised*, has a dead-man switch, or distinguishes "not measured" from
"measured and good", is **unknown** — none of those concepts appears in any SleepHQ source.

### 10.1 The "completely secure at all times" claim, and the policy that does not say that

Two vendor statements about the same property do not agree:

| Source | Statement |
|---|---|
| FAQ ("Is my data secure?") | "All SleepHQ data is encrypted **both at rest and in flight ensuring your data is completely secure at all times**." |
| Privacy policy §3 | "We are working with Oneleet to implement a formal security program, and are **progressing towards SOC 2 Type II certification**... controls are **designed to meet** HIPAA requirements... We engage independent security firms to perform regular penetration testing" |
| Play data-safety panel | "**Data is encrypted in transit**" — with no at-rest statement |
| Terms | Medical disclaimer; no security representation |

Following the rule that **the vendor's most conservative statement governs**, this file records: SleepHQ
states that data is encrypted in transit; it states that data is encrypted at rest in its FAQ but not in
its store data-safety declaration; and its security programme is described as *in progress*, not attested.
The phrase **"completely secure at all times"** is an absolute claim that no system can support, and it is
the kind of formulation this project's own rules forbid. It is recorded as a public claim, quoted exactly,
and **not** treated as a security finding in either direction.

---

## 11. Time handling

| Item | Finding | Class |
|---|---|---|
| Per-night time zone | `Api_V1_MachineDateSerializer.time_zone` (string) and `time_offset` (integer) | documented contract |
| Correctable offset | `PUT /v1/machine_dates/{id}` accepts **`time_offset`** only | documented contract |
| Session re-dating | `moved_from_machine_date_id`, `moved_from_date` on sessions | documented contract |
| Account time zone | `Api_V1_UserSerializer.time_zone` | documented contract |
| Team time zone | `Api_V1_TeamSerializer.time_zone` (plus `locale`, `energy_unit`) | documented contract |
| Day boundary | **Not documented.** Whether a "machine date" is a midnight or noon boundary, and whether it is derived from the device clock or the account time zone, is `unknown` | unknown |

SleepHQ clearly treats the night's time identity as *data that can be wrong and can be corrected*, at
per-night granularity, with a record of sessions having been moved. AnxietyWatch2 does the same thing by a
different mechanism (store the raw device time, keep a versioned transform, apply it at read time). SleepHQ
additionally carries a **per-team and per-user time zone**, which is what a clinic-facing product needs and
which AnxietyWatch2 — a single-owner system — does not model.

---

## 12. Privacy, data rights and export

| Item | Finding | Class |
|---|---|---|
| Data collected | Name, Email, **Birth date**, Gender, Time Zone, CPAP Data, Apple Health Data (optional, iOS), Android Health Connect Data (optional), O2 Ring Data | public claim (Privacy Policy) |
| Stated purpose | "To build, improve and provide technical support"; **"We also perform anonymised statistical analysis and research to develop new products and services"** | public claim |
| Sharing | "You control your data. You can upload it, store it, share it, download it and delete it all on your terms. **We don't share it without your permission.**" | public claim |
| Access | Right to request data | public claim |
| Portability | "At any time, you can download all data from your SleepHQ account" | public claim |
| Correction | "complete control to modify your data through SleepHQ" | public claim |
| Erasure | "You can completely delete your account & all associated data" | public claim |
| Raw-file export | `download_url` on import files — **a signed URL that expires in 5 minutes** | documented contract |
| Analytics/telemetry subprocessors | **None named.** The privacy policy names no analytics, crash-reporting or advertising vendor | public claim (by omission) |
| Jurisdiction / governing law | **Not stated** in the privacy policy or the terms page fetched. The entity is Australian | unknown |
| Policy date / last-updated | **Not stated** on the privacy policy page fetched | unknown |
| Retention period | **Not stated** | unknown |
| Deletion completeness | Claimed. Not observed | public claim |

**Two structural differences from every other candidate so far.** SleepHQ names **no analytics or
advertising subprocessors at all** — which is a marked contrast with Bearable and Welltory, both of which
name Meta/Google/TikTok or concede telemetry. And its privacy policy contains **no date and no governing
law**, which is a documentation gap rather than a privacy virtue, and is recorded as such.

The Play data-safety declaration ("Personal info and Health and fitness" collected; **no data shared with
third parties**) is consistent with the privacy policy's "we don't share it without your permission" — a
genuine agreement between two independent vendor surfaces, which is worth recording because Bearable's
equivalent pair did **not** agree.

**The portability claim is backed by a documented mechanism.** "Download all data" is not only marketing:
the API documents a per-file `download_url` returning the original uploaded files, with a 5-minute signed
URL. For a CPAP user that means the raw SD-card contents can be retrieved. This is a real portability
property, and it is the strongest of any candidate so far apart from OSCAR (which publishes its schema).

---

## 13. Clinical, regulatory and safety claims

| Statement | Source | Class |
|---|---|---|
| "The therapy data, charts, scores and educational content on SleepHQ are provided to help you understand your own sleep and therapy. **They are not a diagnosis and are not medical advice.**" | Terms | public claim |
| "Always discuss your results with your treating clinician, and **do not start, stop or change any therapy or device setting without their advice**." | Terms | public claim |
| "**SleepHQ is not a medical device** and does not diagnose, treat, cure or prevent any medical condition." | Play listing | public claim |
| "The O2 Ring Pro is a **medical grade device** with high levels of accuracy and a 1 second sample rate." | Shop | public claim |
| "progressing towards SOC 2 Type II certification"; HIPAA "designed to meet" | Privacy Policy | public claim |

**The two claims about medical status are not obviously consistent, and the conservative one governs.**
The *software platform* explicitly disclaims being a medical device. The *O2 Ring Pro* is described as a
"medical grade device". Those are different objects — a hardware sensor versus a software platform — so the
statements can coexist; but the ring's "medical grade" claim is unsupported by any cited clearance or
standard on the page fetched, and is therefore recorded as **vendor marketing** pending evidence. No FDA,
TGA, CE or other regulatory registration is claimed anywhere on the properties fetched, and none was
searched for in a registry. **Unknown, not absent.**

The disclaimer language is nonetheless the most careful of any commercial candidate so far, and it is
close in substance to the position this project takes: not a diagnosis, not medical advice, do not change
therapy settings on the basis of it.

---

## 14. What SleepHQ says about CPAP AHI — and why it matters here

This is the most consequential set of public statements in the candidate, and it comes from the founder,
published under his name.

### 14.1 "What the CPAP Machine Misses" (Nick Dunn, 2024-12-16)

A worked case: a machine flagged **51 events**, AHI **6.32**; a manual review by five sleep technologists
(each with, as stated, up to 14 years of experience) produced a lowest count of **115 events**, AHI
**14.3** — "almost double the AHI the CPAP machine provided". The stated cause is the machine's hypopnoea
threshold (">50% reduction in airflow for 10 seconds or more"). The article's recommendations are to review
the night's flow data manually, to use oximetry to validate, and it names **OSCAR** alongside SleepHQ as
tools for getting "a clearer picture".

### 14.2 "MyAir" comparison article (Nick Dunn, 2024-09-29)

Claims that machine-generated AHI **underestimates** severity; that on ResMed AirSense 10 "if the device
scored an AHI of zero, it could actually be closer to a 10, and if it showed a 16, it could have been more
like 25-30"; that Philips DreamStation was "more honest"; and concludes: **"Trust Your Oxygen Levels, Not
the MyAir Score."**

### 14.3 What is and is not supported by these statements

**What they are.** A vendor founder's own published criticism of device-scored AHI, with specific numbers,
from a person describing himself as a sleep technologist. They are `public claim`.

**What they are not.** They are **not independent validation**, and this file does not treat them as such.
The article says "in a study where simulated breathing data was analyzed by both CPAP machines and top
respiratory physicians" — **no study is named, no author, no journal, no year, and no link**. No external
citation link exists anywhere in either article. The specific figures (zero → 10, 16 → 25-30) therefore
have no traceable source, and are recorded as **unsourced vendor claims**.

**Why they still matter for this comparison.** AnxietyWatch2's decision not to compute an AHI, and to store
the device's raw flow and pressure signals instead, is a design position. It now has **a commercial
competitor's founder arguing the same underlying point in public**: the machine's own AHI is an algorithmic
estimate that under-counts hypopnoeas, and the raw signal plus oximetry is the more defensible evidence.
That is *corroboration in the weak sense* — agreement from a party with no motive to flatter this project
— and it is **not** evidence that V2's approach is correct, validated, or clinically adequate. V2 has no
published validation at all, and neither does SleepHQ.

**The internal tension is real and is recorded without resolving it.** SleepHQ's marketing tells users to
"track your AHI", and its API exposes `ahi`, `oa_count`, `ca_count`, `h_count` and `ua_count` per session.
Its founder simultaneously publishes that device AHI is unreliable and under-counts. Both can be true at
once — SleepHQ surfaces the machine's flags *and* teaches users not to trust them, which is arguably the
honest arrangement — but the product's headline metric is the same one its founder argues is wrong. This is
the same pattern seen in OSCAR (candidate 4), where the vendor's own internal review is more conservative
than the product's presentation.

---

## 15. Evidence table

| ID | Claim | Source | Class | Date | Confidence |
|---|---|---|---|---|---|
| S-1 | SleepHQ Pty Ltd, founded by Nick Dunn (sleep technologist) and Adam (web developer), Melbourne / Phillip Island | `about` page | public claim | 2026-10-06 | High |
| S-2 | Self-described as "the first ever cloud based, cpap data analytics platform" | `about` page | public claim | 2026-10-06 | Medium (unverifiable) |
| S-3 | "Over 100,000 patients use SleepHQ in over 160 countries" | Privacy Policy | public claim (vendor-published) | 2026-10-06 | Medium |
| S-4 | "Over 100k+ happy pappers" | home page | public claim (marketing) | 2026-10-06 | Low (copy) |
| S-5 | Free tier "free to use forever": 1 device, 1 member, most recent 30 days viewable | home page pricing | public claim | 2026-10-06 | High |
| S-6 | Pro: US $14.90/month or **US $149.00/year** (17% off; $12.42/mo); 14-day trial | home page pricing | public claim | 2026-10-06 | High |
| S-7 | Pro unlocks: unlimited data/members/devices, journal with notes/flags/trends, custom PDF reports with AI Insights, Academy, Health sync, Community, **API access** | home page pricing | public claim | 2026-10-06 | High |
| S-8 | Magic Uploader Pro listed on the free tier with "(requires separate purchase)" | home page pricing | public claim | 2026-10-06 | High |
| S-9 | Magic Uploader Pro **requires a Pro subscription** | shop product page | public claim | 2026-10-06 | High — **conflicts with S-8** |
| S-10 | Magic Uploader Pro list price AUD $399.00 | shop product JSON | public claim | 2026-10-06 | Medium (JS-rendered) |
| S-11 | O2 Ring Pro list price AUD $299.00 | shop product JSON | public claim | 2026-10-06 | Medium (JS-rendered) |
| S-12 | "Please note **only SleepHQ branded devices** are compatible" | FAQ | public claim | 2026-10-06 | High |
| S-13 | O2 Ring Pro: SpO₂, pulse, movement; **1 s sample rate**; 24 h battery; USB-C; 12-month warranty | shop | public claim | 2026-10-06 | High |
| S-14 | O2 Ring Pro: **vibration alerts when SpO₂ or heart rate falls below preset thresholds** | shop | public claim | 2026-10-06 | High |
| S-15 | App: "set personalised alerts" for oxygen/pulse events | App Store | public claim | 2026-10-06 | High |
| S-16 | **"Fixed three crashes in the sleep alarm"** | Play release notes | static-only finding | 2026-09-13 | High |
| S-17 | "You're now told when a night's recording fails to save" | Play release notes | static-only finding | 2026-09-13 | High |
| S-18 | AirSense 11 (and AirCurve 11) supported, eight listed variants | supported-devices page | public claim | 2026-10-06 | High |
| S-19 | ResMed AirMini unsupported; stated as a permanent property of the machine | supported-devices page | public claim | 2026-10-06 | High |
| S-20 | 19+ supported families across ResMed, Respironics, F&P, Löwenstein, BMC | supported-devices page | public claim | 2026-10-06 | High |
| S-21 | Magic Uploader reads the machine's **SD card over Wi-Fi** via a bundled Wi-Fi SD card; ResMed Series 10/11 only | shop + supported-devices | public claim | 2026-10-06 | High |
| S-22 | Uploader "syncs your complete CPAP data with SleepHQ every 10 minutes" | App Store copy | public claim | 2026-10-06 | Medium |
| S-23 | Uploader pairs with the O2 Ring Pro over **Bluetooth** | shop | public claim | 2026-10-06 | High |
| S-24 | Phone upload via an **On-The-Go SD card reader** | App Store / Play | public claim | 2026-10-06 | High |
| S-25 | "All SleepHQ data is encrypted both at rest and in flight ensuring your data is **completely secure at all times**" | FAQ | public claim | 2026-10-06 | Medium — **overreach, see S-26** |
| S-26 | "working with Oneleet... **progressing towards** SOC 2 Type II"; HIPAA controls **"designed to meet"**; penetration testing | Privacy Policy | public claim | 2026-10-06 | High |
| S-27 | Play data safety: collects Personal info + Health and fitness; **no data shared with third parties**; encrypted in transit; deletion requestable | Play listing | public claim | 2026-10-06 | High |
| S-28 | Collects Name, Email, **Birth date**, Gender, Time Zone, CPAP data, Health data, O2 Ring data | Privacy Policy | public claim | 2026-10-06 | High |
| S-29 | "We also perform **anonymised statistical analysis and research** to develop new products and services" | Privacy Policy | public claim | 2026-10-06 | High |
| S-30 | "We don't share it without your permission"; access, portability, correction, erasure rights | Privacy Policy | public claim | 2026-10-06 | High |
| S-31 | Privacy policy carries **no date and no governing law** | Privacy Policy page | static-only finding | 2026-10-06 | High |
| S-32 | **No analytics, crash-reporting or advertising subprocessor is named** | Privacy Policy | public claim (by omission) | 2026-10-06 | Medium |
| S-33 | Share links: "**No personally identifiable information is shared**"; deactivatable | App Store / FAQ | public claim | 2026-10-06 | Medium (unverified) |
| S-34 | Clinician sharing via a SleepHQ clinical account link, or a share link | FAQ | public claim | 2026-10-06 | High |
| S-35 | Enterprise: "Screen, test, monitor, report, and manage patients" across CPAP, oral appliances, oxygen | Enterprise page | public claim | 2026-10-06 | High |
| S-36 | Therapeutic/device disclaimer: "not a diagnosis and are not medical advice"; do not change settings | Terms | public claim | 2026-10-06 | High |
| S-37 | "SleepHQ **is not a medical device**" | Play listing | public claim | 2026-10-06 | High |
| S-38 | O2 Ring Pro described as "a **medical grade device** with high levels of accuracy" | shop | public claim | 2026-10-06 | Low (uncited) |
| S-39 | Billing by **Stripe**; **no refunds**; auto-renewing; promotional email survives cancellation unless opted out | Terms | public claim | 2026-10-06 | High |
| S-40 | Founder article: machine flagged 51 events / AHI 6.32; manual review 115 events / AHI 14.3 | article, 2024-12-16 | public claim (unsourced) | 2024-12-16 | Low |
| S-41 | Founder article: AirSense 10 "scored an AHI of zero, it could actually be closer to a 10"; 16 → 25-30 | article, 2024-09-29 | public claim (**no study named or linked**) | 2024-09-29 | Low |
| S-42 | Founder recommends **OSCAR** alongside SleepHQ for reviewing flow data | article, 2024-12-16 | public claim | 2024-12-16 | High |
| S-43 | Pro includes "**Custom PDF reports with AI Insights**" — no further description anywhere | home page pricing | public claim | 2026-10-06 | High (existence) / unknown (nature) |
| S-44 | Public REST API, OAuth2 password grant, scopes `read`/`write`/`delete`, token lifetime 7200 s | API reference | documented contract | 2026-10-06 | High |
| S-45 | 21 documented public endpoints across machines, machine dates, sessions, imports, files, journals, teams, masks, devices | `swagger.json` | documented contract | 2026-10-06 | High |
| S-46 | Per-session fields include `ahi`, `oa_count`, `ca_count`, `h_count`, `ua_count`, `o2_score`, `duration_90`, `drop_90`, `drop_3`, `drop_4`, `*_95` pressure/flow percentiles | `swagger.json` | documented contract | 2026-10-06 | High |
| S-47 | Per-night record has `time_zone`, `time_offset`, `large_leak`, and **eleven `jsonb` summary blocks** | `swagger.json` | documented contract | 2026-10-06 | High |
| S-48 | `PUT /v1/machine_dates/{id}` accepts **`time_offset`** only; sessions carry `moved_from_machine_date_id` / `moved_from_date` | `swagger.json` | documented contract | 2026-10-06 | High |
| S-49 | **`content_hash` is documented as MD5 of (file content + file name)**, path excluded | API reference | documented contract | 2026-10-06 | High |
| S-50 | Import-file export via a `download_url` **signed URL expiring in 5 minutes** | `swagger.json` | documented contract | 2026-10-06 | High |
| S-51 | Only `POST /v1/teams/{team_id}/imports` is capped: 20/hour, 100/24 h per team; fixed buckets; child imports per calendar month | API reference | documented contract | 2026-10-06 | High |
| S-52 | Clinic/enterprise accounts have **additional API endpoints not in the public reference** | API reference | documented contract | 2026-10-06 | High |
| S-53 | Journal resource is **daily**: `date`, `step_count`, `weight_grams`, `feeling_score`, `sleep_stages`, `notes`, `active_energy_joules` | `swagger.json` | documented contract | 2026-10-06 | High |
| S-54 | `team` resource has `time_zone`, `locale`, `energy_unit`, `teams_type` | `swagger.json` | documented contract | 2026-10-06 | High |
| S-55 | Import records carry `status`, `progress`, `failed_reason`, `programatic` (sic) | `swagger.json` | documented contract | 2026-10-06 | High |
| S-56 | Session field is misspelled **`time_over_leak_threshhold`** in the vendor's schema | `swagger.json` | static-only finding | 2026-10-06 | High |
| S-57 | iOS 3.0.34 released **2026-10-05**; initial release 2023-11-09; iOS 18.1+; 4.73★/204 ratings; 17+ | iTunes lookup | public claim | 2026-10-06 | High |
| S-58 | Android updated 2026-09-13; 10K+ installs; 4.4★/76 reviews | Play listing | public claim | 2026-10-06 | High |
| S-59 | No vendor-authored SleepHQ GitHub organisation found; community third-party projects exist (a Raspberry Pi + Wi-Fi SD uploader, an API client) | GitHub search | static-only finding | 2026-10-06 | Medium |
| S-60 | Free tier's 30-day limit is a **view** limit; retention vs deletion **not stated** | home page pricing | public claim + unknown | 2026-10-06 | High (the limit) / unknown (the behaviour) |

---

## 16. Relationship to AnxietyWatch

### 16.1 Versus AnxietyWatch v1

| Dimension | v1 | SleepHQ | Reading |
|---|---|---|---|
| CPAP data acquisition | OSCAR CSV export, manual | SD card (manual, OTG reader, or Wi-Fi uploader) | SleepHQ is more convenient and reads the raw card, not a derived CSV |
| Device coverage | Whatever OSCAR exports | ~19 families, listed explicitly | SleepHQ far wider, and it lists what it *cannot* read |
| Oximetry | CSV columns | SleepHQ O2 Ring Pro **only** | v1 is more permissive; SleepHQ locks the sensor |
| Analysis depth | AHI, leak, pressure from OSCAR's CSV | Device event counts, per-session percentiles, breath-by-breath, `o2_score` | SleepHQ deeper on the CPAP axis; both depend on the device's own scoring |
| Sharing with a clinician | PDF reports | Live share links + clinical account connections | **SleepHQ ahead** |
| Panic/mood capture | Panic-time capture on the phone | Daily journal entry with a `feeling_score` and custom flags | **v1 ahead on timing fidelity**; SleepHQ's journal is daily-granularity |
| Medication tracking | Present | **Not found** | v1 ahead |
| Weather / aircraft / songs | Weather, song, aircraft | **Not found** | v1 ahead |
| Cost | Free | US $149/yr + AUD $399 uploader + AUD $299 ring | **v1 ahead** |
| Hosting | Optional sync to a user's own server | Hosted cloud, account required | **v1 ahead on ownership** |
| Anxiety inference | v1's CNS/correlation work attempts CPAP↔anxiety | **Nothing found linking CPAP data to anxiety** | Axis SleepHQ does not occupy |

**Verdict against v1: a partial competitor to v1's CPAP/SpO₂ subsystem, and a stronger product within that
subsystem.** SleepHQ does more with CPAP data, gets it off the machine more conveniently, and shares it far
better. v1's advantages are the ones that are not about CPAP data: panic-time subjective capture,
medication, environmental context, cost and data ownership. For a user whose only need is "understand my
CPAP therapy", SleepHQ is the better tool.

### 16.2 Versus AnxietyWatch2

| Dimension | V2 status | SleepHQ | Reading |
|---|---|---|---|
| AirSense 11 transport | **BLE streaming, continuous, no card handling** (`resmed-air11-ble-stream`, 6 of 7 tasks done; physical gate open) | SD card over Wi-Fi; 10-minute sync | **Architectural divergence.** V2 needs no extra hardware; SleepHQ needs a AUD $399 dongle or manual card work |
| Oximeter | EMAY `SleepO2` (any BLE target) | SleepHQ O2 Ring Pro only | **V2 ahead on replaceability**, SleepHQ ahead on measured integration |
| AHI | **Not computed.** Raw flow/pressure stored; device events decoded and classified but read by no consumer | Per-session `ahi` + event counts published | Different answers to the same doubt — see §14 |
| Percentiles | **None computed anywhere in V2** | `*_95` per session, `duration_90`, `drop_90`, `drop_3`, `drop_4`, `o2_score` | SleepHQ derives more; V2 stores the signal and defers |
| Alerting | Multi-modal alerting + supervised alarm + hardware dead-man switch (**V2 specified target**) | **"sleep alarm"** shipped, with a crash history; ring vibration alerts | **SleepHQ has this in production; V2 has it specified.** Recorded as an advantage for the competitor |
| Sharing | PDF reports (**specified target**); no cloud | Live share links, clinical accounts, community | **SleepHQ clearly ahead** |
| Hosting | Private LAN / self-hosted | Hosted cloud with account | Different products; neither is objectively better without the user's constraint |
| Data model | Append-only `samples`; tombstone deletion; explicit NULL for gaps; versioned clock transform | Mutable records with `updated_at`; import/file deletion; `jsonb` summaries; editable `time_offset` | **V2 ahead on append-only integrity; SleepHQ ahead on correction ergonomics** |
| Missing data | Never `locf()`/`interpolate()`; explicit NULLs | **Not documented** — `jsonb` summaries are opaque | Unknown, so not scored |
| Provenance | Raw-frame journal written before decode; replayable; attempt ledger specified | `content_hash` + `fingerprint_base64` per file; import `status`/`failed_reason` | **Both take provenance seriously, differently.** V2 records the *frame*; SleepHQ records the *file* |
| Cost | Self-hosted | US $149/yr + optional hardware | V2 ahead for a self-hoster; SleepHQ ahead for a non-technical user |
| Clinical sharing | None specified beyond reports | Clinical accounts, enterprise tier | **SleepHQ ahead** |

**Verdict against V2: a partial competitor to the AirSense 11 ingestion path, occupying a different
architecture.** SleepHQ is what a user buys today to analyse an AirSense 11 in the cloud; AnxietyWatch2 is
a self-hosted system that streams from the same machine and does not sell anything. They overlap on *the
device and the night*, and barely at all on *the product*: SleepHQ has alerting, sharing and clinical
workflows in production, none of which V2 has shipped, and V2 has an ownership and integrity posture that
SleepHQ does not offer and does not claim to.

---

## 17. Weaknesses, omissions and unresolved questions

**Findings, not speculations:**

1. **Proprietary oximeter lock-in**, stated by the vendor: "only SleepHQ branded devices are compatible."
2. **The AirSense 11 path requires either manual card handling or a AUD $399 device plus a Pro
   subscription.** The Bluetooth-only integration is between the *uploader* and the *ring* — not between
   the platform and the CPAP machine.
3. **The free tier's history limit is described as a view limit with no retention statement** — whether
   older data is deleted is unknown (S-60).
4. **"AI Insights" is named and never described** (S-43). Which AI, where data is processed, and whether it
   is opt-in are all unknown.
5. **An absolute security claim ("completely secure at all times") conflicts with a security programme the
   vendor describes as in progress** (S-25 vs S-26).
6. **Two vendor surfaces disagree about whether the Magic Uploader works on the free tier** (S-8 vs S-9).
7. **The AHI-accuracy research underpinning the founder's articles is unnamed and unlinked** (S-40, S-41).
8. **The platform's headline metric is the one its founder publicly argues is unreliable** (§14.3).
9. **The documented content hash is MD5** (S-49).
10. **The OAuth contract is the deprecated password grant**, with the vendor's own documentation noting
    that the refresh token it returns cannot be used (S-44). *(`documented contract`; not a runtime finding)*
11. **The privacy policy is undated, names no jurisdiction, states no retention period, and names no
    subprocessors** (S-31, S-32).
12. **The schema misspells a field a consumer must match**: `time_over_leak_threshhold` (S-56).
13. **Nothing found links CPAP data to anxiety, mood or panic.** Absence-of-evidence; recorded as unknown.
14. **The mobile apps are closed-source and were not inspected.** The Android APK was not obtained and no
    Play listing permits lawful retrieval; the iOS binary was not obtained. All app-level statements here
    come from store copy and release notes.
15. **No vendor GitHub organisation or published source exists**, so the source-inspection mode used for
    OSCAR is unavailable for this candidate (S-59).
16. **No clinical validation, peer-reviewed study or regulatory clearance is published** by SleepHQ. The
    enterprise case articles are vendor-authored (S-35 area).

**Open questions to carry into `PROMPT_2.md`:** what is inside the `jsonb` summary blocks; what "AI
Insights" actually is; the night-boundary convention; whether the free-tier limit deletes or hides; whether
the sleep alarm is supervised and whether it distinguishes "not measured" from "measured and good"; and
what the share-link payload actually contains.

---

## 18. Unverified scope

- No SleepHQ endpoint was called. **Zero runtime observations.**
- No account was created; no trial was started; nothing was purchased.
- No app binary was decompiled or executed. The Android APK was not obtained and no iOS binary was
  obtained.
- No share link was created or fetched, so the PII claim in S-33 is unverified.
- No regional price (AUD/CAD/EUR/GBP) was recorded beyond the USD figures.
- The prices in S-10/S-11 were read from JavaScript-embedded product JSON; a currency-converted or
  account-specific price may differ.
- Search-engine discovery was unreliable for this candidate (queries returned the same unrelated result
  set); the vendor's own properties were used instead, which is why the artifact set is site-complete but
  third-party-report-poor.
- No regulatory registry (FDA, TGA, CE) was searched for the O2 Ring Pro's "medical grade" claim.
- The clinic/enterprise API surface is documented as existing but **not published**; it was not inspected.

---

## Related files

- `docs/app_comparison/sleephq/sleephq_IMPLEMENTATION_INSPECTION.md` — what was inspected, and what was not
- `docs/app_comparison/sleephq/PROMPT_2.md` — the candidate-specific second-stage prompt
- `docs/app_comparison/sleephq/sleephq_COMPARISON.md` — the comparison report
- `docs/app_comparison/CANDIDATE_SHORTLIST.md` — selection and scoring
- `docs/app_comparison/RESEARCH_METHOD.md` — method, evidence classes, inspection modes
- `docs/app_comparison/PROCESS_STATE.md` — pipeline state
