# Prompt 2 — Evidence-first candidate audit

Use this version when the most important goal is to determine what **{APP_NAME}** can actually be shown to do, rather
than what its marketing suggests. Replace `{APP_NAME}` and `{APP_SLUG}` before use.

Write the final report to:

```text
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_COMPARISON.md
```

## Inputs

Read:

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md
```

Use the candidate file's source ledger, then verify important claims against primary sources. Record the research date
and do not treat an inaccessible or outdated feature page as proof of current availability.

If an authorized Android or iOS artifact exists, do not skip implementation inspection: read
`{APP_SLUG}_IMPLEMENTATION_INSPECTION.md` and audit it alongside public sources, or perform the inspection before
finalizing if the file is absent. Require artifact identity, version/build, platform, hash, tools, runtime environment,
authorization boundary, and limitations. Separate public claims from static code/resource findings, reachable runtime
observations, experimental or unreachable paths, inference, and unknowns.

An implementation-inspection pass may examine manifests/entitlements, permissions, background modes, screens, data
models, API clients, BLE/HealthKit identifiers, local storage, export paths, notifications, analytics/advertising SDKs,
subscriptions, feature flags, endpoints, and error handling. For an Android emulator, configure the free mitmproxy stack at `10.0.2.2:8080`
before launching or installing the target app and record the proxy setup. Follow
`docs/app_comparison/FREE-ANDROID-TRAFFIC-INSPECTION.md`; do not send real user data or log into production services.
A string, dead code path, bundled SDK, or endpoint is not proof of a user-accessible feature. If
inspection is unavailable or blocked, document that as an evidence gap.

## Core rules

For every material claim, assign one evidence state:

- **A — primary documented:** official documentation or product behavior explicitly supports it;
- **B — public claim:** the vendor/project claims it, but operational detail is limited;
- **C — credible independent observation:** a reliable third party demonstrates or reports it;
- **D — inference:** a reasonable interpretation, clearly labeled as such;
- **U — unknown:** insufficient evidence;
- **N — explicitly not supported:** the product documents a limitation.

Include a claim ledger with: claim, product, evidence state, source, date/version, and caveat. Resolve contradictions
where possible; otherwise preserve them.

Do not infer that a feature exists from a screenshot, category label, vague phrase, or adjacent capability. Distinguish
current, beta, experimental, planned, discontinued, region-limited, and plan-limited features.

## Comparison baseline

Use these status labels for AnxietyWatch:

- **V1 documented/implemented**
- **V2 current**
- **V2 specified target**
- **Unresolved**

Compare {APP_NAME} separately with v1, v2 current, and v2 target. Explain when the comparison is between a mobile app
and an underlying Raspberry Pi platform rather than equivalent products.

## Required report

Create these sections in `{APP_SLUG}_COMPARISON.md`:

1. **Executive finding:** the strongest supported conclusion and the most important unknown.
2. **Product and availability:** identity, target users, platforms, region, plans, version, and hardware.
3. **Claim-versus-implementation reconciliation:** public claim, static finding, runtime observation, and final
   disposition for each major feature.
4. **Evidence ledger:** all material claims with confidence/evidence state.
5. **Feature matrix:** columns for AnxietyWatch v1, AnxietyWatch2 current, AnxietyWatch2 specified target, {APP_NAME},
   and evidence status.
6. **Evidence-backed workflows:** panic logging, sleep review, CPAP/respiratory investigation, medication follow-up,
   environmental interruption analysis, clinical sharing, and offline/phone-independent operation.
7. **Data and analysis audit:** raw versus summary data, live versus retrospective collection, provenance, freshness,
   missingness, personal baselines, correlations, and AI context.
8. **Privacy and cost audit:** hosting, retention, deletion, export, telemetry, accounts, subscriptions, hardware, and
   external API costs.
9. **What cannot be concluded:** unsupported claims, missing documentation, blocked inspection, and unresolved
   contradictions.
10. **Balanced conclusion:** direct competition, partial competition, complementarity, and best-fit users.

Use prose to explain why important differences matter. A feature matrix without source-linked reasoning is incomplete.

Do not claim clinical validity, diagnosis, or emergency-monitoring capability unless explicitly supported by reliable
evidence. Do not treat the absence of public evidence as proof that a feature is absent.
