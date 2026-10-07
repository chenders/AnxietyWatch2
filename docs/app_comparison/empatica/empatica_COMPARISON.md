# Empatica EpiMonitor vs AnxietyWatch v1 and AnxietyWatch2

Candidate 7 of 10. Research date **2026-10-06**.
Companion documents: `empatica_CLAIMED_FEATURES_AND_PURPOSE.md` (Phase 1 evidence ledger, E-1…E-83),
`empatica_IMPLEMENTATION_INSPECTION.md` (Phase 2A, 27-claim reconciliation), `PROMPT_2.md` (second-stage
brief). Pipeline state: `../PROCESS_STATE.md`. Method: `../RESEARCH_METHOD.md`.

**Evidence classes:** `[PUBLIC CLAIM]` · `[REGULATOR RECORD]` · `[THIRD PARTY]` · `[INFERENCE]` ·
`[UNKNOWN]`. **There are no `[runtime observation]` rows for this candidate.** No Empatica app was installed,
no account created, no device worn, no endpoint called, no traffic intercepted. Every statement below is the
vendor's published words, an independent regulator's record, or an explicitly-labelled inference.

---

## 1. Executive finding

**Empatica EpiMonitor is the only candidate in this package that ships an alarm whose failure mode is a
person not receiving help, and it is therefore the only one that has had to answer — in public, to a
regulator, and in a contract — the questions AnxietyWatch2's alarm layer will face.** It answers them well in
some places and not at all in others, and the pattern of where it does which is the finding.

Four points carry the comparison:

1. **The regulated artifact and the marketing page report different things, and the page's number is
   untraceable.** The product page claims "98% detection accuracy against the gold standard of hospital-based
   EEG measurement". The FDA summary that cleared the device (`K250515`) reports **positive percentage
   agreement and false-alarm rate per 24 hours**, states plainly that "**No clinical studies have been
   performed on the subject device**", establishes the shipped configuration by **bench testing for
   equivalence** to a 2018 predicate, and derives its real-world figures from **different hardware
   (Embrace2)**. It is not that the vendor's number is false — it is that **no reader can reach it from the
   page that makes it, and the page does not mention the metric the regulator does (false alarms).**

2. **The alert path is a four-hop cloud chain with a physical precondition and a commercial gate.**
   Watch → Bluetooth (phone within **15 ft**) → phone internet → Empatica cloud → **call + SMS to caregivers**.
   Caregiver count is plan-gated at **1 / 3 / 10**, and the device requires an **active subscription** to
   function at all. There is no local, offline or self-hosted configuration in either the consumer product or
   the research platform — the vendor's own architecture diagram (`/cloud-api/`) shows the same chain ending
   at "Empatica Cloud". AnxietyWatch2's stated design is the inverse: store and alarm on owner-controlled
   hardware, no vendor in the path.

3. **The marketing page and the licence it links to are written in opposite postures, and both are current
   vendor text.** "designed to keep you and your loved ones safe" sits alongside a EULA stating the vendor
   "**cannot guarantee that a call or text message will reach any Caregiver**", that results are not
   warranted "**ACCURATE, COMPLETE, RELIABLE, CURRENT, OR ERROR-FREE**", that liability "**SHALL NOT EXCEED
   THE PURCHASE PRICE PAID**", and that there is no liability under any circumstance for "**PERSONAL INJURY OR
   DEATH**". Every clause is ordinary consumer-software practice and a product cannot warrant a phone network.
   The finding is the **coexistence without reconciliation on the same page**, and that a user reading only
   the page receives the version with no caveat.

4. **False alarms are a first-class design problem here, and AnxietyWatch2 has not stated a number.** The
   regulator's tables give empirically one false alert every ~3 days at the paediatric real-world rate
   (0.34/24 h). The vendor ships three mitigations: a **user cancellation window before caregivers are
   contacted**, **adjustable sensing precision**, and an optional **low-sensitivity mode** whose stated
   purpose is reducing false alarms. AnxietyWatch2's supervised-alarm specification does not state an
   acceptable false-alarm rate for any alert type — and **that gap is visible only because this candidate
   exists in the shortlist**.

**The most useful sentence for a reader:** EpiMonitor is what a clinically serious alerting product looks
like after it has been through a regulator — including the parts the regulator would not let it claim, the
parts its own contract declines to promise, and the parts its own marketing page says anyway.

---

## 2. Product and availability

| | Empatica EpiMonitor | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| What it is | FDA-cleared seizure monitor + caregiver alerting | iOS anxiety/panic self-tracking app | Multi-source vitals collector + store + analysis, owner-hosted |
| Physiology | EDA, PPG, accel/gyro, skin temperature; GTC seizure detection | self-report, HealthKit, CPAP CSV | CPAP BLE, oximeter, chest strap, radar |
| Regulatory posture | Class II, prescription-gated in the US; CE Class IIa claimed | not a medical device | explicitly not a medical device |
| Hardware | EmbracePlus, **$399**, mandatory subscription | iPhone existing | Pi + sensors (owner's) |
| Subscription | **$17.40 / $26.90 / $52.90** per month, mandatory | none | none |
| Caregiver reach | 1 / 3 / 10 by plan | none | specified (V2 target), not plan-gated |
| Availability | US, UK, EU, AU, NZ, CA; 8 languages claimed; iOS confirmed | personal | personal |
| Legacy | Embrace2 withdrawn; E4 + software suite sunset 2025-02-14 | — | — |

**Availability facts worth carrying.** The iOS EpiMonitor app (`com.empatica.healthmonitor.alert`, v**1.78.1**)
last shipped **2026-03-24** — ~6.5 months before this research date — while the companion **Empatica Care**
app shipped **2026-09-24** `[THIRD PARTY]`. The vendor claims Android availability; **no Play listing was
located or retrieved**, four candidate identifiers returning 404 — which is **not evidence of absence**,
because Play returns 403 to some non-browser clients. Recorded `[UNKNOWN]`.

**Corporate identity is split and matters.** FDA clearance holder is **Empatica S.r.l.** (Milan); the App
Store seller is also the S.r.l.; the **privacy notice is issued by Empatica, Inc.** (Cambridge MA). For a
data-protection question the contracting and controlling entities differ from the marketing face of the site.

---

## 3. System boundary and architecture

**Empatica — the documented chain `[PUBLIC CLAIM]`:**

```
[EmbracePlus watch]         EDA · PPG(4ch) · accel · gyro · skin temp
   │                        EpiAlgo v2.1 → "unusual patterns … GTC seizures ≥ 20 s"
   │ Bluetooth 5            ◄── phone MUST be within 15 ft / ~5 m
   ▼
[dedicated smartphone + EpiMonitor app]
   │ requires Bluetooth + internet
   ▼  phone's internet connection
[Empatica Cloud]  ── phone call + SMS ──►  caregiver 1 … N     N = 1 / 3 / 10 by plan
   ▲
   └── active subscription required; no local or offline mode documented

[research platform, same shape — /cloud-api/]
[EmbracePlus] → BLE → [Care App] → internet → [Empatica Cloud] → S3, Avro/CSV → [customer's CTMS]
```

**AnxietyWatch2 — the specified shape `[V2 target]`, labels per `../PROCESS_STATE.md`:**

```
[AirSense 11 BLE] [EMAY SleepO2] [Polar H10] [Acconeer XE125]
        └────────────┴──────┬───────┴────────────┘
                            ▼
               vitals-decode (pure, no_std, no I/O)
                            ▼
              vitals-core (spool, COPY encoder)
                            ▼
          TimescaleDB `samples` hypertable (append-only, tombstone)
                        [shipped in part]
                            ▼
        cns-engine ──► klaxond / supervised alarm ──► multi-modal alert
        [specified, V2 target]      [specified]        [amendment 03, specified]
```

**Where the two diagrams differ, in one line each:**

| Axis | Empatica | AnxietyWatch2 |
|---|---|---|
| Alarm survives the vendor? | No — the vendor is a hop | Yes by design — the vendor is not in the path |
| Alarm survives the phone? | No — 15 ft, Bluetooth, internet | Watchdog/dead-man switch specified (V2 target) |
| Alarm survives a lapse of payment? | No — subscription is mandatory | Not applicable |
| Alerting layer runs where? | Vendor cloud | Owner's hardware (specified) |
| Data lands where? | Empatica Cloud, then customer S3 | Owner's TimescaleDB |
| Third party can stop collection? | Yes — a documented remote control | No such control exists or is specified |

The last row is not a criticism of Empatica — for a trial sponsor, a remote stop is a feature. It is included
because it makes concrete how different the two products' threat models are.

---

## 4. Data path and source comparison

| | Empatica EpiMonitor / EHMP | AnxietyWatch2 |
|---|---|---|
| Sensors | EDA, 4-ch multi-wavelength PPG, 3D accel + gyro, digital skin temperature, ambient light + magnetometer (EmbraceMini) | CPAP BLE (ResMed AirSense 11), oximeter (EMAY SleepO2), chest strap (Polar H10: HR + RR intervals), radar (Acconeer XE125) |
| Transport | BLE to phone, then phone internet to vendor cloud | BLE to Pi-side collectors |
| Storage | Empatica Cloud; research output as **Avro/CSV in S3** | TimescaleDB `samples` hypertable, append-only, tombstone for removal |
| Canonical unit | not published on the surfaces read | `vitals/registry.toml`, one unit per type, converted **once at the decoder** |
| Thresholds | not published; detection sensitivity is a mode, not a number | registry-owned, single source of truth |
| Derived measures | "**300+**" digital measures (EmbracePlus/Cloud API), "**200+**" (EmbraceMini) — **none enumerated, no units, no windows, no ranges published** | registry types only; **no AHI type, no percentile computation** `[verified in tree]` |
| Raw data | "raw data and an extensive range of digital biomarkers"; "no black box algorithms" | raw samples retained; raw `samples` dropped at 2 years per amendment 01 |
| Missing data model | **not published** — `[UNKNOWN]`; wearing detection is published as a *platform* signal | three states required; "not measured" is never rendered as measured |

**The sharpest data-path contrast is not the sensors — it is the enumerability of the derived layer.**
Empatica sells "300+ digital measures" without a published list, unit, window or range. AnxietyWatch2's
governing rule is the opposite: units and valid ranges come from `vitals/registry.toml` and nowhere else,
and a value out of range is **rejected, named and quarantined, never clamped**. For the comparison this is a
real point in V2's favour, and it is also a fair one against Empatica: a research customer buying a
biomarker package cannot, from public information, find out what any of the 300 measures is.

---

## 5. Claim-versus-documented-contract reconciliation

Of 27 checkable claims, **12 match as stated, 7 are documented-only or unverifiable, 2 are partially
supported, and 6 do not match as stated** (`empatica_IMPLEMENTATION_INSPECTION.md` §7). The failures:

| Claim | Artifact | Result |
|---|---|---|
| "510(k) FDA-clearance … ( K221282 - K230457 )" | openFDA | **Incomplete** — both exist and describe the *platform*; neither names **EpiMonitor**. The EpiMonitor clearances (K232915, K250515) are uncited. |
| "CE-certified as a class IIa medical device" | `/legal/` artifacts | **Unverified** — the certificate the vendor links is ISO 13485 and states it is "not … proof that the statutory requirements of … Regulations (UE) 2017/745 … have been fulfilled". No MDR certificate or notified body published. |
| "98% detection accuracy … against … hospital-based EEG" | K250515 | **Untraceable** — see §1 and §5.1. |
| "No black box algorithms" vs "AI-powered" | same page | **Conflict** — recorded, unresolved. |
| Battery "up to 7 days" vs "up to 28 days" | two vendor pages | **Conflict** — most conservative governs. |
| The published User Manual is readable | measured locally | **Does not match** — no fonts, no text layer, 178 bytes extracted across 178 pages. |

**Four of the six are the same shape**: *the vendor's own published artifact does not support the vendor's own
published text.* That is now the fourth candidate in this package with vendor self-contradiction or drift —
`oscar` (header relabel, compression-level drift), `sleephq` (typo'd fields, "completely secure" against a
policy "progressing towards" certification), `nightscout` (a security audit contradicting both the code and
its own co-published requirements document) — and the first where the artifacts are **regulatory** and the
stakes are a person's safety. **Documentation drift is a category of defect, not a one-off**, and it survives
the presence of a regulator.

### 5.1 The 98% figure, stated precisely

The regulator's document for the device reports PPA and FAR, not accuracy, and includes the sentence "**No
clinical studies have been performed on the subject device**" `[REGULATOR RECORD]`:

| Data source (low-sensitivity mode) | Subgroup | N patients | GTCS | PPA | corrected PPA | 95% CI |
|---|---|---|---|---|---|---|
| EMU (adjudicated) | 6–21 | 12 | 19 | 0.895 | 0.791 | 0.619–0.925 |
| EMU (adjudicated) | >21 | 12 | 17 | **1.000** | 0.905 | 0.891–0.917 |
| Real-world (**Embrace2** hardware) | 6–21 | 601 | 1157 | 0.87 | 0.86 | 0.78–0.92 |
| Real-world (**Embrace2** hardware) | >21 | 843 | 3625 | **0.80** | 0.77 | 0.64–0.87 |

False alarms per 24 h: **0.70 / 0.28** (EMU, 6–21 / >21) and **0.34 / 0.25** (real-world). High-sensitivity
(rest/sleep) performance on the shipped hardware is supported by **bench equivalence** to the 2018 predicate
K181861, not by measurement on the subject device in this document.

Three things this does and does not establish:

- **It does not establish that 98% is wrong.** The figure may come from K181861's clinical work or from a
  publication not located in this pass.
- **It does establish that the page's number cannot be reached from the page.** No population, mode,
  confidence interval or citation accompanies it.
- **It establishes that the page omits the metric the regulator considered essential.** FAR does not appear
  on the product page at all, and the vendor's own optional low-sensitivity mode is described as existing "to
  reduce false alarm incidents".

**A note on `corrected PPA`.** The term appears in the decisive tables **without definition**, and the
corrected values are materially lower than the raw ones (1.000 → 0.905; 0.87 → 0.86; 0.80 → 0.77). An
undefined term in the table carrying the performance claim is a recordable finding — the same class as
SleepHQ's undefined `status`/`progress`/`omitted` domains, in a document of higher evidentiary weight.

---

## 6. Feature matrix

Legend: **Y** = published/shipped · **V2** = specified for AnxietyWatch2, not shipped · **—** = absent ·
**?** = `[UNKNOWN]` (checked and inconclusive, or not published).

| Capability | Empatica EpiMonitor | v1 | AnxietyWatch2 |
|---|---|---|---|
| Automated detection layer | **Y** (EpiAlgo v2.1, on-watch) | — | **V2** (`cns-engine`) |
| Independent alert path to a third party | **Y** (call + SMS, 1/3/10 caregivers) | — | **V2** (multi-modal, amendment 03) |
| Alert survives phone loss | — (needs phone ≤ 15 ft) | — | **V2** (dead-man switch) |
| Alert survives vendor outage | — (cloud is a hop) | — | **V2** (local path) |
| User cancellation window before alert | **Y** (length not published) | — | **?** — not found in the spec read; **check the tree** |
| Adjustable detection sensitivity | **Y** (high / low modes) | — | **?** — registry thresholds exist for measured types; no equivalent detection-mode concept found |
| False-alarm rate published | **Y** (in the FDA record) | — | — (no acceptable rate stated for any alert type) |
| Wearing/absent-signal detection | **Y** (platform signal, published) | — | **?** |
| Three-state data rule (good / bad / not measured) | **?** — not published | — | **Y** (governing rule, `AMENDMENTS.md`) |
| Reject-don't-clamp | **?** — not published | — | **Y** (`h10_build.rs`, `bp_build.rs`) |
| Append-only store, tombstone removal | **?** | — | **Y** (amendment 01) |
| Canonical unit, converted once at the decoder | **?** | — | **Y** (`vitals/registry.toml`) |
| Published derived-measure definitions | — ("300+" unenumerated) | — | **Y** for measured types; **no AHI, no percentiles** |
| Published output contract | — (client-only) | — | **?** |
| Self-hosting / offline operation | — | n/a (local app) | **Y** (owner-hosted by design) |
| Account required | **Y** | — | — |
| Subscription required | **Y** (mandatory) | — | — |
| Prescription required | **Y** (US) | — | — |
| Caregiver count gated by plan | **Y** | — | — |
| Existing clearance / regulatory evidence | **Y** (9 clearances) | — | — |
| Vendor-published standards list | **Y** (ISO 14971, IEC 60601-1/-1-2/-1-6/-1-11, IEC 62304, IEC 62366-1, ISO 10993) | — | — |
| Security certification published | **Y** (ISO/IEC 27001:2022; ISO 13485) | — | — |
| EULA / contractual limits published | **Y** | — | — |
| CPAP / respiratory / SpO₂ data | — (codes DQA + LEL in the platform's record, no such product sold) | **Y** (OSCAR CSV import) | **Y** |
| Environmental / contextual data | — | — | **V2** |

**Read this matrix in both directions.** Empatica's row-wise advantages are real: it has a cleared detection
layer, a shipped third-party alert path, published false-alarm data, a published standards list, security
certifications and an explicit contract. AnxietyWatch2's advantages are structural: no vendor in the alert
path, an enumerable unit/threshold source of truth, reject-don't-clamp, three-state data and no account,
subscription or prescription between the user and their own data. **Neither is a superset of the other, and
the matrix is a genuine trade rather than a scoreline.**

---

## 7. Derived values versus stored signals

**Empatica.** The public surfaces name **"300+ digital measures"** (EmbracePlus / Cloud API) and **"200+"**
(EmbraceMini) without enumerating any of them `[PUBLIC CLAIM]`. The FDA record names the alerting *decision*
precisely — GTC seizures ≥ 20 s, with PPA and FAR per subgroup and mode — because that is what was cleared.
So the candidate has **an extremely well-defined performance metric for one derived output and an entirely
undefined set for 300 others**. That split is the finding: the regulated output is measured and published,
the commercial one is neither.

**AnxietyWatch2.** Units, valid ranges and thresholds come from `vitals/registry.toml` and nowhere else; the
decoder converts once; `vitals/generated/` is regenerated in CI. The tree contains **no AHI type and no
percentile computation at all** — an important correction to any assumption that V2 derives clinical
statistics. Event classification (`EventKind` in `crates/vitals-decode/src/events.rs`) exists and is tested
but is read by **no consumer**: **implemented and unwired**.

**Why this section matters to the comparison.** In the two completed candidates where derived values were
reachable, the defect was always at the *derivation* boundary: OSCAR relabelled a p90 as `Pressure_95th`;
SleepHQ stored a field the model did not have; Nightscout's **A1c statistic depended on the display unit**,
producing 95.8% instead of 6.9% in mg/dL mode. Empatica is the case where **the derived layer cannot be
audited from outside at all** — 300 measures, no definitions. AnxietyWatch2's registry is the discipline that
makes that class of defect impossible in principle: a derived value whose inputs are not registry-typed does
not get computed. That is the transferable lesson, and it is worth stating that V2 currently derives
**nothing clinical**, so it is a rule holding over an empty set.

---

## 8. Real-time, overnight and failure mode

| Situation | Empatica (documented) | AnxietyWatch2 (specified, V2 target) |
|---|---|---|
| Continuous overnight monitoring | Y — watch records continuously, E-ink, up to 7 days (28 claimed elsewhere) | Y — collectors + store |
| Real-time alert | Y — detection on-watch, alert via cloud | Y — supervised alarm, local |
| Phone out of range | **Requirement stated (15 ft); user-facing behaviour not published** `[UNKNOWN]` | No phone in the path |
| Phone Bluetooth dropped | **Not published** `[UNKNOWN]` | n/a |
| Phone loses internet | EULA names "loss of Internet connectivity"; app behaviour `[UNKNOWN]` | Local network target |
| Subscription lapses | Store page: device requires an active subscription. **Whether alerts stop and whether the user is warned: not published** | n/a |
| Vendor cloud outage | No continuity commitment found; the E4 sunset is the worked example of a vendor disabling software ("rendering services inoperative") | Vendor not in the path |
| Data gap while unsynced | Watch has on-board flash; research path described as "non-stop data recording synced with the Care or Care Lab App" — so delay rather than loss, **for the research path** | Spool + append-only store; clock correction is versioned metadata (amendment) |
| Alert cancelled by the user | **Y** — cancellation window before caregivers are contacted; **length not published** | `[UNKNOWN]` |
| Alert fires and caregiver does not act | EULA: vendor "cannot verify or influence and shall not be liable in respect of any conduct of the Caregiver(s)" | `[UNKNOWN]` |
| Absent reading rendered as fine | `[UNKNOWN]` for this vendor | Forbidden by the governing rule |

**Failure-mode discipline applied to this file itself.** Several of the cells above began as hypotheses —
*the app probably does not tell you when Bluetooth drops* — and none of them could be tested, because the app
cannot lawfully be obtained. **They are therefore recorded as `[UNKNOWN]` open questions, not as findings.**
That is the difference between this candidate and `nightscout`, where five hypotheses were checked against
source and **refuted** (and dropped). Where an implementation exists, hypotheses get tested; where one does
not, they get recorded as gaps and nothing more.

**The one asymmetry the vendor states itself:** the watch is described as recording continuously to on-board
flash and syncing through the app, which means a phone absence is a *delay* for **data** — while the **alert**
is stated to require the phone, Bluetooth, internet, an active subscription and the vendor's cloud. Data
survives more failure modes than the alarm does. That is the same distinction AnxietyWatch2's design takes
seriously, and it is the reason this candidate is directly useful even though the physiology is unrelated.

---

## 9. Provenance, missingness and reproducibility

**Provenance — Empatica.** Strong where it is regulated, absent where it is commercial. Every clearance is in
openFDA with dates, product codes and applicant; the classification definitions are quotable; K250515 gives
subgroups, denominators, confidence intervals and the analysis types (bench, retrospective). But the transfer
mechanism is **Avro/CSV in S3 with documentation supplied only to clients**, so a research customer's
provenance is whatever the vendor provides contractually. Nothing equivalent to a per-record source, unit or
validity flag is published.

**Missingness — Empatica.** `[UNKNOWN]`. The vendor publishes no absent-data model on any surface read. It
*does* publish **wearing detection** as a platform signal for trial sponsors — showing that the vendor
considers device-presence a distinct quantity from device-output. Whether an absent reading is distinguished
from a normal one in the alert path cannot be determined from public artifacts. This is the comparison's
sharpest open question for the candidate and aligns with the package's recurring theme: **"not measured" is
the state most often collapsed into a reassuring default.**

**Reproducibility — this inspection.** All artifacts are SHA-256 hashed and the retrieval is scripted
(`empatica_IMPLEMENTATION_INSPECTION.md` §10). Two hashes are themselves evidence: the 264-byte redirect is
byte-identical at both `developer.empatica.com` and the GitHub Pages URL, confirming the CNAME chain rather
than inferring it; and `/legal/` was fetched twice in separate passes with identical bytes, confirming the
page carrying two findings was stable. **One artifact resisted:** the 178-page, 112.8 MB User Manual was
retrieved in full and **has no fonts and no text layer** (`pdffonts` empty; `pdftotext` returns 178 bytes
across 178 pages), so it is neither searchable, nor screen-reader accessible, nor machine-checkable, and
contributes no rows to the reconciliation table.

**Reproducibility — AnxietyWatch2.** Registry-generated types, CI regeneration and diff, fixed reference
timestamps in tests, no wall clock in assertions, tolerance rather than float equality. The one related
defect class in this package came from the *other* side: **v1's by-session parser is coupled to OSCAR's CSV
column position**, and OSCAR's header has already grown from 22 to 27 columns since it was written — so
`Leak_95th` now lands in `Hours_Used`. Positional coupling to a third party's output is a provenance failure
in waiting.

---

## 10. Local, cloud, privacy and operational cost

| | Empatica | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| Where data lives | Empatica Cloud + customer S3 | device + user's iCloud (per v1 docs) | owner's TimescaleDB |
| Account | required for the features that matter | none | none |
| Ongoing cost | **$399 + $17.40–$52.90/mo**, mandatory | none | hardware only |
| Offline operation | none documented | works offline | by design |
| Retention | "as long as reasonably necessary…", no period | n/a | raw `samples` dropped at 2 years (amendment 01) |
| Encryption | "Bluetooth 5 with encrypted pairing and data transfer"; ISO/IEC 27001:2022 claimed | n/a | amendment 02 |
| Research/privacy documents | EHMP policy + participant policy + **DPA**; EpiMonitor per jurisdiction (US, WA, GB, EU×7); Parkinson's; Embrace2; website | — | `AMENDMENTS.md` |
| Security evidence published | ISO/IEC 27001:2022 statement; ISO 13485 certificate | — | — |
| Open to inspection | no | no | yes (owner's own system) |

**The privacy notice is the finding in this section.** The US notice expressly covers "the EmbracePlus watch,
the associated App, and the associated EpiMonitor Account page", is dated **2024-12-01** (≈22 months before
this research date), and alongside health data describes **Google Analytics**, **pixel tags**,
**cross-device tracking**, **advertising networks**, and sharing "customer list information (such as your
name, email address and other contact information)" with third parties for targeted advertising.

Two ambiguities are recorded rather than resolved: **whether the advertising language is meant to reach
watch-derived health data or only the web Portal's browsing activity is not determinable from the document**;
and the notice states that additional notices control on conflict, so the consumer notice may not be the
governing instrument at all. **This is a documentation finding — it says nothing about actual processing**,
and it is recorded with the most conservative reading governing.

**Operational cost is a user-facing property, not a footnote.** The alert path's availability is a function of
a subscription, a dedicated phone, a 15-foot radio range, internet, and the vendor's cloud. Each is stated by
the vendor. AnxietyWatch2's specified design removes each — and in exchange, it inherits the responsibility
for them, which is precisely the trade this comparison exists to make visible.

---

## 11. Alerting and the alarm path

This is the layer the candidate was shortlisted for (shortlist dimensions C22–C24).

**What Empatica ships** `[PUBLIC CLAIM]` + `[REGULATOR RECORD]`:

1. **Detection** — EpiAlgo v2.1 on-watch; GTC seizures ≥ 20 s; explicitly **not** non-motor seizures
   (complex partial, absence, non-convulsive). Cleared as product code **POS**, whose definition adds: "**does
   not predict seizure onsets, and is not intended as a standalone seizure monitoring device**."
2. **Delivery** — "a phone call and a text message" to active caregivers; manual self-trigger as well.
3. **False-alarm handling** — an explicit **cancellation window** before caregivers are contacted, plus
   **adjustable sensing precision** (high for rest/sleep, low for low-intensity activity) and an option to
   switch detection off entirely.
4. **Empirical burden** — FAR per 24 h: 0.70 / 0.28 (EMU) and **0.34 / 0.25** (real-world, paed. / adult).
5. **Contractual limit** — the vendor "**cannot guarantee that a call or text message will reach any
   Caregiver**"; no accuracy or reliability warranty; liability capped at the purchase price; **no liability
   for personal injury or death**.

**What AnxietyWatch2 specifies `[V2 target]`:** a `cns-engine` producing alerts into a supervised alarm layer
with multi-modal delivery (amendment 03) and a hardware dead-man switch, running on the owner's hardware.

**The three things the comparison should take from the pairing:**

1. **A false-alarm rate is a product parameter.** Empatica has a number, a mode to reduce it and a cancellation
   window. AnxietyWatch2 has none of the three visible in the specification read, and its alarm layer is
   specified — so this is the right moment to require a stated acceptable rate, a cancellation path and a
   sensitivity control, or to record deliberately that they are not wanted.
2. **Alert cancellation before the caregiver is contacted** is a requirement the vendor's own marketing implies
   but does not quantify.** The window's length is unpublished. V2 should decide its own, and publish it.
3. **The regulator's sentence is the strongest argument for V2's three-state rule.** A cleared Class II
   seizure monitor is *defined* as not being a standalone monitoring device. The regulated world's own answer
   to "what if the signal isn't there" is a mandatory scope limit. V2's answer is a data model that cannot
   render "not measured" as measured — a stronger, structural version of the same idea.

**One point where the candidate is unequivocally ahead.** Empatica publishes a **wearing-detection** signal
and its trial customers can see whether the device is on the body. V2's three-state rule is the right
principle, but Empatica has shipped a first-class "is the sensor on the person" quantity, which is the
concrete thing a caregiver actually wants to know. Recorded as a genuine advantage.

---

## 12. User-facing consequences

**A user of EpiMonitor receives:** a cleared detection device with published false-alarm statistics, an
automated caregiver alert by call and SMS, a cancellation window, adjustable sensitivity, a seizure diary,
plan-gated reports, an app in 8 languages, and a research platform behind it. **They also accept:** a
prescription, a $399 device, a mandatory subscription, a plan-gated number of caregivers, a dedicated phone
kept within 15 feet with Bluetooth and internet, a cloud in the path, a EULA that declines to warrant
delivery or accuracy and caps liability at the purchase price, a privacy notice permitting advertising and
list-matching disclosures, and a publishing manual they cannot search or screen-read.

**A user of AnxietyWatch v1 receives:** local self-tracking with a CPAP CSV import and HealthKit integration,
no subscription, no account, no vendor in the path — and an upstream data format they do not control, whose
column layout has already changed under them since the importer was written.

**A user of AnxietyWatch2 receives (specified):** their own store on their own hardware, feeding specified
detection and alarm layers, with unit and threshold definitions in one regenerated file, no account, no
subscription, no plan gates — and, at the research date, a partially built system: six-track changes at
various completion, no AHI derivation, an event classifier that nothing consumes, and alarm-layer
requirements that are specified but not shipped.

**The honest summary of the trade.** Empatica gives a user a **regulated, evidenced, working alert** with a
cloud, a fee and a contract. AnxietyWatch2 proposes **ownership and independence** with unfinished work and
no regulatory standing. A user with epilepsy is clearly better served by EpiMonitor today. A user who wants
their own data under their own control gets something Empatica does not offer. **These are different
products answering different questions, and the comparison is most useful when it says so.**

---

## 13. Limitations, unverified claims and evidence gaps

**None of the following is a finding; all are gaps. Absence of evidence is `[UNKNOWN]`, not proof of absence.**

| # | Gap | Why it matters |
|---|---|---|
| **E1** | **The User Manual's content.** 178 pages, 112.8 MB, **no fonts and no text layer**; retrieved in full and unreadable. It is the surface most likely to state limits in plain language to the wearer. | The standalone-reliance question (Q2) is the candidate's most direct question about the alert layer and it is blocked by an artifact that **exists**. Closing it needs an OCR decision, not a fetch. |
| **E2** | **The 98% figure's source.** No citation on the page; not reconstructible from K250515; the **Publications** page was not consulted. | Either a source exists or a medical device page makes an untraceable performance claim. |
| **E3** | **Any runtime behaviour at all.** Alert latency, cancellation window length, out-of-range handling, lapsed-subscription behaviour, sensor-failure behaviour. | These are the questions the comparison most wants answered and this inspection cannot answer any of them. No lawful binary; iOS-only; prescription-gated. |
| **E4** | **The Cloud API contract.** No OpenAPI, endpoints, auth scheme, rate limits or error catalogue published; "provided" to commercial clients. | The integration surface cannot be inspected from public artifacts. This is why the candidate's mode cannot be `Performed` on an API surface. |
| **E5** | **Android presence.** Four candidate identifiers returned 404; Play returns 403 to non-browser clients, so this is **not** evidence of absence. | The vendor claims Android; the claim is unchecked. |
| **E6** | **Plan detail.** The support-centre article returned HTTP 403 (Cloudflare). | Annual pricing and per-plan differences come from the store page alone. |
| **E7** | **K232915 summary.** The FDA path returned 404 at this date; only K250515 was retrieved. | K250515 covers the same device name and product code, so this is a retrieval failure, not an evidential gap about the clearance's existence. |
| **E8** | **HIPAA and GDPR artifacts.** "compliance with industry standards including HIPAA, GDPR and ISO" is asserted on a marketing page. An ISO/IEC 27001:2022 statement and an ISO 13485 certificate are published; no BAA, DPA or GDPR artifact was fetched, though the EHMP **Data Processing Agreement** is listed on `/legal/`. | The strong version of the compliance claim is asserted where the published artifacts support only the weak version. |
| **E9** | **"corrected PPA" is undefined** in K250515. | An undefined term in the decisive table. |
| **E10** | **No security.txt, penetration-test report, vulnerability-disclosure programme or incident history found.** | `[UNKNOWN]`; the Internet Archive's CDX index shows only a 2023-07-31 crawler *probe* of `/.well-known/security.txt` with no archived response, which is the absence of a capture, not evidence about the file. |
| **E11** | **Only nine surfaces were read.** The use-case, actigraphy, sleep, CRO, academic-research, eCOA, digital-biomarkers, publications, case-studies, compliance and resources-library pages were not enumerated. | Nothing in this comparison is a complete account of the vendor's claims. |

**Three standing cautions for anyone quoting this comparison.**

- **The 98% claim is not characterised as false here.** It is characterised as untraceable from the page that
  makes it, against a regulator document that reports different metrics.
- **No medical or safety judgement is offered.** A 510(k) with bench equivalence and retrospective analyses is
  a lawful, common route to market; the observation is that the marketing page does not describe the route it
  took.
- **A gap is not a defect.** E1–E11 are things this inspection could not determine. Several of them would
  probably be resolved in the vendor's favour.

---

## 14. Balanced conclusion

**Empatica EpiMonitor is the most evidentially serious candidate in this shortlist and, in the specific matter
of its headline number, the least transparent.** Both are true and the comparison is worthless if either is
dropped.

What it does that nothing else here does:

- Ships an alert whose purpose is to bring a person help, with a cleared indication defining exactly what it
  will and will not detect.
- Publishes **false-alarm rates with confidence intervals and denominators**, and ships three separate
  mitigations for false alarms.
- Publishes a **standards list** (ISO 14971, IEC 60601-1 series including usability and home-healthcare,
  IEC 62304, IEC 62366-1, ISO 10993), an **ISO/IEC 27001:2022** statement and a linked **ISO 13485**
  certificate.
- States its own limits in plain language: no non-motor seizures, ages 6+, phone within 15 ft, compatible
  device required, no support for obsolete versions.
- Publishes a **contract** that says exactly what it will not promise — which is more than most consumer
  health products do, and is a positive to record.

What it does not do, and what the comparison should carry forward:

- **The headline accuracy figure has no reachable source**, and the metric the regulator considered decisive
  (false alarms) is absent from the page.
- **The clearance citation names the platform, not the device**, so a reader checking the supplied numbers
  finds the wrong clearances.
- **The compliance artifact the vendor links disclaims the claim beside it** — a placement problem, not an
  impropriety, but a reader is left without the MDR evidence they were looking for.
- **The published instructions for use cannot be searched or read by assistive technology.**
- **The manufacturer's own document says no clinical studies were performed on the device**, while the
  product page presents a clinical accuracy figure.
- **The privacy notice covering the device service permits advertising and list-matching disclosures**, and
  was last updated 22 months before this research date.

**For AnxietyWatch v1:** no product overlap. EpiMonitor is a system v1 could never be the alerting path for.
The transferable lesson is upstream: v1's CPAP import rests on OSCAR's CSV layout, whose header has already
grown from 22 to 27 columns, and Empatica's own E4 withdrawal shows how completely a vendor can retire a data
interface — four days from sunset to an emptied developer site.

**For AnxietyWatch2:** the candidate is a specification reference, not a competitor. It shows what a shipped
alarm layer costs and what it buys — a cancellation window, a sensitivity control, wearing detection, a
published false-alarm rate, and a contract that is candid about delivery. **Three concrete actions this
comparison supports, none of which is taken here:** state an acceptable false-alarm rate for each alert type;
decide and publish the cancellation window; and keep the three-state rule, because the regulator's own
definition of this device class requires it.

**The last word belongs to the vendor's own documents, because they say it better than this comparison can.**
The product page: *"designed to keep you and your loved ones safe."* The EULA linked from that page: *"EMPATICA
cannot guarantee that a call or text message will reach any Caregiver."* And the regulation defining the
device: *"does not predict seizure onsets, and is not intended as a standalone seizure monitoring device."*
All three are current. A reader who sees only the first has been told something the other two do not support —
and that, rather than any comparison to AnxietyWatch2, is the most useful finding of candidate 7.

---

## Related files

- `empatica_CLAIMED_FEATURES_AND_PURPOSE.md` — Phase 1 public-claims research; evidence ledger E-1…E-83;
  artifact hashes; the eight recorded surface conflicts
- `empatica_IMPLEMENTATION_INSPECTION.md` — Phase 2A; inspection-mode record; the manual text-layer test;
  27-claim reconciliation; gaps G1–G10; reproduction script
- `PROMPT_2.md` — second-stage brief (base `PROMPT2-EVIDENCE-FIRST.md` + supplementary
  `PROMPT2-PRIVACY-AND-OPERATING-MODE.md`)
- `../PROCESS_STATE.md` — pipeline state, candidate dossier, residual uncertainty R1–R11
- `../RESEARCH_METHOD.md` — evidence classes, inspection modes, standing sub-checks
- `../CANDIDATE_SHORTLIST.md` — ranking and rationale
- `../nightscout/nightscout_COMPARISON.md`, `../sleephq/sleephq_COMPARISON.md`,
  `../oscar/oscar_COMPARISON.md` — the comparisons this one is written to sit beside
