# Tasks — durable EMAY SleepO2 ingestion

**Status:** Blocked pending a stable EMAY source identity or an explicitly accepted exception to the
identity rule.

**Amends:** `docs/specs/AMENDMENTS.md` (mandatory); no accepted amendment is changed.

## Task index

- [ ] 1.1 Establish a stable EMAY record identity
- [ ] 1.2 Implement the durable collector after identity approval
- [ ] 1.3 Add PostgreSQL-backed panel history after durable ingestion
- [ ] 1.4 Activate and verify on pi-host

### T-E1 — Establish a stable EMAY record identity

**Tier:** investigation/decision · **Host:** either for documentation, Pi only for non-disruptive
inspection · **Context budget:** ~3k · **Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`,
`docs/specs/2026-08-03-amendment-05-corrections-and-runtime-safety.md`,
`docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md`,
`crates/vitals-decode/src/emay.rs`, `crates/emay-spike/src/main.rs`,
`docs/research/2026-08-03-ua651ble-ble-capture.md`

**Creates:** an evidence/decision note for EMAY identity.

**Do:** Confirm whether the EMAY protocol or a documented alternate mode provides a retransmission-
stable counter/timestamp/record identifier or a confirmed store-and-forward channel. Inspecting the
already-connected device is allowed; active scans, unverified writes, and overnight resets are not.

**Verify:** `! rg -n "delivery sequence|delivery index|Session::next_seq" openspec/changes/emay-durable-ingestion/design.md`

**Expect:** exit 0 only while the design correctly refuses the unapproved host-delivery identity.

### T-E2 — Implement the durable collector after identity approval

**Tier:** implement-complex · **Host:** either for portable tests, Pi for Linux build/live use ·
**Context budget:** ~8k · **Depends:** T-E1 decision · **Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, `vitals/registry.toml`, `crates/vitals-decode/src/emay.rs`,
`crates/vitals-core/src/lib.rs`, `crates/vitals-core/src/session.rs`,
`crates/vitals-proto/src/lib.rs`, `crates/emay-spike/src/main.rs`, `deploy/vitalsd.service`

**Creates:** a production Linux collector, tests/fuzzing required by any new byte parser, and a
service definition.

**Do:** Use only the approved source identity. Route values through `vitalsd` → COPY spool →
PostgreSQL; use generated enums and registry ranges; preserve absent/no-finger state; do not add a
second store or disturb H10/BP.

**Verify:** a task-specific local/Pi build, test, clippy, and live append command that proves the
approved identity is used and EMAY rows advance in PostgreSQL.

**Expect:** exit 0 only after T-E1 has a recorded approved identity decision.

### T-E3 — Add PostgreSQL-backed panel history

**Tier:** implement-standard · **Host:** either · **Context budget:** ~5k · **Depends:** T-E2 ·
**Review required:** no

**Reads:** `docs/design/pi-panel/direction-a-clinical/live-vitals.html`,
`tools/panel_vitals_status.py`

**Creates:** only the existing history read-model tests/projection changes.

**Do:** Add EMAY latest/delta/full-history fields to the existing PostgreSQL-backed panel projection
and merge them into the existing graph. Keep the bounded live overlay only for drain latency. Do not
create a parallel durable history file or alter H10/BP semantics.

**Verify:** an exact command that runs the publisher tests, JavaScript syntax/layout checks, and
`git diff --check`.

### T-E4 — Activate and verify on pi-host

**Tier:** implement-complex · **Host:** Pi · **Context budget:** ~5k · **Depends:** T-E2, T-E3 ·
**Review required:** yes

**Reads:** `docs/specs/AMENDMENTS.md`, the approved identity note, collector/service definitions,
`deploy/stopgap/live-vitals-watch`

**Do:** Build and verify before the service switch; stop the display-only service and start exactly
one approved durable EMAY collector. Do not restart H10/vitalsd/PostgreSQL/kiosk, run active scans,
or reboot the Pi.

**Verify:** a recorded pi-host command must show the durable unit active, the current EMAY stream
fresh, the EMAY source/type row count increasing, and H10 still advancing.

## Completion gate

No task may be ticked until the identity issue is resolved under the accepted amendments. The final
change requires actual EMAY rows in PostgreSQL, database-backed history after page reload, healthy
H10/EMAY operation, and final status/chat records. Until then, the display-only graph and service
remain the honest safe state.
