# OSCAR — implementation inspection

**Slug:** `oscar`
**Inspection date:** 2026-10-06
**Phase:** Phase 2A (authorized implementation inspection)
**Outcome:** **PERFORMED** — the shipped application's complete source was obtained and read from a
public GPLv3 repository. This is the first candidate in this research where the actual product
implementation could be inspected.

---

## 1. Summary of outcome

| Target | Inspected? | Basis |
|---|---|---|
| Source of the released version 2.0.1 (`CrimsonNape/oscar-sql`, branch `master`) | **Yes** | Public GPLv3, no authentication, no DRM, cloned at a pinned commit |
| Source of the 1.x line (`pholy/OSCAR-code`) | **No** | Metadata only; 2.0 is the current release and the data model under comparison |
| Published design and schema documentation (`Notes/`) | **Yes** | In the cloned tree |
| The compiled binaries (`.exe`, `.dmg`, `.deb`) | **No** | Not needed; the source is the authority and the binaries add nothing |
| Runtime behaviour | **No** | OSCAR was not installed or executed (see §6) |

**Why this is a step change for the research.** Candidates 1–3 closed with "not performed" or "partially
performed", and each package had to argue from vendor claims, store metadata, or a published API
contract. OSCAR's implementation is fully readable, so the questions that stayed open for Bearable,
Welltory and Exist — how is an invalid value handled, is missing data distinguishable from a
measurement, is provenance recorded — are here answered from code.

**Two findings matter more than the rest.**

First, **v1's CPAP importer is coupled to OSCAR's report vocabulary by column *position*, and OSCAR's
report header has already grown.** v1's by-session parser hardcodes indices 0–20 behind a `count >= 21`
guard, and its format detector is a six-token prefix that a widened header still satisfies. The header
v1 was written against — 22 columns, matching OSCAR 2.0.1 exactly — has become 27 columns in the
inspected commit, with five new columns inserted mid-header. Against the widened header the guard passes,
the detector fires, and every field from index 4 onward is silently misassigned, including reading
`Leak_95th` as `Hours_Used`. See §5.1.

Second, **OSCAR answers the invalid-data question three different ways in one program**, and the answers
disagree with each other: hold the previous sample (ResMed EDF), reject by name and drop (Apple Health),
or store `0.0` as the sentinel for absence and hide it at export (the summary store). The newest answer
agrees with this project's rules; the oldest violates them; the store's answer violates them in a way
that is invisible to anything reading the published database. See §5.3.

The second finding is the more instructive one and the first is the more consequential. Both are
reported with their evidence class attached, and neither was observed at runtime.

---

## 2. Authorization basis

| Requirement | Status |
|---|---|
| Lawful to obtain | **Yes** — GPLv3 source, publicly hosted, no authentication |
| Circumvents a technological protection measure | **No** — `git clone` over public HTTPS |
| Requires defeating DRM, pinning, or anti-debugging | **No** |
| Requires a credential or account | **No** |
| Reads private or personal data | **No** — source code only, no user data touched |
| Terms-of-service conflict | **None identified.** GPLv3 explicitly grants the right to read, modify and redistribute |
| Owner decision required | **No** — the licence grants the access |

The licence is not incidental to the inspection: it is *why* the inspection is possible. GPLv3 requires
source availability, which is the property that makes OSCAR the only candidate on this shortlist whose
claimed-versus-actual behaviour can be checked without the owner authorising a reverse-engineering
decision. That is worth stating in the comparison in its own right.

---

## 3. Artifact record — inspected

| Item | Value |
|---|---|
| Repository | `https://gitlab.com/CrimsonNape/oscar-sql.git` |
| Branch | `master` |
| Commit inspected | `28df2e46e35e6260e186fced997edadcf001cc24` |
| Commit date | 2026-10-04 |
| Commit subject | "Save imported journal entries and remove legacy summary-file code" |
| Clone depth | 1 (shallow, single commit) |
| Working-tree size | 59 MB |
| Licence | GNU GPL v3 (`COPYING`), confirmed in the tree |
| Repo signals | 10 ★, 13 forks, created 2026-06-07, last activity 2026-10-04 (GitLab API) |
| The 1.x repo (metadata only) | `gitlab.com/pholy/OSCAR-code`, 189 ★, 112 forks, created 2019-01-23, last activity 2026-01-31 |

Relevant top-level directories in the tree:

| Path | Contents |
|---|---|
| `oscar/SleepLib/` | Data model, session/event/day/machine, and `loader_plugins/` (30 files) |
| `oscar/database/` | SQL schema and ~20 repositories |
| `oscar/exports/` | CSV and report exporters |
| `oscar/network/` | OAuth2 and the Dropbox/Google Drive/OneDrive uploaders |
| `oscar/tests/` | 14 test files using Qt Test |
| `Notes/` | Published design, schema and engineering documentation |
| `Building/` | Per-platform build instructions |

---

## 4. Static findings — architecture

### 4.1 The data model is a normalised SQL schema, and the waveforms are blobs

`oscar/database/database_schema.cpp` is 1,956 lines and creates ~20 tables. The load-bearing ones:

| Table | Role | Noteworthy columns |
|---|---|---|
| `sessions` | One CPAP session | `start_time`, `end_time`, `duration`, `enabled`, **`summary_only`**, `no_settings`, `events_loaded`, `UNIQUE(machine_id, session_id)` |
| `session_settings` | Machine configuration per session | plus `json_value TEXT` (schema v9) for structured data |
| `session_channels` | Channel metadata per session | |
| `session_channel_values` | Value/time summary for a channel | `value INTEGER NOT NULL`, `count`, `time_ms`, `UNIQUE(session_channel_id, value)` |
| `respiratory_events` | Individual apnoea/hypopnoea/RERA events | `event_type`, `start_time`, `end_time`, `duration`, **`desaturation REAL`**, `severity` |
| `session_summaries` / `daily_summaries` | Materialised statistics caches | 35 statistics per day; invalidation via `sessions_hash` |
| `event_lists` / `event_data` | Waveform and event series | `data_blob`, `data_compressed`, `compression_method`, **`checksum`** |
| `channels` / `channel_options` | Channel dictionary | |
| `device_time_corrections` | The clock-correction ledger | see §4.5 |
| `app_preferences`, `graph_layouts`, `profile_preferences` | Settings, migrated out of XML/`.shg` files |
| `report_tree` | Self-referencing report hierarchy | system vs user nodes |

Three observations:

- **`summary_only` is a first-class fidelity marker.** A session imported from a card that carries only
  summary data is flagged, so the absence of a waveform is recorded rather than implied. This is the same
  discipline as this project's "not measured" state, and it exists precisely because some supported
  devices (the BMC G3X cards, per the release notes) supply summary data only.
- **Waveforms are not rows.** They are compressed BLOBs with a CRC16 checksum in the header. The
  inspected code compresses at **level 6**, and only when the data is at least 500 bytes and compression
  saves more than 10% (`event_data_repository.cpp:526–540`, rationale: "6 is ~2x faster than 9 with only
  ~1-2% larger output"). The vendor's schema document still states "qCompress level 9 achieves 40-60%
  compression" (`SCHEMA_DESIGN_PHILOSOPHY.md:66`), so **the published design record and the shipped code
  disagree on the compression level** — a small discrepancy, and a concrete instance of documentation
  drift in the same repository as the header drift in §5.1. OSCAR is a *dense waveform archive*;
  AnxietyWatch is a sparse scalar store. They are not the same kind of database and should not be compared
  as if they were.
- **Derived statistics are cached, with an invalidation key.** `daily_summaries` and `session_summaries`
  are materialised views keyed by `sessions_hash`. This is a performance decision with a correctness
  hazard, and the hazard fired: schema v7 exists solely because value/time summaries were not persisted,
  so "weighted averages defaulted to simple averages, causing significant inaccuracies in pressure
  statistics and other metrics". A cached statistic that is silently computed the wrong way is exactly
  the failure this project's rules target.

### 4.2 There is no wireless stack and no continuous collection

A search across all C++ and project files for `QLowEnergy`, `QBluetooth`, `bluetooth`, `gatt` returns no
Bluetooth implementation. The only near-hit is a user-facing string in `oscar/oximeterimport.cpp`:

> OSCAR is currently compatible with Contec CMS50D+, CMS50E, CMS50F and CMS50I serial oximeters.
> (Note: Direct importing from bluetooth models is **probably not** possible yet)

The "live streaming mode" strings in `cms50_loader.cpp` and `md300w1_loader.cpp` switch a *device*
attached by cable into a transfer mode so its stored buffer can be read. They are not real-time
collection.

**Consequence for the comparison:** the difference between OSCAR and AnxietyWatch2 is not one of degree.
OSCAR has no path by which a value can reach the analysis layer while the night is in progress. V2's
whole reason to exist — a supervised alarm, a dead-man switch, overnight monitoring — is unbuildable on
OSCAR's architecture without adding an ingestion path that does not exist.

### 4.3 No telemetry, and the only network code points at the user's own storage

A search for `telemetry`, `analytics`, `google-analytics`, `sentry`, `crashreport`, `mixpanel`,
`amplitude` and `firebase` across the tree produced one match, and it is prose inside a loader comment
about an unrelated reconstruction ("the flow amplitude and the leak stayed…"). There is no crash
reporter, no usage analytics, no update check.

`oscar/network/` contains `oauth2_handler`, `dropbox_uploader`, `googledrive_uploader`,
`onedrive_uploader`, and generic `cloud_uploader`/`cloud_downloader`. Every one is user-initiated and
targets a storage account the user owns. There is no OSCAR endpoint to send anything to.

This is a genuinely strong privacy position and it is verifiable rather than asserted — the strongest
form of privacy claim available.

### 4.4 Profiles are not encrypted; crypto exists for a device, not for the user

Schema v14 moved preferences into the database, and 2.0's release notes state "Remove password support".
The database is a plain SQLite file; at-rest protection is whatever the OS and disk provide.

`SleepLib/crypto.cpp` implements AES-256 and AES-256-GCM through bundled Botan. Its only callers outside
tests are in `prs1_loader.cpp:371–425`, where it decrypts the Philips Respironics SD-card payload using
keys embedded in the source (`s_oscar_key`, `s_common_key`, `salted_key`). So the crypto is for reading
the user's own device, not for protecting the user's data.

Recorded as a static observation only. The propriety of carrying vendor decryption keys is a question
this research is not competent to decide and does not take a position on.

### 4.5 Device clock problems get a typed, reasoned, reversible ledger

This is the best-designed thing in the tree and the finding most worth carrying into AnxietyWatch2.

`database_schema.cpp:1519–1533` creates:

```sql
CREATE TABLE IF NOT EXISTS device_time_corrections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    machine_id  INTEGER NOT NULL REFERENCES machines(id) ON DELETE CASCADE,
    date_from   TEXT    NOT NULL,
    date_to     TEXT,
    type        TEXT    NOT NULL CHECK(type IN ('timezone','travel','dst','reset','offset','drift')),
    offset_ms   INTEGER,
    c0_ms       INTEGER,
    c1          REAL DEFAULT 0.0,
    reason      TEXT,
    applied_at  TEXT NOT NULL DEFAULT (datetime('now')),
    undone_at   TEXT
)
```

Four properties, each deliberate:

1. **The reason is enumerated and constrained.** `timezone`, `travel`, `dst`, `reset`, `offset`, `drift` —
   a CHECK clause means an unnamed correction cannot be written. This is the same instinct as "guards
   must say why", enforced by the schema.
2. **Drift is modelled as a model, not a number.** `c0_ms` (intercept) and `c1` (rate) describe a linear
   drift, so a slowly-wrong device clock is corrected continuously rather than by one blunt offset.
   A single `offset_ms` is available for the instantaneous cases.
3. **Corrections are reversible and audited.** `applied_at` and `undone_at` make the correction a
   recorded event with a lifecycle. Nothing rewrites the underlying timestamps destructively.
4. **The correction is scoped to a date range and a machine**, so a user who travels with a CPAP and
   changes timezone mid-record can say exactly which nights were affected.

AnxietyWatch2 stores UTC epoch seconds and nothing else, which is the right canonical form — but this
table addresses a problem the UTC rule does not solve by itself: **what to do when the device that
produced the timestamp was wrong.** V2's AirSense 11 pipeline will meet the same problem. This is
recorded as a design precedent worth considering, not as a recommendation to adopt it.

### 4.6 Tests exist in depth, and nothing runs them automatically

`oscar/tests/` holds 14 Qt Test files. Assertion counts (`QCOMPARE`/`QVERIFY`/`QVERIFY2`):

| File | Assertions |
|---|---|
| `applehealthtests.cpp` | **340** |
| `apextests.cpp` | 117 |
| `eventstabtests.cpp` | 8 |
| `machinetests.cpp` | 8 |
| `resmedtests.cpp`, `prs1tests.cpp`, `viatomtests.cpp`, `sessiontests.cpp`, `versiontests.cpp`, `rawdatatests.cpp`, `cryptotests.cpp`, `deviceconnectiontests.cpp`, `dreemtests.cpp`, `zeotests.cpp` | 0 matched by this pattern (suite scaffolding, or assertions via other macros) |
| **Total matched** | **473** |

Because `applehealthtests.cpp` alone carries 340 of them, the Apple Health path — the newest, and the
one that shares a data source with AnxietyWatch v1 — is the most heavily tested path in the program.

The test build is also serious about correctness: `oscar.pro:800–828` compiles tests with `-Werror` and,
on Linux and macOS, **AddressSanitizer** (`-fsanitize=address`).

And then: **there is no CI configuration anywhere in the repository.** No `.gitlab-ci.yml`, no
`.github/`, no `Jenkinsfile`, no `*.yml` of any kind. The test suite runs only when a developer types
`CONFIG+=test` into a qmake command. The infrastructure is high-quality; the enforcement is absent. For a
project whose correctness matters to people adjusting therapy, that is the most consequential process
finding in this inspection.

### 4.7 Destructive data management is a design choice, made explicitly

Two mechanisms remove data:

- **"Purge Range of Days"** (approved 2026-05-16): deletes a contiguous date range, per data type
  (six radio buttons keyed to `MachineType`), with the same granularity as the existing single-day purge.
- **The no-migration policy** (schema v12): "Schema version mismatch now requires a fresh database and
  data reimport. Incremental migrations are no longer supported." The stated rationale is that this
  "ensures data integrity — no partial or broken upgrade states."

The second is the more interesting one. It is a deliberate choice to fail closed rather than half-migrate,
which is defensible — but the cost is that an OSCAR upgrade can invalidate the user's accumulated record,
with the SD card as the only backup. For a tool whose purpose is long-term review, and whose users may
have years of data, that is a real durability exposure. AnxietyWatch's append-only rule exists to make
this class of loss impossible; OSCAR chose the opposite trade and documented it honestly.

Note the honest counterweight: 2.0 also added multiple databases, a startup integrity check, a guard
preventing two instances from opening the same database, and transactional storage of waveforms that
previously lived as thousands of loose `.001` files. On balance the 2.0 rewrite *decreased* the risk of
data loss relative to the file-based design, while the no-migration policy increases it across versions.

---

## 5. Static findings — the reconciliation points

### 5.1 v1's CSV importer is coupled to OSCAR's own report vocabulary — by position

This is the finding that makes OSCAR a dependency rather than a competitor. It is verified from both
trees, and it produced the most consequential result in this inspection.

**OSCAR has two CSV producers, and v1 consumes one format from each.**

| OSCAR producer | Output | v1 detector |
|---|---|---|
| The legacy dialog exporter (`oscar/exports/exportcsv.cpp`) | Details / Summary / Sessions | `isOSCARFormat()` — the daily path |
| The database-driven system report (`oscar/docs/system_reports.orf`) | `Daily Summaries/by Day`, `Session Summaries/by Session` | `isOSCARBySessionFormat()` — the by-session path |

They are different code paths with different vocabularies, and the distinction matters because only one
of them has been widened since the version v1 was written against.

#### 5.1.1 The daily path: a literal match on the dialog exporter

`exportcsv.cpp:190–217` builds the Summary header as

```cpp
header = tr("Date") + sep + tr("Session Count") + sep + tr("Start") + sep + tr("End") + sep +
         tr("Total Time") + sep + tr("AHI");
```

with `const QString sep = ",";` at line 160. v1's `isOSCARFormat()` is
`hasPrefix("date,session count,start,end,total time,ahi")` — the same six tokens, in the same order.

#### 5.1.2 The by-session path: an exact 22-column match, verified against the vendor's design record

v1's by-session detector is `hasPrefix("date,start,ahi,rdi,oa")`, and its own comment states the
contract: *"OSCAR 'by session' export: one row per mask-on/mask-off session, 22 columns"*
(`CPAPImporter.swift:291`).

OSCAR's `Session Summaries/by Session` report began 2.0.1 as exactly those 22 columns. The vendor's own
design record lists them — `Notes/specs/2026-06-10-csv-report-fields-design.md`, the approved design for
the release note *"[update] Add more fields to Session/by Session and Daily/by Day CSV reports"*:

> `Date, Start, AHI, RDI, OA, UA, H, CA, RERA, Pressure_Avg, Pressure_Min, Pressure_Max,
> Pressure_95th, Leak_Avg, Leak_Max, Leak_95th, SpO2_Avg, SpO2_Min, Pulse_Avg, Hours, Hours_Used,
> Machine`

That is v1's expected header verbatim, token for token, in order. The correspondence is not merely
exact — it is **vintage-matched**: v1's importer was written against the report OSCAR 2.0.1 shipped.

#### 5.1.3 The finding: the header grew, and v1 parses by position with a length-only guard

v1's `parseBySessionRow` receives the split fields and **no header**, so every mapping is a hardcoded
index (`CPAPImporter.swift:709–765`):

```swift
ahi: dbl(2, "AHI"),  rdi: dbl(3, "RDI"),
oa: int(4, "OA"),    ua: int(5, "UA"),  h: int(6, "H"),  ca: int(7, "CA"),
rera: int(8, "RERA"),
pressureAvg: dbl(9, "Pressure_Avg"),   pressureMin: dbl(10, "Pressure_Min"),
pressureMax: dbl(11, "Pressure_Max"),  pressure95th: dbl(12, "Pressure_95th"),
leakAvg: dbl(13, "Leak_Avg"),  leakMax: dbl(14, "Leak_Max"),  leak95th: dbl(15, "Leak_95th"),
spo2Avg: dbl(16, "SpO2_Avg"),  spo2Min: dbl(17, "SpO2_Min"),  pulseAvg: dbl(18, "Pulse_Avg"),
hoursUsed: Double(fields[20])
```

The only structural guard is a row-length test:

```swift
guard fields.count >= 21 else { return .skip(reason: "expected at least 21 columns, found \(fields.count)") }
```

`21` is the 22-column layout's minimum. It is a length check, not a schema check.

**Meanwhile, in the inspected commit, the by-Session report has 27 columns.**
`oscar/docs/system_reports.orf:145` opens the report and its `SELECT` aliases run `:153–179`; token 5
onward now reads, before `OA`:

```sql
ROUND(ss.oahi, 2) as OAHI,
ROUND(ss.cahi, 2) as CAHI,
ss.obstructive_count as OA,
ss.unclassified_count as UA,
ss.all_apnea_count as A,
ss.hypopnea_count as H,
ss.obstructive_hypopnea_count as OH,
ss.central_hypopnea_count as CH,
ss.clear_airway_count as CA,
ss.rera_count as RERA,
```

Five columns — `OAHI`, `CAHI`, `A`, `OH`, `CH` — were inserted mid-header by the central/obstructive
hypopnea work specified in `Notes/specs/2026-08-06-central-obstructive-hypopnea-design.md`.

**Two things follow, and they point in opposite directions.**

*The detector still fires.* `hasPrefix("date,start,ahi,rdi,oa")` is matched by
`date,start,ahi,rdi,oahi,cahi,...` — because `OAHI` begins with `OA`. The prefix check passes on the
widened header by coincidence of naming, not by design.

*The parser then silently misassigns everything.* With the 27-column layout, v1 reads:

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

Index 20 is the worst of them. `guard ... hoursUsed >= 0` passes for a leak value, and
`hoursUsed.isFinite` passes, so a row whose real usage was 6.5 h and whose leak 95th was 12.4 is stored
as **12.4 hours of therapy**. No warning is raised. Nothing is skipped. The row count guard is satisfied
because `27 >= 21`.

**Severity, stated precisely.** v1 does *not* corrupt data against OSCAR **2.0.1**, which is the release
its by-session importer was written against and whose header matches exactly (§5.1.2). The failure
arrives with the **next OSCAR release**, which will ship the 27-column report. The reason no version
guard exists is that v1 identifies the format by a prefix and validates it by row length — neither of
which can detect a widened header.

**Labelled as *inference*, not observation:** the conclusion that the 27-column form postdates 2.0.1
rests on three things — the specification that introduced it is dated 2026-08-06, after the 2026-06-23
release; the report file's own header note (`system_reports.orf:14`) states that these columns "arrived
with schema v18" while `database_schema.h` sets `CURRENT_SCHEMA_VERSION = 19`; and the 2.0.1 release
notes mention no central/obstructive hypopnea work. The 2.0.1 binary was not run to confirm which header
it emits. The latent-break conclusion holds on either branch; only the timing changes.

**One narrowing factor.** The correspondence is with OSCAR's *report*, not its database. A user who
never runs the by-session report never produces the file. The exposure is real but requires the user to
have exported it.

#### 5.1.4 Two further static caveats

1. **The dialog exporter's header strings are localised.** Every token in `exportcsv.cpp` is wrapped in
   `tr(...)`, so a French or German OSCAR writes a different header and v1's literal prefix will not
   fire. The separator is *not* localised (`sep = ","` is a literal), so the failure mode is header
   detection, not field splitting. The ORF report's aliases (`as AHI`, `as OA`) are fixed identifiers,
   so the by-session path is not exposed to this. Recorded as a static reading; no localised build was
   run.
2. **Days without CPAP data produce no row at all.** `exportcsv.cpp:230`:
   `Day *day = p_profile->GetDay(date, MT_CPAP);  // Only export days with CPAP data.` A day the CPAP
   did not record is absent from the CSV rather than present with zeros. This is the correct behaviour
   and consistent with v1's own treatment of unscored hours (`CPAPBySessionImportTests.swift:162`).

#### 5.1.5 What the correspondence proves, and what it does not

It proves that v1's CPAP semantics — which columns exist, what they are called, and the noon-to-noon day
boundary (`date(s.start_time/1000 - 43200, 'unixepoch', 'localtime')` in the same report) — were derived
from OSCAR's **report output vocabulary**, not independently. That is a genuine dependency and it is the
reason an upstream header change is v1's problem rather than OSCAR's.

It does not prove the numbers agree. Two implementations of AHI, RERA and leak can share a column name
and disagree about what goes in it. §5.2 and §5.3 address that separately.

### 5.2 Leak percentile: OSCAR's number is not the device's number

`Notes/Developer Notes/LEAK_PERCENTILE_METHOD_REVIEW_2026-04-16.md` is an internal method review with
reproduced numbers. Its findings, in its own order:

**Reproduced facts.** For a tester's dataset, day 2026-04-02: the device's `STR` record reports
`Leak.95` raw = `1` at gain `0.02 L/s`, i.e. **1.2 L/min**. OSCAR's PLD-derived p95 for the same night is
**2.4 L/min**. Ratio **2.0**. A second night in the same profile agrees exactly (6.0 L/min both ways), so
the discrepancy is not a uniform scaling error.

**Cause, established from code.** The device-reported `leak50`/`leak95` values *are* parsed
(`resmed_loader.cpp:1969–1980`, in the inspected commit) but are not persisted — `StoreSummaryStatistics()`
stores max only, and the p50/p95 lines are commented out (`resmed_loader.cpp:2905–2909`, the two stores at
`:2906–2907`). OSCAR therefore computes leak percentiles from the PLD time-series instead of reporting the
device's. The review's own assessment:

> This is a product-behavior mismatch, not necessarily a pure math bug.

**Line-number note, because it bears on provenance.** The review cites the revision *it* examined, and two
of its citations do not resolve to the same code in the inspected commit: its `resmed_loader.cpp:1681-1691`
(the leak-label parse) is at **`:1969–1980`** here, and its `:2540-2544` (the commented-out p50/p95 stores)
is at **`:2905–2909`**. The file has grown by roughly 300 lines above each location, which is weak but
consistent evidence that the review predates the commit inspected. The remaining citations below are quoted
**as the review states them** and were not re-resolved against the inspected tree.

**Two defects found on the way, neither the cause of the 2.0× case:**

- **Gain handling in day-level aggregation.** `Day::percentile` merges raw values from all sessions into
  one map and multiplies by a *single* `gain` at the end (review's citation: `day.cpp:365–387`, `399–421`),
  only logging if gains differ. The multi-day/profile path does the safer thing and normalises per session
  (review's citation: `profiles.cpp:2290–2310`). The review: "If same channel has different gains across
  sessions in one day, `Day::percentile` can mis-scale values" — a "real defect candidate".
- **Time over-accumulation in the waveform branch of `updateCountSummary()`.** `valsum` accumulates
  across eventlists and is converted to time each iteration (review's citation:
  `session.cpp:1298–1314`); with multiple waveform eventlists "earlier counts can be re-applied multiple
  times".

**Why this matters here.** AnxietyWatch v1 imports leak values through its OSCAR CSV path. The values it
imports are therefore OSCAR's recomputed percentiles, which the review shows can differ from the
manufacturer's reported value by a factor of two — and which are subject to a gain-handling defect in one
of the two code paths that compute them. Any leak-based conclusion drawn from v1's CPAP data inherits
that uncertainty. This is a concrete, evidence-backed limitation on v1's data, and it is exactly the kind
of thing a comparison report should surface.

AnxietyWatch2 does not inherit it: V2 decodes the AirSense 11 itself, over BLE, with its own decoders.

### 5.3 Three ingestion paths, three different answers to invalid data

This is the finding this inspection was worth doing for.

**Path A — ResMed EDF (older).** `resmed_loader.cpp:4285–4368`. Having established that the EDF
end-of-data padding decodes to `-1` for most signals and `0` for pressure channels, the loader does:

```cpp
if ((c == -1 || (isPressure && c == 0)) && t_min>=0 ) {
    // -1 is the Null value sentinel for most signals; pressure signals use 0
    // (digital_minimum 0). Either way it is end-of-data padding, not a reading.
    c=last; //reset the current sample to the last one to skip over using it.
    // sometimes the resmed devices create this condition.
    // this change stops false error values from deing displayed.
}
```

The sentinel is replaced by **the previous sample**. The comment is candid about the motivation: the
substitution prevents "false error values" from being *displayed*. Also nearby: pressure channels have
their first 10 seconds discarded (`startpos = 5`, "Shave the first 10 seconds of pressure data") and
device-computed values their first 20 (`startpos = 10`), because the values are unstable at session
start.

Assessed against this project's rules, this is precisely what is forbidden: *"Never clamp an
out-of-range value. Reject it, name the reason, quarantine it. A clamped reading is indistinguishable
from a real one downstream."* A held-over sample is indistinguishable from a real sample in the stored
series, and it propagates into every statistic computed from that channel. The `-1`/`0` sentinel is
named in a comment rather than recorded in the data, so a consumer of the database cannot tell how many
samples were fabricated. The discarded first-10-seconds are a different case — a deliberate, reasoned
exclusion — but they are also silent: nothing marks those samples as excluded rather than never-taken.

**Path B — Apple Health (newer, 2026).** `applehealthDataParsing.cpp:337–446`. The opposite policy:

```cpp
case RecordKind::OxygenSaturation: {
    float value = valueText.toFloat(&ok);
    if (ok) {
        const QStringView unit = attributes.value(QStringLiteral("unit"));
        if (unit != QStringLiteral("%")) {
            warnMalformed(QStringLiteral("skipping OxygenSaturation with unknown unit %1")
                              .arg(unit.toString()));
            return;
        }
        // Apple writes fractions (0.96) under unit "%"; third-party apps may write 96.
        if (value >= 0.0f && value <= 1.0f) {
            value *= 100.0f;
        }
        // 0 and negatives are no-reading sentinels, not measurements.
        if (!qIsFinite(value) || value <= 0.0f || value > 100.0f) {
            warnMalformed(QStringLiteral("skipping OxygenSaturation with out-of-range value %1")
                              .arg(valueText.toString()));
            return;
        }
        out.spo2.append(AppleHealthSample{startMs, value, vitalsSourceId()});
    }
    break;
}
```

Every property this project requires is present, in the newest loader:

| Project rule | OSCAR's Apple Health path |
|---|---|
| Reject, never clamp | `return;` after logging — the sample is dropped, not adjusted |
| Name the reason | every skip logs a specific message naming the type and the offending value |
| Never coerce a sentinel | comment and code: "0 and negatives are no-reading sentinels, not measurements" |
| Validate the unit | `%` required for SpO2; `degC`/`degF`/`K` for wrist temperature; `kg`/`lb`/`st`/`g`/`oz` for body mass; unknown units are rejected by name |
| Convert once, at the boundary | `degF`/`K` → `degC` and `lb`/`st`/`g`/`oz` → `kg` happen **in the parser**, exactly as "converted once, at the decoder" requires |
| Handle the ambiguous encoding explicitly | SpO2 `0.96` and `96` are both accepted and normalised |
| Never fabricate a value | no equivalent of `c = last` anywhere in this file |

So the same program, written by the same team, contains both the anti-pattern and the correct pattern —
and the correct one is the newer code. That is the most useful thing in this package: it is a
demonstration, in a real shipped GPL application, that the discipline this project mandates is
achievable in an app written by a volunteer team with no commercial pressure.

**But the rejection is not surfaced, and that is a real gap.** `warnMalformed` is a lambda over a local
counter:

```cpp
int malformedWarnings = 0;
const auto warnMalformed = [&malformedWarnings](const QString &message) {
    if (malformedWarnings < 10) {
        qWarning().noquote() << "AppleHealthParser:" << message;
    }
    ++malformedWarnings;
};
```

The counter is incremented and never read again. Only the first ten messages reach stderr, and the total
never reaches the user. `AppleHealthImportSummary` — the struct that reports what an import did — carries
`sleepSessions`, `oxiSessions`, `weightDays`, `skippedExisting`, `sleepSourceCounts`,
`ignoredVitalsCounts`, `importedVitalsSources`, `chosenSleepSource` and `validFile`, and **no field for
skipped-invalid samples**. So a value rejected for being physiologically impossible is a stderr line,
capped at ten, and then gone.

This project's rule is that failing closed is correct but failing *silently* is not, and that a rejected
value should be quarantined — which implies a durable record. OSCAR rejects correctly and logs
transiently. The gap between "correct rejection" and "quarantine" is exactly the gap between what OSCAR
does and what the rule asks for. It is a small gap, and it is worth naming precisely because the rest of
the path is right.

**Path C — the summary store: `0.0` is written as the sentinel for absence, and hidden only at export.**

The third answer comes from the storage layer. From the vendor's CSV field design
(`Notes/specs/2026-06-10-csv-report-fields-design.md`):

> **Oximetry blanking:** `daily_summaries`/`session_summaries` store **0.0** (not NULL) for SpO2/Pulse
> when there is no oximetry. To deliver blank cells (and avoid a misleading "0.0% SpO2"), wrap those
> columns in `NULLIF(col, 0)` → NULL → empty CSV field. This is display suppression, not a calculation;
> 0 never occurs as a real SpO2/pulse reading. `Leak_Unint_Avg` is **not** wrapped — 0 is a legitimate
> value there.

And in the shipped report, that is exactly what the SQL does:

```sql
ROUND(NULLIF(ss.spo2_avg, 0), 1) as SpO2_Avg,
ROUND(NULLIF(ss.spo2_min, 0), 1) as SpO2_Min,
ROUND(NULLIF(ss.pulse_avg, 0), 1) as Pulse_Avg,
```

This is the anti-pattern this project forbids, expressed precisely: *"Invalid or absent data
contributes nothing. Never coerce a sentinel to a number."* In OSCAR's store, an absent SpO2 **is** the
number `0.0`.

Three things make this more interesting than a plain defect:

1. **The knowledge is present and per-column.** OSCAR explicitly distinguishes "0 means absent"
   (`SpO2`, `Pulse` → masked) from "0 means zero" (`Leak_Unint_Avg` → deliberately not masked, with the
   reason recorded). It is not confused about the semantics.
2. **The repair is at the consumer boundary, not the producer.** The `NULLIF` sits in the report, so the
   sentinel is un-hidden for anything that reads the database directly — which OSCAR actively invites,
   by publishing the schema and shipping a Python reader (§4.1, X1–X2). A third-party consumer querying
   `session_summaries.spo2_avg` gets `0.0` and no marker saying it means "no oximeter".
3. **The rule-violation is masked by an accident of benefit to v1.** v1's `dbl()` helper treats an empty
   CSV cell as nil ("Empty cell = 'not reported' → nil"), so v1 reads OSCAR's blanked cell correctly —
   because OSCAR suppressed the sentinel before v1 saw it. v1's own store would then hold `nil`, which is
   right. The chain is correct at both ends and wrong in the middle, and the middle is the part OSCAR
   publishes for reuse.

The three policies in one program, side by side:

| Path | Absent / invalid value | Encoded as | Recoverable downstream? |
|---|---|---|---|
| ResMed EDF | end-of-data sentinel `-1` / `0` | previous sample, in place | **No** — indistinguishable from a real sample |
| Apple Health | out-of-range or unknown-unit value | dropped, named in a stderr line | **Partly** — logged (first ten only), absent from the summary |
| Summary store | no oximetry collected | the number `0.0` | **Yes, but only via the report's `NULLIF`** |

That is the finding in its full form. OSCAR is not careless about absent data — it names the sentinel in
a comment, rejects it by name in one loader, and distinguishes sentinel-zero from real-zero per column.
It is that it encodes absence as a *value* at each of the three layers where it makes the decision, and
then repairs it at the point of display. Every layer that reads before the repair sees a measurement
where there was none.

### 5.4 Provenance is recorded, and duplicates are resolved deterministically

Still in the Apple Health path, which is the one that shares a data source with AnxietyWatch v1.

**Samples carry their source.** `AppleHealthSample{startMs, value, vitalsSourceId()}` — every reading
carries an identifier for the HealthKit `sourceName` that produced it, and a `vitalsSourceCounts` map
tallies which sources contributed. Provenance is per-sample, not per-import.

**A deterministic source-selection rule** (`applehealthDataParsing.cpp:158–177`): choose the Apple Watch
source with the most sleep intervals; ties break alphabetically, case-insensitively; if no Watch source
exists and there is exactly one source, take it. Deterministic, documented in code, and it has a chooser
UI for the ambiguous case (`m_sleepSourceChooser`, invoked only when more than one source exists). If the
user dismisses the chooser, the import **aborts before writing anything** —
`// Chooser dismissed: abort before anything is written.`

That is a good pattern: an unresolved ambiguity stops the write rather than being defaulted away.

**Unwanted sources are excluded and counted, per source.** `isAllowedVitalsSource()` permits the chosen
sleep source plus Apple first-party sources (`Apple Watch`, `iPhone` — with a comment explaining that
"Apple can attribute watch-measured SpO2 to the iPhone"). Anything else is dropped, and the number of
dropped samples is recorded per source in `ignoredVitalsCounts`, which *is* carried into the import
summary and asserted in tests. So this class of exclusion **is** surfaced. That contrast with §5.3 is
telling: OSCAR reports what it excluded by source, but not what it excluded for being invalid.

**Re-import is idempotent per night.** `existingSleepNights` / `existingOxiNights` cause an
already-present night to be skipped and counted in `skippedExisting`, and the release notes describe a
separate check for "whether the SD card being imported is same as last time". Re-importing the same
HealthKit export does not duplicate data.

**One assumption is stated rather than verified** (`applehealth_loader.cpp:364`):

```cpp
// Stages are published only after a night ends, so the newest night is never partial.
```

This is a belief about Apple's behaviour used to justify importing the newest night. Nothing enforces
it. If Apple ever publishes a partial night, OSCAR will import it as complete. Compare with this
project's rule that "missing data holds state; it never reassures" — under which the newest night would
be marked provisional until closed. Recorded as an assumption in code, not a defect.

### 5.5 OSCAR refuses to print numbers it cannot measure

Three separate places, which together make a consistent pattern:

1. **`unclassified_count`, not `central_count`.** Schema v10, from `SCHEMA_DESIGN_PHILOSOPHY.md`:

   > **Semantic accuracy**: Most CPAP machines cannot distinguish between true Central Apneas and other
   > unclassified events
   >
   > **Correct terminology**: "Unclassified Apnea" (UA) is more accurate than assuming all are "Central
   > Apnea" (CA)

   A field was renamed and existing data migrated, because the old name asserted more than the machine
   measured. The AHI arithmetic was unchanged — only the claim about what the number *means* was
   corrected. This is a textbook application of "an inference is never presented as a measurement", and
   it is the closest philosophical match to this project found in any candidate.

2. **`OHI=`/`CHI=` are printed only when the data exists.** From
   `Notes/specs/2026-08-06-central-obstructive-hypopnea-design.md`:

   > Printed report (§3.3) shows `OHI=`/`CHI=` only when the day actually has OH or CH data, rather than
   > printing `0.00` for every other device. `OAHI=`/`CAHI=` always show.

   A `0.00` and "this device cannot report this" are different states, and OSCAR renders them
   differently.

3. **Capability-gated channels.** The same spec keeps the single Hypopnea channel "for devices that do
   not" score central and obstructive separately, and computes
   `oahiChannels = ahiChannels − cahiChannels` so "the identity holds by construction" rather than by
   convention. Channels exist only where the device can produce them.

Two more examples from the engineering notes reinforce it:

- **Schema v15 dropped `dst_enabled` from `user_info`** because it was "Stored but never read by
  application". Dead state was identified and removed — the inverse of adding a field that implies a
  capability.
- **The v16 post-mortem** is a model of honest self-assessment:

  > The `machine_id` column was a design mistake: every caller passed `0` intending "combined", but the
  > calculation code silently rewrote that to the first enabled CPAP session's machine id, contradicting
  > the original "NULL = combined" documentation. The corresponding read-side helpers … queried
  > `machine_id IS NULL` and so could never find the rows the writer stored. **None of those finders had
  > any in-tree callers, which is why the mismatch was invisible.**

  A writer and its readers disagreeing about a sentinel, invisible because nothing called the readers —
  the same family of defect as a coerced reading, caught by an unrelated investigation and documented
  publicly.

The pattern is not uniform (§5.3's `c = last` is in the same program), but it is strong, deliberate, and
concentrated in the parts of the program that were written or revised most recently.

### 5.6 Reliability work that AnxietyWatch v1 has already inherited

Two 2.0.1 fixes are specifically about the ResMed AirSense 11, the machine at the centre of both
AnxietyWatch versions:

- `[fix] Correct import second session of day with ResMed 11 series machines`
- `[fix] Correct session start time if ResMed EDF start date is corrupted`

And a broader one: `[fix] Fix Resmed loader error when session starts <60 sec after previous one ends`.

The existence of the first tells us that the AirSense 11's "two sessions in one day" case has been a
real source of import errors, and that OSCAR has had to correct its handling of it. Since v1's CPAP
understanding is derived from OSCAR's reverse engineering, v1 is exposed to the same edge case. V2's own
AirSense 11 work should treat a same-day second session as a known hazard rather than a hypothetical one.

The bug log itself is worth recording as an artifact: `Notes/Developer Notes/BUG_FIXES.md` is **6,748
lines** of dated, root-caused entries, each naming files, the observed symptom, the mechanism, and what
was changed. The two most recent entries (2026-10-04) are both **silent data loss on a save path** — the
#307 entry describes imported journal notes, weight, feelings and bookmarks "lost at the next launch"
because a save wrote only legacy files, and the #308 entry describes ResMed settings "lost at the next
launch" for the same structural reason: something marked data changed and nothing ever saved it. Both
were found by a sweep for code still reading 1.x files, which also removed write paths that had no
readers ("still wrote `Summaries.xml.gz` … although nothing read them").

---

## 6. Runtime observation

**None. OSCAR was not installed, compiled or executed.**

Nothing in this package is an observation of running software. The reason is proportionate: the source is
public and complete, so executing the binaries would add little that reading the code does not already
establish, while adding the cost of installing a Qt6 desktop application and sourcing CPAP data files to
feed it. Where a claim depends on behaviour rather than code — compiled binary equivalence to the
inspected commit, the localised-header failure mode, and current status of the 2026-04 leak defects —
that is stated as a limit in §7 rather than assumed away.

**Evidence classes used in this document:**

- **Static finding** — read directly from the pinned commit. Almost everything here.
- **Documented** — the vendor's own published design documents, including internal method reviews. Used
  for design intent and for the leak review's reproduced numbers, which are the vendor's findings rather
  than mine.
- **Public claim** — from the download page.
- **Inference** — labelled wherever used.
- **Unknown** — listed in §7.

---

## 7. Claim reconciliation

| # | Claim | Verified in code? | Finding |
|---|---|---|---|
| X1 | "Access to all OSCAR data from other programs" | **Yes, as a contract** | Data is a local SQLite file with published schema docs and a Python reader. No API gate. Not executed. |
| X2 | "Demo program and specs showing how to read data from OSCAR using Python" | **Yes** | `Notes/Accessing OSCAR Data/oscar_waveform_demo.py` and `python_waveform_demo_spec.md` exist, plus a second copy under `Notes/Waveform Demo/`. |
| X3 | "Adjust CPAP times … (replaces CPAP clock drift)" | **Yes, and better than claimed** | `device_time_corrections` is typed, reasoned, range-scoped, drift-modelled and reversible (§4.5). |
| X4 | "Migrated OSCAR and OSCAR data to an SQL database" | **Yes** | ~20 tables; preferences, layouts and report trees all moved out of files (§4.1). |
| X5 | CSV export of database data | **Yes** | Three modes (Details/Summary/Sessions) plus templates; days without data are omitted (§5.1). |
| X6 | "Check whether SD card being imported is same as last time" | **Yes** | Present for the Apple Health path as per-night idempotence; the SD-card check is described in release notes and was not located in the tree. |
| X7 | "Correct import second session of day with ResMed 11 series machines" (v2.0.1 fix) | **Partly** | The fix is recorded in the release notes and the AirSense 11 handling was read; the specific defect and its patch were not isolated in this inspection. |
| X8 | Support for the listed loaders | **Yes** | 19 loaders registered in `main.cpp:1267–1285` (§7 of the claims file). |
| X9 | "Viatom now accepts 100% SpO2 without warning" | **Yes** | Consistent with the Viatom loader emitting `OXI_SPO2` up to 100; the change is recorded in the release notes. |
| X10 | "Feelings meter … range of 0 to 100" | **Yes** | Present as a UI change; the journal/settings schema carries the value. Subjective data is stored alongside objective, but is not scored or correlated. |
| X11 | Statistics page shows oximetry data recorded by the CPAP | **Yes** | Oximetry channels sit on the same timeline as CPAP channels and are aggregated in `daily_summaries`. |
| X12 | Profile password protection | **No — removed** | Release notes: "Remove password support". The database is unencrypted at rest (§4.4). |
| X13 | Privacy: no data leaves the machine | **Yes, strongly** | No telemetry of any kind; network code is limited to user-owned storage via OAuth2 (§4.3). |
| X14 | Wireless / Bluetooth device support | **No — absent by design** | No BLE stack; the UI says bluetooth oximeters are "probably not" supported (§4.2). |
| X15 | Real-time / live monitoring | **No — absent** | No ingestion path reaches analysis during a night; "live streaming mode" is a cable transfer mode (§4.2). |
| X16 | Leak percentile equals the device's reported leak | **No — and the vendor documents the divergence** | Percentiles are recomputed; a 2.0× case is reproduced in the vendor's own review (§5.2). |
| X17 | Central apnoea counts are central apnoeas | **Corrected by the vendor** | Renamed to `unclassified_count`; the vendor states most machines cannot make the distinction (§5.5). |
| X18 | Pure central-apnoea users get an OHI/CHI line of 0.00 | **No — deliberately omitted** | Rendered only when the data exists (§5.5). |
| X19 | Invalid values are handled consistently | **No — two opposed policies** | ResMed holds the previous sample; Apple Health rejects by name (§5.3). |
| X20 | Every rejected value is visible to the user | **No** | Malformed skips reach stderr (first ten only) and are absent from the import summary; source-level exclusions *are* counted (§5.3, §5.4). |
| X21 | Missing nights are represented as zero | **No** | Absent days produce no CSV row and no stored sample (§5.1). |
| X22 | The database survives version upgrades | **No — by stated policy** | A schema mismatch may require a fresh database and reimport (§4.7). |
| X23 | The test suite validates the code | **Written but not enforced in CI** | 473 assertions, `-Werror` and ASan; no CI configuration exists (§4.6). |
| X24 | OSCAR is GPLv3 and its source is inspectable | **Yes** | `COPYING` in the tree; the licence is why this inspection was possible at all (§2). |
| X25 | The by-session report is stable across versions | **No — the header widened** | 22 columns in the design record for 2.0.1; 27 in the inspected commit, with `OAHI`/`CAHI`/`A`/`OH`/`CH` inserted mid-header (§5.1.3). |
| X26 | v1 detects an OSCAR format change before importing it | **No** | Detection is a prefix match and validation is `count >= 21`; a widened header satisfies both (§5.1.3). |
| X27 | Absent oximetry is stored as absent | **No — stored as `0.0`** | `daily_summaries`/`session_summaries` hold `0.0` for missing SpO2/pulse; blanked only by `NULLIF` in the report SQL (§5.3, path C). |
| X28 | The day and session reports agree about percentiles | **No — 90th vs 95th, vendor-flagged** | Day report was labelled `Pressure_95th` while storing a p90; the vendor relabelled it `Pressure_90th`. Session is a genuine p95 (§5.1.2). |
| X29 | Session times are stored as device-local strings | **No — epoch milliseconds** | `start_time` is epoch ms; local rendering happens at query time via `'unixepoch','localtime'` (§11 of the claims file). |

---

## 8. Evidence gaps

Recorded so downstream work does not treat these as settled.

1. **No runtime observation.** The compiled 2.0.1 artefacts were not compared to the inspected commit.
   The inspected commit is the released line's tip as of 2026-10-04, and the download page points at this
   repository, but binary/source equivalence was not verified.
2. **The leak defects may be fixed.** The review is dated 2026-04-16 and describes code as it stood then.
   The *method* (recomputed percentiles) was confirmed in the current tree; whether the gain-handling and
   time over-accumulation defects in the review's items C and D remain was **not** re-confirmed against
   the current source. Treat X16 and §5.2's defect list as "reported by the vendor in April 2026",
   not as "present today".
3. **The localised-CSV-header failure was not reproduced.** It follows from `tr()` on the header tokens,
   read in this commit. Untested.
4. **The 1.x line was not inspected.** Several loaders present in the 2.0 tree (`mseries`, `dreem`,
   `somnopose`, `zeo`, `serialoximeter`) are not in the 2.0 registration list; their status is unknown.
5. **The 2.0 database rewrite's history is short and single-author-hosted.** The repo was created
   2026-06-07 under an individual namespace and the release is linked from the official download page.
   Whether the wider OSCAR Team reviews it was not determined.
6. **No user, download or install figures exist to check.** Star and fork counts are the only scale
   signals and are not user counts.
7. **`AppleHealthImportSummary`'s non-use in the GUI was checked by name only.** The struct is referenced
   in tests and the loader; no GUI consumer was found by search. A consumer reached by a different name,
   or through a base-class interface, would have been missed.
8. **Respironics key provenance was not investigated.** The keys are in the source; where they came from
   and their legal status were not examined, and no position is taken.
9. **The "check whether the SD card is the same as last time" logic was not located in the tree.** The
   release notes describe it; the Apple Health idempotence mechanism was found instead.
10. **Whether v1's OSCAR-CSV import has ever been exercised against real OSCAR output is unknown.** The
    header correspondence is verified from both source trees; an end-to-end run was not performed, and no
    evidence of one was found.
11. **Which header OSCAR 2.0.1 actually emits was not confirmed by running it.** The 22-column form is
    taken from the vendor's design record for the report that 2.0.1 shipped; the 27-column form is read
    from the inspected commit. The binary was not executed to confirm either (§5.1.3, labelled inference).
12. **`isOSCARBySessionFormat` was read, but not run against the 27-column header.** The prefix match
    follows from string comparison and is stated as a static reading, not a test result.
13. **The other 17 loaders were not examined for their invalid-data policy.** The three-policy finding in
    §5.3 rests on the ResMed EDF, Apple Health and summary-store paths only, which are the three that
    touch AnxietyWatch v1's data. It is not a claim about OSCAR as a whole.
14. **Whether the summary-store `0.0` sentinel reaches v1 in any form was not tested.** The `NULLIF`
    masking is read from the report SQL; a v1 import against a real file was not performed.

---

## 9. Ethical and legal limits applied

- Access was through a public, GPLv3-licensed repository over public HTTPS. No protection measure was
  circumvented, no authentication was bypassed, and no credential was used.
- No user data, profile database, exported CSV or CPAP record was obtained, read or processed. The
  inspection touched source code, in-tree documentation and build files only.
- Nothing was executed, so no claim about runtime behaviour is made anywhere in this package.
- Vendor-published internal documents (the leak review, the schema philosophy, the bug log) are treated as
  **the vendor's own statements**, quoted with attribution and dated. They are not independent
  validation, and their reproduced numbers are reported as the vendor's findings.
- No position is taken on the propriety of embedded Respironics decryption keys, nor on the legal status
  of importing vendor SD-card data. Both are described factually and left alone.
- OSCAR is a volunteer, unfunded project serving people who use CPAP machines. This inspection is
  forensic in method but not adversarial in intent, and the negative findings (§5.2, §5.3, §4.6, §4.7)
  are reported precisely because the same tree contains strong positive findings that make them
  informative rather than merely critical.

---

## 10. Reproduction

Every finding above is reproducible from the pinned commit.

```sh
# 1. Obtain the released source (GPLv3, public, no authentication)
mkdir -p /tmp/aw2research/oscar && cd /tmp/aw2research/oscar
git clone --depth 1 https://gitlab.com/CrimsonNape/oscar-sql.git oscar-sql
cd oscar-sql
git log -1 --format='%H %ad %s'   # expect 28df2e46e35e6260e186fced997edadcf001cc24, 2026-10-04

# 2. Licence and lineage
head -20 COPYING                     # GNU GPL v3
head -20 README                      # SleepyHead 1.1.0 derivative

# 3. The two opposed invalid-data policies
sed -n '4285,4370p' oscar/SleepLib/loader_plugins/resmed_loader.cpp     # c=last
sed -n '337,446p'   oscar/SleepLib/loader_plugins/applehealthDataParsing.cpp

# 4. Rejections are logged but not surfaced
sed -n '228,237p'   oscar/SleepLib/loader_plugins/applehealthDataParsing.cpp
sed -n '26,37p'     oscar/SleepLib/loader_plugins/applehealth_loader.h   # summary has no malformed field
sed -n '230,236p'   oscar/SleepLib/loader_plugins/applehealthDataParsing.cpp   # warn cap: 10

# 5. Provenance, source selection, idempotence
sed -n '126,176p'   oscar/SleepLib/loader_plugins/applehealthDataParsing.cpp
sed -n '200,253p'   oscar/SleepLib/loader_plugins/applehealth_loader.cpp
sed -n '355,430p'   oscar/SleepLib/loader_plugins/applehealth_loader.cpp

# 6. The time-correction ledger
grep -n -A22 'device_time_corrections' oscar/database/database_schema.cpp

# 7. The CSV vocabulary v1 matches, and the header drift
sed -n '159,232p'   oscar/exports/exportcsv.cpp
sed -n '145,180p'   oscar/docs/system_reports.orf   # by Session: current column list
sed -n '1,140p'     'Notes/specs/2026-06-10-csv-report-fields-design.md'  # the 22-column record
sed -n '1,80p'      'Notes/specs/2026-08-06-central-obstructive-hypopnea-design.md'

# v1 side of the contract (read-only, different repository)
grep -n -A4 'isOSCARBySessionFormat\|isOSCARFormat' \
     ../AnxietyWatch/AnxietyWatch/Services/CPAPImporter.swift
sed -n '709,766p'   ../AnxietyWatch/AnxietyWatch/Services/CPAPImporter.swift  # positional indices
sed -n '291,295p'   ../AnxietyWatch/AnxietyWatch/Services/CPAPImporter.swift  # "22 columns"

# 8. No telemetry, no BLE, no CI
grep -rniE 'telemetry|google-analytics|sentry|crashreport|mixpanel|amplitude|firebase' \
     --include=*.cpp --include=*.h --include=*.pro oscar/
grep -rniE 'QLowEnergy|QBluetooth' --include=*.cpp --include=*.h oscar/
find . -path ./.git -prune -o \( -name '*.yml' -o -name '*.yaml' -o -name 'Jenkinsfile' \) -print
grep -n -A30 'CONFIG+=test' oscar/oscar.pro

# 9. Tests and their enforcement
for f in oscar/tests/*.cpp; do printf '%-28s %s\n' "$(basename $f)" \
  "$(grep -c 'QCOMPARE\|QVERIFY' $f)"; done

# 10. The vendor's own design record
wc -l Notes/Database/*.md Notes/Developer/../Developer\ Notes/BUG_FIXES.md
cat  'Notes/Database/SCHEMA_DESIGN_PHILOSOPHY.md'
cat  'Notes/Developer Notes/LEAK_PERCENTILE_METHOD_REVIEW_2026-04-16.md'
cat  'Notes/specs/2026-08-06-central-obstructive-hypopnea-design.md'

# 11. The public-info gate for anything committed from this work
# run from the AnxietyWatch2 repository root
python3 scripts/check-public-info.py
```

Note for reproduction: the leak review and bug log quote a tester's local Windows profile path. That path
is deliberately **not** reproduced in this document or anywhere in the committed package.

## Related files

- `oscar_CLAIMED_FEATURES_AND_PURPOSE.md` — vendor claims, device coverage, and the v1 dependency evidence
- `PROMPT_2.md` — candidate-specific second-stage prompt
- `oscar_COMPARISON.md` — the comparison against AnxietyWatch v1 and v2
- `../RESEARCH_METHOD.md` §6.1 — inspection-mode taxonomy
