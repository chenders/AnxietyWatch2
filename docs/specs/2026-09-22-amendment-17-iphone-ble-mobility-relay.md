# Amendment 17 — Opt-in iPhone BLE mobility relay

**Date:** 2026-09-22

**Status:** Accepted by operator

**Supersedes:** Stage 7's blanket exclusion of iPhone BLE sessions and the corresponding
`no BLE`/`no BLE keep-alive` wording in the Apple-client specification and task plan. It does not
supersede the Pi's authority, the independent local alarm requirement, or the unknown-is-not-healthy
invariant.

**Affects:** `SPEC-ios-client.md` §§1–2, 5, 7, 8, and 12; Stage 7 design/proposal/tasks; future
Apple-client transport and physical-device acceptance work.

## Decision

The iPhone may provide an **opt-in, bounded BLE source relay** for a supported physiological device.
This is a mobility feature: it allows a person to move around the house while the device's BLE
connection reaches the iPhone and the iPhone forwards the stream over the private local network.
It is not a second health authority and it is not a replacement for the Pi's independent collector
and alarm path. The expected technical feasibility conclusion is **yes for an active, measured
mobility-indicator mode**: BLE notifications can be received by an iPhone and forwarded over a local
Wi-Fi network. The evidence gate determines which foreground/locked/background modes can be claimed;
it must not be presented as an unconditional indefinite background guarantee.

The preferred first hardware path is:

```text
BLE physiological device ──┐
                            │ CoreBluetooth
Pi-derived condition ─ LAN ▶ iPhone app ── BLE GATT state writes ──▶ existing nRF52840 NeoPixel bracelet
                            │
                            └─ authenticated LAN source relay ──▶ Pi relay ingress
                                                                      │
                                                                      ▼
                                                       decoder/ingest/analysis/condition
```

The iPhone may therefore be a BLE central for both a supported sensor and the bracelet, subject to
measured connection limits. The existing BLE bracelet remains the first indicator target. A Wi-Fi
bracelet/controller is deferred until a specific measured requirement justifies it; it is not needed
to prove the mobility concept.

The Pi remains authoritative for validation, canonical units, durable storage, provenance, analysis,
monitoring state, and alarm decisions. A NeoPixel bracelet is an ambient convenience indicator—not an
emergency detector or sole alert channel—intended to be easier to notice than constantly watching a
vitals screen. Its GATT protocol and color policy require separate implementation and acceptance.

## Required boundaries

- The relay is explicitly enabled by the user and has a visible `active`, `stopped`, `disconnected`,
  `stale`, or `unknown` state. It must not silently become a general-purpose BLE collector.
- CoreBluetooth may receive the supported sensor's notifications, but the iPhone does not become the
  authoritative store, run CNS analysis, compute alarm tiers, or clear/suppress the Pi alarm path.
- The preferred source path forwards sensor-identifiable frames and provenance to the Pi. The iPhone
  must not invent a stable `(source, type, session_id, seq)` identity from packet arrival time. A
  reconnect creates a new session; a source without a retransmission-stable identity is rejected or
  remains an indicator-only stream, never silently ingested as durable samples.
- The prototype should reuse the existing authenticated private-LAN client transport. Small bounded
  sensor-frame batches may be sent with an ordinary request, and the Pi response may include its latest
  condition. A time-bounded streaming connection is a later optimization only if measurements show
  bounded requests are inadequate; no socket, timer, audio session, location workaround, or BLE
  keep-alive may be used to defeat iOS process suspension.
- A prototype relay frame needs only bounded size, source identity, BLE/relay session identity,
  monotonic sequence, phone receipt timestamp, raw BLE payload, and an explicit Pi acknowledgement or
  rejection. The Pi preserves the existing source identity rules; no new medical-grade protocol is
  required for the ambient accessory. The relay still exposes received, accepted, rejected, stale,
  and unknown states.
- The Pi-to-iPhone indicator-state message must contain a monotonic state sequence, Pi generation or
  observation timestamp, state/freshness metadata, and an explicit expiry. For the prototype, a compact
  message such as `AW2,1,AB12,1042,1790085000,15,GREEN,FRESH` is sufficient: protocol, version, Pi
  generation, sequence, Pi observation epoch seconds, validity seconds, condition, and freshness. The
  iPhone reduces validity by observed relay delay; it never restarts the full validity period merely
  because it received an old response. The iPhone-to-bracelet Nordic UART or similar writable GATT
  update preserves those fields, and the bracelet firmware independently expires the last state into
  `unknown`/stale when updates stop. A stale bracelet must not continue showing the last reassuring
  color indefinitely.
- If iOS suspends the app, Bluetooth delivery stops, Wi-Fi is unavailable, the Pi is unreachable, or
  the stream exceeds its age budget, the ambient indicator enters an unmistakable non-reassuring
  unknown/stale state. It must never display a reassuring state merely because the last packet was once
  good. This protects against false reassurance; the bracelet is not expected to replace the Pi alarm.
- The Pi's local collectors, detector, klaxon, and hardware dead-man path must continue to operate
  with the iPhone powered off. The relay and bracelet cannot be the only safety channel.
- Bluetooth and local-network permissions are requested only when the user enables this feature.
  Enrollment credentials remain in the accepted Keychain/Secure Enclave lifecycle and are never put in
  BLE advertisements, URLs, logs, fixtures, or bracelet firmware.
- Physical-device evidence must measure foreground and locked/background behavior separately. The
  product may claim real-time convenience only within the measured mode and age budget; iOS does not
  provide an unconditional indefinite background real-time guarantee.

## Delivery and ownership

Relay frames are not HealthKit samples and must not be flattened into the HealthKit provenance
contract. The Pi-side relay ingress maps the supported sensor into the existing source/decoder/ingest
contracts. The Pi publishes only a small, authenticated indicator-state contract to the iPhone; the
iPhone mirrors it to the BLE bracelet without recomputing the condition. The bracelet receives no Pi
credential, stores no authoritative history, and is not a source of truth. A later Wi-Fi bracelet may
use the same state contract only after a separate hardware decision.

The first implementation task should start with the existing authenticated private-LAN transport:
send small bounded source-frame batches and return the latest Pi condition in the response. In the
first-draft private-LAN HTTP mode, the request is authenticated/signed but the response condition is
not cryptographically authenticated unless the Pi signs that object or HTTPS is used; documentation
must not call the whole exchange mutually authenticated. This is acceptable for a convenience
prototype, not for a safety decision. A persistent WebSocket or other streaming protocol is not
required initially and should be added only if physical measurements demonstrate that request/response
latency is inadequate. Ordinary journal and HealthKit delivery remains bounded, durable, and retryable
HTTP as specified elsewhere.

## Acceptance gate

The prototype is ready for a bounded physical trial when tests provide evidence for:

1. simultaneous connection to one supported physiological BLE device and the nRF52840 bracelet;
2. foreground sensor-to-Pi and Pi-to-bracelet responsiveness;
3. locked/background behavior, including any Core Bluetooth state-restoration behavior;
4. walking range, Wi-Fi interruption, BLE interruption, Pi/bracelet power interruption, and recovery;
5. bracelet sequence/expiry behavior, including purple/blue stale or unavailable display instead of
   retaining green;
6. one overnight-length battery trial; and
7. continued Pi collection, detection, local alarm, and dead-man acceptance with the phone powered off.

The EMAY device gets an early device-specific test because its protocol may require periodic BLE
keep-alive writes; the Polar H10 notification stream may be simpler. A limitation of one sensor's
background mode restricts that relay mode, not the overall architecture. A persistent streaming
protocol, stronger delivery guarantees, or Wi-Fi bracelet hardware remain future work only if these
measurements justify them.

Until this evidence exists, the feature is experimental and must be labeled as such. It may improve
mobility, but it must never be represented as a guaranteed medical or safety monitor.
