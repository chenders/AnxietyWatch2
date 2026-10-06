# AnxietyWatch / AnxietyWatch2: Discoverability Descriptions and Categories

## Scope and interpretation

This review covers:

- `AnxietyWatch2` — the current repository (`./`)
- `AnxietyWatch v1` — `../AnxietyWatch`

For this exercise, fully specified AnxietyWatch2 features in `openspec/` are treated as part of the product scope. AnxietyWatch v1's aspirational roadmap is not treated as implemented unless its code or documentation presents the capability as implemented.

This synthesis draws from the repositories' READMEs, requirements, feature documentation, architecture and client specifications, OpenSpec proposals and task queues, and the principal source trees.

## Overall positioning

### AnxietyWatch v1

AnxietyWatch v1 is primarily a feature-rich iPhone/watchOS application centered on:

- Anxiety journaling and rapid panic-time logging
- Medication, prescription, pharmacy, and dose-follow-up tracking
- HealthKit and Apple Watch physiological data
- Polar H10 beat-to-beat HRV
- EMAY pulse oximetry and Oura data
- CPAP/AirSense data import
- Personal baselines and anxiety correlations
- Clinical PDF reports and JSON/CSV export
- Self-hosted Flask/PostgreSQL synchronization
- Optional server-side AI analysis
- Early CNS-depression detection research

Its primary identity is **an anxiety self-management and personal health-informatics app**.

### AnxietyWatch2

AnxietyWatch2 expands the system so that the authoritative center is a Raspberry Pi rather than the phone. Its planned product scope includes:

- Always-on BLE and physiological sensor collection
- Polar H10, EMAY, blood-pressure cuff, AirSense 11, HealthKit, CPAP SD/Wi-Fi ingestion, and future XE125 radar sensing
- Registry-driven units, ranges, provenance, and source identity
- Durable append-only spooling and PostgreSQL/TimescaleDB storage
- Rust decoders and a Linux BlueZ broker
- Personal-baseline CNS monitoring and fused alert tiers
- Independent local alarm channels, dead-man supervision, and technical alarms
- Rust admin console, bedside display, reports, exports, AI review jobs, and audit trails
- Secure iPhone HealthKit/journal client with durable outbox delivery
- Watch Quick Log
- Optional iPhone BLE mobility relay and ambient bracelet indicator
- Self-hosted operation, encrypted backups, restore verification, staged deployment, and degraded-mode recovery

Its primary identity is **a local-first home physiological-monitoring and safety platform with a mental-health data layer**.

## 10 search-ready descriptions

These descriptions are intentionally phrased as search concepts for finding similar applications, research projects, or open-source systems.

1. **Local-first Raspberry Pi physiological monitoring system with BLE wearable sensors and an independent bedside alarm**

2. **Privacy-preserving overnight sleep, respiratory, and CNS-depression monitoring platform for home use**

3. **Open-source personal health data platform combining anxiety journaling, medication tracking, HealthKit, CPAP, pulse oximetry, and HRV**

4. **Wearable vital-sign gateway with Rust protocol decoders, source provenance, durable time-series storage, and data-quality states**

5. **Anxiety self-tracking app that correlates subjective mood ratings with HRV, sleep, CPAP quality, exercise, medications, and wearable data**

6. **Quantified-self mental-health application with personal baselines, longitudinal correlations, medication efficacy measurement, and clinical reports**

7. **Patient-generated health-data system for preparing evidence-based psychiatric and sleep-medicine appointments**

8. **Home sleep-apnea and anxiety monitoring system integrating AirSense CPAP data, overnight SpO₂, respiration, pulse, and HRV**

9. **Self-hosted personal health observability stack with edge collection, private dashboards, exports, audit trails, backups, and optional AI analysis**

10. **Thin iPhone HealthKit and journal client backed by a Raspberry Pi physiological system of record, with offline capture and secure synchronization**

### Useful discovery keywords

- `local-first health monitoring`
- `Raspberry Pi vital signs`
- `BLE pulse oximeter logger`
- `home CNS depression monitoring`
- `anxiety HRV journal`
- `CPAP anxiety correlation`
- `personal health informatics`
- `quantified-self mental health`
- `self-hosted HealthKit`
- `patient-generated health data`
- `open-source wearable gateway`
- `overnight respiratory monitoring`
- `personal medication efficacy tracking`
- `clinical health data export`
- `edge computing health monitoring`

## What makes the product stand out

The following capabilities are more distinctive than a generic health dashboard or anxiety journal. They are useful both as product-positioning language and as search terms for finding comparable projects.

### 1. Real-time CPAP integration, not just nightly summaries

The system is designed to consume the AirSense 11 while therapy is happening: live flow, pressure, leak, respiratory-rate, ventilation, SpO₂, pulse, and related therapy signals. That enables context-aware monitoring such as distinguishing a genuine physiological change from a mask-off, large-leak, stalled-bridge, or otherwise mechanically invalid measurement.

This is substantially different from importing yesterday's AHI and usage report. It connects the therapy machine itself to the monitoring pipeline in near real time while keeping the integration read-only and prohibiting therapy-control commands.

**Search terms:** `real-time CPAP monitoring`, `AirSense 11 live flow pressure stream`, `CPAP mask-off detection`, `read-only CPAP BLE integration`.

### 2. A genuinely multi-modal account of a person, not only sensor numbers

A selected analysis period can combine:

- HRV, beat-to-beat RR intervals, heart rate, SpO₂, respiration, sleep, exercise, activity, and blood pressure
- CPAP usage, AHI, leak, pressure, flow, and respiratory context
- Anxiety ratings, free-text notes, tags, random check-ins, pain or symptom context, and medication doses
- Medication follow-up ratings that create before/after efficacy observations
- Songs that were stuck in the user's head, song occurrences, artist/album metadata, lyrics, and timing
- Barometric pressure and pressure changes as local weather-front/environmental context
- Prior correlations, clinical reports, and previous analysis results near the newly selected date range

The distinctive idea is that the subjective record is not treated as noise around the physiology. The song, note, medication, sleep, weather context, and physiological signals are all possible explanations for the same day.

### 3. Date-range AI analysis that uses the whole longitudinal context

The analysis workflow is intended to accept a selected date range and examine the complete available context rather than a single exported metric. It can combine raw observations, daily summaries, journal material, medications, CPAP data, songs and lyrics, barometric context, existing correlations, and nearby historical findings.

A strong analysis run should be able to say not merely that HRV was low, but that a particular combination—such as poor sleep, CPAP leak, elevated pressure change, a medication event, and a recurring song or journal theme—resembled or differed from earlier periods.

Previous runs are treated as evidence to compare and review, not as unquestioned truth. AnxietyWatch2's specified AI workflow includes supervised jobs, cost and egress preflight, persisted prior/candidate versions, side-by-side review, explicit acceptance, audit records, and failure states that cannot be presented as successful analysis.

**Important cost qualification:** the software is designed to be free/open source and self-hostable, but an external commercial model API may charge for usage. The product should therefore be described as having no subscription or platform fee, not as guaranteeing zero cost for every optional AI provider.

### 4. A personal N-of-1 laboratory for medication efficacy

The dose workflow records anxiety at medication time and prompts for a later rating. Repeated over weeks, this produces individual before/after observations for each drug and dose. It can reveal whether a medication appears helpful for this person, whether the effect is changing, and whether usage frequency and perceived benefit are moving in opposite directions.

That is more specific than a medication reminder and more useful than a generic mood chart: it creates a longitudinal personal evidence base for a clinician conversation without pretending to establish causality.

**Search terms:** `personal medication efficacy tracker`, `N-of-1 anxiety medication study`, `dose response mood journal`.

### 5. High-fidelity Polar H10 HRV rather than occasional smartwatch snapshots

The Polar H10 integration captures individual RR intervals over BLE, derives per-minute RMSSD, SDNN, pNN50, LF power, HF power, and LF/HF measures, supports overnight/background recording and state restoration, and preserves raw RR archives so calculations can be revisited later.

This makes the project useful both as an anxiety application and as a personal autonomic-physiology data-collection platform. It also provides a stronger signal for overnight analysis than relying only on sparse Apple Watch HRV samples.

### 6. Reverse-engineered support for an inexpensive, accurate oximeter

The EMAY SleepO2 integration is a standout hardware-access feature. Rather than requiring a premium clinical monitor, the project decodes a vendor-specific Bluetooth protocol from an inexpensive consumer pulse oximeter and turns its overnight SpO₂ and pulse stream into usable, provenance-preserving data.

The repository documents protocol discovery through vendor-app decompilation, including a checksum detail that ordinary awake captures would not reliably expose. The project can accurately be described as supporting a previously undocumented or previously unsupported device protocol in the open-source ecosystem. A claim that it was literally “never cracked anywhere in the world” would require external evidence and should not be made without it.

**Search terms:** `reverse engineered SleepO2 BLE protocol`, `open-source EMAY oximeter`, `inexpensive overnight pulse oximeter data logger`, `consumer oximeter Bluetooth decoder`.

### 7. Multiple independent sources with explicit disagreement and missing-data semantics

The system does not simply average every available signal. It preserves which device produced each observation, its identity and timing, quality state, freshness, and whether it was absent or invalid. It distinguishes measured-good, measured-bad, and not-measured.

Missing physiology does not become zero, a stale stream does not become healthy, a bad CPAP stream does not silently support an alarm, and gaps are not interpolated into reassuring lines. This is a major differentiator from consumer dashboards that display a polished number without explaining its provenance or age.

### 8. A safety architecture where the phone and screens are not dependencies

The intended overnight path runs on a mains-powered Raspberry Pi with a durable spool, local storage, watchdog supervision, resource priorities, independent alarm channels, and a hardware dead-man path. The iPhone, web dashboard, bedside display, house internet, and cloud notification providers are viewers or redundant channels—not prerequisites for local collection and alarm behavior.

The system also distinguishes physiological alarms from technical alarms. A dead sensor, failed database, stale stream, or failed backup should be shown as a named infrastructure problem rather than mistaken for either a healthy person or a physiological emergency.

### 9. Panic-aware interaction plus rich retrospective analysis

The product serves two very different moments:

- During panic: one-tap or sub-five-second severity capture, large accessible controls, a Watch Quick Log, medication “last taken” context, and minimal cognitive load.
- During calm review: trend charts, personal baselines, compound triggers, medication timelines, song context, clinical reports, and AI-assisted analysis.

Many applications optimize for either rapid logging or deep analysis. This product explicitly tries to support both ends of the cycle without making the panic-time flow depend on completing a full form.

### 10. Free, open, private, and export-first by design

The projects are MIT-licensed and designed without subscriptions, advertising, telemetry, engagement tracking, or a required vendor cloud. Data can remain on personally controlled hardware, and the system supports JSON/CSV export, clinical PDFs, self-hosted storage, encrypted backups, and restore verification.

The honest positioning is:

- free and open-source software;
- no required subscription or hosted account;
- self-hostable and export-first;
- hardware, electricity, storage, and optional third-party AI/model costs are still real costs.

This combination—deep physiological collection, personal mental-health context, local authority, and inspectable/exportable data—is unusual even when individual features exist elsewhere.

### 11. A previously unsupported device becomes a research instrument

The project does not stop at the devices with polished public APIs. It investigates vendor protocols, records raw evidence before decoding, fuzzes hostile input, preserves replay fixtures, and promotes new sources only after validation. The same pattern applies to EMAY, AirSense, BlueZ, and the proposed XE125 radar path.

That makes AnxietyWatch2 partly a health application and partly an open hardware/protocol-archaeology project: inexpensive hardware can become useful when its data path is understood, tested, and documented.

### 12. Clinical communication without pretending that correlation is diagnosis

The system is designed to turn months of fragmented personal observations into a concise clinician-facing report: anxiety distribution, medication use and follow-up, sleep and CPAP quality, HRV trends, data completeness, notable correlations, and explicit gaps.

It aims to improve the psychiatric or sleep-medicine conversation while preserving the distinction between observational correlation, clinical interpretation, and validated diagnosis.

### 13. Tracking aircraft and environmental noise as possible sleep interrupters

The broader system also tracks aircraft activity and environmental noise so that a bad night is not reduced to an unexplained sleep score. Aircraft observations can include nearby flight presence and movement context such as range, altitude, bearing, phase, freshness, and map/history links; environmental sound exposure can come from HealthKit and related sensors.

The important product idea is attribution: compare aircraft and noise events against sleep stages, awakenings, heart rate, HRV, respiratory signals, CPAP events, and the user's morning anxiety report. This can help answer questions such as whether a particular flight corridor, recurring overnight noise pattern, or loud event coincides with fragmented sleep and a worse next day.

This is especially distinctive because the system joins an external environmental explanation—planes and noise—to the same personal timeline as physiology, CPAP, medication, subjective symptoms, songs, and anxiety. The aircraft/noise tracker is currently a separate operational surface, so a unified product description should say that the capability exists in the ecosystem while noting that console integration remains a product-boundary issue.

**Search terms:** `aircraft noise sleep tracker`, `flight tracking sleep disruption`, `environmental noise and sleep correlation`, `personal sleep interruption attribution`, `Raspberry Pi aircraft noise monitor`.

## Especially distinctive combined description

> **An open-source, privacy-first personal health laboratory that combines panic-aware anxiety journaling, medication N-of-1 tracking, high-fidelity wearable HRV, inexpensive reverse-engineered pulse oximetry, real-time CPAP therapy data, aircraft and environmental-noise tracking, weather context, and longitudinal AI analysis—running on a local Raspberry Pi rather than depending on a phone or cloud service.**

## Claims to phrase carefully

- Say **“real-time AirSense therapy-stream integration”**, not that every CPAP analysis is real-time; nightly SD/EDF imports remain a separate path.
- Say **“barometric pressure and weather-front context”** unless an external weather-service integration is explicitly added; the documented implementation primarily measures local barometric pressure.
- Say **“previously undocumented or unsupported EMAY/SleepO2 protocol”** rather than globally claiming that nobody had ever cracked it.
- Say **“no subscription or required hosted service”** rather than promising that hardware, electricity, or optional AI API calls cost nothing.
- Say **“experimental early-warning and safety research”**, not “medical device,” “diagnostic,” or “emergency monitor.”

## Best-fit categories

1. **Mental health and anxiety tracking**  
   Best primary category for AnxietyWatch v1.

2. **Personal health and wellness monitoring**  
   The broadest category covering both projects.

3. **Sleep and respiratory health**  
   Covers CPAP, overnight SpO₂, respiration, sleep quality, and AirSense integration.

4. **Home physiological monitoring**  
   Best category for the Raspberry Pi platform, with the caveat that it is experimental and non-clinical.

5. **Wearable and BLE health-device companion**  
   Covers Polar, EMAY, blood-pressure, AirSense, Oura, and mobility-relay integrations.

6. **Medication management and adherence**  
   Covers dose logging, prescriptions, follow-ups, personal efficacy, and medication context.

7. **Quantified self and personal informatics**  
   Covers personal baselines, self-experimentation, longitudinal correlations, and data export.

8. **Patient-generated health data / clinical care preparation**  
   Covers clinical reports, data completeness, provider discussions, and evidence-backed appointments.

9. **Personal safety and assistive alerting**  
   Covers the local klaxon, bed shaker, remote notifications, dead-man supervision, and technical alarms.

10. **Privacy-first self-hosted health infrastructure / edge IoT**  
    Especially appropriate for AnxietyWatch2's Rust, Raspberry Pi, private-LAN, backup, and deployment architecture.

## Recommended store-style positioning

For a conventional mobile-store classification:

- **Primary:** Health & Fitness
- **Secondary options:** Medical, Mental Health, Sleep, or Personal Health Monitoring

The projects should not be described as clinically validated medical devices, diagnostic systems, or emergency monitors. Their accurate positioning is experimental, privacy-first personal monitoring and health-data software with a future-oriented home safety path.
