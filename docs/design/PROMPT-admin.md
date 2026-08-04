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

**AnxietyWatch2** runs on a Raspberry Pi 5 beside a bed. It collects data from body-worn sensors
over Bluetooth all night, stores it in PostgreSQL, detects CNS-depression events, and sounds an
alarm if one occurs. There is exactly one user: the person being monitored, who is also the engineer
who built and maintains it.

This console is not for reviewing health data — that is a separate app. It does two jobs:

> **1. Is this machine going to work tonight?**
> **2. Let me operate it.**

Job 1 is monitoring. Job 2 is real operational work — issuing and revoking credentials, kicking off
long-running analysis jobs, uploading data files, editing records, reviewing AI-generated text.
Roughly half the surface is actions, not readouts. **A design that covers only job 1 is incomplete
and will be rejected.**

## What it manages

**Daemons** — `vitalsd` (the sample store), per-radio collectors (Polar H10, EMAY oximeter, A&D BP
cuff), `cns-engine` (detection), `klaxond` (alarm), `wal-tail` (SQLite→Postgres replication). Each
with uptime, restart count, memory, last error.

**Radios** — Bluetooth adapters, paired devices, RSSI, connection state, reconnect history, per-source
last-sample age and expected cadence.

**Storage** — Postgres size and growth rate, table row counts, SQLite WAL depth, NVMe wear and free
space, retention/compaction status.

**Backups** — last full, last incremental, WAL archive lag, and **the last verified restore** (a
restore into a scratch cluster with row-count comparison, which either passed or failed).

**The alarm path** — last self-test result and timestamp, audio device present, volume, amplifier
responding.

**Ingest** — samples/minute by source, rejected samples and why, the quarantine table.

**Config** — the type/source registry, thresholds, provider keys (presence and last-4 only, never
values), feature flags.

**Logs** — recent daemon output, filterable by unit and severity.

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

1. **Overview** — the whole machine on one screen. Every subsystem, current state, freshness.
   Readable in five seconds.
2. **Services** — per-daemon detail, logs, restart controls, resource use over time.
3. **Data** — ingest rates by source, storage growth, quarantine, retention. Browse and search
   records. Upload CPAP files (pattern D).
4. **Recovery** — backups, WAL archive, restore-verification history, and a prominent "what would I
   lose if this died right now?" answer.
5. **Analysis** — trigger runs, watch them, read results, review and accept AI-generated text
   (patterns B and C).
6. **Access** — API keys: issue, list, revoke. Third-party credentials (ResMed, Oura). Pattern A.
7. **Records** — patient and psychiatrist profiles, therapy schedule, conflicts, songs. CRUD forms
   with AI-assist actions.
8. **Config** — registry, thresholds, feature flags.

Screens 5–7 are where the existing system's real work happens. Do not treat them as an afterthought
to the monitoring screens.

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
   the explicit "unknown/stale" treatment, type scale, spacing, and every component in all states:
   default, loading, empty, stale, unknown, warning, critical. Plus form controls, buttons by
   severity, and modals.
2. **High-fidelity mockups of all eight screens** with realistic sample data.
3. **At least one screen per direction in a degraded state** — a collector crashed, a backup 9 days
   stale, a restore verification that failed.
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
