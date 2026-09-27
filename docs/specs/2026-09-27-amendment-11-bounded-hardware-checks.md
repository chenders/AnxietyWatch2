# Amendment 11 — bounded hardware checks

**Date:** 2026-09-27
**Status:** Accepted by owner instruction
**Supersedes:** hardware-check duration requirements in task contracts and runbooks
**Affects:** all hardware verification tasks; explicitly excludes the ResMed AirSense 11 integration check

## 1. Ten-minute ceiling

Every individual hardware check or live hardware capture MUST complete within ten minutes. This
is a bounded bring-up check, not an endurance claim. A task may run several independent checks, but
no one check may keep hardware active longer than ten minutes.

The sole exception is the dedicated ResMed AirSense 11 integration check. That integration may use
whatever duration its own acceptance contract requires. The exception does not apply to Polar H10,
EMAY, A&D, XE125, audio, Bluetooth, storage, or other hardware checks.

## 2. T-B1 replacement

T-B1 is a ten-minute Polar H10 hardware-path check. It proves discovery, pairing, GATT service
resolution, subscription to Heart Rate Measurement (`0x2A37`), valid notifications, and the gap
ceiling. It MUST NOT require an overnight run or claim eight-hour continuity.

The gap histogram tool accepts an explicit `--require-duration-minutes` gate. T-B1 no longer
requires a drift figure: ten minutes is sufficient to prove the notification path but is not an
honest basis for an overnight clock-drift conclusion. A drift figure may still be reported when
estimable.

## 3. What this does not change

This amendment changes verification duration only. It does not make a short check evidence of
long-term reliability, alter sensor identity, promote an experimental source, or waive review.
The ResMed exception is narrow and must not be generalized to another device.
