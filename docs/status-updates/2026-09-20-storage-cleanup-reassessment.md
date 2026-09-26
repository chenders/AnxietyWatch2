# Storage cleanup reassessment

## What I got wrong

I applied the production append-only rule as though it answered the immediate prototype capacity
problem. It does not. A tombstone preserves an audit fact, but it does not materially reduce the
physical size of `samples`; adding a recovery manifest also does not help if the goal is to reclaim
space and reduce PostgreSQL load.

The correct operational sequence for this situation is:

1. Stop all collection and draining. Incoming records are explicitly not a priority during this
   recovery window.
2. Create an unfiltered recovery set off `minidude`, including the database data described as
   planes/noise and CNS depression-warning data.
3. Verify that the off-Pi artifacts are readable and complete.
4. Inventory the exact relations and predicates for the two projects.
5. Remove project-exclusive data that the operator has authorized, so the live database actually
   becomes smaller.
6. Measure the result before buying hardware or redesigning storage.

I had incorrectly treated step 5 as forbidden in all circumstances instead of recognizing that the
current production contract and the prototype recovery objective are in conflict.

## Constraint that still requires an explicit contract change

The repository's currently accepted contract says that `samples` is append-only and that retirement
uses tombstones. I cannot silently violate that accepted contract, even with a backup. If the
planes/noise or CNS-only records are in dedicated non-`samples` tables, their cleanup can proceed
under the operator-authorized, exact cleanup manifest after backup verification.

If those records are inside `samples`, physical removal requires a deliberate amendment or an
explicitly accepted prototype-purge contract. The backup is the recovery boundary, but it does not
make an unrecorded schema-contract change safe or reproducible.

This is now a clearly identified decision, not a reason to retain all data indefinitely.

## Other decisions re-examined

### Direct backup versus a true full backup

The direct-to-Mac path is good for preserving row data without consuming Pi space: it streams an
unfiltered custom-format `pg_dump`, globals, WAL/recovery metadata, and the pending spool, then
encrypts them on the Mac. However, it is a logical recovery set, not a physical pgBackRest backup
with a WAL archive. I will not describe it as a verified physical full backup. The production
physical-backup/restore gate remains distinct.

### Keeping ingestion alive

That was unnecessary for this recovery. The watcher is now prepared to stop the panel, drainer,
`vitalsd`, and known collectors after the first evidence capture. No incoming data needs to be
preserved while backup and cleanup are underway.

### Keeping the pending spool indefinitely

That was also too conservative for the stated goal. The spool must be included in the off-Pi
recovery set first. Once its archive and manifest are verified, it can be considered for controlled
cleanup as part of the approved prototype purge; it must not be concatenated or have offsets manually
advanced.

### Buying another NVMe immediately

A second NVMe may still be worthwhile for production isolation, endurance, backup, and failure-domain
reasons, but it is not the first response to a database containing data that the owner has already
identified for removal. Measure the post-cleanup database and workload first.

### Recovery manifests

The recovery manifest belongs off-Pi with the recovery artifacts. It must not be inserted into the
live database as a substitute for cleanup or allowed to increase PostgreSQL load.

## Incident framing confirmed

The machine is being treated as an unstable prototype incident system, not as a normally operating
production collector. This prototype is not currently used for real healthcare monitoring or clinical
health decisions. Permanent data loss would be undesirable, but it is an acceptable fallback risk if
necessary to recover a stable host and learn from the failure. The reliability measures and workload controls have failed to keep it usable. Therefore:

- incoming data collection is intentionally stopped while the incident is investigated;
- services and safeguards that would normally remain enabled may be stopped or changed in a controlled,
  evidence-preserving sequence;
- live data that would normally never be deleted may be removed after the off-Pi recovery set is
  complete and verified, because the immediate objective is to recover a stable, usable host;
- the normal production rules are not being quietly forgotten: the incident exception, its scope, the
  backup identifier, and each destructive action will be recorded so production can be rebuilt
  correctly afterward.

The sequence is not "keep collecting while hoping the host stabilizes." It is: preserve evidence and
backup, stop intake and load, diagnose stepwise, perform authorized cleanup/intervention, then test
whether the host is stable.

## Current state

No destructive cleanup has run: `minidude` is still unreachable. The local watcher and direct-to-Mac
recovery script are prepared. Once the host returns, the sequence is evidence capture, stop intake,
direct backup, verification, classification, controlled intervention/cleanup, and stability
measurement. I understand that normal production deletion rules are not the objective during this
incident; any conflict with an accepted schema contract will be recorded as an explicit incident
exception before execution rather than silently ignored.
