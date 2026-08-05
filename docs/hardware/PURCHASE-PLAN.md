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

## Order 1 — unblocks the next tasks · ~$156

Nothing else can proceed past Stage 3 without these. T-A1 (root on NVMe) and T-A2 (audio path) are
both waiting on hardware, not code.

Prices below are what was actually in the carts on 2026-08-04, not estimates. Three vendors: Amazon
for most of it, PiShop for the RTC cell, 18650BatteryStore for the UPS cells. The NVMe and the
ethernet cable came from stock rather than being bought, which is most of why this is far below the
$403 the carts first totalled.

| Item | $ | Where | Why this one |
|---|---|---|---|
| **Geekworm X1202 UPS** | 50 | [Amazon `B0CRZ4ZXQW`](https://www.amazon.com/Geekworm-X1202-Raspberry-Shutdown-Detection/dp/B0CRZ4ZXQW) · [Geekworm](https://geekworm.com/products/x1202) | **4 cells, not 2** — see the runtime table below. Reports battery state over I2C, so the software can *see* the power situation rather than just die. Also takes 6–18 V (≥3 A) on a DC5521 barrel, where the 2-cell X1200 needs a USB-C source that genuinely negotiates 5 A |
| **Geekworm X1202-C1 case** | 19.00 | [Amazon `B0D59ZK1QR`](https://www.amazon.com/Geekworm-X1202-C1-Raspberry-Active-Cooler/dp/B0D59ZK1QR) · [Geekworm](https://geekworm.com/products/x1202-c1) | Built for exactly this stack: Pi 5 + X1202 + active cooler + a top PCIe M.2 HAT (X1000/X1001/X1003/X1004/X1015 all named). 100 × 90.8 × 62.8 mm. **Each case fits only its own UPS** — an X1200-C1 will not take an X1202 |
| **4× Samsung 35E 18650**, **flat-top, unprotected** | 23.96 | [18650BatteryStore](https://www.18650batterystore.com/products/samsung-35e-18650-3500mah-8a-battery) ($5.99 ea) | **Not protected cells.** Geekworm: *"Do not use the battery with built-in protection circuit because it will limit the discharge current"* — the X1202 does overcurrent, overvoltage and reverse-cell protection on the board. It is also why the spec caps length at 65.3 mm: a protected cell is 68–70 mm and does not physically fit. **Flat-top is correct here** — the X1202's holders are the Keystone pattern, coil spring on the negative end and a flat plate on the positive, which a flat top meets face-on (the "flat tops don't make contact" failure applies to holders with a recessed ring or a raised nub). ≤65.3 × 18.5 mm, all four from one batch. Geekworm names the 35E and the Panasonic NCR18650B. Ignore anything advertising >3600 mAh — those numbers are fiction |
| **Geekworm X1001 M.2 HAT** | 13.90 | [Amazon](https://www.amazon.com/s?k=Geekworm+X1001) · [Geekworm](https://geekworm.com/products/x1001) | Top-mounts, takes 2280. The official HAT+ is 2230/2242 only, which cuts you off from most known-good drives |
| **NVMe SSD** | **0 — from stock** | — | Already on hand; see the NVMe note below for what to verify and what it would have cost. **Avoid WD SN550/SN580/SN350/SN850 and anything Phison or MAP1202** — they are the documented Pi 5 detection and I/O-error failures. The Pi 5's PCIe is Gen2 ×1 (Gen3 ×1 with a dtparam), so any modern drive is throttled well below its rating; capacity and reliability matter here, speed does not |
| **Official Active Cooler** | 9.85 | [Amazon](https://www.amazon.com/s?k=Raspberry+Pi+5+Official+Active+Cooler) · [Raspberry Pi](https://www.raspberrypi.com/products/active-cooler/) | Top HATs are designed around its height. The CanaKit "Mega Heat Sink" is taller and probably will not fit under the X1001. Geekworm's own H505 is the other supported option and the case names both |
| **Raspberry Pi RTC battery** (Panasonic ML-2020, 45 mAh) | 5.50 | [PiShop SKU 1055-1](https://www.pishop.us/product/rtc-battery-for-raspberry-pi-5/) | **Must be rechargeable.** The Pi 5 trickle-charges J5; a primary cell like a CR2032 on a charging circuit risks rupture — and nearly every Amazon result for this search is a CR2032 holder. Without it the clock resets to 1970 on power loss, which breaks TIMESTAMPTZ chunking *and* TLS validation, so the out-of-band alerts die at the same moment everything else does |
| **UGREEN USB audio adapter** | 11.99 | [Amazon](https://www.amazon.com/s?k=UGREEN+USB+audio+adapter+TRS) | The Pi 5 has no analog output at all — the 3.5 mm jack was removed for PCIe. Right now the box physically cannot make a sound. Take the **dedicated TRS headphone jack**, not a TRRS combo: combo jacks auto-detect and switch modes on what is plugged in, which is a bad property for the one device whose job is to reliably make noise |
| **FUNLOGY 14 W powered speaker** | 21.99 | [Amazon](https://www.amazon.com/s?k=FUNLOGY+14W+USB+powered+speaker) | **USB-powered on purpose** — it draws from the Pi, which is on the UPS, so it survives an outage where a mains speaker would not. Physical volume dial, and no LEDs, which matters in a dark bedroom |
| **Cat6 ethernet cable** | **0 — from stock** | — | Removes a whole failure class and frees the shared WiFi/BT antenna. The wifi already stranded this box once |

*The RASPIAUDIO MEGA_AMP and similar are GPIO HATs, and that header is already spoken for.*

### The NVMe

**Supplied from stock — verify it before assembling.** "Samsung EVO" spans two electrically
incompatible product lines, and one of them fails in a way that looks fine until it does not
enumerate:

| | Works in the X1001? |
|---|---|
| 970 EVO / 970 EVO Plus / 980 / 980 PRO / 990 EVO / 990 PRO | **Yes** — M.2 NVMe |
| **860 EVO M.2** | **No** — an M.2 2280 stick that speaks SATA. It slots in and never appears |
| 870 EVO / 860 EVO 2.5" | No — SATA, wrong form factor entirely |

The physical tell is the notches: a SATA M.2 drive is **B+M keyed, two notches**; NVMe is **M keyed,
one notch**. Or read the model number — 970/980/990 are NVMe, 860/870 are SATA.

**What it would have cost, for whoever reads this later.** The plan originally specified a Samsung
980 500 GB at ~$45. As of 2026-08-04 that part is discontinued and remaining stock is $349 new /
$149 used; the 970 EVO Plus 500 GB is $242. The whole NVMe market is running roughly 3–5× its
historical level — the cheapest Pi-5-safe 2280 drives are the Crucial P310 500 GB at $132 and E100
1 TB at $160, both Phison-controller parts this plan otherwise avoids. At those levels a 1 TB 990
EVO Plus at $237 was the sane buy. Re-check before paying anything like that.

**Why the 4-cell UPS and not the 2-cell one** (~$30 more all-in). A real protected 18650 holds
~12.2 Wh; this box draws about 7 W with the panel lit. That gives:

| | Panel on | Panel blanked on battery |
|---|---|---|
| 2 cells (X1200) | ~3 h | ~4 h |
| **4 cells (X1202)** | **~6 h** | **~8 h** |

A night is eight hours. Four cells with the display blanked rides out a whole-night outage; two cells
do not reach it under any configuration. For a device whose entire job happens overnight, that is the
line. Vendor "up to 6 / up to 10 hours" claims are bare-Pi-idle and not comparable to each other.

**What the UPS does and does not cover.** Detection, the GPIO piezo, the out-of-band notification
path, and — because it is USB-powered off the Pi rather than mains — the speaker all ride through an
outage. The **12 V bed shaker and strobe do not**: they run from their own mains PSU. Since the
shaker is the primary alarm channel, an outage still degrades the alarm to its audible channels
unless that 12 V rail is separately backed up.

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
| Order 1 — unblocks the plan (actual carts, before shipping/tax) | 156 |
| Order 2 — the alerting chain | 75 |
| Order 3 — backup, before service | 70 |
| Order 4 — TPM | 28 |
| **Through to a klaxon in service** | **~330** |

Order 1 splits as **$126.73 Amazon** (6 items) + **$23.96 18650BatteryStore** + **$5.50 PiShop**.
Buying the NVMe instead of using one on hand would have added $237.62 at current prices.

If only one thing gets bought from Order 2, the **$25 bed shaker** is the item most likely to
determine whether the system does its job.
