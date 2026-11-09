# Attachments

Evidence files, committed verbatim. They are **not** inputs to any build and must never be
edited — their bytes are the record, and a hash that stops matching is the point.

## `2026-09-10-air11-build-registry.toml`

The registry the deployed `AirSense` collector was actually built against.

| | |
|---|---|
| Captured from | `$HOME/aw2-air11-build/vitals/registry.toml` on the Pi |
| Captured on | 2026-11-09 |
| File date | 2026-09-10 22:15 |
| Size | 3,614 bytes |
| `sha256` | `eda48c2b0423f82d0803a275193c72a8aee06f917f694d0657ba7df78cf197bc` |

### Why it is here

`$HOME/aw2-air11-build` is **not a git repository**. This file existed as a single
uncommitted copy on one disk, in no history anywhere, and it is the authoritative record of
what 110,666,216 stored sample rows mean. One `rm` from unrecoverable.

It declares the `AirSense` types at ids **9–23**:

```
 9  respiratory_rate         bpm        15  leak                     L/min
10  patient_flow             L/s        16  tidal_volume             mL
11  mask_pressure            cmH2O      17  minute_ventilation       L/min
12  mask_pressure_2s         cmH2O      18  target_minute_ventilation L/min
13  inspiratory_pressure     cmH2O      19  ie_ratio                 ratio
14  expiratory_pressure      cmH2O      20  snore_index              index
                                        21  flow_limitation          ratio
                                        22  inspiratory_duration     seconds
                                        23  trigger_cycle_event      code
```

`vitals/registry.toml` on `main` assigns ids 9–29 to HealthKit types, independently, in #52.
Both writers committed rows to the same `samples.type` column, so eleven ids carry two
meanings — id 15 is both `exercise_time` in minutes and `leak` in litres per minute.

Neither scheme is wrong. The id namespace is global but was used as if per-source, and each
source used exactly one scheme for its whole history.

### What makes this recoverable rather than lost

Every affected row carries its own meaning. All 110,651,486 source-5 rows have an `extra`
JSON envelope with a `signal` field, and the `type → signal` mapping is **1:1 with zero
deviations across the entire table** and stable across eras. Source 2 likewise self-attests
through `extra.healthkit_type`. So the resolution map is not an interpretation imposed on the
data — it is a projection of data already in it, re-derivable forever.

This file is the independent corroboration of that projection, and the only machine-readable
copy of the declaration side. The others die on contact:

- `vitals_type` on the Pi — overwritten by the next `psql -f vitals/generated/registry.sql`
- the deployed binary — gone on rebuild

See `openspec/changes/sample-type-resolution/` for the remediation, and amendment 20 §6 for
the id-permanence rule this violates on the type axis.
