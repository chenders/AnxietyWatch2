# Linux Bluetooth / BlueZ collector audit

**Scope:** static audit of `$HOME/Source/AnxietyWatch2`, `$HOME/Source/aw2-air11`, and `$HOME/Source/aw2-wellue-fix`. This document is an investigation report and rewrite plan, not an implementation.

**Safety status:** no Pi connection, physical action, service/config/udev/firmware change, Wellue re-enable, or H10 acceptance claim was made. Full device addresses and credentials are intentionally omitted.

**Evidence labels:**

- **Code/config:** directly present in a checked-in file.
- **Recorded runtime:** a checked-in report records an earlier live observation; it is not a fresh observation from this audit.
- **Inference:** a consequence of the code or recorded evidence.
- **Unknown/gate:** not established without a controlled runtime or physical check.

## 1. Executive finding

The current system has several independent BlueZ clients, each with its own D-Bus-backed `bluer::Session` and local lifecycle policy. There is no checked-in, repository-wide adapter broker, discovery lease, connection lease, or controller power owner.

The most important conflict is not that every collector scans continuously. It is that the same physical adapter is treated as if discovery, kernel autoconnect, direct `Device1.Connect`, GATT resolution, notification subscription, and adapter power were independently composable. They are not a process-local resource. The recorded Wellue/BP incident is consistent with this design defect:

- Wellue repeatedly attempted discovery while its configured target was absent.
- `bluetoothctl show` reported `Discovering: yes`.
- BP was paired/trusted but disconnected and was waiting for kernel autoconnect.
- Stopping and disabling Wellue changed discovery to `no`; BP then delivered frames without a BP restart or manual connect.

This is strong evidence of cross-collector arbitration failure, but it does **not** prove that discovery was the only BP failure mechanism. Kernel accept-list state, bonded encryption resumption, controller slot exhaustion, and BlueZ/kernel behavior remain separate hypotheses.

The rewrite boundary should therefore be **all Linux BlueZ access**, not merely the Wellue scan loop. One adapter owner should own controller identity, power, discovery, device-object materialization, connect/disconnect serialization, GATT lifecycle, and BP management/autoconnect. Device protocol state machines, pure decoding, journals, and the `vitalsd` durable append boundary should remain outside that owner.

## 2. Repository and service topology

The primary checkout does not contain the current `vitals-collectors` production crate, BP collector, H10 collector, AirSense sources, or Wellue crate. Those are in the two related checkouts. This is an observed repository split, not evidence that corresponding binaries are absent from the Pi.

| Source/path | Repository role | Checked-in runtime owner | BlueZ activity | Current status/evidence |
|---|---|---|---|---|
| EMAY durable | `AnxietyWatch2` | `vitals-emay.service` | Known-object connect, GATT, write, notify; no production scan | Production durable path in code; unit is present |
| EMAY legacy/spike | `AnxietyWatch2` and copied related trees | `emay-live.service` | Discovery, direct connect, GATT, notify/write | Legacy/stopgap; documentation says do not run beside durable collector |
| Polar H10 | `aw2-air11` | `vitals-h10.service` | Direct connect, service resolution, notify, disconnect/retry | Production collector; H10 physical acceptance remains gated |
| H10 spike | all three trees in copied form | Manual | Discovery, direct connect, GATT, notify | Investigative path; can contend when run |
| A&D UA-651BLE BP | `aw2-air11` | `vitals-bp.service` plus privileged autoconnect unit/rule | No direct connect in collector; reacts to kernel/BlueZ link and subscribes | Production rule is deliberate and must be preserved |
| BP stopgap helper | `AnxietyWatch2` | `vitals-bp-connect.service` | Direct Python D-Bus `Device1.Connect`, properties, GATT write | Competing historical/stopgap ownership model |
| AirSense 11 | `aw2-air11` | `vitals-air11.service` | Explicit adapter, direct connect, bounded discovery fallback, GATT, notify/write | Production read-only application stream |
| AirSense onboarding | `aw2-air11` | Manual operator tool | Bounded discovery, direct GATT connect, app-level pairing, disconnect | Separate operator-authorized path |
| Wellue/Viatom | `aw2-wellue-fix` | `wellue-collector.service` | Discovery, direct connect, GATT, notify, possible adapter power-cycle | Intentionally disabled; do not re-enable during this audit |
| Wellue probe | `aw2-wellue-fix` | Manual | Discovery, direct connect, GATT inventory, notify | Another independent owner when run |
| GATT inspection | `AnxietyWatch2` | Manual | `busctl` subprocess reads BlueZ object tree/properties | Read-only inspection tool; no scan |
| BP capture tools | all relevant trees | Manual | Python D-Bus object manager/properties/GATT notification | Diagnostic tooling; not production ownership |

## 3. Exact BlueZ entry points and lifecycle

### 3.1 EMAY durable collector (`AnxietyWatch2`)

**File:** `crates/emay-collector/src/main.rs`

- Linux imports and BlueZ types: lines **38-39**.
- `known_device()` obtains `adapter.device(target)`, reads `Device::is_connected()`, and calls `Device::connect()` if needed: lines **255-269**.
- GATT service and characteristic enumeration: lines **280-305**. The fixed EMAY objects are service `FF12`, write `FF01`, and notify `FF02`: lines **42-45**.
- Production adapter/session boundary: `Session::new()`, `default_adapter()`, `set_powered(true)`, and `known_device()`: lines **1363-1378**.
- Notification subscription: lines **1379-1385**.
- Storage/live protocol lifecycle, including setup, storage paging, keepalive, and stop: lines **337-366**, **1212-1343**.
- Durable publication through `vitalsd`: lines **869-919**. Anonymous live frames are intentionally status-only: lines **1094-1131**.

The durable collector does **not** call `discover_devices()`. If the configured unpaired device object is absent, it reports failure rather than materializing the object by scanning. Its direct `connect()` has no collector-local timeout in `known_device()`; this is a material lifecycle gap relative to H10, Wellue, and AirSense.

**Unit:** `deploy/vitals-emay.service`

- Requires/starts after `bluetooth.target` and `vitalsd.service`: lines **7-8**.
- Runs `emay-collector`, restarts, and supplies the configured target/socket/status environment: lines **14-24**.
- Restricts address families to `AF_UNIX`: line **39**. Whether that is sufficient for the deployed D-Bus path is **unknown** and requires a Linux runtime gate.

### 3.2 EMAY legacy/spike (`AnxietyWatch2`)

**File:** `crates/emay-spike/src/main.rs`

- Target-object lookup and discovery fallback: lines **303-340**.
- `Session::new()`, `default_adapter()`, and `set_powered(true)`: lines **504-506**.
- Direct connection: lines **518-521**.
- GATT enumeration and notification subscription: lines **524-543**.
- Protocol writes, including keepalive and stop: lines **407-417**, **462-494**.

This is an independent discovery and connection owner. It is not equivalent to the durable collector merely because it uses the same EMAY UUIDs. The historical `deploy/stopgap/emay-live.service` runs this binary and only declares `bluetooth.target` dependency at lines **5-6** of that unit; the service definition and README explicitly warn against running it alongside the durable collector.

### 3.3 Polar H10 production collector (`aw2-air11`)

**File:** `crates/vitals-collectors/src/bin/h10_collector.rs`

- Connection cancellation via `Device::disconnect()`: lines **240-251**.
- Bounded `is_connected()`, direct `connect()`, `ServicesResolved` wait, service/characteristic resolution: lines **255-305**.
- Notification subscription and reconnect watchdog: lines **312-405**.
- Session creation and direct adapter/device selection: lines **417-425**.
- Cleanup after failed or stale connect, specifically to avoid BlueZ `Operation already in progress`: lines **438-441**.
- Fixed Heart Rate service `0x180D` and measurement `0x2A37`: lines **31-34**.

The production H10 collector does not discover. It uses `default_adapter()` and a configured target object, then owns the direct link and reconnect loop. It has bounded operation timeouts, but no global reservation preventing a concurrent scan or another process from manipulating the same controller.

**Unit:** `deploy/vitals-h10.service`, lines **3-4** require `bluetooth.service` and `vitalsd.service`; lines **24-27** allow Unix, netlink, and Bluetooth address families. The unit does not depend on an adapter broker or the BP autoconnect unit.

### 3.4 H10 investigative spike

**File:** `AnxietyWatch2/crates/hr-spike/src/main.rs`

- Existing paired-object check and discovery fallback: lines **208-210**.
- Direct connect through the test backend: lines **353-379**.
- `Session::new()`, `default_adapter()`, and `set_powered(true)`: lines **408-410**.
- Fixed H10 service/characteristic resolution and notification capture: lines **391-395** and **285**.
- Operator instructions mention `bluetoothctl pair`, `btmgmt add-device`, and removal/re-pairing: lines **300-312**. These are instructions, not production subprocess execution.

The spike is a separate scan/connect path. Running it during a production capture would be an uncoordinated adapter operation even though its tests are mostly portable.

### 3.5 BP production collector and the no-direct-connect boundary (`aw2-air11`)

**File:** `crates/vitals-collectors/src/bin/bp_collector.rs`

The no-direct-connect rule is explicit in the module documentation at lines **1-14**: this process does not call `Device1.Connect`; the kernel autoconnect path is expected to create the link, after which the collector reacts.

- Session/journal setup: lines **204-236**.
- BlueZ session, `default_adapter()`, `set_powered(true)`, target object, pairing/trust checks: lines **253-270**.
- Explicit waiting message naming the boundary: lines **278-279**.
- Poll for `is_connected()`, wait for services resolution, then capture until disconnect: lines **280-304**.
- BP service `0x1810`, measurement `0x2A35`, and notification/indication subscription: lines **184-221**.
- Raw journal and durable append path: lines **87-163**.

The production collector is therefore a link/GATT consumer, not a discovery owner or direct-connect owner. It still calls `set_powered(true)` and asserts trust, so it is not fully passive at the adapter boundary.

**Kernel/udev ownership:**

- `deploy/99-vitals-bp-autoconnect.rules`, lines **1-4**, starts the autoconnect unit only for `KERNEL=="hci0"`.
- `deploy/vitals-bp-autoconnect.service`, lines **3-12**, documents that the management accept-list does not survive reboot/hotplug and orders the helper before BP.
- Its `ExecStart` invokes `btmgmt add-device -a 2 -t 1`: line **19**.
- `deploy/vitals-bp.service`, lines **3-4**, requires only Bluetooth and `vitalsd`; it does not require the autoconnect unit, so the ordering boundary is not a complete dependency contract by itself.

This is a split owner: privileged `btmgmt`/kernel registration, unprivileged production collector, and BlueZ D-Bus properties/GATT. A rewrite must preserve BP's no-`Device1.Connect` and `sync: true` durability semantics while making the split explicit and coordinated.

### 3.6 BP stopgap D-Bus connector (`AnxietyWatch2`)

**Files:** `deploy/stopgap/vitals-bp-connect` and `deploy/stopgap/vitals-bp-connect.service`

- Adapter is configurable but defaults to `hci0`: lines **44-47**.
- It uses Python `dbus`, BlueZ object paths, `GattCharacteristic1`, `Device1`, and properties: lines **44**, **98-120**, **141-148**.
- It polls `Connected`/`RSSI`: lines **154-187**.
- It calls `Device1.Connect()` when the target appears to advertise: lines **191-195**.
- Its header says it expects another collector's continuous discovery and then connects the BP target: lines **8-33**.
- It deliberately leaves the link for `bp-collector`: lines **67-68**.
- The service is ordered after/requires `bluetooth.service` and before BP: `vitals-bp-connect.service` lines **7-11** and **16-26**.

This path directly conflicts with the production BP module rule. It must be classified as legacy/stopgap and excluded from the new ownership model, not silently combined with kernel autoconnect.

### 3.7 AirSense 11 production collector (`aw2-air11`)

**File:** `crates/vitals-collectors/src/as11_bluez.rs`

- `Air11BleTransport::connect()` opens a BlueZ session, resolves the named adapter/device, checks connection, attempts direct connect, then falls back to bounded discovery: lines **387-419**.
- It resolves service/characteristics, validates write/notify properties and MTU, and subscribes: lines **637-685**.
- It owns a bounded write queue: lines **782-833**.
- Disconnect and writer cleanup: lines **523-530**.
- Discovery guard, filter save/restore, and bounded scan: lines **688-750**.
- Exact FD56 service and TX/RX UUID constants: lines **46-48**.

The production AirSense path is application-authenticated and read-only at the device protocol layer; it does not perform generic BlueZ pairing or control RPC. It does, however, perform direct BlueZ connection and a filtered scan when the object is absent.

**File:** `crates/vitals-collectors/src/bin/resmed_air11_collector.rs`

- Adapter option defaults to `hci0` and is parsed at lines **709-766**.
- Production unit passes `--adapter hci0`: `deploy/vitals-air11.service` line **17**.
- Reconnect supervision invokes `Air11BleTransport::connect()` at lines **336-351** and retries at lines **282-306**.

**Unit:** `deploy/vitals-air11.service`, lines **3-4** require Bluetooth and `vitalsd`; line **17** fixes the adapter name; lines **27-30** sandbox the process and permit `AF_UNIX AF_NETLINK AF_BLUETOOTH`.

### 3.8 AirSense onboarding

**Files:** `crates/vitals-collectors/src/as11_onboarding.rs` and `src/bin/resmed_air11_onboard.rs`

- Adapter lookup and powered check: lines **560-569**.
- Local active-discovery rejection, filter save/restore, and bounded candidate scan: lines **569-598**, **628-667**.
- Target/device lookup and direct GATT connect: lines **671-748**.
- Bounded disconnect: lines **791-800**.
- CLI adapter default `hci0` and `--adapter` option: `resmed_air11_onboard.rs` lines **93-104**, usage line **170**.
- Pairing tool calls discovery and then `Air11PairingLink::connect()`: lines **213-249**; it disconnects the temporary link at lines **249-261**.

The local `is_discovering()` guard prevents this path from taking over an already-active scan, but it is not a cross-process lock. It cannot prevent a race between the check and scan start, and it does not coordinate with Wellue, EMAY spike, H10 spike, or the BP kernel path.

### 3.9 Wellue collector (`aw2-wellue-fix`)

**File:** `crates/wellue-collector/src/main.rs`

- The target finder checks an existing object, then unconditionally starts `discover_devices()` if the target is not already connected: lines **211-238**.
- It does not check `is_discovering()` and does not restore a previous discovery filter.
- Direct connect, service resolution, and notification subscription: lines **507-562**.
- Disconnect cleanup: lines **437-450**.
- Controller reset on selected stale-link errors: lines **457-501**. This performs `set_powered(false)` and `set_powered(true)` on the shared adapter.
- Session/default-adapter/power-on and retry loop: lines **611-648**.
- Fixed target requirement and refusal to discover an arbitrary device: lines **658-659**.
- Local tests cover retry interval and reset predicate only: lines **684-711**.

**Unit:** `deploy/wellue-collector.service`, lines **7-8** require `bluetooth.target` and `vitalsd`; lines **14-19** select the user/group and environment; lines **20-35** restart and sandbox the collector. The brief requires this unit to remain disabled during the investigation.

The 10-second scan/one-minute retry is a local mitigation, not arbitration. The adapter power-cycle path is more dangerous than discovery alone because it can disrupt every other link on the controller.

### 3.10 Wellue probe

**File:** `crates/wellue-probe/src/main.rs`

- Target discovery: lines **41-57**.
- Default adapter and power-on: lines **144-155**.
- Direct connect: lines **167-174**.
- GATT inventory and notification capture: lines **70-100**, **180-188**.

This is a separate manual discovery/connect owner and must be brokered or physically isolated before use on a production appliance.

### 3.11 Python D-Bus and subprocess tools

**BP capture:** `tools/bp_capture.py`

- Explains why `bluetoothctl connect` was not used and why kernel accept-list autoconnect is required: lines **4-12**.
- Imports Python D-Bus and BlueZ interfaces: lines **26-32**.
- Opens the system bus and enumerates object/GATT trees: lines **193-225**.
- Calls `GattCharacteristic1.StartNotify()`: lines **243-260**.
- Subscribes to `PropertiesChanged`: lines **406-409**.
- Declares that kernel autoconnect brings the link up: lines **434-440**.

**EMAY GATT inspection:** `tools/emay-investigation/inspect_emay_gatt.py`

- Uses `subprocess.run()` to invoke `busctl`: lines **8-13**.
- Reads the BlueZ object tree and only accepts an already-known `/org/bluez/hciN` object: lines **16-22**.
- Reads `GattService1`, `GattCharacteristic1`, and `GattDescriptor1` properties and optionally calls `ReadValue`: lines **40-99**.
- It does not scan or connect.

**Raw HCI:** no checked-in application source was found that opens `AF_BLUETOOTH`/`SOCK_RAW` or uses HCI ioctl APIs directly. `btmgmt` is invoked as a privileged subprocess by the BP systemd unit; Python D-Bus and `busctl` are separate entry points. This conclusion is limited to the inspected source/config files and does not characterize binaries installed on the Pi.

## 4. Operation inventory

| Operation | Checked-in callers | Ownership problem |
|---|---|---|
| Open BlueZ D-Bus session | All Rust BlueZ clients listed above; `Session::new()` at the cited source boundaries | Multiple independent sessions have no shared lease state |
| Select default adapter | EMAY durable, EMAY spike, H10 production/spike, BP, Wellue, Wellue probe | Default can change when controllers appear/disappear |
| Select explicit adapter | AirSense production/onboarding; BP stopgap environment | Production AirSense and BP rules assume `hci0` |
| Power on | EMAY, H10, BP, Wellue, probes/spikes | Several processes can mutate shared controller power |
| Power off/on recovery | Wellue lines **457-501** | One optional collector can reset active links owned by other sources |
| Start discovery | EMAY spike, H10 spike, AirSense fallback/onboarding, Wellue, Wellue probe | No repository-wide lock; Wellue has no active-scan check |
| Check discovery state | AirSense production/onboarding only | Local check is race-prone and does not cover all callers |
| Save/restore discovery filter | AirSense production/onboarding | Not established for Wellue/EMAY/H10 spikes |
| Direct connect | EMAY, H10, AirSense, onboarding, Wellue, probes/spikes, BP stopgap | Competes with BP's kernel-autoconnect policy and other connection attempts |
| Kernel autoconnect registration | `btmgmt add-device` systemd unit | Privileged path tied to `hci0`; not represented as a broker lease |
| Disconnect/cancel | H10, AirSense, Wellue; BP waits for link end; EMAY shutdown path | Cancellation is source-local; no owner/priority policy |
| GATT service/characteristic resolution | Every source-specific transport | Repeated independently; no centralized object lifecycle |
| Notify/indicate subscription | EMAY, H10, BP, AirSense, Wellue, Python capture | Multiple processes may subscribe or manipulate one device object |
| Pair/trust/agent | BP checks/sets `Trusted`; onboarding performs application-level pairing; operator docs invoke `bluetoothctl pair` | No checked-in shared pairing/agent owner; no `Agent1` implementation found |
| Management/raw HCI | `btmgmt` subprocess only; no raw HCI application socket found | Controller-management policy is outside Rust collector lifecycle |

## 5. Adapter/controller assumptions

### 5.1 `hciN` is not a stable identity

Observed code/config assumptions:

- BP udev rule matches `hci0`: `aw2-air11/deploy/99-vitals-bp-autoconnect.rules:1-4`.
- BP autoconnect comments and command are in `aw2-air11/deploy/vitals-bp-autoconnect.service:3-19`.
- AirSense production unit passes `--adapter hci0`: `aw2-air11/deploy/vitals-air11.service:17`.
- AirSense collector and onboarding default to `hci0`: `resmed_air11_collector.rs:709-766` and `resmed_air11_onboard.rs:93-104`.
- EMAY, H10, BP, Wellue, and their probes use `default_adapter()` at the source ranges cited above.
- Stopgap BP defaults its adapter environment to `hci0`: `AnxietyWatch2/deploy/stopgap/vitals-bp-connect:44-47`.

Recorded historical evidence says the intended Sena UD100-G03/CSR controller was once `hci1` beside the onboard controller and could later become `hci0` after onboard Bluetooth was disabled. A later acceptance record reports only one powered `hci0` and says the required Sena `hci1` was absent: `AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md:76-94`. These facts are mutually incompatible with using `hci0` or `hci1` as a durable hardware identity.

### 5.2 Current controller is unknown

The repository does not establish the controller currently attached to the appliance. The following are all unknown until a controlled read-only runtime gate:

- controller Bluetooth address;
- USB VID/PID and physical bus path;
- driver and firmware;
- BlueZ version and kernel version at the same observation;
- whether the active device is the intended Sena/CSR adapter;
- whether onboard Bluetooth is enabled;
- adapter connection-slot capacity and behavior under concurrent links.

The G1 rule remains absolute: do not apply `dtoverlay=disable-bt` until the Sena dongle is verified UP as the intended controller. No such operation was attempted here.

### 5.3 Stable target identity is also incomplete

Most collectors use a configured static address. BP pairing documentation correctly emphasizes IRK/bond state rather than static-MAC trust, but the production code still obtains a `Device` from a configured address or `hciN` object. EMAY is explicitly unpaired/unbonded in the hardware notes, so its transient BlueZ object may disappear across restart. AirSense has a configured application target plus a bounded discovery fallback. The rewrite needs a stable target identity model that distinguishes:

1. controller identity;
2. bonded/resolvable peer identity;
3. current BlueZ object path/address;
4. application protocol identity; and
5. absence, out-of-range, unpaired, connected-elsewhere, and stale-object states.

## 6. Contention analysis

### 6.1 Code-level contention

1. **Discovery singleton:** Wellue starts discovery without checking whether another process owns it (`wellue-collector/src/main.rs:211-238`). AirSense checks `is_discovering()` but only locally (`as11_bluez.rs:688-724`; `as11_onboarding.rs:569-591`). EMAY spike, H10 spike, and Wellue probe also start discovery. There is no cross-process lock or broker.
2. **BP autoconnect versus user-space discovery:** BP waits for kernel autoconnect (`bp_collector.rs:1-14,278-304`), while the stopgap helper explicitly depends on another process's discovery and calls `Device1.Connect` (`vitals-bp-connect:8-33,183-195`). This creates two incompatible BP ownership models.
3. **Adapter power:** every direct collector can call `set_powered(true)`; Wellue can call `set_powered(false)` and then `true` during recovery (`wellue-collector/src/main.rs:457-501`). No code checks whether H10, EMAY, AirSense, or BP holds an active link before resetting the controller.
4. **Device object materialization:** EMAY durable and direct-connect H10 paths assume an address-backed BlueZ object; AirSense and Wellue have different discovery fallbacks. The same missing-object state therefore produces different retry behavior and different pressure on discovery.
5. **Connection cancellation:** H10 and AirSense cancel pending operations, but source-local cleanup does not prevent another process from attempting the same peer/controller operation.
6. **Service startup:** systemd `After=`/`Requires=` declarations serialize startup against Bluetooth and `vitalsd`, not against peer collectors or a shared adapter owner. A systemd unit can therefore start while another unit is scanning or changing adapter state.

### 6.2 Recorded runtime evidence

The brief records the strongest incident evidence at `docs/reviews/bluetooth-rewrite/BRIEF.md:14-20`:

- absent Wellue target plus repeated discovery;
- adapter `Discovering: yes`;
- BP paired/trusted but disconnected and waiting;
- Wellue stop/disable followed immediately by discovery `no`;
- BP frames 8 and 9 became durable without BP restart/manual connect.

This correlates Wellue discovery with BP recovery. It is not a fresh observation from this audit and does not exclude a coincident kernel/autoconnect recovery or timing effect.

Separate recorded EMAY evidence shows the configured unpaired EMAY object was absent, a single bounded discovery recreated it, and the existing collector then connected: `AnxietyWatch2/docs/status-updates/2026-09-05T08-09-24Z-emay-recovery.md:65-78,127-148,184-197`. This supports the missing-transient-object hypothesis but does not prove that every missing object should be recovered by every collector scanning independently.

Separate AirSense status symptoms must not be folded into the discovery incident. The brief explicitly records a possible `streaming` versus therapy-lifecycle/session-state problem at `BRIEF.md:21`.

### 6.3 Contention matrix

| Actor | Scans | Direct connect | Kernel autoconnect | Adapter power mutation | Link/GATT |
|---|---:|---:|---:|---:|---|
| EMAY durable | No | Yes | No | On | FF12/FF01/FF02 |
| EMAY spike/legacy | Yes | Yes | No | On | FF12/FF01/FF02 |
| BP production | No | **No** | **Yes** via external management unit | On | 1810/2A35 indication |
| BP stopgap | No, depends on another scan | **Yes** | No | No | Device properties/GATT write |
| H10 production | No | Yes | No | On | 180D/2A37 notify |
| H10 spike | Yes | Yes | No | On | 180D/2A37 notify |
| AirSense production | Bounded fallback | Yes | No | Adapter selected by name | FD56/RPC |
| AirSense onboarding | Bounded filtered | Yes | No | Requires powered adapter | FD56 pairing |
| Wellue collector | **Yes** | Yes | No | **Off/on recovery** | provisional NUS |
| Wellue probe | Yes | Yes | No | On | arbitrary notify |

## 7. Instrumentation and observability gaps

The current code has useful source-local logs and status files, but cannot answer the central arbitration questions reliably.

### Missing or inadequate facts

- No stable `controller_id` in collector logs. Logs generally show `adapter.name()` or no adapter at all, not controller Bluetooth address, USB path, VID/PID, driver, BlueZ version, or kernel version.
- No discovery lease/owner id, D-Bus unique name, process id, start time, stop time, filter, duration, or reason for every discovery transition.
- No single event stream correlates `is_discovering`, `DeviceAdded/Removed`, direct connect attempts, kernel management events, `Connected`, `ServicesResolved`, GATT resolution, notification subscription, and durable append.
- No metric distinguishes “device object absent,” “device out of range,” “not advertising,” “connected elsewhere,” “connect operation pending,” “bond/encryption failure,” and “controller reset.”
- No controller-wide count of active links, discovery users, pending operations, or leases.
- No evidence in the production path that a discovery filter is restored after every Wellue/legacy timeout/error; AirSense has a local restore path, but no global owner verifies it.
- No audit record for adapter power mutation or a guard showing which links could be disrupted before Wellue resets the adapter.
- BP’s passive wait can look like a healthy quiet interval. The collector names degraded `ServicesResolved` failure, but does not expose the kernel accept-list action/whitelist state in its status path.
- Source status is process-centric. A running collector does not prove a usable BlueZ link or fresh physiological evidence; this is especially relevant to H10 acceptance.
- Existing tests exercise local predicates and fake protocol streams, not two independent processes contending for one BlueZ adapter.

### Required broker telemetry

Every operation should emit a structured event containing at least:

`controller_id`, current adapter object/name, peer identity, lease id, owner, operation, priority, start/deadline/end, result, BlueZ error name, kernel-management result if applicable, discovery filter, connection state, `ServicesResolved`, and whether the action changed controller power.

The health projection should expose three separate facts: **measured link**, **transport readiness**, and **fresh durable sample**. “Connected” alone must not be rendered as measured or healthy.

## 8. Proposed rewrite boundary

### 8.1 One broker per physical controller

Create one Linux Bluetooth supervisor/broker per physical controller. It alone owns:

- one BlueZ D-Bus session and adapter object;
- stable controller resolution using controller address plus sysfs/USB identity, never `hciN` alone;
- controller power, rfkill, and recovery policy;
- the singleton discovery operation and filter restoration;
- device-object materialization and current peer mapping;
- serialized connect/disconnect, service resolution, and notification subscription;
- management/autoconnect registration for BP through a narrowly scoped privileged helper or broker boundary; and
- leases, deadlines, cancellation, and owner-visible blocker reasons.

A collector must not call `discover_devices`, `set_powered`, adapter reset, or direct `Device1.Connect` after migration. The BP exception is preserved as a broker-managed kernel/autoconnect operation; it must not become an ordinary direct-connect retry.

### 8.2 Broker/collector IPC

Use a private Unix IPC protocol with typed operations such as:

- `ResolveController(role_or_stable_id)`;
- `AcquireDiscovery(filter, priority, deadline)`;
- `AcquirePeer(peer_identity, mode, deadline)`;
- `Subscribe(characteristic, notification_mode)`;
- `Write(characteristic, bounded_payload)`;
- `EnableBpAutoconnect(peer_identity)`;
- `Release(lease_id)`;
- event stream for `DiscoveryState`, `DeviceObject`, `Connected`, `ServicesResolved`, `GattReady`, `Notification`, `Disconnected`, `ControllerChanged`, and `LeaseBlocked`.

Each lease names the controller, peer, owner, operation, deadline, and recovery owner. Discovery is queued or rejected with the current owner named; it is never silently stolen. Broker crash recovery must close/clear owned operations rather than leave an unbounded scan or power-cycle loop.

Priority should reflect source semantics rather than service startup order:

1. BP autoconnect and an already-established BP indication session;
2. established continuous H10/EMAY/AirSense links;
3. bounded recovery for production sources;
4. operator onboarding and diagnostic probes;
5. optional Wellue discovery.

The exact policy requires physical/runtime validation. The priority ordering is a design proposal, not a claim that the current adapter can satisfy all sources concurrently.

### 8.3 Keep protocol and safety boundaries outside the broker

The broker must not decode physiology, apply registry ranges, write SQL, or own source-specific durable identity. Keep these in existing protocol workers:

- BP: raw journal before decode, `sync: true` append before ATT-confirmation-visible completion, and explicit connected-but-unresolved state.
- H10: 0x180D/0x2A37 notification decoding, RR/HR session identity, watchdog, and `vitalsd` publication.
- EMAY: FF12 command/notification multiplexing, stored-session identity, and live anonymous status-only boundary.
- AirSense: FD56 framing, application authentication, read-only RPC allowlist, journals, and session state.
- Wellue: provisional protocol path remains disabled until the global arbitration and physical/protocol gates pass.

`vitalsd` remains the sole durable append boundary. The broker transports bounded GATT events; it does not become a second spool or database writer.

## 9. Staged migration and rollback

No stage below was executed.

### Stage 0 — read-only baseline

Collect, without changing state, the controller inventory, BlueZ version, kernel version, active adapter power/discovery state, BlueZ object tree, paired/bond state, installed/enabled unit state, and current per-source status. Reconcile the result with this document. Wellue remains disabled.

**Gate:** controller identity is verified as the intended hardware; `hciN` is recorded only as a current alias; G1 remains closed until the Sena requirement is actually met.

### Stage 1 — broker shadow/test mode

Build the broker against sanitized traces and fake BlueZ interfaces. In shadow mode it must not scan, connect, power-cycle, pair, modify management state, or start production collectors.

Required tests include discovery race, filter restoration, missing object, target appearing/disappearing, connect timeout cancellation, connected-but-unresolved, controller renumbering, broker restart, and simultaneous lease requests.

### Stage 2 — migrate bounded discovery

Move AirSense production fallback and onboarding first, because those paths already have explicit bounded filters and active-scan errors. Then migrate manual probes and legacy discovery paths. Do not run old and brokered discovery clients together on one controller.

**Rollback:** stop the broker client path, restore the previously reviewed collector binary/unit, and keep Wellue disabled. Do not restore BP by enabling the direct-connect stopgap.

### Stage 3 — migrate continuous links

Move H10 and EMAY link ownership to the broker while preserving their existing protocol workers, journals, status semantics, and `vitalsd` contracts. Acceptance must show fresh source-specific measurements, not merely a process or BlueZ `Connected` property.

### Stage 4 — migrate BP last

Unify the privileged management/autoconnect registration and passive BP link consumer behind the broker boundary. Preserve:

- no production `Device1.Connect` for BP;
- kernel/management accept-list semantics;
- `ServicesResolved` degradation reporting;
- raw frame journal before decode; and
- `sync: true` durable append before the source can lose the frame.

**Gate:** fresh BP frames and durable rows across cold boot, adapter hotplug, a missed advertisement, and a concurrent bounded discovery request. No direct-connect fallback is acceptable.

### Stage 5 — Wellue decision gate

Wellue stays disabled. Reconsider only after exact device/protocol evidence, broker lease tests, controller identity verification, no-power-cycle-while-leased enforcement, and explicit owner approval. The disabled state is the rollback/default state.

### Stage 6 — retire legacy paths

Only after a bounded live window, restart test, and rollback rehearsal should the old direct BlueZ code and units be removed. Retain old binaries, units, raw journals, and configuration rollback material until the new path has passed operator verification.

## 10. Unresolved questions and runtime/physical gates

1. **Which controller is physically present now?** Need address, USB VID/PID, bus path, driver, BlueZ version, kernel version, and power state from one read-only observation.
2. **Is the intended Sena/CSR adapter present and UP?** Historical design and acceptance records conflict; no current claim is valid.
3. **Which units/binaries are actually installed and enabled?** Repository files do not establish Pi deployment state.
4. **Does the current kernel/BlueZ preserve BP kernel autoconnect and indication behavior after cold boot, USB hotplug, and controller renumbering?**
5. **What are the actual controller connection slots and failure modes with H10, EMAY, AirSense, and BP?** Documented slot counts are not current hardware evidence.
6. **Can bounded broker discovery coexist with BP autoconnect without starvation?** Must be tested with the actual controller and cuff.
7. **Does BlueZ permit the required filtered discovery and object materialization behavior when another link is active?**
8. **What exact state explains BP bonded/trusted but `Connected: no` in each incident?** Discovery contention is supported, but kernel accept-list, encryption resumption, and timing remain alternatives.
9. **Can the BP Date Time write establish the desired device timestamp flag?** The repository marks this untested; it must not change the durability/identity contract.
10. **Does EMAY’s `AF_UNIX` sandbox permit the deployed `bluer`/D-Bus path?** Requires a controlled Linux service test.
11. **Can every broker operation be cancelled without leaving BlueZ `Operation already in progress` or an active scan?**
12. **What stable identity survives adapter reboot/hotplug?** Address alone and `hciN` alone are insufficient design candidates.
13. **Can the AirSense `streaming`/therapy `complete` symptom be reproduced with discovery disabled?** Treat it as separate until shown otherwise.
14. **What is the exact Wellue hardware/protocol and enablement state?** Do not infer from the repository; do not re-enable it for investigation.
15. **What broker restart behavior preserves BP durable semantics and prevents duplicate sessions for H10/EMAY/AirSense?**
16. **Does an external phone/vendor app hold any source’s single-connection slot during acceptance?** Must be controlled and recorded, not guessed.

## 11. Exact files, APIs, and tests to change or inspect

### BlueZ transport and protocol boundaries

- `AnxietyWatch2/crates/emay-collector/src/main.rs`: `known_device`, `characteristics`, `CollectorBackend::connect`, storage/live lifecycle, status publication.
- `AnxietyWatch2/crates/emay-spike/src/main.rs`: `find`, `CaptureBackend`, BlueZ connection and legacy discovery.
- `AnxietyWatch2/crates/hr-spike/src/main.rs`: H10 discovery/connect boundary and test backend.
- `aw2-air11/crates/vitals-collectors/src/as11_bluez.rs`: `Air11BleTransport::connect`, `discover_target`, `resolve_pipe`, `write_packet`, disconnect.
- `aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs`: `discover_target`, `Air11PairingLink::connect`, disconnect and filter restoration.
- `aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs`: passive link state machine, trust assertion, services resolution, raw journal, `sync: true` append.
- `aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs`: direct connection, cancellation, watchdog, reconnect and notification subscription.
- `aw2-air11/crates/vitals-collectors/src/bin/resmed_air11_collector.rs`: adapter option, reconnect supervision, status projection.
- `aw2-wellue-fix/crates/wellue-collector/src/main.rs`: discovery, direct connect, power reset, retry, status.
- `aw2-wellue-fix/crates/wellue-probe/src/main.rs`: manual probe discovery/connect/GATT ownership.

### Deployment and adapter identity

- `aw2-air11/deploy/99-vitals-bp-autoconnect.rules`.
- `aw2-air11/deploy/vitals-bp-autoconnect.service`.
- `aw2-air11/deploy/vitals-bp.service`.
- `aw2-air11/deploy/vitals-h10.service`.
- `aw2-air11/deploy/vitals-air11.service`.
- `AnxietyWatch2/deploy/vitals-emay.service`.
- `AnxietyWatch2/deploy/stopgap/emay-live.service`.
- `AnxietyWatch2/deploy/stopgap/vitals-bp-connect` and `.service`.
- `aw2-wellue-fix/deploy/wellue-collector.service`.
- `AnxietyWatch2/scripts/sync-to-pi.sh`: deployment boundary only; do not run it as part of this audit.

### Diagnostic evidence

- `docs/reviews/bluetooth-rewrite/BRIEF.md`.
- `docs/specs/AMENDMENTS.md`, amendment 05, and amendment 06.
- `aw2-air11/docs/research/2026-08-03-ua651ble-ble-capture.md`.
- `AnxietyWatch2/docs/status-updates/2026-09-05T08-09-24Z-emay-recovery.md`.
- `AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md`.
- `aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md`: historical controller/G1 evidence only.
- `AnxietyWatch2/tools/emay-investigation/inspect_emay_gatt.py`.
- `tools/bp_capture.py` in the relevant checkout.

### Required broker APIs/tests

The implementation pass should add a Linux-only BlueZ adapter abstraction with fake/test backends and tests for:

1. stable controller resolution independent of `hciN`;
2. one active discovery owner per controller;
3. discovery filter restoration on success, timeout, cancellation, and D-Bus failure;
4. lease priority, queueing, cancellation, expiry, and owner-labelled blockers;
5. no adapter power-cycle while another lease is active;
6. device-object materialization and address/IRK identity transitions;
7. direct-connect timeout plus guaranteed cancellation;
8. `Connected` versus `ServicesResolved` as separate states;
9. GATT resolution and notification subscription ownership;
10. BP no-direct-connect enforcement, kernel autoconnect registration, and `sync: true` reply handling;
11. broker restart without abandoned discovery or duplicate source sessions;
12. cross-process integration trace proving absent Wellue cannot hold discovery indefinitely or starve BP;
13. source-specific fresh-measurement gates for H10, EMAY, BP, AirSense, and—only after approval—Wellue.

## 12. Bottom line

The current local fixes—bounded Wellue discovery, AirSense active-scan rejection, H10 connect cancellation, BP startup ordering, and EMAY bounded recovery guidance—are useful source-level mitigations. They do not create a global policy. The rewrite should centralize all BlueZ resource ownership, preserve BP’s special kernel-autoconnect/durability boundary, and leave protocol decoding and durable ingestion in their existing safety-oriented layers. Hardware identity, current deployment state, H10 acceptance, and Wellue enablement remain gates, not assumptions.
