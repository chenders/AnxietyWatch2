# Hardware recommendations

**Date:** 2026-08-03
**Related:** [`HSM-RECOMMENDATIONS.md`](HSM-RECOMMENDATIONS.md) · amendment 02 (integrity) · amendment 03 (alerting)

Everything worth attaching to this system, ordered by whether it blocks progress, what it buys, and
what it costs. Prices are indicative — check before ordering.

**Nothing below the "blocking" section is required.** The software discovers what is present and
reports what it has.

---

## Already on hand

| Item | Notes |
|---|---|
| Raspberry Pi 5, 8 GB | Verified: 47 °C idle, `throttled=0x0`, 2.4 GHz |
| CanaKit case, 45 W PSU, active cooler | Both the PSU and the cooling are needed at sustained load |
| 128 GB microSD | Becomes `/boot/firmware` only once NVMe lands |
| Hosyond 7" DSI touchscreen | Verified connected, 800×480, driving under wayfire |
| Sena UD100-G03 BLE dongle | Arriving. Class 1, external antenna |
| A&D UA-651BLE BP monitor | Arriving. Standard SIG `0x1810` — no reverse engineering |
| Polar H10, EMAY SleepO2 | From v1 |

---

## 1. Blocking — the system cannot reach Stage 3 without these

### M.2 HAT + NVMe SSD — ~$45–70

PostgreSQL must not run on the microSD: continuous small writes (WAL, checkpoints, autovacuum) hit
write amplification, and SD cards fail without warning and without SMART data. For a system whose job
is catching an event overnight, silent storage death is the worst failure mode available.

Gate G2 blocks all of Group C on this. The PCIe bridge is live and `BOOT_ORDER=0xf461` already tries
NVMe, so it is physical installation only.

- Any Pi 5 M.2 HAT (official, Geekworm X1001/X1004, Pimoroni NVMe Base).
- **256 GB–1 TB is plenty.** With TimescaleDB compression at 90–95% on cold chunks, years of RR
  intervals are not a storage problem.
- Prefer a HAT with **stacking headers** — the TPM and the alerting relay board also want the GPIO
  header. See the allocation table in `HSM-RECOMMENDATIONS.md` §4.

### USB DAC + amplifier + speaker — ~$25–50

The Pi 5 has **no analog audio** — the 3.5 mm jack was removed for PCIe. Today the box has HDMI
output only and nothing plugged into HDMI, so it physically cannot make a sound. That is the one
thing this project exists to do.

Any USB audio adapter works; a powered speaker matters more than DAC quality. This is an alarm, not
a hi-fi.

**Enroll the card by name, not number.** A USB DAC that resets gets a new ALSA card index, and
`/etc/asound.conf` pointing at the old one accepts writes to nowhere. Amendment 03 §6 makes the
self-test check the enrolled identity.

---

## 2. Highest value per dollar

### UPS HAT — ~$40–60 · *not currently in any spec, and it should be*

**A mains blip at 3 a.m. kills monitoring silently.** No power, no collectors, no detection, no
alarm — and nothing to report afterwards, because the thing that would report it is also off. Every
other failure mode in this design has a detection story. This one does not.

- **[Geekworm X1200](https://geekworm.com/products/x1200)** — 2× 18650, 5.1 V 5 A, purpose-built for
  the Pi 5, pogo-pin connection, and exposes battery voltage and capacity over I2C so the software
  can *see* it. Roughly 30 minutes of runtime.
- Waveshare UPS HAT (E) is an equivalent alternative.

What the software should do with it, once fitted: treat mains loss as a **monitoring-degraded event**
that alerts immediately rather than a silent countdown, and shut down cleanly before the battery
dies rather than being cut mid-write. Thirty minutes is not enough to sleep through an outage — it
is enough to be told about one.

Check stacking clearance against the M.2 HAT before ordering; both want the same physical space.

### RTC battery — ~$5 · *cheapest real fix on this list*

The Pi 5 has a dedicated RTC connector (J5) and **ships without a battery**. Without one, the clock
resets on every power loss and the system boots believing it is some default date until NTP
corrects it.

In a time-series system where `timestamp` is the primary key and gap detection drives the alarm, a
few minutes of samples stamped with a wrong clock is a data-integrity problem, not an inconvenience.
Buy the rechargeable cell for the J5 connector.

### 12 V piezo siren + MOSFET/relay — ~$10

Bypasses the **entire audio stack** — no USB, no ALSA, no card enumeration, no DAC firmware, no
mixer state. A GPIO pin, a MOSFET, and a sounder.

It is also self-testable in a way the DAC is not: pulse the pin and sense current draw on the load.
Current flowed or it did not, which is much closer to "a sound happened" than any software
acknowledgement.

### Bed shaker — ~$20–40

The fire-safety literature on waking sleeping people reports **bed shakers with an intermittent pulse
as 100% effective at all hearing levels, from deep sleep**. Since the event being detected impairs
arousal itself, this is the single most likely thing to work when it matters, and amendment 03 makes
it the primary channel rather than the backup.

Sonic Alert, Bellman & Symfon, or a generic 12 V shaker. Drive it from GPIO through a relay — not a
smart plug, which would add a radio and a hub to the dependency chain of the most important channel.

### GPIO relay HAT + 12 V PSU — ~$20

Drives the siren, the shaker, and a strobe from one board. 2–4 channels is plenty.

> **~$65 covers siren + shaker + relay + PSU** — the largest single reliability improvement available
> to this system, and it gives two alert channels whose dependencies are disjoint from everything
> already fragile.

---

## 3. Security

### TPM 2.0 module — ~$28 delivered

**[ANAVI TPM 2.0 Vertical](https://www.crowdsupply.com/anavi-technology/anavi-tpm-2-0-for-raspberry-pi)
from Crowd Supply — $20 + $8 shipping, in stock.** Infineon SLB9672, open hardware, and it uses only
10 of the 40 GPIO pins so the alerting relay board can share the header. The vertical variant is
stated not to interfere with the Pi 5 Active Cooler.

Fallback if it goes out of stock: Infineon `IRIDIUMSLM9670TPM20TOBO1` via DigiKey (~$64).

**Do not buy a TPM from an Amazon marketplace seller.** The popular GeeekPi listing has ~20% one-star
reviews reporting TPM **1.2** parts shipped as 2.0. TPM 1.2 is a different specification — no SHA-256
PCR bank, different API, and `systemd-cryptenroll --tpm2-device` does not work with it. For a
component whose entire value is provenance, the ~$40 premium buys the supply chain.

Gives you non-extractable signing keys, a monotonic counter that makes history rewriting detectable,
and PCR-sealed LUKS unlock through the standard `systemd-cryptenroll` path.

Full detail, alternatives, setup and the Pi 5 early-boot caveat: [`HSM-RECOMMENDATIONS.md`](HSM-RECOMMENDATIONS.md).

**Order it before T-D7 if you want it at all** — keys must be generated inside the module, so any key
created beforehand is permanently a software key.

---

## 4. Alerting — optional channels

| Item | ~Cost | Buys |
|---|---|---|
| **Zigbee USB coordinator** (Sonoff ZBDongle-E, ConBee II, SLZB-06) | $25 | Hue and most Zigbee gear **locally — no bridge, no vendor cloud, works with the internet down**. Also brings smart plugs and sensors |
| 12 V strobe | $15 | The strongest visual option per the literature. Shares the relay board |
| LTE USB modem / HAT | $30+ | Makes the out-of-band voice path independent of home internet |
| Second speaker in another room | $20 | Cheap acoustic redundancy if the bedroom door is closed |

On lights specifically: for the *alarm*, go bright and cold — the opposite of the bedside panel's
warm-dim design. The panel is ambient and must not disturb sleep; the alarm exists to end it.

---

## 5. Sensors — improving the data itself

### ABPM cuff — ~$1,000+ · the one that answers the actual clinical question

The A&D UA-651BLE is **manual-trigger only**. It cannot produce a nocturnal series, and nocturnal
blood pressure is the measurement the whole exercise keeps pointing at.

A true ambulatory monitor cycles every 20–30 minutes overnight and computes nocturnal dipping.
A&D's own **TM-2441** is the obvious candidate given the `0x1810` collector already being written for
the UA-651BLE — same manufacturer, same standard service, so the collector may work unchanged.

Genuinely expensive, and clinical rather than consumer. But it is the difference between spot
readings and the pattern that matters, and it is worth pricing before assuming it is out of reach —
these turn up refurbished.

### Contactless / environmental — ~$15–60 each

| Item | Buys |
|---|---|
| mmWave presence + respiration module (LD2410, LD2450) | Respiration and presence with nothing worn. Redundant to the chest strap, and works on nights nothing is worn |
| CO₂ sensor (SCD40/SCD41) | Bedroom CO₂ correlates with sleep quality and can be genuinely high in a closed room |
| Temperature / humidity (SHT41, BME280) | Environmental context, and it protects the box: alarms on enclosure temperature |
| Barometric (BMP390) | v1 tracked barometric pressure against symptoms; a local sensor beats a weather API |

These are I2C and share the bus. Modest value individually; the mmWave module is the interesting
one because it needs nothing worn.

---

## 6. Reliability and network

| Item | ~Cost | Buys |
|---|---|---|
| **Ethernet cable** | $5 | The box is on WiFi. Wired removes an entire failure class, and frees the shared WiFi/BT antenna |
| 1 TB external drive | $60 | pgBackRest repository + archive tier. Currently the repo would share the NVMe, which makes "restore from the failed device" theatre |
| Off-box backup target (NAS, second Pi, cloud) | varies | Backup tier 2, deferred in T-C2. **Needed before Stage 5 puts the klaxon into service** |
| Spare microSD, pre-imaged | $10 | Recovery time after a card failure goes from a day to ten minutes |

The 1 TB drive and an off-box target are the same requirement seen twice: right now, all three backup
tiers would live on one disk.

---

## 7. Physical fit

**You cannot have the Pi bolted to the back of the screen *and* an under-mounting UPS *and* an
under-mounting NVMe base.** All three want the Pi's underside, and only one can have it.

### The dimensions

| | Size |
|---|---|
| Hosyond 7" DSI panel | 164.89 × 102.00 × 12.25 mm (from the listing) |
| Raspberry Pi 5 | 85 × 56 mm, mounting holes 58 × 49 mm |
| Geekworm X1200-C1 case (Pi + UPS + cooler + top M.2) | 90.8 × 69 × 62.8 mm |

### What mounts where

| Add-on | Mounts | Uses |
|---|---|---|
| Official M.2 HAT+ | **top** | PCIe FFC + GPIO. **2230/2242 only — a standard 2280 SSD does not fit** |
| Geekworm X1001 / X1004 | **top** | PCIe FFC, supports 2280 |
| Pimoroni NVMe Base | **bottom** | PCIe FFC. Frees the whole GPIO header and allows a taller cooler |
| Geekworm X1200 UPS | **bottom**, pogo pins | Does **not** use the 40-pin header, so it stacks well with top HATs |
| TPM module | **top**, GPIO SPI | Physically sits over the first ~10 pins |
| Screen standoffs | **bottom** | The Pi bolts flat to the panel |

### The resolution: do not bolt the Pi to the screen

The DSI ribbon is flexible and the screen came with one. Treat the panel as a **separate bedside
display on a ribbon** and keep the Pi in a case that can stack. That buys back the Pi's underside for
the UPS, which is the add-on with no top-mounting alternative.

Working stack, bottom to top:

```
X1200 UPS          (pogo pins, no GPIO)
  └─ Pi 5
       └─ active cooler        (fits between Pi and a top HAT)
            └─ X1001 M.2 HAT   (PCIe FFC, takes 2280)
                 └─ TPM        (GPIO — needs a riser, see below)
```

The **Geekworm X1200-C1 metal case** is built for exactly this combination — Pi 5 + X1200 + active
cooler + a top PCIe-to-M.2 HAT. If you go this route, buy the case with the UPS.

**The TPM still needs a home.** A top M.2 HAT sits over the GPIO header even when it does not
electrically use it. Options:

- A **GPIO stacking header / riser** (~$5) raising the pins above the M.2 HAT, TPM on top.
- A **GPIO ribbon breakout** taking SPI0 off to a small board mounted elsewhere in the case.
- Skip the M.2 HAT's physical footprint problem entirely by using the **Pimoroni NVMe Base
  underneath** — but that costs you the UPS, and the UPS matters more.

### Two things to measure before ordering

I could not verify these remotely:

1. **The CanaKit "Mega Heat Sink" height.** The kit ships a large heatsink plus a fan rather than the
   official Active Cooler. Top-mounting HATs are designed around the *official* cooler's height, and
   a taller aftermarket heatsink may not fit underneath one. Measure it, or plan to swap to the
   official Active Cooler (~$5).
2. **The Hosyond standoff height**, if you do decide to screen-mount. It determines whether anything
   at all fits between the Pi and the panel.

### Simplest configuration that works

If the stack is more trouble than it is worth: **Pimoroni NVMe Base underneath, screen on its ribbon,
TPM on the free GPIO header, no UPS.** Everything fits, nothing conflicts, and you give up only the
power-loss protection — which can be added later with an external 5 V UPS on the mains side instead
of a HAT.

---

## 8. What I would skip

| | Why |
|---|---|
| PoE HAT | Adds heat and cost; the PSU is fine and PoE does not survive a mains cut any better |
| Fancy DAC / hi-fi speaker | It is an alarm. Loud and reliable beats accurate |
| Second Pi for redundancy | Doubles the maintenance surface to solve a problem a UPS and a backup solve more cheaply |
| Camera module | No use here, and a camera in a bedroom is a privacy liability with no clinical payoff |
| Larger NVMe than 1 TB | Timescale compression makes it moot |
| GPS / GNSS HAT for time | NTP plus the RTC battery is sufficient; this is not a stratum-1 problem |

---

## 9. Suggested order

**Now — unblocks the plan (~$100):**
M.2 HAT + NVMe · USB DAC + speaker · RTC battery · ethernet cable

**Next — largest reliability gain (~$125):**
Piezo siren · bed shaker · relay HAT · 12 V PSU · UPS HAT

**Then — security and alert breadth (~$110):**
TPM 2.0 module · Zigbee coordinator · 12 V strobe

**When convenient:**
1 TB drive and an off-box backup target — both required before the klaxon goes into service

**Consider separately:**
ABPM cuff, if nocturnal BP is the question being asked
