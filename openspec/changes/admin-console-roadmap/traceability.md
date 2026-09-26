# Admin-console design traceability

This table proves that the design was converted into work rather than reduced to the most visible UI
features. A row is covered when its task is merged and its own `Verify:` exits 0, or when an accepted
amendment explicitly supersedes it.

| Design source | Commitment | Queue owner |
|---|---|---|
| §2.2–§2.6 | Three-state liveness, ages/expectations, attempt/effect/high-water evidence, silence, idempotence, page freshness, precedence, five vocabularies, optional/muted/severity | M1c, M2e, M2g, M2k |
| §3 / Amendment 08 | Ten sections by milestone; live layout evaluation then pruning | M1d, M1e, M2j, M3i, M4f, M5c/M5d, M6d |
| §4.1 Overview | Promoted exceptions, quiet healthy state, build/schema identity | M1c |
| §4.2 Display path | Compositor, connector, Wayland socket, browser as separate claims | already shipped; regression in M1c |
| §4.3 Services | Manifest-derived units; process + health; uptime/restarts/memory/error/logs; host detail; audited controls | M2b, M2c, M2d, M2j |
| §4.4 Ingest | Source cards, attempt/success/delta/high-water/ownership/precedence; read-time CPAP transforms | shipped M1 plus M1c, M3g |
| §4.5 Data | Targeted coverage, keyset paging, explicit windows, filters/export, NULL semantics, per-day view | M3c, M3d, M3i |
| §4.6 Recovery | Backup/WAL/strict restore/loss window/data-loss reader | M3f, M5d |
| §4.7 Analysis | Correct lifecycle, cancel/history, cost before action, unified ledger, parse failure, explicit egress | M4a–M4f |
| §4.8 Records | Domain CRUD, prompt preview, versioning, active selection | M5a–M5c |
| §4.9 Config/access | Credential presence/last-4, one-time key, expiry/rotation, validated settings, audit | M2a, M2g, M2h |
| §4.9b Email | Gmail app password, STARTTLS, recorded test, coalescing, visible failure, off alarm path | M2e, M2f, M2k |
| §4.9c Telemetry | Opt-in OTLP, strict metric boundary, credentials/region/test/export health, bounded noncritical queue | M2i, M2j, M2k |
| §4.10 Alarm | Two observable channels, evidence ages, never-tested unknown, independent test operations | M6a–M6d |
| §4.10a / Amendment 16 | Operator-directed APNs targeting, user/all-device fanout, registration, redaction, audit, token invalidation, and distinct delivery facts | M6e, T-I22 |
| §5 | Proportionate confirmation, patterns A–D, forms, persistent upload history, accessibility | M2a/M2b/M2h, M3e, M4a/M4d, M5c, milestone suites |
| §6 push channel | Redundant alarm push and operator-directed APNs remain Pi-owned; phone-silence APNs exclusion remains | M6c–M6e, T-I22, X1, X6 |
| §6 clinical PDF | Pi-side clinical report owner | M4e, M4f |
| §6 JSON/CSV | Portable scoped export | M3d, M3i |
| §6 correlation recompute | Supervised recompute | M4a, M4b, M4f |
| §6 timezone | Validated operator setting | M2h |
| §6 schema/build identity | Overview readout | shipped; regression in M1c |
| §6 log sanitisation | One Rust sanitisation path | M2c |
| §6 phone silence | Pi-owned detector/escalation | M2e, M2f, X1 |
| §6 phone health access | Phone status beacon surfaced on Pi | X2 |
| §6 reconciliation | Fresh projection rebuild | M5e |
| §6 hidden correctness | receipt-time replay, finite rejection, nil-AHI exclusion, unfiltered baselines | X3/X4, M4b/M4e, X6 |
| §6 OAuth revoke/key rotation/read-only DB | Explicit replacement/decommission evidence | M2h, M3d, X6 |
| S1 CSRF | All state changes protected | M2a, M2k |
| S2 per-source throttle | No global lockout | M2a, M2k |
| S3 bounded sessions | Session expiry | M2a, M2k |
| S4 visible transport state | LAN cleartext/TLS posture visible | M2a, M2h, M2k |
| S5 pre-disk body cap | Upload/request limit before buffering | M2a, M3e, M2k |
| S6 DB role/timeout | Real read-only role | M3d |
| S7 sanitisation | No raw secrets/errors | M2c and every endpoint task |
| S8 egress opt-in/stated | Destination and payload before send | M2f, M2i, M3d, M4c |
| §8 non-goals | No health viewer merge, multi-user system, sample editing, or remote shell | all; specifically M2b, M3c, M3g |
| §8b reported/changed | Stored freshness | shipped schema; M2e/M2g |
| §8b transition/jitter/evidence/taxonomy | Supervised repeat behavior | M2e, M2f, M4a |
| §8b overrides/defaults | Registry immutable, override rows | M2g |
| §8b platform envelope | Domain + platform sample classes | shipped domain class; X3/X4 |
| §8b HealthKit/Health Connect | Per-platform source and pure mapper path | X3/X4; later Android follow-up after X3 |
| §8b CPAP/myAir/SD | Batch parser carve-out and supervised upload/import | M3e; collector plan owns pollers |
| §8b BLE operations | Collector supervisor, not console | pi5 core Stage 4; visible via M2c |
| Criterion 1–3 | M0 foundations | already shipped |
| Criteria 4–10 | M1 | M1c |
| Criteria 11–13 | M2 | M2k |
| Criteria 14–15 | M3+ | M3i/M5d |
| Criterion 16 | M4 | M4c/M4f |
| Criterion 17 | M6 | M6d |
| Criteria 18–19 | accessibility + regression activation | every milestone suite |
| §9c bind/auth | Decide at point of use | M2a (M0 stayed private as shipped) |
| §9c watch topology | Stage 7 only | deliberately not queued here; Stage 7 decision owns it |
| §9c HA machinery | borrow behavior, not Python framework | all M2 tasks; Amendment 07 |
| Open Q1 | Phone-silence escalation | X1 |
| Open Q2 | Retention ownership | M3b |
| Open Q3 | Records placement | M5a |
| Open Q4 | SQL pane | M3h |
| GEN1 historical backfill | One-time verified migration | X5 |
| GEN1 complete decommission | No unowned capability | X6 |

## Generation-1 admin workflow reference

`legacy-admin-reference.md` inventories the useful behaviour of the 26-route/17-template Flask admin
and maps each workflow to its generation-2 owner or explicit drop. It is subordinate to accepted
amendments and `GEN1-REASSESSMENT.md`; it supplies operator expectations and fixture leads, not an
implementation-language or schema precedent.

## Explicit external ownership

The following design commitments are not duplicated as console implementation tasks, but are not
lost:

- BLE recovery ladder, slot accounting, classified backoff, and learned intervals remain in the
  pi5-vitals-core collector-supervisor plan; M2 consumes their evidence.
- Watch Quick Log relays through the phone for Stage 7; the client implementation and evidence live
  in `openspec/changes/ios-client-stage7/` T-I16. Direct watch-to-Pi remains deferred.
- The phone-side HealthKit proxy, temporary outbox, viewer, and subjective inputs are owned by
  `openspec/changes/ios-client-stage7/`; this roadmap's T-X1–T-X6 own their Pi-side dependencies.
- Android Health Connect follows the platform contract established by T-X3; it is a later platform
  task rather than a blocker for HealthKit.
- myAir and SD-card parser internals remain out-of-process batch tooling under Amendment 07; M3e and
  M4a provide their supervised lifecycle and evidence.
