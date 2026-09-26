# Design — Pi prototype storage degraded mode

## 1. Operating states

The host exposes one of these states in an operator-readable status file and service journal:

| State | Meaning | Allowed behavior |
|---|---|---|
| `production` | All production limits and durability rules apply | Safety-relevant and ordinary data follow their accepted contracts |
| `recovery` | Backup/restore evidence is being collected | Optional workloads are stopped; no schema changes |
| `prototype_degraded` | Temporary measured load reduction is active | Only the limits listed in the accepted profile apply |
| `blocked` | Required evidence is absent or invalid | Fail closed; do not enable relaxed behavior |

Every transition records timestamp, operator, reason, active profile, backup identifier, and the
value that caused the transition. A generic `degraded` message is insufficient.

## 2. Recovery set

The recovery set is a versioned manifest containing:

- a fresh encrypted pgBackRest full backup when a physical production backup is required, or a
  direct-to-Mac logical recovery set when protecting Pi free space during prototype recovery;
- PostgreSQL globals, extension inventory, relation/catalog metadata, and backup label;
- archived WAL required by the selected restore point;
- all pending spool segments, copied byte-for-byte with filename, size, mtime, checksum, and spool
  identity metadata;
- a manifest of the active source/type registry, service units, package versions, database plan,
  and relevant non-secret configuration;
- a record of secrets/configuration that are required for recovery without placing secret values in
  the repository or an unencrypted export.

The recovery set is transferred off `pi-host` to an operator-selected destination. The destination
must be outside the Pi's failure domain. Verification includes checksums, archive readability, a
non-vacuous database restore/read test, and spool manifest comparison. The transfer is not complete
when only a local NVMe copy exists.

When Pi free space is a concern, the prototype path streams `pg_dumpall --globals-only`, a custom
format `pg_dump`, WAL/recovery-boundary metadata, and a tar stream of the pending spool directly over
SSH to the Mac, where each artifact is encrypted and stored. No backup archive is staged on
`pi-host`. This is a logical recovery set: WAL metadata is not a physical WAL archive, and it does
not replace the production pgBackRest physical-backup/restore gate.

Because a full PostgreSQL backup does not include files waiting in the spool, the spool copy is a
separate mandatory artifact. The spool remains untouched on the Pi after copying.

### 2.1 Separate-project data inventory

The recovery set must retain all database data, including records about planes, noise, and the CNS
depression-warning system. The direct `pg_dump` is deliberately unfiltered for this reason.

After the backup is independently verified, inventory candidate data by actual table, column,
foreign key, source/type identity, row counts, timestamps, and code/schema references. A name such
as `noise` or `cns` is not proof that a row is used only by that project. Data is eligible for
separation only when the inventory proves it is project-exclusive.

- Project-exclusive non-`samples` rows may be exported to the separate project and removed only
  under an explicit, recorded cleanup decision after backup verification.
- Shared rows, shared tables, provenance rows, and uncertain rows remain in this database.
- During the prototype cleanup window, live collection is stopped and incoming records are not a
  priority. The production append-only rule for `samples` remains in force unless a separate,
  accepted amendment explicitly changes it; a backup alone does not silently authorize a new data
  model.
- Dropping tables, changing foreign-key ownership, or destructive migration remains separately
  gated and is not authorized by a broad description of the data.

The cleanup manifest must identify the exact relation/predicate, row count, export checksum, backup
identifier, reason, and rollback/recovery path. If a predicate cannot prove project exclusivity, the
correct result is `retain`.

## 3. Load-control order

During this recovery window:

1. Stop optional panels and raw dashboard queries.
2. Stop all collectors, `vitalsd`, and the drainer so incoming records and drain work do not compete
   with backup and cleanup.
3. Stream and verify the recovery set directly on the Mac.
4. Inventory project-exclusive records and execute only the exact operator-approved cleanup scope.
5. Measure the reduced database/filesystem state before deciding whether a second NVMe is still
   justified.

No step may concatenate segments, change offsets, or mutate `samples` under the current production
contract. If a future prototype purge requires changing that contract, it must be an accepted
amendment with a physical-backup, restore, and rollback design. The profile must be reversible by
one documented command and must survive/reveal a reboot rather than silently reverting to an unsafe
default.

## 4. Data classes

The initial prototype profile treats BP, EMAY, and any source designated safety-relevant by the
registry as lossless. High-rate H10/RR materialization is not automatically declared disposable;
its reduction requires measured spool capacity, an explicit operator decision, and a named source/
type policy. Missing data remains missing/pending and is never rendered as reassuring zero or good
health.

## 5. Second-NVMe decision

A second NVMe is a candidate, not a presumption. Before purchase or installation, collect:

- PostgreSQL data/WAL latency and checkpoint/WAL write pressure;
- spool append/fsync latency and drain throughput;
- filesystem queue depth, I/O wait, free space, and temperature;
- CPU, memory, connection, query, and filesystem-scan pressure separately;
- power-supply/undervoltage and PCIe link/thermal evidence; and
- cost, capacity, endurance, backup destination, and failure-domain comparison.

Evaluate at least these topologies:

1. current single-device layout;
2. database/WAL on one device and spool/backup repository on another; and
3. database on one device with off-box backup, leaving spool on the other where that preserves
   recovery semantics.

A second NVMe is justified only if controlled measurements show storage contention is a material
cause of the failure or backlog and the new topology does not create unacceptable power, thermal,
PCIe, or recovery risk. It cannot be used to excuse unbounded queries, connection churn, or O(files)
scans.

## 6. Production return

Return requires:

- production profile enabled and degraded profile disabled;
- no unverified pending migration or schema drift;
- bounded drain and query behavior verified under representative load;
- spool backlog decreasing with a named rate and no unexplained gaps;
- watchdog and safety-path resource priorities intact;
- off-box backup and restore verification still green; and
- operator acceptance recorded with the measured capacity and selected storage topology.

## 7. Safety and rollback

The first rollback is to stop optional workloads and re-enable the last verified profile, not to
reboot repeatedly or disable the watchdog. If a profile causes spool exhaustion risk, it fails closed
to `blocked` and alerts with the measured bytes/free-space values. No automatic cleanup is allowed.
