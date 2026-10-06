# Prompt 2 — Bearable versus AnxietyWatch v1 and AnxietyWatch2

**Base variant:** `../PROMPT2-USER-VALUE.md` (chosen because Bearable's comparison value is the daily
user workflow and what the person gets out of it, not a technical-system audit).

This prompt is tailorable but self-contained: it should be usable after reading the candidate research
file, without reading the shortlist.

---

## Assignment

Produce a careful, evidence-based comparison of **Bearable** against **AnxietyWatch v1** and
**AnxietyWatch2**. Write the result to:

```text
./docs/app_comparison/bearable/bearable_COMPARISON.md
```

A feature grid alone is not sufficient. The report must explain the practical difference to a person
choosing what to use, in prose, with concrete examples.

## Candidate identity

| Field | Value |
|---|---|
| Name | Bearable (Symptom, Mood & Period Tracker) |
| Slug | `bearable` |
| Vendor | Bearable Ltd (UK), company no. 12089632, ICO reg. ZB189459 |
| Website | https://bearable.app/ |
| iOS | https://apps.apple.com/us/app/bearable-mood-symptoms/id1482581097 |
| Android | https://play.google.com/store/apps/details?id=com.bearable |
| Platforms | iOS, iPadOS, macOS (iOS app), Android — **no web or desktop client** |
| Model | Freemium. Free tier + Premium subscription. Play Store IAP range US$4.99–$199.99 |
| Identifiers | `com.bearable` (Android), `co.codeleap.bearable` (iOS bundle) |
| Research date | 2026-10-06 |

## Read first

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/bearable/bearable_CLAIMED_FEATURES_AND_PURPOSE.md
./docs/app_comparison/bearable/bearable_IMPLEMENTATION_INSPECTION.md
./docs/app_comparison/RESEARCH_METHOD.md
```

The implementation-inspection file records **inspection not performed**. Do not silently drop that
limitation, and do not compensate for it by treating public marketing copy as verified behaviour.

## AnxietyWatch status labels (use these exactly)

- **V1 documented/implemented** — represented as implemented by AnxietyWatch v1 code or documentation.
- **V2 current** — present and usable in the current AnxietyWatch2 repository.
- **V2 specified target** — fully specified in `openspec/`, not necessarily implemented.
- **Unresolved** — mentioned but insufficiently specified or evidenced.

Never present a V2 specified target (the CNS engine, the supervised alarm, the hardware dead-man
switch) as shipped. The AnxietyWatch2 README is explicit that **the alarm does not exist yet**.

## Evidence rules

Classify every material statement as one of:

- **Public claim** — what Bearable's vendor says.
- **Documented behaviour** — what official Bearable documentation specifies in detail.
- **Observed / independently reported** — credible third-party demonstration, e.g. a user report.
- **Static-only finding** — implementation evidence not observed at runtime.
- **Inference** — reasonable deduction, labelled as such.
- **Unknown / unverified** — cannot be established.
- **Not supported** — a stated limitation or a documented absence.

Rules:

- Never present a marketing claim as a verified fact. The App Store's "over 2M patients & doctors" is
  unverifiable and must be labelled accordingly.
- Absence of evidence is **unknown**, not proof of absence. Bearable showing no CPAP feature anywhere
  is strong absence-of-evidence; it is not proof that no CPAP code exists in the binary.
- Do not invent features from screenshots or store copy.
- Prefer primary sources. Record URL, title, publisher, date and access date for anything new.
- Never assert float equality, and never turn a correlation into a causal claim.
- Where Bearable's own vendor guidance is more conservative than its marketing, quote the vendor's
  conservative version (this matters for medication).

## Candidate-specific unresolved questions

Answer as many of these as the evidence allows, and state clearly which remain open:

- **Q1.** What is the actual current price of Bearable Premium, per region and per billing period?
  (Not published by the vendor; Play Store lists only a range.)
- **Q2.** Can arbitrary imported health-platform metrics — HRV, SpO₂, respiratory rate — enter the
  correlation engine as *outcomes*, or only Bearable's fixed outcome set (mood, symptoms, sleep,
  energy)?
- **Q3.** When an imported metric is missing or stale for a day, does Bearable treat that day as no
  data, as zero, or as a normal observation? This is the closest analogue to AnxietyWatch's
  three-state honesty rule and it deserves a direct answer.
- **Q4.** Do blank or skipped entries get excluded from correlations, or treated as low/zero values?
- **Q5.** Does any report show statistical significance, confidence, or sample size, or only effect
  direction and magnitude?
- **Q6.** What exactly does the CSV export contain — raw timestamped events or daily aggregates; what
  timezone and timestamp format; what unit conventions?
- **Q7.** What do the Android data-safety declaration's "2 others" (sharing) and "5 others"
  (collection) resolve to concretely, and does any of it cover health data?
- **Q8.** Is the account an absolute gate — can any logging happen with no account at all?
- **Q9.** What did the cited oncology systematic review actually conclude about Bearable specifically?
- **Q10.** How are custom scales stored and exported, and is there any accuracy or accessibility
  documentation for them?

## Comparison taxonomy for this candidate

Compare along these axes, and state for each whether the conclusion concerns the **v1 app**, the
**v2 Pi platform**, the **combined vision**, or **only a subsystem**:

1. **Capture burden** — how many taps and how much attention to log an anxiety episode, a medication
   dose, and an unprompted check-in.
2. **Subjective vs objective integration** — subjective ratings as ground truth; objective data as
   imported context versus captured signal.
3. **Analysis method** — factor-present/factor-absent group comparison versus personal physiological
   baselines; explainability; confounder handling; the vendor's own "kindness/sleep" false-attribution
   example is the concrete test case.
4. **Physiology** — HealthKit/Health Connect read-only import versus direct BLE capture with
   provenance.
5. **Respiratory and sleep** — CPAP, AHI, leak, oximetry, SpO₂, respiratory rate.
6. **Medication efficacy** — correlational review versus a paired dose-triggered before/after
   measurement.
7. **Temporal resolution** — daily check-ins versus second-resolution overnight streams; and what
   that difference makes possible or impossible.
8. **Clinical output** — CSV plus screenshots versus a generated PDF clinical report.
9. **Operating model** — mandatory cloud account versus local-first and self-hosted.
10. **Failure behaviour** — what each product does when the phone, internet, cloud, or account is
    unavailable, and whether anything still captures or alarms.
11. **Cost** — free tier limits, subscription, hardware, electricity, optional AI/API charges.
    Distinguish *free software* from *free operation*.
12. **Environmental context** — weather, barometric pressure, aircraft/noise. Bearable has essentially
    none of this and that should be stated plainly rather than padded.

For each capability, record both **presence** and **maturity**: current, beta, experimental, planned,
discontinued, or unknown. Also record whether the capability is **native**, dependent on
**third-party hardware**, dependent on a **cloud service**, or achievable **only through manual
export**.

## User journeys to compare

For each, describe the data path, the user's effort, how fast useful feedback arrives, what the
product explains, and what remains uncertain:

1. Logging anxiety during an acute panic episode.
2. Understanding why last night's sleep was poor.
3. Finding out whether breathing, SpO₂, HRV or noise contributed.
4. Determining whether a medication appears helpful across repeated doses.
5. Investigating aircraft or environmental noise as a sleep interrupter.
6. Combining notes, symptoms, medication, songs/lyrics, weather context and physiology.
7. Producing something useful for a psychiatric or sleep-medicine appointment.
8. Needing collection or safety behaviour when the phone, internet or cloud is unavailable.

Describe missing data and uncertainty as part of the user experience rather than hiding them.
Do not promise that correlation proves causation.

## Required report structure

1. **Plain-language executive summary** — what Bearable, AnxietyWatch v1 and AnxietyWatch2 each
   fundamentally help a person do; whether Bearable is a direct competitor, partial competitor,
   complement, or a different category; the largest unresolved uncertainty.
2. **Audience and operating model** — intended users, required devices, technical skill, setup burden,
   ongoing attention, account requirements, subscription/hardware cost, where each system works.
   Include a claim-versus-observed note, stating explicitly that implementation inspection was not
   performed and why.
3. **Claim-versus-implementation reconciliation** — reproduce the evidence-class separation; state
   that every row is unresolved by inspection and explain what that does and does not imply.
4. **Feature-level comparison table** with exactly these columns:

   | Capability | AnxietyWatch v1 | AnxietyWatch2 current | AnxietyWatch2 specified target | Bearable | Evidence/status notes |

5. **User-journey narrative** — the eight scenarios above, as prose.
6. **Privacy, operating model and cost** — where data is collected, processed, stored, synced,
   exported, retained and deleted; account requirements; telemetry; vendor lock-in; cloud
   dependencies; and what happens to the user's data if Bearable Ltd ceases operation.
7. **Balanced conclusion** — best-fit user for each product; strongest overlaps; gaps on both sides;
   competitor, complement, or subsystem-only competitor; which findings are well-supported and which
   remain uncertain.

## Tone and claim discipline

- Do not describe either project as a validated diagnostic, medical device, or emergency monitor.
- Bearable's own medication guidance ("correlative and not causal ... do not make any decisions about
  your medication using Effect On reports") is the correct register. Match or exceed it.
- AnxietyWatch's own warning is equally binding: it is experimental, not clinically validated, and
  **its alarm does not exist yet**.
- Where AnxietyWatch2 is simply absent rather than better, say so. Bearable is a shipped product with
  hundreds of thousands of users; AnxietyWatch2 has a validated toolchain and a designed detector.
  Do not imply parity that does not exist.

## Sources to use

Primary, from the candidate evidence file:

- https://bearable.app/ — product site
- https://bearable.app/privacy-policy/ — privacy policy, updated 2025-02-06
- https://bearable.app/terms-and-conditions/
- https://bearable.app/support/common-questions/bearable-free-vs-premium-features/
- https://bearable.app/support/common-questions/how-to-share-data-with-a-doctor-therapist-or-other-medical-team-member/
- https://bearable.app/support/common-questions/can-i-write-or-import-data-to-another-app-from-bearable/
- https://bearable.app/support/common-questions/how-to-track-the-weather-in-bearable/
- https://bearable.app/support/common-questions/is-there-a-browser-or-desktop-version-of-bearable/
- https://bearable.app/support/common-questions/why-do-i-need-to-create-an-account/
- https://bearable.app/support/howto/how-to-find-correlations/
- https://bearable.app/support/howto/the-factor-effect-report/
- https://bearable.app/support/howto/how-to-use-bearable-to-manage-your-medication/
- https://bearable.app/support/howto/sync-with-google-health-connect-apple-health/
- https://bearable.app/science-research/
- https://play.google.com/store/apps/details?id=com.bearable
- https://apps.apple.com/us/app/bearable-mood-symptoms/id1482581097
- Lu DJ et al., *Evaluation of Mobile Health Applications to Track Patient-Reported Outcomes for
  Oncology Patients: A Systematic Review*, Advances in Radiation Oncology 2021;6(1),
  DOI 10.1016/j.adro.2020.09.016

Add sources only when necessary, and record URL, title, publisher, date and access date.

## Output

```text
./docs/app_comparison/bearable/bearable_COMPARISON.md
```

After writing it, append a short **residual uncertainty** list to
`./docs/app_comparison/PROCESS_STATE.md` and mark the pilot candidate's comparison as produced, with
the command or check used to verify the file exists.
