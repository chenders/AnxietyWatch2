# Handoff — aircraft noise forecast, and the FR24 work that is next

Written 2026-08-11 by an agent session that ran out of runway (pi has no MCP, so
it could not use the `fr24api` MCP server). Read this before touching the ADS-B
path. Everything below was verified on the hardware, not assumed.

Read `AGENTS.md` first — it governs. `docs/specs/AMENDMENTS.md` is mandatory for
every task. Nothing here overrides either.

---

## 1. Read this before you run anything

### The radar UI exists only on the Pi. It is not in git.

| | lines |
|---|---|
| `docs/design/pi-panel/direction-a-clinical/index.html` (tracked) | 2640 |
| the same file deployed on `minidude` | 2851 |

The ~211-line difference is the entire air-traffic radar: `live-radar`,
`drawRadar()`, `showAircraftModal()`, `wireAircraftModal()`, `aircraftIcon()`.
`grep -c 'live-radar\|drawRadar'` returns **0** against the tracked file.

The README for this stopgap says "updating the panel is `git pull`". **That is
now a destructive operation.** A `git pull` in `/home/chris/anxietywatch-panel`
would revert the panel to a version with no radar at all.

Reconciling this is the highest-value cleanup available and it is unstarted.
The Pi's copy is authoritative. Suggested route: pull the Pi's `index.html` back
into the repo on a branch, diff it against tracked, and land the radar properly.

`/home/chris/anxietywatch-panel` currently reports `M .../index.html` plus a
scattering of stale `.vitals_status.*` temp files.

### Two more files live only on the Pi

- `/usr/local/bin/panel-status-merge` — hand-edited in place to widen its
  passthrough key list to `("adsb_planes", "adsb_top", "adsb_predictions",
  "adsb_advisory")`. **This edit is not in git.** `deploy/stopgap/panel-status-merge-adsb.conf`
  is only a marker recording that it happened.
- `/usr/local/bin/panel-vitals-status` — pre-existing, untracked.

### Secrets that must never be committed

- **Station coordinates** (the home address) live only in
  `/etc/systemd/system/panel-adsb-status.service.d/location.conf` on the Pi.
  The tests derive their fixture station from PDX instead, so the address is
  nowhere in the repo. Keep it that way.
- **The FR24 API key** must go the same way. See §5.

---

## 2. Current state

Branch `feat/clinical-monitor-kiosk`, worktree `/Users/chris/Source/aw2-clinical-monitor`.

```
ed243c7 style(adsb): drop the standing no-alarm disclaimer
3b0bee8 feat(adsb): forecast what is about to be loud, and when
f332d4a fix(adsb): score thrust state and stop at the runway
8f6a73b feat: forecast unarmed aircraft proximity
f7186a1 feat: publish dump1090 data to kiosk
```

Not yet pushed, no PR. Per `AGENTS.md` this must land through a reviewed PR:
worktree → branch → commit → push → PR → `/respond-to-copilot` → squash-merge.

### Files owned by this work

| path | what |
|---|---|
| `deploy/stopgap/panel-adsb-status` | the publisher daemon (Python, stdlib only) |
| `deploy/stopgap/panel-adsb-status.service` | unit; station coords come from a drop-in |
| `deploy/stopgap/panel-adsb-advisory.js` | panel rendering, loaded by `index.html` |
| `deploy/stopgap/test_panel_adsb_status.py` | 37 tests, all passing |
| `deploy/stopgap/panel-status-merge-adsb.conf` | marker for the untracked merger edit |
| `deploy/stopgap/README.md` | model, thresholds, limits, the FR24 comparison |

### Data path on `minidude`

```
RTL-SDR (Realtek RTL2838, USB 1:3)
  └─ dump1090-mutability  --write-json /run/dump1090-mutability
       └─ panel-adsb-status      reads aircraft.json, writes adsb_status.json
            └─ panel-status-merge  merges into status.json
                 └─ panel-http (127.0.0.1:8975, also 0.0.0.0)
                      └─ cage + chromium --kiosk  →  DSI panel
```

All five units active. Kiosk URL:
`http://127.0.0.1:8975/index.html?kiosk=1&live&demo`
Remote: `http://192.168.1.48:8975/index.html?kiosk=1&live&demo`

### Verify

```sh
cd deploy/stopgap && python3 -m unittest test_panel_adsb_status     # 37 tests
ssh minidude 'python3 /tmp/verify_adsb.py'                          # live feed
ssh minidude 'systemctl is-active dump1090-mutability panel-adsb-status \
    panel-status-merge panel-http panel-kiosk'
# screenshot the real panel:
ssh minidude 'sudo -u chris env XDG_RUNTIME_DIR=/run/user/1000 \
    WAYLAND_DISPLAY=wayland-0 grim /tmp/k.png' && scp minidude:/tmp/k.png /tmp/
```

---

## 3. The model, and why each piece is shaped that way

Every rule below was written against a specific observed failure. Do not relax
one without reading why it exists.

**Relative loudness, never decibels.** `noise_index` is a received level in dB
against an *unvalidated* reference. Differences rank aircraft; the absolute
number means nothing. There is no microphone, so nothing here has ever heard an
aeroplane. The panel says `ESTIMATED, NEVER MEASURED` on every frame. The radar
canvas hardcodes a `NNdB` suffix, so `panel-adsb-advisory.js` intercepts
`CanvasRenderingContext2D.fillText` to strip it.

**Thrust state dominates.** Climb vs idle is ~15 dB — larger than a 6.8 km
station-position error turned out to be. Derived from `vert_rate`:
`CLIMB ≥ +400 fpm`, `DESCENT ≤ −400 fpm`, else `LEVEL`.

**Projections terminate at the runway.** A descent aimed at a field stops at
*that field's* elevation, not ours (PDX is 31 ft, the station ~200 ft). Arrival
attribution requires the ground track to point within 35° of the field, so a
helicopter descending toward a hospital is not blamed on PDX. Rotorcraft are
exempt from termination entirely.

**Proximity is independent of loudness.** `OVERHEAD` / `NEAR` / `DISTANT` comes
from geometry alone and never needs a category. This is what lets an
unidentifiable aircraft still be reported as overhead.

**Unknown is not OK.** No emitter category or no vertical rate → `risk:
"UNKNOWN"`, `noise_index: null`, but geometry and countdown are still reported.
An unscored *overhead* aircraft is ranked **first** in the forecast list,
ahead of anything scored. An aircraft we scored and found quiet is dropped;
absence of a score is not evidence of quiet, but a score is.

**Guards name their blocker.** `"descending into PDX; below station height at
closest approach"`, never `"suppressed"`. An earlier version reported "below the
loudness threshold" at an index of 92 — that bug is what this rule prevents.

**Thresholds** (all unvalidated, `deploy/stopgap/panel-adsb-status`):
`VERY_LOUD_INDEX 88` · `LIKELY_INDEX 78` · `POSSIBLE_INDEX 68` ·
horizon 180 s · `OVERHEAD ≤ 1500 m and ≤ 5000 ft` · `NEAR ≤ 4000 m and ≤ 8000 ft`.

**Still `NOT ARMED`,** and it stays that way until a microphone validates any of
it. Do not arm anything.

---

## 4. What was measured, and what it cost

Georeferenced from `FlightAS783.jpg` (a FlightRadar24 screenshot, in the repo
root). Two independent pixel scales — 55.0 m/px from latitude, 52.0 m/px from
longitude — agree, which is what confirms the blue dot is the house.

| bug | consequence |
|---|---|
| station left on a coarse city-centre placeholder | **6.8 km** off; every range and CPA described noise over downtown. Reported CPA 10.24 km against a true 5.50 km |
| descent extrapolated below the runway | ASA783 rated `LIKELY` at a projected 0 ft while on short final at 700 ft, to a runway 6.4 km north |
| runway floor taken as `max(station, field)` | clipped the projection *above* the runway, reporting an altitude the aircraft never flies |
| thrust state ignored | ~15 dB error; ranking put a light single at 1 km above a departing narrowbody at 4 km |
| `window.live` | the page declares `live` as a top-level `const`, which binds lexically and **never** on `window`. Every advisory rendered empty |
| `str(None)` | literal `"None"` printed as a callsign |
| 5 s minimum lead time | silenced the panel exactly when an aircraft was overhead |

**Structural finding:** the house is ~6.4 km **due south of the PDX terminal**.
The 10/28 runways lie east–west along that latitude, so straight-in arrivals and
east departures never come closer than ~6.4 km. What *does* get close is traffic
cutting the southeast corner onto final — ASA783's true CPA was 5.50 km.

---

## 5. The next task: FR24 enrichment

### Why

A 7-minute survey of this receiver measured **50% of aircraft broadcasting no
emitter category at all**. Those score `UNKNOWN` and cannot be ranked. Emitter
category is also uselessly coarse when present — `A3` covers a 737 and an E175.

The owner reports, from experience, that **FlightRadar24 does show the Life
Flight air ambulances** that pass low over the house on their way to the
hospital ~1.6 km southwest. That is the loudest routine event at this address and
the whole reason the forecast exists. A single receiver cannot do MLAT, which is
very likely why FR24 sees them and we may not.

### The constraint that shapes the whole design

**`panel-adsb-status` can never use MCP.** It is a Python daemon running
unattended on a Raspberry Pi at 3am with no agent attached. MCP is an
agent-to-tool protocol.

So the MCP server is for *your* interactive exploration — learning the API
shape, checking what fields come back, confirming Life Flight is visible. The
shipped integration must be a plain HTTP client inside `panel-adsb-status`.
Use the MCP to design it; do not try to ship it.

### API facts

```
base    https://fr24api.flightradar24.com
headers Accept: application/json
        Accept-Version: v1
        Authorization: Bearer <FR24_API_KEY>
useful  GET /api/usage?period=24h                      cheap auth check + quota
        GET /api/live/flight-positions/full?bounds=N,S,W,E
```

MCP server is configured at
`~/.claude.json` → `projects` → `/Users/chris/Source/AnxietyWatch2` →
`mcpServers` → `fr24api` (`npx -y @flightradar24/fr24api-mcp`), key in
`env.FR24_API_KEY`. It is set to a real 85-char token as of this writing.

Docs the owner found useful:
- https://fr24api.flightradar24.com/docs/mcp-server
- https://www.flightradar24.com/blog/b2b/using-claude-and-the-flightradar-api-for-dummies/

**Known local snag:** this Mac's `python3` (3.13, python.org build) has no CA
bundle — a direct `urllib` call fails with `CERTIFICATE_VERIFY_FAILED`. Run
`/Applications/Python 3.13/Install Certificates.command`, or `pip install
certifi` and pass `ssl.create_default_context(cafile=certifi.where())`. The Pi
does not have this problem.

### Design requirements

1. **Cache by ICAO hex.** We poll at 1 Hz. Type, registration and operator are
   static per airframe, so look each hex up once and persist it. An uncached
   per-tick query will burn the quota in an afternoon.
2. **Fail loudly, never silently.** If FR24 is unreachable or out of quota, that
   is a *third state*, not an empty sky. Surface it on the panel the way
   `unscored` already is. Never let a network fault read as "no aircraft".
3. **Never block the publish loop.** `panel-adsb-status` must keep writing every
   second whether or not FR24 answers. Enrichment is strictly additive; a lookup
   in flight must not delay a tick.
4. **Local ADS-B stays authoritative for position.** FR24 adds identity, not
   geometry. Live trajectory from our own receiver is lower-latency and does not
   depend on the internet.
5. **Key goes in a systemd drop-in,** never in git:
   ```sh
   sudo systemctl edit panel-adsb-status
   # [Service]
   # Environment=FR24_API_KEY=...
   ```
6. **Map type → source level.** Replace the four-bucket `SOURCE_LEVELS` guess
   with a per-type table once real types arrive. ICAO/EASA noise certification
   figures (EPNdB, takeoff/approach/sideline) are public and would make the
   index defensible for the ~30 types that actually fly over Portland.
7. **Tag air ambulances explicitly.** Life Flight Network airframes are
   identifiable by registration/operator. They deserve their own class rather
   than a generic rotorcraft source level.
8. Every change gets a test. `deploy/stopgap/test_panel_adsb_status.py` is the
   pattern: pure decision functions, no network in the tests.

### A cheaper alternative worth pricing first

Feeding dump1090 to FlightAware (`piaware`) or ADSBexchange (`mlat-client`)
returns **MLAT positions into the local feed** — free, no quota, no per-lookup
latency, and it keeps working when the internet does not. That closes the
"invisible helicopter" gap directly. FR24's distinct advantage is *range*: their
network sees aircraft long before our antenna's horizon does, which is the
earlier-warning problem rather than the identification one.

Both may be worth having. They solve different halves.

---

## 6. Open requests not started

- **Hue early-warning light.** Asked for and never delivered. Design notes that
  came out of the discussion and should survive: it must be visibly distinct
  from a medical alarm (the panel's klaxon red is spoken for — use the radar's
  cyan), must capture and restore the light's prior state so it cannot leave a
  bedroom lamp on at 3am, needs a cooldown so it does not fire per aircraft, and
  must fail loudly if the bridge is unreachable. Hue's local v1 API
  (`/api/<user>/lights/<id>/state`) is sufficient and needs no cloud; v2 exists
  and v1 is deprecated but still supported on current bridges.
- **Reconcile the tracked and deployed `index.html`.** See §1. This is a
  standing hazard, not a nicety.
- **Get `panel-status-merge` into git,** with its widened key list.
- **The microphone.** The owner already has most of the parts for an outdoor
  wireless mic. Nothing in this model is validated until it exists; it is the
  only thing that can turn `noise_index` into a real prediction, and the only
  thing that could ever justify arming anything.

---

## 7. Useful scratch files on the Pi

Left in `/tmp` on `minidude`, not part of the deploy:

- `verify_adsb.py` — prints the live merged feed and asserts the advisory is unarmed
- `fake_aircraft.py` — writes a synthetic `aircraft.json` (includes a `LIFEGD1`
  air-ambulance case) so the forecast list can be rendered at a quiet hour
- `show_forecast.py` — dumps the forecast rows with countdowns
- `rotor_survey.py` — the category-coverage survey that produced the 50% figure

To render the panel against synthetic traffic: stop `panel-adsb-status`, run the
publisher one-shot with `DUMP1090_AIRCRAFT_JSON=/tmp/fake.json`, screenshot, then
`systemctl start panel-adsb-status` to restore the real feed.
