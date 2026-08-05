# What to take from generation 1 — and what not to

**Date:** 2026-08-05 · **Status:** draft for review
**Companions:** [`SPEC-admin-console.md`](SPEC-admin-console.md) · the architecture at
[`../specs/2026-08-03-pi5-vitals-core-design.md`](../specs/2026-08-03-pi5-vitals-core-design.md)

A four-agent audit of generation 1 produced an inventory: 26 admin routes, 17 templates, 40 tables,
plus the iOS surfaces and the ops tooling. That inventory is **not a migration checklist.** This
document is the judgement pass — each concept assessed against what gen-2 is actually *for*, with a
verdict and a reason.

The default answer is **not** "carry it over." Gen-2 changed three things that invalidate a lot of
gen-1 by construction, and the audit's framing — *"what must not be lost"* — quietly assumes the
opposite.

---

## 1. The three inversions that decide most of this

Everything below follows from these. They come from the architecture doc, not from me.

**1. HealthKit is demoted to source #2 — demoted, not dropped.** The Pi reads Polar RR intervals and
oximetry directly over BLE, on mains power, continuously. That is a better measurement than
HealthKit's and it is available when the phone is asleep, in another room, or dead.

The phone still syncs everything it has — HealthKit, iOS, and Watch data — and that is a **real
source** carrying things only Apple can see: sleep stages, wrist temperature, the Watch's own resting
HR. What changed is its *standing*. It is one source among many rather than the authority, and both
the Watch and iOS streams are **entirely optional**: their absence is a fact about the setup, not a
fault.

That optionality is architectural, not a caveat. Two consequences that bind elsewhere in this project:

- **No derived value may require HealthKit to be present.** Baselines, correlations, and reports are
  computed from whatever sources exist. A missing optional source narrows the evidence; it never
  produces a null that overwrites a real value — the gen-1 failure where authorization loss let
  aggregation write nils over restored data.
- **An absent optional source must not render as a fault.** "I didn't wear the watch" and "the watch
  feed is broken" are different facts and the console must distinguish them. This is the
  `NOT_CONFIGURED` state — declared-absent, excluded from rollups, never a question mark demanding
  attention. Without it, `UNKNOWN` becomes wallpaper and stops meaning anything.

**2. `samples` is append-only with an HLC, and everything else is a view over it.** No `UPDATE`, no
`DELETE`; removal is a tombstone. Merge is commutative, replay is exact, and a bug in a derived value
is fixed by recomputing the view rather than migrating data.

**3. The life-safety path is static binaries under systemd; conveniences live in Docker.** The admin
console is explicitly a convenience — restartable, read-mostly, off the critical path.

---

## 2. Verdicts

`REPLACE` — the capability matters, the gen-1 implementation is the wrong shape for gen-2.
`ADOPT` — port substantially as-is; it earned its keep.
`EXTRACT` — the code is disposable, the *knowledge inside it* is not.
`DROP` — gen-2 makes it unnecessary or impossible.
`DEFER` — real, but not now.

### 2.1 Storage and the data model

| Gen-1 | Verdict | Reasoning |
|---|---|---|
| **`health_snapshots`** — 36-column daily aggregate, hand-maintained by `SnapshotAggregator` | **DROP the table, REPLACE with a continuous aggregate** | This is a materialized view maintained by application code, and every bug it produced follows from that: frozen shells, nils overwriting real values, late-landing overnight data never re-aggregated, three months undetected. A TimescaleDB CAGG recomputes from source and **cannot** hold a value its inputs no longer support. `samples_1min` already exists in `db/`; daily rollups are the same mechanism. This single change deletes an entire bug class rather than porting its workarounds |
| **`quantity_health_samples`** — 250k-row per-sample HealthKit mirror | **DROP the table — keep the data** | The phone **does** push its HealthKit/iOS/Watch history to the Pi, and that is a real source. What dies is the *separate table*: it was a second store with its own provenance columns, its own restore path (which nobody wrote — hence the 224k-row hole), and its own sync semantics. In gen-2 those samples are rows in `samples` with `source = healthkit`, which is exactly what `source` is for; the registry already declares it `id = 2, transport = "push"`. Parallel machinery for data that belongs in the one table everything else is a view over is the defect, not the data |
| **`samples` + `sample_tombstones` + `delta_sync_log`** — the CRDT delta-sync trio | **DROP two, ADOPT one** | `samples` is the design's own substrate and survives. `delta_sync_log` — roughly one ledger row per synced row — exists to reconstruct what changed; with an HLC on every row, **the data is its own change log**. `sample_tombstones` survives as the append-only deletion mechanism, which the design already specifies |
| **`sensor_sessions.rr_archive`** — write-only BYTEA, ~80–120 KB/night, no GET route | **REPLACE** | The intent is right and important — archive raw RR so future HRV measures can be re-derived without re-pairing. But under append-only, raw RR intervals are just `rr_interval` samples in the store, at their natural rate. There is no separate archive blob to have a missing read route for |
| **The 40-table domain schema** (journal, meds, Rx, pharmacies, labs, songs) | **ADOPT, unchanged in shape** | This is not time-series and does not belong in `samples`. It is ordinary relational domain data that works. Move it in Stage 6 with its schema substantially intact. Resist the urge to redesign it — it is the least broken part of gen-1 |
| **`sync_log`** — success-only ledger | **REPLACE with an attempt ledger** | The defect is structural: the insert sits inside the `try`. The replacement is not a better `sync_log`, it is a different table — a row written *before* the work with an expected duration, updated with the outcome. See `SPEC-admin-console.md` §2.3 |

### 2.2 Sync and the phone relationship

| Gen-1 | Verdict | Reasoning |
|---|---|---|
| **`SyncService` cursor drain loop** — `lastSyncDate` advanced after I/O, with its documented race and the `bulkOnly` re-introduction of it | **DROP entirely** | Cursors become HLC positions derived from data already written. There is no wall clock in the sync path to be wrong. This is the design doc's own §5 entry and it is correct — the fix is not a more careful cursor, it is removing the concept that could be wrong |
| **`RestoreFromServer` + the empty-store guard + `restoreGuardBlockers`** | **DROP the mechanism, EXTRACT the lesson** | Append-only + HLC merge means restore *is* a merge and "is the store empty?" is never asked. The guard, its blocker-naming, and the entire class of "a write before the gate bricked restore" go away. **The lesson that survives** — a guard which fails closed must name its blocker — is already generalised into the design's structured-health rule |
| **Per-table importers for restore** (the `QuantityHealthSample` gap: synced up, no way down) | **DROP** | One narrow table with generic replication. There is no per-table importer to forget to write |
| **`SyncFailureNotifier`** — phone-side "sync failed" notification | **REPLACE, and relocate** | The phone cannot notify about a sync it never woke up to attempt. Silence detection belongs on the always-on box. Keep the *idea*, move the owner |
| **Bidirectional CRDT push/pull with `cursor_format_version`** | **DEFER, and probably simplify** | Gen-2 makes the Pi the single system of record and the phone a thin client. Full bidirectional CRDT was solving multi-master reconciliation. Once the phone stops computing and starts rendering a projection, the phone→Pi direction is an enrichment push and the Pi→phone direction is a read. That may not need an HLC merge at all. **Do not port this before Stage 7 decides what the phone actually still writes** |

### 2.3 The analysis and reporting layer — the product's actual point

| Gen-1 | Verdict | Reasoning |
|---|---|---|
| **Correlation engine** — Pearson over 7 signals, `MINIMUM_PAIRED_DAYS = 12`, staleness fingerprint | **ADOPT the statistics, REPLACE the plumbing** | The value here is domain judgement, not code: the minimum-paired-days floor, the p-value discipline, the decision to compare "anxiety on abnormal vs normal days." That is what took thought. The implementation is ~240 lines of Python that could equally be SQL against the hypertable. **Port the constants and the reasoning; rewrite the execution.** One real change: it should not run inline on an HTTP request thread |
| **`ReportGenerator`** — clinical PDF via Core Graphics | **REPLACE, keep the layout decisions** | This is the product's reason for existing — the artifact that turns fifteen minutes with a psychiatrist into a data-informed conversation. But Core Graphics is iOS-only, and under Stage 7 the phone stops computing. The Pi should generate it. **Carry across verbatim:** the section structure, the nil-AHI exclusion, the unfiltered-baseline rule, and the data-completeness note. Those are clinical correctness, not rendering |
| **`DataExporter`** — JSON/CSV over 10+ entity types | **ADOPT** | Export-first is a stated product principle and the sync payload already matches the export format. Cheap to keep, expensive to lose |
| **`job_dispatcher`** — ThreadPoolExecutor DAG, no cancel/retry/requeue, reaper as a page-load side effect | **REPLACE** | The DAG shape is fine; the execution model is not. Threads inside a gunicorn worker die silently on deploy and recover only via a stale-sweep. Gen-2 should use a Postgres-backed job table — which the attempt ledger (§2.1) already is — with a supervised runner. Cancel and retry become possible rather than being absent by accident |
| **The four unmetered AI endpoints** (profile refine, two summary generators, psychiatrist research) with a hardcoded model differing from the dropdown | **ADOPT the features, REPLACE the accounting** | Every billed call goes through one path with one ledger. The hardcoded second model is a bug, not a design |
| **Psychiatrist adverse-information research** — web search on a named real person, persisted, injected into later prompts | **DROP** | The audit flagged this as the highest-liability route in gen-1, and I agree, but not for liability reasons: it stores unverified adverse claims about an identified private individual in a health database and then feeds them to an LLM as context. That is a data-quality problem before it is anything else — it pollutes the evidence chain the product exists to keep clean. The *therapy-schedule and conflict* context that feeds prompts is legitimate and stays |

### 2.4 The admin console itself

| Gen-1 | Verdict | Reasoning |
|---|---|---|
| **Flask + Jinja + one `<style>` block** | **REPLACE — already decided** | `docs/specs/2026-08-03-language-choices.md` picks FastAPI + vanilla HTML/CSS/JS, with the reasoning already written (ASGI-native SSE for job status, OpenAPI generated for the iOS client and dashboard). Nothing to re-litigate |
| **`/admin/data` generic browser** — 14 of 40 tables, `LIMIT` with no `OFFSET`, 500-row ceiling | **REPLACE, and re-scope** | The *idea* — a generic read-only browser — is the highest-value surface gen-1 has, and it is why `/query-prod` exists. But gen-2's shape is different: seven tables today, one of which is a hypertable with two years of raw retention. `OFFSET` pagination is wrong here; keyset on the time index is right, and `COUNT(*)` on the hypertable must never be issued. "All 40 tables" is a gen-1 number and should not be inherited as a requirement |
| **`/query-prod` slash command** — SSH + `docker exec psql`, read-only enforced by prompt text | **REPLACE with a Postgres role** | The command's own docs say to promote repeated queries to a server-side endpoint. A real `pg_read_all_data` role with a statement timeout retires the SSH-to-prod pattern instead of reproducing it |
| **`verify-schema.yml`** — manual CI dispatch to run `alembic current` | **DROP the workflow, ADOPT the readout** | A CI job standing in for a missing admin page. Gen-2 uses sqitch; the deployed change name belongs on the Overview |
| **Single shared `ADMIN_PASSWORD`, no CSRF, global lockout** | **REPLACE** | Not a port. See `SPEC-admin-console.md` §7 |
| **`log_sanitize.py`** — redacts vendor tokens from exception text | **EXTRACT** | 37 lines of Python that will not survive into Rust or FastAPI as code. What must survive is the *rule*: operator-visible error text is sanitised, and there is one place that does it. If gen-2 logging is written fresh without this, credentials start appearing in the journal |
| **Fernet-encrypted settings keyed off `SECRET_KEY`**, rotation unimplemented | **REPLACE** | Amendment 02 already covers encryption at rest. Note the latent bug being escaped: rotating `SECRET_KEY` in gen-1 orphans every encrypted value with no re-encrypt path |

### 2.5 Ingest and devices

| Gen-1 | Verdict | Reasoning |
|---|---|---|
| **`ezshare_sync.py`** — WiFi SD → STR.EDF → CPAP rows, 10-minute loop | **ADOPT the parser, REPLACE the supervision** | The EDF parsing is real work that is already correct. The supervision is not: no `try/finally`, so a parse error leaks a connection and freezes the status string at its last value, while `|| true` in compose hides the exit code. Observed live: it ran a no-op loop for hours, then died nine days ago, invisibly |
| **`ezshare_clock.py`** — persisted whole-day offset for the CPAP's 2008-epoch reset | **ADOPT, but make it visible and versioned** | The correction is necessary and the reasoning is sound. The defect is that the offset is invisible, non-resettable, and applied to a value that is the primary key. Under append-only it should be *metadata applied at read time*, never a mutation |
| **`resmed_sync.py`** — myAir cloud pull with meaningful exit codes | **ADOPT** | Works. Needs supervision and a real UI, not a rewrite |
| **`as11_collector.py`** — asyncio TCP to the CPAP bridge, 5s watchdog, no route/CLI/unit | **ADOPT the collector, ADD supervision** | A collector nobody can observe. Under gen-2 it becomes a systemd unit against the `vitalsd` socket like every other collector |
| **`as11_ws.py`'s `ingest_ts_utc` keying** — replay window keyed on server receipt, not device timestamp | **EXTRACT — this is the highest-value line in the audit** | A deliberate choice so a skewed CPAP clock cannot mask a live desaturation. It is four words of code and a paragraph of reasoning, and reimplementing this subsystem naively loses it silently. Encode as a test, not a comment |
| **EMAY live BLE streaming** (iOS) | **DROP from the phone** | Already superseded — gen-2's `emay-spike` reads the ring directly and the bedside panel displays it. This is the inversion working as intended |
| **Polar H10 background BLE with state restoration, reconnect backoff, 10-min grace** | **EXTRACT, then DROP** | Hundreds of lines of hard-won iOS-specific work — a stable `restoreIdentifier`, a transient-state scan latch, recovery re-binding after a mid-night kill. **All of it exists to work around a constraint gen-2 removes.** A mains-powered Pi that never backgrounds needs none of it. What transfers is the *reconnect-backoff shape* and the grace period, which are about rolling over in bed, not about iOS |

### 2.6 The iOS surfaces

| Gen-1 | Verdict | Reasoning |
|---|---|---|
| **HealthKit authorization + the 4-signal probe grid** | **KEEP ON PHONE — irreducible** | HealthKit auth is iOS-only. But its *state* must reach the Pi: a phone that silently loses health access is invisible from the Pi today, and that exact freeze ran three months undetected. This needs a phone→Pi status beacon that does not exist in either generation |
| **`Rebuild All History`, `Refresh Recent Snapshots`** | **DROP** | Both exist to repair the hand-maintained snapshot table. With a continuous aggregate there is nothing to rebuild |
| **watchOS Quick Log** — colour-graded severity grid, sub-5-second logging during panic | **KEEP ON PHONE, untouched** | This is the product at its most essential and it is on exactly the right hardware. Nothing about gen-2 improves it |
| **Dose-triggered 30-minute follow-up** | **KEEP ON PHONE** | Needs local notifications and the person. Server-side scheduling would be worse |
| **Random check-ins** | **KEEP scheduling on phone, MOVE preferences to the Pi** | Config is config |

---

## 2.7 Platform health stores are a *class*, not a special case

HealthKit is the first platform health store this project ingests. It will not be the last — Android's
Health Connect is wanted later, in the same shape. That is a constraint on decisions being made now,
not a future feature, because getting it wrong means a second ingest path bolted alongside the first.

**What already generalises correctly.** The registry declares *physiological concepts* —
`hr`, `spo2`, `sleep_stage`, `rr_interval` — not vendor identifiers, with a canonical unit per type
and conversion at the decoder. That is exactly the right shape: a Health Connect receiver converts
into the same canonical types and nothing downstream knows or cares which phone it came from.

**What needs deciding before the second platform arrives:**

| Question | Recommendation |
|---|---|
| One source id for "platform health," or one per platform? | **One per platform.** `healthkit` and `health_connect` are different measurement pipelines with different quality characteristics and different gaps. Provenance is the reason `source` exists; collapsing them discards it for no gain |
| Where does platform-specific mapping live? | **A per-platform mapper, a shared receiver.** The transport is identical — a batch of typed samples pushed over the network. Only the *translation* is platform-specific, and it belongs on the same side as the fuzzed wire decoders: pure, `(vendor payload) → canonical samples`, no I/O |
| Sleep stages | **The hard case, and the one to design deliberately.** Apple's stage set and Health Connect's do not align one-to-one. `sleep_stage` needs a canonical enum in the registry with an explicit, documented mapping per platform — and a rule for what happens to a stage that has no canonical equivalent. Do not let the first platform's enum become the canonical one by accident |

**The genuinely unresolved one — sub-provenance.** A HealthKit sample carries its *own* origin:
which app or device wrote it into HealthKit. A resting HR from the Watch, the same value typed in by
hand, and one written by a third-party app are three different facts, and gen-1 tracked this
(`source_bundle_id` / `source_name` / device) and *used* it — it is how oximetry rows deduped against
HealthKit rows.

Gen-2's `source` is a single integer. Collapsing a platform store into one id loses that second
dimension. This needs a home before the HealthKit receiver is written, and the options are: a
sub-source field in the registry, structured `extra` per `(source, type)`, or a separate provenance
column. **The one unacceptable answer is discarding it** — dedupe against directly-read sensors
depends on it, and this project will have both a Polar-measured HR and a HealthKit-relayed HR for the
same minute.

---

## 3. What this deletes from the migration register

`SPEC-admin-console.md` §6 currently reads as "do not lose these." Judged against gen-2's goals,
several entries should not survive as written:

- **The restore/repair reconciliation UX** — gen-2 has no empty-store guard to build a UI around.
- **Per-table restore importers** — one generic replication path, nothing to enumerate.
- **The `HealthSnapshot` re-aggregation triggers** — a CAGG has no manual rebuild.
- **`quantity_health_samples` coverage** — the table should not exist.

And one entry gets *stronger*, not weaker: **the clinical PDF and export path**. Under an
anxiety-tracking product whose stated purpose is walking into an appointment with evidence, that is
not a migration risk item — it is close to the point of the whole system, and it currently has no
gen-2 owner at all.

---

## 4. Open questions this raised

1. **Does the phone still write anything the Pi cannot derive?** Journal entries and medication doses,
   certainly. If that is the whole list, the sync relationship is much simpler than gen-1's CRDT and
   should be designed as such in Stage 7 rather than ported ahead of it.
2. **Where does the correlation engine run?** SQL against the hypertable is attractive and would put
   the computation next to the data. But the statistics discipline is the valuable part and rewriting
   it invites subtle error. Port first, optimise later, with the gen-1 output as a fixture.
3. **Does the domain schema (journal, meds, Rx) go in the same Postgres as `samples`?** Same cluster
   is obvious. Whether it shares a database with a two-year hypertable is a backup-granularity
   question, not a modelling one.
4. **What is the historical backfill plan?** ~224k app-only oximetry rows plus years of journal and
   CPAP history need to land in `samples` and the domain tables once. That is a one-time migration
   with its own verification, and it is not currently a task anywhere.
