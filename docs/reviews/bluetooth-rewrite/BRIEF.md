# Bluetooth rewrite investigation brief

## Mission

Design, without implementation or operational changes, a complete rewrite plan for Linux Bluetooth access in AnxietyWatch2. Focus on BlueZ on Raspberry Pi and the specific USB Bluetooth dongle/adapter used by the appliance. The plan may recommend replacing the current architecture without an artificial scope limit. Do not design iOS CoreBluetooth behavior except where it crosses the Linux boundary.

All findings must be evidence-backed. Distinguish observed facts, code observations, hypotheses, and decisions requiring physical validation. Do not invent hardware or protocol evidence, and do not expose credentials, private keys, target secrets, or personal data.

## Historical context and observed evidence

- The appliance has multiple BLE sources: A&D UA-651BLE blood-pressure cuff, ResMed AirSense 11, EMAY SleepO2, Wellue/Viatom KS-60FWB, and Polar H10. The project also has a Sena UD100/CSR USB adapter history and a Pi Bluetooth adapter. Exact current controller identity must be read from code/deployment/evidence, not assumed.
- The current Pi is `chris@192.168.1.48`. Do not perform physical actions or change services during this investigation.
- BP amendment rule: the BP collector must not call `Device1.Connect`; it waits for privileged/kernel autoconnect. BP frames are durable before acknowledgement because the cuff data is not reconstructible.
- A recent live incident demonstrated cross-collector contention:
  - `wellue-collector` repeatedly attempted discovery for configured target `00:00:00:03:E3:F1` while the device was absent.
  - `bluetoothctl show` reported the adapter `Discovering: yes`.
  - BP target `00:09:1F:98:EB:CF` was paired/trusted but `Connected: no`; BP had no recent log entries and was waiting for autoconnect.
  - `wellue-collector` was then stopped and disabled temporarily. Discovery changed to `no` immediately.
  - Without restarting BP or manually calling `Connect`, the BP collector logged frames 8 and 9 durable in the vitalsd spool, and PostgreSQL showed fresh source 6 rows at the same timestamp (systolic/diastolic/MAP/pulse values).
  - This is strong evidence of an adapter/discovery arbitration defect, not merely an isolated cuff failure.
- AirSense currently has a separate symptom: its BLE link can report `streaming` while therapy lifecycle says `complete` and the bedside feed calls old values stale even when the owner reports that the machine is physically blowing. Treat this as potentially separate protocol/session/event-state logic, not proof of discovery contention.
- H10 acceptance remains blocked by lack of fresh strap measurements. Do not fabricate acceptance.
- Wellue is currently intentionally disabled pending this investigation. Do not re-enable it.
- Earlier stabilization work added bounded Wellue discovery/cooldown, BP startup ordering, and AirSense status/panel projections, but the incident above shows that local mitigations may not establish a global adapter policy.

## Repository/code locations to inspect

Primary appliance repository:
- `/Users/chris/Source/AnxietyWatch2`
- Rust collectors: `crates/vitals-collectors/`
- deployment/systemd/udev: `deploy/`, `deploy/stopgap/`
- shared decode/core crates: `crates/vitals-decode/`, `crates/vitals-core/`, `crates/vitalsd/`
- registry: `vitals/registry.toml`
- relevant docs/specs: `docs/specs/AMENDMENTS.md`, amendment 06, amendment 17, `docs/design/`, `openspec/`

Related repositories:
- `/Users/chris/Source/aw2-air11`
- `/Users/chris/Source/aw2-wellue-fix`

Known relevant files from prior work include:
- `crates/vitals-collectors/src/bin/bp_collector.rs`
- `crates/vitals-collectors/src/bp_build.rs`
- Wellue collector `crates/wellue-collector/src/main.rs` in the Wellue repository
- AirSense session/status/collector files in `aw2-air11`
- `deploy/99-vitals-bp-autoconnect.rules`
- `deploy/vitals-bp.service`, `deploy/vitals-h10.service`, and Wellue service definitions
- `scripts/sync-to-pi.sh`

## Required output

Produce Markdown only. Each specialist must write:
1. an evidence-backed report,
2. a proposed rewrite architecture and boundaries,
3. a staged migration/rollback plan,
4. explicit unresolved questions and physical/runtime gates,
5. a list of exact files/APIs/tests to inspect or change.

The Bluetooth Implementation Librarian must maintain the authoritative inventory of Bluetooth entry points, BlueZ object/API usage, adapter/controller assumptions, service ownership, and evidence. Specialists may ask it questions through request Markdown files; the librarian must answer with file/line references or mark the answer unknown.

No agent may edit Rust, SQL, systemd, udev, firmware, hardware, or runtime configuration as part of this investigation.
