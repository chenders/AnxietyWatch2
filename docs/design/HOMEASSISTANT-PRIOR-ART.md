# Home Assistant prior art — what to take

**Date:** 2026-08-05 · **Status:** research findings; the normative subset is folded into
[`SPEC-admin-console.md`](SPEC-admin-console.md) §8b
**Companion:** [`HOMEASSISTANT-ARCHITECTURE.md`](HOMEASSISTANT-ARCHITECTURE.md) — how HA is built
(integration model, entity model, coordinator pattern, `unavailable` vs `unknown`, the Bluetooth
stack) and what of that transfers.

Verdicts: **PORT** (take the code) · **REIMPLEMENT** (take the protocol knowledge) ·
**USE-AS-IS** (add as a dependency) · **IGNORE**.

---

## 1. The headline: HealthKit push already exists, Apache-2.0, eight days old

**Home Assistant's iOS companion app shipped HealthKit push on 2026-07-28.** This is the single most
valuable thing in the survey and it solves a problem this project has not started.

- [iOS PR #4923](https://github.com/home-assistant/iOS/pull/4923) — HealthKit sensors
- [iOS PR #5291](https://github.com/home-assistant/iOS/pull/5291) — background delivery
- Source: `Sources/Shared/API/Webhook/Sensors/Health/`, **Apache-2.0**

**23 metrics, including exactly the set this project cares about** — `oxygen_saturation`,
`respiratory_rate`, `heart_rate_variability` (SDNN), `resting_heart_rate`, `heart_rate`,
`blood_pressure_systolic`/`_diastolic`, `body_temperature`.

**The mechanism is the part worth taking:** `HKObserverQuery` + `enableBackgroundDelivery(for:
frequency: .immediate)`, with a 5 s signal debounce and a 30 s minimum interval. Opt-in per metric,
read authorization requested only for the enabled subset, never writes.

**Sleep is deliberately excluded**, and their comment says why: HealthKit models it as a *category*
type rather than a quantity type, so it needs its own query and state shape. That tells us where the
hard part is before we hit it.

**Android Health Connect ships too** — `HealthConnectSensorManager.kt` in
[home-assistant/android](https://github.com/home-assistant/android), Apache-2.0, 26 sensors, live
since app 2025.5. That is the "wanted later" Android leg, already written.

**Caveat:** merged to `main` but not yet in a tagged App Store release, and the docs PR is still open.

**The transport is trivially reimplementable server-side.** `POST /api/mobile_app/registrations` once
(authenticated) returns a `webhook_id`; everything after is unauthenticated
`POST /api/webhook/<id>` with `{"type": ..., "data": ...}`, message types `register_sensor` then
batched `update_sensor_states`. Optional encryption is libsodium `secretbox` — no custom crypto.
**~200 lines on the Pi.**

**One asymmetry that matters for us:** iOS is genuinely event-driven via `HKObserverQuery`; Android is
a 15-minute `PeriodicWorkRequestBuilder` poll. Not push.

> **Verdict: PORT.** Apache-2.0 Swift, lifts into the existing iOS app, and the webhook envelope is
> the client write API this project needs anyway. It also gives the Android leg for free later.
> Note the licence obligation: Apache-2.0 requires attribution and a NOTICE.

---

## 2. BLE health profiles — a confirmed void

A full `git grep` of `home-assistant/core` for `0x2A37`, `0x2A35`, `0x1810`, `0x180D`, `0x1822`
returns **zero GATT-level hits**. There is no HA integration for the standard Heart Rate, Blood
Pressure, or Pulse Oximeter services, and nothing for A&D, Omron or Beurer anywhere.

**Consequence:** the A&D cuff and the oximeter profile must be written from the SIG specs, including
IEEE-11073 SFLOAT. HA contributes **connection management**, not decoders. That is already the plan.

| Item | Licence | Verdict |
|---|---|---|
| [`polar-python`](https://github.com/zHElEARN/polar-python) | MIT | **USE-AS-IS** — the one real find. RR intervals *and* the Polar PMD service for raw ECG/ACC/PPG. No HA wrapper exists |
| [`bleak-retry-connector`](https://github.com/Bluetooth-Devices/bleak-retry-connector) | MIT | **USE-AS-IS** — retry/backoff plus service-cache reuse. Directly the `fix/t4-ble-reconnect` problem class |
| [`habluetooth`](https://github.com/Bluetooth-Devices/habluetooth) | Apache-2.0 | **REIMPLEMENT** — the **connection-slot broker** is the idea. Three concurrent GATT streams on one adapter is exactly where limits bite |
| `bthome` | MIT | **IGNORE** for vitals — the v2 object-ID catalogue defines **no** HR, BP or SpO2 types, and it is advertisement-only |
| `theengs/decoder` | **GPL-3.0** | **IGNORE** — copyleft, advert-only, and its Polar entry decodes broadcast BPM with no RR |
| `Rudertier/medisana_blood_pressure` | **CC BY-NC 4.0** | **IGNORE** — a content licence on code, non-commercial only. Legally toxic |

These are Python, so "USE-AS-IS" means *for a Python collector*. Under amendment 07 the collectors
are Rust — so in practice `bleak-retry-connector` and `polar-python` are **REIMPLEMENT** targets, and
their value is the retry discipline and the PMD service knowledge rather than the code.

---

## 3. CPAP

| Item | Licence | Verdict |
|---|---|---|
| [`resmed_myair_sensors`](https://github.com/prestomation/resmed_myair_sensors) | MIT | **PORT** the `client/` subpackage — current **Okta OAuth2 + PKCE** and the per-region GraphQL endpoint map. Already proven separable (`myair-py` is a soft fork of exactly that) |
| [`hms-cpap`](https://github.com/hms-homelab/hms-cpap) | MIT | **PORT / REIMPLEMENT** — reads the SD card (ezShare or local FS) for real-time session data. Its `NOTICE` is a model clean-room statement about OSCAR |
| **OSCAR** | **GPL-3.0** | **REIMPLEMENT ONLY.** Never copy a line. Formats are not copyrightable; the code is |
| `edfio` | BSD-3 | **USE-AS-IS** — cleanest EDF reader. None of them understand ResMed channel semantics |

**Correction to stale lore:** current myAir auth is Okta OAuth2 + PKCE, **not** AWS Cognito. Also
worth recording: the AirSense 11 has **no WiFi radio** — cellular to ResMed cloud only, so the ezShare
SD adapter is the sole local path.

---

## 4. Environmental — two findings that change hardware choices

**BMP390 over BME280 for barometric pressure.** ~0.016 Pa resolution versus ~1 Pa RMS noise. This
project correlates *small* pressure swings with anxiety; BME280 would bury the signal it is there to
detect. Pair a BMP390 with an SHT4x rather than using a combined part.

**The CO2 pitfall, and it fails in the dangerous direction.** Automatic self-calibration is **on by
default** on SCD40/41, SCD30 and MH-Z19, and assumes the sensor periodically sees ~420 ppm outdoor
air. A bedroom that is never ventilated never provides that baseline, so the algorithm ratchets its
assumed zero upward over weeks and **understates** ppm — it fails toward "everything's fine," which is
the wrong direction for a health monitor. **Disable ASC; do periodic manual forced calibration with
the window open.**

Also flagged: `bme680_bsec` uses Bosch's proprietary BSEC blob, whose licence **forbids distributing
compiled firmware containing it**. Direct conflict with publishing build artifacts. Plain `bme680` is
fine.

The whole Adafruit CircuitPython driver set is MIT and runs on the Pi's own I²C bus, so **there is no
capability argument for ESP32 sensing nodes — only a placement argument.**

---

## 5. Alarm output hardware — and the ESPHome licence split

[`aioesphomeapi`](https://github.com/esphome/aioesphomeapi) is **MIT** and speaks ESPHome's native
protobuf API on port 6053 **with no Home Assistant anywhere**. That is what makes ESPHome usable as
plain remote-GPIO infrastructure.

**The licence, verbatim from ESPHome's LICENSE:** the C++/runtime codebase (`.c .cpp .h .hpp .tcc
.ino`) is **GPLv3**; the Python codebase and everything else is **MIT**. So generated ESP32 firmware
is GPLv3-encumbered, and vendoring ESPHome C++ components into `firmware/` would be too. Talking to a
device over the wire creates no obligation.

**Two hazards for a standalone (non-HA) deployment, both documented:**

1. **`api.reboot_timeout` defaults to 15 minutes** — the ESP self-reboots if no API client connects.
   With the Pi as sole client, any Pi reboot or network gap over 15 min forces the alarm actuator into
   a reconnect window at an unpredictable moment. **Set `reboot_timeout: 0s`.**
2. **`api.connected` goes stale** — if the Pi vanishes without a clean TCP close it can read `true`
   indefinitely. ESPHome offers no "coordinator went silent, fail safe" primitive; that heartbeat has
   to be built.

Also: there is **no `siren` platform** in ESPHome. Compose from `switch: gpio`, `rtttl` and `light`.

> **Verdict:** a placement-versus-reliability tradeoff, not a clear win. ESP32 buys bedside placement
> and OTA, and costs an entire extra failure domain (AP, DHCP, RF) in a nocturnal alarm path plus
> nondeterministic latency. **If the Pi can sit at the bedside, direct GPIO is the more defensible
> klaxon path.** Use ESPHome only for what must be remote.

---

## 6. Respiration sensing — a warning worth heeding

The project's shortlist named the **Seeed MR60BHA2** 60 GHz radar for contactless respiration. The
independent evidence is bad: [WUSTL Neurotech Hub](https://neurotechhub.wustl.edu/evaluating-a-60ghz-mmwave-radar-system/)
benchmarked the sibling chip against a Polar H10 — **reference at 138 bpm, sensor never exceeded
80 bpm** — with community reports of breath rate stuck at 1 bpm. The vendor's accuracy claims are
unreplicated marketing.

**Presence detection from it is solid. Vitals from it are not.** Given the CNSRisk invariant that
*indeterminate ≠ safe*, feeding MR60BHA2 breath or heart rate into anything that can escalate **or
clear** a tier would violate the design without extensive bench validation against the Polar H10
first.

The only device in the survey with real PSG validation is the **Emfit QS** — [JCSM](https://jcsm.aasm.org/doi/10.5664/jcsm.9754)
reports 95% limits of agreement of −2.5 to +2.2 breaths/min. Its HA integration is LGPL-3.0 and 2.5
years stale; the local HTTP polling is cleaner to reimplement than to link.

---

## 7. Licensing summary

**Avoid or quarantine:** OSCAR (GPL-3, formats only) · ESPHome C++ runtime (GPL-3, no vendoring, no
firmware redistribution) · `theengs/decoder` (GPL-3) · `ha-emfitqs` (LGPL-3) · Bosch BSEC
(proprietary, forbids firmware distribution) · `medisana_blood_pressure` (CC BY-NC — not an OSS
licence) · `andrewjfreyer/monitor` (no licence = all rights reserved).

**Everything else worth taking is MIT or Apache-2.0** — both companion apps, the whole HA BLE stack,
`polar-python`, `aioesphomeapi`, `resmed_myair_sensors`, `hms-cpap`, `edfio`, and the Adafruit driver
set. No conflict with open-sourcing under MIT or Apache-2.0, **provided Apache-2.0 attribution and
NOTICE obligations are honoured** for the companion-app code.

---

## 8. What to do with this

Ranked, with the reasoning already given above:

1. **Port the iOS HealthKit push + reimplement the webhook envelope.** It is the client write API this
   project needs, it is Apache-2.0, it is eight days old, and it hands over the Android leg later.
2. **Reimplement the Polar RR + PMD knowledge** from `polar-python`.
3. **Adopt the connection-slot broker concept** from `habluetooth` and the retry discipline from
   `bleak-retry-connector` — three concurrent GATT streams on one adapter is the coming problem.
4. **Port the myAir Okta/GraphQL client**; pair with `hms-cpap` for the local SD path and copy its
   clean-room NOTICE pattern for OSCAR.
5. **Revisit the barometric part choice (BMP390) and disable CO2 auto-calibration** — both change
   hardware decisions already in the purchase plan.

**On modelling the collector layer after HA's shape:** the architecture research landed and its
recommendation is recorded in [`HOMEASSISTANT-ARCHITECTURE.md`](HOMEASSISTANT-ARCHITECTURE.md) §3 —
borrow the specific behaviours (jitter, listener-gated polling, the failure taxonomy,
evidence-beats-timer), do not model the layer on HA's machinery. The final call stays deferred to
Stage 4 per the spec's §9c. The `unavailable` vs `unknown` distinction — and HA's near-total lack of
automatic staleness detection — is cited in the spec's §8b as independent precedent for the
governing law.
