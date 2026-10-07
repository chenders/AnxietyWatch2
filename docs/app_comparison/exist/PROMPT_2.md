# Prompt 2 — Exist versus AnxietyWatch v1 and AnxietyWatch2

Base variant: **`PROMPT2-TECHNICAL-SYSTEMS.md`**
Supplementary block: **`PROMPT2-PRIVACY-AND-OPERATING-MODE.md`** (§ Operating-model audit)
Candidate: **Exist** (shortlist rank 3 of 10)
Prepared: 2026-10-06

## Assignment

Write a technical-systems comparison of **Exist** against AnxietyWatch v1 and AnxietyWatch2. The governing
question for this candidate is not *does Exist measure more than AnxietyWatch* — it plainly does not, and
it measures nothing at all. It is:

> **Exist publishes the correlation record it stores: a coefficient, a p-value, a sample size, a lag
> offset, and two separate confidence vocabularies. What exactly does its analytical model compute, what
> does it provably not compute, and what should AnxietyWatch copy or refuse from a system that had to
> solve the same missing-data and false-attribution problems twelve years earlier with 940 paying
> users?**

Exist was selected as a candidate for three reasons, and the comparison should be organised around them:

1. **It is the only candidate whose statistical contract is inspectable.** The public API reference ships
   the actual correlation object. This is static evidence about the analysis layer, not marketing.
2. **It independently solved missing-data handling the way AnxietyWatch specifies** — by excluding absent
   days rather than coercing them — and shipped that twelve years ago at consumer scale.
3. **It is the closest business-shape analogue on the shortlist**: independent, non-VC, privacy-first, no
   AI, no analytics, ~940 paying users, published revenue.

A fourth reason is negative and equally important: Exist **measures nothing**. It has no sensor, no CPAP,
no oximetry, no alarm and no local store. The comparison must not reward it for the data its users'
other devices produce.

## Candidate identity

| Field | Value |
|---|---|
| Product | Exist |
| Vendor | Hello Code Pty Ltd (Melbourne, Australia) — two founders, bootstrapped |
| Slug | `exist` |
| Android | `co.hellocode.exist` |
| iOS | `co.hellocode.Exist`, trackId 986201088 |
| Platforms | iOS, Android, **web** (`exist.io`) |
| Model | One plan, no tiers. 30-day trial, then paid |
| Pricing | **$6.99/month USD** or **$62.90/year (25% off)**; free to download, account required. Play copy says "US$6/month" |
| Required hardware | **None of its own** — but nearly useless without at least one connected service |
| Scale | **940 paid users** (vendor-published); Android 10K+ downloads / 4.5★; iOS 4.02★ from 47 ratings |
| Inspection | **Partially performed** — public API reference, attribute catalogue, help centre, public roadmap, published revenue inspected; app binary **not** inspected; no runtime observation |

## Read first

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/exist/exist_CLAIMED_FEATURES_AND_PURPOSE.md
./docs/app_comparison/exist/exist_IMPLEMENTATION_INSPECTION.md
./docs/app_comparison/CANDIDATE_SHORTLIST.md
./docs/app_comparison/RESEARCH_METHOD.md
```

**Do not re-run implementation inspection.** The app binary cannot be lawfully obtained and
`exist_IMPLEMENTATION_INSPECTION.md` records why and what was inspected instead. The published API
reference **was** inspected and its findings are static evidence — use them as such. There are **no
runtime observations** for this candidate; do not introduce any.

Go to the primary sources rather than relying on this brief. In particular, if a claim about the
statistical model matters to the verdict, cite the field and the reference page, not this file.

## AnxietyWatch status labels (use these exactly)

- **V1 documented/implemented** — exists in `../AnxietyWatch` today
- **V2 current** — exists in `./` today
- **V2 specified target** — fully specified in `openspec/` but not yet shipped
- **Unresolved** — no evidence either way

Never present a V2 specified target (supervised alarm, hardware dead-man switch, CNS pipeline stages) as
shipped. Never present a V1 capability as present in V2.

## Evidence rules

Apply these to every claim in the report.

1. **Label every finding** as: public claim · static finding (authorized published artifact) · runtime
   observation · experimental-or-unreachable · inference · unknown. For Exist there are **no runtime
   observations**.
2. **Never present a reverse-engineering or documentation-derived observation as a public product
   claim**, and never present a public claim as an implementation fact.
3. **Absence of evidence is unknown, not proof of absence.** *No CPAP attribute appears in the published
   attribute catalogue* is a finding; *Exist cannot support CPAP* is not.
4. **The API reference documents endpoints; it does not show a live system.** A documented field is a
   contract, not a verified behaviour. Do not describe any endpoint as "working".
5. **No p-value is a product claim.** The fact that Exist computes and exposes a p-value is a finding
   about engineering discipline. It is **not** evidence that Exist's correlations are correct, and it is
   not evidence that its method is superior to a different method — the statistic used, the correction
   applied (if any), and the thresholds behind `strong` and `confident` are **not published**.
6. **Do not convert a coefficient into a causal statement**, and do not treat Exist's own causation
   disclaimer as evidence that its outputs are causally safe. Quote the disclaimer and then assess the
   structural limit: the schema is pairwise.
7. **Require a mechanism before crediting a capability.** "Exist can even uncover what happens when you
   don't track a tag" is a marketing sentence with no published mechanism. Say so.
8. **Vendor-published scale figures are vendor-published.** The 940-user and revenue figures are strong
   evidence of order of magnitude precisely because they are awkward for marketing, but they are still
   self-reported.
9. **Never present the vendor's own statement about its limits as independent validation** — and here the
   converse also applies: do not treat the vendor's candour about confounding as a substitute for
   confounder control.
10. **Peer-reviewed venue ≠ product validity; correlation ≠ causation; absence ≠ failure.** Also: a
    documented *policy* is not a demonstrated *implementation*.
11. **Never state float equality.** Use tolerances.
12. **No medical, clinical, diagnostic or emergency claims without evidence.** Exist makes none, and the
    report should say so plainly rather than implying a hidden one.

## Candidate-specific unresolved questions

Answer these explicitly. If the evidence does not exist, say so and label the consequence.

- **E1** How are the absolute marketing claim ("won't ever sell your data") and the privacy policy's
  acquisition carve-out ("may be transferred as part of the sale, merger, or dissolution") reconciled —
  and what notice and opt-out would a user actually receive?
- **E2** How are "N/A" days treated *inside* the correlation computation? Are pairs dropped pairwise, or
  does intermittent coverage depress the effective `period` and therefore `stars`? Trace the statistical
  consequence of the documented exclusion rule.
- **E3** What is the mechanism behind "Exist can even uncover what happens when you don't track a tag"?
  Is tag absence a categorical level, and is it reported with the same strength/confidence treatment?
- **E4** Why does the Play data-safety declaration not list Health and fitness among collected types, given
  the app's entire function? Is Health Connect data governed by a separate declaration regime?
- **E5** What is the full `value_type` enumeration, and does Exist enforce **one canonical unit per
  attribute**? Can two services supply different units to the same attribute — and if so, what happens to
  the average and the correlation? This is the sharpest technical question against AnxietyWatch2's
  one-unit-per-type rule.
- **E6** What are the numeric thresholds behind `strong=1` and `confident=1`? The star meanings are
  described in words; the cut-offs are not published. Is `p < 0.05` implied anywhere?
- **E7** Does the PDF export carry correlations with their strength and confidence, or only charts and
  averages? This determines whether Exist's clinical-adjacent artifact is comparable to AnxietyWatch's
  report at all.
- **E8** Are insight texts ("Last night's sleep was surprisingly long") produced by a documented
  personal-baseline rule? The baseline mechanism is inferred from phrasing, not published.
- **E9** What is the actual hosting jurisdiction? Only "outside the United States" is stated.
- **E10** What does `offset` represent, and how many lags does Exist test? Marketing says "two consecutive
  days"; the API exposes an offset field.
- **E11** How does the up-front credit-card requirement interact with the 30-day trial in jurisdictions
  with cooling-off rights, and is there a documented refund policy?
- **E12** When a connected service stops supplying data for an attribute, does the attribute freeze at its
  last value, go to "N/A", or get disabled? Freezing would silently manufacture stale data — the failure
  mode AnxietyWatch2's three-state rule exists to prevent, and the most consequential test of whether
  Exist's missing-data discipline is real.

## Comparison taxonomy for this candidate

Organise the technical comparison around these axes, which is where Exist actually overlaps or instructs.

### 1. Analytical contract (the centrepiece)

- What a correlation *is*: `value`, `p`, `percentage`, `period`, `offset`, `stars`,
  `strength_description`, `stars_description`.
- **Strength and confidence as separate axes**, and the vendor's own explanation that confidence reflects
  *how much data*, not how strong the relationship is.
- The pairwise ceiling: exactly two attribute slots, no covariate, no model field. Contrast with
  AnxietyWatch's correlation engine and with what a confounder would require structurally.
- The three-week minimum and the weekly Monday refresh as a **deliberate latency budget** for statistical
  validity. Contrast with AnxietyWatch's real-time path: what does each buy and cost?
- Lagged checks (documented "two consecutive days") versus AnxietyWatch's cross-day handling.
- The insight type system (`dow_sleep`), priority 1–4, and clause composition.
- Compare against AnxietyWatch v1's correlation work and v2's specified analysis, labelling each.

### 2. Missingness, staleness and the three-state rule

- Exist: sparse date/value lists, no zero-fill, "N/A" exclusion, three-week gate. Two states in
  practice (present / absent) with a rendered label.
- AnxietyWatch2: three explicit states (measured-good, measured-bad, **not measured**), sentinel rejection
  for SFLOAT `0x07FF`/`0x0800`, no clamping, quarantine.
- The sharpest question: **Exist never ingests a sentinel-bearing sensor protocol.** Argue whether its
  omission model generalises to one, and what it would have to add. Do not assume it would work.
- Staleness: Exist's hourly-to-daily pull versus AnxietyWatch's stream. What happens to a *stale* value in
  each, and does Exist distinguish stale from absent? (Question E12.)

### 3. Provenance and attribute ownership

- The published ownership model: one designated `service` per attribute, an `available_services` fallback
  list, documented takeover on disconnect, and history retained after disconnect.
- **Attribute-level provenance only** — no per-sample provenance, no device identity, no quality flag.
  Contrast with AnxietyWatch's provenance requirements.
- Correction delegated to the source service for synced data, versus a fill-once-per-date,
  never-delete-vendor-managed model
  for manual data. Neither is append-only and neither has a tombstone or an audit trail.
- Compare with AnxietyWatch's append-only samples and tombstone removal; name the difference precisely.

### 4. Integration surface — the actionable finding

- The documented write path: "Attribute ownership", "Creating attributes", "Updating values", a
  "Sharing your integration" guide. Help article 118 names the API as the supported alternative to file
  import.
- Assess concretely: could AnxietyWatch-derived data reach a user's Exist account as a user-defined daily
  attribute, and what would the correlation layer then do with it? What would the user gain that they
  cannot get from AnxietyWatch alone?
- Then state the counterweight: an integration inverts AnxietyWatch2's local-first claim, adds a cloud
  dependency and a third party to the data path. Do not recommend; assess.

### 5. Collection, real-time and overnight (where Exist is absent)

- Exist has no sensor, no BLE, no stream, no overnight capture, no alarm, no watchdog, no dead-man
  behaviour. Say this as a **scope boundary**, not a deficiency — and then state what follows: no
  real-time capability to compare, no failure mode in common, no alarm semantics to reconcile.
- Imported summaries only: daily HRV and HR numbers, sleep stages — no RR intervals, no raw signal.
- Explicitly note there is **nothing** in the CPAP, oximetry, respiratory-rate or blood-pressure domain
  listed in the published attribute catalogue, labelled as absence of evidence.

### 6. Environmental context

- The broadest weather set on the shortlist: nine fields including **air pressure**.
- Contrast with AnxietyWatch v1's barometric input and v2's environment tracking; note that
  **aircraft overflight and environmental noise are absent from Exist, as from every candidate so far.**
- The architectural point worth keeping: because everything is an attribute, a new environmental variable
  costs Exist nothing. Is that a lesson for AnxietyWatch's registry-driven design, or a warning about
  generic attribute models?

## Failure-mode scenarios to walk through

For each, state what Exist does, what v1 does, what v2 does or specifies, and what remains unknown.

1. **A sensor stream drops during an overnight monitoring window.** Exist: nothing to drop. v1/v2: the
   three-state rule. Does Exist's model have any analogue?
2. **The phone is unavailable.** Exist: cloud-side sync continues from connected services; the user's own
   measurement contribution stops. v1/v2: local capture consequences.
3. **Internet or cloud services are unavailable.** Exist: **no new data at all** — total dependency. v1/v2:
   what continues locally, and what is lost.
4. **A reading is stale, malformed, out of range, or absent.** Exist: absent is excluded; stale is
   **unknown** (E12); malformed/out-of-range rejection is **not documented anywhere** (E5). v1/v2: reject,
   name the reason, quarantine — never clamp.
5. **A clinician needs an export with source and quality context.** Exist: PDF/CSV/JSON, but
   attribute-level provenance only, no quality flags, no unit guarantees, and no clinical framing.
   v1/v2: report contents and provenance.
6. **An alert path has a technical failure.** Exist: **no alert path exists**, so the scenario is out of
   scope — state that rather than scoring it as a pass.

## Operating-model audit (supplementary block)

Borrowed from `PROMPT2-PRIVACY-AND-OPERATING-MODE.md`, condensed, because Exist's business posture is
itself the finding.

- **No AI, explicitly** — quoted, deliberate, and directly opposed to Welltory's GPT-based coach and
  Bearable's "export to ChatGPT" advice. Assess whether a refusal is a *feature*: what does it preserve,
  and what does a user give up?
- **No analytics, no advertising** — Google Analytics removed 2024-09-19 (recorded in the policy's own
  update history), no ad partner named, Play declares **no data shared**.
- **Named sub-processors:** Help Scout, Stripe, Mapbox, Apple Weather, Campaign Monitor, Sentry — with
  Sentry noted as potentially capturing attribute-related data in error logs. Weigh that honestly; it is a
  real exception to "no telemetry".
- **Non-US hosting, unnamed jurisdiction** (E9).
- **Retention and deletion:** backups for "a commercially reasonable period"; payment records and support
  emails retained; disconnecting a service does not delete history.
- **Cost and lock-in:** $6.99/mo or $62.90/yr, no hardware of its own, PDF+CSV+JSON export and a
  documented read/write API — the **lowest lock-in on the shortlist**, and the only candidate where the
  exit path is both documented and programmatic. Contrast with Welltory's 30-day free-tier deletion and
  Bearable's subscription-gated exports.
- **Scale and sustainability:** 940 paying users, published revenue, twelve years of operation, two
  founders. Compare this honestly to AnxietyWatch's own likely shape. What does it suggest about which
  privacy-first claims are affordable and which are aspirational?
- **"Free" discipline:** Exist is not free. Neither is AnxietyWatch2's Pi + sensors. State both.

## Required report structure

1. Executive comparison and closest technical overlap
2. System boundary and architecture diagram in ASCII or Mermaid (Exist as an aggregator; where it has no
   box at all)
3. Source and data-path comparison
4. Claim-versus-implementation reconciliation, including static findings from the published API reference
   (there are no runtime observations)
5. Feature matrix — columns: v1 · v2 current · v2 target · Exist · evidence/maturity
6. Real-time, overnight, and failure-mode comparison
7. Provenance, data-quality, missingness, and reproducibility comparison
8. Local/cloud/privacy and operational-cost comparison
9. User-facing consequences of the technical differences
10. Limitations, unverified claims, blocked inspection, and evidence gaps
11. Balanced conclusion: whole-product competition versus subsystem competition versus integration

## Tone and claim discipline

- Exist's statistics are genuinely good; say so plainly and without inflation, and do not let that become
  a general endorsement. A well-instrumented aggregator is still an aggregator.
- Do not criticise Exist for lacking CPAP, oximetry or alarms as though it promised them. It did not. The
  correct framing is scope, and the correct question is whether the scope is *honest* — which here it is.
- Where Exist is better than AnxietyWatch — statistical transparency, export breadth, missing-value
  exclusion, cost, no-telemetry posture — say so directly. The comparison is not advocacy.
- Distinguish **unknown** from **absent** from **not built** throughout, and never let a documentation gap
  read as a defect.
- Keep the three verdicts separate: **partial competitor to v1** (analysis and context layer),
  **not a competitor to v2**, **plausible integration partner and methodological precedent**.

## Sources to use

- `https://developer.exist.io/reference/correlations/` — the correlation record, filters
- `https://developer.exist.io/reference/attributes/` — attribute catalogue and value types
- `https://developer.exist.io/reference/insights/` — insight record, priority, baseline-relative text
- `https://developer.exist.io/reference/averages/` — weekly averages with day-of-week breakdown
- `https://exist.io/apps-data-syncing/` — importable attributes and connected services
- `https://exist.io/about/values/` — no AI, no analytics, no sale, export, deletion, funding
- `https://exist.io/privacy/` — sub-processors, hosting, retention, update history
- `http://hellocode.co/stats/` — published revenue and paid-user count
- `https://kb.exist.io/` — articles 10, 12, 14, 17, 18, 21, 36, 37, 55, 59, 62, 63, 99, 116, 118
- Play: `co.hellocode.exist` · iTunes Lookup: `co.hellocode.Exist`

Prefer these primaries over this brief. If a primary source is unreachable, record the failure rather than
substituting a secondary summary.

## Output

Write the report to:

```text
./docs/app_comparison/exist/exist_COMPARISON.md
```

Cite every factual claim. Label confidence and evidence class. Where the answer is unknown, write
"unknown" and name the artifact that would resolve it.
