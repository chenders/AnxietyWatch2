# Reliability-plan entry-point questions

No `LIBRARIAN.md` was found in the thin-client, AnxietyWatch2, aw2-air11, or aw2-wellue-fix trees. Please answer with exact repository-relative file:line references before implementation begins:

1. Which repository/branch is the authoritative Linux deployment source for the rewrite? The inspected trees diverge: `AnxietyWatch2` contains the stopgap Python BP bridge and early `hr-spike`/EMAY wiring; `aw2-air11` contains `vitals-collectors` and `hci0` deployment; `aw2-wellue-fix` contains the provisional Wellue collector.
2. What is the currently verified physical controller identity, USB path/OUI, BlueZ version, kernel version, adapter name, and whether the active controller is `hci0` or `hci1`? The design/runbook requires an `hci1` gate before `disable-bt`, while the inspected udev/unit paths target `hci0`.
3. What exact systemd units and binaries are active on the Pi for BP, H10, EMAY, AirSense, and Wellue? Confirm Wellue remains disabled and identify the deployment commit/artifact hashes.
4. Is privileged kernel accept-list registration intended to remain a separate helper, or may the proposed single BlueZ owner issue management operations? The BP collector must not invoke `Device1.Connect`; the answer must preserve that invariant.
5. What BlueZ/D-Bus API contract and per-device concurrency limits have been validated on this controller, especially for simultaneous H10, EMAY, BP, AirSense, and Wellue operation?
6. Which existing evidence bundle (journal, raw-frame journals, `btmon`/D-Bus trace, PostgreSQL rows) is authoritative for the absent-Wellue/BP-measurement incident, and where is its sanitized file:line/index recorded?

Until answered, the plan treats controller identity, active-unit inventory, and live coexistence results as unknown and does not claim H10 or Wellue acceptance.
