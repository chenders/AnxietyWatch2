# Prompt 2 — Comprehensive candidate comparison

Use this as the default second-stage prompt for a selected candidate. Replace every `{PLACEHOLDER}` before using it.

## Assignment

You are comparing **{APP_NAME}** with AnxietyWatch v1 and AnxietyWatch2. Produce a careful, evidence-based report at:

```text
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_COMPARISON.md
```

The report must explain both feature-level overlap and meaningful product differences in prose. A grid alone is not sufficient.

## Read first

Read these local files:

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md
```

Also review the candidate's cited primary sources when available. Use the research date recorded in the candidate file. Do not silently substitute current marketing pages for older evidence without recording the change.

If this is an Android or iOS candidate, do not skip implementation inspection. Read `{APP_SLUG}_IMPLEMENTATION_INSPECTION.md` when present; if an authorized artifact exists but the file is absent, perform the inspection before finalizing or explicitly document why it could not be performed. Use it to compare public claims with static implementation evidence and authorized runtime observations. Treat the following as separate evidence classes:

- public claim;
- reachable behavior observed at runtime;
- static implementation evidence that was not observed at runtime;
- experimental, gated, dead, or unreachable code;
- inferred behavior;
- unknown or blocked behavior.

The inspection record should identify the exact artifact, version/build, platform, SHA-256, tools, device/emulator, authorization boundary, and limitations. It may cover manifests/entitlements, permissions, background modes, screens, data models, APIs, BLE and HealthKit identifiers, local storage, exports, notifications, analytics/advertising SDKs, subscriptions, feature flags, cloud dependencies, and error handling. Do not treat a string, endpoint, SDK, or code path as a user-accessible feature without evidence that it is reachable.

If an authorized artifact was unavailable or inspection was not permitted, say so explicitly in the report. Do not bypass DRM, authentication, licensing, encryption, certificate pinning, anti-debugging, or platform protections; do not extract credentials or private user data. Record blocked inspection as an evidence limitation, not as proof that the feature does or does not exist.

Use these AnxietyWatch status labels:

- **V1 documented/implemented:** represented as implemented by v1 code or documentation.
- **V2 current:** present and usable in the current AnxietyWatch2 repository.
- **V2 specified target:** fully specified in `openspec/`, but not necessarily implemented.
- **Unresolved:** mentioned but insufficiently specified or evidenced.

Never present a V2 specified target as a currently shipped feature.

## Comparison method

Compare the products at the correct level. {APP_NAME} may be a mobile app, wearable ecosystem, home-monitoring product, cloud service, research project, or only a subsystem competitor. State whether each conclusion concerns:

- the AnxietyWatch v1 application;
- the AnxietyWatch2 Raspberry Pi platform;
- the combined AnxietyWatch product vision; or
- only a particular subsystem such as CPAP, HRV, anxiety journaling, medication tracking, pulse oximetry, aircraft/noise context, AI analysis, or clinical reporting.

For every important capability, classify each product as:

- **Supported**
- **Partial or indirect**
- **Explicitly not supported**
- **Not found in reviewed evidence**
- **Unclear because of access, region, plan, or version limits**

Also record capability maturity: current, beta, experimental, planned, discontinued, or unknown.

Do not infer a feature merely because a product has a nearby feature. For example, a sleep score is not automatically CPAP support; an AI chatbot is not automatically longitudinal multimodal analysis; cloud sync is not self-hosting; and a notification is not an independent local alarm.

## Required report structure

### 1. Executive summary

Give a concise answer to:

- What is {APP_NAME} primarily for?
- Which AnxietyWatch version or subsystem is the closest comparison?
- Where is {APP_NAME} clearly stronger?
- Where is AnxietyWatch clearly stronger?
- Where are they complementary rather than direct competitors?
- What is the largest unresolved uncertainty?

### 2. Scope and evidence boundary

Record the candidate's platforms, availability, region/plan restrictions, hardware requirements, research date, and source limitations. Separate public claims, documented behavior, independent observation, inference, and unknowns.

Use citations linked to the candidate evidence file. Add new sources only when necessary and record URL, title, publisher, date, and access date.

### 3. Claim-versus-implementation reconciliation

When implementation inspection is available, compare major public claims against static findings and authorized runtime observations. Identify features that are:

- claimed and observed;
- claimed but not observed;
- implemented and reachable but not publicly advertised;
- present only in experimental, gated, dead, or unreachable code; or
- unresolved because inspection was blocked or incomplete.

Include the artifact/version boundary and do not disclose credentials, private data, or sensitive exploit details. If inspection was unavailable, state why and how that limits the comparison.

### 4. Feature-level comparison

Include a table with these columns:

| Capability | AnxietyWatch v1 | AnxietyWatch2 current | AnxietyWatch2 specified target | {APP_NAME} | Evidence/status notes |
|---|---|---|---|---|---|

Cover, where relevant:

- anxiety, panic, mood, symptom, and free-text journaling;
- medication, dose timing, adherence, and personal efficacy follow-up;
- HealthKit, Apple Watch, Polar H10, HRV, pulse, SpO₂, respiration, and blood pressure;
- sleep, CPAP, AirSense, respiratory events, and overnight operation;
- aircraft, environmental noise, barometric/weather context, and sleep-interruption attribution;
- real-time streams versus nightly imports;
- raw data, summaries, provenance, quality states, stale data, and missing data;
- personal baselines, correlations, longitudinal analysis, and AI review;
- lyrics/song context and other unusual personal inputs;
- clinical reports, exports, provider sharing, and audit history;
- local alarms, technical alarms, phone/cloud notifications, and failure independence;
- privacy, self-hosting, data ownership, account requirements, and telemetry;
- cost, subscriptions, hardware, electricity, and optional AI/API charges.

### 5. Narrative comparison by user journey

Compare concrete scenarios:

1. Capturing anxiety during a panic episode
2. Understanding a poor night's sleep
3. Investigating whether CPAP or respiratory events mattered
4. Testing a medication over repeated doses
5. Finding whether an aircraft/noise event coincided with an awakening
6. Reviewing months of data with a clinician
7. Continuing to collect or alarm when the phone, internet, or cloud is unavailable

For each scenario, explain the data path, user effort, feedback timing, and important limitations.

### 6. Privacy, operating model, and cost

Explain where data is collected, processed, stored, synchronized, exported, retained, and deleted. Identify cloud dependencies, account requirements, vendor lock-in, telemetry, model/API dependencies, and hardware dependencies. Distinguish free software from free operation.

### 7. Balanced conclusion

State:

- best-fit user for {APP_NAME};
- best-fit user for AnxietyWatch v1;
- best-fit user for AnxietyWatch2;
- strongest overlaps;
- important gaps on both sides;
- whether the products are competitors, complements, or competitors only in one subsystem;
- which findings are well-supported and which remain uncertain.

Do not describe either project as a validated diagnostic, medical device, or emergency monitor unless the evidence explicitly supports that claim. Do not turn correlation into causation.
