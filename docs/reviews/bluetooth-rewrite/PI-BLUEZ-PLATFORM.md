# Raspberry Pi / BlueZ platform investigation

## Scope and evidence discipline

This is a repository investigation only. No SSH, Pi command, physical action, service change,
Wellue re-enable, source edit, or configuration edit was performed. The current runtime state is
therefore **unknown** unless a dated repository record explicitly reports a prior observation.

Evidence labels used below:

- **Observed repository evidence** — code, deployment manifests, committed history, or dated
  status records.
- **Repository design/proposal** — an intended design or recommendation, not proof that the Pi has
  that hardware or configuration.
- **Physical/runtime gate** — must be checked on the appliance; cannot be established from this
  checkout.

`docs/specs/AMENDMENTS.md` was read in full. The relevant accepted constraints are Amendment 06's
A&D kernel-autoconnect and durability rules, the generated Rust/`bluer` direction, and the crate
boundary that keeps BlueZ in Linux-only daemon/collector code. Amendment 17 is relevant only if a
future phone BLE relay is introduced; it does not change the Pi controller policy. No
`LIBRARIAN.md` was found in the thin-client, AnxietyWatch2, Air11, or Wellue repositories searched,
so there is no separate authoritative Bluetooth inventory to query. No request file was created.

## Executive findings

1. The repository contains strong historical evidence for a **Sena UD100-G03 branded external
   adapter using a CSR8510-family controller**, but it does not prove the current controller.
2. The historical adapter identity moved from `hci1` to `hci0` when the onboard controller was
   disabled. `hciN` is an enumeration index, not hardware identity.
3. Dated Pi records then became contradictory: one 2026-09-05 check saw one `hci0` controller and
   no required `hci1`; a follow-up saw zero controllers and no Sena USB device. The required
   current identity and topology are consequently **unresolved**.
4. The incident described in the brief is consistent with a real global-resource defect:
   Wellue owned adapter discovery while BP depended on kernel auto-connect. Per-collector bounded
   retries do not provide global arbitration.
5. The rewrite should make one Linux supervisor the sole owner of BlueZ/D-Bus discovery,
   controller selection, kernel auto-connect registration, and connection scheduling. Source
   decoders and the `vitalsd` append boundary should remain separate.
6. BP must remain a special kernel-autoconnect source. The rewrite must not restore a generic
   `Device1.Connect` call for the cuff.

## Controller and adapter topology

### Historical topology

The initial Pi baseline in `docs/specs/2026-08-03-pi5-vitals-core-design.md:17-22` recorded a Pi 5
running kernel `6.12.96+rpt-rpi-2712`, with only the onboard UART controller (`hci0`, onboard
address redacted) and no Sena visible in `lsusb`. That document is an early baseline, not current
state.

The committed T-A3 evidence in `AnxietyWatch2` history (`bb93e14`) records the following observed
snapshot:

```text
USB: 0a12:0001 Cambridge Silicon Radio Bluetooth Dongle (HCI mode)
Sena: BD address 00:01:95:C9:AF:00, hci1, Bus: USB, UP RUNNING
       HCI/LMP 4.0, manufacturer Cambridge Silicon Radio (10)
Onboard: hci0, Bus: UART
```

The same task record says the adapter was checked rather than trusted from the product name because
`0a12:0001` CSR dongles are commonly counterfeited. The record used the `00:01:95` OUI and HCI
4.0/CSR report as supporting evidence for the Sena identity. This is repository evidence of a
successful historical validation, not proof that the same USB device is plugged in now.

The T-A4 record (`3518291`) says the onboard controller was subsequently disabled with
`dtoverlay=disable-bt`; `hciuart` was disabled; and the surviving Sena changed from `hci1` to
`hci0` while remaining `Bus: USB`, `UP RUNNING`, with the same historical BD address. The record
explicitly states that adapter indices follow registration order and that all remaining `hci1`
references were wrong. The follow-up documentation commit is `1c63b93`.

### Later dated runtime evidence

The later T-S5 record at `AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md:76-92`
reports:

- exactly one controller, `hci0`, with address `00:01:95:C9:AF:00`;
- the required Sena `hci1` was not present;
- G1 was not green, and no `dtoverlay=disable-bt` action was made.

The address matches the historical Sena address, but this record does not itself include the USB
bus/manufacturer evidence needed to identify the live `hci0` as Sena. The same-day follow-up at
`.../2026-09-05T19-48-52-0700-ts5-follow-up.md:42-51` reports a materially different later
snapshot: zero controllers, no `/sys/class/bluetooth`, no default BlueZ controller, and `lsusb`
showing only Pi USB hubs plus an Espressif `303a:1001` device. This establishes runtime drift or a
changed test state; it does not establish the state now.

**Current topology conclusion:** the current controller count, onboard-overlay state, USB adapter
presence, controller BD address, bus, kernel version, BlueZ version, and adapter power/rfkill state
all require a fresh, authorized physical/runtime check. Do not describe `hci0`, `hci1`, or
`00:01:95:C9:AF:00` as the current controller without that check.

### Multi-controller assumptions found in the repository

The early design assumed the Sena would bind as `hci1` and then disabled onboard Bluetooth. That
assumption was disproven by T-A4 itself. Current Air11 deployment still passes `--adapter hci0`
(`aw2-air11/deploy/vitals-air11.service:14-17`), while BP autoconnect's udev rule matches
`KERNEL=="hci0"` (`aw2-air11/deploy/99-vitals-bp-autoconnect.rules:1-4`). Those are post-T-A4
single-controller assumptions, not stable hardware bindings.

The collectors also call `default_adapter()` in several places:

- BP: `aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:248-256`;
- H10: `.../src/bin/h10_collector.rs:421-423`;
- EMAY: `AnxietyWatch2/crates/emay-collector/src/main.rs:1363-1378`;
- Wellue: `aw2-wellue-fix/crates/wellue-collector/src/main.rs:604-621`.

If a second controller is present, `default_adapter()` can select the wrong controller for a
source. If a USB adapter is removed and reinserted, the index can change again. A source-specific
adapter name in an environment file does not solve identity if the name is `hciN`.

## USB adapter history and kernel policy

### What is supported by repository evidence

- Product history names **Sena UD100-G03** (`README.md`, hardware recommendations, and the T-A3/T-A4
  task records).
- The capture record identifies it as a **CSR8510**-family device and records BlueZ 5.66 and
  kernel 6.12.96 (`docs/research/2026-08-03-ua651ble-ble-capture.md:1-5`).
- The historical USB descriptor was `0a12:0001`, with CSR manufacturer/HCI evidence and a Sena OUI.
- The onboard Pi radio was UART; the external adapter used `btusb` and appeared as USB.
- The adapter was observed RF-killed on its first bring-up attempt and succeeded after retry. T-A3
  records this as error 132 during early `btusb` initialization. T-A4 therefore required retry
  with backoff and loud failure rather than treating `bluetooth.service` active as radio-ready.
- T-A4's order of operations was safety-critical: bring up and reboot-test the external adapter
  first, then disable onboard Bluetooth. The accepted G1 rule remains absolute: never apply
  `dtoverlay=disable-bt` before the external controller is verified UP and usable.

### What is not established

The repository does not prove that the current USB device is genuine Sena, that it still reports
`0a12:0001`, that its firmware/HCI version is unchanged, that it has a stable USB serial number,
or that it is plugged into the same physical port. A generic CSR VID/PID is not sufficient identity.
The current Pi's adapter capacity, slot behavior, RF performance, and USB power stability are also
not established by the historical captures.

### Kernel/BlueZ consequences

1. **Controller indices are volatile.** Resolve a controller by a stable hardware identity and
   retain the runtime `hciN` only as an observed index. Candidate identity inputs are the
   controller BD address, USB sysfs devpath/port, VID/PID, serial if present, HCI manufacturer and
   version, and the expected bus. The rewrite must define what combination is sufficient; do not
   silently fall back to the first adapter.
2. **`bluetooth.service` is not adapter readiness.** The historical T-A3 record found the daemon
   enabled while the second controller was down at boot. Readiness requires a controller object,
   powered state, rfkill state, and a successful controller-level operation.
3. **Kernel management state is separate from BlueZ pairing state.** The BP unit documents that
   the LE auto-connect/white-list registration does not survive reboot or USB hotplug and that
   trust alone did not repopulate it. `btmgmt add-device -a 2 -t 1` is therefore a boot/hotplug
   operation, not a one-time pairing operation (`vitals-bp-autoconnect.service:1-19`).
4. **The current udev trigger is index-bound.** The rule fires only for `hci0`; it can miss the
   controller if enumeration changes. It also supplies no controller identity to the oneshot.
5. **The observed BP kernel behavior is not generic connect behavior.** The capture reports that
   `bluetoothctl connect` issued no LE Create Connection during the cuff's short advertising burst;
   kernel auto-connect was required (`docs/research/2026-08-03-ua651ble-ble-capture.md:97-110`).
6. **A bonded device can still be unusable.** The same capture records an `unexpected SMP command
   0x0b` path on kernel 6.12.96 and requires a named state for `Connected` without
   `ServicesResolved` (`...ua651ble-ble-capture.md:114-139`). Pairing/trust must not be used as
   evidence of usable GATT.
7. **BlueZ is a shared daemon/resource.** Adapter discovery, filters, device objects, connection
   attempts, and GATT operations are visible through one `bluetoothd`/D-Bus control plane. Separate
   processes can have separate `bluer::Session` objects while still contending for the same adapter.

The Raspberry Pi migration document's claims about onboard Wi-Fi/BT coexistence, CSR8510 external
antenna, BlueZ version, and simultaneous-link capacity are design recommendations, not current
acceptance evidence. The Pi was historically on 5 GHz, which reduces one interference source, but
it does not prove BLE continuity.

## Discovery and autoconnect semantics

### Existing source behavior

| Source | Existing behavior | Platform consequence |
|---|---|---|
| A&D UA-651BLE | `bp-collector` explicitly waits for `Connected`; it does not call `Device1.Connect`; it subscribes only after `ServicesResolved` (`bp_collector.rs:248-313`). A root oneshot registers `btmgmt` kernel auto-connect intent. | Correct source-specific policy, but registration and udev ownership are outside the collector and tied to `hci0`. A connected-but-unresolved cuff is explicitly degraded rather than healthy. |
| Polar H10 | Direct `Device1.Connect`, waits for services, subscribes to 0x2A37, checks `Connected` and notification-idle watchdog; cancels a stuck connection with `Device1.Disconnect` (`h10_collector.rs:241-321`, `401-445`). | Good local deadlines, but it independently opens the default adapter and can race other policy. |
| AirSense 11 | Uses a configured adapter name, tries direct connect, then performs a five-second FD56-filtered LE discovery fallback. It rejects an already-discovering adapter, saves/restores the filter, and validates address/service/RSSI/name (`as11_bluez.rs:381-419`, `688-760`). | Bounded and read-only, but the discovery lock is only a local check, not a host-wide lease. A pre-existing scan causes rejection rather than arbitration. |
| EMAY SleepO2 | Uses `default_adapter`, a known address, and direct `Device1.Connect`; it does not recreate an absent/unmaterialized object through discovery (`emay-collector/main.rs:255-267`, `1363-1383`). | Address-only retry cannot succeed when BlueZ has no usable device object. The recovery record confirms a bounded discovery recreated the object. EMAY accepts one central, so the old live collector must not run beside the durable one. |
| Wellue/KS-60FWB | Uses `default_adapter`, checks a known connected object, otherwise starts unfiltered discovery for 10 seconds, retries after one minute, and may power-cycle the adapter for narrow stale-link errors (`wellue-collector/main.rs:211-240`, `457-500`, `604-651`). | Local cooldown reduces but does not eliminate global scan contention. This is the collector named in the brief's discovery storm and is currently intentionally disabled. |

### Incident interpretation

The brief's incident is strong repository evidence of adapter arbitration failure: Wellue repeatedly
owned discovery, `bluetoothctl show` reported `Discovering: yes`, BP was paired/trusted but not
connected and was waiting for kernel autoconnect, and stopping Wellue changed discovery to `no`
without restarting BP. BP then received durable frames. This is not proof that every BP failure is
caused by discovery, but it is evidence that discovery ownership can block the higher-priority
store-and-forward path.

The source-local Wellue timeout/cooldown and BP startup ordering are therefore insufficient as a
rewrite boundary. The system needs one admission decision for every scan and every controller
power/management operation.

### Required semantics for the rewrite

- A scan is a **singleton per controller**. No source collector can call `StartDiscovery` directly.
- Scans are acquired through a typed, expiring lease with owner, source, purpose, duration, filter,
  priority, and cancellation reason. BP autoconnect protection has higher priority than optional
  discovery.
- The supervisor must record and restore the prior filter/discovery state, and must fail closed if
  it cannot establish ownership or restore state. It must never leave an indefinite scan behind.
- An address in configuration is not enough to infer a present BlueZ `Device1` object. The state
  machine must distinguish absent, advertising, materialized, connecting, connected,
  services-resolved, subscribed, streaming, bonded-but-unusable, and adapter-unavailable.
- Reconnect policy must be source-specific: BP waits for kernel intent; H10 and other connectable
  peripherals use bounded connect attempts; optional sources use bounded advertisement-triggered
  discovery, not an unbounded wall-clock scan loop.
- `Connected=true` is not a physiological liveness signal. `ServicesResolved`, subscription
  success, and source-appropriate notification/data evidence remain separate gates.
- A scanner must not be used as proof that an empty advertisement stream is healthy. The repository
  proposal in `aw2-air11/docs/design/HOMEASSISTANT-ARCHITECTURE.md:182-240` recommends a wedge detector,
  but its slot counts and timing constants remain proposals requiring Pi measurement.

## Failure modes and safe operational gates

### Failure matrix

| Failure | Observable symptom | Required interpretation/gate |
|---|---|---|
| USB dongle absent, wrong device, or bad USB power | `lsusb` lacks expected device; no controller or unexpected VID/PID | Adapter identity is unknown; do not disable onboard Bluetooth or call G1 open. |
| Controller enumerates under a new index | `hci0`/`hci1` changes across boot/hotplug | Re-resolve stable identity; never route by index. Re-register BP kernel intent on that controller. |
| `bluetoothd` active but no usable controller | systemd active, `btmgmt info` empty, no default adapter | Infrastructure is down; source services must report unavailable, not restart-loop as healthy. |
| RF-kill or delayed `btusb` bind | bring-up returns error 132 or transient missing object | Retry with bounded backoff, report the exact blocker; do not declare ready after one failed `set_powered`. |
| Shared discovery contention | `Discovering=yes`, one collector's scan blocks another or `StartDiscovery` returns busy | Supervisor denies/preempts by policy; never let optional Wellue scan silently starve BP. |
| Device object absent | `Device1` lookup/connect fails with “not present or removed” | Distinguish off/out-of-range/not advertising/other central from pairing fault; do not claim sensor failure from address alone. |
| Stale connect operation | connect timeout followed by “operation already in progress” | Wait for actual disconnect; bounded `Device1.Disconnect` cancellation before retry. |
| Bonded but unresolved | `Connected=true`, `ServicesResolved=false` past deadline; kernel SMP evidence | Named degraded state; do not score as quiet/healthy. Repair pairing only through an authorized gate. |
| Adapter hotplug | controller returns but BP produces no frames | Re-register kernel auto-connect; verify controller and white-list state before accepting BP readiness. |
| GATT stream is stale | BlueZ still says connected but no H10/Wellue/EMAY notifications | Source watchdog must classify stale separately from process heartbeat and reconnect only within policy. |
| Competing EMAY centrals | device disappears or does not advertise to Pi | Keep `emay-live.service` disabled; one durable owner only. |
| Automatic adapter reset harms other sources | one collector power-cycles the shared adapter | No source collector may power-cycle a shared controller; supervisor must gate recovery and record impact. |

### Non-negotiable gates

1. **G1 adapter identity:** before any `disable-bt` overlay or onboard-radio removal, verify on the
   physical Pi that the intended external adapter is present in USB topology, reports the expected
   controller/manufacturer evidence, is powered and UP/RUNNING after cold boot, and survives a
   bounded test scan. This gate is closed by the dated T-S5 records.
2. **Controller topology:** record all controllers, bus, sysfs path, address, VID/PID/serial, kernel,
   BlueZ, rfkill, and overlay state. If any field is unknown, retain the existing topology and
   report unknown; do not infer from `hciN`.
3. **BP safety:** never reintroduce `Device1.Connect` for A&D. Register `btmgmt` kernel intent on
   the resolved controller, wait for `Connected` and `ServicesResolved`, journal raw indications,
   and await the `sync:true` `vitalsd` reply as specified by Amendment 06.
4. **Discovery safety:** every scan must be bounded, authorized where operator-directed, filtered
   to its target, and owned by the single supervisor. Do not re-enable Wellue during this review.
5. **No concurrent topology owners:** do not run legacy collectors and the new supervisor against
   the same adapter. Rollback must stop one owner completely before starting the other.
6. **Readiness:** `After=bluetooth.service`/`bluetooth.target` is only ordering. Source units must
   depend on a supervisor readiness contract that includes the selected controller and policy
   registration, not merely daemon activation.

## Proposed rewrite architecture and boundaries

This is a proposal for the rewrite, not an implementation decision.

### 1. One BlueZ platform supervisor

Create a Linux-only `vitals-ble-supervisor` (name provisional) with one long-lived D-Bus session and
one controller inventory. It owns:

- stable controller resolution and readiness;
- `org.bluez.Adapter1` power/discovery/filter state;
- `org.bluez.Device1` connection lifecycle and `ServicesResolved` gates;
- GATT characteristic discovery, notification subscriptions, and bounded writes;
- one scan coordinator per controller, exposed through a host-wide lease service;
- the privileged kernel-management seam for BP auto-connect registration;
- source priority, connection-slot admission, cancellation, and backoff;
- controller hotplug/rfkill events and named health state.

The supervisor may use `bluer` for D-Bus, as adopted by the amendment index, but the kernel
management operation must be a single audited privileged seam. Whether that seam uses a carefully
scoped `btmgmt` helper or the Linux Bluetooth management socket directly is an implementation choice
that requires Pi validation; it must not be duplicated in udev and collector shells.

### 2. Abstract source transports

Source collectors should receive typed link events and bounded notification streams from the
supervisor over a private Unix IPC protocol, or run as supervised tasks inside the supervisor if
that produces a clearer single-owner boundary. They should not open independent `bluer::Session`s,
call `default_adapter()`, start discovery, or power-cycle an adapter.

The source layer retains:

- protocol-specific GATT UUIDs and read-only/application protocol rules;
- pure decoder and sample-building code;
- source-specific sequence/session/durability policy;
- raw journals for sources whose accepted rules require them;
- append calls to `vitalsd`.

`vitalsd` remains the single append/spool boundary. `vitals-core` remains free of BlueZ/systemd;
`vitals-decode` remains pure `no_std`; no BlueZ dependency should move into either crate.

### 3. Stable identity model

Introduce a typed `ControllerId`/inventory record containing at least:

- controller BD address and runtime index as separate fields;
- USB-vs-UART bus and stable sysfs devpath/physical port;
- VID/PID, serial when actually present, HCI manufacturer/version;
- rfkill/powered/UP state and BlueZ object path;
- observed-at timestamp and evidence quality.

Configuration selects a validated hardware identity, never an `hciN` string. If the identity cannot
be resolved uniquely, the supervisor fails closed with the fields that conflicted. A human-readable
name such as `sena-ud100` is metadata, not the selector.

### 4. Source policy table

The supervisor should load explicit policies rather than infer behavior from a generic connect loop:

- **A&D:** kernel auto-connect registration, no userspace `Connect`, highest protection for its
  short store-and-forward advertising window, `ServicesResolved` and indication subscription gate.
- **H10:** direct connect with a bounded cancellation path, notification-idle watchdog, and source
  session minting on reconnect.
- **AirSense:** read-only direct link plus an explicit operator onboarding/discovery lease; production
  collection must not initiate an unbounded scan.
- **EMAY:** one durable central, known-object/discovery policy owned by the supervisor, stored-session
  transfer before anonymous live status, and no legacy parallel service.
- **Wellue:** optional/preemptible bounded discovery only after higher-priority policies are safe;
  currently disabled remains the safe posture.

### 5. Deployment boundary

Replace per-collector BlueZ ownership with one supervisor unit and a readiness socket/status
contract. Source units should require that contract and receive a stable `ControllerId`/link handle.
A udev add event may notify the supervisor of controller hotplug, but must not match `hci0` and start
a source-specific service. BP registration must be performed for the controller that the supervisor
resolved, with a named result and retry policy.

Keep the current systemd hardening and memory priorities, but move `AF_BLUETOOTH`/management
capability to the smallest supervisor surface. Collectors should retain only the IPC and vitalsd
permissions they need. No service should treat `Restart=always` as proof that a radio is healthy.

## Staged migration and rollback

### Stage 0 — evidence freeze and topology qualification

- Preserve the current deployment artifacts and journals.
- On an authorized Pi run, capture controller inventory, USB/sysfs path, overlay/hciuart state,
  rfkill, BlueZ/kernel versions, pairing database presence, and current service ownership.
- Close G1 only if the external adapter identity and cold-boot readiness are both proven. No overlay,
  service, or adapter change belongs in this investigation.

**Rollback:** none; this stage is read-only.

### Stage 1 — policy/state-machine seam

- Implement and test portable policy types for controller identity, scan leases, source states,
  priorities, deadlines, and named failure classes.
- Add Linux integration seams around the actual BlueZ APIs without pretending a mock proves Pi
  behavior. Keep real BlueZ acceptance on the Pi.
- Define supervisor status and readiness output before moving a collector.

**Rollback:** no runtime cutover; retain existing units.

### Stage 2 — supervisor observe-only / non-invasive inventory

- Run the supervisor in an inventory mode that does not scan, pair, connect, power-cycle, or alter
  kernel auto-connect state.
- Compare its controller identity with the recorded physical gate and prove index churn handling
  across an authorized reboot/hotplug test.

**Rollback:** stop the observe-only process; no source changes.

### Stage 3 — A&D canary

- Move BP kernel-registration and link observation under the supervisor first because BP has the
  strongest transport/durability constraint.
- Preserve raw journals, `sync:true` append, frame-count evidence, and the no-`Device1.Connect`
  assertion.
- Test boot, USB re-enumeration, short advertising bursts, unresolved bonded state, and one real
  durable reading before advancing.

**Rollback:** stop the new supervisor and confirm it no longer owns the controller; restore the
known-good BP unit/udev topology and re-register kernel intent on the validated controller. Do not
run both owners concurrently.

### Stage 4 — H10 and AirSense

- Move continuous H10 link/watchdog handling, then AirSense read-only transport and explicitly
  leased onboarding.
- Demonstrate that an AirSense or H10 failure cannot start or retain a scan that blocks BP.
- Keep application pairing and therapy-control boundaries unchanged.

**Rollback:** source-by-source, with the supervisor disabled for that source and the prior service
restored only after ownership is released.

### Stage 5 — EMAY and Wellue last

- Move EMAY only with one-central enforcement and stored-session/live-state separation.
- Keep Wellue disabled through the rewrite review. Re-enable only after the global discovery lease,
  preemption, cooldown, and BP protection are proven in a bounded acceptance run.

**Rollback:** disable the optional source and preserve its journal/status; do not reset the shared
adapter merely to recover an optional source.

### Stage 6 — retire direct BlueZ paths and evaluate multi-controller mode

- Remove direct `bluer` sessions from collectors only after equivalent supervisor acceptance exists.
- Retire the `hci0` udev rule and `--adapter hci0` deployment assumption.
- Treat intentional multi-controller support as a separate measured feature. Do not add a second
  adapter to production merely because the architecture can represent one.

**Rollback:** retain versioned old/new unit manifests and an explicit one-owner switch. A rollback
must preserve spool/raw journals and must never require destructive database changes.

## Observability contract

The supervisor should publish both machine-readable status and journal events. Every event should
include source/controller/operation/phase, a monotonic sequence or correlation ID, elapsed time,
result, and the value that blocked an action. Sensor addresses and credentials must be redacted from
ordinary logs.

Minimum fields and metrics:

- controller inventory: stable identity, runtime `hciN`, bus, sysfs path, VID/PID/serial presence,
  BD address fingerprint, HCI/manufacturer/version, BlueZ/kernel version, powered, rfkill, UP/RUNNING;
- lifecycle: controller add/remove, power/rfkill transitions, daemon availability, readiness time,
  hotplug re-registration result;
- discovery: lease owner/source/purpose, requested filter, start/stop, duration, pre-existing state,
  restoration result, denied/conflicting requests, scan watchdog expiry;
- links: source state transitions, connect/disconnect latency, `ServicesResolved` latency, GATT
  discovery/subscription result, notification age, stale-stream detection, retry class/backoff;
- capacity: active links and observed admission failures per controller; do not present unmeasured
  slot limits as facts;
- BP: auto-connect registration controller/result, kernel-to-Connected latency, unresolved deadline,
  indication count, raw-journal fsync result, durable append acknowledgement, lost/limbo frame count;
- health: distinct `unknown`, `degraded`, `unavailable`, and measured-good states. A process
  heartbeat must never substitute for source data or controller readiness;
- incident correlation: supervisor operation ID linked to systemd unit, kernel Bluetooth log, and
  `btmon` evidence for an authorized diagnostic run.

The admin/status surface should show controller identity and ownership, not just “Bluetooth active”
or a bare `hci0` label.

## Unresolved physical and runtime validation

These questions remain open and must be answered on the Pi, not inferred from repository prose:

1. Is the Sena UD100-G03 physically present now? If so, is it the same CSR controller, what are its
   USB port/devpath, VID/PID, serial behavior, BD address, HCI/LMP version, and manufacturer?
2. Is the onboard UART controller enabled or disabled now? Is `/boot/firmware` carrying the
   `disable-bt` overlay, and is `hciuart` enabled? This requires an authorized physical/runtime
   check; no change should be made during validation.
3. How many controllers appear after cold boot, after USB hotplug, and after a BlueZ restart? Does
   the external device retain its stable identity while its `hciN` index changes?
4. What are the current kernel, BlueZ, `btusb`, firmware, rfkill, and systemd versions? Are they
   still the versions in the 2026-08-03 capture?
5. Does the selected controller reliably remain powered and UP/RUNNING at boot, including the
   historical RF-kill(132) early-bind window?
6. Does BP kernel auto-connect registration target the intended controller after reboot and hotplug,
   and does it survive the exact current address type/IRK state?
7. Are the current A&D, H10, AirSense, EMAY, and Wellue pairing/bonding/address-type states known?
   Which devices are single-central, store-and-forward, or stream sources at runtime?
8. What is the measured link/slot behavior when H10, AirSense, EMAY, and BP policies coexist? The
   repository's slot numbers and coexistence recommendations are not acceptance evidence.
9. Can bounded supervisor-owned discovery materialize EMAY/AirSense/Wellue objects without blocking
   BP auto-connect, and does filter restoration work after success, timeout, cancellation, and
   supervisor crash?
10. What is the safe recovery boundary for this appliance: D-Bus power cycle, management-socket
    power cycle, USB port reset, or operator-only intervention? No automatic USB reset should be
    enabled based only on prior-art prose.

## Exact files, APIs, and tests

### Existing files to inspect before implementation

**Evidence and policy**

- `docs/reviews/bluetooth-rewrite/BRIEF.md`
- `docs/specs/AMENDMENTS.md`, especially Amendment 06 and the tool/crate-boundary summary
- `AnxietyWatch2/docs/research/2026-08-03-ua651ble-ble-capture.md`
- `AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md`
- `AnxietyWatch2/docs/status-updates/2026-09-05T19-48-52-0700-ts5-follow-up.md`
- `AnxietyWatch2/docs/status-updates/2026-09-05T08-09-24Z-emay-recovery.md`
- `AnxietyWatch2/docs/specs/2026-08-03-pi5-vitals-core-design.md`
- `aw2-air11/docs/design/HOMEASSISTANT-ARCHITECTURE.md` (proposal/prior art, not acceptance)

**BlueZ/source transports**

- `aw2-air11/crates/vitals-collectors/src/as11_bluez.rs`
- `aw2-air11/crates/vitals-collectors/src/bin/resmed_air11_collector.rs`
- `aw2-air11/crates/vitals-collectors/src/bin/resmed_air11_onboard.rs`
- `aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs`
- `aw2-air11/crates/vitals-collectors/src/bp_build.rs`
- `aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs`
- `aw2-air11/crates/vitals-collectors/src/h10_reconnect.rs`
- `AnxietyWatch2/crates/emay-collector/src/main.rs`
- `aw2-wellue-fix/crates/wellue-collector/src/main.rs`

**Deployment and host integration**

- `aw2-air11/deploy/vitals-air11.service`
- `aw2-air11/deploy/vitals-h10.service`
- `aw2-air11/deploy/vitals-bp.service`
- `aw2-air11/deploy/vitals-bp-autoconnect.service`
- `aw2-air11/deploy/99-vitals-bp-autoconnect.rules`
- `aw2-wellue-fix/deploy/wellue-collector.service`
- `AnxietyWatch2/deploy/vitals-emay.service`
- `AnxietyWatch2/deploy/stopgap/emay-live.service` and `deploy/stopgap/README.md` (historical,
  must remain disabled)
- `AnxietyWatch2/scripts/sync-to-pi.sh` and `aw2-air11/scripts/sync-to-pi.sh`
- `aw2-air11/scripts/air11-pi-acceptance.sh`

### BlueZ/Linux APIs to isolate and audit

- D-Bus `org.bluez.Adapter1`: `Powered`, `Discovering`, `StartDiscovery`, `StopDiscovery`, and
  `SetDiscoveryFilter`.
- D-Bus `org.bluez.Device1`: `Address`, `AddressType`, `Paired`, `Trusted`, `Connected`,
  `ServicesResolved`, `Connect`, and `Disconnect`.
- D-Bus `org.bluez.GattCharacteristic1`: `StartNotify`, `StopNotify`, `WriteValue`, and the
  negotiated MTU/flags exposed by the Rust binding.
- Rust `bluer`: `Session::new`, `default_adapter`, `adapter(name)`, `Adapter::device`,
  `discover_devices`, `is_discovering`, `discovery_filter`, `set_discovery_filter`,
  `set_powered`, `Device::connect`, `disconnect`, `is_connected`,
  `is_services_resolved`, `Characteristic::notify`, and `Characteristic::write`.
- Linux Bluetooth management: the `btmgmt add-device -a 2 -t 1` operation currently used for BP,
  plus explicit controller selection and result reporting. Validate whether the final implementation
  uses `btmgmt` through a constrained helper or the management socket; do not leave this decision
  implicit.
- udev/systemd: Bluetooth controller add/remove events, stable sysfs/devpath matching, supervisor
  readiness, and capability boundaries. Do not use `KERNEL=="hci0"` as hardware identity.
- Runtime evidence commands for a future authorized Pi gate: `lsusb`, `udevadm info`,
  `btmgmt info`, `bluetoothctl show`, `bluetoothctl info`, `rfkill`, `/sys/class/bluetooth`,
  `bluetoothd --version`, `uname -a`, and a bounded `btmon` trace. These were not run here.

### Tests and acceptance cases

Existing test seams worth preserving:

- `as11_bluez.rs` tests around FIG framing and event-subscription coverage (`:970+`);
- `h10_reconnect.rs` tests for disconnected, notification-idle, cadence watchdog, and capped
  backoff;
- Wellue tests for ten-second discovery/one-minute retry and narrow adapter-reset policy
  (`wellue-collector/src/main.rs:688+`);
- EMAY backend/storage/status tests (`emay-collector/src/main.rs:1440+`);
- portable BP sample/durability tests in `bp_collector.rs`/`bp_build.rs`.

The rewrite must add policy tests for:

1. stable controller resolution when `hciN` changes;
2. duplicate/ambiguous controller identity and fail-closed diagnostics;
3. no controller, rfkill, delayed `btusb`, and daemon-active/no-adapter states;
4. one discovery lease per controller, priority/preemption, timeout, cancellation, crash cleanup,
   and filter restoration;
5. Wellue discovery denied while BP protection is active;
6. BP auto-connect registration on boot and controller-add events, with no userspace `Connect`;
7. bonded-but-unresolved `ServicesResolved` timeout;
8. stale `Connected=true` notification streams and connect cancellation before retry;
9. absent/unmaterialized known devices and the explicit discovery path for AirSense/EMAY/Wellue;
10. no concurrent legacy/new BlueZ owner during migration;
11. supervisor restart with source sessions and raw/spool durability preserved;
12. redacted, named observability for every blocked operation.

Host tests can prove policy and protocol seams. They cannot prove BlueZ, systemd, kernel, USB, or
radio behavior. The final acceptance set must run on the real Pi and include cold boot, adapter
hotplug, controller-index change, bounded discovery contention, BP short-advertisement delivery,
H10 notification continuity, AirSense read-only stream, and EMAY single-central behavior. No
acceptance claim should be made until those runtime exit codes and evidence artifacts exist.
