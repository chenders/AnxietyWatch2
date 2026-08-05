# EMAY Overnight Pulse Oximeter

High-fidelity overnight SpO2 and pulse rate from an [EMAY SleepO2](https://emayinc.com/) wrist or finger pulse oximeter. In AnxietyWatch2, the EMAY is integrated as a Linux Bluetooth LE peripheral, streaming live metrics to the `vitalsd` collector via BlueZ.

> **Verification needs hardware.** The `vitals-decode` crate is pure Rust and fuzz-tested, but full integration testing requires an actual EMAY device's BLE connection.

---

## What it is

EMAY oximeters are an inexpensive alternative to a sleep-lab study for capturing overnight desaturation patterns. They emit a continuous stream of SpO2 and pulse readings over a custom UART-over-BLE GATT service.

`vitalsd` connects to the EMAY device via BlueZ, buffers the incoming UART chunks, and hands them to the pure decoders in `vitals-decode`. The decoded readings are then appended to the PostgreSQL `samples` table.

---

## Data Flow

```
EMAY SleepO2 (BLE UART TX)
       │  Chunked payload (SpO2, PR, state)
       ▼
vitalsd (BlueZ / tokio)          ──── Maintains connection, handles MTU
       │
       ▼
vitals-decode                    ──── Pure Rust decoding `fn decode_emay(buf: &[u8]) -> Option<Reading>`
       │
       ▼
vitals-core (Spool)              ──── Buffers and deduplicates
       │
       ▼
PostgreSQL (`samples` table)     ──── Append-only UTC storage
```

### Pure Decoding

The protocol is binary. `vitals-decode` handles byte-parsing with the following constraints:
- **No I/O or Allocation:** The decoder takes `&[u8]` and returns an `Option` or `Result`.
- **Safe Indexing:** `get()` is used exclusively to avoid panics on malformed or truncated packets (`indexing_slicing` is denied).
- **Missing Data Handling:** If the device loses contact (e.g., emits `0`), the reading is discarded rather than stored as zero. A zero SpO2 reading is physically impossible and would corrupt clinical statistics.

### Crate Boundaries

- **`vitals-registry`**: Defines the source ID (`VitalsSource`) and metrics (`VitalsType` for SpO2 and HR) used by the EMAY.
- **`vitals-decode`**: Contains the protocol parser.
- **`vitalsd`**: Handles the Linux-specific BlueZ connection.

---

## Known limits & Hardware Caveats

- **Auto Power-Off:** The device typically powers off after roughly 2 minutes without a BLE connection. If the link drops during the night and is not quickly recovered, the recording session may terminate entirely.
- **Coexistence:** Using a dedicated CSR8510A10 USB BLE dongle with an external antenna is strongly recommended to prevent drops caused by Raspberry Pi onboard WiFi/BT coexistence issues.
- **Single Host:** The EMAY connects to only one host at a time. While `vitalsd` holds the connection, the EMAY companion mobile app cannot connect.

---

## Future Work

- Supporting off-device file transfer: If multi-hour BLE streaming proves unstable in some RF environments, adding support for pulling the onboard memory after the session (similar to the older CSV share-sheet import approach) could act as a fallback.
