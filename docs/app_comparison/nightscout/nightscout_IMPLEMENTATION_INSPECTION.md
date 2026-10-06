# Nightscout — implementation inspection

**Slug:** `nightscout`
**Inspection date:** 2026-10-06
**Phase:** Phase 2A (authorized implementation inspection)
**Outcome:** **PERFORMED** — the complete source of the released version was obtained from a public
AGPL-3.0 repository and read at a pinned commit. This is the **second** candidate in this research where
the actual product implementation could be inspected (`oscar` was the first). **Nothing was executed.**

---

## 1. Summary of outcome

| Target | Inspected? | Basis |
|---|---|---|
| Source of the released version 15.0.8 (`nightscout/cgm-remote-monitor`, branch `master`) | **Yes** | Public AGPL-3.0, no authentication, no DRM, cloned at a pinned commit |
| The project's published audits, requirements, test specs and design documents | **Yes** | In the cloned tree (`docs/audits/`, `docs/requirements/`, `docs/test-specs/`, `docs/data-schemas/`, `docs/proposals/`, `docs/meta/`) |
| The published OpenAPI description of API3 (`lib/api3/swagger.yaml` / `.json`) | **Yes** | In the cloned tree |
| The project documentation site (`nightscout.github.io`) | **Yes** | Fetched and hashed; used for the public-claims file |
| The Nightscout Foundation site | **Yes** | Fetched and hashed |
| The compiled/deployed application | **No** | Nothing was built, installed, started or deployed |
| Any Nightscout instance, public or private | **No** | No URL was contacted; no request was made to any deployed site |
| Any Mobile API client, uploader app, or third-party follower app | **No** | Out of scope |
| Runtime behaviour of any kind | **No** | See §6 |

**Why this candidate could be inspected at all.** Nightscout is open source under AGPL-3.0 and its full
implementation ships in the repository. There is no APK to obtain, no DRM, no account, and no licence that
restricts reading. That is the same basis on which OSCAR was inspected, and it is the only basis on which
any closed-source candidate can be inspected in this package.

**What is different about inspecting this candidate versus OSCAR.** OSCAR is a desktop analysis
application. Nightscout is a **network service with access control, an API, alarms and retention** — the
parts of AnxietyWatch2 that OSCAR does not overlap. Its findings therefore land on the platform-layer
questions: what the default access policy is, what happens to absent data, what an alarm does when the
data stops, what a delete means, and what a derived statistic is computed from.

**Five findings matter more than the rest.**

1. **The shipped default grants anonymous read access to everything, and the project documents that fact.**
   `lib/settings.js:39` sets `authDefaultRoles: 'readable'`; `lib/authorization/storage.js:144` defines
   `readable` as `['*:*:read']`. The security documentation's own warning is "In a default setup, anyone
   with your Nightscout URL can view your site." A boot-time admin notice mitigates it for users who open
   their own admin page (§5.1).
2. **A derived clinical statistic is unit-dependent, wrong in the inspected release, and both the vendor
   and the code agree on why.** The 15.0.8 documentation states the Daily Stats estimated A1c "is wrong
   when glucose is displayed in mg/dL (for example 95.8% instead of 6.9%)". The report series is scaled
   into the display unit at `lib/report/reportclient.js:847`, and the A1c formula multiplies that already
   scaled average by `MMOL_TO_MGDL` at `lib/report_plugins/dailystats.js:111`. The published figures
   reconcile arithmetically with the source (§5.3).
3. **The alarm subsystem treats absent data as unremarkable.** `lib/plugins/simplealarms.js:21` cannot
   raise a glucose alarm without a current reading; `lib/plugins/timeago.js:103–104` renders an absent
   reading as neutral `current` (comment: `//assume current`); `lib/api/entries/index.js:66` stamps a
   missing `type` with the route's type (comment: `// bless absence with known type`); and four of the six
   severity levels render identically to `NONE` (§§5.4, 5.5).
4. **Any valid token can silence any alarm, and alarms are broadcast to every subscriber.** The alarm
   socket has no rooms and its token-path `ack` handler performs no permission check
   (`lib/api3/alarmSocket.js:71–90`); the README states the same thing in prose: alarms "can be cleared by
   any watcher of the data" (§5.7).
5. **The project audits itself, publishes the results, and three of the audited subsystems' documents do
   not describe the code.** The token/JWT audit text is wrong while the requirements document beside it is
   right (§5.2), the role table is incomplete (§5.16), and the brute-force delay list is described as
   self-clearing when it is a one-shot `setTimeout` (§5.17).

**One methodological result is worth recording alongside them.** Five hypotheses raised during this
inspection were **checked and refuted**, and are therefore *not* recorded as findings (listed in §5.20).
One of them — a stale `TODO` asserting that a fallback "doesn't work" when the code at the pinned commit
plainly does work — would have become a false finding had the comment been trusted. This is the same
discipline the package applies to candidates ("never mark a candidate complete on a model's assertion"),
applied per-finding.

---

## 2. Authorization basis

| Question | Determination |
|---|---|
| Is the artifact lawfully obtainable? | **Yes.** Public repository under AGPL-3.0. `git clone` over HTTPS, no credentials, no acceptance of terms beyond the licence, no circumvention of any technical protection. |
| Is reading it permitted? | **Yes.** The licence grants the right to run, study, modify and redistribute. Reading the source is the intended use. |
| Was any protective measure bypassed? | **No.** No DRM, no encryption, no certificate pinning, no anti-debugging, no obfuscation, no authentication was encountered or evaded. |
| Was any account created or credential used? | **No.** Nothing in this inspection used a Nightscout account, token, API key or deployment. |
| Was anything executed? | **No.** The repository was cloned and read. No build, no install, no server, no test run, no `npm`, no MongoDB. |
| Was any deployed instance contacted? | **No.** No Nightscout URL, public or private, was requested. |
| Is private data involved? | **No.** Only the project's own published repository and public documentation site. |
| Was the repository modified? | **No.** Read-only. No commit, branch, issue or pull request was created. |
| Does publishing these findings raise a licence issue? | **No.** Findings are described in my own words with short quotes and `file:line` citations for the reader's own verification. No source is reproduced in bulk, and no code is copied into AnxietyWatch2. |

**Decision:** inspection **performed**, as a read-only static review of a public source tree.

---

## 3. Artifact record — inspected

| Field | Value |
|---|---|
| Repository | `https://github.com/nightscout/cgm-remote-monitor.git` |
| Branch | `master` |
| Pinned commit | `92d0834219aa771b5837dbcbf1baeb839a200cf6` |
| Commit date | 2026-09-04 19:12:30 +0100 |
| Commit subject | "Merge pull request #8482 from nightscout/dev" |
| Version (`package.json`) | `15.0.8` |
| Matching release | GitHub release **v15.0.8**, "Release 15.0.8 Chocolate Chip Muffin", published 2026-09-04T18:27:47Z |
| Licence | **AGPL-3.0** |
| Engine requirement | Node `>=20.x` |
| Runtime dependencies | 65 in `package.json` |
| Tree size / file count | ~15 MB / 764 files (working tree, excluding `.git`) |
| Local clone path | `/tmp/aw2research/nightscout/repo` (outside the AnxietyWatch2 repository) |
| Tooling used | `git`, `ls`, `wc`, `grep -n`, `sed -n`, `shasum -a 256`, `curl` (documentation site only), `python3` (local text extraction) |

**The pinned commit is the release commit for 15.0.8.** That matters for §5: every documentation-drift
finding below is a disagreement between **the 15.0.8 documentation and the 15.0.8 code**, not a
disagreement with an older or newer release.

### 3.1 Web artifacts fetched (for the public-claims file)

All under `/tmp/aw2research/nightscout/web/`, SHA-256 recorded at fetch time:

| Artifact | SHA-256 |
|---|---|
| `admin_tools.html` | `eee26fc0aee06f187700e0bc8a363b17c20e99bc64447607fcaa3bc4f13853f2` |
| `landing.html` | `e4829b2c72328c4b429e6528fd070f32f152e314598dccd5747120d56a9560ac` |
| `pushover.html` | `443a48ca7bce1908ac391e8bd5e04bb10d3287d5e3a54247610e874e4a7d2d86` |
| `reports.html` | `f5e363e1ed79638b807a407f0d3218dfe857a545fefd4396a2de58ededab0e52` |
| `security.html` | `50b8596dd519ea2614d2785570c6733b61f9a9418d774b0a4c43e8e3bd4618f9` |
| `admin_tools.txt` | `f41a68337eabcd8b15ac0a22157677b3a6ae1fc5dd70aaa4476e8e740c0a1f8c` |
| `landing.txt` | `5d8cf9690bc85120b7efe5a1ba8e381f8cbfc17925f7fdae82e973267022308e` |
| `pushover.txt` | `5546f39b2c076a5d35fe95b3876440853fdbf0a1dd346a4746633952a540d058` |
| `reports.txt` | `a4defff84097c72dd5f921c5e88d8657d4deb38ecb34fab7ac7b7a08e105913e` |
| `security.txt` | `b91e27ca446e8f180f54fa803cefc15ed8b8627da4492b308abbc83221adef14` |

### 3.2 What was read, and what was not

**Read in full or in the relevant part:** `lib/settings.js`; `lib/levels.js`; `lib/constants.json`;
`lib/units.js`; `lib/notifications.js`; `lib/sandbox.js`; `lib/authorization/{index,storage,delaylist}.js`;
`lib/server/{env,enclave,bootevent,purifier,storage-purifier,entries,treatments,profiles,devicestatus,food,activity}.js`;
`lib/api/entries/index.js`; `lib/api/shared/write-payload.js`; the whole `lib/api3/` generic-operation
pipeline (`collection`, `index`, `const.json`, `swagger.yaml`, `alarmSocket.js`,
`generic/{create,delete,history,patch,read,update}/*`, `shared/{operationTools,stringTools,writePurifier,dateTools}.js`,
`storage/mongoCollection/{modify,utils}.js`, `mongoCachedCollection/index.js`); `lib/plugins/{index,ar2,simplealarms,timeago,bgnow,webhook}.js`;
`lib/admin_plugins/` (listing plus `cleanentriesdb.js`); `lib/report/reportclient.js`;
`lib/report_plugins/dailystats.js`; `lib/data/dataloader.js`; `docs/audits/{security-audit,data-layer-audit}.md`;
`docs/requirements/*`; `docs/data-schemas/` (listing); `docs/proposals/` (listing plus the agent-control-plane
RFC header); `docs/meta/DOCUMENTATION-PROGRESS.md`; `README.md`; `CHANGELOG.md`; `CONTRIBUTING.md`;
`package.json`; `Makefile`.

**Not read:** the remaining alarm transports (`pushover`, `maker`, `speech`, `alexa`, `googlehome` plugins);
most of `lib/reports/`; `lib/times.js` and the clock code; `lib/storage/*`; the `docs/proposals/` bodies
(other than the RFC header); `docs/test-specs/*` bodies; the server renderer and the browser client; the
`tests/` bodies (counted, not read); the 1.x/2.x API implementations beyond the delete path; and most of
`lib/api3/generic/{search,setup,status,version}`.

**A note on `file:line` provenance.** Every line reference in §5 was read from the pinned tree at the time
of writing this document and re-checked before the commit that added it. No line reference is carried
over from a secondary source. Where a vendor document states its own line numbers, they are identified as
the vendor's, not adopted.

---

## 4. Static findings — architecture

### 4.1 Shape of the system

A single Node/Express process serving a browser client, with MongoDB as the only store.

```
  uploaders (phone apps)            browsers / followers            third parties
        |                                   |                            |
        |  API v1 / v2 / v3 (HTTP)          |  HTTP + WebSocket          |  IFTTT webhooks
        v                                   v                            v
  +--------------------------------------------------------------------------+
  |  express app  (lib/server/app.js, lib/server/server.js)                   |
  |    static client + views + bundle                                        |
  |                                                                          |
  |  lib/api  (v1)   lib/api2  (v2)      lib/api3 (v3, "3.0.5")              |
  |      |                |                   |                             |
  |      |                |    generic op pipeline:                           |
  |      |                |    create -> validate -> insert -> autoPrune      |
  |      |                |    read / search / update / replace / patch       |
  |      |                |    delete -> markAsDeleted (isValid=false)        |
  |      |                |    history -> srvModified delta sync              |
  |      |                |                   |                             |
  |  authorization: roles, subjects, tokens, JWT (lib/authorization)          |
  |  purifier: storage-purifier (v1/v2) + writePurifier (v3)                  |
  |                                                                          |
  |  bus (lib/bus.js):  tick -> data-loaded -> notifications.checkNotifications|
  |                     admin-notify -> adminnotifies                        |
  |  plugins (lib/plugins): ar2, simplealarms, timeago, bgnow, webhook, ...    |
  |  sockets: storageSocket, alarmSocket (namespace-wide emit)                |
  +--------------------------------------------------------------------------+
                                   |
                                   v
                             MongoDB collections
                    entries / treatments / profiles /
                    devicestatus / food / activity / settings
```

### 4.2 The request/lifecycle path

- `lib/server/bootevent.js` orchestrates startup on the `bootstrap` event: environment load, purge of a
  stale `env.settings` cache under a version change, admin-notification copy (`:22–30`), the
  readable-by-world notice (`:149–152`), and the background loop that turns bus ticks into data loads and
  notification checks (`:310–335`).
- Data load populates `ctx.ddata` and stamps `ddata.lastUpdated = Date.now()` on every load
  (`lib/data/dataloader.js:60–80`).
- Alarms are evaluated on the `data-loaded` bus event; `lib/bus.js:35` drives `tick`, with a debounce
  (`UPDATE_DEBOUNCE_WAIT` / `UPDATE_MAX_WAIT`, leading and trailing).
- `lib/plugins/index.js` iterates enabled plugins and calls `checkNotifications` on each inside a
  `try/catch`.

### 4.3 The API3 generic-operation pipeline

API3 is not a set of route handlers; it is a **pipeline** assembled per collection
(`lib/api3/generic/collection.js`). The core operations are `read`, `search`, `create`, `update`,
`replace`, `patch`, `delete`, `history`, `status`, `version`. Storage is abstracted behind
`storage/mongoCollection/*` with a cached wrapper (`mongoCachedCollection/index.js`) and a projection
helper (`shared/fieldsProjector.js`).

Two properties of this design are relevant here:

- **Every write path calls `autoPrune()` after the write** (`create/insert.js:48`, `update/replace.js:60`,
  `patch/operation.js:96`, `delete/operation.js:68,96`). Pruning is therefore driven by write traffic, at
  most hourly (§5.10).
- **The pipeline distinguishes "permanently" from "soft" delete** (`generic/delete/operation.js:20–24`),
  and the soft path sets `isValid: false` plus a new `srvModified` (`:80`) rather than removing the row.
  Reads filter invalidated documents out (`mongoCachedCollection/index.js:57`), and `history` does not
  (`generic/history/operation.js`, documented at `swagger.yaml:412`).

### 4.4 Authorisation model

- `lib/authorization/storage.js` defines roles with coarse permission strings; the built-in ones are
  `admin` (`*`), `denied`, `status-only` (`api:status:read`), `readable` (`*:*:read`), `careportal`
  (`api:treatments:create`), `devicestatus-upload` (`api:devicestatus:create`) and `activity`
  (`api:activity:create`).
- Subjects are documents with a name, roles and a computed `accessToken`; the token is
  `abbrev + '-' + digest.substring(0, 16)` where `abbrev = name.toLowerCase().replace(/[\W]/g, '').substring(0, 10)`
  and `digest` is a SHA-1-based subject hash (`storage.js:187–193`).
- JWTs are signed with a symmetric key read at startup from a generated random-string file
  (`lib/server/enclave.js:30`), with a default lifetime of `'8h'` (`:59`). `enclave.setJWTKey` exists
  (`:54`) but is **never called** anywhere in the tree.
- `lib/authorization/index.js` consults the collection and enforces permissions; `delaylist.js` implements
  a failed-authentication delay; the `admin-notify` bus event is emitted on authentication failure
  (`index.js:212`).

### 4.5 Alarm architecture

Three layers, and the seams between them are where the findings are:

- **Plugin layer** (`lib/plugins/*`): each plugin computes a level and an event. The level vocabulary is
  six values (§4.6). Plugins are enabled by name in `settings.enable`; the server-default list is
  assembled in `lib/plugins/index.js` and enabled per settings.
- **Notification layer** (`lib/notifications.js`): module-level in-memory state; an `alarms` array of
  `{ level, group, label, silenceTime, lastAckTime, lastEmitTime }`; `silenceTime` defaults to
  `THIRTY_MINUTES`; `lastAckTime` starts at `0`. Silencing is a wall-clock comparison against
  `ctx.ddata.lastUpdated` (`:60`). Auto-ack uses a 1 ms silence (`:46`) so the next cycle clears it.
- **Delivery layer**: browser (`visualizeAlarm`), the storage/alarm WebSocket (`lib/api3/alarmSocket.js`),
  the IFTTT/Maker webhook plugin, and the Pushover plugin.

The severity vocabulary lives in `lib/constants.json` and `lib/levels.js`; only `WARN` and `URGENT` count
as alarms, and `levels.toStatusClass` returns `'current'` for every other value.

### 4.6 Data model and units

Canonical glucose storage is **mg/dL**. `consts.MMOL_TO_MGDL = 18.01559`. Conversion helpers live in
`lib/units.js`; `mgdlToMMOL` rounds to one decimal place and **returns a string**
(`(…).toFixed(1)`), while `mmolToMgdl` returns an integer via `Math.round`. At least twenty-five call
sites convert explicitly across `lib/` (settings, ddata, sandbox, client renderer, clock client, report
plugins, careportal components, basal profile). The consequence of a rounded, string-typed, presentation-
oriented conversion helper being available to computation is §5.13 and §5.3.

### 4.7 Storage, retention and the administrative surface

- Collections: `entries`, `treatments`, `profiles`, `devicestatus`, `food`, `activity`, `settings`
  (`lib/constants.json`, `lib/server/*.js`).
- Retention in the application is either **opt-in auto-pruning** (§5.10) or **manual admin tools**
  (`lib/admin_plugins/`), and the manual tools hard-delete. `cleanentriesdb` pre-fills 180 days for
  entries; `daterangedelete` deletes by date range; `futureitems` removes future-dated documents.
- The vendor's own `docs/audits/data-layer-audit.md` §9 records "**No built-in backup mechanism**" and
  "**No point-in-time recovery**", and recommends adding a data-export API, daily `mongodump` and oplog
  point-in-time recovery. None of the three was found implemented.
- Sanitisation on write is real: `lib/server/{entries,treatments,profiles,devicestatus,food,activity}.js`
  each import `./storage-purifier`, and API3 additionally routes through
  `lib/api3/shared/writePurifier.js`.

---

## 5. Static findings — the reconciliation points

Each finding is labelled with its type. **Static-only finding** means read from the tree; **inference**
means my reasoning; **vendor-published internal document** means the project's own audit/requirements text.

### 5.1 The default access policy publishes the data, and the project says so

**Static-only finding.** `lib/settings.js:39` sets `authDefaultRoles: 'readable'`. `readable` is defined
as `['*:*:read']` (`lib/authorization/storage.js:144`) — read on every resource, for callers who present
no credentials. The project's security documentation states the consequence in its own words: "**In a
default setup, anyone with your Nightscout URL can view your site.**", and recommends
`AUTH_DEFAULT_ROLES=denied`.

**Vendor-published internal document, counterpoint.** `docs/audits/security-audit.md` recommended
defaulting to `denied`. The recommendation was not applied at the inspected commit.

**Mitigation confirmed.** `lib/server/bootevent.js:149–152` pushes a `persistent: true` "Nightscout
readable by world" notice into the admin notification channel when the default is in effect, and
`adminNotifiesEnabled: true` by default (`lib/settings.js:71`). A user who opens their own admin page sees
it. This is a real mitigation and is recorded as one; it is not equivalent to a safe default, since the
population that never opens the admin page is exactly the population most exposed.

**Related permissive default.** `ALLOW_UNRESTRICTED_FRAME_EMBEDDING` defaults to `true`
(`lib/server/env.js:85` `readENVTruthy("ALLOW_UNRESTRICTED_FRAME_EMBEDDING", true)`), matching the
documentation's "temporary default is true". Here the project has stated an intention to tighten it.

### 5.2 Credentials: an audit, a requirements document and the code give three different answers

This is the sharpest documentation-drift finding in the package, because it is a **three-way disagreement
inside one project** on the subject of authentication.

| Source | Date | Token derivation | JWT signing key | JWT default lifetime |
|---|---|---|---|---|
| `docs/audits/security-audit.md` | 2026-01 | `name.replace(' ','-').toLowerCase() + '-' + hash.substring(0,16)` | `API_SECRET` | **1 hour** |
| `docs/requirements/authorization-security-requirements.md` (REQ-AUTH-021/022/023) | 2026-01 | — | — | — |
| Code at pinned commit | 2026-09 | `name.toLowerCase().replace(/[\W]/g,'').substring(0,10) + '-' + getSubjectHash(_id).substring(0,16)`, digest = SHA-1 of the token (`lib/authorization/storage.js:187–193`) | `secrets[jwtKey] = readKey('randomString')` from `node_modules/.cache/_ns_cache/randomString` (`lib/server/enclave.js:21–30`, `package.json` `post-generate-keys`) | **`'8h'`** (`lib/server/enclave.js:59`) |

**The requirements document describes the code correctly; the audit does not.** The audit is not merely
stale in dates — its description of the mechanism is wrong in three separate respects (the token
abbreviation uses a different transformation and a different character class; the JWT is not signed with
`API_SECRET`; the default lifetime is eight times what the audit states).

**Two consequences that matter operationally:**

1. **A redeploy that wipes `node_modules` silently rotates the JWT signing key**, because the key is read
   from a generated file inside `node_modules/.cache`. Every previously issued JWT stops verifying.
   Recorded as an inference from the key's provenance and `setJWTKey` being unused; nothing was run to
   confirm the rotation.
2. **The effective default session lifetime is 8 hours, not 1**, which is the more conservative of the two
   statements for a *user* but the less conservative for an *auditor* relying on the document.

**Not resolved in the vendor's favour or against it.** The audit is dated January 2026 and the pinned
commit is September 2026, so "divergence at the commit" must not be read as "wrong when written". What can
be stated is that a reader who follows the vendor's audit today would be misinformed about the credential
design of the release they are running.

### 5.3 A shipped derived statistic whose value depends on the user's display unit

**Public claim about the vendor's own release, mechanism verified in code.** The 15.0.8 report
documentation states:

> "In Nightscout 15.0.8, the Daily stats estimated A1c is wrong when glucose is displayed in mg/dL (for
> example 95.8% instead of 6.9%). This is fixed in the next release. The value shown in mmol/L is correct."

**Mechanism, from the tree:**

- `lib/report/reportclient.js:847` — the report series is built with
  `sgv: client.utils.scaleMgdl(obj.y)`, so the values handed to report plugins are in the **user's display
  unit**. The report's own range thresholds are scaled the same way (`:40–41`), so Time-in-Range
  classification is unit-consistent.
- `lib/report_plugins/dailystats.js:78,96–111` — `daysRecords = datastorage[day].statsrecords`;
  `record.sgv = parseFloat(record.sgv)`; `sum += record.sgv`; `average = sum / daysRecords.length`; then
  `(average * consts.MMOL_TO_MGDL + 46.7) / 28.7`.
- The A1c coefficient is written for an input in **mmol/L**. In mg/dL display the average is multiplied by
  18.01559 a second time.

**Arithmetic check (derived here, not run against the application, and consistent with the vendor's
published example):**

| Display unit | Average | Substituted value | Published result |
|---|---|---|---|
| mg/dL | 150 mg/dL | `(150 × 18.01559 + 46.7) / 28.7` = **95.8%** | 95.8% (documentation's erroneous figure) |
| mmol/L | 8.4 mmol/L | `(8.4 × 18.01559 + 46.7) / 28.7` = **6.9%** | 6.9% (documentation's correct figure) |

**Two findings, not one.** The defect is the first: **a derived clinical statistic whose numeric output
depends on a display preference**, shipped in the inspected release. The second is the disclosure: the
project states the defect, its magnitude, the units in which the value is correct, and that a fix is
coming, in the user documentation for the release.

**Why this is the centrepiece for the comparison.** AnxietyWatch2's rule — one canonical unit per type,
converted once, at the decoder, never at a later layer — exists to make this class of defect impossible.
Nightscout stores mg/dL canonically and has a conversion helper whose name and return type
(`mgdlToMMOL` → `toFixed(1)` string) mark it as a presentation function, yet display-scaled values reach a
clinical derivation. This is the third candidate in the package where a derived value's label or
correctness did not match its computation (OSCAR's p90 stored as `Pressure_95th`; SleepHQ's typo'd
`time_over_leak_threshhold`).

### 5.4 Absent data is deliberately treated as present: `//assume current` and `// bless absence with known type`

**Static-only finding, from the project's own comments.** Two independent locations state a posture
toward missing data, and neither is a bug report:

**`lib/plugins/timeago.js` — the staleness pill.**

```
:20   if (!sbx.extendedSettings.enableAlerts) { ... return; }
:24   var lastSGVEntry = sbx.lastSGVEntry();
:26   if (!lastSGVEntry || lastSGVEntry.mills >= sbx.time) {   // no entry -> no notification
:88     return 'current';
:101  var status = 'current';
:103  if (!lastSGVEntry) {
:104    //assume current
```

The default settings enable this alarm (`alarmTimeagoWarn: true`, 15 min; `alarmTimeagoUrgent: true`,
30 min — `lib/settings.js:27–30`). The gap is not that the alarm is off; it is that **it returns early
when there is no reading at all**, and the pill renders the same neutral `current` in that case as for a
fresh reading. A user looking at the pill cannot distinguish "current" from "no data", and the alarm
intended to catch that condition does not fire for it.

**`lib/api/entries/index.js` — the type fallback.**

```
:61  function force_typed_data (opts) {
:66    // bless absence with known type
:67    // TODO: this doesn't work, it assigns the query rather tham the type
:68    if (!data.type && opts.type) {
:69      data.type = opts.type;
```

The route constructs `type_params.type` as a **string** (`:130–133`: `req.query.find.type` or
`req.params.model`), and `:69` assigns it, so absent types are stamped with the route's type. The `TODO`
asserting that the assignment is broken **appears stale at the pinned commit** — see §5.20, where this is
recorded as a hypothesis checked and refuted. The behaviour is the finding: an entry with no declared type
is not rejected and not marked unknown; it is given the type the caller's request implied.

**Recorded as a pattern, not as two incidents.** The project's codebase contains two independent, candid
statements of the same design choice: *absence is not a distinct state; assume the ordinary case*. This is
the clearest available contrast with AnxietyWatch2's three-state rule ("measured-and-good,
measured-and-bad, and not measured; the third is never rendered, logged, or scored as the first").

### 5.5 The severity model has six levels and a display with two

**Static-only finding.** `lib/constants.json:19–24`:

```
LEVEL_URGENT: 2, LEVEL_WARN: 1, LEVEL_INFO: 0,
LEVEL_LOW: -1, LEVEL_LOWEST: -2, LEVEL_NONE: -3
```

`lib/levels.js:26–28` — `isAlarm` is true only for `WARN` and `URGENT`.
`lib/levels.js:39–48` — `toStatusClass` returns `'current'` unless the level is `WARN` or `URGENT`.

So `INFO`, `LOW`, `LOWEST` and `NONE` are four distinct severities that render **identically**. `NONE`
means "this signal is not in an alarm state"; `LOW` and `LOWEST` mean "measured and mildly out of range".
The presentation has no way to distinguish them, and there is no distinct rendering for "not measured" at
all.

`LEVEL_NONE` is assigned by `lib/plugins/{insulinage,sensorage,pump,batteryage}.js` (`insulinage.js:86`,
`sensorage.js:139`, `pump.js:218,234,263`), and `pump.js:270` tests `result.level > levels.NONE` — a
severity comparison used as a presence check.

### 5.6 The glucose alarm cannot fire without a current reading, and the LOW sentinel is uncategorised

**Static-only finding.** `lib/plugins/simplealarms.js:21`:

```
if (scaledSGV && lastSGVEntry && lastSGVEntry.mgdl > 39 && sbx.time - lastSGVEntry.mills < times.mins(10).msecs)
```

Three gates, all of which must hold: a scaled value exists, an entry exists, the value is **strictly
greater than 39**, and the entry is less than ten minutes old. An absent reading, a reading older than ten
minutes, or a reading at exactly 39 produces **no alarm of any kind** from this plugin.

**The 39 sentinel.** Dexcom reports 39 mg/dL as its "LOW" value (`sbx.displayBg` maps `39 → 'LOW'`,
`401 → 'HIGH'`). `mgdl > 39` appears in five places: `simplealarms.js:21`, `bgnow.js:96` (with the candid
comment `//TODO maybe lower instead of expecting dexcom?`), `rawbg.js:64,84,97`, and
`treatmenttocurve.js:19`.

**A value of exactly 39 is in no category** — `bgnow.js`'s `notError` requires `entry.mgdl > 39` while
`isError` tests `!entry || !entry.mgdl || entry.mgdl < 39`, so at 39 both predicates are false. The
predictive engine agrees with neither: `ar2.js:6` sets `BG_MIN = 36; //Not 39, but why?`. And the **report
layer includes** a 39 reading: `reportclient.js:861–866` filters out only `d.y < 39` from reports, so 39
survives into averages and percentiles.

**Net effect (inference from the four code paths, no runtime observation):** the single most clinically
urgent value the sensor can report — "below the measurable floor" — is excluded from the glucose alarm,
is neither an error nor a non-error in the display logic, is included in derived statistics, and is
special-cased with four different constants across the codebase. The two `//TODO`/`//Not 39, but why?`
comments show the maintainers know the boundary is inconsistent.

### 5.7 Any token can silence any alarm; alarms broadcast to everyone

**Static-only finding, matching the README's own prose.** The README states: "Alarms are generated for
high and low values, **which can be cleared by any watcher of the data.**"

`lib/api3/alarmSocket.js`:

- `:46–47` — the socket accepts a `subscribe` message.
- `:52` — the bus's `notification` events are re-emitted onto the namespace: `ctx.bus.on('notification', self.emitNotification)`, and `emitNotification` calls `self.namespace.emit(...)`. **`socket.join` appears nowhere in the file**, so there are no rooms: every subscriber receives every alarm and every announcement, for every group.
- `:67–90` — the **native token** path validates the access token, then registers `socket.on('ack', ...)` at `:83`, calling `ctx.notifications.ack(level, group, silenceTime, true)` at `:84` **with no permission check**. Any valid token can acknowledge — silence — any alarm, globally.
- `:132` — the **JWT** path does check a scope: `ack: ctx.authorization.checkMultiple('notifications:*:ack', auth.shiros)`.

The project's own documentation (`lib/api3/doc/alarmsockets.md:74`) states that "Ne special rights are
required" (a typo for "No") to subscribe. Combined with the default anonymous-read policy (§5.1), the population able to see
alarms is effectively "anyone with the URL", and the population able to silence them is "anyone with any
token".

**Ack state is in-memory and wall-clock based.** `lib/notifications.js` holds `alarms` at module level;
`silenceTime` defaults to `THIRTY_MINUTES` (`:10`), `lastAckTime` starts at `0` (`:11`), and the silence
test is `ctx.ddata.lastUpdated > alarm.lastAckTime + alarm.silenceTime` (`:60`). `ddata.lastUpdated` is
refreshed on every data load (`lib/data/dataloader.js:60–80`), so silencing advances with wall clock
rather than with data. A process restart clears all acknowledgement and silence state.

**Hypothesis checked and refuted.** An earlier hypothesis held that `ddata.lastUpdated` freezes when data
stops arriving, which would freeze the silence comparison and could suppress an alarm indefinitely. It was
checked and **refuted**: `lastUpdated` is set to `Date.now()` on every load tick and the tick is not
driven by incoming data. The hypothesis is not recorded as a finding (§5.20).

### 5.8 A failing alarm plugin logs and disappears

**Static-only finding.** `lib/plugins/index.js:196–202`:

```
plugins.checkNotifications = function checkNotifications (sbx) {
  plugins.eachEnabledPlugin(function eachPlugin (plugin) {
    if (plugin.checkNotifications) {
      try {
        plugin.checkNotifications(sbx.withExtendedSettings(plugin));
      } catch (error) {
        console.error('Plugin error on checkNotifications(): ', plugin.name, error);
      }
```

The same pattern is used for `visualizeAlarm` (`:210–216`). The log names the failing plugin. There is
**no admin notification, no persisted state, and no UI indication** — even though the admin-notification
channel exists and is used for security warnings (§5.18). A plugin that throws on every tick is
indistinguishable, from the user's side, from a plugin that has nothing to report.

This is recorded as an architectural observation rather than a defect claim: swallowing a plugin exception
to keep the alarm loop alive is a defensible choice, and the trade-off was evidently made deliberately.
What is notable for the comparison is that the failure is **not surfaced**, in a subsystem whose purpose
is to surface things.

### 5.9 Two delete semantics in one process, against one collection

**Static-only finding.** The tree contains both answers, and both are live:

- **API3 soft-deletes.** `lib/api3/generic/delete/operation.js:20–24` routes a normal delete to
  `markAsDeleted`; `:80` sets `isValid: false` and a fresh `srvModified`. With `?permanent=true` it calls
  `deletePermanently` → `deleteOne`. Reads reject invalidated documents (`mongoCachedCollection/index.js:57`).
  `history` is a `srvModified` delta sync that **includes** invalidated documents, documented at
  `swagger.yaml:412` ("Deleted documents will appear with `isValid` = `false`").
- **API v1/v2 hard-delete.** `lib/server/entries.js:52` calls `await api().deleteMany(query_for(opts))`.
- **The admin tools hard-delete** (§5.11).

So "delete" means tombstone-with-history on one API, permanent removal on another, and permanent removal
with a confirmation dialog in the UI. A record's survival therefore depends on which client deleted it.
There is nothing wrong with a new API providing better semantics; the finding is that both are reachable
in the same process against the same data, so there is no single answer to "is this record gone?".

### 5.10 `autoPrune` is opt-in, hard-deletes, and its filter can reach recently modified documents

**Static-only finding.** `lib/api3/generic/collection.js:32`:

```
self.autoPruneDays = app.setENVTruthy('API3_AUTOPRUNE_' + colName.toUpperCase());
```

**No default argument.** `setENVTruthy` (`lib/api3/index.js:20–33`) looks up `CUSTOMCONNSTR_*` and plain
env vars, and coerces only the exact strings `on/true/off/false`; anything else is returned as the raw
string. With the variable unset the value is `undefined`, and `autoPrune` returns immediately because
`stringTools.isNumberInString(undefined)` is false (`collection.js:131–132`). A numeric string such as
`"7"` works, since `isNumberInString` accepts it. **Default behaviour: no pruning.**

**Hypothesis checked and refuted.** An earlier hypothesis held that `setENVTruthy` coerced the value to a
boolean and thereby broke numeric `API3_AUTOPRUNE_*` values. It was checked and **refuted**: the function
returns the raw string unless it is exactly one of the four truthy/falsy words. Not recorded as a finding
(§5.20).

**When enabled** (`collection.js:129–160`):

- It runs at most once per hour (`nextAutoPrune` is set to now + 3600 s at `:158`).
- The filter is a **three-way `$or`** (`:142–146`): `srvCreated < cutoff`, **or** `created_at < cutoff`,
  **or** `date < cutoff`.
- It calls `self.storage.deleteManyOr(filter, …)` (`:149`), which resolves to
  `utils.parseFilter(filterDef, 'or')` → a MongoDB `$or` (`lib/api3/storage/mongoCollection/{modify,utils}.js`).
- `$or` means **any one** condition suffices. A document created long ago but patched yesterday is deleted,
  because its `srvCreated` is old. `srvModified` — the field the system itself uses to mean "this changed"
  — is **not** among the prune criteria.
- It is a **hard delete**: no `isValid` tombstone, no `history` entry, no audit trail beyond a
  `console.info` count.
- Soft-deleted tombstones are pruned too.

### 5.11 The published autopruning default that the code does not implement

**Public claim (the project's own OpenAPI description) versus code.** `lib/api3/swagger.yaml` line 30, in
`info.description`: "Autopruning of obsolete documents (optional, **default is only `DEVICESTATUS`=60**)".
`swagger.json` carries the same text. The code implements no default (§5.10).

**Vendor-published internal document, third position.** `docs/audits/data-layer-audit.md` §5.4 documents
the behaviour as opt-in — `DEVICESTATUS_DAYS=2`, `ENTRIES_DAYS=0`, `TREATMENTS_DAYS=0`, with
"0 = no auto-prune" — which **agrees with the code**, while using **different environment-variable names**
from the ones the code reads (`API3_AUTOPRUNE_*`). An operator who set the audit's variable names would
configure nothing.

**Three surfaces, two positions.** The most conservative reading — nothing is pruned unless explicitly
configured, and even then the operator's expectation about *which* documents go is unreliable (§5.10) —
governs. The disagreement is recorded, not resolved.

### 5.12 No physiological validation anywhere in API3

**Static-only finding.** `lib/api3/generic/create/validate.js:15–24` checks exactly one thing before
delegating: that `doc.identifier` is a non-whitespace string. `lib/api3/shared/operationTools.js:56–75`
(`validateCommon`) then checks exactly three more:

- `date` is a number and greater than `apiConst.MIN_TIMESTAMP` (`946684800000`);
- `utcOffset`, if present, is a number within `[MIN_UTC_OFFSET, MAX_UTC_OFFSET]` (±1440);
- `app` is a non-empty string.

**There is no range check on any physiological value.** A glucose reading of `-500`, `0`, or `99999` is
stored. The project's `docs/requirements/data-shape-requirements.md` specifies the JSON *structure*
(single document versus array, and similar), not physiology, and its title is accurate about that: the
project validated shape, not values.

**This is the exact inverse of the AnxietyWatch2 rule.** `crates/vitals-decode` and the registry require a
value to be inside the type's declared range, reject out-of-range values with a named reason, quarantine
them rather than clamp them, and treat absence as distinct from a measurement (planned:
`assert!(b.rejected.is_empty(), "absent is not rejected")`). Nightscout, at the store boundary, has none of
that: the API accepts any number for the physiological field and the
clamping/sentinel handling that does exist is scattered in the plugins (§§5.6, 5.13).

### 5.13 Units are converted at many layers, and a presentation helper is available to computation

**Static-only finding.** Canonical storage is mg/dL with `MMOL_TO_MGDL = 18.01559`. `lib/units.js` provides
`mgdlToMMOL` (rounds to one decimal and returns a **string**) and `mmolToMgdl` (`Math.round` → integer).
More than twenty-five explicit conversion sites were found in `lib/` alone — `settings.js`, `data/ddata.js`,
`sandbox.js:258–273`, `client/renderer.js`, `client/clock-client.js:79,140`, report plugins,
`client-core/careportal/*`, `basalprofile.js:69`.

**A derived consequence (inference, arithmetic only, not run):** a 1 mg/dL round-trip through
`mgdlToMMOL` → `mmolToMgdl` is not lossless — `100 mg/dL → 5.6 mmol/L → 101 mg/dL`. The helper rounds for
display and returns a string, and it is reachable from computation.

**Where it actually went wrong in this release is §5.3.** The report series is scaled to display units and
then fed to an A1c formula written for mmol/L. The finding for the comparison is not "Nightscout converts
in many places" — that is a style difference. It is that **the display unit reached a clinical
derivation**, and the project shipped a wrong number as a result.

### 5.14 PHI and credentials are logged

**Static-only finding.** Two paths:

- `lib/api3/shared/operationTools.js:114–126` — on an `identifier` mismatch the code logs
  `console.warn(...)` and then `console.log(doc)` — the **whole document**, which for an entry is the
  glucose value, device and timestamp — and keeps the client-supplied identifier.
- `lib/api3/alarmSocket.js:74` logs `message.accessToken` on authorisation failure; `:123` logs
  `message.jwtToken`; and `:166`-region failure handling logs the whole message, which on the web path can
  carry a session token.

No runtime observation, and no claim is made about where those logs go in a given deployment. Recorded
because glucose values with device identifiers are health data, and because the project has an
admin-notification channel specifically for security warnings that is not used here.

### 5.15 The predictive alarm's thresholds are code literals, and its forecast is clamped

**Static-only finding.** `lib/plugins/ar2.js:5–11`:

```
var BG_REF = 140; //Central tendency
var BG_MIN = 36; //Not 39, but why?
var BG_MAX = 400;
var WARN_THRESHOLD = 0.05;
var URGENT_THRESHOLD = 0.10;
var AR = [-0.723, 1.716];
```

- `:194–209` (`checkForecast`) sets the level purely from `forecast.avgLoss` against the two literals.
  `selectEventType` (`:211–224`) only chooses the high/low **name** from the user's settings. So
  `alarmHigh`, `bgTargetTop` and similar settings **rename the event; they do not move the trigger**.
- `:101` — the loss uses `Math.pow(log10(result.predicted[j].mgdl / 120), 2)`, a **hard-coded 120**,
  while the forecast itself is referenced to `BG_REF = 140` at `:272–273`. Two different centres in one
  engine.
- `:302–308` — `mgdl` is computed and then clamped: `Math.max(BG_MIN, Math.min(BG_MAX, mgdl))`, and
  `:54–55` builds the displayed line from the clamped value. A forecast below 36 is displayed as 36.

**A guard that does not do what it appears to.** `:62` is
`if (sbx.time - sbx.lastSGVMills() > times.mins(10).msecs) return;`. When there is no entry,
`sandbox.lastSGVMills()` returns `undefined` (`lib/sandbox.js:184–190`), so the comparison is
`NaN > n`, which is `false`, and the guard does **not** return. The condition ends up handled further down
by `okToForecast` (`:252–261`, requiring `bgnow.mean >= BG_MIN`), which produces no forecast — so the
outcome is still no alarm, via a different path (inference from the two functions; not run).

**The comparison point.** AnxietyWatch2's registry is the single source of truth for types, units, ranges
*and thresholds*, and the rule is explicit: never hardcode a range or a threshold. `ar2.js` is the exact
opposite, in a file whose alarm settings a user would reasonably believe control the alarm.

### 5.16 The security audit's role table does not match the code

**Vendor-published internal document versus code.** `docs/audits/security-audit.md` (January 2026) states
that the audit found a role table that does not match the code — it omits `status-only`; it gives
`readable` as `api:*:read, notifications:*:ack` where the code has `*:*:read`
(`lib/authorization/storage.js:144`) and where the `:ack` scope is a *socket* check performed separately
(`lib/api3/alarmSocket.js:132`); and it names a role `activity-create` where the code names it `activity`.

Recorded as divergence at the pinned commit, with the audit's date and the commit's date both given. The
permission *strings* are the enforcement mechanism, so an operator configuring roles from the audit is
configuring from an approximation.

### 5.17 The brute-force delay list is a one-shot, and the audit calls it a strength

**Static-only finding.** `lib/authorization/delaylist.js`:

```
:7    const DELAY_ON_FAIL = env?.settings?.authFailDelay ?? 5000;
:8    const FAIL_AGE = 60000;
:15   ipDelayList[ipString] = now + DELAY_ON_FAIL;
:19   ipDelayList[ipString] = entry + DELAY_ON_FAIL;
:43   setTimeout(function clearList () {
:46     if (Date.now() > ipDelayList[key] + FAIL_AGE) { ... }
```

`clearList` is scheduled **once**, 30 seconds after module load, via `setTimeout`. `setInterval` does not
appear in the file. The inline comment says "Clear items older than a minute", and the project's
`docs/requirements/authorization-security-requirements.md` (REQ-BRUTE-003) specifies clearing 60 seconds
after the last delay. After the first 30 seconds the pruning function never runs again, so the map grows
without bound and any entry recorded after that point is **never cleared** — a delayed address stays
delayed until the process restarts, and a client that trips the delay repeatedly accumulates entries.

**Vendor-published internal document.** `docs/audits/security-audit.md` describes this subsystem as a
strength and states that it "Auto-clears entries after 60 seconds of inactivity". It does not, at the
pinned commit.

### 5.18 Three separate admin-notification channels

**Static-only finding.** The project surfaces administrative warnings through three mechanisms that do not
share an implementation:

1. **A bus event with a single emitter.** `lib/adminnotifies.js:35` consumes an `admin-notify` event; the
   only emitter in the tree is `lib/authorization/index.js:212` (authentication failure).
2. **Direct boot-time injection into `env.notifies`.** `lib/server/env.js:115` pushes a persistent
   "Weak API_SECRET detected" notice when the secret's measured entropy is below 60 bits, and `:121`
   pushes "MongoDB password and API_SECRET match" — both with concrete, actionable text. These are copied
   into `ctx.adminnotifies` at `lib/server/bootevent.js:22–30`.
3. **A boot-time notice for the access-control default.** `lib/server/bootevent.js:149–152`, `persistent:
   true`, "Nightscout readable by world".

`adminNotifiesEnabled` defaults to `true` (`lib/settings.js:71`), and `adminnotifies.clean` drops
non-persistent notices after 12 hours on a tick.

**Recorded as architectural spread, not as a defect.** Each channel is defensible; the observation is that
"warn the operator about something" has three implementations, and the loudest warning in the product —
"readable by world" — shares no code with the security warnings raised for a weak secret, which means the
discovery of one does not imply the discovery of the other.

### 5.19 Version and documentation drift

**Static-only finding.** `CHANGELOG.md`'s newest entry is `## [15.0.7] - 2026-03-XX (Unreleased)`, while
the package version and the GitHub release both say **15.0.8** (published 2026-09-04), and the
documentation site refers to 15.0.8 behaviour. The changelog is therefore behind the release record and
cannot be used to establish what a version changed.

Two smaller items in the same class: `cleanentriesdb.js:22`'s action description for deleting entries
older than 180 days reads "Useful when uploader battery status is not properly updated" (a copy-paste from
another action), and `docs/audits/security-audit.md`'s environment table lists `mongodb ^3.6.0` while
`package.json` pins `^5.9.2`.

### 5.20 Hypotheses checked and refuted (not recorded as findings)

Recorded because the package's rule is that a plausible hypothesis is not a finding until it is checked,
and because one of these shows what would have happened if it had not been.

| Hypothesis | Check | Result |
|---|---|---|
| `ddata.lastUpdated` freezes when data stops, freezing the alarm-silence comparison and suppressing alarms indefinitely | `lib/data/dataloader.js:60–80`, `lib/server/bootevent.js:310–335`, `lib/bus.js:35` | **Refuted.** `lastUpdated` is set to `Date.now()` on every tick, and the tick is driven by the clock, not by arriving data. |
| Documentation claims `activity` data is sanitised but the purifier's collection list omits it, so the docs overstate | `lib/server/activity.js` imports, `lib/server/storage-purifier.js`, `lib/api3/shared/writePurifier.js` | **Refuted.** `lib/server/activity.js` does import the storage purifier. The documentation is accurate. |
| The documented "readable by the world" admin message has no implementation | `lib/server/bootevent.js:149–152` | **Refuted.** It exists, with `persistent: true`. |
| `setENVTruthy` coerces values to booleans, breaking numeric `API3_AUTOPRUNE_*` configuration | `lib/api3/index.js:20–33`, `lib/api3/generic/collection.js:131–134` | **Refuted.** It returns the raw string except for the four exact truthy/falsy words; `"7"` works. |
| The `force_typed_data` `TODO` is accurate: the type fallback "doesn't work" | `lib/api/entries/index.js:61–69`, `:130–133` | **Refuted at the pinned commit.** `type_params.type` is a string (`req.query.find.type` / `req.params.model`) and `data.type = opts.type` assigns it. The `TODO` appears stale. Had it been trusted, the package would have recorded a false defect and missed the real finding (absence is *stamped* with a known type). |

---

## 6. Runtime observation

**None, and this section exists to make that unambiguous.**

| Would-be observation | Status | What it would require |
|---|---|---|
| Does the application start? | **not observed** | `npm install`, a MongoDB instance, `npm start` |
| Does the default expose data to an unauthenticated caller? | **not observed** (code says it does; §5.1) | starting instance + an HTTP request |
| Does the mg/dL A1c figure render as 95.8%? | **not observed** (documentation says it does and the code explains it; §5.3) | starting instance + a browser, or a unit test of the function |
| Do plugins' alarms fire? | **not observed** | starting instance + a data feed |
| Does the delay list stop pruning? | **not observed** (code says it does; §5.17) | long-running process + repeated failed auth |
| Does pruning delete a recently modified document? | **not observed** (code says it can; §5.10) | instance + MongoDB + `API3_AUTOPRUNE_*` set |
| Does the alarm socket broadcast cross-group? | **not observed** (code says it does; §5.7) | instance + two WebSocket clients |

**Every statement in §5 is a reading of source code.** Code that is never run is evidence of intent and
structure; it is not evidence of behaviour. A deployment may differ from the tree — an operator may set
`AUTH_DEFAULT_ROLES=denied`, may disable the ar2 plugin, may never enable auto-pruning. The findings are
statements about **what the released source says**, and the documentation-drift findings are statements
about **what the released documentation says relative to that source**.

**A decision deliberately not taken.** Because the stack is open source, a local instance would have been
possible, and it is the single most valuable step available for this candidate. It was not performed: it
requires installing a database and a Node application from the internet onto a machine, and that is a
separate authorisation decision from reading a repository. It is recorded in §8 as the top evidence gap,
and in `PROMPT_2.md` as the first thing to do if the owner authorises a runtime phase.

---

## 7. Claim reconciliation

Public claims checked against the pinned source. "Matches" means the code does what the claim says.

| # | Public claim | Source | Verdict |
|---|---|---|---|
| 1 | "open-source cloud application … visualize, store and share … in real-time" | documentation | **Matches** (self-hosted web app; "cloud application" is the project's own framing of an internet-reachable service) |
| 2 | "not supported by any company"; "DIY … supported by volunteers" | documentation | **Matches** (no vendor entity; a 501(c)(3) foundation funds the ecosystem) |
| 3 | "In a default setup, anyone with your Nightscout URL can view your site." | security docs | **Matches** (`settings.js:39`, `authorization/storage.js:144`) |
| 4 | "Nightscout itself does not directly interface with any devices." | security docs | **Matches** (no device transports in the tree) |
| 5 | Sanitisation covers entries, treatments, profiles, devicestatus, food and activity | security docs | **Matches** (all six import the purifier; hypothesis of overstatement refuted, §5.20) |
| 6 | "Alarms … can be cleared by any watcher of the data." | README | **Matches** (`alarmSocket.js:83–90`, no rooms) |
| 7 | Glucose predicted 0.5 h ahead with a second-order autoregressive model | README | **Matches** (`ar2.js:11`, `AR = [-0.723, 1.716]`) |
| 8 | A1c/GMI "are not laboratory measurements and must not replace clinical testing" | reports docs | **Matches** (labels are marked `est*`; formula cited) |
| 9 | Daily Stats A1c "is wrong when glucose is displayed in mg/dL (for example 95.8% …) … The value shown in mmol/L is correct." | reports docs | **Matches, mechanism identified** (`reportclient.js:847` + `dailystats.js:111`; arithmetic reconciles, §5.3) |
| 10 | Remote notifications are achieved "through a combination of Nightscout, IFTTT, Google, and Pushover" | notifications docs | **Matches** (the plugin path and the IFTTT path both exist; the docs describe the IFTTT path as the recommended one) |
| 11 | Pushover acknowledgement causes alarm fatigue when unacknowledged | notifications docs | **Recorded as the vendor's own statement** about a third-party service; no code contradicts it |
| 12 | Admin Tools delete records by date range / remove old profiles / remove future items | admin docs | **Matches** (`lib/admin_plugins/*`) |
| 13 | Autopruning "default is only `DEVICESTATUS`=60" | API3 swagger | **Does not match.** No default is implemented (`collection.js:32`); the vendor's own audit agrees with the code. Most conservative governs (§5.11) |
| 14 | `ALLOW_UNRESTRICTED_FRAME_EMBEDDING` "temporary default is true" | security docs | **Matches** (`env.js:85`) |
| 15 | The Foundation is a 501(c)(3) with EIN 47-2130306 and no industry funding | foundation site | **Recorded as the Foundation's claim**; not verified against any register |
| 16 | Security audit: token derivation, JWT signing key, 1-hour lifetime | audit doc | **Does not match.** Code differs in all three; the requirements document beside it matches the code (§5.2) |
| 17 | Security audit: delay list "Auto-clears entries after 60 seconds of inactivity" | audit doc | **Does not match.** One-shot 30 s `setTimeout` (§5.17) |
| 18 | Security audit: role table | audit doc | **Does not match.** Omits `status-only`; misstates `readable`; renames `activity` (§5.16) |
| 19 | Security audit environment: `mongodb ^3.6.0` | audit doc | **Does not match.** `package.json` pins `^5.9.2` |
| 20 | "Nightscout 15.0.8 … fixed in the next release" (A1c) | reports docs | **Forward-looking**; the pinned commit is the 15.0.8 release commit, so the fix is not in the inspected revision |

**Summary.** Of twenty checkable claims, **fourteen match, five do not, and one is forward-looking**. Four
of the five failures are in the project's own security audit and its own OpenAPI text — the documents a
third party would read to understand the system without reading the code.

---

## 8. Evidence gaps

1. **No runtime observation of any kind** (§6). This is the largest gap in the package for this candidate,
   and the only one a runtime phase would close.
2. **The alarm delivery transports were not read.** `pushover`, `maker`, `speech`, `alexa` and
   `googlehome` are identified as plugins but not inspected, so §5's alarm findings rest on `ar2`,
   `simplealarms`, `timeago`, `bgnow`, `webhook` and the notification/socket layers. The IFTTT/Pushover
   description in the claimed-features file is documentation-based.
3. **Time handling was not read.** `lib/times.js` and the clock/profile paths (including DST handling and
   the `device_time_corrections` equivalent) were not examined, so no statement is made about time-zone or
   clock-skew correctness.
4. **Retention beyond API3 was not examined.** The `lib/storage/*` index definitions and any TTL indexes
   were not read; the claim in §5.10 that default retention is unbounded is based on the application-level
   pruning path, not on MongoDB index configuration.
5. **The test suite was counted (141 files) but not read or run**, and the project's own
   `docs/test-specs/{coverage-gaps,flaky-tests}.md` were not read. No statement is made about coverage or
   pass rate.
6. **The API3 search/read/projection paths were not read in depth**, so nothing is said about pagination
   correctness, projection leakage or dedup behaviour, other than the constants in `const.json`.
7. **`lib/client/` and the browser rendering layer were only sampled**, so visual-state claims (for
   example how `toStatusClass`'s single `'current'` class actually appears to a user) rest on the class
   mapping, not on the CSS.
8. **The MG/dL LOW/HIGH sentinel mapping** was read in `sandbox.js`/`bgnow.js` only; the full set of
   callers of `displayBg` and the 401 boundary was not enumerated.
9. **Whether any finding is known, tracked or fixed upstream is unknown.** Only the pinned commit was
   examined, and `master` has moved since (last push 2026-10-04, two days before the pin's inspection
   date).
10. **The other `docs/proposals/` RFCs were not read** (OIDC actor identity, conflict resolution, MongoDB
    modernisation, testing modernisation), so nothing is claimed about the project's roadmap.

---

## 9. Ethical and legal limits applied

- **Read only.** No file in the clone was modified; no branch, tag, issue, PR or comment was created on
  the upstream project.
- **No execution, no deployment, no network service.** Nothing was built, started, installed or run;
  no MongoDB was installed; no Nightscout instance — the project's or anyone else's — was contacted.
- **No credentials.** No Nightscout account, token, API secret or API key was created, requested, used or
  sought.
- **No patient data.** Nothing in this inspection involved a real person's glucose data. All example
  values in §5.3 are arithmetic substitutions made here.
- **No third-party code reproduced.** Findings are described in my own words with short quotes and
  `file:line` references. No source is copied into AnxietyWatch2, and §5 does not alter the
  `cpap-ezshare-ingest` boundary rule ("no GPL implementation code is copied") in any way — no code from
  this inspection is intended for reuse.
- **Licence respected.** AGPL-3.0 grants the right to read, study and modify. The licence's source-
  availability obligations attach to distribution of the software or of a modified version, neither of
  which occurs here.
- **Vendor documents attributed, not adopted.** Every quotation from `docs/audits/`, `docs/requirements/`
  or `docs/proposals/` is identified as the project's own statement with its own date and status, and is
  never presented as independent validation.
- **No claim of authority.** This inspection describes the inspected revision. It does not claim to
  describe any other version, any deployment, or the project's current state.

---

## 10. Reproduction

Every step is read-only and requires no credentials. Expect the outputs shown; the hashes pin the exact
revision inspected.

```sh
# 1. Obtain the source at the inspected revision (public, no authentication).
git clone https://github.com/nightscout/cgm-remote-monitor.git /tmp/aw2research/nightscout/repo
cd /tmp/aw2research/nightscout/repo
git checkout 92d0834219aa771b5837dbcbf1baeb839a200cf6

# 2. Confirm the revision.
git rev-parse HEAD
# expect: 92d0834219aa771b5837dbcbf1baeb839a200cf6
git log -1 --format='%H %ci %s'
# expect: ...2026-09-04 19:12:30 +0100 Merge pull request #8482 from nightscout/dev

# 3. Confirm the version and licence.
python3 -c "import json;d=json.load(open('package.json'));print(d['version'],d['license'],d['engines'])"
# expect: 15.0.8 AGPL-3.0 {'node': '>=20.x'}

# 4. Confirm the default access policy (finding 5.1).
grep -n "authDefaultRoles" lib/settings.js
# expect: 39:    , authDefaultRoles: 'readable'
grep -n "readable" lib/authorization/storage.js
# expect: a role whose permissions are [ '*:*:read' ]
grep -n "readable by world" lib/server/bootevent.js
# expect: the persistent admin notice

# 5. Confirm the unit-dependent A1c (finding 5.3).
grep -n "scaleMgdl" lib/report/reportclient.js | head
# expect: the report series is built from client.utils.scaleMgdl(...)
grep -n "MMOL_TO_MGDL" lib/report_plugins/dailystats.js
# expect: the A1c formula multiplies the already-scaled average

# 6. Confirm the absent-data posture (finding 5.4).
grep -n "assume current" lib/plugins/timeago.js
grep -n "bless absence" lib/api/entries/index.js

# 7. Confirm the alarm gates and the 39 sentinel (finding 5.6).
grep -n "mgdl > 39" lib/plugins/simplealarms.js lib/plugins/bgnow.js
grep -n "BG_MIN" lib/plugins/ar2.js

# 8. Confirm the alarm socket has no rooms and no scope check on the token path (finding 5.7).
grep -c "socket.join" lib/api3/alarmSocket.js
# expect: 0
sed -n '71,95p' lib/api3/alarmSocket.js

# 9. Confirm the opt-in prune with the three-way $or (finding 5.10).
grep -n "API3_AUTOPRUNE" lib/api3/generic/collection.js
sed -n '129,160p' lib/api3/generic/collection.js

# 10. Confirm the swagger default that is not implemented (finding 5.11).
grep -n "default is only" lib/api3/swagger.yaml

# 11. Confirm no physiological validation (finding 5.12).
sed -n '15,24p' lib/api3/generic/create/validate.js
sed -n '56,80p' lib/api3/shared/operationTools.js

# 12. Confirm the one-shot delay list (finding 5.17).
grep -n "setTimeout\|setInterval" lib/authorization/delaylist.js
# expect: one setTimeout, no setInterval

# 13. Confirm the JWT key provenance and lifetime (finding 5.2).
grep -n "randomString\|'8h'\|setJWTKey" lib/server/enclave.js

# 14. Confirm the version/changelog drift (finding 5.19).
head -5 CHANGELOG.md

# 15. Confirm the vendor's own audit statements (findings 5.2, 5.16, 5.17).
grep -n "API_SECRET\|60 seconds\|mongodb" docs/audits/security-audit.md | head

# 16. Confirm the vendor's requirements document agrees with the code, not the audit (finding 5.2).
grep -n "REQ-AUTH-02[123]\|REQ-BRUTE-003" docs/requirements/authorization-security-requirements.md
```

**Documentation artifacts.** The web pages used for the public-claims file were fetched with a browser
user agent and are hashed in §3.1; re-fetching them later may not reproduce the same bytes, since the
documentation site is a living site.

---

## Related files

- `nightscout_CLAIMED_FEATURES_AND_PURPOSE.md` — public claims, pricing, governance and the evidence table.
- `PROMPT_2.md` — the candidate-specific second-stage prompt, built on the unresolved questions below.
- `nightscout_COMPARISON.md` — the comparison against AnxietyWatch v1 and AnxietyWatch2.
- `../RESEARCH_METHOD.md` — the inspection-mode taxonomy this document belongs to, and its limits.
- `../PROCESS_STATE.md` — candidate 6's position in the queue and its residual uncertainty.
