# Design prompt — "Noise Watch" aircraft-noise panel

Paste the text below into Claude Design and attach a screenshot of the current
screen. Written 2026-08-12 against the implementation in
`deploy/stopgap/noise-watch.html`.

---

## The prompt

I have a working screen and I want you to propose **at least three genuinely
different design directions** that improve it. The attached screenshot is the
current build. Please treat it as a starting point to argue with, not a template
to restyle.

### What this screen is

It is a **domestic aircraft-noise early-warning display**. It runs in two places
from one HTML file:

1. **A 7-inch bedside panel** — a Raspberry Pi driving a DSI display at
   **exactly 800×480**, in a bedroom, in kiosk mode, read at 3am. It is a fixed
   appliance: no scrolling, no keyboard, a touchscreen but rarely touched.
2. **A browser on the house LAN** — laptop or phone, where there is room for
   more detail.

It has **nothing to do with health or medical monitoring**. Its entire job is to
answer one question, ideally from across a dark room:

> **Is something about to be loud over this house, and when?**

The owner's specific motivation is warning that a noise is coming so it does not
startle him or his cat. It is not an alarm system, it controls nothing, and it
never needs to be acknowledged or dismissed.

### The real-world situation it describes

The house sits in Portland, Oregon, about **1.6 km east of a hospital helipad**
(Providence Portland Medical Center) and about **6.5 km south of the airport**
(PDX). Two things generate the complaints:

- **Life Flight air ambulance helicopters** serving that helipad. Measured from
  real flight-track data: one crossed the house at **0.95 km range, 600 ft
  altitude, climbing at over 1000 fpm** — the loudest configuration a helicopter
  has. These fly roughly **6–7 times a day**, at every hour of the day and
  night, with no quiet window.
- **Police patrol aircraft**, which *orbit* rather than pass through, so they
  are loud for far longer than a helicopter transiting overhead.

**The single most important design fact:** because the helipad is only 1.6 km
away, a departing helicopter is overhead about **34 seconds** after it first
appears in the data. That 34 seconds is set by geometry and cannot be improved
by better software. The countdown is therefore the most valuable object on the
screen, and it is often very short.

### The data available to draw

Refreshed roughly once per second. Per aircraft:

| Field | Notes |
|---|---|
| Registration / callsign / Mode S hex | Often partly missing; sometimes an aircraft is fully anonymous |
| Operator | e.g. "Life Flight Network", "Portland Police Bureau" — or **not identified** |
| Aircraft type | ICAO code (`A119`, `B407`, `EC35`, `C182`) — or absent |
| Airframe class | helicopter / jet / light aircraft / **unknown** |
| Range and bearing from the house | Metres and degrees |
| Altitude, ground speed, track, vertical rate | Vertical rate gives climb/level/descent, which dominates loudness |
| Predicted closest approach | Absolute timestamp, plus distance and altitude at that moment |
| Loudness band | `VERY LOUD` / `LOUD` / `AUDIBLE` / `FAINT` / **`UNKNOWN`** — a relative estimate, never a measurement, with no microphone anywhere in the system |
| Nearby landmarks | Hospital helipads and airports, given as **bearing and range from the house only** |

Two data sources feed it, and **their health must always be visible**: a local
radio receiver on the roof, and a commercial flight-tracking API. Either can
fail independently.

### Non-negotiable constraints

1. **Exactly 800×480** for the panel layout. Larger layouts may use the window.
2. **No web fonts, no CDN, no network assets.** The appliance is offline; only
   system font stacks and inline SVG.
3. **It lives in a dark bedroom.** The existing system's design rule is that
   *escalation is paid for in lit area* — the screen is near-black when nothing
   is happening, and brightness and lit area grow as an aircraft closes. A quiet
   sky must not light the room at 3am. Please respect this idea or replace it
   with something demonstrably better for a bedroom.
4. **Red is reserved.** A sibling medical screen uses red (`#E2453C`) for
   high-priority physiological alarm and nothing else. An aircraft must never be
   mistakeable for a medical alarm. The aircraft colour is currently cyan.
5. **Colour can never be the only channel.** Shape, position or enclosure must
   carry the same distinction, for colour-blindness and for daylight legibility
   on a dim panel.
6. **"Unknown" is a first-class state, never blank.** Roughly half of all
   aircraft broadcast no type at all, and air ambulances are over-represented
   among them. An unidentified aircraft overhead is the *most* important thing
   the screen can show — it must never render as absence, as "quiet", or as an
   empty cell. Likewise a **dead data feed must never look like an empty sky**.

### The existing colour tokens

Contrast-verified against pure black; reuse or deliberately replace them:

```
--bg        #000000   panel ground, true black
--rule      #5E6670   borders
--rule-quiet #3A4048  hairlines
--air       #3FC8D8   aircraft channel (cyan)
--watch     #E0A227   top loudness band (amber)
--nibp      #DCE1E6   primary readout (near-white)
--unknown   #8B97A3   indeterminate, deliberately hue-free
--label     #7A828C   region labels
```

### What the current design does

- A countdown as the hero, with the aircraft's identity, operator, type,
  altitude, range and flight phase beside it, plus the absolute clock time of
  closest approach and the current time (so a frozen page is obvious).
- A north-up plan view with the house at centre, range rings, labelled
  landmarks, and aircraft drawn as silhouettes — helicopter vs aeroplane —
  rotated to their heading, with a ring around police aircraft and a cross badge
  on air ambulances.
- A "watching" list of everything forecast, each with its own countdown.
- A feed-health strip along the bottom.
- When nothing is inbound, almost the entire screen is black.

### What I think is weak — argue with me

- The quiet state is *most* of the time, and it currently looks like an empty
  page rather than a considered resting state.
- The relationship between the countdown, the map and the list is not obvious;
  they read as three separate widgets rather than one instrument.
- Nothing conveys **duration** — how long the noise will last — which for an
  orbiting police aircraft matters more than the closest-approach moment.
- There is no sense of history: nothing shows what flew over earlier tonight, or
  whether this is a busy night or a quiet one.

### What I want from you

**At least three distinct directions**, not three colour variations of the same
layout. For each, please give me:

1. A one-line thesis for the direction and who it serves best.
2. The **800×480 panel layout**, in both its states: nothing inbound, and an
   aircraft 22 seconds away.
3. How the larger browser layout differs and what extra it earns.
4. What you changed about the information hierarchy and **why**, in terms of the
   34-second warning budget.
5. How "unknown" and "feed dead" look, specifically.

Vary things that actually matter: what the hero is; whether geometry belongs on
the screen at all; whether time is shown as a number, a bar, an arc or a
timeline; whether the resting state should be truly dark, an ambient status, or
something informative like a history strip. One direction may be a deliberate
risk — I would rather see a strong idea I reject than three safe ones.

Please avoid the default "dark dashboard with cards and a big stat" look. This
is an **instrument**, not a dashboard, and it competes only with a bedside clock
for attention at 3am.
