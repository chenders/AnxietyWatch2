# Design — Stage 7 thin Apple client

## 1. Context

Generation one is an iOS/watchOS application with local SwiftData storage, HealthKit aggregation,
BLE sessions, analysis, reporting, and bidirectional server sync. AnxietyWatch2 reverses authority:
the Pi captures sensors, stores the record, computes analysis, generates reports, and alarms without
the phone. Stage 7 keeps only the irreducibly mobile behavior, plus the opt-in Amendment 17 BLE
mobility relay whose job is transport rather than ownership or analysis.

The normative behavior is in [`docs/design/SPEC-ios-client.md`](../../../docs/design/SPEC-ios-client.md).
This file explains the implementation seams and task ordering.

## 2. Component shape

```text
clients/apple/
├── AnxietyWatchApp/       SwiftUI composition and six phone surfaces
├── AnxietyWatchWatch/     Quick Log only
├── Packages/
│   ├── AWClientCore/      value types, state machines, retry, registry, no Apple UI
│   ├── AWTransport/       versioned API client and generated OpenAPI transport types
│   ├── AWOutbox/          temporary durable queue + receipt metadata
│   ├── AWHealthKit/       queries, anchors, observer bridge, transport records
│   ├── AWBLERelay/        CoreBluetooth source relay, Pi-state mirror, and bounded LAN stream
│   └── AWDomain/          journal/song/dose/check-in commands and projections
├── Tests/                 unit, contract, persistence, UI, accessibility
└── AnxietyWatch.xcodeproj
```

Dependencies point inward: UI and Apple adapters depend on pure packages; pure packages do not import
SwiftUI, HealthKit, WatchConnectivity, or persistence frameworks. The server remains Rust. There is
no shared SQLite/SwiftData schema and no generated Swift registry enum.

## 3. Two local persistence classes

### 3.1 Temporary durable outbox

The outbox prevents person-entered data from disappearing while the Pi is unreachable. It is not a
history database. Capture and enqueue commit atomically; terminal acknowledgement deletes payload
content atomically. Only non-content receipt metadata may remain for bounded diagnostics.

Data protection must support HealthKit background delivery after first unlock. The implementation
task measures locked-device behavior and records the selected `NSFileProtection` and Keychain
accessibility classes; it may not weaken protection silently to make a background test pass.

### 3.2 Expiring display cache

The cache stores server projections only. It is keyed by endpoint, client identity, API/registry
version, and query. Every value carries server generation/observation time and local fetch time.
Eviction loses convenience, never the record. Stale cached content remains visibly stale.

Neither store supports local trend analysis, local report generation, or a “restore server” action.

## 4. Delivery state machines

### 4.1 Write item

```text
capturing → queued → sending → acknowledged → payload deleted
                    ↘ retry_wait ─┘
                    ↘ rejected
                    ↘ conflict
```

Only `acknowledged`, `rejected`, and `conflict` are terminal transport outcomes. Rejected/conflict
content remains until the user exports, corrects by creating a new version, or explicitly discards
it. Cancellation returns `sending` to `queued`.

### 4.2 HealthKit anchor

```text
load committed anchor
  → anchored query page
  → transaction(enqueue every add/delete, persist next anchor)
  → trigger outbox drain
```

Persisting an anchor outside the enqueue transaction is prohibited. The server absorbs replay by
HealthKit UUID and the accepted platform identity contract.

### 4.3 Viewer read

```text
fresh server response → render live + cache
server failure + usable cache → render stale + age/reason
server failure + no cache → unknown/unavailable
```

There is no path from missing response to an empty healthy model.

## 5. HealthKit parity strategy

Generation one has overlapping but non-identical sets: authorization types, sample-mirror types,
anchored observer types, and snapshot inputs. T-I0 creates a machine-readable parity ledger from all
four and classifies every type:

- `proxy_sample` — raw source sample belongs in the platform envelope;
- `proxy_category` — category/event mapping is required (for example sleep);
- `context_only` — required only if an accepted server feature consumes it;
- `excluded` — named owner-approved reason;
- `blocked_registry` — intended, but no canonical registry representation yet.

The target is broad parity with the data generation one used, not parity with its local aggregates.
Snapshot outputs, trend thresholds, source precedence, and derived values are server work and never
cross as if they were measured HealthKit samples.

## 6. API seams

| Contract | Server owner | Client owner |
|---|---|---|
| secure enrollment, credential rotation/revoke | T-M2a/T-I1 | T-I4 |
| OpenAPI/version/error envelope | T-I2 | T-I3 |
| platform sample identity/provenance | T-X3/T-X4 | T-I6/T-I7 |
| BLE relay framing, source identity, and stale state | Amendment 17/T-I23 Pi ingress | T-I23 |
| phone status beacon | T-X1/T-X2 | T-I9 |
| domain record version/tombstone schema | T-M5a/T-M5b | T-I12–T-I15 |
| viewer queries/reports | M3/M4 | T-I10/T-I11 |
| migration ingest/report | T-X5/T-X6 | T-I19 |

Server and Swift contract tests consume the same committed JSON fixtures. OpenAPI generates transport
shapes, not policy. Registry JSON supplies physiological identifiers, units, ranges, and source
capabilities. The client state machines remain handwritten and exhaustively tested so an unknown API
enum cannot inherit a safe-looking default.

## 7. Enrollment and trust

The M0 envelope registration route is localhost-only. Production enrollment is deliberately gated.
The target flow is:

1. Authenticated console creates a one-time, short-lived enrollment artifact.
2. User scans or transfers it to the app and confirms endpoint/device identity.
3. App and server establish a per-device revocable credential over authenticated TLS.
4. App stores it in Keychain and discards enrollment material.
5. Console shows fingerprint, scope, created/last-used/expiry, and revoke.

T-I1 chooses the exact credential protocol with a threat model. It must use platform/vetted crypto,
not a new construction, and preserve queued writes across rotation. Cleartext LAN enrollment and a
permanent QR secret are not valid shortcuts.

## 8. Immediate upload without false promises

“Send as soon as possible” means every event requests a drain immediately and no application policy
adds a waiting window. iOS may still suspend the process or deny background time. Therefore:

- event callbacks enqueue before returning;
- foreground capture starts a drain immediately;
- app activation and network-path recovery request another drain;
- HealthKit background delivery and BG tasks request bounded drains;
- jitter applies only after a failed attempt;
- Pi-side silence and oldest-outbox age expose when “possible” has not occurred.

A socket would not grant background execution and therefore does not solve the primary limitation.

Amendment 17 is a narrow exception for an explicitly active mobility relay. The prototype starts with
small bounded authenticated request/response batches and mirrors Pi state to the BLE nRF52840 bracelet
while iOS permits it. The relay must publish lifecycle evidence and the bracelet must independently
enter stale/unknown when updates expire. This is technically feasible as an active measured mode; it
is not a promise of indefinite background delivery. A persistent stream is future work only if
measurements justify it. The ambient indicator is convenience feedback, not an emergency detector,
and the Pi alarm path remains independent. A Wi-Fi bracelet is deferred.

## 9. Input architecture

Journal, rating, tags, selected song/free-form song stub, medication dose, and check-in responses are
commands with client IDs. Server projections populate pickers and history. The phone can create while
offline against a bounded cached catalog, but authoritative catalog enrichment and duplicate merging
run on the Pi.

Express Mode commits rating + times + ID + origin in one tap. Notes, tags, song, and pain can augment
through a new immutable version; they cannot turn one-tap capture into a multi-step requirement.

## 10. Watch topology

The watch uses a tiny durable queue and WatchConnectivity. The phone acknowledges only after its own
outbox transaction commits. This leaves two observable hops and no Pi secret on the watch. Direct
watch networking stays out until acceptance data demonstrates relay loss that outweighs credential
and retry complexity.

## 11. Rollout slices

1. **Contracts and skeleton:** parity ledger, threat model, OpenAPI, project, pure core.
2. **Reliable write substrate:** enrollment, outbox, retry, observability.
3. **HealthKit proxy:** registry mapper, authorization, anchored/background queries, beacon.
4. **Viewer:** read/cache model and live server projections.
5. **Subjective inputs:** journal/song, doses/check-ins, random prompts, watch.
6. **Migration and hardening:** gen-1 import, security/privacy/accessibility/energy, physical-device
   acceptance, decommission.

No slice presents blocked backend behavior with synthetic production success. Development fixtures
are visibly stamped synthetic.

## 12. Rejected alternatives

- **Keep generation one's app as-is and change its server URL:** rejected; preserves local authority,
  analysis, BLE, and bidirectional-sync defects.
- **No local persistence:** rejected; person-entered data would vanish whenever iOS kills the app or
  the Pi is unreachable.
- **Local long-term mirror “for offline”:** rejected; creates a second system of record and restore
  problem. A bounded stale projection cache is sufficient for viewing.
- **WebView/PWA:** rejected; HealthKit, reliable local notifications, background delivery, and watch
  relay require native APIs.
- **Permanent ordinary-sync WebSocket:** rejected; it does not buy iOS background execution and adds
  a liveness surface. A bounded authenticated stream is permitted only for the explicit Amendment 17
  relay and only with physical evidence.
- **Make the bracelet the emergency path:** rejected; the bracelet is ambient convenience feedback and
  must become stale/unknown when the phone relay is unavailable. The Pi detector and local alarm remain
  independent.
- **Replace the existing BLE bracelet with Wi-Fi hardware first:** deferred; the nRF52840 BLE bracelet
  is the initial target, and Wi-Fi hardware requires a specific measured reason.
- **Watch talks directly to Pi:** deferred; more credentials and retry logic for no demonstrated need.
- **Port local snapshot/analysis code:** rejected; the Pi owns computation and provenance.
- **Treat the iPhone as a second authority:** rejected; the BLE relay forwards source frames and
  provenance while the Pi owns validation, storage, analysis, and alarm decisions.
