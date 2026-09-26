# Public prior art: Linux BlueZ per-adapter broker and lease scheduler

**Research mode:** read-only public-source/document review. No repository runtime, source,
configuration, systemd, udev, or Pi state was changed.

**Research cutoff:** the public revisions named below were captured during this review. URLs are
pinned to commits where available; moving-branch links are called out explicitly. The report
separates an exact reusable implementation from an adjacent design pattern.

## Executive recommendation

**Build a small project-specific Rust broker around the existing BlueZ daemon. Do not replace
BlueZ, and do not adopt Home Assistant or Bleak as the runtime scheduler.** Reuse the following
ideas:

1. BlueZ D-Bus as the single kernel-facing daemon interface, with one broker-owned discovery
   operation per adapter.
2. Home Assistant `habluetooth`'s per-device active-window scheduler, ownership handoff, connecting
   fallback, watchdog, and explicit scan-mode model. Port the algorithm and its tests; do not copy
   its Python process architecture.
3. `bleak-retry-connector`'s transient-error taxonomy, out-of-slot backoff, stale-connection cleanup,
   and explicit diagnostic advice. Treat its slot count as a configured hint, not truth.
4. NetworkManager's D-Bus object-cache/name-owner/property-event lifecycle patterns, especially
   initial `GetManagedObjects`, `InterfacesAdded/Removed`, `PropertiesChanged`, cancellation, and
   cleanup after BlueZ restarts.
5. BlueZ management-socket operations only where the BP invariant requires them, behind an explicit
   broker policy. The BP collector still must not call `Device1.Connect`.

No public project reviewed here is an exact, production-ready implementation of a **cross-process,
per-adapter BLE discovery/connect/GATT lease scheduler** with priority, fairness, cancellation,
stable controller identity, and the project's BP durability rule. That missing combination is the
reason to build a narrow broker rather than select one dependency as the solution.

## Comparison table

| Project/source | Revision reviewed | What it actually coordinates | Exact reusable implementation? | Main limitation for this rewrite | License/integration |
|---|---|---|---|---|---|
| BlueZ daemon D-Bus API/source | BlueZ Git commit `8b4a4176063831476bc9244d20f43b552e4b5554` | Kernel/controller access, D-Bus adapter/device/GATT objects, discovery sessions and merged filters | **Underlying substrate only** | No application-level fair lease scheduler; D-Bus clients can still compete through separate processes | BlueZ daemon source is GPL-licensed; use its public D-Bus API rather than copy/link daemon code |
| Home Assistant `habluetooth` | `7.1.2`, commit `efc10fb2f55df617780db255667e1af96f6b8277` | In-process HA scanner registry, active scan windows, scanner ownership, retries/watchdogs, configured slot accounting | **Closest scheduling patterns; not a broker** | Python/asyncio library; locks are process-local; no external lease protocol; experimental mgmt path is not production-wired | Apache-2.0; copying requires notices/license compliance; Rust port of behavior is safer |
| `bleak-retry-connector` | `4.7.1`, commit `0bdeaf11d91aa954ee0973ba06c9aba52ddb9971` | Retry/backoff, stale disconnect waits, D-Bus device lookup, optional BlueZ path-based slot manager | **Connection reliability helper only** | No discovery arbitration, no cross-process ownership, slot count is supplied/configured and often inferred | MIT; suitable as a conceptual reference or separate Python dependency, not a Rust runtime dependency |
| Bleak BlueZ backend | `develop` source link; exact commit not captured in this review | Per-client BlueZ D-Bus scanner/client abstraction | **API/backend reference only** | A Bleak scanner is not a global adapter broker; independent clients can issue independent operations | MIT; do not treat the moving branch URL as a pinned dependency |
| NetworkManager BlueZ manager | NetworkManager commit `5e638b3d1281c0dbf089ae70c4e5871887f95307` | Bluetooth PAN/DUN network activation, D-Bus object cache, property/event lifecycle, cancellation | **Adjacent lifecycle pattern** | It does not schedule BLE discovery/GATT leases and its connection model is network-profile-specific | GPL-2.0-or-later for the project components; use behavior as a pattern, not source without compliance |
| ConnMan Bluetooth plugin | ConnMan 1.45 commit `e410e40bc61e5573c2a592bfaebc0fb283ec0fab` | BlueZ D-Bus PAN/network proxies, adapter/device property watching, network connect/disconnect | **Adjacent lifecycle pattern** | PAN only; no BLE scan, GATT, slot, or lease scheduler | GPL-2.0-or-later project; pattern-only unless license review approves copying |
| BlueZ management interface / `btmgmt` | Same BlueZ revision above | Controller management commands/events, including discovery and accept-list operations | **Privileged-operation substrate** | A management socket is not a fair application lease service; it does not stop D-Bus clients from competing | Use command/API boundary; avoid copying GPL implementation into the broker |

## 1. BlueZ: what the daemon already arbitrates

### 1.1 Adapter and discovery semantics

The authoritative public adapter contract is:

- [BlueZ `org.bluez.Adapter` API, commit `8b4a4176`](https://github.com/bluez/bluez/blob/8b4a4176063831476bc9244d20f43b552e4b5554/doc/org.bluez.Adapter.rst)
- [BlueZ adapter implementation, `src/adapter.c`, commit `8b4a4176`](https://github.com/bluez/bluez/blob/8b4a4176063831476bc9244d20f43b552e4b5554/src/adapter.c)
- [BlueZ management protocol, commit `8b4a4176`](https://github.com/bluez/bluez/blob/8b4a4176063831476bc9244d20f43b552e4b5554/doc/mgmt-protocol.rst)

The relevant public contract is important for the proposed broker:

- `org.bluez.Adapter1.StartDiscovery()` and `StopDiscovery()` operate on the adapter's discovery
  activity. `Discovering` is an adapter-level property, not a private property of one collector.
- `SetDiscoveryFilter()` accepts per-client discovery preferences, but BlueZ combines active client
  filters for the adapter. It is not an isolation boundary: a second client can broaden the
  effective scan while another client believes it installed a narrow filter.
- BlueZ maintains discovery users/sessions internally. That prevents every D-Bus caller from
  independently owning the controller, but it is not a priority queue, starvation-free scheduler,
  deadline/lease API, or policy boundary between application processes.
- The object-manager surface (`GetManagedObjects`, `InterfacesAdded`, `InterfacesRemoved`) and
  `org.freedesktop.DBus.Properties.PropertiesChanged` are the correct way for a broker to rebuild
  state after device appearance, disappearance, adapter removal, and BlueZ restart.
- The management protocol is lower-level than D-Bus. It is appropriate for controller-wide
  operations such as accept-list setup or an explicitly broker-owned scan, but opening a management
  client does not automatically prevent another process from issuing D-Bus operations.

**Implication:** BlueZ is already a daemon-level multiplexer, but it is not the application's
lease scheduler. A broker must ensure collectors never call `StartDiscovery`, `StopDiscovery`,
`SetDiscoveryFilter`, adapter power methods, or connect/disconnect methods directly.

### 1.2 Device and GATT semantics

Relevant upstream contracts:

- [BlueZ `org.bluez.Device` API, commit `8b4a4176`](https://github.com/bluez/bluez/blob/8b4a4176063831476bc9244d20f43b552e4b5554/doc/org.bluez.Device.rst)
- [BlueZ GATT characteristic API, commit `8b4a4176`](https://github.com/bluez/bluez/blob/8b4a4176063831476bc9244d20f43b552e4b5554/doc/org.bluez.GattCharacteristic.rst)
- [BlueZ agent manager API, commit `8b4a4176`](https://github.com/bluez/bluez/blob/8b4a4176063831476bc9244d20f43b552e4b5554/doc/org.bluez.AgentManager.rst)

BlueZ exposes device `Connect`, `Disconnect`, pairing/trust state, services-resolved state, and
GATT characteristic operations over D-Bus. It does not expose an application-level exclusive
connection lease. In particular:

- BlueZ can reject or serialize operations internally, but callers still need an owner-side policy
  for two collectors requesting the same device, cancellation of a stale connect, and deciding who
  may disconnect a link owned by another collector.
- A characteristic notification/indication subscription is a protocol operation with device- and
  controller-specific consequences. BlueZ does not know that BP's ATT confirmation must be followed
  by a `sync: true` application append before the sample is considered handled.
- Agent registration has an ownership/lifecycle problem of its own. A broker should not let a
  short-lived collector register a global pairing agent and then disappear without cleanup.
- BlueZ's `hciN` object path/name is an operating-system enumeration, not a durable physical
  controller identity. The broker needs a resolver based on controller address plus validated
  sysfs/USB identity; the public BlueZ API alone does not provide the project's desired
  hotplug-stable identity policy.

### 1.3 Exact reusable part

Reuse BlueZ's APIs and event model; do not reimplement BlueZ. The broker can use the existing Rust
`bluer` boundary already selected by this project, or a small D-Bus binding, while owning one
session and one state machine per resolved controller.

Do **not** reuse BlueZ source as an embedded scheduler. That would duplicate the daemon's controller
state and create a second authority rather than solve contention.

## 2. Home Assistant and `habluetooth`

### 2.1 Revision and public files

The reviewed public release is `habluetooth` 7.1.2, declared in
[`pyproject.toml`, commit `efc10fb2`](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/pyproject.toml).
Its license is Apache-2.0. Relevant pinned files are:

- [Central manager](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/manager.py)
- [Active-window scheduler](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/auto_scheduler.py)
- [Bleak-backed scanner](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/scanner_bleak.py)
- [Experimental management-socket scanner](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/scanner_mgmt.py)
- [Experimental management-socket client](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/client_mgmt.py)

### 2.2 What is genuinely reusable

`AutoScanScheduler` is the strongest prior art found. Its documented algorithm:

- registers active-scan requests by device address, duration, and interval;
- assigns each device to the scanner currently considered its owner;
- fires a per-device active window on exactly one scanner;
- moves the request when advertisement ownership changes between scanners;
- advances the due time before awaiting the scan so an ownership flip cannot double-fire;
- routes a due request to a non-connecting fallback scanner when the owner is busy connecting;
- defers with a bounded retry when there is no usable fallback; and
- enforces at most one outstanding window per scanner worker.

These invariants are documented at [the pinned scheduler source, lines 1-172](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/auto_scheduler.py#L1-L172).

The scanner implementation also has useful local patterns:

- a start/stop lock around each scanner;
- a watchdog for a scanner that went quiet or failed to stop/start;
- restoration of discovery filters after a successful start;
- passive/active/auto scan modes; and
- explicit diagnostics for adapter allocation and mode.

The manager's conceptual model—advertisement source, current device owner, and scheduled scan
window—is directly applicable to a broker that needs to reconcile discovery requests without
letting each collector own BlueZ discovery.

### 2.3 What it does not provide

`habluetooth` is a library inside Home Assistant, not a system service. Its locks and tasks are
Python process-local. It assumes the Home Assistant process is the principal scanner registry; it
does not authenticate or arbitrate arbitrary external processes that can independently call BlueZ.
The Bleak-backed scanner's own documentation explicitly treats multiple scanners as useful when
there are multiple physically separate adapters, not as a multi-client solution for one adapter
([`scanner_bleak.py` lines 220-230](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/scanner_bleak.py#L220-L230)).

It also does not give this project:

- a cross-process discovery lease or priority/fairness policy;
- an exclusive per-device GATT connection lease;
- a stable controller identity independent of `hciN`;
- an adapter power/recovery lease that protects unrelated links;
- BP kernel-autoconnect ownership and the project's durable ACK boundary; or
- a Rust implementation compatible with this repository's crate boundaries.

### 2.4 Experimental mgmt path: close, but not a finished broker

The same release includes an explicitly experimental `HaScannerMgmt`:

- It drives discovery through the BlueZ management socket and uses advertisement monitors for
  passive mode.
- It routes GATT over raw L2CAP ATT rather than the normal BlueZ D-Bus GATT objects.
- It tracks live peer addresses itself because the usual D-Bus-path slot manager cannot see raw
  L2CAP connections.
- It pauses scanning during a connection attempt through a scanner `connecting()` context.
- It falls back to the Bleak-backed scanner if mgmt is unavailable.

Those statements and the production caveat are in the pinned module docstring
([`scanner_mgmt.py` lines 1-27](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/scanner_mgmt.py#L1-L27)); the adapter-index, live-connection, and start/stop logic are at
[lines 73-213](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/scanner_mgmt.py#L73-L213).

This is an excellent **design reference for a scan-pause/connect critical section**, but not an
exact implementation to deploy here:

- The module says Home Assistant does not select it in production yet.
- It parses an `hciN` suffix into an adapter index and requires an adapter BD address
  ([lines 76-107](https://github.com/Bluetooth-Devices/habluetooth/blob/efc10fb2f55df617780db255667e1af96f6b8277/src/habluetooth/scanner_mgmt.py#L76-L107)). That is useful for a pinned runtime adapter but does not solve this project's stable identity requirement.
- Its slot gate is a local live-peer set against a configured limit, not a controller reservation
  protocol shared with other processes.
- Raw L2CAP ATT bypasses the project's current `bluer`/BlueZ GATT boundary and would require a
  separate security, pairing, MTU, and protocol validation effort.

**Conclusion:** port the scheduler invariants and possibly the scan-pause state machine; do not
adopt `habluetooth` wholesale or make its experimental mgmt scanner the production broker.

## 3. Bleak and `bleak-retry-connector`

### 3.1 Bleak

Public references:

- [Bleak repository, Linux BlueZ backend](https://github.com/hbldh/bleak/tree/develop/bleak/backends/bluezdbus)
- [Bleak Linux backend documentation](https://bleak.readthedocs.io/en/latest/backends/linux.html)
- [Bleak repository license](https://github.com/hbldh/bleak/blob/develop/LICENSE)

The reviewed Bleak role is an abstraction layer: `BleakScanner` and `BleakClient` turn BlueZ
D-Bus operations into a Python API. Bleak does not claim to be a global adapter arbitration
service. Separate clients can instantiate scanners/clients and the caller is responsible for
coordination.

The links above use Bleak's moving `develop` branch because an exact Bleak commit was not captured
in this review. **Version/commit: UNKNOWN for the inspected Bleak source.** Pin a release and commit
before depending on it. The project should not add Bleak to the Rust runtime merely to obtain a
scheduler that it does not provide.

### 3.2 Retry connector

The reviewed release is `bleak-retry-connector` 4.7.1, commit
`0bdeaf11d91aa954ee0973ba06c9aba52ddb9971`:

- [README and connection contract](https://github.com/Bluetooth-Devices/bleak-retry-connector/blob/0bdeaf11d91aa954ee0973ba06c9aba52ddb9971/README.md)
- [Retry/error taxonomy and backoff](https://github.com/Bluetooth-Devices/bleak-retry-connector/blob/0bdeaf11d91aa954ee0973ba06c9aba52ddb9971/src/bleak_retry_connector/__init__.py)
- [BlueZ slot manager and stale-disconnect helpers](https://github.com/Bluetooth-Devices/bleak-retry-connector/blob/0bdeaf11d91aa954ee0973ba06c9aba52ddb9971/src/bleak_retry_connector/bluez.py)
- [Package metadata/version/license](https://github.com/Bluetooth-Devices/bleak-retry-connector/blob/0bdeaf11d91aa954ee0973ba06c9aba52ddb9971/pyproject.toml)

Reusable ideas include:

- distinguish transient D-Bus/connect errors from an out-of-slot signal;
- use a longer backoff after controller slot rejection or connection cancellation;
- wait for a failed device to actually disconnect before reconnecting;
- close stale connections discovered by address; and
- expose diagnostic advice rather than silently retrying forever.

Its `BleakSlotManager` is not a true lease manager. It tracks allocations by BlueZ D-Bus device path
and uses an adapter slot count registered by the application. The source comments explicitly note
that the manager cannot see raw L2CAP connections and that adapter slot counts are not generally
queryable. This matters for the proposed design: a broker can use slot accounting as a reservation
and diagnostic hint, but must still handle a real connection failure and must never claim that a
configured slot count proves controller capacity.

**Recommendation for this component:** port the error classes/backoff policy and stale-disconnect
invariants into the Rust broker. Do not depend on its Python `BleakClient` or treat its slot manager
as the cross-process arbitration mechanism.

## 4. NetworkManager and ConnMan

### 4.1 NetworkManager

Revision reviewed: commit
`5e638b3d1281c0dbf089ae70c4e5871887f95307`.

- [NetworkManager BlueZ manager source](https://github.com/NetworkManager/NetworkManager/blob/5e638b3d1281c0dbf089ae70c4e5871887f95307/src/core/devices/bluetooth/nm-bluez-manager.c)
- [NetworkManager Bluetooth device source](https://github.com/NetworkManager/NetworkManager/blob/5e638b3d1281c0dbf089ae70c4e5871887f95307/src/core/devices/bluetooth/nm-device-bt.c)

The code is an adjacent, mature D-Bus lifecycle pattern, not a BLE broker. It:

- establishes a BlueZ D-Bus connection and caches managed adapter/device objects;
- consumes `GetManagedObjects`, `InterfacesAdded`, `InterfacesRemoved`, and
  `PropertiesChanged` events;
- tracks adapter address/powered state and device address/adapter/connected/paired state;
- cleans the cache when BlueZ's name owner disappears;
- uses cancellables and connection timeouts; and
- performs network-specific PAN/DUN activation.

The source's D-Bus event handling is visible at
[`nm-bluez-manager.c` lines 2060-2179](https://github.com/NetworkManager/NetworkManager/blob/5e638b3d1281c0dbf089ae70c4e5871887f95307/src/core/devices/bluetooth/nm-bluez-manager.c#L2060-L2179).
Its network connect/disconnect path is at
[lines 2585-2775](https://github.com/NetworkManager/NetworkManager/blob/5e638b3d1281c0dbf089ae70c4e5871887f95307/src/core/devices/bluetooth/nm-bluez-manager.c#L2585-L2775).

A particularly relevant ownership lesson is the DUN disconnect comment: NetworkManager avoids
calling `Device1.Disconnect()` because it does not know whether another user also uses the device.
That is the same class of hazard as a Wellue collector power-cycling an adapter used by BP/H10.
The broker should require an explicit owner/lease before disconnect or power recovery.

What cannot be reused directly:

- NetworkManager's Bluetooth device is a network profile (PAN/DUN), not a GATT protocol session.
- It has no BLE discovery scheduler, GATT subscription lease, raw-frame journal, or physiological
  durability semantics.
- Copying GPL source into a separate project would require a deliberate license review and is not
  justified when the useful idea is the public D-Bus lifecycle pattern.

### 4.2 ConnMan

Revision reviewed: ConnMan 1.45, commit
`e410e40bc61e5573c2a592bfaebc0fb283ec0fab`.

- [ConnMan Bluetooth plugin, tag/revision source](https://git.kernel.org/pub/scm/network/connman/connman.git/tree/plugins/bluetooth.c?h=1.45)
- [ConnMan repository](https://git.kernel.org/pub/scm/network/connman/connman.git/)

ConnMan's plugin uses BlueZ D-Bus proxies for Bluetooth network profiles. It watches adapter/device
properties, connects/disconnects network services, and deliberately keeps its scope narrow so
non-PAN Bluetooth connections are not disrupted. It does not scan for BLE devices, manage GATT
services, count BLE connection slots, or arbitrate multiple application clients.

Reuse only the narrow lesson: an infrastructure manager should watch BlueZ object/property changes,
keep its profile ownership explicit, and avoid disconnecting a device it does not own. ConnMan is
not a candidate implementation for this broker.

## 5. Other arbitration-related findings

### 5.1 BlueZ management socket

The management protocol is the closest upstream low-level control plane for adapter-wide operations:
[BlueZ `mgmt-protocol.rst`, pinned revision](https://github.com/bluez/bluez/blob/8b4a4176063831476bc9244d20f43b552e4b5554/doc/mgmt-protocol.rst).
It is useful for:

- controller discovery commands;
- accept-list/device management needed by the BP path;
- controller state and event observation; and
- operations that should not be expressed as an arbitrary collector's D-Bus call.

It is **not** a lease scheduler. It does not give this project a durable client identity, priority,
fair queue, deadline, or protection against an unrelated D-Bus process issuing a conflicting
operation. If the broker uses mgmt, it must still be the sole project-owned controller manager and
must enforce the lease policy above it.

### 5.2 No exact public full broker found

Among the public upstream sources reviewed, no project provided all of the following as one
reusable implementation:

- one per-adapter owner across independent processes;
- discovery request coalescing/filter policy;
- explicit connection/GATT/pairing leases;
- adapter power/recovery exclusivity;
- controller identity stable across `hciN` renumbering;
- cancellation and crash cleanup;
- priority/fairness/starvation bounds; and
- source-specific policy such as BP kernel autoconnect plus durable ACK-before-handling.

The closest pieces are complementary: `habluetooth` has the scan scheduler, the experimental HA
mgmt path has a scan-pause/connect critical section, `bleak-retry-connector` has retry/slot
heuristics, and NetworkManager has robust D-Bus lifecycle handling. None is an exact drop-in.

## 6. Proposed broker boundary informed by prior art

### 6.1 Broker owns

One broker process per resolved physical adapter should own:

- one BlueZ D-Bus session and, if needed, one management-socket client;
- adapter discovery start/stop/filter and the merged-filter policy;
- controller power/rfkill/recovery operations;
- object-manager/property subscriptions and BlueZ name-owner recovery;
- a stable `controller_id` resolved from BD address plus validated sysfs/USB identity;
- discovery leases and active-window scheduling;
- one connection-operation lease per device, with explicit deadlines/cancellation;
- persistent connection/GATT subscription ownership;
- pairing-agent registration, if ever needed; and
- capacity reservations and diagnostics, while treating actual slot limits as unknown until
  runtime evidence exists.

Collectors should receive broker events and bounded raw GATT payloads through a narrow IPC API. They
should not import BlueZ bindings or call adapter/device methods directly.

### 6.2 Lease types

| Lease | Owner | Required policy |
|---|---|---|
| Discovery | broker scheduler | One active operation per adapter; merge/queue requests; save/restore filters; report current owner and blocker |
| Active-window | source request | Duration/interval/deadline; port HA owner handoff and connecting fallback |
| Connect operation | source/device | Serialize conflicting attempts; cancel stale attempt; wait for `Connected`/`ServicesResolved` explicitly |
| Persistent device/GATT | collector session | One owner unless protocol explicitly allows sharing; release on process death/IPC disconnect |
| Controller recovery | broker | Exclusive; never power-cycle with unrelated live leases; named blocker in refusal |
| BP management | broker/kernel policy | Preserve no-`Device1.Connect` in BP collector; management action must be auditable and separate |
| Pairing agent | broker/operator | One registered owner, bounded lifetime, cleanup on cancellation/BlueZ restart |

### 6.3 What remains source-specific

The broker must not absorb physiology, registry units/ranges, application authentication, SQL, spool
format, or the BP raw-journal/durability contract. The existing project amendments remain authoritative:
BP's `sync: true` append and raw journal are collector/storage concerns, while the broker reports link
and subscription milestones.

## 7. Adopt, wrap, or build decision

### Adopt unchanged: **No**

- BlueZ itself should be adopted as the underlying daemon/API.
- No reviewed external project supplies the complete broker.
- Adopting `habluetooth` would introduce a Python/asyncio service and still leave an external-client
  arbitration gap.
- Adopting Bleak would introduce a client abstraction without solving arbitration.
- NetworkManager and ConnMan solve different Bluetooth network problems.

### Wrap: **Partially**

A future broker may wrap:

- `bluer`/BlueZ D-Bus for normal adapter/device/GATT operations;
- BlueZ mgmt operations for explicitly privileged BP/controller actions;
- a Rust port of `habluetooth` scheduler invariants;
- a Rust port of `bleak-retry-connector` backoff/error classifications; and
- a D-Bus object-cache/state machine modeled on NetworkManager.

Do not wrap multiple independent high-level client libraries in each collector. That would preserve
the current split ownership.

### Build: **Yes, narrow and project-specific**

Build the broker in Rust inside the Linux collector boundary, with a stable IPC contract and pure
unit-testable scheduling core. The first implementation should deliberately exclude raw L2CAP ATT
and alternate protocol stacks; use the existing BlueZ/`bluer` GATT path unless a measured BlueZ
limitation creates a separate approved task.

## 8. Licensing and integration caveats

- **BlueZ:** use the daemon's documented D-Bus and management interfaces. Do not copy daemon source
  into the project without a GPL/compliance review. The daemon remains a separate system component.
- **`habluetooth`:** `pyproject.toml` declares Apache-2.0. A direct code port is not automatically
  license-free: preserve required notices and assess whether copied structure/code triggers NOTICE
  obligations. An independently implemented Rust state machine based on documented behavior is the
  lower-risk integration route.
- **Bleak and `bleak-retry-connector`:** both declare MIT. Attribution/license retention is still
  required if code is copied; the project has no reason to add their Python runtime dependencies.
- **NetworkManager and ConnMan:** their projects contain GPL-licensed components. Use their public
  behavior and D-Bus design as reference; do not copy source into the Rust broker without reviewing
  the exact file license and distribution consequences.
- **Protocol/API use:** calling BlueZ over D-Bus or the management interface is an interface
  integration, not linking against the BlueZ daemon. Still pin/document the supported BlueZ API,
  kernel, and adapter behavior and test against the appliance's actual versions.
- **Version drift:** `habluetooth` and its experimental mgmt code are active projects. Pin any
  imported algorithm/test evidence to the commit listed here; do not depend on `main`/`develop`
  behavior implicitly. Bleak's exact reviewed commit is **UNKNOWN** and must be pinned before any
  dependency decision.

## 9. Gaps to close before implementation

These are not answered by public prior art:

1. Does the actual Pi's BlueZ/kernel/controller permit the required combination of BP accept-list
   management, active discovery, H10/EMAY/AirSense links, and Wellue protocol operation?
2. What is the actual controller capacity and failure behavior? Public retry libraries explicitly
   treat slot limits as difficult to query; they do not replace a controlled measurement.
3. Does the project require one broker process per physical adapter or one broker for all adapters?
   The recommended default is one per adapter with a supervisor resolving physical identity.
4. What is the exact broker IPC/authentication model and crash cleanup behavior?
5. Does a merged BlueZ discovery filter meet every source's advertisement needs, or must the broker
   time-slice active windows and use passive monitoring where possible?
6. Can the BP kernel-autoconnect policy be represented as a broker-owned management lease without
   violating the no-direct-`Device1.Connect` invariant?
7. Which collectors need persistent connections versus short discovery/connect windows?
8. What evidence proves that adapter power-cycle recovery is safe, or should it be prohibited in
   production entirely?

## 10. Suggested acceptance tests

Before migration, test the broker core without hardware:

- two discovery requests with overlapping and disjoint filters;
- one collector disappearing while it owns discovery;
- filter restoration after timeout, cancellation, BlueZ restart, and adapter removal;
- active-window ownership flip without double firing;
- connecting fallback and bounded retry;
- two connect requests for the same device;
- one persistent GATT lease blocking an unrelated disconnect;
- slot-hint exhaustion versus actual connect failure;
- attempted recovery while another lease is live, with a named blocker;
- `hci0`/`hci1` renumbering while stable controller identity remains constant; and
- BP management/autoconnect event flow proving the BP collector never issues `Device1.Connect`.

Then run a controlled Pi gate, separately and only with owner approval, for controller identity,
BlueZ version, live units, discovery arbitration, connection coexistence, and rollback. Public prior
art supplies algorithms and failure modes; it does not establish those physical facts.
