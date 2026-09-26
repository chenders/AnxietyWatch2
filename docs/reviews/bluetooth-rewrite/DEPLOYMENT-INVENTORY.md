# T-BT-0 — deployment inventory and stable-controller contract

**Status:** evidence freeze complete; physical/runtime qualification not run.
**Date:** 2026-09-26
**Branch:** `task/bluetooth-broker-openspec`

This is a repository/document inventory only. No Pi connection, SSH command, service change, pairing,
adapter change, Wellue enablement, `/boot/firmware` edit, or runtime configuration change was made.
The current installed artifact and controller topology remain unknown.

## 1. Authority and evidence rules

The investigation brief names three divergent repositories:

| Repository | Evidence role | Authority conclusion |
|---|---|---|
| `/Users/chris/Source/AnxietyWatch2` | Primary design/deployment tree; durable EMAY and historical stopgaps | Not sufficient alone: it does not contain the current `vitals-collectors` production crate or current AirSense/Wellue sources. |
| `/Users/chris/Source/aw2-air11` | Production Rust BP/H10/AirSense collectors and related deployment | Contains the production collector paths inspected by the audit, but the installed revision is not established. |
| `/Users/chris/Source/aw2-wellue-fix` | Provisional Wellue collector, probe, deployment, and hardware/G1 history | Wellue remains disabled; this is not evidence that its unit is enabled on the Pi. |

**Authoritative deployed artifact: UNKNOWN.** No checked-in file proves which repository, branch, commit,
sync artifact, or systemd unit set is installed on the current Pi. A runtime qualification must record:
repository/artifact identity, commit or content hash, unit-file hashes, binary hashes, and deployment time
before any cutover decision.

The following are evidence classes:

- **Code/config:** checked-in source or deployment content.
- **Recorded runtime:** a dated repository record of a prior live observation.
- **Provisional budget:** a timeout/retry value visible in code or a report, not a radio acceptance limit.
- **Unknown/gate:** requires an authorized read-only physical/runtime check.

## 2. Current service and BlueZ ownership map

This is a code ownership map, not a claim that all entries are enabled concurrently.

| Source/tool | Checked-in owner/path | BlueZ or management operations | Current disposition |
|---|---|---|---|
| BP production | `aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs`; `vitals-bp.service` | BlueZ session/default adapter, trust/status/GATT subscription; waits for kernel-created link; no userspace `Device1.Connect` | Production path; preserve no-direct-connect and `sync: true`. |
| BP kernel policy | `aw2-air11/deploy/vitals-bp-autoconnect.service`; `99-vitals-bp-autoconnect.rules` | Privileged `btmgmt add-device -a 2 -t 1`; udev currently matches `KERNEL=="hci0"` | Separate policy boundary; controller/index assumption unresolved. |
| BP stopgap | `AnxietyWatch2/deploy/stopgap/vitals-bp-connect` and `.service` | Python D-Bus properties/RSSI and `Device1.Connect` | Legacy competing ownership model; must not be restored as hidden fallback. |
| H10 production | `aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs`; `vitals-h10.service` | Direct connect, service resolution, notify, disconnect/cancel, reconnect | Direct owner pending migration; physical acceptance remains gated. |
| H10 spike | `AnxietyWatch2/crates/hr-spike/src/main.rs` | Discovery, direct connect, GATT notify | Manual investigative owner; must not run against a production adapter without an explicit gate. |
| AirSense production | `aw2-air11/crates/vitals-collectors/src/as11_bluez.rs`; `resmed_air11_collector.rs`; `vitals-air11.service` | Named adapter, direct connect, bounded FD56 discovery fallback, GATT, notify/write | Direct owner; separate therapy lifecycle issue remains separate. |
| AirSense onboarding | `aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs`; onboarding binary | Bounded filtered discovery, connect, pairing link, disconnect | Operator path; local discovery guard is not a global lease. |
| EMAY durable | `AnxietyWatch2/crates/emay-collector/src/main.rs`; `vitals-emay.service` | Default adapter, power-on, known-device connect, GATT write/notify | Direct owner; no production scan in this path. |
| EMAY legacy/spike | `AnxietyWatch2/crates/emay-spike/src/main.rs`; stopgap `emay-live.service` | Discovery, power-on, direct connect, GATT write/notify | Historical/stopgap; must not coexist with durable EMAY. |
| Wellue collector | `aw2-wellue-fix/crates/wellue-collector/src/main.rs`; unit | Unfiltered discovery, direct connect, GATT notify, possible adapter power-cycle | Intentionally disabled; do not re-enable. |
| Wellue probe | `aw2-wellue-fix/crates/wellue-probe/src/main.rs` | Discovery, direct connect, GATT inventory/notify | Manual owner; disabled unless separately authorized. |
| Diagnostic tools | `tools/bp_capture.py`, `tools/emay-investigation/inspect_emay_gatt.py` | Python D-Bus/GATT observation; `busctl` inspection; no raw HCI application source found | Read-only/diagnostic paths; not production ownership. |

### 2.1 Direct BlueZ entry-point inventory

The audit found these application-level operation classes:

- `bluer::Session::new`, `default_adapter`, `adapter(name)`, and device-object access in the Rust
  collectors and probes.
- `Adapter::set_powered(true)` in BP, H10, EMAY, Wellue, probes, and spikes; Wellue also has a
  selected `set_powered(false)`/`true` recovery path.
- `Adapter::discover_devices` in EMAY spike, H10 spike, AirSense fallback/onboarding, Wellue, and
  Wellue probe. Wellue does not check an existing discovery or restore a prior filter.
- `Device::connect` in EMAY, H10, AirSense, Wellue, probes/spikes, and the BP stopgap. Production BP
  is the explicit exception and must not issue it.
- `Device::disconnect` in H10, AirSense, and Wellue cleanup paths.
- GATT service/characteristic resolution and notify/write in all source transports.
- `btmgmt` only in the privileged BP autoconnect unit; no checked-in raw HCI application socket was
  found in the inspected files.

At the inventory freeze no repository-wide lease, broker, adapter-generation, or controller-power owner existed. The current rewrite branch now contains the policy, protocol, Linux owner, and broker daemon front door; this does not assert that the daemon is installed or running on the Pi.

## 3. Controller and version evidence

### 3.1 Historical external-controller evidence

The repository records a prior successful bring-up of a branded Sena UD100-G03 / CSR-family USB
controller:

- USB descriptor recorded as `0a12:0001` Cambridge Silicon Radio Bluetooth Dongle (HCI mode).
- Historical topology had onboard UART `hci0` and the USB controller as `hci1`.
- The historical record identifies a CSR/Sena manufacturer/OUI and `Bus: USB`, but this is not current
  physical evidence.
- A later record says the onboard controller was disabled and the surviving USB controller became
  `hci0`. It explicitly says remaining `hci1` references were then wrong.

A later dated acceptance record reports one powered `hci0`, says the required Sena `hci1` was absent,
and leaves G1 closed. A same-day follow-up records zero controllers and no `/sys/class/bluetooth` in
a different snapshot. These records establish index/topology drift, not the current state.

### 3.2 Version evidence

An A&D capture record reports kernel `6.12.96` and BlueZ `5.66` for that capture. Those values are
historical evidence only. The current kernel, BlueZ version, `btusb`/firmware, rfkill state, and
systemd version are **unknown** until captured together on the appliance.

### 3.3 Stable-controller contract

The future broker must resolve a typed controller identity, not an `hciN` string. The selector must
match one and only one controller using the strongest available combination of:

- controller Bluetooth address, stored/redacted as an approved fingerprint in ordinary logs;
- transport (`USB` versus `UART`);
- stable sysfs USB devpath/physical port;
- VID/PID and serial when genuinely present;
- HCI manufacturer/version evidence; and
- expected appliance role (for example, the verified Sena role).

The current `hciN` name, BlueZ object path, power state, rfkill state, and observed-at timestamp are
runtime fields, not selector identity. The broker must fail closed when:

- no controller matches;
- more than one controller matches;
- required stable fields are absent;
- the controller is present but not usable; or
- the configured role conflicts with observed manufacturer/transport evidence.

Every failure must name the blocker and value, for example `controller unresolved: matches=0,
role=sena-ud100`, rather than merely reporting “Bluetooth unavailable.” A successful resolution must
record both stable identity evidence and the current `hciN` alias.

No current selector value is approved by this inventory. The physical gate must establish it.

## 4. BP kernel-policy and durability inventory

The production BP boundary is:

1. A privileged boot/hotplug path registers kernel accept-list/autoconnect intent with `btmgmt`.
2. The production BP collector waits for the kernel/BlueZ-created connection.
3. It waits for `ServicesResolved`, subscribes to the `0x2A35` indication, journals raw bytes before
   decode, appends with `sync: true`, and requires the durable reply.
4. The broker rewrite must not add a userspace `Device1.Connect` transition.

The current accept-list rule matching `hci0` is an index-bound implementation defect. T-BT-0 does not
change it; a later implementation task must make controller selection and hotplug registration use
the stable selector contract. The old Python stopgap that calls `Device1.Connect` is excluded from the
new ownership model.

## 5. Provisional source timing budgets

These are code-derived starting points for later measurement, not accepted radio budgets:

| Source | Observed repository timeout/retry behavior | Status |
|---|---|---|
| BP | `ServicesResolved` deadline of about 10 seconds; ATT indication/raw-journal/`sync: true` path | Transport deadline candidate; physical delivery and hotplug behavior unknown. |
| H10 | Bounded direct-connect/service-resolution and notification-idle watchdog; the investigation reports a 30-second connect deadline and capped backoff | Candidate source budget; fresh strap acceptance remains blocked. |
| AirSense | Five-second target discovery window with FD56 filter; bounded connect/write/reconnect paths | Candidate transport budget; therapy lifecycle is separate. |
| EMAY | Legacy/spike discovery can hold a one-minute window; durable path uses known-object connect; live keepalive is about 1.2 seconds | Code/protocol values; one-central behavior and D-Bus sandbox require runtime validation. |
| Wellue | Ten-second discovery window and one-minute retry; selected failures can power-cycle adapter | Disabled source; these values do not authorize re-enable or adapter reset. |

The implementation pass must measure advertising windows, connect latency, notification cadence,
keepalive deadlines, queue/fsync latency, and acceptable gaps on the actual adapter before converting
these candidates into scheduler policy.

## 6. Runtime/physical gates still open

The following cannot be closed from repository evidence:

1. Which artifact/revision and units are installed/enabled on the current Pi.
2. Whether the Sena USB device is present, genuine, in the expected port, and currently the selected
   controller.
3. Current controller count, stable address/devpath/VID/PID, current `hciN`, rfkill/power/UP state,
   kernel, BlueZ, driver, and firmware versions.
4. Whether onboard Bluetooth is enabled or disabled; no overlay or `hciuart` change is authorized by
   this task.
5. Whether BP kernel accept-list registration succeeds after cold boot, USB hotplug, controller
   renumbering, and the actual address/IRK state.
6. Actual simultaneous-link capacity and packet-loss/latency behavior.
7. Whether the Wellue/BP incident mechanism is discovery ownership, controller capacity, encryption
   resumption, kernel accept-list state, or a combination.
8. Whether external phone/vendor applications hold source connections during acceptance.

The authorized future read-only qualification should collect `lsusb`, `udevadm info`, `btmgmt info`,
`bluetoothctl show`, `/sys/class/bluetooth`, `rfkill`, `bluetoothd --version`, `uname -a`, service
status/unit hashes, and a bounded `btmon` trace. It must not modify state merely to obtain evidence.

## 7. T-BT-0 conclusion

The deployment inventory is frozen with explicit unknowns. The stable-controller contract is defined:
match a unique verified hardware identity, record `hciN` only as an alias, and fail closed with the
blocking values when resolution is absent or ambiguous. The rewrite branch implements the policy,
protocol, Linux owner, and private-socket daemon against this contract, but no current controller
selector, installed artifact, or runtime cutover is asserted until the physical/runtime gates above
are closed.
