# Technical Handoff: Communicating Directly With a ResMed AirSense 11 — BLE stream and SD/CAN boundaries

## TL;DR

- **Authenticated AirSense 11 BLE therapy streaming is real.** A read-only probe
  connected to the documented AirSense service, completed the application
  challenge/response session, used the encrypted JSON-RPC lane, and received
  live `StreamData` for flow, pressure, ventilation, leak, respiratory rate,
  tidal volume, snore, and flow-limitation signals.
- **BLE is still not a control or historical-data path for AnxietyWatch2.** The
  Rust collector uses saved credentials and a fixed observation-only RPC
  allowlist. A separate bounded onboarding command may discover the FD56 target
  and perform the documented first application pairing, but it never sends
  settings, therapy transitions, reset, erase, OTA, or other control operations.
- **EzShare SD remains the independent durable/archive fallback.** Its EDF
  mirror/import path is specified separately in
  [`hms-cpap-ezshare-integration.md`](hms-cpap-ezshare-integration.md) and is not
  replaced or modified by the BLE work.
- **CAN remains a protocol/control reference, not the Linux collector transport.**
  AirCANnect demonstrates useful AirSense mappings, but its ESP32/CAN runtime is
  not copied into AnxietyWatch2.

This handoff records research evidence and boundaries. The separate
`resmed-air11-onboard` command is the approved path for operator-authorized
first discovery/pairing; it is not service activation or live-collection
authorization.

## Key Findings

1. **The AirSense BLE service exposes an authenticated application stream.**
   The tested GATT service is `0000fd56-0000-1000-8000-00805f9b34fb`, with the
   documented TX and RX characteristics. FIG frames can cross arbitrary GATT
   notification boundaries and use header/payload IEEE CRC32 checks.

2. **The application session is authenticated and encrypted.** Saved pairing
   credentials complete the plaintext challenge exchange; normal application
   traffic then uses the session-derived AES-256-CBC envelope on encrypted
   VCIDs. A successful GATT connection alone is not evidence of an authenticated
   therapy stream.

3. **Live therapy values are available at their native resolutions.** The tested
   stream delivered 25-Hz flow/pressure/event values and lower-rate therapy
   summaries. `_RR2` produced respiratory-rate values during therapy. `_HRT`
   and `_SAO` were accepted stream ids but remained JSON `null` without an
   external oximeter; null is not zero and is not support evidence.

4. **The device clock is not trustworthy for stored history.** The observed
   device clock was stale (`2008-01-13T22:34:38.707Z`). A production collector
   must use host receipt time for stored timestamps, retain the raw device start
   time and an explicit untrusted-clock marker, and must not synchronize the
   machine as part of read-only streaming.

5. **Stream identity needs an explicit source-position rule.** A production
   collector must derive `seq` from the device stream position (`startTime` and
   sample index), use a new persistent `session_id` after each authenticated
   connection, and never substitute host arrival order. If source position is
   unusable, numeric ingest must stop and report the blocker.

6. **AirCANnect is useful evidence but unsafe as a direct port.** Its code
   contains AirSense BLE framing, SRP, AES, RPC, and stream mappings, but it also
   supports settings/control and OTA surfaces on a different embedded runtime.
   AnxietyWatch2 will reimplement only the bounded read-only subset in Rust.

## Read-only production boundary

The proposed collector's final send boundary permits only:

- `RequestSession` and `CheckSessionIntegrity` for saved-credential session
  authentication;
- `GetVersion` and `GetDateTime` for read-only diagnostics; and
- `StartStream` with the fixed therapy stream ids and bounded intervals.

It does not initiate first pairing or save credentials. It does not send `Set`,
`EnterTherapy`, `EnterStandby`, `ResetDevice`, `EraseData`, time-setting, OTA,
spool extraction, arbitrary `Get`, or CAN commands. There is no assumed
`StopStream`; disconnect ends observation.

## Recommendations

Use the proposed read-only Rust Air11 change for low-latency live therapy
telemetry only after its offline replay, Linux build, supervision, and physical
acceptance gates pass. The implementation and admin-page status contract are
documented in `resmed-air11-ble-stream.md`; use that runbook for bounded Pi
evidence and operator presentation. Keep the EzShare SD/EDF path enabled as
the independent archive and recovery route. Treat missing/null signals and bad
device time as unknown evidence, not as reassuring values.
