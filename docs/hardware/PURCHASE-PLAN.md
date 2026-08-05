# What to buy, in the order you need it

**Date:** 2026-08-04
**Source:** [`HARDWARE_RECOMMENDATIONS.md`](HARDWARE_RECOMMENDATIONS.md) — the reasoning behind every
choice here. This document is only the shopping list.

Decided: **the Pi goes in a case with the UPS underneath, and the 7" panel hangs off its DSI ribbon.**
That settles the whole physical-fit question — top-mounted M.2 HAT, standard 2280 SSDs, UPS keeps the
underside.

Prices are indicative. Every link is a manufacturer or a first-party storefront where one exists,
because three items on this list have known counterfeit or mis-shipped-part problems.

---

## Order 1 — unblocks the next tasks · ~$150

Nothing else can proceed past Stage 3 without these. T-A1 (root on NVMe) and T-A2 (audio path) are
both waiting on hardware, not code.

| Item | ~$ | Where | Why this one |
|---|---|---|---|
| **Geekworm X1200 UPS + X1200-C1 case** | 55 | [Amazon (bundle)](https://www.amazon.com/Geekworm-X1200-C1-Raspberry-Shutdown-Detection/dp/B0D7D4B5S7) · [Geekworm](https://geekworm.com/products/x1200) | The case is built for exactly this stack: Pi 5 + X1200 + cooler + a top PCIe M.2 HAT. Reports battery state over I2C, so the software can *see* the power situation rather than just die |
| **2× 18650 cells**, button-top, protected | 15 | any reputable cell seller | The X1200 ships empty. **Flat-top cells do not make contact** |
| **Geekworm X1001 M.2 HAT** | 17 | [Geekworm](https://geekworm.com/products/x1001) | Top-mounts, takes 2280. The official HAT+ is 2230/2242 only, which cuts you off from most known-good drives |
| **Samsung 980 500 GB** (MZ-V8V500B) | 45 | [Amazon](https://www.amazon.com/dp/B08V83JZH4) | 2280, TLC, on every Pi 5 compatibility list. **Avoid WD SN550/SN580/SN350/SN850 and anything Phison or MAP1202** — they are the documented Pi 5 detection and I/O-error failures |
| **Official Active Cooler** | 5 | [Raspberry Pi](https://www.raspberrypi.com/products/active-cooler/) | Top HATs are designed around its height. The CanaKit "Mega Heat Sink" is taller and probably will not fit under the X1001 |
| **Raspberry Pi RTC battery (ML2020 + JST-SH)** | 5 | search `Raspberry Pi RTC battery` or `ML2020 JST-SH` | **Must be rechargeable.** The Pi 5 trickle-charges J5; a primary cell like a CR2032 on a charging circuit risks rupture. Without it the clock resets to 1970 on power loss, which breaks TIMESTAMPTZ chunking *and* TLS validation — so the out-of-band alerts die at the same moment everything else does |
| **USB audio adapter** | 10 | [PiShop](https://www.pishop.us/product/usb-audio-adapter-works-with-raspberry-pi/) | The Pi 5 has no analog output at all — the 3.5 mm jack was removed for PCIe. Right now the box physically cannot make a sound |
| **Powered speaker**, any | 15–25 | anywhere | It is an alarm. Loud and reliable beats accurate |
| **Ethernet cable** | 5 | anywhere | Removes a whole failure class and frees the shared WiFi/BT antenna. The wifi already stranded this box once |

*Skip the RASPIAUDIO MEGA_AMP and similar — they are GPIO HATs, and that header is already spoken
for.*

---

## Order 2 — the alerting chain · ~$75

This is what turns a monitor into an alarm. Largest reliability gain per dollar on the list, and the
two channels have **disjoint dependencies** from the audio stack.

| Item | ~$ | Where | Why this one |
|---|---|---|---|
| **Sonic Alert Super Shaker, 12 V** | 25 | [Amazon](https://www.amazon.com/Super-Shaker-12v-White-Vibrator/dp/B005GGQX7C) | **The primary channel**, not the backup. Fire-safety research finds bed shakers with an intermittent pulse 100% effective at all hearing levels from deep sleep — and the event being detected impairs arousal itself |
| **Tokatuker B604 piezo × 2** | 20 | [Amazon](https://www.amazon.com/dp/B079L2KBG7) | Self-driving, 2–12 V, ~3.8 kHz. Bypasses USB, ALSA, card enumeration and DAC firmware entirely. Buy two — reliability reviews are mixed and it is $10 |
| **IRLZ44N MOSFETs**, genuine Infineon, ×5 | 9 | any parts seller listing V<sub>GS(th)</sub> 1.0–2.0 V | **Not** the common "IRF520 driver module" — IRF520 needs ~10 V of gate drive, so from a 3.3 V GPIO it conducts partially, runs hot and degrades silently. The `L` in IRLZ is the logic-level marker |
| **Resistors: 150–220 Ω and 10 kΩ** | 5 | any parts seller | One of each per channel. The **10 kΩ gate-to-source pulldown is not optional** — Pi GPIO floats from power-on until configured, and without it the siren sounds at every boot |
| **1N4007 diodes** | 3 | any parts seller | Flyback across the shaker. It is a motor; switching it without one puts a reverse spike across the MOSFET |
| **Screw-terminal proto board** | 8 | anywhere | If soldering is unappealing, Adafruit and Pololu sell single-channel logic-level MOSFET breakouts with the resistor and pulldown already fitted |
| **12 V 2 A PSU** | 10 | anywhere | Drives shaker and siren |
| **INA219 current sensor** (optional) | 5 | anywhere | I²C, inline with the siren. Turns "we drove the pin" into "current flowed" — the closest thing to physical proof a sound happened |

**Pin allocation, already checked clear of the TPM** (which occupies physical 17–26):

| Load | BCM | Physical |
|---|---|---|
| Piezo siren | GPIO5 | 29 |
| Bed shaker | GPIO6 | 31 |
| Strobe | GPIO13 | 33 |

---

## Order 3 — before the klaxon goes into service · ~$70

Right now all three backup tiers would live on the same disk, which makes "restore from the failed
device" theatre.

| Item | ~$ | Where |
|---|---|---|
| **1 TB external drive** | 60 | anywhere — pgBackRest repository + archive tier |
| **Spare microSD, pre-imaged** | 10 | anywhere — turns a card failure from a day into ten minutes |
| Off-box backup target (NAS, second Pi, cloud) | varies | Tier 2, deferred in T-C2 |

---

## Order 4 — security · ~$28

| Item | ~$ | Where | Why this one |
|---|---|---|---|
| **ANAVI TPM 2.0 Vertical** | 20 + 8 ship | [Crowd Supply](https://www.crowdsupply.com/anavi-technology/anavi-tpm-2-0-for-raspberry-pi) | Infineon SLB9672, open hardware, uses 10 of 40 pins so the alerting board shares the header. Stated not to foul the Active Cooler |

**Do not buy a TPM on Amazon.** The popular GeeekPi listing has ~20% one-star reviews reporting
TPM **1.2** parts shipped as 2.0 — a different specification, no SHA-256 PCR bank, and
`systemd-cryptenroll --tpm2-device` does not work with it. For a part whose entire value is
provenance, the premium buys the supply chain.

**Timing matters:** order it before T-D7 if you want it at all. Keys must be generated *inside* the
module, so anything created beforehand is permanently a software key.

---

## Later — worth it, not urgent

| Item | ~$ | Buys |
|---|---|---|
| **Seeed MR60BHA2** 60 GHz mmWave | 25 | The only contactless option that actually measures **respiration**. Buy from the [Seeed Studio official Amazon storefront](https://www.amazon.com/stores/SeeedStudio) — not The Pi Hut (VAT you do not owe), and not any "Generic" $89 listing, which is the discontinued BHA1 at 4× price. Range for breathing is ~2 m, so it mounts above or beside the bed, and it is specified for one person |
| Zigbee coordinator (Sonoff ZBDongle-E) | 25 | Hue and most Zigbee gear locally — no bridge, no vendor cloud, works with the internet down |
| 12 V strobe | 15 | Strongest visual channel. Shares the MOSFET board. Go bright and cold — the opposite of the panel's warm-dim design |
| LTE USB modem | 30+ | Makes the out-of-band voice path independent of home internet |
| CO₂ (SCD41), temp/humidity (SHT41) | 15–30 | I²C, share the bus. Environmental context |

### The expensive one, priced separately

**A&D TM-2441 ABPM cuff — $1,000+.** The UA-651BLE is manual-trigger only; it cannot produce a
nocturnal series, and nocturnal blood pressure is the measurement this whole project keeps pointing
at. Same manufacturer and same `0x1810` service as the cuff already being written for, so the
collector may work unchanged. These turn up refurbished — worth pricing before assuming it is out of
reach.

---

## Not worth buying

PoE HAT · hi-fi DAC · second Pi for redundancy · camera module · NVMe larger than 1 TB · GNSS HAT for
time. Reasons in [`HARDWARE_RECOMMENDATIONS.md`](HARDWARE_RECOMMENDATIONS.md) §8.

---

## Running total

| | ~$ |
|---|---|
| Order 1 — unblocks the plan | 150 |
| Order 2 — the alerting chain | 75 |
| Order 3 — backup, before service | 70 |
| Order 4 — TPM | 28 |
| **Through to a klaxon in service** | **~325** |

If only one thing gets bought from Order 2, the **$25 bed shaker** is the item most likely to
determine whether the system does its job.
