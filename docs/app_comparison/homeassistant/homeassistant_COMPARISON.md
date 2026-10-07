# Home Assistant versus AnxietyWatch v1 and AnxietyWatch2 — comparison

**Candidate:** Home Assistant (shortlist rank 9 of 10)
**Slug:** `homeassistant`
**Research date:** 2026-10-06
**AnxietyWatch2 baseline:** the V2 tree at the commit range recorded in `../PROCESS_STATE.md`
**Home Assistant baseline:** tag **2026.9.4**, commit `9212531f40a0b7b23229a90d688dd79d9dfccff4` (2026-09-27)

**Evidence classes used throughout.** `[CLAIM]` public claim · `[SRC]` static finding, published source at
the pinned commit · `[DOC]` documented by the project, not verified in code · `[3P]` third-party metadata ·
`[INFER]` reasoning, labelled · `[UNKNOWN]` not established. **There are zero runtime observations in this
report.** Nothing was installed or executed.

---

## 1. Executive finding

**Home Assistant does not compete with AnxietyWatch.** It measures no body, makes no physiological claim,
is unregulated, and has no clinical function. The whole-product verdict is a non-contest, and this report
does not manufacture one.

**The subsystem verdict is the opposite, and it is uncomfortable for nobody but AnxietyWatch.** Home
Assistant has run the exact platform shape V2 is building — a local, always-on collector on small hardware,
with a local database, a retention policy, an alarm path and an operator console — for over a decade,
across millions of instances, as a non-profit. Reading its source produced five findings, and three of
them are load-bearing for V2's own design rules.

**Finding 1 — the alarm path has the exact failure mode V2 forbids.** Home Assistant's state machine has
first-class `unknown` and `unavailable` states (`const.py:303,313`) `[SRC]`. Its most mature alarm
primitive, the `alert` integration, has **no branch for either**. The alert ends whenever the watched
state differs from the configured alarm state (`entity.py:98`) `[SRC]`, and on that transition, if a
message had been sent, it delivers the user's own `done_message` (`entity.py:115-126,148`) `[SRC]`. A
sensor going offline therefore reads as *the problem is resolved*. **A missing signal reassures — in a
platform that already has the missing state available to test for.** That is the single sharpest external
vindication in this package of V2's rule that a missing signal must hold state, and it is also the
strongest warning: the correct three-state model lives in the core, and one equality test downstream
destroys it.

**Finding 2 — the three-state model exists and is lost twice.** It is present in the state machine
(`const.py:303,313`) and in the documentation (`last_reboot`'s "state will be unavailable if the timestamp
cannot be determined") `[DOC]`, and it is lost at the alarm layer (Finding 1) and at storage: `unknown`
and `unavailable` are written into `states.state` as those literal strings, a removed state is written as
the **empty string `""`**, and the column is a nullable `String(255)` with no unit and no provenance
(`db_schema.py:422,478-490`) `[SRC]`. Three distinct absences collapse onto three string values inside one
untyped column. **The shortlist's hypothesis that `a general-purpose hub has no three-state honesty
model` is refuted** (`../CANDIDATE_SHORTLIST.md:317`) and the correction is recorded here and in
`PROCESS_STATE.md`.

**Finding 3 — retention is a scheduled hard delete for history and an upsert for aggregates.** Raw
`states` and `events` rows are deleted once older than `purge_keep_days`, whose default is **10**, and
short-term statistics are deleted in the same pass (`purge.py:55,498-511`; `recorder/__init__.py:105`)
`[SRC]`. There is no tombstone and no record of what was removed. Hourly `statistics` are the opposite: a
unique index on `(metadata_id, start_ts)` makes each entity-hour an **upserted row** (`db_schema.py:696-722`)
`[SRC]`, and nothing in the scheduled purge deletes them, so long-term aggregates are effectively kept
forever. The aggregate layer also carries two things V2 does not have — a **coverage weight**
(`mean_weight`) stored beside the mean, and **circular-mean** SQL helpers (`statistics.py:144-162`)
`[SRC]`. Both are recorded as positives.

**Finding 4 — a delivery-confirmation mechanism exists and stops after one hop.** Local push requires the
app to confirm within `PUSH_CONFIRM_TIMEOUT = 10` seconds; on timeout the channel is torn down and a single
`fallback_send` runs (`push_notification.py:10,33-62`) `[SRC]`. Receipts are not persisted, there is no
second fallback, no escalation, and no failure record. V2's supervised alarm and hardware dead-man switch
are **V2 specified target and not built**. Neither system has a finished answer to *what if the fallback
also fails and nobody notices?*, and the report says that rather than scoring a win.

**Finding 5 — one file in the inspected release does not parse.** `components/sensor/recorder.py:239`
reads `except ValueError, TypeError:`; `ast.parse` fails with "multiple exception types must be
parenthesized", and the bytes are identical in the checkout, the git object database, the published tag
and on `dev` (`ea8cf79e…`) `[SRC]`. This is a static fact about a file. Its runtime consequence is
**inference**, is labelled as such everywhere it appears below, and is not used as a rhetorical weapon.
The module is imported by its own test file (`test_sensor_recorder.py:48-52`, 56 tests), and the same idiom
appears correctly five lines below (`:244-249`, `with suppress(ValueError, TypeError):`) `[SRC]`. Ruff
selects rule `E` with no ignore for this path while the only `"syntax-error"` entry lives in the separate
pylint section `[SRC]`. **Why CI appears not to have caught it is unexplained; the tension is recorded and
left unresolved.**

**What the report concludes.** The two systems do not compete; they share a platform layer, and in that
layer Home Assistant supplies (a) evidence that a three-state model is not sufficient by itself, (b) two
concrete numeric techniques worth copying, and (c) a caution about Linux Bluetooth that V2 already tracks
and should keep tracking. The direction of learning is almost entirely one-way — and it is not, in the
main, toward V2.

---

## 2. Product and availability

| | Home Assistant | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| Category | Home-automation platform | iOS anxiety/health tracker | Pi-hosted vitals collector + clients |
| Measures a body | **No** | Yes (HealthKit, manual, CPAP) | Yes (CPAP, BLE, oximeter, Polar H10) |
| Regulatory claim | **None** | None | None |
| Owner | Open Home Foundation (non-profit) | Individual project | Individual project |
| Apps | iOS `io.robbie.HomeAssistant` v2026.9.3, free, min OS 16.4 `[3P]`; Android | iOS (SwiftUI) | iOS client + Rust console/API |
| Hub | Self-hosted: HAOS / Container / Core / Supervised; HA Green, Connect ZBT-2/ZWA-2 | None | Raspberry Pi 5 `[V2 current]` |
| Licence | Apache-2.0 `[3P]` | — | — |
| Paid surface | Optional Home Assistant Cloud, out of scope of the project privacy policy `[DOC]` | None | None |
| Scale | 91,266 stars, 38,804 forks `[3P]`; "over 1 million users" `[CLAIM]` | personal | personal |

The asymmetry that matters for the rest of the report: Home Assistant's *breadth* (1,500+ integrations
claimed `[CLAIM]`) sits on top of a deliberately shallow data model — an entity has one current state and
a log of state changes. AnxietyWatch2's narrowness sits on top of a deliberate, typed measurement model.
Neither is a deficiency; they are answers to different questions. But it means every comparison below is
about **the shape of the data**, not about who has more features.

---

## 3. System boundary and architecture

```
ANXIETY WATCH 2  (target shape)                   HOME ASSISTANT 2026.9.4 (as read)
──────────────────────────────────               ────────────────────────────────────────────
 source (ResMed Air11, BLE,                       companion app (Nabu Casa, Inc)
 oximeter, Polar H10)                             last-value sensors, incl. health_connect_*
        │  bytes                                        │  state + last_updated
        ▼                                                ▼
 vitals-decode  (pure, no_std)                    state machine  entity_id → state:str≤255
 decode → Option<T>, reject+name                  reserved: "unknown" | "unavailable"
        │                                                │  state_changed events
        ▼                                                ├──────────────────────┐
 vitals-core spool (COPY binary)                  ┌ alert ────────────┐  ┌ automation ┐
        │                                          │ eq-test on state  │  │ triggers   │
        ▼                                          │ no absent branch  │  └────────────┘
 TimescaleDB  samples  APPEND-ONLY                 │ in-memory firing  │
 (source, type, time, value)                       └───────────────────┘
 tombstone rows; no UPDATE/DELETE                        │
 gap-fill via time_bucket_gapfill; no locf        ┌ recorder ─────────────────────────────────┐
        │                                          │ states  String(255) · NO unit · NO prov.  │
        ▼                                          │   purge deletes rows > purge_keep_days=10 │
 alerting (supervised alarm — V2 spec target)      │ statistics         hourly  UPSERT         │
 operator console (V2 spec target)                 │ statistics_short_term  5-min, purged      │
                                                  │ statistics_meta    units, name            │
 habluetooth (3rd-party) — watchdog values         └───────────────────────────────────────────┘
 NOT INSPECTED [UNKNOWN]                           habluetooth (3rd-party) [UNKNOWN]
```

Two boundaries the diagram makes explicit:

- **Home Assistant's absence handling is a layer property, not a schema property.** The states are
  reserved constants; the storage column is untyped; the alert layer ignores them. Nothing propagates the
  concept end-to-end.
- **V2's absence handling is a schema property.** `VitalsSource`/`VitalsType` are generated enums, the
  validity decision happens at the decoder, and the store is append-only with tombstones. The comparison
  is between a convention and a type.

---

## 4. Data path and source comparison

| Aspect | Home Assistant `[SRC]`/`[DOC]` | AnxietyWatch v1 | AnxietyWatch2 |
|---|---|---|---|
| Unit of data | entity state (string ≤255) + attributes JSON | typed models, HealthKit samples | `(source, type, time, value)` in `samples` |
| Identity | `entity_id`; no per-sample device identity | per-sample model | source × type × time in the registry |
| Unit location | per-sensor on the doc page; `statistics_meta` for statistics; **none on `states`** | per-model | **one canonical unit per type, converted once at the decoder** |
| Value domain | free string; `unknown`/`unavailable` reserved | Swift types | registry range, validated at decode |
| Types | ~1,500 integrations, no central type registry for measurements | fixed enum-ish set | `vitals/registry.toml`, generated enums |
| Health signals | **26 Health Connect sensors**, last value, 30 days `[DOC]` | HealthKit | registry types incl. 17 Air11 types |
| Derived values | hourly statistics; circular means; coverage weight | correlation engine | analysis layer (V2 scope) |

The single sharpest structural contrast: **Home Assistant has no type registry**. A sensor's unit, range
and meaning live in documentation and in an integration's Python, not in a schema. AnxietyWatch2's
registry exists precisely to make that impossible — "the unit and valid range come from
`vitals/registry.toml`; never hardcode either." Home Assistant demonstrates the cost of the alternative
in miniature: a 26-row documentation table that spells the same unit two ways (`celsus` beside `celsius`)
`[DOC]` and lists `steps`, `floors` and `kcal` as units alongside `mmHg` and `mg/dL`, with conversion left
to whatever renders it. The typo is a documentation defect, recorded as such; the mixed-unit list is the
architectural point.

---

## 5. Claim-versus-documented-contract reconciliation

Full reconciliation is in `homeassistant_IMPLEMENTATION_INSPECTION.md` §7: **50 checkable claims — 31
match · 10 documented-only · 2 partial · 7 outside classes.** The rows that matter for the verdict:

| Claim | Verdict | Basis |
|---|---|---|
| "local control and privacy first" | match | Apache-2.0 source, self-hosted architecture as read |
| `unknown`/`unavailable` are real states | match | `const.py:303,313` |
| Alert has three states, documented as idle/on/off | match | docs table matches `entity.py:83-89` exactly |
| Alert ends when the watched state changes away, incl. to absent, and sends `done_message` | match | `entity.py:98,115-126`; no branch for either absent state |
| Alert development is frozen | match | `entity.py:3,33` |
| `purge_keep_days` default 10; purge deletes states/events/short-term stats | match | `purge.py:55,498-511`; `recorder/__init__.py:105` |
| Removed state stored as `""`; absent states stored as literal strings | match | `db_schema.py:478-490` |
| Statistics upserted on `(metadata_id, start_ts)`; `mean_weight` records coverage | match | `db_schema.py:663,696-710` |
| Local push confirms within 10 s, else one fallback; no escalation or persisted receipt | match | `push_notification.py:10,33-62` |
| Health sensors are last-value, 30 days, units per sensor incl. one misspelling | match (docs) | companion reference table |
| `sensor/recorder.py` fails to parse and therefore cannot load | **partial** | parse failure verified four ways; the *consequence* is inference |
| Alert state does not survive a restart | **partial** | in-memory fields and `cancel_on_shutdown=True` verified; the component's state-reproduction file was **not read** |
| "1500+ integrations"; "top open source project by contributors in 2025"; "over 1 million users"; `averageUserRating = 0` | outside classes | marketing counts, superlatives, and an unresolved ratings-field ambiguity — none enter a tally |
| The hypothesis that the recorder checks absent states explicitly | outside classes | **refuted**: no `STATE_UNAVAILABLE`/`STATE_UNKNOWN` reference exists in `components/recorder/` |

**A documentation-versus-code tension worth naming.** The alert documentation's three-state table is
*accurate* — it describes exactly what the code does. The defect is not a false statement; it is a true
description of a two-state rule inside a platform that has three states. That is the class of
documentation defect this package is built to surface: the page answers the question it was asked and not
the one the reader has. **Nothing in the documentation claims the alert handles an absent watched entity.**

---

## 6. Feature matrix

Columns: **v1** (AnxietyWatch v1) · **v2 now** (exists in `./` today) · **v2 target** (specified in
`openspec/`, not shipped) · **HA** (Home Assistant 2026.9.4) · **evidence/maturity**.

| Capability | v1 | v2 now | v2 target | HA | Evidence / maturity |
|---|---|---|---|---|---|
| Typed measurement series with one canonical unit | per-model | registry + generated enums | — | **No** (free-string state; no unit on `states`) | `[SRC]` HA; V2 registry |
| Append-only store with tombstones | No | **Yes** (`samples`) | — | **No** (hard delete; upsert aggregates) | `[SRC]` HA; V2 amendment 01 |
| Three-state honesty (good / bad / not measured) | No | **Yes** | — | **Partial** — real in the state machine, absent in alert and storage | `[SRC]` |
| Sentinel rejection, never clamp | No | **Yes** (`0x07FF`/`0x0800`) | — | **Partial** — substitutes `unknown` for an invalid *length*, does not parse values | `[SRC]` `core.py:2500-2507` |
| Quarantine with a named reason | No | **Yes** | — | **No** | `[SRC]` HA |
| Provenance per record | No | **Yes** | — | **No** (entity_id only) | `[SRC]` HA |
| Retention policy | manual | **Yes** | — | **Yes** — purge after 10 days, no tombstone | `[SRC]` |
| Coverage weighting in aggregates | No | No | — | **Yes** (`mean_weight`) | `[SRC]` — **HA is ahead** |
| Circular statistics | No | No | — | **Yes** (`circular_mean`) | `[SRC]` — **HA is ahead**; no V2 type is circular today |
| Gap-fill without interpolation | No | **Yes** (`time_bucket_gapfill`) | — | No named equivalent; sparse history | `[SRC]` HA; V2 amendment 01 |
| Delivery confirmation for notifications | No | No | **Yes** | **Yes** — 10 s confirm, one fallback | `[SRC]` |
| Supervised alarm path | No | No | **Yes** | **No** | V2 target; HA has repeats + ack, no supervision |
| Hardware dead-man switch | No | No | **Yes** | **No** | V2 target; HA alert is in-memory and restart-unsafe |
| Alert acknowledgement | Yes (manual) | — | — | **Yes** (`can_acknowledge`; explicit error when disabled) | `[SRC]` `entity.py:201-206` — **positive** |
| BLE scanner liveness | — | **Yes** (V2 scope) | — | **Yes**, external package; 5-min device staleness | `[SRC]` const; watchdog values `[UNKNOWN]` |
| Always-on local operation | No (phone) | **Yes** (Pi) | — | **Yes** | `[SRC]` HA architecture as read |
| Operator console | No | No | **Yes** | **Yes** | documented; console not inspected `[UNKNOWN]` |
| Health/physiological sensors | **Yes** | **Yes** | — | **Reports only** (26 Health Connect types, last value) | `[DOC]` |
| Analysis of health signals | Yes (CNS pipeline, v1) | **Yes** | — | **No** | `[SRC]`-grounded absence claim for HA |

Read the matrix by row, not by column count. Home Assistant wins the aggregation and coverage rows
outright. V2 wins the typing, provenance, append-only, tombstone and quarantine rows outright. The alarm
rows are split: Home Assistant has a *built but shallow* mechanism; V2 has a *specified but unbuilt*
deeper one — and the matrix must not be read as V2 winning a row it has not built.

---

## 7. Derived values versus stored signals

**Home Assistant.**

- Stored: entity states as strings; a log of changes; sparse history that is deleted after 10 days
  `[SRC]`.
- Derived: hourly `statistics` with `mean`, `mean_weight`, `min`, `max`, `sum`, `state`, `last_reset_ts`,
  upserted on `(metadata_id, start_ts)`, plus 5-minute short-term rows and circular-mean helpers `[SRC]`.
- **No analysis of the health signals.** Nothing in the platform scores, correlates or interprets the
  Health Connect values; the only inference in the path is performed by Google and attributed to it
  (`[DOC]` "All data is provided by Google" for sleep confidence/segment).
- **Consequence for the comparison:** the derived/stored boundary is drawn at *aggregation*, not at
  *interpretation*. Home Assistant has no layer that must decide whether a derived number is a
  measurement — which is exactly the layer V2's registry and three-state rule govern.

**AnxietyWatch2.** Stored signals are typed and validated at the decoder; derived values live in the
analysis layer, which is where the rule that a decoder must not clamp an out-of-range value and must not
coerce a sentinel has to keep holding.
The comparison point is that Home Assistant never had to solve it, so it is not a model here either way.

**Two transferable techniques, recorded as positives.**

1. **`mean_weight`.** Storing a coverage weight next to the mean is a simple, honest way to keep "this
   hour was half-covered" attached to the aggregate. V2's gap-fill rule solves a related problem
   differently; the idea of carrying the coverage explicitly is worth considering.
2. **Circular mean.** `circular_mean` with `weighted_sum_sin`/`weighted_sum_cos` (`statistics.py:144-162`)
   `[SRC]` is the correct treatment for a circular quantity. **No current V2 registry type is circular**,
   so this is recorded as a technique V2 is not currently wrong about — not as a missing feature. If a
   circular type is ever added, this is prior art.

---

## 8. Real-time, overnight and failure-mode comparison

| Scenario | Home Assistant (as read) | V2 |
|---|---|---|
| **Overnight sensor stream drops** | BLE device goes `unavailable` after 5 min (`bluetooth/const.py:34`) `[SRC]`; if an alert watches that entity, the alert **ends and may send `done_message`** (`entity.py:98,115-126`) `[SRC]`. **The missing signal reassures.** | Rule: the missing signal **holds state**; never reassures, never escalates alone |
| **Phone unavailable** | Hub unaffected; companion app is transport only `[INFER from architecture]` | Client is not the collector for v2; Pi is |
| **Internet / cloud unavailable** | Local operation continues; local push is attempted first, cloud fallback simply unavailable (`push_notification.py:33-62`) `[SRC]` | Local-first by design |
| **Reading stale** | Only the BLE device path has a documented staleness bound (5 min); health sensors are "last recorded" with no stated staleness rule `[DOC]` — see H8 | Explicit staleness policy in the three-state model |
| **Reading malformed** | Filtered by parseability, not by a named rejection (`sensor/recorder.py:226-249`) `[SRC]`; over-long states replaced by `unknown` (`core.py:2500-2507`) `[SRC]` | Reject, name the reason, quarantine |
| **Reading out of range** | No range concept for a state it does not parse anyway | Registry range, validated at decode; never clamped |
| **Reading absent** | First-class states; stored as literal strings; **not** handled by the alert | First-class, schema-enforced, holds state |
| **Alert path technical failure** | `ServiceNotFound` logged and retried next interval (`entity.py:183-193`) `[SRC]`; no health state, no escalation, no record | V2 specified target |
| **Restart while alerting** | `_firing`/`_ack`/`_next_delay` are in-memory; callback `cancel_on_shutdown=True` → alert returns to `idle` `[SRC]`; `reproduce_state.py` **not read** (H3, gap) | V2 specified target: supervised alarm; dead-man switch |

**The scenario the report exists to answer is the first row.** Walking it precisely, with evidence
classes kept separate:

- `[SRC]` Home Assistant's state machine will mark the device `unavailable` — the correct three-state
  behaviour.
- `[SRC]` The alert entity's condition logic is an equality test against the configured alarm state, with
  no branch for absent states.
- `[INFER]` Therefore, in a running instance, an alert watching that device stops firing and emits the
  resolution message. **This step is inference from the code, not an observation.** It is the inference a
  reader should test first.
- `[SRC]` The platform declares the alert integration's development frozen, so a reader knows not to
  expect the branch to be added.
- `[DOC]` The documentation's three-state table omits the absent case entirely.

That chain is why the finding is durable rather than fragile: it rests on a file read in full, a
documented state table, and a platform-declared freeze — not on a single line.

**Second-strongest scenario: the restart.** An alert that a user is relying on does not survive a reboot
of the hub, and there is no dead-man switch to notice. V2's answer is a **specified target**, not built.
The comparison states the gap on both sides.

---

## 9. Provenance, missingness and reproducibility

**Home Assistant.**

- **Provenance:** `entity_id` and a context ID. No device identity per sample, no quality flag, no
  algorithm version, no reason-for-change. A value in `states` cannot say which physical device produced
  it beyond which entity wrote it. `[SRC]` `db_schema.py:404-437`
- **Missingness:** three string values in one untyped column — `"unknown"`, `"unavailable"`, `""` — plus
  the absence of a row. Four situations, no enum, no NULL semantics on the value. `[SRC]`
- **Reproducibility:** raw history is deleted after 10 days by default `[SRC]`; hourly aggregates persist.
  **A user cannot reconstruct what the raw values were, or that anything was removed**, because the
  removal leaves no trace. That is the concrete cost of no tombstone, stated without advocacy.
- **Reproducibility positives:** circular statistics are implemented correctly for circular quantities;
  aggregate coverage is weighted; the platform documents that a state is `unavailable` when a timestamp
  cannot be determined, which is an honest statement about the limit of a derived value.

**AnxietyWatch2.**

- Provenance is a registry-level `(source, type)` pair, not an entity name.
- Missingness is schema-enforced with three states, sentinel rejection, no clamping, and quarantine with
  a named reason.
- Reproducibility is protected by append-only `samples` and tombstones: removal is itself a record.

**The comparison's sharpest question, put plainly:** in five years, an operator asks *what did the
oximeter actually report on that night, and was any of it thrown away?* Home Assistant answers *here are
the hourly means, and no record exists of what was purged.* V2 intends to answer *here are the samples,
here is the tombstone.* That difference is the whole argument for the append-only rule, and it is
supplied here by a working system's *absence* of the mechanism rather than by a design assertion.

---

## 10. Local/cloud/privacy and operational cost

**Home Assistant.**

- **Local-first is real and architecturally visible:** the state machine, recorder and alert path all run
  in the user's instance; the companion app is transport; the recorder is a local database `[SRC]`.
- **Telemetry is opt-in with published retention:** "Nothing is sent unless the user opts in"; analytics
  in a self-hosted Plausible for "a maximum of 60 days"; Cloudflare Analytics disabled while Cloudflare
  still processes "IP addresses and user-agent information" — the project states both halves `[DOC]`.
- **Three entities, three surfaces:** the non-profit Foundation owns the project; **Nabu Casa, Inc**
  publishes the apps; **Home Assistant Cloud** is a separate legal entity and is **outside the scope of
  the privacy policy** `[DOC]` `[3P]`. The "non-profit, can't be sold" claim attaches to the project, not
  to every surface a user touches. Recorded as a scope fact; no impropriety is implied.
- **Push and crash reporting "may rely on third-party infrastructure"** and the third parties are **not
  named on that page** `[DOC]`. That is a genuine privacy gap and it is recorded as one.
- **A self-limiting claim:** the iOS description itself recommends Home Assistant Cloud as "the most
  secure and simple way" to get remote access `[CLAIM]` — an honest statement that "it runs locally" has
  a boundary, made by the project rather than extracted from it.
- **Operational cost:** free to self-host; hardware is the user's. Cloud is optional and its price was
  **not retrieved** (`[UNKNOWN]`).

**AnxietyWatch2.** Local-first is the design premise; the network-trust and enrollment amendments
(`09`–`11`) are V2 current or V2 specified target depending on the change, and are labelled as such by
the reader of those files — this report does not re-verify them.

**Where Home Assistant instructs V2 most on this axis:** it shows that local-first, privacy-first,
non-profit positioning is compatible with a real, honest, *unnamed-third-party* gap in the push path, and that
disclosing the cloud carve-out is what keeps the overall posture honest. The lesson is about disclosure
discipline, not about architecture.

---

## 11. Alerting and the alarm path

This is the section the candidate was selected for. Everything in it is `[SRC]` unless marked.

**What Home Assistant has built.**

- An alert entity with three documented states: `idle`, `on`, and `off` = "the condition for the alert is
  true but it was acknowledged" (`entity.py:83-89`; docs table identical) `[SRC]`+`[DOC]`.
- Repetition from a configured list of minute delays, **saturating** at the last value
  (`entity.py:128-139`) `[SRC]` — a simple, predictable, bounded schedule.
- Acknowledgement, with an **explicit failure that names its blocker** when acknowledgement is disabled:
  `ServiceValidationError("This alert cannot be acknowledged")` (`entity.py:201-206`) `[SRC]`. Recorded
  as a **positive** and a direct analogue of V2's "guards must say why".
- A completion message armed only after a message has actually been sent (`_send_done_message`,
  `entity.py:148`) `[SRC]` — a genuine piece of care: the platform will not announce resolution of an
  alert it never raised.
- Delivery confirmation on the local push channel with a 10-second timeout and one fallback
  (`push_notification.py:10,33-62`) `[SRC]`, and an explicit `False` for a receipt that matches nothing
  `[SRC]`.

**What it does not have.**

- **No absent-state branch** (Finding 1) — the failure mode V2's rule forbids.
- **No escalation to a second recipient** and **no second fallback** `[SRC]`.
- **No persisted alarm or receipt state**; the alert's firing state is in-memory and does not survive a
  restart `[SRC]`.
- **No delivery-health state**: `ServiceNotFound` is logged and retried at the next interval and nothing
  else happens (`entity.py:183-193`) `[SRC]` — no record, no escalation, no operator signal.
- **No dead-man switch.** Nothing notices that no alert has fired when one should have.

**The honest comparison.**

| | Home Assistant | AnxietyWatch v2 |
|---|---|---|
| Alarm primitive exists | **Yes, built** | No — supervised path is **V2 specified target** |
| Handles an absent signal safely | **No** — ends the alert and may announce resolution | **Specified:** missing signal holds state |
| Acknowledgement with an explicit refusal | **Yes** | — |
| Delivery confirmation | **Yes**, 10 s, one fallback | **Specified** |
| Escalation / second channel | **No** | **Specified** |
| Survives a restart while firing | **No** | **Specified** (supervised alarm + dead-man switch) |
| Delivery-health visibility | **No** | **Specified** |

**The verdict, stated carefully.** Home Assistant built the alarm plumbing V2 has only specified — and
built it in a way that fails V2's central rule at the first absent reading. V2 has specified a deeper
design and built none of it. **Neither is a finished answer.** What the candidate contributes is not a
score; it is the evidence that the rule must be enforced and tested *at the alarm layer*, and that a
correct state machine upstream is not sufficient. That is a narrow, testable, and load-bearing
conclusion, and it is the reason this candidate is in the package.

---

## 12. User-facing consequences

1. **A user of Home Assistant can be told a problem is over because the sensor disappeared.** In a leak,
   door, temperature or — with a health sensor wired in — a monitoring alert, the platform's own
   resolution message can be delivered on the transition to `unavailable` `[SRC]`+`[INFER]`. The user has
   no way to see, from the alert, that the signal went away rather than normalised. This is the
   user-visible form of Finding 1.
2. **A user cannot audit what was deleted.** After 10 days, raw history is gone with no tombstone `[SRC]`.
   An operator asking *what did the sensor report that night* gets an aggregate and no record of removal.
3. **A user relying on an alert across a reboot loses it.** In-memory firing state plus
   `cancel_on_shutdown=True` means the alert returns to `idle` `[SRC]`, with the caveat that the
   component's reproduction path was not read (H3).
4. **A user gets real local-first behaviour.** The core promise holds: state, storage and alerting run in
   their instance; telemetry is opt-in; the recorder is theirs `[SRC]` `[DOC]`.
5. **A user reading the health sensors gets a broad but shallow list.** 26 types, last value, 30 days, no
   analysis, one misspelled unit `[DOC]`. Useful as a dashboard; not a measurement record.
6. **A user reading the privacy policy gets an honest boundary and one real gap.** The cloud carve-out is
   disclosed; the push/crash third parties are not named `[DOC]`.

For AnxietyWatch2's own users, the transferable consequence is a **test requirement**: V2 should have a
test that asserts an alert does *not* fire a resolution path when its input goes to the not-measured
state. Home Assistant demonstrates precisely which defect such a test catches, in a codebase where the
state existed to be checked.

---

## 13. Limitations, unverified claims and evidence gaps

**This report's own limits.**

1. **No runtime observation.** Nothing was installed, started or contacted. Every behavioural statement
   about Home Assistant is a static reading or an inference, labelled.
2. **Scope of the source inspection is narrow.** Eleven files were read out of 110 in a sparse checkout of
   one release; the platform has ~1,500 integrations. `components/automation/`'s trigger and condition
   semantics — the other half of a real alarm path — were **not read** (gap G9). That is the largest
   single limit on §11.
3. **`alert/reproduce_state.py` was not read**, so the restart findings (R3/R34) rest on entity-level
   evidence: in-memory fields, `Entity` inheritance, and `cancel_on_shutdown=True`. This is the weakest
   source-derived claim in the report and is flagged twice.
4. **The syntax error's consequence is inferred, not observed** (Finding 5). Its presence is verified four
   ways and its import by the module's own test file is verified; what a running instance does with the
   module is not. **Why the project's CI appears not to have caught a non-parsing file is an unresolved
   tension** (gap G14), and the function containing the error is referenced by no test (gap G15).
5. **The REST/WebSocket API contracts were not read** (H12); the external interface is not compared.
6. **Deployment, backup, upgrade and onboarding were not inspected** (H13), despite the shortlist naming
   Home Assistant the benchmark for them. Only the documentation's story exists here, and it is marked
   `[DOC]`.
7. **`habluetooth`'s watchdog values are unknown** (H4); only the re-export and
   `UNAVAILABLE_TRACK_SECONDS = 300` are verified.
8. **Android/Play metadata was not retrieved**; only iOS store metadata exists `[3P]`.
9. **Nabu Casa cloud pricing and its separate privacy notice were not retrieved.**
10. **The `averageUserRating = 0` field is an unresolved ambiguity**, not a discrepancy, and is excluded
    from all tallies.
11. **"1500+ integrations" is single-sourced** to the home page; the integrations index states no count.
12. **"Top open source project by contributors in 2025" names no ranking source.**
13. **The correction to the shortlist is recorded in this package but not written back into
    `CANDIDATE_SHORTLIST.md`** — a reader of that file alone still sees the refuted hypothesis.

**Claims in this report that are inference, not fact** — listed so they are not mistaken for evidence:

- That a running instance sends the `done_message` when a watched sensor goes absent. *(Inference from
  `entity.py:98,115-126`; not observed.)*
- That the parse failure prevents the statistics sensor path from loading. *(Inference from a file that
  does not parse; not observed.)*
- That an alert is lost across a restart in practice. *(Inference from in-memory fields; the reproduction
  path was not read.)*
- That no inference other than Google's is made in the health path — an absence claim grounded in the
  components read, not in the whole platform.

---

## 14. Balanced conclusion

**Whole-product competition: none.** Home Assistant and AnxietyWatch share no product surface. Home
Assistant measures no body, claims no physiological result, is not regulated, and does not compete for a
user who wants a CPAP-linked anxiety or vitals record. Reversing it: AnxietyWatch is not a home-automation
platform and does not compete for a user who wants to turn on a lamp with a motion sensor. Of the ten
candidates in this package, this is the one where the whole-product verdict is least interesting.

**Subsystem competition: real, one-directional, and mostly to AnxietyWatch's benefit.** In the platform
layer they share — local collection, local storage with retention, local alarm path, operator console,
Raspberry Pi target — Home Assistant is a decade ahead in operational maturity and supplies exactly three
things this package needs:

1. **Evidence that a three-state model is necessary and not sufficient.** The absent states exist in the
   core and are destroyed by one equality test downstream. V2's rule is thereby vindicated *and* given a
   concrete failure mode to test against. This is the strongest external support in the package for the
   rule, because it comes from a platform with no incentive to confirm it.
2. **Two transferable numeric techniques V2 does not have** — coverage weighting (`mean_weight`) and
   circular means — recorded as positives, with the explicit note that V2 has no circular type today and
   is therefore not currently wrong.
3. **Independent corroboration of a risk V2 already tracks.** Home Assistant, on its own authority,
   recommends an ESP32 proxy over a locally attached adapter because "Linux kernel updates have previously
   broken Bluetooth functionality." V2 is building a Pi + local BlueZ path. The recommendation is
   evidence about the general risk, not about V2's implementation, and it is recorded as such.

**In the other direction, V2's design is ahead of Home Assistant's on the rows that matter most for a
physiological record:** typed series with one canonical unit, provenance, append-only storage with
tombstones, schema-enforced three states, sentinel rejection, no clamping, quarantine with a named
reason. Home Assistant's free-string state in a nullable `String(255)` column is the counter-example that
makes the registry look less like over-engineering and more like a boundary.

**And Home Assistant is ahead of V2 on rows V2 has not built:** a working alarm primitive with
acknowledgement, an explicit refusal when acknowledgement is disabled, a delivery-confirmation protocol
with a bounded timeout, and a real retention policy that runs. V2's deeper alarm design is a **specified
target**, and this report does not pretend otherwise.

**The one-sentence verdict.** Home Assistant is not a competitor and not a template; it is a mature
witness that a correct three-state model dies at the first downstream equality test, and the correct
response is not to copy its platform but to test V2's rule where Home Assistant's failed.

---

## Related files

- `homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` — public claims, the 26-sensor table, the claim ledger
  `H-1…H-35`, and the shortlist correction (§14.4)
- `homeassistant_IMPLEMENTATION_INSPECTION.md` — pinned tree, artifact hashes, the syntax-error
  verification chain, and the 50-claim reconciliation
- `PROMPT_2.md` — the candidate-specific Prompt 2 that produced this report
- `../CANDIDATE_SHORTLIST.md` — entry 9 (see limitation 13)
- `../RESEARCH_METHOD.md` — inspection modes and the standing sub-checks this report follows
- `../PROCESS_STATE.md` — candidate status and residual uncertainty
