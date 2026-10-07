# Prompt 1 — Competitive research for AnxietyWatch v1 and AnxietyWatch2

## Purpose

Create a research package that identifies the ten most relevant competing or adjacent products for **AnxietyWatch v1**
and **AnxietyWatch2**, researches their public claims, and prepares a tailored second-stage prompt for a deeper
comparison of each candidate.

This file is the research brief for a future AI research session. It should produce research artifacts, not merely a
list of product names or a superficial feature matrix.

## Execution modes and sequencing

Run this workflow in **pilot mode first** unless the owner explicitly requests full execution.

### Pilot mode

1. Build and save the complete ten-candidate shortlist.
2. Select only the highest-ranked candidate, or the specific candidate named by the owner.
3. Process that one candidate through every applicable step: public-claims research, authorized implementation
   inspection, candidate-specific Prompt 2, and the eventual Prompt 2 comparison report.
4. Validate the candidate's output files and record blockers, evidence gaps, tool failures, and process improvements.
5. Stop after that candidate. Do not begin research, decompilation, or comparison work on candidate 2 until the owner
   reviews the pilot and explicitly continues.

### Full sequential mode

After pilot approval, process candidates strictly in shortlist order. There must be exactly one **active candidate** at
a time. Finish all applicable work for the active candidate before opening the next candidate's research or
implementation artifact.

Do not parallelize candidate research, decompilation, runtime testing, web research, or Prompt 2 generation across
candidates. Do not prefetch or inspect the next app while the current app is unresolved. Subtasks for the active
candidate may be organized internally, but their results must be consolidated before moving on.

For each candidate, “complete” means that the following exist or explicitly document why they could not be produced:

- `{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md`
- `{APP_SLUG}_IMPLEMENTATION_INSPECTION.md`
- `PROMPT_2.md`
- `{APP_SLUG}_COMPARISON.md`

Maintain a queue/status record in `./docs/app_comparison/PROCESS_STATE.md` with the shortlist order, active candidate,
per-candidate status, start/completion timestamps, blockers, and the explicit reason for moving to the next candidate.
Never mark a candidate complete based only on a model's assertion; verify the output files and record the command or
inspection result used.

## Product context

Before researching competitors, read:

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
```

Treat fully specified AnxietyWatch2 features in `openspec/` as part of the intended product scope for comparison. Do not
automatically treat every aspirational AnxietyWatch v1 roadmap item as implemented; distinguish implemented v1
capabilities from future ideas in the source material.

Maintain these status labels throughout the research:

- **V1 documented/implemented:** represented as implemented by AnxietyWatch v1 code or documentation.
- **V2 current:** present and usable in the current AnxietyWatch2 repository.
- **V2 specified target:** fully specified in `openspec/`, but not necessarily implemented.
- **Unresolved:** mentioned but not sufficiently specified or evidenced.

A competitor may be a strong match for one layer and not another. Compare AnxietyWatch v1 as an application,
AnxietyWatch2 as a platform, and the combined product vision separately rather than treating them as interchangeable
apps.

Understand that the projects have different centers of gravity:

- **AnxietyWatch v1:** an iPhone/watchOS anxiety self-management and personal-health-informatics app with journaling,
  medication tracking, HealthKit, wearable physiology, CPAP, personal baselines, correlations, clinical reports, and
  self-hosted sync.
- **AnxietyWatch2:** a local-first Raspberry Pi physiological-monitoring platform that expands the v1 goals with
  always-on sensor collection, real-time CPAP integration, provenance-preserving storage, aircraft/noise context,
  CNS-depression monitoring, independent alarm architecture, server-side analysis, and a thin Apple client.

Do not reduce either product to “a mood tracker” or “a generic fitness tracker.” The comparison must account for both
the user-facing application and the underlying monitoring platform.

## Phase 1 — Select the top 10 candidates

Identify ten products that provide the most useful comparison, not merely ten popular mental-health apps. The list may
include commercial apps, open-source projects, research platforms, self-hosted systems, wearable ecosystems, or
specialized sleep/medical-data tools.

Use a balanced candidate mix where appropriate. Consider candidates in these groups:

1. Anxiety, panic, or mental-health journaling apps
2. Personal health and quantified-self platforms
3. Sleep, respiratory, CPAP, or pulse-oximetry products
4. Medication tracking and medication-efficacy tools
5. Wearable/HealthKit physiological monitoring products
6. Patient-generated health-data and clinical-reporting tools
7. Local-first, privacy-first, or self-hosted health systems
8. Home monitoring, edge-IoT, or personal safety systems

### Candidate-selection criteria

First build a short internal inventory of the major AnxietyWatch capabilities and assign each one a status label from
the Product context section. Use that inventory to avoid selecting candidates based only on the most visible
anxiety-journal features.

Score or qualitatively assess each candidate using:

- Similarity of purpose, not only similarity of UI
- Overlap with anxiety, panic, sleep, medication, or physiological monitoring
- Support for subjective and objective data together
- Personal baselines, correlations, and longitudinal analysis
- CPAP, respiratory, SpO₂, HRV, or other relevant sensor support
- Real-time versus retrospective data collection
- AI or advanced analysis capabilities
- Clinical reports or clinician-sharing workflows
- Privacy, local-first operation, self-hosting, exportability, and ownership of data
- Hardware breadth, BLE support, and unusual-device support
- Availability, maturity, geographic/platform coverage, and evidence quality

Do not select ten candidates solely because they have the highest download count. Include niche products when they
compete strongly with an important part of either AnxietyWatch version.

For reproducibility, use a lightweight 0–3 score for each dimension below, while explaining the judgment in words rather
than treating the total as objective truth:

- Whole-product or major-layer relevance
- Anxiety, mental-health, sleep, medication, or physiological overlap
- Objective-data and subjective-context integration
- Real-time, overnight, or always-on capability
- Privacy, local-first operation, self-hosting, or exportability
- Evidence quality, availability, and current product maturity

A subsystem specialist can be selected despite a lower whole-product score when it provides an unusually important
comparison—for example, CPAP, pulse oximetry, aircraft/noise context, or personal health-data infrastructure.

### Required candidate-selection deliverable

Create:

```text
./docs/app_comparison/CANDIDATE_SHORTLIST.md
```

The shortlist must contain:

- The ten selected candidates, ranked by relevance with a brief scoring rationale
- Product name, company/project, URL, platforms, and product type
- Which AnxietyWatch version each candidate most directly overlaps
- A short reason for inclusion
- The main comparison dimensions it represents
- Important uncertainty or evidence gaps
- Whether the candidate is primarily an app, wearable ecosystem, home-monitoring system, data platform, or
  infrastructure project
- Which AnxietyWatch layer it competes with: v1 app, v2 platform, or only a specific subsystem
- At least three credible alternatives considered but not selected, with reasons

Use sanitized, stable directory slugs for candidate folders, for example:

```text
oura
welltory
bearable
oscar
open-mhealth
```

Do not use unsanitized product names as filesystem paths.

## Phase 2 — Research each candidate's public claims

For every selected candidate, thoroughly examine publicly available material, including where relevant. Research the
product as it is publicly available at the research date; do not silently treat announced, beta, discontinued,
region-limited, or roadmap features as generally available.

- Official product pages and feature documentation
- App Store, Google Play, GitHub, documentation, help-center, and pricing pages
- Privacy policy, terms, data-export, and account-deletion information
- Hardware and integration documentation
- Clinical or research publications linked by the product
- Public release notes and changelogs
- Public screenshots or videos
- Credible independent reviews, when useful for checking practical behavior

Separate evidence into these categories:

1. **Publicly claimed:** what the company or project says it does
2. **Documented behavior:** what official documentation specifies in detail
3. **Observed or independently reported:** what credible third parties demonstrate
4. **Unknown or unverified:** what cannot be established from public evidence
5. **Not supported:** an explicit limitation or missing capability

Do not present marketing claims as verified facts. Record source URLs, page titles, publisher, publication/update date
when available, and access date. Prefer primary sources and quote or paraphrase precisely.

For each candidate, create:

```text
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md
```

Each file should cover:

- Research date and a one-paragraph executive summary
- Whether implementation inspection was performed, unavailable, or not permitted
- Product purpose and target user
- Product availability status: active, beta, discontinued, region-limited, or unclear
- User-facing workflows
- Data sources and supported hardware
- Real-time, background, and overnight behavior
- Anxiety, mood, symptom, medication, and subjective-input features
- Sleep, CPAP, respiratory, SpO₂, HRV, and environmental-data features
- Weather, aircraft, noise, or other sleep-context features
- AI, statistical analysis, correlations, personalization, and trend history
- Reports, clinician sharing, export, and integrations
- Privacy, hosting, account requirements, telemetry, and data ownership
- Pricing, subscriptions, hardware costs, geographic limitations, and platform availability
- Clinical/regulatory claims and disclaimers
- Evidence table with URLs and confidence labels
- Link to `{APP_SLUG}_IMPLEMENTATION_INSPECTION.md` when it exists
- Key similarities and differences versus AnxietyWatch v1 and AnxietyWatch2, separated into current v1, current v2, and
  specified v2 target where relevant
- Whether the candidate competes with the whole product or only a subsystem
- Important weaknesses, omissions, or unresolved questions

When a candidate offers an AI feature, explicitly investigate whether it can use the full available context—physiology,
self-reported ratings, notes, medications, songs/lyrics, sleep, environmental data, and previous analyses—or only a
narrow subset.

When a candidate offers sleep or physiological monitoring, distinguish:

- live stream versus nightly import;
- raw samples versus summaries;
- personal baselines versus population thresholds;
- measured, stale, missing, and inferred values;
- local alarm behavior versus phone/cloud notifications.

### Phase 2A — Authorized implementation inspection for mobile apps

Public-claims research is not enough for an Android or iOS candidate when a lawful, authorized application artifact is
available. Perform a separate implementation-inspection pass only when the researcher has permission to inspect that
exact APK, AAB-derived APK, IPA, source build, test build, or device installation. Public availability alone is not
authorization to bypass protections or access private data.

Record the artifact's app name, package/bundle identifier, version/build, source, acquisition date, SHA-256 hash,
platform, device/emulator used, tools, and inspection limits. Never put credentials, tokens, private user data, or
exploitable secret material in the report.

Use a layered process:

1. **Static inventory:** inspect Android manifest/resources/permissions, iOS `Info.plist`/entitlements, URL schemes,
   background modes, declared services, bundled frameworks/SDKs, feature flags, strings, assets, local database
   structures, file formats, and documented or embedded endpoints. For Android, account for base and split APKs, DEX
   files, native libraries, and resource tables; for iOS, account for the IPA payload, Mach-O binary, embedded
   frameworks, Objective-C/Swift metadata, and resources.
2. **Code and resource review:** decompile or disassemble only as permitted. Look for implemented screens, data models,
   API clients, BLE/service identifiers, HealthKit types, import/export paths, notification/alarm code,
   analytics/advertising SDKs, subscription gates, experiments, debug-only paths, remote configuration, authentication
   flows, certificate-pinning behavior, hard-coded endpoints, and error handling. A string or dead code is not proof
   that a feature is reachable, and a discovered secret must never be copied into the report.
3. **Authorized runtime observation:** use only a controlled device or emulator and synthetic/local test data. For an
   Android emulator, configure the emulator HTTP proxy to `10.0.2.2:8080` **before launching or installing the target
   app**, verify that mitmproxy is listening, and record the emulator/proxy configuration in the inspection report. Do
   not create accounts, log into production services, submit real user data, or perform destructive actions against any
   server. Use the free-software procedure in `docs/app_comparison/FREE-ANDROID-TRAFFIC-INSPECTION.md`. If an authorized
   sandbox or proxy is used to observe attempted connections, record destinations and data categories without bypassing
   authentication, DRM, licensing, encryption, certificate pinning, anti-debugging, or platform protections. Do not
   install a proxy CA on a personal or production device, and never defeat certificate pinning. On a disposable
   authorized emulator, installing the mitmproxy CA may be used to inspect an app that accepts user-installed CAs; if
   the app pins certificates or traffic still cannot be observed, record that limitation rather than bypassing it. Block
   or isolate unintended production connections where possible. Exercise relevant workflows offline or in an authorized
   test environment, record visible behavior, background behavior, permissions, local storage, logs, and network
   destinations, and mark anything that cannot be observed as blocked or unknown.
4. **Claim reconciliation:** compare public claims with static evidence and runtime evidence. Classify each feature as
   publicly claimed and observed, publicly claimed but not observed, implemented but not publicly claimed, present only
   as unreachable/experimental code, contradicted by the artifact, or unresolved.
5. **Non-feature findings:** record material observations about privacy, telemetry, third-party SDKs, permissions, cloud
   dependence, data retention, exportability, local storage, accessibility, performance, subscription gates, and
   security-relevant design. Do not turn a code smell into a confirmed vulnerability without evidence.

Use appropriate tooling for the platform—for example Android package/resource/code inspection and iOS
archive/Mach-O/resource inspection—but do not prescribe or perform protection bypasses. If authorization or a usable
artifact is unavailable, create the inspection deliverable anyway and state **not performed**, the reason, and what
remains unknown.

Create, when applicable:

```text
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_IMPLEMENTATION_INSPECTION.md
```

This file must keep public claims, static findings, runtime observations, inferences, and unknowns in separate sections.
Include a claim-reconciliation table with columns for feature, public claim, static evidence, authorized runtime
observation, reachability/maturity, and conclusion. Prompt 2 must read it when present and use it to distinguish claimed
features from implemented and reachable features.

## Phase 3 — Create a tailored Prompt 2 for each candidate

For every candidate, create:

```text
./docs/app_comparison/{APP_SLUG}/PROMPT_2.md
```

Use one of the reusable prompt variants in this directory as the starting point:

```text
./docs/app_comparison/PROMPT2.md
./docs/app_comparison/PROMPT2-EVIDENCE-FIRST.md
./docs/app_comparison/PROMPT2-TECHNICAL-SYSTEMS.md
./docs/app_comparison/PROMPT2-USER-VALUE.md
./docs/app_comparison/PROMPT2-PRIVACY-AND-OPERATING-MODE.md
```

Choose the variant according to the candidate's primary comparison value, then tailor it with the candidate name, slug,
URLs, unresolved questions, and evidence. Do not copy a generic prompt without adapting it. Prompt 2 must instruct a
future AI to perform a deeper, feature-level and general comparison. It should be usable on its own after the candidate
research file is read, while still linking back to the shared AnxietyWatch product description. It should include:

- The candidate's name and official URLs
- The relevant AnxietyWatch v1 and AnxietyWatch2 context
- Candidate-specific questions that remain unresolved
- A source list and evidence-quality requirements
- A comparison taxonomy suited to that candidate
- Instructions to distinguish claims, observed behavior, inference, and unknowns
- Questions about data ownership, privacy, export, local processing, and subscription lock-in
- Questions about hardware, live monitoring, accuracy, provenance, and missing-data behavior
- Questions about AI scope, historical context, explainability, cost, and human review
- Questions about clinical usefulness without overstating medical validity
- A required narrative comparison in addition to a feature matrix
- A balanced conclusion identifying where AnxietyWatch is stronger, weaker, complementary, or simply different
- A concrete final output structure containing, at minimum:
  1. executive summary;
  2. availability and scope notes;
  3. feature-level comparison table with separate columns for AnxietyWatch v1, AnxietyWatch2 current, AnxietyWatch2
     specified target, and the candidate;
  4. narrative comparison organized by the candidate's strongest overlap areas;
  5. privacy, hosting, cost, hardware, and integration trade-offs;
  6. evidence limitations and unresolved questions; and
  7. balanced conclusion and best-fit user profile
- An explicit distinction between capability presence, capability maturity, and evidence quality
- Instructions to identify whether a feature is native, dependent on third-party hardware, dependent on a cloud service,
  or achievable only through manual export
- A required output path and filename for the eventual candidate comparison report, such as `{APP_SLUG}_COMPARISON.md`

If the future research requires inspecting an Android or iOS package, restrict that work to applications and artifacts
for which the researcher has lawful authorization. Do not present reverse-engineering observations as public product
claims.

## Required final research package

At the end of the research session, produce or update:

```text
./docs/app_comparison/PROCESS_STATE.md
./docs/app_comparison/CANDIDATE_SHORTLIST.md
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_IMPLEMENTATION_INSPECTION.md
./docs/app_comparison/{APP_SLUG}/PROMPT_2.md
```

Create the implementation-inspection file even when inspection cannot be performed; state the reason and the resulting
evidence gap. The five reusable Prompt 2 variants are templates and do not replace the candidate-specific `PROMPT_2.md`
files.

In pilot or full execution mode, run the tailored Prompt 2 for the active candidate before marking that candidate
complete. Do not claim that the eventual `{APP_SLUG}_COMPARISON.md` files exist until Prompt 2 has actually been run.
Prompt 1 creates the research inputs and comparison prompts; the later comparison stage creates those reports.

Also update or create:

```text
./docs/app_comparison/RESEARCH_METHOD.md
```

`RESEARCH_METHOD.md` should record:

- Research date and model/tool used
- Candidate-selection rules
- Search boundaries and geographic/platform assumptions
- Source-quality rules
- Which claims remain unverified
- Any paywalls, login requirements, unavailable apps, or inaccessible documentation
- Ethical and legal limits applied to package or protocol inspection

## Quality bar

The output must:

- Compare products by goals and capabilities, not superficial feature counts
- Treat absence of evidence as unknown, not as proof of absence
- Avoid inventing features from screenshots or marketing language
- Distinguish AnxietyWatch v1 from AnxietyWatch2 throughout
- Include cost and privacy trade-offs, not only functionality
- Explain important differences in prose with concrete examples
- Identify partial competitors that overlap strongly in only one area
- Avoid medical, diagnostic, or emergency-monitoring claims unless the source explicitly makes and supports them
- Preserve uncertainty and conflicting evidence instead of forcing a definitive ranking
- State clearly when a comparison is between a mobile app and an underlying platform, rather than implying that the
  products have equivalent scope
