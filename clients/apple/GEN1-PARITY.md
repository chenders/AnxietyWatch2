# Generation-one Apple-client parity report

This report is the readable companion to [`GEN1-PARITY.yaml`](GEN1-PARITY.yaml). The YAML file is
the machine-checked ledger; this report explains the decisions. It inventories generation-one only—it
does not authorize a type to ship. Every retained HealthKit type remains gated on an accepted
`source.healthkit` registry mapping, canonical unit/range, sub-provenance contract, and Pi receiver.

## Boundary

Stage 7 retains three Apple-client jobs: proxy HealthKit source records, capture person-local inputs,
and display Pi projections. The phone does not keep a health archive or compute snapshots, trends,
baselines, CNS tiers, correlations, or reports. The watch keeps Quick Log and relays it through the
phone without a Pi credential.

## Inventory result

The review found **32 distinct HealthKit types** across generation-one authorization, observer/anchor,
local-mirror, snapshot-input, UI/feature-documentation, and direct-query sets:

| Classification | Count | Meaning |
|---|---:|---|
| `proxy_sample` | 25 | Preserve each source quantity sample and provenance; Pi converts and stores it. |
| `proxy_category` | 2 | Preserve category/interval records; Pi maps stages/categories and projects totals. |
| `context_only` | 2 | Demographics are not samples and require an explicit purpose/minimization contract. |
| `blocked_registry` | 2 | Do not ship until the named semantic/identity gap is accepted. |
| `excluded` | 1 | Outside the accepted Stage 7 platform-sample receiver. |

### HealthKit type ledger

| Type | Classification |
|---|---|
| Heart-rate variability SDNN | `proxy_sample` |
| Heart rate | `proxy_sample` |
| Resting heart rate | `proxy_sample` |
| Respiratory rate | `proxy_sample` |
| Oxygen saturation | `proxy_sample` |
| Sleeping wrist temperature | `proxy_sample` |
| Step count | `proxy_sample` |
| Active energy burned | `proxy_sample` |
| Exercise time | `proxy_sample` |
| Environmental audio exposure | `proxy_sample` |
| Systolic blood pressure | `proxy_sample` |
| Diastolic blood pressure | `proxy_sample` |
| Blood glucose | `proxy_sample` |
| VO2 max | `proxy_sample` |
| Walking heart-rate average | `proxy_sample` |
| Headphone audio exposure | `proxy_sample` |
| Walking steadiness | `proxy_sample` |
| Atrial-fibrillation burden | `proxy_sample` |
| Walking speed | `proxy_sample` |
| Walking step length | `proxy_sample` |
| Walking double-support percentage | `proxy_sample` |
| Walking asymmetry percentage | `proxy_sample` |
| Time in daylight | `proxy_sample` |
| Physical effort | `proxy_sample` |
| Body mass | `proxy_sample` (authorization reconciliation required) |
| Sleep analysis | `proxy_category` |
| Workout | `proxy_category` |
| Date of birth | `context_only` |
| Biological sex | `context_only` |
| General body temperature | `blocked_registry` |
| Heartbeat series | `blocked_registry` |
| Clinical lab-result record | `excluded` |

## Reconciliation findings

1. **Body mass appears in the mirror but not `allReadTypes`.** It remains a parity target, but T-I8
   must reconcile authorization before it can be enabled.
2. **General body temperature appears in the mirror while authorization requests sleeping wrist
   temperature.** They must not share semantics accidentally. The registry owner must accept separate
   or deliberately mapped concepts.
3. **Heartbeat series is queried to derive RR intervals but is absent from `allReadTypes`.** It remains
   blocked until authorization, series identity, gap behavior, and source-sample transport are defined.
4. **Blood-pressure correlation itself is deliberately absent from read authorization.** Stage 7 keeps
   the systolic and diastolic quantity samples; it must not add the correlation type to the request,
   because generation one documents that doing so terminates the app with an Objective-C exception.
5. **Clinical lab records use a separate authorization and structured-document API.** Flattening them
   into registry-valued samples would lose meaning and violate minimization, so they are excluded from
   this capability pending a dedicated proposal.
6. **Read access cannot be inferred from HealthKit authorization status.** Stage 7 reports request,
   query, sample, outbox, acknowledgement, and Pi-receiving facts independently.

## Source samples are not generation-one aggregates

The retained unit is the HealthKit source record: UUID, type, start/end instants, original value and
unit or category, source revision, device, allowlisted metadata, and query batch identity.

The ledger separately names **49 generation-one `HealthSnapshot` aggregate fields**. Those fields—including
HRV averages/minima, sleep totals, SpO2 burden, glucose variability, daily activity, gait summaries,
CPAP values, and sensor-derived indices—are not proxy payload types. They are removed from the phone
runtime and replaced, where retained, by Pi projections. This prevents an aggregate from being
mistaken for a source sample merely because both appeared in generation one.

## User-visible capability decisions

### Retained on iPhone/watchOS

- Anxiety rating (1–10), optional note, tags, song, backdated subject time, and Express Mode.
- Medication-dose capture, dose follow-ups, and random check-ins, with prompt outcomes preserved.
- watchOS Quick Log with durable relay through iPhone.
- Today, journal, medication, history/trend, report, Health access/sync, settings, and diagnostics
  surfaces, backed by Pi records and projections.
- Immediate temporary outbox delivery and explicit pending/acknowledged state.

### Removed or replaced

- Polar H10, EMAY, Oura, and cuff BLE sessions move to Pi collectors.
- Local HealthKit mirror/archive, daily snapshots, baseline/trend/CNS/correlation analysis, and local
  PDF generation are removed.
- Snapshot rebuild/refresh and store-replacement restore controls are obsolete.
- Bidirectional CRDT and wall-clock sync cursors are replaced by idempotent one-way writes and opaque
  Pi read cursors.
- Device CSV/clinical-document imports require Pi ingest or a future dedicated contract; they are not
  smuggled into the thin client.

## Sanitized fixtures

`Fixtures/GEN1/` contains synthetic quantity, sleep-category, and journal records. They use fixed
example UUIDs, UTC timestamps, `com.example.fixture.*` provenance, and plainly synthetic text. They
contain no exported HealthKit data, production endpoint, credential, or personal identifier.
