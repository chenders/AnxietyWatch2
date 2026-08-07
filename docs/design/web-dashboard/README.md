# Web dashboard — Direction A, "Clinical"

Real-data web dashboard, imported from the Claude Design project `6bfab67a`
(`AnxietyWatch - Clinical Dashboard.html`) and built against
`../PROMPT-web-dashboard.md`. `vitals-console` serves it at `/dashboard` so all
read-API calls are same-origin.

```
cargo run -p vitals-console
open http://127.0.0.1:8480/dashboard
node palette-check.mjs index.html     # the colour gate — exit 0 or 1
```

| File | What it is |
|---|---|
| `index.html` | The whole thing. Self-contained styles, renderers, and real-API view model. |
| `palette-check.mjs` | Colour gate with an exit code. Run it before changing any token. |

Four screens plus the design system, reachable from the header:

- **Now** — is it working right now? Alarm stack, numerics with printed limits,
  per-sensor liveness, risk tier, last 20 minutes.
- **Last night** — one session, four synchronised lanes on a shared x-axis, event log.
- **Trends** — 7/30/90 nights against the user's own rolling median. Never population norms.
- **System health** — will it work tonight? Daemon, alarm path, durability, coverage.
- **Design system** — tokens, within-plot separation, all component states.

**THEME** switches to the light ramp. There is no simulation toggle or demo
fallback: dead and absent feeds render as named not-measured states.

## The thing this design is for

> "No data" must never look like "everything is fine."

Three states where most dashboards have two: measured-and-fine, measured-and-bad,
and **not measured**. The third is periwinkle, hatched, glyphed `?`, valued `— —`,
and carries an age counter that keeps climbing. In a chart the line simply stops
and the gap is drawn as hatch between two dashed walls. Nothing ever interpolates
across missing data, because a continuous line there is a claim the system cannot
make.

This matches the invariant in `AGENTS.md`: *a missing signal holds state; it never
reassures and never escalates.*

## Deviations from the imported design

Each of these is a change from what the design tool produced, with the reason.

**1. The design's CVD claim was wrong, so the numbers are now computed.**
The imported design printed, on its own design-system screen, that "every pair in
the set stays ≥ 18 ΔE (CIE76) after Brettel–Viénot–Mollon simulation" and that
cyan/green was the closest pair. Measured, the worst pair is `--unknown` against
`--spo2` at **ΔE 2.9** (deuteranopia) and **12.8** unsimulated — and cyan/green is
not close to being the tightest. Both halves of the claim were false.

The fix is not to argue with a better claim. Section 1 of the design-system screen
now computes every contrast ratio from the token the browser has resolved, and
section 2 computes the CVD separations the same way, so switching theme re-derives
the whole table. `palette-check.mjs` is the same arithmetic behind an exit code.
Nothing about the palette is asserted in prose anywhere.

**2. The palette is kept anyway, because the generic rule does not apply here.**
The usual categorical-palette rule — all N hues mutually separable — is right for
N series sharing one set of axes. This design never does that: every signal owns a
lane with its own fixed scale and a printed monospace label, so identity is carried
by position and text and hue is redundant. Re-stepping the palette to satisfy a
rule for a hazard this design does not have would break a coherent system for
nothing. A sweep of ~7,500 candidate hues confirmed it is not a one-token fix
either: with those five signal hues fixed, **no** sixth colour clears the floors.

What the gate checks instead is the pairs that genuinely share a plot box. Five of
them sit below the floor and are carried entirely by non-hue channels:

| Pair | ΔE | What actually separates them |
|---|---|---|
| `--spo2` ↔ `--unknown` (dark) | 2.9 | hatch texture vs solid stroke |
| `--cpap` ↔ `--unknown` (dark) | 3.6 | hatch texture vs solid stroke |
| `--hr` ↔ `--hi` (light) | 4.2 | vertical rule vs horizontal trace |
| `--resp` ↔ `--hi` (light) | 4.4 | vertical rule vs horizontal trace |
| `--cpap` ↔ `--unknown` (light) | 4.4 | hatch texture vs solid stroke |

Those secondary channels are load-bearing, not decorative. If anyone ever
"simplifies" the gap fill to a flat tint, the SpO₂ chart stops being readable.

**3. Reserved status colours are reused as signal hues.** In the light theme
*every* status token is the same hex as a signal token — `--hi`=`--bp`,
`--med`=`--resp`, `--low`=`--spo2`, `--ok`=`--hr`; in the dark theme `--ok`=`--hr`
and `--hi` sits ΔE 2.6 from `--bp`. This is survivable only because status always
ships a glyph and the two families never share a region. The gate enumerates every
collision so that changing one hex moves exactly one line of output.

`--bp` is the one signal with no plot — blood pressure is a numeric tile only, so
its collision with `--hi` is cosmetic today. **Give BP a lane and it stops being
cosmetic.** Resolve it first.

**4. No webfont.** The design linked IBM Plex from Google Fonts. A dashboard that
needs the network to render type renders wrong at 3 a.m. on a Pi with no route
out, so the stack uses IBM Plex when the machine has it and falls back to the
platform mono/grotesque otherwise. Tabular numerics stay aligned either way, which
is the property that matters.

**5. Trend markers are stroked, not filled.** The trend plots use
`preserveAspectRatio="none"`, so a 1000-unit viewBox stretches to the card width
and compresses x roughly 2:1 against y. The design's filled `a4,4` circles came
out as invisible 2px splinters. They are now zero-length subpaths with a round
linecap and `vector-effect="non-scaling-stroke"` — the only geometry that survives
an anisotropic viewBox.

**6. Smaller fixes.** Trend units no longer repeat the measure already in the
label (`"% min SpO₂ (nadir)"` beside `NIGHTLY SpO₂ NADIR`), which was wrapping the
header onto three lines. The numeric grid's minimum went 230px → 218px so six
numerics hold one row at desktop instead of orphaning the sixth — a monitor whose
numeric block reflows is one whose values are not where the eye left them.
`aria-selected` on the nav became `aria-current="page"`, which is what a plain
button is allowed to carry.

## Wiring it up

`model()` in `index.html` is the single seam. It returns the whole view model;
the three direction renderers remain presentation-only consumers.

| Screen | Real source |
|---|---|
| Now | `/api/health`, `/api/latest`, and `/api/query` for the current 20-minute traces |
| Last night | `/api/query` over the previous completed noon-to-noon UTC window |
| Trends | `/api/nightly`, already aggregated to one point per UTC day server-side |
| System health | `/api/health`, `/api/recovery`, and `/api/clock` |

Two invariants preserved by the real-data wiring:

- **A gap must arrive as a gap.** The renderer keys off `null`, not off a
  sentinel and not off an absent key. Anything that coerces missing to a number
  defeats the entire design.
- **Trends must aggregate server-side.** Each card draws one point per night. A
  session is ~30,000 samples; shipping raw samples to compute a nightly median in
  the browser is the wrong side of the wire.

The design brief and the two directions not built here (`Night Report`,
`Terminal`) are in the Claude Design project; `../PROMPT-web-dashboard.md` is the
prompt that produced them.
