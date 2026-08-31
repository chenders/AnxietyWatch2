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

## Live vitals kiosk

`live-vitals.html` is the no-demo bedside view for an overnight record. It keeps
Polar H10 and Wellue as separate primary cards, then shows the latest durable
A&D UA-651BLE reading below them. Every card shows the source state, its last
value, and the age of that value. Missing or stale data is shown as such; the
page never fills a gap or treats a process heartbeat as a physiological sample.

On the Pi it is available at:

```text
http://pi-host:8975/live-vitals.html?kiosk=1
```

The page polls the merged `status.json` once per second. H10 is marked
`WORKING` only for a fresh physical-device status with `live: true`; Wellue is
marked `WORKING` only when its physical link, finger state, writer, and sample
are fresh. BP is displayed as the latest durable reading and retains its
`arrival_estimate` provenance.

`live-vitals-watch` is a display check, not an alarm. To run the ten-hour
check on the Pi:

```sh
sudo systemd-run --unit=live-vitals-watch --property=User=pi-user \
  --property=RuntimeMaxSec=10h --collect \
  /usr/local/libexec/live-vitals-watch --duration=36000 --interval=60
```

It logs `OK` or `DEGRADED` with named source states, page availability, and
service states; it does not restart sensors or invent values. Each check also
atomically publishes `health.json` beside the page. The kiosk's bottom
**SYSTEM HEALTH** section reads that document and shows sensor-source state,
the vitals core, spool drainer, database query path, and page/kiosk state. A
stale checker document is rendered as `WATCHER STALE` rather than being treated
as healthy.

## Removing it

```sh
sudo systemctl disable --now panel-kiosk emay-live panel-http
sudo rm /etc/systemd/system/{panel-kiosk,emay-live,panel-http}.service
sudo rm -rf /etc/systemd/system/panel-kiosk.service.d
sudo systemctl daemon-reload
sudo systemctl enable --now lightdm        # put the desktop back
sudo rm -rf /opt/anxietywatch-panel                # the checkout
```

Also revoke the deploy key in the repository's **Settings → Deploy keys**, and
delete `~anxietywatch/.ssh/anxietywatch_deploy*` plus its `~anxietywatch/.ssh/config` stanza.

Nothing else on the Pi depends on any of it. When the real path lands (T-C1..C3
for the sensor adapters, then a read endpoint on `vitalsd`), this whole directory
goes away rather than being grown into the real thing.
