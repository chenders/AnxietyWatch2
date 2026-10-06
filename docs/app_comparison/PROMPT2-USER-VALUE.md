# Prompt 2 — User journeys and product-value comparison

Use this version when the goal is to explain the practical difference to a person deciding what to use, rather than to
perform a primarily technical audit. Replace `{APP_NAME}` and `{APP_SLUG}`.

Write the final report to:

```text
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_COMPARISON.md
```

## Inputs and boundaries

Read:

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/{APP_SLUG}/{APP_SLUG}_CLAIMED_FEATURES_AND_PURPOSE.md
```

Use the documented evidence and clearly distinguish current AnxietyWatch v1, current AnxietyWatch2, and the specified
AnxietyWatch2 target. Do not present planned Raspberry Pi capabilities as already available.

If the candidate is an Android or iOS app and an authorized artifact is available, do not skip inspection: read
`{APP_SLUG}_IMPLEMENTATION_INSPECTION.md`, or perform the inspection before finalizing if the file is absent. For an
Android emulator, configure `10.0.2.2:8888` before launching or installing the target app and record the proxy setup.
Use it to identify the difference between advertised, implemented, reachable, gated, experimental, and unverified
user-facing behavior. Record the artifact/version and inspection limits. Do not send real user data, log into
production services, bypass platform or app protections, or access private data; if inspection is unavailable, preserve
that uncertainty.

## Explain the products through real situations

For each scenario below, compare what a person must do, what data is available, how quickly useful feedback arrives,
what the product explains, and what remains uncertain:

1. A person needs to log anxiety during an acute panic episode.
2. A person wants to understand why last night's sleep was poor.
3. A person wants to see whether CPAP, breathing, SpO₂, HRV, or noise may have contributed.
4. A person is tracking whether a medication appears helpful over repeated doses.
5. A person wants to investigate aircraft or environmental noise as a possible sleep interrupter.
6. A person wants to combine notes, symptoms, medication, songs/lyrics, weather context, and physiology.
7. A person wants a useful report for a psychiatric or sleep-medicine appointment.
8. A person needs collection or local safety behavior when a phone, internet connection, or cloud service is
   unavailable.

Do not promise that correlation proves causation. Describe missing data and uncertainty as part of the user experience
rather than hiding them.

## Required report structure

### 1. Plain-language executive summary

Explain what {APP_NAME}, AnxietyWatch v1, and AnxietyWatch2 each fundamentally help a person do. State whether
{APP_NAME} is a direct competitor, a partial competitor, a complement, or a different product category.

### 2. Audience and operating model

Compare intended users, required devices, technical skill, setup burden, ongoing attention, account requirements,
subscription/hardware costs, and where the system works. Include a short claim-versus-observed-behavior note when
implementation inspection is available.

### 3. User-journey comparison

Use a table followed by detailed narrative:

| User need | AnxietyWatch v1 | AnxietyWatch2 current | AnxietyWatch2 specified target | {APP_NAME} | Practical trade-off |
|-----------|-----------------|-----------------------|--------------------------------|------------|---------------------|

Include panic-time interaction, overnight monitoring, retrospective analysis, medication follow-up, clinical sharing,
environmental context, and failure/offline behavior.

### 4. What the person learns

Compare personal baselines, trends, correlations, explanations, AI assistance, clinician reports, and the risk of
overinterpretation. Identify whether the product helps with raw data collection, actionable context, or both.

### 5. Trust, privacy, and control

Explain who controls data, where it goes, what is exportable, whether cloud or subscription access is required, what
hardware is needed, and what happens if the service ends.

### 6. Claim, implementation, and evidence caveats

List important features that are publicly claimed but not observed, implemented but not publicly advertised, present
only behind a gate or in experimental code, or still unknown. Do not turn a decompilation clue into a user promise.

### 7. Balanced conclusion

State the best-fit user for each product, the largest strengths and weaknesses, and whether using both could be
complementary. Keep medical, diagnostic, and emergency claims conservative and evidence-based.

Use source links throughout. Mark every material statement as supported, partial, unknown, or explicitly unsupported
when the evidence warrants it.
