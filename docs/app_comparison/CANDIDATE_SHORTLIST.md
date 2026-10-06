# Candidate shortlist — AnxietyWatch v1 / AnxietyWatch2 competitive research

**Research date:** 2026-10-06
**Brief:** `docs/app_comparison/PROMPT1.md`
**Mode:** full sequential run — the shortlist was built in full and **all ten candidates have now been
processed end to end**, one at a time. The rank-10 entry (MindShift) completed on 2026-10-06. Per-candidate
status and the queue are in `PROCESS_STATE.md`; the evidence rules are in `RESEARCH_METHOD.md`.
**Corrections:** two shortlist hypotheses were **tested and corrected** during the run — entry 9
(Home Assistant, three-state model refuted) and entry 10 (MindShift, operator and commercial model). Both
are marked inline in the entries below rather than silently rewritten.

Status labels used throughout this package:

| Label | Meaning |
|---|---|
| **V1 documented/implemented** | Represented as implemented by AnxietyWatch v1 code or documentation |
| **V2 current** | Present and usable in the current AnxietyWatch2 repository |
| **V2 specified target** | Fully specified in `openspec/`, not necessarily implemented |
| **Unresolved** | Mentioned but not sufficiently specified or evidenced |

---

## 1. Capability inventory used for selection

The shortlist was chosen to cover the capabilities below, not to cover the most visible anxiety-journal
features. Sources: `../AnxietyWatch/README.md`, `../AnxietyWatch/docs/FEATURES.md`,
`../AnxietyWatch/AnxietyWatch/Services/`, and this repository's `README.md` plus `openspec/changes/`.

| # | Capability | Status | Notes |
|---|---|---|---|
| C1 | Subjective anxiety severity logging (1–10), notes, tags | **V1 documented/implemented** | plus watchOS quick-log |
| C2 | Random/unsolicited mood check-ins | **V1 documented/implemented** | |
| C3 | Medication dose logging | **V1 documented/implemented** | |
| C4 | Dose-triggered before/after efficacy measurement (N-of-1) | **V1 documented/implemented** | paired 0-min / 30-min rating |
| C5 | HealthKit ingestion of 20+ types with source attribution | **V1 documented/implemented** | multi-source provenance |
| C6 | Personal rolling baselines (compared to self, not population) | **V1 documented/implemented** | 30-day HRV baseline |
| C7 | Physiological correlation engine | **V1 documented/implemented** | first intelligence layer |
| C8 | Polar H10 raw RR / beat-to-beat HRV, background recording | **V1 documented/implemented** | |
| C9 | CPAP data via SD-card import (AirSense 11, OSCAR CSV, EDF leak) | **V1 documented/implemented** | retrospective |
| C10 | EMAY overnight pulse-oximeter import + live BLE streaming | **V1 documented/implemented** | |
| C11 | Clinical PDF report generation | **V1 documented/implemented** | |
| C12 | Self-hosted sync server, no third-party cloud | **V1 documented/implemented** | |
| C13 | Optional LLM analysis over selected date range | **V1 documented/implemented** | opt-in, server-side |
| C14 | Barometric pressure / environmental sound context | **V1 documented/implemented** | |
| C15 | Song/lyric (earworm) capture with server-side lyric lookup | **V1 documented/implemented** | |
| C16 | Oura OAuth, FHIR lab results, pharmacy/prescription scanning | **V1 documented/implemented** | breadth of integrations |
| C17 | Always-on BLE collection on a Raspberry Pi | **V2 current** | collectors + `vitalsd` |
| C18 | Provenance-preserving append-only store (`(source,type,session_id,seq)`) | **V2 current** | |
| C19 | Pure `no_std` decoders + fuzz targets, reject-never-clamp | **V2 current** | |
| C20 | Real-time AirSense 11 live stream | **V2 current** | bounded read-only path |
| C21 | Three-state honesty model (good / bad / not measured) | **V2 current** | architectural invariant |
| C22 | Quality gate → personal baseline → fusion → tier machine (`cns-engine`) | **V2 specified target** | designed, not built |
| C23 | Supervised alarm channels + hardware dead-man switch (`klaxond`) | **V2 specified target** | designed, not built |
| C24 | Independent alarm path that survives phone loss / internet loss | **V2 specified target** | |
| C25 | Aircraft/environmental-noise context as a sleep interrupter | **Unresolved** | idea-level, see project description §13 |
| C26 | XE125 radar respiratory rate | **V2 current** | decoder + OpenSpec change |

### Selection scoring key

Each candidate was scored 0–3 on six dimensions. Totals are a sorting aid, not objective truth; the
written rationale is the actual justification.

| Key | Dimension |
|---|---|
| A | Whole-product or major-layer relevance |
| B | Anxiety / mental-health / sleep / medication / physiological overlap |
| C | Objective-data and subjective-context integration |
| D | Real-time, overnight, or always-on capability |
| E | Privacy, local-first, self-hosting, exportability |
| F | Evidence quality, availability, maturity |

---

## 2. The ten candidates, ranked

### 1. Bearable — `bearable`

| Field | Value |
|---|---|
| Company / project | Bearable Ltd (UK), `BEARABLE LIMITED`, company no. 12089632 |
| URL | https://bearable.app/ · https://apps.apple.com/us/app/bearable-mood-symptoms/id1482581097 · https://play.google.com/store/apps/details?id=com.bearable |
| Platforms | iOS, iPadOS, macOS (via App Store), Android |
| Product type | Consumer mobile app (commercial, freemium + subscription) |
| Overlaps | **AnxietyWatch v1**, whole product |
| Competing layer | v1 app |
| Score | A 3 · B 3 · C 2 · D 1 · E 1 · F 3 = **13** |

**Reason for inclusion.** Closest single whole-product analogue to AnxietyWatch v1's daily loop:
mood/symptom/medication/factor logging → correlation reports → better clinician conversation.
This is the app a prospective AnxietyWatch v1 user is most likely already using, which makes it the
most decision-relevant comparison in the set.

**Main comparison dimensions.** Subjective logging burden and customisation; correlation quality and
what it can be computed over; Free-vs-Premium gating of the analysis layer; CSV-only export versus
PDF clinical reporting; cloud account and server-side encryption versus local-first storage;
HealthKit as one-way import versus provenance-preserving sensor capture; absence of any real-time or
overnight monitoring.

**Uncertainty / evidence gaps.** Exact subscription prices are not published on the vendor website
(Play Store lists an in-app-purchase range only). Whether arbitrary imported HealthKit metrics can
enter the correlation engine is not documented. The cited peer-reviewed paper is about
patient-reported-outcome collection, not about correlation validity. The "2M patients" figure is
marketing copy on the App Store listing.

---

### 2. Welltory — `welltory`

| Field | Value |
|---|---|
| Company / project | Welltory Inc. |
| URL | https://welltory.com/ |
| Platforms | iOS, Android; web dashboard |
| Product type | Consumer app + cloud analytics (freemium subscription) |
| Overlaps | **AnxietyWatch v1** physiology and AI layers |
| Competing layer | v1 physiology + analysis; partial v2 analysis layer |
| Score | A 2 · B 3 · C 3 · D 2 · E 1 · F 3 = **14** |

**Reason for inclusion.** The strongest comparator for the physiological-biomarker and
automated-analysis half of v1: HRV-derived stress/energy, an "AI" interpretive layer, and a very
broad wearable/integration list. Useful for asking whether AnxietyWatch's personal-baseline approach
is genuinely different from a population/algorithmic norm.

**Main comparison dimensions.** Live spot HRV measurement versus continuous overnight capture;
proprietary algorithmic inference versus published personal baselines; how much of the "insight" is
explainable; subscription lock-in; cloud processing of cardiovascular data; whether subjective
input is treated as ground truth or as a soft signal.

**Uncertainty / evidence gaps.** Marketing claims of "100+ biomarkers" and peer-reviewed validation
need per-claim verification against the actual publications. The precise measurement inputs
(camera PPG, chest strap, watch) and their error characteristics are not established from the
homepage alone.

---

### 3. Exist.io — `exist`

| Field | Value |
|---|---|
| Company / project | Exist (Hello Code) |
| URL | https://exist.io/ |
| Platforms | iOS, Android, web |
| Product type | Quantified-self aggregation + correlation service (subscription) |
| Overlaps | **AnxietyWatch v1** correlation engine |
| Competing layer | v1 analysis layer |
| Score | A 2 · B 2 · C 3 · D 1 · E 1 · F 2 = **11** |

**Reason for inclusion.** The most directly comparable *correlation engine* to v1's `BaselineCalculator`
+ correlation work, including weather and calendar context that overlaps AnxietyWatch's
environmental-context ambitions (C14, C25). It also tests the value of merging objective service
data with manual attributes.

**Main comparison dimensions.** Breadth of automatic sources versus depth on physiology;
statistical presentation and false-positive risk; whether personal baselines or population
comparisons are used; cloud-only architecture; exportability and account requirement.

**Uncertainty / evidence gaps.** Current pricing, data-export fidelity, and the statistical method
behind its correlation findings are not established from public pages at this research date.

---

### 4. OSCAR — `oscar`

| Field | Value |
|---|---|
| Company / project | OSCAR / Apnea Board (open source, community) |
| URL | https://www.sleepfiles.com/OSCAR/ |
| Platforms | Windows, macOS, Linux, Chromebook, Raspberry Pi OS (desktop) |
| Product type | Open-source desktop CPAP data analysis application |
| Overlaps | **AnxietyWatch v1** CPAP import (C9) and **V2 specified target** CPAP subsystem |
| Competing layer | subsystem (CPAP/respiratory) |
| Score | A 1 · B 2 · C 1 · D 1 · E 3 · F 3 = **11** |

**Reason for inclusion.** Free and permanently free, community-maintained, and it is the practical
destination of the SD-card data AnxietyWatch v1 imports. It is the reference implementation for
what a sleep clinician actually looks at, and a fair benchmark for whether AnxietyWatch's CPAP
handling is credible or shallow. Also the closest peer to AnxietyWatch2's open-source posture.

**Main comparison dimensions.** Depth of respiratory-event analysis; raw waveform/EDF handling;
open-source and offline operation; desktop versus always-on Pi deployment; no live streaming, no
alarm, no anxiety context.

**Uncertainty / evidence gaps.** Exact current device coverage and analysis feature list need
verification against release notes. A manual-selection note on the download page (referring to a
beta test group) makes the public availability status of the current version ambiguous and was not
resolved.

---

### 5. SleepHQ — `sleephq`

| Field | Value |
|---|---|
| Company / project | SleepHQ |
| URL | https://sleephq.com/ |
| Platforms | Web + mobile; pairs with a SleepHQ O2 Ring |
| Product type | Commercial CPAP data platform (free tier + paid) |
| Overlaps | **AnxietyWatch v1** CPAP/SpO₂ import; **V2 specified target** reporting and sharing |
| Competing layer | subsystem (CPAP + oximetry + sharing) |
| Score | A 2 · B 3 · C 2 · D 1 · E 1 · F 3 = **12** |

**Reason for inclusion.** Covers the combination AnxietyWatch actually cares about —
AirSense 11 data plus overnight SpO₂ plus longitudinal statistics — and adds the clinician/
peer-sharing workflow that AnxietyWatch approaches via PDF reports and a self-hosted server.
It is a benchmark for CPAP data depth and for how much of that data a consumer platform can expose.

**Main comparison dimensions.** CPAP data depth and refresh latency (nightly import versus live
stream); combined CPAP + oximetry analysis; community sharing versus private self-hosting; cloud
account and subscription cost; whether it can inform anything about anxiety.

**Uncertainty / evidence gaps.** Pricing tiers, data-export rights, and Cloud API availability were
not established. "Over 100k+ happy pappers" is marketing copy.

---

### 6. Nightscout — `nightscout`

| Field | Value |
|---|---|
| Company / project | Nightscout (open-source #WeAreNotWaiting community, `nightscout/cgm-remote-monitor`) |
| URL | https://nightscout.github.io/ |
| Platforms | Self-hosted web app; many host options (Docker, Raspberry Pi, Azure, Fly.io, …) |
| Product type | Self-hosted personal health-data platform |
| Overlaps | **AnxietyWatch2** platform architecture |
| Competing layer | v2 platform |
| Score | A 3 · B 1 · C 2 · D 3 · E 3 · F 3 = **15** |

**Reason for inclusion.** The closest existing model for what AnxietyWatch2 is attempting at the
platform layer: self-hosted, continuously-ingested physiological data, user-owned, with
community-defined uploaders/downloaders and an alerting path. AnxietyWatch2 should be measured
against how Nightscout solves deployment, access control, and always-on operation on a Pi.

**Main comparison dimensions.** Self-hosting ergonomics and documented deployment paths; security
and privacy documentation for a health endpoint; data retention and history; alerting; the gap
between CGM (a well-supported consumer device) and a heterogeneous BLE sensor fleet; and the
project's maturity and community size versus AnxietyWatch2's.

**Uncertainty / evidence gaps.** The physiology is single-modality (glucose), so overlap with
AnxietyWatch2 is architectural rather than clinical. The security posture of typical DIY
deployments is frequently criticised publicly and was not independently assessed here.

---

### 7. Empatica (EmbracePlus / EpiMonitor) — `empatica`

| Field | Value |
|---|---|
| Company / project | Empatica Inc. |
| URL | https://www.empatica.com/embraceplus/ |
| Platforms | Wearable + mobile app + clinician portal; US, UK, EU, AU, NZ, CA |
| Product type | Regulated wearable monitoring ecosystem with alerting |
| Overlaps | **AnxietyWatch2** CNS-monitoring and alarm layers (C22, C23, C24) |
| Competing layer | v2 subsystem (CNS detection + alarm) |
| Score | A 2 · B 2 · C 3 · D 3 · E 1 · F 3 = **14** |

**Reason for inclusion.** The most relevant existing product for the part of AnxietyWatch2 that does
not exist yet: autonomic/CNS monitoring from a body-worn sensor with an *independent alert path* and
caregiver notification. It is the clearest comparator for what a clinically validated version of the
`klaxond`/`cns-engine` ambition looks like — and what regulatory and evidence burden it requires.

**Main comparison dimensions.** Alarm channel design, latency, and failure behaviour; how false
positives and "not measured" states are handled; regulatory classification and clinical evidence;
closed wearable/app ecosystem versus open BLE; cost and geographic availability; privacy of
continuous physiological data.

**Uncertainty / evidence gaps.** Consumer availability is in flux — the Embrace2 page states it is
"No longer available for purchase" — and much of the current offering is oriented to clinical trials
and research rather than consumers. Regulatory status per market is not established here. This
candidate is therefore a *subsystem and evidence* comparator, not a direct product competitor.

---

### 8. Oura — `oura`

| Field | Value |
|---|---|
| Company / project | Ōura Health Oy |
| URL | https://ouraring.com/ |
| Platforms | Ring hardware + iOS/Android app; subscription membership; API |
| Product type | Wearable ecosystem |
| Overlaps | **AnxietyWatch v1** wearable physiology; **V2 specified target** planned Oura import |
| Competing layer | v2 subsystem (wearable data source) + v1 physiology |
| Score | A 2 · B 2 · C 2 · D 3 · E 1 · F 3 = **13** |

**Reason for inclusion.** AnxietyWatch2 explicitly plans Oura as a data import, and v1 has an Oura
OAuth manager. Oura is also the strongest example of an overnight-physiology wearable with its own
closed interpretation layer, which is directly comparable to AnxietyWatch's "personal baseline"
framing and to its refusal to render a value that was not measured.

**Main comparison dimensions.** Overnight sensor set (sleep staging, HRV, temperature, SpO₂,
respiratory rate) versus the AnxietyWatch2 sensor fleet; closed app interpretation versus
receipts-and-provenance; subscription gating of insights and raw-data access; API and export
availability; whether the platform can serve as a *source* rather than a *competitor*.

**Uncertainty / evidence gaps.** Claims such as "86% of Oura Members see their health improve" and
the symptom-radar narrative are marketing copy with a footnote. Raw-data access via API may require
membership; the exact tier boundaries were not verified.

---

### 9. Home Assistant — `homeassistant`

| Field | Value |
|---|---|
| Company / project | Open Home Foundation (non-profit) |
| URL | https://www.home-assistant.io/ |
| Platforms | Self-hosted hub (incl. Raspberry Pi) + iOS/Android apps |
| Product type | Open-source local-first home/edge automation platform |
| Overlaps | **AnxietyWatch2** platform layer (always-on edge, local automation, local alerting) |
| Competing layer | v2 platform |
| Score | A 3 · B 0 · C 1 · D 3 · E 3 · F 3 = **13** |

**Reason for inclusion.** The most mature open-source local-first edge platform, with a Raspberry Pi
target, local automations that run without internet, a large integration fleet (1,500+), and a
documented local-only privacy stance. AnxietyWatch2's admin console, always-on collector service,
deployment story and local alarm path all have direct equivalents here, so it is the best available
benchmark for operational quality rather than clinical function.

**Main comparison dimensions.** Local-only operation guarantees; automation/alarm reliability and
failure signalling; integration and onboarding ergonomics; deployment, backup and upgrade story;
how a health-specific platform must differ from a general-purpose hub; and how "unknown" states are
surfaced.

> **Correction applied 2026-10-06, after inspection (refuted hypothesis).** This entry originally claimed
> that a general-purpose hub has no three-state honesty model. **That hypothesis was tested and
> refuted.** Home Assistant's state machine has `STATE_UNKNOWN` and
> `STATE_UNAVAILABLE` as **first-class states** (`const.py:303,313`, tag `2026.9.4`,
> commit `9212531f…`). What the inspection found instead is sharper and is the reason this candidate
> still scores where it does: the three-state model is lost **downstream** — the `alert` integration has
> **no branch for either absent state** and ends the alert, sending the resolution message, when the
> watched entity goes offline (`alert/entity.py:98,115-126,148`). The finding is a **layer** finding, not
> a platform finding. See `homeassistant/homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` §14.4,
> `homeassistant/IMPLEMENTATION_INSPECTION.md`, and `PROCESS_STATE.md` gap **R12**.

**Uncertainty / evidence gaps.** No physiological or clinical function; overlap is entirely
architectural. Specific alarm/notification reliability guarantees are not established here.

---

### 10. MindShift CBT — `mindshift`

| Field | Value |
|---|---|
| Company / project | **247 Labs Inc** (for-profit Toronto consultancy) — *corrected after inspection; previously recorded as Anxiety Canada* |
| URL | https://mindshiftcbt.com/ |
| Platforms | iOS, Android |
| Product type | Free-to-consumer CBT app with a paid B2B/organisational surface — *corrected after inspection; previously recorded as "free, non-commercial"* |
| Overlaps | **AnxietyWatch v1** anxiety-specific layer, and free/no-subscription positioning at the consumer tier |
| Competing layer | v1 app (adjacent, not overlapping) |
| Score | A 1 · B 3 · C 1 · D 1 · E 3 · F 3 = **12** |

**Reason for inclusion.** Deliberately included as the *category* comparator. AnxietyWatch's name
places it in the anxiety-app market, where the dominant purpose is intervention (CBT tools, thought
records, exposure) rather than measurement and correlation. The consumer app is free with no
subscription, making it the fairest test of the claim that AnxietyWatch is not simply another
mood tracker — and a useful discoverability benchmark for the anxiety search category.

> **Correction applied 2026-10-06, after inspection (materially altered premise).** This entry originally
> attributed the app to **Anxiety Canada (non-profit)** and described it as **free and non-commercial**.
> Both are superseded by the storefront records and the vendor's own site: the App Store `sellerName`
> and the Google Play developer are **247 Labs Inc**, and the site says the app was "originally developed
> in partnership with Anxiety Canada" — past tense. A B2B surface (`/for-clinics`, `/institutions`) with
> **unpublished** pricing runs beside the "Always Free" consumer app, with Clerk organizations and Stripe
> billing present in the vendor's published client. **The consumer tier is still free and the evidence
> base is still real; the operator and the commercial model are what changed.** See
> `mindshift/mindshift_CLAIMED_FEATURES_AND_PURPOSE.md` §14.4 and `PROCESS_STATE.md` gap **G-8**.

**Main comparison dimensions.** Purpose (change anxiety now vs. understand anxiety over time);
whether any physiology is involved; monetisation and data posture; evidence base and clinical
review; what each product does during a panic attack versus after one.

**Uncertainty / evidence gaps.** Feature list and current maintenance status were taken from the
product site; the underlying evidence base is asserted by the publisher and was not independently
verified at this research date.

---

## 3. Ranking summary

| Rank | Slug | Name | Type | Primary layer | Score |
|---|---|---|---|---|---|
| 1 | `bearable` | Bearable | Consumer app | v1 whole product | 13 |
| 2 | `welltory` | Welltory | App + cloud analytics | v1 physiology + AI | 14 |
| 3 | `exist` | Exist.io | Aggregation service | v1 correlation engine | 11 |
| 4 | `oscar` | OSCAR | Open-source desktop app | CPAP subsystem | 11 |
| 5 | `sleephq` | SleepHQ | Commercial platform | CPAP + oximetry + sharing | 12 |
| 6 | `nightscout` | Nightscout | Self-hosted platform | v2 platform | 15 |
| 7 | `empatica` | Empatica | Wearable + clinical ecosystem | v2 CNS + alarm | 14 |
| 8 | `oura` | Oura | Wearable ecosystem | v2 data source | 13 |
| 9 | `homeassistant` | Home Assistant | Open-source edge platform | v2 platform | 13 |
| 10 | `mindshift` | MindShift CBT | Free-to-consumer app (247 Labs Inc) | v1 adjacent category | 12 |

The numeric score is deliberately *not* the ranking. Bearable is ranked first despite a lower total
than Nightscout because dimension A (whole-product relevance to v1) is decisive for a pilot:
it is the only candidate whose daily user workflow is close enough that a feature-level comparison
is meaningful line by line. Nightscout and Home Assistant score highly on architecture while having
almost no clinical overlap, which is exactly why they are ranked lower and classified as platform
comparators rather than product competitors.

## 4. Alternatives considered but not selected

| Product | Why considered | Why not selected |
|---|---|---|
| **Daylio** (daylio.net) | Very widely used mood/micro-journal app; strong discoverability benchmark in the mood-tracker category | Overlaps Bearable's ground with less breadth; no physiology, no medication efficacy, no clinician reporting. Would add a second mood-tracker row rather than a new dimension. |
| **Withings Sleep Analyzer / Somnofy** | Under-mattress and radar-based home sleep-apnea and respiratory monitoring — the closest commercial analogue to AnxietyWatch2's XE125 radar respiratory-rate work | Fails as a whole-product comparison; if added it should be added specifically as a radar/respiratory subsystem comparator, ideally after the XE125 decoder is deployed and can be compared on measured output. |
| **Medisafe** | The best-known medication-reminder app; medication is a core AnxietyWatch capability | Reminders and adherence only, with no before/after efficacy measurement and no physiological data — it would not test AnxietyWatch's actual differentiator (dose-triggered paired N-of-1 ratings). |
| **Apple Health Records / FHIR-based PGHD tools** | Directly relevant to AnxietyWatch v1's `FHIRLabResultParser` and clinical-record import | Apple's own tooling is not a comparable product, and the FHIR ecosystem is a standard rather than a competitor. Better covered later as a standards/interop topic than as a shortlist row. |
| **Visible (makevisible.com)** | HRV-based pacing for ME/CFS and Long Covid — symptom + physiology + personal-baseline overlap | Strong conceptual overlap but a narrower condition-specific audience than Bearable, which already covers the same comparison dimensions with more breadth. |
| **Loop / OpenAPS / AndroidAPS** | Self-built closed-loop systems with local control and safety interlocks — architecturally instructive for the alarm path | The alerting comparison is better served by Empatica (validated alerting with clinical evidence) and the self-hosting comparison by Nightscout. Three DIY-closed-loop entries would crowd the list. |
| **eMoods** | Would have been the strongest free, offline, no-account comparator with a printable clinician report for v1's core loop | **Not selectable at this research date.** `emoods.net` returned no DNS A record from the research workstation, and the site could not be retrieved, so no claims could be evidenced. Re-test before any future inclusion. |

## 5. Known limitations of this shortlist

- Ten candidates cannot cover every AnxietyWatch2 subsystem. Aircraft/noise context (C25) and
  blood-pressure collection have no dedicated candidate.
- Oura, Bearable and Welltory all have subscription tiers whose exact current prices are not
  consistently published; shortlist cost comparisons will be approximate unless checked in-store.
- Empatica's consumer availability is in flux, so its row is an evidence/alerting comparator rather
  than a live alternative a user could adopt today.
- No candidate combines real-time CPAP streaming, always-on BLE collection, provenance-preserving
  storage and an independent alarm path; that combination is the part of AnxietyWatch that appears
  to be genuinely unoccupied, but this shortlist only samples ten products and does not prove
  absence.
