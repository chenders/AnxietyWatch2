# Nightscout versus AnxietyWatch v1 and AnxietyWatch2

Candidate 6 of 10. Research date **2026-10-06**.
Base variant `PROMPT2-TECHNICAL-SYSTEMS.md` + `PROMPT2-PRIVACY-AND-OPERATING-MODE.md`.
Companions: `nightscout_CLAIMED_FEATURES_AND_PURPOSE.md`, `nightscout_IMPLEMENTATION_INSPECTION.md`,
`PROMPT_2.md`.

**Evidence classes in this report.** `[PUBLIC CLAIM]` the project's own published statement ·
`[VENDOR DOC]` a document the project authors and ships in its own repository (audit, requirements, design
proposal) — the vendor's statement about itself, not independent validation · `[STATIC]` read from the
published source tree at the pinned commit · `[INFERENCE]` my reasoning, labelled · `[UNKNOWN]` no evidence
found. **There are no runtime observations in this report.** Nightscout was never built, installed,
started or contacted; no instance, public or private, was requested; no account, token or key was created.

**AnxietyWatch status labels.** `[V1 impl]` exists in `../AnxietyWatch` today · `[V2 current]` exists in
`./` today · `[V2 target]` fully specified in `openspec/`, not shipped · `[V2 unwired]` present and tested
but read by no consumer · `[unresolved]` no evidence either way.

**Pinned Nightscout revision for every `[STATIC]` finding:**
`92d0834219aa771b5837dbcbf1baeb839a200cf6` — the release commit of **v15.0.8**, 2026-09-04. Any statement
below about Nightscout's code is a statement about that revision and nothing else.

---

## 1. Executive finding

**Nightscout is not a competitor to either AnxietyWatch version. It is the closest existing model for the
platform layer AnxietyWatch2 is building, and its failures are more instructive than its features.**

The shortlist placed Nightscout at #6 because its architectural overlap is high and its clinical overlap is
near zero. Reading the source confirms both halves of that, and sharpens what the overlap actually is:

- **Where the overlap is real:** a self-hosted service that continuously ingests physiological data from a
  wireless device, stores it without loss, derives statistics, shares it with a chosen audience, raises
  alarms, and prunes old data. AnxietyWatch2 is building that, with a different device and different
  physiology.
- **Where it is not real:** Nightscout's physiology is glucose and insulin. It has no CPAP, no SpO₂, no
  respiratory rate, no sleep depth, no anxiety, no panic, no medication adherence, no weather, no aircraft
  noise, and no environmental context of any kind. It is also not a mobile app — it publishes no app, and
  therefore has no push channel of its own.

**The comparison produces one result that is stronger than the rest, and it is not a feature comparison.**

AnxietyWatch2's rules — *one canonical unit per type, converted once at the decoder*; *three states, and
"not measured" is never rendered as measured*; *reject, don't clamp*; *the registry is the single source of
truth for ranges and thresholds*; *append-only, tombstone for removal*; *never `locf()`/`interpolate()`* —
are each answered, in the pinned Nightscout source, by a concrete counter-instance:

| AnxietyWatch2 rule | Nightscout at v15.0.8 | Evidence |
|---|---|---|
| One unit, converted once at the decoder | Report series is scaled into the user's **display unit** and then fed to an A1c formula written for mmol/L. The release shipped **95.8% instead of 6.9%** for mg/dL users, and the project's own documentation says so. | `[STATIC]` `reportclient.js:847`, `dailystats.js:111`; `[PUBLIC CLAIM]` 15.0.8 report docs |
| Three states; "not measured" is never rendered as measured | `//assume current` for an absent reading; `// bless absence with known type` for an absent type; four of six severity levels render as `current`; the glucose alarm cannot fire without a current reading, and the stale-data alarm returns early when there is no reading to be stale | `[STATIC]` `timeago.js:26,88,101–104`, `entries/index.js:66–69`, `levels.js:26–48`, `simplealarms.js:21` |
| Reject out-of-range; never clamp | No physiological range check exists anywhere in the write path — `identifier` structure, `date`, `utcOffset` and `app` are the only validations. The predictive alarm clamps its forecast into [36, 400] before display. | `[STATIC]` `api3/generic/create/validate.js:15–24`, `api3/shared/operationTools.js:56–75`, `plugins/ar2.js:308` |
| Registry is the single source of truth for ranges and thresholds | Alarm thresholds and the AR coefficients are file literals; the user's alarm settings rename the event but do not move the trigger | `[STATIC]` `plugins/ar2.js:5–11,194–209,211–224` |
| Append-only; removal is a tombstone | API3 soft-deletes and keeps history; API v1/v2 and the admin tools hard-delete; the optional auto-prune hard-deletes and its `$or` filter can reach a recently modified document | `[STATIC]` `api3/generic/delete/operation.js:20–24,80`, `server/entries.js:52`, `api3/generic/collection.js:129–160` |
| Never `locf()`/`interpolate()` | Foreground display tolerates absence by treating it as current (`//assume current`); auto-prune removes rather than gaps | `[STATIC]` as above |

**And the comparison produces a second, less comfortable result for AnxietyWatch2.**

Nightscout has seventeen years of field deployment in real homes, through real phone-battery and
connectivity failures, with a documented deployment path for users who cannot run a server (a ten-provider
paid market), a public versioned API description, a published requirements-and-test-specification
programme, and an explicit "do not rely on this alone" posture stated to every user. AnxietyWatch2 has none
of those things yet, and no specification substitutes for them. **The rules that Nightscout violates are
the rules AnxietyWatch2 wrote in response to exactly these failure modes — but Nightscout is the one with
the operational record.** That is the honest framing of this comparison, in both directions.

**What the owner can take from it:**

1. **The six rules above are validated, not merely asserted.** Each has a verifiable counter-instance in a
   mature, widely deployed system, and two of them (units, absent data) have *user-visible consequences*
   the project itself documented. This is the strongest external evidence in the package that the rules
   address real failure modes rather than aesthetic preferences.
2. **The "no account, no cloud dependency" position is defensible on operational grounds,** not only on
   privacy grounds: Nightscout's own documentation states that a working internet connection and
   third-party cloud services are required, and its recommended alerting path needs IFTTT, Google and
   Pushover accounts. A local alerting path is a genuinely different product, not a philosophical
   preference.
3. **Three things Nightscout does that AnxietyWatch2 does not, and probably should consider:** a
   `history`/delta-sync surface for clients; a public machine-readable API description; and an explicit
   non-dependence disclaimer shown to users.
4. **One thing to be careful about:** Nightscout's abandonment of a specific rule did not stop it from
   functioning for thousands of people. Any claim that AnxietyWatch2's rules are *necessary* for a working
   system is contradicted by the evidence. They are necessary for **not producing a wrong number
   downstream**, which is a narrower and more defensible claim.

---

## 2. Product and availability

| | Nightscout | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| Form | Self-hosted web application (`[PUBLIC CLAIM]`) | iOS app (`[V1 impl]`) | Rust daemon + iOS client (`[V2 current]` client-side, daemon on a Pi) |
| Physiology | glucose, insulin, carbs, pump/loop status | anxiety/panic EMA, CPAP CSV import, HealthKit | AirSense 11 BLE, SpO₂ (EMAY SleepO2), Polar H10 HR/RR, Acconeer XE125 (`[V2 current]`/`[V2 target]` per change) |
| Store | MongoDB, document shapes per collection | HealthKit + optional self-hosted sync | TimescaleDB `samples` hypertable + aggregates |
| First-party mobile app | **none** (`[PUBLIC CLAIM]`) | yes | iOS client `[V2 current]` |
| Account | none in the software | none | none |
| Cost | free; ten advertised hosting providers (`[PUBLIC CLAIM]`) | free | free |
| Licence | **AGPL-3.0** | proprietary/closed | private |
| Steward | volunteers; Nightscout Foundation 501(c)(3) funds the ecosystem (`[PUBLIC CLAIM]`) | owner | owner |
| Version inspected | 15.0.8, 2026-09-04 | current tree | current tree |
| Inspection basis | public AGPL source, pinned commit | repository read | repository read |
| Runtime inspected | **no** | n/a | n/a |

**The form difference is the whole comparison.** Nightscout asks the user to operate a server; AnxietyWatch
v1 asks the user to install an app; AnxietyWatch2 asks the user to operate a small box. Nightscout's answer
to "what if the user cannot operate a server" is a paid hosting market. AnxietyWatch2 has not needed to
answer that question yet, and the honest reading of Nightscout's history is that it is a real question with
a real cost.

---

## 3. System boundary and architecture

### 3.1 Nightscout at v15.0.8

```
 uploaders (3rd-party apps)        browsers / followers        3rd parties
      |                                  |                        |
      |  HTTP: API v1 / v2 / v3          |  HTTP + WebSocket       |  IFTTT webhooks (ns-event)
      v                                  v                        v
 +-------------------------------------------------------------------------+
 |  Node/Express process                                                    |
 |    authorization: roles, subjects, tokens (SHA-1 digest), JWT (8h, key    |
 |                    from a generated randomString file, not API_SECRET)    |
 |                                                                          |
 |    lib/api (v1)      lib/api2 (v2)      lib/api3 (v3, "3.0.5")            |
 |       |                   |                |                             |
 |       |                   |     generic pipeline per collection:          |
 |       |                   |       create -> validate(shape only) -> insert |
 |       |                   |       -> autoPrune (opt-in, hard delete)      |
 |       |                   |       read / search / update / replace / patch |
 |       |                   |       delete -> markAsDeleted (isValid=false) |
 |       |                   |       history -> srvModified delta sync       |
 |       |                   |                                              |
 |    purifier: storage-purifier (v1/v2) + writePurifier (v3)                |
 |                                                                          |
 |    bus: tick -> data-loaded -> notifications.checkNotifications (plugins) |
 |         admin-notify -> adminnotifies                                    |
 |    plugins: ar2 (predictive), simplealarms, timeago, bgnow, webhook, ...  |
 |    sockets: storageSocket, alarmSocket (namespace-wide emit, no rooms)    |
 +-------------------------------------------------------------------------+
                                    |
                                    v
                    MongoDB: entries / treatments / profiles /
                    devicestatus / food / activity / settings
                    (application-level retention: opt-in prune
                     or manual admin hard-delete; no built-in backup)
```

Notable boundaries, all `[STATIC]`:

- **Device boundary:** there is none. Nightscout never talks to a sensor; uploader apps push over HTTP, and
  the project says so ("Nightscout itself does not directly interface with any devices" — `[PUBLIC CLAIM]`,
  verified: no device transports in the tree).
- **Store boundary:** MongoDB, with the purifier on the way in and **no physiological validation at all**
  (`api3/generic/create/validate.js:15–24`).
- **Alert boundary:** the process computes levels and emits on a bus; *delivery* to a human is a browser, a
  WebSocket, or a third-party plugin.
- **Retention boundary:** the operator's, by default. The application prunes only if configured, and the
  admin tools delete manually and permanently.

### 3.2 AnxietyWatch2, as specified

```
 AirSense 11 ──BLE──┐
 EMAY SleepO2 ─BLE──┤   (Linux box, Rust)
 Polar H10 ────BLE──┼──> vitals-collectors ──> vitals-decode (pure, no I/O)
 Acconeer XE125 ────┘                              │
                                                   v
                              COPY-binary spool (attempts precede work)
                                                   │
                                                   v
                       TimescaleDB: samples hypertable (append-only)
                       PK (source, type, session_id, seq); no UPDATE/DELETE
                       tombstone rows for removal
                         |                    |
                         |  7d compression    |  samples_1min continuous aggregate
                         |                    |  time_bucket_gapfill for gaps
                         v                    v
                    raw dropped at 2 years
                                                   │
                                                   v
                              iOS client (stage 7 in progress) + admin console
                              (roadmap 17/29)
```

Boundaries: the device boundary is the radio (no server-side uploader); the store boundary validates
against the registry (`vitals/registry.toml` — 17 types for `resmed_air11`, each with a unit, a valid range
and thresholds); rejection is a first-class outcome with a named reason, and absence is explicitly not a
rejection (`bp_build.rs:127`).

---

## 4. Data path and source comparison

| | Nightscout | AnxietyWatch2 |
|---|---|---|
| How data arrives | third-party uploader app → HTTP → API | device → BLE → collector on the box |
| Who writes | any client with a token (default: anonymous read, token to write) | the daemon |
| Validation on ingest | shape only: `identifier`, `date`, `utcOffset`, `app` | registry: unit, range, threshold per type; out-of-range rejected with a reason |
| Unit conversion | at the store boundary? no — mg/dL canonical, converted at many display and computation sites (`[STATIC]` `units.js`, 25+ call sites) | once, at the decoder (`[V2 current]`; `registry.toml` `conversion` for `resmed_air11` is `{ leak = 60.0, tidal_volume = 1000.0 }`) |
| Duplicate handling | `API3_DEDUP_FALLBACK_ENABLED = true`; `identifier` computed and compared, with a mismatch path that logs the whole document | implicit in the PK `(source, type, session_id, seq)`; the ezShare attempt ledger is `[V2 target]` (`cpap-ezshare-ingest`, 0/6) |
| Missing data | no row; displayed as `current` in the staleness pill; excluded from some computations, included in others | no row; the three-state rule applies; `time_bucket_gapfill` for series, and `locf()`/`interpolate()` are prohibited |
| Clock corrections | profile/timezone handling; `date`/`created_at`/`srvCreated` fallbacks | clock correction is versioned metadata and attempts precede work (`cpap-ezshare-ingest` invariants 6 and 7, `[V2 target]`) |
| Provenance | `srvCreated`, `srvModified`, `app`, `device`; `history` for versioned sync | source, type, session, seq; integrity and provenance are amendments 01 and 02 |

**The most instructive contrast is the validation row.** Nightscout's store boundary accepts any number for
a physiological field; AnxietyWatch2's rejects out-of-range values by name. Nightscout has deployed for
years with the permissive boundary and works, because the sensors and uploaders do the filtering upstream.
That is a legitimate architecture as long as nothing downstream treats the stored value as validated. The
A1c defect (§7) is what happens when something downstream does.

---

## 5. Claim-versus-code reconciliation

Twenty checkable Nightscout claims were reconciled against the pinned tree; fourteen match, five do not,
and one is forward-looking. Full table in `nightscout_IMPLEMENTATION_INSPECTION.md` §7. The five that do
not match:

| Claim | Source | What the code does |
|---|---|---|
| Autopruning "default is only `DEVICESTATUS`=60" | `[PUBLIC CLAIM]` API3 swagger `info.description` | No default at all (`collection.js:32` reads the env var with no default argument). The project's own audit agrees with the code, using different variable names. |
| Token derivation `name.replace(' ','-').toLowerCase() + '-' + hash.substring(0,16)`; JWTs signed with `API_SECRET`; 1-hour lifetime | `[VENDOR DOC]` `docs/audits/security-audit.md`, Jan 2026 | Different abbreviation rule; JWT key read from a generated `randomString` file, **not** `API_SECRET`; lifetime `'8h'`. The **requirements document beside the audit describes the code correctly.** |
| Delay list "Auto-clears entries after 60 seconds of inactivity" (listed as a strength) | `[VENDOR DOC]` same audit | One-shot `setTimeout(clearList, 30000)`; no `setInterval`. After 30 s the map is never pruned. |
| Role table including `readable` = `api:*:read, notifications:*:ack` | `[VENDOR DOC]` same audit | Code has `readable` = `*:*:read`; omits `status-only`; names `activity` where the audit says `activity-create`. |
| Audit environment: `mongodb ^3.6.0` | `[VENDOR DOC]` same audit | `package.json` pins `^5.9.2`. |

**And two checked claims that hold, which matter just as much:**

- **The sanitisation claim is accurate.** The security page says incoming entries, treatments, profiles,
  device status, food and activity are sanitised; all six server modules import the purifier, and API3
  additionally uses `writePurifier`. A hypothesis that the docs overstated this was checked and
  **refuted**.
- **"Alarms … can be cleared by any watcher of the data"** (README, `[PUBLIC CLAIM]`) matches the code: no
  socket rooms, and the token-path `ack` handler has no permission check.

**Why this section exists in a comparison.** Both AnxietyWatch2 and Nightscout rely on written
specifications — OpenSpec tasks with `Verify:` lines in one case, an audit/requirements/test-spec programme
with traceability matrices in the other. Nightscout's programme is more extensive and better published than
AnxietyWatch2's, and three of the audited subsystems' documents still do not describe the release they
ship with. The lesson is not "audits are worthless"; it is that **an audit is a claim, and the exit code —
or the code — is the evidence.** AnxietyWatch2's own rule ("acceptance is a shell command's exit code,
never your own assessment") is the right response to exactly this, and Nightscout is a large-scale
demonstration of why.

---

## 6. Feature matrix

`[N/A]` = the product has no such surface; `[—]` = not applicable to that product's purpose.

| Capability | Nightscout 15.0.8 | AnxietyWatch v1 | AnxietyWatch2 | Evidence |
|---|---|---|---|---|
| Self-hosted | yes (it is the product) | no (local app + optional sync) | yes (local box) | `[PUBLIC CLAIM]` / `[V1 impl]` / `[V2 current]` |
| No account required | yes | yes | yes | `[PUBLIC CLAIM]` / `[V1 impl]` / `[V2 current]` |
| Wireless device ingest | no (HTTP upload from a phone) | no BLE; CPAP CSV import | BLE: AirSense 11, H10, XE125, EMAY | `[STATIC]` (no device transports) / `[V1 impl]` / `[V2 current]`+`[V2 target]` |
| CPAP / sleep data | **`[N/A]`** | CSV import (OSCAR by-session) | AirSense 11 BLE stream, 17 registry types | `[STATIC]` / `[V1 impl]` / `[V2 current]` |
| SpO₂ | `[N/A]` | via HealthKit only | EMAY SleepO2 decoder | `[STATIC]` / `[V1 impl]` / `[V2 current]` |
| Respiratory rate | `[N/A]` | `[N/A]` | `[V2 target]` (xe125, 2/5 tasks) | per change status |
| Anxiety / panic capture | `[N/A]` | yes | yes (`[V2 current]`) | `[V1 impl]` / `[V2 current]` |
| Medication tracking | `[N/A]` (treatments are insulin) | `[N/A]` | subjective capture in progress | repository |
| Weather / aircraft / environmental context | `[N/A]` | weather context spec exists | `[V2 target]`/`[V2 current]` per change | repository |
| AI / ML analysis | fixed 2-coefficient AR forecast only | correlation engine, no trained model | `[V2 target]` | `[STATIC]` `ar2.js:11` / `[V1 impl]` / repository |
| Derived clinical statistic | A1c, GMI, TIR, fluctuation metrics — client-side, unit-dependent (one shipped defect) | correlation engine outputs | **deliberately none** (no AHI) | `[STATIC]` + `[PUBLIC CLAIM]` / `[V1 impl]` / `[V2 current]` |
| Append-only store | no (soft-delete + hard-delete coexist) | n/a (HealthKit) | yes | `[STATIC]` / `[V2 current]` |
| Tombstone removal | API3 only | n/a | yes | `[STATIC]` / `[V2 current]` |
| Retention policy defined by the product | no (operator's choice; opt-in prune) | n/a | yes: 7 d compression, 1-min aggregate, raw dropped at 2 y | `[STATIC]` + `[VENDOR DOC]` audit / `[V2 current]` |
| Built-in backup | no ("No built-in backup mechanism", `[VENDOR DOC]`) | server-side restore path | not established | audit §9 / `[V1 impl]` / `[UNKNOWN]` |
| Machine-readable API description | yes (OpenAPI for API3; contains one unimplemented default) | no | `[UNKNOWN]` at this granularity | `[STATIC]` |
| Delta-sync / versioned history | yes (API3 `history`) | sync service | `[UNKNOWN]` | `[STATIC]` |
| Access control model | roles + subjects + tokens + JWT; **default is anonymous read** | none (local app) | private LAN, no account | `[STATIC]` / `[V1 impl]` / `[V2 current]` |
| Programmable thresholds | partly: sensor/pump thresholds are settings, AR2 thresholds are not | some | registry, per type | `[STATIC]` `ar2.js` / `[V1 impl]` / `[V2 current]` |
| Alarm when data stops | **does not fire when nothing has arrived**; renders `current` | `[UNKNOWN]` | `[V2 target]` supervised alarm | `[STATIC]` `timeago.js:26,103–104` |
| Alarm when a value is out of range | fires, but not for `39` (bottom-of-scale) or a reading older than 10 min | risk alerts | `[V2 current]` per type | `[STATIC]` `simplealarms.js:21` |
| Alarm silencing | any valid token can silence any alarm; state in memory | `[UNKNOWN]` | `[V2 target]` | `[STATIC]` `alarmSocket.js:83–90` |
| Remote alert delivery | Pushover plugin; or IFTTT + Google + Pushover (four cloud services) | iOS local notifications | `[V2 target]` multi-modal, amendment 03 | `[PUBLIC CLAIM]` + `[STATIC]` |
| Price | $0 | free | free | `[PUBLIC CLAIM]` |
| Regulated medical device | explicitly not | explicitly not | explicitly not | `[PUBLIC CLAIM]` |
| Non-dependence warning to users | yes, explicit and repeated | disclaimer | disclaimer | `[PUBLIC CLAIM]` |
| Data leaves the operator's control | only if the operator chooses a host or a cloud alert path | only via HealthKit/optional sync | not by design | `[PUBLIC CLAIM]` / `[V1 impl]` / `[V2 current]` |

---

## 7. Units and derived values — the centrepiece

This is the axis where the package has its strongest evidence, so it is worked through in full.

### 7.1 What Nightscout does

- **Canonical store: mg/dL.** `consts.MMOL_TO_MGDL = 18.01559`.
- **Conversion helpers:** `lib/units.js` — `mgdlToMMOL` rounds to one decimal and **returns a string**
  (`(…).toFixed(1)`); `mmolToMgdl` returns an integer.
- **Conversion sites:** more than twenty-five explicit conversions in `lib/` alone.
- **The report path scales before computing.** `lib/report/reportclient.js:847` builds the report series
  with `sgv: client.utils.scaleMgdl(obj.y)`, i.e. **into the user's display unit**. The report's range
  thresholds are scaled the same way (`:40–41`), so Time-in-Range is unit-consistent.
- **The A1c formula does not.** `lib/report_plugins/dailystats.js:96–111` computes
  `average = sum / daysRecords.length` from those scaled values and then
  `(average * consts.MMOL_TO_MGDL + 46.7) / 28.7` — an expression written for an input in **mmol/L**.

### 7.2 What that produces, and what the project says it produces

The 15.0.8 report documentation states, as a note on the Daily Stats report:

> "In Nightscout 15.0.8, the Daily stats estimated A1c is wrong when glucose is displayed in mg/dL (for
> example 95.8% instead of 6.9%). This is fixed in the next release. The value shown in mmol/L is correct."

Arithmetic check, performed here and consistent with the project's published figures:

| Display unit | Average | Substitution | Result |
|---|---|---|---|
| mg/dL | 150 mg/dL | `(150 × 18.01559 + 46.7) / 28.7` | **95.79** → printed `95.8%` |
| mmol/L | 8.4 mmol/L | `(8.4 × 18.01559 + 46.7) / 28.7` | **6.87** → printed `6.9%` |

`[INFERENCE]` from `[STATIC]` code plus `[PUBLIC CLAIM]` documentation. Not run.

### 7.3 Reading it fairly

Three things must be said together, or the finding is distorted in one direction or the other:

1. **It is a defect in a derived clinical statistic, in a release, that its own users can see.** Not a code
   style preference, not a latent bug: a number on a screen, with a unit-dependent value.
2. **The project disclosed it, in the user documentation, with the magnitude, the affected unit, the
   correct unit, and the fix plan.** That is the correct handling of a known defect, and the opposite of a
   silent one.
3. **The root cause is not "Nightscout cannot do units".** Time-in-Range in the same report is
   unit-consistent, because the thresholds are scaled to match the series. One formula kept an assumption
   its input no longer satisfied. The mechanism is narrow; the consequence is that display state reached a
   clinical derivation.

### 7.4 Why it is the relevant comparison for AnxietyWatch2

AnxietyWatch2's rule is: *one canonical unit per type; converted once, at the decoder; never at a later
layer*, with the unit and the valid range taken from `vitals/registry.toml` and never hardcoded. The
`resmed_air11` source in the registry carries `conversion = { leak = 60.0, tidal_volume = 1000.0 }`, so
device-native units become canonical before anything downstream can see them.

Nightscout is what the rule is protecting against, in a form that can be checked: a store in one unit, a
presentation helper available to computation, and a derived statistic that silently depends on which unit
the user prefers. It is the same failure class as OSCAR's Day report storing a **p90** under the label
`Pressure_95th` (a label/derivation mismatch) and SleepHQ's typo'd `time_over_leak_threshhold`. Three
candidates, three instances, one mechanism: **the label and the computation drift apart when they are not
derived from one source.**

---

## 8. Absent and invalid data — the three-state question

### 8.1 Nightscout's posture, stated by its own code

| Location | Code | Comment |
|---|---|---|
| `plugins/timeago.js:101–104` | `var status = 'current'; if (!lastSGVEntry) { … }` | `//assume current` |
| `plugins/timeago.js:26` | `if (!lastSGVEntry \|\| lastSGVEntry.mills >= sbx.time) { return; }` | no entry → no notification |
| `api/entries/index.js:66–69` | `if (!data.type && opts.type) { data.type = opts.type; }` | `// bless absence with known type` |
| `levels.js:26–48` | `isAlarm` = WARN/URGENT; `toStatusClass` → `'current'` otherwise | four of six levels render identically |
| `plugins/simplealarms.js:21` | requires a current reading and `mgdl > 39` | no alarm without a current reading |
| `api3/generic/create/validate.js:15–24` | validates `identifier` structure only | no physiological range check anywhere |

**What this means in practice (`[INFERENCE]` from the code, not observed):** the product's default
rendering state is `current`, and it is reachable when there is no data, when the data is old, when a
value is at the bottom of the scale, and when a signal is merely low-but-not-alarming. The alarm that
exists to catch stale data (`timeago`, enabled by default at 15/30 minutes) returns early when there is no
reading at all. So "the sensor stopped sending" and "everything is fine" are separated by the user noticing
the age indicator, not by an alarm, and in the no-reading case not even by the indicator.

### 8.2 AnxietyWatch2's posture

`[V2 current]` `crates/vitals-decode` returns `Option`/`Result`; absence is a distinct outcome and, at the
build boundary, `assert!(b.rejected.is_empty(), "absent is not rejected")` in `bp_build.rs:127` makes
"absent" and "rejected" separate categories by construction. Out-of-range values are rejected with a named
reason and quarantined, never clamped. Missing signals hold state and never reassure or escalate on their
own.

`[V2 target]` the supervised alarm and the ezShare ingest path (`cpap-ezshare-ingest`, 0/6 tasks) are where
the runtime behaviour of this policy will be established.

### 8.3 The honest counter-argument

Nightscout's permissive posture is defensible from its own premises. It is a display and alerting layer fed
by hardware that has already decided what is measurable; refusing a value at the API boundary could drop a
genuine reading; and a `LOW` sentinel that is excluded from the *alarm* path is arguably correct, because
the pump and the uploader app are already alarming on it. Against that: **the same permissive posture is
what let a display unit reach a clinical derivation**, and "the user will notice the indicator" is not a
safety property. The two positions are not reconciled here, and should not be: Nightscout's is a
demonstrated operational success, AnxietyWatch2's is a specified rule with no operational record yet.

---

## 9. The store — append-only, deletion, retention

| | Nightscout 15.0.8 | AnxietyWatch2 |
|---|---|---|
| Update/delete of a physiological row | API3: soft-delete (`isValid:false`, new `srvModified`), retained in `history`; `?permanent=true` removes. v1/v2: hard delete. Admin tools: hard delete. | No `UPDATE`, no `DELETE`; removal is a tombstone row (`[V2 current]`) |
| Versioned sync | `history` operation, `srvModified` delta, `Last-Modified`/`ETag` | `[UNKNOWN]` at this granularity |
| Scheduled retention | opt-in only; when enabled, hourly, hard delete, three-way `$or` (`srvCreated` OR `created_at` OR `date`), `srvModified` not a criterion | fixed policy: 7-day compression, 1-minute aggregate, raw dropped at 2 years (`[V2 current]`) |
| Documented default retention | swagger claims `DEVICESTATUS`=60; code has none; vendor audit agrees with the code but names the variables differently | the policy above, in `AMENDMENTS.md` |
| Gaps in a series | none — no row. Reports compute over rows present. | `time_bucket_gapfill`; `locf()`/`interpolate()` explicitly prohibited (`AMENDMENTS.md:61`) |
| Backup | none built in ("No built-in backup mechanism", "No point-in-time recovery" — `[VENDOR DOC]`); hosting providers advertise backups | `[UNKNOWN]` |
| Manual purge | admin tools, hard delete, UI confirmation string | not a design goal |

**The `$or` prune is the most concrete store finding (`[STATIC]`).** With pruning enabled, a document is
deleted if **any one** of `srvCreated`, `created_at` or `date` precedes the cutoff. A row created a year ago
and corrected yesterday is removed on the next hourly pass, and so are tombstones — with no `history` entry,
because `history` only records `srvModified` changes and a prune is not one. The field the system itself
uses to mean "this changed recently" is not consulted.

**The three-way conflict on the pruning default (§5.11 of the inspection) is recorded, not resolved.** The
most conservative surface governs: nothing is pruned unless the operator configures it.

---

## 10. Alarms and alert delivery

### 10.1 The alarm path, side by side

| Property | Nightscout 15.0.8 | AnxietyWatch2 |
|---|---|---|
| Threshold source | settings for sensor/pump alarms; **file literals for the predictive engine** (`ar2.js:5–11`), where user settings only rename the event | `vitals/registry.toml`, per type (`[V2 current]`) |
| Out-of-range value presentation | clamped in the forecast display (`ar2.js:308`, `[36,400]`) | rejected and quarantined; never clamped (`[V2 current]`) |
| Missing data | no alarm; stale pill renders `current` | three-state; missing holds state (`[V2 current]` + `[V2 target]` supervised alarm) |
| Bottom-of-scale value (39) | excluded from glucose alarms; also special-cased in `bgnow`, `rawbg`, `treatmenttocurve`; included in reports | `[V2 target]` per-type handling |
| Who can silence | any valid token, globally; the README says so in prose | `[V2 target]` |
| Ack persistence | in-memory; cleared by a restart; silence is wall-clock based | `[UNKNOWN]` |
| Plugin/component failure | exception logged with the plugin name, then dropped; no admin notice, no persisted state | `[V2 target]` |
| Delivery to a phone | browser, or Pushover plugin, or IFTTT + Google Sheets + Pushover | `[V2 target]` multi-modal (amendment 03) |

### 10.2 What the alert-delivery comparison is actually saying

Nightscout's own documentation recommends a path with **four cloud services and three accounts**
(Nightscout → IFTTT Webhooks → Google Drive/Sheets → IFTTT applet → Pushover → phone), documents the
alternatives' limits (IFTTT SMS: 100 messages/month; IFTTT notifications: cannot target devices; the
backing spreadsheet: 2,000 rows before it rotates and must be re-pointed), and documents an **alarm-fatigue
failure mode in the integrated path**: Pushover's mandatory acknowledgement means that failing to
acknowledge "may end up getting alarm fatigue fairly quickly as the alarm repeated itself".

This is a mature, widely deployed alerting system, and reaching a human phone from it requires third-party
cloud services. That is the strongest available external argument for Amendment 03's local multi-modal
target being a **genuine product difference rather than a preference** — and it is also a caution: the
reason Nightscout took this route is that it has no first-party app, and AnxietyWatch2's local path has its
own unsolved problem (a property's phone must be reachable from a box on the LAN). The comparison supports
the target; it does not validate any particular implementation of it.

### 10.3 A note on the alarm failure surface

`lib/plugins/index.js:196–202` catches every plugin exception, logs `Plugin error on checkNotifications():`,
the plugin name and the error, and continues. The same pattern covers `visualizeAlarm`. Nothing is
persisted and nothing is raised to the operator, even though an admin-notification channel exists and is
used for security warnings. Whether this is a defect is arguable; what is not arguable is that **an alarm
component can stop working without anything in the product saying so.** AnxietyWatch2's supervised-alarm
target exists for this reason, and Nightscout's `docs/audits/data-layer-audit.md` and
`docs/test-specs/coverage-gaps.md` show that the maintainers inspect this area deliberately.

---

## 11. Operating mode, privacy and cost of ownership

### 11.1 What Nightscout asks of the operator

- A server, a MongoDB instance, an update process, and backups (which the application does not provide).
- A working internet connection and third-party cloud services, per its own documentation.
- Alert delivery through accounts at IFTTT and Pushover, or a self-assembled equivalent, if alarms must
  reach a phone.
- Understanding of its own security page, because the default publishes the data.

### 11.2 What it gives back

- **No company in the path.** Volunteer maintenance; a 501(c)(3) Foundation that states it takes no
  industry funding and names every major diabetes company it is unaffiliated with.
- **No account, no subscription, no feature gate** in the software.
- **A paid-host option for people who cannot run a server** — ten providers advertised with pricing links.
- **A public versioned API** and a documented shape per collection.
- **Explicit non-dependence disclaimers** shown to users.

### 11.3 Privacy, precisely

- **There is no privacy policy, and the reason is structural:** the project is not a data controller
  because it does not hold anyone's data. The controller is the operator — the user, or the host they pay.
- **The project says nothing about what a host may do with the data**, and simultaneously says that
  third-party operation is not a supported scenario and that its liability is "extremely complicated".
- **The default is public**, with a boot-time admin notice for users who open their own admin page.
- **A privacy assurance in the project's FAQ about a third party** ("Facebook will only share the
  information that you allow it to share") is unverifiable and is recorded as such.

**Compare with AnxietyWatch2:** no account, no server operated by anyone else, no cloud dependency in the
alerting design target, and a store on the owner's own hardware. On the privacy axis the two are not
comparable at all — but the comparison has a cost side that must be stated: **Nightscout's privacy position
is achieved by the operator accepting the operational burden, and its paid-hosting market exists because
most people cannot.** AnxietyWatch2 targets the same privacy posture while also targeting a single-box
installation, which is a different and harder problem. Whether it succeeds is `[V2 target]` and will be
decided by the ezShare, admin-console and supervised-alarm changes, not by this comparison.

---

## 12. Documentation as an artifact

Nightscout publishes more about itself than any other candidate in this package: system audits, formal
requirements, test specifications with traceability matrices, data schemas, design proposals, and an
OpenAPI description, plus a progress tracker describing an "audit-first methodology" begun in January 2026
and marking data-shape, API v1 compatibility and authorisation/security as *Complete*.

At the pinned release commit:

- **Two documents in the same subsystem disagree with each other** — the security audit and the
  authorisation/security requirements document describe different credential designs, and the code matches
  the requirements document.
- **The audit misdescribes three things** (token derivation, JWT key, JWT lifetime), one of which (the key
  coming from a regenerable file rather than `API_SECRET`) has an operational consequence: a redeploy that
  clears `node_modules` silently invalidates every issued JWT.
- **The audit describes the brute-force delay list as self-clearing; it is a one-shot `setTimeout`.**
- **The published API description documents a default retention the code does not implement.**
- **The changelog is behind the release it ships with.**

And, in the other direction:

- **The sanitisation claim is accurate** (verified, with a hypothesis refuted).
- **A `TODO` claiming a fallback "doesn't work" is stale** — the code works (verified, hypothesis
  refuted; §5.20 of the inspection). Had the comment been trusted, the package would have recorded a false
  defect **and missed the real one**, which is that an entry with no type is stamped with the caller's
  type rather than left unknown.

**Generalisation, offered cautiously.** AnxietyWatch2's own process rests on two written controls:
`AMENDMENTS.md` as mandatory reading that overrides stale task briefs, and `Verify:` commands as the
acceptance definition. Nightscout is the case study for why the second control is the load-bearing one: it
has an extensive written-control regime and three audited subsystems whose documents do not describe the
release. The transferable finding is that **a written audit must be re-derived from the artifact it claims
to describe, and an exit code is the only part of that which cannot drift.**

---

## 13. Failure modes, limitations and evidence gaps

### 13.1 Failure-mode walk-through (from `PROMPT_2.md`)

Answers are the inspected code's behaviour; **none was observed.**

| # | Scenario | Nightscout per the code | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|---|
| 1 | Uploader dies at 02:00 | Glucose alarm cannot fire (no current reading, `simplealarms.js:21`); `timeago` returns early when there is no entry (`:26`); the pill renders `current` (`:103–104`). If a reading exists but is old, `timeago` can fire (15/30 min defaults). | `[UNKNOWN]` | `[V2 target]` supervised alarm; three-state rule `[V2 current]` |
| 2 | Sensor reports bottom-of-scale (39) | Excluded from glucose alarms; neither error nor non-error in `bgnow`; included in report statistics; `ar2` uses 36 with a comment asking why | `[UNKNOWN]` | `[V2 target]` |
| 3 | User switches display unit | A1c estimate changes value; Time-in-Range does not | `[UNKNOWN]` | units fixed at the decoder |
| 4 | Operator enables pruning after a year | Documents deleted if any of `srvCreated`/`created_at`/`date` is old — including recently modified rows and tombstones; no history entry | n/a | retention is a fixed policy, not an operator switch |
| 5 | An alarm token leaks | Any valid token can silence any alarm, globally; alarms broadcast to every subscriber; ack state is in memory | `[UNKNOWN]` | `[V2 target]` |
| 6 | An alarm plugin throws every tick | Logged once per tick with the plugin name; no admin notice; no persisted state | `[UNKNOWN]` | `[V2 target]` |
| 7 | An uploader sends glucose `0`, then a valid value | `0` is stored: no range check exists. `bgnow.isError` treats it as an error (`!entry.mgdl`), reports include it in `d.y < 39` filtering terms, alarms exclude it via `> 39` | `[UNKNOWN]` | rejected with a named reason and quarantined |
| 8 | Redeploy with fresh `node_modules` | JWT signing key is regenerated from `node_modules/.cache/_ns_cache/randomString`; previously issued JWTs stop verifying (inference from key provenance; `setJWTKey` is never called) | n/a | n/a |

### 13.2 Limitations of this comparison

1. **No runtime observation of Nightscout at all.** Everything in §§4–10 about behaviour is a reading.
2. **The alarm delivery transports were not read** (`pushover`, `maker`, `speech`, `alexa`, `googlehome`),
   so §10.2 rests on the project's documentation.
3. **Time handling, `lib/storage` indexes and the test suite were not read**; no statement is made about
   DST correctness, TTL indexes or coverage.
4. **The other `docs/proposals/` documents were not read.** One is an RFC on an "agentic control plane" for
   insulin delivery, dated January 2026, status *Draft*. It is **not a feature and not an accepted plan**,
   and it is mentioned here only so that its existence is not discovered later and mistaken for one.
5. **AnxietyWatch2's own numbers are quoted from the specified design** (registry contents, retention
   policy, amendment rules) and from the change-status counts, not from a running system.
6. **The comparison is deliberately unbalanced in evidence quality:** Nightscout's side is code-verified,
   AnxietyWatch2's side is specification-verified. The asymmetry is a statement about maturity, not about
   correctness, and it should not be read as either system being better than the evidence about it.

### 13.3 Evidence gaps worth closing, in order

1. **An authorised runtime check of the default exposure and the A1c defect** (`PROMPT_2.md` Q1–Q2). Both
   are cheap and both convert a code reading into a demonstrated behaviour.
2. **A check of the project's issue tracker for the drift findings** (Q4) — whether the maintainers know.
3. **Reading the purifier, the storage indexes and one alarm transport** (Q7, Q10) — each is short.
4. **Reading the ten hosting providers' terms** (Q6) — the entire privacy surface for most non-technical
   users, currently `[UNKNOWN]`.
5. **A survey of the ecosystem** (Q11) — the only way this candidate could become a closer competitor than
   the shortlist assumes.

---

## 14. Balanced conclusion

**Nightscout is the right comparison for AnxietyWatch2 and the wrong comparison for AnxietyWatch v1.**

For v1, the overlap is limited to a shared "what happens when the signal stops" question, and Nightscout's
answer (assume current, do not alarm) is worth one paragraph in v1's risk discussion and no more.

For AnxietyWatch2, Nightscout is the only candidate that has already built the layer AnxietyWatch2 is
specifying, at scale, for seventeen years, under real failure conditions. Four conclusions follow, and they
point in different directions.

**1. AnxietyWatch2's six core rules are externally validated by counter-instance.** Unit handling, absent
data, invalid data, threshold sourcing, append-only storage and gap semantics each have a concrete,
verifiable counter-example in the pinned Nightscout source, and two of them (units and absent data) have
documented user-visible consequences. That is the strongest form of support this package can produce
without running either system.

**2. But the rules are not necessary for a working system, and the comparison should not pretend
otherwise.** Nightscout violates all six and is deployed in thousands of homes. The defensible claim for
the rules is narrower and stronger than "this is how you build it": **they prevent a wrong number or a
false reassurance from reaching a user.** AnxietyWatch2's own reasoning should be held to that claim.

**3. Nightscout's operating model is not a failure of ambition; it is a cost.** Self-hosting, no account
and no company in the path are the right position, and Nightscout demonstrates what that costs: a server, a
database, backups the application does not provide, an internet connection and third-party cloud services
for alerting, and a paid hosting market for the majority who cannot do it themselves. AnxietyWatch2 has
chosen the same position with a smaller box and a local alerting target. That is a genuine improvement in
the design — and its difficulty has not yet been tested.

**4. Three things Nightscout does that AnxietyWatch2 should consider on their merits, not as features to
match:** a versioned delta-sync surface (`history` + `srvModified`) for clients; a public, machine-readable
API description; and an explicit non-dependence warning shown to users. The first two are engineering; the
third costs nothing and is the cheapest safety improvement in this entire package.

**Finally, the most transferable finding is not about Nightscout at all.** A project with an extensive
self-audit programme, published traceability matrices and a formal requirements set shipped a release whose
own security documentation misdescribes its credential design, whose API description documents a retention
default that does not exist, and whose report layer printed a unit-dependent clinical number it then had to
document as wrong. Every one of those was found by reading the artifact, not the document about it. That is
the same rule AnxietyWatch2 already states for its own tasks — *acceptance is a shell command's exit code,
never your own assessment* — and it is the single most useful thing this candidate has to teach.

---

## Related files

- `nightscout_CLAIMED_FEATURES_AND_PURPOSE.md` — public claims, governance, pricing, evidence table
  `N-1`…`N-60`.
- `nightscout_IMPLEMENTATION_INSPECTION.md` — pinned-commit findings §5.1–§5.20, the refuted hypotheses, and
  the reproduction commands.
- `PROMPT_2.md` — the second-stage prompt, including the fourteen unresolved questions.
- `../CANDIDATE_SHORTLIST.md` — why this candidate is #6.
- `../PROCESS_STATE.md` — queue position, residual uncertainty, and the running owner decision.
- `../RESEARCH_METHOD.md` §6.1 — the inspection-mode taxonomy and its limits.
