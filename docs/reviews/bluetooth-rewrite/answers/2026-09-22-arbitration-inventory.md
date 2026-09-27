# Answer: BlueZ arbitration inventory

**Request:** `2026-09-22-arbitration-inventory.md`
**Status:** ANSWERED, with runtime unknowns explicitly marked.
**Evidence date:** static repository/document inspection only; no Pi access or service changes.

## 1. Linux Bluetooth entry points

### Rust/BlueZ entry points

| Repository/path | Entry point and ownership | Exact evidence |
|---|---|---|
| `AnxietyWatch2/crates/emay-collector/src/main.rs` | Production EMAY durable collector; opens BlueZ, default adapter, known-device connect, FF12/FF01/FF02 GATT, notification stream | `main.rs:1363-1386`, UUIDs `main.rs:42-45`, known-device `main.rs:255-300` |
| `AnxietyWatch2/crates/emay-spike/src/main.rs` | EMAY overnight/legacy spike; default adapter, active discovery, direct connect, GATT notify | `emay-spike/src/main.rs:238-249,329-361,567-612` |
| `AnxietyWatch2/crates/hr-spike/src/main.rs` | H10 acceptance spike; default adapter, active discovery, direct connect, 180D/2A37 notify | `hr-spike/src/main.rs:102-121,207-220,365-411` |
| `aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs` | Production BP; default adapter and known target; **waits for autoconnect, does not call `Device1.Connect`**; 1810/2A35 indication | `bp_collector.rs:1-14,184-221,253-304` |
| `aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs` | Production H10; default adapter, direct connect/reconnect, 180D/2A37 notify, disconnect cleanup | `h10_collector.rs:240-307,309-430` |
| `aw2-air11/crates/vitals-collectors/src/as11_bluez.rs` | Production AirSense transport; explicit adapter, direct connect, bounded FD56 discovery fallback, exact TX/RX subscription | `as11_bluez.rs:374-432,637-681,688-750` |
| `aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs` | Operator AirSense discovery/pairing; explicit adapter, bounded FD56 discovery, direct GATT pairing link | `as11_onboarding.rs:303-394,454-561` |
| `aw2-air11/crates/vitals-collectors/src/bin/resmed_air11_collector.rs` | AirSense process wrapper/supervisor around `Air11BleTransport` | `resmed_air11_collector.rs:330-429,695-766` |
| `aw2-air11/crates/vitals-collectors/src/bin/resmed_air11_onboard.rs` | Interactive operator wrapper; requires confirmation, invokes bounded discovery/pairing | `resmed_air11_onboard.rs:1-7,151-190,205-254` |
| `aw2-wellue-fix/crates/wellue-collector/src/main.rs` | Production Wellue; default adapter, active discovery, direct connect, provisional NUS notify, and selected adapter power-cycle recovery | `wellue-collector/src/main.rs:211-264,438-501,517-629` |
| `aw2-wellue-fix/crates/wellue-probe/src/main.rs` | Wellue first-contact probe; default adapter, active discovery, direct connect, arbitrary GATT inventory/notify capture | `wellue-probe/src/main.rs:41-63,65-124,140-185` |

The primary checkout lacks the `vitals-collectors` and production Wellue paths; that absence is
recorded in `LIBRARIAN.md`, not treated as proof that deployed binaries are absent.

### Python, command, udev, and deployment entry points

- `AnxietyWatch2/deploy/stopgap/vitals-bp-connect` is a D-Bus Python connector. It obtains
  `org.bluez.Device1` and `org.freedesktop.DBus.Properties`, polls `Connected`/`RSSI`, and calls
  `Device1.Connect()` `[vitals-bp-connect:116-195]`.
- `aw2-air11/deploy/vitals-bp-autoconnect.service` runs privileged
  `btmgmt add-device -a 2 -t 1 "$BP_CUFF_MAC"` `[vitals-bp-autoconnect.service:15-20]`.
- `aw2-air11/deploy/99-vitals-bp-autoconnect.rules` starts that unit when Bluetooth `hci0` is
  added `[99-vitals-bp-autoconnect.rules:1-4]`.
- `AnxietyWatch2/scripts/sync-to-pi.sh` only uses `rsync` and SSH and explicitly identifies
  BlueZ work as Pi-only; it is not a Bluetooth client `[scripts/sync-to-pi.sh:1-28]`.
- A repository search found no checked-in production use of raw `AF_BLUETOOTH` sockets, explicit
  `org.bluez.Agent1`, `bluetoothctl` subprocesses, or D-Bus property-signal subscriptions.
  `bluetoothctl`, `hcitool`, and related commands occur in documentation/runbooks; the checked-in
  production management subprocess is `btmgmt` above. This is a code-search result, not proof
  that an untracked installed script cannot exist.

## 2. BlueZ object/API usage

- **Session/adapter:** `bluer::Session::new()`, `default_adapter()` and
  `Adapter::set_powered(true)` are used by EMAY, H10, BP, Wellue, Wellue probe, and spikes:
  `AnxietyWatch2/crates/emay-collector/src/main.rs:1367-1378`,
  `aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs:420-423`,
  `aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:253-256`,
  `aw2-wellue-fix/crates/wellue-collector/src/main.rs:613-623`.
  AirSense uses `Session::adapter(adapter_name)` instead `[as11_bluez.rs:395-400]`.
- **Discovery:** `Adapter::discover_devices()` is called by EMAY spike
  `[emay-spike/src/main.rs:329-343]`, H10 spike `[hr-spike/src/main.rs:207-220]`, Wellue
  `[wellue-collector/src/main.rs:221-238]`, Wellue probe `[wellue-probe/src/main.rs:48-61]`,
  AirSense production fallback `[as11_bluez.rs:719-750]`, and AirSense onboarding
  `[as11_onboarding.rs:454-507]`.
- **Discovery arbitration:** AirSense production and onboarding check
  `Adapter::is_discovering()`, install an LE UUID filter, and restore the prior filter
  `[as11_bluez.rs:688-710; as11_onboarding.rs:428-453]`. Wellue and the two spikes do not call
  `is_discovering()` before starting discovery. There is no global lease/lock in the checked-in
  code.
- **Device lifecycle:** `Adapter::device(address)`, `Device::is_connected()`, `connect()`, and
  `disconnect()` occur in the source-specific paths cited in Section 1. H10 explicitly cancels
  failed/pending connections via `disconnect()` `[h10_collector.rs:240-252]`; Wellue does the same
  `[wellue-collector/src/main.rs:438-455]`; AirSense disconnects through its transport
  `[as11_bluez.rs:524-530]`.
- **Pair/trust:** BP reads `is_paired()` and `is_trusted()` and may call `set_trusted(true)`
  `[bp_collector.rs:260-278]`; no collector calls a generic BlueZ pair method. AirSense's
  application pairing is its own FD56 protocol, not `Agent1`/BlueZ pairing
  `[as11_onboarding.rs:454-561]`. H10 pairing/autoconnect is documented as an operator step,
  while the collector only checks pairing `[bp_collector.rs:260-270]` and the H10 spike checks
  pairing `[hr-spike/src/main.rs:372-382]`.
- **GATT/ATT:** all production GATT operations are through `bluer::gatt::remote` service and
  characteristic objects. `Characteristic::notify()` is used for BP 2A35, H10 2A37, EMAY FF02,
  Wellue NUS, AirSense RX, and probe-selected characteristics. AirSense additionally uses
  `write_ext()` with bounded MTU chunks `[as11_bluez.rs:793-833]`.
- **Properties/subscriptions:** code polls `Connected`, `ServicesResolved`, RSSI, pairing and
  trust through `bluer` methods or Python D-Bus properties. `AdapterEvent` streams are used for
  discovery. No checked-in collector subscribes to a general BlueZ `PropertiesChanged` signal;
  this is an important missing seam for a broker.

## 3. Adapter/controller assumptions

- `default_adapter()` is assumed by EMAY, BP, H10, Wellue, probes, and spikes. AirSense defaults
  to `hci0` and the unit hardcodes `--adapter hci0` `[resmed_air11_collector.rs:709-766;
  deploy/vitals-air11.service:17]`.
- The BP udev rule matches `KERNEL=="hci0"` `[aw2-air11/deploy/99-vitals-bp-autoconnect.rules:4]`.
  The stopgap Python connector defaults `BP_ADAPTER` to `hci0` `[AnxietyWatch2/deploy/stopgap/vitals-bp-connect:46-48]`.
- Historical evidence records Sena/CSR USB `0a12:0001`, OUI `00:01:95`, initially `hci1`, and
  subsequent renumbering to `hci0` after onboard Bluetooth was removed
  `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:1267-1324]`.
- Current physical controller identity, USB path, whether the active device is Sena, and whether
  the system currently has one or more controllers are **UNKNOWN**. The later acceptance record
  saw one `hci0` and explicitly says required `hci1` was absent
  `[AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md:76-82]`.
- `dtoverlay=disable-bt` is a destructive/runtime gate and was not touched. The required ordering
  is documented at `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:190-220]`.

## 4. Ownership and startup/order

| Source | Unit | Startup/order and owner |
|---|---|---|
| BP | `aw2-air11/deploy/vitals-bp.service` | after/requires `bluetooth.service vitalsd.service`; `vitals` user; restart `[3-37]` |
| BP autoconnect | `aw2-air11/deploy/vitals-bp-autoconnect.service` | after/requires `bluetooth.service`, before BP, root `btmgmt` oneshot `[3-22]` |
| BP stopgap | `AnxietyWatch2/deploy/stopgap/vitals-bp-connect.service` | after/requires Bluetooth, before BP, `vitals:bluetooth` `[7-26]` |
| H10 | `aw2-air11/deploy/vitals-h10.service` | after/requires Bluetooth and vitalsd; `vitals`; restart `[3-37]` |
| AirSense | `aw2-air11/deploy/vitals-air11.service` | after/requires Bluetooth and vitalsd; explicit `hci0`, `vitals`; restart `[1-53]` |
| EMAY durable | `AnxietyWatch2/deploy/vitals-emay.service` | after/requires `bluetooth.target vitalsd.service`; operator login plus `vitals`; restart `[7-24]` |
| EMAY legacy | `aw2-air11/deploy/stopgap/emay-live.service` | after/requires `bluetooth.target`; runs `emay-spike` `[5-32]` |
| Wellue | `aw2-wellue-fix/deploy/wellue-collector.service` | after/requires `bluetooth.target vitalsd`; `vitals` plus Bluetooth; restart `[7-35]`; brief says disabled |

Every source shares the adapter even when its systemd unit only orders against Bluetooth readiness.
No unit expresses a global discovery or connection lease.

## 5. BP invariant and autoconnect evidence

The production BP source comments and implements the boundary: it never calls `Device1.Connect`,
waits for a kernel-connected device, waits for services to resolve, then subscribes to 2A35
`[aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:1-14,253-304]`. The separate privileged
unit registers kernel auto-connect with `btmgmt add-device -a 2 -t 1` and is retriggered by the
hci0 udev rule `[aw2-air11/deploy/vitals-bp-autoconnect.service:15-20; 99-vitals-bp-autoconnect.rules:1-4]`.

The stopgap Python helper is a separate, conflicting path: it calls `Device1.Connect()` when RSSI
is present `[AnxietyWatch2/deploy/stopgap/vitals-bp-connect:183-195]`. It must not be mistaken for
the production BP invariant.

Amendment 06 requires `sync: true`, receipt of the vitalsd reply, and a raw journal because the
kernel ATT confirmation precedes application durability `[aw2-air11/docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md:110-149]`.

## 6. Wellue contention and BP recovery

**Code evidence:** Wellue starts discovery without checking `Adapter::is_discovering()` and retries
at one-minute intervals after a ten-second search `[aw2-wellue-fix/crates/wellue-collector/src/main.rs:25-39,211-238,613-639]`.
The BP production collector waits for kernel autoconnect rather than scanning or direct connecting
`[aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:253-304]`. The stopgap helper's header
explicitly describes reliance on EMAY discovery and the conflict `[AnxietyWatch2/deploy/stopgap/vitals-bp-connect:8-33]`.

**Live/runtime evidence:** the brief records Wellue repeatedly attempting discovery, `bluetoothctl
show` reporting `Discovering: yes`, BP paired/trusted but disconnected, discovery changing to `no`
immediately after Wellue was stopped, and BP frames 8/9 plus fresh source-6 PostgreSQL rows
without a BP restart or manual `Connect` `[AnxietyWatch2/docs/reviews/bluetooth-rewrite/BRIEF.md:14-20]`.
That is strong evidence of an adapter/discovery arbitration defect, but the underlying journal,
D-Bus trace, and PostgreSQL query transcript are not present in the inspected repository paths:
**UNKNOWN**.

## 7. Existing tests and missing seams

Existing tests cover Wellue discovery/reset predicates `[aw2-wellue-fix/crates/wellue-collector/src/main.rs:681-712]`,
AirSense candidate selection and discovery guards `[aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs:566-605]`,
H10 link/capture seams `[aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs:490-539]`,
EMAY mixed notification framing and live/storage behavior `[AnxietyWatch2/crates/emay-collector/src/main.rs:1647-1836,2251-2565]`,
and BP sample/durability policy `[aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:305-350; aw2-air11/crates/vitals-collectors/src/bp_build.rs:86-173]`.

Missing or insufficient seams are:

1. one process-wide discovery lease with owner, deadline, filter save/restore, and cancellation;
2. controller identity independent of `hciN`, including hotplug/renumbering tests;
3. broker tests proving Wellue absence cannot starve BP autoconnect;
4. connection-slot exhaustion and stale `Operation already in progress` isolation;
5. adapter power-cycle refusal while other source leases are live;
6. BlueZ property-event/`ServicesResolved` failure injection;
7. BP kernel-management integration without any `Device1.Connect` call; and
8. a sanitized replay of the reported Wellue/BP incident with exit-code acceptance.

No test can establish the current physical controller, active units, or live concurrency limits;
those remain runtime gates.
