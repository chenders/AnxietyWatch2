# Prompt 2 — Technical platform and monitoring comparison

Use this version for a candidate such as a CPAP product, pulse-oximeter system, wearable platform, self-hosted health
stack, home-monitoring project, or other technically deep competitor. Replace `{APP_NAME}` and `{APP_SLUG}`.

Write the report to:

```text
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_COMPARISON.md
```

## Read first

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md
```

Treat AnxietyWatch2's OpenSpec capabilities as a specified target, not automatically as shipped implementation. Label
every AnxietyWatch capability as v1 documented/implemented, v2 current, v2 specified target, or unresolved.

If {APP_NAME} has an authorized Android or iOS artifact, do not skip inspection: read
`{APP_SLUG}_IMPLEMENTATION_INSPECTION.md`, or perform the inspection before finalizing if the file is absent. For an
Android emulator, configure `10.0.2.2:8888` before launching or installing the target app, verify Charles is listening,
and record the proxy setup. Include the artifact version/hash, inspection tools and limits, and a
claim-versus-implementation reconciliation. Distinguish reachable runtime behavior from static code, strings, bundled
SDKs, endpoints, feature flags, experimental paths, dead code, and unknowns. Do not send real user data, log into
production services, or bypass DRM, authentication, licensing, encryption, certificate pinning, anti-debugging, or
platform protections. If inspection is unavailable, state exactly what could not be established.

## Technical comparison questions

Determine, with citations, how {APP_NAME} handles:

- supported sensors, devices, protocols, BLE profiles, imports, and vendor APIs;
- raw samples versus derived summaries;
- live streams, background collection, overnight operation, and nightly imports;
- event timestamps, sample identity, ordering, retransmission, deduplication, and provenance;
- measured-good, measured-bad, stale, missing, inferred, and rejected data;
- calibration, validation, ranges, units, quality flags, and device disagreement;
- local processing, phone processing, cloud processing, and external model/API dependencies;
- storage, retention, backups, export, deletion, and recovery after interruption;
- personal baselines, correlations, AI context, explainability, and human review;
- local alarms, notifications, watchdogs, dead-man behavior, and technical-fault handling;
- behavior when the phone, internet, cloud, database, or a sensor is unavailable;
- access control, enrollment, encryption, network trust, and auditability.

Distinguish an architecture claim from demonstrated behavior. Do not reward a product for a feature merely because its
hardware could theoretically provide it.

## Authorized technical inspection

Do not extract credentials, private user data, or secrets. Keep technical observations separate from
public marketing claims and cite the artifact, version, and inspection method.

## Required report structure

1. Executive comparison and closest technical overlap
2. System boundary and architecture diagram in ASCII or Mermaid
3. Source and data-path comparison
4. Claim-versus-implementation reconciliation, including static findings and runtime observations where authorized
5. Feature matrix with columns for v1, v2 current, v2 target, {APP_NAME}, and evidence/maturity
6. Real-time, overnight, and failure-mode comparison
7. Provenance, data-quality, missingness, and reproducibility comparison
8. Local/cloud/privacy and operational-cost comparison
9. User-facing consequences of the technical differences
10. Limitations, unverified claims, blocked inspection, and evidence gaps
11. Balanced conclusion: whole-product competition versus subsystem competition

Explicitly compare these scenarios:

- a sensor stream drops during an overnight monitoring window;
- the phone is unavailable;
- internet or cloud services are unavailable;
- a reading is stale, malformed, out of range, or absent;
- a clinician needs an export with source and quality context;
- an alert path has a technical failure.

Do not call a system medically validated, diagnostic, or an emergency monitor without appropriate evidence. A local
notification is not automatically an independent alarm.
