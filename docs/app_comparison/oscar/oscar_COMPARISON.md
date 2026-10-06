# OSCAR versus AnxietyWatch v1 and AnxietyWatch2

**Candidate:** `oscar` — OSCAR, the Open Source CPAP Analysis Reporter
**Comparison date:** 2026-10-06
**Vendor version assessed:** 2.0.1 (released 2026-06-23); source inspected at `28df2e46` (2026-10-04)
**Inspection mode:** **Performed** — full GPLv3 source read; no runtime observation
**Base prompt:** `PROMPT2-EVIDENCE-FIRST.md`, with the `PROMPT2-TECHNICAL-SYSTEMS` supplementary block

---

## 1. Executive finding

**OSCAR is not a competitor to either AnxietyWatch version. It is an upstream dependency of v1, and a
reference implementation whose newest code follows this project's rules and whose oldest code violates
them.**

Three findings, in order of consequence.

**First — v1's CPAP importer is coupled to OSCAR's report vocabulary by column *position*, and that
vocabulary has already changed.** v1's by-session parser maps fields at hardcoded indices 0–20 behind a
`count >= 21` guard, and detects the format with a six-token prefix. The report v1 was written against —
22 columns, matching OSCAR 2.0.1 exactly, token for token — is **27 columns in the inspected source**,
with `OAHI`, `CAHI`, `A`, `OH` and `CH` inserted mid-header. The detector still fires (because `OAHI`
begins with `OA`), the guard still passes (because `27 >= 21`), and every field from index 4 onward is
silently misassigned — including **`Leak_95th` being read as `Hours_Used`**, so a 6.5-hour night with a
leak 95th of 12.4 becomes 12.4 hours of therapy. No warning, no skip. See §3.

**Second — OSCAR answers the invalid-data question three different ways in one program**, and the
answers contradict each other:

| Path | Invalid or absent value | Encoded as | Recoverable downstream? |
|---|---|---|---|
| ResMed EDF (oldest) | end-of-data sentinel `-1`/`0` | the **previous sample**, written in place | **No** |
| Apple Health (newest) | out-of-range or unknown-unit | dropped, named reason, first ten to stderr | **Partly** |
| Summary store | no oximetry collected | the number **`0.0`**, hidden only by `NULLIF(col,0)` at export | **Only via the report** |

The newest path agrees with this project's rules almost exactly. The oldest violates them in the specific
way the rules forbid. The store violates them in a way that is invisible to anything reading the
published database — which OSCAR actively invites by publishing the schema and shipping a Python reader.
See §7.2.

**Third — the leak number v1 imports is not the number the CPAP machine reported.** OSCAR parses the
device's `Leak.95` and then does not persist it, recomputing the percentile from the time-series instead.
The vendor's own method review reproduces a **2.0×** disagreement with the device on one night and names
a gain-handling defect in one of the two code paths that compute the statistic. Any leak-based conclusion
v1 draws sits downstream of that. See §3.3.

**Verdict on shortlist position: dependency and reference, not competitor.** OSCAR is where v1's CPAP
vocabulary came from, and it is the only artefact on the shortlist that shows a volunteer team getting
"not measured" right in 2026 and wrong in 2013 — in the same repository, with the diffs visible. On
interoperability and exit it beats both AnxietyWatch versions outright; on collection, real-time response
and privacy-by-architecture it is not in the same category, because it is a desktop review tool and
claims nothing else.

---

## 2. Product and availability

| Field | OSCAR | AnxietyWatch v1 | AnxietyWatch v2 |
|---|---|---|---|
| Kind | Desktop sleep-data review application | iOS consumer health app | Pi collector + iOS client + admin console |
| Licence | **GNU GPL v3** | Proprietary (owner's) | Proprietary (owner's) |
| Vendor | The OSCAR Team (voluntary) | Owner | Owner |
| Version | 2.0.1, 2026-06-23 | — | In development |
| Platforms | Windows 10/11, macOS 13+/15.7+, Debian 12/13, Ubuntu 22.04/24.04/26.04, Chromebook, RasPiOS 12 | iOS | Linux (Pi 5) + iOS |
| Mobile / web | None | iOS only | iOS client + web dashboard |
| Price | **Free.** "OSCAR never asks for payment — It is free and always will be" | Free | Free (self-hosted) |
| Funding | Donations to Apnea Board (hosting) | — | — |
| Account | **None** | None | Device enrollment, operator approval |
| Inspection mode | **Performed** (full source read) | n/a | n/a |

**On the licence as a comparative fact.** OSCAR's GPLv3 obligations are why this package contains an
implementation inspection at all while candidates 1–3 could only reach "not performed" or "partially
performed". That is an **inspectability** guarantee, not a security or privacy guarantee, and the
distinction is worth keeping straight: source availability lets anyone check the claims, and it says
nothing by itself about whether the code is good.

---

## 3. The dependency boundary

### 3.1 What v1 inherits — and the distinction that matters

v1's CPAP model is inherited from OSCAR. Eight touchpoints, verified in the v1 tree:

| # | v1 evidence | Location | Inherited kind |
|---|---|---|---|
| 1 | CPAP path described as "reverse-engineered by OSCAR community" | `REQUIREMENTS.md:52` | **semantic** |
| 2 | "CPAP SD card data parser (based on OSCAR format documentation)" | `REQUIREMENTS.md:296` | **semantic** |
| 3 | First-class session source `source == .oscar` | `ModelEnumTests.swift:29–51` | vocabulary + provenance |
| 4 | `CPAPImporter.isOSCARBySessionFormat` | `CPAPImporter.swift:291–295` | **vocabulary** |
| 5 | `isOSCARFormat` — daily Summary header | `CPAPImporter.swift:283–285` | **vocabulary** |
| 6 | Noon-to-noon day boundary | `CPAPBySessionImportTests.swift:97` | **semantic** |
| 7 | Unscored hours stay out of AHI | `CPAPBySessionImportTests.swift:162` | **semantic** |
| 8 | `importSource: "oscar"` round-trips | `ReportGeneratorTests.swift:463`, `RestoreFromServerTests.swift:169` | provenance |

The distinction is not academic. A **vocabulary** dependency (a header string, a date convention) breaks
when OSCAR changes its output — loudly if there is a schema check, silently if there is not. A
**semantic** dependency (what AHI counts, what a session is, which hours are scored) does not break at
all when OSCAR changes its code; it simply *is* OSCAR's definition, frozen at the moment v1 was written,
and it diverges slowly as OSCAR revises its own understanding.

Both kinds are present, and they have different remedies. §3.2 is the vocabulary break. §3.3 is a
semantic one that has already diverged.

### 3.2 The vocabulary break — verified, and it has already happened upstream

OSCAR has **two** CSV producers and v1 consumes one format from each:

| OSCAR producer | Output | v1 detector |
|---|---|---|
| Legacy dialog exporter (`oscar/exports/exportcsv.cpp`) | Details / Summary / Sessions | `isOSCARFormat()` — the daily path |
| Database-driven system report (`oscar/docs/system_reports.orf`) | `Daily Summaries/by Day`, `Session Summaries/by Session` | `isOSCARBySessionFormat()` — the by-session path |

**The daily path** is a literal match. `exportcsv.cpp:190–217` writes
`tr("Date") + "," + tr("Session Count") + "," + tr("Start") + "," + tr("End") + "," + tr("Total Time") + "," + tr("AHI")`,
and v1 matches `"date,session count,start,end,total time,ahi"`. Same six tokens, same order.

**The by-session path** was an exact 22-column match. v1's own comment states the contract it was
written against — *"OSCAR 'by session' export: one row per mask-on/mask-off session, 22 columns"*
(`CPAPImporter.swift:291`) — and the vendor's design record for the report 2.0.1 shipped lists exactly
those 22 columns in exactly that order, under the release note *"[update] Add more fields to Session/by
Session and Daily/by Day CSV reports"* (`Notes/specs/2026-06-10-csv-report-fields-design.md`). v1's
importer is vintage-matched to OSCAR 2.0.1.

**The header has since grown.** `oscar/docs/system_reports.orf:145` opens the report and its `SELECT`
aliases run `:153–179`; it now emits 27 columns. The five insertions and what v1 does with them:

| v1 index | v1 believes | OSCAR now sends | v1 stores it as |
|---|---|---|---|
| 4 | OA | **OAHI** | `obstructiveEvents` |
| 5 | UA | **CAHI** | `ua` |
| 6 | H | **OA** | `hypopneaEvents` |
| 7 | CA | **UA** | `centralEvents` |
| 8 | RERA | **A** (all apnoea) | `reraEvents` |
| 9 | Pressure_Avg | **H** (hypopnea count) | `pressureMean` |
| 10 | Pressure_Min | **OH** | `pressureMin` |
| 11 | Pressure_Max | **CH** | `pressureMax` |
| 12 | Pressure_95th | **CA** | `pressure95th` |
| 13 | Leak_Avg | **RERA** | `leakAvg` |
| 14 | Leak_Max | **Pressure_Avg** | `leakMax` |
| 15 | Leak_95th | **Pressure_Min** | `leakRate95th` |
| 16 | SpO2_Avg | **Pressure_Max** | `spo2Avg` |
| 17 | SpO2_Min | **Pressure_95th** | `spo2Min` |
| 18 | Pulse_Avg | **Leak_Avg** | `pulseAvg` |
| 20 | **Hours_Used** | **Leak_95th** | `totalUsageMinutes` |

Why nothing stops this:

```swift
// CPAPImporter.swift:293–295 — the detector
normalizedHeader(header).hasPrefix("date,start,ahi,rdi,oa")
// → "date,start,ahi,rdi,oahi,cahi,..." satisfies it, because OAHI begins with OA

// CPAPImporter.swift:713–716 — the only structural guard
guard fields.count >= 21 else { return .skip(reason: "expected at least 21 columns, found \(fields.count)") }
// → 27 >= 21
```

Then `parseBySessionRow` — which receives the split fields and **no header** — maps by index. The result
is a fully-parsed, entirely wrong row. Index 20 is the worst: `guard ... hoursUsed.isFinite, hoursUsed >= 0`
passes for a leak value, so the leak 95th becomes the therapy duration.

**Timing, with its uncertainty stated.** v1 is correct against OSCAR 2.0.1. The break arrives with the
next OSCAR release. That the 27-column form is unreleased is an **inference** from two facts: the
specification that introduced it is dated 2026-08-06 (after the 2026-06-23 release), and the 2.0.1
release notes contain no mention of central/obstructive hypopnea work. The 2.0.1 binary was not executed
to confirm which header it emits. **The conclusion holds on either branch; only the timing changes.**

**Narrowing factors, stated fairly.** The correspondence is with OSCAR's *report*, not its database, so
a user who never exports the by-session report never produces the file. And v1's guard is at least a
guard — a 19-column file is rejected. v1 fails on *widening*, not on truncation.

### 3.3 The semantic divergence — already present

v1 imports OSCAR's leak values. The vendor's own method review
(`Notes/Developer Notes/LEAK_PERCENTILE_METHOD_REVIEW_2026-04-16.md`) reports:

- The device's `STR` record reports `Leak.95` = **1.2 L/min** for one night; OSCAR's PLD-derived p95 for
  the same night is **2.4 L/min**. Ratio **2.0**. A second night agrees exactly, so this is not a
  uniform scaling error.
- Cause: `leak50`/`leak95` *are* parsed (`resmed_loader.cpp:1969–1980`) but not persisted — the p50/p95
  stores are commented out (`resmed_loader.cpp:2905–2909`). OSCAR recomputes instead of reporting.
- Two further defect candidates: gain handling in `Day::percentile` when sessions in one day have
  different gains, where the multi-day path does the safer per-session normalisation; and
  time over-accumulation in the waveform branch of `updateCountSummary()`.

The review's own conclusion is measured: *"This is a product-behavior mismatch, not necessarily a pure
math bug."* That is the correct framing and this report adopts it. The method choice is defensible
(recomputing from a partitioned time-series is a legitimate way to get a percentile); what it means for
v1 is narrower and specific: **v1's leak numbers are OSCAR's numbers, not the machine's numbers, and they
can differ materially on a given night.**

**One correction to a common assumption.** OSCAR's store is *not* device-local wall-clock strings.
`sessions.start_time` holds epoch **milliseconds**, and the day is derived at query time as
`date(s.start_time/1000 - 43200, 'unixepoch', 'localtime')` — the stored epoch minus exactly 12 hours.
That is closer to AnxietyWatch2's rule than a naive reading suggests. The real difference is where the
epoch came from: OSCAR computes it from the CPAP device's own clock, so a wrong device clock produces a
wrong epoch value, which is why the correction ledger exists. AnxietyWatch2 derives its epoch from the
collector's clock at ingest.

### 3.4 Does v2 inherit any of it? No — and the reason is structural

AnxietyWatch2's AirSense 11 path does not inherit OSCAR's leak or AHI semantics, for a reason that is
stronger than "different code":

**v2 does not compute AHI, and does not compute percentiles anywhere.** The registry's AirSense 11 source
declares raw instantaneous signals — `patient_flow` (L/s), `mask_pressure`, `mask_pressure_2s`,
`inspiratory_pressure`, `expiratory_pressure` (cmH2O), `leak` (L/min, range 0–300), `tidal_volume` (mL),
`minute_ventilation`, `target_minute_ventilation`, `ie_ratio`, `snore_index`, `flow_limitation`,
`inspiratory_duration`, `trigger_cycle_event` (`vitals/registry.toml:317–329`) — plus the shared `hr`,
`spo2` and `respiratory_rate`. There is no AHI type, no per-event registry type, and no percentile
computation anywhere in the v2 tree. Device-native leak is L/s and is converted to L/min **once, at
decode**, via the registry's `conversion = { leak = 60.0 }` (`:328`).

**One precision the comparison owes the reader, because it narrows the gap:** v2 *does* decode the
device's own scored events. `crates/vitals-decode/src/events.rs` classifies `HypopneaEnd`,
`CentralApneaEnd`, `ObstructiveApneaEnd`, `ApneaEnd`, `ReraEnd`, `CsrStart` and `CsrEnd` into a
bounded `EventKind` (`:20–48`), capped at 16 records per notification (`:8`). But the classification is
not consumed by anything: the collector calls `decode_event`, then feeds only the **raw event name**
into the session lifecycle (`resmed_air11_collector.rs:467`, `:536`), which acts on
`TherapyStart`/`MaskOn`/`TherapyStop`/`MaskOff`/`StandbyStarted` and ignores the rest
(`as11_session.rs:156`, `:168`); `event.kind` has no reader outside `events.rs` and its tests. So v2 has
the **classification implemented and tested, and not yet wired to a consumer** — which is neither "no
event handling" nor "an AHI computation", and should be read as the former direction of travel rather
than as a shipped capability.

So the leak-percentile question cannot propagate: v2 stores the device's leak signal, and any percentile
is a query over that signal in the analysis layer, computed from v2's own samples with `time_bucket_gapfill`
rather than by a device-firmware or third-party convention. The overlap with OSCAR is the *concept* of
leak — not the implementation, not the column, not the definition.

**Status, stated honestly.** `openspec/changes/resmed-air11-ble-stream` is 6 of 7 tasks complete; the
open one is T-A11-6, the bounded on-Pi physical acceptance gate. So this path is **v2 current, pending
physical acceptance** — not "shipped" and not "aspirational". Separately, `openspec/changes/cpap-ezshare-ingest`
— the SD-card/EDF ingest path, which is where v2's clock-metadata and attempt-ledger rules live — is
**0 of 6 tasks: v2 specified target, nothing built.** Today, v2 has no EDF parser and therefore no
exposure to OSCAR's EDF sentinel handling.

### 3.5 A second, smaller instance of the same failure mode

The header drift in §3.2 is a documented-output-versus-actual-output divergence that no mechanism
detected. There is a second one, smaller and harmless, recorded because it shows the pattern is not a
one-off: OSCAR's published schema philosophy states that waveform BLOBs use "qCompress level 9" achieving
"40-60% compression" (`Notes/Database/SCHEMA_DESIGN_PHILOSOPHY.md:66`), while the inspected code calls
`qCompress(data, 6)` with an explicit rationale — "6 is ~2x faster than 9 with only ~1-2% larger output"
(`event_data_repository.cpp:531`, gated on ≥ 500 bytes and a >10% saving, `:526–540`).

Consequences: none, here — the level is recorded per BLOB and both ends decompress correctly. The
relevance is the mechanism, not the magnitude. A consumer that trusted the documentation would draw a
wrong inference about storage size, and nothing in the project compares the document to the code. That is
the same absence of a contract check that makes §3.2 latent rather than immediate, and it is the reason
v1 cannot rely on OSCAR's own description of its outputs as a schema.

---

## 4. Evidence ledger

Every load-bearing claim, with source, class and date. Nothing in this table is a runtime observation.

| # | Claim | Source | Class | Date |
|---|---|---|---|---|
| 1 | OSCAR is GPLv3; source fully available | `COPYING` in pinned tree | static finding | 2026-10-06 |
| 2 | 2.0.1 released 2026-06-23; SQL database rewrite; new loaders; Time Corrections replaces Clock Drift | `sleepfiles.com/OSCAR/` release notes | public claim | 2026-10-06 |
| 3 | No telemetry, analytics or crash reporting of any kind | source search across tree | static finding | 2026-10-06 |
| 4 | No Bluetooth/BLE implementation anywhere | source search; UI text "probably not" supported | static finding | 2026-10-06 |
| 5 | 19 loaders registered | `oscar/main.cpp:1267–1285` | static finding | 2026-10-06 |
| 6 | `device_time_corrections` is typed, reasoned, date-scoped, drift-modelled, reversible | `database_schema.cpp:1519–1533` | static finding | 2026-10-06 |
| 7 | `session_summaries.summary_only` marks summary-fidelity sessions | `database_schema.cpp:680–694` | static finding | 2026-10-06 |
| 8 | 473 test assertions across 14 files; no CI configuration exists | `oscar/tests/`; `find` for CI files | static finding | 2026-10-06 |
| 9 | By-session report shipped by 2.0.1 had exactly 22 columns matching v1's header in order | vendor design record + `CPAPImporter.swift:291` | static finding | 2026-10-06 |
| 10 | By-session report is now 27 columns; five inserted mid-header | `oscar/docs/system_reports.orf:145–179` | static finding | 2026-10-06 |
| 11 | v1's detector and guard both accept the widened header | string comparison of both trees | static finding (not executed) | 2026-10-06 |
| 12 | v1's parser is positional; index 20 is `Hours_Used` and now receives `Leak_95th` | `CPAPImporter.swift:709–765` | static finding | 2026-10-06 |
| 13 | The 27-column form postdates the 2.0.1 release | spec date 2026-08-06 vs release 2026-06-23; `system_reports.orf:14` — "arrived with schema v18", `CURRENT_SCHEMA_VERSION = 19`; release notes | **inference** | 2026-10-06 |
| 14 | ResMed EDF substitutes the previous sample for the sentinel (`c = last`) | `resmed_loader.cpp:4360–4368` | static finding | 2026-10-06 |
| 15 | Apple Health rejects by named reason and drops, with parse-time unit conversion | `applehealthDataParsing.cpp:337–446` | static finding | 2026-10-06 |
| 16 | Rejection count is local, capped at 10 stderr lines, absent from the import summary | `applehealthDataParsing.cpp:230–236`; `applehealth_loader.h:26–37` | static finding | 2026-10-06 |
| 17 | Absent oximetry is stored as `0.0` and masked only by `NULLIF` at export | vendor design record; `system_reports.orf:175,177` | vendor statement + static | 2026-10-06 |
| 18 | Device `Leak.95` is parsed but not persisted; percentiles are recomputed | `resmed_loader.cpp:1969–1980`, `:2905–2909` | static finding | 2026-10-06 |
| 19 | A 2.0× divergence with the device's own `Leak.95` is reproduced | `LEAK_PERCENTILE_METHOD_REVIEW_2026-04-16.md` | **vendor statement** | 2026-04-16 |
| 20 | Day report stored a p90 under the name `Pressure_95th`; vendor relabelled it | vendor design record | vendor statement | 2026-06-10 |
| 21 | `central_count` renamed `unclassified_count`; OHI/CHI printed only when data exists | schema v10 notes; `Notes/specs/2026-08-06-...md` | vendor statement | 2026-10-06 |
| 22 | Sessions are stored as epoch milliseconds; day derived at query time | `system_reports.orf:154` | static finding | 2026-10-06 |
| 23 | v2's AirSense 11 source declares instantaneous signals (17 types); no AHI type; no percentile computation in the tree; device event records decoded and classified but consumed by nothing | `vitals/registry.toml:317–329`; `events.rs:19–51`; `as11_session.rs:156,168` | static finding (v2) | 2026-10-06 |
| 24 | v2 rejects out-of-range with the value and range named: "rejected, never clamped" | `h10_build.rs:123–157`; test at `:273` | static finding | 2026-10-06 |
| 25 | v2 distinguishes absent from invalid: `assert!(b.rejected.is_empty(), "absent is not rejected")` | `bp_build.rs:127` | static finding | 2026-10-06 |
| 26 | v2 logs rejections to stderr; the durable attempt ledger is specified, not built | `h10_collector.rs:330`; `cpap-ezshare-ingest` at 0/6 | static finding | 2026-10-06 |
| 27 | v2's CPAP-file ingest specifies clock correction as versioned metadata, never updating samples | `cpap-ezshare-ingest/design.md` §6 | v2 specified target | 2026-10-06 |
| 28 | v2 specifies a clean-room OSCAR boundary: "no GPL implementation code is copied" | `cpap-ezshare-ingest/design.md` §11 | v2 specified target | 2026-10-06 |
| 29 | Waveform compression level: design doc says level 9, inspected code uses level 6 | `SCHEMA_DESIGN_PHILOSOPHY.md:66` vs `event_data_repository.cpp:526–540` | vendor statement + static finding | 2026-10-06 |

---

## 5. Feature matrix

Status labels: **v1 documented** · **v1 implemented** · **v2 current** · **v2 specified target** ·
**v2 aspiration**. OSCAR entries are implementation findings unless marked *claim*.

| Capability | OSCAR 2.0.1 | AnxietyWatch v1 | AnxietyWatch v2 |
|---|---|---|---|
| CPAP device support | ~19 loaders, 12+ CPAP families | OSCAR CSV import only | AirSense 11 over BLE (**v2 current, pending physical acceptance**); EDF/ez Share (**v2 specified target**) |
| Data arrival | SD card, USB/serial, file, shared profile — **post-hoc, manual** | Manual CSV import | Continuous BLE streaming (**current**); HTTP card mirror (**specified target**) |
| Real-time monitoring | **None.** "Live streaming mode" is a cable transfer mode | Post-hoc analysis | Continuous ingest; supervised alarm (**v2 specified target**) |
| Apnoea event counts | Yes — per-event table, AHI/RDI/OAHI/CAHI per session and day | Yes, from CSV | **Not computed.** Device event records are decoded and classified (`events.rs:19–51`) but not indexed, not persisted as samples, and read by no consumer |
| Leak | Percentile recomputed; 2.0× divergence documented | Inherited from OSCAR CSV | Device's own leak signal, L/min, converted once at decode |
| Pressure metrics | Yes, incl. min/max/percentile | Yes, from CSV | `mask_pressure`, `inspiratory_pressure`, `expiratory_pressure` (raw) |
| Oximetry | Contec CMS50/F37, MD300W1, Viatom; plus CPAP-recorded | Via CSV columns | EMAY `SleepO2` — decoder present (`vitals-decode/src/emay.rs`, registry `emay` provides `spo2`,`hr`); **v2 current** |
| Waveform / breath-by-breath | Yes — compressed BLOBs, CRC16 | No | Raw flow/pressure stream (**current**) |
| Heart rate / HRV | Via oximeter pulse and Apple Health HRV SDNN | Polar H10, phone | Polar H10, phone (**current**) |
| Respiratory rate | Apple Health only | — | AirSense 11 (registry type; **pending physical acceptance**); Acconeer XE125 (**v2 specified target** — 2 of 7 task boxes) |
| Subjective capture | Post-hoc journal + 0–100 "feelings" meter at a desk | Panic-time capture on the phone | Panic-time capture on the phone (**v2 in progress** — `ios-client-stage7`, 15 of 24 task boxes) |
| Environmental context | None | Weather, song, aircraft | Weather, song, aircraft (**per v2 scope**) |
| Alarm / response | **None** | Alerts | Supervised alarm + hardware dead-man switch (**v2 specified target**) |
| Automatic data loss detection | No (a fix is recorded for lost journal entries) | — | `max(seq) − count(*)` lost-beat count; decode-loss counter |
| Missing-data policy | Three inconsistent policies (§7.2) | n/a | Never `locf()`/`interpolate()`; `time_bucket_gapfill` returns explicit NULLs |
| Append-only store | No — `UPDATE`/`DELETE`; "Purge Range of Days"; no-migration upgrades | — | Yes; removal is a tombstone |
| Portable data export | **CSV (3 modes + templates), published SQLite schema, Python reader** | CSV/PDF reports | — |
| Third-party read access | **Published schema docs + Python demo program** | — | SQL via the admin console (**v2 in progress** — `admin-console-roadmap`, 17 of 46 task boxes) |
| Cloud/account | None | Optional sync | Private LAN / self-hosted |
| Telemetry | **None found** | — | None |
| At-rest encryption of the data file | **No** (password support removed in 2.0) | — | LUKS (v2 design) |
| Tests | 473 assertions, `-Werror`, ASan; **no CI** | Test suite | CI + pre-push gates |
| Source availability | **Yes, GPLv3** | No | No |

**Where OSCAR is simply better than both AnxietyWatch versions:** portable data access and exit. A
third party can read an OSCAR database without asking OSCAR for permission, because the schema is
published and a reader program is shipped. Neither AnxietyWatch version offers an equivalent.

**Where the comparison is not meaningful:** alarm, response, real-time and mobility. OSCAR is a review
tool with no ingestion path that reaches analysis during a night, so it cannot have these. That is an
architectural fact about a different kind of product, not a deficiency.

---

## 6. Evidence-backed workflows

| Workflow | OSCAR | v1 | v2 |
|---|---|---|---|
| "Review last night in detail" | **Strong.** Breath-by-breath graphs, event table, 35 daily statistics | CSV-derived summary | Raw streams + derived views |
| "Review the last year's trend" | **Strong.** Statistics page, custom report tree, CSV export | Reports | Continuous aggregates |
| "Find what woke me up" | Manual, post-hoc inspection | Correlations incl. aircraft/noise | Correlations + real-time context |
| "Get help while the event is happening" | **Not possible** | Alerts | Supervised alarm (**specified target**) |
| "Keep a contemporaneous note about how I felt" | Journal, typed later at a desk | Panic-time capture | Panic-time capture |
| "Move my data to another tool" | **CSV, published schema, Python reader** | CSV/PDF | — |
| "Prove my therapy is adequate to a clinician" | Device-format CSV that a clinician already recognises | Reports | Reports |
| "Run it without an account or a server" | Yes, by construction | Yes | Yes (self-hosted) |

The last row is where the two projects genuinely agree, and it is worth naming: both are offline-first
and neither requires an account. That is a shared position against candidates 1–3, not a difference
between them.

---

## 7. Data and analysis audit

### 7.1 Storage shape (T1)

OSCAR stores **dense waveforms as compressed, CRC-checked BLOBs** (compressed at level 6 in the inspected
code; the vendor's design record still says level 9 — see §3.5) plus materialised daily and session caches
keyed by `sessions_hash`. v2 stores **sparse scalar samples, append-only**, in a TimescaleDB
hypertable keyed by `(source, type, session_id, seq)` with the timestamp explicitly *not* the key.

What each shape makes easy:

- OSCAR can answer "show me the shape of the flow limitation between 03:12 and 03:14" because the
  waveform is a first-class stored object. **v2 can answer it for AirSense 11 because the raw flow and
  pressure signals are stored as samples**, but at sample granularity through a query, not as a
  pre-packaged series.
- v2 can answer "how many samples did we lose last night" (`max(seq) − count(*)`) and "was this sample
  signed" (`batch_id`), because identity is per-sample. OSCAR cannot: a lost waveform region is
  indistinguishable from a quiet one, since the sentinel substitution in §7.2 writes the previous sample
  over it.

**Materialised-cache exposure (T2).** OSCAR caches derived statistics with a hash invalidation key, and
the hazard has fired: schema v7 exists because value/time summaries were not persisted, so "weighted
averages defaulted to simple averages, causing significant inaccuracies in pressure statistics and other
metrics". v2's derived values are continuous aggregates and query-time computations over append-only
samples, so a wrong derivation is a wrong *query*, re-runnable against unchanged input, rather than a
wrong *stored* value that must be recomputed and rewritten. OSCAR's design makes a silent wrong number
possible in a way that v2's does not — and OSCAR's own bug log shows it happening.

**Fidelity markers (T4).** OSCAR's `summary_only`, `no_settings`, `events_loaded` and `enabled` flags let
a session say "this is incomplete, and here is how". The nearest v2 equivalent is the
`pi-prototype-storage-degraded-mode` specification plus `batch_id IS NULL` meaning "did not arrive
signed". Mapped:

| OSCAR flag | Meaning | v2 equivalent |
|---|---|---|
| `summary_only` | No waveform was available for this session | **None** — no equivalent fidelity marker |
| `no_settings` | Session has no device configuration | **None** |
| `events_loaded` | Per-event rows exist for this session | Implicit in whether event samples exist |
| `enabled` | Session included in reports | **None** |
| — | — | `batch_id IS NULL` — arrived unsigned |
| — | — | `max(seq) − count(*)` — measured packet loss |
| — | — | Degraded-mode state (**v2 specified target**) |

**This is a place where OSCAR is ahead and v2 should not pretend otherwise.** "This session is
summary-only" is exactly the three-state discipline this project mandates, and it is *implemented* in
OSCAR. v2 has the concept (degraded mode, unsigned batches) but not, today, a per-session fidelity marker.

**Capability gating (T3).** OSCAR gates channels on device capability and enforces
`oahiChannels = ahiChannels − cahiChannels` "by construction", and prints `OHI=`/`CHI=` only when the
data exists rather than `0.00` for every other device. v2's registry generates enums where an `i32` that
could hold any value cannot be transposed at a call site. Both are the same instinct arrived at
differently: OSCAR validates at computation time, v2 makes the invalid state unrepresentable in the type.
v2's approach is stronger where it applies, and does not apply to OSCAR's problem — OSCAR's channels are
dynamic per device, which is why it gates at runtime.

**Import idempotence (T5).** OSCAR skips an already-present night and counts it in `skippedExisting`,
with a deterministic sleep-source chooser that **aborts before writing anything** if the user dismisses
the ambiguity dialog. v2 deduplicates under `(source, type, session_id, seq)` with `ON CONFLICT DO
NOTHING`, where a reconnect mints a *new* `session_id` so there is no false dedup across a discontinuity.
OSCAR's "same night" is a date plus source choice; v2's is a session identity. v2's can distinguish two
separate recording sessions on one night; OSCAR's per-night skip could not.

### 7.2 Invalid-data and missing-data handling — the centrepiece

**OSCAR, path by path:**

*Path A — ResMed EDF, `resmed_loader.cpp:4360–4368`:*
```cpp
if ((c == -1 || (isPressure && c == 0)) && t_min>=0 ) {
    c=last; //reset the current sample to the last one to skip over using it.
    // this change stops false error values from deing displayed.
}
```
The sentinel is named in a comment and then written over with a real value. Downstream, the fabricated
sample is indistinguishable from a measurement, and nothing records how many were fabricated.

*Path B — Apple Health, `applehealthDataParsing.cpp:337–446`:*
```cpp
// 0 and negatives are no-reading sentinels, not measurements.
if (!qIsFinite(value) || value <= 0.0f || value > 100.0f) {
    warnMalformed(QStringLiteral("skipping OxygenSaturation with out-of-range value %1")...);
    return;   // dropped, not adjusted
}
```
Plus parse-time unit conversion (`degF`/`K`→`degC`, `lb`/`st`/`g`/`oz`→`kg`), rejection of unknown units
by name, and per-sample source attribution. Verified: the guard names its blocker and the value that
blocked it — *"guards must say why"*, implemented. The gap is that the count is local, capped at ten
stderr lines, and absent from `AppleHealthImportSummary`.

*Path C — the summary store:*
`daily_summaries`/`session_summaries` hold **`0.0`** for SpO2/pulse when no oximetry exists, and the
report masks it with `ROUND(NULLIF(ss.spo2_avg, 0), 1)`. The vendor's own words: *"This is display
suppression, not a calculation; 0 never occurs as a real SpO2/pulse reading."* OSCAR gets the per-column
discrimination right — `Leak_Unint_Avg` is deliberately **not** masked, because 0 is legitimate there —
but encodes absence as a value at every layer and repairs it only at the point of display.

**v2, for comparison:**

```rust
// h10_build.rs — the implemented path
built.rejected.push(format!(
    "hr={bpm} outside registry range {hr_lo}..{hr_hi} bpm: \
     rejected, never clamped"
));
// bp_build.rs — absent is not invalid
assert!(b.rejected.is_empty(), "absent is not rejected");
```
And the storage rule: never `locf()` or `interpolate()`; `time_bucket_gapfill` so gaps come back as
explicit NULLs.

**The honest scorecard:**

| Property | OSCAR | v2 |
|---|---|---|
| Invalid value never becomes a number | **No** in path A, yes in B, **no** in C | **Yes** — "rejected, never clamped", tested |
| Rejection names its reason and its value | Yes in B (transient) | **Yes** — value and range named, uncapped |
| Absent distinguishable from measured | Yes in B; **no** in C (stored `0.0`) | **Yes** by construction (NULL gap-fill) |
| Absent distinguished from *invalid* | **No** — path C conflates; path B treats 0 as the sentinel | **Yes, explicitly and tested** |
| Rejection record is durable | **No** — stderr, first 10 only | **No today** — returned as named reasons, logged to stderr by the collector |
| The rejected bytes are recoverable | No | **Yes for the BP path** — a raw-frame journal is written before decode, so a decoder bug is replayable |

So v2's *rules* and its *implemented rejection behaviour* are better. v2 is **not** yet better on
durability of the rejection record: rejections are `eprintln!`-ed, exactly as OSCAR's are `qWarning`-ed.
v2 makes the record durable only in the `cpap-ezshare-ingest` attempt-ledger design (invariant 7:
"Success, failure, timeout, no-op replay, and **rejected rows are distinguishable**") — which is **0 of 6
tasks built**. Until then, both projects log and move on, and OSCAR has the excuse of 2013-era Qt while
v2 does not.

### 7.3 Time (O7)

OSCAR's `device_time_corrections` table is the best-designed artefact in its tree and is worth stating in
full, because v2 has independently arrived at the same principle:

```sql
type TEXT NOT NULL CHECK(type IN ('timezone','travel','dst','reset','offset','drift')),
offset_ms INTEGER, c0_ms INTEGER, c1 REAL DEFAULT 0.0,
reason TEXT, applied_at TEXT NOT NULL DEFAULT (datetime('now')), undone_at TEXT
```

Enumerated-and-constrained reason; instantaneous offset *or* linear drift model (`c0_ms` intercept,
`c1` rate); date-range and machine scoping; `applied_at`/`undone_at` lifecycle. Nothing rewrites stored
timestamps.

v2's `cpap-ezshare-ingest` design invariant 6: *"Clock correction is metadata. Store raw device time and
a versioned transform plus derivation evidence. Apply the transform at read time; changing it appends a
version and never updates samples."* This is **better** — versioned and append-only, versus OSCAR's
mutable ledger with an `undone_at` column — and it is **specified, not built** (0/6).

**The concrete failure v2 would meet without it**, since the BLE path is the one that ships first: the
AirSense 11's clock drifts, and v2 timestamps samples from the collector's clock at arrive-time while the
device's own session boundaries come from the device. Over months the two diverge, and there is today no
v2 mechanism that says "this session's device clock was 40 minutes behind". OSCAR's `drift` type with an
intercept and rate is the answer to exactly that, and it is implemented. This is recorded as a **live
gap in v2 relative to OSCAR**, not as a recommendation to copy the schema.

### 7.4 Refusing to print what cannot be measured (O11 part)

Three OSCAR decisions, all matching this project's "unknown is not OK" rule:

1. `central_count` → **`unclassified_count`** in schema v10, with existing data migrated, because "most
   CPAP machines cannot distinguish between true Central Apneas and other unclassified events". Only the
   claim about meaning changed; the arithmetic did not.
2. `OHI=`/`CHI=` printed "only when the day actually has OH or CH data, rather than printing `0.00` for
   every other device".
3. `oahiChannels = ahiChannels − cahiChannels` so "the identity holds by construction".

And the inverse discipline: schema v15 **dropped `dst_enabled`** from `user_info` because it was "Stored
but never read by application" — dead state removed rather than left to imply a capability.

**Process (O11).** OSCAR has 473 test assertions, `-Werror`, AddressSanitizer on Linux/macOS, a published
schema philosophy, and a 6,748-line dated bug log with root causes — and **no CI at all**. The tests run
only when a developer types `CONFIG+=test`. v2 has CI, pre-push gates, `Verify:` contract lines that
cannot be silently edited, and an amendment index that supersedes stale briefs.

Which process catches what: v2's `Verify:`-line discipline and amendment index are designed to catch
*stale intent* — a task brief describing a superseded design. OSCAR's v7 weighted-average defect was a
silently wrong derived value, which a CI-run regression test would have caught and which nothing in
OSCAR's process could have. OSCAR's two 2026-10 silent-data-loss bugs (journal notes and ResMed settings
"lost at the next launch", both because data was marked changed and nothing ever saved it) were found by
a manual sweep, not by a test — and a CI-run test would have caught those too. Conversely, the amendment
index addresses a failure mode OSCAR has no mechanism for and v2's own history has already suffered.
Neither process is a superset. Neither project has an excuse for the gap the other fills: OSCAR has the
tests and no runner; v2 has the runner.

---

## 8. Local, cloud, privacy and operating cost

| Axis | OSCAR | v1 | v2 |
|---|---|---|---|
| Where data lives | Local SQLite file | Device + optional server | Pi, self-hosted |
| Account required | **No** | No | Enrollment, operator approval |
| Telemetry | **None — verified across the tree** | — | None |
| Network code | User-owned Dropbox/Google Drive/OneDrive via OAuth2 only | Optional sync | Private LAN; HTTPS for public use |
| Cloud-hosted by vendor | No | No | No |
| Encryption at rest | **No** — plain SQLite; profile passwords removed in 2.0 | — | LUKS (v2 design) |
| Vendor-authored research | — | v1 documented | — |
| Cost | **Zero. Verified: "never asks for payment"** | Zero | Hardware + electricity |
| Uninstall/exit | Take the SQLite file | — | — |

**On "verified no telemetry".** This is a genuinely strong position, and it is strong in the way that
matters: it is *checkable*. A search for telemetry identifiers across the whole tree returns nothing, and
the only network code targets storage the user owns. But the guarantee is only as good as the licence:
because the code is GPLv3, the absence is verifiable by anyone and cannot be quietly reversed without
someone noticing. A closed-source app can make the same claim and no one can check it — which is exactly
why Exist's "no analytics" assertion was recorded as a *claim* in candidate 3's package and OSCAR's
absence is recorded as a *finding* here.

**On "free".** OSCAR's free claim is unqualified and verifiable: no account, no tier, no upsell, no
feature gate, no telemetry to monetise, and a licence that forbids closing it. It is the least ambiguous
"free" claim on the entire shortlist, and it is the standard against which the others' pricing was judged.

---

## 9. What cannot be concluded

### From the inspection

1. **No runtime observation of OSCAR at all.** No binary was executed, no data imported, no report
   generated, no CSV inspected as output. Every OSCAR finding is a static reading or a vendor statement.
2. **Which header 2.0.1 emits was not confirmed by running it.** The latent-break timing (§3.2) is an
   **inference** from dates and release notes.
3. **v1's detector was not executed against the 27-column header.** The prefix match is a string
   comparison.
4. **The `0.0` oximetry sentinel was not traced through a real v1 import.**
5. **Only three invalid-data paths were examined.** The three-policy finding is about the ResMed EDF,
   Apple Health and summary-store paths — the three that touch v1's data. It is not a claim about OSCAR
   as a whole.
6. **The leak defects may be fixed.** The review is dated 2026-04-16. The *method* was confirmed in the
   current tree; the specific gain-handling and over-accumulation defects were not re-confirmed.
7. **The localised-header fragility was read, not reproduced.** No localised build was run. It affects
   the dialog exporter; the ORF report's aliases are fixed identifiers.
8. **The 1.7.2 line was not inspected**, and the five unmigrated loaders (`mseries`, `dreem`,
   `somnopose`, `zeo`, `serialoximeter`) were not confirmed as active in 2.0.
9. **No user, install or download figures exist.** Star and fork counts are not user counts.
10. **The 2.0 line is a short, single-namespace fork.** Whether the wider OSCAR Team reviews it was not
    determined.

### Left open by the candidate-specific questions

- **O3 (leak scope):** which of v1's leak values arrive via OSCAR versus an EDF path was not fully
  traced. v1 supports an EDF leak-parsing path as well as OSCAR CSV, so the §3.3 qualification should be
  applied to the OSCAR-imported values specifically, not to every leak number in v1. **This narrows the
  finding and the narrowing is not fully quantified here.**
- **O5:** whether any *v2* path holds a value over the way `resmed_loader.cpp` does was checked in the
  H10, BP and AirSense registry paths and not found; the other collectors were not audited for it.
- **O6:** whether "quarantine" as the rules state it is implementable in v2's store was not resolved.
  What is established: v2 returns named rejections and logs them; it does not persist them.
- **O9 (OSCAR as a v2 output target):** assessed as **feasible in principle and already bounded by
  policy** — v2's `cpap-ezshare-ingest` design states the clean-room position explicitly ("OSCAR
  documentation/file formats may inform tests; no GPL implementation code is copied. Record attribution
  for every runtime library"). Emitting a CSV that OSCAR can read would not create a GPL obligation the
  way linking would, but this was not legally reviewed and no recommendation is made.
- **O12 (closing the gaps):** the smallest evidence set would be (a) compiling OSCAR at the pinned
  commit and running the Apple Health suite, (b) confirming the 2.0.1 binary's by-session header against
  a real export, and (c) running v1's importer against a 27-column fixture to confirm the misassignment
  empirically rather than by reading. **None of these was executed.**

---

## 10. Balanced conclusion

### The case that OSCAR is not a competitor

It is a desktop review application that reads a CPAP card after the fact. It has no wireless ingestion, no
real-time path, no alarm, no phone, no mobility, and no continuous collection — and it does not claim any
of them. Placing it beside AnxietyWatch2 as a rival would misdescribe both products. The honest
classification is **dependency plus reference**.

### The case that it belongs in the comparison anyway — and this is the stronger case

1. **v1's CPAP numbers come from it.** Not as an influence, but as the actual source of the CSV
   vocabulary, the noon-to-noon day boundary, the unscored-hours rule and the event-scoring semantics. A
   comparison report that omits OSCAR omits the upstream of v1's CPAP path.
2. **The upstream has changed, and v1 cannot detect it.** This is a concrete, verified, forward-looking
   risk to v1's data, and it is the kind of finding a comparison exists to surface. It is not a
   hypothetical: the widened header is in the inspected source today.
3. **It is the only implementation on the shortlist.** For Bearable, Welltory and Exist the packages had
   to close on "not performed" or "partially performed". OSCAR's source answers the questions the others
   could not, and the answers are specific.
4. **It shows the discipline this project mandates is achievable without a commercial incentive.** The
   Apple Health loader does exactly what `AGENTS.md` requires — reject, name the reason, never coerce a
   sentinel, convert units once at the parser — in a volunteer GPL project. That is worth more as
   evidence than any vendor's assurance.
5. **It shows the discipline failing in the same repository.** `c = last`, the `0.0` sentinel in the
   store, and 27 loaders whose invalid-data policy was never audited. The contrast between the 2026 path
   and the 2013 path, in one diffable tree, is the most useful thing in this package.

### Where OSCAR is ahead of both AnxietyWatch versions

- **Exit and interoperability.** Published schema, Python reader, three CSV modes. No equivalent in
  either AnxietyWatch version.
- **Per-session fidelity markers.** `summary_only` is implemented; v2 has no per-session equivalent.
- **Device clock correction.** A typed, reasoned, drift-modelled, reversible ledger, implemented. v2's
  better-designed rule is specified and unbuilt.
- **Breadth of device support.** ~19 loaders. Neither AnxietyWatch version is close.
- **Unambiguous free.** No tier, no upsell, no telemetry to monetise, a licence that keeps it that way.

### Where v2 is ahead

- **The store cannot fabricate a value** — samples are append-only, `seq` identity makes loss measurable,
  and gaps return as explicit NULLs.
- **Rejection names its blocker and its value, uncapped, and absent is explicitly not invalid** — with a
  test asserting it.
- **The raw bytes survive decode** for the journaled paths, so a decoder bug is replayable.
- **The instrument is connected to the person.** Real-time and panic-time, which is the whole difference
  between reviewing a night and responding to one.
- **CI and `Verify:` contracts**, which OSCAR lacks entirely.

### The one thing to carry forward

If a single finding from this package is acted on, it should be **§3.2**. v1's by-session importer will
silently corrupt every field of a widened OSCAR export, and the change that triggers it is already
committed upstream. The fix is a schema check rather than a length check — verify the header's tokens,
not the row's width — and it is a small, bounded, reversible change. This report identifies the defect
and the mechanism; **it does not make the change, and no change was made.**

---

## Related files

- `oscar_CLAIMED_FEATURES_AND_PURPOSE.md` — vendor claims, device coverage, evidence table O-1…O-50h
- `oscar_IMPLEMENTATION_INSPECTION.md` — the full-source inspection and claim reconciliation X1–X29
- `PROMPT_2.md` — the candidate-specific second-stage prompt this report answers
- `../PROCESS_STATE.md` — sequencing state and residual uncertainty for this candidate
- `../RESEARCH_METHOD.md` §6.1 — the inspection-mode taxonomy
