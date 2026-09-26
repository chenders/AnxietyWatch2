# Tasks — Linux BlueZ broker and collector access rewrite

> For agentic workers: read each task's `Reads:` list plus `docs/specs/AMENDMENTS.md`, which is always in scope. Work one task at a time. Acceptance is the shell command on its `Verify:` line, not an assessment. Do not tick a box until the command exits 0. Never run old and new BlueZ owners concurrently.

## Goal

Move Linux application-level Bluetooth ownership to one broker per verified physical adapter while preserving BP kernel autoconnect, raw-frame durability, source isolation, and explicit unknown/degraded states.

## Global constraints

- Rust, not C; `cargo clippy --all-targets -- -D warnings` is mandatory for changed Rust.
- Only the broker may use application-level BlueZ/D-Bus operations after migration. The narrow BP kernel-policy boundary is the only named exception.
- No generic D-Bus escape hatch crosses the broker socket.
- BP must never receive a capability or code path that calls `Device1.Connect`; it waits for kernel autoconnect and preserves the `sync: true`/raw-journal contract from amendment 06.
- `hci0`/`hci1` are runtime names, not hardware identity. Never bind by index alone.
- No unbounded discovery, retry, adapter reset, or per-source global power cycle.
- Wellue remains disabled. H10 acceptance requires fresh physical measurements. AirSense therapy lifecycle remains separate.
- No task changes `/boot/firmware`, applies `dtoverlay=disable-bt`, pairs devices, re-enables Wellue, or performs an unapproved Pi operation.
- Acceptance evidence distinguishes measured-good, measured-bad, stale/degraded, and unknown.

## Task index

- [x] T-BT-0 — Freeze deployment inventory and stable-controller contract · investigation
- [x] T-BT-1 — Add pure lease scheduler and broker capability model · implement-complex
- [x] T-BT-2 — Add the versioned broker protocol and fake transport · implement-complex
- [x] T-BT-3 — Implement the Linux BlueZ adapter owner · implement-complex · Linux/Pi
- [x] T-BT-4 — Isolate and instrument the BP kernel-policy boundary · implement-complex · amendment 06
- [x] T-BT-5 — Migrate one source through the broker with reversible ownership · implement-complex · gated
- [x] T-BT-6 — Migrate remaining collectors and retire direct BlueZ paths · implement-complex · gated
- [x] T-BT-7 — Add coexistence, failure-injection, and Pi acceptance gates · implement-complex · review required
- [x] T-BT-8 — Add the Linux broker daemon and service boundary · implement-complex · Linux/Pi

---

### T-BT-0 — Freeze deployment inventory and stable-controller contract

**Depends:** none
**Amends:** none
**Reads:** `docs/reviews/bluetooth-rewrite/BRIEF.md`, `docs/reviews/bluetooth-rewrite/LIBRARIAN.md`, `docs/reviews/bluetooth-rewrite/COLLECTOR-AUDIT.md`, `docs/reviews/bluetooth-rewrite/PI-BLUEZ-PLATFORM.md`, `docs/specs/AMENDMENTS.md`
**Modifies:** `openspec/changes/linux-bluez-broker/design.md` or a dated evidence document only; no runtime files

Resolve the authoritative deployed artifact/revision, enabled units, direct BlueZ call sites, adapter/controller identity evidence, BlueZ/kernel versions, BP accept-list registration evidence, and source-specific timing budgets. Record contradictions as unknowns rather than choosing between `hci0` and `hci1`. Define the stable adapter selector and the refusal behavior for an ambiguous or missing match. This task is documentation and evidence only; it does not connect to the Pi or change services.

**Verify:** `test -s docs/reviews/bluetooth-rewrite/LIBRARIAN.md && rg -q "hci0|hci1" docs/reviews/bluetooth-rewrite/PI-BLUEZ-PLATFORM.md && rg -q "unknown|Unknown" openspec/changes/linux-bluez-broker/design.md`

---

### T-BT-1 — Add pure lease scheduler and broker capability model

**Depends:** T-BT-0
**Amends:** none
**Reads:** `openspec/changes/linux-bluez-broker/proposal.md`, `openspec/changes/linux-bluez-broker/design.md`, `docs/reviews/bluetooth-rewrite/ARBITRATION-ARCHITECTURE.md`, `docs/reviews/bluetooth-rewrite/RELIABILITY-TEST-PLAN.md`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/vitals-core/Cargo.toml`, `crates/vitals-core/src/lib.rs`, `docs/specs/AMENDMENTS.md`
**Creates:** `crates/vitals-ble-broker/` policy modules and tests

Create the broker package's pure policy layer without BlueZ, I/O, or global state. Model adapter-global and device-local leases, source priorities, deadlines, preemption only at safe boundaries, lease expiry, retry release, adapter generations, bounded worker queues, capability sets, and named blocker values. Include a BP capability that cannot represent `Connect`, adapter reset, power mutation, or generic D-Bus calls. Keep source state distinctions (`Connected`, `ServicesResolved`, `Subscribed`, `Streaming`, `Stale`, `Degraded`, `Unknown`) explicit.

**Verify:** `cargo test -p vitals-ble-broker --lib && cargo clippy -p vitals-ble-broker --all-targets -- -D warnings`

---

### T-BT-2 — Add the versioned broker protocol and fake transport

**Depends:** T-BT-1
**Amends:** none
**Reads:** `openspec/changes/linux-bluez-broker/design.md`, `docs/reviews/bluetooth-rewrite/RELIABILITY-TEST-PLAN.md`, `crates/vitals-ble-broker/Cargo.toml`, `crates/vitals-ble-broker/src/lib.rs`, `crates/vitals-ble-broker/src/lease.rs`, `Cargo.toml`, `docs/specs/AMENDMENTS.md`
**Creates:** broker protocol/client modules and fake transport tests

Define bounded, versioned Unix-socket messages for `Hello`, `Acquire`, typed operations, `Frame`, `Release`, `Cancel`, and `Health`. Authenticate worker kind and capabilities at handshake, include broker generation and adapter selector, bound frame and queue sizes, redact device identifiers in errors, and reject unknown capabilities. No raw BlueZ object paths or arbitrary method invocation may cross the protocol. Test worker death, cancellation, expiry, backpressure, generation changes, malformed frames, and BP capability rejection.

**Verify:** `cargo test -p vitals-ble-broker --test protocol && cargo clippy -p vitals-ble-broker --all-targets -- -D warnings`

---

### T-BT-3 — Implement the Linux BlueZ adapter owner

**Depends:** T-BT-1, T-BT-2
**Amends:** none
**Reads:** `openspec/changes/linux-bluez-broker/design.md`, `docs/reviews/bluetooth-rewrite/LIBRARIAN.md`, `docs/reviews/bluetooth-rewrite/COLLECTOR-AUDIT.md`, `docs/reviews/bluetooth-rewrite/PI-BLUEZ-PLATFORM.md`, `crates/vitals-ble-broker/Cargo.toml`, `crates/vitals-ble-broker/src/lib.rs`, `crates/vitals-ble-broker/src/lease.rs`, `crates/vitals-ble-broker/src/protocol.rs`, `Cargo.toml`, `Cargo.lock`, `crates/vitals-collectors/Cargo.toml`, `docs/specs/AMENDMENTS.md`
**Creates:** broker BlueZ adapter modules and Linux contract tests

Implement the only application-level BlueZ/D-Bus owner. Resolve an adapter by stable selector and record current `hciX` only as runtime evidence. Wrap discovery/filter snapshot and restoration, device connect/disconnect, service resolution, notification/indication subscription, writes, deadlines, cancellation, and adapter-generation changes in typed operations. Serialize discovery and new-connect admission, serialize per-device GATT writes, expose ownership and blocker values, and classify BlueZ/D-Bus/USB/controller failures. Do not add a generic D-Bus call. Observe BP kernel-created connections without initiating BP connect.

**Verify:** `cargo test -p vitals-ble-broker --test bluez_contract --features linux && cargo clippy -p vitals-ble-broker --all-targets --features linux -- -D warnings`

---

### T-BT-4 — Isolate and instrument the BP kernel-policy boundary

**Depends:** T-BT-2, T-BT-3
**Amends:** amendment 06 — BP identity and durability
**Reads:** `docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md`, `docs/reviews/bluetooth-rewrite/ARBITRATION-ARCHITECTURE.md`, `docs/reviews/bluetooth-rewrite/COLLECTOR-AUDIT.md`, `docs/specs/AMENDMENTS.md`
**Modifies:** future BP kernel-policy helper/unit/rule and BP broker client; exact paths are selected after T-BT-0

Keep kernel accept-list registration separate from the unprivileged broker and BP worker. Make controller selection, address type, registration result, hotplug re-registration, and failure values observable. Move BP indication observation/subscription and bounded raw-frame delivery through the broker without adding a userspace connect path. Preserve raw journal-before-decode and `append { sync: true }` reply ordering. Remove the old userspace BP connector only after the physical gate proves kernel-created connection, subscription, and durable spool evidence. A broker or worker crash must be represented as durability unknown, never success.

**Verify:** `! rg -n 'Device1\\.Connect' crates/vitals-collectors/src/bin/bp_collector.rs crates/vitals-ble-broker 2>/dev/null && cargo test -p vitals-ble-broker --test bp_policy`

---

### T-BT-5 — Migrate one source through the broker with reversible ownership

**Depends:** T-BT-3 and T-BT-4
**Amends:** none
**Reads:** `openspec/changes/linux-bluez-broker/design.md`, `docs/reviews/bluetooth-rewrite/ARBITRATION-ARCHITECTURE.md`, `docs/reviews/bluetooth-rewrite/RELIABILITY-TEST-PLAN.md`, `crates/vitals-ble-broker/Cargo.toml`, `crates/vitals-ble-broker/src/lib.rs`, `crates/vitals-ble-broker/src/protocol.rs`, `crates/vitals-ble-broker/src/bluez.rs`, `crates/vitals-collectors/Cargo.toml`, `crates/vitals-collectors/src/bin/h10_collector.rs`, `crates/vitals-collectors/src/h10_reconnect.rs`, `crates/vitals-collectors/src/h10_build.rs`, `deploy/vitals-h10.service`, `docs/specs/AMENDMENTS.md`
**Modifies:** exactly one selected collector, its unit, and broker client wiring; selection requires owner approval after T-BT-0

Use the H10 worker as the first source migration target only after the deployment inventory and offline/fake tests are green; this is a transport migration, not H10 acceptance. Remove its direct BlueZ capability and route discovery/connect/GATT operations through the broker. Preserve protocol decoding, source identity, status, and ingest. Add a source-level rollback that stops the new client, proves lease/subscription release, and starts the prior approved owner without overlap. This task must not re-enable Wellue.

**Verify:** `cargo test --workspace && test -z "$(rg -n 'bluer|Device1\\.(StartDiscovery|StopDiscovery|Connect|Disconnect)' crates/vitals-collectors/src/bin/h10_collector.rs || true)" && git diff --check`

---

### T-BT-6 — Migrate remaining collectors and retire direct BlueZ paths

**Depends:** T-BT-5
**Amends:** amendment 06 — BP identity and durability
**Reads:** `openspec/changes/linux-bluez-broker/design.md`, `docs/reviews/bluetooth-rewrite/COLLECTOR-AUDIT.md`, `docs/reviews/bluetooth-rewrite/LIBRARIAN.md`, `docs/specs/AMENDMENTS.md`, `docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md`, `crates/vitals-ble-broker/Cargo.toml`, `crates/vitals-ble-broker/src/lib.rs`, `crates/vitals-ble-broker/src/protocol.rs`, `crates/vitals-ble-broker/src/client.rs`, `crates/vitals-ble-broker/src/bluez.rs`, `crates/vitals-collectors/Cargo.toml`, `crates/vitals-collectors/src/bin/bp_collector.rs`, `crates/vitals-collectors/src/bp_build.rs`, `crates/vitals-collectors/src/bin/bp_journal_replay.rs`, `deploy/vitals-bp.service`, `deploy/vitals-bp-autoconnect.service`, `deploy/99-vitals-bp-autoconnect.rules`
**Modifies:** BP observation/subscription path, kernel-policy helper contract, deployment units, and worker dependencies one source at a time. H10 is already brokered; absent SleepO2/AirSense/EMAY production sources remain separate follow-up work.

The authorized `btmgmt` kernel-policy helper is retained as the narrow BP exception. It is excluded from the application-worker static scan, which must still reject direct BlueZ/D-Bus access in collectors.

Migrate each remaining authorized source independently. Keep established streams alive while another source discovers, subject to measured capacity. Keep Wellue disabled unless a separate authorization gate passes. Remove direct `bluer`/D-Bus access from workers, stopgap discovery/connect loops, and source-level adapter resets only after the replacement path and rollback are verified. Preserve AirSense transport/session and therapy lifecycle as separate state machines.

**Verify:** `cargo clippy --workspace --all-targets -- -D warnings && test -z "$(rg -n 'bluer::|StartDiscovery|StopDiscovery|Device1\.(Connect|Disconnect)\b' crates/vitals-collectors/src crates/emay-collector/src deploy --glob '!vitals-bp-autoconnect.service' 2>/dev/null || true)" && rg -q 'btmgmt add-device' deploy/vitals-bp-autoconnect.service && git diff --check`

---

### T-BT-7 — Add coexistence, failure-injection, and Pi acceptance gates

**Depends:** T-BT-4, T-BT-5, T-BT-6
**Amends:** amendment 05 — corrections and runtime safety; amendment 06 — BP identity and durability; all current safety amendments governing unknown/degraded state
**Reads:** `openspec/changes/linux-bluez-broker/design.md`, `docs/reviews/bluetooth-rewrite/RELIABILITY-TEST-PLAN.md`, `docs/reviews/bluetooth-rewrite/PI-BLUEZ-PLATFORM.md`, `docs/specs/AMENDMENTS.md`, `docs/specs/2026-08-03-amendment-05-corrections-and-runtime-safety.md`, `docs/specs/2026-08-03-amendment-06-bp-identity-and-durability.md`, `crates/vitals-ble-broker/src/lease.rs`, `crates/vitals-ble-broker/src/bp.rs`, `crates/vitals-ble-broker/src/protocol.rs`, `crates/vitals-ble-broker/tests/bluez_contract.rs`, `crates/vitals-ble-broker/tests/bp_policy.rs`
**Creates:** deterministic integration tests, sanitized evidence fixtures, and acceptance documentation

Add and run the absent-Wellue plus BP-measurement test, discovery filter restoration tests, worker death/backpressure tests, broker/BlueZ restart tests, adapter-generation tests, BP journal/`sync: true` tests, and finite-window/overnight coexistence evidence. Record packet gaps, notification latency, keepalive misses, reconnects, discovery hold time, queue delay, append latency, and unknown/degraded transitions. Linux/Pi evidence must be distinguished from host/fake tests. No source is accepted from a process-start or stale `Connected` observation.

**Verify:** `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && git diff --check`

---

### T-BT-8 — Add the Linux broker daemon and service boundary

**Depends:** T-BT-1, T-BT-2, T-BT-3, T-BT-4, T-BT-5, T-BT-6
**Amends:** none
**Reads:** `openspec/changes/linux-bluez-broker/design.md`, `docs/reviews/bluetooth-rewrite/DEPLOYMENT-INVENTORY.md`, `docs/specs/AMENDMENTS.md`, `crates/vitals-ble-broker/Cargo.toml`, `crates/vitals-ble-broker/src/lib.rs`, `crates/vitals-ble-broker/src/bluez.rs`, `crates/vitals-ble-broker/src/client.rs`, `crates/vitals-ble-broker/src/protocol.rs`, `deploy/vitals-bp.service`, `deploy/vitals-h10.service`
**Creates:** `crates/vitals-ble-broker/src/server.rs`, `crates/vitals-ble-broker/src/bin/vitals-ble-broker.rs`, `deploy/vitals-ble-broker.service`
**Modifies:** broker lease cleanup and owner integration only; no Pi/runtime changes

Add the Linux broker executable listening on `/run/vitals/ble-broker.sock`. Authenticate the private-socket worker handshake and stable adapter selector, resolve the configured controller against local sysfs evidence, and open the existing typed Linux BlueZ owner. Dispatch only typed leases and operations, enforce BP's no-userspace-connect capability, stream bounded notifications, and release device leases on worker disconnect. Keep Wellue disabled and keep the `btmgmt` helper separate. The systemd unit must run unprivileged, create the socket with bounded permissions, and retain the safety-path resource settings. Host verification does not close Linux/BlueZ/Pi acceptance.

**Verify:** `cargo test -p vitals-ble-broker --lib && cargo clippy -p vitals-ble-broker --all-targets -- -D warnings && test -s deploy/vitals-ble-broker.service && rg -q 'ble-broker.sock' deploy/vitals-ble-broker.service && git diff --check`
