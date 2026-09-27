# Linux BlueZ rewrite — reliability and verification test plan

**Status:** design only. No Rust, SQL, systemd, udev, firmware, hardware, or runtime changes were made. Wellue remains disabled. No H10 acceptance is claimed.

## 1. Evidence boundary and findings

### Observed facts

| Fact | Evidence |
|---|---|
| The appliance has five relevant BLE sources: A&D BP, AirSense 11, EMAY, Wellue/Viatom, and Polar H10. The controller identity must not be assumed. | `docs/reviews/bluetooth-rewrite/BRIEF.md:11` |
| The incident was cross-collector contention: absent Wellue discovery remained active, `bluetoothctl show` reported `Discovering: yes`, BP was paired/trusted but disconnected, and after Wellue stopped, BP frames became durable without a BP restart or manual `Connect`. | `docs/reviews/bluetooth-rewrite/BRIEF.md:14-21` |
| BP is intentionally not allowed to call `Device1.Connect`; it waits for privileged/kernel autoconnect, and its frames must be durable before acknowledgement. | `docs/reviews/bluetooth-rewrite/BRIEF.md:13`; `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:1-16,276-301` |
| The current BP implementation journals and fsyncs each raw frame before decode, then requests `sync: true` from vitalsd. | `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:87-122,208-241` |
| Existing collectors independently open BlueZ sessions, select an adapter, power it, and operate on devices. | `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:251-255`; `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs:414-438`; `$HOME/Source/aw2-air11/crates/vitals-collectors/src/as11_bluez.rs:387-419`; `$HOME/Source/AnxietyWatch2/crates/emay-collector/src/main.rs:1367-1385` |
| H10 has both a `Connected` watchdog and notification-idle watchdog because BlueZ can report a stale link while the notification future remains pending. | `$HOME/Source/aw2-air11/crates/vitals-collectors/src/h10_reconnect.rs:1-67,105-168`; `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs:384-412` |
| AirSense has a bounded discovery fallback, but its transport still owns direct connection/discovery behavior. | `$HOME/Source/aw2-air11/crates/vitals-collectors/src/as11_bluez.rs:387-419,688-750` |
| Wellue's local mitigation is a 10-second discovery window and one-minute retry, but the collector still performs discovery itself. | `$HOME/Source/aw2-wellue-fix/crates/wellue-collector/src/main.rs:30-47,211-238,613-651` |
| Wellue's adapter reset is a collector-local recovery path, not a global adapter policy. | `$HOME/Source/aw2-wellue-fix/crates/wellue-collector/src/main.rs:457-505` |
| The Wellue replay is synthetic and explicitly marked; its offline vertical slice does not prove a physical Wellue protocol or coexistence. | `$HOME/Source/aw2-wellue-fix/crates/wellue-replay/src/main.rs:1-15,237-258`; `$HOME/Source/aw2-wellue-fix/scripts/wellue-replay-test.sh:1-55`; `$HOME/Source/aw2-wellue-fix/docs/setup-logs/wellue-ks60f.md:36-44,88-103` |
| H10 connection behavior and dropout characteristics require physical testing; concurrent H10 and EMAY links may induce loss. | `$HOME/Source/aw2-air11/docs/hardware/POLAR_H10.md:3-5,48-57` |
| Fresh H10 measurements were absent in the available evidence; `hr=0` was rejected and acceptance was not claimed. | `$HOME/Source/aw2-air11/docs/status-updates/2026-09-13T05-40-43Z.md:5-7`; `$HOME/Source/aw2-air11/docs/status-updates/2026-09-13T05-48-11Z.md:1-10` |
| The current deployment evidence is internally inconsistent about adapter numbering: the design gate speaks of Sena as `hci1`, while the BP udev/unit paths target `hci0`. | `$HOME/Source/AnxietyWatch2/openspec/changes/pi5-vitals-core/design.md:177-180`; `$HOME/Source/aw2-air11/deploy/99-vitals-bp-autoconnect.rules:1-4`; `$HOME/Source/aw2-air11/deploy/vitals-bp-autoconnect.service:1-20` |

### Governing constraints

- The indexed amendments require Rust/`bluer`, typed generated source/type identifiers, unique restart-safe sessions, append-only storage, raw journals for synced sources, and no coercion of absent values. `docs/specs/AMENDMENTS.md:36-102`.
- A BlueZ/Pi run is evidence only when its command exits zero and its bundle contains the raw witness data. A fake passing test cannot replace the real Pi gate.
- `LIBRARIAN.md` was not found in the thin-client, AnxietyWatch2, aw2-air11, or aw2-wellue-fix trees. Missing authoritative inventory facts are requested in `requests/2026-09-22-reliability-entry-points.md`.

## 2. Proposed rewrite boundary

### Single owner per adapter

Introduce one Linux-only `vitals-bluetooth`/`vitals-btd` owner for each controller. It is the only process permitted to mutate BlueZ adapter/device/GATT state. Collectors become protocol clients and retain decoding, source identity, journaling, sample construction, and vitalsd append policy.

The owner must:

1. Open one D-Bus/BlueZ session per adapter.
2. Own adapter power, discovery filters, discovery leases, Device1 connection/disconnection, GATT resolution, notification subscriptions, and bounded writes.
3. Serialize discovery and connection operations through one arbiter. No collector may start an unbounded scan.
4. Preserve a distinct BP `AwaitKernelAutoconnect` operation. The BP client must never issue `Device1.Connect`; privileged kernel accept-list registration remains an explicit, auditable operation.
5. Give every operation a deadline, cancellation path, owner, source, correlation id, and named failure reason.
6. Publish adapter/device state as measured-good, measured-bad, or not-measured. `Connected=true` alone is never a physiological measurement.
7. Keep Wellue as a disabled/optional lease until its physical protocol and safety gates pass.

### Proposed testable APIs

These are seams to implement later, not changes in this investigation.

```text
AdapterPort
  snapshot() -> AdapterSnapshot
  set_powered(bool) -> Result
  acquire_discovery(DiscoveryLease) -> Result<DiscoveryHandle>
  release_discovery(DiscoveryHandle) -> Result
  events() -> Stream<AdapterEvent>

DevicePort
  inspect(target) -> Result<DeviceSnapshot>
  await_kernel_autoconnect(target, deadline) -> Result<ConnectedDevice>
  connect(target, deadline) -> Result<ConnectedDevice>       # never used by BP
  disconnect(target, reason, deadline) -> Result

GattPort
  resolve(device, service, characteristics, deadline) -> Result<GattEndpoint>
  subscribe(endpoint, deadline) -> Stream<Result<Bytes>>
  write(endpoint, bytes, deadline) -> Result

BluetoothPolicy
  on(event, now) -> Vec<Action>
  on_timeout(token, now) -> Vec<Action>
  invariant_failures() -> Vec<NamedFailure>

TelemetrySink / RawJournal / Clock
  record(event)
  append_sync(source, session, raw_frame) -> Result
  now() -> Instant
```

The public manager protocol should expose typed operations such as `Acquire`, `AwaitLink`, `OpenGatt`, `Subscribe`, `Write`, `Release`, and `Snapshot`; it must not expose a generic arbitrary D-Bus method or arbitrary object path. Source ids and type ids are generated enums at the collector boundary, not integers supplied by a client.

## 3. Deterministic test pyramid

### L0 — pure and state-machine tests (every host)

Create a fake-clock, scripted-event policy harness. It must run without BlueZ, D-Bus, I/O, wall clock, or physical devices.

Required cases:

- discovery lease acquisition, priority, fairness, deadline, cancellation, and release on every success/error/timeout path;
- a second discovery request is rejected or queued with the blocking owner named;
- prior discovery filters are restored even when target discovery, stream polling, or cleanup fails;
- no action is emitted after a lease deadline; no retry loop can run without advancing fake time;
- connect timeout followed by explicit cancellation cannot produce a later `Operation already in progress` loop;
- `Connected=true` followed by `ServicesResolved=false`, notification silence, object removal, and stream end are distinct states;
- BP produces `AwaitKernelAutoconnect`, never `Connect(target)`;
- GATT characteristic mismatch, duplicate UUID, missing notify/indicate capability, and invalid MTU fail closed;
- one source session ends before a reconnect mints a new restart-safe session id;
- a BP indication is journaled before decode and before `sync` acknowledgement;
- a vitalsd timeout reports durability unknown and never reports durable;
- malformed, absent, sentinel, and out-of-range values produce no physiological row and name the rejected value/range;
- H10 idle watchdog, EMAY keepalive timeout, AirSense response deadline, and Wellue packet deadline all advance independently;
- concurrent source failures do not cancel unrelated source sessions;
- a stale source is unknown, not good; an absent optional source does not become a healthy source.

Existing seams to preserve and move behind the shared policy include `DeviceProbe`/`CaptureBackend` in `$HOME/Source/AnxietyWatch2/crates/hr-spike/src/main.rs:325-386`, `CollectorBackend` in `$HOME/Source/AnxietyWatch2/crates/emay-collector/src/main.rs:88-108`, `bp_build::samples` in `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bp_build.rs:1-50`, and `assess_link`/`ReconnectBackoff` in `$HOME/Source/aw2-air11/crates/vitals-collectors/src/h10_reconnect.rs:56-168`.

### L1 — decoder, identity, and replay tests

- Run all registry-backed decoder fixtures and fuzz targets for BP, H10, EMAY, and Wellue. Include truncated input, extra bytes, invalid flags, IEEE-11073 absent sentinels, bad CRC/framing, non-finite values, and out-of-range values.
- Verify per-source row counts, canonical units, timestamps, source/type enums, restart-safe session ids, and sequence semantics against fixed timestamps.
- Replay every synced raw journal twice. The second replay must deduplicate by `(source,type,session_id,seq)` without dropping a legitimate reconnect session.
- Extend the existing Wellue replay only as synthetic evidence; every result remains tagged `synthetic=true` and cannot satisfy physical Wellue acceptance.
- Required existing references: `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bp_build.rs:50-169`; `$HOME/Source/aw2-air11/crates/vitals-collectors/src/h10_build.rs:40-250`; `$HOME/Source/aw2-wellue-fix/crates/wellue-replay/src/main.rs:1-15,237-258`.

### L2 — BlueZ/D-Bus fake and contract tests

Use two layers; do not call a fake a hardware acceptance test.

1. **Object-model fake:** a deterministic `org.bluez` service on a private test bus implementing only the required `ObjectManager`, `Properties`, `Adapter1`, `Device1`, `GattService1`, and `GattCharacteristic1` methods/properties/signals. Script object creation/removal, discovery state, filter changes, `Connected`, `ServicesResolved`, `RSSI`, method errors, and notification streams.
2. **D-Bus contract test:** run the real `bluer` client against that service and assert exact object paths, signatures, signal ordering, error names, cancellation, filter restoration, and notification subscription behavior. The fake must reject unlisted methods so accidental generic control calls are detected.

The fake must inject: `org.bluez.Error.InProgress`, `NotReady`, `Failed`, `DoesNotExist`, D-Bus disconnect, delayed replies, never-replied calls, object removal during GATT resolution, stale `Connected`, discovery already active, failed filter restoration, duplicate characteristic UUIDs, and a notification stream that ends or goes silent.

A compile/CI guard must also reject direct `bluer::Session`, `Adapter`, `Device`, `discover_devices`, `Device::connect`, and `set_powered` use outside the manager implementation, with a deliberate allow-list for test fixtures and the Linux adapter port. This turns “one owner” into a checkable boundary rather than a convention.

### L3 — source integration tests

The integration harness starts a fake manager, fake vitalsd socket, deterministic clocks, and one client per source. It records an append journal and manager event trace. Every test asserts both the final rows and the event ordering.

#### Required absent-Wellue/BP regression

This is the minimum reproduction of the incident, with no Wellue re-enable on the Pi:

1. Configure Wellue as absent and optional. Request its bounded discovery lease.
2. Assert discovery starts only under that lease, is not renewed indefinitely, and ends at the deadline with `target_not_advertised`.
3. While the Wellue lease is active or cooling down, inject the A&D cuff advertisement and kernel-autoconnect event.
4. Assert `Discovering` is released/false before the BP connection is expected; assert the BP path contains no `Device1.Connect` action.
5. Inject BP `Connected`, `ServicesResolved`, `0x2A35` indication, journal fsync success, vitalsd sync append, and acknowledgement.
6. Assert all present BP fields are durable, share one delivery sequence, absent fields produce no row, and the event trace proves `raw_journal_sync -> decode -> append(sync=true) -> durable_ack`.
7. Repeat with Wellue absent for at least 100 cycles and with the BP event arriving during Wellue discovery, after Wellue timeout, and during cooldown.
8. Inject Wellue service errors and manager restart between cycles. BP must still connect; the optional Wellue failure must be telemetry, not a global adapter wedge.

Acceptance evidence is the fake-bus trace, manager telemetry, BP raw-journal hash/cursor, vitalsd response trace, and final row identity set. A synthetic BP frame proves arbitration and durability only; it does not prove the physical cuff radio.

## 4. Coexistence matrix

The matrix is a test inventory, not an assertion that any row has passed. `D` means bounded discovery, `S` means sustained stream, `F` means store-and-forward/autoconnect, and `R` means read-only authenticated stream.

| Scenario | Expected policy and evidence |
|---|---|
| BP alone (`F`) | No collector `Device1.Connect`; kernel-autoconnect/manager observation, GATT resolution, indication, journal-before-ack, durable rows. |
| H10 alone (`S`) | Bounded connect/resolve/subscribe, notification watchdog, reconnect cancellation, RR gap/sequence telemetry. No H10 acceptance claim until fresh physical strap data. |
| EMAY alone (`S` + stored transfer) | Keepalive and notification path share the lease; stored records remain source-identifiable and replayable; storage sync cannot starve live status. |
| AirSense alone (`R`) | Exact FD56 service/characteristics, bounded target discovery fallback, authenticated read-only RPC, event stream and data stream remain separately observable. |
| Wellue alone (optional `S`) | Disabled by default; only a bounded lease, explicit target, raw journal, provisional status, and no safety use. |
| BP + absent Wellue | Primary regression above. `Discovering` must not remain true after the Wellue deadline; BP must connect and become durable. |
| BP + present Wellue | Wellue may use a bounded low-priority lease; BP advertisement/autoconnect is not starved. Record acquisition order, lease durations, BP latency, and Wellue release. |
| BP + H10 | BP short advertisement and H10 sustained notification coexist without H10 watchdog false positives or BP loss. |
| BP + EMAY | EMAY bidirectional keepalive cannot suppress BP autoconnect; BP durability is measured independently of EMAY live status. |
| BP + AirSense | AirSense discovery fallback is serialized and bounded; BP remains available during AirSense reconnect. |
| H10 + EMAY | Measure notification gaps, queue/backpressure, CPU, and controller errors; this is specifically called out as a possible loss case in `POLAR_H10.md:55-57`. |
| H10 + AirSense | AirSense reconnect/discovery cannot interrupt H10 subscription; any H10 loss must be named and recovered. |
| EMAY + AirSense | EMAY keepalive and AirSense RPC writes are serialized without starvation; both status streams distinguish link from measurement. |
| Wellue absent + each of H10, EMAY, AirSense | Repeated absent-target attempts never own the adapter indefinitely or change another source's state. |
| All five configured, Wellue absent | Full arbitration test: four active/eligible sources remain healthy, Wellue remains unknown/disabled, and no scan lease leaks. |
| All five configured, Wellue present | Final physical canary only after Wellue protocol/safety gates; not part of the current acceptance claim. |
| Adapter restart/hotplug with all configured | Recreate the owner, restore only approved leases, re-register BP kernel state, mint new sessions, and report all transitions. |

Each row must run in at least these phases: cold boot, steady state, source reconnect, manager restart, `bluetoothd` restart, and adapter disappearance/reappearance. The all-sources test is not satisfied by running separate single-source tests.

## 5. Fault injection and expected result

| Injected fault | Required result |
|---|---|
| Wellue absent, repeated target discovery | Fixed bounded attempts, cooldown, `target_not_advertised`, no permanent `Discovering=yes`. |
| Discovery already active by another owner | Named lease conflict; no filter overwrite; BP and existing streams remain observable. |
| D-Bus reply timeout/disconnect | Operation deadline, cancellation/cleanup, measured-bad or unknown state; no fake success. |
| Device object disappears/reappears | Re-resolve by approved identity, never arbitrary target; new session after discontinuity. |
| `Connected=true`, `ServicesResolved=false` | `bonded_but_unusable`/equivalent named state; no physiological-good status. |
| Connect hangs or returns `InProgress` | Deadline, explicit `Disconnect` cancellation where permitted, bounded backoff, no tight loop. |
| GATT service/characteristic missing or duplicated | Fail closed with UUID/operation named; do not subscribe to a different characteristic. |
| Notification stream ends, stalls, or `Connected=false` while pending | Source-specific stale/degraded state, reconnect policy, and counter; no silent healthy state. |
| Malformed frame, invalid CRC, absent sentinel, out-of-range value | Raw frame retained where required; no physiological row; quarantine reason includes value/range. |
| vitalsd unavailable, append rejected, or append reply times out | Streaming source reports unconfirmed; BP retains raw frame and reports durability unknown, never durable. |
| Raw journal write/fsync failure or disk full | Stop/flag the affected synced source before acknowledging unrecoverable data; emit named storage blocker. |
| Manager process crash | systemd restart; leases and sessions are rebuilt; duplicate manager is refused; collectors do not independently touch BlueZ. |
| `bluetoothd` restart | Reconnect D-Bus, re-read adapter identity, restore policy, and prove BP accept-list state; no blind `Connect`. |
| USB adapter unplug/power loss | Detect controller loss, preserve unknown state, record boot/controller identity, and recover only after the physical runtime gate. |
| Duplicate collector/client | Single-instance lock or manager rejection with source/owner named; no duplicate samples or competing subscriptions. |
| Backpressure/slow subscriber | Bounded queue; BLE ingest cannot allocate without limit; drop only non-durable status updates, never acknowledged BP data. |
| OOM/resource pressure | Safety-path units retain their configured priority; source state becomes unknown/degraded, not good. |

Every injected case needs a deterministic test id, input trace hash, expected event sequence, observed event sequence, and exit code.

## 6. Telemetry and evidence bundle

### Manager metrics

Use bounded labels: generated source enum, adapter identity hash, operation, and named reason. Never log credentials or full private addresses.

- `bluez_operation_total{operation,source,result}`
- `bluez_operation_duration_seconds{operation,source}`
- `bluez_discovery_lease_active{adapter,owner}` and `bluez_discovery_lease_expired_total`
- `bluez_discovery_conflict_total{owner,blocker}`
- `bluez_device_state{source,state}` where state is `measured_good`, `measured_bad`, or `not_measured`
- `bluez_connect_attempt_total`, `bluez_connect_cancel_total`, `bluez_services_resolved_total`
- `bluez_notification_total`, `bluez_notification_gap_seconds`, `bluez_notification_timeout_total`
- `bluez_dbus_error_total{operation,error_name}` and `bluez_adapter_recovery_total{reason}`
- `bp_raw_journal_total`, `bp_raw_journal_sync_fail_total`, `bp_durable_ack_total`, `bp_durability_unknown_total`
- `source_samples_total`, `source_quarantined_total{reason}`, `source_reconnect_total{reason}`

### Structured event fields

`trace_id`, `manager_pid`, `boot_id`, `adapter_identity_hash`, `source`, `operation`, `phase`, monotonic start/end, deadline, result, named blocker/value, session id, sequence/cursor, and raw-journal offset. Do not use packet arrival time as identity; retain it only as evidence/annotation in accordance with `docs/specs/AMENDMENTS.md:64-90`.

### Required evidence bundle per run

- command line, exact commit/artifact hashes, test id, start/end UTC, exit code;
- fake-bus script and event trace hash for L2/L3;
- manager and collector logs with credentials/addresses redacted;
- raw journal path, byte count, fsync result, hash, and replay cursor;
- vitalsd append replies and final row identity counts;
- systemd unit status/PID/restart count and manager single-instance witness;
- on Pi: `boot_id`, kernel/BlueZ versions, `btmgmt info`, `bluetoothctl show`, adapter USB identity, rfkill state, and bounded `btmon`/D-Bus trace;
- for soak tests: per-source notification count, gaps, reconnects, malformed/quarantined count, stale intervals, controller errors, and resource high-water marks.

## 7. Acceptance criteria

A rewrite candidate is not accepted unless every applicable command exits zero and its evidence bundle is retained.

1. **Portable correctness:** decoder, registry, row-builder, state-machine, and fault-injection tests pass; no warning under `cargo clippy --workspace --all-targets --all-features -- -D warnings`; project coverage gates pass (`cargo llvm-cov --workspace --all-features --fail-under-file-lines 90 --fail-under-lines 95 --summary-only`).
2. **Ownership:** static guard proves no collector outside the manager boundary opens/mutates BlueZ; one-manager and duplicate-manager tests pass.
3. **D-Bus contract:** fake-service tests pass for all listed methods, properties, signals, error names, deadlines, cancellation, and cleanup. No arbitrary method/object-path escape hatch exists.
4. **BP durability:** absent-Wellue regression passes repeatedly; no BP `Device1.Connect` action exists; every acknowledged BP frame has a preceding raw-journal fsync and durable vitalsd reply; a timeout is reported as unknown.
5. **No discovery starvation:** every discovery lease has an owner/deadline; after absent Wellue, discovery is false or owned by a recorded active lease; other sources continue.
6. **Coexistence:** every matrix row needed for the deployment exits zero, including the combined-source rows and restart/hotplug phases. A single-source pass is insufficient.
7. **Fault safety:** all injected errors produce named, bounded, non-reassuring outcomes and no unbounded retry, scan, allocation, or duplicate session.
8. **Runtime soak:** a staged soak records per-source gaps, reconnects, and stale state. Exact duration and gap SLOs must be approved from the source cadence before implementation; no result may be retroactively reinterpreted as H10 acceptance.
9. **H10 gate:** remains blocked until a fresh, nonzero physical strap measurement and the project’s separate physical H10 gate pass. Existing unit/fake/emulator results are not H10 acceptance.
10. **Wellue gate:** remains disabled until physical GATT/protocol, timestamp/PI semantics, reconnect, no-finger, battery, and safety-use checks pass. Synthetic replay never opens this gate.

## 8. Pi-specific runtime gates (planned, not run)

| Gate | Evidence required | Current status |
|---|---|---|
| P0 inventory | Librarian answer, authoritative repo/commit, active units, exact binary hashes, no duplicate collectors | Unknown; request issued |
| P1 controller identity | Physical USB identity/OUI/path, `btmgmt info`, `hciconfig`/`bluetoothctl` adapter state, BlueZ/kernel versions | Unknown; no Pi action taken |
| P2 G1 safety | Verify Sena is UP as `hci1` before any `disable-bt` action. Never apply the overlay during this investigation. Reconcile this with deployed `hci0` rules first. | Closed |
| P3 D-Bus/BlueZ health | `bluetoothd` active, system bus healthy, adapter powered, rfkill clear, manager single-instance witness | Not run |
| P4 identity/pairing | Approved bonds/IRKs and address types for BP/H10/AirSense; no opportunistic pairing; exact source mapping | Not run; do not change bonds |
| P5 resource/safety | systemd sandbox, OOM priorities, memory/disk headroom, raw-journal permissions, spool lock, vitalsd readiness | Not run |
| P6 bounded canary | Operator-authorized finite run, no Wellue enablement, no firmware/overlay changes, exact logs/metrics/rows captured | Not run |
| P7 reboot/hotplug | Separate authorized run proving adapter reappearance, manager recovery, BP accept-list restoration, new sessions, and no duplicate units | Not run |
| P8 physical source gates | Fresh BP measurement, H10 nonzero HR/RR, EMAY live/stored evidence, AirSense bounded read-only evidence, Wellue only after its gate | Not run; H10 and Wellue acceptance not claimed |

The existing Pi transfer helper correctly states that BlueZ/systemd/PostgreSQL work must run on Linux/Pi rather than macOS: `$HOME/Source/aw2-air11/scripts/sync-to-pi.sh:1-29`. It must not be used to perform this investigation’s prohibited runtime changes.

## 9. Staged migration, canaries, and rollback

### Stage 0 — inventory and golden traces

- Answer the librarian request and reconcile repository/deployment drift.
- Capture sanitized current traces from code/tests and existing raw journals; do not enable Wellue or touch the Pi.
- Freeze current acceptance baselines: BP durability, H10 unknown status, EMAY status, AirSense status, and Wellue-disabled state.

**Exit:** authoritative entry-point inventory and replayable incident trace, or stop.

### Stage 1 — pure manager policy and fake D-Bus

- Implement the policy/state machine and ports behind the proposed interfaces.
- Add deterministic tests, fake D-Bus contract tests, static ownership guard, and coverage.
- Run synthetic BP/EMAY/H10/AirSense/Wellue event scripts; do not use synthetic data as physical acceptance.

**Exit:** all portable/contract commands exit zero and no direct collector BlueZ mutation remains in the candidate design.

### Stage 2 — observer-only canary

- Run a non-mutating observer against the existing BlueZ event surface, if the runtime owner can be proven safe. It must not start discovery, connect, pair, power-cycle, or enable services.
- Compare observed adapter/device events with the fake contract and record telemetry shape.

**Exit:** no unexpected object paths/signals or missing required facts. If observer isolation cannot be proven, skip this stage rather than create a second owner.

### Stage 3 — one-source manager canary

- Stop the legacy source owner under an explicit operator-controlled window; never run old and new mutating owners together.
- Canary first with a non-Wellue source and deterministic rollback artifact. BP is a separate durability canary, not a generic stream canary.
- Leave Wellue disabled. Do not use the canary to claim H10 acceptance.

**Exit:** source-specific live evidence, manager telemetry, no lease leak, no duplicate rows, and clean rollback rehearsal.

### Stage 4 — BP plus absent-Wellue canary

- With Wellue still disabled/absent, run the physical BP path only after P0-P6 are open.
- Reproduce the incident under the manager, prove BP durable frames, and verify no `Device1.Connect` from the BP client.
- Repeat after a manager restart and, separately, after an approved adapter hotplug/reboot test.

**Exit:** BP durability and anti-starvation evidence meet the approved SLO; otherwise revert immediately.

### Stage 5 — source-by-source and pairwise rollout

- Add EMAY, AirSense, and H10 one at a time; run their single-source and pairwise rows before adding the next source.
- AirSense lifecycle/therapy-state anomalies remain a separate protocol/state problem unless the trace proves a BlueZ arbitration cause.
- H10 physical qualification remains a distinct gate; no “manager can connect” result is H10 acceptance.

**Exit:** required coexistence rows, fault injections, restart evidence, and stale/unknown semantics pass.

### Stage 6 — Wellue requalification (future, gated)

- Only after a recorded operator decision and all physical Wellue gates pass may the optional Wellue lease be enabled.
- First run inventory/protocol capture, then bounded live stream, then BP coexistence, then longer soak. Any failure returns to disabled state.

**Exit:** physical Wellue evidence, safety review, and all-source matrix pass. This stage is not authorized by this document.

### Rollback

Trigger rollback on any unacknowledged BP frame, unexpected `Discovering=yes`, manager duplication, unknown state rendered/scored as good, unbounded retry/scan, raw-journal/fsync failure, or an unexplained source gap beyond the approved gate.

1. Stop the new manager/client units in the controlled canary window; do not kill unrelated services.
2. Preserve logs, telemetry, raw journals, spool files, unit status, and controller evidence before cleanup.
3. Restore the last known-good binaries and unit files from their recorded artifact hashes. Never run old and new BlueZ mutators simultaneously.
4. Restore the prior BP kernel accept-list registration path and verify it by evidence, not by assumption.
5. Confirm `bluetoothd`, adapter identity, `Discovering`, BP connection state, vitalsd, and spool health. Do not re-enable Wellue as a rollback probe.
6. Re-run the deterministic replay and the smallest safe BP durability check; do not claim recovery until its command exits zero.
7. Keep all append-only data. Do not `UPDATE` or `DELETE` samples; use raw-journal replay/dedup and a new session after any discontinuity.

## 10. Unresolved questions and exact files to inspect/change

### Unresolved questions

1. Which repository and branch is authoritative for the Linux rewrite and deployment?
2. What controller is physically active now, and is its stable adapter name `hci0` or `hci1` after boot and hotplug?
3. Which units are currently active, and is Wellue definitely disabled on the Pi?
4. Does the future owner perform privileged kernel accept-list management, or does a separate helper remain? Either way, how is the BP no-`Device1.Connect` invariant mechanically enforced?
5. What BlueZ/kernel/bluer versions and controller connection/discovery limits are proven on the appliance?
6. What exact latency, gap, and recovery SLOs are approved for BP, H10, EMAY, AirSense, and optional Wellue?
7. What is the authoritative sanitized evidence bundle for the reported absent-Wellue/BP incident?
8. Can a private fake `org.bluez` service be run in CI with the exact deployed D-Bus policy, or is a Linux contract runner required?
9. Which AirSense discovery and therapy lifecycle observations are transport failures versus protocol/session-state failures?
10. Which physical Wellue packet/GATT evidence, if any, is authoritative? Current Wellue documentation explicitly says protocol and safety semantics remain provisional.

### Exact files to inspect/change during implementation

**Thin-client / governing documentation**

- `docs/reviews/bluetooth-rewrite/BRIEF.md`
- `docs/specs/AMENDMENTS.md`
- `docs/reviews/bluetooth-rewrite/requests/2026-09-22-reliability-entry-points.md`
- `docs/reviews/bluetooth-rewrite/RELIABILITY-TEST-PLAN.md`

**AnxietyWatch2 early/stopgap BlueZ entry points**

- `$HOME/Source/AnxietyWatch2/crates/hr-spike/src/main.rs` — `DeviceProbe`, `CaptureBackend`, discovery, service wait, notification loop, and Linux backend (`:135-142,207-285,325-411`).
- `$HOME/Source/AnxietyWatch2/crates/emay-spike/src/main.rs` — discovery and bidirectional test backend (`:329-432,565-620`).
- `$HOME/Source/AnxietyWatch2/crates/emay-collector/src/main.rs` — `CollectorBackend` and production BlueZ backend (`:88-108,1358-1437`).
- `$HOME/Source/AnxietyWatch2/deploy/stopgap/vitals-bp-connect` — legacy Python D-Bus BP bridge (`:44-55,99-149,183-200`).
- `$HOME/Source/AnxietyWatch2/deploy/stopgap/vitals-bp-connect.service` and `deploy/vitals-emay.service` — stopgap ownership/order.

**aw2-air11 current collectors and deployment**

- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs`
- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bp_build.rs`
- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs`
- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/h10_build.rs`
- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/h10_reconnect.rs`
- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/as11_bluez.rs`
- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs`
- `$HOME/Source/aw2-air11/crates/vitals-collectors/src/as11_status.rs`
- `$HOME/Source/aw2-air11/deploy/vitals-bp.service`
- `$HOME/Source/aw2-air11/deploy/vitals-bp-autoconnect.service`
- `$HOME/Source/aw2-air11/deploy/99-vitals-bp-autoconnect.rules`
- `$HOME/Source/aw2-air11/deploy/vitals-h10.service`
- `$HOME/Source/aw2-air11/deploy/vitals-air11.service`
- `$HOME/Source/aw2-air11/scripts/sync-to-pi.sh`
- `$HOME/Source/aw2-air11/scripts/air11-pi-acceptance.sh`

**aw2-wellue-fix provisional source**

- `$HOME/Source/aw2-wellue-fix/crates/wellue-collector/src/main.rs`
- `$HOME/Source/aw2-wellue-fix/crates/wellue-probe/src/main.rs`
- `$HOME/Source/aw2-wellue-fix/crates/wellue-replay/src/main.rs`
- `$HOME/Source/aw2-wellue-fix/scripts/wellue-replay-test.sh`
- `$HOME/Source/aw2-wellue-fix/deploy/wellue-collector.service`
- `$HOME/Source/aw2-wellue-fix/docs/hardware/WELLUE_KS60F.md`
- `$HOME/Source/aw2-wellue-fix/docs/setup-logs/wellue-ks60f.md`

No file in these implementation/deployment lists should be changed as part of this investigation. The next change is blocked on the unresolved controller/ownership facts and the physical/runtime gates above.
