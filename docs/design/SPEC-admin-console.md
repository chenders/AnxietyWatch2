# Specification — AnxietyWatch2 admin console

**Status:** draft for review · **Date:** 2026-08-05
**Companion:** [`PROMPT-admin.md`](PROMPT-admin.md) is the *design* brief — how the console should look
and feel, and the three visual directions. **This document is the *functional* brief** — what it must
do, and why. Where the two disagree, this one governs behaviour and that one governs form.

**Method:** derived from a four-agent audit of the generation-1 Flask console
(`~/Source/AnxietyWatch/server`) — 26 routes, 17 templates, 40 tables — plus a live session against
the running deployment. Findings labelled **[observed]** were seen in the running system, not
inferred from code.

**This document is expected to change as it is built.** It was written before most of what it
describes exists. Amend it when implementation contradicts it — see [§9b](#9b-how-this-document-changes).

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

Beneath the table sit several distinct defect classes — success reported without effect, no declared
expectation, absent-state UI — and §2.2–§2.3 address each in turn. The deepest is structural, not
cosmetic: **`sync_log` is a success-only ledger.** The `INSERT INTO sync_log` sits *inside* the
`try` block; the `except` rolls back and returns 500. A failed sync leaves no row at all. The
dashboard's freshest-row query therefore cannot distinguish "you stopped syncing" from "every sync
since has failed."

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

A backup never verified is not a passing backup. A channel untested for 40 days is not a working
channel. A collector reporting no errors because it never started is not healthy.

**No status may be displayed without the age of the observation behind it.** "OK" is meaningless;
"OK · checked 4m ago" is a fact. A status whose underlying check is older than its expected interval
degrades to `UNKNOWN` **automatically**, without anyone writing a rule for that specific source.

This law has independent precedent — and a cautionary counter-example — in Home Assistant, whose
availability model gets the distinction right per-integration but has no automatic staleness in the
core, with the predictable result. See §8b.

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

### 2.3b Writes are idempotent on content

**Every write endpoint must be idempotent: re-sending data the server already holds is a no-op that
returns the same result as the first send.** Not an error, not a duplicate, not a 409.

This is a rule about the ingest API rather than the console, but the console is its main witness, and
it removes a whole class of the defects §2.1 catalogues.

**What it buys.** A client that does not know whether its POST landed can simply send again. That
single property:

- Makes a **durable client outbox trivial** — retry until acknowledged, no delivery-state tracking.
- Removes the need for the ack to be reliable, which is the actual bug in the gen-1 watch path: a
  delivered entry whose acknowledgement was lost gets re-sent and silently duplicated, and survives
  today only because a natural-key upsert three hops downstream happens to collapse it.
- Makes **at-least-once delivery sufficient**, so no layer needs exactly-once.

**What it requires.**

| | Rule |
|---|---|
| **Identity** | A stable, **client-generated** id on every record, minted at the point of capture — by the device the user touched, not by whatever receives it. UUIDv7 is the recommended shape: it carries creation time, so it sorts naturally and needs no separate ordering column |
| **Same id, identical content** | No-op. Return the existing record and the same status as the original create |
| **Same id, *different* content** | **Not a replay — an edit or a conflict.** It is resolved deliberately: the write is rejected with a distinct conflict status, recorded as `rejected` on its attempt, and surfaced in the console for operator resolution — never silently overwritten. This is exactly what gen-1 does wrong: `ON CONFLICT (timestamp) DO UPDATE` cannot tell the two cases apart, so a second device's edit destroys the first with no signal |
| **Same content, no id** | Rejected. Content-hash identity is tempting and wrong here — two genuinely separate journal entries with the same severity and empty notes are not the same event |

**Scope.** The client-generated id applies to **domain records** — journal entries, songs, tags,
severity ratings, doses. Pushed platform-store samples keep their ingest identity
`(source, type, session_id, seq)` per Amendments 01/05/06 — which is idempotent by construction
(`ON CONFLICT DO NOTHING` on a stable key). Both payload classes ride the **same** phone→Pi
envelope (§8b): the envelope is transport, and the payload class selects the identity contract.
The two identity schemes serve the same rule and never mix.

**The console consequence, and it matters.** Once writes are idempotent, a healthy system produces
**many legitimate no-op successes** — so "the sync succeeded" says even less than it did before.
Every attempt record must therefore distinguish **created · updated · replayed (no-op) · rejected**,
and the ingest surface must show the breakdown rather than a success count. A source whose every
attempt succeeds as `replayed` is the §2.3 high-water-mark case wearing a new costume: working
perfectly, moving nothing.

### 2.4 The law applies to the console itself

The console is a status source and is not exempt.

- **Ages are recomputed continuously client-side.** A page opened at 23:40 and read at 03:10 must not
  still claim "checked 4m ago." Rendered ages advance whether or not new data arrives.
- **The page carries its own feed state, as an overlay — not by rewriting each card.** If the status
  feed goes stale past its own cadence, the *document* degrades and says so, and every card's
  confidence drops with it. It does **not** mutate the individual computed states underneath: an
  event-driven source declared on-demand (§2.5) is not time-degraded by a page problem, and a card
  that was `FAILING` must not lose its reason. Gen-1's crude 5-second meta-refresh is rejected in §5;
  this is its replacement, and the replacement is mandatory.

**Current confidence and last known outcome are separate facts, and both are shown.** This is the
rule that resolves the apparent conflict between the overlay above and the `FAILING` > `UNKNOWN`
precedence in §2.5. A stale feed means the console cannot currently vouch for anything — it does not
mean the last thing it saw stopped being true:

> `UNKNOWN` — feed stale 18m · **last known: FAILING at 02:14, sensor disconnected**

Erasing the second line is the failure. The reason a source was failing is the only fact that lets
the operator fix it, and it is exactly what gen-1 threw away.
- **Clock health is a subsystem.** Every age in this document is computed against the Pi's clock. NTP
  sync state and RTC battery presence appear on the Overview. This is not hypothetical: this project
  has already hit two clock hazards — the CPAP epoch offset (§4.4) and the device-clock-skew replay
  window (§6).

### 2.5 Cadence — the boundary cases

The three-state rule needs these pinned down or it cannot be implemented uniformly.

| Case | Rule |
|---|---|
| **Cadence measures what?** | Attempts. "Last success age" is a second, separately rendered fact — a source attempting every 10 min and failing every time is `FAILING`, not `UNKNOWN` |
| **Stale *and* last-seen-failing** | `FAILING` wins. Promotion order is `FAILING` > `UNKNOWN` > `LIVE` |
| **Event-driven sources** (manual upload, SD card) | Declared **on-demand**: state is the outcome and age of the last event, never time-degraded. Labelled as on-demand so an old timestamp does not read as a fault |
| **In-flight work** | Every attempt record carries an expected duration. An attempt still running past it degrades to `UNKNOWN` — otherwise a killed process renders as "running" forever, which is precisely the gen-1 defect in §4.7 |
| **Legitimately variable cadence** | Declare the *upper bound*. If no honest bound exists, the source is on-demand |

**Cadence is visible, auditable config** (§4.9), and **the expectation renders next to the age** —
"4m ago · expect ≤ 10m". Age alone lets a loosened cadence silently recreate gen-1: a source declared
at 7 days shows 5 silent days as `LIVE`, which is the original incident laundered through config.

### 2.5b The three states are for health assertions only

`LIVE` / `FAILING` / `UNKNOWN` answer one question: *can this thing be relied on right now?* They are
not a universal enum, and forcing every state machine through them produces nonsense — **a completed
upload is not `LIVE`; a queued job is not `UNKNOWN`; a muted fault is not a health state.**

Five separate vocabularies, each rendered in its own visual language:

| Axis | Values | Applies to |
|---|---|---|
| **Health assertion** | `LIVE` · `FAILING` · `UNKNOWN` | Sources, daemons, channels, backups — anything that can be relied on or not |
| **Lifecycle** | `queued` · `running` · `succeeded` · `failed` · `cancelled` | Jobs, uploads, imports — work with a beginning and an end |
| **Process state** | `active` · `activating` · `failed` · `inactive` | Supervised units, straight from the supervisor |
| **Operator suppression** | `muted until <date>, by operator` | An acknowledgement, never a health value |
| **Configuration** | `NOT_CONFIGURED` — declared absent | Optional sources and channels (`GEN1-REASSESSMENT.md` §1). A **standing declaration, not a nightly fact**: an enabled wearable that went unworn is `UNKNOWN` (optionally muted, §2.6), never `NOT_CONFIGURED`. Rendered outside the health palette, excluded from rollups, never promoted. M1 criterion 9 (§9) is its acceptance test |

Any component may carry a health assertion **alongside** its domain state — a job runner has both a
lifecycle for each run and a health assertion about whether the runner itself is trustworthy. What it
may not do is collapse them into one field. M1 criterion 4 (§9) applies to the health axis
only.

### 2.6 The failure mode this law creates, and its bound

Three states with no severity dimension trade gen-1's false calm for **habituation**. If a benign
`UNKNOWN` — an unworn ring, a backup that cannot be verified until the storage lands — permanently
occupies the promoted slot, the operator learns to skim the exact state everything else depends on.
That converges on gen-1's outcome by the opposite route.

Two mechanisms bound it:

- **Severity is a second axis, not a fourth state.** Each subsystem declares whether a fault is
  *consequential* (data being lost or the record going wrong) or *routine*. Promotion order sorts
  on severity first, then state.
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
| 10 | **Alarm path** | Is the CNS early-warning subsystem armed and testable. | **M6, last** |

The Alarm path is the tenth section, but it is outside the current nine-screen design deliverable:
its screen is designed when M6 begins, after the daemons and contracts it depends on exist.

### 3.1 Milestones

**Delivery order is a function of §7b, not of how interesting a section is.** An earlier draft ranked
purely by operator frequency and put Data in the first release — which is blocked on storage
hardware, so that schedule stalled before any loop closed.

| | Contains | Gated on |
|---|---|---|
| **M0** — foundations, no UI | The observation/attempt schema · per-source cadence in the registry · the liveness evaluator as one pure function · an HTTP service · the client write API skeleton — the §8b envelope carrying the §2.3b domain-record class — with a crude test page, deliberately not part of the console · the bind and auth decision | Nothing. None of it exists; everything below needs all of it |
| **M1** — first usable console, read-only | Overview · Display path · Ingest for the registry sources · **attempt history** · registry left-join so a never-seen source appears · console self-freshness · clock state · build and schema identity · host readouts (disk, thermal, undervoltage, network, watchdog) | M0 |
| **M2** — operations | Services with logs and audited restart · **the silence detector** (§2.3) — the first consumer of the liveness evaluator that runs without a page open, feeding outbound email · outbound email with its test loop · operator-override storage for display metadata (§8b) · S1–S5 | M1 |
| **M3** — data | Targeted integrity views, then scoped export | The sample store → storage tasks → the M.2 HAT |
| **M4** — analysis | Job lifecycle, cost ledger, AI review loop · clinical PDF report generation | M3 + a job runner |
| **M5** — records and recovery | Domain CRUD · backups and restore verification | Schema that does not exist yet |
| **M6** — alarm path | Channel availability, self-test, arming | Daemons that do not exist yet |

**Vocabulary.** M-numbers are this console's milestones. *Stage N* refers to the build stages of the
pi5 vitals-core plan (the architecture design this console sits inside); the *M.2 HAT* is the NVMe
storage hardware and has nothing to do with milestone M2.

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
already produced multiple distinct failure modes in deployment, including a systemd unit reporting
`active` while the screen showed nothing.

Required: compositor running, the correct DRM connector attached and reporting `connected`, a Wayland
socket that actually appeared, and the browser process alive. All four are filesystem reads —
`/sys/class/drm` and socket paths, never DRM ioctls; the console backend does not need DRM master or
membership in the render groups. Service-up and screen-lit are different claims, and gen-2 has
already learned the difference the hard way.

**Deliberately not required: the age of the last rendered frame.** An earlier draft asked for it.
Obtaining presentation timestamps for a separate browser process means a compositor extension or DRM
polling — a disproportionate amount of new machinery for a gauge. The checks above catch every
failure mode this project has actually hit. If a screen is physically dark despite all four passing,
that is a hardware or driver fault, not something the console can see.

### 4.3 Services

The machine's own daemons. `PROMPT-admin.md` mocks this screen up; without it here, restarting a
daemon is formally out of scope while the design shows a button for it.

**Derive the unit list from the deployment manifest, never hardcode it.** A hardcoded list goes stale
the moment a daemon is added or renamed, and the console then reports confidently on a set that no
longer matches reality. Whatever is under supervision is what appears — including the display path
(§4.2) and the scheduled importers (§4.4). Per unit: supervisor process state *and* the computed
health assertion, as separate axes (§2.5b), plus uptime, restart count, memory, last error, and last
log lines.

**Detailed host state belongs here too.** M1 puts the immediate disk, thermal, undervoltage,
network, and watchdog readouts on Overview; M2 Services adds history, diagnostics, and controls.
These are the ways this box silently stops recording, and none of them involves a daemon crashing:

- **NVMe free space and wear**, with a projected time-to-full. A disk that fills overnight ends
  collection without any process failing.
- **Thermal and undervoltage flags.** Undervoltage in particular degrades radios and storage before
  anything logs an error.
- **Hardware watchdog present and armed.**
- **Network link and signal.** The wifi has already stranded this box once.
- **Mains present / UPS state and charge**, once the UPS is fitted.

- **Restart and stop are operations with audit entries** (§4.9) and confirmation proportionate to
  consequence (§5). Restarting the lyrics fetcher and restarting the sample-store daemon during an
  active capture are not the same act and must not feel the same.
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
- The failure reason when failing, sanitised (§7, S7)
- What this source **uniquely owns** — i.e. what is unrecoverable if it stays dark

**Precedence must be visible.** Four sources write CPAP session rows under two asymmetric,
independently maintained skip-lists. Which source owns a given night is currently unanswerable
without SQL. Show it.

**Derived-state that silently corrupts must be inspectable.** The CPAP clock-offset correction guesses
an offset for a device epoch bug. In gen-1 it was applied at write time and the corrected date was
the primary key, so a wrong offset filed clinical data under wrong dates with no way to see, confirm,
or reset it.

**Gen-2 must not reproduce that, and structurally cannot if the offset stays out of the write path.**
Samples are append-only and identity is `(source, type, session_id, seq)` — not a timestamp — so a
correction may never rewrite stored rows. Requirements:

- Store the offset as **versioned metadata with its derivation evidence**, and apply it **at read
  time**. Raw device time is always recoverable.
- Show the raw device time, the applied transform, its version, and when it was derived.
- Changing it is a new version, not an edit. A **preview** states which nights move and which would
  collide *before* anything is committed, and both versions remain auditable afterwards.
- Read-time views apply the selected transform version. Continuous aggregates over immutable raw
  timestamps do **not** dynamically rebucket when metadata changes: any corrected-time materialized
  projection is versioned by the transform and explicitly refreshed or rebuilt from raw samples.
  Changing an offset never migrates or rewrites `samples`.

This keeps the requirement inside §8's non-goal — the console never edits physiological samples.

**Milestone split.** Displaying the current transform — raw device time, applied version, its
derivation — is M1, read-only. Minting a new version, with its preview of which nights move, is
**M3**: it is a mutation, and the preview needs the sample store to compute against.

### 4.5 Data

- **Nothing invisible — and no generic browser.** Every stored table is reachable through *some*
  surface: a targeted integrity view, an aggregate, or scoped export (criterion 14, §9). Gen-1
  failed the coverage half — 14 of 40 tables browsable, the largest table and the app-only oximetry
  rows with no surface anywhere — but "all 40 tables" is a gen-1 number, and a generic all-tables
  browser is explicitly **not** the requirement (`GEN1-REASSESSMENT.md` §2.4): gen-2 has seven
  tables, one of them a hypertable.
- **Real pagination, sized to the store.** Keyset pagination on the time index — never `OFFSET`,
  and never `COUNT(*)` against the hypertable. Gen-1's `LIMIT` with no `OFFSET` and a 500-row
  ceiling made **row 501 of any table unreachable through the UI, ever**. Tables that cannot be
  paged safely are aggregated instead — never listed.
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

- Stored credentials: presence and last-4 only, never values. A newly generated API key is the sole
  exception: its plaintext is revealed exactly once at creation (pattern A, §5), then becomes
  unrecoverable. Expiry and rotation are first-class concepts — gen-1 keys live forever.
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
| **Redundant push alert channel** | Channel continuity, not a feature cut — the one channel that reaches the operator away from the console. §4.10 |
| **Clinical PDF report generation** | A documented pre-appointment workflow with no successor. Owner: Analysis (§4.7, M4) |
| **JSON/CSV export** | Browse-and-search is not the same as emit-a-portable-file |
| **Correlation recompute** | Never had a button; easy to lose silently |
| **Timezone setting** | Drives day-bucketing across analysis and correlations |
| **Schema revision + pending migrations** | Today: a manually-dispatched CI job |
| **Deployed build identity** | Gen-1 shows none; gen-2 should not inherit that |
| **Log sanitisation** | Token redaction exists in gen-1. Fresh logging code starts leaking credentials into the journal |
| **"Phone has gone dark"** | Belongs on the Pi, exists nowhere today. §2.3 |
| **Phone-side health-access state** | Irreducible to the Pi, but its *state* must surface here. A phone that silently loses health-data access is invisible from the Pi today — and that exact freeze ran undetected for three months. Owner: Ingest (§4.4, M1), supplied by the phone status beacon |
| **Device↔server reconciliation** | Reframed by gen-2, not carried as-is: with the Pi as system of record, reprovisioning a phone is a projection rebuild, so the empty-store guard and the per-table importers do **not** migrate (`GEN1-REASSESSMENT.md` §3). What needs an owner is the capability — a fresh install rebuilds its view completely — and the surviving lesson that a guard which fails closed names its blocker. Owner: Recovery (§4.6, M5) |

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
| Three states `LIVE` / `FAILING` / `UNKNOWN` | `Liveness` is `Live` / `Stale` / `NeverSeen` | **There is no `FAILING` in `Health`.** `Stale` reads like "bad" but means only "past cadence" and belongs in `UNKNOWN`. Required mapping: `Live → LIVE`, `Stale → UNKNOWN`, `NeverSeen → UNKNOWN`. In M1, `FAILING` is derived from the attempt ledger's recorded bad outcome (§2.3), not from `Liveness`; a future daemon-level bad-data signal would require new protocol plumbing |
| A never-seen source shows `UNKNOWN` | A source that never reported is **absent from the map**, not present as `NeverSeen` | The console must left-join the registry's known sources against reported ones to synthesise "never seen." The daemon has no census to diff against — this is console-side work, and M1 criterion 7 (§9) is otherwise unsatisfiable for a collector that never started |
| Each source declares its cadence | One global constant for the whole daemon; the registry has no cadence field | **M0 criterion 2 (§9) is untestable today.** Per-source cadence in the registry is a named prerequisite: schema change plus codegen |

Additionally, the heartbeat inputs (`monitoring_window_active`, tier evaluation, drain progress) are
public fields with **no request verb that can set them**. Today `monitoring_window_active` can never
become true, so the heartbeat reads healthy unconditionally — which is precisely the defect §2.1
indicts, sitting inside the one contract this console consumes. Fixing it means adding verbs.

### Honest sequencing

**Buildable now, against a live contract:** a subset of §4.1 Overview and §4.4 Ingest — for the five
registry sources only — driven by the daemon's `Health` response, plus the registry left-join above.

**Blocked on the sample store** (→ storage tasks → the M.2 HAT): §4.5 Data, §4.7 Analysis.

**Blocked on schema that does not exist yet:** §4.6 Recovery, §4.8 Records, §4.9 Config & access, and
every attempt/audit requirement in §2.3.

**Blocked on daemons that do not exist yet, and built last by choice:** §4.10 Alarm path.

An earlier draft of this section claimed the alarm path could be built now. That was wrong, and
wrong in the most expensive direction — it is the most blocked section in the document.

## 8. Non-goals

- Reviewing health data — that is the web dashboard.
- Multi-user, roles, or permissions. One operator.
- Editing physiological samples. The console inspects and exports them; corrections are versioned
  read-time transforms (§4.4), never row edits.
- Replacing systemd, Postgres, or the shell. This surfaces state and triggers work; it is not a
  general-purpose remote console.

---

## 8b. Prior art — Home Assistant

Two research passes were run against Home Assistant: how it is built
([`HOMEASSISTANT-ARCHITECTURE.md`](HOMEASSISTANT-ARCHITECTURE.md)) and which of its integrations
carry knowledge this project needs ([`HOMEASSISTANT-PRIOR-ART.md`](HOMEASSISTANT-PRIOR-ART.md)).
This section is the normative subset; the reasoning and the receipts live in those documents.

### Patterns adopted as requirements

| Pattern | Requirement here | Source |
|---|---|---|
| **`reported_at` distinct from `changed_at`** | Every reading and attempt carries the time it was last *affirmatively asserted*, separate from when its value last changed. Freshness is a stored field, never a derived query. HA retrofitted this (`last_reported`, 2024) after losing the steady-vs-dead distinction | ARCH §2.3a |
| **Transition logging** | One log line on entering a failure state, silence while it persists, one line on recovery. Applies to collectors, importers, channels, and the console's own checks | ARCH §2.3c |
| **Jitter on every scheduled repeat** | All timers (polls, retries, refreshes) add random jitter so N failing things cannot resonate | ARCH §3 |
| **Evidence beats a timer** | Data arriving from a source in retry backoff short-circuits the backoff immediately | ARCH §3 |
| **Failure taxonomy** | Every collector/job failure is classified transient / needs-credentials / terminal, and each class is handled differently (retry with backoff · surface for reauth, stop polling · stop, surface) | ARCH §3 |
| **Learned BLE staleness intervals** | BLE collectors may refine the declared cadence with a learned advertising interval (max of a sample window + wobble), with a per-device sleepy override. The registry cadence remains the floor — learned values may tighten expectation, never loosen it past the declaration | ARCH §2.3b |
| **Operator-override / system-default split** | Any operator-editable display metadata (source names, units, precision) stores the system default beside the operator value; display resolves operator-or-default, so upgrades never stomp customisation. Overrides are rows in the console's own schema (M2); system defaults stay in the git-owned registry — **the console never writes git** | ARCH §4 |
| **Never silently stop persisting** | Under load, bound the queue and apply backpressure or spill to disk. Dropping persistence to protect the process — HA's recorder-shedding behaviour — is prohibited; here the data is the product | ARCH §8.3 |

The statistics semantics (time-weighted means, gaps-not-zeros, retention decided at schema time) are
independent HA precedent for choices Amendment 01 and the CNS invariants already made — cited in
ARCH §5, no spec change needed.

### Precedent for the governing law

HA's `unavailable` (can't reach it) vs `unknown` (reached it, no value) distinction is the closest
existing precedent for §2's three states — and HA's gap is the stronger evidence: **a failed poll
writes no state at all, so the previous value sits in the state machine looking healthy forever**,
and exactly 2 of 1,481 integrations implement any staleness expiry. The largest home-telemetry
project ships the defect §2.1 indicts, as default behaviour. That is why liveness belongs in the
core evaluator (M0), not in per-source code: left to integrations, it appears 2 times in 1,481.

### Machinery explicitly not adopted

The stringly-typed state machine (typed values and sum-type status instead) · UI-driven
configuration state (`.storage/`; the registry stays declarative TOML in git) · a generic
multi-backend recorder (one engine, typed schema, retention at design time — Amendment 01) · the
entire third-party-safety apparatus (manifests, quality tiers, runtime blocking-IO watchdog),
which is mechanism substituting for trust this project does not need. Receipts in ARCH §8.

### Integrations to translate

Verdicts, licences, and reasoning in [`HOMEASSISTANT-PRIOR-ART.md`](HOMEASSISTANT-PRIOR-ART.md);
this is the commitment list. These are product and collector obligations rather than console
behaviour; they are recorded here, once, at the operator's direction so the commitment has a
normative home. Apache-2.0 ports carry attribution — and the NOTICE file, where upstream ships one.

| What | From (licence) | Action |
|---|---|---|
| iOS HealthKit push — `HKObserverQuery` + background delivery, 23 metrics | HA companion iOS (Apache-2.0) | **Port** into the iOS app at the phone-ingest stage |
| The webhook envelope — register-once / update-many, libsodium secretbox | HA companion protocol | **Reimplement** in the axum service as the single phone→Pi write surface. The envelope is transport only; it carries two payload classes with two identity contracts — domain records (client UUIDv7, §2.3b) and pushed platform-store samples (ingest identity, Amendments 01/05/06). M0 ships the envelope with the domain-record class; the sample class lands at the phone-ingest stage |
| Android Health Connect sensors | HA companion Android (Apache-2.0) | **Port**, at the later Android stage |
| myAir cloud client — Okta OAuth2 + PKCE, per-region GraphQL | `resmed_myair_sensors` (MIT) | **Port** the `client/` subpackage as the Python batch importer's upgrade path (Amendment 07's carve-out). Gen-1's working `resmed_sync.py` is adopted meanwhile (`GEN1-REASSESSMENT.md`); this is what replaces it when the auth flow shifts |
| SD-card CPAP ingest | `hms-cpap` (MIT); OSCAR is GPL-3 — **formats only, never code** | **Reimplement**, copying hms-cpap's clean-room NOTICE pattern |
| Polar RR + PMD service knowledge | `polar-python` (MIT) | **Reimplement** in the Rust collector |
| BLE operations layer — adapter recovery ladder, wedge watchdog, slot accounting, error-classified backoff, learned intervals | `habluetooth` / `bleak-retry-connector` (Apache-2.0 / MIT) | **Reimplement** in the collector supervisor; ARCH §6 is the porting guide |
| ESPHome as remote GPIO | `aioesphomeapi` (MIT; ESPHome C++ is GPL-3 — talk to it, never vendor it) | **Use only for what must be physically remote**; `reboot_timeout: 0s` and a Pi-side heartbeat are mandatory |

---

## 9. Acceptance criteria

**Criteria are per-milestone (§3.1).** An earlier draft listed eleven criteria under "the build is
not done until," several of which depend on substrate that does not land until M3–M6 — so no
milestone could pass and the list gated nothing. A criterion outside the current milestone is **out
of scope for that release**, not an unticked box.

### M0 — foundations

1. The backend liveness evaluator is **one pure function** with an injectable clock, unit-tested at
   the boundary (`age == effective cadence`, where the effective cadence is the declared floor,
   possibly tightened by a learned interval — §8b). It is the only implementation of source-health
   policy: API responses carry its evaluated state and expiry deadline, and the silence detector
   (M2) calls it directly. The console may compare that deadline with its local clock to advance ages
   and apply the document-level stale-feed overlay (§2.4); it does not reimplement source-health
   precedence or cadence policy in JavaScript.
2. Every source in the registry declares a cadence; a source without one is a **build-time error**,
   not a silently-`LIVE` card. This criterion is the acceptance contract for the registry schema and
   codegen work already named in the M0 milestone row (§3.1); M0 cannot complete before they land.
3. An attempt record exists before its work begins and carries an expected duration; killing the
   process mid-run leaves a row that degrades to `UNKNOWN` rather than reading "running" forever.

### M1 — first usable console

4. Every health assertion is one of three states, and the third is visually distinct from both others
   on every panel. The M0 attempt schema makes `FAILING` reachable in M1: a named failed attempt must
   render it in the M1 acceptance test. (Health axis only — §2.5b.)
5. No status renders without the age of its observation, and the **expectation renders beside it**.
6. Ages advance client-side; with the network dropped, the document degrades to `UNKNOWN` within one
   interval while each card **retains its last known outcome and reason**.
7. Killing any collector produces a visible `UNKNOWN` within its cadence and names which one.
   Restarting the daemon leaves every registry source visible as `UNKNOWN` — none disappears.
8. A source whose high-water mark has not advanced across N successful runs is visibly degraded, not
   `LIVE`.
9. An optional source that is declared-absent renders as such, and does not appear in rollups or
   demand attention.
10. Clock-sync state, build identity, and schema revision are on the Overview.

### M2 — operations

11. Every destructive action has confirmation proportionate to its consequence and lands in the audit
    log with actor and timestamp.
12. Each of S1–S5 has a named test.
13. Email and telemetry each have a test action whose result is recorded; configured-but-never-tested
    reads `UNKNOWN`, and a failing exporter is visible as `FAILING`.

### M3 and later

14. Targeted integrity views answer the source/date questions without a generic table browser;
    export is scoped and complete for the range requested.
15. NULL, zero, and parse-failure are visually distinct wherever provenance exists to distinguish
    them.
16. Cost is shown before a billed action, and a running total exists. *(M4)*
17. Both alarm channels can be tested independently; a never-tested channel reads `UNKNOWN`. *(M6)*

### Every milestone

18. The accessibility bar in `PROMPT-admin.md` is met with **computed** ratios, not asserted ones,
    for whatever shipped.
19. Each of the five gen-1 failures in §2.1 becomes a regression test in the milestone that first
    ships the corresponding capability. Later milestones keep applicable tests passing; an earlier
    milestone is not gated on a feature it has not introduced.

---

## 9b. How this document changes

**This spec is expected to be wrong in places, and finding out is the point.** It was written before
most of what it describes exists (§7b), from an audit of a different system. Contact with real
hardware, real data rates and a real contract will contradict some of it. **Discovering that is a
success, not a failure**, and the spec must not become something implementation works around.

Use the existing amendment mechanism in [`../specs/AMENDMENTS.md`](../specs/AMENDMENTS.md). That log
already does this well — it carries entries that supersede earlier amendments outright, including
*"both were wrong"* and *"the hardware disproved it."* The record of the mistake is most of the
value: it stops the same reasoning being repeated, and it is why §2.1's evidence is worth more than
any assertion in this document.

**What warrants an amendment:**

- Measurement contradicts an estimate. Several numbers here are guesses awaiting data — per-source
  cadences, the `N` in "high-water mark unmoved across N runs", retention. **Replace them with what
  you measure**; do not defend them.
- A contract cannot express a requirement, as with `FAILING` having no representation on the wire.
- A requirement proves unbuildable at proportionate cost — the "age of the last rendered frame" was
  cut for exactly this reason.
- Building it reveals the requirement was solving the wrong problem, or a better solution now exists.

**What does not:** finding it inconvenient, or not getting to it. That is a milestone slip (§3.1),
not a spec change — and recording it as one is how a plan quietly stops describing reality.

**The rule that keeps it honest:** an amendment states what it supersedes and **why the earlier
version was wrong**. An edit that quietly improves the text loses the reason, and the reason is the
part that generalises.

---

## 9c. Decisions deferred to the point of use

These need a human call, and **none of them blocks starting.** Ask when the work reaches them; do not
resolve them early, and do not let them hold up M0.

| Decision | Needed by | Why it can wait |
|---|---|---|
| **Bind and auth posture** — **DECIDED by [Amendment 09](../specs/2026-08-30-amendment-09-mobile-enrollment-and-tls.md)** | T-M2a | Axum remains loopback; Caddy exposes only `/api/v1/client/*` over public system-trusted TLS, while the operator console is WireGuard-only. Pinning is not used initially. T-M2a implements the decision and security middleware before public routing is enabled |
| **Watch: peer node or thin sender with a durable outbox** | Stage 7 ("thin the phone") | Nothing in M0–M3 touches the watch. The existing behaviour keeps working meanwhile. Note that the watch is *already* a durable peer for sensor data — quick-log is the anomaly, not the norm — so the smaller change may simply be making quick-log consistent with what the watch already does |
| **Whether to model the collector layer on Home Assistant's integration/coordinator shape** | Stage 4 (the BLE fleet) | The research landed with a recommendation: borrow the behaviours (§8b), not the machinery — see [`HOMEASSISTANT-ARCHITECTURE.md`](HOMEASSISTANT-ARCHITECTURE.md) §3. What remains at Stage 4 is confirming that against the real collector code, not an open design question |

The general rule: **a decision that is not yet load-bearing should be recorded, not made.** Making it
early spends judgement on incomplete information and then anchors everything after it.

---

## 10. Open questions

1. **Where does the phone-silence notice escalate?** It is the Pi's job (§2.3), but the Pi notifying
   about the phone that also carries Pi notifications needs a defined escalation path that does not
   loop.
2. **Retention.** Several high-volume tables have no pruning job in either generation. Does the
   console show retention state, trigger compaction, or both?
3. **How much of Records migrates at all?** **Decided for the iPhone-first MVP:** journal and
   anxiety records migrate into the same PostgreSQL database under a separate `domain` schema. The
   iPhone health viewer owns capture, editing, history, and deletion; the console owns operational
   conflict review, recovery, audit, and administration. Medication, check-in, and other legacy
   records remain deferred until separately scoped.
4. **Does the SQL pane ship in v1?** S6 makes it safe, but the same discipline argues that a console
   answering the real questions removes the need for it.
