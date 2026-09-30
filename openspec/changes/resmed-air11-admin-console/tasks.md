# Tasks — ResMed AirSense 11 admin and kiosk console

The conductor ticks a box only after running that task's **Verify** command and
seeing exit 0. This change does not authorize credential provisioning, service
activation, pairing, or physical Pi acceptance.

- [x] T-A11-ADMIN-1 — Ship the safe Rust admin projection, private-LAN monitor, and kiosk · implement-complex · host either

**Amends:** `docs/specs/AMENDMENTS.md` is mandatory for every task. This change
follows amendments 07 and 08. Its private-LAN deployment shape is proposed and
must be reviewed against amendment 09 before production acceptance; it does not
change the mobile public API or permit public exposure.

## T-A11-ADMIN-1 — Ship the safe Rust admin projection, private-LAN monitor, and kiosk

**Context budget:** ~18k tokens · **Depends:** the read-only AirSense collector change · **Review required:** yes, including security and UI review

**Reads:** `AGENTS.md` · `docs/specs/AMENDMENTS.md` · `docs/specs/2026-08-05-amendment-07-api-in-rust.md` · `docs/specs/2026-08-06-amendment-08-layout-evaluation.md` · `docs/specs/2026-08-30-amendment-09-mobile-enrollment-and-tls.md` · `openspec/changes/resmed-air11-admin-console/{proposal,design}.md` · `openspec/changes/resmed-air11-admin-console/specs/resmed-air11-admin-console/spec.md` · `crates/vitals-console/src/{api/air11,api/mod,api/postgres,handlers,lib,main}.rs` · `crates/vitals-console/assets/{console,index,dir-a,dir-b,dir-c}.js` · `crates/vitals-console/assets/{air11,air11-kiosk}.css` · `crates/vitals-console/assets/air11-kiosk.html` · `deploy/vitals-console.service` · `deploy/vitals-console-air11.rules` · `db/{deploy,revert,verify}/air11_admin.sql` · `db/sqitch.plan` · `scripts/check-air11-public.py` · `scripts/check-console-assets.py` · `.github/workflows/ci.yml` · `docs/integration/resmed-air11-ble-stream.md` · `docs/integration/resmed-air11-user-guide.md` · `docs/design/web-dashboard/index.html`

**Creates/updates:** the Rust AirSense projection and fixed-action boundary,
non-secret config/audit migration, LAN policy and service confinement, all three
admin renderings, the read-only kiosk, dashboard link, asset/public-information
checks, OpenSpec artifacts, and operator documentation. Do not add direct
PostgreSQL writes to the collector, expose credentials, add arbitrary commands
or RPCs, alter H10/EMAY/EzShare behavior, or run physical acceptance.

**Do:** Keep status, measurement, process, credential, configuration, logs, and
audit states separate and fail closed. Read runtime data only through bounded
server-side projections. Keep AirSense credentials outside PostgreSQL and the
browser, using optional systemd-loaded credential isolation under `ProtectHome`.
Require the fixed policy flag, per-process action token, separate console-admin
capability, and exact confirmation before mutations. Audit before and after
fixed service actions, report uncertain outcomes, promote important errors, and
retain stale last-known kiosk content without relabeling it healthy. Render one
view-model through Service Mode, Preflight, and Flowpath and keep machine state
out of physiology tiles.

**Verify:** `openspec validate resmed-air11-admin-console --type change --strict --no-interactive && python3 scripts/check-public-info.py --self-test && python3 scripts/check-air11-public.py --self-test && python3 scripts/check-console-assets.py --self-test && python3 vitals/tools/gen_registry.py --check && cargo fmt --all --check && cargo test --workspace --all-features --no-fail-fast && cargo clippy --workspace --all-targets --all-features -- -D warnings && git diff --check`

**Expect:** exit 0. This proves host-safe compilation, tests, static confinement,
asset syntax, migration structure, and public-information hygiene only. It does
not claim credentials, bonds, service activation, route changes, or live Pi
measurements.
