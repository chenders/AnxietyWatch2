# Linux BlueZ arbitration and rewrite architecture

**Investigation status:** design-only. No Rust, SQL, systemd, udev, firmware, hardware, Pi, or
runtime changes were made. The librarian inventory is recorded in `docs/reviews/bluetooth-rewrite/LIBRARIAN.md`; its request
and answer files identify the evidence that remains unknown and requires physical/runtime validation.

## 1. Decision summary

The current model has no adapter-wide owner. Each BLE process opens its own `bluer`/D-Bus session
and can independently discover, connect, disconnect, subscribe, write, retry, or reset the adapter.
That is sufficient for isolated prototypes but cannot enforce a policy over one shared controller.
The incident recorded in the brief is therefore a systemic arbitration failure: discovery held `yes`
while Wellue retried, then changed to `no` immediately when Wellue stopped, and BP resumed without a
restart or manual connect ([`BRIEF.md:14-20`](BRIEF.md)).

**Recommended architecture:** a **per-adapter broker and lease scheduler**, with one broker process
owning all application-level BlueZ access for that adapter, separate source workers retaining
protocol decoding and sample policy, and a separate narrowly privileged kernel-policy boundary for
BP's LE auto-connect accept list. A logical system registry may supervise multiple brokers, but it
must not become a second BlueZ client.

The broker is warranted because it establishes the missing invariants:

- at most one discovery owner and one adapter discovery policy;
- one serialized operation stream per device, with bounded adapter-wide connect admission;
- source priorities, cooldowns, deadlines, and fair scheduling in one place;
- no collector can bypass arbitration by calling `StartDiscovery`, `Device1.Connect`, or
  `Adapter1.SetPowered` directly;
- a failing optional source cannot reset or power-cycle a controller serving other sources;
- per-source retries are isolated from adapter-global operations;
- every operation has an owner, lease, deadline, generation, result, and observable blocker.

A broker does **not** make one physical adapter independent. A controller reset, BlueZ failure, USB
loss, RF collision, or power fault still affects all links on that adapter. The architecture must
expose that shared failure domain and preserve source-level unknown states; it must not claim that a
lease is safety independence.

Wellue remains disabled. H10 remains unaccepted because fresh strap measurements are absent. AirSense
BLE lifecycle/data behavior is treated as a separate protocol/session question, not as evidence that
discovery arbitration is the cause. No section below authorizes enabling either source.

## 2. Evidence and inventory

### 2.1 Repository boundary and evidence quality

The named repositories are not a single internally consistent revision:

- The primary `AnxietyWatch2` README describes the production collector fleet as incomplete while
  listing BLE capture spikes and the target Sena UD100-G03 hardware
  ([`AnxietyWatch2/README.md:14-31`](../../../../AnxietyWatch2/README.md),
  [`AnxietyWatch2/README.md:121-122`](../../../../AnxietyWatch2/README.md)).
- Its operational registry records a 2026-09-05 observation of only a powered `hci0`, no required
  `hci1`, H10 bonded-but-disconnected, and EMAY unavailable; it explicitly says this is not a healthy
  Pi claim ([`AnxietyWatch2/docs/ops/log-registry.md:62-78`](../../../../AnxietyWatch2/docs/ops/log-registry.md)).
- The Air11 tree records a different intended state: the Sena is the sole controller and enumerates
  as `hci0` after an accepted hardware change ([`aw2-air11/docs/HERDR-GUIDE.md:239-242`](../../aw2-air11/docs/HERDR-GUIDE.md)).
- Other design text still describes a future Sena `hci1` and disabling onboard Bluetooth
  ([`aw2-air11/docs/specs/2026-08-03-pi5-vitals-core-design.md:339-344`](../../aw2-air11/docs/specs/2026-08-03-pi5-vitals-core-design.md)).

These are repository/runtime records, not a verified current controller inventory. **Unknown:**
which tree corresponds to the currently installed artifact, which physical controller is present,
its stable identity, BlueZ/kernel versions, and which units are enabled. No Pi action is permitted in
this investigation.

### 2.2 Current access paths by source

| Source | Observed application access | Discovery/connect authority today | Arbitration finding |
|---|---|---|---|
| **A&D BP** | `bp-collector` creates a BlueZ session, powers the default adapter, obtains a `Device`, waits for `Connected`/`ServicesResolved`, then calls `Characteristic::notify`; it explicitly never calls `Device1.Connect` ([`AnxietyWatch2/crates/vitals-collectors/src/bin/bp_collector.rs:1-16,249-301`](../../../../AnxietyWatch2/crates/vitals-collectors/src/bin/bp_collector.rs)). | A root oneshot runs `btmgmt add-device -a 2 -t 1`; a udev rule reruns it only for `KERNEL=="hci0"` ([`AnxietyWatch2/deploy/vitals-bp-autoconnect.service:1-20`](../../../../AnxietyWatch2/deploy/vitals-bp-autoconnect.service), [`AnxietyWatch2/deploy/99-vitals-bp-autoconnect.rules:1-4`](../../../../AnxietyWatch2/deploy/99-vitals-bp-autoconnect.rules)). An older stopgap independently polls RSSI and calls `Device1.Connect` ([`AnxietyWatch2/deploy/stopgap/vitals-bp-connect:1-37,141-194`](../../../../AnxietyWatch2/deploy/stopgap/vitals-bp-connect)). | The intended kernel boundary is correct for the store-and-forward cuff; the old stopgap is a bypass and must not coexist with the new policy. Kernel accept-list state is controller state, not a durable BlueZ pairing property. |
| **Polar H10** | Direct `device.connect()`, waits for service resolution, discovers `0x180D/0x2A37`, subscribes, monitors `Connected` and notification idle, then disconnects on timeout ([`AnxietyWatch2/crates/vitals-collectors/src/bin/h10_collector.rs:240-333,417-470`](../../../../AnxietyWatch2/crates/vitals-collectors/src/bin/h10_collector.rs)). | The collector owns connect/reconnect; systemd grants `AF_BLUETOOTH` ([`AnxietyWatch2/deploy/vitals-h10.service:1-30`](../../../../AnxietyWatch2/deploy/vitals-h10.service)). | Per-device timeout and backoff exist, but there is no adapter-wide connect admission or lease. This is not H10 acceptance. |
| **EMAY SleepO2** | The spike scans, connects, writes a start sequence, sends a keepalive every 1.2 s, and declares a two-minute silence; the same direct discovery/connect shape exists in the production collector lineage ([`AnxietyWatch2/crates/emay-spike/src/main.rs:249-308,447-521`](../../../../AnxietyWatch2/crates/emay-spike/src/main.rs)). | `emay-live`/related collector owns discovery and direct connect; the historical service is explicitly a stopgap ([`AnxietyWatch2/deploy/stopgap/emay-live.service:1-25`](../../../../AnxietyWatch2/deploy/stopgap/emay-live.service)). | The keepalive stream is a long-lived connection, not a reason to hold discovery. Its discovery phase must be a short broker lease; once connected it must not starve BP or other discovery. |
| **Wellue/Viatom** | The Wellue collector calls `discover_devices()` for a ten-second window, directly connects, resolves GATT, subscribes, retries every minute, and may reset the adapter after selected failures ([`aw2-wellue-fix/crates/wellue-collector/src/main.rs:34-44,214-239,438-545,632-651`](../../aw2-wellue-fix/crates/wellue-collector/src/main.rs)). | Its unit is independently startable and asks for `bluetooth.target`; it is configured through `WELLUE_MAC` ([`aw2-wellue-fix/deploy/wellue-collector.service:1-30`](../../aw2-wellue-fix/deploy/wellue-collector.service)). | Bounded discovery/cooldown is only a local mitigation. Adapter reset is an unsafe global side effect for a source that is optional and currently disabled. It must move behind broker policy and remain disabled pending its separate live evidence gate. |
| **AirSense 11** | The Air11 tree has a direct BlueZ transport: exact target connect, bounded FD56-filtered discovery fallback, GATT TX/RX resolution, notification subscription, and disconnect ([`aw2-air11/crates/vitals-collectors/src/as11_bluez.rs:389-419,524-530,688-775`](../../aw2-air11/crates/vitals-collectors/src/as11_bluez.rs)). | It refuses to start fallback discovery when `Adapter1` already reports discovery active, rather than coordinating with the existing owner ([`aw2-air11/crates/vitals-collectors/src/as11_bluez.rs:688-713`](../../aw2-air11/crates/vitals-collectors/src/as11_bluez.rs)). The service pins `--adapter hci0` and permits BlueZ access ([`aw2-air11/deploy/vitals-air11.service:3-17,44`](../../aw2-air11/deploy/vitals-air11.service)). | This is a correct fail-closed local check but not arbitration: it can lose indefinitely to another process, and another process can begin discovery immediately afterward. The current `streaming` versus `therapy complete` symptom must remain a separate state/event investigation. |

The inventory remains incomplete despite the librarian report. In particular, no authoritative
answer currently establishes every enabled service, every subprocess that can touch BlueZ, all
adapter names in the installed deployment, or whether an external operator tool is active. The
request and answer files name those exact questions and distinguish repository evidence from runtime gates.

### 2.3 Kernel autoconnect and BP durability boundary

The BP boundary is not optional:

1. The cuff advertises briefly and is store-and-forward; the kernel accept list is intended to catch
   that window. The capture tool records that userspace `bluetoothctl connect` did not issue an LE
   create-connection command, while `btmgmt add-device -a 2` is the observed kernel path
   ([`AnxietyWatch2/tools/bp_capture.py:1-16,434-440`](../../../../AnxietyWatch2/tools/bp_capture.py)).
2. Amendment 06 makes the identity a per-connection drain index and acknowledges that ATT
   confirmation occurs before the application can durably store the frame
   ([`docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md:43-58,79-84,148-154`](../../../specs/2026-08-03-amendment-06-bp-identity-and-durability.md)).
3. Therefore `bp-collector` must receive the indication, journal raw bytes before decode, append with
   `sync: true`, and wait for the durable reply; it must never regain a direct `Connect` path
   ([`AnxietyWatch2/crates/vitals-collectors/src/bin/bp_collector.rs:4-16,89-165,226-301`](../../../../AnxietyWatch2/crates/vitals-collectors/src/bin/bp_collector.rs), [`docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md:134-154`](../../../specs/2026-08-03-amendment-06-bp-identity-and-durability.md)).
4. A bonded device that reaches `Connected` but never resolves services within ten seconds is a
   named degraded state, not healthy liveness ([`docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md:167-185`](../../../specs/2026-08-03-amendment-06-bp-identity-and-durability.md)).

The broker may observe the kernel-created `Device1` and own `StartNotify`, but it must not issue a
BP connect request. A small privileged kernel-policy component may maintain the accept list; it must
not be given generic D-Bus or source-collector authority. The ATT ACK-to-journal loss window remains
a fact even after arbitration is fixed.

## 3. Why local fixes are insufficient

The repositories already contain useful local controls:

- H10 has a 30-second connect deadline, cancellation via `Device1.Disconnect`, notification-idle
  detection, and capped backoff ([`AnxietyWatch2/crates/vitals-collectors/src/bin/h10_collector.rs:31-46,240-303,438-470`](../../../../AnxietyWatch2/crates/vitals-collectors/src/bin/h10_collector.rs)).
- Wellue bounds its scan and retries ([`aw2-wellue-fix/crates/wellue-collector/src/main.rs:28-44,688-708`](../../aw2-wellue-fix/crates/wellue-collector/src/main.rs)).
- Air11 refuses to scan while another discovery is active ([`aw2-air11/crates/vitals-collectors/src/as11_bluez.rs:688-713`](../../aw2-air11/crates/vitals-collectors/src/as11_bluez.rs)).

None of these controls owns the shared resource. A process can still start discovery before another
process checks, leave a changed discovery filter behind, initiate a connect while a different source
is in a short advertising window, or reset the adapter while unrelated links are active. The brief's
stop-Wellue/BP-recovery sequence is the strongest current causal evidence, but it does not by itself
prove whether BlueZ reference counting, controller scan scheduling, kernel accept-list programming,
or a particular driver behavior is the immediate mechanism. The rewrite should remove the race
rather than depend on identifying one exact mechanism.

## 4. Candidate rewrite architectures

### A. Independent collectors with advisory lock — **reject**

Add a filesystem or D-Bus lock that collectors voluntarily acquire around discovery/connect.

- **Pros:** small diff; keeps source workers independent.
- **Cons:** cannot prevent an old binary, diagnostic tool, or future collector from bypassing it;
  cannot enforce adapter filter restoration; does not centralize retry budgets or controller reset;
  cannot make leases observable. This repeats the current failure mode with a convention.

### B. Shared broker/lease scheduler over BlueZ — **recommend**

One broker process per physical adapter owns the BlueZ D-Bus session. Collectors become source
workers with no BlueZ dependency beyond a broker client. The broker serializes adapter-global
operations and device-local GATT operations, while workers retain source-specific protocol, decode,
identity, durable-journal, and `vitalsd` append behavior.

- **Pros:** enforceable single owner; explicit discovery leases; fair priority; bounded retries;
  no optional source can reset the adapter; process-level worker isolation; one place for controller
  generation and metrics; migration can be source-by-source.
- **Cons:** broker is a new shared failure point; all links on one adapter still share hardware;
  broker protocol and bounded frame backpressure must be designed carefully; BlueZ behavior still
  needs Linux/Pi validation.
- **Failure isolation:** use one broker process per adapter, not one global process; keep each source
  worker separate; broker connection state is per device; a worker crash releases only its leases.
  A broker crash is an adapter-domain outage and must trigger explicit degraded state for all leases,
  bounded restart, and no false healthy samples.

### C. Single monolithic collector supervisor — **conditional alternative**

One process owns BlueZ and runs BP/H10/EMAY/Wellue/AirSense tasks internally, sharing a scheduler.

- **Pros:** simplest internal API and lowest D-Bus overhead; easy global ordering.
- **Cons:** a panic, deadlock, memory leak, or unbounded task in one source can affect every source;
  deployment and rollback are all-or-nothing; source-specific credentials and protocol bugs share a
  failure domain. It is acceptable only as a laboratory harness, not as the production safety path.

### D. Dedicated adapter per source — **physical isolation option, not the first rewrite**

Assign H10, SleepO2, BP, and other sources to separate USB controllers and run current direct
collectors independently.

- **Pros:** isolates controller failures and reduces RF scheduling pressure; can be the right answer
  if measured concurrent links lose packets.
- **Cons:** requires verified hardware, stable identity, USB power/placement, explicit adapter
  binding, and potentially separate BlueZ policy; it does not solve discovery races on any one
  adapter; it increases physical/runtime complexity. Current repository evidence conflicts on hci0/
  hci1, so it cannot be assumed.
- **Use:** retain as an escalation path after measurement, or use a second broker for a verified
  second adapter. Do not apply an onboard-Bluetooth overlay as part of this review; Gate G1 remains
  absolute.

### E. Replace BlueZ with raw kernel management/GATT — **reject for production**

Use raw management/HCI or a custom GATT implementation for all sources.

- **Pros:** potentially direct control of scanning and kernel accept-list behavior.
- **Cons:** recreates pairing, IRK, GATT cache, ATT indications, encryption, MTU, and reconnect
  semantics; duplicates BlueZ's security and device-object logic; leaves no credible path for the
  AirSense authenticated GATT or source coexistence without a much larger unvalidated stack. Keep
  raw `btmgmt` limited to the privileged BP controller policy boundary.

## 5. Proposed architecture and boundaries

```text
                       privileged kernel-policy boundary
                   (narrow btmgmt accept-list operations)
                                      │
                                      ▼
  BlueZ / adapter hciX ◀── vitals-ble-broker-hciX ──┬── BP worker
       one D-Bus owner                                ├── H10 worker
                                                      ├── SleepO2 worker
                                                      ├── Wellue worker (disabled)
                                                      └── AirSense worker
                                                           │
                                                           ▼
                                      vitalsd Unix socket / COPY spool
```

### 5.1 Ownership rules

**Broker owns:**

- one BlueZ D-Bus session and one selected adapter object;
- `Adapter1` power/discovery filter/start/stop operations;
- all `Device1` connect/disconnect operations except the BP kernel-autoconnect prohibition;
- service resolution, characteristic lookup, notification/indication subscription, and GATT writes;
- per-adapter and per-device operation queues, deadlines, cancellation, retry admission, and lease
  expiry;
- adapter generation changes and recovery classification;
- broker status, metrics, operation IDs, lease records, and source-neutral raw frame delivery.

**Source worker owns:**

- device-specific protocol state after the broker supplies the GATT stream;
- pure decoder, registry conversion/range checks, source identity, session/sequence policy;
- source-specific raw journal policy where not moved into the broker transport journal;
- `vitalsd` append and its durability contract;
- source status and liveness/measurement distinction;
- no BlueZ object, adapter power, discovery, connect, disconnect, or reset capability.

**Kernel-policy helper owns:**

- only the BP (and any future explicitly approved store-and-forward devices) kernel accept-list
  operation through `btmgmt`/management API;
- controller selection, address type, registration result, and hotplug re-registration;
- no generic `Device1.Connect`, no GATT, no source data, no database, and no adapter reset.

Initially this can remain the existing boot/hotplug boundary represented by
`vitals-bp-autoconnect.service` and `99-vitals-bp-autoconnect.rules`; the rewrite should make the
boundary explicit and instrumented before considering a smaller privileged helper. A broker must
consume its state, not silently reproduce it as an unprivileged approximation.

### 5.2 Broker client interface (proposed, typed and versioned)

The exact wire encoding is a later implementation decision; the semantic API should be fixed first.
Use a private Unix socket, bounded frames, protocol version, client identity, and a broker generation.
No raw D-Bus paths or generic method names should cross this interface.

```text
Hello {
  protocol_version,
  worker_kind,              // bp | h10 | sleepo2 | wellue | air11
  adapter_selector,
  device_selector,          // configured opaque reference; logs redact address
  requested_capabilities
}

Acquire {
  intent,                   // kernel_observe | discover | connect | gatt | stream
  priority,
  deadline,
  max_hold,
  preemptible,
  reason
} -> Lease { lease_id, adapter_generation, expires_at }

ObserveDevice / DiscoverTarget / Connect / Resolve / Subscribe / Write
  // only operations permitted by the lease and source policy

Frame {
  lease_id,
  adapter_generation,
  device_session,
  characteristic_uuid,
  indication_or_notification,
  received_monotonic,
  received_utc,
  payload               // bounded raw GATT bytes; no unbounded buffering
}

Release { lease_id, reason }
Cancel  { lease_id, operation_id, reason }
Health  {} -> BrokerHealth
```

The broker must not expose a generic `CallD-Bus(method, path, args)` escape hatch. The BP worker's
capability set must omit `connect`; it receives `ObserveDevice` and `Subscribe` only after kernel
connection. `SetPowered`, adapter reset, pair/trust, discovery-filter mutation, and kernel policy
are broker/admin or privileged-policy operations, never worker operations.

`Frame` delivery requires bounded per-worker queues. A slow worker gets a named backpressure result
and its lease is released; the broker must not block all notification streams on one worker. For BP,
the raw frame must be in a durable journal before the broker acknowledges delivery as accepted to a
worker. This preserves Amendment 06's raw-frame-before-decode invariant while recognizing that the
broker is now the transport owner. High-rate H10/AirSense journals may use the accepted bounded
fsync policy; BP retains per-frame durability.

### 5.3 Lease and scheduling policy

The scheduler has two layers:

1. **Adapter-global leases:** discovery, discovery-filter mutation, controller power/recovery, and
   admission of a new connect attempt. Only one discovery lease exists. A controller recovery lease
   excludes all other operations and is never available to an optional source's ordinary retry.
2. **Device-local leases:** connect/disconnect, service resolution, GATT operation, and stream
   ownership for one device. GATT writes are serialized per device. Independent established links
   may continue while another device is being discovered, subject to measured controller capacity.

Suggested policy, subject to physical timing validation:

| Priority | Source/operation | Policy |
|---|---|---|
| P0 | BP kernel-autoconnect observation and indication subscription | Never initiate connect; do not preempt a connected BP drain; broker observes and subscribes promptly. |
| P1 | Active H10 and SleepO2 streams | Preserve established stream and keepalive; bounded reconnect attempts; no discovery while a reconnect window has a higher-priority owner. |
| P2 | AirSense authenticated stream | Bounded connect/discovery and authenticated stream; protocol/lifecycle state remains source-owned. |
| P3 | Wellue | Disabled until separately authorized; when enabled later, one bounded discovery lease, long cooldown, no adapter reset, and no unbounded retry. |
| P4 | Diagnostics/onboarding | Explicit operator lease only; never silently competes with safety sources. |

A lower-priority discovery lease may be preempted only before the source is connected and only at a
safe operation boundary. The broker must not stop an established H10/SleepO2/AirSense stream merely
to search for an absent optional device. A failed operation releases its lease before retry backoff;
backoff is not a held lease.

## 6. State machines

### 6.1 Adapter broker

```text
STARTING
  -> ADAPTER_UNAVAILABLE       (BlueZ/session/adapter deadline)
  -> READY

READY
  -> DISCOVERY_LEASED          (one owner, filter snapshot, deadline)
  -> CONNECT_ADMISSION         (one bounded new-connect slot)
  -> DEGRADED                  (BlueZ error, generation changed)

DISCOVERY_LEASED
  -> TARGET_FOUND -> RELEASE_FILTER -> READY
  -> DEADLINE/ERROR -> RESTORE_FILTER -> READY or DEGRADED
  -> PREEMPT_REQUESTED -> stop only at safe boundary -> RESTORE_FILTER -> READY

CONNECT_ADMISSION
  -> CONNECTED -> DEVICE_OWNED
  -> TIMEOUT/ERROR -> CANCEL -> BACKOFF (source-local)
  -> ADAPTER_FAILURE -> DEGRADED

DEGRADED
  -> RECOVERY_LEASED           (only supervisor/admin, named reason/value)
  -> READY                     (verified generation and adapter properties)
  -> ADAPTER_UNAVAILABLE       (recovery failed)
```

`DISCOVERY_LEASED` must expose the previous filter, requested filter, owner, start/end monotonic
clock, and restore result. If filter restore fails, that is a broker/adapter degraded state, not a
successful timeout. An already-active discovery is no longer reported merely as “busy”; health says
`discovery lease denied: owner=<redacted source>, generation=<n>, deadline=<...>`.

### 6.2 Per-device source session

```text
IDLE
  -> WAITING_FOR_BROKER_LEASE
  -> DISCOVERING (only if broker granted discovery)
  -> CONNECTING
  -> CONNECTED
  -> SERVICES_RESOLVING
  -> SUBSCRIBING
  -> STREAMING
  -> SILENT_OR_STALE
  -> DISCONNECTING
  -> BACKOFF
  -> IDLE
```

Every edge has a deadline and named reason. `CONNECTED` is not `STREAMING`; `ServicesResolved` is
not a measurement; and old samples never turn a new session healthy. A worker receives a broker
`adapter_generation` and must discard the session on generation change. A reconnect mints a new
source session only at the existing source-specific authenticated/usable boundary.

For BP, replace `DISCOVERING -> CONNECTING` with:

```text
KERNEL_WAIT -> CONNECTED_BY_KERNEL -> SERVICES_RESOLVING -> SUBSCRIBING -> DRAINING -> KERNEL_WAIT
```

No BP worker or broker policy can transition `KERNEL_WAIT` by calling `Device1.Connect`.

### 6.3 Retry state

Retries are source-local state machines scheduled by the broker, not loops that retain a lease:

```text
FAILED(operation, class, value)
  -> CLASSIFY
     absent/not-advertising       -> cooldown
     discovery-owned              -> queued lease request
     connect-in-progress          -> cancel/observe existing operation
     services-unresolved          -> bounded disconnect, then backoff
     worker-backpressure          -> release lease, worker degraded
     adapter-generation-changed  -> await broker recovery
     protocol/credential failure  -> terminal until operator action
  -> BACKOFF(delay, attempt, jitter)
  -> REQUEST_LEASE
```

Backoff must include attempt number, last error class, source, device reference, and next eligible
monotonic time. It must not issue `SetPowered(false/true)` as a per-device retry. Controller recovery
is a separately authorized operation with a blast-radius gate.

## 7. Source coexistence and failure isolation

| Source | Desired broker contract | What can coexist | What must not happen |
|---|---|---|---|
| BP | Kernel observation lease; immediate GATT indication subscription; raw frame journal; `sync:true` append acknowledgment | Established H10/SleepO2/AirSense links, if controller measurements show adequate capacity | BP worker calls `Connect`; Wellue holds continuous discovery; another worker changes BP's device state; a broker timeout hides the ACK/durability gap. |
| H10 | Persistent stream lease; bounded connect and notification watchdog; per-session worker | BP kernel observation and SleepO2 stream in the same adapter only after packet-loss/latency evidence | Claiming acceptance from an emulator or stale connected property; H10 reconnect storm starving discovery or BP. |
| SleepO2 | Persistent stream plus scheduled keepalive writes; discovery only during initial/reconnect lease | H10 and BP if measured; no need for continuous discovery after connection | Holding discovery for the stream; allowing missed keepalive writes due to an unbounded shared queue; using live anonymous frames as durable identity without accepted source evidence. |
| Wellue | Disabled configuration. Future opt-in receives one bounded discovery/connect lease and strict cooldown | It may coexist only after live protocol, identity, and overnight evidence; it is optional and lowest priority | Re-enable it during this investigation; let it reset the adapter; run alongside an uncoordinated broker/direct collector. |
| AirSense | Exact GATT pipe and authenticated stream through broker; worker owns RPC/session lifecycle | Its established stream can coexist with others subject to measured adapter capacity | Treating `streaming` as therapy active, or treating its separate lifecycle symptom as proof of adapter contention. |

The concurrency decision must be evidence-driven. “BlueZ supports multiple connections” is not an
acceptance claim for this appliance: the H10 hardware notes specifically warn that simultaneous
oximeter/H10 links may lose packets ([`aw2-wellue-fix/docs/hardware/POLAR_H10.md:52-57`](../../aw2-wellue-fix/docs/hardware/POLAR_H10.md)). If loss or latency violates source budgets, move a
source to a verified second adapter/broker; do not hide loss with retries or interpolation.

## 8. Staged migration and rollback

All stages below are a plan, not executed work.

### Stage 0 — Freeze and inventory

- Answer the librarian request with exact files, enabled units, BlueZ objects, adapter/controller
  identity, and runtime evidence.
- Define the installed artifact revision and redact device identifiers in committed evidence.
- Establish per-source budgets from physical observations: advertising windows, connection latency,
  notification cadence, keepalive deadline, and maximum acceptable gaps.
- No Wellue enablement, H10 acceptance, adapter reset, overlay change, or Pi action.

**Rollback:** none; documentation-only.

### Stage 1 — Offline broker contract and Linux test seams

- Add the broker protocol types and pure lease/scheduler state machine tests without changing any
  runtime unit.
- Add fake transport tests for discovery ownership, filter restoration, operation cancellation,
  worker crash/lease expiry, queue saturation, adapter-generation changes, and priority fairness.
- Build against the real BlueZ API on Linux in CI/Pi preparation; do not substitute a macOS mock for
  hardware behavior.

**Gate:** all state-machine tests pass; no direct BlueZ call sites have been silently omitted from the
inventory.

**Rollback:** remove the uninstalled broker artifact; no production path has changed.

### Stage 2 — Broker observe-only and kernel-policy telemetry

- Run the broker in an explicit observe-only mode that does not start discovery, connect, power, or
  reset operations.
- Have the broker report adapter generation, current discovery state, BlueZ object events, kernel
  autoconnect registration result, and source lease requests from a test client.
- Keep BP's existing privileged registration boundary; instrument the result before replacing any
  unit. The broker must prove it can see kernel-created BP links without initiating them.

**Gate:** bounded Linux evidence shows no second discovery owner and no change to BP behavior. This is
not H10 acceptance and does not enable Wellue.

**Rollback:** stop the broker; existing approved source paths remain the only paths, with no overlapping
broker clients.

### Stage 3 — First source cutover, using a reversible low-risk path

- Cut one source at a time from direct `bluer` operations to the broker client. Use an emulator or
  controlled source fixture for protocol/stream tests first.
- Retain the source worker process and its decoder/append contract. The worker must be unable to open
  BlueZ after cutover.
- Prefer a source whose live acceptance is already separately gated; do not use this stage to claim
  H10 acceptance. Select the actual first source only after Stage 0/1 evidence and owner approval.

**Gate:** source rows/statuses remain equivalent; broker records discovery/connect/subscribe lease
transitions; unrelated sources show no discovery disruption.

**Rollback:** stop that broker client, verify its lease is released and no broker-owned connection
remains, then start the prior source path alone. Never run old and new owners simultaneously.

### Stage 4 — BP special cutover

- Move BP GATT observation and indication delivery to the broker while leaving the privileged kernel
  accept-list registration separate.
- Preserve the raw journal-before-decode and `sync:true` reply contract. Test broker crash after ATT
  delivery, worker crash, vitalsd timeout, journal ENOSPC, and adapter hotplug as distinct states.
- Retire the old direct `vitals-bp-connect` stopgap only after proving the broker never calls
  `Device1.Connect` for BP and the kernel policy is registered on the actual controller.

**Gate:** a bounded authorized physical test proves a kernel-created BP connection reaches subscription
and durable spool evidence; a broker discovery lease cannot block the cuff. This is a durability and
arbitration gate, not a claim about unrelated sources.

**Rollback:** restore the previous BP collector plus the approved kernel registration only after
stopping the broker BP client and verifying no duplicate subscription/owner exists. Do not restore the
old userspace `Connect` workaround as a hidden fallback.

### Stage 5 — Remaining sources and controller partition decision

- Cut SleepO2, AirSense, and then any separately authorized Wellue path one at a time, with source
  protocol/session tests and independent acceptance gates.
- Wellue remains disabled unless its own live protocol/identity/safety decision is accepted. H10
  remains an unaccepted source until fresh measurements satisfy its gate.
- Measure concurrent-link packet loss, notification latency, discovery-to-connect time, broker queue
  delay, and reconnect recovery. If the shared controller fails budgets, deploy a second verified
  adapter and a second per-adapter broker rather than adding more retries.

**Rollback:** source-level cutover rollback as above; if controller-level instability appears, move
only the affected source back to its prior approved path or disable it, while preserving active
safety-source leases. No global adapter power cycle as an automatic rollback.

## 9. Observability, safety gates, and failure handling

### 9.1 Required broker evidence

Every broker event carries a monotonic event id, broker generation, adapter selector, redacted source
and device reference, lease id, operation id, deadline, result class, and duration. Required metrics
and status fields include:

- adapter present/powered, controller identity evidence, BlueZ session state, generation, and last
  object-manager event;
- discovery owner, requested/restored filters, active duration, denials, preemptions, and restore
  failures;
- per-source lease wait/hold time, queue depth, deadline failures, backoff attempt, next eligible
  time, and last named error;
- per-device `Connected`, `ServicesResolved`, subscription state, notification/indication counts,
  notification age, GATT operation queue depth, and stream epoch;
- broker/worker disconnect reason, adapter-generation changes, slow-worker drops, journal result,
  and `vitalsd` append result/durability state;
- kernel-policy registration controller, address type, result, hotplug trigger, and evidence of the
  **particular value** that blocked registration;
- separate states for measured-good, measured-bad, unavailable, stale, degraded, and unknown.

Do not log credentials, session keys, full device addresses, raw AirSense authentication traffic, or
unredacted private identifiers. Raw medical frames remain in the source journal under the accepted
retention/access rules, not in general broker logs.

### 9.2 Safety gates

1. **Controller identity gate:** do not bind or change an adapter until the actual USB/controller
   identity, `hci` name, power state, and BlueZ version are verified. The conflicting hci0/hci1
   records mean code defaults are not evidence.
2. **G1 gate:** never apply `dtoverlay=disable-bt` until the Sena dongle is verified UP as `hci1`;
   the review performs no such action. Later records claiming the Sena as hci0 must be reconciled
   with the exact installed state first.
3. **Single-owner gate:** before enabling a broker client, prove no direct collector, diagnostic,
   stopgap, onboarding command, or old unit can use that adapter. A running process is not proof of a
   lease; inspect call sites and deployment ownership.
4. **Discovery gate:** no unbounded scan. Every discovery has a source, filter, deadline, owner,
   restoration result, and cooldown. A blocked lease names its current owner and generation.
5. **Connect gate:** only the broker or the kernel-policy boundary may connect. BP's broker capability
   explicitly excludes connect. Per-source retries cannot reset the adapter.
6. **Durability gate:** BP raw bytes are journaled before decode and `sync:true` is acknowledged before
   the frame is considered handled. A timeout is durability unknown, not success.
7. **Coexistence gate:** do not claim simultaneous-source support until measured overnight/finite-window
   evidence records packet loss, gaps, reconnects, keepalive misses, and source-specific sample
   freshness.
8. **Acceptance gate:** H10 fresh physical strap measurements are required; no emulator or connected
   property substitutes for acceptance. Wellue remains disabled. AirSense streaming status does not
   override its therapy lifecycle blocker.
9. **Rollback gate:** stop the new client, confirm lease release and no lingering GATT subscription,
   then start the old owner. Never permit dual ownership during rollback.
10. **Alarm gate:** broker, storage, database, phone, or display failures must produce unknown/degraded
    source state. They must not manufacture reassuring physiology or fire the clinical alarm merely
    because infrastructure failed.

## 10. Unresolved physical and runtime questions

These are deliberately questions, not inferred answers:

1. Which exact artifact is deployed, and is the active controller the onboard radio, Sena USB
   adapter, or another device? What are its stable USB identity, hci name, firmware, kernel, BlueZ,
   and D-Bus broker versions?
2. Does the installed controller support the required number of simultaneous LE links with the
   H10, SleepO2, BP's intermittent kernel connection, and any AirSense link without packet loss or
   missed keepalives? What is the measured RF/Wi-Fi environment and USB power/placement?
3. How does this kernel/BlueZ combination behave when kernel BP auto-connect, BlueZ discovery, and an
   existing GATT stream overlap? The incident shows correlation, not the exact internal cause;
   btmon/kernel/BlueZ evidence is needed.
4. Is the accept list populated after cold boot, BlueZ restart, USB replug, and adapter hotplug? Does
   the address type/IRK path remain correct? Does the udev rule target the actual controller?
5. What happens to every lease and in-flight GATT operation when `bluetoothd`, the D-Bus broker, the
   USB device, or the controller resets? Can the broker identify a new adapter generation without
   treating old `Connected` properties as current?
6. What is the precise BlueZ discovery ownership/refcount/filter behavior when independent clients
   call `StartDiscovery`/`StopDiscovery`? The rewrite should not rely on undocumented refcounting;
   it must eliminate independent callers.
7. Can the broker receive BP indications and persist the raw frame fast enough under the ATT ACK
   boundary, including journal-full and worker-crash cases? What queue size and fsync latency preserve
   the required loss visibility?
8. Does SleepO2 require the documented keepalive timing on the actual device/firmware, and what is the
   maximum scheduler/write latency under concurrent streams?
9. What are the AirSense device-specific facts? The Air11 implementation tree models an authenticated
   stream, while the primary integration research says BLE is provisioning-only and therapy data uses
   SD/EzShare. The reported `streaming`/`complete` mismatch must be resolved independently.
10. What live GATT identity/protocol and retransmission behavior does Wellue actually provide? Until
    answered, the collector must stay disabled and no broker test may be called Wellue acceptance.
11. What external phone/official-app connections compete for H10, SleepO2, Wellue, or AirSense
    ownership, and can each peripheral hold more than one central?
12. What finite physical test is sufficient to establish a safe first cutover, and who authorizes it?
    No Pi operation, pairing, re-enable, adapter change, or overlay change is part of this report.

## 11. Exact files, APIs, and tests to inspect or change

The following is the implementation impact list for a future approved change. It is not a request to
edit these files now.

### New proposed files/APIs

- `crates/vitals-ble-broker/Cargo.toml` and `src/main.rs`: one broker process per adapter; Linux-only
  BlueZ ownership, bounded D-Bus operations, broker generation, lease expiry, and status.
- `crates/vitals-ble-broker/src/lease.rs`: pure adapter/device lease state machine and priority/fairness
  policy; tests for discovery exclusivity, preemption boundaries, cooldown, and retry classification.
- `crates/vitals-ble-broker/src/bluez.rs`: the only `bluer` access for the broker; typed wrappers for
  adapter discovery/filter, `Device1` connection state, GATT resolution, subscription, writes, and
  generation changes; no generic D-Bus escape hatch.
- `crates/vitals-ble-broker/src/protocol.rs`: versioned Unix-socket `Hello`, `Acquire`, operation,
  `Frame`, `Release`, `Cancel`, and `Health` messages with bounded payloads and redacted errors.
- `crates/vitals-ble-broker/tests/`: fake BlueZ event/operation traces, filter restoration, one
  discovery owner, per-device serialization, broker restart, worker death, backpressure, and adapter
  generation tests. Linux integration tests must run against real BlueZ; macOS tests cover only pure
  scheduling.
- `crates/vitals-ble-kernel-policy/` (or a narrowly scoped replacement for the current oneshot):
  privileged `btmgmt`/management boundary with tests for controller selection, BP address type,
  idempotent registration, hotplug, and explicit blocker reporting. It must not expose generic source
  or D-Bus control.

### Existing collector files to migrate

- `AnxietyWatch2/crates/vitals-collectors/src/bin/bp_collector.rs`: replace direct `bluer::Session`,
  adapter/device polling, and notification acquisition with the BP broker capability; retain
  `SessionMinter`, decode, raw journal/durability semantics, `sync:true`, and the ten-second
  `ServicesResolved` degraded state. Add a test that the BP client cannot encode/send `Connect`.
- `AnxietyWatch2/crates/vitals-collectors/src/bin/h10_collector.rs` and
  `AnxietyWatch2/crates/vitals-collectors/src/h10_reconnect.rs`: move connect/disconnect/service/
  notify operations to broker leases; retain pure backoff/idle policy. Add tests for a discovery owner
  blocking H10 without a retry storm and for cancellation of a pending connect.
- `AnxietyWatch2/crates/emay-collector/src/main.rs` and
  `AnxietyWatch2/crates/emay-spike/src/main.rs`: move discovery/connect/GATT writes/notifications to
  the broker; retain the keepalive state machine and unknown/no-finger semantics. Test bounded write
  latency and keepalive behavior when another source is discovering.
- `aw2-wellue-fix/crates/wellue-collector/src/main.rs`: migrate direct discovery/connect/reset to the
  broker; remove any source-level adapter reset from the eventual implementation; preserve quarantine,
  raw journal, and status. Add a disabled-policy test and do not use it to re-enable Wellue.
- `aw2-air11/crates/vitals-collectors/src/as11_bluez.rs`: replace direct BlueZ transport with a
  broker transport adapter while retaining exact FD56/TX/RX resolution, bounded notifications, MTU,
  and read-only RPC boundaries. Add tests for discovery lease denial and filter restore.
- `aw2-air11/crates/vitals-collectors/src/bin/resmed_air11_collector.rs`, `as11_status.rs`, and
  `as11_supervision.rs`: map broker failures/generation changes into explicit link/therapy/measurement
  blockers; do not merge this with the separate AirSense lifecycle investigation.
- `crates/vitals-collectors/Cargo.toml`: remove direct `bluer` from worker targets once all Linux
  access has moved; keep it only in the broker crate. Verify no worker retains `AF_BLUETOOTH` solely
  for an unremoved direct path.

### Deployment and policy files to review in the future

- `AnxietyWatch2/deploy/vitals-bp-autoconnect.service` and
  `AnxietyWatch2/deploy/99-vitals-bp-autoconnect.rules`: make controller selection, registration
  result, and hotplug behavior explicit; preserve the privileged/non-privileged split.
- `AnxietyWatch2/deploy/vitals-bp.service`, `deploy/vitals-h10.service`, the EMAY unit, and
  `aw2-wellue-fix/deploy/wellue-collector.service`: replace direct BlueZ permissions/ownership with
  broker socket access and explicit broker ordering; keep separate source failure domains.
- `aw2-air11/deploy/vitals-air11.service`: replace direct BlueZ access with broker socket access and
  preserve credential/state/status restrictions.
- `AnxietyWatch2/deploy/stopgap/vitals-bp-connect` and its service: remove only after the broker BP
  path and kernel policy have passed the BP gate; it must not be a hidden fallback or run concurrently.
- `AnxietyWatch2/deploy/stopgap/emay-live.service`: remain historical/disabled; it must not coexist
  with a broker-owned SleepO2 path.
- `deploy/vitalsd.service` and `crates/vitalsd/src/{main.rs,state.rs}`: no BlueZ ownership should be
  added. Review only the broker-client health projection and source/adapter blocker propagation over
  the existing Unix protocol.

### Integration and acceptance tests

- Static call-site test: only broker/kernel-policy crates contain `bluer`, `Device1.Connect`,
  `Adapter1.StartDiscovery`, `SetPowered`, adapter reset, or raw `btmgmt`; exceptions are named and
  tested.
- Lease model tests: concurrent discover requests, filter restoration failure, deadline expiry,
  priority fairness, worker disconnect, broker restart, adapter generation, and no retry while holding
  a lease.
- Source tests: BP `Connect` capability rejection plus raw-journal/sync-append ordering; H10
  reconnect/notification watchdog; SleepO2 keepalive; AirSense authenticated stream re-subscription;
  Wellue disabled policy and bounded future path.
- Linux BlueZ tests: real `ObjectManager` events, `Connected`, `ServicesResolved`, GATT
  notify/indicate, one active discovery owner, adapter hotplug, kernel accept-list evidence, and
  controller reset classification. No macOS BlueZ mock may be reported as hardware success.
- Coexistence test: controlled finite-window and overnight evidence for BP/H10/SleepO2/AirSense,
  measuring link uptime, packet/notification gaps, retry counts, discovery hold time, append latency,
  and per-source unknown/stale transitions. Wellue is excluded until its own authorization.
- Existing repository contracts: run the exact task `Verify:` commands for every changed task,
  `cargo clippy --workspace --all-targets -- -D warnings`, decoder fuzz smoke tests, service static
  checks, and `git diff --check`. Acceptance is the command exit code, not a report narrative.

## 12. Final recommendation

Adopt **Architecture B**, implemented as one broker per verified physical adapter, with separate
source workers and a separate privileged kernel-policy boundary. Do not solve the incident by adding
another collector-side discovery timeout, another `Connect` workaround, or an automatic adapter
reset. Those are local reactions to a global ownership problem.

The first non-operational deliverable should be the complete librarian inventory and pure lease
state-machine tests. The first physical gate should establish controller identity and observe BP
kernel-autoconnect without changing the adapter. Only then should one source be cut over. Wellue
stays disabled, H10 stays unaccepted, AirSense lifecycle remains a separate question, and no claim of
coexistence is made until measured Linux evidence exists.
