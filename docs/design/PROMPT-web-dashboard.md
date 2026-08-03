# Design prompt — AnxietyWatch web dashboard

Paste everything below the line into Claude's design feature. Bring the returned
files back to the repo and they'll be wired to the `vitalsd` query and health endpoints.

---

I need **three distinct design directions**, each with its own complete design system, for a
single-user clinical health monitoring web app. Deliver each as self-contained HTML/CSS with no
external dependencies and no build step.

## Product

**AnxietyWatch.** A Raspberry Pi continuously collects data from body-worn sensors overnight,
detects CNS-depression events, and sounds an alarm if one occurs. This web interface is how the
single user — the person being monitored, who is also the engineer who built it — reviews what
happened and confirms the system is actually working.

This is **not** a consumer wellness app. No streaks, no scores, no encouragement, no gamification,
no badges. The user is technically sophisticated and wants density and precision, not reassurance.
Treat it as an instrument someone's safety depends on.

## Data displayed

- SpO₂ (%), heart rate (bpm), HRV as RMSSD (ms), RR intervals (s)
- Blood pressure — systolic / diastolic / MAP (mmHg), irregular-pulse flag
- CPAP — AHI, mask leak, pressure, usage hours
- Sleep stages, respiratory rate, barometric pressure
- Overnight sessions: 6–9 hour continuous traces, roughly 30,000 points
- Per-sensor liveness — last sample age, connection state
- A risk tier state machine: `clear → watch → confirm → klaxon`
- Medications, journal entries, lab results

## The single most important constraint

**"No data" must never look like "everything is fine."**

Three states exist where most dashboards show two:

| | State | Must read as |
|---|---|---|
| 1 | Measured, within range | OK |
| 2 | Measured, abnormal | ALERT |
| 3 | **Not measured / can't assess** | **A QUESTION** |

State 3 must be visually distinct from state 1 at a glance, must never be green, and must never be
silent. A sensor that fell off at 2 a.m. must not render as a calm dashboard. Design an explicit
visual language for "unknown" that reads as neither safe nor alarming, and show me how it appears in
every component — tiles, charts, summaries, lists.

**Chart gaps are part of this.** A line chart must make a data gap obviously a gap. Never
interpolate across missing data — a continuous line is a claim the system cannot make.

## Screens (all three directions must cover all four)

1. **Now** — live status. Current tier, live SpO₂/HR, last BP, per-sensor liveness, time since last
   sample per source. Answers *"is it working right now?"*
2. **Last night** — one session in detail. Multi-lane synchronized time series (SpO₂ / HR / RR /
   CPAP events) sharing one x-axis, with detected events marked.
3. **Trends** — 7/30/90-day windows, personal rolling baselines, deviation from the user's own
   normal. Never population norms.
4. **System health** — daemon status, backup freshness, last verified restore, alarm self-test
   result, storage. Answers *"will it work tonight?"*

## The three directions

Give each a name, a one-paragraph rationale, and its own full design system. They should be
genuinely different bets, not three coats of paint on one layout.

**Direction A — Clinical.** Model it on a real hospital patient monitor: Philips IntelliVue, GE
CARESCAPE, Dräger. Every convention those carry, done properly — per-parameter color coding held
consistent across every screen, waveform lanes with fixed vertical scales and visible gridlines,
numerics in dedicated boxed regions at consistent positions, alarm-limit indicators shown inline
next to the value they bound, priority-ranked alarm banners (high/medium/low with the distinct
visual weight each carries), a technical-alarm channel separate from physiological alarms, and
timestamps on absolutely everything. It should look like equipment, be legible across a room, and be
instantly familiar to anyone who has worked a hospital floor. Do not soften it for consumer taste —
this direction earns its keep by being uncompromising.

**Direction B and C — your call.** Two other coherent directions that solve the same problem
differently. Take real positions. Examples of the *kind* of bet I mean, not a menu to pick from:
a data-journalism approach where the night reads as an annotated narrative; a spatial/timeline-first
approach where everything is anchored to one scrubable 24-hour spine; a terminal/monospace approach
with keyboard-first navigation. Surprise me, but make each internally consistent and defensible.

## Accessibility — hard requirements, not aspirations

- **WCAG 2.2 AA minimum throughout.** Text contrast ≥ 4.5:1 normal, ≥ 3:1 large (≥ 24px, or ≥ 18.66px
  bold). Non-text UI and graphical objects ≥ 3:1. State the computed ratio for every color pair in
  the design system — don't assert compliance, show the number.
- **Never encode meaning in hue alone** (WCAG 1.4.1). Every status pairs color with a glyph, a
  label, a fill-vs-outline distinction, or position.
- **Colorblind-safe.** Verify categorical palettes are separable under deuteranopia, protanopia, and
  tritanopia. State how you verified.
- **Body text ≥ 16px.** Data labels ≥ 14px. Nothing below 14px anywhere.
- **Tabular figures** for all numerics so values don't reflow as they tick.
- Visible focus indicators, full keyboard navigation, respects `prefers-reduced-motion`.
- Dark theme is primary — this gets viewed at 3 a.m. Light theme is a designed second theme with its
  own validated steps, not an inverted filter.

## Chart requirements

- One color per physiological signal, held across every screen that signal appears on. Specify the
  tokens.
- **Never a dual-axis chart.** Two measures of different scale become two charts, small multiples,
  or an indexed common base.
- Thin marks, recessive gridlines, selective direct labels — never a number on every point.
- Legend present whenever ≥ 2 series share a plot.
- Sequential scales: one hue, light→dark. Diverging: two hues with a neutral gray midpoint. Never a
  rainbow.

## Craft bar

Restrained color used with intent, so that when color appears it means something. Generous negative
space around critical numbers. Typography that makes values scannable — a clear hierarchy of value
vs unit vs label vs delta. It should feel trustworthy at 3 a.m.

**Avoid:** rounded pastel cards, gradient hero sections, emoji, motivational copy, big circular
progress rings, drop shadows used decoratively, and anything that reads as a fitness tracker. Also
avoid a brown/tan/parchment-centered palette — it reads as a document, not an instrument.

## Deliverables per direction

1. **Design system page** — color tokens with computed contrast ratios, the chart palette, the
   explicit "indeterminate" treatment, type scale, spacing scale, and every component in all its
   states: default, loading, empty, stale, indeterminate, warning, critical.
2. **High-fidelity mockups of all four screens** with realistic sample data.
3. **At least one screen per direction shown in a degraded state** — a sensor offline mid-session,
   with the resulting gap visible in the chart.
4. Responsive: phone primary, desktop secondary.
