# Design prompt — Raspberry Pi bedside panel (800×480 touch)

Paste everything below the line into Claude's design feature. A working reference implementation
already exists at `docs/design/pi-panel/index.html` — treat it as one solved direction to beat, not
as a constraint.

---

I need **three distinct design directions**, each with its own complete design system, for a
fixed-function bedside status panel. Deliver each as self-contained HTML/CSS, no external
dependencies, no build step, sized exactly 800×480.

## The device

A **Hosyond 7" DSI display, 800×480, landscape, capacitive touch**, mounted on a Raspberry Pi 5
beside a bed. It runs one fullscreen app and nothing else — no browser chrome, no OS UI, no window
management. It is on all night, every night.

## Physical viewing conditions — these drive everything

- Viewed from **~3 feet away** while lying in bed.
- The room is **fully dark**. Eyes are dark-adapted; pupils are wide.
- It is often read **half-asleep, at 3 a.m.**, possibly during a medical event.
- **The panel must not emit enough light to disturb sleep.** It is in the bedroom all night.
- 800×480 at arm's length is a **status light, not a dashboard.** Do not try to fit a dashboard onto
  it. Anything requiring more than about two seconds to read has failed.

These constraints conflict — bright enough to read instantly, dim enough to sleep beside. Resolving
that tension well is the core of this design problem. Take a position on it and explain the position.

## What the panel shows

- **Risk tier**: `clear → watch → confirm → klaxon`, plus a distinct **can't-assess** state
- Live SpO₂ (%) and heart rate (bpm), each with a short recent trend
- Last blood pressure (systolic/diastolic/MAP) and how long ago it was taken
- Per-sensor liveness: Polar H10, EMAY oximeter, A&D BP cuff, HealthKit feed — each with last-sample age
- System health: alarm self-test result, backup freshness, last verified restore, disk

## The single most important constraint

**"No data" must never look like "everything is fine."**

If SpO₂ stops arriving at 2 a.m., the panel must not read as calm. The tier itself degrades to
**CAN'T ASSESS** — it does not stay at `clear`, and it does not escalate to an alarm either. Three
states, not two:

| State | Reads as |
|---|---|
| Measured, in range | OK |
| Measured, abnormal | ALERT |
| **Not measured** | **A QUESTION** |

Design an explicit visual language for the third. It must be distinguishable from "fine" at three
feet, in the dark, half-asleep, in under two seconds. Show it in every component.

**Trend gaps count.** A sparkline must terminate where data stopped, not bridge the gap. A
continuous line across missing data is a claim the system cannot make.

## Touch — this is a touchscreen, design for it

- **All interactive targets ≥ 44×44 px** (WCAG 2.5.5 AAA). No exceptions, no dense controls.
- Assume an imprecise, half-asleep finger. No small hit areas, no adjacent destructive controls.
- **Silencing the alarm must be press-and-hold, never a tap.** A hand brushing the panel while
  rolling over must not be able to dismiss a life-safety alarm. Design the hold interaction and its
  progress feedback; make the requirement legible the instant contact starts, and make releasing
  early reset to zero with no partial credit.
- Design the resting/wake behavior: what the panel looks like untouched at 3 a.m. versus when a hand
  lands on it, and how it behaves when the tier escalates on its own.
- No hover states. No cursor. No scrollbars. Nothing that requires precision.

## The three directions

Give each a name, a one-paragraph rationale, and its own full design system. Genuinely different
bets, not three palettes on one layout.

**Direction A — Clinical.** Model it on a real hospital patient monitor: Philips IntelliVue, GE
CARESCAPE, Dräger. Every convention those carry, done properly — per-parameter color coding held
rigidly consistent, numerics in fixed boxed regions that never move, alarm limits shown inline
beside the value they bound, priority-ranked alarm states with the distinct visual weight each
carries, technical alarms kept on a separate channel from physiological ones, and timestamps on
everything. It should look like equipment and be instantly familiar to anyone who has worked a
hospital floor. Adapt the conventions to a dark bedroom rather than a lit ward, but do not soften
them for consumer taste — this direction earns its keep by being uncompromising.

**Direction B and C — your call.** Two other coherent directions. Real positions, each internally
consistent. The kind of bet I mean, as illustration rather than a menu: a near-invisible ambient
object that is almost black at rest and only resolves into information when something changes; a
single-glyph approach where one large abstract form encodes state through shape rather than reading;
a typographic approach with no chrome at all, where hierarchy is carried entirely by type. Surprise
me.

## Accessibility — hard requirements

- **WCAG 2.2 AA minimum.** Text ≥ 4.5:1 normal, ≥ 3:1 large (≥ 24px, or ≥ 18.66px bold). Non-text UI
  and graphical objects ≥ 3:1. **State the computed contrast ratio for every color pair** — don't
  assert compliance, show the number.
- **Never encode meaning in hue alone** (WCAG 1.4.1). Pair every status with a glyph, a label, or a
  fill-vs-outline distinction that survives being printed in grayscale.
- **Colorblind-safe** under deuteranopia, protanopia, and tritanopia. State how you verified.
- Given the 3-foot viewing distance: primary values ≥ 56px, labels ≥ 14px, **nothing below 14px
  anywhere on the panel.**
- Tabular figures throughout so numbers don't reflow as they tick.

## Craft bar

Restrained color used with intent, so that when color appears it means something. It should feel
like a considered object in a bedroom, not a piece of software someone left running.

**Avoid:** rounded pastel cards, gradients, emoji, drop shadows, circular progress rings, anything
that reads as a fitness tracker or a smart-home app. **Do not center the palette on brown, tan, or
parchment** — it reads as a document rather than an instrument. Avoid large fields of saturated blue
(worst case for melatonin at night), though dim cool neutrals are fine.

## Deliverables per direction

1. **Design system page** — color tokens with computed contrast ratios, type scale sized for 3-foot
   viewing, spacing, touch-target specification, and the explicit "indeterminate" treatment.
2. **The panel rendered in all five states**, each at exactly 800×480:
   `clear` · `can't assess` · `watch` · `confirm` · `klaxon`
3. **The hold-to-silence interaction** shown across its states: idle, in-progress, completed,
   released-early.
4. **The resting/dimmed state** versus the touched/awake state.
