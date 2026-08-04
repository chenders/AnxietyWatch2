# Stopgap — real EMAY numbers on the bedside panel, at boot

Gets a live SpO₂ and pulse onto the 7″ DSI panel **without** the storage, query
or detection paths. It wires the sensor process straight to the browser and
bypasses the architecture on purpose.

**What this is not.** There is no CNS detection here, no tier machine, no alarm,
and no storage — nothing is written down and nothing is watching the numbers.
The panel says so on every frame: the tier reads `NOT MONITORING` and the bar
along the bottom reads `DISPLAY ONLY · THIS PANEL CANNOT WAKE YOU`. Do not sleep
next to this expecting it to do anything.

It exists to answer one question early: *does the UI work on the real panel, with
real data?*

```
emay-spike  ──writes──▶  /var/lib/anxietywatch-panel/status.json
                                      │
                         python http.server on 127.0.0.1:8975
                                      │
                         cage + chromium --kiosk  ──▶  DSI panel
```

## Install

```sh
sudo apt-get install -y cage chromium-browser python3
sudo install -d -o chris -g chris /var/lib/anxietywatch-panel

# The panel itself. Re-run after every change to index.html.
scp docs/design/pi-panel/direction-a-clinical/index.html \
    chris@minidude:/var/lib/anxietywatch-panel/index.html

# The capture binary.
cargo build --release -p emay-spike           # on the Pi, or cross-build
sudo install -m755 target/release/emay-spike /usr/local/bin/

sudo cp deploy/stopgap/*.service /etc/systemd/system/
sudo systemctl daemon-reload
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
ssh chris@minidude 'python3 -c "
import json,time,sys
d=json.load(open(\"/var/lib/anxietywatch-panel/status.json\"))
age=time.time()-d[\"written_at\"]
print(d)
sys.exit(0 if age < 15 else 1)"'

# 2. The panel is being served.
ssh chris@minidude 'curl -fsS http://127.0.0.1:8975/index.html >/dev/null && echo served'

# 3. Something is actually on the screen.
ssh chris@minidude 'systemctl is-active panel-kiosk'
```

A stale `status.json` is not a failure of this stopgap — the panel is designed to
notice it and switch to `CAN'T ASSESS`, naming which of the three faults it is
(process gone, link lost, or no sample). That behaviour is the point; if the ring
comes off your finger at 2 a.m. the panel must not keep showing 97.

## Not verified from a laptop

The Rust and the page are tested. **These unit files are not** — systemd, Wayland
and the DSI panel only exist on the Pi, so the first run is the first test. Most
likely things to need adjusting:

- `chromium-browser` vs `chromium` (differs by image).
- `cage` flags, and whether it wants `WLR_LIBINPUT_NO_DEVICES=1` when no
  keyboard is attached.
- `XDG_RUNTIME_DIR` must match `id -u chris` (assumed 1000 below).
- Screen blanking. Nothing here disables DPMS; if the panel goes black after ten
  minutes that is the compositor, not the page.

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

## Removing it

```sh
sudo systemctl disable --now panel-kiosk emay-live panel-http
sudo rm /etc/systemd/system/{panel-kiosk,emay-live,panel-http}.service
sudo systemctl daemon-reload
```

Nothing else on the Pi depends on any of it. When the real path lands (T-C1..C3
for the sensor adapters, then a read endpoint on `vitalsd`), this whole directory
goes away rather than being grown into the real thing.
