# Change: ios-client-stage7 — thin Apple client, HealthKit proxy, and BLE mobility relay

## Why

The architecture assigns the iPhone four jobs—HealthKit bridge, journal/input, viewer, and an
optional BLE mobility relay—but no accepted client specification or executable Stage 7 queue exists.
The generation-one app contains valuable panic-path and HealthKit behavior, yet porting its storage,
analysis, legacy BLE, snapshot, and bidirectional-sync architecture would recreate exactly what
AnxietyWatch2 moved to the Pi.

The absence is already blocking server decisions: platform sample provenance, phone silence, domain
record shape, enrollment, OpenAPI, and historical backfill all have a server half but no concrete
client contract.

## What changes

- Add `docs/design/SPEC-ios-client.md` as the Stage 7 functional contract.
- Define the phone as a **thin client**: Pi-backed viewer, subjective-input capture, HealthKit
  proxy, and an opt-in transport-only BLE mobility relay. The Pi remains authoritative.
- Require upload at the earliest iOS-permitted opportunity through a durable temporary outbox; raw
  payloads are removed locally once acknowledged and never become a long-term phone archive.
- Establish generation-one HealthKit coverage parity as the target while requiring every type to land
  in the registry and server mapper before the client sends it.
- Preserve generation-one journal ratings, notes, tags, songs, Express Mode, doses, check-ins, and
  watch Quick Log while moving durable authority and all analysis to the Pi.
- Permit a measured BLE-device → iPhone → private Wi-Fi → Pi source-relay path, plus Pi condition →
  iPhone → BLE nRF52840 bracelet mirroring, so the user can move around the house without treating the
  bracelet as an emergency detector. Defer a Wi-Fi bracelet until a specific requirement justifies it.
- Add explicit security, privacy, accessibility, background-delivery, identity, conflict, migration,
  and physical-device acceptance requirements.
- Add a dependency-aware task queue with shell `Verify:` contracts and traceability.

## Architectural outcome

```text
HealthKit ─┐
Journal  ──┼─▶ temporary durable outbox ─▶ HTTPS client API ─▶ Pi system of record
Watch Log ─┘          │                         │
                      └─ delete payload on ACK  └─ analysis / reports / long history

BLE sensor ─▶ bounded iPhone relay stream ─▶ Pi decoder/analysis ─▶ condition over LAN ─▶ iPhone
                                                                                 │
                                                                                 └─ BLE GATT ─▶ ambient nRF52840 bracelet

Pi projections / reports ─▶ bounded expiring display cache ─▶ iPhone viewer
```

The outbox can retain an unacknowledged item as long as necessary to prevent data loss, but it is not
queryable history. Acknowledged content is removed from local persistence immediately; long-term
history is fetched from the Pi.

## Impact

- Planning and documentation only; no production code or schema changes in this change.
- New implementation home: `clients/apple/` in this repository.
- Server-side tasks are coordinated with `admin-console-roadmap` T-M2a, T-X1–T-X6, M3–M6 rather
  than duplicated.
- The generation-one repository remains a behavior/fixture source until verified migration and
  decommission, not the Stage 7 implementation home.

## Non-goals

- No iOS/watchOS code in this planning change.
- No acceptance of new registry types or domain schema by implication.
- No choice of insecure remote enrollment merely to reuse the localhost-only M0 route.
- No local analysis, snapshot aggregation, legacy BLE collector, HealthKit writes, or alarm
  dependency. The only permitted iPhone BLE session is the explicit Amendment 17 transport relay.
- No claim that iOS background execution guarantees immediate delivery; the Pi detects silence.

## Gates

| Gate | Blocks | Resolution owner |
|---|---|---|
| G-IOS-AUTH | production enrollment/write transport | T-I1 with admin T-M2a |
| G-IOS-PROVENANCE | HealthKit sender | admin T-X3/T-X4 + T-I6/T-I7 |
| G-IOS-BEACON | app/status telemetry | admin T-X1/T-X2 + T-I9 |
| G-IOS-RECORDS | journal/song/dose/check-in persistence | admin T-M5a/T-M5b + T-I12–T-I15 |
| G-IOS-REPORTS | complete viewer | admin M3/M4 + T-I10/T-I11 |
| G-IOS-MIGRATION | generation-one decommission | admin T-X5/T-X6 + T-I19 |
| G-IOS-ALARM-PUSH | remote alarm display and operator-directed APNs messages | admin M6 + T-I22 |
| G-IOS-BLE-RELAY | bounded BLE source relay plus Pi-state-to-iPhone-to-BLE-bracelet ambient indicator | T-I23 |

## Completion

The change plan is complete when every task is verified and merged, explicitly superseded by an
accepted amendment, or recorded as deferred with its named gate and owner. Core Stage 7 runtime
completion is the twelve-item acceptance contract in `SPEC-ios-client.md` §12.3; the opt-in mobility
relay has the additional Amendment 17/T-I23 evidence gate before it is presented as supported.
