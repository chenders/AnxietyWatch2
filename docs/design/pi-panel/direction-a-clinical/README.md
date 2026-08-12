# Bedside panel — Direction A, "Clinical"

800 × 480 panel for the Hosyond 7″ DSI on the Pi 5, implemented from the Claude
Design project `b8c918e3` (`A - Clinical.dc.html` + `ClinicalPanel.dc.html`) and
built against `../../PROMPT-pi-panel.md`.

```
open index.html                     # review mode: state buttons + notes
open 'index.html?kiosk=1'           # the panel alone — how it runs on the device
open 'index.html?kiosk=1&state=klaxon'
open 'clinical-monitor.html?kiosk=1&live&demo'  # Clinical-only, no aircraft view
node panel-check.mjs                # the gate — exit 0 or 1
```

| File | What it is |
|---|---|
| `index.html` | The active kiosk screen with live inputs, experimental views, and RTL-SDR aircraft tracking. Self-contained; the 800×480 frame plus review scaffolding that `?kiosk=1` hides. |
| `clinical-monitor.html` | An alternate Clinical-only copy. It omits aircraft tracking and experimental swipe views, uses a conventional bedside-monitor layout, and retains the prominent live `NOT ARMED` safety state. |
| `panel-check.mjs` | Contrast, tier separation, layout arithmetic and touch target, behind an exit code. |

`../index.html` is the earlier hand-built panel. `PROMPT-pi-panel.md` names it as
"one solved direction to beat", so it is the **baseline and stays untouched** —
this is the challenger, not a replacement. Directions B (*Penumbra*) and C
(*Semaphore*) exist in the design project and are not built here.

## The position it takes

The brief's constraints genuinely conflict: bright enough to read instantly at
3 a.m. from three feet, dim enough to sleep beside all night. This direction
resolves it with one rule:

> **Dim by lit area, never by contrast.**

Every glyph sits at 5:1 or better against pure black, because a half-asleep eye
cannot afford a low-contrast reading. What shrinks at night is how much of the
field is emitting at all — roughly 4 % at `clear`, and about 1.5 % once the panel
rests. Escalation is paid for in area, fill and motion, never in saturation.
Nothing is large and bright at the same time until something is actually wrong.

Five states, plus resting: `clear`, `can't assess`, `watch`, `confirm`, `klaxon`.

**`can't assess` is a tier, not an error toast.** Missing data degrades the tier
and raises a *technical* alarm on a channel separate from the physiological one —
a dead oximeter is not a sick patient. It never flashes red and never silently
holds at `clear`. Four redundant cues fire together: hue drops to neutral slate,
every border turns dashed, digits become `- -`, and a `?` appears. None of it is
hue, so it survives grayscale and all three dichromacies. This is the same
invariant as `AGENTS.md`: *a missing signal holds state; it never reassures and
never escalates.*

## What checking the design found

The token table was **exactly right — every one of the twelve ratios it
states**. (Its thirteenth row is `--bg`, the ground the rest are measured
against, which carries no ratio.) That is worth
stating plainly, because the two claims that were wrong were both safety claims,
and a table this reliable is precisely the kind you stop re-deriving.

**1. "Grayscale luminance separation between the tier states is ≥ 1.9:1 in every
adjacent pair" is false in every adjacent pair.**

| Adjacent pair | Claimed | Actual |
|---|---|---|
| CLEAR → CAN'T ASSESS | ≥ 1.9:1 | 1.31:1 |
| CAN'T ASSESS → WATCH | ≥ 1.9:1 | 1.33:1 |
| WATCH → CONFIRM | ≥ 1.9:1 | **1.00:1** — the same amber |
| CONFIRM → KLAXON | ≥ 1.9:1 | 1.82:1 |

The design was right that amber and red collapse under red-green dichromacy, and
right that hue therefore must not be the only difference. It was wrong about what
catches them: luminance does not separate the tiers either. So the gate asserts
the thing that is actually true — every adjacent pair differs in **at least two**
non-hue channels (glyph, word, border style, fill, motion), and all four pairs
clear that with four. Two rather than one, so that losing a channel to a
rendering quirk or a half-open eye still leaves the tiers distinct.

**2. Amber/red collapse is worse than described, in a useful way.** They converge
in hue under both red-green dichromacies, but luminance still rescues them under
*protanopia* (2.23:1) and does not under *deuteranopia* (1.65:1) or tritanopia
(1.80:1). Deuteranopia is also the more common. The gate asserts the collapse is
real, so that a future palette change which happens to separate them reports that
this justification has gone stale instead of leaving it standing.

**3. The row heights do not "= 480 exactly".** 22 + 86 + 152 + 54 + 34 + 56 = 404,
plus five 8 px gaps = 444, plus 26 px of vertical padding = **470**. The remaining
10 px is absorbed above the silence bar by `margin-top:auto`. Harmless — but it
was an arithmetic claim nothing checked. The real requirement is that the panel is
exactly 800 × 480 and the silence bar sits flush on its bottom padding edge, which
is what makes the layout non-reflowing; `index.html` asserts the rendered geometry
at runtime and `panel-check.mjs` asserts the sums offline.

## Other deviations

**Sparklines are 132 px, not 172.** The columns are 1.12fr / 1fr / 1.12fr, so the
HR box has ~207 px of content width; a 172 px spark plus the "steady / 30 m" note
came to ~237 px and the trend line was drawn underneath the text. One width for
all three columns, sized to the narrowest, so the boxes stay identical.

**The klaxon flash is not disabled under `prefers-reduced-motion`.** It is the
alarm, not decoration, and WCAG 2.3.3 scopes that preference to non-essential
motion. At 1 Hz it is an order of magnitude under the 3 Hz photosensitive-seizure
threshold. Everything else on the panel is static.

**Review scaffolding is outside the panel.** State buttons and explanatory text
live below the 800 × 480 frame and vanish under `?kiosk=1`. The panel itself is a
fixed-size box, so nothing in the scaffolding can affect its layout.

## Hold to silence

The only touch target on the panel: 768 × 56 px, the full content width, so an
imprecise finger cannot miss it and there is nothing adjacent to hit by mistake.
Sensor and system rows are read-only — at 3 a.m. a second target is a way to press
the wrong thing.

The contract, verified in a browser rather than assumed:

| | Bar reads | Fill |
|---|---|---|
| idle | `HOLD 3 s TO SILENCE` | 0 % |
| held 1.2 s | `KEEP HOLDING — 1.8 s` | 40 % |
| **released early** | `RELEASED — RESET TO 0` | **0 %** |
| 1.6 s later | back to idle | 0 % |
| held 3 s | `AUDIO SILENCED 2:00` | 100 % |

Zero on early release, not 40 %. A progress bar that kept partial credit invites a
half-press at 3 a.m.; this one cannot be satisfied by accident, which is the only
property that matters for silencing a medical alarm. A tap does nothing at all.

Silencing stops the **audio**, for two minutes, and the panel says so. The visual
alarm never silences. At `can't assess` the bar reads `NOT SILENCEABLE — FIX THE
SENSOR`: there is no alarm to silence, and absence is not something you can
dismiss.

## Wiring it up

`STATES` in `index.html` is the only seam — each entry is one complete panel
state. Replacing them with live `vitalsd` data is the whole integration. Two
things to preserve:

- **A missing sample must arrive as `null`.** The sparkline's gap treatment keys
  off it. Anything that coerces missing to a number defeats the design.
- **Only `clear` may rest.** `armRest()` enforces it. Any other tier is either an
  alarm or an unanswered question, and both must stay lit.
