# Candidate shortlist v2 — AnxietyWatch / AnxietyWatch2 competitive research

**Revision date:** 2026-10-07 (v2 — rebuilt after the previous run was reset).
**Brief:** `docs/app_comparison/PROMPT1.md` · **Method:** `RESEARCH_METHOD.md` · **State:** `PROCESS_STATE.md`

**What changed from v1, and why.**

1. **OSCAR is removed.** It is a desktop CPAP-data analysis tool that reads the same SD-card export this
   project imports — a **data source and dependency, not a competitor**. The v1 list ranked it fourth and
   spent a whole candidate budget establishing that. Its credentials are recorded in §4 so the exclusion
   is a decision with reasons rather than a silent deletion.
2. **A comparability gate now runs before the ranking.** A candidate that a user cannot adopt *instead of*
   AnxietyWatch is not a comparison candidate, whatever its technical relevance. This is the gate OSCAR
   failed, and it is applied to every row in §4.
3. **Inspection is no longer a ranking dimension.** v1's dimension F mixed "evidence quality" with "can we
   see inside it", which quietly rewarded candidates with published APIs. Evidence quality still scores;
   whether we can inspect the artifact does not, because inspection is now mandatory for every candidate.
4. **A liveness requirement.** A candidate a user cannot obtain today (withdrawn, region-locked,
   prescription-gated with no consumer route) is recorded as an evidence comparator and marked as such,
   rather than ranked as an alternative.

**Fact provenance.** Vendor, platform and URL facts below were verified during the 2026-10-06/07
reconnaissance. They are carried over here as the starting point **and re-verified by the pipeline for
each candidate before its research is written** — a shortlist entry is a hypothesis, not a finding.

---

## 1. Capability inventory used for selection

The shortlist covers the capabilities below, not the most visible anxiety-journal features. Sources:
`../AnxietyWatch/README.md`, `../AnxietyWatch/docs/FEATURES.md`, `../AnxietyWatch/AnxietyWatch/Services/`,
this repository's `README.md` and `openspec/changes/`.

| # | Capability | Status | Note |
|---|---|---|---|
| C1 | Subjective anxiety severity logging (1–10), notes, tags | v1 | plus watchOS quick-log |
| C2 | Random/unsolicited mood check-ins | v1 | |
| C3 | Medication dose logging | v1 | |
| C4 | Dose-triggered before/after efficacy measurement (N-of-1) | v1 | paired 0-min / 30-min rating |
| C5 | HealthKit ingestion of 20+ types with source attribution | v1 | multi-source provenance |
| C6 | Personal rolling baselines, self-referenced not population-referenced | v1 | 30-day HRV baseline |
| C7 | Physiological correlation engine | v1 | first intelligence layer |
| C8 | Polar H10 raw RR / beat-to-beat HRV, background recording | v1 | |
| C9 | CPAP data via SD-card import (AirSense 11, OSCAR CSV, EDF leak) | v1 | retrospective |
| C10 | EMAY overnight pulse-oximeter import + live BLE streaming | v1 | |
| C11 | Clinical PDF report generation | v1 | |
| C12 | Self-hosted sync server, no third-party cloud | v1 | |
| C13 | Optional LLM analysis over a selected date range | v1 | opt-in |
| C14 | Barometric pressure / environmental sound context | v1 | |
| C15 | Song/lyric (earworm) capture | v1 | |
| C16 | Oura OAuth, FHIR lab results, prescription scanning | v1 | breadth of integrations |
| C17 | Always-on BLE collection on a Raspberry Pi | v2 current | collectors + `vitalsd` |
| C18 | Provenance-preserving append-only store | v2 current | `(source,type,session_id,seq)` |
| C19 | Pure `no_std` decoders + fuzz targets, reject-never-clamp | v2 current | |
| C20 | Real-time AirSense 11 live stream | v2 current | bounded read-only path |
| C21 | Three-state honesty model (good / bad / not measured) | v2 current | architectural invariant |
| C22 | Quality gate → personal baseline → tier machine (`cns-engine`) | v2 specified | designed, not built |
| C23 | Supervised alarm channels + hardware dead-man switch (`klaxond`) | v2 specified | designed, not built |
| C24 | Independent alarm path surviving phone or internet loss | v2 specified | |
| C25 | Aircraft / environmental-noise context as a sleep interrupter | idea | see `PROJECT-DESCRIPTIONS-AND-CATEGORIES.md` §13 |
| C26 | XE125 radar respiratory rate | v2 current | decoder + OpenSpec change |

**Selection scoring key** (0–3 each; the total sorts, the written rationale decides):

| Key | Dimension |
|---|---|
| A | Whole-product or major-layer relevance |
| B | Anxiety / mental-health / sleep / medication / physiological overlap |
| C | Objective-data and subjective-context integration |
| D | Real-time, overnight, or always-on capability |
| E | Privacy, local-first, self-hosting, exportability |
| F | Evidence quality, availability and maturity **of the product** (not of its API) |

---

## 2. Comparability gate

| Tier | Test | Consequence |
|---|---|---|
| **T1 — product competitor** | A person with AnxietyWatch's use case could choose this product *instead of* AnxietyWatch | Admitted; ranked by score |
| **T2 — platform comparator** | A person would deploy or self-host this instead of the AnxietyWatch2 platform | Admitted with the label; A and F weigh differently |
| **T3 — tool, format or data source** | AnxietyWatch *consumes* or interoperates with it | **Not admitted.** Recorded in §4 with the reason |

---

## 3. The ten candidates

| Rank | Slug | Name | Tier | Primary layer | Score |
|---|---|---|---|---|---|
| 1 | `bearable` | Bearable | T1 | v1 whole product | 13 |
| 2 | `visible` | Visible | T1 | v1 physiology + v2 alarm | 15 |
| 3 | `welltory` | Welltory | T1 | v1 physiology + analysis | 14 |
| 4 | `empatica` | Empatica (EmbracePlus / EpiMonitor) | T1 | v2 CNS + alarm | 14 |
| 5 | `oura` | Oura | T1 | v1 wearable physiology | 13 |
| 6 | `nightscout` | Nightscout | T2 | v2 platform | 15 |
| 7 | `homeassistant` | Home Assistant | T2 | v2 platform | 13 |
| 8 | `sleephq` | SleepHQ | T1 | v1 CPAP + oximetry | 12 |
| 9 | `mindshift` | MindShift CBT | T1 | v1 adjacent category | 12 |
| 10 | `exist` | Exist.io | T1 | v1 correlation engine | 11 |

### 1. Bearable — `bearable` — T1 — 13

**Vendor** Bearable Ltd (UK). **URL** https://bearable.app/ · iOS `id1482581097` · Android `com.bearable`.
**Overlaps** AnxietyWatch v1 as a whole product. **Score** A 3 · B 3 · C 2 · D 1 · E 1 · F 3.

**Why first.** The closest whole-product analogue to v1's daily loop — mood, symptom, medication and factor
logging → correlation reports → a better clinician conversation. It is the app a prospective v1 user is
most likely already using, which makes it the most decision-relevant comparison in the set.

**Dimensions.** Logging burden and customisation; what the correlation engine may be computed over;
Free-versus-Premium gating of the analysis layer; CSV export versus PDF clinical reporting; cloud account
and server-side encryption versus local-first storage; HealthKit as one-way import versus
provenance-preserving capture; the absence of any real-time or overnight monitoring.

**Uncertainties.** Subscription prices are not published on the website (Play lists an in-app-purchase
range only). Whether arbitrary imported HealthKit metrics enter the correlation engine is undocumented. The
cited paper concerns patient-reported-outcome collection, not correlation validity. The "2M patients" figure
is store-listing copy.

### 2. Visible — `visible` — T1 — 15 (scored in the v2 revision)

**Vendor** Visible Health Inc. (US) / Visible Health UK Ltd. **URL** https://makevisible.com/ ·
iOS `com.makevisible.visible` (id `1624474919`) · Android `com.makevisible.visible` (100K+ installs).
**Overlaps** v1 physiology and v2's alarm path. **Score** A 2 · B 3 · C 3 · D 3 · E 1 · F 3.

**Why in the ten.** It is the only candidate that combines the two things the other nine never did: a
**shipped continuous-monitoring consumer alert** ("Get a phone notification when you are exerting or pacing
ahead for the day") competing directly with v2's specified alarm path, and a **vendor-authored
peer-reviewed paper on the vendor's own data** (npj Digital Medicine, 2026, n = 4,244) whose
competing-interests statement names the founder among the co-authors. Admitted to the ten in the v2
revision, having been a §4 reserve in v1.

**Dimensions.** Camera-PPG morning reading versus the Continuous/overnight mode; the Stability Score's
four-day minimum and ninety-day baseline; the missing-data rule (no band at night means no score, with no
morning substitute); PacePoints and the two alert types; what the CSV export omits (live continuous heart
rate and Heart Rate Zones) and what only the clinician dashboard shows; the consent-mode regional split in
the served client; and the evidence scope of the published paper versus the tier it is used to sell.

**Uncertainties.** Whether the paid continuous tier has any independent validation at all (the paper
validates the free tier's daily reading). The four conflicting statements about the reading's minimum age,
and the vendor's own "create an account using your own age" onboarding instruction.

### 3. Welltory — `welltory` — T1 — 14

**Vendor** Welltory Inc. **URL** https://welltory.com/. **Overlaps** v1 physiology and AI layers.
**Score** A 2 · B 3 · C 3 · D 2 · E 1 · F 3.

**Why.** The strongest comparator for the biomarker-and-automated-analysis half of v1: HRV-derived
stress/energy, an "AI" interpretive layer, and a broad wearable integration list. It is the test of whether
a personal-baseline approach is genuinely different from a population/algorithmic norm.

**Dimensions.** Spot HRV versus continuous overnight capture; proprietary inference versus published
baselines; how much of the "insight" is explainable; subscription lock-in; cloud processing of
cardiovascular data; whether subjective input is ground truth or a soft signal.

**Uncertainties.** "100+ biomarkers" and the peer-reviewed-validation claims need per-claim verification
against the actual publications. The measurement inputs and their error characteristics are not
established from the homepage.

### 4. Empatica — `empatica` — T1 — 14

**Vendor** Empatica Inc. **URL** https://www.empatica.com/embraceplus/. **Overlaps** v2's CNS-monitoring
and alarm layers (C22–C24). **Score** A 2 · B 2 · C 3 · D 3 · E 1 · F 3.

**Why.** The most relevant existing product for the part of v2 that does not exist yet: autonomic
monitoring from a body-worn sensor with an independent alert path and caregiver notification — the clearest
picture of what a clinically validated version of `klaxond`/`cns-engine` looks like, and what evidence and
regulatory burden it carries.

**Dimensions.** Alarm channel design, latency and failure behaviour; false positives and not-measured
states; regulatory classification and clinical evidence (the regulator record is a first-class surface
here); closed wearable ecosystem versus open BLE; cost and availability; privacy of continuous
physiological data.

**Uncertainties.** Consumer availability is in flux and much of the current offering is oriented to
clinical trials. Regulatory status per market is established from the regulator record during the run.

### 5. Oura — `oura` — T1 — 13

**Vendor** Ōura Health Oy. **URL** https://ouraring.com/. **Overlaps** v1 wearable physiology and v2's
planned import. **Score** A 2 · B 2 · C 2 · D 3 · E 1 · F 3.

**Why.** v2 explicitly plans Oura as a data import and v1 has an Oura OAuth manager — so it is both a
comparator and a potential source. It is also the strongest example of an overnight-physiology wearable
with its own closed interpretation layer, directly comparable to v1's personal-baseline framing and to
v2's refusal to render a value that was not measured.

**Dimensions.** Overnight sensor set versus the v2 fleet; closed interpretation versus receipts and
provenance; subscription gating of insights *and* of raw-data access; API and export availability; whether
the platform can serve as a source rather than a competitor.

**Uncertainties.** Marketing claims such as "86% of Oura Members see their health improve" carry footnotes;
raw-data API access may require membership and the exact tier boundaries need in-product verification.

### 6. Nightscout — `nightscout` — T2 — 15

**Vendor** the #WeAreNotWaiting community (`nightscout/cgm-remote-monitor`, AGPL-3.0).
**URL** https://nightscout.github.io/. **Overlaps** the v2 platform layer. **Score** A 3 · B 1 · C 2 · D 3 · E 3 · F 3.

**Why.** The closest existing model for what v2 attempts at the platform layer: self-hosted, continuously
ingested physiological data, user-owned, with community uploaders/downloaders and an alerting path. v2
should be measured against how Nightscout handles deployment, access control and always-on operation on a
Pi — and against its mistakes, which are visible in its own source.

**Dimensions.** Self-hosting ergonomics and documented deployment paths; security and privacy documentation
for a health endpoint; retention and history; alerting; the gap between a single well-supported consumer
device (CGM) and a heterogeneous BLE sensor fleet; project maturity versus v2's.

**Uncertainties.** Single-modality physiology (glucose), so overlap is architectural rather than clinical.
The security posture of typical DIY deployments is criticised publicly and is assessed here from the
published source, not from other people's instances.

### 7. Home Assistant — `homeassistant` — T2 — 13

**Vendor** Open Home Foundation. **URL** https://www.home-assistant.io/. **Overlaps** the v2 platform
layer: always-on edge, local automation, local alerting. **Score** A 3 · B 0 · C 1 · D 3 · E 3 · F 3.

**Why.** The most mature open-source local-first edge platform, with a Pi target, automations that run
without internet, a 1,500+ integration fleet, and a documented local-only stance. v2's admin console,
collector service, deployment story and alarm path all have direct equivalents here, which makes it the best
available benchmark for operational quality rather than clinical function.

**Dimensions.** Local-only guarantees; automation and alarm reliability including failure signalling;
integration and onboarding ergonomics; deployment, backup and upgrade story; how *unknown* is surfaced; and
how a health platform must differ from a general-purpose hub.

**Uncertainties.** No physiological or clinical function; the overlap is entirely architectural. Published
source is inspected at a pinned tag, so findings are `[STATIC]` unless a running instance is observed.

### 8. SleepHQ — `sleephq` — T1 — 12

**Vendor** SleepHQ. **URL** https://sleephq.com/. **Overlaps** v1's CPAP/SpO₂ import and v2's specified
reporting and sharing. **Score** A 2 · B 3 · C 2 · D 1 · E 1 · F 3.

**Why.** It covers the combination this project actually cares about — AirSense 11 data plus overnight SpO₂
plus longitudinal statistics — and adds the clinician and peer-sharing workflow that we approach via PDF
reports and a self-hosted server.

**Dimensions.** CPAP data depth and refresh latency (nightly import versus live stream); combined CPAP +
oximetry analysis; community sharing versus private self-hosting; subscription cost; whether any of it can
inform anything about anxiety.

**Uncertainties.** Pricing tiers and data-export rights need in-product verification. Install figures on the
site are marketing copy.

### 9. MindShift CBT — `mindshift` — T1 — 12

**Vendor** 247 Labs Inc. **URL** https://mindshiftcbt.com/. **Overlaps** v1's anxiety-specific layer and
its free consumer positioning. **Score** A 1 · B 3 · C 1 · D 1 · E 3 · F 3.

**Why in the ten.** It is the deliberate *category* comparator. AnxietyWatch's name places it in the
anxiety-app market, where the dominant purpose is intervention — CBT tools, thought records, exposure —
rather than measurement and correlation. A free consumer tier with a paid B2B surface beside it makes it the
fairest test of whether AnxietyWatch is "another mood tracker", and a useful discoverability benchmark for
the anxiety search category.

**Dimensions.** Purpose (change anxiety now versus understand anxiety over time); whether physiology is
involved at all; monetisation and data posture; evidence base and clinical review; and what each product
does *during* a panic attack versus after one.

**Uncertainties.** The consumer app is distributed as a store binary; the served web client is the
reachable published surface. The evidence base is asserted by the publisher and is checked against the
registration and the publication during the run.

### 10. Exist.io — `exist` — T1 — 11

**Vendor** Exist (Hello Code). **URL** https://exist.io/. **Overlaps** v1's correlation engine.
**Score** A 2 · B 2 · C 3 · D 1 · E 1 · F 2.

**Why.** The most directly comparable *correlation engine* to v1's baseline and correlation work, including
weather and calendar context that overlaps this project's environmental-context ambitions (C14, C25). It
also tests the value of merging objective service data with manually entered attributes.

**Dimensions.** Breadth of automatic sources versus physiological depth; statistical presentation and
false-positive risk; personal baselines versus population comparisons; cloud-only architecture;
exportability and account requirement.

**Uncertainties.** Current pricing, export fidelity, and the statistical method behind its correlation
findings are established from its published API reference and documentation during the run.

---

## 4. Considered and not selected

### 4.1 Removed from the list in the v2 revision

| Product | Why it was in v1's list | Why it is out |
|---|---|---|
| **OSCAR** (sleepfiles.com/OSCAR, GPLv3, Apnea Board community) | Free and permanently free; the practical destination of the SD-card data v1 imports; the reference for what a sleep clinician looks at | **T3 — it is a tool and a data source, not a competitor.** It reads the same SD-card export this project ingests (`OSCAR CSV` is already a v1 import path). A user does not choose OSCAR *instead of* AnxietyWatch; we can consume it. Instrumental value: it stays useful to this project as a **dependency and as a reference implementation** for respiratory-event depth — that belongs in `docs/`, not in the comparison set. Its CPAP-data model is the yardstick v1's import path should be measured against, and that measurement is a task for the CPAP workstream. |

### 4.2 Reserves — admitted if a ten-slot opens

| Product | Tier | Why it is the next one in |
|---|---|---|
| **eMoods** (`emoodtracker.com`; iOS `com.yottaram.eMoods`, Android `my.tracker`, 500,000+ installs, "No data collected" on Play) | T1 | The strongest free, fully offline, no-account comparator with printable clinician PDF reports — the closest match to v1's core loop from the privacy side. Already verified live and current (v1.3.28, updated 2025-12-07). Held in reserve rather than admitted, so the ten keep their layer coverage. |
| **Daylio** | T1 | The most widely used micro-journal; a discoverability benchmark for the mood-tracker category. Overlaps Bearable with less breadth; would add a second mood-tracker row rather than a new dimension. |
| **Withings Sleep Analyzer / Somnofy** | T1 (subsystem) | Under-mattress and radar-based home sleep-apnea monitoring — the closest commercial analogue to the XE125 radar respiratory-rate work (C26). Best admitted once the XE125 decoder is deployed and can be compared on measured output. |
| **Medisafe** | T1 (subsystem) | The best-known medication-reminder app. Reminders and adherence only — no before/after efficacy measurement and no physiology — so it would not test C4, which is the differentiator. |
| **Loop / OpenAPS / AndroidAPS** | T1 (subsystem) | Self-built closed-loop systems with local control and safety interlocks; architecturally instructive for the alarm path. The alerting comparison is better served by Empatica and the self-hosting comparison by Nightscout. |

### 4.3 Excluded on the comparability gate

| Product / thing | Why considered | Why excluded |
|---|---|---|
| **Apple Health Records / FHIR-based PGHD tools** | Directly relevant to v1's `FHIRLabResultParser` and laboratory-result import | **T3.** FHIR is a standard and Apple's tooling is a platform feature, not a product a user adopts instead of AnxietyWatch. Belongs in an interop note. |
| **CPAP vendor apps** (ResMed myAir, Philips DreamMapper) | They own the device data flow AnxietyWatch imports | **T3/adjacent.** They are device companion apps for a device we already read directly; the comparison they offer is to v1's *import path*, not to AnxietyWatch as a product. Recorded here so the omission is deliberate. |
| **Smartwatch platforms** (Apple Watch, Garmin, Fitbit) | They ship the sensors and some of the scores | **T2 at best and too broad.** A platform cannot be compared feature-by-feature with a product; individual apps on them are covered through the wearable candidates (Oura, Empatica). |

---

## 5. Known limitations of this shortlist

- Ten candidates cannot cover every subsystem. Aircraft/noise context (C25) and blood-pressure collection
  have no dedicated candidate.
- Bearable, Welltory and Oura all have subscription tiers whose exact current prices are not consistently
  published; cost comparisons are approximate until checked in-store.
- Empatica's consumer availability is in flux, so its row is an evidence-and-alerting comparator as much as
  a live alternative.
- The two platform comparators (Nightscout, Home Assistant) have almost no clinical overlap. They are
  ranked on their layer, not on product similarity, and their findings do not transfer to the v1
  comparison.
- No candidate combines real-time CPAP streaming, always-on BLE collection, provenance-preserving storage
  and an independent alarm path. That combination appears unoccupied, but ten products do not prove absence.
