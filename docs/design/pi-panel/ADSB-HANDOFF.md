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

Measured against a FlightRadar24 screenshot of ASA783, taken 2026-08-11
10:25 local. Two independent pixel scales derived from it -- 55.0 m/px
from latitude and 52.0 m/px from longitude -- agree, which is what
establishes that the station marker is the house. The screenshot is not
kept; the numbers it produced are below and pinned by the tests. Two independent pixel scales — 55.0 m/px from latitude, 52.0 m/px from
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

## 5. FR24 — verified against the live API on 2026-08-11

Everything in this section was run against the real API, not read off a page.
The credential works. Historic access works.

> ### CORRECTIONS, 2026-08-12 — read these before trusting §5
>
> §5 was written from a partial survey. Implementation measured the following
> against the live API and the vendor documentation. **Where this block and the
> rest of §5 disagree, this block wins.** Shipped in `deploy/stopgap/`:
> `panel-fr24-poll`, `panel_loud_operators.py`, `panel_noise_log.py`,
> `panel-fr24-poll.service`, `test_panel_fr24_poll.py` (54 tests).
>
> 1. **"Cost is per RETURNED AIRCRAFT, not per query" is incomplete.** There is
>    a **flat 1-credit floor on any query returning no results** — measured
>    (5 empty polls → 5 credits) and documented verbatim at
>    `/docs/credit-overview`. This floor, not the per-aircraft rate, is what
>    prices continuous polling, because nearly every poll is empty.
>
> 2. **The identity cache "cached forever" is not permitted, and would not have
>    worked anyway.** FR24's storage rules cap retention at 30 days from
>    receipt. Separately, the API returns `reg`, `hex` **and** `callsign` as
>    null for the Portland Life Flight tails on the summary and registration
>    endpoints — a `hex -> type` cache would have had nothing to key on for
>    exactly the aircraft that matter. Only N443LF (Redmond) is named. A
>    registration-filtered poll, the obvious cheap design, silently misses the
>    target fleet.
>
> 3. **Identity is recoverable without the API.** `flight-tracks` still carries
>    the callsign, and FR24's public aircraft pages give the **Mode S address**,
>    which the aircraft broadcasts whether or not it broadcasts a category. The
>    roster in `panel_loud_operators.py` therefore identifies these helicopters
>    **locally, for free**, and keeps working if the subscription lapses.
>    N406LF (`A4C3EB`) is Portland-based and by volume the most likely overhead.
>
> 4. **The account holds 666,000 credits/month, not the documented 333,000**,
>    and they do not roll over. Aggressive polling inside a cycle is free;
>    sustained 3 s polling is not (~925k/mo).
>
> 5. **Warning time is ~34 s and geometrically fixed.** Measured from FR24's
>    first fix at liftoff to closest approach for a Providence departure
>    (`41032ec0`, N776LF: 0.95 km at 600 ft, +1088 fpm). No poll rate beats it.
>
> 6. **Quiet hours do not exist.** Across 14 days of Portland-area Life Flight
>    helicopters, **no hour of the day is ever empty** (6.6 flights/day;
>    quietest 02:00 and 05:00 at one flight each in 14 days). Gating FR24 by
>    time trades coverage for money — it is not free.
>
> 7. **A 403 from this API is usually not an auth failure.** urllib's default
>    User-Agent is rejected at the edge with an HTML body while the identical
>    request from curl returns 200. Set a User-Agent; check the content type
>    before blaming the credential.
>
> 8. **The vendor's geofence example has a bug for our use case.** Its
>    `alerted` set is never cleared, so an aircraft that leaves and returns is
>    suppressed forever — which would silence the Providence *departure*, the
>    louder half of the visit.
>
> 9. **`categories=H` was silently ignored** on a `flight-summary` query that
>    also carried `airports=`; the response contained fixed-wing traffic.
>    Filter on explicit ICAO types when the category filter matters.

### Why it matters here

The owner is emphatic, from direct experience, and the data agrees:
**Life Flight air ambulances are the majority of the noise problem at this
address**, not PDX jets. Earlier drafts of this document treated FR24 as a
range-extension nice-to-have and pushed MLAT-via-feeder first. That was wrong
and is corrected here. FR24 tracks these helicopters; that is the requirement.

A 7-minute survey of our own receiver measured **50% of aircraft broadcasting no
emitter category at all**, and the category is uselessly coarse when present
(`A3` covers a 737 MAX and an E175 alike). FR24 returns the exact ICAO type and
registration, which fixes both.

### What it actually returns

`GET /api/live/flight-positions/full?bounds=N,S,W,E` fields:

```
alt, callsign, dest_iata, dest_icao, eta, flight, fr24_id, gspeed, hex, lat,
lon, operating_as, orig_iata, orig_icao, painted_as, reg, source, squawk,
timestamp, track, type, vspeed
```

Our receiver called `AC9869` category `A1`. FR24: **type `C172`, reg `N910TW`**.

### Life Flight, confirmed

`GET /api/flight-summary/light?flight_datetime_from=..&flight_datetime_to=..&airports=KUAO&categories=H`

Aurora State (KUAO) is Life Flight Network's base. **59 helicopter flights in
7 days** (~8.4/day). Type mix matches their fleet: `A119` ×23, `B407` ×11,
`EC35` ×4, `EC45` ×2. Registration `N443LF` is confirmed Life Flight ("LF").

Pulling `GET /api/flight-tracks?flight_id=<fr24_id>` for 5 of those flights and
measuring closest approach to the station:

| flight | points | closest | altitude there |
|---|---|---|---|
| 40ff898f | 328 | **6.04 km** | 550 ft |
| 4100ef5a | 855 | **6.78 km** | 1,100 ft |
| 4100819a | 626 | 33.30 km | 600 ft |
| 41022416 | 393 | 32.99 km | 775 ft |
| 41022096 | 406 | 34.82 km | 225 ft |

Two of five came within 7 km at 550–1,100 ft. Tracks carry full lat/lon/alt/
gspeed/timestamp series — **this is the calibration set the model has never
had.** Pull a larger sample, find the ones that actually cross the house, and
fit the thresholds against them instead of guessing.

### Account, rate limits, cost — the owner decides, not you

**Cost is never a blocker here.** Report numbers, ask, move on. A previous draft
of this document declared FR24 "cannot be a continuous feed at any sane price"
and then designed around that verdict. That was wrong twice over: the owner
holds a substantial credit balance, and the decision was never the author's to
make. Do not do this again.

**Measured account usage, last 30 days** (`GET /api/usage?period=30d`):

| endpoint | requests | credits |
|---|---|---|
| `live/flight-positions/light` | 574 | 46,453 |
| `flight-summary/light` | 151 | 7,997 |
| `flight-tracks` | 142 | 5,680 |

Well past the Explorer allowance, so the account is **Essential or above** —
confirmed independently by the sibling project (below), whose config comments
name the Essential plan and whose original limits were 30/min.

There is **no balance endpoint** in the API. Usage is queryable; remaining
credits are visible only in the FR24 web dashboard. Ask the owner rather than
inferring.

Published tiers: Explorer $9/mo, 30,000 credits, 10 q/min, 30 days history ·
Essential $90/mo, 333,000 credits, 30 q/min, 2 years · Advanced $900/mo,
4,050,000 credits, 90 q/min, unlimited.

**Cost is per RETURNED AIRCRAFT, not per query.** `live/flight-positions/full`
is 8 credits per aircraft in the response; `light` is 6. So the bounding box is
as much a cost lever as the poll interval. At $0.0003/credit:

| interval | 20 km box (~5 ac) | 6 km box (~2 ac) |
|---|---|---|
| 6 s | $5,184/mo | $2,074/mo |
| 30 s | $1,037/mo | $415/mo |
| 60 s | $518/mo | $207/mo |
| 5 min | $104/mo | $41/mo |
| 30 min | $17/mo | $7/mo |

Aircraft counts are from a single 23:40 sample; daytime is higher. Levers, in
rough order of effect: poll interval, box size, `light` vs `full`, and
altitude/category filters to shrink the returned set.

A per-airframe identity cache (`hex -> type/reg/operator`) is ~8 credits per new
tail, cached forever — Life Flight's whole fleet costs about $0.02 once, and it
solves the 50%-no-category problem outright. Worth doing whatever else is
chosen. It does not preclude continuous polling; both can run.

### Prior art: `~/Source/phx-pd-helicopter-tracker`

**Read this before writing any FR24 client.** It is a working helicopter-noise
tracker by the same owner, against the same API, and it has already paid for the
operational lessons:

- `backend/app/services/fr24_official_api.py` — base URL, `Accept-Version: v1`
  headers, `live/flight-positions/light` usage.
- `backend/app/services/fr24_rate_limiter.py` — Redis-backed limiter with
  per-minute/hour/day buckets, a **10 s minimum delay between requests**, and a
  30 s backoff on 429. Its comments record that the limits were tightened from
  30/min after real rate-limit pain. I reproduced that pain in this session by
  firing eight requests in a burst and getting `429 Rate limit exceeded`.
- **Serve stale rather than nothing.** On a rate-limit block it returns cached
  data up to an hour old rather than failing. Bounds queries cache for 300 s.
  That pattern maps directly onto our "never block the publish loop" rule.
- `fr24_sdk_service.py`, `fr24_mock_service.py` — SDK wrapper and a mock for
  tests, so the test suite never touches the network.
- `download_fr24_flights.py`, `app/api/endpoints/historical_import.py` — bulk
  historical import, which is the shape of the calibration job we want.
- `FR24_MONTHLY_CREDIT_LIMIT` in `app/core/config.py` — a self-imposed monthly
  credit ceiling. Worth copying as a guard so a bug cannot drain the balance.

Also on disk and possibly relevant: `~/Source/phx-noisy-helicopters`,
`~/Source/phx-helicopter-tracker`, `~/Source/phx-pd-helicopters`.

### Practical gotchas already paid for

- **Airport codes must be ICAO for GA fields.** `airports=UAO` returns nothing;
  `airports=KUAO` works. `PDX` and `KPDX` both work.
- `categories=H` (Helicopters) cannot be used alone — combine with another
  filter. Other letters: P C M J T B G D V O N.
- `categories=H` on *historic positions* returned empty for our box: that
  endpoint is a point-in-time snapshot, so a helicopter must be airborne at that
  exact second. Use `flight-summary` for history, not positions.
- Bounds order is `north,south,west,east`.
- Headers: `Accept: application/json`, `Accept-Version: v1`,
  `Authorization: Bearer <token>`.
- **This Mac's python3 has no CA bundle** (`CERTIFICATE_VERIFY_FAILED`). `curl`
  works fine and uses the system store; the Pi is also fine. Fix properly with
  `Install Certificates.command` or `pip install certifi`.
- The Pi returned an HTML **maintenance page** with HTTP 403 at one point. That
  is not an auth failure — check the content type before concluding anything
  about the credential.
- Official SDK: `pip install fr24sdk`, `client.live.flight_positions.get_light(bounds=...)`,
  token via `FR24_API_TOKEN`. Docs carry a near-identical use case:
  `/docs/examples-and-use-cases/use-case-triggering-alerts-when-a-flight-enters-a-circular-area`.
- Useful third-party prior art: `stiles/pyfr24` (CLI, track export, lookup by
  registration), `Flightradar24/fr24api-sdk-python` (official).
- Check `/docs/storage-rules` before persisting any FR24 payload — there are
  contractual limits on retention.

### Key handling

Currently in two places. The MCP one is for interactive exploration only; the
daemon can never use MCP.

- MCP: `~/.claude.json` -> projects -> this repo -> `mcpServers.fr24api.env.FR24_API_KEY`
- Pi: `~/.fr24_key` on minidude, mode 600. For the daemon, move it to
  `sudo systemctl edit panel-adsb-status` as `Environment=FR24_API_KEY=...`.
  **Never in git.**

### Still worth pricing separately

Feeding dump1090 to FlightAware or ADSBexchange returns MLAT positions into the
local feed for free. That is a *supplement* for real-time coverage, not a
replacement for FR24's identification and history. They solve different halves
and the FR24 half is the one tied to the actual complaint.

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
