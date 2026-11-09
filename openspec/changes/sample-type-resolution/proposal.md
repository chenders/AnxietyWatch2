# Proposal: Resolve `samples.type` at read, because the rows already know

`samples.type` holds two incompatible numbering schemes, and 110,666,216 rows are currently
mislabelled by the registry that claims to describe them.

The deployed `resmed-air11-collector` (`/usr/local/bin/resmed-air11-collector`, built
2026-10-09) was compiled in `$HOME/aw2-air11-build` — a Pi-local directory that is
**not a git repository** — against a registry declaring the `AirSense` types at ids **9–23**.
Main's #52 (79201d3, 2026-09-26) independently assigned ids **9–29** to HealthKit types.
Both writers committed rows. Eleven ids carry two meanings:

| id | `main` declares | source 5 actually wrote | rows |
|---|---|---|---|
| 9 | `sleep_stage` (stage_code 0–4) | `respiratory_rate` | 642,836 |
| 10 | `resting_hr` (**bpm**) | `patient_flow` (**L/s**) | 51,764,090 |
| 11 | `respiratory_rate` | `mask_pressure` (cmH2O) | 51,764,088 |
| 15 | `exercise_time` (**min**) | `leak` (**L/min**) | 642,835 |
| 16 | `environmental_audio` (dBASPL) | `tidal_volume` (mL) | 642,836 |
| 17 | `blood_glucose` (mg/dL) | `minute_ventilation` | 642,836 |
| 20 / 21 | `headphone_audio` / `walking_steadiness` | `snore_index` / `flow_limitation` | 642,835 / 642,586 |

Id 15 is simultaneously minutes and litres per minute. That breaks **one canonical unit per
type** at the id level, and it is worse than a renamed id: the values are silently in the
wrong dimension and nothing downstream can tell.

## Neither scheme is wrong

The type-id namespace is global but was used as if per-source, and **each source used exactly
one scheme for its entire history**. Source 2's 16,763 rows match main's names exactly;
source 5's 110.6M match the build-directory registry's exactly. So this is not corruption to
be repaired — it is an ambiguity to be resolved.

## The rows already carry their own meaning

This is what makes a read-time map honest rather than a guess. Measured over the full table,
not sampled:

```
src5_total | with_extra | with_signal
 110651486 |  110651486 |   110651486
```

Every source-5 row has an `extra` JSON envelope with a `signal` field, and the
`type → signal` mapping is **1:1 with zero deviations across all 110.6M rows**, stable
across eras (oldest-300, a mid-October window, and newest-300 per id all agree). Source 2
self-attests the same way through `extra.healthkit_type`.

The resolution map is therefore **a projection of data already in the rows**, total and
re-derivable from its own source forever — not an interpretation imposed on them. That is
what no other remedy offers.

## Why the alternatives lose more than they recover

**Tombstone and reingest — unavailable.** Three independent blockers. There is no tombstone
mechanism for `samples`: no column, no table, and the primary key *is*
`(source, type, session_id, seq)`, so a tombstone row collides with the row it tombstones.
The upstream does not cover the range — the first journal's mtime is 2026-09-12 12:27:18Z
while the first source-5 row is 11:51:42Z, leaving ~20 minutes and ≈28,000 rows with no
journal, plus 757.7 h of gaps in a 1,398 h span and one quarantined unopenable journal. And
no replay tool exists for the `AirSense`; re-derivation would have to reproduce `session_id`
and `seq` exactly across 38,677 sessions or identity changes anyway. It would destroy 110.6M
rows to recover information those rows already carry.

**Renumber — forbidden and futile.** Moving stored rows means `UPDATE samples SET type`,
which `AGENTS.md` forbids outright, and amendment 20 §6 makes ids permanent. Moving the
*declaration* instead makes source 2's 16,763 rows the newly-mislabelled set and still does
not solve it, because source 4's type 9 needs a `(source, type)` map regardless.

**Accept and document — not legitimate, because the evidence is still destructible.** The
machine-readable copies of the old scheme are `vitals_type` on the Pi, which the next
`psql -f vitals/generated/registry.sql` overwrites; the build-directory registry, now
preserved at `docs/specs/attachments/2026-09-10-air11-build-registry.toml`; and the deployed
binary, which dies on rebuild. Accepting leaves every reader to parse 241-byte JSON out of
110M rows to learn what it is looking at.

## A structural fact that constrains the end state

`vitals_type.name` is `NOT NULL UNIQUE` (`db/deploy/registry_tables.sql:9`). So `patient_flow`
cannot be held at both 10 and 30, which means **leaving `vitals_type` on the old scheme is not
a stable resting place**: ids 24–43 could never be inserted, permanently blocking HealthKit
ingest, the SD importer, and any collector rebuild. `registry.toml` is the single source of
truth, so main's scheme is the declaration — but reconciling to it must come *last*, through a
guard that refuses unless every affected pair is already resolvable.

## What this change does

Adds `sample_type_resolution`, keyed on `(source, stored_type, valid_from)` and referencing
`vitals_type.name` rather than its id, so the map survives the reconciliation. Populates it
**generated from the attestation, never hand-written**. Adds a `samples_resolved` view using
`LEFT JOIN`, so an unresolved row surfaces as `resolved = false` rather than vanishing — three
states, and the view is where a reader meets them. Extends the amendment-20 §6 guard to permit
a relabel only when every affected pair is covered. Then, and only then, reconciles
`vitals_type` and rebuilds the collector at a session boundary with the era bound recorded in
the same transaction.

The cost is a permanent read-time indirection: the stored id is non-canonical forever, and a
consumer joining `vitals_type` on `s.type` alone gets a confidently wrong answer. That is made
impossible by a CI gate, not discouraged by a comment.

Nothing is inserted, updated or deleted in `samples`. Fully reversible: the map is a new
table and the reader change is a code revert.
