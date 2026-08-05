# Polar H10 Chest Strap

High-fidelity HRV recording from a paired Polar H10. The Polar H10 streams **every individual heartbeat interval** (RR intervals) over Bluetooth Low Energy, allowing for beat-to-beat HRV computation. In AnxietyWatch2, this stream is captured directly by `vitalsd` on the Raspberry Pi.

> **Verification needs hardware.** The data decoders are pure and unit-testable, but the BlueZ connection behavior and dropout characteristics require physical testing with the H10 strap.

---

## What it is

A pipeline that turns a Polar H10 chest strap into a reference HRV sensor:

- Beat-to-beat **RR intervals** (the millisecond gap between consecutive heartbeats) stream from the strap at ~1 Hz over the standard Bluetooth `0x180D` Heart Rate service.
- `vitalsd` collects these measurements via BlueZ and tokio.
- The raw RR-interval stream is routed through `vitals-core` and safely decoded by `vitals-decode`.
- The intervals and computed HRV are stored immutably in the PostgreSQL `samples` table.

---

## Data flow

```
Polar H10 (BLE 0x180D ┆ 0x2A37 notify)
       │  HR + 1..n RR intervals every ~1s
       ▼
vitalsd (BlueZ / tokio)          ──── Linux Bluetooth stack
       │
       ▼
vitals-decode                    ──── Decodes 0x180D payload into RR intervals / HR
       │
       ▼
vitals-core                      ──── Spool and aggregation
       │
       ▼
PostgreSQL (`samples` table)
```

Key behaviors:

- **Pure Decoders:** In `vitals-decode`, parsing the `0x180D` payload is 100% pure (`no_std`, no I/O, no allocation). Out-of-range values are rejected rather than clamped.
- **Append-Only Storage:** Measurements are appended as UTC epoch seconds. Invalid or absent data is never coerced to a valid number.
- **Continuous Connection:** Transmitting RR intervals requires an active BLE connection. `vitalsd` contains robust reconnection logic to handle roll-overs or bathroom breaks.

---

## Hardware and Setup

- **Strap:** Polar H10. The strap must be wet to transmit properly. Dry pads produce immediate connection drops and artifacted data.
- **BLE Hardware:** A branded CSR8510A10 USB BLE dongle with an external antenna is recommended on the Raspberry Pi. The onboard Pi radio suffers from severe 2.4GHz coexistence drops when streaming high-frequency data for hours.

---

## Known limits & Hardware Caveats

- **Overnight Drops:** Multi-hour drops are a known issue even in the official Polar SDK. Mitigations include reducing the MTU, ensuring fresh batteries, and applying conductive gel to the strap pads.
- **Single Connection Constraints:** The H10 generally limits concurrent connections (though it can be configured to support two in the app). If `vitalsd` is connected, it may block pairing with a watch or phone simultaneously.
- **Bandwidth/Throughput:** Concurrent connections (e.g., streaming both an EMAY oximeter and Polar H10) to the same BlueZ adapter can induce packet loss. If drops become frequent, deploying a dedicated USB adapter for the Polar may be required.

---

## Future work

- Server-side HRV re-derivation tooling that consumes the raw RR archive and computes alternative frequency/time-domain HRV measures (LF/HF ratio, SDNN, RMSSD).
- Aggregation across multiple sub-sessions per night.
