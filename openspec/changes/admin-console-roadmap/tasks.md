# Tasks — admin-console-roadmap

The conductor ticks a box only after running that task's `Verify:` command and seeing exit 0.
`AGENTS.md` and `docs/specs/AMENDMENTS.md` are mandatory for every task. Each task is a separate
worktree/branch/commit/PR. `BLOCKED` tasks remain queued but must not be dispatched before their gate.

## Queue order

### M1 closeout

- [x] T-M1c — M1 behavioural acceptance and accessibility regression suite · implement-complex · review
- [ ] T-M1d — Live direction determination · operator-judgment · **BLOCKED on real-use evaluation**
- [ ] T-M1e — Prune losing console/dashboard directions · mechanical · **BLOCKED on T-M1d**

### M2 operations

- [x] T-M2a — Bind/auth posture and security middleware (S1–S5) · implement-complex · review
- [x] T-M2b — Authenticated allowlisted audited service operations · implement-complex · review
- [x] T-M2c — Service detail, sanitised logs, and crash-loop health · implement-complex · review
- [x] T-M2d — Host history and diagnostics · implement-standard · review
- [x] T-M2e — Silence detector daemon using the core evaluator · implement-complex · review
- [x] T-M2f — Outbound email configuration, sender, coalescing, and test loop · implement-complex · review
- [x] T-M2g — Operator overrides, optional-source declarations, and expiring mutes · implement-complex · review
- [x] T-M2h — Credentials, audit/config UI, and one-time key reveal · implement-complex · review
- [ ] T-M2i — Optional OpenTelemetry/OTLP exporter and test loop · implement-complex · review
- [ ] T-M2j — Services and Config screens in the surviving direction · implement-standard · **BLOCKED on T-M1e**
- [ ] T-M2k — M2 milestone acceptance suite · implement-complex · review

### M3 data integrity

- [ ] T-M3a — Resolve Timescale hypertable/identity contradiction by amendment · architecture-decision · **BLOCKED on operator judgment**
- [ ] T-M3b — Decide retention/compaction ownership and expose its contract · architecture-decision
- [ ] T-M3c — Targeted sample integrity query API with keyset pagination · implement-complex · **BLOCKED on T-M3a**
- [ ] T-M3d — Scoped JSON/CSV export with a real read-only DB role · implement-complex · **BLOCKED on T-M3a**
- [ ] T-M3e — Upload lifecycle, parser isolation, duplicate handling, and validation history · implement-complex
- [ ] T-M3f — Quarantine and data-loss ledger readers · implement-standard
- [ ] T-M3g — Versioned CPAP clock-transform metadata, preview, and rebuild contract · implement-complex · **BLOCKED on T-M3a**
- [ ] T-M3h — Decide whether a constrained SQL pane ships · operator-judgment · default no
- [ ] T-M3i — Data screen and M3 acceptance suite · implement-complex · **BLOCKED on T-M3c–T-M3g**

### M4 analysis

- [ ] T-M4a — Postgres-backed supervised job lifecycle with cancel/retry/recovery · implement-complex
- [ ] T-M4b — Port correlation statistics with gen-1 fixtures and correctness review · implement-complex
- [ ] T-M4c — Unified billed-call ledger, preflight cost, budget, and egress confirmation · implement-complex
- [ ] T-M4d — AI generate/review/accept versioning loop · implement-complex
- [ ] T-M4e — Clinical PDF report generation with correctness fixtures · implement-complex
- [ ] T-M4f — Analysis screen and M4 acceptance suite · implement-complex · **BLOCKED on T-M4a–T-M4e**

### M5 records and recovery

- [x] T-M5a — Decide domain-schema placement and record ownership · architecture-decision
- [x] T-M5b — Versioned domain schema and deliberate conflict resolution · implement-complex · **BLOCKED on T-M5a**
- [ ] T-M5c — Records CRUD, prompt preview, selection visibility, and forms · implement-complex
- [ ] T-M5d — Recovery history, restore evidence, and data-loss exposure · implement-complex
- [ ] T-M5e — Projection rebuild/reprovision contract and verification · implement-complex
- [ ] T-M5f — M5 acceptance suite · implement-complex · **BLOCKED on T-M5b–T-M5e**

### M6 alarm path — last

- [ ] T-M6a — Alarm daemon/status protocol contracts · architecture-decision · **BLOCKED on core alarm daemons**
- [ ] T-M6b — Local klaxon evidence and independent test operation · implement-complex
- [ ] T-M6c — Remote push evidence, token age, sweep, and test operation · implement-complex
- [ ] T-M6d — Alarm screen and M6 acceptance suite · implement-complex · **BLOCKED on T-M6a–T-M6c**

### Cross-plan obligations

- [ ] T-X1 — Decide phone-silence escalation path without a notification loop · architecture-decision
- [ ] T-X2 — Phone health-access/status beacon through the envelope · implement-complex
- [x] T-X3 — Platform-sample envelope class and sub-provenance contract · architecture-decision
- [x] T-X4 — HealthKit sample receiver and canonical mapper · implement-complex · **BLOCKED on T-X3**
- [ ] T-X5 — Historical backfill plan and verified one-time migration · implement-complex · **BLOCKED on T-M3a/T-X3**
- [ ] T-X6 — Gen-1 decommission capability ledger · review

---

## Task briefs

### T-M1c — M1 behavioural acceptance and accessibility regression suite

**Amends:** 07, 08.

**Reads:** `docs/design/SPEC-admin-console.md` §2.2–§2.6, §4.1–§4.4, §7 S7, §9 M1 and Every milestone · `docs/design/PROMPT-admin.md` Accessibility · `crates/vitals-console/assets/` · `crates/vitals-console/src/api/` · existing console tests.

**Do:** Add deterministic integration/browser-independent checks for M1 criteria 4–10 and applicable
criterion 19 regressions: reachable `FAILING` from a failed attempt; ages plus expectation; stale
page overlay retaining last-known outcome/reason; registry left-join; frozen high-water mark;
`NOT_CONFIGURED`; clock/build/schema identity. Add computed WCAG contrast checks for shipped token
pairs, semantic-control/label checks, no sub-14px text, visible focus, and reduced-motion handling.
No wall-clock assertions; fixtures use fixed times.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console && node crates/vitals-console/assets/accessibility-check.mjs && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M1d — Live direction determination

**Amends:** 08 §2.4 and §3.4.

**Reads:** Amendment 08 in full · `docs/design/SPEC-admin-console.md` §2.4, §3–§4 · all live-evaluation notes recorded by the operator.

**Do:** Operator uses all console and dashboard directions with live data, chooses one direction per
surface, and records date, chosen direction, evidence, rejected alternatives, and any required
follow-up in Amendment 08 and `AMENDMENTS.md`. This is judgment-gated; agents must not choose.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && rg -n 'Direction determination:.*(A|B|C).*Date:.*Evidence:' docs/specs/2026-08-06-amendment-08-layout-evaluation.md`

### T-M1e — Prune losing directions

**Amends:** 08.

**Reads:** Amendment 08 including T-M1d's recorded decision · `crates/vitals-console/assets/` · `docs/design/web-dashboard/index.html` · direction tests.

**Do:** Delete losing template/style/renderer sets and both runtime switchers; preserve the shared
view model and data seams. Update docs and tests to the recorded direction. Do not redesign.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console && node docs/design/web-dashboard/palette-check.mjs docs/design/web-dashboard/index.html && ! rg '\?dir=|render[ABC]|dir-[abc]\\.(css|js)' crates/vitals-console/assets docs/design/web-dashboard/index.html && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2a — Bind/auth posture and security middleware (S1–S5)

**Amends:** 07 and 09.

**Reads:** Amendment 09 in full · `docs/design/SPEC-admin-console.md` §4.9, §7 S1–S5/S7, §9c bind/auth decision · `docs/design/PROMPT-admin.md` pattern A · `crates/vitals-console/src/handlers.rs`, `main.rs`, `models.rs` · `deploy/vitals-console.service`.

**Do:** Implement Amendment 09's loopback axum bind, WireGuard-only admin origin, Caddy route
separation, single-operator Argon2id authentication, per-source throttling that cannot globally lock
out the operator, bounded sessions, visible transport state, recent-auth evidence, CSRF protection on
every state-changing browser request, and request-body caps applied before buffering or disk writes.
Secrets are never logged. Add one named test for each S1–S5 plus public-route/admin separation; do not
make the public client origin available until those tests pass.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console security_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2b — Authenticated allowlisted audited service operations

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §2.3, §4.3, §4.9, §5, §7 S1/S7, §8 non-goals, §9 M2 criterion 11 · `crates/vitals-console/src/api/ops.rs`, `services.rs`, `handlers.rs`, `db.rs` · `db/deploy/attempts.sql` · deployment unit manifests · T-M2a auth/CSRF interfaces.

**Do:** Add `POST /api/services/{unit}/{action}` for exactly `restart|stop`. Require authenticated
operator session, valid CSRF, and constant-time validation of `VITALS_CONSOLE_OPS_TOKEN` as a
second operation capability. Derive allowed units from installed/deployment manifests; reject path,
alias, template, and unknown unit names before invoking a command. No shell and no arbitrary
systemctl verbs. Write an attempt before execution (`kind=service_operation`, subject names
unit/action, fixed expected duration), finish it succeeded/failed/cancelled with sanitised detail and
effect, and write an immutable audit entry with actor, timestamp, target, action, confirmation class,
and attempt id. Every guard names the blocked action and value. High-consequence units require the
strong confirmation class supplied by T-M2a/UI; stopping the console itself is refused unless a
separate local-only maintenance contract is later specified. Inject the executor and test without
calling real systemd.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console service_operation_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2c — Service detail, sanitised logs, and crash-loop health

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §2.2, §2.5b, §4.3, §7 S7 · `crates/vitals-console/src/api/services.rs`, `ops.rs` · deployment manifests · core liveness evaluator.

**Do:** Expose manifest-derived units with process state separate from computed health, uptime,
restart count, memory, last error, and bounded journal lines filterable by unit/severity. Centralise
sanitisation and test token/password/URL/user-path redaction. A restart storm is `FAILING` with count
and threshold, not `LIVE`. Unknown command/parsing evidence is `UNKNOWN`, never empty success.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console services_ && cargo test -p vitals-console sanitize_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2d — Host history and diagnostics

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §4.3 · existing `api/host.rs`, `clock.rs`, `recovery.rs` · `SysOps` injection pattern.

**Do:** Add bounded history and diagnostics for NVMe free/wear plus projected time-to-full, thermal,
undervoltage, watchdog, network link/signal, and mains/UPS when present. Unsupported/unmeasured is
unknown/absent, never zero. Every threshold and freshness age is visible and tested with fixtures.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console host_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2e — Silence detector daemon using the core evaluator

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §2.2–§2.6, §3.1 M2, §8b transition logging/jitter/evidence · `crates/vitals-core/src/liveness.rs` · attempt/status schemas · registry cadence API · daemon unit conventions.

**Do:** Add an off-page supervised Rust process that loads registry/attempt evidence and calls
`vitals_core::liveness::evaluate` directly. It must not duplicate precedence or cadence. Persist
status observations with distinct reported/changed times; emit one transition on failure and one on
recovery; jitter repeats; honour declared-absent and unexpired operator mutes without relabelling
them healthy. Produce notification events for T-M2f, coalesced by transition identity. Fixed-time
unit/integration tests cover silent phone, stale collector, failing attempts, recovery, mute expiry,
and process restart.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console silence_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2f — Outbound email configuration, sender, coalescing, and test loop

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §4.9b, §7 S7/S8, §9 M2 criterion 13 · attempt schema · encryption-at-rest amendment 02 · T-M2e notification events.

**Do:** Store Gmail app password encrypted, write-only, presence/last-4 only. Fixed
`smtp.gmail.com:587` STARTTLS; configure from/destination. Record attempts before test/send. Add
rate-limited/coalesced digest behaviour and immediate meaningful transitions; failures degrade the
channel. Configured-never-tested is `UNKNOWN`. Email never blocks ingest or joins alarm independence.
Use an injected SMTP client and protocol-level test fixture; no live credentials in tests.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console email_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2g — Operator overrides, optional-source declarations, and expiring mutes

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §2.5b–§2.6, §4.9, §8b operator override rule · registry generated types · Sqitch conventions.

**Do:** Add Sqitch changes and Rust APIs for operator values beside immutable registry defaults,
optional-source `NOT_CONFIGURED` declarations, and actor/time-bounded mutes. Upgrades must not stomp
overrides; console never writes registry files. Expiry restores the underlying health automatically.
Audit all mutations.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console override_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && scripts/verify-sqitch-change.sh operator_overrides source_declarations health_mutes`

### T-M2h — Credentials, audit/config UI, and one-time key reveal

**Amends:** 07, 08 until T-M1e lands, and 09.

**Reads:** Amendment 09 enrollment/device lifecycle · `docs/design/SPEC-admin-console.md` §4.9, §5, §7 · `docs/design/PROMPT-admin.md` pattern A and forms/accessibility · `legacy-admin-reference.md` API keys/ResMed/settings rows · gen-1 `server/admin.py` key/settings routes and `server/templates/keys.html`, `resmed_settings.html`, `app_settings.html` · T-M2a/T-M2g APIs · envelope-client schema.

**Do:** Add credential lifecycle with plaintext enrollment-artifact reveal exactly once,
fingerprints/last-4 thereafter where applicable, and mobile device/key public-key fingerprints,
scopes, expiry, last-used, rotation, revocation, and audit evidence. Add the audit reader and validated
timezone/cadence/config forms with dirty-state save/discard plus transport-security state. Enrollment,
recovery, and revocation require Amendment 09 recent-auth evidence. Apply proportionate confirmations
and semantic accessible controls. Never return a stored token, private key, or credential plaintext.
Every response that contains newly minted plaintext uses `Cache-Control: no-store, max-age=0` and
`Pragma: no-cache`; test the headers.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console credential_ && node crates/vitals-console/assets/accessibility-check.mjs && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2i — Optional OpenTelemetry/OTLP exporter and test loop

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §4.9c, §7 S8, §9 M2 criterion 13 · Amendment 07 · metrics emission sites.

**Do:** Add vendor-neutral OpenTelemetry OTLP export, disabled by default. Permit only counters and
timings named by the spec; reject measurement values, words, and high-cardinality labels. Bounded
queue, drop-and-count under backpressure, never block critical paths. Store key encrypted and
write-only; region endpoint is US/EU enum. Record test attempt and export health; never-tested is
`UNKNOWN`, failures `FAILING`.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test --workspace telemetry_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2j — Services and Config screens

**Amends:** 08 as resolved by T-M1d/T-M1e.

**Reads:** chosen-direction decision · `docs/design/SPEC-admin-console.md` §4.3, §4.9–§4.9c, §5 · `docs/design/PROMPT-admin.md` screens 4–5/accessibility · M2 APIs.

**Do:** Implement Services and Config & access screens over direction-agnostic view models. Include
separate process/health axes, logs, host history, confirmations, credentials, email, telemetry,
overrides/mutes, audit, ages/expectations, and all empty/failing/unknown states. No client-side health
policy.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console && node crates/vitals-console/assets/accessibility-check.mjs && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M2k — M2 milestone acceptance suite

**Amends:** 07.

**Reads:** `docs/design/SPEC-admin-console.md` §9 M2 and Every milestone · all M2 implementation tests.

**Do:** Add one conductor-runnable suite proving criteria 11–13, S1–S5, accessibility, and applicable
gen-1 regressions against isolated fixtures/scratch DB. Include unauthenticated, CSRF, stale,
never-tested, failed-exporter, failed-email, and audit-before-action paths.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-admin-m2.sh`

### T-M3a — Resolve Timescale hypertable/identity contradiction

**Amends:** 01, 04, 05, 06.

**Reads:** amendments 01/04/05/06 in full · Sqitch sample/hypertable/CAGG changes · the two recorded M0b contradictions · PostgreSQL/Timescale uniqueness constraints.

**Do:** Measure the running schema and evaluate valid designs without adding timestamp to ingest
identity or permitting silent duplicate/drop behaviour. Write and obtain operator acceptance for an
amendment that supersedes the contradictory requirements, updates affected tasks/docs, and supplies
a migration/rollback/verification plan. No implementation before acceptance.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-amendment-index.sh && rg -n 'samples_hypertable.*resolved|Timescale.*identity.*decision' docs/specs/AMENDMENTS.md docs/specs/*amendment*.md`

### T-M3b — Decide retention/compaction ownership

**Amends:** whichever storage amendment T-M3a establishes.

**Reads:** design open question 2, §4.5, §8b statistics · storage amendments · existing retention scripts.

**Do:** Record whether the console observes, triggers, or only links to retention/compaction; define
freshness, attempts, safety guards, and audit. Update M3 tasks accordingly.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && rg -n 'Retention ownership decision: (observe|trigger|both)' docs/specs/*.md`

### T-M3c — Targeted sample integrity query API

**Amends:** 07 and T-M3a's accepted amendment.

**Reads:** design §4.5, §7 S6/S7 · `legacy-admin-reference.md` data-browser row · gen-1 `server/admin.py` `BROWSABLE_TABLES`/data route and `server/templates/data.html` · query API code · sample schema/CAGGs · registry types.

**Do:** Add source/date targeted integrity views and per-day cross-source view. Keyset pagination only;
never hypertable `COUNT(*)`, `OFFSET`, interpolation, or unbounded rows. Explicit window/completeness
metadata. NULL/zero/parse-failure remain distinct when provenance supports it. Typed generated
source/type identifiers at Rust boundaries.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console data_query_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M3d — Scoped JSON/CSV export with a real read-only DB role

**Amends:** 07.

**Reads:** design §4.5, §6 JSON/CSV migration item, §7 S6/S8 · export precedent in GEN1 reassessment · `legacy-admin-reference.md` data-browser row · T-M3c APIs.

**Do:** Create least-privilege read-only Postgres role with statement timeout; scoped source/date
JSON and CSV streaming export with explicit schema/version and completeness. Attempt/audit each
export; enforce body/output limits and cancellation. No generic arbitrary SQL endpoint.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console export_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check && scripts/verify-sqitch-change.sh console_readonly_role`

### T-M3e — Upload lifecycle and validation history

**Amends:** 07.

**Reads:** design §4.5, §5 pattern D, §7 S5/S7 · `legacy-admin-reference.md` ResMed/CPAP rows · gen-1 `server/admin.py` ResMed and CPAP routes, `server/templates/resmed_settings.html`, `cpap_upload.html`, `server/resmed_sync.py`, and `server/edf_parser.py` · Amendment 07 batch-parser carve-out · attempt schema.

**Do:** Add capped-before-disk uploads, content/format validation, isolated out-of-process Python
batch parser invocation, queued/running/succeeded/failed/cancelled lifecycle, accepted/rejected/replay
breakdown with row reasons, duplicate-file no-op behaviour, persistent history, and cancellation.
Remove uploaded PHI and parser scratch files after success, failure, and cancellation, with tests for
all three paths. Zero accepted rows is not silent success. Also provide the concrete operator-triggered
job for a configured myAir import, using an import-specific supervised child-process lifecycle with
attempt/history/sanitisation and cancellation; it does not depend on M4's later general analysis-job
runner. Recurring poller ownership remains with the collector plan.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console upload_ && cargo test -p vitals-console import_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M3f — Quarantine and data-loss ledger readers

**Amends:** 07.

**Reads:** design §4.5–§4.6, §6 data-loss migration item · quarantine/data-loss schemas from core plan.

**Do:** Add bounded, sanitised targeted readers with reason, provenance, freshness, and export links.
Absent schema/evidence is `UNKNOWN` with named blocker, not empty success. No mutation of samples.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console quarantine_ && cargo test -p vitals-console data_loss_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M3g — Versioned CPAP clock transforms

**Amends:** T-M3a's accepted amendment.

**Reads:** design §4.4 · GEN1 reassessment `ezshare_clock.py` verdict · sample immutability amendments · aggregate contracts.

**Do:** Add append-only transform metadata with version, derivation evidence, actor, and timestamps.
Read API shows raw device time and applied version. Preview computes moved nights and collisions
before minting a version. Samples are never updated. Any corrected-time materialization is keyed by
transform version and explicitly rebuilt/refreshed with attempt/audit evidence.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console cpap_transform_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M3h — Decide whether a constrained SQL pane ships

**Amends:** none unless decision changes scope.

**Reads:** design open question 4, §7 S6, §8 non-goals · T-M3c/T-M3d coverage.

**Do:** After targeted views exist, operator decides whether a SQL pane still has unique value.
Record `ship` or `do not ship` with evidence. If ship, create a separate task requiring parsed
single-statement SELECT-only input, read-only role, timeout, row cap, audit, and no secrets.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && rg -n 'SQL pane decision: (ship|do not ship)' docs/specs/*.md`

### T-M3i — Data screen and M3 acceptance suite

**Amends:** chosen UI direction and T-M3a.

**Reads:** design §4.5, §5 upload, §9 criteria 14–15/18–19 · M3 APIs.

**Do:** Implement targeted integrity, shared date cursor, explicit windowing, NULL semantics,
quarantine/loss, scoped export, upload lifecycle, and CPAP transform preview. Add deterministic M3
acceptance script.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-admin-m3.sh`

### T-M4a — Postgres-backed supervised job lifecycle

**Amends:** 07.

**Reads:** design §2.3, §2.5b, §4.7 · GEN1 reassessment job-dispatcher verdict · `legacy-admin-reference.md` analysis/job fixture notes · gen-1 `server/job_dispatcher.py` · attempt schema · systemd conventions.

**Do:** Add job schema/runner with queued/running/succeeded/failed/cancelled, lease/heartbeat,
expected duration, recovery after runner death, cancel/retry, parameters, outcome, and attempt links.
No HTTP-worker threads and no page-load reaper.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test --workspace job_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && scripts/verify-sqitch-change.sh jobs`

### T-M4b — Correlation statistics port

**Amends:** 07 batch-analysis carve-out.

**Reads:** design §4.7 and §6 undocumented correctness · GEN1 reassessment correlation verdict · gen-1 correlation implementation/fixtures only as explicitly listed in the dispatched task.

**Do:** Port minimum paired-day floor, p-value discipline, abnormal-vs-normal comparison, nil-AHI
exclusion, unfiltered baselines, and completeness notes. Fixed reference dataset must match gen-1
expected output within stated tolerances; never assert float equality. Independent physiological
accuracy review required.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-correlation-fixtures.sh && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M4c — Unified billed-call ledger and egress preflight

**Amends:** 07.

**Reads:** design §4.7, §7 S8, §9 criterion 16 · GEN1 reassessment AI accounting verdict · `legacy-admin-reference.md` patient/profile/analysis accounting rows · gen-1 `server/admin.py` generation routes and `server/analysis.py` model/cost metadata.

**Do:** Route every billed call through one ledger. Estimate model, tokens/data, cost, destination,
and health-data scope before submission; enforce configurable budget; require proportionate
confirmation; record actual cost/outcome and parse failures. No hardcoded hidden model.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console billed_call_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && scripts/verify-sqitch-change.sh billed_calls`

### T-M4d — AI generate/review/accept loop

**Amends:** 07.

**Reads:** design §4.7–§4.8, §5 pattern C · `PROMPT-admin.md` pattern C · `legacy-admin-reference.md` patient/psychiatrist/conflict rows · gen-1 `server/templates/patient_profile.html`, `psychiatrist_profile.html`, and `conflict_detail.html` · T-M4a/T-M4c.

**Do:** Persist prior and candidate versions; support generate/review side-by-side/edit/regenerate/
accept. Nothing overwrites until acceptance. Parse failure stays failed, never synthesized completed.
Attempt, audit, cost, and egress records link to each version.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console content_review_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M4e — Clinical PDF report generation

**Amends:** 07 batch-job carve-out.

**Reads:** design §4.7 and §6 PDF owner · GEN1 reassessment report verdict · gen-1 report section/layout code explicitly listed in dispatch · T-M4a.

**Do:** Generate clinical PDF on the Pi as a supervised job. Preserve section structure, nil-AHI
exclusion, unfiltered-baseline rule, and completeness note. Golden semantic fixtures verify content;
PDF metadata/version and source ranges are recorded. Independent physiological accuracy review.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-clinical-report-fixtures.sh`

### T-M4f — Analysis screen and M4 acceptance suite

**Amends:** chosen UI direction.

**Reads:** design §4.7, §5 patterns B/C, §9 criterion 16/18–19 · `legacy-admin-reference.md` analysis rows and fixture notes · gen-1 `server/admin.py` analysis routes, `server/templates/analysis.html`, `analysis_detail.html`, `server/job_dispatcher.py`, and `server/conflict_analysis.py` · all M4 APIs.

**Do:** Implement run launch/history, elapsed/cancel/retry, preflight cost/budget/egress, result and
AI-content review, and PDF download. Add deterministic acceptance script covering failed parse,
runner death, cancel, budget refusal, and successful report.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-admin-m4.sh`

### T-M5a — Decide domain-schema placement and record ownership

**Amends:** none unless architecture changes.

**Reads:** design open question 3, §4.8, §8 · GEN1 reassessment domain-schema and open-question rows · backup granularity requirements.

**Do:** Decide same DB vs separate DB/schema, and whether Records remains in the console or moves to
the health viewer. Preserve envelope identity/conflict requirements. Record decision and update M5
scope before implementation.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && rg -n 'Domain schema placement:|Records surface owner:' docs/specs/*.md`

### T-M5b — Versioned domain schema and conflict resolution

**Amends:** T-M5a decision.

**Reads:** design §2.3b, §4.8–§4.9, §5 · GEN1 domain shape explicitly inventoried in task dispatch · `legacy-admin-reference.md` profile/conflict/therapy/song rows · gen-1 `server/schema.sql` relevant domain tables and `server/admin.py` corresponding routes · existing domain_records/envelope code.

**Do:** Add relationships for journal, medications/doses/follow-ups, prescriptions, pharmacies,
profiles, therapy schedule/conflicts, labs, songs/tags while preserving client UUID identity. Changes
create versions; same-id/different-content remains a surfaced conflict until deliberate resolution.
Audit actor/time and link prompt-selection state. Preserve manually editable lyrics, but do not carry
forward the unowned Genius scraper; any future external lyrics fetch requires a separately reviewed
egress contract and task.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console domain_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && scripts/verify-sqitch-change.sh domain_schema domain_versions domain_conflicts`

### T-M5c — Records CRUD and prompt preview

**Amends:** chosen UI direction and T-M5a.

**Reads:** design §4.8, §5 forms/pattern C · `PROMPT-admin.md` screen 8 · `legacy-admin-reference.md` profile/conflict/therapy/song rows · gen-1 templates `patient_profile.html`, `psychiatrist_profile.html`, `conflicts.html`, `conflict_detail.html`, `therapy_schedule.html`, `songs.html`, and `song_detail.html` · T-M5b APIs.

**Do:** Add accessible validated forms, dirty-state save/discard, history, conflict resolution,
side-by-side AI review, assembled prompt preview, and explicit marker for the record feeding the next
run. No unversioned overwrite.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console records_ && node crates/vitals-console/assets/accessibility-check.mjs && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M5d — Recovery history and data-loss exposure

**Amends:** 07.

**Reads:** design §4.6, §6 recovery items, §9 criteria 14–15 · existing recovery API/script and restore verifier.

**Do:** Persist and expose full/incremental/WAL evidence, strict restore-verification history,
row-count comparisons, freshness, and a computed plain-language loss window. Never-verified or
malformed/stale evidence is `UNKNOWN`; failed is `FAILING`. Link data-loss ledger. No restore action
into a live cluster.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console recovery_ && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M5e — Projection rebuild/reprovision contract

**Amends:** relevant Stage 7 replication decision.

**Reads:** design §6 device/server reconciliation · GEN1 reassessment sync/restore verdicts · Stage 7 tasks/amendments.

**Do:** Specify and implement a fresh client's complete projection rebuild from the Pi using the
accepted cursor/order contract. Stable replay is idempotent; no empty-store guard or per-table
importer. Every fail-closed guard names its blocker. Test interruption and replay.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-projection-rebuild.sh`

### T-M5f — M5 acceptance suite

**Amends:** T-M5a and Stage 7 decision.

**Reads:** design §4.6/§4.8, §9 M3-and-later/every milestone · M5 APIs.

**Do:** Add end-to-end fixed fixtures for versioned CRUD/conflicts, prompt preview, restore history,
loss window, and projection rebuild interruption/replay.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-admin-m5.sh`

### T-M6a — Alarm daemon/status protocol contracts

**Amends:** 03, 05 and any later alarm amendments.

**Reads:** design §4.10, §7b missing-contract note, §9 criterion 17 · alarm amendments/core tasks · `vitals-proto`.

**Do:** Once alarm daemons exist, define typed Rust request/response contracts for channel state,
evidence age, dependencies, token/sweep/delivery facts, and test attempts. A channel never tested is
`UNKNOWN`. Do not fabricate state from service activity. Update protocol tests and affected docs.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test --workspace alarm_status_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-M6b — Local klaxon evidence and independent test operation

**Amends:** 03, 05.

**Reads:** design §4.10 · alarm amendments · accepted T-M6a protocol · klaxond interfaces.

**Do:** Expose last self-test result/age, audio identity, volume floor, amplifier/current evidence,
GPIO continuity, and dependency tier. Test operation is authenticated, confirmed, audited, marked
test, and recorded in the real ledger. ALSA write alone cannot pass. Never-tested/stale is UNKNOWN.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-local-klaxon-console.sh`

### T-M6c — Remote push evidence and test operation

**Amends:** accepted remote-alert architecture.

**Reads:** design §4.10 and §6 redundant-push migration item · T-M6a protocol · push/backstop implementation tasks.

**Do:** Expose configured state, token count/age, sweep cadence/result, delivered-alert ledger,
backstop and external timer evidence, and Critical Alerts entitlement. Independent test operation is
authenticated/audited and cannot count as a real alert. Never-delivered-and-never-tested is UNKNOWN.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-remote-alert-console.sh`

### T-M6d — Alarm screen and M6 acceptance suite

**Amends:** chosen UI direction.

**Reads:** design §4.10, §9 criterion 17/18–19 · `PROMPT-admin.md` product restraint · M6 APIs.

**Do:** Add the tenth normal section without making the console an alarm appliance. Show both
channels, evidence/freshness/dependency tier, and independent confirmed tests. Add acceptance suite
proving never-tested, stale, failed, and successful-test states.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-admin-m6.sh`

### T-X1 — Decide phone-silence escalation path

**Amends:** none until accepted.

**Reads:** design §2.3 and open question 1 · M2 email design · existing phone/push channel architecture.

**Do:** Operator chooses a non-looping escalation path and cadence/severity policy. Record why it is
independent enough and update T-M2e/T-M2f or remote-push tasks. Do not infer this decision.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && rg -n 'Phone-silence escalation decision:' docs/specs/*.md`

### T-X2 — Phone health-access/status beacon

**Amends:** relevant Stage 7 phone relationship decision.

**Reads:** design §4.4 and §6 phone health-access item · GEN1 reassessment HealthKit auth verdict · `docs/design/SPEC-ios-client.md` §3.5 and §4.3 · envelope API · registry/source health contracts.

**Do:** Define an idempotent phone status payload with per-type configured state, authorization-request
outcome, observer/background-delivery outcome, last query outcome, last matching sample,
last-successful Pi acknowledgement, outbox count/oldest age, app/registry build, and
reported/changed times. HealthKit read permission is not introspectable, so the payload must not
claim “granted.” Send through the authenticated envelope; store separately from physiological
samples; surface on Ingest using core health policy. Never coerce denial, no matching sample, query
failure, or missing beacon to healthy.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test --workspace phone_status_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-X3 — Platform-sample envelope class and sub-provenance contract

**Amends:** 01/02/05/06 as needed.

**Reads:** design §8b webhook envelope · GEN1 reassessment §2.7 in full · `docs/design/SPEC-ios-client.md` §4.2 · `clients/apple/GEN1-PARITY.yaml` when T-I0 lands · sample identity/provenance amendments · registry.

**Do:** Decide and document sub-provenance representation for platform store samples, canonical
sleep-stage mapping with unmappable behavior, stable session/seq identity, duplicate relationship to
direct sensors, and pure mapper boundary. Define how the generation-one HealthKit parity families
receive canonical registry entries or named exclusions; parity is source-sample coverage, not phone
snapshot/analysis parity. Update schema/registry/amendments before receiver code.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-amendment-index.sh && rg -n 'platform sample sub-provenance|sleep-stage mapping' docs/specs/*.md`

### T-X4 — HealthKit sample receiver and canonical mapper

**Amends:** T-X3.

**Reads:** accepted T-X3 contract · envelope API · registry generated types/ranges · decoder purity/testing rules.

**Do:** Add sample payload class, pure mapper to canonical generated types/units, strict finite/range
rejection, raw provenance, durable ingest identity, and quarantine. Cover every accepted
`proxy_sample`/`proxy_category` row from the generation-one parity ledger; every remainder has a named
registry blocker or exclusion. No HealthKit-required derived value and no ingestion of phone-computed
snapshots as measurements. Add fuzz target for byte/payload parsing and fixtures for
stages/subsources/replays.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test --workspace healthkit_ && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && cargo fuzz run healthkit_payload -- -max_total_time=30`

### T-X5 — Historical backfill and verified one-time migration

**Amends:** accepted T-M3a/T-X3 contracts.

**Reads:** GEN1 reassessment open question 4 · accepted schemas/mappers · backup/restore and attempt contracts.

**Do:** Inventory historical app-only oximetry, journal, medication, CPAP, and related domain data;
write resumable idempotent migration with source counts, hashes/high-water marks, rejects/quarantine,
and attempt evidence. Dry-run and restore-tested rollback first. No source deletion until T-X6.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-historical-backfill.sh --scratch --require-source-counts --require-replay-noop`

### T-X6 — Gen-1 decommission capability ledger

**Amends:** none.

**Reads:** design §6 in full · GEN1 reassessment in full · `legacy-admin-reference.md` in full · all merged roadmap tasks.

**Do:** For every migration-risk capability, record replacement task/commit, verification evidence,
intentional drop with reason, or explicit deferral owner. Include push, PDF, export, recompute,
timezone, schema/build identity, sanitisation, phone silence/access, reconciliation, undocumented
correctness rules, token revocation, key rotation, and DB read-only enforcement. Do not decommission
until no row is unowned.

**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-gen1-capability-ledger.sh`
