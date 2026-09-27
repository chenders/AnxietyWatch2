# Design — Linux BlueZ broker and collector access rewrite

## 1. Evidence and decision

The current implementation has independent BlueZ clients. They can race on adapter-global discovery and filter state, compete for connection capacity, retry without a shared budget, and in the Wellue path reset a shared adapter. The recorded sequence—Wellue absent while discovery remained active, then BP recovering after Wellue stopped without a BP restart—makes a global ownership boundary the simplest reversible hypothesis test and the correct architectural fix.

The proposal is a **per-adapter broker with a lease scheduler**, not a monolithic collector supervisor and not an advisory lock. One broker owns application-level BlueZ access for one adapter. Source workers remain separate processes so a protocol bug or worker crash does not take down every source. A logical supervisor may supervise multiple brokers, but it must not become another BlueZ client.

The broker does not make the radio independent. USB loss, controller reset, BlueZ failure, RF contention, and adapter capacity remain shared failure domains and must be visible as degraded/unknown state.

## 2. Public prior art and reuse boundary

BlueZ's `bluetoothd` is already the low-level D-Bus owner:

- <https://github.com/bluez/bluez/blob/master/doc/org.bluez.Adapter.rst>

It does not provide the source-level leases, priorities, durable frame boundary, or BP policy boundary required here.

Home Assistant's open-source Bluetooth stack is the strongest adjacent implementation:

- <https://github.com/Bluetooth-Devices/habluetooth>
- <https://github.com/Bluetooth-Devices/bleak-retry-connector>
- <https://github.com/Bluetooth-Devices/bluetooth-adapters>

Its active-scan scheduler, adapter history, connection-slot accounting, stale-connection cleanup, and backoff behavior are useful reference designs. They are Python/in-process libraries tied to Bleak and do not model this project's Rust boundaries, source-worker isolation, BP kernel autoconnect, or ACK-to-journal durability hole. They are therefore inspiration and evidence, not appliance dependencies.

Rust `bluer`/D-Bus bindings remain appropriate low-level implementation tools, but only the broker may use them after migration. No generic D-Bus escape hatch crosses the worker protocol.

## 3. Boundaries

```text
                    privileged kernel-policy boundary
                (narrow BP accept-list operations only)
                                  |
                                  v
BlueZ adapter <-> vitals-ble-broker-hciX <-> source workers
                                      |          |-- BP
                                      |          |-- H10
                                      |          |-- SleepO2
                                      |          |-- AirSense
                                      |          `-- Wellue (disabled)
                                      v
                            vitalsd / spool
```

### Broker owns

- One BlueZ D-Bus session and one verified adapter object.
- Adapter power/discovery/filter operations.
- All non-BP `Device1` connect/disconnect operations.
- Service resolution, characteristic lookup, notification/indication subscriptions, and GATT writes.
- Adapter-global and device-local queues, deadlines, cancellation, retry admission, lease expiry, and adapter generation.
- Redacted operation IDs, metrics, health, and source-neutral raw-frame delivery.

### Source worker owns

- Source protocol/session state after receiving the transport stream.
- Pure decoding, registry conversion/range checks, source identity, sequence policy, status, and ingest.
- Raw journal and `vitalsd` append semantics, including BP's `sync: true` request.
- No BlueZ object, adapter power, discovery, connect, disconnect, pairing, filter mutation, reset, or raw `btmgmt` capability.

### Kernel-policy boundary owns

- BP accept-list registration through the Linux management interface or the existing narrowly scoped `btmgmt` service/rule.
- Controller selection, address type, idempotent registration, and hotplug re-registration.
- No generic D-Bus access, GATT access, source data, database access, or adapter reset.

The existing BP autoconnect unit/rule remains separate until its registration and controller identity are instrumented and the broker has passed its BP gate. The old userspace BP connector is not a fallback and must be retired before broker ownership is enabled.

## 4. Broker protocol

The exact serialization is an implementation decision, but the semantic interface is fixed before the first source migration. Use a private Unix socket, protocol version, bounded frame sizes, worker identity, adapter selector, and broker generation.

```text
Hello { protocol_version, worker_kind, adapter_selector,
        device_selector, requested_capabilities }

Acquire { intent, priority, deadline, max_hold, preemptible, reason }
  -> Lease { lease_id, adapter_generation, expires_at }

ObserveDevice / DiscoverTarget / Connect / Resolve / Subscribe / Write
  -> operation_id / result

Frame { lease_id, adapter_generation, device_session,
        characteristic_uuid, indication_or_notification,
        received_monotonic, received_utc, payload }

Release { lease_id, reason }
Cancel { lease_id, operation_id, reason }
Health {} -> BrokerHealth
```

The broker must reject unsupported capabilities at handshake. The BP capability set omits `Connect`. There must be no `CallD-Bus(method, path, args)` operation. Payloads and per-worker queues are bounded; slow workers receive a named backpressure result and release their lease instead of blocking established streams.

For BP, raw bytes must be journaled durably before the broker acknowledges frame delivery as accepted to the worker. The worker still performs the source append with `sync: true` and must observe the reply. This narrows and exposes the ACK-to-journal gap; it does not pretend the kernel ATT confirmation is application durability.

## 5. Scheduling and state

There are two lease layers:

1. **Adapter-global:** discovery, filter mutation, new-connect admission, power/recovery. Only one discovery lease exists. Recovery excludes all other operations and is never an ordinary source retry.
2. **Device-local:** connect/disconnect, service resolution, GATT operations, and stream ownership for one device. GATT writes serialize per device. Established links may continue while another device is discovered if measured capacity permits it.

Initial policy priorities:

| Priority | Source/operation | Rule |
|---|---|---|
| P0 | BP kernel observation and indication subscription | Never initiate connect; subscribe promptly after kernel connection. |
| P1 | H10 and SleepO2 established streams | Preserve streams and keepalives; bounded reconnect. |
| P2 | AirSense | Bounded discovery/connect and authenticated stream; lifecycle remains source-owned. |
| P3 | Wellue | Disabled until separately authorized; then bounded discovery and long cooldown, never source-triggered adapter reset. |
| P4 | Diagnostics/onboarding | Explicit operator lease only. |

Retries do not hold leases. Every failure is classified, the lease is released, and the next attempt is scheduled with source, attempt number, error class/value, and monotonic eligibility time. Controller recovery is a separately authorized operation with a named blast-radius gate.

Required states distinguish `Connected`, `ServicesResolved`, `Subscribed`, `Streaming`, `Silent/Stale`, `Degraded`, and `Unknown`. A broker generation change invalidates old leases and source sessions. A BP session uses `KERNEL_WAIT -> CONNECTED_BY_KERNEL -> SERVICES_RESOLVING -> SUBSCRIBING -> DRAINING`, with no userspace connect transition.

Discovery state records owner, generation, previous/requested/restored filters, start/end monotonic time, deadline, and restoration result. A failed restoration is degraded state, not a successful timeout.

## 6. Stable adapter identity

`hci0`, `hci1`, and later indexes are runtime names, not hardware identities. The broker must resolve a configured adapter selector against verified controller evidence—USB path/serial/address and validated udev properties as available—then record both stable evidence and current `hciX`. It must refuse ambiguous or missing matches. Controller identity, BlueZ/kernel versions, and the deployed artifact are physical/runtime gates; this design does not infer them from conflicting repository records.

The broker must handle BlueZ restart, D-Bus loss, USB replug, controller reset, and hotplug as adapter-generation changes. It must not treat stale `Connected` properties or an old lease as current health.

## 7. Migration

### Stage 0 — inventory freeze

Resolve the deployed revision, enabled units, controller identity, BlueZ/kernel versions, all direct call sites, and source budgets. Do not touch the Pi, re-enable Wellue, change overlays, or claim H10 acceptance.

### Stage 1 — offline policy and protocol

Implement pure lease/scheduler policy and fake transport tests. Implement the versioned broker protocol and capability rejection. No production unit changes.

### Stage 2 — observe-only Linux broker

Run only an explicit observe-only mode. It may observe BlueZ object events, discovery state, adapter generation, and BP kernel-policy results, but may not start discovery, connect, power, or reset. Keep the existing approved paths and do not run overlapping owners.

### Stage 3 — one reversible source

Select the first source only after Stage 0/1 evidence and owner approval. Move its worker behind the broker, verify equivalent status/data and no unrelated discovery disruption, then stop the old owner before any rollback or retry.

### Stage 4 — BP

Move observation/subscription and raw-frame delivery behind the broker while retaining the privileged accept-list boundary. Prove the broker protocol cannot encode BP connect and retire the stopgap userspace connector only after the physical BP gate passes.

### Stage 5 — remaining sources

Migrate SleepO2, AirSense, H10, and any later authorized Wellue path one at a time. Measure packet loss, gaps, keepalive misses, discovery latency, broker queue delay, and reconnects. If capacity fails, add a verified second adapter and broker; do not compensate with unbounded retries.

Rollback at every stage is source-level: stop the new client, confirm lease release and no lingering subscription, then start the old approved owner. Never run old and new BlueZ owners concurrently. No automatic adapter reset is a rollback strategy.

## 8. Verification and acceptance

Before implementation is accepted:

- Static call-site checks show only the broker contains application `bluer`/D-Bus operations; the kernel-policy exception is explicit.
- Pure tests cover discovery exclusivity, fairness, deadline, filter restoration, cancellation, worker death, backpressure, generation changes, and retry release.
- Linux contract tests cover ObjectManager events, `Connected`, `ServicesResolved`, GATT subscription, discovery ownership, hotplug, and controller-generation changes.
- The deterministic absent-Wellue plus BP-measurement test proves an absent optional source cannot prevent a kernel-created BP drain.
- BP tests cover raw journal-before-decode, `sync: true`, broker crash, journal-full, and worker backpressure.
- Physical/Pi gates record controller identity, kernel accept-list registration, measured coexistence, and rollback evidence.

No emulator, stale property, or successful process start substitutes for physical acceptance. Unknown remains unknown.

## 9. Open decisions before implementation

- Exact stable controller selector fields available on the installed Pi and USB adapter.
- Whether the narrow kernel helper remains `btmgmt` or becomes a separate management API crate.
- Exact wire encoding and broker socket permissions.
- Measured connection slots, discovery timing, keepalive budgets, and source priorities.
- Whether any source requires a second verified adapter.
- AirSense transport/lifecycle boundary and Wellue identity/retransmission evidence.
