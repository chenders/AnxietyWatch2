# Bluetooth Implementation Librarian

**Scope:** Linux BlueZ implementation inventory for the rewrite investigation.  This is a
static repository/document review only. No Pi connection, physical action, service change,
Wellue re-enable, or runtime/config/code change was performed.

**Repositories inspected:**

- Primary: `$HOME/Source/AnxietyWatch2` (`AnxietyWatch2` below).
- Related: `$HOME/Source/aw2-air11` (`aw2-air11`) and
  `$HOME/Source/aw2-wellue-fix` (`aw2-wellue-fix`).

The primary checkout currently does **not** contain `crates/vitals-collectors/`,
`bp_collector.rs`, `h10_collector.rs`, AirSense sources, or the production Wellue crate.
Those paths are present in the related repositories and are inventoried below. Absence from
the primary checkout is an observed repository fact, not evidence that the deployed Pi lacks
the corresponding binaries.

## 1. Evidence rules and status

The controlling brief requires evidence-backed findings and prohibits hardware/runtime changes
(`[BRIEF.md:3-7]`). It also records the recent Wellue/BP discovery contention incident and
explicitly says that Wellue remains disabled `[BRIEF.md:14-24]`. The amendment index makes Rust,
`bluer`, the append-only spool, and the BP `sync: true` durability exception authoritative
`[docs/specs/AMENDMENTS.md:51-52,97-102,173-174]`; amendment 06 is the specific BP correction.

Labels used here:

- **Observed in code/config** — directly present in a checked-in file.
- **Documented evidence** — a checked-in report records a live/runtime observation; it is not
  a fresh observation made during this review.
- **Hypothesis/design implication** — plausible consequence or proposed boundary, not proof.
- **Unknown/gate** — the repository does not establish the fact; physical/runtime validation is
  required.

No full Bluetooth address, credential, key, or target secret is reproduced here.

## 2. Executive findings

1. The current design has **multiple independent BlueZ clients**. They each open a `bluer::Session`,
   select an adapter, and can call discovery, connect/disconnect, GATT discovery, and notification
   subscription. There is no checked-in global adapter broker.
2. Discovery is the clearest shared resource. Production AirSense and onboarding explicitly reject
   an already-active discovery; Wellue and EMAY spike/legacy paths start discovery without a
   repository-wide lock. The brief's incident records `Discovering: yes` while Wellue retried and
   BP waited `[BRIEF.md:14-20]`.
3. BP has a deliberate exception: the production collector must not call `Device1.Connect`; it
   waits for kernel/BlueZ autoconnect and consumes an indication. However, the primary stopgap
   `vitals-bp-connect` is an independent D-Bus helper that does call `Device1.Connect`
   `[AnxietyWatch2/deploy/stopgap/vitals-bp-connect:136-195]`. This is an explicit competing
   ownership model, not a harmless implementation detail.
4. Adapter naming is not stable. Historical task evidence says the Sena was `hci1` with the
   onboard UART controller present, then renumbered to `hci0` after `dtoverlay=disable-bt`
   `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:1267-1324]`. Production AirSense
   defaults to/ships `--adapter hci0` `[aw2-air11/crates/vitals-collectors/src/bin/resmed_air11_collector.rs:709-766; aw2-air11/deploy/vitals-air11.service:17]`,
   while BP/H10/Wellue/EMAY use `default_adapter()`.
5. Current controller identity and topology are **not settled**. Historical documentation records
   both a Sena CSR adapter and an onboard controller, followed by a later runtime acceptance record
   that saw only `hci0` and says the required Sena `hci1` was absent
   `[AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md:76-82]`.
   The rewrite must resolve controllers by stable identity and require a physical/runtime gate;
   it must not infer identity from `hciN`.

## 3. Authoritative entry-point inventory

### 3.1 Primary checkout: durable EMAY collector

**File:** `AnxietyWatch2/crates/emay-collector/src/main.rs`

- Linux-only BlueZ dependency boundary: `bluer::{Adapter, Address, Device}`
  `[38-39]`; the crate manifest uses `bluer` with `bluetoothd`, `futures`, and Tokio
  `[AnxietyWatch2/crates/emay-collector/Cargo.toml:16-19]`.
- Objects/API: `bluer::Session::new()`, `default_adapter()`, `Adapter::set_powered(true)`,
  `Adapter::device(target)`, `Device::is_connected()`, `Device::connect()`,
  `Device::services()`, `Service::characteristics()`, `Characteristic::write()`, and
  `Characteristic::notify()` `[main.rs:255-297,1367-1385]`.
- Fixed GATT objects: service `FF12`, write characteristic `FF01`, notify characteristic
  `FF02` `[main.rs:42-45]`. The code rejects a known device that lacks both exact
  characteristics `[main.rs:272-300]`.
- Lifecycle: `known_device()` obtains the target object and calls `connect()` if it is not
  connected `[main.rs:255-269]`; `CollectorBackend::connect()` uses the default adapter and
  subscribes to `FF02` `[main.rs:1363-1385]`. There is no active discovery in this production
  collector. If the BlueZ device object is absent, this path reports an error rather than
  materializing it by scan.
- Connection ownership: the process owns one EMAY GATT link, one write characteristic, and one
  notification stream. During the link it sends setup/stop/start/keepalive commands and periodically
  re-enters device storage `[main.rs:337-366,1212-1343]`. Storage transfer and live streaming share
  the same notification stream; `discard_live_frame()` deliberately filters live bytes while
  storage responses are assembled `[main.rs:391-479,492-581]`.
- Data boundary: durable stored rows go through the `vitalsd` Unix socket with `sync: false`
  `[main.rs:869-919]`; anonymous live `EB` frames are status-only and explicitly not durable
  `[main.rs:1094-1131]`.
- Adapter assumptions: default adapter, adapter powered by the collector, configured `EMAY_MAC`;
  no stable adapter address or controller bus identity is used `[main.rs:1367-1378,2584-2626]`.

**Unit:** `AnxietyWatch2/deploy/vitals-emay.service`

- Starts after/requires `bluetooth.target` and `vitalsd.service`; runs as `chris` with supplementary
  group `vitals`; loads `/etc/default/vitals-emay`; passes `VITALS_SOCKET` and status path;
  restarts forever `[3-24]`.
- Restricts address families to `AF_UNIX` `[39]` even though the binary reaches BlueZ through the
  D-Bus-backed `bluer` API. Whether this is sufficient for the deployed D-Bus transport is an
  unverified runtime question.
- This unit is distinct from the related repository's legacy `emay-live.service`; the primary
  unit runs `emay-collector`, while the stopgap unit runs `emay-spike` (see below).

### 3.2 Primary checkout: EMAY spike and other adjacent tools

**File:** `AnxietyWatch2/crates/emay-spike/src/main.rs`

- Linux BlueZ boundary uses `Adapter`, `AdapterEvent`, `Address`, `Device`; fixed EMAY `FF12`,
  `FF01`, `FF02` UUIDs `[AnxietyWatch2/crates/emay-spike/src/main.rs:238-249]`; its discovery and GATT path is `[AnxietyWatch2/crates/emay-spike/src/main.rs:329-361,567-612]`.
- It calls `Session::new()`, `default_adapter()`, `set_powered(true)`, `Adapter::device()`,
  `Adapter::discover_devices()`, `Device::is_connected()`, `Device::connect()`, service and
  characteristic enumeration, `notify()`, and writes setup/keepalive/stop commands. The `find()`
  path starts a one-minute adapter discovery window `[AnxietyWatch2/crates/emay-spike/src/main.rs:329-343]` and connects/resolves/subscribes in `[AnxietyWatch2/crates/emay-spike/src/main.rs:567-612]`.
- The spike is an independent discovery owner and is not a safe shared-resource abstraction.
  Its historical service definition is `AnxietyWatch2/deploy/stopgap/emay-live.service`, which
  requires `bluetooth.target`, runs `emay-spike`, and restarts `[aw2-air11/deploy/stopgap/emay-live.service:5-32]`.

**File:** `AnxietyWatch2/crates/hr-spike/src/main.rs`

- Separate H10 acceptance entry point. It uses BlueZ `Adapter`, `AdapterEvent`, `Address`, and
  `Device`, scans for the target, checks pairing, connects, waits for services, resolves standard
  Heart Rate service `0x180D` / measurement `0x2A37`, and subscribes with `notify()`
  `[AnxietyWatch2/crates/hr-spike/src/main.rs:102-121,207-220,365-411]`.
- It is an investigative capture tool, not a systemd collector. It still competes for discovery
  and adapter connection slots when run on the appliance.

**Other primary adjacent entry points:** `AnxietyWatch2/tools/emay-investigation/inspect_emay_gatt.py`
(and the referenced old Swift protocol material) are investigation tooling, not production units.
`AnxietyWatch2/scripts/sync-to-pi.sh` is deployment-adjacent, not a BlueZ client: it explicitly
says BlueZ work/debugging is Pi-only and uses `rsync`/SSH `[scripts/sync-to-pi.sh:1-28]`.

### 3.3 BP production collector and kernel autoconnect path (aw2-air11)

**File:** `aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs`

- Code comment explicitly says the collector never calls `Device1.Connect`; it waits for kernel
  autoconnect, then reacts to `Connected` and GATT resolution `[1-14]`.
- BlueZ objects/API: `Session::new()`, `default_adapter()`, `set_powered(true)`, `adapter.device()`;
  `Device::is_paired()`, `is_trusted()`, `set_trusted(true)`, `is_connected()`,
  `is_services_resolved()`, `services()`, `characteristics()`, and `Characteristic::notify()`
  `[253-270,184-221,280-304]`.
- Fixed GATT objects are Blood Pressure service `0x1810` and measurement `0x2A35`
  `[43-44,184-199]`. BlueZ selects indication CCCD from the characteristic flags when
  `notify()` is called `[211-221]`.
- Ownership/lifecycle: the process opens a restart-safe session minter and raw journal, waits for
  a kernel-established link, waits up to 10 seconds for `ServicesResolved`, then drains one
  connection until it ends `[87-104,253-304]`. It mints one session per connection and journals
  raw bytes before decode `[211-236]`.
- Durability: the collector sends `Request::Append { sync: true }` and requires a durable reply
  `[109-163]`; this implements amendment 06's ACK-boundary rule
  `[aw2-air11/docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md:134-149]`.
- This is a **device link consumer**, not a discovery owner. It depends on external kernel/BlueZ
  autoconnect state.

**Files:** `aw2-air11/deploy/99-vitals-bp-autoconnect.rules` and
`aw2-air11/deploy/vitals-bp-autoconnect.service`

- udev starts the oneshot when `SUBSYSTEM=="bluetooth"` and `KERNEL=="hci0"` are added
  `[99-vitals-bp-autoconnect.rules:1-4]`.
- The root-owned oneshot runs `btmgmt add-device -a 2 -t 1 "$BP_CUFF_MAC"`
  `[vitals-bp-autoconnect.service:3-22]`. This is the only checked-in privileged kernel
  accept-list registration found.
- The `hci0` match is an adapter-number assumption. It is incompatible with a rewrite that treats
  controller indices as stable.

### 3.4 BP stopgap D-Bus connector (primary checkout)

**Files:** `AnxietyWatch2/deploy/stopgap/vitals-bp-connect` and
`AnxietyWatch2/deploy/stopgap/vitals-bp-connect.service`

- Python imports `dbus`, obtains `org.freedesktop.DBus.ObjectManager`,
  `org.bluez.Device1`, `org.freedesktop.DBus.Properties`, and reads `Connected`/`RSSI`;
  it constructs `/org/bluez/{adapter}/dev_<MAC>` `[vitals-bp-connect:116-148]`.
- It calls `Device1.Connect()` when RSSI is present `[183-195]`, with polling/backoff and an
  optional `0x2A08` Date Time write `[50-65,105-114]`.
- The service runs it as unprivileged `vitals:bluetooth`, after/requiring `bluetooth.service`,
  and before `vitals-bp.service` `[vitals-bp-connect.service:7-26]`.
- This helper's own header says it intentionally relies on EMAY's continuous discovery
  `[vitals-bp-connect:8-33]`. That is direct evidence of cross-collector coupling. It is not
  consistent with the production BP rule or with a single-owner rewrite; classify it as a
  stopgap/legacy competing path until deployment status is independently established.

### 3.5 Polar H10 collector (aw2-air11)

**File:** `aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs`

- Config: `H10_MAC`, `VITALSD_SOCKET`, `H10_STATE_DIR`, optional status/history paths
  `[55-76]`.
- BlueZ objects/API: per reconnect cycle `Session::new()`, `default_adapter()`,
  `set_powered(true)`, `adapter.device()`, `Device::is_connected()`, `connect()`,
  `is_services_resolved()`, `services()`, `characteristics()`, `notify()`, and
  `disconnect()` `[241-292,309-430]`.
- Fixed GATT objects: Heart Rate service `0x180D`, measurement `0x2A37` `[31-33]`.
- It does not start discovery in the production collector; it attempts a configured device
  object directly. It does own reconnect, timeout, notification-idle watchdog, and disconnect
  cleanup `[35-51,418-468]`. A failed connect is actively cancelled with `Device1.Disconnect`
  to avoid BlueZ's `Operation already in progress` loop `[241-264]`.
- Service lifecycle: `aw2-air11/deploy/vitals-h10.service` requires Bluetooth and vitalsd, runs as
  `vitals`, restarts, and permits `AF_UNIX AF_NETLINK AF_BLUETOOTH`
  `[3-37]`.

The older `AnxietyWatch2/crates/hr-spike` does scan, so H10 has two distinct operational paths:
production direct-connect and investigative discovery.

### 3.6 AirSense 11 production collector and onboarding (aw2-air11)

**Files:** `aw2-air11/crates/vitals-collectors/src/as11_bluez.rs`,
`src/as11_onboarding.rs`, `src/bin/resmed_air11_collector.rs`,
`src/bin/resmed_air11_onboard.rs`

- Production collector takes an explicit adapter name, default `hci0`
  `[resmed_air11_collector.rs:695-766]`; systemd passes `--adapter hci0`
  `[deploy/vitals-air11.service:17]`.
- `Air11BleTransport::connect()` opens a D-Bus/BlueZ session, resolves the named adapter,
  obtains the configured device, checks `Connected`, attempts direct `connect()`, and only then
  invokes bounded discovery fallback `[as11_bluez.rs:374-419]`.
- Discovery arbitration is explicit: it checks `Adapter::is_discovering()` and returns
  `DiscoveryAlreadyActive`; it saves/restores the discovery filter; the filter is LE and restricted
  to FD56; the scan has a five-second target window `[as11_bluez.rs:688-750]`. This is the only
  production path currently found that refuses to take over an already-active scan.
- GATT objects: service `FD56`, TX `a6220002-...`, RX `a6220003-...`
  `[as11_bluez.rs:46-48,637-681]`. It resolves exact service/characteristics, checks TX write
  capability, RX notify/indicate capability, MTU, subscribes to RX notifications, and writes
  bounded FIG packets `[as11_bluez.rs:419,637-681,793-833]`.
- Lifecycle ownership: the transport owns one device, RX stream, bounded writer queue, and
  disconnect; the collector owns authenticated read-only application RPCs, raw journal, status,
  and reconnect supervision `[as11_bluez.rs:374-530; resmed_air11_collector.rs:330-429]`.
- Onboarding is a separate operator entry point. It performs a bounded FD56-filtered discovery,
  rejects active discovery, selects exactly one candidate or an explicit address, then connects
  and subscribes for the two fixed pairing methods `[as11_onboarding.rs:303-394,454-561]`.
  `resmed_air11_onboard` requires interactive authorization and never starts the collector
  `[resmed_air11_onboard.rs:1-7,151-190]`.
- This source therefore has the most explicit local arbitration, but it still competes globally
  with Wellue/EMAY/H10 spike paths because its “already discovering” check is not a shared lock.

### 3.7 Wellue/Viatom KS-60FWB collector and probe (aw2-wellue-fix)

**Files:** `aw2-wellue-fix/crates/wellue-collector/src/main.rs`,
`src/wellue-probe` (actual path `crates/wellue-probe/src/main.rs`)

- Production Wellue imports `Adapter`, `AdapterEvent`, `Address`, `Device` and uses the default
  adapter, powers it on, then loops forever `[wellue-collector/src/main.rs:15,613-629]`.
- `find()` first checks a configured target object; if not already connected it starts
  `Adapter::discover_devices()` and waits up to 10 seconds for `DeviceAdded` matching the exact
  address `[211-238]`. It does not check `Adapter::is_discovering()` or restore a prior discovery
  filter. This is a direct global discovery contention point.
- It connects, waits up to 10 seconds for `is_services_resolved()`, resolves a provisional NUS
  service/characteristic, subscribes with `notify()`, journals every notification, and forwards
  decoded samples through `vitalsd` `[244-264,517-591]`. The UUID constants come from
  `vitals_decode::wellue` (`NUS_SERVICE_UUID`, `NUS_TX_UUID`).
- Error lifecycle: after a failed connected run it calls `Device::disconnect()`; for selected stale
  link errors it calls `Adapter::set_powered(false)`, waits one second, then `set_powered(true)`
  `[438-501]`. This is **controller-wide power ownership** by one collector and is unsafe as a
  global policy while H10/BP/AirSense/EMAY may be connected. The code tests the reset predicate,
  not the physical safety or multi-client consequence `[698-711]`.
- Its bounded local mitigation is a 10-second scan and one-minute retry delay
  `[25-39]`, but the brief's incident shows that local bounds do not establish global arbitration.
- Unit `aw2-wellue-fix/deploy/wellue-collector.service` requires `bluetooth.target` and `vitalsd`,
  runs as `vitals` with supplementary `bluetooth`, restarts every 10 seconds, and is configured
  by `WELLUE_MAC` `[7-35]`. The brief says it is intentionally disabled; this report does not
  re-enable it.
- `wellue-probe` is a separate read-only/first-contact tool. It uses the same default adapter,
  scans for a target, powers the adapter, inventories all GATT services/characteristics, and can
  subscribe to a caller-selected UUID for a bounded raw capture
  `[wellue-probe/src/main.rs:15-83,104-178]`. It is another discovery and connection owner when
  run, not a production broker.

## 4. Adapter/controller and hardware history

### Checked-in claims and contradictions

- The hardware plan recommends a Raspberry Pi 5 with an external CSR8510A10/Sena UD100-G03
  dongle `[aw2-wellue-fix/docs/architecture/raspberry_pi_migration.md:7,16,39-40]`.
- Historical task evidence records a CSR USB device `0a12:0001`, Sena OUI `00:01:95`, initially
  as USB `hci1`, while the onboard UART was `hci0`; it also records an initial RF-kill delay and
  that the USB adapter was not automatically raised at boot
  `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:1267-1299]`.
- The same evidence records that after disabling onboard Bluetooth, the Sena renumbered to
  `hci0`, and says all remaining `hci1` references were then wrong
  `[.../tasks.md:1307-1335]`. This is strong repository evidence that `hciN` is not an identity.
- A later acceptance record reports exactly one powered controller at `hci0`, says the required
  Sena `hci1` was not present, leaves G1 not green, and says H10/EMAY continuity is unknown or
  failed `[AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md:76-94]`.

**Current conclusion:** the exact controller connected to the current appliance is **unknown**.
The checked-in history is not a substitute for `lsusb`, stable controller address, bus/path, driver,
BlueZ version, and post-boot state collected in one controlled runtime gate. Do not apply or infer
`dtoverlay=disable-bt`; the G1 warning is explicit `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:190-220]`.

### Device/protocol evidence relevant to ownership

- A&D capture: Sena CSR8510, Linux kernel `6.12.96`, BlueZ `5.66`; the device delivered four
  buffered BP measurements `[aw2-air11/docs/research/2026-08-03-ua651ble-ble-capture.md:3-9]`.
- A&D `0x2A35` is an indication; the checked-in evidence says a kernel ATT confirmation can occur
  before application durability, and `bluetoothctl connect` issued no LE Create Connection for
  the cuff `[.../ua651ble-ble-capture.md:73-112]`. The rewrite must preserve the production BP
  no-`Device1.Connect` rule and `sync: true` boundary, not regress to a polling connector.
- The same capture documents bonded-but-unresolved encryption (`unexpected SMP command 0x0b`) and
  requires a named `Connected`-but-no-`ServicesResolved` state `[.../ua651ble-ble-capture.md:114-135]`.
- The historical design document says the external adapter and onboard radio have different
  coexistence/slot characteristics, but its slot counts and adapter behavior are design evidence,
  not a current Pi measurement `[aw2-air11/docs/design/HOMEASSISTANT-ARCHITECTURE.md:204-227]`.

## 5. Cross-collector contention matrix

| Actor | Discovery | Direct connect | Kernel/autoconnect | Controller power | Link/GATT ownership |
|---|---:|---:|---:|---:|---|
| EMAY collector | No in production; spike/legacy yes | Yes | No | `set_powered(true)` | FF12/FF01/FF02; storage and live share link |
| BP production | No | **No** | **Yes**, external `btmgmt` | `set_powered(true)` in process | 1810/2A35 indication |
| BP stopgap helper | No scan itself; depends on another scan's RSSI | **Yes, Device1.Connect** | No | No | D-Bus Device1/Properties; clock write |
| H10 production | No | Yes | No | `set_powered(true)` | 180D/2A37 notify; reconnect/disconnect |
| H10 spike | Yes | Yes | No | Yes | 180D/2A37 notify |
| AirSense production | Bounded fallback only | Yes | No | Adapter selected by name | FD56/TX/RX; bounded filtered scan |
| AirSense onboarding | Yes, bounded filtered | Yes for pairing | No | Requires adapter powered | FD56 pairing link |
| Wellue collector | **Yes, no global arbitration** | Yes | No | **Yes; may power-cycle adapter** | provisional NUS notify |
| Wellue probe | Yes | Yes | No | Yes | arbitrary selected notify characteristic |

The strongest observed contention is Wellue discovery versus BP autoconnect, as recorded in the
brief `[BRIEF.md:14-20]`; the code inventory identifies additional possible contention from H10
spike, EMAY legacy, AirSense onboarding, and Wellue probe. This matrix is a code ownership map,
not proof that every actor was concurrently enabled on the Pi.

## 6. Proposed rewrite architecture and boundaries

This is a proposal for specialist coordination, not an implementation decision.

### 6.1 One adapter owner

Create one Linux `bluetooth-supervisor`/broker per physical controller. It alone owns:

- one BlueZ D-Bus session and adapter object;
- controller power/rfkill/recovery policy;
- the singleton discovery operation and discovery filter restoration;
- stable controller identification (BD address plus sysfs/USB identity), never `hciN` alone;
- device-object materialization and connection slot accounting;
- serialized connect/disconnect/GATT discovery/subscription operations; and
- per-device lifecycle/events delivered to collectors over a narrow IPC protocol.

Collectors must not call `discover_devices`, `set_powered`, adapter power-cycle, or direct
`Device1.Connect` themselves. The BP policy is a broker operation backed by kernel autoconnect/
management state, with the broker reporting `Connected`, `ServicesResolved`, and indication
subscription readiness. The broker must expose ownership, deadline, cancellation, and blocker
reason—not merely a boolean.

This follows the accepted design observation that BLE scanning is a singleton and should have one
owned scanner task with broadcast advertisement delivery `[docs/specs/AMENDMENTS.md:222-227]`, while
fixing the current problem that each collector currently has its own local guard.

### 6.2 Keep protocol/physiology below the broker

Retain device-specific protocol state machines and pure decoders outside the broker:

- BP: 1810/2A35 indication, raw journal before decode, `sync: true` append and explicit
  bonded-but-unresolved state.
- H10: 180D/2A37 notification, RR/HR sequence and reconnect semantics.
- EMAY: FF12/FF01/FF02 command/notification multiplexing, stored-session identity, and live
  status-only boundary.
- AirSense: FD56 FIG/RPC transport and application authentication/read-only allowlist.
- Wellue: provisional NUS path remains disabled until physical protocol evidence and global policy
  acceptance.

The broker should transport bounded raw GATT notifications and typed link events; it should not
know physiological units, registry ranges, SQL, spool format, or application credentials.
`vitalsd` remains the sole durable append boundary as required by the amendments.

### 6.3 Ownership contract

Every lease should name `controller_id`, `device_id`, collector, operation, deadline, and recovery
owner. A collector crash releases its lease; a broker crash does not leave an unbounded scan or
power-cycle loop. Discovery requests must be queued/rejected with the current owner named. A device
must have one connection owner at a time unless the protocol explicitly supports shared
subscriptions. No collector can reset an adapter that has other leases.

## 7. Staged migration and rollback plan

No stage below was executed.

1. **Inventory/read-only baseline:** establish the controller identity, BlueZ version, active units,
   D-Bus objects, discovery state, and per-device ownership without changing state. Gate: reconcile
   runtime output with this document; Wellue remains disabled.
2. **Broker shadow mode:** add the broker as a read-only observer or test-harness process with no
   production lease and no adapter mutation. Test discovery serialization, device-object loss,
   controller renumbering, `ServicesResolved` timeout, and cancellation using emulators/fakes and
   captured sanitized traces.
3. **Migrate bounded discovery clients:** move AirSense onboarding and production fallback, then
   H10/EMAY/Wellue probe paths, to broker leases. Keep the old units disabled or isolated. Rollback:
   stop the broker client path and restore the previously installed collector binary/unit only if
   the owner explicitly authorizes runtime action.
4. **Migrate streaming collectors:** H10 and EMAY first, preserving their separate journals and
   `vitalsd` contracts. Acceptance is source-specific measured evidence plus no unowned discovery.
5. **Migrate BP last:** replace the split stopgap/kernel ownership with one broker-managed
   autoconnect policy. Verify the `Connected`/`ServicesResolved` degraded state and the durable
   append reply before considering a frame handled. Do not replace it with direct `Device1.Connect`.
6. **Wellue decision gate:** only after a global arbitration test, exact hardware/protocol evidence,
   and owner approval. The current brief's disabled state is the rollback/default state.
7. **Retire old paths:** only after a bounded live window and restart/rollback test. Retain old
   binaries, unit definitions, and journals until the new path has an operator-verified rollback.

## 8. Known facts, hypotheses, and unknowns

### Known/documented facts

- Multiple independent BlueZ clients and several independent discovery entry points exist (Sections
  3 and 5).
- BP production intentionally waits for kernel autoconnect; the stopgap helper calls `Connect()`.
- Wellue discovery does not check for an existing scan and can power-cycle the shared controller.
- AirSense has a local active-discovery guard/filter restore, not a global lock.
- Historical controller indices changed from Sena `hci1` to Sena `hci0` after onboard removal.
- The brief records a discovery/BP recovery correlation, not merely a Wellue device failure.

### Hypotheses requiring specialist/runtime testing

- A single broker will eliminate the observed BP starvation without reducing source continuity.
- The current incident was caused primarily by discovery ownership rather than controller slot
  exhaustion, BlueZ/kernel behavior, or a separate BP autoconnect failure.
- Direct-connect retries from H10/EMAY/AirSense can consume controller slots or leave stale BlueZ
  operations that amplify other collectors' failures.
- Wellue's adapter power-cycle path can disrupt otherwise healthy links; code makes this plausible,
  but no concurrent physical trace was collected here.
- Controller-stable identity can be reliably derived from BD address plus USB/sysfs identity across
  reboot/hotplug on the actual Pi.

### Unknowns and physical/runtime gates

1. What controller(s) are physically present now; BD address, USB VID/PID, bus path, driver, and
   stable identity? **Gate before any adapter/config change.**
2. Is the Sena UD100-G03 actually the active adapter, and is it CSR8510 hardware rather than a
   counterfeit or another CSR device?
3. Does the current BlueZ/kernel version support the required kernel autoconnect and indication
   behavior after cold boot and USB hotplug?
4. What are the actual controller connection slots and failure behavior under H10 + EMAY + AirSense
   concurrency? Do not treat documented CSR/BCM slot counts as current evidence.
5. Does BlueZ permit the desired broker discovery filter/monitor behavior without starving BP?
6. Does a Wellue scan request coexist with kernel BP autoconnect when mediated by one owner?
7. Does an A&D Date Time write make timestamp bit 1 appear? The repository marks this untested
   `[aw2-air11/docs/research/2026-08-03-ua651ble-ble-capture.md:68-71]`.
8. Can bonded BP reconnect reach `ServicesResolved` reliably, or must fresh pairing remain an
   operational gate?
9. Which units/binaries are actually installed/enabled on the current appliance? Repository unit
   files alone do not answer this.
10. Can AirSense's `streaming`/therapy `complete` symptom be reproduced independently of discovery
    contention? The brief says not to conflate them `[BRIEF.md:21]`.
11. What is the exact current Wellue enablement state? The brief controls this investigation:
    do not re-enable it `[BRIEF.md:23]`.
12. What broker restart semantics preserve BP durability and avoid duplicate/replayed sessions?

## 9. Exact questions specialists should ask

Specialists may ask these verbatim in request files:

1. “List every checked-in caller of `Adapter::discover_devices()` and whether it checks
   `is_discovering()` or restores the prior filter.”
2. “Which checked-in path calls `Device1.Connect`, and which rule forbids it for production BP?”
3. “What exact BlueZ object/API sequence does production BP use from process start to first
   durable row?”
4. “Which code can call `Adapter::set_powered(false/true)`, and what other collectors could it
   disrupt?”
5. “Which paths use `default_adapter()` versus an explicit adapter name, and what are the defaults?”
6. “Where is the stable controller identity represented? If nowhere, mark it unknown.”
7. “What are the exact GATT UUIDs, notification/write operations, and lifecycle owner for each
   source?”
8. “Which discovery filters are installed, and are they restored after timeout/error?”
9. “What is the cancellation path after connect timeout or notification failure?”
10. “Which unit starts each entry point, under which user/group, and what Bluetooth dependency does
    it declare?”
11. “Which source has a raw journal before decode, and which append requests use `sync: true`?”
12. “What checked-in evidence supports Sena/CSR, `hci0`/`hci1`, BlueZ version, and G1? Separate
    historical evidence from current runtime evidence.”
13. “Can the AirSense lifecycle symptom be established without starting discovery?”
14. “What exact physical/runtime command and exit code would open the controller identity gate,
    without applying a configuration change?”
15. “What rollback restores the prior collector without re-enabling Wellue or changing the adapter?”

## 10. Request/response protocol

A specialist may create a Markdown request at:

```text
docs/reviews/bluetooth-rewrite/requests/<request-id>.md
```

The request should contain:

```markdown
# Request <request-id>
Requester: <specialist>
Question: <one precise question, or a numbered set>
Scope: <repositories/files allowed>
Needed by: <optional>
```

The librarian answers each existing request at the matching path:

```text
docs/reviews/bluetooth-rewrite/answers/<request-id>.md
```

Each answer must include the request id, answer status (`ANSWERED`, `PARTIAL`, or `UNKNOWN`),
exact repository-relative file and line references, observed facts versus inference, and any
physical/runtime gate. No credentials, private keys, full sensitive addresses, or invented hardware
evidence may appear. An absent/empty `requests/` directory means there are currently no specialist
requests; at this review, no request files existed and therefore no answer files were created.

## 11. Exact files/APIs/tests for the next specialist pass

**Primary/architecture:**

- `AnxietyWatch2/crates/emay-collector/src/main.rs`: `known_device`, `CollectorBackend`,
  `characteristics`, `storage_sync`, `live_with_shutdown`.
- `AnxietyWatch2/deploy/vitals-emay.service` and stopgap BP files.
- `aw2-air11/crates/vitals-collectors/src/as11_bluez.rs`: `Air11BleTransport::connect`,
  `discover_target`, `write_packet`, `resolve_pipe`.
- `aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs`: `discover_target`,
  `Air11PairingLink::connect`.
- `aw2-air11/crates/vitals-collectors/src/bin/{bp_collector,h10_collector,resmed_air11_collector}.rs`.
- `aw2-wellue-fix/crates/wellue-collector/src/main.rs` and `wellue-probe/src/main.rs`.

**Deployment/controller:**

- `aw2-air11/deploy/99-vitals-bp-autoconnect.rules`.
- `aw2-air11/deploy/vitals-bp-autoconnect.service`, `vitals-bp.service`, `vitals-h10.service`,
  `vitals-air11.service`.
- `aw2-wellue-fix/deploy/wellue-collector.service`.
- `AnxietyWatch2/scripts/sync-to-pi.sh` (deployment boundary only; do not run it for this review).

**Evidence/spec/tests:**

- `AnxietyWatch2/docs/reviews/bluetooth-rewrite/BRIEF.md` and `docs/specs/AMENDMENTS.md`.
- `AnxietyWatch2/docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md`.
- `aw2-air11/docs/research/2026-08-03-ua651ble-ble-capture.md`.
- `aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md` (historical hardware/G1 evidence;
  not a current runtime assertion).
- Existing tests cover Wellue discovery/reset predicates `[aw2-wellue-fix/crates/wellue-collector/src/main.rs:681-712]`,
  AirSense candidate selection and discovery guards `[aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs:566-605]`,
  H10 link/capture seams `[aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs:490-539]`,
  EMAY mixed notification framing and live/storage behavior `[AnxietyWatch2/crates/emay-collector/src/main.rs:1647-1836,2251-2565]`,
  and BP sample/durability policy `[aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:305-350; aw2-air11/crates/vitals-collectors/src/bp_build.rs:86-173]`.

**Rewrite tests to require:** one active discovery owner; filter restoration on every error;
controller identity independent of `hciN`; lease cancellation; no adapter power-cycle while another
lease is active; BP no-direct-connect; BP `sync: true` reply handling; named
`Connected`/`ServicesResolved` degradation; and an integration trace proving Wellue absence cannot
hold discovery indefinitely or starve BP.
