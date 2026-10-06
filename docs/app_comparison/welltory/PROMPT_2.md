# Prompt 2 — Welltory versus AnxietyWatch v1 and AnxietyWatch2

Base variant: **`PROMPT2-EVIDENCE-FIRST.md`**
Candidate: **Welltory** (shortlist rank 2 of 10)
Prepared: 2026-10-06

## Assignment

Write an evidence-first comparison of **Welltory** against AnxietyWatch v1 and AnxietyWatch2. The
governing question for this candidate is not "how many features does Welltory have" — it has more than
either AnxietyWatch. It is:

> **Welltory publicly claims a personal-baseline HRV design, and publicly documents a fixed medical
> threshold that overrides it. Which governs the verdict a user actually sees, and does the vendor's own
> published evidence support the accuracy claim its store listing makes?**

Welltory was selected as a candidate because it is the closest public analogue to AnxietyWatch's core
methodological choice — compute a baseline for *this* person rather than comparing to population
thresholds — and because its own support documentation describes that choice failing. Both halves need
to be assessed on the evidence, not on the marketing.

## Candidate identity

| Field | Value |
|---|---|
| Product | Welltory |
| Vendor | Welltory Inc. / "Welltory Technologies, Inc." (Redwood City, CA, USA) |
| Slug | `welltory` |
| Android | `com.welltory.client.android` |
| iOS | bundle identifier not confirmed; store listing not retrieved |
| Platforms | iOS, Android, web (`app.welltory.com`), Huawei AppGallery, Galaxy Store |
| Model | Free tier (30-day retention) + Premium subscription + Lifetime purchase |
| Pricing | Annual **$99/yr** ($8.25/mo shown); **$599** lifetime; Play IAP range $4.99–$79.00/item; no monthly plan shown |
| Required hardware | **None** — phone camera PPG is sufficient. Chest strap or smartwatch optional |
| Inspection | **Partially performed** — vendor's public research repos and datasets inspected; app binaries not |

## Read first

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/welltory/welltory_CLAIMED_FEATURES_AND_PURPOSE.md
./docs/app_comparison/welltory/welltory_IMPLEMENTATION_INSPECTION.md
./docs/app_comparison/CANDIDATE_SHORTLIST.md
./docs/app_comparison/RESEARCH_METHOD.md
```

Verify important claims against primary sources where reachable. Where a source was not retrieved
(notably the iOS App Store listing and the science page's athlete-validation study), say so rather than
filling the gap from the vendor's summary of it.

Do **not** re-run implementation inspection. The app binary was not obtained and the inspection file
records why. The published research artifacts **were** inspected; use those findings as static evidence
and keep them clearly separated from documented claims and from any runtime observation — of which there
are none for this candidate.

## AnxietyWatch status labels (use these exactly)

- **V1 documented/implemented** — exists in `../AnxietyWatch` today
- **V2 current** — exists in `./` today
- **V2 specified target** — fully specified in `openspec/` but not yet shipped
- **Unresolved** — cannot be determined from available evidence

## Evidence rules

Assign every material claim one state:

- **A — primary documented** — official documentation or product behaviour explicitly supports it
- **B — public claim** — vendor claims it, operational detail limited
- **C — credible independent observation** — a reliable third party demonstrates or reports it
- **D — inference** — reasonable interpretation, clearly labelled
- **U — unknown** — insufficient evidence
- **N — explicitly not supported** — the product documents a limitation

Additional rules specific to this candidate:

1. **Do not attribute Welltory's own studies to independent validation.** Welltory employees authored the
   Sensors 2021 PPG method paper. Peer review occurred; independence did not. Third-party papers in
   `hrv-covid19` used the *dataset*, not the product.
2. **Do not upgrade a preprint to peer-reviewed.** The vendor's HRV/COVID paper is a medRxiv preprint with
   14 subjects.
3. **Keep the vendor's own limitation statements in the record.** The chronic-conditions article and the
   HRV-can't-help-if-regulation-is-impaired statement are the strongest evidence in the whole file.
   Quote them; do not paraphrase them into something weaker or stronger.
4. **Preserve the store-copy versus support-doc contradiction** on which wearables supply HRV. Do not
   silently pick the friendlier reading.
5. **Absence of evidence is not absence.** No CPAP feature was found anywhere; that is not proof the
   binary lacks one. Write "no evidence found", not "does not have".
6. **Never state float equality.** Any accuracy comparison is a tolerance, a correlation, or a stated
   agreement range — never "equal".
7. **Never convert correlation into causation**, and never repeat a vendor correlation claim as a finding.
8. **No clinical, diagnostic or emergency-monitoring claims** unless the vendor explicitly supports them
   — and here the vendor explicitly does not.
9. **The 30-day deletion and the fixed threshold are the two findings most likely to be softened
   accidentally.** State both in the vendor's own terms.

## Candidate-specific unresolved questions

Answer each explicitly, or record it as unresolved. Do not let any of them disappear into prose.

- **W1** Store copy says Fitbit, Garmin, Oura and Mi Band sync for "heart health data"; support
  documentation says those devices **cannot** supply HRV to Welltory. Does "sync" mean HRV, or only
  activity/sleep context? Which reading is operative for a buyer?
- **W2** What is the fixed "medical threshold", and under what conditions does it override the personal
  baseline? Can a user tell which rule produced a given verdict?
- **W3** Is there **any** medication, supplement or dose-tracking feature, and any before/after efficacy
  flow attached to a dose? (Experiments exist for arbitrary activities — is a medication variant of it?)
- **W4** What does the data export actually produce — format, granularity, per-metric coverage, timezone
  handling, units, and automation? Can it be scheduled before the 30-day free-tier deletion?
- **W5** Where are the 21 HRV metrics computed — on-device or server-side? What happens to a measurement
  taken offline?
- **W6** Which "clinically validated tests" exist for anxiety, burnout and depression? Are the
  instruments named and referenced, or is the phrase marketing?
- **W7** Which sleep and vital-sign metrics are genuinely ingested per platform versus merely enumerated
  as categories in the privacy policy (respiratory rate, SpO₂, ECG, EDA, perfusion index, snoring)?
- **W8** How reliable is `My patterns` in practice? The vendor requires ~7 occurrences; one six-month
  user reported zero connections. Is there a documented minimum-data rule, and what is it?
- **W9** What do the Play data-safety "3 others" (shared) and "3 others" (collected) resolve to?
- **W10** Beyond the printable blood-pressure report, is any output clinician-shareable?
- **W11** Are the science-page claims — April 2022 athlete validation, IEEE EMBC Berlin presentation,
  "25,000 studies" — backed by retrievable, applicable evidence?
- **W12** Is the stress/recovery episode detection preventive in any way, or purely descriptive after
  the fact? Does it ever act without a user opening the app?

## Comparison taxonomy for this candidate

Compare along these axes. The first three carry most of the weight.

1. **Baseline methodology** — personal baseline, fixed threshold, what overrides what, and how the user
   can tell. This is the reason Welltory is on the list.
2. **Evidence quality behind accuracy claims** — P1 (n=13, resting, camera) and P2 (n=14 preprint) versus
   the scope of the store claim.
3. **Data retention and destructive behaviour** — 30-day free-tier deletion, subscription-blocked
   deletion, versus an append-only store that never deletes samples.
4. **Measurement provenance** — PPG or watch summary versus Polar H10 raw RR at ~1 Hz with per-sample
   provenance; whether the *source* of a number survives to the report.
5. **Continuous versus on-demand collection** — no overnight capture versus H10 background recording.
6. **Respiratory and PAP domain** — no CPAP/AHI/leak support versus v1's SD import and v2's live stream.
7. **Missing-data semantics** — how each side treats a gap, a stale value, or a partially covered night.
8. **AI and analysis** — external GPT requiring two subscriptions and beta disclaimers, versus v1's
   Anthropic-based analysis and v2's specified local analysis with provenance.
9. **Correlation and context inputs** — automatic weather and ~100 integrations versus v1's automatic
   barometric pressure and v2's aircraft/environmental-noise direction.
10. **Local safety and alarm** — nothing versus v1's local alerts and v2's specified supervised alarm and
    dead-man switch.
11. **Privacy, hosting and telemetry** — US cloud, named advertising pixels and analytics SDKs, versus
    local-first and self-hosted with no telemetry claimed.
12. **Cost and hardware dependency** — $99/yr or $599 lifetime with no required sensor, versus free
    software with H10/Pi hardware cost.

## User journeys to compare

Walk each journey for v1, v2 current, v2 target and Welltory, marking evidence state at each step.

1. **Panic logging at the moment it happens** — including whether either product can log without the
   phone, and what happens if the user does nothing.
2. **Sleep review the morning after** — noting Welltory's documented hours-to-a-day sync latency and
   that free-tier data older than 30 days is gone.
3. **CPAP / respiratory investigation** — v1's SD import and v2's live stream against a product with no
   PAP capability found.
4. **Medication follow-up** — v1's dose-triggered paired rating; Welltory's Experiments design, which is
   paired but not dose-attached.
5. **Environmental interruption analysis** — automatic weather on both sides; aircraft and noise on the
   AnxietyWatch side only.
6. **Clinical sharing** — v1's PDF report versus Welltory's printable BP report.
7. **Offline / phone-independent operation** — the sharpest divergence. V2's Pi keeps collecting with no
   phone and no network. Welltory's core analysis is documented as dependent on vendor servers.
8. **Longitudinal self-study over a year** — what survives on each side, given the 30-day cliff.

## Required report structure

Write to `./docs/app_comparison/welltory/welltory_COMPARISON.md` with these sections:

1. **Executive finding** — the strongest supported conclusion and the most important unknown.
2. **Product and availability** — identity, target users, platforms, plans, hardware, version.
3. **Claim-versus-implementation reconciliation** — public claim, static finding, runtime observation,
   final disposition. Carry forward the inspection file's 14 rows; do not re-decide them silently.
4. **Evidence ledger** — claim, product, evidence state, source, date, caveat.
5. **Feature matrix** — columns for AnxietyWatch v1, V2 current, V2 specified target, Welltory, and
   evidence status.
6. **Evidence-backed workflows** — the eight journeys above.
7. **Data and analysis audit** — raw versus summary, live versus retrospective, provenance, freshness,
   missingness, baselines, correlations, AI context.
8. **Privacy and cost audit** — hosting, retention, deletion, export, telemetry, accounts, subscriptions,
   hardware, external API costs (including the ChatGPT Plus requirement).
9. **What cannot be concluded** — unsupported claims, missing documentation, the uninspected binary, and
   unresolved contradictions W1–W12.
10. **Balanced conclusion** — direct competition, partial competition, complementarity, best-fit users.

Prose must explain why differences matter. A matrix without source-linked reasoning is incomplete.

## Tone and claim discipline

- Welltory is a **mature, shipped product with real published research**. Say so. Its PPG work against
  Polar H10 is directly relevant to AnxietyWatch and is a genuine credibility signal.
- Do not present AnxietyWatch as better. Where Welltory is ahead — zero required hardware, HRV analysis
  depth, automatic environment integration, breadth of integrations, scale — say that plainly.
- Do not present the fixed-threshold finding as a gotcha. It is the vendor's own published limitation,
  and it is *useful* to AnxietyWatch as a documented failure mode to design against.
- Do not present Welltory's vendor-authored research as independent validation, and do not present its
  third-party dataset reuse as product validation.
- Do not claim Welltory reads the user's data in a specific way when only the policy describes it.
- Never claim a capability exists because a support article says a data *category* exists.
- Keep the three AnxietyWatch statuses distinct throughout. V2 specified targets are not shipped.

## Sources to use

Primary, in this order of preference:

1. `welltory_CLAIMED_FEATURES_AND_PURPOSE.md` §15 evidence table (W1–W26)
2. `welltory_IMPLEMENTATION_INSPECTION.md` (static findings, verification commands)
3. Vendor support articles at `help.welltory.com` — especially the chronic-conditions article, the
   personal-baseline article, the incompatible-devices article, plans and My patterns
4. Vendor privacy policy, dated 2026-09-01
5. `welltory.com/plans/` for current prices
6. Google Play listing for scale figures and the data-safety declaration
7. Verified citations: `10.3390/s21206798`, `10.3390/s20247068`, `10.3389/fdata.2022.801998`,
   `10.3390/s21248424`, `10.1101/2021.07.02.21259891`

Do not fabricate a source. If a page 404s — `welltory.com/pricing/` and `terms-of-use` both did — record
the failure and use what was actually retrieved.

## Output

```text
./docs/app_comparison/welltory/welltory_COMPARISON.md
```

After writing it, update `./docs/app_comparison/PROCESS_STATE.md` for candidate 2, and verify:

```sh
ls -l docs/app_comparison/welltory/
grep -c '' docs/app_comparison/welltory/welltory_COMPARISON.md
```
