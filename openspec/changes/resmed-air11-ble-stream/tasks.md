# Tasks — read-only ResMed AirSense 11 BLE therapy streaming

The conductor ticks a box only after running that task's **Verify** command and
seeing exit 0. OpenSpec acceptance does not authorize pairing, deployment, or
activation on the Pi.

- [x] T-A11-0 — Publish the separate contract and correct the stale BLE handoff · documentation · host either
- [x] T-A11-1 — Freeze registry mappings and implement hostile-input FIG/StreamData decoders · implement-complex · host either
- [ ] T-A11-2 — Implement portable SRP/session crypto and the final read-only RPC gate · implement-complex · host either
- [ ] T-A11-3 — Build the Linux BlueZ transport, source identity, and sample builder · implement-complex · Linux/Pi for transport, either for tests
- [ ] T-A11-4 — Connect journaling, vitalsd append, reconnect supervision, and status · implement-complex · Linux/Pi for transport, either for tests
- [ ] T-A11-5 — Add the hardened service and offline/replay acceptance gates · implement-standard · host either
- [ ] T-A11-6 — Perform bounded Pi activation and live acceptance · implement-complex · **physical/governance gate**

**Amends:** `docs/specs/AMENDMENTS.md` is mandatory for every task. This change follows amendments 01, 02, 04, 05, 06, and 07; it does not amend any accepted decision.

---

## T-A11-0 — Publish the separate contract and correct the stale BLE handoff

**Context budget:** ~7k tokens · **Depends:** none · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `docs/integration/resmed_bluetooth_research.md` · `docs/integration/hms-cpap-ezshare-integration.md` · `openspec/config.yaml` · `openspec/changes/repository-recovery-and-lifecycle/reviews/openspec-authoring-guidance.md` · `openspec/changes/emay-durable-ingestion/{proposal,design,tasks}.md` · `openspec/changes/cpap-ezshare-ingest/{proposal,design,tasks}.md` · `vitals/registry.toml`

**Creates/updates:** the four OpenSpec artifacts under `openspec/changes/resmed-air11-ble-stream/` and the ResMed BLE research handoff only. Do not modify the EzShare change or its implementation.

**Do:** Record the empirical BLE correction, explicit non-goals, accepted storage/identity amendments, source/type contract, read-only RPC boundary, host-time policy, and physical deployment gate. Keep all captures, credentials, device addresses, serials, and raw identifiers out of Git. Cross-link the EzShare fallback without changing its semantics.

**Verify:** `openspec validate resmed-air11-ble-stream --type change --strict --no-interactive && python3 scripts/check-public-info.py --self-test && git diff --check`

**Expect:** exit 0; validation is structural evidence only and does not claim implementation or hardware acceptance.

---

## T-A11-1 — Freeze registry mappings and implement hostile-input FIG/StreamData decoders

**Context budget:** ~12k tokens · **Depends:** T-A11-0 · **Review required:** yes, including a physiology/unit review

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `vitals/registry.toml` · `vitals/tools/gen_registry.py` · `vitals/tools/tests/test_gen_registry.py` · `crates/vitals-registry/src/lib.rs` · `crates/vitals-decode/src/lib.rs` · `crates/vitals-decode/src/{bp,emay,hr,sfloat}.rs` · `$HOME/Source/airbreak-plus/docs/as11/{bluetooth_protocol,rpc_protocol,rpc_streams}.md` · `$HOME/Source/airbreak-plus/docs/{edf_signals,edf_str_signals}.md` · `$HOME/Source/aircannect/src/{as11_ble_fig,stream_frame,as11_stream_signals}.cpp` · `$HOME/Source/aircannect/include/{as11_ble_fig,stream_frame,as11_stream_signals}.h`

**Creates/updates:** registry TOML, generated artifacts via `vitals/tools/gen_registry.py`, portable decoder modules, sanitized fixtures, decoder tests, and one fuzz target per new decoder. No BlueZ, SQL, systemd, socket, or PostgreSQL work.

**Do:** Add permanent AirSense source/type entries without reusing ids. Implement no-std, no-allocation, `Option`-returning FIG and JSON StreamData decoders with bounded arrays, exact CRC/length checks, short/long stream-id mapping, explicit null validity, and no truncation. Add tests for arbitrary fragment boundaries, multiple packets, CRC/truncation/oversize, malformed JSON, values, aliases, nulls, unknown ids, and measured zero. Read the registry for all conversion/range checks; do not duplicate a range literal in decoder code.

**Verify:** `python3 vitals/tools/gen_registry.py --check && cargo fmt --all --check && cargo test -p vitals-registry -p vitals-decode --all-features && cargo clippy -p vitals-registry -p vitals-decode --all-targets --all-features -- -D warnings && cargo check --manifest-path fuzz/Cargo.toml`

**Expect:** exit 0; every byte parser is panic-free and fuzz-buildable on the host. Do not tick this task from a source report or from a test command that was not run.

---

## T-A11-2 — Implement portable SRP/session crypto and the final read-only RPC gate

**Context budget:** ~12k tokens · **Depends:** T-A11-1 · **Review required:** yes, security review

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-ble-stream/{proposal,design}.md` · `$HOME/Source/airbreak-plus/docs/as11/{bluetooth_protocol,rpc_protocol}.md` · `$HOME/Source/aircannect/src/{as11_ble_srp,as11_ble_crypto,as11_ble_rpc_link}.cpp` · `$HOME/Source/aircannect/include/{as11_ble_srp,as11_ble_crypto,as11_ble_rpc_link}.h` · `crates/vitals-decode/Cargo.toml` · `crates/vitals-collectors/Cargo.toml` · `crates/vitals-collectors/src/lib.rs`

**Creates/updates:** bounded portable crypto/protocol code, deterministic public test vectors with synthetic keys, JSON-RPC request/response handling, and tests that record the final send boundary. No live BLE writes and no credentials.

**Do:** Reproduce the fixed-width SRP proof formula, HMAC challenge response, SHA-256 nonce derivation, AES-256-CBC zero-padding envelope, and strict JSON-RPC correlation. Keep secrets out of `Debug`, errors, fixtures, and logs. Make the allowlist enforced at the final send function, reject all control/OTA/pairing methods in production, and test that forbidden methods cause zero transport writes. Use fixed buffers or bounded containers in radio-input parsing and reject oversize/invalid UTF-8/padding.

**Verify:** `cargo fmt --all --check && cargo test -p vitals-decode -p vitals-collectors --all-features && cargo clippy -p vitals-decode -p vitals-collectors --all-targets --all-features -- -D warnings && cargo check --manifest-path fuzz/Cargo.toml`

**Expect:** exit 0; synthetic protocol vectors pass, malformed envelopes fail closed, and the allowlist test observes no forbidden GATT write.

---

## T-A11-3 — Build the Linux BlueZ transport, source identity, and sample builder

**Context budget:** ~14k tokens · **Depends:** T-A11-2 · **Review required:** yes, including a physiology/unit review

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-ble-stream/{proposal,design}.md` · `vitals/registry.toml` · `crates/vitals-core/src/session.rs` · `crates/vitals-core/src/lib.rs` · `crates/vitals-decode/src/lib.rs` · `crates/vitals-collectors/src/{lib,h10_reconnect}.rs` · `crates/vitals-collectors/Cargo.toml` · `crates/vitals-proto/src/lib.rs` · `deploy/vitalsd.service` · `deploy/vitals-h10.service` · `$HOME/Source/aircannect/{include/as11_ble_rpc_link.h,src/as11_ble_rpc_link.cpp}`

**Creates/updates:** portable Air11 configuration/normalization and sample-building modules, the Linux `resmed-air11-collector` binary, bounded BlueZ GATT transport, and transport/build tests. Do not pair, scan indefinitely, or touch system overlays/network routes.

**Do:** Require the external credential file and normalize only MAC-shaped six-octet forms to `AA:BB:CC:DD:EE:FF`; reject opaque CoreBluetooth UUIDs without an explicit Linux mapping. Connect to the exact service/chars, wait with deadlines for resolution/notifications, write using negotiated MTU, authenticate, and create a fresh persistent session after authentication. Derive source ordinals from monotonic device `startTime`/sample positions, preserve host receipt timestamps and raw clock metadata, use generated enums, and enforce all registry ranges/conversions. A bad source position must quarantine rather than fall back to `Session::next_seq()`.

**Verify:** `cargo fmt --all --check && cargo test -p vitals-collectors --all-features && cargo clippy -p vitals-collectors --all-targets --all-features -- -D warnings && cargo build --bin resmed-air11-collector --all-features`

**Expect:** exit 0 on a host for portable logic; the Linux binary must compile in the Linux CI/Pi environment. A macOS host stub may exit 64 and must not mock BlueZ.

---

## T-A11-4 — Connect journaling, vitalsd append, reconnect supervision, and status

**Context budget:** ~14k tokens · **Depends:** T-A11-3 · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-ble-stream/{proposal,design}.md` · `crates/vitals-collectors/src/bin/resmed_air11_collector.rs` · `crates/vitals-core/src/{lib,session,spool}.rs` · `crates/vitals-proto/src/lib.rs` · `crates/vitalsd/src/{main,state}.rs` · `crates/vitals-collectors/src/bin/{h10_collector,bp_collector}.rs` · `deploy/vitalsd.service` · `deploy/vitals-h10.service`

**Creates/updates:** the Air11 raw journal/status implementation, vitalsd client, reconnect loop, bounded backoff/deadlines, replay tests, and any narrowly required vitalsd health seam. No direct PostgreSQL access and no schema/migration change.

**Do:** Journal each bounded StreamData envelope before semantic decode without auth material; rotate and fsync on a finite policy. Publish validated batches with `Request::Append { sync: false }`, validate the response, and preserve durability-unknown state on timeout. Reconnect by re-authenticating and reissuing the fixed stream with a new session; expose counts for null/unavailable/quarantined/dropped/retry states. Atomically publish status with no credentials, serials, full MAC, or session keys. Ensure H10, EMAY, vitalsd, and the EzShare path remain untouched in behavior.

**Verify:** `cargo fmt --all --check && cargo test --workspace --all-features --no-fail-fast && cargo clippy --workspace --all-targets --all-features -- -D warnings && git diff --check`

**Expect:** exit 0; offline tests prove append framing, journal interruption/recovery, replay identity, reconnect re-subscription, and named degraded states. PostgreSQL acceptance is separate.

---

## T-A11-5 — Add the hardened service and offline/replay acceptance gates

**Context budget:** ~9k tokens · **Depends:** T-A11-4 · **Review required:** yes

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-ble-stream/{proposal,design,specs/resmed-air11-ble-stream/spec}.md` · `deploy/vitalsd.service` · `deploy/vitals-h10.service` · `deploy/vitals-bp.service` · `.github/workflows/ci.yml` · `scripts/check-public-info.py`

**Creates/updates:** `deploy/vitals-air11.service`, offline replay/fixture gates, public-information/provenance checks, and service static tests. Do not enable the unit or alter `/boot/firmware`.

**Do:** Constrain the service to BlueZ/D-Bus, the vitalsd socket, external credential loading, state, and status; retain OOM protection, bounded resources, `NoNewPrivileges`, `ProtectSystem`, and explicit restart behavior. Ensure credentials are loaded privately from the operator-managed `~/.air11` path without an environment file or argv secret. Add CI checks that fail on raw captures, identifiers, credentials, copied AirCANnect implementation, missing fuzz targets, registry drift, or an allowlist regression.

**Verify:** `python3 scripts/check-public-info.py --self-test && python3 vitals/tools/gen_registry.py --check && cargo fmt --all --check && cargo test --workspace --all-features --no-fail-fast && cargo clippy --workspace --all-targets --all-features -- -D warnings && git diff --check`

**Expect:** exit 0; static service/replay checks pass without hardware. This task does not claim that the service is enabled or that the Pi has credentials.

---

## T-A11-6 — Perform bounded Pi activation and live acceptance

**Context budget:** ~8k tokens · **Depends:** T-A11-5 · **Review required:** yes · **Gate:** explicit operator authorization and physical/Pi host required

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-ble-stream/{proposal,design,tasks}.md` · approved service file · collector status implementation · `docs/integration/resmed_bluetooth_research.md` · `docs/integration/hms-cpap-ezshare-integration.md`

**Do:** Stop before any action that pairs, enables, scans, changes an adapter, changes an overlay, reboots, changes the EzShare route, or activates the service. After the operator explicitly authorizes the exact bounded run, verify an existing BlueZ bond/IRK and the operator-managed credential file, then run the collector for a finite window. Prove authentication, fixed stream subscription, host timestamp/clock metadata, null preservation, source/type rows through vitalsd and PostgreSQL, reconnect/restart replay behavior, and that H10/EMAY/EzShare remain healthy. Record unknown/unavailable and every failed gate; do not call unknown evidence success.

**Verify:** `scripts/air11-pi-acceptance.sh --duration 60 --require-postgres --require-h10 --require-emay --require-ezshare-unchanged`

**Expect:** exit 0 only on the Pi after explicit authorization. The command must fail closed when credentials, bond, source rows, host-time metadata, database append, reconnect evidence, or unaffected-source evidence is missing; no chat report substitutes for its exit code.
