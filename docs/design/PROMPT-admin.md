# Design prompt — AnxietyWatch2 admin console

Paste everything below the line into Claude's design feature.

**Note:** this is a *different* surface from `PROMPT-web-dashboard.md`. That one is about the
**body** — physiological data, trends, what happened last night. This one is about the **machine** —
daemons, radios, database, backups, keys. Different job, different user mindset, different design.
Keep them visually related but do not merge them.

---

I need **three distinct design directions**, each with its own complete design system, for the admin
console of a self-hosted health monitoring appliance. Deliver each as self-contained HTML/CSS — no
external dependencies, no build step.

## Product

**AnxietyWatch** tracks anxiety from both sides — the severity the user reports, and the physiology
their devices record — so that a fifteen-minute psychiatrist appointment becomes a data-informed
conversation instead of an exchange of impressions. A journal entry is the anchor; everything else is
context around it: medication doses with efficacy follow-up, heart-rate variability from a chest
strap, sleep and CPAP nights, pulse oximetry, ring data, labs, prescriptions. A correlation engine
finds which metrics actually move with anxiety, and a report generator turns that into something you
can hand a clinician.

**AnxietyWatch2** relocates the always-on parts onto a Raspberry Pi 5 beside the bed — the radios
that hold Bluetooth sensors all night, and the database that becomes the system of record. The phone
keeps journaling, health-store access, and viewing.

It is **not a medical device**, not FDA-cleared, and not on a path to clearance. It is a personal
tool, aiming at a first version that is genuinely useful to one person. One user: the person tracked,
who is also the engineer who built it.

*(There is an in-progress nocturnal early-warning feature that can sound an alarm. It is one feature
among twenty and is scheduled last. **Do not design this console as an alarm appliance.**)*

This console is not for reviewing health data — that is a separate app. It does two jobs:

> **1. Is the record I am building actually complete and correct?**
> **2. Let me operate the machine that builds it.**

Job 1 is integrity monitoring. Job 2 is real operational work — issuing and revoking credentials,
kicking off long-running jobs, uploading data files, editing records, reviewing AI-generated text.
Roughly half the surface is actions, not readouts. **A design that covers only job 1 is incomplete
and will be rejected.**

**Why integrity is the organising idea.** The product's whole claim is *evidence instead of
impressions* — and that is only as good as the data behind it. A sync gap is a hole in the report you
hand your psychiatrist. A frozen daily aggregate means correlations computed on stale inputs, still
returning confident numbers that are simply wrong. A silently dead CPAP feed guts half the
sleep-apnea-anxiety thesis. **None of these announce themselves. All of them look exactly like a
working system that had a quiet week.** That is what this console exists to make visible.

## What it manages

**Daemons** — the sample-store daemon, per-radio collectors (chest strap, oximeter, BP cuff), the
spool drain, and the scheduled importers. Each with uptime, restart count, memory, last error, and
recent log output. The unit list is derived from the deployment manifest, not hardcoded.

**Host** — NVMe free space and wear with a projected time-to-full, thermal and undervoltage flags,
hardware watchdog armed, network link and signal, mains/UPS state. **These are the ways this box
silently stops recording without any process crashing**, and they matter as much as daemon state.

**Radios** — Bluetooth adapters, paired devices, signal, connection state, reconnect history,
per-source last-sample age against its **declared expected cadence**.

**Ingest** — per source: last attempt and last success as distinct facts, rows moved, **whether the
newest data timestamp actually advanced**, rejected samples and why, and the quarantine table.

**Storage** — Postgres size and growth rate, table row counts, **spool depth and oldest unspooled
age, drain progress and latency**, NVMe wear and free space, retention and compaction status.

**Backups** — last full, last incremental, WAL archive lag, and **the last verified restore** (a
restore into a scratch cluster with row-count comparison, which either passed or failed).

**Jobs** — correlation recomputes and AI analysis runs: queued, running with elapsed time, succeeded,
failed with a reason, cancellable, with a run history and a cost ledger.

**Config** — the type/source registry including per-source cadences, thresholds, provider keys
(presence and last-4 only, never values), feature flags, an outbound email channel, and optional
telemetry that is off by default.

**Logs** — recent daemon output, filterable by unit and severity, with credentials redacted.

## The single most important constraint

**"Unknown" must never look like "healthy."**

Three states, where most admin dashboards show two:

| State | Reads as |
|---|---|
| Checked, and good | OK |
| Checked, and bad | PROBLEM |
| **Not checked / stale / never ran** | **A QUESTION** |

A backup that has never been verified is not a passing backup. A self-test that hasn't run in 40
days is not a working alarm. A collector that reports no errors because it never started is not
healthy. Design an explicit visual language for "we don't know," distinct from both good and bad,
and show it on every panel.

**Freshness is part of every status.** No status indicator appears without the age of the
observation behind it. "OK" alone is meaningless; "OK · checked 4m ago" is a fact. A status whose
underlying check is older than its expected interval degrades to unknown automatically.

**Show the expectation next to the age** — "4m ago · expect ≤ 10m". Age alone hides the standard it
is being judged against, and a source with a loose cadence can then sit silent for days looking fine.

### Four things this constraint does *not* mean

These are the places a three-state design usually goes wrong. Get them right and the rest follows.

**1. The three states are for *health assertions only.*** They answer "can this be relied on right
now?" They are not a universal enum. A completed upload is not OK; a queued job is not a question; a
muted fault is not a health state. Design **four separate visual vocabularies**:

| Axis | Values |
|---|---|
| Health assertion | good · bad · unknown |
| Lifecycle | queued · running · succeeded · failed · cancelled |
| Process state | active · activating · failed · inactive |
| Operator suppression | muted until *date*, by operator |

A component may show a health assertion **beside** its lifecycle state. It must never collapse them.

**2. "Not configured" is not "unknown."** An optional source the user does not own, or simply did not
wear last night, must render as *declared absent* — excluded from rollups, visible but quiet. If
every optional thing shows a question mark, the question mark becomes wallpaper and stops meaning
anything. **This is the single fastest way to destroy the value of the third state.**

**3. Current confidence and last known outcome are separate facts, and both are shown.** When the
page's own feed goes stale, the console cannot vouch for anything — but that does not erase what it
last saw:

> **unknown** — feed stale 18m · *last known: failing at 02:14, sensor disconnected*

Losing that second line is a design failure. The reason is the only thing that lets the user fix it.

**4. The page is not exempt.** A page opened at 23:00 and read at 03:30 must not still claim
"checked 40s ago." Ages advance on screen; the document carries its own freshness; and when the feed
dies, the *document* degrades — as an overlay, without rewriting each card's underlying state.

### Severity is a second axis, not a fourth state

Some faults are consequential (data being lost, the record going wrong); some are routine. Sort by
severity first, then state — otherwise one permanently-unknown benign subsystem occupies the top of
the screen forever and trains the user to ignore it.

## Operations — four interaction patterns, all required

These are the half a monitoring dashboard does not have. Design each explicitly, in all its states.

**A. Credential lifecycle.** Issue an API key, revoke one, list existing keys. The plaintext key is
shown **exactly once**, at creation, and is unrecoverable afterwards — thereafter only a fingerprint
and last-4 are displayed. Design the one-time reveal so it is obvious it will not be shown again,
and so it is not trivially left on screen for a screenshot. Revocation is destructive and immediate.

**B. Long-running jobs.** Triggering a Claude analysis run takes minutes. A button that posts and
reloads the page is not a design for this. Show queued / running / succeeded / failed as distinct
states, with elapsed time, the ability to cancel, and a run history with each run's parameters and
outcome. A failed run shows why, in a form that can be acted on.

**C. AI-generated content review.** Several actions generate prose the user then edits and accepts —
profile summaries, research notes, analysis narratives. Design the generate → review → accept /
edit / regenerate loop. The prior version must remain visible while reviewing the new one, and
nothing is overwritten until explicitly accepted.

**D. File upload with validation.** CPAP data files are uploaded and parsed. Show parse progress,
rows accepted, rows rejected **and why**, and what happens when the same file is uploaded twice. A
silent success on a file that parsed zero rows is a defect.

Also required throughout: **ordinary forms** (profiles, therapy schedule, app settings, third-party
credentials) with validation, dirty-state handling, and explicit save/discard.

## Screens

Nine. **The first three are the first release** — design those most carefully, and show them most
completely. The rest are real and must be designed, but they land later.

1. **Overview** — the whole machine and the state of the record, on one screen. Every subsystem,
   current state, freshness. Readable in five seconds, from bed, on a phone.
2. **Display path** — is the bedside screen actually lit: compositor running, connector attached,
   socket present, browser alive. *Service-up and screen-lit are different claims.*
3. **Ingest** — per source: state, last attempt **and** last success, rows moved, whether the newest
   data timestamp advanced, what that source uniquely owns, and the attempt history.
4. **Services** — per-daemon detail, logs, restart controls, resource use over time, **plus host
   state** (disk, thermal, undervoltage, watchdog, network, power).
5. **Config & access** — API keys: issue, list, revoke (pattern A). Third-party credentials. The
   registry and its per-source cadences. Outbound email. Optional telemetry, off by default. Audit
   log.
6. **Data** — targeted integrity views by source and date range, quarantine, retention, then scoped
   export. Upload data files (pattern D). *Not a generic all-tables browser.*
7. **Analysis** — trigger runs, watch them, read results, review and accept AI-generated text
   (patterns B and C), and see cost **before** spending it.
8. **Records** — journal, medications, prescriptions, pharmacies, profiles, therapy schedule, songs.
   CRUD forms with AI-assist actions.
9. **Recovery** — backups, WAL archive, restore-verification history, and a prominent "what would I
   lose if this died right now?" answer.

Screens 7–9 are where a lot of the real work happens. Do not treat them as an afterthought to the
monitoring screens.

**One screen deliberately excluded from prominence: the alarm path.** It exists, it is scheduled
last, and it should appear as a normal section rather than a headline. A design that makes this
console look like an alarm panel has misread the product.

## The three directions

Name each, give a one-paragraph rationale, and a full design system. Genuinely different bets, not
three palettes on one layout.

**Direction A — Clinical / biomedical equipment.** Model it on the service and technician interfaces
of real hospital equipment — the screens behind the patient-facing display on a Philips IntelliVue,
GE CARESCAPE, or a Dräger ventilator. Rigid information hierarchy, fixed field positions that never
reflow, dense tabular readouts, explicit units on everything, alarm-priority conventions
(high/medium/low) with the distinct visual weight each carries, technical alarms strictly separated
from physiological ones, and timestamps on every value. It should look like the maintenance mode of
a regulated medical device. Do not soften it — this direction earns its keep by being
uncompromising.

**Direction B and C — your call.** Two coherent alternatives. Real positions. As illustration, not a
menu: a systems-console approach with monospace and keyboard-first navigation; a topology approach
where the machine is drawn as a live dataflow diagram and health is read off the graph; an
inspection approach modeled on aircraft pre-flight checklists where every item is explicitly
signed-off or explicitly not. Surprise me.

## Accessibility — hard requirements

- **WCAG 2.2 AA minimum.** Text ≥ 4.5:1 normal, ≥ 3:1 large (≥ 24px, or ≥ 18.66px bold). Non-text UI
  and graphical objects ≥ 3:1. **State the computed ratio for every color pair** — show the number,
  don't assert compliance.
- **Never encode meaning in hue alone** (WCAG 1.4.1). Pair every status with a glyph, a label, or a
  fill-vs-outline distinction that survives grayscale printing.
- **Colorblind-safe** under deuteranopia, protanopia, and tritanopia. State how you verified. Note
  that red/amber is the classic collision and a status palette lives or dies on it.
- Body text ≥ 16px, data labels ≥ 14px, nothing below 14px.
- Tabular figures on all numerics so values don't jitter as they refresh.
- Full keyboard navigation, visible focus, respects `prefers-reduced-motion`.
- Dark theme primary; light theme is a designed second theme with its own validated steps, not an
  inverted filter.

## Color

- Reserve status colors strictly for status. A status hue must never also be "category 4" in a chart.
- **Do not center the palette on brown, tan, or parchment** — it reads as a document rather than an
  instrument.
- Consider whether "healthy" needs a color at all. On a console someone checks at 3am, an
  all-neutral resting state where color appears *only* on exception is a legitimate and often better
  choice. If you use green, justify it.
- Charts: one color per metric held across every screen. Never a dual-axis chart. Sequential = one
  hue light→dark; diverging = two hues with a neutral midpoint; never a rainbow.

## Craft bar

Dense but not cluttered. Restrained color used with intent. Typography carrying the hierarchy.
Destructive controls (restart a daemon, run a restore) visually separated from safe ones and
requiring deliberate confirmation — never a single click adjacent to something routine.

**Avoid:** rounded pastel cards, gradient headers, emoji, motivational copy, circular progress rings,
decorative drop shadows, dashboards that look like a SaaS marketing page, and any layout where the
most important number is not the largest thing on screen.

## Deliverables per direction

1. **Design system page** — color tokens with computed contrast ratios, the status palette including
   the explicit "unknown/stale" treatment **and the separate "not configured" treatment**, all four
   status vocabularies, type scale, spacing, and every component in all states: default, loading,
   empty, stale, unknown, not-configured, warning, critical. Plus form controls, buttons by severity,
   and modals.
2. **High-fidelity mockups of all nine screens** with realistic sample data. Screens 1–3 are the
   first release — show those most completely.
3. **At least one screen per direction in a degraded state** — a collector crashed, a backup 9 days
   stale, a restore verification that failed.
3b. **The attempt-history component**, in its own right: a run list showing attempted / succeeded /
   failed with reasons and durations, and the difference between *silent* (no attempts at all) and
   *failing* (attempts happening, all failing). **This is the component that carries the product's
   central claim** — design it, do not bury it in a table.
3c. **A first-run / empty state.** Nothing configured, no data yet, no attempts. Distinct from
   degraded — a fresh install must not look broken.
4. **All four operational patterns, each shown across its full state sequence:**
   - **A** — issue a key (including the one-time reveal), the list afterwards, and revocation with
     its confirmation.
   - **B** — a long-running job as queued → running (with elapsed time and cancel) → succeeded, and
     separately → failed with an actionable reason. Plus the run history.
   - **C** — generate → review (old and new side by side) → accept / edit / regenerate.
   - **D** — file upload as idle → uploading → parsed-with-rejections, showing rejected rows and
     their reasons.
5. **A destructive-action confirmation** that is proportionate — revoking a key and deleting a
   therapy session should not feel identical, and neither should be a single click adjacent to
   something routine.
6. Responsive: desktop primary, phone secondary (it will get checked from bed).
