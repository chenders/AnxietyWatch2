# Tasks — operator-authorized AirSense 11 onboarding, pairing, and discovery

The conductor ticks a box only after running the task's `Verify` command and
seeing exit 0. This change is **Accepted** by the owner on 2026-09-11. It does
not authorize collector activation or live acceptance until the implementation
task passes its Verify command.

**Amends:** This accepted change supersedes the blanket no-pair/no-scan
physical wording in `resmed-air11-ble-stream` T-A11-3/T-A11-6 and the linked
operator documentation. It does not amend the production read-only RPC
allowlist or the accepted storage, provenance, and source-isolation decisions.

- [x] T-A11-ONBOARD-1 — Implement separate bounded discovery and required first-pair onboarding when credentials are absent · implement-complex · Linux/Pi

## T-A11-ONBOARD-1 — Implement separate bounded discovery and required first-pair onboarding when credentials are absent

**Context budget:** ~12k tokens · **Depends:** `resmed-air11-ble-stream` T-A11-5 · **Review required:** yes, including security and physical-operation review

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `openspec/changes/resmed-air11-operator-onboarding/{proposal,design}.md` · `openspec/changes/resmed-air11-ble-stream/{proposal,design,tasks}.md` · `openspec/changes/resmed-air11-ble-stream/specs/resmed-air11-ble-stream/spec.md` · `docs/integration/resmed_bluetooth_research.md` · `docs/integration/resmed-air11-ble-stream.md` · `docs/integration/resmed-air11-user-guide.md` · `crates/vitals-collectors/src/{as11_onboarding_bluez,as11_config,as11_crypto}.rs` · `crates/vitals-collectors/src/bin/resmed_air11_collector.rs` · `scripts/air11-pi-acceptance.sh` · `deploy/vitals-air11.service`

**Creates/updates:** a separate operator onboarding command/workflow, bounded BlueZ discovery, required explicit AirSense application pairing when credentials are absent, private credential mapping updates, synthetic tests, and synchronized operator documentation. Keep the production collector's final read-only RPC boundary unchanged. Do not enable services, modify adapters/routes/firmware, or alter H10/EMAY/EzShare behavior.

**Do:** implement the proposal's finite confirmation, FD56-filtered discovery, unambiguous target selection, cleanup restoration, passkey-gated first-pair flow when credentials are absent, atomic private credential update, secret redaction, and negative tests. Keep all physical actions outside host-only tests. Update every superseded no-pair/no-scan comment and operator document only after this proposal is approved.

**Verify:** `openspec validate resmed-air11-operator-onboarding --type change --strict --no-interactive && python3 scripts/check-public-info.py --self-test && python3 scripts/check-air11-public.py --self-test && cargo fmt --all --check && cargo test --workspace --all-features --no-fail-fast && cargo clippy --workspace --all-targets --all-features -- -D warnings && git diff --check`

**Expect:** exit 0. Host verification proves the bounded seams and retained read-only collector boundary only. A separate authorized Pi run is required for live discovery, any first pairing, and subsequent AirSense acceptance.
