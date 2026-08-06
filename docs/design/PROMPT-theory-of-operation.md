# Design prompt — "Theory of operation" one-pager

Paste everything below the line into Claude's design feature. Companion to
`PROMPT-progress-map.md`: that page maps what is *built*; this one maps how the finished machine
*works*. Bring the returned files back to `docs/design/theory-of-operation/claude-design/`.

---

I need **one self-contained HTML page** that explains how a completed home medical-monitoring
platform operates, end to end. Single file, no build step, no dependencies, no external requests;
system font stacks are fine. It should survive printing to black-and-white on one or two sheets.

## Who reads it

Two people. The engineer-owner at 3 a.m., two years from now, holding a fault and needing to
remember how the machine is supposed to behave. And a clinician or collaborator seeing the system
for the first time, who needs to believe it was built by someone careful. Neither reader is being
sold anything. There is no third audience.

## What the machine is

A Raspberry Pi 5 on a nightstand is the system of record for one person's health platform. Every
night it captures body-worn sensor data, watches for CNS depression, and sounds an alarm that can
wake that person if the pattern demands it. Every morning it answers "what happened last night."
Continuously it proves to itself that it still works. Those are the three loops the page must
draw — the **night path**, the **morning path**, and the **trust loop** — and they run through
shared plumbing but serve different masters: the night path serves the alarm, the morning path
serves understanding, the trust loop serves the other two.

**This page describes the system as designed, complete.** One line near the title says so and
points at the progress map for build status. Do not hatch or hedge individual elements — the
honesty boundary is drawn once, at the top.

## The material — draw from this, all of it is real

**Night path (capture → alarm).**
- Sensors, with their cadences: Polar H10 chest strap (beat-to-beat RR intervals, continuous, the
  only true HRV source) · EMAY SleepO2 pulse oximeter (SpO₂ + pulse, ~1/s) · A&D UA-651BLE blood
  pressure cuff (on-demand readings) · ResMed AirSense 11 CPAP (one import per night: AHI, mask
  leak, pressure) · phone HealthKit bridge (hourly push) · Oura ring via cloud (daily) · journal
  entries and medication doses, entered by hand — the anchor the physiology is context for.
- One long-range BLE dongle (Sena UD100-G03) is the only radio. Collectors decode each device's
  protocol and append rows **already encoded in PostgreSQL binary COPY format** to an fsync'd
  spool — a sample survives a crash from the moment it is spooled. A drain process streams spool
  bytes into TimescaleDB (PostgreSQL 16, hypertable on NVMe, compressed after 7 days, raw dropped
  at 2 years). Gaps are stored and served as NULLs; nothing ever interpolates across missing
  physiology.
- A detection pipeline reads the live stream: per-source quality gate → baseline-relative severity
  scoring → cross-sensor fusion → a hysteretic tier machine. Five tiers: `clear`, `can't assess`,
  `watch`, `confirm`, `klaxon`. Missing or artifact data can never escalate a tier and can never
  clear one — it holds, as `can't assess`. Only observed primary evidence (SpO₂, respiration)
  moves the machine past `watch` in either direction.
- The alarm is a **stack of channels ordered by what they refuse to depend on**: a GPIO piezo
  siren (a pin, a MOSFET, 12 V — no USB, no audio stack, self-tested by sensing its own current
  draw) · a bed shaker on a relay · a USB DAC + amplifier + speaker · Zigbee devices · a push to
  the phone · a watch haptic. The klaxon fires the stack; the independent channels exist because
  any one of them can be dead without notice.
- The whole hop from radio packet to tier evaluation carries a service-level bound: **p99 under
  2 seconds**. Above 5 seconds is itself a fault the system must surface. Latency is part of how
  the machine works, so it belongs on this page with its number.

**Morning path (storage → understanding).**
- A query daemon exposes three verbs: `Query` (history), `Health` (liveness and freshness of
  every source and subsystem), `Subscribe` (live stream). Everything below draws through them.
- The bedside panel: 800×480, a status light readable at 3 a.m. from three feet. **A viewer,
  never a dependency — the alarm sounds identically with the panel unplugged.** Draw that fact;
  it is the single most misunderstood line in the architecture.
- The web dashboard: last night as four synchronized lanes, trends against the person's own
  rolling median (never population norms), system health.
- The admin console: the operator's room. **Technical alarms only** — a dead oximeter is not a
  sick patient, and the two kinds of alarm never share a voice, a channel, or a screen.
- The phone, thinned to three jobs: HealthKit bridge in, journal in, viewer out.
- Downstream artifacts: a clinical PDF for appointments; AI analysis jobs that run under a
  monthly cost cap and produce reviewable text, never silent edits.

**Trust loop (the machine watching itself).**
- Hardware watchdog, 15 s. CPU governor pinned. Undervoltage and thermal readouts.
- Nightly full backup plus WAL shipping, three retention tiers, **and a scheduled restore into a
  scratch cluster — because a backup that has never been restored is a hypothesis, not a backup.**
- A standing answer to "if this box died right now, what would I lose?" measured in minutes.
- Alarm self-test on a schedule; per-source cadence registry so a silent sensor is distinguishable
  from a failing one; every status everywhere carries the age of the observation behind it.

**The invariant that outranks the others: unknown never looks like healthy.** Three states exist
where most systems show two — measured-and-fine, measured-and-bad, and not-measured — and the
third must be visually louder than the first everywhere it appears.

## The visual problem — take one position

This is a *mechanism* page, and mechanism pages die by generic flowchart. So: **no
node-and-arrow diagram soup.** If a reader could mistake any region of the page for autogenerated
graph layout — uniform rounded boxes, orthogonal connector arrows, evenly spaced ranks — that
region has failed. Choose a stronger organizing physics. Four candidate framings; steal one,
combine at your peril, or beat them all:

- **The night as a score.** The page is one night, 22:10 to 07:40, drawn like an orchestral score
  or a multi-channel strip chart: each subsystem is a stave, events are marks with real
  timestamps (drain cycles, the 04:06 CPAP import, the 06:00 backup, a 03:14 tier escalation
  played out mark by mark until the siren line fires).
- **A service-manual theory-of-operation sheet.** The signal path as numbered test points, TP1
  through TPn, laminated-card register: at each point, what a healthy reading is, what its
  absence means, and which page of the console shows it. The register old HP and Tektronix
  manuals earned.
- **A cutaway section.** Strata from skin to silicon — body, radio, decode, spool, database,
  judgment, alarm — drawn like a building section or a geological column, with the three loops
  threading vertically through the layers at different widths.
- **The life of one sample.** Follow a single RR interval from chest to consequence, every hop
  stamped with its share of the 2-second budget, the page widening from one sample to the
  aggregate views built from millions of it.

Whichever you choose, commit: one spine, stated in the first screenful, carried to the end.

## Register — what this page must not sound or look like

- Every sentence carries a fact. If a sentence would survive on a startup landing page, cut it.
- Real numbers, everywhere, from the material above. If you need a number that is not given here,
  invent it *and mark it invented* — a plausible unmarked number is worse than a gap.
- The system's own vocabulary: spool, drain, hypertable, tier, klaxon, can't-assess, cadence,
  restore verification. Never "data pipeline", "insights", "seamless", "robust", "leverages".
- No emoji, no icon grids, no gradient hero, no three-equal-cards row, no decorative
  illustration, no uniform drop shadows. Decoration that carries no information is banned;
  asymmetry that follows the content is welcome.
- Dark page is fine (this thing lives next to a bed), but it must print clean to B/W — pattern
  and shape carry meaning first, hue second, like the progress map's print rules.
- Include a short contrast-audit comment in the source, values computed against your actual
  surfaces. The other pages in this repo do; this one is not the exception.
- The progress map and the console mockups set the register — engineering documents with a pulse.
  Match the register. Do not imitate their layouts.
