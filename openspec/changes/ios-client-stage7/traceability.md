# Stage 7 Apple-client traceability

A row is complete only when its task's `Verify:` exits 0 or an accepted amendment explicitly
supersedes it.

| Specification commitment | Task owner |
|---|---|
| §1 four jobs and Pi authority | T-I0, T-I3, T-I21, T-I23 |
| §1.2 no legacy BLE/local analysis/local authority; bounded relay exception | T-I3, T-I10, T-I21, T-I23 |
| §2 immediate upload | T-I5, T-I7, T-I9 |
| §2 batch transport without added delay | T-I7, T-I9 |
| §2 watch relays through phone | T-I16 |
| §2 registry JSON, no generated Swift enum | T-I2, T-I6 |
| §2 HealthKit read-only | T-I8, T-I17 |
| §3.1 Today/server monitoring projection | T-I10, T-I11 |
| §3.2 rating/notes/tags/song/backdate/version | T-I12, T-I13 |
| §3.2 one-tap Express Mode | T-I13, T-I18 |
| §3.3 dose/check-in and arming acknowledgement distinction | T-I14 |
| §3.4 server history/report/gaps | T-I10, T-I11 |
| §3.5 per-type HealthKit facts | T-I8, T-I9, T-I11 |
| §3.6 endpoint/version/TLS/redacted diagnostics | T-I4, T-I7, T-I11 |
| §4.1 client ID/idempotence/version/tombstone | T-I2, T-I12 |
| §4.2 generation-one HealthKit parity ledger | T-I0, T-I6, T-I8 |
| §4.2 HealthKit sub-provenance | admin T-X3/T-X4, T-I6 |
| §4.3 measurement-free status beacon | admin T-X1/T-X2, T-I9 |
| §4.4 opaque server cursors and expiring projection caches | T-I10 |
| §5.1 outbox is temporary, ACK deletes content | T-I5 |
| §5.1 unacknowledged data never age-evicted | T-I5, T-I17 |
| §5.2 retry/cancel/single drain | T-I7 |
| §5.3 bounded iOS background behavior and relay suspension/stale state | T-I9, T-I18, T-I23 |
| §6.3 BLE source relay and Pi-state-to-BLE-bracelet mirror | T-I23 |
| §5.4 anchor advances atomically with enqueue | T-I8 |
| §6 Rust versioned API/OpenAPI | T-I2 |
| §6.1 secure production enrollment | T-I1, T-I4 |
| §6.2 stable item outcomes/errors/versioning | T-I2, T-I7 |
| §7 MASVS security/privacy | T-I1, T-I4, T-I5, T-I17 |
| §8 accessibility and panic path | T-I11, T-I13, T-I16–T-I18 |
| §9 local notifications | T-I14, T-I15 |
| §9.2 remote APNs messages and alarm information | admin M6, Amendment 16, T-I22 |
| §9.3 watch durable two-hop relay | T-I16 |
| §10 shared failure taxonomy/evidence | T-I7, T-I9–T-I12 |
| §11 migration and no-op replay | admin T-X5/T-X6, T-I19, T-I21 |
| §12 automated tests | every implementation task; T-I17 |
| §12.2 physical-device evidence | T-I18, T-I23 |
| §12.3 Stage 7 completion and optional relay gate | T-I20, T-I23 |

## Generation-one carry/drop ownership

| Generation-one behavior | Stage 7 decision | Owner |
|---|---|---|
| HealthKit authorization/probe | keep and narrow; report facts, not inferred grant | T-I8/T-I9 |
| General HealthKit data set | proxy source samples with parity ledger; no phone analysis | T-I0/T-I6/T-I8 |
| SwiftData HealthKit mirror | replace with temporary outbox; delete content on ACK | T-I5/T-I21 |
| SnapshotAggregator and trend thresholds | drop from phone; server projections only | T-I10/T-I21 |
| Journal rating/notes/tags/song | keep as thin input | T-I12/T-I13 |
| Express Mode | keep protected one-tap path | T-I13/T-I18 |
| Medication dose/follow-up | keep local interaction, Pi authority/policy | T-I14 |
| Random check-ins | keep local scheduling, Pi policy | T-I15 |
| watchOS Quick Log | keep through phone relay | T-I16 |
| Polar/EMAY/cuff BLE | drop legacy collector sessions; supported mobility relay is separately retained only under Amendment 17 | T-I21/T-I23 |
| Correlation/CNS/report generation | drop from phone | T-I10/T-I21 |
| Rebuild snapshots/repair server | drop | T-I21 |
| Bidirectional CRDT/restore | replace with idempotent push + ordinary reads | T-I5/T-I7/T-I10 |
| Local long-term journal/health history | drop; Pi is sole long-term owner | T-I5/T-I10/T-I21 |
| Historical app-only records | one-time verified migration | T-I19 |

## Cross-plan dependencies

- `admin-console-roadmap` T-M2a and T-I1 jointly resolve enrollment/auth/TLS.
- T-X1/T-X2 own Pi silence and status-beacon receiving; T-I9 owns the sender.
- T-X3/T-X4 own platform identity, provenance, canonical mapper, and receiver; T-I6/T-I8 own the
  registry-driven source payload and HealthKit queries.
- T-M5a/T-M5b own authoritative domain schema/versioning; T-I12–T-I15 own capture commands.
- M3/M4 own projections, analysis, and reports; T-I10/T-I11 only fetch and render them.
- T-X5/T-X6 own server backfill/decommission evidence; T-I19/T-I21 own the Apple side.
- M6 owns remote alarm transport and operator-directed APNs fanout; T-I22 owns iPhone token
  registration, receipt, protected-content handling, and physical-device evidence. The Pi local
  alarm never depends on the phone, and Amendment 14 phone-silence escalation never uses APNs.
- Amendment 17/T-I23 owns the optional BLE source relay and Pi-state-to-BLE nRF52840 bracelet
  mirror. Its stream may be suspended and its indicator firmware may expire to stale; it is never the
  sole emergency path. Wi-Fi bracelet hardware is deferred.
