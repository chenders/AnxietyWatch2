# Prompt 2 — SleepHQ versus AnxietyWatch v1 and AnxietyWatch2

Base variant: **`PROMPT2-TECHNICAL-SYSTEMS.md`**
Supplementary block: **`PROMPT2-PRIVACY-AND-OPERATING-MODE.md`** (§ Required operating-model audit)
Candidate: **SleepHQ** (shortlist rank 5 of 10)
Prepared: 2026-10-06

## Assignment

Write a technical-systems and operating-model comparison of **SleepHQ** against AnxietyWatch v1 and
AnxietyWatch2. The governing question for this candidate is not whether SleepHQ would be useful — it is,
visibly — but what it reveals about two decisions AnxietyWatch has made in the opposite direction:

> **SleepHQ reads the same machine AnxietyWatch2 reads, and publishes the derived numbers AnxietyWatch2
> refuses to compute — an AHI, four event counts, pressure percentiles, desaturation durations and an
> undefined "o2_score". It reaches the device through an SD card and a AUD $399 Wi-Fi dongle where
> AnxietyWatch2 uses Bluetooth, and it locks oximetry to a AUD $299 branded ring. Which of its
> architecture choices should AnxietyWatch copy, which should it refuse, and what does it prove about the
> cost of refusing to publish an AHI?**

SleepHQ was selected as a candidate for four reasons, and the comparison should be organised around them:

1. **It is the closest commercial analogue to the subsystem AnxietyWatch actually builds.** Same device
   family, same night, same oximetry-plus-CPAP intent. This is the first candidate where the overlap is
   on the *data*, not merely on the *category*.
2. **It publishes a machine-readable interface contract** — 21 paths, 11 models, and an exact hash
   algorithm. That is the strongest implementation-grade evidence available for a closed commercial
   product in this candidate set, and it is where the analysis layer becomes legible.
3. **It has shipped the two features AnxietyWatch2 has only specified**: real-time alerting (a "sleep
   alarm", with a crash history in its own release notes) and clinician sharing. It is therefore the
   reference case for what those features cost to operate.
4. **Its founder publicly argues that device-scored AHI under-counts hypopnoeas**, while the platform
   publishes an AHI per session. That is the single most interesting tension in the candidate set, and it
   must be reported without being resolved in AnxietyWatch's favour.

A fifth reason is negative: **SleepHQ knows nothing about anxiety.** Nothing found on any SleepHQ property
connects CPAP data to mood, panic or anxiety. The comparison must not credit it with a capability it does
not claim.

## Candidate identity

| Field | Value |
|---|---|
| Product | SleepHQ |
| Vendor | **SleepHQ Pty Ltd** (Melbourne / Phillip Island, Australia) |
| Founders | Nick Dunn (sleep technologist) and Adam (web developer) |
| Slug | `sleephq` |
| Android | `com.sleephq.android` — 10K+ installs, 4.4★ / 76 reviews, updated 2026-09-13 |
| iOS | `com.sleephq.ios`, id 6451459781 — 4.73★ / 204 ratings, 17+, iOS 18.1+, v3.0.34 (2026-10-05) |
| Platforms | **Web**, iOS, Android |
| Model | Freemium with hardware. Free "Basic" forever (1 device, 1 member, **most recent 30 days viewable**); **Pro US $14.90/month or US $149.00/year** (17% off), 14-day trial |
| Hardware | **Magic Uploader Pro AUD $399** (ResMed 10/11; Wi-Fi SD card; **requires Pro**); **O2 Ring Pro AUD $299** (the only oximeter supported) |
| Claimed scale | "Over 100,000 patients… in over 160 countries" (vendor-published; Play shows 10K+ Android installs) |
| CPAP support | AirSense 11 / AirCurve 11, AirSense 10 / AirCurve 10, S9, Lumis; Respironics DreamStation 2/1; F&P SleepStyle, ICON; Löwenstein prisma; BMC G3/G2S. **AirMini explicitly unsupported** |
| Self-hosting | **None. Hosted cloud, account required** |
| Inspection | **Documented-contract inspection performed** — the full published OpenAPI specification and reference were read. **No source was read; no binary was obtained; no endpoint was called. Zero runtime observations** |

## Read first

```text
./docs/app_comparison/PROJECT-DESCRIPTIONS-AND-CATEGORIES.md
./docs/app_comparison/sleephq/sleephq_CLAIMED_FEATURES_AND_PURPOSE.md
./docs/app_comparison/sleephq/sleephq_IMPLEMENTATION_INSPECTION.md
./docs/app_comparison/CANDIDATE_SHORTLIST.md
./docs/app_comparison/RESEARCH_METHOD.md
docs/specs/AMENDMENTS.md
```

**Do not re-run implementation inspection.** The mobile binaries cannot be lawfully obtained for this
exercise and `sleephq_IMPLEMENTATION_INSPECTION.md` records why. **No endpoint may be called** — not one,
not even a public one — because doing so would create a runtime observation that the inspection file
explicitly does not contain, and would risk an account requirement the authorization for this exercise
does not cover. The published specification **was** read; use it as documented-contract evidence only.

`docs/specs/AMENDMENTS.md` is mandatory reading. Several V2 statements that this candidate makes
consequential — the alerting model, the append-only rule, the NULL-not-`locf` rule, the network-trust
model — are governed by amendments rather than by the original design, and a brief that contradicts an
accepted amendment is wrong.

Go to the primary sources rather than relying on this brief. If a field name or a path matters to the
verdict, cite it from the specification, not from this file.

## AnxietyWatch status labels (use these exactly)

- **V1 documented/implemented** — exists in `../AnxietyWatch` today
- **V2 current** — exists in `./` today
- **V2 specified target** — fully specified in `openspec/` but not yet shipped
- **Implemented and unwired** — present in the tree, compiled and tested, but read by no consumer
- **Unresolved** — no evidence either way

Never present a V2 specified target (supervised alarm, hardware dead-man switch, CNS pipeline stages,
ezShare ingestion, third-party read access) as shipped. Never present a V1 capability as present in V2.

## Evidence rules

Apply these to every claim in the report.

1. **Name the evidence class.** *Public claim* / *documented contract* / *static-only finding* /
   *runtime observation* / *inference* / *unknown*. SleepHQ has **zero runtime observations**; do not
   manufacture one, and do not describe a documented field as an observed behaviour.
2. **A documented contract is not an observed behaviour.** *`SessionSerializer` publishes `ahi`* is a
   statement about the interface. "SleepHQ computes an AHI" is a different statement, and is not
   established unless the vendor says so. Say which one you mean.
3. **An undocumented value domain is a finding.** `status`, `progress`, `usage`, `omitted` and
   `teams_type` are typed and undefined. Do not infer their meanings.
4. **Absence of evidence is not proof of absence.** If nothing found links CPAP data to anxiety, write
   *unknown*, not *no such feature*.
5. **A vendor's own research is not independent validation.** The two founder articles citing AHI
   under-counting name **no study, no author, no journal and no link**. They are unsourced vendor claims.
   Quote their most conservative reading, and never restate their figures as established fact.
6. **The most conservative vendor statement governs.** Where the FAQ says "completely secure at all
   times" and the privacy policy says "progressing towards SOC 2 Type II", the policy governs. Where the
   pricing page implies the Magic Uploader works on the free tier and the shop says Pro is required, the
   shop governs.
7. **Do not upgrade a claim to a measurement.** "1 second sample rate" is the vendor's statement about a
   device nobody in this exercise has held.
8. **Never present a V2 commitment as a cost advantage without its cost.** Self-hosting is cheaper in
   subscription terms and more expensive in time, hardware and maintenance. Say both.
9. **No float equality.** Use tolerances.
10. **Cite the vendor's own words for anything decisive**, with the date, and label it as the vendor's
    statement.

## Candidate-specific unresolved questions

Answer these if the sources allow; mark the rest `unknown` and say what would close them.

| # | Question | Why it matters |
|---|---|---|
| Q1 | Is the session `ahi` **relayed from the machine** or **re-derived** by SleepHQ? | Determines whether the founder's published criticism of device AHI applies to SleepHQ's own headline number. **Not documented** |
| Q2 | What is inside the eleven `jsonb` summary blocks? | The per-night analysis depth is otherwise unknowable |
| Q3 | What is `o2_score`, and what are `duration_90`, `drop_90`, `drop_3`, `drop_4`? | Four or five oximetry-derived quantities published with no unit, threshold or equation |
| Q4 | What do `_95` suffixed fields actually estimate? | A percentile is published with only a naming convention behind it |
| Q5 | Does the free tier **hide or delete** data past 30 days? | A privacy and portability fact, and the comparison to Welltory's deleting free tier turns on it |
| Q6 | What is "AI Insights" — model, provider, location of processing, opt-in status? | The privacy policy permits "anonymised statistical analysis and research"; an unnamed AI feature in a health platform is a material unknown |
| Q7 | Is the **sleep alarm** supervised, and does it distinguish "not measured" from "measured and good"? | This is precisely what AnxietyWatch2 specifies; the release notes prove the alert path is real and was crashing |
| Q8 | What does a share link actually contain? | "No personally identifiable information is shared" is unfalsifiable from outside |
| Q9 | Is `omitted` a three-state marker? | It is the only field that might carry "this session was excluded" |
| Q10 | What is the night boundary, and is the date device-derived or account-derived? | `time_zone` and `time_offset` both exist; the convention is undocumented |
| Q11 | Why does the SDK document MD5 for `content_hash`, and what is `fingerprint_base64`? | Two file identifiers, one with a published algorithm and one without |
| Q12 | Is the platform's own position on AHI coherent with its advertising of AHI? | Founder says unreliable; marketing says track it; the API publishes it |

## Comparison taxonomy for this candidate

### 1. Data path and architecture — the centre of the comparison

Put the two ingestion paths side by side as diagrams and as failure analyses:

- **SleepHQ:** machine → SD card → (manual copy | phone OTG reader | Wi-Fi SD card in the AUD $399
  uploader) → HTTPS upload → two-stage server processing (`POST …/imports`, then
  `POST …/imports/{id}/process_files`) → per-night `machine_date` → per-session records.
- **V2:** machine → BLE notifications → `vitals-decode` → COPY-binary spool → TimescaleDB `samples`
  hypertable → read views. No card, no vendor hardware, no cloud, no account.

Then answer, with evidence: **what does each path cost the user** — in money, in hardware, in daily
manual steps, in what breaks when a component fails, and in who can see the data? The App Store's
"every 10 minutes" versus V2's continuous BLE stream is a claim-versus-claim comparison; do not treat
either as measured.

### 2. Derived values versus stored signals — the decision the two products answer oppositely

This is where the candidate earns its place. Compare:

| | SleepHQ | V2 |
|---|---|---|
| AHI | Published per session, plus `oa_count`/`ca_count`/`h_count`/`ua_count` | **Not computed anywhere** |
| Percentiles | `pressure_set_95`, `ipap_set_95`, `epap_set_95`, `flow_limit_95` | **None computed** |
| Oximetry derivations | `o2_score`, `duration_90`, `drop_90`, `drop_3`, `drop_4` | Raw SpO₂ samples only |
| Device events | Summarised into counts | `EventKind` **decoded and classified, then read by nothing** (`crates/vitals-decode/src/events.rs`; collector consumes only the event name via `observe_event`) |
| Raw signal | Stored as SD-card files and processed server-side; **no documented waveform endpoint** | Raw flow/pressure/leak samples in the hypertable |

Argue both positions honestly. SleepHQ's position — compute the index, show it, teach the user not to
trust it — has the merit of giving a user something to act on and something to discuss with a clinician.
V2's position — store the signal, publish no index, keep the device's own event classification
unconsumed — has the merit of not manufacturing a number whose validity the vendor itself questions;
its cost is that a user gets no summary at all until the analysis layer ships. **Name that cost
explicitly.** A comparison that makes V2's refusal look free is not a comparison.

### 3. Oximetry: replaceable sensor versus branded ring

Compare V2's EMAY `SleepO2` BLE target with SleepHQ's O2 Ring Pro, and be precise about what is and is
not established:

- SleepHQ **states** the lock ("only SleepHQ branded devices are compatible") — design, not accident.
- The ring **claims** 1-second sampling, 24 h battery, USB-C and a 12-month warranty, and vibration
  alerts; none of that was measured here.
- The ring's alerting is an **on-device threshold alert**, in the same family as V2's specified
  multi-modal alerting and its **hardware dead-man switch** (V2 specified target). Compare what is
  known: SleepHQ's is shipped; V2's is specified. **Do not compare a shipped alert to a specified
  dead-man switch as though they were equivalent.**

### 4. Missing data, the three-state rule, and what each system does with a gap

- SleepHQ: **not documented.** The `jsonb` summaries are opaque; `omitted` is unenumerated;
  `summary_calculated` is a boolean whose false-state semantics are unstated.
- V2: gaps become explicit NULLs; `time_bucket_gapfill` must not be replaced by last-observation-carried-
  forward or interpolation (amendment 01 and its corrections); absent is not invalid
  (`bp_build.rs` asserts `absent is not rejected`).

State plainly that V2's rule is a stronger *commitment* and that SleepHQ's is **unknown**, not weaker.
An opaque field is not evidence of coercion.

### 5. Alerting and the alarm path — where the competitor is ahead in production

SleepHQ's release notes say "**Fixed three crashes in the sleep alarm**" and "**You're now told when a
night's recording fails to save**". Treat both as static findings with real weight:

- The alarm is **shipped and was unstable** — i.e. operating a real-time alert path cost this vendor
  three crash fixes in one release.
- The vendor added an explicit **failure notification** for a recording that did not save, which is the
  "unknown is not OK" concern being retrofitted after the fact.

Then hold V2 to the same scrutiny: the supervised alarm and the hardware dead-man switch are
**V2 specified targets**, not shipped. What does the comparison say about the *engineering cost* of the
alerting path, and about what happens when the alert path itself fails? (`PROMPT2-TECHNICAL-SYSTEMS.md`
scenario: "an alert path has a technical failure.") Answer it for both, and say which answer is evidence
and which is design intent.

### 6. Subjective data, anxiety and the axis SleepHQ does not occupy

SleepHQ's documented subjective surface is **daily**: `JournalSerializer` has `date`, `feeling_score`
(integer, no stated range), `notes`, `sleep_stages` (text), `weight_grams`, `step_count`. It is
**writable** via `POST /v1/teams/{team_id}/journals` and `PUT /v1/journals/{id}`.

There is **no documented write path for a symptom, a panic event, a time-stamped feeling, or a
medication dose.** Compare that with V1's panic-time capture and with V2's subjective capture
(`ios-client-stage7`, partially complete — **V2 in progress**, not shipped). This is the clearest axis on
which the products do not compete, and it is a documented-contract fact rather than an inference.

### 7. Sharing, clinicians and the absence of a self-hosted option

SleepHQ has live share links, clinician accounts and an enterprise tier; V2 has PDF reports as a
**specified target** and no sharing at all. Compare honestly: this is a real, shipped capability that V2
does not have. Then examine what the sharing model *requires* — an account, a hosted service, a
subscription, and a share-link payload whose contents are unverified — and what V2's ownership model
requires instead. Do not resolve the trade-off; describe it.

## Scenarios to walk through

Run each scenario for **V1, V2 (current), V2 (target) and SleepHQ**, and label the answer's evidence
class:

1. **A sensor stream drops overnight.** For SleepHQ: the card still has the data; the next sync catches
   up — *public claim*. For V2: BLE disconnects mid-notification; the session lifecycle acts only on
   `TherapyStart`/`MaskOn`/`TherapyStop`/`MaskOff`/`StandbyStarted`.
2. **The phone is unavailable overnight.** SleepHQ: the uploader is independent for CPAP, but the ring
   pairs over Bluetooth to the phone or the uploader — *public claim*. V2: the Pi collects; the phone is
   a client.
3. **The internet is unavailable.** SleepHQ: the uploader's stated behaviour is to cache and sync on
   return — *public claim*. V2: nothing depends on the internet (amendment 09 network-trust rules).
4. **A reading is stale, malformed, out of range, or absent.** Contrast SleepHQ's three opposed
   invalid-data policies (as seen in OSCAR, candidate 4) with V2's absent-versus-invalid distinction.
5. **A clinician needs an export with source and quality context.** SleepHQ: a share link, or the
   documented per-file `download_url` (signed, 5-minute expiry) — *documented contract*. V2: a PDF
   report, **specified target**.
6. **The alert path fails.** SleepHQ: evidence of three crash fixes in one release. V2: unshipped; the
   dead-man switch is the design answer and has no evidence behind it yet.

## Required report structure

Merge the base and supplementary structures. Use this order:

1. **Executive finding** — is SleepHQ a whole-product competitor or a subsystem competitor, and to which
   version?
2. **Product and availability** — identity, platforms, pricing, hardware, the free-tier boundary
3. **System boundary and architecture diagram** — ASCII or Mermaid, both systems, including the cloud
   boundary and the hardware boundary
4. **Data path and source comparison** — including SD-card/hardware versus BLE, and the two-stage
   processing contract
5. **Claim-versus-documented-contract reconciliation** — carry the heading name from the inspection file
   so the two documents agree; keep public claims, documented contract, static findings and inference in
   separate columns
6. **Feature matrix** — columns for V1, V2 current, V2 specified target, SleepHQ, and evidence/maturity
7. **Derived values versus stored signals** — the AHI/percentile/`o2_score` question, argued both ways
8. **Real-time, overnight and failure-mode comparison** — including the six scenarios above
9. **Provenance, data quality, missingness and reproducibility comparison**
10. **Local/cloud/privacy and operational-cost comparison** — collection, processing location, account
    requirement, subprocessors, encryption statements and their conflict, retention, portability,
    lock-in, total cost of ownership
11. **Alerting and the alarm path** — what is shipped, what is specified, and what the vendor's own
    release notes reveal about the cost
12. **User-facing consequences** — what a user of each product can and cannot do, in plain language
13. **Limitations, unverified claims, evidence gaps and blocked inspection**
14. **Balanced conclusion** — whole-product versus subsystem competition, and what AnxietyWatch should
    copy, refuse, or watch

## Tone and claim discipline

- Neutral and evidence-led. No advocacy for either product.
- **Do not treat SleepHQ's claimed user numbers, hardware specifications or "medical grade" ring
  description as verified.** They are vendor statements.
- **Do not treat V2's specified alerting, sharing or ingestion as shipped.**
- Quote the vendor exactly where the wording carries the meaning, especially "not a medical device",
  "progressing towards SOC 2 Type II", "completely secure at all times", and "only SleepHQ branded
  devices are compatible".
- Where SleepHQ is genuinely better — clinician sharing, device breadth, shipped alerting, an
  integration-friendly API — **say so without qualification**, then state what it costs.
- Where V2 is genuinely better — no account, no vendor hardware, append-only integrity, replaceable
  oximetry, explicit NULLs — **say so, then state what V2 has not yet built**.
- Do not end with a recommendation to abandon either product. End with what each is for.

## Sources to use

Primary, in this order:

1. `docs/app_comparison/sleephq/sleephq_IMPLEMENTATION_INSPECTION.md` — the specification, paths,
   models, the MD5 contract, the auth model, the rate-limit disclosure
2. `docs/app_comparison/sleephq/sleephq_CLAIMED_FEATURES_AND_PURPOSE.md` — vendor claims with IDs S-1…S-60
3. The OpenAPI document itself (`https://sleephq.com/api/swagger.json`) if a field needs re-checking
4. `../AnxietyWatch` (v1) and this repository (`./`) for the AnxietyWatch side
5. `docs/specs/AMENDMENTS.md` for any V2 claim about alerting, storage, network trust or time

Do **not** call any SleepHQ endpoint. If a fact requires a call, record it as an evidence gap instead.

## Output

Write to:

```text
./docs/app_comparison/sleephq/sleephq_COMPARISON.md
```

Aim for the depth of `docs/app_comparison/oscar/oscar_COMPARISON.md`. Every material statement carries an
evidence class. Every V2 statement carries a status label. Every claim about SleepHQ's behaviour that the
vendor does not explicitly document is labelled `unknown`.
