# Prompt 2 — Privacy, ownership, and operating-model comparison

Use this version when the candidate's most important comparison is privacy, self-hosting, cloud dependence, subscription
economics, resilience, or user control. Replace `{APP_NAME}` and `{APP_SLUG}`.

Write the report to:

```text
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_COMPARISON.md
```

## Inputs

Read:

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md
```

Use the candidate's cited privacy policy, terms, technical documentation, pricing, export/deletion documentation, and
outage/support history where available. Separate policy promises from technically demonstrated behavior.

If an authorized Android or iOS artifact is available, do not skip inspection: read
`{APP_SLUG}_IMPLEMENTATION_INSPECTION.md`, or perform the inspection before finalizing if the file is absent. For an
Android emulator, configure `10.0.2.2:8888` before launching or installing the target app and record the proxy setup.
Reconcile privacy claims with observed or statically evidenced permissions, background modes, analytics/advertising
SDKs, endpoints, local storage, export paths, subscription gates, and account behavior. Record artifact/version/hash
and inspection limits. Do not send real user data, log into production services, bypass protections, or access private
data; if inspection is unavailable, state the resulting uncertainty.

Compare {APP_NAME} against:

- AnxietyWatch v1 as a self-hosted iPhone/watchOS application and personal-health system;
- AnxietyWatch2 current capabilities; and
- AnxietyWatch2's specified local-first Raspberry Pi target.

Never present the specified target as shipped implementation.

## Required operating-model audit

Investigate and cite:

- data collection, categories, sensitivity, and minimization;
- local versus phone versus cloud processing;
- account, login, vendor-cloud, internet, and subscription requirements;
- telemetry, advertising, tracking, analytics, and third-party sharing;
- encryption in transit and at rest, key custody, enrollment, and access control;
- retention, deletion, backups, restore behavior, and account closure;
- raw-data access, export formats, portability, interoperability, and lock-in;
- hardware ownership, device compatibility, replacement, and electricity/hosting costs;
- AI/model providers, prompt/data retention, egress, cost, and human review;
- behavior during phone, network, cloud, database, storage, or sensor failure;
- local alarm behavior versus remote notification behavior;
- accessibility, panic-time cognitive load, and setup/maintenance burden.

Do not treat “private,” “secure,” “offline,” “AI-powered,” or “free” as established facts without supporting detail.
“Free” must distinguish software price from hardware, hosting, electricity, subscription, and optional API costs.

## Required report structure

1. Executive verdict on ownership, control, and operating dependence
2. Data-flow diagram showing collection, processing, storage, sync, export, and notification boundaries
3. Claim-versus-implementation privacy reconciliation
4. Privacy and data-governance comparison
5. Feature matrix with columns for AnxietyWatch v1, AnxietyWatch2 current, AnxietyWatch2 specified target, {APP_NAME},
   and evidence status
6. Total-cost and lock-in analysis
7. Failure and continuity comparison: what still works when dependencies disappear
8. Practical user experience: setup, maintenance, panic-time use, overnight use, and clinician sharing
9. Security/privacy uncertainties, blocked inspection, and claims that cannot be verified
10. Balanced conclusion identifying the best-fit operating model and the products' complementary uses

Use citations and confidence labels. Explain important differences in prose instead of reducing privacy to a checklist.
Do not claim regulatory, clinical, diagnostic, or emergency-monitoring status without evidence.
