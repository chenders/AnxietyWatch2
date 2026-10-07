# Visible vs AnxietyWatch v1 and AnxietyWatch2 — comparison

**Candidate:** Visible (candidate 11) — Visible Health Inc. / Visible Health UK Ltd, `com.makevisible.visible`
**Research date:** 2026-10-06 / 2026-10-07
**Inspection mode:** *Performed — published client, vendor organisation, vendor documentation and peer-reviewed publication.* **No app binary was inspected and no runtime observation exists.** The vendor's Terms of Use prohibit reverse engineering, so the Phase 2A route is closed by contract; see `visible_IMPLEMENTATION_INSPECTION.md` §2 and §15.
**Runtime observations in this report:** zero.

Companion files: `visible_CLAIMED_FEATURES_AND_PURPOSE.md` (claim ledger V-1…V-64), `visible_IMPLEMENTATION_INSPECTION.md` (I-1…I-14, N-1…N-5, D-1…D-11, L-1…L-4), `PROMPT_2.md`.

---

## 1. Executive finding

Visible is the closest thing in this package to AnxietyWatch's *product idea* — a condition-specific app that
turns a physiological signal into a pacing decision — and it is the **furthest** thing from AnxietyWatch's
*evidence posture*. Both halves of that sentence matter, and they point in opposite directions.

**What Visible has that AnxietyWatch does not.** It ships a real-time consumer alert on continuous heart rate
(`/how-it-works`: "Real-time alerts & notifications"; "Get a phone notification when you are exerting or
pacing ahead for the day."). It publishes a peer-reviewed paper on its own data with a competing-interests
statement, an ethics determination and a limitations section. It labels its algorithm versions. It states a missing-signal
rule that is the closest published analogue in this whole package to AnxietyWatch2's third state. None of those
is shipped in V2.

**What the artifacts do not carry.** Four findings, each verified against the vendor's own bytes:

1. **The published evidence validates the free tier, not the paid one.** The paper's cohort is built from
   "a 60-s morning PPG assessment with evening symptom severity reports" and reports that "Within-person
   increases in HR and decreases in HRV in the morning were associated with worsening symptom reports in the
   evening." The product page sells **continuous** monitoring, PacePoints and real-time alerts. The vendor's own
   paper announcement says what comes next: "Next, we'll build on these findings by bringing in continuous
   monitoring, sleep and activity data, and testing whether acting on these early signals improves outcomes
   over time." So the flagship evidence is the free tier's once-daily reading, and the vendor says so itself.
   This is an **evidence-scope finding**, not a claim of impropriety.
2. **The continuous modality that the paid tier sells is itself labelled a beta.** The overnight/continuous
   Stability route lives in the Early Access Hub, whose own article states "Features in the Early Access Hub may
   include bugs or other issues" and "Features in the Early Access Hub are still being built, so some things may
   not work as expected." A vendor declaring its own beta is honest; the finding is that the *paid marketing
   surface* does not carry that qualifier.
3. **The vendor's most absolute privacy claim appears on four pages in two different wordings, and four
   counter-surfaces contradict it.** The FAQ and `/research` say "All your Visible data is stored in an
   encrypted database and isn't shared with any third parties unless you choose to opt in to a research study";
   the two help articles say "All Visible data is stored in an encrypted database and is not shared with any
   third parties unless you explicitly opt in to a research study." Against it: the vendor's own privacy policy
   ("we may share personally identifiable information with Meta for advertising targeting purposes"), the
   vendor's own Terms ("By using Visible, you give us permission to use your anonymous information to improve
   the Service for all users."), and the vendor's own Google Play Data Safety declaration, whose entire
   **Data shared** section is one entry — **App activity → App interactions**, purpose **Analytics**. The term
   "your Visible data" is undefined, so this is recorded as an **unresolved conflict with an undefined term**,
   not as a false statement.
4. **A score-changing algorithm update shipped with no version identifier.** The vendor publishes labelled
   versions — v1.1 (2023-04-11), v1.2 (2023-05-30) — but the 2026-09-23 post "An even more accurate Stability
   Score" tells users that already-recorded days move ("a day that previously gave you a 4, might now give you a
   3") and names no version. The validated model is from 2026-03-24. Which algorithm produced a historical
   score is therefore `[UNKNOWN]`. **The stronger hypothesis — that the vendor versions nothing — is refuted**
   and is not recorded.

**Where AnxietyWatch2 is genuinely behind, stated plainly.** V2's alarm path, CNS engine and dead-man switch are
**specified, not shipped**. V2 has no published evidence base. V2 has no clinician surface. V2 has **no crisis
routing** — a tree-wide search for `crisis` across `*.rs`/`*.md`/`*.toml`/`*.swift`, excluding
`docs/app_comparison/`, returns **zero** matches (the only tracked match anywhere is one prose sentence in
`docs/reviews/2026-08-03-moonshot-r3.txt:430`, and no crisis-routing code exists). Visible routes at the point
of use, in the user's own language.

**Where AnxietyWatch2 is genuinely ahead.** V2 implements the third state *at the presentation layer*, not only
at the score: `AGENTS.md:118-119` states the rule and its two consequences. Visible implements it at the score —
"There is no option to take a morning reading instead" — and **abandons it at the clinician dashboard**, which
"only displays sections when there is information to share" (help article, 2026-09-30). V2's store is
append-only with tombstones; V2's export posture is signal-first where Visible's is derived-first.

Different clinical domains, different stages, different cost models. The transferable parts are the *disciplines*
— evidence labelling, algorithm versioning, the missing-signal rule — not the intervention.

---

## 2. Product and availability

| | Visible | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| Operator | Visible Health Inc. (Wilmington DE, 251 Little Falls Drive) + Visible Health UK Ltd (7 Savoy Court, London WC2R 0EX, "a wholly owned subsidiary company") | Individual / open project | Individual / open project |
| Platform | iOS `com.makevisible.visible` track id 1624474919; Android `com.makevisible.visible` | iOS (`AnxietyWatch`, `com.` — v1 tree) | Linux (Raspberry Pi) + iOS client stage |
| Version at research date | iOS **v26.9.3**; `currentVersionReleaseDate` 2026-09-28; released 2022-08-29; 160,522,240 bytes; min iOS 15.0 | — | `clients/apple/` stage-7 change, `Proposed` |
| Store posture | Free download; **Membership only available "in US and UK"** | Not distributed | Not distributed |
| Age rating | App Store advisory **12+**; Play "Everyone"; **Terms' own floor 16** | n/a | n/a |
| Ratings (as published) | iOS US 4.77395★ / 3,601; iOS GB 4.77 / 2,533; Play (as served) — see §13 | n/a | n/a |
| Downloads band | Play **100K+**; store copy claims "over 250,000+ people" (a member count, not an install count) | n/a | n/a |
| Commercial model | Membership + hardware. US: Annual Membership $179.88, Monthly $19.99, Armband 2.0 $79.70 (combined annual-tier total $259.58; monthly-tier band $79.70, total $99.69). UK: Annual membership £131.88 ("(25% off)"), Monthly £14.99, Band 2.0 £49.70 ("(41% off)") / £84.70 in the monthly tier, combined total £181.58. Vendor statement: "We don't make money by selling devices for a profit." | Free, source-available | Free, open source |
| Hardware | **Visible Band 2.0** — "Visible Band 2.0 is manufactured by Polar, a world leader in heart-rate monitoring since 1977." Previous generation was a retail **Polar Verity Sense**, still supported and upgradable | Polar H10 (v1 supports Polar HRM) | Polar H10 (`[source.polar_h10]`), plus AirSense 11, A&D cuff, EMAY, ACCONEER XE125 |
| Pricing presentation | Two headline percentages on different bases, neither stated: US "Save 25%" is membership-only ($239.88 → $179.88); UK "Save 31%" is combined membership-plus-band (£264.58 → £181.58). The UK membership component's "(25% off)" label implies a smaller discount than its own two figures (≈26.7%). Totals arithmetically consistent — **no arithmetic defect recorded** | n/a | n/a |
| Document dates | Terms "Last updated: 14th July 2022"; Privacy Policy "Last updated: 27 Feb 2025", footer "© 2025 Visible Health Inc"; help articles dated 2025-10-01, 2025-11-04, 2026-07-24, 2026-09-30 | README/REQUIREMENTS with dates in-tree | `AMENDMENTS.md`, dated change directories |

**A 2022-dated Terms document governs a 2026 product tier it does not describe** (band, clinician dashboard,
overnight mode are absent from it). That is recorded as a **lifecycle fact** with the exact date, not a
compliance finding.

**The hardware lineage is directly relevant.** Visible's band is a Polar-manufactured OEM device, and the vendor's
GitHub organisation forks `polarofficial/polar-ble-sdk` (created 2025-01-30, default branch `master`, license
`NOASSERTION`, **23 commits behind, 0 ahead**). The organisation has **two public repositories, both forks, with
zero vendor-authored commits**; `async-zip` is identical to its upstream. That is an accurate description of a
*consumer* of a vendor SDK. AnxietyWatch2 consumes the same family of hardware through
`[source.polar_h10]` in `vitals/registry.toml:272` and ships `crates/vitals-collectors/src/bin/h10_collector.rs`.
Visible's paid tier is gated on *its* Polar device; V2's ingestion is device-agnostic and registry-driven.

---

## 3. System boundary and architecture

Both diagrams below are drawn **from documentation only**. No box is drawn from a runtime observation, and no
vendor server topology is invented.

### Visible — documented components only

```
   ┌──────────── Visible Band 2.0 (manufactured by Polar) ───────────┐
   │  all-day heart rate · screenless · waterproof · 5-day battery   │
   │  firmware, transport and on-device processing: UNPUBLISHED      │
   └──────────────────────────────┬─────────────────────────────────┘
                                  │ transport/protocol not documented
                                  ▼
   ┌──────────── Visible app, iOS + Android (closed source) ─────────────────┐
   │  smartphone-camera PPG check-in (60 s)   ·  symptom check-in (0–3)      │
   │  Stability score 1–5 (morning OR overnight modality — mutually          │
   │      exclusive, switching resets the baseline)                          │
   │  PacePoints budget · PaceSetter day end (6 pm–4 am) · 2 alert types     │
   │  CSV data export · Health Report PDF (needs 30 d) · notes · meds        │
   └───────┬───────────────────────────────────────────────┬─────────────────┘
           │ vendor cloud — region not documented;        │ live, revocable
           │ policy says "both within the EEA and the     │ view-only link
           │ USA"; help centre says "we generally keep    ▼
           │ it within the ... EEA"                Clinician Dashboard
           ▼                                       (desktop/tablet; sections
   ┌──────────── vendor backend (undocumented) ──── hidden when empty)
   │  "an encrypted database" · anonymous research dataset (opt-in)
   └──────┬────────────────────────────────────────┬─────────────────────────┘
          │ website visitors & purchasers          │ app users w/ marketing consent
          │ ("Meta and Google's conversion         │ ("personally identifiable
          │ tracking tools ... up to 180 days")    │ information with Meta")
          ▼                                        ▼
      Meta / Google                           Meta (hashed on ingestion)

   Not present anywhere in the corpus: public API · OpenAPI · webhook ·
   integration · Apple Health / HealthKit / Health Connect import · offline mode ·
   CPAP · SpO₂ · sleep staging · weather or environmental ingestion.
```

### AnxietyWatch2 — from `AMENDMENTS.md`, `vitals/registry.toml`, `openspec/`

```
  ResMed AirSense 11 (BLE, read-only) ──┐
  Polar H10 (BLE) ──────────────────────┤   crates/vitals-collectors/src/bin/
  A&D UA-651BLE cuff (BLE) ─────────────┼──► bp_collector · h10_collector ·
  EMAY (BLE) ───────────────────────────┤    resmed_air11_collector ·
  ACCONEER XE125 (resp. rate) ──────────┤    resmed_air11_onboard
  HealthKit (iPhone, opaque identity) ──┘            │
                                                     │ raw-frame journal written
                                                     │ before decode
                                                     ▼
                        spool: rows appended already encoded in PostgreSQL
                        binary COPY format (`append { sync: true }` only where
                        loss is unrecoverable — BP)
                                                     │  vitals-drain → private
                                                     │  TEMP staging table
                                                     ▼
                        PostgreSQL 16 + TimescaleDB
                          samples hypertable, PRIMARY KEY (source, type,
                          session_id, seq); compressed after 7 d;
                          samples_1min continuous aggregate; raw dropped at 2 y;
                          time_bucket_gapfill ⇒ gaps are explicit NULLs
                          (locf()/interpolate() forbidden on physiological series)
                                                     │
                                                     ▼
                        vitalsd · vitals-console (admin, private)
                                                     │
                                                     ▼
                        klaxond ─► klaxon (hardware; OOMScoreAdjust=-1000)
                                └► 555 hardware dead-man switch (tier-2)
                                └► APNs to operator-selected device(s)

  Ingestion openness: vitals/registry.toml is the single source of truth
  (8 sources, 43 types); vitals/generated/ is regenerated and diffed in CI.
```

The two boundaries are of different kinds. Visible's diagram has a **vendor cloud in the middle with two
undocumented edges** (device→app, app→backend). V2's diagram has **no vendor cloud at all**: the Pi is
authoritative, the spool is local, and the only outbound surface is the APNs path and the private LAN.

---

## 4. Data path and source comparison

| | Visible | V1 | V2 |
|---|---|---|---|
| Primary physiology | Band HR (paid) or 60-s camera PPG (free) | HealthKit + Polar HRM (`PolarHRMService`, `PolarHRMParser`) | Polar H10, AirSense 11, A&D cuff, EMAY, XE125, HealthKit |
| CPAP / therapy data | **Absent from corpus** | `CPAPImporter` + `CSVImportRouter` (OSCAR summary sheet) | `resmed-air11-ble-stream` live therapy stream + `cpap-ezshare-ingest` |
| Environment / context | **Documented confounder only** ("You are sensitive to changes in the temperature…"); a *recruiting study* on atmospheric conditions | `BarometerService` + `BarometricReading` (`CMAltimeter`, "Some people are significantly pressure-sensitive") | `[type.environmental_audio]` (dBASPL, `vitals/registry.toml:96`), `headphone_audio` mapped from HealthKit. **No barometric type exists in V2** — a v1→v2 scope difference |
| Sample cadence | Morning once daily (free) / all-day continuous (paid) | Snapshot-based, HealthKit-shaped | Per-source `expected_cadence_s` in the registry (AirSense 11 = 1 s) |
| Storage | Vendor cloud, "an encrypted database" | Local SwiftData/models | Postgres 16 + TimescaleDB, append-only |
| Identity of a record | Not documented | Not documented | `PRIMARY KEY (source, type, session_id, seq)`; `seq` derived from what the source makes stable across retransmission; a source that is neither confirmed nor payload-identifiable "must not be ingested" |
| Lost-sample accounting | Not documented | Not documented | `max(seq) - count(*)` per session is the **lost-beat count** |
| Gap representation | Score omitted; dashboard section omitted | Not documented | `time_bucket_gapfill` so "gaps come back as explicit NULLs"; `locf()`/`interpolate()` forbidden |
| Schema/algorithm version in the record | **No** per-record algorithm version found in any artifact | No | Migrations are versioned (`db/`, Sqitch, deploy/revert/verify); V2 versions *schemas*, not scores |

**The single sharpest operational difference in this section.** Visible's export article states plainly: "This
continuous, all-day heart rate can't be exported or downloaded." The derived score *is* exportable. V2's posture
is the inverse: raw samples are the store's primary content and everything else is derived from them.

---

## 5. Claim-versus-documented-contract reconciliation

Condensed from `visible_IMPLEMENTATION_INSPECTION.md` §9. Verdict counts are machine-counted from that table.

| Claim | Best available evidence | Verdict |
|---|---|---|
| Band hardware is Polar-manufactured | Vendor page + supported retail Polar Verity Sense + Polar BLE SDK fork in the vendor org | **Match** |
| Missing overnight reading ⇒ no score, no substitute | Help-centre article | **Match** (documented rule) |
| Raw continuous heart rate cannot be exported | Help-centre export article | **Match** (documented rule) |
| Clinician dashboard omits empty sections | Help-centre article | **Match** (documented rule) |
| "All your Visible data… isn't shared with any third parties…" | FAQ + `/research` + 2 help articles, **two wordings across four pages**, versus privacy policy + Play Data Safety + Terms | **Unresolved** |
| Minimum age 16 | Terms + help centre versus the same help article's under-16 guardian path | **Unresolved** |
| Paper eligibility ≥18 | Paper versus the paper's own reported age range | **Unresolved (internally inconsistent)** |
| Analysis code available | Paper's Code-availability sentence names no location | **Does not match** |
| Algorithm versions are published | Blog history v1.1 / v1.2 | **Match** |
| The validated algorithm is the shipped algorithm | 2026-03-24 paper versus 2026-09-23 unlabelled change | **Unresolved** (no artifact links them) |
| Continuous monitoring is validated | Paper's morning PPG-cohort input versus the paid tier's continuous features | **Does not match** (scope) |
| Beta-flagged overnight mode is presented as beta where it is sold | Early Access Hub article versus `/how-it-works` | **Does not match** (qualifier placement) |
| "the data export does not include your live continuous heart rate data or Heart Rate Zones" | Export article | **Match** |
| "It's not possible to accurately switch between the two [modalities]" | Overnight article | **Match** |
| Retention period | Privacy policy: "for as long as necessary…" — no period stated | **Undefined** |
| Research data deletability | Privacy policy: "cannot be deleted as it cannot be linked back to you" | **Documented limitation** |
| Play Data Safety: "Data is encrypted in transit" | Play listing | **Public claim (self-declared)** |
| No third-party health-data ingestion | Help centre: "We don't currently support HRV data from Apple Health or Garmin…" | **Documented absence** |

The pattern is consistent: **every match is a documented rule, and every conflict sits where a marketing or
absolute surface meets a contract, a store declaration or a second vendor page.**

---

## 6. Feature matrix

Columns: v1 · v2 current · v2 target · Visible · evidence/maturity.

| Capability | v1 | v2 current | v2 target | Visible | Evidence / maturity |
|---|---|---|---|---|---|
| Continuous HR ingest from chest strap | shipped (Polar HRM) | shipped (`h10_collector.rs`, `[source.polar_h10]`) | — | paid tier only, proprietary band | V2: shipped · Visible: `[DOCUMENTED]` |
| Phone-camera PPG HRV | not present | not present | not specified | **shipped** (60-s morning check-in) | `[DOCUMENTED]` |
| CPAP therapy streaming | import only (`CPAPImporter`, OSCAR summary sheet) | code present (`resmed_air11_collector.rs`); change is `Proposed` and "does not authorize pairing, deployment, activation, or an overnight run" | live read-only BLE stream | **not present** | V2: code present, not authorized |
| CPAP report ingest | shipped (`CPAPImporter`) | `cpap-ezshare-ingest` (Proposed) | — | not present | v1: shipped |
| Blood pressure | not present | shipped (`bp_collector.rs`, `[source.ad_ua651ble]`) | — | not present | shipped |
| Respiratory rate | not present | `xe125-respiratory-rate` (Accepted for implementation) | — | paper model input only, no consumer display | V2: accepted; Visible: `[PUBLISHED RESEARCH]` |
| SpO₂ | via HealthKit | `[type.spo2]`, AirSense 11 provides it | — | **not present** (no clause in corpus) | — |
| Sleep staging | `SleepEfficiencyCalculator` | `[type.sleep_stage]` | — | "Sleep – self-reported scores are shown for all members, duration and sleep times are shown for members enrolled in sleep tracking via the Early Access Hub" | Visible: partial, beta-gated |
| Environmental noise | not present | `[type.environmental_audio]` + `headphone_audio` (HealthKit-mapped) | — | not present | V2: shipped mapping |
| Barometric pressure | shipped (`BarometerService`, `BarometricReading`) | **not present** (no registry type) | not specified | not present (documented as a confounder only) | v1-only capability |
| Weather / atmospheric data ingested | not present | not present | **specified as a product interest** (v2 context work; no registry type at this date) | **not ingested** — a recruiting study instead | Visible: `[PUBLIC CLAIM]` |
| Append-only store with tombstones | no | shipped (`vitals-console`: `test_domain_versions_and_tombstones_are_append_only`) | — | not documented | V2: shipped + test |
| Explicit-NULL gap semantics | no | shipped (`AMENDMENTS.md` storage: "gaps come back as explicit NULLs") | — | score omitted; dashboard section omitted | V2: shipped rule |
| Real-time consumer alert | shipped (CNS alert stack) | **specified** ("Alarm path — the CNS early-warning subsystem (built last)") | klaxon + 555 dead-man switch + APNs | **shipped** (2 alert types on the band stream) | V2: spec only · Visible: `[DOCUMENTED]` |
| Hardware dead-man switch | no | **specified** (555, tier-2, heartbeat conditional on the whole chain) | yes | not described | V2: spec only |
| Third state ("not measured") at the score | no | shipped rule (`AGENTS.md:118-119`) | — | **shipped** at the score ("There is no option to take a morning reading instead") | both: documented rules |
| Third state at a presentation surface | no | shipped rule applies to all surfaces | — | **not implemented** on the clinician dashboard | Visible: `[DOCUMENTED]` omission |
| Data export | `DataExporter` | report generator (`SPEC-admin-console.md:28`) | clinician-handable report | CSV + Health Report PDF; **raw HR excluded** | Visible: `[DOCUMENTED]` |
| Clinician surface | no | **not present** at this date | "something you can hand a clinician" (spec) | **shipped** — live revocable link | Visible: `[DOCUMENTED]` |
| Public API / webhook / integration | no | no | not specified | **not present** | all three: none |
| Apple Health / Health Connect import | shipped (HealthKit) | shipped (HealthKit mapper) | — | **not present** ("We don't currently support HRV data from Apple Health or Garmin…") | Visible: documented absence |
| Crisis routing | README-only (`README.md:57`) | **not present** (tree-wide search: zero code matches) | not specified | routes at the point of use | V2: not present |
| Offline operation | n/a | by design (Pi-local; spool) | — | **not present**; measurement itself needs connectivity | Visible: `[DOCUMENTED]` |
| Published peer-reviewed evidence | no | no | no | **yes** (npj Digital Medicine 9:257, 2026) | Visible: `[PUBLISHED RESEARCH]` |
| Labelled algorithm version | n/a | schemas are versioned, scores are not applicable | — | v1.1, v1.2; **2026 change unlabelled** | Visible: partial |
| Registry / single source of truth for units and ranges | no | shipped (`vitals/registry.toml`, CI-regenerated `vitals/generated/`) | — | not documented | V2: shipped |

**Nothing in this table is a runtime observation.** V2 statuses were read from the tree; Visible statuses were
read from the vendor's documents and stores.

---

## 7. Derived values versus stored signals

This is the axis on which the two products are most cleanly inverted, and the inversion has a concrete user
consequence.

**Visible.** The store holds the raw signal — the app displays an all-day heart rate and Heart Rate Zones — but
the **export surface does not**. Verbatim: "This continuous, all-day heart rate can't be exported or
downloaded." And: "If you have a Visible membership, please note that the data export does not include your live
continuous heart rate data or Heart Rate Zones." The alternative offered is a vendor-hosted, live, revocable,
view-only link: "However, if you'd like to share it with a clinician, share a live link to your Clinician
Dashboard instead - this is a live, view-only link, not a downloadable file." The Health Report PDF is narrower
still: "The Health Report only includes heart rate measurements from Stability."

So on Visible: **the derived score leaves, the raw signal does not.** A user's own physiological record is
portable in aggregate and non-portable in detail, and a clinician can see the detail only inside the vendor's
live session. That is a boundary the vendor draws explicitly, and it is recorded as a boundary — not as a
defect. It does, however, make the vendor the custodian of the only complete copy.

**AnxietyWatch2.** The store *is* the raw signal. `samples` is a hypertable of per-source measurements keyed
`(source, type, session_id, seq)`; derived values are computed downstream and are not the durable artifact. The
`openspec` spool writes raw frames to a journal **before** decode, so a decoder bug is replayable. The export
question is therefore not *will you give me back my own data* but *how much of the raw stream do you want*.

**V1** occupies a middle position: it stores what HealthKit exposes plus locally-captured signals
(`BarometricReading` is "stored locally since Core Motion barometric data is not persisted by the system"), and
`DataExporter` is the outbound surface.

**A second inversion, on scoring transparency.** Visible's scores are opaque by design — the Stability score
combines resting HR, HRV and self-reported sleep and symptoms into 1–5, and no weight or formula is published.
V2's units, ranges and precisions come from `vitals/registry.toml` and are regenerated into code, so the
*inputs* are declared even where the *analysis* is not. That is a real difference and it favours V2 — but it is a
difference about the inputs, not about the model, and V2's CNS model is not shipped at all.

---

## 8. Real-time, overnight and failure behaviour

**Visible's overnight path.** Documented as a 4-hour window beginning 30 minutes after sleep onset, ignoring
periods of movement, averaging readings with good signal. Two uncited generalisations sit inside the method
description: "Studies suggest the early hours of sleep are more sensitive to reflect recovery than the whole
sleep period." and "Increasingly, researchers are questioning the idea of a 'good' heart rate variability (HRV)."
`[DOCUMENTED]`, no citation found in the corpus.

**Modality exclusivity.** Morning camera and overnight band readings are not interchangeable: "It's not possible
to accurately switch between the two." and switching creates a new baseline. Combined with the missing-signal
rule, this is a coherent design — but it means the free tier's measurement and the paid tier's measurement
produce **different series**, which is directly relevant to §1 finding 1: the paper's cohort used the free tier's
route.

**Failure cases, as documented.**

| Scenario | Visible (documented) | v1 | v2 |
|---|---|---|---|
| A measurement cannot be taken | "If you don't [have a strong and stable internet connection], the heart rate reading may not start or complete." A measurement gated on the network | local | local; spool+sync, never a measurement gate |
| A score cannot be computed | no score that day; no substitute reading offered | not documented | third state holds |
| A spike of movement pollutes the overnight window | moving periods ignored | n/a | n/a |
| A user takes a heart-rate-altering medication | "you won't generate as many PacePoints or receive exertion notifications, even if it feels like you are exerting yourself"; "you might generate PacePoints even when at complete rest"; mitigation is a **manual** zone retune | n/a | n/a |
| A clinician views a month with no data | the dashboard "will not display sections when there is no data to include from your band" | n/a | explicit NULLs from `time_bucket_gapfill` |
| A decoder mis-parses | not applicable (no such surface) | n/a | raw-frame journal makes it replayable |
| The alarm path is silent | not described | `CNSAlarmChannelPolicy`, `CNSAlertTierMachine` | specified: 555 heartbeat conditional on the whole chain being alive |

**The medication row is the sharpest failure-mode finding in this candidate.** The vendor documents a confounder
that **desensitises a real-time alert**, states the mitigation, and places that information in a help article
rather than in the alert. It is a documented degradation of a safety-adjacent path — quoted, not inferred, and
the vendor is to be credited for documenting it at all.

**The connectivity row is the second.** On Visible, a *measurement* depends on the network. On V2 the
architecture exists to prevent exactly that class of coupling. The comparison should not overstate this: the
sentence is the vendor's own description of a failure condition, not a measured failure, and no test was run.

**What V2 has not got.** A real-time consumer alert is `shipped` in V1 and `specified` in V2. Visible ships one.
Nothing in V2's tree emits a consumer-facing real-time alert today.

---

## 9. Provenance, missingness, reproducibility

**Provenance.**

| Question | Visible | v1 | v2 |
|---|---|---|---|
| Which source produced this row? | Not documented per record; the band is the only paid source | Not documented | `source` and `type` are **generated enums** (`VitalsSource`, `VitalsType`), never bare integers or strings; `node_id` retained as provenance |
| Which algorithm produced this score? | **No** per-record version; v1.1/v1.2 labels exist in blog history only, and the 2026-09-23 change is unlabelled | n/a | Scores are not the published artifact; schema versions are |
| Was this row received intact? | Not documented | Not documented | `batch_id` NULL ⇒ "did not arrive signed"; `max(seq) - count(*)` is the lost-beat count |
| Where was this computed? | Not documented | On device | On the Pi; local time never enters the store |

**Missingness.** This is where the comparison has a genuine positive for the candidate.

Visible's score layer implements the third state properly: "If you choose to get your Stability Score from
overnight readings and forget to wear your band to bed, you will not receive a Stability Score that day. There
is no option to take a morning reading instead." That is a refusal to coerce, and it parallels V2's rule at
`AGENTS.md:118-119` almost line for line.

The vendor also draws the stability-versus-wellness distinction explicitly, and states its own boundary: "A
stable score doesn't guarantee you'll feel good, and an unstable score doesn't necessarily mean something is
wrong." That is the honest-boundary class this package records as a positive.

**And then abandons the rule at the presentation layer.** The clinician dashboard article (2026-09-30) says:
"The dashboard only displays sections when there is information to share" and "To keep the dashboard clean and
easy to read, it will not display sections when there is no data to include from your band." On that surface,
*not measured* and *measured and unremarkable* are the same thing — an omission. That is precisely the failure
mode `AGENTS.md:119` forbids ("A missing signal holds state; it never reassures"). The vendor's reason is stated
and reasonable; the comparison records the choice, not a fault.

**Reproducibility.** The paper reports a walk-forward cross-validated predictive model with AUC 0.82–0.85 with
morning biometrics and 0.73–0.83 without, and an analysis over `n = 4244` with an average of 125 biometric
observations per participant. It also states: "The analysis code is available at upon reasonable request." That
sentence names no repository, archive or contact, so the reproducibility claim has **no addressable artifact**.
Data availability is conditional: "De-identified data may be made available upon reasonable request, provided
that appropriate institutional and ethical approvals are obtained." V2's equivalent is `vitals/registry.toml`
plus CI-regenerated code — a different kind of reproducibility (of *inputs*, not of *findings*), and V2 has no
findings to reproduce.

**Two verified defects inside the paper's own text**, recorded verbatim and unresolved:
- Eligibility is stated as "People aged ≥18 years … were eligible for inclusion", while the results report
  "Participants ranged in age from 13 to 83 years (M = 46.3, SD = 12.4)."
- Code availability reads "The analysis code is available at upon reasonable request." (dangling "at").

**Ethics and funding, as published.** "No personal identifiable information was collected from this dataset.
Thus, the dataset was determined to be exempt from human subject research by the Mount Sinai Program for the
Protection of Human Subjects." — an **exempt determination**, not IRB approval. Funding: "The authors did not
receive specific funding for this study. A.A. and D.P. received research support from the Steven & Alexandra
Cohen Foundation." Competing interests: "A.A. received consulting fees from Visible Health Inc. not related to
the production of this manuscript. R.P., P.C., and H.L. are current or former employees of Visible Health Inc.
and did not play a role in the study design or decision to publish the manuscript."

**This is not independent validation**, and the reason is structural rather than accusatory: three authors are
current or former employees of the vendor — including the founder named on the vendor's own About page — and the
cohort is the vendor's users, measured by the vendor's app. A ClinicalTrials.gov search for the vendor as
sponsor returned **no registered trial**; the work is a retrospective analysis of app data. The venue, the
review process and the journal's disclosure requirements are independent, and the paper's own limitation
statement is unusually candid: "Due to the retrospective study design, limited information was collected on
demographics, making the generalizability of the findings uncertain." Both facts belong in the record: a
vendor-affiliated paper **and** a candid one.

**One more provenance finding, recorded because it is a mechanism the vendor itself names.** The help article on
age requirements says "You must be at least 16 years old to create a Visible account", then states that
"Under-16s can use Visible, but a parent or guardian must accept the Agreement on their behalf and take full
responsibility for their use of the service", then "Visible hasn't been officially tested with members under 18",
and instructs: "Create an account using your own age." The Terms state a flat floor: "You must be at least 16
years old (the “Minimum Age”) to create an account." Against the paper's ≥18 eligibility and its reported 13–83
range, **four surfaces cannot all be true of the same account-creation rule**. The vendor's own instruction names
a mechanism — registering under the guardian's age — that would put a guardian's age in the account's age field.
That is recorded as a **documented provenance hazard** with the mechanism quoted, not as a conclusion about the
cohort.

---

## 10. Local, cloud, privacy and operational cost

**Cloud dependence.** Visible is cloud-based end to end: the app talks to a vendor backend for the score, the
clinician dashboard is vendor-hosted and live, and no offline mode is published. The vendor's own measurement
article ties even the *reading* to connectivity. V2 is Pi-local, with the admin console private (amendment 09)
and the public phone API HTTPS-only; the first draft is permitted HTTP only to a manually configured private-LAN
address (amendment 10).

**Disclosure surfaces, side by side.**

| Surface | Visible | V2 |
|---|---|---|
| Privacy policy | Published, "Last updated: 27 Feb 2025" | n/a (self-hosted) |
| Cookie/consent notice | **Consent Mode config served in the client**; no browser-visible cookie banner found in the served documents | n/a |
| Analytics on the marketing site | Meta + Google conversion tracking, "Event data is retained by Meta and Google for up to 180 days and processed in accordance with a Data Processing Agreement." | none |
| Analytics in the app | Consent Mode block plus Play declaration | none |
| Store data-safety declaration | Play: **Data shared** = one entry, **App activity → App interactions**, purpose **Analytics**; "Data is encrypted in transit"; "You can request that data be deleted" is available | n/a |
| Legal basis | Consent for wellness data; contract/legitimate interest elsewhere as described | n/a |
| Retention | "We will only retain your personal data for as long as necessary to fulfil the purposes we collected it for…" — **no period** | 2-year raw retention, stated in `AMENDMENTS.md` |
| Deletion | In-app deletion erases server-side access/wellness/wearable data; anonymised research data "cannot be deleted as it cannot be linked back to you" | append-only with tombstones; deletion is a tombstone row |
| Transfer | "We store and process personal data both within the EEA and the USA." versus help centre "we generally keep it within the European Economic Area (EEA)" + "There might be times when our third-party service providers need to transfer your data outside the EEA" | none |

**The served consent configuration, read as a static artifact.** The vendor ships a Google Consent Mode block
whose own comments label the branches: `// EU/EEA + UK + CH → opt-in` with a 33-entry region list and all
advertising signals `denied` by default, and `// Everyone else (US incl. CCPA states, Canada, etc.) → opt-out`
with signals `granted` by default. `ads_data_redaction` and `url_passthrough` are enabled. In the GPC branch,
`ad_storage`, `ad_user_data` and `ad_personalization` are denied while **`analytics_storage` remains granted**.
This is a `[FOUND IN PUBLISHED CLIENT]` static finding: it states what the code is written to do. Nothing here
observes a tag firing, and nothing here says a banner appeared.

**Certifications.** No SOC 2, ISO 27001, HIPAA, gzip or similar claim was found in the corpus. The security
statement available is "an encrypted database" plus Play's "Data is encrypted in transit". Compare candidate 9
(Home Assistant, self-hosted, no cloud) and candidate 5 (SleepHQ, whose own documentation describes a SOC 2
Type II programme still in progress) — the absence here is an absence *from the corpus*, not a claim about the
vendor's controls.

**Operational cost, stated by whom.** Visible's own framing: "We don't make money by selling devices for a
profit. Instead, we make hardware devices available to you as cheaply as we possibly can. Because we don't make
money from selling armbands, it's your membership fee that powers everything we do." Hardware margin is
`[UNKNOWN]` — it is a vendor statement about its own model and nothing in the corpus quantifies it. Refunds are
restricted: "However, for hygiene reasons, we're unable to provide refunds for the device cost unless your
armband is faulty." Membership is only available in the US and UK. V2's cost model is compute, storage and
hardware on the owner's Pi.

---

## 11. Alerting and the alarm path

Both directions belong here.

**What Visible has that V2 has not shipped.** A real-time consumer alert on a continuous physiological stream,
with two documented types (Exertion and PaceSetter), an adjustable PaceSetter day end (6 pm–4 am), and the
product-page promise "Get a phone notification when you are exerting or pacing ahead for the day." The alert is
on the paid tier. It is a shipped product feature in the sense that the vendor documents it as one.

**What V2 specifies and Visible does not describe.** V2's alarm path is a specified subsystem with a hardware
tier: the alarm output must be "provably OFF from power-on until `klaxond` takes control", a failed storage fault
must never fire the klaxon ("An alarm that cries wolf is a broken alarm"), self-tests must be able to fail, and a
**555 hardware dead-man's switch** provides the only tier-2 independence, with its heartbeat "conditional on the
whole chain being alive — a monitoring window active, a fresh primary sample, a recent tier evaluation, and drain
progress". Independence is reported in tiers. `docs/design/SPEC-admin-console.md:599` places the alarm path last
in the build order. **All of this is specified, not shipped.**

**The alert-quality asymmetry.** Visible's own help centre documents a case where a medication class silently
desensitises the alert: "you won't generate as many PacePoints or receive exertion notifications, even if it
feels like you are exerting yourself", with the inverse error possible at rest, and a manual user mitigation.
V2's specified design contains the same *class* of hazard analysis in a different form — the dead-man heartbeat
is explicitly conditioned on the whole chain being alive "because a healthy `klaxond` pulsing while every
collector is dead is the worst false negative available". The two vendors are solving the same problem from
opposite ends: Visible documents the degradation and asks the user to retune; V2 specifies an alarm that must be
able to prove its own chain is alive.

**Crisis routing.** Visible routes at the point of use. V2 does not route at all: a tree-wide search for `crisis`
across `*.rs`, `*.md`, `*.toml` and `*.swift`, excluding `docs/app_comparison/`, returns **zero** matches. The
only tracked match anywhere is a single prose sentence in `docs/reviews/2026-08-03-moonshot-r3.txt:430` ("The
system may alarm after the crisis is over"), which is a review note about alarm timing and not a routing path.
**V1 has a README-only crisis notice (`README.md:57`).** For a product whose domain is anxiety, this is the
clearest single gap in the comparison, and it is recorded as **not present**, with the search that establishes
it. No remediation is proposed here.

**Where V2's alerting is stronger on paper.** V2's alert path is required to survive OOM (`OOMScoreAdjust=-1000`
plus `MemoryMin`/`MemoryHigh` on the alarm processes, with Postgres and Docker adjusted *positively*), to be
testable, and to hold state rather than reassure on a missing signal. Visible's documented failure modes are a
connectivity-gated measurement and a medication-confounded alert. That is a design contrast, not a defect
finding — Visible has not published a hardware alarm specification because it is not building a hardware alarm.

---

## 12. User-facing consequences

Concrete differences a user would meet:

1. **Where the data lives.** A Visible user's continuous heart rate exists in the vendor's database and can be
   seen by the user in the app and by a clinician through a live vendor link, but cannot be taken away. A V2
   user's samples live on their own Pi in a store designed to be append-only and exportable at the signal level.
2. **What happens when the network is down.** Visible documents that "the heart rate reading may not start or
   complete." V2 measures into a local spool and syncs later.
3. **What happens when a day is missed.** Visible refuses to invent a score — a genuine strength — but a
   clinician looking at that month sees nothing at all rather than a marked gap.
4. **What a score means.** Visible publishes 1–5 with the boundary "A stable score doesn't guarantee you'll feel
   good". It does not publish the formula. V2 publishes units, ranges and precisions from a single registry but
   its analysis layer is not shipped, so there is no score to interpret yet.
5. **Whether the alert is trustworthy under medication.** Visible's own help centre says it may not be, and the
   remedy is manual.
6. **What "free" means.** Visible's app is free to download and free to use at the daily-reading level; the
   alerting, continuous monitoring and clinician surfaces are the paid tier, and the paid tier is US/UK only.
   V2 is free and open source but is not a consumer product at this date.
7. **What happens at the point of crisis.** Visible routes. V1 has a README notice. V2 has nothing.
8. **What happens to the age field.** On Visible, a user under 16 is instructed to have a guardian accept the
   Agreement, and the onboarding instruction is to "Create an account using your own age" — so the stored age may
   be the guardian's. That is a data-provenance consequence for the user and the researcher, not only for the
   vendor.

---

## 13. Limitations, unverified claims and evidence gaps

**Structural.**

- **No runtime observation exists for this candidate** — zero, as for every candidate in this package. The app
  was not installed, no traffic was observed, no endpoint was called, and no account was created. The vendor's
  Terms prohibit reverse engineering, which closes the binary route by contract.
- **The inspection surface is documents.** Marketing pages, legal documents, store records, 26 of 95 help
  articles, the vendor's GitHub organisation metadata, and one peer-reviewed paper with its PMC full text. Every
  "the app does X" statement in the vendor's documents is a `[DOCUMENTED]` claim, never a verified behaviour.
- **Retrieval failures are recorded as `[UNKNOWN]`, not absence.** `/study` and `/product-demo` returned ~13 KB
  JavaScript shells; `/pricing` and `/science` returned **genuine 404s** and are therefore verified absent, a
  different class. The distinction is maintained throughout.
- `/research` and `/join-research` serve **byte-identical** documents (SHA-256
  `f0e17c9c974a27f8663bc4c70b6de3f2d76909261b592e5b0ea0b0c6428d18e6`). They are one surface counted twice, and
  are not treated as corroboration.
- **Four pages carrying the vendor's most absolute privacy claim are not four sources**, and because the wording
  differs between the FAQ/`/research` version and the help-centre version, they are not one quotation either.

**Unknowns that a written report cannot close.**

`[UNKNOWN]` items: whether the vendor defines "your Visible data" anywhere; the PacePoints curve and thresholds;
the Stability weights; whether the continuous path has any validation at all; whether the 2026-09-23 algorithm
differs from the validated one and by how much; the vendor cloud's region and topology; whether the clinician
dashboard shows an explicit empty state; whether the app contains an Apple Health import path the help centre
does not describe; whether a moderation or community policy exists; the two further studies' venues and designs.
Each is recorded with what would resolve it in `PROMPT_2.md`.

**Claims the vendor makes that no artifact carries.** The Atmospheric-conditions study recruitment ("Many people
notice their symptoms change with the weather…") is a *study*, not a product feature. The AI language in the 2025
plans post ("a cutting-edge AI algorithm that predicts your susceptibility to crashes") is not restated in the
2026 paper, whose reported statistic is a walk-forward cross-validated AUC range — a group-level discrimination
measure, not a per-user accuracy and not a validated real-time prediction. The rating/install figures across
stores are different quantities and are recorded without reconciliation.

**Deliberately not recorded.**

- An **arithmetic defect** in the price tables: the totals are consistent, so the hypothesis was refuted and no
  finding is recorded. What remains is a presentation observation about percentage bases.
- An **absence of algorithm versioning**: refuted by the published v1.1 and v1.2 posts.
- Any inference that the missing continuous-monitoring validation, the unlabelled change, or the privacy
  conflict reflects intent. None is supported by the artifacts and none is recorded.

**Evidence gaps carried forward in the deliverables.** `visible_CLAIMED_FEATURES_AND_PURPOSE.md` §19 and
`visible_IMPLEMENTATION_INSPECTION.md` §13 list them; the machine-lifecycle facts (the two 2026-10-06 reboots
that cleared the scratch corpus, and the re-fetch with SHA-256 re-verification) are at
`visible_IMPLEMENTATION_INSPECTION.md` §14 (L-1…L-4).

---

## 14. Balanced conclusion

**The two products are not competitors and the comparison is not a ranking.** Visible sells a supported,
condition-specific pacing service to consumers in two countries, with a hardware margin given away by design and
a membership fee carrying the business; AnxietyWatch is an owner-operated measurement and alerting system with
no customers. What is comparable is the **engineering and evidence discipline each brings to the same problem**:
turning a physiological signal into something a person can act on, without lying about what is known.

**Where Visible is plainly better, and V2 should say so.**

- It ships the alert. V2 specifies one.
- It has a peer-reviewed evidence base with an ethics determination, a competing-interest statement, funding
  disclosure, a data-availability statement and a candid limitations paragraph. V2 has none.
- It publishes labelled algorithm versions, twice, with dates and version numbers. V2 versions schemas but has
  nothing comparable to point at for its analysis layer.
- Its missing-signal rule at the score level — "There is no option to take a morning reading instead" — is the
  closest published practice in this package to V2's third state, and it is a refusal to coerce a value.
- It publishes its own boundaries: "A stable score doesn't guarantee you'll feel good, and an unstable score
  doesn't necessarily mean something is wrong." and "Importantly, this doesn't mean a single reading can tell you
  exactly when a crash is coming, or explain everything on its own."
- It routes a user in crisis at the point of use. V2 does not route at all.

**Where the artifacts fall short of the product's own surfaces, recorded without imputing motive.**

- The **validation does not reach the paid feature set**. The paper validates a once-daily morning reading; the
  membership sells continuous all-day monitoring and real-time alerts; the vendor's own paper announcement
  describes continuous monitoring as the *next* step; and the continuous modality is a feature-flagged beta whose
  own article warns that "some things may not work as expected" while the paid marketing surface carries no such
  qualifier.
- The **most absolute privacy claim on four pages, in two wordings, has four of the vendor's own counter-surfaces
  against it** — the privacy policy's Meta advertising sharing, the Terms' broad permission, Play's Data shared
  entry, and, on the same site, the Marketing section of the policy. The decisive term is undefined, so the
  finding is an unresolved conflict, and the narrow reading the vendor would need is not one the vendor states.
- **Algorithm provenance is incomplete for historical scores**: two labelled versions, then an unlabelled change
  that moves already-recorded values, six months after the evaluation that the marketing cites.
- **The paper's own reproducibility statement has no addressable artifact**: "The analysis code is available at
  upon reasonable request."
- **The third state is implemented at the score and abandoned at the clinician dashboard**, where empty sections
  are omitted "to keep the dashboard clean and easy to read" — the exact omission V2's rule forbids.

**Where AnxietyWatch2 is plainly better, and should say so carefully.**

- **Signal-first storage and export.** V2's durable artifact is the measurement; Visible's export excludes the
  raw continuous heart rate by design, and the only route to it for a clinician is a vendor-hosted live link.
- **Explicit gap semantics through the whole stack.** `time_bucket_gapfill` returning explicit NULLs, plus a
  written rule that a missing signal "holds state", reaches surfaces where Visible's does not.
- **Append-only store with tombstones**, with a test that asserts it.
- **A registry as the single source of truth** for sources, types, units, ranges and thresholds, with generated
  code regenerated and diffed in CI — so a unit or a range cannot drift silently. Visible publishes no such
  artifact.
- **Device-agnostic ingestion**: Polar H10, A&D, EMAY, ACCONEER XE125, ResMed AirSense 11 and HealthKit behind
  one registry, where Visible's paid tier is gated on one vendor-branded OEM band.
- **Provenance by construction**: generated source/type enums rather than integers or strings, `batch_id` NULL
  meaning "did not arrive signed", a lost-beat count derived from sequence identity, and a raw-frame journal that
  makes a decoder bug replayable.

**The one gap on V2's side that should not be softened.** There is no crisis routing in AnxietyWatch2, and V1's
is a README line. Visible handles this at the point of use. For a system whose stated subject is anxiety, that is
the transferable lesson from this candidate that costs the least and matters the most — and it is recorded here
as an observation about state, not as a proposal.

**What transfers, and what does not.** The clinical intervention does not: pacing for energy-limiting illness and
vitals-plus-therapy collection for anxiety are different problems with different users. What transfers is the
discipline: label the evidence class of every claim; version the algorithm, not only the schema; refuse to
manufacture a value for a day that was not measured; do not let a presentation layer delete the difference
between "absent" and "normal"; and state the boundary of your own evidence in the vendor's own words, as both
Visible's paper and its blog do. On three of those five V2 is already the stronger of the two. On the fourth it
is equal at the score and behind at the dashboard. On the fifth — the published evidence base — V2 has nothing
to point at all, and that is the honest conclusion of this comparison.

---

## Related files

- `visible_CLAIMED_FEATURES_AND_PURPOSE.md` — claim ledger V-1…V-64, §11 age contradiction, §12.3 the absolute
  claim and its counter-surfaces, §13.2–§13.4 evidence scope, §16 ledger, §17 inconsistencies, §20 artifact
  digests
- `visible_IMPLEMENTATION_INSPECTION.md` — inspection mode, the contract prohibition, static findings, the
  paper's verified defects, algorithm provenance, claim-reconciliation table, inferences, documentation defects
  D-1…D-11, machine-lifecycle facts L-1…L-4, ethical and legal limits
- `PROMPT_2.md` — the candidate-specific second-stage brief
- `../PROMPT2-EVIDENCE-FIRST.md`, `../PROMPT2-PRIVACY-AND-OPERATING-MODE.md` — base and supplementary variants
- `../mindshift/mindshift_COMPARISON.md`, `../oura/oura_COMPARISON.md`, `../empatica/empatica_COMPARISON.md` —
  the closest comparisons in evidence posture
- `../PROCESS_STATE.md`, `../RESEARCH_METHOD.md`, `../CANDIDATE_SHORTLIST.md`
- AnxietyWatch2: `docs/specs/AMENDMENTS.md`, `vitals/registry.toml`, `docs/design/SPEC-admin-console.md`,
  `AGENTS.md`, `openspec/changes/`
- AnxietyWatch v1: `README.md`, `REQUIREMENTS.md`, `docs/FEATURES.md`, `AnxietyWatch/Services/CNSRisk/`
