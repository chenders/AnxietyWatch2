# Stopgap — real EMAY numbers on the bedside panel, at boot

Gets a live SpO₂ and pulse onto the 7″ DSI panel **without** the storage, query
or detection paths. It wires the sensor process straight to the browser and
bypasses the architecture on purpose.

**What this is not.** There is no CNS detection here, no tier machine, no alarm,
and no storage — nothing is written down and nothing is watching the numbers.
The panel says so on every frame: the tier reads `NOT ARMED` and the bar
along the bottom reads `DISPLAY ONLY · THIS PANEL CANNOT WAKE YOU`. Do not sleep
next to this expecting it to do anything.

It exists to answer one question early: *does the UI work on the real panel, with
real data?*

```
  git checkout /opt/anxietywatch-panel
        │  docs/design/pi-panel/direction-a-clinical/
        │      index.html      <- tracked; `git pull` updates the panel
        │      status.json     <- gitignored; emay-spike rewrites it every second
        ▼
  python http.server on 127.0.0.1:8975
        │
  cage + chromium --kiosk  ──▶  DSI panel
```

**Updating the panel is `git pull`.** It was previously a copy under
`/var/lib`, which meant every change needed an `scp` and went silently stale
the moment one was forgotten. Runtime data lives in the same directory on
purpose: that path is gitignored, so it never dirties the working tree and a
pull can never conflict with it. No restart is needed for a page change — the
HTTP server reads from disk per request; only a Chromium reload is required,
which `systemctl restart panel-kiosk` does.

## Install

```sh
sudo apt-get install -y cage chromium-browser python3 git
sudo useradd --system --create-home --shell /usr/sbin/nologin anxietywatch
sudo install -d -o anxietywatch -g anxietywatch /opt/anxietywatch-panel

# The panel is served straight out of a git checkout — `git pull` IS the
# deploy step. A read-only DEPLOY KEY is used rather than an account
# credential: it is scoped to this one repository, cannot push, and is
# revocable from the repo's Settings -> Deploy keys.
sudo -u anxietywatch install -d -m 700 ~anxietywatch/.ssh
sudo -u anxietywatch ssh-keygen -t ed25519 -N "" -C "pi-panel-readonly" \
    -f ~anxietywatch/.ssh/anxietywatch_deploy
sudo -u anxietywatch tee ~anxietywatch/.ssh/config >/dev/null <<'CFG'
Host github-anxietywatch
  HostName github.com
  User git
  IdentityFile ~/.ssh/anxietywatch_deploy
  IdentitiesOnly yes
CFG
sudo chmod 600 ~anxietywatch/.ssh/config
# Add ~anxietywatch/.ssh/anxietywatch_deploy.pub as a read-only deploy key, then:
sudo -u anxietywatch git clone --branch main github-anxietywatch:chenders/AnxietyWatch2.git \
    /opt/anxietywatch-panel

# The capture binary.
cargo build --release -p emay-spike           # on the Pi, or cross-build
sudo install -m755 target/release/emay-spike /usr/local/bin/

sudo cp deploy/stopgap/*.service /etc/systemd/system/

# The stopgap drain and enricher use PostgreSQL peer authentication as the
# dedicated service account. Provision only the objects they need; do not make
# the service account a database owner or superuser.
sudo -u postgres psql -d vitals <<'SQL'
DO $$ BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'anxietywatch') THEN
    CREATE ROLE anxietywatch LOGIN;
  END IF;
END $$;
GRANT CONNECT ON DATABASE vitals TO anxietywatch;
GRANT USAGE ON SCHEMA public TO anxietywatch;
GRANT SELECT, INSERT, UPDATE ON aircraft, aircraft_pass, flight_route TO anxietywatch;
-- Noise archive (sqitch changes noise_event, aircraft_track,
-- noise_analysis_helpers must be deployed first). No UPDATE: events,
-- candidates, verdicts and track points are written once, never edited.
GRANT SELECT, INSERT ON noise_event, noise_event_corroboration,
                        noise_event_candidate, noise_attribution,
                        aircraft_track_point TO anxietywatch;
GRANT SELECT ON aircraft_noise_history, noise_event_audibility,
                aircraft_track_gaps TO anxietywatch;
SQL
sudo -u anxietywatch psql -d vitals -c 'SELECT 1 FROM aircraft_pass LIMIT 0'
# Schema drift blocked all ingest for two weeks in a sibling project while a
# check command sat unused; these fail loudly if `sqitch deploy` was missed.
sudo -u anxietywatch psql -d vitals -c 'SELECT 1 FROM noise_event LIMIT 0'
sudo -u anxietywatch psql -d vitals -c 'SELECT 1 FROM noise_event_candidate LIMIT 0'
sudo -u anxietywatch psql -d vitals -c 'SELECT 1 FROM noise_attribution LIMIT 0'
sudo -u anxietywatch psql -d vitals -c 'SELECT 1 FROM aircraft_track_point LIMIT 0'

sudo systemctl daemon-reload

# cage needs the display, so the desktop session has to stop first.
sudo systemctl disable --now lightdm
sudo systemctl enable --now panel-http emay-live panel-kiosk
```

Set the ring's address once:

```sh
sudo systemctl edit emay-live      # add:  [Service]\nEnvironment=EMAY_MAC=XX:XX:...
```

## Verify

Each of these fails loudly rather than quietly doing nothing:

```sh
# 1. The capture is writing, and the reading is fresh.
ssh "$PI_HOST" 'python3 -c "
import json,time,sys
d=json.load(open(\"/opt/anxietywatch-panel/docs/design/pi-panel/direction-a-clinical/status.json\"))
age=time.time()-d[\"written_at\"]
print(d)
sys.exit(0 if age < 15 else 1)"'

# 2. The panel is being served.
ssh "$PI_HOST" 'curl -fsS http://127.0.0.1:8975/index.html >/dev/null && echo served'

# 3. Something is actually on the screen.
ssh "$PI_HOST" 'systemctl is-active panel-kiosk'
```

A stale `status.json` is not a failure of this stopgap — the panel is designed to
notice it and switch to `CAN'T ASSESS`, naming which of the three faults it is
(process gone, link lost, or no sample). That behaviour is the point; if the ring
comes off your finger at 2 a.m. the panel must not keep showing 97.

## Deployed and verified on the Pi, 2026-08-04

Running: EMAY `SleepO2 device` at 1 Hz, SpO₂ and pulse on the DSI panel, all
three units enabled and pulled in by `graphical.target`.

Three things the first deploy found, all now fixed in the units above:

**1. The desktop had to go.** The Pi was running the full Raspberry Pi OS desktop
(lightdm → wayfire), and wayfire would not honour `chromium --kiosk`: the window
came up roughly 500×45 in the top-left corner. Reproduced from a manual launch as
well as from the service, so it was chromium-under-wayfire, not systemd. `cage`
replaces it — one window, no window manager, nothing that can take the screen.
`lightdm` is now `disabled`, not removed:

```sh
sudo systemctl disable --now panel-kiosk && sudo systemctl enable --now lightdm
```

puts the desktop back exactly as it was.

**2. gnome-keyring raised a modal password prompt over the panel.** It is not a
chromium dialog, so `--noerrdialogs` never had a chance at it — a blocking modal
over a bedside monitor is precisely what that flag list exists to prevent.
`--password-store=basic` stops chromium asking for a secret store at all.

**3. `cage` needs a real logind session** to get the DRM device and the
touchscreen. `PAMName=login` + `TTYPath=/dev/tty1` + `StandardInput=tty-fail`
are what create it; without them cage exits immediately unable to open a backend.

### Headless wifi: one bad handshake used to take it down for good

Worth knowing about before running any Pi this way. Disabling `lightdm` removes
the desktop session, and with it NetworkManager's secret agent. On 2026-08-04 a
single failed 4-way handshake — while roaming between two APs on the same SSID —
was reported by wpa_supplicant as `reason=WRONG_KEY`. NetworkManager concluded
the stored PSK might be wrong, asked an agent for a new one, found none, and
gave up **permanently** rather than retrying the key on disk that was perfectly
good. The Pi sat there with the panel running happily and no network at all,
and since `cage` owns tty1 and no keyboard is attached, the only way back in
was to pull its power.

Two changes, both applied:

```sh
# Retry forever. The default of 4 attempts assumes somebody is present to
# notice — false for an appliance with no keyboard.
sudo nmcli connection modify <ssid> connection.autoconnect-retries 0
# Powersave is a common cause of exactly this handshake flakiness on Pi wifi.
sudo nmcli connection modify <ssid> 802-11-wireless.powersave 2
```

plus `panel-net-watchdog` (script + timer, in this directory): once a minute, if
the interface has **no address**, bring the connection up, and cycle the radio if
that fails. Deliberately narrow — it does not ping the internet or test DNS,
because a router reboot is not something a bedside panel should answer by
cycling its radio.

Still open:

- **Screen blanking is not configured.** Nothing here disables DPMS. If the panel
  goes black after some minutes that is cage, not the page.
- **The mouse pointer is visible** over the panel. Cosmetic on a touchscreen;
  `unclutter` would hide it.
- `XDG_RUNTIME_DIR` must match the UID assigned to the dedicated `anxietywatch` service account.

## Query string

The kiosk unit loads `?kiosk=1&live&demo`.

| | |
|---|---|
| `kiosk=1` | hides the review scaffolding — the 800×480 panel and nothing else |
| `live` | poll `status.json` once a second |
| `live=<url>` | poll somewhere else |
| `demo` | fill BP, the other three sensors and the system row with moving pseudo-data |

**Drop `&demo` to see only what is real.** With it, the top strip reads
`DEMO · SpO₂/HR REAL · REST SIMULATED` on every frame, because a fabricated
`SELF-TEST 02:00 PASS` sitting beside a true SpO₂ would borrow its credibility.
Without it those regions read `none` and `not in test`, which is the honest
picture of what this stopgap actually knows.

SpO₂ and HR are never simulated under either flag. If the ring stops reporting
they go to `- -` and the tier drops to `CAN'T ASSESS`.

## Aircraft noise forecast — `panel-adsb-status`

An RTL-SDR dongle feeds `dump1090-mutability`; `panel-adsb-status` turns
`/run/dump1090-mutability/aircraft.json` into an advisory that
`panel-status-merge` folds into `status.json`. It is display only and reads
`NOT ARMED` on every frame.

**It forecasts proximity, not sound.** Nothing here has ever heard an aeroplane.
Since 2026-08-13 `noise_index` is an estimated *outdoor A-weighted LAmax in
dBA* at the station, anchored to published certification and FAA AEDT data
rather than to the invented scale that preceded it. It carries +/-5-10 dB
absolute, so it is rendered as a band and never as a number. Ranking between
types is much tighter, roughly +/-2-3 dB. There is still no microphone, so
there is no measured indoor loudness and no evidence about what startles a cat.

Bands are set relative to an assumed 40 dBA suburban ambient
(`ADSB_AMBIENT_DBA`), not as absolute constants.

| | |
|---|---|
| horizon | 180 s — terminal-area traffic turns, and constant heading is fiction past that |
| `LIKELY` | ≥ ambient+15 dBA (55) at closest airborne approach — intrusive |
| `POSSIBLE` | ≥ ambient+5 dBA (45) — reliably detectable |
| ranking | by predicted loudness, **not** by who arrives first |
| unscorable | `UNKNOWN`, carrying no numbers — never `LOW` |

**Set the station coordinate in a drop-in, never in git:**

```sh
sudo systemctl edit panel-adsb-status   # STATION_LAT / STATION_LON / STATION_ALT
```

Without it the unit exits and says which variable is missing, rather than
starting up and quietly measuring distances from somewhere else.

```sh
cd deploy/stopgap && python3 -m unittest test_panel_adsb_status
```

### What the FlightRadar24 comparison found, 2026-08-11

Checking a live `LIKELY` against FR24's own track of ASA783 broke four
assumptions at once. Each is now a test in `test_panel_adsb_status.py`.

- **The station was 6.8 km wrong.** It had been left on a coarse city-centre
  reference, so every range, bearing and closest approach was computed for
  downtown. Reported CPA 10.24 km; true 5.50 km.
- **A landing aircraft was called a threat.** The old model extrapolated
  ASA783's −1088 fpm descent to 0 ft over the house and rated it `LIKELY`. It
  was on short final at 700 ft, and the runway is 6.4 km north. Projections now
  terminate at the runway, and a pass requires ≥ 100 ft above the station.
- **The runway floor is the field's, not ours.** PDX sits at 31 ft and this
  station near 200 ft. Flooring the descent at the station truncates the
  projection above the runway and reports an altitude the aircraft never flies.
- **Thrust state was ignored.** It is worth ~15 dB — larger than the position
  error — and `vert_rate` carries it for free. A narrowbody at takeoff thrust
  4 km away outranks a light single gliding in at 1 km; the old distance-only
  ordering had that exactly backwards.

### Known limits

- Constant heading cannot represent a turn, and arrivals are mostly turn. The
  ASA783 track flew due west, then broke onto 300° for its last 5 km.
- Source levels are published-figure intuition, not measurements, and are keyed
  on the coarse ADS-B emitter category rather than the actual type.
- No terrain, no wind, no temperature inversion, no open-window state.
- No flight-plan or route lookup, so warning time is limited to reception range.
- **No validation loop.** Until an indoor microphone records what these passes
  actually sound like, every threshold above is a guess and the advisory stays
  `NOT ARMED`.

## Flightradar24 as a second, independent source — `panel-fr24-poll`

The local receiver reports ~50% of aircraft with **no ADS-B emitter category**,
which makes them unscorable — and the aircraft that actually generate complaints
here are over-represented in that group. This daemon polls FR24 for the same
airspace so identification does not depend on the dongle hearing anything.

It is a **separate process** from `panel-adsb-status` on purpose: the publish
loop rewrites the panel once a second and must never block on HTTPS latency, a
retransmit, or a rate-limit backoff. This daemon owns the network and writes a
cache; the publisher reads the cache and never waits.

### What it is for

Life Flight Network air ambulances serving the **Providence Portland** helipad,
1.6 km west of the station. Measured 2026-08-11 from FR24 tracks (`41032ec0`,
N776LF): a departure off that pad crossed the station at **0.95 km, 600 ft,
climbing at +1088 fpm** — the loudest configuration a rotorcraft has.

**FR24 is the guarantee of detection; the roster is only the guarantee of
identification.** The Mode S roster below removes FR24 from the *identification*
path, but not from the *detection* path — it fires only for aircraft the local
receiver actually hears, and as of 2026-08-12 the receiver has never been
observed to hear one of these helicopters.

There is good evidence it should be able to: all 9,648 track points across the
20 sampled flights, including all 365 points within 10 km of the station, carry
`source: ADSB` — these aircraft broadcast 1090ES themselves, and 1–7 km at
200–1,100 ft is trivial line-of-sight. Only ~5 of 20 sorties came within 7 km,
so exposure is roughly 1–2/day and the 7-minute category survey would very
likely have caught none. That is consistent with a working receiver rather than
a broken one — but it is unverified, and until one of the six roster hexes is
observed locally, FR24 is the only source that can be relied on.

**Warning time is ~34 seconds and that is a hard ceiling.** It is the interval
between FR24's first fix at liftoff and closest approach, set by the pad being
1.6 km away. No poll rate can beat it; the poll interval only decides how much
of those 34 s is usable.

### Portland Police air support — the local receiver's job, not FR24's

The other reported noise source. Both airframes are **fixed-wing Cessna 182T**;
PPB operates no helicopters, so a rotorcraft orbiting Portland belongs to
another agency and must not be attributed here.

| Reg | Mode S | Airframe |
|---|---|---|
| N2163J | `A1D296` | Cessna 182T (2003) |
| N425EJ | `A50E9F` | Cessna 182T (2015) |

**These are invisible to `panel-fr24-poll` as configured.** It filters
`categories=H,N`, and a 182T is category `T` (General Aviation). That is a
deliberate trade, not an oversight: widening to `T` would return every light
aircraft in the box at 6 credits each, and PPB is the case the *dongle* handles
best — they orbit for long periods at 3,400–5,400 ft, comfortably inside line of
sight, so there is no 34-second warning problem to solve. The roster makes them
identifiable locally for free. If FR24 coverage of PPB is wanted anyway, a
separate registration-filtered poll (`registrations=N2163J,N425EJ`) is the cheap
route — 1 credit when they are not flying — and needs a cost decision first.

Match on **Mode S, never callsign**. The flight ID is pilot-entered, often
blank, and rotatable. N2163J is reported to fly as `IDAHO44` and the retired
N739MR as `IDAHO33` ("IDAHO" being the law-enforcement ATC series), but
N425EJ's is undocumented, so the roster does not match on any of them.

**`N739MR` is excluded by Mode S address, not just by name.** It is the 172N
PPB surplused and sold to a private owner in February 2024, and it still flies
out of Aurora — five flights in the 14-day sample, the most frequent
registration in it. Its current address is **`A9ED9D`**, confirmed both from the
registry research and directly in that sample. Excluding it by registration
alone would never have fired, because a receiver reports the address.

### Neighbouring agencies — so they are not misattributed

Several agencies fly the same Cessnas out of the same fields on the same
orbiting missions. Each is on the roster under its own operator, so a flight is
named correctly rather than blamed on Portland:

| Reg | Mode S | Aircraft | Operator |
|---|---|---|---|
| N575RW | `A76288` | Cessna R172K | Washington County Sheriff (OR) |
| N910TW | `AC9869` | Cessna R172K | Multnomah County Sheriff (OR) |
| N931EM | `ACE9D0` | Cessna 182P | Clackamas County Sheriff (OR) |
| N92PD | `ACBD1A` | **Bell OH-58C** | Clark County Sheriff (WA) |
| N2237S | `A1EF84` | Cessna T182T | Oregon State Police |
| N2181N | `A1D9C2` | Cessna 182T | State of Oregon (OSP use probable) |

`N92PD` is the **only police rotorcraft** in the area. PPB has never operated a
helicopter, so a police helicopter overhead is Clark County's.

`AC9869` matters for a second reason: the handoff records the local receiver
hearing that exact address, with FR24 resolving it to N910TW. **The dongle
demonstrably receives low-flying local law-enforcement traffic.**

Loiter, not transit, is the PPB signature: a patrol aircraft orbits, so it is
loud for far longer than a helicopter passing through, and a model tuned only to
closest approach will understate it. Orbit detection (turn accumulation over a
rolling window, positional containment, altitude stability) is not built yet.

### Finding the unknown ones by ear — `panel-heard`

The owner can recognise these aircraft by sound, but they often broadcast no
registration and their type is shared with thousands of civil Cessnas. The ear
is the only sensor that currently works, so `panel-heard` turns it into data.

```sh
panel-heard                        # "I hear it now"
panel-heard -n "low orbit, west"   # with a note
panel-heard --ago 90               # heard it 90 s ago
panel-heard --report               # rank candidates
```

The scoring is a **likelihood ratio**, not a set intersection, and the
difference is the whole point:

```
Mode S       heard     quiet   ratio  known as
A1D296      6/6       0/54      55.0  N2163J (Portland Police Bureau)
BUSY01      6/6       4/54      11.0  -
```

Both aircraft were overhead at every mark. An intersection ranks them equal and
returns the busiest aircraft in the sky as a confident answer. Scoring
`P(overhead | heard) / P(overhead | not heard)` separates them, because the
frequent flyer is also up during quiet windows.

It reports `INSUFFICIENT` until at least five marks exist, and names the blocker
with its numbers rather than offering a best guess — one coincidence must not
create a roster entry that is then trusted forever. Quiet windows are sampled
from gaps at least 30 minutes clear of any mark, since an aircraft may still be
audible either side of one.

**Mark every time you hear one**, including when you are confident you already
know which aircraft it is. The quiet windows carry as much information as the
loud ones and can only be inferred from a complete record. The tool proposes; it
never edits the roster — verify a candidate against the FAA registry first.

### Cost, measured 2026-08-12 and confirmed in the vendor docs

| | credits |
|---|---|
| query returning **no** aircraft | **1** (flat floor) |
| `live/flight-positions/light` | 6 per returned aircraft |
| `live/flight-positions/full` | 8 per returned aircraft |
| `flight-tracks` | 40 per flight (~$0.012) |

The 1-credit floor is what prices continuous polling: at 3 s almost every poll
is empty, so the bill is ~1 credit per poll regardless of filtering. Filtering
still matters — it keeps the *non*-empty polls small.

The account carries **666,000 credits/month** (double the documented Essential
allowance) and they **do not roll over** — unused credits expire on the billing
date. Approximate monthly cost:

| interval | credits/mo | fits 666k? |
|---|---|---|
| 3 s | ~925k | no — ~$70/mo top-up |
| 5 s | ~555k | yes, with headroom |
| 12 s | ~215k | comfortably |

Default is **3 s for the prototype**, matching FR24's own ~3 s per-aircraft
refresh so no update is aliased away. At 92 kt a 5 s interval carries up to
235 m of position error — a fifth of the whole closest-approach distance.
**5 s is the sustainable steady state.** `FR24_CREDIT_CEILING` is a hard
per-cycle stop so a bug cannot drain the balance.

### Identification without spending anything

`panel_loud_operators.py` maps **Mode S address → airframe**. The addresses were
collected from FR24's public aircraft pages and cross-checked against the API
(which independently returned `A556C2` for N443LF). Because the Mode S address
is broadcast whether or not a category is, this makes identification free,
local, and independent of the API entirely:

| tail | type | Mode S | note |
|---|---|---|---|
| N406LF | B407 | `A4C3EB` | Portland-based (HIO/PDX); most likely overhead |
| N776LF | A119 | `AA8051` | the measured 0.95 km Providence departure |
| N826LF | EC35 | `AB4825` | |
| N843LF | EC45 | `AB8BFE` | |
| N458LF | B407 | `A590D4` | |
| N443LF | B407 | `A556C2` | Redmond-based; the one tail the API will name |

FR24 returns `reg`, `hex` and `callsign` as **null** for the Portland tails on
the summary and registration endpoints, so the roster is what recovers the
registration — and therefore what makes the flight-playback link constructible.

### The loud-aircraft log

`panel_noise_log.py` appends one row per **pass** (not per poll) to
`/var/lib/panel-noise/loud-aircraft.jsonl`, for the admin console. Each row
carries identity, geometry, phase, provenance and deep links back to FR24
(`/data/aircraft/<reg>#<fr24_id>`).

Two rules worth knowing before changing it:

- **Retention is 6 months for FR24-sourced rows, by owner decision.** FR24's
  terms specify 30 days. The exposure is confined to one constant; the module
  docstring records the decision so it is not mistaken for an oversight. Rows
  from the local receiver carry no expiry — no licence governs our own antenna.
- **A returning aircraft gets a second row.** FR24's own geofence example keeps
  an `alerted` set it never clears, which would permanently suppress the
  Providence *departure* — the louder half of the visit.

An unscored aircraft is never logged as quiet: `noise_index` stays `null` and
the band stays `UNKNOWN`. Absence of a score is not evidence of quiet.

### Install and verify

```sh
sudo install -m 0755 panel-fr24-poll /usr/local/bin/
sudo install -m 0644 panel_loud_operators.py panel_noise_log.py /usr/local/bin/
sudo install -m 0644 panel-fr24-poll.service /etc/systemd/system/

# Key AND station coordinates are secrets. Neither may ever be committed.
sudo systemctl edit panel-fr24-poll     # FR24_API_KEY, STATION_LAT, STATION_LON
sudo systemctl enable --now panel-fr24-poll

# 54 tests, no network.
cd deploy/stopgap && python3 -m unittest test_panel_fr24_poll
# Feed must read FRESH, and `aircraft` must be null whenever it is not.
jq '{feed_state, error, credits, n: (.aircraft|length?)}' /run/panel-fr24/fr24_aircraft.json
```

### Gotchas already paid for

- **A 403 here is usually not an auth failure.** urllib's default User-Agent is
  rejected at the edge with an HTML body while the identical request from curl
  returns 200. `USER_AGENT` must stay set. The error path checks content type
  and says so rather than blaming the credential.
- Bounds order is `north,south,west,east`, quantised to 3 dp — and rounded
  **outward**, because rounding to nearest silently clips ~55 m off the watched
  circle.
- `categories=H` alone is documented as needing a companion filter; `bounds`
  satisfies that. `N` (non-categorised) is included deliberately — excluding it
  would rebuild the exact blind spot this daemon exists to close.
- No push/webhook exists; polling is the only option. `/api/usage` is capped at
  **1 call per minute**, so it cannot gate the loop — hence the local ledger.
- There is **no balance endpoint**. `/api/balance`, `/credits`, `/subscription`,
  `/account` and `/me` all 404. Balance is visible only in the web dashboard.

## Aircraft noise events — capture, attribution, history

The question this answers: **which aircraft caused the noise the resident (or
the cat) just marked**, per event, from physics — not from repeated sightings
of a tail. Exact identity is supporting history; the goal stays general
warning heuristics.

### Real evidence has two entry paths

**The bedroom-door Hue button** (`panel-hue-aircraft-label`), deliberately
unbound from Hue room/scene behavior and consumed from the bridge's local v2
event stream:

- one press = the resident heard and noticed an aircraft;
- two presses within 1.35 s = the cat heard it and was frightened — recorded
  as BOTH outcomes (`human_heard` + `cat_scared_top_priority`), because a
  double press encompasses a human-heard episode. Cat is top priority.

The parser accepts every button_report and logs each one: a press whose
`initial_press` is lost in a stream-reconnect blind window (this bridge sends
no keepalives, so the stream times out every ~90 s; one press was really lost
this way on 2026-08-17) is **recovered from its unpaired release** rather
than silently discarded. The state file carries `stream_last_activity_at`, so
the admin console can tell a live stream from a half-open socket that still
claims "connected". **A replacement button gets a new resource id** — update
`HUE_AIRCRAFT_BUTTON_ID` in the drop-in or every press is silently filtered.

**Operator reports** (`panel-aircraft-report`): the owner also reports events
in words, with timestamp metadata of their own. `--ago/--at/--iso` record a
REAL, attribution- and training-eligible event (a naive ISO timestamp is
refused, never guessed); `--corroborates EVENT_ID` preserves the words and
timing against an existing bridge event **without ever creating a second
event**. See `recovered-operator-reports-2026-08-17.md` for the backfill of
this session's chat reports.

Every real event is fsynced to `/var/lib/panel-noise/aircraft-labels.jsonl`
with the event time, FULL feed snapshots (entire ADS-B status and FR24 cache
documents — position data is the canonical hard-to-recover-later record),
recent completed passes, and the cached METAR (`panel-weather-poll`:
temperature, wind, gusts — the acoustic weather; the press path only ever
reads the cache, and a dead poller means "weather: not measured", never a
blocked press).

### Per-event attribution — `panel-aircraft-attribute`

Runs after every real event (and via the report tool). For each event it
ranks every aircraft either feed could see — live candidates, FR24's
complementary picture, pass-log rows within ±180 s — on general physical
features only: modelled closest-approach dBA, ground distance, timing
corrected for slant-path sound propagation, airframe class from the audited
type table, and thrust phase. **CONFIRMED needs a physical-evidence floor and
a margin over the runner-up; anything less stays AMBIGUOUS or UNATTRIBUTED
with the blocking values named.** A false assignment would poison the
history, so ambiguity is a first-class verdict, not a failure. Synthetic
never enters. Output (`aircraft-attributions.json`) carries every candidate
verbatim plus per-airframe and per-flight history — and never edits a roster
or threshold.

The old `panel-heard` five-mark recurring-tail survey is **not part of this
pipeline and is not authoritative for it**; it remains only as a standalone
legacy tool for the police-tail identification problem it was built for.

### The archive is PostgreSQL; files are the write-ahead record

`sqitch deploy` (changes `noise_event`, `aircraft_track`,
`noise_analysis_helpers`) creates: `noise_event` (+corroborations) with full
JSONB evidence and weather; `noise_event_candidate` — every aircraft
considered, all data, FR24-only rows carrying the owner's 182-day retention
deadline; `noise_attribution` — verdict HISTORY, one row per changed verdict
per event, each naming its method; the `aircraft_noise_history` view; and the
`aircraft_track_point` hypertable — dense positions from both feeds (1 s
inside 4 km, 5 s to 15 km) so a pass can be REPLAYED locally without vendor
playback. `panel-noise-drain` moves all spools into Postgres idempotently and
names remaining disk space every run. Raw FR24 poll responses are **not**
retained anywhere — the pass log, candidate rows and track archive are the
retention mechanism, by design.

`panel-fr24-track-fetch` (timer) archives the definitive FR24 track for every
CONFIRMED flight with an fr24_id — once, at ~40 credits — with
attempt-bookkeeping so a broken flight id is retried at most 3 times, then
skipped with its stored error named. Free when nothing new is confirmed.

### History is visible everywhere

`panel-adsb-status` attaches `noise_history` (distinct human/cat counts,
first/last timestamps, event ids) to every published aircraft. The
noise-watch (browser and kiosk) draws distinct marks — cat head in amber (top
priority), human figure in cyan — on map markers, the hero, the queue and
the detail table, **including for aircraft that are currently quiet**: the
mark means "this airframe has woken this house before". The admin console
adds paginated recent-evidence and known-airframe lists, plus a WARNINGS queue
where each fired or explicitly suppressed prediction can be marked accurate,
inaccurate, or unknown. Known airframes retain FR24 deep links. Unknown means the operator cannot reliably determine either outcome
and contributes nothing to training. Unreviewed warnings leave the queue after
three hours and receive an append-only `expired_unreviewed` database lifecycle
row; the warning evidence itself is never deleted.
Every list defaults to 10 rows (10/25/100 choices); page and size live in the
URL without a reload. Contextual help opens a full explanation. `/` is
read-only status/history; settings and all save controls live separately at
`/settings`. Both routes provide three live presentations over the same state
and action layer: Operations Deck, Daily Briefing and Map Lens. The `view`
query parameter permits a direct link; otherwise `noiseWatchAdminDesign` in
browser local storage restores the last selected design, with Operations Deck
as the fallback. Changing designs never changes or duplicates evidence. Map
Lens plots only measured receiver bearing/distance inside its labelled radius;
missing or out-of-range geometry is not invented or clamped. Both pages
preserve their dark palette and status colors in Print / Save as PDF.

The bottom of Settings contains the persistent global Hue alarm control. A
disable requires typing `DISABLE ALARMS`; re-enabling is available in the same
location. The operation writes only the existing persisted overrides plus
`hue_enabled`, so it cannot reset unrelated choices. Unsaved form edits block
the global action by name rather than being discarded. While disabled, both
admin routes show a red alert banner and the kiosk shows a compact crossed
alarm icon with `ALARMS OFF`. The kiosk retains the last explicit disabled
state through a transient `hue_status.json` read failure instead of falsely
looking armed.

### Install

Install after deleting the button's old Hue behavior in the Hue app or API,
and after `sqitch deploy` + the grants above:

```sh
sudo install -m 0755 panel-hue-aircraft-label panel-aircraft-attribute \
  panel-aircraft-report panel-aircraft-learn panel-aircraft-inject \
  panel-warning-backfill panel-weather-poll panel-fr24-track-fetch panel-heard \
  /usr/local/bin/
sudo install -m 0644 panel_candidate_id.py /usr/local/bin/
sudo install -m 0644 panel-hue-aircraft-label.service \
  panel-aircraft-learn.service panel-aircraft-learn.timer \
  panel-weather-poll.service panel-weather-poll.timer \
  panel-fr24-track-fetch.service panel-fr24-track-fetch.timer /etc/systemd/system/
# Export the bridge's certificate once over a trusted local path, verify its
# fingerprint against the bridge, then install it root-owned:
sudo install -d -o root -g root -m 0755 /etc/panel-hue
sudo install -o root -g root -m 0644 bridge.pem /etc/panel-hue/bridge.pem
sudo systemctl edit panel-hue-aircraft-label  # HUE_BRIDGE, HUE_USERNAME, HUE_AIRCRAFT_BUTTON_ID, HUE_CERTIFICATE
sudo systemctl edit panel-fr24-track-fetch    # FR24_API_KEY
sudo systemctl daemon-reload
sudo systemctl enable --now panel-hue-aircraft-label panel-aircraft-learn.timer \
  panel-weather-poll.timer panel-fr24-track-fetch.timer
sudo systemctl restart panel-adsb-status panel-admin   # pick up history + track capture
```

Use the explicit injector for plumbing tests. It has no option capable of
creating real or training-eligible evidence:

```sh
panel-aircraft-inject --presses 1 --note "operator test of single-press path"
panel-aircraft-inject --presses 2 --note "operator test of cat-label path"
```

### The nightly learner

`panel-aircraft-learn` joins eligible real labels (button AND operator
reports) to completed passes, builds one deterministic quiet control per
ten-minute bucket **outside ±300 s of EVERY eligible real label across
outcomes** — a moment the human marked is not "quiet" for the cat model,
an aircraft was audibly there — and uses chronological 60/20/20 partitions.
Human and cat outcomes keep separate evidence and performance gates (a cat
event implies the human heard it; never the reverse). Output is append-only
`aircraft-model-proposals.jsonl`; every row says `review_required`, and the
tool never edits `settings.json`. Reviewed warnings with their exact saved
prediction score become explicit positive (`accurate`) or negative
(`inaccurate`) examples for the human model. Journal-recovered warnings lack
the original exact score; their reviews remain durable and visible but are
excluded from training rather than substituting an approximate pass score.

```sh
panel-aircraft-learn --dry-run       # inspect without appending
panel-aircraft-learn                 # append one reviewable proposal
```

The button's battery state is health information, not an ingestion gate. A
connected button at a reported low battery continues to produce real labels;
monitor it and replace the battery, but do not silently discard its events.

### Verify

```sh
cd deploy/stopgap && python3 -m unittest \
  test_panel_hue_aircraft_label test_panel_aircraft_attribute \
  test_panel_aircraft_learn test_panel_aircraft_inject \
  test_panel_aircraft_report test_panel_weather_poll \
  test_panel_fr24_track_fetch test_panel_noise_drain \
  test_panel_adsb_status test_panel_admin test_panel_warning_backfill
ssh "$PI_HOST" 'systemctl is-enabled panel-hue-aircraft-label panel-aircraft-learn.timer panel-weather-poll.timer panel-fr24-track-fetch.timer && systemctl is-active panel-hue-aircraft-label && python3 -m py_compile /usr/local/bin/panel-hue-aircraft-label /usr/local/bin/panel-aircraft-attribute /usr/local/bin/panel-aircraft-report /usr/local/bin/panel-aircraft-learn /usr/local/bin/panel-aircraft-inject /usr/local/bin/panel-weather-poll /usr/local/bin/panel-fr24-track-fetch'
# The archive accepts writes and the views exist (fails loudly if sqitch was missed):
ssh "$PI_HOST" 'sudo -u anxietywatch psql -d vitals -c "SELECT 1 FROM noise_event LIMIT 0" -c "SELECT 1 FROM aircraft_track_point LIMIT 0" -c "SELECT 1 FROM noise_event_audibility LIMIT 0"'
```

## Removing it

```sh
sudo systemctl disable --now panel-kiosk emay-live panel-http panel-adsb-status panel-fr24-poll panel-hue-aircraft-label panel-aircraft-learn.timer panel-weather-poll.timer panel-fr24-track-fetch.timer
sudo rm /etc/systemd/system/{panel-kiosk,emay-live,panel-http,panel-adsb-status,panel-fr24-poll,panel-hue-aircraft-label,panel-aircraft-learn,panel-weather-poll,panel-fr24-track-fetch}.service
sudo rm /etc/systemd/system/{panel-aircraft-learn,panel-weather-poll,panel-fr24-track-fetch}.timer
sudo rm -rf /etc/systemd/system/panel-kiosk.service.d \
  /etc/systemd/system/panel-hue-aircraft-label.service.d
# Remove the pinned certificate only after confirming no other Hue client uses it.
sudo rm -rf /etc/panel-hue
# Labels and state are evidence and are retained by default. To erase them
# deliberately, archive as required, then remove /var/lib/panel-noise separately.
sudo systemctl daemon-reload
sudo systemctl enable --now lightdm        # put the desktop back
sudo rm -rf /opt/anxietywatch-panel                # the checkout
```

Also revoke the deploy key in the repository's **Settings → Deploy keys**, and
delete `~anxietywatch/.ssh/anxietywatch_deploy*` plus its `~anxietywatch/.ssh/config` stanza.

Nothing else on the Pi depends on any of it. When the real path lands (T-C1..C3
for the sensor adapters, then a read endpoint on `vitalsd`), this whole directory
goes away rather than being grown into the real thing.
