# Answer: reliability-plan entry-point questions

**Request:** `2026-09-22-reliability-entry-points.md`
**Status:** PARTIAL / UNKNOWN where runtime evidence is required.
**Scope:** repository evidence only; no Pi access, service changes, Wellue re-enable, or runtime/config changes.

## 1. Authoritative Linux deployment source

There is no single authoritative checked-in deployment source that can be established from the
three trees. The repositories visibly diverge:

- `AnxietyWatch2` contains the durable EMAY collector
  `[AnxietyWatch2/crates/emay-collector/src/main.rs:1363-1386]`, the stopgap Python BP bridge
  `[AnxietyWatch2/deploy/stopgap/vitals-bp-connect:116-195]`, and the EMAY/H10 spike entry points
  `[AnxietyWatch2/crates/emay-spike/src/main.rs:238-249,329-361,567-612; AnxietyWatch2/crates/hr-spike/src/main.rs:102-121,207-220,365-411]`.
  It does not contain `crates/vitals-collectors/` or the production Wellue crate in the checked
  out tree.
- `aw2-air11` contains the production `vitals-collectors` BP/H10/AirSense implementations and
  their units: `vitals-bp.service` `[aw2-air11/deploy/vitals-bp.service:1-37]`,
  `vitals-h10.service` `[aw2-air11/deploy/vitals-h10.service:1-37]`, and
  `vitals-air11.service` `[aw2-air11/deploy/vitals-air11.service:1-53]`.
- `aw2-wellue-fix` contains the provisional Wellue collector and unit
  `[aw2-wellue-fix/crates/wellue-collector/src/main.rs:211-264,517-629; aw2-wellue-fix/deploy/wellue-collector.service:1-35]`.

**Answer:** the rewrite brief's primary context is `/Users/chris/Source/AnxietyWatch2`, but the
currently inspected source is a multi-repository/divergent implementation. The authoritative
*future* deployment source is **UNKNOWN** and requires an owner decision before migration.

## 2. Current verified physical controller identity

**UNKNOWN; no current physical/runtime verification was performed.** Checked-in evidence is
historical and contradictory:

- Historical task notes record `0a12:0001 Cambridge Silicon Radio`, Sena OUI `00:01:95`, USB
  `hci1`, while the onboard UART was `hci0` `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:1267-1284]`.
- The same notes record that after onboard Bluetooth was disabled, the Sena renumbered to `hci0`
  `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:1307-1324]`.
- A later acceptance record reports exactly one controller at `hci0`, says required Sena `hci1`
  was absent, and leaves G1 not green `[AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md:76-82]`.
- The historical A&D capture identifies the test environment as Sena UD100-G03/CSR8510, kernel
  `6.12.96+rpt-rpi-2712`, BlueZ `5.66` `[aw2-air11/docs/research/2026-08-03-ua651ble-ble-capture.md:3-6]`.

The following current facts are all **UNKNOWN**: physical USB path, current USB VID/PID, current
controller BD address, current BlueZ version, current kernel version, active adapter name, and
whether the active controller is `hci0` or `hci1`. The design/runbook's G1 gate must not be treated
as presently open; its required ordering is `[aw2-wellue-fix/openspec/changes/pi5-vitals-core/tasks.md:190-220]`.

## 3. Active units, binaries, Wellue state, and hashes

The repository defines these intended units and binaries:

- BP: `vitals-bp.service` → `/usr/local/bin/bp-collector`
  `[aw2-air11/deploy/vitals-bp.service:6-14]`.
- H10: `vitals-h10.service` → `/usr/local/bin/h10-collector`
  `[aw2-air11/deploy/vitals-h10.service:6-17]`.
- EMAY durable: `vitals-emay.service` → `/usr/local/bin/emay-collector`
  `[AnxietyWatch2/deploy/vitals-emay.service:15-24]`.
- AirSense: `vitals-air11.service` → `/usr/local/bin/resmed-air11-collector`
  `[aw2-air11/deploy/vitals-air11.service:14-22]`.
- Wellue: `wellue-collector.service` → `/usr/local/bin/wellue-collector`
  `[aw2-wellue-fix/deploy/wellue-collector.service:15-22]`.

Current active/enabled state of BP, H10, EMAY, AirSense, and Wellue is **UNKNOWN** from the
repository alone. The brief explicitly controls this investigation and says Wellue is intentionally
disabled `[AnxietyWatch2/docs/reviews/bluetooth-rewrite/BRIEF.md:21-24]`; I did not re-enable it.

The checked-in acceptance record is a historical deployment report, not current live state. It
records `vitals-emay` auto-restarting with an absent EMAY object and `vitals-h10` timing out
`[AnxietyWatch2/docs/status-updates/2026-09-05T18-16-00-0700-ts5-linux-and-pi-acceptance.md:68-69]`.
It records hashes for `vitalsd`, `vitals-spool-drain`, `emay-collector`, and their units, but not
BP, H10, AirSense, or Wellue artifacts `[...ts5-linux-and-pi-acceptance.md:42-60]`. The current
installed artifact hashes and deployment commit are therefore **UNKNOWN**.

## 4. Privileged accept-list ownership versus BP `Device1.Connect`

Current intended ownership is split:

- `vitals-bp-autoconnect.service` is a root oneshot that runs
  `btmgmt add-device -a 2 -t 1 "$BP_CUFF_MAC"` `[aw2-air11/deploy/vitals-bp-autoconnect.service:15-20]`.
- The udev rule retriggers that unit for Bluetooth `hci0` add events
  `[aw2-air11/deploy/99-vitals-bp-autoconnect.rules:1-4]`.
- Production `bp-collector` explicitly documents that it never calls `Device1.Connect`; it waits
  for kernel autoconnect, then observes `Connected`/`ServicesResolved` and subscribes to 2A35
  `[aw2-air11/crates/vitals-collectors/src/bin/bp_collector.rs:1-14,253-304]`.
- The separate stopgap Python helper does call `Device1.Connect()` when it sees RSSI
  `[AnxietyWatch2/deploy/stopgap/vitals-bp-connect:183-195]`; this is a competing legacy/stopgap
  path and must not be used as evidence that production BP is allowed to connect directly.

**Answer to proposed owner question:** the current repository intent is that privileged kernel
accept-list registration remains a separate helper. A future single BlueZ owner *may* be proposed
to own management-socket operations, but that is a design decision, not an accepted current fact.
Any rewrite must preserve the invariant: **the BP collector itself must not invoke
`Device1.Connect`.**

## 5. Validated BlueZ API contract and concurrency limits

No complete per-controller concurrency contract has been validated in the inspected evidence.
The code shows these local contracts:

- AirSense checks/blocks active discovery, uses an LE FD56 filter, and restores the prior filter
  `[aw2-air11/crates/vitals-collectors/src/as11_bluez.rs:688-750]`.
- Wellue starts discovery without checking global discovery state and can power-cycle the adapter
  after selected failures `[aw2-wellue-fix/crates/wellue-collector/src/main.rs:211-238,457-501]`.
- H10 bounds connect, service resolution, subscribe, notification idle, and disconnect cleanup
  `[aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs:35-51,240-328,389-468]`.
- EMAY owns a connected notification stream and periodically multiplexes live and stored transfers
  `[AnxietyWatch2/crates/emay-collector/src/main.rs:1260-1355]`.
- Amendment/design material says slot exhaustion may present as a missing D-Bus device and gives
  historical CSR/BCM slot observations, but explicitly says the limit cannot be queried
  `[aw2-air11/docs/design/HOMEASSISTANT-ARCHITECTURE.md:204-212]`. This is not a current controller
  measurement.

Validated simultaneous operation for H10 + EMAY + BP + AirSense + Wellue is **UNKNOWN**. The brief
records a strong Wellue-discovery/BP recovery correlation `[AnxietyWatch2/docs/reviews/bluetooth-rewrite/BRIEF.md:14-20]`,
but no controlled concurrency trace or slot measurement is present in the inspected paths.

## 6. Authoritative evidence bundle for the Wellue/BP incident

The authoritative checked-in index for the reported incident is the controlling brief:

- Wellue repeatedly attempted discovery and `bluetoothctl show` reported `Discovering: yes`
  `[AnxietyWatch2/docs/reviews/bluetooth-rewrite/BRIEF.md:14-16]`.
- BP was paired/trusted but disconnected and waiting for autoconnect
  `[BRIEF.md:17]`.
- Stopping/disabling Wellue changed discovery to `no`; BP then logged frames 8 and 9 durable and
  PostgreSQL showed fresh source-6 rows without a BP restart or manual Connect
  `[BRIEF.md:18-20]`.

The checked-in A&D capture is a separate, earlier evidence bundle: it records the Sena/CSR test
platform, no LE Create Connection from `bluetoothctl connect`, the kernel confirmation/durability
boundary, and bonded SMP failure `[aw2-air11/docs/research/2026-08-03-ua651ble-ble-capture.md:73-135]`.
It does not provide the later Wellue/BP incident's raw journal, D-Bus trace, or PostgreSQL query
transcript.

Therefore the exact sanitized incident journal/trace bundle path and event timestamp are
**UNKNOWN**. The repository records the incident narrative in the brief, but does not contain
independent machine-readable runtime artifacts sufficient to reproduce or audit every step.

## 7. Exact unresolved questions and test gaps

The next implementation/review pass should answer these without assuming hardware facts:

1. Which repository/branch is owner-approved as the deployment source?
2. What stable controller identity (BD address + USB/sysfs path) is present now, independent of
   `hciN`?
3. Which units are active/enabled now, and which exact binaries/units are installed?
4. Can one owner serialize discovery and restore filters after every failure?
5. Can a broker own kernel management/autoconnect without allowing BP to issue
   `Device1.Connect`?
6. What happens when discovery, H10 connect, EMAY live traffic, BP autoconnect, AirSense fallback,
   and Wellue discovery overlap?
7. What are the actual controller connection slots and failure codes under controlled load?
8. Can an adapter power-cycle be refused while another source lease is active?
9. Can the reported Wellue/BP incident be replayed from sanitized event traces?

Existing tests provide partial seams: Wellue reset/discovery predicates
`[aw2-wellue-fix/crates/wellue-collector/src/main.rs:681-712]`, AirSense candidate selection
`[aw2-air11/crates/vitals-collectors/src/as11_onboarding.rs:566-605]`, H10 capture/link seams
`[aw2-air11/crates/vitals-collectors/src/bin/h10_collector.rs:490-539]`, and EMAY framing/live/storage
coverage `[AnxietyWatch2/crates/emay-collector/src/main.rs:1647-1836,2251-2565]`. There is no
existing global discovery lease, stable-controller resolver, or multi-client arbitration test
seam in the inspected code; those are **missing**, not implicitly verified.
