# Nightscout — claimed features and purpose

Candidate 6 of 10 in `docs/app_comparison/CANDIDATE_SHORTLIST.md`.
Research date: **2026-10-06**. Web sources fetched on that date; source reading performed against a pinned
commit (see §1.1).

**Evidence classes used throughout.** Each claim is labelled with exactly one of:

- **public claim** — the vendor's own published statement (project documentation site, README, foundation
  site, app-store listings, repository metadata)
- **vendor-published internal document** — a document the project authors and ships in its own repository
  (audits, requirements, design proposals). This is the *vendor's* statement about itself. It is quoted
  with its own date and status. **It is not independent validation, and its numbers were not re-measured
  here.**
- **static-only finding** — read out of the published source tree, not from running anything
- **runtime observation** — measured by running software. **There are none in this file.** One was
  accidentally possible under a generous reading and is explicitly declined in §1.2.
- **inference** — my reasoning from evidence, explicitly labelled
- **unknown** — no evidence found in either direction

**Recurring reading rule applied here.** Where two current Nightscout surfaces disagree, the **most
conservative of the two governs** for the purpose of describing the product, and the disagreement is
recorded rather than resolved. Where the project's docs disagree with the project's code, that is recorded
as **documentation drift** and neither side is silently preferred.

---

## 1. Executive summary

Nightscout is a **self-hosted, open-source web application for storing, visualising, sharing and alerting
on continuous glucose monitor (CGM) data**. It is the reference implementation of the #WeAreNotWaiting
movement that began in 2013, when John Costik sent his son's CGM readings to a web page so a school could
watch them. It is maintained by volunteers, licensed **AGPL-3.0**, and deployed by the user onto their own
infrastructure — a Raspberry Pi, a Docker host, a NAS, a PaaS free tier, or one of ten commercially
advertised hosting services.

It is on the shortlist because it is the **closest existing model for the layer AnxietyWatch2 is building
underneath the clinical features**: a self-hosted, user-owned service that continuously ingests
physiological data from a wireless device, stores it without loss, derives statistics from it, shares it
with a chosen audience, and raises alarms when something is wrong. That architectural overlap is high. The
**clinical** overlap is low — Nightscout's physiology is glucose and insulin, and it has no CPAP, SpO₂,
respiratory, anxiety, medication-adherence or context dimension.

Six findings shape everything that follows.

1. **It is the only candidate with no company behind it, and one of only two with a governance
   structure.** The software is maintained by volunteers; the organisation that funds the surrounding
   ecosystem is the **Nightscout Foundation, a US IRS-recognised 501(c)(3) non-profit** (EIN
   `47-2130306`), which states it receives no funding from and has no affiliation with Dexcom, Insulet,
   Tandem, Medtronic, BetaBionics, Sequel, Abbott, Senseonics, or any other commercial diabetes company.
   The software documentation simultaneously says the project is "**not supported by any company**".
   Both statements are consistent and the distinction — a non-profit funding an ecosystem, not a vendor
   supporting a product — matters for the comparison.
2. **Its default access-control setting publishes your data, and the project says so.** The documentation
   warns in a callout: "**In a default setup, anyone with your Nightscout URL can view your site.**" The
   shipped default is `authDefaultRoles = 'readable'`, which grants anonymous callers read on everything
   (§10). This is the single most important security fact about Nightscout, and the project discloses it
   prominently — a positive for candour that does not change the exposure.
3. **Alarm delivery to a phone is delegated to third-party cloud services, and the vendor documents the
   resulting alarm fatigue as a known drawback.** The integrated path is a Pushover plugin; the
   documentation's recommended path is Nightscout → IFTTT → Google Drive/Sheets → IFTTT → Pushover, four
   external services and three accounts. The docs state plainly that Pushover's mandatory acknowledgement
   "may end up getting alarm fatigue fairly quickly as the alarm repeated itself."
4. **A derived clinical statistic was shipped wrong in the inspected release, and both the cause and the
   numbers are verifiable.** The documentation for **15.0.8** carries a note that the Daily Stats estimated
   A1c "is wrong when glucose is displayed in mg/dL (for example **95.8%** instead of 6.9%)". Reading the
   source explains it exactly: the report series is scaled into the user's display unit
   (`reportclient.js:847`), and the A1c formula then multiplies that already-scaled average by
   `MMOL_TO_MGDL` a second time (`dailystats.js:111`). A unit preference leaks into a derived clinical
   statistic. This is the cleanest single illustration in the whole package of why AnxietyWatch2's rule —
   one canonical unit, converted once, at the decoder — exists.
5. **"Not measured" is deliberately rendered as "current", in two independent places, with candid
   comments.** `lib/plugins/timeago.js:103–104` carries `//assume current` for the case where there is no
   reading at all; `lib/api/entries/index.js:66` carries `// bless absence with known type`. Neither is a
   bug report or a TODO. They are the codebase stating its posture toward absent data, and it is the
   opposite of the three-state rule AnxietyWatch2 is built around.
6. **The project runs its own audit → requirements → test-spec traceability programme, and three of the
   resulting documents do not describe the code at the inspected commit.** `docs/meta/DOCUMENTATION-PROGRESS.md`
   records an "audit-first methodology" begun in January 2026 with formal requirements and test
   specifications, marked *Complete* for data-shape, API v1 compatibility and authorisation/security. The
   security audit's description of token derivation, JWT signing and JWT lifetime is wrong; the
   **requirements** document published alongside it is right; the code is right (§5.2). The audit's role
   table is incomplete. This is not an argument that the programme is worthless — it is an argument that
   "the vendor audited it" is not a substitute for reading the code, which is also the conclusion the
   packages for OSCAR and SleepHQ reached by different routes.

### 1.1 What "inspection" means for this candidate

Nightscout is the **second candidate to reach full source inspection** (`oscar` was the first). The
software is AGPL-3.0 and published in full, so the implementation can be read rather than inferred. The
pinned tree is recorded in `nightscout_IMPLEMENTATION_INSPECTION.md` §3; every `file:line` in this
document was re-verified against that tree at the time of writing, not carried over from a secondary
document.

### 1.2 Why there are no runtime observations

Because the stack is open source, it is *technically* possible to install it and run it locally, and that
would have produced runtime observations. It was **not done**, and the reason is scope, not difficulty:
standing up a MongoDB instance to observe Nightscout is a separate authorisation decision with its own
resource cost, and the Phase 2A rule for this package is that a new runtime surface requires an explicit
decision. Nothing here should be read as a claim that the software does or does not run.

---

## 2. Positioning

| | |
|---|---|
| Self-description | "Nightscout (also known as **CGM in the Cloud**) is an open-source cloud application used by people with diabetes and parents of kids with diabetes to visualize, store and share the data from their Continuous Glucose Monitoring sensors in real-time" (documentation landing page) |
| Repository description | "nightscout web monitor" (GitHub API, 2026-10-06) |
| Category the project places itself in | DIY diabetes technology; the anchor of the #WeAreNotWaiting ecosystem alongside Loop, Trio and OpenAPS/AndroidAPS |
| Stated model | one Nightscout site holds **one person's** data; the ecosystem has three roles — **Site** (the server), **Uploader** (a phone or device pushing data in), **Follower** (a browser or app reading it out) |
| Supported by | "Unless you're paying for a hosted Nightscout including support, this project is 'Do It Yourself' (DIY) and supported by volunteers" |
| Support claim | "**not supported by any company**" (documentation landing page) |
| Stated values | "free to contribute, free to access, free to criticize, free to try, free to depend on, free to modify, and free to distribute" |
| Cost | "**free** - $0 with this option" |
| Dependency the project itself names | "This project requires a working internet connection and availability of third-party cloud services" |

The last row is a rare thing to find a self-hosted project state about itself, and it is candid: Nightscout
is self-hosted but not self-contained.

---

## 3. Identity, legal and governance

There are **two distinct entities** here, and conflating them would misdescribe the project.

**The software.** `nightscout/cgm-remote-monitor`, AGPL-3.0, default branch `master`. The documentation
states the project is not supported by any company and is run by volunteers. No commercial entity is
named as the maintainer on the documentation landing page.

**The Foundation.** `https://www.nightscoutfoundation.org/` (linked as the organisation blog from the
GitHub organisation) describes itself as:

- "a recognized 501(c)3 nonprofit that relies on donations to fund its mission"
- "a US IRS Recognized 501(c)3 Organization", Tax EIN `47-2130306`
- a body that "began as a grassroots movement initiated by a group of parents, engineers, and advocates",
  tracing to John Costik's 2013 DIY project and the #WeAreNotWaiting movement
- a supporter of Nightscout *and* of the automated insulin delivery projects Loop, Trio and
  OpenAPS/AndroidAPS
- explicit that it "does not have any type of affiliation or receive funding from any of the corporations
  mentioned", naming Dexcom (G6, G7, Follow), Insulet (Omnipod Dash, O5), Tandem (t:slim), Medtronic,
  BetaBionics (iLet), Sequel (Twiist), Abbott (Libre) and Senseonics (Eversense) **"and any other
  commercially available products"**
- a provider of scholarships for STEM students, conference funding for developers, and volunteer-run
  support communities

**Vendor-published comparative claim, recorded as a claim.** The Foundation site also states: "The quality,
expertise, and response time of the Nightscout support teams **exceeds that of most commercially available
diabetes-related technologies**." Nothing found measures this, and the comparison is made by the body
operating the support teams. It is recorded here as an unsupported self-assessment, in the same class as
SleepHQ's "completely secure at all times" and Welltory's contested claims. It is not evidence of quality
in either direction; it is evidence of how the project describes itself.

**What this means for the comparison.** The Foundation gives Nightscout something no other candidate in
this package has: a **non-commercial legal entity** with a stated anti-industry-funding position, a
charitable purpose, and an ecosystem it funds. That is structurally the nearest thing on the shortlist to
AnxietyWatch2's "no company, no account, private" posture — and it is worth being precise about the
difference: the Foundation does **not** operate a service for users. Users operate their own sites. The
Foundation funds the ecosystem and the projects; it is not a data processor and publishes no privacy
policy (§10.3).

---

## 4. Availability and platforms

### 4.1 There is no first-party mobile application

Nightscout is a web application. The project publishes **no iOS or Android app of its own**; the ecosystem
provides third-party uploaders (xDrip+, Loop, Trio, AndroidAPS, and dozens of others) and third-party
followers/clients. The documentation's own framing of the three roles (Site / Uploader / Follower) is
built around this.

Consequence for comparison: **Nightscout has no push notification of its own.** Everything a phone shows
either comes from the web app running in a browser, or from a third party. This is a structural difference
from both AnxietyWatch v1 (an iOS app) and AnxietyWatch2 (Rust daemon + iOS client), and it explains why
alarm delivery is assembled out of IFTTT and Pushover (§7.3).

### 4.2 Repository metadata (GitHub API, 2026-10-06)

| Field | Value |
|---|---|
| Stars | 2,836 |
| Forks | **73,382** |
| Watchers/subscribers | 290 |
| Open issues | 142 |
| Created | 2014-05-22 |
| Last push | 2026-10-04 |
| Archived | false |
| Default branch | `master` |
| Repository size | 37,212 KB |
| Licence | AGPL-3.0 |
| Latest release | **v15.0.8**, "Release 15.0.8 Chocolate Chip Muffin", published **2026-09-04** |
| GitHub organisation | `nightscout` (39 public repos, 551 followers, created 2014-05-21) |

**Reading the fork count correctly.** The 73,382 forks against 2,836 stars is not a community-size
signal. Nightscout's documented deployment method includes "fork the repository and deploy from your
fork" (the project's own GitHub-based update and redeploy instructions exist precisely because users
deploy from a fork), so a large fraction of those forks are **running installations, not contributions**.
Recorded as an inference from the deployment model documented by the project itself. The star count,
watchers and 142 open issues are the conventional signals, and by those Nightscout is a small, long-lived,
widely-deployed project rather than a large contributor community.

### 4.3 Deployment targets documented by the project

Docker, Raspberry Pi, Ubuntu, NixOS, Synology NAS, MongoDB Atlas, Azure, Northflank, Railway, Fly.io,
Render, Google Cloud, and Heroku (with separate Heroku Eco and Basic plan pages). Shared community
tooling: `start.sh`, `Makefile`, `Dockerfile`, `docker-compose.yml`, `azuredeploy.json`, `app.json`,
`Procfile`, `web.config`.

### 4.4 Engineering hygiene visible in the repository

- **141 files** in `tests/` (counted at the pinned commit).
- Test runner configured via `Makefile` with explicit MongoDB connection and `AUTH_FAIL_DELAY` settings for
  the test environment; the `Makefile` carries a candid comment about why `mocha` is invoked through an
  "odd path" and about coverage instrumentation (attributed to `XXX.bewest`).
- `.github/workflows/`: `main.yml`, `codeql-analysis.yml`, `close-accidental-sync-prs.yml`.
- A `NODE_ENV=test` guard was added in 15.0.7's unreleased notes specifically to prevent "accidental
  production database modification" from tests — a real safety control, added deliberately.
- The repository contains an extensive **vendor-published internal documentation programme**: `docs/audits/`
  (`data-layer-audit.md`, `security-audit.md` and others), `docs/requirements/` (data shape, API v1
  compatibility, authorisation/security), `docs/test-specs/` (authorisation tests, shape handling tests,
  coverage gaps, flaky tests, manual smoke checklist), `docs/proposals/`, `docs/data-schemas/`, and
  `docs/meta/DOCUMENTATION-PROGRESS.md`, which describes an "audit-first methodology" started in January
  2026 with traceability matrices, marking data shape, API v1 compatibility and authorisation/security as
  *Complete*.

That last item is a real strength, and it is also the source of the sharpest finding in §5.2: a project
that publishes formal requirements and test specifications for a subsystem can still have its **audit**
document disagree with its own **requirements** document and its own code.

### 4.5 Documentation-and-version drift

`CHANGELOG.md`'s most recent entry is `## [15.0.7] - 2026-03-XX (Unreleased)`, while `package.json` and the
GitHub release record **15.0.8**, published 2026-09-04 — and the documentation site references 15.0.8
behaviour ("Since 15.0.8, the table also shows an estimated A1c…"). So the shipped release record is
**ahead of** the changelog, and the changelog is not a complete release history. Recorded as a
documentation-hygiene item, not a functional defect; its practical effect is that the changelog cannot be
used to establish what a given version changed.

---

## 5. Pricing, hosting and operating model

### 5.1 The software is free

"**free** - $0 with this option" (documentation landing page). The licence is AGPL-3.0. There is no
account, no subscription, no in-app purchase and no feature gate in the software.

### 5.2 The advertised hosting market

The documentation landing page lists **ten commercial hosting services with pricing links**:

| Service | Notable published details |
|---|---|
| T1Pal | associated with Ben West in the project's own history |
| NS10BE | `ns.10be.de/en/pricing.html`; servers selectable in **Germany, Finland or France**; "A backup of your database and settings is performed every 6-8 hours"; uptime monitoring; CSV import |
| Nightscout Pro | `nightscout.pro/#pricing`; page dated 2022 |
| Serendipity Bio | `serendipitybio.com/pricing` |
| Nightscout4u | — |
| Nightscout-easy | `nightscout-easy.ru` |
| OpenSource.Clinic | `opensource.clinic/pricing`; Netherlands; NEN 7510-certified data centres claimed |
| diab.ninja | — |
| Nightscout Host | `nightscout.host/#pricing`; Sweden |
| nightscout.host | — |

A free, AGPL-licensed project generating a ten-provider paid market around itself is a legitimate and
unusual outcome, and it is the mechanism by which most users who are not willing to run a server get one.

### 5.3 An unreconciled conflict inside the project's own published surfaces

The same project that advertises ten third-party operators writes in its security documentation:

> "The system has not been designed for scenarios where a third party operates a Nightscout installation on
> behalf of others and if such installations are made, both the users and operators of such installations
> need to understand there is a large body of legislation across the world that covers use cases and data
> storage enabled by Nightscout and **the liability issues around such installations are extremely
> complicated**."

Both statements are current, on the project's own sites. Recording the disagreement rather than resolving
it: the most conservative reading — and the one the project itself uses when warning users — is that
third-party hosting is **not a supported operating mode**, notwithstanding that it is prominently
advertised and is how most non-technical users will run the software. The ten providers do not appear to
be making that disclosure on the project's behalf.

---

## 6. Data model and API

### 6.1 Collections

The documented data shapes cover glucose **entries** (sgv, mbg, calibration), **treatments** (insulin,
carbs, boluses, temporary basals, site/pump changes), **profiles** (basal schedules, carb ratios, insulin
sensitivity, target ranges), **devicestatus** (pump, loop, uploader, battery, reservoir, clock), **food**,
and **activity** — the last being an addition whose own storage-purification path is discussed in §10.5.

`docs/data-schemas/` publishes formal schema documents for `entries`, `treatments` and `profiles`.

### 6.2 Three API generations in one process

The source tree contains API v1, v2 and v3 implementations side by side (`lib/api/`, `lib/api2/`,
`lib/api3/`). API3 reports itself on every response as `API3_VERSION "3.0.5"`
(`lib/api3/const.json`). Limits and behaviours are documented in code as constants:
`API3_MAX_LIMIT = 1000`, `API3_SECURITY_ENABLE = true`, `API3_DEDUP_FALLBACK_ENABLED = true`,
`API3_CREATED_AT_FALLBACK_ENABLED = true`. The batch writer accepts up to `MAX_BATCH_ITEMS = 10000`
(`lib/api/shared/write-payload.js`), with the comment that Loop backfills are documented to use batches of
up to 1,000 records and the limit is "ten times that capacity".

**The three generations do not share delete semantics** — API3 soft-deletes and keeps a versioned history,
while v1/v2 delete permanently (§5.9 of the inspection). Two answers to one question, in one process,
against one collection.

### 6.3 A published API description

`lib/api3/swagger.yaml` and `swagger.json` are shipped in the repository and describe API3 in OpenAPI
terms, including a `info.description` that states, among other things, an autopruning default. That
default is **not implemented** (§7.2 of the inspection). The documented interface is a real artifact and a
real claim; it is not a description of the running service.

---

## 7. Alarms and notifications

Alarms are the feature that overlaps most directly with **AnxietyWatch2 amendment 03 (multi-modal
alerting)** and with the supervised-alarm safety target, so this section is deliberately specific.

### 7.1 A documented six-level severity scale

`lib/constants.json` and `lib/levels.js` define:

| Level | Value | Display name | Raises an alarm? |
|---|---|---|---|
| URGENT | 2 | Urgent | yes |
| WARN | 1 | Warning | yes |
| INFO | 0 | Info | no |
| LOW | −1 | Low | no |
| LOWEST | −2 | Lowest | no |
| NONE | −3 | None | no |

`levels.isAlarm` is true for WARN and URGENT only. `levels.toStatusClass` maps **everything that is not
WARN or URGENT** to the CSS class `'current'` — so INFO, LOW, LOWEST and NONE render identically in the
UI. The scale is richer than the presentation, and the presentation collapses four of its six values into
one visual state (§5.7 of the inspection).

### 7.2 Local browser alarms, and what they do when there is no data

The plugin set under `lib/plugins/` implements per-signal alarms, each with a documented environment
variable pair (`*_ENABLE_ALERTS`, `*_WARN`, `*_URGENT`). The documentation's own table lists them: glucose
(`BG`), `AR2` predictive, `AR` predictive, `SGV`/`BG` staleness (`TIMEAGO`), `DB` database, `IAGE`
insulin age, `LOOP`, `OPENAPS`, `PUMP` clock/reservoir/battery, `UPBAT` uploader battery,
`CAGE` cannula age, `SAGE` sensor age, `BAGE` battery age.

Two code-level behaviours in this set are worth stating precisely, because they define what the alarms do
in the case that matters most:

- **The glucose alarm is skipped entirely when the reading is absent or old.**
  `lib/plugins/simplealarms.js:21` requires `scaledSGV && lastSGVEntry && lastSGVEntry.mgdl > 39 &&
  sbx.time - lastSGVEntry.mills < times.mins(10).msecs` before any alarm can fire.
- **The stale-data alarm is enabled by default** (`alarmTimeagoWarn: true`, 15 min; `alarmTimeagoUrgent:
  true`, 30 min — `lib/settings.js:27–30`), but the plugin returns early when there is no entry to be
  stale (`lib/plugins/timeago.js:26–28`), and renders the stale pill as neutral `current` when it has no
  reading (`:103–104`, comment `//assume current`).

The combination is the finding: the alarm that exists to say "the data has stopped" is on by default and
does not fire when the data has never arrived, and the pill that shows data age displays the same visual
state in that case as for a current reading.

### 7.3 Getting an alarm to a phone requires third-party services

The integrated path is a Pushover plugin (`PUSHOVER_API_TOKEN`, `PUSHOVER_USER_KEY`,
`PUSHOVER_ANNOUNCEMENT_KEY`). The documentation's recommended path is:

**Nightscout → IFTTT Webhooks (`ns-event`) → Google Drive/Sheets → IFTTT applet → Pushover → phone**

This requires an IFTTT account, a Pushover account and a Google account. The project documents the
trade-off it is working around:

> "The drawback for that method is that you could not necessarily fine-tune the alerts … and **Pushover had
> a demanding acknowledgment requirement. If you failed to acknowledge an alert, you may end up getting
> alarm fatigue fairly quickly as the alarm repeated itself.**"

The documentation also records the limits of the alternatives: IFTTT's SMS service is "limited to 100
message each month … some users may exceed that pretty easily"; IFTTT's own notification service "does not
have the ability to distinguish between devices the way Pushover allows"; and the sheets backing the
pipeline accept "up to 2000 active rows in a given spreadsheet. After that, the applet will automatically
create a new spreadsheet" — which the user must then re-point the second applet at.

**This is a genuinely load-bearing contrast with AnxietyWatch2's amendment 03.** Nightscout's remote
alerting is *assembled by the user out of four cloud services*, each with its own account, quota and
failure mode; the vendor has documented an alarm-fatigue failure mode caused by the acknowledgement
requirement; and the target platform (phone) is reached only through a third party. Whether
AnxietyWatch2's planned local multi-modal alerting does better is a matter for its own tests, not
something this document can assert.

### 7.4 Who can silence an alarm

The README states: "**Alarms are generated for high and low values, which can be cleared by any watcher of
the data.**" The code matches this: the alarm socket broadcasts to its whole namespace with no rooms, and
its native-token `ack` handler performs no permission check, so any valid access token can acknowledge —
that is, silence — any alarm globally (§5.17 of the inspection). For a system whose documented default
makes everyone a reader (§10), the practical audience able to silence an alarm is "anyone with the URL
who has any token".

### 7.5 Acknowledgement state does not survive a restart

Acknowledgement and silence state lives in a module-level in-memory map (`lib/notifications.js`), so a
process restart clears it. Silence is tested against wall-clock time
(`ctx.ddata.lastUpdated > alarm.lastAckTime + alarm.silenceTime`), and `lastUpdated` is refreshed on every
data load.

### 7.6 A plugin that throws logs and disappears

`lib/plugins/index.js` wraps each enabled plugin's `checkNotifications` in `try/catch` and logs
`console.error('Plugin error on checkNotifications(): ', plugin.name, error)`. The log names the plugin;
there is **no admin notice, no persisted state, no UI indication**. An alarm plugin that fails silently is
exactly the failure mode a supervised-alarm design exists to prevent, and it is worth noting that the
project's own admin-notification infrastructure (§5.20 of the inspection) is not used here.

### 7.7 Predictive low-glucose alarm

The default predictive alarm engine `lib/plugins/ar2.js` forecasts 30 minutes ahead with a second-order
autoregressive model (`AR = [-0.723, 1.716]`, reference BG 140) and raises WARN/URGENT from the average
weighted forecast loss against thresholds `0.05` and `0.10`. Every one of those numbers is a literal in
the file; the thresholds are **not user-configurable** — the user's `alarmHigh`/`bgTargetTop` settings
change the event's *name*, not its trigger point. The forecast is also clamped into `[36, 400]` before
display (`ar2.js:308`), so a forecast below 36 is shown as 36. Recorded in §5.16 of the inspection, where
the point is that this is the **inverse** of AnxietyWatch2's registry-as-single-source-of-truth rule: the
thresholds live in the code, and the user interface implies otherwise.

---

## 8. Reports and derived statistics

### 8.1 The report set

The documentation lists: **Day to day, Week to week, Daily Stats, Distribution, Hourly stats, Percentile
Chart, Weekly Distribution, Calibrations, Treatments, Profiles, Loopalyzer**. Filters include date range or
preset periods (Today, Last 2/3 days, Last week, Last 2/3 weeks, Last month, Last 3 months), key notes or
event type, which weekdays to include, target-range bottom and top, and sort order. The page warns that a
long period "might take some time" to render.

### 8.2 Derived statistics, and the disclaimers around them

The reports compute **A1c estimate** (DCCT and IFCC), **GMI**, **Revised GMI**, Time in Range, Time in
fluctuation and Time in rapid fluctuation, Mean Total Daily Change, Mean Hourly Change, and Out of Range
RMS. The documentation carries unusually explicit disclaimers:

> "A1c, GMI, and Revised GMI are estimates derived from available CGM readings. They are not laboratory
> measurements and must not replace clinical testing or medical advice."

and, for the A1c estimate, "only a rough estimate … does not replace laboratory testing", with the source
formula cited (Nathan et al., *Diabetes Care* 31.8 (2008): 1473–1478).

**This is a positive and it is recorded as one.** The project names the derivation, cites the published
formula, and labels the output as an estimate.

### 8.3 A shipped derived-statistic defect, disclosed by the vendor and explained by the code

The Daily Stats report documentation carries this note for **15.0.8**:

> "In Nightscout 15.0.8, the Daily stats estimated A1c is **wrong when glucose is displayed in mg/dL** (for
> example **95.8%** instead of 6.9%). This is fixed in the next release. The value shown in mmol/L is
> correct."

This is a fully reconcilable finding, and it is worth setting out the mechanism because it is the sharpest
available contrast with AnxietyWatch2's unit rule:

- `lib/report/reportclient.js:847` builds the report series with `sgv: client.utils.scaleMgdl(obj.y)` — the
  values are converted into the user's **display unit** before the report plugins see them. The report's
  target-range thresholds are scaled the same way (`:40–41`), so range classification is unit-consistent.
- `lib/report_plugins/dailystats.js:98–111` computes `average = sum / daysRecords.length` from those
  already-scaled values, then computes `(average * consts.MMOL_TO_MGDL + 46.7) / 28.7`.

The A1c formula is written for an input in **mmol/L**. In mg/dL display the average is therefore converted
a second time. Arithmetic check (derived here, not run against the application, and consistent with the
vendor's own published example): an average of **150 mg/dL** gives
`(150 × 18.01559 + 46.7) / 28.7 = 95.8%` — the exact figure the documentation publishes — while an average
of **8.4 mmol/L** gives `(8.4 × 18.01559 + 46.7) / 28.7 = 6.9%`, the value the documentation calls
correct.

Recorded as: a **derived clinical statistic whose value depends on the user's display-unit preference**,
shipped in the inspected release, disclosed in the project's user documentation with a workaround
statement, and explained by the source. Both the disclosure and the defect are findings. The parallel to
OSCAR's `Pressure_95th`/p90 mislabel and to SleepHQ's typo'd field is deliberate: this is the third
candidate in the package where a *derived* value's correctness or label did not match its computation.

### 8.4 Retention of derived data

Nothing found indicates that the reports are stored; they are computed on demand from the stored
collection. The retention question therefore belongs to §9.

---

## 9. Retention, export, deletion

### 9.1 Cleanup is an administrative surface, and it hard-deletes

`lib/admin_plugins/` provides the UI tools for housekeeping: `cleanentriesdb`, `cleantreatmentsdb`,
`cleanprofiledb`, `cleanstatusdb`, `daterangedelete`, `delete-count`, `futureitems`, plus `roles` and
`subjects`. The documentation's Admin Tools page lists them as "Delete records by date range", "Remove old
profile records", "Remove future items from Database", and per-collection database maintenance panels.
These paths **hard-delete** and are guarded by a confirmation string in the UI rather than by a tombstone.
`cleanentriesdb` removes entries older than **180 days** and its own action description carries a
copy-paste defect (a battery-status sentence used as the retention description).

### 9.2 A published autopruning default that the code does not implement

`lib/api3/swagger.yaml`'s `info.description` states: "Autopruning of obsolete documents (optional, default
is only `DEVICESTATUS`=60)". The code does not implement a default. `lib/api3/generic/collection.js:32`
reads `app.setENVTruthy('API3_AUTOPRUNE_' + colName.toUpperCase())` **with no default argument**, and
`setENVTruthy` returns the raw string, so with the variable unset the value is `undefined` and
`isNumberInString(undefined)` is false, which makes `autoPrune` return early. The project's own
`docs/audits/data-layer-audit.md` §5.4 documents the behaviour as opt-in with `ENTRIES_DAYS=0` and
`TREATMENTS_DAYS=0` meaning "no auto-prune" — using **different environment-variable names** from the ones
the code reads (`API3_AUTOPRUNE_*`).

So: swagger claims a permissive default, the code implements none, and the vendor's audit agrees with the
code while naming the variables wrongly. The **most conservative** reading — data is retained unless the
operator opts in — governs. The disagreement is recorded, not resolved.

### 9.3 What the pruning variable actually does when enabled

When it is enabled, `autoPrune` (`lib/api3/generic/collection.js:129–150`) runs at most hourly and calls
`deleteManyOr(filter)` where the filter is a three-way `$or` on `srvCreated`, `created_at` and `date`. Any
document for which **any one** of those three fields predates the cutoff is **hard-deleted** — including a
document that was created long ago but modified recently, and including soft-deleted tombstones.
`srvModified` is not among the prune criteria. See §5.12 of the inspection.

### 9.4 Backup is not built in

`docs/audits/data-layer-audit.md` §9 states there is "**No built-in backup mechanism**", "**No
point-in-time recovery**", and recommends adding a data export API, daily `mongodump` and oplog-based
restore. The hosting providers advertise backups (NS10BE: "every 6-8 hours"), which is part of what their
customers are paying for. For a self-hoster, the backup story is the database's, not the application's.

---

## 10. Privacy and security, as published

### 10.1 The default exposes the data, and the project says so loudly

The security documentation's own warning:

> "**In a default setup, anyone with your Nightscout URL can view your site.**"

with guidance to set `AUTH_DEFAULT_ROLES=denied`. The shipped default in `lib/settings.js:39` is
`authDefaultRoles: 'readable'`, and `lib/authorization/storage.js` defines the `readable` role as
`['*:*:read']` — read on every resource, for anonymous callers. The project's audit recommended defaulting
to `denied`; that recommendation was not applied.

There is a mitigation and it is a real one: `lib/server/bootevent.js:149–152` pushes a persistent
"Nightscout readable by world" notice into the admin notifications when the readable default is in effect.
A user who would see it is a user who has opened their own admin page. Recorded as a mitigating factor,
not as an equivalence.

### 10.2 A related permissive default, with a stated expiry plan

`ALLOW_UNRESTRICTED_FRAME_EMBEDDING` is documented as "**Its temporary default is true … The permissive
default is expected to change in a future release.**" The code agrees: `lib/server/env.js:85`
`readENVTruthy("ALLOW_UNRESTRICTED_FRAME_EMBEDDING", true)`. Recorded as a permissive default that the
project has flagged and dated, rather than discovered.

### 10.3 There is no privacy policy, and the reason is structural

No privacy policy, data-processing agreement or retention statement was found on any project property. The
security page's only statement on the subject is that a third-party operator inherits "a large body of
legislation across the world" and "extremely complicated" liability. That is consistent with the
architecture: the project is not a data controller because it does not hold anyone's data — the user (or
the hosting provider they pay) does. The practical consequence for a user is that **the privacy of a
Nightscout site is determined by whoever operates it**, and the project says nothing about what a hosting
provider may do with the data. Compared with the other candidates in this package — which all publish
privacy policies and can be read against them — the Nightscout privacy surface is empty by design.

### 10.4 A privacy assurance about a third party that the project cannot make

The documentation's FAQ section states: "Some people express concerns about using Facebook: if you're
worried about your privacy just remember **Facebook will only share the information that you allow it to
share**."

This is the project's advice to its users about a third party's data practices. It cannot be verified
here, no test or document supports it, and it conflicts with the widely documented reality that platform
data-sharing includes what users do not explicitly authorise (inference on my part; nothing was tested).
Recorded as an unverifiable third-party assurance in project documentation — the same class of finding as
a vendor claiming independent validation it did not have.

### 10.5 What the project does sanitise, verified in code

The security documentation states that incoming data is sanitised for "glucose entries, treatments,
profiles, device status, food, and activity data". Checked against the tree: this is **correct**.
`lib/server/{entries,treatments,profiles,devicestatus,food,activity}.js` each import
`./storage-purifier`, and the API3 path additionally uses `lib/api3/shared/writePurifier.js`. A hypothesis
that the documentation overstated this (because the purifier's file list appeared not to include
`activity`) was checked and **refuted** — `lib/server/activity.js` does import the purifier. Recorded as a
verified accurate claim, and as a refuted hypothesis (see the method note in `RESEARCH_METHOD.md`).

### 10.6 The token/JWT design, as audited, as specified and as implemented

This is the three-way disagreement that matters most, because it is the one where a reader of the vendor's
audit would be actively misled about credential lifetime and signing:

| Source | Claim |
|---|---|
| `docs/audits/security-audit.md` (Jan 2026) | tokens derived as `name.replace(' ','-').toLowerCase() + '-' + hash.substring(0,16)` from `API_SECRET`; JWTs "signed with `API_SECRET`"; default expiry **1 hour** |
| `docs/requirements/authorization-security-requirements.md` (Jan 2026) | (per REQ-AUTH-021/022/023) — describes the code correctly |
| Code at the pinned commit | token = `name.toLowerCase().replace(/[\W]/g,'').substring(0,10) + '-' + hash.substring(0,16)`, digest = SHA-1 of the access token (`lib/authorization/storage.js:187–193`); default JWT lifetime **`'8h'`** (`lib/server/enclave.js:59`); JWTs signed with a key read from `node_modules/.cache/_ns_cache/randomString`, **not** `API_SECRET` |

The consequences are not cosmetic: a **redeploy that wipes `node_modules` silently rotates the JWT key and
invalidates every issued JWT**, and the actual default session lifetime is eight hours rather than one.
The requirements document and the code agree; the audit does not. Recorded as documentation drift with the
code as the observed state, and with the audit's January 2026 date and the pinned commit's September 2026
date noted so that "divergence at the commit" is not misread as "wrong when written".

### 10.7 Brute-force protection

The audit describes the failed-authentication delay list as a strength and states it "Auto-clears entries
after 60 seconds of inactivity". The code uses a single `setTimeout(clearList, 30000)`
(`lib/authorization/delaylist.js:43`) where the comment says "Clear items older than a minute" and the
requirements document says 60 seconds **after the last delay**; `setInterval` appears nowhere in the file.
After the first 30 seconds the delay map is never pruned, so it grows without bound and a flagged address
stays delayed until the process restarts. Recorded as a second audit-vs-code divergence in the same
subsystem.

### 10.8 Data and credentials in logs

Two code paths log more than they should: a client-supplied `identifier` that does not match the computed
one causes the whole document — glucose value, device and date — to be logged with `console.log(doc)`
(`lib/api3/shared/operationTools.js:114–126`), and the alarm socket logs whole messages on failure
(`lib/api3/alarmSocket.js:74`, `:166`), which on the web path can include a session token. Recorded as a
static finding; no runtime observation.

---

## 11. Clinical, regulatory and safety claims

Nightscout's disclaimers are unusually explicit, and they are the strongest set of honest boundaries in
this package:

- "**not officially approved or regulated for diabetes therapy and/or treatment in any way**"
- "**Do not rely only on Nightscout as the only way you have of knowing your blood glucose values and
  trends**"
- "Make sure you're ready to cope with an unexpected failure and always have alternative ways to check your
  blood glucose levels"
- "This project requires a working internet connection and availability of third-party cloud services"
- the report-level disclaimers on A1c/GMI (§8.2)

The project also states that **"Nightscout itself does not directly interface with any devices"** — data
arrives from uploader apps, which is both an architectural fact and a liability boundary.

**Recorded as positives.** A project that tells users not to depend on it, that names itself
unregulated, that names the dependency on third-party cloud services, and that labels its derived clinical
statistics as estimates has given its users the information they need to calibrate their trust. This is
the correct handling of the "unknown is not OK" problem at the *documentation* level. Whether the software
upholds it at the *code* level is a separate question, and §5 of the inspection is where the two are
compared.

---

## 12. What Nightscout does not do

Scope, stated plainly and verified against the tree:

- **No CPAP, no sleep data, no AHI, no mask leak, no respiratory rate, no SpO₂.** The only oximetry
  adjacent surface found is a `rawbg`/`bgnow` "raw BG" path that is derived from CGM signal rather than from
  a pulse oximeter. There is no CPAP importer, no ResMed support and no SD-card or BLE device path of any
  kind.
- **No anxiety, panic, mood or medication-adherence feature.** Treatments are insulin and carbohydrate
  records. No EMA, no CBT content, no panic guidance.
- **No environmental or contextual input.** No weather, no aircraft noise, no air quality, no location.
  The nearest thing is a free-text `notes` field on treatments and food records.
- **No AI or machine-learning feature in the application.** The only model in the tree is the
  second-order autoregressive glucose forecast in `lib/plugins/ar2.js` — a fixed two-coefficient linear
  model, not a trained system, and its coefficients are literals. (The repository does contain a
  *proposal* document about AI agents in diabetes control; see §15.)
- **No first-party mobile app, and therefore no first-party push notifications.**
- **No multi-user model.** One site is one person's data. Roles exist, and a `careportal` role can be
  granted, but the data model is personal.

---

## 13. Evidence table

| ID | Claim or fact | Class | Source |
|---|---|---|---|
| N-1 | Project is an open-source cloud application for visualising, storing and sharing CGM data in real time; also known as "CGM in the Cloud" | public claim | documentation landing page |
| N-2 | "not supported by any company"; "DIY … supported by volunteers" | public claim | documentation landing page |
| N-3 | Values: "free to contribute, free to access, free to criticize, free to try, free to depend on, free to modify, and free to distribute" | public claim | documentation landing page |
| N-4 | "free - $0 with this option" | public claim | documentation landing page |
| N-5 | "requires a working internet connection and availability of third-party cloud services" | public claim | documentation landing page |
| N-6 | Three-role model: Site, Uploader, Follower | public claim | documentation landing page |
| N-7 | Ten commercial hosting providers advertised, with pricing links | public claim | documentation landing page |
| N-8 | NS10BE servers in DE/FI/FR; backup every 6–8 hours | public claim | documentation landing page |
| N-9 | "The system has not been designed for scenarios where a third party operates a Nightscout installation on behalf of others"; "liability issues … are extremely complicated" | public claim | security documentation |
| N-10 | "In a default setup, anyone with your Nightscout URL can view your site." | public claim | security documentation |
| N-11 | `ALLOW_UNRESTRICTED_FRAME_EMBEDDING` "temporary default is true … expected to change in a future release" | public claim | security documentation |
| N-12 | Sanitisation covers entries, treatments, profiles, devicestatus, food, activity | public claim, **verified in code** | security documentation + `lib/server/*.js` imports |
| N-13 | "Nightscout itself does not directly interface with any devices" | public claim | security documentation |
| N-14 | "not officially approved or regulated for diabetes therapy and/or treatment in any way" | public claim | documentation landing page |
| N-15 | "Do not rely only on Nightscout as the only way you have of knowing your blood glucose values and trends" | public claim | documentation landing page |
| N-16 | "Alarms … can be cleared by any watcher of the data." | public claim, **verified in code** | README + `lib/api3/alarmSocket.js` |
| N-17 | "blood glucose values are predicted 0.5 hours ahead using an autoregressive second order model" | public claim, **matches code** | README + `lib/plugins/ar2.js` |
| N-18 | Report set: Day to day, Week to week, Daily Stats, Distribution, Hourly stats, Percentile Chart, Weekly Distribution, Calibrations, Treatments, Profiles, Loopalyzer | public claim | reports documentation |
| N-19 | A1c/GMI are estimates, "not laboratory measurements and must not replace clinical testing or medical advice" | public claim | reports documentation |
| N-20 | A1c formula source cited (Nathan et al., *Diabetes Care* 31.8 (2008) 1473–1478) | public claim | reports documentation |
| N-21 | "In Nightscout 15.0.8, the Daily stats estimated A1c is wrong when glucose is displayed in mg/dL (for example 95.8% instead of 6.9%) … The value shown in mmol/L is correct." | public claim about the vendor's own released version; **mechanism verified in code** | reports documentation + `lib/report_plugins/dailystats.js:111`, `lib/report/reportclient.js:847` |
| N-22 | Remote notifications recommended via IFTTT + Google Drive/Sheets + Pushover | public claim | remote notifications documentation |
| N-23 | Pushover acknowledgement "may end up getting alarm fatigue fairly quickly as the alarm repeated itself" | public claim | remote notifications documentation |
| N-24 | IFTTT SMS limited to 100 messages/month; IFTTT notifications cannot target devices; sheets cap at 2000 rows | public claim | remote notifications documentation |
| N-25 | Admin Tools: delete by date range, remove old profiles, remove future items, per-collection maintenance | public claim, **matches code** | admin tools documentation + `lib/admin_plugins/` |
| N-26 | Autopruning "optional, default is only `DEVICESTATUS`=60" | public claim (OpenAPI `info.description`); **not implemented in code** | `lib/api3/swagger.yaml` |
| N-27 | Nightscout Foundation is a US 501(c)(3), EIN 47-2130306, donation-funded | public claim | foundation site |
| N-28 | Foundation has no affiliation with and receives no funding from Dexcom, Insulet, Tandem, Medtronic, BetaBionics, Sequel, Abbott, Senseonics "and any other commercially available products" | public claim | foundation site |
| N-29 | Nightscout Foundation began with John Costik's 2013 project; #WeAreNotWaiting | public claim | foundation site |
| N-30 | Foundation supports Loop, Trio and OpenAPS/AndroidAPS | public claim | foundation site |
| N-31 | "The quality, expertise, and response time of the Nightscout support teams exceeds that of most commercially available diabetes-related technologies" | public claim (self-assessment, unmeasured) | foundation site |
| N-32 | "These projects are available free-of-charge to individuals willing to build their own tools" | public claim | foundation site |
| N-33 | Facebook privacy advice: "Facebook will only share the information that you allow it to share" | public claim (third-party assurance, unverifiable) | documentation FAQ |
| N-34 | Repository: 2,836 stars, 73,382 forks, 290 watchers, 142 open issues, AGPL-3.0, created 2014-05-22, pushed 2026-10-04 | public claim (platform metadata) | GitHub API |
| N-35 | Latest release v15.0.8 "Chocolate Chip Muffin", 2026-09-04 | public claim | GitHub releases API |
| N-36 | CHANGELOG's newest entry is `[15.0.7] - 2026-03-XX (Unreleased)` | static-only finding | `CHANGELOG.md` |
| N-37 | 141 test files; CI workflows `main.yml`, `codeql-analysis.yml`, `close-accidental-sync-prs.yml` | static-only finding | repository tree |
| N-38 | Formal requirements and test specifications for data shape, API v1 compatibility and authorisation/security, marked Complete | vendor-published internal document | `docs/meta/DOCUMENTATION-PROGRESS.md`, `docs/requirements/`, `docs/test-specs/` |
| N-39 | "No built-in backup mechanism"; "No point-in-time recovery" | vendor-published internal document | `docs/audits/data-layer-audit.md` §9 |
| N-40 | Audit describes token derivation from `API_SECRET`, JWTs signed with `API_SECRET`, 1-hour expiry | vendor-published internal document | `docs/audits/security-audit.md` |
| N-41 | Audit: delay list "Auto-clears entries after 60 seconds of inactivity", listed as a strength | vendor-published internal document | `docs/audits/security-audit.md` |
| N-42 | Audit environment table lists `mongodb ^3.6.0` | vendor-published internal document (stale) | `docs/audits/security-audit.md` |
| N-43 | `authDefaultRoles: 'readable'` shipped default | static-only finding | `lib/settings.js:39`, `lib/authorization/storage.js` |
| N-44 | A persistent "readable by world" admin notice is raised at boot under that default | static-only finding | `lib/server/bootevent.js:149–152` |
| N-45 | Six severity levels with `LEVEL_NONE: -3`; only WARN/URGENT are alarms; all others render `current` | static-only finding | `lib/constants.json:19–24`, `lib/levels.js:17–48` |
| N-46 | Glucose alarms require a current reading and exclude `mgdl === 39` | static-only finding | `lib/plugins/simplealarms.js:21` |
| N-47 | Stale-data alarm is enabled by default (15/30 min) but returns early when no entry exists | static-only finding | `lib/settings.js:27–30`, `lib/plugins/timeago.js:26–28` |
| N-48 | `//assume current` for absent data; `// bless absence with known type` | static-only finding (vendor's own comments) | `lib/plugins/timeago.js:103–104`, `lib/api/entries/index.js:66` |
| N-49 | AR2 thresholds are file literals; forecast clamped to [36, 400]; internal reference constant 120 vs 140 | static-only finding | `lib/plugins/ar2.js:5–11`, `:101`, `:308`, `:54–55` |
| N-50 | Plugin exceptions are logged and dropped with no admin notice | static-only finding | `lib/plugins/index.js:196` |
| N-51 | Alarm socket has no rooms; token-path ack has no permission check; web path checks `notifications:*:ack` | static-only finding | `lib/api3/alarmSocket.js`, `lib/api3/doc/alarmsockets.md` |
| N-52 | Ack/silence state is in-memory; silence is wall-clock based | static-only finding | `lib/notifications.js` |
| N-53 | API3 soft-deletes and keeps history; API v1/v2 hard-delete | static-only finding | `lib/api3/generic/delete/operation.js`, `lib/server/entries.js` |
| N-54 | `autoPrune` is opt-in with no default; the three-way `$or` filter can delete recently-modified documents | static-only finding | `lib/api3/generic/collection.js:32,129–150`, `lib/api3/storage/mongoCollection/{modify,utils}.js` |
| N-55 | API3 validates structure only; no physiological range check on `sgv` | static-only finding | `lib/api3/generic/create/validate.js`, `lib/api3/shared/operationTools.js:56–75` |
| N-56 | Whole-document logging on identifier mismatch; whole-message logging on alarm-socket failure | static-only finding | `lib/api3/shared/operationTools.js:114–126`, `lib/api3/alarmSocket.js:74,166` |
| N-57 | JWT key comes from a generated `randomString` file, not `API_SECRET`; default lifetime 8 h | static-only finding | `lib/server/enclave.js:30,54,59` |
| N-58 | Delay list uses a single 30-second `setTimeout`; no `setInterval` | static-only finding | `lib/authorization/delaylist.js:43` |
| N-59 | Units: canonical mg/dL, scattered conversion, `mgdlToMMOL` returns a rounded string | static-only finding | `lib/units.js`, `lib/constants.json` |
| N-60 | Three separate admin-notice channels (bus event with one emitter, boot injection, boot-time notice) | static-only finding | `lib/adminnotifies.js`, `lib/server/env.js:115,121`, `lib/server/bootevent.js:149–152` |

---

## 14. Relationship to AnxietyWatch

### 14.1 Versus AnxietyWatch v1

**Little direct competitive overlap.** v1 is an iOS app for anxiety, panic and CPAP-data analysis; Nightscout
is a self-hosted web service for glucose. Neither does the other's job.

**One real point of contact: the alerting and "what happens when data stops" question.** v1's CNS risk
pipeline and Nightscout's alarm engine both have to decide what to do about a missing signal, and they are
worth comparing because they answer differently in verifiable ways (§§5.5–5.7 of the inspection).

**A second point of contact, worth naming: the operating model.** v1 is a locally installed app with an
optional self-hosted sync service (`SyncService`), HealthKit as the store, and no account. Nightscout is a
self-hosted service with no app and a database the user must run. Both are "you own it" designs reached
from opposite directions, and Nightscout is the field test of what that costs a non-technical user — which
is the audience that produced its paid hosting market.

### 14.2 Versus AnxietyWatch2

This is the comparison that matters, and it is architectural rather than clinical.

| Dimension | Nightscout | AnxietyWatch2 (current spec) |
|---|---|---|
| Deployment | self-hosted Node + MongoDB, or a paid host | local box (Pi) + Rust daemon, TimescaleDB |
| Transport into the system | phone uploaders over HTTP/HTTPS | BLE from the device (AirSense 11), SpO₂ over BLE |
| Data model | document store; shapes documented per collection | typed hypertable, `(source, type, session_id, seq)` PK |
| Units | mg/dL canonical, converted at multiple layers; display unit reaches derived statistics | one canonical unit per type, converted once at the decoder |
| Missing data | absence; rendered as neutral/current in two places (`//assume current`) | three states, "not measured" never rendered as measured |
| Invalid data | structure-checked only; no physiological range validation | reject with a named reason, quarantine, never clamp |
| Derived values | A1c/GMI/TIR computed in the client from display-scaled series | AHI deliberately not computed; registry-defined types only |
| Deletion | soft-delete in API3 + hard delete in v1/v2 and admin tools | append-only, tombstone only |
| Retention | operator's choice; audit documents an opt-in prune; raw retention unbounded by default | raw `samples` dropped at 2 years; 1-minute aggregate; 7-day compression |
| Alarms | default engine thresholds in code; broadcast to all subscribers; ack without permission check | registry-defined thresholds; supervised alarm target |
| Multi-modal alerting | assembled from IFTTT + Google + Pushover; vendor documents alarm fatigue | amendment 03 target, on local hardware |
| Access control | default grants anonymous read, disclosed; no third-party-operator design | private LAN, no account |
| Governance | volunteer project + 501(c)(3) foundation; no company support | owner-maintained |

**Where Nightscout is ahead of AnxietyWatch2 and it is worth saying so:**

1. **It has seventeen years of field deployment.** It has run in thousands of homes, on real networks, with
   real phones, through real battery and connectivity failures. AnxietyWatch2's operational record is not
   comparable, and no amount of specification substitutes for it.
2. **It has a published, versioned, machine-readable API description** (OpenAPI swagger for API3) and a
   documented shape per collection. That is a real integration surface, even with the drift noted in §9.2.
3. **It has a formal requirements-and-test-spec programme with traceability matrices** for three
   subsystems — something AnxietyWatch2's own OpenSpec process resembles, but with more published
   requirements documents behind it.
4. **It publishes explicit non-dependence disclaimers to its users.** AnxietyWatch2's documentation is
   careful about claims, but Nightscout states the safety boundary in one sentence and repeats it.
5. **It has a non-commercial foundation and a ten-provider hosting market.** Whatever else that market
   does, it means a non-technical user can run Nightscout today without owning a server. AnxietyWatch2 has
   no equivalent and does not appear to want one.

**Where AnxietyWatch2's design is ahead, and why the code evidence supports it:** unit handling, retention
policy, deletion semantics, absent-data semantics, invalid-data handling, and threshold configurability.
In each of those five areas the pinned Nightscout source contains a concrete instance of the problem the
AnxietyWatch2 rule exists to prevent, and in the A1c case it is a shipped, user-visible wrong number that
the vendor's own documentation acknowledges.

**The honest summary.** Nightscout is not a competitor for AnxietyWatch2's product. It is the best available
**reference implementation of the platform layer**, with seventeen years of operational scars of exactly
the kind AnxietyWatch2 is trying to design out at specification time. Its failures are more instructive
than its features, and its candour about those failures — the default-read disclosure, the A1c note, the
alarm-fatigue paragraph, the archive of its own audits — is the reason the comparison is possible at all.

---

## 15. Weaknesses, omissions and unresolved questions

1. **No runtime observation of any kind.** Nothing here establishes that any code path works.
2. **The security audit's inaccuracies may not be the only ones.** Three were caught because they were
   checkable against the code; the rest of the audit (MongoDB hardening guidance, deployment
   recommendations) was not verified line by line.
3. **`docs/proposals/` contains an RFC on an "Agentic Control Plane for Automated Insulin Delivery
   Systems"** (version 1.0, dated January 2026, status "**Draft (2026 Proposal)**", "Authors: Nightscout
   Community"). It proposes AI agents participating in insulin therapy management alongside a primary
   controller. **It is not a feature, not an accepted plan, and not a shipped capability**, and nothing in
   this document treats it as one. It is recorded only because it is a published artifact showing how part
   of this community is thinking about autonomous agents in a safety-critical loop, which is a useful
   contrast with AnxietyWatch2's explicitly conservative stance. The other documents in `docs/proposals/`
   (OIDC actor identity, conflict resolution, MongoDB modernisation, testing modernisation) were not read.
4. **No privacy policy exists, so no privacy claims can be verified or falsified.**
5. **The Foundation's claims (501(c)(3) status, EIN, absence of industry funding) were not independently
   verified** against IRS records or any register. They are recorded as the Foundation's statements.
6. **Every hosting provider's claims are third-party claims** (backup frequency, data-centre location,
   NEN 7510 certification) and none were checked.
7. **The 141 test files were counted, not run.** No claim is made about coverage or pass rate. The
   repository publishes a `docs/test-specs/flaky-tests.md` and a `coverage-gaps.md`; neither was read in
   detail.
8. **`lib/api3/` was read selectively.** The generic operation pipeline, validation, storage modifiers and
   the alarm socket were read; the read/search/fields-projection and dedup paths were not.
9. **The plugin set was read selectively.** `ar2`, `simplealarms`, `timeago`, `bgnow`, `webhook` were read;
   the other alarm transports (Pushover, Maker/IFTTT, speech, Alexa, Google Home) were identified but not
   read in detail — which means the *delivery* side of §7.3 rests on the documentation rather than on code.
10. **No CPAP-adjacent code was found, but the search was targeted** at identifiers (`cpap`, `resmed`,
    `ahi`, `spo2`, `oximet*`), not exhaustive.
11. **Whether any of the findings are known to the maintainers, tracked, or fixed in an unreleased branch
    is unknown.** Only the pinned commit was examined; the default branch has moved since.

---

## 16. Unverified scope

- Foundation legal status, EIN, and the absence of industry funding (recorded as claims, not verified).
- Hosting providers' security, backup and certification claims.
- The `docs/proposals/` RFCs' status in the project's actual roadmap.
- Test coverage, test pass rate, and the contents of the published coverage-gap and flaky-test documents.
- Alarm delivery through Pushover/IFTTT/Alexa/Google Home/speech (documentation only, no code read).
- Anything about how the software behaves at runtime.

---

## Related files

- `nightscout_IMPLEMENTATION_INSPECTION.md` — the pinned-commit source inspection (findings, line
  references, reproduction commands).
- `PROMPT_2.md` — the candidate-specific second-stage prompt, including the questions this research could
  not answer.
- `nightscout_COMPARISON.md` — the side-by-side comparison against AnxietyWatch v1 and AnxietyWatch2.
- `../CANDIDATE_SHORTLIST.md`, `../PROCESS_STATE.md`, `../RESEARCH_METHOD.md` — where this candidate sits in
  the process, and the method rules applied to it.
