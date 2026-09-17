# Amendment 13 — Platform-sample envelope and sub-provenance

**Date:** 2026-09-17

**Status:** Accepted

**Amends:** Amendments 01, 02, 05, and 06; `SPEC-ios-client.md` §4.2; the platform-sample
stage of the client envelope

**Affects:** T-X3, T-X4, T-I6, T-I8, T-I19

## 1. Platform samples are a distinct envelope payload class

The authenticated phone-to-Pi envelope carries two payload classes:

1. **Domain records**, identified by client UUIDv7 and governed by the domain-record contract.
2. **Platform samples**, identified by the originating platform-store sample and governed by this
   amendment.

The envelope is transport only. It does not assign physiological meaning, convert units, aggregate
values, or create a local health archive. A platform-sample batch contains bounded source items with:

- the HealthKit sample UUID;
- the original HealthKit type identifier;
- UTC start and end instants;
- the original numeric value and HealthKit unit identifier, before canonical conversion;
- source revision fields when available: bundle identifier, source name, product type, and version;
- device metadata when available, restricted to the allowlist in §3;
- the query/observer batch identifier and the phone's record instant.

The phone sends only HealthKit types enabled by the fetched registry. It never sends generation-one
snapshot fields, daily totals, trend values, thresholds, scores, reports, or other phone-computed
measurements as platform samples.

## 2. platform sample sub-provenance and storage

`source = healthkit` identifies the platform pipeline. It is not sufficient provenance by itself.
The server preserves the originating HealthKit source and device as a versioned UTF-8 JSON object in
`samples.extra`:

```json
{
  "schema": 1,
  "platform": "healthkit",
  "sample_id": "<HealthKit sample UUID>",
  "healthkit_type": "<HealthKit type identifier>",
  "source": {"bundle_id": "…", "name": "…", "product_type": "…", "version": "…"},
  "device": {"manufacturer": "…", "model": "…", "hardware": "…", "os_version": "…"}
}
```

Absent fields are omitted, not replaced with empty strings or invented values. Unknown metadata is
rejected or omitted according to the field allowlist; it is never copied wholesale. The server
canonicalizes this object before writing it. The sample UUID is retained in the object even though
the key also encodes it, so exports and investigations can explain identity without reverse
engineering a key.

The existing generic `extra BYTEA` column is the deliberate schema home for this versioned
sub-provenance. No second HealthKit table, local mirror, or source-specific primary key is added.
This preserves the single `samples` narrow waist and its COPY column order. The primary key remains
`(source, type, session_id, seq)`.

A HealthKit sample is never deduplicated against a Polar, Watch, cuff, or other direct-sensor row.
The distinct source and preserved sub-provenance let projections identify same-period observations
without deleting either fact.

## 3. Identity and mapper boundary

The phone does not invent `session_id` or `seq`. The server's pure platform mapper derives the
stable 128-bit ingest identity from the HealthKit sample UUID by placing its first eight bytes in
`session_id` and its last eight bytes in `seq`, each interpreted as a signed two's-complement
`BIGINT` without changing the bits. This is a lossless split: the pair is stable across retries and
has no dependence on arrival time, query order, or the phone process.

For this store-and-forward source, the pair is an opaque sample identity; `seq` has no chronological
meaning and must not be used as a time cursor. Time queries use `timestamp` and the sample's UTC
interval. A retry with the same UUID and canonical payload is an idempotent duplicate. A UUID/key
match with different immutable payload or provenance is a conflict and is quarantined with the
sample UUID and blocking reason; it is never overwritten.

The mapper boundary is pure: `(platform payload, fetched registry) -> canonical sample(s) or named
quarantine reason`. It performs strict finite/value/range checks and one canonical unit conversion,
but no I/O, network access, Keychain access, clock read, aggregation, analysis, or global state. The
receiver owns authentication, bounded decoding, idempotency, persistence, and audit evidence around
that boundary.

## 4. sleep-stage mapping

The registry type `sleep_stage` uses canonical integer codes:

| Code | Canonical stage |
|---:|---|
| 0 | awake |
| 1 | asleep_unspecified |
| 2 | core |
| 3 | deep |
| 4 | rem |

HealthKit maps `HKCategoryValueSleepAnalysis.awake`, `asleepUnspecified`, `asleepCore`,
`asleepDeep`, and `asleepREM` one-to-one to these codes. The interval and source provenance are
retained; merging overlapping intervals, totals, and sleep reports remain Pi projections.

The future Health Connect mapper must explicitly map `awake`, `out_of_bed`, `sleep/light`,
`sleep/deep`, `sleep/REM`, and `sleep/unspecified` to the same codes. An unknown, `unknown`, or
platform stage without a defensible canonical equivalent is quarantined as
`unmappable_sleep_stage:<platform value>`. It is never mapped to awake, zero, or
`asleep_unspecified` merely to keep a row.

## 5. Parity ledger disposition

The following generation-one families may receive canonical platform-sample rows when their
registry mapping and receiver contract land: heart rate, HRV SDNN, sleeping wrist temperature,
respiratory rate, oxygen saturation, blood pressure components, blood glucose, body mass, activity
quantities, gait quantities, audio exposure, atrial-fibrillation burden, sleep categories, and
workout intervals. Each accepted type must name its registry type, unit, range, HealthKit identifier,
and receiver fixture before enabling it on the phone.

The following are named exclusions from the platform-sample class until a separate contract exists:

- demographic characteristics, because they are context and require a minimization/purpose contract;
- clinical lab documents, because they are structured clinical records rather than scalar samples;
- heartbeat series, until stable series identity and gap semantics are specified;
- general body temperature, until its semantics are separated from sleeping wrist temperature;
- any generation-one aggregate, snapshot, trend, threshold, score, or analysis value.

An exclusion is recorded in the parity ledger with its reason and gate. It is not silently omitted and
is not represented as a zero or an absent measurement.
