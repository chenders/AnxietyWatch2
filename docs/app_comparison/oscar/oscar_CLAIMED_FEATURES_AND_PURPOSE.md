# OSCAR — claimed features and purpose

**Slug:** `oscar`
**Research date:** 2026-10-06
**Phase:** Phase 2 (public claims) + Phase 2A (authorized implementation inspection)
**Implementation inspection:** **fully performed** from published source — see `oscar_IMPLEMENTATION_INSPECTION.md`
**Shortlist rank:** 4 of 10 (`../CANDIDATE_SHORTLIST.md`)

---

## 1. Executive summary

OSCAR ("Open Source CPAP Analysis Reporter") is a free, GPLv3, offline desktop application for
reviewing sleep-therapy data. It reads the SD cards, files and cables of roughly two dozen CPAP
machines and a handful of oximeters, and displays the results as graphs and statistics down to the
breath-by-breath level. It is a derivative of SleepyHead 1.1.0, which its author abandoned; the OSCAR
Team continued it. Version 2.0.1 was released 2026-06-23.

**The most important fact about OSCAR for this project is not competitive. It is a dependency.**
AnxietyWatch v1 already consumes OSCAR's output: v1 has a first-class `source == .oscar` enum value, an
`isOSCARBySessionFormat` importer, and a test asserting that a 04:00 session belongs to the previous day
because that is *"OSCAR's noon-to-noon convention"*. v1's `REQUIREMENTS.md` names the OSCAR community's
reverse engineering as the basis for its own CPAP parser. OSCAR is therefore the closest thing the v1
CPAP data path has to an upstream reference implementation, and a defect in OSCAR's leak or AHI maths is
inherited by v1.

Three findings make OSCAR worth its own package even so:

1. **v1's CPAP importer is coupled to OSCAR's report vocabulary by column position, and that vocabulary
   has already changed.** v1 parses the by-session export at hardcoded indices 0–20 with only a
   `count >= 21` guard, and detects the format by a six-token prefix. The report v1 was written against
   (22 columns, matching OSCAR 2.0.1 exactly) is 27 columns in the inspected source, with five columns
   inserted mid-header. The detector still fires, the guard still passes, and every field from index 4
   onward is silently misassigned — including `Leak_95th` being read as `Hours_Used`. This is the most
   consequential finding in the package. See §13.1.

2. **A single codebase demonstrates three different invalid-data policies, all of them this project
   argues about.** The ResMed EDF loader substitutes the *previous sample* for an end-of-data sentinel
   (`c = last`), with a comment saying this "stops false error values from being displayed". The much
   newer Apple Health loader does the opposite: it rejects an out-of-range value, **names the reason**,
   and skips it — `"skipping OxygenSaturation with out-of-range value %1"` — including an explicit
   comment that "0 and negatives are no-reading sentinels, not measurements". And the summary store
   writes the number **`0.0`** for absent oximetry, hiding it only at export with `NULLIF(col, 0)`. The
   paths were written years apart and arrived at three different answers.

3. **OSCAR sometimes refuses to print a number it cannot measure.** The Central/Obstructive Hypopnea
   work specifies that `OHI=`/`CHI=` are shown "only when the day actually has OH or CH data, rather
   than printing `0.00` for every other device". Separately, machines that merely *score* a
   central-looking event get it recorded as `unclassified_count`, not `central_count`, because "most
   CPAP machines cannot distinguish between true Central Apneas and other unclassified events". Both are
   the same instinct this project encodes as *unknown is not OK*.

3. **The project documents its own inaccuracies in public, at length.** A 6,748-line dated bug log, a
   published schema design philosophy that admits a column "was a design mistake", and a leak-percentile
   method review that reproduces a **2.0× disagreement** with the device's own reported `Leak.95` and
   identifies a gain-handling defect in one of two implementations of the same statistic.

Against that, OSCAR is not a competitor to either AnxietyWatch version, and the differences are
categorical rather than incremental:

| Axis | OSCAR | AnxietyWatch v2 |
|---|---|---|
| Data arrival | User moves an SD card, or plugs in a cable, after the fact | Continuous BLE streaming while the event is happening |
| Wireless devices | **None in tree.** OSCAR's own UI text: bluetooth oximeters are "probably not" supported | AirSense 11, Polar H10, oximeter, all wireless |
| Sensors | CPAP + oximeter only, no phone at all | Phone, ECG chest strap, oximeter, CPAP, weather |
| Subjective capture | Free-text journal + a 0–100 "feelings" meter, entered later at a desk | Panic-time capture on the phone, on demand |
| Alarm / response | None. It is a review tool | Supervised alarm with a hardware dead-man switch (V2 **specified**, not shipped) |
| Platform | Windows / macOS / Linux / Chromebook / RasPiOS desktop only | Raspberry Pi collector + iOS client |
| Deletion | First-class "Purge Range of Days"; schema upgrades require a fresh database and reimport | Samples are append-only; removal is a tombstone |

**Verdict.** OSCAR is best understood as v1's upstream format authority and as a worked example — in
one GPL repository — of a device-data pipeline that got provenance and "not measured" right in its
newest ingestion path and wrong in its oldest, while leaving v1's importer exposed to a format change
that has already happened upstream.

---

## 2. Positioning

OSCAR describes itself in two registers.

The download page's short form:

> OSCAR is PC software developed for reviewing and exploring data produced by CPAP and related machines
> used in the treatment of sleep apnea. OSCAR never asks for payment — It is free and always will be
> free.

The long form adds the purpose and the audience:

> OSCAR provides for the viewing of the high-resolution sleep data that is generated by the CPAP and
> stored on the SD card. OSCAR converts this data to graph form allowing the user to view this data down
> to a breath by breath level. […] It is this information that enables you to provide data for discussion
> with your doctor and/or to view and optimize your personal therapy.
>
> OSCAR is also used by many physicians and therapists who consult with patients, and by medical
> researchers comparing patient/machine responses across multiple platforms. OSCAR provides visual and
> statistical data details and enables comparisons across machine types and manufacturers, that are not
> available from proprietary manufacturer software and data products. Professionals who rely on OSCAR to
> provide research and to serve patient needs will lose this valuable resource if data becomes
> inaccessible, or only available through the manufacturer's data products.

That last sentence is the project's stated political purpose: **manufacturer-independent access to
device data the user already generated.** It is a data-liberation argument, not a clinical one. The
README repeats the author's original framing — Mark Watkins wrote SleepyHead "to help lessen the
exploitation of others", and asks that resellers of derivatives credit the work.

Note what is *not* in the positioning: OSCAR does not claim to diagnose, to treat, or to replace a
clinician. It claims to display. Its own framing is that it gives the user something to *discuss*.

---

## 3. Identity, licensing and governance

| Item | Value |
|---|---|
| Name | OSCAR — Open Source CPAP Analysis Reporter |
| Lineage | Derivative of **SleepyHead 1.1.0**, abandoned by Mark Watkins (aka "Jedimark") |
| Copyright | Portions (C) 2011–2018 Mark Watkins; portions (C) 2019–2026 The OSCAR Team `<oscar@oscar-team.org>` |
| Licence | **GNU GPL v3** (`COPYING`) |
| Language / stack | C++ with Qt (Qt6 for 2.0; Qt5 for 1.7.2) |
| Current release | **2.0.1, released 2026-06-23** (per the download page) |
| Source hosting | GitLab |
| Governance | Voluntary team; no company; donations to **Apnea Board** underwrite server costs |
| Funding | None. "OSCAR never asks for payment" |
| Trademark / DRM | None observed. No licence gate, no activation, no account |

**Two live source repositories, and they are not the same lineage:**

| Repo | Role | Signals (2026-10-06) |
|---|---|---|
| `gitlab.com/pholy/OSCAR-code` | OSCAR 1.x mainline; "A fork of sleepyhead/sleepyhead-code" | 189 stars, 112 forks, created 2019-01-23, last activity 2026-01-31 |
| `gitlab.com/CrimsonNape/oscar-sql` | The **2.0 SQL** line, and the source the 2.0.1 download page points at | 10 stars, 13 forks, created 2026-06-07, last activity 2026-10-04 |

GitLab reports no licence metadata for either project even though `COPYING` is GPLv3 in the tree — a
metadata gap, not a licensing one.

The 2.0 line is a **fork hosted under an individual's namespace rather than the project namespace**, and
it was created only months before the 2.0.1 release. The download page links to it as the official source
for 2.0.1. Two consequences worth recording plainly: the 2.0 database rewrite has a short public history,
and the release's provenance depends on one contributor's namespace. This is a governance observation,
not an allegation — the code is GPLv3 either way.

Scale signals: two orders of magnitude larger than Exist's user base, but the numbers are community
counts, not user counts. GitLab stars/forks (above), plus a private "Beta Test Group" the download page
tells testers not to share the software outside of. No download count, no active-user figure, and no
self-published revenue were found.

---

## 4. Availability and platforms

Desktop only. The 2.0.1 download page lists:

| Platform | Artefact |
|---|---|
| Windows | `OSCAR-2.0.1-Win64.exe` — **Windows 10 and 11 only**; 7/8/8.1 no longer supported |
| macOS | `OSCAR20-2.0.1-Intel.dmg` (macOS 15.7 Sequoia+), `OSCAR20-2.0.1-ARM.dmg` (macOS 13 Ventura+) |
| Debian | 12 and 13, `amd64` |
| Ubuntu | 22.04, 24.04, 26.04 LTS only |
| Chromebook | `amd64` and `arm64`, Debian 12/13 |
| RasPiOS | 12, `armhf` and `arm64` |

Plus a separate 1.7.2 page "for Older Systems that do not support 2.0 or Qt6".

There is **no mobile application, no web application and no hosted service**. A RasPiOS ARM64 build
exists, which means OSCAR will run on the same class of hardware as the AnxietyWatch2 collector — but as
a desktop GUI launched by a human, not as a service.

---

## 5. Pricing and funding

Unambiguous, and verified in the source tree (no licence check, no activation, no account):

- **Free.** "OSCAR never asks for payment — It is free and always will be free."
- Donations are solicited for **Apnea Board**, to "help offset additional server costs" — i.e. hosting,
  not the developers.
- No paid tier, no subscription, no in-app purchase, no upsell. No account is required to run it.

The only cost the user bears is the SD-card reader the requirements document in AnxietyWatch v1 also
budgets for ($10–15).

---

## 6. Claimed capabilities

The vendor's own v2.0 highlights list, quoted verbatim from the download page (abridged to the
feature-bearing lines):

**Major update**
- Migrated OSCAR and OSCAR data to an SQL database
- Share part of a profile with other users
- CSV export of data in the database
- **Access to all OSCAR data from other programs**
- **Adjust CPAP times as needed and align data from oximeters (replaces CPAP clock drift)**

**Backup, restore, share**
- Share part of a profile by file or direct to Dropbox or Google Drive
- Restore a shared profile from file or from a URL
- Backup and restore part or all of a profile

**Data export**
- CSV export format has predefined and user configured templates
- Journal notes can be exported as a report
- **Demo program and specs showing how to read data from OSCAR using Python**

**Loaders**
- Support for ReactHealth/BMC G3 and G3X CPAP
- Support for Yuwell YH-580, YH-690, YH-820, YH-825, YH-830 CPAP
- Support for BMC G3 B20A legacy format CPAP
- Added identification of Löwenstein prisma25S and prisma25ST
- BMC G3X loader distinguishes mask-on from total-hours
- **Check whether SD card being imported is same as last time** (disable check in Preferences)
- **Correct session start time if ResMed EDF start date is corrupted**
- **Correct import second session of day with ResMed 11 series machines**
- Viatom now accepts 100% SpO2 without warning

**Daily page**
- Adjust times for CPAP with wrong times and align data from oximeters and other devices
- Add optional session bars to SpO2, Pulse, and Movement graphs
- **Feelings meter on Daily Notes panel now has a range of 0 to 100**

**Statistics page**
- Add Steady Breathing to new Other Statistics section
- Changes to Device Settings allows similar machines to be combined
- Statistics page shows oximetry data for data recorded by CPAP machine

**Preferences**
- **CPAP Clock Drift is replaced by menu item Data/Time Corrections**

**Profiles**
- Remove password support

**Database support**
- Advanced user can have multiple databases and switch between them
- Prevent two OSCAR instances from opening the same database at the same time
- Automatic database integrity check at startup

**Privacy-relevant UI detail**
- "Remove profile name from title bar when taking screenshot"

Three claims in that list are load-bearing and are inspected in
`oscar_IMPLEMENTATION_INSPECTION.md`: the **Python demo + specs** (a documented external read path),
**Data/Time Corrections** (a reversible clock-correction ledger), and the **AirSense 11 second-session
fix** (OSCAR 2.0 has had to repair AirSense 11 imports, the same machine V1's and V2's pipelines target).

---

## 7. Supported devices

Nineteen loaders are registered in `oscar/main.cpp`. Mapping them to their families:

| Family | Loader class | Devices |
|---|---|---|
| ResMed | `ResmedLoader` | S9, AirSense/AirCurve 10 and 11, AirMini, Lumis, Astral, Stellar (EDF) |
| Philips Respironics | `PRS1Loader` | System One 50/60 series, DreamStation (encrypted payload — see below) |
| Philips Respironics | *(M-Series loader present in tree)* | M-Series |
| Fisher & Paykel | `FPIconLoader`, `SleepStyleLoader` | ICON, SleepStyle |
| DeVilbiss | `IntellipapLoader` | IntelliPAP |
| Löwenstein | `PrismaLoader` | Prisma series (prisma25S/25ST identification added in 2.0) |
| BMC / ReactHealth | `BmcLoader`, `BmcG3xLoader` | G3, G3X, G3 B20A legacy |
| Weinmann | `WeinmannLoader` | SOMNOseries |
| Resvent | `ResventLoader` | iBreeze |
| Yuwell | `YuwellLoader` | YH-580/690/820/825/830 |
| Sefam | `SefamLoader` | S.Box and relatives |
| Apex | `ApexLoader` | Apex |
| VREM | `VREMLoader` | VREM |
| Oximeters | `CMS50Loader`, `CMS50F37Loader` | Contec CMS50D+/E/F/I (serial) |
| Oximeters | `MD300W1Loader` | MD300W1 |
| Oximeters | `ViatomLoader` | Viatom/Wellue, including O2Ring and POD2 |
| Phone | `AppleHealthLoader` | **Apple Health export (XML or ZIP)** — new in 2.0 |

Present in the tree but not in the 2.0 registration list: `mseries_loader`, `dreem_loader`,
`somnopose_loader`, `zeo_loader`, `serialoximeter`. Treat their 2.0 status as **unknown**; they were
not confirmed active in this inspection.

The **Apple Health loader is the single most consequential entry for this comparison.** It means OSCAR
2.0 reads the same HealthKit export a phone produces, and therefore overlaps directly with the data
source AnxietyWatch v1 is built on. What it extracts is listed in §9.

---

## 8. How data arrives (import mechanism)

Every path is **post-hoc and manual**:

- **SD card** — the primary path. Copied from the CPAP machine's card; OSCAR reads the vendor's EDF or
  proprietary files. On Windows it can find the card on WebDAV-mounted drives.
- **USB / serial cable** — the Contec CMS50 family and the MD300W1 oximeter are downloaded over a cable.
- **File** — Viatom/Wellue oximeter exports, Apple Health export XML/ZIP, and OSCAR's own exported CSVs
  and profile backups.
- **Shared profile** — a backup file, or a URL to one (added in 2.0).

There is **no wireless ingestion of any kind**. The oximeter import dialog states it plainly: OSCAR is
compatible with the serial CMS50 models, and "Direct importing from bluetooth models is **probably not**
possible yet". A source-tree search for a Bluetooth or BLE stack returns nothing. The "live streaming
mode" strings in the CMS50 and MD300W1 loaders refer to switching the *device* into a transfer mode over
the cable so its stored buffer can be downloaded — not to real-time collection.

Implications to carry into the comparison:

- OSCAR's data is at minimum one SD-card removal old, and typically one sleep period old.
- OSCAR cannot participate in anything that must happen *during* the night or *at the moment* of an
  event. It has no alarm, no notification and no phone.
- OSCAR's "align data from oximeters" feature exists precisely because the CPAP and the oximeter are
  separate devices with separate clocks, read at separate times. AnxietyWatch2's collector has the same
  problem and solves it by streaming both into one UTC timeline.

---

## 9. Analysis, reporting and the subjective layer

**What is graphed and tabulated.** High-resolution waveforms (flow, pressure, leak, snore, and device
diagnostic channels) are presented "down to a breath by breath level", alongside per-day and per-session
statistics. Named channels in `machine_common.h` include AHI, apnoea, clear-airway apnoea, hypopnoea
(with obstructive/central split in 2.0), RERA, flow limitation, periodic breathing, Cheyne–Stokes
respiration (CSR), leak flag, expiratory pressure, NRI, VS (vibratory snore), and Steady Breathing with
its flag.

**Statistics.** The 2.0 schema materialises **35 statistics per day** into a `daily_summaries` cache,
"20–100× faster" than computing on demand, plus a session-level summary cache. These are derived values
with a cache-invalidation key (`sessions_hash`).

**Leak percentile is computed, not read.** OSCAR recomputes leak p50/p95 from the pressure/leak
time-series rather than persisting the device's own reported `Leak.95`. This is deliberate in the code
(the device-reported percentile persistence is commented out) and it produces a documented disagreement
with the manufacturer — see `oscar_IMPLEMENTATION_INSPECTION.md` §5.2 for the reproduced 2.0× case and
the gain-handling defect it found.

**Subjective layer.** OSCAR is not purely objective. The Daily Notes panel carries a **"feelings" meter
with a range of 0–100**, journal notes, weight, and bookmarks. Notes can be exported as a report. Journal
entries are also importable. This is the same *shape* as AnxietyWatch v1's subjective anxiety entries —
but entered at a desk during review, not captured at the moment of the event, and not scored or
correlated by OSCAR.

**Reporting.** A report tree (system reports + user-created, arbitrarily nested), a printed/PDF daily and
statistics report, and templated CSV export at three aggregation levels: **Details** (DateTime, Session,
Event, Data/Duration), **Summary** (Date, Session Count, Start, End, Total Time, AHI, then per-channel
counts, averages, P90s and maxima), and **Sessions** (the same without the session count).

**The CSV vocabulary is the interoperability boundary that matters for this project.** The v1 importer's
expected header string `Date,Session Count,Start,End,Total Time,AHI` is byte-for-byte the header OSCAR's
exporter writes for the Summary mode. That correspondence is verified against both trees in
`oscar_IMPLEMENTATION_INSPECTION.md` §5.1.

---

## 10. Interoperability claims

This is where OSCAR 2.0 is unusually generous, and where it stands apart from every closed candidate in
this shortlist:

- **"Access to all OSCAR data from other programs."** The application data is an SQL database on the
  user's own disk, readable by any SQLite client. There is no API gate and no account.
- **Published SQL schema documentation** in `Notes/Database/`: `DATABASE_SCHEMA.md` (616 lines),
  `DATABASE_SCHEMA_REFERENCE.md` (1,398 lines), `SCHEMA_DESIGN_PHILOSOPHY.md` (282 lines) and
  `USEFUL_QUERIES.sql`.
- **A Python reference reader**: `Notes/Accessing OSCAR Data/oscar_waveform_demo.py` with a companion
  `python_waveform_demo_spec.md`, and a second copy under `Notes/Waveform Demo/`.
- **CSV export with user-configurable templates.**
- **An application-data export for the details level**: `Notes/Database/oscar_details_csv.py`.
- **A URI scheme for restore** (`Notes/Developer Notes/URI_SCHEME_RESTORE.md`).
- **Profile sharing** by file, by direct upload to the user's own Dropbox/Google Drive, or from a URL.

Three of these claims were confirmed in the tree (schema docs, Python demo, CSV templates). None was
executed, so all are recorded as **documented, not verified**. The distinction matters: a published
schema description is a contract, and this project does not treat a contract as observed behaviour.

Note also what the sharing feature is *not*: it is not a vendor service. Dropbox and Google Drive are the
**user's own accounts**, used as transport. There is no OSCAR server holding user data.

---

## 11. Time handling

Two mechanisms, both relevant to AnxietyWatch's UTC-only rule.

1. **A reversible correction ledger.** 2.0 replaced "CPAP Clock Drift" with a **Data/Time Corrections**
   facility. The schema's `device_time_corrections` table stores, per correction: a date range, a
   `type` constrained by a CHECK clause to `('timezone','travel','dst','reset','offset','drift')`, an
   `offset_ms`, a linear drift model (`c0_ms` intercept and `c1` rate), a free-text `reason`, and both
   `applied_at` and `undone_at` timestamps. Corrections are **typed, reasoned, auditable and
   reversible** — they are not silent rewrites of stored timestamps.

2. **A day-boundary convention that v1 has already adopted.** OSCAR groups a night noon-to-noon, so a
   04:00 session belongs to the previous day. AnxietyWatch v1's tests assert this exact behaviour and
   attribute it to OSCAR. Verified in the SQL — the session report derives the day as
   `date(s.start_time/1000 - 43200, 'unixepoch', 'localtime')`, i.e. the stored epoch value minus
   exactly 12 hours, rendered in the machine's local timezone (`oscar/docs/system_reports.orf:154`).

**Correction to a common assumption about (1) and (2):** session times are stored as **epoch
milliseconds**, not as device-local wall-clock strings. The store is therefore epoch-based, and the
local-date rendering happens at query time — which is closer to AnxietyWatch2's rule than a naive
reading suggests. The real difference is *where the epoch value came from*: OSCAR computes it from the
**CPAP device's own clock**, so a device whose clock was wrong produces a wrong epoch value, and the
correction ledger exists to compensate after the fact. AnxietyWatch2 derives its epoch seconds from the
collector's clock at ingest. That is a real modelling difference — and the reason the two systems cannot
share a storage layer even if they agree on where a night begins.

---

## 12. Privacy and network posture

**No telemetry of any kind was found in the tree.** A search for `telemetry`, `analytics`,
`google-analytics`, `sentry`, `crashreport`, `mixpanel`, `amplitude` and `firebase` across all C++ and
project files returned a single unrelated prose match in a loader comment. There is no crash reporter, no
usage analytics, no update-ping, and no vendor endpoint.

The only network code in the tree is `oscar/network/`: OAuth2 handlers and uploaders/downloaders for
**Dropbox, Google Drive and OneDrive**, plus a generic cloud uploader — all user-initiated, all pointing
at the user's own storage accounts. Two privacy details worth noting:

- 2.0 **removed profile password support**. Profiles are therefore not password-protected at rest. The
  database is a plain SQLite file; protection is whatever the operating system and disk encryption
  provide.
- The UI explicitly strips the profile name from the window title **when taking a screenshot**, so a
  screenshot shared in a forum does not carry a name.

`crypto.cpp` implements AES-256 and AES-256-GCM via the bundled Botan library, but it is called from
`prs1_loader.cpp` to **decrypt the Respironics SD-card payload**, not to protect the user's database.
Some Respironics cards ship encrypted with keys the sleep community long ago published; OSCAR carries
those keys so a user can read the data from their own machine. This is a factual, static observation. It
is **not** an evaluation of the legality or propriety of that, which is not something this research is
competent to decide.

---

## 13. Relationship to AnxietyWatch — the section that changes the framing

Verified by reading both trees on 2026-10-06. OSCAR is upstream of AnxietyWatch v1.

| Evidence | Location |
|---|---|
| `REQUIREMENTS.md` describes the CPAP path as "SD card → custom parser (SQLite/EDF files, reverse-engineered by OSCAR community)" | `AnxietyWatch/REQUIREMENTS.md:52` |
| A checklist item: "CPAP SD card data parser (based on OSCAR format documentation)" | `AnxietyWatch/REQUIREMENTS.md:296` |
| A first-class session source: `session.source == .oscar` | `AnxietyWatch/AnxietyWatchTests/ModelEnumTests.swift:29–51` |
| A dedicated importer: `CPAPImporter.isOSCARBySessionFormat` | `AnxietyWatch/AnxietyWatchTests/CPAPBySessionImportTests.swift:42` |
| Format-routing test: "Routes OSCAR Summary header to CPAP importer" — and the header is the one OSCAR writes | `CSVImportRouterTests.swift:97` |
| Day-boundary test: "OSCAR's noon-to-noon convention, so a 04:00 session belongs to the [previous day]" | `CPAPBySessionImportTests.swift:97` |
| Scoring-gap test: a "blip OSCAR didn't score. Its hours must NOT enter the AHI" | `CPAPBySessionImportTests.swift:162` |
| `importSource: "oscar"` round-trips through report generation, server restore and reconciliation | `ReportGeneratorTests.swift:463`, `RestoreFromServerTests.swift:169` |

So v1's CPAP understanding — the day boundary, the CSV vocabulary, the event-scoring semantics, and the
notion that some hours are unscored and must stay out of AHI — is **inherited from OSCAR**. Two
consequences:

1. **OSCAR's defects are v1's defects.** The leak-percentile method review's reproduced 2.0× divergence
   from the device's reported value, and the confirmed gain-handling inconsistency in `Day::percentile`,
   sit upstream of any leak-based conclusion v1 draws. This is a genuine, evidence-backed finding about
   v1's CPAP data, not a criticism of OSCAR's users.
2. **V2 does not inherit them.** AnxietyWatch2 reads the AirSense 11 directly over BLE and decodes with
   its own pure `fn decode_x(buf: &[u8]) -> Option<T>` decoders, so the AirSense 11 path shares no code
   with OSCAR. The overlap is the *concept* of AHI and leak, not the implementation.

One interoperability fragility is worth recording because it is checkable: OSCAR's **dialog-exporter**
CSV headers are **translated** (`tr("Date")`, `tr("Session Count")`, …). In a non-English OSCAR the
header is localised, while v1 detects format by matching the English strings. The separator, by contrast,
is a hardcoded `,`. So v1's OSCAR-CSV import is expected to work for English OSCAR and to fail header
detection for a localised one. This was read in the exporter source; it was not executed against a
localised build, so it is recorded as a static finding with the mechanism named, not as an observed
failure. (The by-session report's aliases are fixed identifiers, so that path is not exposed to it.)

### 13.1 A dependency on column *position*, and a header that has already grown

This is the finding that gives the dependency real teeth, and it was verified from both trees.

v1's by-session parser receives the split fields **without a header** and maps them by hardcoded index
(`CPAPImporter.swift:709–765`). Its format detector is a six-token prefix, and its only structural
validation is a row-length test:

```swift
// detector
hasPrefix("date,start,ahi,rdi,oa")
// validation
guard fields.count >= 21 else { return .skip(reason: "expected at least 21 columns") }
```

v1's own comment states the contract it was written against: *"OSCAR 'by session' export: one row per
mask-on/mask-off session, 22 columns."* The 22 columns are listed in OSCAR's design record for the
report 2.0.1 shipped, and they match v1's expected header token for token, in order.

**The header is now 27 columns in the inspected OSCAR commit.** `OAHI`, `CAHI`, `A`, `OH` and `CH` were
inserted mid-header by the central/obstructive hypopnea work. Against that header:

- the detector still fires, because `OAHI` begins with `OA`;
- the `count >= 21` guard still passes, because `27 >= 21`;
- and every field from index 4 onward is silently misassigned — v1 reads the hypopnea *count* as
  `pressureMean`, a pressure as `leakAvg`, a pressure as `spo2Avg`, and **`Leak_95th` as `Hours_Used`**,
  so a night of 6.5 hours with a leak 95th of 12.4 becomes 12.4 hours of therapy, with no warning.

**Timing, stated with its uncertainty.** v1 is correct against OSCAR **2.0.1**, the release it was written
against. The break arrives with the next OSCAR release. That the 27-column form is unreleased is an
**inference** from three facts — the specification that introduced it is dated 2026-08-06, after the
2026-06-23 release; the report file's own header note states these columns "arrived with schema v18"
while `CURRENT_SCHEMA_VERSION` is 19; and the 2.0.1 release notes mention no central/obstructive work —
not from running the 2.0.1 binary. The latent-break conclusion holds either way.

The full column-by-column mapping is in `oscar_IMPLEMENTATION_INSPECTION.md` §5.1.3.

### 13.2 Documentation drift is not confined to the report header

One further, smaller instance, recorded because it is the same failure mode: OSCAR's published schema
philosophy states that waveform BLOBs use "qCompress level 9" for "40-60% compression"
(`Notes/Database/SCHEMA_DESIGN_PHILOSOPHY.md:66`, and again at `:14`, `:30`, `:85`), while the inspected
code calls `qCompress(data, 6)` with an explicit rationale — "6 is ~2x faster than 9 with only ~1-2%
larger output" (`oscar/database/event_data_repository.cpp:531`, gated on data ≥ 500 bytes and a >10%
reduction, `:526–541`).

Nothing breaks because of this: the compression level is recorded per BLOB and both ends agree on how to
decompress. It is recorded for two reasons. First, it shows that a discrepancy between OSCAR's
*description* of its output and OSCAR's *actual* output can persist unnoticed in the same repository —
which is the same class of event as §13.1, at a place where nothing enforced agreement. Second, it is a
consumer-facing divergence of exactly the kind that matters to v1: a documented format property that a
consumer could reasonably have relied on, which the implementation has since changed.

---

## 14. Analysis limitations OSCAR states about itself

These are the vendor's own words or code comments, and they are more conservative than its feature list.
Where they conflict with anything in the marketing copy, the conservative statement governs, per this
project's evidence rules.

| Limitation | Source |
|---|---|
| Most CPAP machines cannot distinguish a true central apnoea from an unclassified one, so the count is labelled `unclassified_count`, not `central_count` | `SCHEMA_DESIGN_PHILOSOPHY.md`, schema v10 |
| Percentiles are recomputed from the time-series rather than read from the device, so "disagreement with ResScan/MyAir is possible (and observed)" | `LEAK_PERCENTILE_METHOD_REVIEW_2026-04-16.md` |
| `Day::percentile` "assumes gain is identical across sessions"; if gains differ it "can mis-scale values" — a "real defect candidate" | same review, item C |
| The waveform branch of `updateCountSummary()` "can over-accumulate time across multiple waveform eventlists" | same review, item D |
| Some sessions carry summary data only, with no waveform — the `sessions.summary_only` flag | `database_schema.cpp` |
| Upgrading schema versions may require a **fresh database and reimport**; incremental migration is no longer supported | `SCHEMA_DESIGN_PHILOSOPHY.md`, schema v12 |
| Wireless oximeters are "probably not" supported | `oscar/oximeterimport.cpp` |

That last-but-one item deserves emphasis: OSCAR's stated position is that a schema change may invalidate
the user's database, with the device as the backup. For a tool whose purpose is long-term record review,
that is a genuine durability risk — and it is the exact opposite of AnxietyWatch's append-only rule.

---

## 15. Evidence table

| ID | Claim | Source | Class | Date | Confidence |
|---|---|---|---|---|---|
| O-1 | OSCAR is free and "never asks for payment" | sleepfiles.com/OSCAR/ | public claim | 2026-10-06 | High |
| O-2 | GPL v3 licence | `COPYING` in `CrimsonNape/oscar-sql` | static finding | 2026-10-06 | High |
| O-3 | Derivative of SleepyHead 1.1.0; (C) 2011–2018 Watkins, 2019–2026 OSCAR Team | `README` | static finding | 2026-10-06 | High |
| O-4 | v2.0.1 released 2026-06-23 | sleepfiles.com/OSCAR/ | public claim | 2026-10-06 | High |
| O-5 | 2.0.1 source is `gitlab.com/CrimsonNape/oscar-sql` | sleepfiles.com/OSCAR/ | public claim | 2026-10-06 | High |
| O-6 | 1.x mainline is `gitlab.com/pholy/OSCAR-code`, a SleepyHead fork, 189★/112 forks | GitLab API | static finding | 2026-10-06 | High |
| O-7 | The 2.0 SQL repo has 10★/13 forks, created 2026-06-07 | GitLab API | static finding | 2026-10-06 | High |
| O-8 | Nineteen loaders registered; CPAP, oximeter and Apple Health families | `oscar/main.cpp:1267–1285` | static finding | 2026-10-06 | High |
| O-9 | **Apple Health loader parses sleep stages, HR, SpO2, respiratory rate, HRV SDNN, breathing disturbances, wrist temperature, body mass** | `applehealthDataParsing.cpp:41–65` | static finding | 2026-10-06 | High |
| O-10 | All ingestion is post-hoc: SD card, USB/serial cable, file, shared profile | download page; `oximeterimport.cpp` | static finding | 2026-10-06 | High |
| O-11 | **No Bluetooth/BLE stack anywhere in the tree** | source search, 2026-10-06 | static finding | 2026-10-06 | High |
| O-12 | UI states bluetooth oximeter import is "probably not" possible | `oscar/oximeterimport.cpp:1196` | static finding | 2026-10-06 | High |
| O-13 | Application data is an SQL database; schema documented in `Notes/Database/` (616 / 1,398 / 282 lines) | `Notes/Database/*` | static finding | 2026-10-06 | High |
| O-14 | A Python reference reader and spec are published in-tree | `Notes/Accessing OSCAR Data/` | static finding | 2026-10-06 | High |
| O-15 | Waveforms are stored as compressed BLOBs with a CRC16 checksum. **Code and design doc disagree on the level:** the schema doc says "qCompress level 9 achieves 40-60% compression", the inspected code calls `qCompress(data, 6)` | `SCHEMA_DESIGN_PHILOSOPHY.md:66` (vendor statement) vs `event_data_repository.cpp:526–540` (static finding) | vendor statement + static finding | 2026-10-06 | Medium |
| O-16 | `daily_summaries` materialises 35 statistics per day | `SCHEMA_DESIGN_PHILOSOPHY.md` v6 | static finding | 2026-10-06 | High |
| O-17 | **ResMed EDF path substitutes the previous sample for the end-of-data sentinel (`c = last`)** | `resmed_loader.cpp:4360–4368` (`c=last` at `:4365`) | static finding | 2026-10-06 | High |
| O-18 | **Apple Health path rejects out-of-range values, names the reason, and skips** | `applehealthDataParsing.cpp:337–446` | static finding | 2026-10-06 | High |
| O-19 | Apple Health path comments "0 and negatives are no-reading sentinels, not measurements" | `applehealthDataParsing.cpp:358` | static finding | 2026-10-06 | High |
| O-20 | Apple Health path converts units at parse time (degF/K→degC, lb/st/g/oz→kg) and rejects unknown units | `applehealthDataParsing.cpp:402–437` | static finding | 2026-10-06 | High |
| O-21 | Malformed-value count is capped at 10 stderr lines and **not** carried into the import summary | `applehealthDataParsing.cpp:230–236`; `applehealth_loader.h:26–37` | static finding | 2026-10-06 | High |
| O-22 | Vitals from non-Apple, non-chosen sources are dropped and counted per source (`ignoredVitalsCounts`) | `applehealth_loader.cpp:227–243` | static finding | 2026-10-06 | High |
| O-23 | Sleep source is auto-selected deterministically: most intervals, alphabetical tie-break | `applehealthDataParsing.cpp:158–177` | static finding | 2026-10-06 | High |
| O-24 | Re-importing an already-present night is skipped, per night | `applehealth_loader.cpp:404–429` | static finding | 2026-10-06 | High |
| O-25 | An assumption: "Stages are published only after a night ends, so the newest night is never partial" | `applehealth_loader.cpp:364` | static finding | 2026-10-06 | High |
| O-26 | Device time corrections are typed (timezone/travel/dst/reset/offset/drift), reasoned, and reversible (`undone_at`) | `database_schema.cpp:1519–1533` | static finding | 2026-10-06 | High |
| O-27 | Sessions carry a `summary_only` flag for devices that supply no waveform | `database_schema.cpp:680–694` | static finding | 2026-10-06 | High |
| O-28 | Central apnoea counts were renamed to `unclassified_count` for semantic accuracy | `SCHEMA_DESIGN_PHILOSOPHY.md` v10 | static finding | 2026-10-06 | High |
| O-29 | OHI/CHI are printed only when the day has OH/CH data, "rather than printing `0.00` for every other device" | `Notes/specs/2026-08-06-central-obstructive-hypopnea-design.md` | static finding | 2026-10-06 | High |
| O-30 | Leak p95 is recomputed, not read from the device; a 2.0× disagreement with the device value was reproduced | `LEAK_PERCENTILE_METHOD_REVIEW_2026-04-16.md` | static finding | 2026-10-06 | High |
| O-31 | `Day::percentile` has a confirmed gain-handling defect risk; the profile-level path normalises per session | same review, item C | static finding | 2026-10-06 | High |
| O-32 | `updateCountSummary()` waveform branch can over-accumulate time across eventlists | same review, item D | static finding | 2026-10-06 | High |
| O-33 | A schema mismatch may require a fresh database and reimport | `SCHEMA_DESIGN_PHILOSOPHY.md` v12 | static finding | 2026-10-06 | High |
| O-34 | A 6,748-line dated bug log is published in-tree | `Notes/Developer Notes/BUG_FIXES.md` | static finding | 2026-10-06 | High |
| O-35 | **No telemetry/analytics/crash reporting of any kind** | source search, 2026-10-06 | static finding | 2026-10-06 | High |
| O-36 | Network code is limited to user-owned Dropbox/Google Drive/OneDrive via OAuth2 | `oscar/network/` | static finding | 2026-10-06 | High |
| O-37 | Profile password support was removed in 2.0 | download page, Profiles section | public claim | 2026-10-06 | High |
| O-38 | Screenshots strip the profile name from the title bar | download page, UI section | public claim | 2026-10-06 | High |
| O-39 | AES-256/256-GCM is used to decrypt Respironics SD-card payloads, not to protect the database | `crypto.cpp`; `prs1_loader.cpp:371–425` | static finding | 2026-10-06 | High |
| O-40 | CSV export headers are localised via `tr()`; separator is a hardcoded `,` | `exports/exportcsv.cpp:189–217`; `sep` at `:160` | static finding | 2026-10-06 | High |
| O-41 | Days with no CPAP data are skipped entirely — no rows, no zero fill | `exports/exportcsv.cpp:230` | static finding | 2026-10-06 | High |
| O-42 | A test suite exists (473 assertions in 14 files; 340 in the Apple Health file alone) | `oscar/tests/` | static finding | 2026-10-06 | High |
| O-43 | **No CI configuration exists**; the test build is opt-in via `CONFIG+=test` | repo search; `oscar/oscar.pro:800–828` | static finding | 2026-10-06 | High |
| O-44 | A "Purge Range of Days" action deletes a contiguous date range | `Notes/specs/2026-05-16-purge-range-of-days-design.md`; `purgerangedaysdialog.cpp:24` | static finding | 2026-10-06 | High |
| O-45 | AI is used as a **development** aid only; no AI feature exists in the product | `Notes/AI Aids/` (Claude/code-review notes) | static finding | 2026-10-06 | High |
| O-46 | AnxWatch v1's CPAP path is documented as based on OSCAR's reverse engineering | `AnxietyWatch/REQUIREMENTS.md:52,296` | static finding | 2026-10-06 | High |
| O-47 | v1 has `source == .oscar` and an `isOSCARBySessionFormat` importer | v1 tests | static finding | 2026-10-06 | High |
| O-48 | v1's expected Summary CSV header equals OSCAR's exporter header | both trees | static finding | 2026-10-06 | High |
| O-49 | v1 adopts OSCAR's noon-to-noon day boundary | `CPAPBySessionImportTests.swift:97` | static finding | 2026-10-06 | High |
| O-50a | The `Session Summaries/by Session` report shipped by 2.0.1 had exactly the 22 columns v1 expects, in order | `Notes/specs/2026-06-10-csv-report-fields-design.md` column table; v1 `CPAPImporter.swift:291` ("22 columns") | static finding | 2026-10-06 | High |
| O-50b | The report header in the inspected commit has **27** columns; `OAHI`, `CAHI`, `A`, `OH`, `CH` were inserted mid-header | `oscar/docs/system_reports.orf:145–179` (aliases) | static finding | 2026-10-06 | High |
| O-50c | v1's format detector still matches the widened header, because `OAHI` begins with `OA` | v1 `CPAPImporter.swift:293–295` vs the ORF column list | static finding (not executed) | 2026-10-06 | High |
| O-50d | v1's parser is positional with a `count >= 21` guard only, so a widened header is accepted and misassigned | v1 `CPAPImporter.swift:709–765` | static finding | 2026-10-06 | High |
| O-50e | The 27-column form postdates the 2.0.1 release | 2026-08-06 spec date vs 2026-06-23 release; `system_reports.orf:14` — the columns "arrived with schema v18" while `CURRENT_SCHEMA_VERSION = 19`; 2.0.1 notes mention no hypopnea/OAHI work | **inference** | 2026-10-06 | Medium |
| O-50f | OSCAR's day report stored a p90 under the name `Pressure_95th`, and the vendor relabelled it `Pressure_90th`; the session report is a genuine p95 | `Notes/specs/2026-06-10-csv-report-fields-design.md` ("Percentile finding") | vendor statement | 2026-10-06 | Medium |
| O-50g | Absent oximetry is stored as `0.0`, masked to blank only by `NULLIF` in the report SQL; `Leak_Unint_Avg` is deliberately **not** masked | same design doc ("Oximetry blanking"); `system_reports.orf:175,177` vs `:57` | static finding + vendor statement | 2026-10-06 | High |
| O-50h | Sessions are stored as epoch **milliseconds**; the local day is derived at query time | `system_reports.orf:154` (`s.start_time/1000 - 43200`, `'unixepoch','localtime'`) | static finding | 2026-10-06 | Medium |
| O-50 | OSCAR runs on RasPiOS ARM64 (same hardware class as the V2 collector) | download page | public claim | 2026-10-06 | High |

No runtime observation was made for OSCAR. Nothing in the tree was compiled or executed; see
`oscar_IMPLEMENTATION_INSPECTION.md` §6.

---

## 16. What OSCAR does not claim and does not do

Recorded explicitly, because several of these are the axes on which AnxietyWatch differs.

- **No medical device claim.** OSCAR displays; it does not diagnose or treat. Its stated purpose is to
  produce something to discuss with a doctor.
- **No live or real-time anything.** No streaming, no alarm, no notification, no phone.
- **No biosignal beyond CPAP and oximetry.** No ECG, no HRV from a chest strap, no actigraphy, no
  environmental sensors, no weather.
- **No AI in the product.** Statistics and graphs only.
- **No mobility.** No iOS or Android client.
- **No cloud service.** The user's own storage accounts are transport only.
- **No account, no licence server, no activation.**
- **No correlation engine.** OSCAR presents CPAP and oximetry channels on a shared timeline. It does not
  test whether one thing predicts another, and it offers no experiment or N-of-1 facility.
- **No medication concept.**
- **No self-hosting offer** — in the sense that it is not a service anyone could self-host. It is a
  local program, which is a stronger property along the same axis.

---

## 17. Unverified scope

Stated plainly so downstream prompts do not treat these as settled:

- **Nothing was executed.** No OSCAR binary was run, no data imported, no report generated, no CSV
  inspected as output. Every OSCAR finding in this package is a static reading of published source,
  public documentation, or the vendor's own claims.
- **The 1.7.2 line was not inspected.** Research concentrated on the 2.0 SQL tree because that is the
  current release and the one whose data model matters.
- **The unmigrated loaders** (`mseries`, `dreem`, `somnopose`, `zeo`, `serialoximeter`) were not
  confirmed as active in 2.0.
- **No download, install base or active-user figure was found.** Community star/fork counts are the only
  scale signals, and they are not user counts.
- **The localised-CSV-header fragility** (§13) is a static reading and was not reproduced.
- **The leak-percentile review is an internal document**, dated 2026-04-16. It was not verified against a
  current build, and some of the defects it names may have been fixed since. What is certain from the
  tree is the *method* — percentiles recomputed from the time-series rather than read from the device.
- **Respironics decryption** was observed as code and is reported as such. Its legal status was not
  assessed and is out of scope.
- **Which header OSCAR 2.0.1 emits was not confirmed by running it.** The 22-column form comes from the
  vendor's design record for the report that release shipped; the 27-column form is read from the
  inspected commit. The conclusion that the widened form postdates 2.0.1 is an inference (§13.1).
- **v1's detector was not executed against the 27-column header.** The prefix match follows from string
  comparison and is stated as a static reading.
- **The `0.0` oximetry sentinel was not traced end to end** through a real v1 import. The `NULLIF`
  masking is read from the report SQL.
- **Only three OSCAR invalid-data paths were examined** (ResMed EDF, Apple Health, the summary store).
  The other seventeen loaders were not checked for the same property, so the three-policy finding is not
  a claim about OSCAR as a whole.

## Related files

- `oscar_IMPLEMENTATION_INSPECTION.md` — the full-source inspection, claim reconciliation, reproduction
- `PROMPT_2.md` — the candidate-specific second-stage prompt
- `oscar_COMPARISON.md` — the comparison against AnxietyWatch v1 and v2
- `../CANDIDATE_SHORTLIST.md`, `../PROCESS_STATE.md`, `../RESEARCH_METHOD.md`
