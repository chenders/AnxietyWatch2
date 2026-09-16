# Tasks — ios-client-stage7

The conductor ticks a box only after running that task's `Verify:` command and seeing exit 0.
`AGENTS.md` and `docs/specs/AMENDMENTS.md` are mandatory for every task. Each task gets its own
worktree, branch, commit, review, PR, and conductor-run verification. `BLOCKED` tasks are not
dispatched before their named gate.

## Queue

### Contracts and foundation

- [x] T-I0 — Generation-one capability and HealthKit parity ledger · review
- [x] T-I1 — Production mobile enrollment/authentication threat model and amendment · architecture-decision
- [x] T-I2 — Versioned client API, OpenAPI, and shared contract fixtures · implement-complex
- [x] T-I3 — Apple project and pure-package skeleton · implement-standard

### Reliable temporary delivery

- [x] T-I4 — Secure enrollment, Keychain credential lifecycle, and endpoint trust UI · implement-complex · **BLOCKED on T-I1**
- [x] T-I5 — Temporary durable outbox and terminal-ack deletion · implement-complex
- [ ] T-I6 — Registry-driven platform payload and sub-provenance client mapper · implement-complex · **BLOCKED on G-IOS-PROVENANCE / admin T-X3**
- [x] T-I7 — Immediate drain engine, retry taxonomy, rotation, and sync diagnostics · implement-complex · **BLOCKED on T-I2/T-I4/T-I5**

### HealthKit proxy

- [ ] T-I8 — HealthKit authorization, per-type anchored queries, and parity types · implement-complex · **BLOCKED on T-I0/T-I6 and server T-X4**
- [ ] T-I9 — Observer/background delivery and phone status beacon · implement-complex · **BLOCKED on T-I7/T-I8 and admin T-X1/T-X2**

### Viewer

- [x] T-I10 — Pi read client, expiring projection cache, and unknown/stale model · implement-complex · **BLOCKED on T-I2**
- [ ] T-I11 — Today, history, report, health-access, and diagnostics surfaces · implement-complex · **BLOCKED on T-I9/T-I10 and required M3/M4 APIs**

### Subjective input

- [x] T-I12 — Domain command/version/tombstone client contract · implement-complex · **BLOCKED on G-IOS-RECORDS / admin T-M5a/T-M5b**
- [x] T-I13 — Journal rating, notes, tags, songs, and Express Mode · implement-complex · **BLOCKED on T-I5/T-I7/T-I12**
- [ ] T-I14 — Medication doses and dose-triggered check-ins · implement-complex · **BLOCKED on T-I12 and server medication policy**
- [ ] T-I15 — Random check-ins and notification-outcome evidence · implement-standard · **BLOCKED on T-I12**
- [ ] T-I16 — watchOS Quick Log durable relay · implement-complex · **BLOCKED on T-I13**

### Hardening and transition

- [x] T-I17 — Mobile security, privacy, accessibility, and release checks · review
- [ ] T-I18 — Physical-device background delivery, locked-state, energy, and panic-path acceptance · device-validation · **BLOCKED on T-I9/T-I13/T-I16**
- [ ] T-I19 — Verified generation-one data migration · implement-complex · **BLOCKED on admin T-X5 and T-I8/T-I12**
- [ ] T-I20 — End-to-end Stage 7 acceptance and beta rollout · review · **BLOCKED on T-I11–T-I19**
- [ ] T-I21 — Generation-one decommission ledger and deletion of obsolete services · review · **BLOCKED on T-I20 and admin T-X6**

---

## Task briefs

### T-I0 — Generation-one capability and HealthKit parity ledger

**Amends:** none; inventories inputs to Stage 7 and prevents silent feature loss.
**Context budget:** 10 files / 18k tokens.
**Reads:** `docs/design/SPEC-ios-client.md` §1–§4 and §11 · `docs/design/GEN1-REASSESSMENT.md` §2.2, §2.5–§2.7 · generation-one `AnxietyWatch/Services/HealthKitManager.swift`, `Utilities/SampleCaptureRegistry.swift`, `Utilities/SampleTypeConfig.swift`, `Models/HealthSnapshot.swift`, app/watch navigation roots, and feature documentation.
**Do:** Create `clients/apple/GEN1-PARITY.yaml` and a human-readable report. Inventory every retained
input/view capability and every HealthKit type found in authorization, observer, mirror, snapshot,
and UI sets. Classify each HealthKit type as `proxy_sample`, `proxy_category`, `context_only`,
`blocked_registry`, or `excluded`, with evidence path and accepted owner/reason. Explicitly separate
source samples from generation-one aggregates. Inventory journal rating/notes/tags/song, Express
Mode, medication doses, check-ins, watch Quick Log, viewer/report surfaces, and every capability to
remove (BLE, analysis, snapshots, repair/restore). Capture sanitized fixtures for later tests; no
personal data.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && python3 scripts/verify-ios-parity.py clients/apple/GEN1-PARITY.yaml && git diff --check`

### T-I1 — Production mobile enrollment/authentication threat model and amendment

**Amends:** 02 and 07 through accepted Amendment 09; resolves G-IOS-AUTH with admin T-M2a.
**Context budget:** 12 files / 20k tokens.
**Reads:** Amendment 02 threat model/key custody and Amendment 07 in full · `SPEC-ios-client.md` §6–§7 · `SPEC-admin-console.md` §7 and §9c · existing envelope registration/encryption handlers/schema · T-M2a brief.
**Do:** With operator approval, add an accepted amendment defining production bind/TLS topology,
one-time enrollment, server identity presentation, per-device credential type/scope, request replay
protection, expiry/revocation/rotation, recovery, and whether certificate pinning is justified. It
must explicitly retire localhost plaintext registration as a production flow, use platform/vetted
cryptography, preserve queued records across credential rotation, and map to MASVS-AUTH-1/2/3 and
MASVS-NETWORK-1/2. Update amendment index and both task plans without implementing the protocol.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-amendment-index.sh && python3 scripts/verify-ios-client-plan.py && rg -n 'Enrollment decision:|Replay protection:|Credential rotation:|TLS trust:' docs/specs/*amendment*.md`

### T-I2 — Versioned client API, OpenAPI, and shared contract fixtures

**Amends:** 07 and 09.
**Context budget:** 15 files / 22k tokens.
**Reads:** Amendments 07 and 09 in full · `SPEC-ios-client.md` §4 and §6 · `crates/vitals-console/src/main.rs`, `models.rs`, relevant handlers, and Cargo manifest · admin roadmap T-X2–T-X4 and M5 read/write contracts.
**Do:** Add explicit `/api/v1/client/*` routing and generated OpenAPI for enrollment placeholders,
registry discovery, item-level write acknowledgements, health beacon, projections, and API/version
errors. Only implemented operations advertise success; gated operations return stable named
unavailable errors. Commit representative request/response/error fixtures consumed by Rust and Swift.
Unknown fields remain forward-compatible; incompatible major versions preserve writes. OpenAPI must
not generate physiological enum constants—the registry remains JSON.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && cargo test -p vitals-console client_contract_ && scripts/verify-openapi.sh && cargo clippy -p vitals-console --all-targets -- -D warnings && cargo fmt --all -- --check`

### T-I3 — Apple project and pure-package skeleton

**Amends:** none.
**Context budget:** 12 files / 16k tokens.
**Reads:** `SPEC-ios-client.md` §1–§2, §7–§8, §12 · this change's `design.md` §2 · T-I0 parity report · generation-one project settings and app/watch roots only.
**Do:** Create `clients/apple/AnxietyWatch.xcodeproj`, iPhone and watch targets, and pure packages
`AWClientCore`, `AWTransport`, `AWOutbox`, `AWHealthKit`, and `AWDomain`. Record minimum OS/device
support from actual owner hardware; do not raise it silently. Import only selected reusable UI/value
code with source provenance. Add CI/test schemes, deterministic build settings, privacy manifest,
empty entitlement allowlist, dependency lock, and a no-personal-data fixture bundle. No BLE, local
analysis, local report, or legacy sync target may compile.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-project.sh && swift test --package-path clients/apple/Packages/AWClientCore && scripts/scan-public-info.sh`

### T-I4 — Secure enrollment, Keychain credential lifecycle, and endpoint trust UI

**Amends:** 09.
**Context budget:** 14 files / 20k tokens.
**Reads:** Amendment 09 in full · `SPEC-ios-client.md` §6–§7 · T-I2 OpenAPI/fixtures · project Keychain/transport seams · console credential APIs.
**Do:** Implement one-time enrollment and endpoint confirmation, per-device Keychain storage with the
accepted `ThisDeviceOnly` accessibility, credential status/rotation/revocation, and explicit TLS
state. Separate optional foreground privacy lock from the background credential. Reject embedded
secrets, cleartext production endpoints, trust-all delegates, and incompatible API versions. Rotation
must atomically rebind pending outbox items without re-encryption data loss. Add onboarding and
recovery UI with named blockers.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-auth.sh && scripts/verify-ios-network-security.sh && scripts/verify-ios-tests.sh AWTransportTests AWEnrollmentUITests`

### T-I5 — Temporary durable outbox and terminal-ack deletion

**Amends:** 02 local-data handling and 09 credential rotation/recovery.
**Context budget:** 12 files / 20k tokens.
**Reads:** Amendment 09 credential/rotation profile · `SPEC-ios-client.md` §4.1, §5.1, §7.1, §10 · `design.md` §3–§4 · domain envelope idempotency tests/schema · project persistence seams.
**Do:** Implement one local transaction for capture+enqueue, one drain owner, immutable payloads,
item IDs, classified outcomes, and atomic payload deletion on terminal acknowledgement. Outbox
payloads and item IDs are credential-neutral and receive the active key/signature only at send time;
rotation, revocation, and recovery never rewrite or drop them. Retained receipts may contain only the
allowed non-content fields and expire. Unacknowledged/rejected/conflict items are never age-evicted.
Add Data Protection, backup exclusion, explicit reset/logout loss guards, crash-point tests, and
storage-pressure behavior that preserves writes. Expose no history query over the outbox.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWOutboxTests && scripts/verify-ios-storage-security.sh && scripts/verify-ios-no-local-archive.sh`

### T-I6 — Registry-driven platform payload and sub-provenance client mapper

**Amends:** the accepted platform-provenance decision from admin T-X3.
**Context budget:** 14 files / 22k tokens.
**Reads:** accepted T-X3 amendment/contract · `SPEC-ios-client.md` §2 decisions 6–7 and §4.2 · registry JSON and generator tests · T-I0 parity ledger · T-I2 fixtures · generation-one device provenance implementation/tests.
**Do:** Implement registry fetch/cache/version validation and a pure HealthKit-to-envelope transport
mapper. Preserve HealthKit UUID, original identifier/unit/value, start/end, source revision, device,
allowlisted metadata, and batch identity. Do not mint server `(session_id, seq)`, convert to canonical
units, hardcode numeric IDs/ranges, or discard sub-provenance. Unknown/unsupported/malformed values
are rejected with named reasons and remain diagnosable, never coerced. Add parity-ledger-driven
mapping fixtures shared with the server.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWRegistryTests AWHealthKitMapperTests && scripts/verify-platform-fixtures.sh && ! rg 'source(Id)?\s*=\s*[0-9]|type(Id)?\s*=\s*[0-9]' clients/apple --glob '*.swift'`

### T-I7 — Immediate drain engine, retry taxonomy, rotation, and sync diagnostics

**Amends:** 09.
**Context budget:** 14 files / 22k tokens.
**Reads:** Amendment 09 request/rotation profile · `SPEC-ios-client.md` §5–§6 and §10 · T-I2 API fixtures · T-I4 credential interface · T-I5 outbox interface · core reconnect/backoff tests for jitter precedent.
**Do:** Drain immediately on enqueue when execution/network permits; also trigger on foreground,
network recovery, app activation, and explicit Sync Now. Send bounded batches with item-level ACKs.
Implement cancellation-safe single-owner draining, retryable/nonretryable taxonomy, bounded jittered
backoff, `Retry-After`, request/response limits, and credential rotation. Persist attempt evidence and
show pending count/oldest age/last ACK/sanitized reason. A no-op replay is success but not progress.
No timer/poll/socket keeps the app alive.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWSyncEngineTests AWTransportFaultTests && scripts/verify-ios-no-polling.sh`

### T-I8 — HealthKit authorization, per-type anchored queries, and parity types

**Amends:** accepted platform-provenance and registry changes produced by T-X3/T-X4.
**Context budget:** 16 files / 25k tokens.
**Reads:** `SPEC-ios-client.md` §3.5, §4.2, §5.4, §7.3 · T-I0 parity ledger · registry mappings · generation-one HealthKit manager, capture registry, access probe, and correction/lookback tests · T-I5/T-I6 interfaces.
**Do:** Request read-only HealthKit access for the enabled registry subset with per-type rationale.
Implement a separate anchored query/anchor per type, including categories such as sleep through their
accepted mapping. Commit every query page's additions/deletions and next anchor in the same outbox
transaction. Preserve correction/deletion semantics defined by the server. Never infer read grant;
report ask/query/sample facts separately. Cover every parity-ledger type that is implemented and make
blocked/excluded reasons visible. Never aggregate or analyze on phone.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWHealthKitAuthorizationTests AWAnchoredQueryTests AWHealthKitParityTests && python3 scripts/verify-ios-parity.py clients/apple/GEN1-PARITY.yaml --require-owned`

### T-I9 — Observer/background delivery and phone status beacon

**Amends:** accepted T-X1/T-X2 phone-silence/status contract.
**Context budget:** 15 files / 22k tokens.
**Reads:** `SPEC-ios-client.md` §3.5, §4.3, §5.3, §10 · accepted beacon/silence decision · T-I7/T-I8 interfaces · generation-one observer/background delivery code · Home Assistant prior art §1.
**Do:** Register observers and immediate background delivery for enabled types. Observer callbacks
enqueue bounded query work and complete promptly; BG tasks perform bounded drains. Emit a
measurement-free status beacon with app/registry version, per-type request/query/sample/ACK facts,
outbox count/oldest age, and assertion time. The Pi owns cadence degradation. Add reboot,
force-quit/no-wake, locked-device, denied-type, never-sampled, query-failure, and stale-beacon
fixtures; no state may imply fresh data merely because registration succeeded.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWBackgroundDeliveryTests AWStatusBeaconTests && scripts/verify-phone-beacon-fixtures.sh`

### T-I10 — Pi read client, expiring projection cache, and unknown/stale model

**Amends:** 07 client API.
**Context budget:** 14 files / 20k tokens.
**Reads:** `SPEC-ios-client.md` §3.1, §3.4, §4.4, §10 · T-I2 OpenAPI/fixtures · dashboard query contracts · registry JSON.
**Do:** Implement typed reads for latest projection, history/gaps, receiving state, policies,
catalogs, analysis, and reports as their server endpoints exist. Cache bounded server projections by
endpoint/client/schema/query with expiration. Render server failure + cache as stale with age/reason;
failure + no cache as unknown. Preserve null gaps and unknown enum values. Use opaque server cursors
and explicit UTC windows; no local aggregate or trend computation.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWReadClientTests AWProjectionCacheTests AWUnknownStateTests && scripts/verify-ios-no-analysis.sh`

### T-I11 — Today, history, report, health-access, and diagnostics surfaces

**Amends:** none.
**Context budget:** 18 files / 25k tokens.
**Reads:** `SPEC-ios-client.md` §3, §8, §10 · T-I9 status model · T-I10 read/cache model · accepted server projection/report contracts · generation-one retained viewer surfaces from T-I0 only.
**Do:** Build the five non-input phone surfaces over server view models: Today, history, reports,
Health access/sync, and settings/diagnostics. Show ages, expectations, source/provenance, stale/unknown,
coverage gaps, pending outbox, and version/TLS state. Graphs preserve gaps and provide text summaries.
Reports are server artifacts. Add loading/empty/partial/failing/offline states, Dynamic Type, VoiceOver,
contrast, Reduce Motion, and no color-only status. Do not read outbox content as history.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWViewerUITests AWViewerSnapshotTests && scripts/verify-ios-accessibility.sh`

### T-I12 — Domain command/version/tombstone client contract

**Amends:** accepted record-ownership/schema decision from admin T-M5a/T-M5b.
**Context budget:** 14 files / 22k tokens.
**Reads:** `SPEC-ios-client.md` §3.2–§3.3, §4.1, §9 · accepted domain schema/version contract · T-I2 write fixtures · T-I5 outbox · generation-one domain model behavior identified by T-I0.
**Do:** Implement pure command models for journal, song stub/reference, medication dose, dose/random
check-in, immutable superseding version, and tombstone. Mint stable IDs before persistence; separate
subject/recorded UTC times; represent absence explicitly. Same ID+same content replays; same ID+
different content becomes conflict. Provide fixtures shared with Rust. Do not add a local domain
history schema or mutate acknowledged content.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWDomainContractTests && scripts/verify-domain-fixtures.sh && scripts/verify-ios-no-local-archive.sh`

### T-I13 — Journal rating, notes, tags, songs, and Express Mode

**Amends:** none.
**Context budget:** 18 files / 25k tokens.
**Reads:** `SPEC-ios-client.md` §3.2, §5.1, §8 · T-I0 journal parity rows/fixtures · T-I12 commands · server song/catalog projection contract · generation-one Journal/Express/song views named by parity ledger.
**Do:** Implement severity 1–10 with named bands, optional notes/tags/song, backdating with distinct
record time, entry detail/version/conflict state, and one-tap Express Mode. Capture+enqueue commits
before success feedback and triggers immediate drain. Song search/enrichment calls the Pi; offline
capture can select bounded cached results or create a free-form stub. Optional fields can be added by
new version but never delay Express capture. Add stale catalog, offline, duplicate tap, termination,
and accessibility tests; median physical-device Quick Log remains under five seconds.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWJournalTests AWExpressModeUITests AWSongInputTests && scripts/verify-ios-accessibility.sh`

### T-I14 — Medication doses and dose-triggered check-ins

**Amends:** accepted medication/domain policy.
**Context budget:** 18 files / 25k tokens.
**Reads:** `SPEC-ios-client.md` §3.3 and §9.1 · T-I12 commands · server medication/check-in policy · generation-one dose/follow-up behavior and fixtures from T-I0 · alarm arming contract when available.
**Do:** Render server-owned medication definitions and stale-aware last-taken values. Log a dose as a
durable command and trigger immediate upload. Schedule the server-configured local follow-up and
capture anxiety plus optional accepted pain fields. Distinguish queued locally from acknowledged and
from server monitoring-window armed; never claim arming on local save. Record answered, declined,
expired, and delivery-unknown separately. Guard duplicate taps without silently deduplicating two
real doses.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWMedicationTests AWDoseCheckinTests AWNotificationLifecycleTests`

### T-I15 — Random check-ins and notification-outcome evidence

**Amends:** none.
**Context budget:** 12 files / 18k tokens.
**Reads:** `SPEC-ios-client.md` §3.3 and §9.1 · T-I12 commands · server check-in policy contract · generation-one random-check-in behavior from parity ledger.
**Do:** Cache versioned server policy (enabled, active hours, frequency), schedule locally with stable
prompt IDs, reconcile policy changes, and enqueue outcomes immediately. Never schedule outside the
accepted local-time window; store all instants in UTC with timezone evidence. Distinguish system
authorization denial, scheduling failure, delivery unknown, expired, declined, and answered. Use
minimal lock-screen content by default.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWRandomCheckinTests AWNotificationLifecycleTests`

### T-I16 — watchOS Quick Log durable relay

**Amends:** none; records Stage 7 topology as phone relay.
**Context budget:** 15 files / 22k tokens.
**Reads:** `SPEC-ios-client.md` §8 and §9.3 · `design.md` §10 · T-I13 command interface · generation-one watch Quick Log files/tests named by T-I0.
**Do:** Implement a minimal color-plus-label severity grid with one-tap capture, stable ID minted on
watch, tiny durable queue, and WatchConnectivity relay. Phone ACKs watch only after its outbox commit;
server ACK is a later fact. Duplicate transfer is a no-op. Surface queued-on-watch, queued-on-phone,
and server-acknowledged distinctly. The watch stores no Pi credential and performs no analysis.
Test disconnected phone, duplicate delivery, process death, queue pressure, VoiceOver, and under-five-
second physical-device capture.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-tests.sh AWWatchRelayTests AWWatchUITests && scripts/verify-ios-watch-boundary.sh`

### T-I17 — Mobile security, privacy, accessibility, and release checks

**Amends:** 02 and 09.
**Context budget:** 20 files / 28k tokens.
**Reads:** Amendments 02 and 09 · `SPEC-ios-client.md` §7–§8 and §12 · all Apple entitlements/Info.plist/privacy manifests · dependency lock/SBOM · transport/storage/logging/UI security tests.
**Do:** Add conductor-runnable MASVS checks for STORAGE-1/2, CRYPTO-1/2 where applicable, AUTH-1/2/3,
NETWORK-1/2, PLATFORM-1/2/3, CODE-3/4, and PRIVACY-1–4. Prove ATS has no production bypass, credentials
use accepted Keychain protection, outbox/cache backup exclusion, app-switcher obscuring, minimal
permissions/entitlements, no secret/value/text logs, bounded decoding, pinned dependencies/SBOM,
privacy manifest, accessibility semantics/contrast/Dynamic Type/reduced motion, and release/debug
configuration separation. Findings block release; jailbreak detection/obfuscation is not added as
security theater without a separate threat decision.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-release-security.sh && scripts/verify-ios-accessibility.sh && scripts/scan-public-info.sh`

### T-I18 — Physical-device background delivery, locked-state, energy, and panic-path acceptance

**Amends:** 09 physical-device authentication/network acceptance.
**Context budget:** 12 files / 16k tokens plus device evidence.
**Reads:** Amendment 09 acceptance mapping · `SPEC-ios-client.md` §5, §8, §12.2 · T-I8/T-I9/T-I13/T-I16 acceptance tests · owner device/OS matrix from T-I3.
**Do:** On the owner's iPhone and Watch, measure HealthKit observer delivery, BG task drains,
force-quit behavior, locked-after-first-unlock Data Protection and Secure Enclave signing access,
notification outcomes, WatchConnectivity outage/recovery, launch/Express/Quick Log timing, and
overnight battery impact. Prove wrong-host/expired/untrusted certificate rejection, replay rejection,
revocation, clock failure, key-rotation overlap, and queued-record preservation. Record OS/build,
evidence timestamps, expected limitations, and status-beacon observations without committing personal
measurements. Reject permanent socket/audio/location/BLE workarounds. Set a measured energy budget and
retry/batch limits; failures name the device state and value.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-device-evidence.sh artifacts/ios-stage7-device-acceptance.json`

### T-I19 — Verified generation-one data migration

**Amends:** accepted historical-backfill plan from admin T-X5.
**Context budget:** 18 files / 28k tokens.
**Reads:** `SPEC-ios-client.md` §11 · accepted T-X5 migration contract · T-I0 parity ledger · T-I8 and T-I12 server contracts · generation-one export models and counts only as scoped by migration plan.
**Do:** Build a read-only exporter and idempotent importer for retained HealthKit samples and domain
records with dedicated migration provenance and an attempt record before work. Use the production
contracts, not direct SQL. Verify per-class counts, min/max dates, source/provenance distribution,
known sentinel records, conflicts/rejections, and a second-run no-op. Do not migrate snapshots as
measurements, demo/synthetic rows as physical data, or obsolete BLE/analysis state. Archive a redacted
migration report and keep generation one read-only on any discrepancy.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-migration.sh --report artifacts/ios-stage7-migration-report.json --require-second-run-noop`

### T-I20 — End-to-end Stage 7 acceptance and beta rollout

**Amends:** none.
**Context budget:** 20 files / 30k tokens plus device/Pi evidence.
**Reads:** `SPEC-ios-client.md` §12.3 in full · all prior task evidence · server T-X/M3/M4 acceptance outputs · deployment runbooks.
**Do:** Add one conductor-run suite for all eleven Stage 7 completion criteria: HealthKit parity and
sub-provenance; denied/absent/stale/rejected paths; offline journal durability and replay no-op;
conflict visibility; watch relay; check-in outcome distinctions; stale/gap viewer; transport/storage
security; accessibility/timing; Pi alarm independence with phone off; and migration parity. Run a
bounded beta with rollback and no data-destructive upgrade. Record every unsupported backend feature
as blocked, not mocked success.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && PI_HOST="$PI_HOST" scripts/verify-ios-stage7.sh --device-evidence artifacts/ios-stage7-device-acceptance.json --migration-report artifacts/ios-stage7-migration-report.json`

### T-I21 — Generation-one decommission ledger and deletion of obsolete services

**Amends:** Stage 7 architecture boundary.
**Context budget:** 18 files / 24k tokens.
**Reads:** T-I0 parity ledger · T-I20 acceptance report · admin T-X6 ledger · generation-one project targets/services named by those ledgers · `SPEC-ios-client.md` §1.2 and §11.
**Do:** Prove every retained capability has a Stage 7 owner and verified evidence, then remove from
the new client all BLE sessions, snapshot aggregation/repair, local analysis/reporting, CRDT/restore,
long-term health/domain mirror, demo seeding, and direct third-party integrations. Preserve only
accepted fixtures and documented clinical rules. Mark generation one read-only/archived; do not
delete its repository or original data. Produce a final capability ledger with retained/replaced/
dropped owner and evidence.
**Verify:** `cd "$(git rev-parse --show-toplevel)" && scripts/verify-ios-decommission.sh clients/apple/GEN1-PARITY.yaml artifacts/ios-stage7-acceptance.json && scripts/verify-ios-no-analysis.sh && scripts/verify-ios-no-local-archive.sh`
