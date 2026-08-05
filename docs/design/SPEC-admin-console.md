# Specification — AnxietyWatch2 admin console

**Status:** draft for review · **Date:** 2026-08-05
**Companion:** [`PROMPT-admin.md`](PROMPT-admin.md) is the *design* brief — how the console should look
and feel, and the three visual directions. **This document is the *functional* brief** — what it must
do, and why. Where the two disagree, this one governs behaviour and that one governs form.

**Method:** derived from a four-agent audit of the generation-1 Flask console
(`~/Source/AnxietyWatch/server`) — 26 routes, 17 templates, 40 tables — plus a live session against
the running deployment. Findings labelled **[observed]** were seen in the running system, not
inferred from code.

---

## 1. What this console is for

### 1.1 What the product is

AnxietyWatch tracks anxiety from both sides — the severity you report and the physiology your devices
record — so that a fifteen-minute psychiatrist appointment can be a data-informed conversation
instead of an exchange of impressions. The journal is the anchor; everything else is context around
it: medication doses with efficacy follow-up, HealthKit metrics, Polar H10 beat-to-beat HRV, Oura
scores, CPAP nights, pulse oximetry, labs, prescriptions, pharmacies, songs. A correlation engine
finds which metrics actually move with anxiety, and a report generator turns the result into
something you can hand a clinician.

CNS-depression early warning is **one feature among many** — an in-progress early-warning aid, not
the product. This console must not be designed as though it were.

**This is explicitly not a medical device**, not FDA-cleared, and not on a path to clearance. The
right bar is *an initial version that is genuinely useful to one person*, not one that would survive
a regulatory audit. Where this document sounds absolute, read it as "get this right eventually,"
not "block the first release on it."

### 1.2 The console's job

One user: the person tracked, who is also the engineer. Two jobs:

> **1. Is the record I am building actually complete and correct?**
> **2. Let me operate the machine that builds it.**

Job 2 is roughly half the surface — issuing credentials, running jobs, uploading files, editing
records, reviewing generated text. A console that does only job 1 is incomplete.

This is **not** the health-data viewer. That is the web dashboard, which answers questions about the
body. This one answers questions about the machine and the integrity of what it recorded. Keep them
visually related; never merge them.

### 1.3 Why integrity is the organising idea

The product's whole claim is *evidence instead of impressions*. That claim is only as good as the
data behind it, and every failure this console exists to surface degrades it the same way:

- A sync gap is a **hole in the report** you hand your psychiatrist.
- Frozen daily snapshots mean **correlations computed on stale inputs** — the engine still returns
  confident numbers, they are just wrong.
- A silently dead CPAP feed guts **half the sleep-apnea-anxiety thesis** the app is built on.
- A medication log with missing follow-ups quietly breaks the **efficacy curve** that makes tolerance
  visible.

None of these announce themselves. All of them look exactly like a working system that had a quiet
week. That is the problem this console solves.

---

## 2. The governing law

Generation 1 has one defect that appears in a dozen costumes, and it is the reason this document
exists.

### 2.1 It reports the last success and calls it status

Every health indicator in gen-1 answers *"when did the good thing last happen?"* None answers *"is
the good thing still happening?"* The operator is left to subtract dates in their head and decide
whether the gap is normal.

What that produced, in the live system **[observed]**:

| What the console showed | What was true |
|---|---|
| "Last Sync" with a date, unstyled, no emphasis | The phone had not synced in **5 days** |
| Nothing at all — the ResMed card renders only `{% if last_sync or last_status %}` | Both CPAP ingest paths were dark; never-synced and silently-stopped are the same absent card |
| A poller logging identical `{parsed: 9, upserted: 9}` every 10 minutes | It was re-upserting the same nine rows forever, then **stopped 9 days ago**. Running and useless, then dead and invisible, looked the same |
| `Last Used` on an API key, two days *newer* than the newest `sync_log` row | Requests were arriving and leaving no trace of their outcome |
| 6 of 6 keys `Active`, one labelled `…-revoke-after`, two never used since April | The page shows `Last Used` and never computes staleness, so an explicit note-to-self sat ignored for two months in plain sight |

The root cause is structural, not cosmetic: **`sync_log` is a success-only ledger.** The
`INSERT INTO sync_log` sits *inside* the `try` block; the `except` rolls back and returns 500. A
failed sync leaves no row at all. The dashboard's freshest-row query therefore cannot distinguish
"you stopped syncing" from "every sync since has failed."

### 2.2 The requirement

**Liveness is a computed property with an expectation attached, never a timestamp the operator
interprets.**

Every source, job, and channel declares an **expected cadence**. The console compares observed age
against that expectation and renders one of three states — never two:

| State | Meaning | Rendered as |
|---|---|---|
| `LIVE` | Observed within expectation | OK, **with the age** |
| `FAILING` | Observed, and bad | PROBLEM, with the reason |
| `UNKNOWN` | Never observed · stale past expectation · check itself did not run | **A QUESTION** — visually distinct from both |

A backup never verified is not a passing backup. A self-test that has not run in 40 days is not a
working alarm. A collector reporting no errors because it never started is not healthy.

**No status may be displayed without the age of the observation behind it.** "OK" is meaningless;
"OK · checked 4m ago" is a fact. A status whose underlying check is older than its expected interval
degrades to `UNKNOWN` **automatically**, without anyone writing a rule for that specific source.

### 2.3 Corollaries that are separately load-bearing

- **Failure must be recordable.** Attempt-scoped records, written *before* the work, updated with the
  outcome. A run that fails, times out, or is killed leaves a row saying so. This is a schema
  requirement, not a UI one — the console cannot show what was never written.
- **Success is not the same as effect — and effect is not row count.** A run that succeeded and
  changed nothing must be distinguishable from one that moved data. Row deltas alone are *not
  sufficient*: the dead poller in §2.1 reported `upserted: 9` on every run and would still read as
  productive under a delta rule. What never moved was the **high-water mark** — the newest data
  timestamp for that source. Every ingest surface shows the source's newest data timestamp and
  whether it advanced since the previous run; *"high-water mark unmoved across N successful runs"* is
  itself a degradable condition for any source expected to produce new data.
- **Silence is a reportable event.** The phone cannot notify about a sync it never woke up to
  attempt. **"Source X has been silent for N intervals" is a condition the Pi owns**, and today it
  exists nowhere in either generation. It surfaces on the Overview and the bedside display; it does
  not need to be a push notification, and specifically must not be delivered only to the device whose
  silence is being reported.

### 2.4 The law applies to the console itself

The console is a status source and is not exempt.

- **Ages are recomputed continuously client-side.** A page opened at 23:40 and read at 03:10 must not
  still claim "checked 4m ago." Rendered ages advance whether or not new data arrives.
- **The page carries its own feed state.** If the status feed goes stale past its own cadence, every
  status on screen degrades to `UNKNOWN` together, and the page says why. Gen-1's crude 5-second
  meta-refresh is rejected in §5; this is its replacement, and the replacement is mandatory.
- **Clock health is a subsystem.** Every age in this document is computed against the Pi's clock. NTP
  sync state and RTC battery presence appear on the Overview. This is not hypothetical: §6 already
  records two clock-skew hazards this project has hit.

### 2.5 Cadence — the boundary cases

The three-state rule needs these pinned down or it cannot be implemented uniformly.

| Case | Rule |
|---|---|
| **Cadence measures what?** | Attempts. "Last success age" is a second, separately rendered fact — a source attempting every 10 min and failing every time is `FAILING`, not `UNKNOWN` |
| **Stale *and* last-seen-failing** | `FAILING` wins. Promotion order is `FAILING` > `UNKNOWN` > `LIVE` |
| **Event-driven sources** (manual upload, SD card) | Declared **on-demand**: state is the outcome and age of the last event, never time-degraded. Labelled as on-demand so an old timestamp does not read as a fault |
| **In-flight work** | Every attempt record carries an expected duration. An attempt still running past it degrades to `UNKNOWN` — otherwise a killed process renders as "running" forever, which is precisely the gen-1 defect in §4.6 |
| **Legitimately variable cadence** | Declare the *upper bound*. If no honest bound exists, the source is on-demand |

**Cadence is visible, auditable config** (§4.8), and **the expectation renders next to the age** —
"4m ago · expect ≤ 10m". Age alone lets a loosened cadence silently recreate gen-1: a source declared
at 7 days shows 5 silent days as `LIVE`, which is the original incident laundered through config.

### 2.6 The failure mode this law creates, and its bound

Three states with no severity dimension trade gen-1's false calm for **habituation**. If a benign
`UNKNOWN` — an unworn ring, a backup that cannot be verified until the storage lands — permanently
occupies the promoted slot, the operator learns to skim the exact state everything else depends on.
That converges on gen-1's outcome by the opposite route.

Two mechanisms bound it:

- **Severity is a second axis, not a fourth state.** Each subsystem declares whether a fault is
  *consequential* (data being lost, the record going wrong, the alarm subsystem unarmed) or
  *routine*. Promotion order sorts on severity first, then state.
- **Expected silence can be acknowledged, time-boxed.** "Travelling until the 14th; the phone will
  not sync" renders as **muted until a date, by the operator** — visibly distinct from `LIVE`, never
  collapsing into it, and expiring on its own.

---

## 3. Information architecture

Gen-1's 11-item flat nav is an accretion log: a content-editing form ("fix these song lyrics") sits
at the same weight as whether data is arriving at all. **Nine** sections, ordered by how often a
healthy operator needs the answer:

| # | Section | Answers | Milestone |
|---|---|---|---|
| 1 | **Overview** | Is the record complete, and is the machine working? Five seconds. | **M1** |
| 2 | **Display path** | Is the bedside screen actually lit, not merely "service active"? | **M1** |
| 3 | **Ingest** | Is data arriving, from which sources, and did anything change? | **M1** |
| 4 | **Services** | Are the daemons up, what did they log, let me restart one. | M2 |
| 5 | **Config & access** | Credentials, registry, cadences, flags, audit, outbound email, telemetry. | M2 |
| 6 | **Data** | What is actually stored — targeted views, then export. | M3 |
| 7 | **Analysis** | Run correlation and AI jobs, watch them, review output, see cost. | M4 |
| 8 | **Records** | Journal, medications, prescriptions, pharmacies, profiles, schedule. | M5 |
| 9 | **Recovery** | Backups, restore verification, what would I lose right now. | M5 |
| — | **Alarm path** | Is the CNS early-warning subsystem armed and testable. | **M6, last** |

### 3.1 Milestones

**Delivery order is a function of §7b, not of how interesting a section is.** An earlier draft ranked
purely by operator frequency and put Data in the first release — which is blocked on storage
hardware, so that schedule stalled before any loop closed.

| | Contains | Gated on |
|---|---|---|
| **M0** — foundations, no UI | The observation/attempt schema · per-source cadence in the registry · the liveness evaluator as one pure function · an HTTP service · the bind and auth decision | Nothing. None of it exists; everything below needs all of it |
| **M1** — first usable console, read-only | Overview · Display path · Ingest for the registry sources · **attempt history** · registry left-join so a never-seen source appears · console self-freshness · clock state · build and schema identity · host readouts (disk, thermal, undervoltage, network, watchdog) | M0 |
| **M2** — operations | Services with logs and audited restart · outbound email with its test loop · S1–S5 | M1 |
| **M3** — data | Targeted integrity views, then scoped export | The sample store → storage tasks → the M.2 HAT |
| **M4** — analysis | Job lifecycle, cost ledger, AI review loop | M3 + a job runner |
| **M5** — records and recovery | Domain CRUD · backups and restore verification | Schema that does not exist yet |
| **M6** — alarm path | Channel availability, self-test, arming | Daemons that do not exist yet |

**Attempt history is in M1, not M3.** It is the single thing that distinguishes *silent* from
*failing repeatedly*, which is this document's entire claim (§2.1). Putting it behind a generic data
browser would be paying more for less: the browser is expensive, security-sensitive, and awkward to
paginate across arbitrary schemas, while attempt history is one narrow table that answers the
highest-frequency question directly.

**The alarm path is built last, for two independent reasons that agree.** It is one in-progress
feature among twenty and not what the product is for — and it is also the *most* blocked section
here: the daemons it would observe do not exist as code, and the daemon fields that would report its
state have no request verb that can set them (§7b). Nothing else waits on it.

**Songs/lyrics** is content CRUD on an experimental correlate. It belongs inside **Records**.

---

## 4. Section requirements

### 4.1 Overview

- Every subsystem, current state, freshness. Readable in five seconds, from bed, on a phone.
- **The largest thing on screen is the most important number.** If everything is `LIVE`, the screen
  should be quiet — an all-neutral resting state where colour appears only on exception is preferred.
- Any subsystem in `UNKNOWN` or `FAILING` is promoted above the quiet ones. The operator never
  scrolls to find a problem.
- Deployed build identity (commit, build time) and schema revision, with pending-migration count.
  Gen-1 shows neither; the schema revision is obtainable today only by dispatching a CI job manually.

### 4.2 The display path is a supervised subsystem

Not in `PROMPT-admin.md`'s subsystem list, and it must be. **A bedside monitor whose screen is dark
is indistinguishable from a healthy one unless the console says otherwise** — and the panel has
already produced five distinct failure modes in deployment, including a systemd unit reporting
`active` while the screen showed nothing.

Required: compositor running, the correct DRM connector attached, a Wayland socket that actually
appeared, the browser process alive, and **the age of the last frame the panel rendered** — not
merely that the service is up. Service-up and screen-lit are different claims and gen-2 has already
learned the difference the hard way.

### 4.3 Services

The machine's own daemons. `PROMPT-admin.md` mocks this screen up; without it here, restarting a
daemon is formally out of scope while the design shows a button for it.

**Enumerate the supervised units** — the sample-store daemon, each radio collector, the detection
engine, the alarm daemon, the replication tailer, the display path (§4.2), and the scheduled jobs
(§4.3). Per unit: state (§2.2), uptime, restart count, memory, last error, and last log lines.

- **Restart and stop are operations with audit entries** (§4.8) and confirmation proportionate to
  consequence (§5). Restarting the lyrics fetcher and restarting the alarm daemon overnight are not
  the same act and must not feel the same.
- **Log access applies S7 sanitisation.** Filterable by unit and severity.
- **Restart count is a first-class signal.** A unit that is `LIVE` because it restarted forty times
  today is not healthy; a crash loop reads as `FAILING`, not `LIVE`.
- This is *not* a general remote shell (§8) — it surfaces state and triggers named operations.

### 4.4 Ingest

One card per source — phone, ResMed cloud, ez Share SD, EDF upload, Oura, AS11 bridge, and each BLE
radio — every card carrying:

- State (`LIVE`/`FAILING`/`UNKNOWN`) computed against that source's declared cadence
- Last attempt **and** last success, as distinct facts
- **Row delta from the last run**, not just status
- The failure reason when failing, sanitised (see §8)
- What this source **uniquely owns** — i.e. what is unrecoverable if it stays dark

**Precedence must be visible.** Four sources write CPAP session rows under two asymmetric,
independently maintained skip-lists. Which source owns a given night is currently unanswerable
without SQL. Show it.

**Derived-state that silently corrupts must be inspectable.** The CPAP clock-offset correction guesses
an offset for a device epoch bug and writes rows under the corrected date — which is the primary key.
A wrong offset files clinical data under wrong dates with no way to see, confirm, or reset it. Expose
the offset, when it was derived, and a way to correct it.

### 4.5 Data

- **All tables, not a subset.** Gen-1 browses 14 of 40; 20 have no surface anywhere, including the
  largest table in the system and the app-only oximetry rows that nothing else can reconstruct.
- **Real pagination.** Gen-1 has `LIMIT` with no `OFFSET` and a 500-row ceiling, so **row 501 of any
  table is unreachable through the UI, ever**. Tables that cannot be paged safely must be aggregated
  instead — never listed.
- **Never conflate truncation with completeness.** "Showing 25 of up to 25 rows" is ambiguous by
  construction. Show the true total, or state explicitly that it is a window.
- Date-range and source filters, sortable columns, and export.
- **NULL must not render as the string `None`.** A never-measured value, a genuine zero, and a parse
  failure must be visually distinct. This is the same law as §2.2 at cell granularity.
- No information may live only in a `title=` tooltip — unreachable by keyboard and touch.
- **A per-day cross-table view.** "Was 2026-07-14 a bad night?" currently means three tabs, three
  sort orders, no shared date cursor. The analysis pipeline already assembles exactly this per-day
  join and throws it away into a prompt.

### 4.6 Recovery

- Last full, last incremental, WAL archive lag.
- **The last verified restore** — a real restore into a scratch cluster with row-count comparison,
  which either passed or failed. Never verified is `UNKNOWN`.
- A prominent, plainly-worded answer to **"what would I lose if this died right now?"**
- The data-loss ledger must have a reader. Gen-1 records dropped row counts with reasons and nothing
  anywhere queries it.

### 4.7 Analysis

- Queued / running / succeeded / failed as genuinely distinct states. **Gen-1 never sets `running` on
  the real code path**, so a live multi-job run reads `pending` for its entire lifetime.
- Elapsed time, cancel, and a run history with each run's parameters and outcome.
- **Cost estimated *before* submission, not after.** Today the price is knowable only by opening the
  detail page once the money is spent. A running total and a budget ceiling are required.
- **Every** billed call appears in one ledger. Gen-1 has four generation endpoints with no cost
  tracking at all, using a hardcoded model that differs from the one the dropdown offers.
- A parse failure that produces synthesised placeholder content must be **recorded as a parse
  failure**, not marked completed.
- Egress is explicit: any action that ships personal health data to a third party states what is
  being sent and how much of it, and requires deliberate confirmation proportionate to that.

### 4.8 Records

Patient and psychiatrist profiles, therapy schedule, conflicts. This is prompt-input curation — it
feeds the analysis context block — and it is the strongest part of gen-1's existing design.

- **Preview the assembled prompt.** Five pages currently contribute to one prompt and no page shows
  the result before it is sent.
- Editing is unversioned in gen-1; the prior text is gone. Pattern C (§5) applies here.
- Mark **which** record will feed the next run when the selection rule is "most recent active" —
  ambiguous the moment there are two.

### 4.9 Config & access

- Credentials: presence and last-4 only, never values. Expiry and rotation as first-class concepts —
  gen-1 keys live forever.
- **The whole settings table is operator-relevant.** ~12 keys are reachable today only via psql,
  including poller status, the CPAP clock offset, and the login-throttle state.
- Validate against reality, not shape. An unvalidated timezone string drives day-bucketing for the
  entire analysis pipeline; a typo is accepted, displayed back as correct, and silently falls back.
- **An audit log, which does not currently exist.** Key revocation, profile edits, destructive
  actions, and job runs, each with actor and timestamp.

---

### 4.9b Outbound email — the independent notification channel

Most of what this console detects is **not urgent but is easy to miss**: a feed that stopped, a
source silent past its cadence, a backup that has not verified, a correlation run on stale inputs.
Those need a channel that survives the phone being dark, flat, or left at the office — which is
precisely the case a push notification cannot cover.

Email through the operator's own Gmail account is the right fit: independent infrastructure, no
Apple dependency, no account to create, arrives on every device already signed in, and it **persists**
rather than vanishing from a lock screen.

**Setup, in the console:**

- From address and destination address (they may differ; sending to yourself is the default).
- **App Password, not the account password** — Gmail requires 2FA enabled and issues a 16-character
  app-specific credential for SMTP. Stored encrypted at rest, write-only in the UI, **never echoed
  back**, displayed thereafter as presence-plus-last-4 only.
- SMTP is `smtp.gmail.com:587` with STARTTLS. Fixed, not a form field — an editable host is a
  misconfiguration surface with no upside for a single-user appliance.
- **A "send test email" button whose result is recorded**, with the outcome and its age shown
  afterwards. Configured-but-never-tested reads `UNKNOWN`, not `LIVE` (§2.2).

**Behavioural requirements:**

- **Not on the alarm path.** SMTP latency and deliverability are unbounded; email must never be a
  channel the CNS klaxon depends on. It carries integrity and operational notices, and *at most* a
  redundant copy of an alarm that has already fired locally.
- **Rate-limited and coalesced.** A source that has been silent for nine days must produce one
  message, not one per check. Digest by default; immediate only for state changes that matter.
- **The failure is visible.** A send that fails leaves an attempt record (§2.3) and degrades the
  channel's state. A notification channel that silently stops is worse than none, because it is
  trusted.
- **Credentials are revocable from Google's side without touching the Pi**, which is a real advantage
  over most auth: if the app password leaks, revoking it at the account is sufficient and the console
  will show the channel failing on its next send.

**Why an app password rather than OAuth2.** OAuth is the more correct answer for a distributed
product and the wrong answer here: it needs a Google Cloud project, a consent screen, a refresh-token
lifecycle, and re-consent handling — significant machinery whose entire benefit is not storing a
credential the operator can revoke in one click anyway. Revisit only if Google withdraws app
passwords.

### 4.9c Optional telemetry — New Relic

**Off by default, opt-in, operational metrics only.** This is observability for the appliance, not
analytics about the person. The distinction is load-bearing and is stated here so it cannot erode:
the project's public promise is that health data never leaves the box, and enabling telemetry must
not weaken it.

**Instrument with OpenTelemetry; export via OTLP to New Relic.** The vendor is a configuration line,
not a dependency compiled into the daemons. That keeps New Relic swappable and keeps vendor SDKs off
the ingest path — the same reasoning that keeps Docker off the life-safety path.

**What may be exported** — shape, not content:

- Daemon uptime, restart counts, memory, per-unit error counts
- Sample **rate** per source, spool depth, drain latency, rejected/quarantined batch counts
- Attempt outcomes and durations by job type
- HTTP latency and error rates for the console itself
- Host: disk free, thermal, undervoltage, clock-sync state

**What may never be exported**, at any setting:

- Sample **values** — no heart rate, no SpO₂, no blood pressure
- Journal text, tags, severity ratings, song titles
- Medication names, prescriptions, pharmacy or provider details
- Anything identifying the person or the device beyond an opaque instance id

The safe rule: **counters and timings leave; measurements and words do not.** A metric whose
cardinality is per-sample or per-entry is prohibited outright — it would both breach the boundary and
exhaust the free tier.

**In the console:**

- Enable/disable, plainly labelled as off by default, with a one-line statement of what leaves.
- Licence key, stored encrypted at rest, write-only, shown thereafter as presence-plus-last-4 (the
  same handling as the Gmail app password, §4.9b).
- Region endpoint (US/EU) — New Relic's OTLP endpoints differ and a wrong region fails silently.
- **Test connection**, with the outcome and its age recorded. Configured-but-never-tested is
  `UNKNOWN`.
- **Export health is itself a status** (§2.2). A telemetry exporter that has been failing for a week
  is exactly the "reports the last success" defect this document exists to prevent — and it is worse
  here, because the operator now believes something is watching.

**Never on the critical path.** Telemetry export failing must not slow, block, or crash ingest, the
detector, or the alarm. Bounded queue, drop on backpressure, count the drops, and surface the count.

**Open-source consequence.** If this repository is published, telemetry ships disabled with no key
and no endpoint. A contributor cloning it must not be able to enable it by accident, and the README's
no-telemetry statement stays accurate as written — because for anyone who has not opted in, it is.

### 4.10 Alarm path — the CNS early-warning subsystem (built last)

Two independent channels. Both must be separately observable, because the entire point is redundancy.

**Local klaxon:** last self-test result and age, audio device present, volume, amplifier responding,
GPIO channel continuity. A self-test older than its interval is `UNKNOWN`, not `LIVE`.

**Remote push (the second channel):** push-token registry with count and per-token age, the delivered-
alert ledger, the backstop evaluator's state, and the heartbeat dead-man's-switch — **including
whether the external timer that drives it has fired recently.** Gen-1 exposes all of this only behind
an API-key JSON endpoint, so the operator cannot see in a browser that the channel is dark. The
credentials it needs are not declared in either compose file, so **the channel is dark by default and
nothing says so.**

**Required readouts:** is push configured · how many devices registered and how stale · when did the
sweep last run against its expected interval · has an alert ever been delivered, and when · is the
Critical Alerts entitlement active or pending.

**Required controls:** fire a test alert down each channel independently, with the result recorded in
the same ledger as a real one and clearly marked as a test.

**Invariant:** a channel that has never delivered anything is `UNKNOWN`, never `LIVE`. Zero delivered
alerts is the expected steady state for a working alarm *and* the signature of a completely broken
one; only a successful test discriminates them.

## 5. Cross-cutting requirements

**Freshness on everything.** §2.2. No exceptions, including inside tables.

**Proportionate confirmation.** Gen-1 has exactly one confirmation dialog in the entire console.
Revoking a key, resolving a conflict, deleting a schedule entry, and overwriting manually-entered
content are all one unguarded click. Destructive controls are visually separated from safe ones, and
confirmation weight scales with consequence — revoking a credential and deleting a row should not
feel identical.

**The four interaction patterns** in `PROMPT-admin.md` §Operations (credential lifecycle, long-running
jobs, AI-content review, file upload with validation) are requirements, not illustrations. Each must
be designed in all its states.

**File upload has no history in gen-1** — outcomes arrive as transient flashes and vanish on the next
navigation. Uploads are attempts and get attempt records like everything else.

**Accessibility is a hard requirement**, specified in `PROMPT-admin.md` §Accessibility. Gen-1 fails it
broadly: ~30 form controls with unassociated or absent labels, status colours failing AA contrast
worst exactly on the high-severity signals, a symbol-only column with no text alternative, and a
5-second `<meta refresh>` that resets focus and scroll with no pause control.

---

## 6. Migration risk register

Capabilities that exist **today** and have no home in the gen-2 plan. Each needs an owner before
generation 1 is decommissioned — and the checklist is worth writing **now, while gen-1 is still
running and inspectable**, not after the box is off.

| Capability | Risk if dropped |
|---|---|
| **Redundant push alert channel** | A safety regression, not a feature cut. §4.2 |
| **Clinical PDF report generation** | A documented pre-appointment workflow with no successor |
| **JSON/CSV export** | Browse-and-search is not the same as emit-a-portable-file |
| **Correlation recompute** | Never had a button; easy to lose silently |
| **Timezone setting** | Drives day-bucketing across analysis and correlations |
| **Schema revision + pending migrations** | Today: a manually-dispatched CI job |
| **Deployed build identity** | Gen-1 shows none; gen-2 should not inherit that |
| **Log sanitisation** | Token redaction exists in gen-1. Fresh logging code starts leaking credentials into the journal |
| **"Phone has gone dark"** | Belongs on the Pi, exists nowhere today. §2.3 |
| **Phone-side health-access state** | Irreducible to the Pi, but its *state* must surface here. A phone that silently loses health-data access is invisible from the Pi today — and that exact freeze ran undetected for three months |
| **Device↔server reconciliation** | Distinct from backup/restore. The restore path carries hard-won invariants (empty-store guard that names its blocker, preserve server-side row identity, mark restored rows as already-synced, page anything large). Recovery covers cluster restore, not this |

**Undocumented operational knowledge** encoded only in gen-1 code, which must be carried across
deliberately: the replay window keyed on server receipt rather than device timestamp, so a skewed
device clock cannot mask a live desaturation; explicit non-finite rejection, so a `NaN` cannot sail
past a floor comparison; nil-AHI nights excluded rather than averaged as zero; baseline windows
computed unfiltered so they are not mislabelled.

**Fix during migration, not after:** third-party OAuth tokens cannot be revoked server-side (the phone
UI admits its disconnect is device-only); encryption-key rotation is unimplemented and would orphan
every encrypted setting; the ad-hoc SQL access path's read-only guarantee is prompt text rather than a
Postgres role.

---

## 7. Security requirements

Carried from the audit as requirements, not observations.

| # | Requirement |
|---|---|
| S1 | **CSRF protection on every state-changing request.** Gen-1 has none anywhere; a cookie attribute is the sole mitigation |
| S2 | **Login throttling is per-source, not global.** Gen-1's counter is global — any unauthenticated caller can lock the sole operator out indefinitely |
| S3 | **Session lifetime is bounded.** Gen-1's admin session is valid until the browser drops the cookie |
| S4 | **Transport security state is visible.** If the session cookie is crossing the LAN in cleartext, the console says so rather than leaving it to be inferred from env |
| S5 | **Request body size is capped before anything touches disk.** Gen-1 writes each uploaded file in full *before* checking its size, with no framework-level cap |
| S6 | **A real read-only Postgres role** with a statement timeout backs any SQL surface — retiring the SSH-to-prod pattern rather than reproducing it |
| S7 | **All operator-visible error text is sanitised.** Gen-1 renders raw subprocess stderr, raw SDK exception text, and raw error strings into the page; a sanitiser exists and is used on some paths but not these |
| S8 | **Outbound egress is opt-in and stated.** Any action that sends data off-box names the destination and the payload before it runs |

---

## 7b. What actually exists, and what this is blocked on

**Read this before estimating anything.** Most of what this document specifies has no implementation
to attach to. That is normal for a spec, but the gap needs stating so nobody plans against a system
that isn't there.

**Exists today:** one daemon with a Unix-socket protocol exposing `Append` / `Query` / `Subscribe` /
`Health`; a registry of five sources; four database tables. That is the whole substrate.

**Does not exist as code anywhere in the workspace:** the alarm daemon, the detection engine, the
replication tailer, any batch importer, any job runner, any LLM client, any web backend to serve this
console — and schema for jobs, attempts, audit, credentials, backups, analysis history, records, or
push tokens.

### Three concrete contract mismatches

These are not future work; they are places where this spec, as written, cannot be built against the
daemon that exists.

| Spec says | Contract says | Consequence |
|---|---|---|
| Three states `LIVE` / `FAILING` / `UNKNOWN` | `Liveness` is `Live` / `Stale` / `NeverSeen` | **There is no `FAILING` on the wire.** `Stale` reads like "bad" but means only "past cadence" and belongs in `UNKNOWN`. Required mapping: `Live → LIVE`, `Stale → UNKNOWN`, `NeverSeen → UNKNOWN`. A source actively producing *bad* data has no representation at all and needs new plumbing |
| A never-seen source shows `UNKNOWN` | A source that never reported is **absent from the map**, not present as `NeverSeen` | The console must left-join the registry's known sources against reported ones to synthesise "never seen." The daemon has no census to diff against — this is console-side work, and §9.4 is otherwise unsatisfiable for a collector that never started |
| Each source declares its cadence | One global constant for the whole daemon; the registry has no cadence field | **§9.3 is untestable today.** Per-source cadence in the registry is a named prerequisite: schema change plus codegen |

Additionally, the heartbeat inputs (`monitoring_window_active`, tier evaluation, drain progress) are
public fields with **no request verb that can set them**. Today `monitoring_window_active` can never
become true, so the heartbeat reads healthy unconditionally — which is precisely the defect §2.1
indicts, sitting inside the one contract this console consumes. Fixing it means adding verbs.

### Honest sequencing

**Buildable now, against a live contract:** a subset of §4.1 Overview and §4.3 Ingest — for the five
registry sources only — driven by the daemon's `Health` response, plus the registry left-join above.

**Blocked on the sample store** (→ storage tasks → the M.2 HAT): §4.4 Data, §4.6 Analysis.

**Blocked on schema that does not exist yet:** §4.5 Recovery, §4.7 Records, §4.8 Config & access, and
every attempt/audit requirement in §2.3.

**Blocked on daemons that do not exist yet, and built last by choice:** §4.2 Alarm path.

An earlier draft of this section claimed the alarm path could be built now. That was wrong, and
wrong in the most expensive direction — it is the most blocked section in the document.

## 8. Non-goals

- Reviewing health data — that is the web dashboard.
- Multi-user, roles, or permissions. One operator.
- Editing physiological samples. The console inspects and exports them; corrections happen at
  ingest.
- Replacing systemd, Postgres, or the shell. This surfaces state and triggers work; it is not a
  general-purpose remote console.

---

## 9. Acceptance criteria

The build is not done until:

1. Every status in the console is one of three states, and the third is visually distinct from both
   others on **every** panel.
2. No status renders without the age of its observation.
3. A source stale past its declared cadence degrades to `UNKNOWN` **without a per-source rule**.
4. Killing any collector produces a visible `UNKNOWN` within its cadence, and the console names which
   one.
5. A failed job leaves a durable record with a reason, and appears in history.
6. Both alarm channels can be tested independently, and a never-tested channel reads `UNKNOWN`.
7. Every table is reachable — paged if pageable, aggregated if not — with no unreachable rows.
8. NULL, zero, and parse-failure are visually distinct in every data cell.
9. Cost is shown before a billed action, and a running total exists.
10. Every destructive action has confirmation proportionate to its consequence, and lands in the audit
    log.
11. The accessibility bar in `PROMPT-admin.md` is met with **computed** ratios, not asserted ones.

---

## 10. Open questions

1. **Where does the phone-silence alarm fire?** It is the Pi's job (§2.3), but the Pi alerting about
   the phone that alerts about the Pi needs a defined escalation path that does not loop.
2. **Retention.** Several high-volume tables have no pruning job in either generation. Does the
   console show retention state, trigger compaction, or both?
3. **How much of Records migrates at all?** It is the least machine-ish part of the console and the
   most personal. It may belong with the health viewer rather than here.
4. **Does the SQL pane ship in v1?** S6 makes it safe, but the same discipline argues that a console
   answering the real questions removes the need for it.
