# Tasks — move AirSense 11 behind the Linux BLE broker

The conductor ticks a box only after running that task's **Verify** command and
seeing exit 0. These tasks authorize host implementation and static checks; they
do not authorize pairing, service enablement, adapter changes, reboot, or live
Pi acceptance.

**Amends:** `docs/specs/AMENDMENTS.md` is mandatory for every task. This change
follows amendments 01, 02, 04, 05, 06, 07, 10, 11, and 12. It supersedes the
old direct-BlueZ transport assumption in the AirSense stream design only where
it conflicts with the broker ownership decision.

- [ ] T-A11-BROKER-0 — Publish the broker migration contract and design · documentation · host either
- [ ] T-A11-BROKER-1 — Expose bounded discovery and write operations in the broker client · implement-complex · host/Linux
- [ ] T-A11-BROKER-COV — Enforce 100% executable broker coverage · implement-complex · Linux/CI
- [ ] T-A11-BROKER-2 — Replace direct AirSense BlueZ transport with the broker lease · implement-complex · Linux/Pi for transport, either for tests
- [ ] T-A11-BROKER-3 — Update stable-selector deployment and direct-owner gates · implement-standard · host either
- [ ] T-A11-BROKER-4 — Run broker transport and offline acceptance checks · implement-complex · host; Pi separately

## T-A11-BROKER-0 — Publish the broker migration contract and design

**Context budget:** ~8k tokens · **Depends:** none · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/config.yaml` · `openspec/changes/resmed-air11-ble-stream/{proposal,design,tasks}.md` · `openspec/changes/resmed-air11-operator-onboarding/{proposal,design,tasks}.md` · broker protocol/client design and current deployment inventory

**Creates:** this change's `proposal.md`, `design.md`, `specs/resmed-air11-broker-transport/spec.md`, and `tasks.md`

**Do:** Record the broker-only ownership boundary, stable adapter-selector contract, typed discovery/write API gap, bounded cancellation, unchanged credentials and read-only RPC semantics, separate discovery prerequisite, rollback rule, and static/Pi acceptance boundary. Do not include private addresses, credentials, raw captures, or machine-specific identifiers.

**Verify:** `openspec validate resmed-air11-broker-transport --type change --strict --no-interactive && git diff --check`

**Expect:** exit 0. Validation does not claim implementation, pairing, deployment, or live data.

## T-A11-BROKER-1 — Expose bounded discovery and write operations in the broker client

**Context budget:** ~10k tokens · **Depends:** T-A11-BROKER-0 · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-broker-transport/{proposal,design}.md` · broker `protocol.rs` · broker `client.rs` · broker `server.rs` · broker `bluez.rs` · broker protocol tests

**Modifies:** `crates/vitals-ble-broker/src/client.rs` and its protocol/client tests in the broker repository

**Do:** Add typed `discover_target_bounded` and `write_bounded` wrappers for the existing operation kinds. Correlate operation IDs, bound target/payload/deadline values, and cancel on timeout using the existing real abortable cancellation path. Keep capabilities explicit, reject undeclared operations, redact identifiers in errors, and do not add a generic D-Bus escape hatch. Add tests for success, broker rejection, timeout cancellation, bounded payloads, and worker release.

**Verify:** `cargo test -p vitals-ble-broker --lib && cargo test -p vitals-ble-broker --test protocol && cargo clippy -p vitals-ble-broker --all-targets --features linux -- -D warnings && git diff --check`

**Expect:** exit 0 in the broker repository. The client API is not accepted if a timed-out operation can retain a BlueZ future or lease.

## T-A11-BROKER-COV — Enforce 100% executable broker coverage

**Context budget:** ~10k tokens · **Depends:** T-A11-BROKER-1 · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-broker-transport/{proposal,design,specs/resmed-air11-broker-transport/spec}.md` · broker `Cargo.toml` · broker source and tests · CI workflow

**Modifies:** broker coverage harness, deterministic fake-owner/protocol tests, and CI coverage gate

**Do:** Add a Linux `cargo llvm-cov` gate for `vitals-ble-broker` with all features and targets. Require 100% executable line, function, and region coverage. Exercise every broker policy, protocol, client, server, capability, cancellation, cleanup, malformed-input, backpressure, generation, and Linux-owner decision path through deterministic tests. Do not exclude uncovered production code merely because it is difficult to exercise; add a test seam or remove dead code. Keep actual BlueZ/Pi acceptance separate from coverage.

**Verify:** `cargo llvm-cov --package vitals-ble-broker --all-features --all-targets --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100 --summary-only`

**Expect:** exit 0 on Linux/CI. Any uncovered executable line, function, or region is a failing task and blocks deployment.

## T-A11-BROKER-2 — Replace direct AirSense BlueZ transport with the broker lease

**Context budget:** ~18k tokens · **Depends:** T-A11-BROKER-COV · **Review required:** yes, including security and physiology/unit review

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-broker-transport/{proposal,design,specs/resmed-air11-broker-transport/spec}.md` · `openspec/changes/resmed-air11-ble-stream/{design,tasks}.md` · AirSense `as11_broker.rs` and `as11_common.rs` · AirSense collector binary · AirSense Cargo manifests · broker client/protocol types

**Modifies:** AirSense transport, collector wiring, Cargo manifests/lockfile, and transport tests

**Do:** Replace `bluer` session/device/characteristic and writer-task ownership with `BrokerClient` source handshake, bounded lease, exact UUID resolution, RX subscription, TX writes, broker event consumption, and release cleanup. Preserve application crypto, FIG reassembly, RPC allowlist, journal ordering, status, sample identity, and reconnect state. Remove the direct discovery fallback from the worker; if materialization is missing, call the typed bounded broker discovery operation under its authorized boundary. Never add a direct-BlueZ fallback or use `hciN` as identity.

**Verify:** `cargo fmt --all --check && cargo test -p vitals-collectors --all-features && cargo clippy -p vitals-collectors --all-targets --all-features -- -D warnings && cargo build --bin resmed-air11-collector --all-features && ! rg -n '(^|::)bluer|hci[0-9]+' crates/vitals-collectors/src/as11_broker.rs crates/vitals-collectors/src/as11_common.rs crates/vitals-collectors/src/bin/resmed_air11_collector.rs`

**Expect:** exit 0 in the integrated AirSense plus broker workspace. A host without Linux BlueZ may run portable tests only; it must not use a mock BlueZ implementation.

## T-A11-BROKER-3 — Update stable-selector deployment and direct-owner gates

**Context budget:** ~9k tokens · **Depends:** T-A11-BROKER-2 · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-broker-transport/{proposal,design,specs/resmed-air11-broker-transport/spec}.md` · `deploy/vitals-air11.service` · broker service/unit and selector configuration · existing AirSense public-information checks

**Modifies:** AirSense service/configuration, deployment documentation, and static checks

**Do:** Replace `--adapter hci0` with the broker's stable selector input. Add broker ordering/requirement without weakening the existing credential, memory, filesystem, vitalsd, and disabled-by-default policy. Add a static gate that rejects direct AirSense BlueZ imports and runtime aliases while allowing the broker implementation itself. Keep the service stopped/disabled in repository verification.

**Verify:** `python3 scripts/check-public-info.py --self-test && python3 scripts/check-air11-public.py --self-test && rg -q 'vitals-ble-broker' deploy/vitals-air11.service && ! rg -n -- '--adapter[= ]+hci[0-9]+|hci[0-9]+' deploy/vitals-air11.service crates/vitals-collectors/src/bin/resmed_air11_collector.rs && git diff --check`

**Expect:** exit 0. This task does not enable or start a unit and does not change `/boot/firmware`.

## T-A11-BROKER-4 — Run broker transport and offline acceptance checks

**Context budget:** ~12k tokens · **Depends:** T-A11-BROKER-3 · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · this change's artifacts · AirSense and broker tests · approved deployment units · existing AirSense acceptance script

**Modifies:** task checklist and dated evidence only after verification; no runtime changes during the host gate

**Do:** Run the exact Verify commands for T-A11-BROKER-1 through T-A11-BROKER-3, then the integrated workspace checks. Record host evidence and unresolved Pi prerequisites without claiming live measurements. A later Pi run must use the separate explicit acceptance command, verify the target is available, and never start the legacy direct-BlueZ owner alongside the broker.

**Verify:** `openspec validate resmed-air11-broker-transport --type change --strict --no-interactive && python3 scripts/check-public-info.py --self-test && python3 scripts/check-air11-public.py --self-test && cargo fmt --all --check && cargo test --workspace --all-features --no-fail-fast && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo llvm-cov --package vitals-ble-broker --all-features --all-targets --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100 --summary-only && git diff --check`

**Expect:** exit 0 for host acceptance. Pi service activation, bounded discovery against the physical ResMed, and live bedside data remain separate gates.
