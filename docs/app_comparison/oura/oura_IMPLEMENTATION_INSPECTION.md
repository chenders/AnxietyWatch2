# Oura — implementation inspection

**Slug:** `oura`
**Research date:** 2026-10-06
**Phase:** 2A (authorized implementation inspection)
**Outcome:** **Partially performed — documented contract only.** No app binary inspected. No source
inspected. **No endpoint called.** No runtime observation of any kind. The artifact inspected is the
**vendor's own published OpenAPI specification**, which is a *published contract describing intended
behaviour*, not a record of observed behaviour.

**Inspection mode.** `PROMPT1.md` Phase 2A recognises several modes for candidates whose binary cannot
lawfully be obtained. For this candidate:

| Mode | Available? | Used? |
|---|---|---|
| Runtime observation on an authorized emulator | No — the consumer app is gated behind an account, and the ring is required hardware | No |
| Decompilation / static analysis of the app binary | **Prohibited by the vendor's own contract** — Terms §II(ii) | No |
| Vendor-published source (GPL/AGPL) | No — nothing published | No |
| **Vendor-published API reference / output contract** | Yes — `openapi-1.41.json`, 454,152 B | **Yes — read statically** |
| Vendor-published artifact inspection (docs, contracts, trust centre) | Yes | **Yes** |
| Vendor-published research / validation articles | Yes | **Yes** |
| Calling the sandbox or any live endpoint | Lawful in principle; **an explicit authorization decision** | **No — not taken** |

This mode — **published interface contract read as a specification** — was first instantiated for
`sleephq` (Swagger 2.0, 21 paths, 11 definitions). Oura's artifact is the same *class* and a much larger
instance (OpenAPI 3.1, 72 paths, 75 schemas, 274 references). **Every finding below is a statement about
what the document says.** Where this file says "documented", it means *published*, never *verified*.

---

## 1. Summary of outcome

| Target | Inspected? | Depth | Basis |
|---|---|---|---|
| `com.ouraring.oura` iOS binary | **No** | — | Account-gated; ring required; decompilation prohibited by Terms §II |
| Android APK | **No** | — | No lawful acquisition path attempted |
| Oura Web API — live behaviour | **No** | — | No endpoint called, including the sandbox (§2) |
| **`openapi-1.41.json` — the published contract** | **Yes** | **Complete document**; 75 schemas, 72 paths, 274 `$ref`s, 75 `x-codeSamples` | Fetched from the vendor's own static path |
| `cloud.ouraring.com/v2/docs` | Yes | Retrieval only | JavaScript shell, 2,008 B — contains no text |
| `ouraring.com` marketing, `/science-and-research`, `/health-radar`, `/why-oura`, `/membership` | Yes | Full text extraction | Public pages |
| `/privacy-policy`, `/terms-and-conditions`, `/security` | Yes | Full text extraction | Public legal and trust surfaces |
| `organizations.ouraring.com` | Yes | Full text extraction | Separate B2B surface |
| Google Play listing | Yes | Full text extraction | Platform metadata |
| Apple App Store metadata | Yes | iTunes Lookup API | Platform metadata |
| Oura blog articles (5) | Yes | Full text extraction | Vendor research and validation claims |
| `/legal`, `/science`, `/for-organizations` | Yes | **Retrieval failure** | JavaScript error shells — no readable body; see §3 |
| Oura GitHub organisation | **Not searched** | — | No published product source is claimed |
| Security posture (TLS, pinning, key storage) | **No** | — | Would require a binary or a live endpoint |

**What this inspection does establish.** The complete documented data model of the Oura Web API: its
schema names, field types, nullability, `required` lists, enumerations, time encodings, webhook
semantics, sandbox coverage, OAuth scopes and rate-limit model. For a candidate whose binary cannot be
touched, **this is the highest-yield artifact available**, and it yielded the strongest positive finding
in this candidate (the tombstone and provenance model, §4.4).

**What this inspection does not establish.** Anything about latency, reliability, sync behaviour, the
app, the ring, the firmware, the algorithm implementation, or whether any documented endpoint behaves as
documented. **No claim in this file is a working-behaviour claim.**

---

## 2. Authorization basis

| Question | Decision | Basis |
|---|---|---|
| Is inspecting the app binary authorized? | **No** | The Terms prohibit it (§2.1). No authorization decision was sought or would have been granted within this package's rules |
| Is calling a live endpoint authorized? | **No** | Not requested for this candidate. The sandbox exists to make a lawful read possible without an account; **calling it would still be a separate explicit authorization decision**, and it was not taken |
| Is reading the published specification authorized? | **Yes** | It is a document the vendor publishes at a public URL for the express purpose of being read by third-party integrators |
| Are the research/validation articles authorization-relevant? | Yes, as **public claims** | They are marketing and scientific surfaces; they are quoted as the vendor's statements |

### 2.1 The contract's own prohibition — quoted as the authorization basis

> "(ii) reverse engineer, disassemble, decompile, decode, adapt, or otherwise attempt to derive or gain
> access to any **Oura Apps** component of the Services, in whole or in part" `[PUBLIC CLAIM]` — Terms
> and Conditions, §II

> "(ix) circumvent, bypass, disable, degrade, or otherwise interfere with any **access controls, usage
> limits, security features, authentication measures, subscription restrictions**, technological
> protection measures, or other technical or contractual restrictions that ŌURA applies to the Services,
> including by using multiple accounts, Credentials, devices, IP addresses, automated tools, or other
> methods to avoid or exceed those restrictions" `[PUBLIC CLAIM]` — Terms and Conditions, §II

This is recorded as a **strengthening of the "not performed" decision**, and as a documented contrast to
`nightscout` (AGPL-3.0, where source inspection was affirmatively licensed) and `empatica` (closed source,
no published client, where the blocker was *acquisition* rather than *prohibition*). Oura is the first
candidate in the package where the vendor's **contract** — not the absence of a lawful artifact — is the
primary reason an implementation inspection did not occur. That is a different fact and is recorded as
such.

**A second clause is authorization-relevant for a different reason:**

> "(vi) attempt to probe, scan, or test the vulnerability of any ŌURA system or network or to breach
> security or authentication measures **without express authorization**" `[PUBLIC CLAIM]` — Terms and
> Conditions, §II

Recorded because an integration test that probes endpoint behaviour beyond documented use is not
authorized by this clause, and no such test was performed. **The "without express authorization" qualifier
is part of the clause and is quoted**: it is what makes the prohibition a *permission* requirement rather
than an absolute ban, and omitting it would have mis-stated the vendor's own text.

**All three clauses in this section were mis-quoted in an earlier draft** and are corrected here against
the fetched page — including an invented verb sequence in (ix) ("remove, deactivate, impair, defeat") that
appears nowhere in the contract. Recorded because a contract clause cited as an authorization basis must
be exact: the correction is visible here rather than silent.

### 2.2 What was *not* done, and why each omission is deliberate

| Not done | Reason |
|---|---|
| No account created | Would require accepting the Terms as a user and providing personal data |
| No OAuth flow executed | Would require an account and a registered application |
| No sandbox request issued | Explicit authorization decision, not taken |
| No webhook receiver stood up | Would require a public callback endpoint and a subscription |
| No certificate pinning or TLS inspection attempted | Prohibited (§2.1) and unnecessary — no traffic was generated |
| No app binary downloaded | Account-gated; and §2.1 |
| No real Oura user's data requested or handled | Not authorized at any point in this package |

---

## 3. Artifact record

### 3.1 Primary artifact

| Property | Value |
|---|---|
| URL | `https://cloud.ouraring.com/v2/static/json/openapi-1.41.json` |
| Discovered via | `<redoc spec-url="/v2/static/json/openapi">` on the vendor's own documentation page |
| Size | **454,152 bytes** |
| SHA-256 | **`a144a3436e9b73da568999fc713257e1d73e12cd3f7024eef45035c9d8f6b557`** |
| `openapi` | `3.1.0` |
| `info.version` | `2.0` (the document's own version; the **file** name carries `1.41`) |
| `servers[0].url` | `https://api.ouraring.com` |
| Paths | **72** |
| Schemas | **75** |
| `$ref` values | **274** — all resolvable, 0 broken, 0 non-schema |
| Security schemes | **4** — all declared and all referenced |
| `x-codeSamples` | **75** (request-side, cURL/Python/JavaScript) |
| `example` / `examples` keywords | **0** |

**A note on the version identity.** The filename says `1.41`; `info.version` says `2.0`. Neither is
wrong — they are two different version namespaces — and the document does not explain the relationship.
Recorded as a small documentation observation, not a defect.

### 3.2 SHA-256 of every artifact this candidate quotes

| Artifact | SHA-256 |
|---|---|
| `openapi-1.41.json` | `a144a3436e9b73da568999fc713257e1d73e12cd3f7024eef45035c9d8f6b557` |
| `ouraring.com/` | `4a8690452c33e09506ef15f834137ea3f2a01ac220881914808ea2457fd86450` |
| `/membership` | `f74dc7a1422b4222e0a6f65da0907d91183c736e74ab1aedaa8196befa06f4d5` |
| `/privacy-policy` | `de77fff164ca9ac8bfc842b66fdc74e31922b383da241cc366048c773bfe9aca` |
| `/terms-and-conditions` | `891c51c238ac8104eb6e8a35007b26335d8ae07ef39fce1998851dd6056509b5` |
| `/security` | `41706e303aef9ff204ce3695466a532dd54aa1bd80eca322b52caa7c84818b2d` |
| `/science-and-research` | `388af90948153be8de72e8017c98586c7f8b3d216208aa6b8f4c5b5c46bd2662` |
| `/health-radar` | `ae42a191f253e0466f87fd70dc163de3967c0e4be57fbee87b46763bcf262abb` |
| `/why-oura` | `264d1e242f685d95d35b81fa11fa84db5741f687f69613919ef1e9bcd14395cb` |
| `/blog/` | `f3b679ba768c15a1bb1f78a40e5390b0e43520d3ef1cc754ae253b20e33c9430` |
| `/blog/how-accurate-is-oura/` | `604f9e344176e3ea96082f21cd7f61f0879b350e42a09c851eec4e87e80cb074` |
| `/blog/new-sleep-staging-algorithm/` | `529bf111d0425bae72c478c6a6a206d83b658c001d49429fba432fd9d6bb0f8d` |
| `/blog/inside-the-ring-developing-ouras-latest-sleep-staging-algorithm/` | `529bf111d0425bae72c478c6a6a206d83b658c001d49429fba432fd9d6bb0f8d` |
| `/blog/how-oura-measures-sleep-and-validates-accuracy/` | `1ad377d76f4b0db99cc9cec2a5521bee105771b97c85d263ff1c7f9f4851aae7` |
| `/blog/sleep-staging-is-not-guesswork/` | `51adcbf9eda140a130023cbc070079ac9eaf23f2798a19e76d62ea899bddf22b` |
| Google Play listing (`com.ouraring.oura`) | `5f23b4ffc616f848258fe8ae64ac0118b577dbd55bea78f7a14016aa8c846673` |
| `organizations.ouraring.com/` | `25876b61d68294504d2af8b51231b35e5efc39969d18d05697cd884476ede602` |
| iTunes Lookup `bundleId=com.ouraring.oura` | `a492b7cec36d17323e0f41e518b0223286d7e8f640ebc519c159fa2f77f07ec3` |
| iTunes Lookup `bundleId=com.ouraring.ouraapp` (empty) | `7f5ce663b726607eae2fde2a6dc438052ac0d1681e636de7cc3377c0bb77e047` |
| `cloud.ouraring.com/v2/docs` (JS shell) | `1029c1a654483a221c8bec090fcc57c9f1e72ca6bc9bbc125862bc71f8132567` |
| **Retrieval failure — error shell** `/science` | `472d2e26f9465ef9bb942d710a02c1d1f1f0a559fb2ff620bc8fdbb7b56e6068` |
| **Error shell** `/legal`, `/legal/privacy-policy`, `/legal/terms-of-service` | `a46d675d2b639278b4f11b9de89f6f14ec0e925a5aa093e9d15df70a0e2aff54` |
| **Error shell** `/for-organizations` | `80e1d8d88efd608cae25f4ae0f57340fc44240115801dc2634b131068e5f3ca0` |
| **A readable 404 page** `/blog/white-papers/` **and** `/blog/inside-the-ring-developing-ouras-latest-sleep-staging-algorithm/` | `f015cc7650b24f41fa49b41494e9a2d5db09c36a61fb31fef236594d7e4ab052` (byte-identical) |

### 3.3 The three error shells — retrieval failures, not 404s

`/science`, `/legal` (and its two children) and `/for-organizations` each returned
`<html id="__next_error__">` with approximately one character of rendered text and **no vendor-authored
"not found" message**. These are recorded as **`[UNKNOWN]` — content could not be retrieved**, and
**not** as evidence that the pages do not exist.

**This distinction is load-bearing and was got wrong in this candidate's first pass.** A **404 page** is a
statement by the vendor that a resource is absent. A **JavaScript error shell** is a failed retrieval,
which supports no conclusion about the resource at all. The distinction is the reason `blog_white-papers.html`
and `blog_inside-the-ring-developing-ouras-latest-sleep-staging-algorithm.html` are byte-identical: both
URLs render the genuine 404 document, so *that* one is a confirmed absence.

### 3.4 Hashes

All 23 rows in §3.2 were computed with `shasum -a 256` from the bytes on disk, on the research date.
**No hash in this file is a placeholder.** The rule applied here — *an unverified hash is worse than no
hash* — was recorded in this package after a fabricated placeholder was written and then replaced with
computed values during the `empatica` candidate, and it is applied here from the start.

---

## 4. Static findings from the published contract

All of §4 is `[DOCUMENTED CONTRACT]` unless marked otherwise. **None of it is observed behaviour.**

### 4.1 Architecture as documented

```
  Oura Ring (5 / 4 / 4 Ceramic)
        │  BLE, vendor-unpublished protocol, ring firmware unpublished
        ▼
  Oura App  (iOS · Android)          ← NOT INSPECTED
        │  user-initiated sync for sleep; background for activity/stress/heart rate
        ▼
  Oura Cloud  — algorithms produce "insights and scores"
        │       (documented as a staged pipeline: Data Collection → Processing → API Availability)
        ├──────────────► api.ouraring.com  ── 19 GET /v2/usercollection/* roots
        │                                    18 /v2/sandbox/* mirrors
        │                                    3 webhook paths (5 operations)
        │                                    4 security schemes; OAuth2 authorizationCode
        └──────────────► Webhooks: push only, documented as lossy (~1 h retry, 10 retries, no backoff)
```

**Documented data-flow properties that matter to this comparison:**

1. **Two-stage access.** A third party reaches the API only through the user's OAuth consent, against a
   **membership-gated** account. `403` is documented as meaning the user's subscription has expired.
2. **The sensor layer is entirely opaque.** No BLE service, characteristic, framing, firmware version or
   sampling configuration is published anywhere in the document. V2's `resmed-air11-ble-stream` and
   `xe125-respiratory-rate` work is at exactly this layer, and **nothing published by Oura is comparable
   to it**.
3. **No raw stream is exposed.** Confirmed by complete schema inventory: every field is a discrete sample
   or an aggregate/classification produced by Oura's own processing.
4. **Processing happens in the vendor's cloud, not locally.** The document states "API Availability:
   **Processed** data becomes available through API endpoints".

### 4.2 The contract is internally consistent

*Checked, not assumed.* Three integrity checks:

| Check | Result |
|---|---|
| `$ref` resolution | **274 `$ref` values — 0 broken, 0 non-schema references** |
| Security schemes | **4 declared in `components.securitySchemes`; all 4 referenced by operations** |
| Path ↔ schema coverage | 72 paths, 75 schemas; every path's response resolves to a declared schema or a documented inline object |

**A hypothesis checked and refuted.** An earlier pass suspected that `ClientIdAuth` and `ClientSecretAuth`
were dangling references — declared schemes with no operation using them, a common documentation defect.
**Checked: both are declared *and* referenced.** The hypothesis is recorded as **refuted** and is
**not** a finding. It appears here only so a reader can see the check was made.

### 4.3 Read-only for health data — a documented boundary

| Operation class | Count | Detail |
|---|---|---|
| `GET` on `/v2/usercollection/*` | **19** | Every data root |
| `GET` on `/v2/sandbox/*` | 18 | Read-only mirrors |
| **Writes anywhere in the document** | **4** | `POST /v2/webhook/subscription`, `PUT /v2/webhook/subscription/{id}`, `DELETE /v2/webhook/subscription/{id}`, `PUT /v2/webhook/subscription/renew/{id}` |

**There is no documented path by which an authorized third party can write to a user's health record.**
The only mutations available are to the integrator's own webhook subscriptions. Recorded as a **boundary
and a positive**, and as a contrast with `nightscout` (where the API permits writing entries and
treatments) and with the V2 roadmap's admin console (which is a write surface by design, behind
authentication).

### 4.4 Record provenance and deletion — the strongest structural finding

Four fields, all in `PublicModifiedSleepModel` and its referenced schemas, carry provenance the
comparison has not previously found in a commercial candidate:

| Field | Documented values | What it establishes |
|---|---|---|
| `type` → `PublicSleepType` | `deleted`, `sleep`, `long_sleep`, `late_nap`, `rest` | **A user-deleted record is a tombstone** (`"deleted" = deleted sleep by user`), and **a rejected false positive is retained with its reason** (`"rest" = Falsely detected sleep / nap, rejected in confirm prompt by user`) |
| `sleep_algorithm_version` → `PublicSleepAlgorithmVersion` | `v1`, `v2` | **Which algorithm generation produced this record** |
| `sleep_analysis_reason` → `PublicSleepAnalysisReason` | `foreground_sleep_analysis`, `bedtime_edit`, `background_sleep_analysis`, `background_created_foreground_updated` | **Why the record was created or revised** |
| `low_battery_alert` | `boolean` | **A device-state condition attached to a biological record** |

**This is the material correction to this candidate's first pass.** Three of the four rules
AnxietyWatch2 treats as non-negotiable appear here independently:

| AnxietyWatch2 rule | Oura's documented counterpart | Match |
|---|---|---|
| "No `UPDATE`, no `DELETE` against `samples`. It is append-only; removal is a **tombstone row**" | `PublicSleepType` = `deleted` | **Independent arrival at the same pattern** |
| "Never clamp an out-of-range value. **Reject it, name the reason, quarantine it.**" | `PublicSleepType` = `rest`, "rejected in confirm prompt by user" | **Independent arrival, applied to a false positive** |
| "the unit and valid range come from `vitals/registry.toml`" + versioned clock correction | `sleep_algorithm_version`, `sleep_analysis_reason` | **Both version what produced a value** |

**The honest framing, and the one this file requires.** These are **values in a contract**, not
constraints enforced by a schema. Oura could remove `deleted` from the enum in `1.42`; V2 cannot remove
the append-only rule without changing a SQL grant. **"Oura does the same thing" is not a supported
conclusion** and is not drawn here.

### 4.5 The missingness model — structurally correct in the numeric path, inconsistent in the string path

**Correct:**

```json
"PublicSample": {
  "interval":  { "type": "number", "description": "Interval in seconds between the sampled items." },
  "items":     { "$ref": "#/components/schemas/Array_Union_float__NoneType__Bits64_" },
  "timestamp": { "$ref": "#/components/schemas/LocalizedDateTime" }
}
"Array_Union_float__NoneType__Bits64_": { "title": "ArrayNullableFloatBits64", "type": "array", ... }
```

A hole is representable **in band**, self-describing, and not a sentinel. `required` is `['interval',
'items', 'timestamp']`; the *elements* are nullable.

Also correct, at the record level:

| Model | `required` | Nullable | Consequence |
|---|---|---|---|
| `PublicDailySleep` | `['id','contributors','day','timestamp']` | `score` | **A score can be null while its record exists** |
| `PublicDailyReadiness` | includes contributors/day/id/timestamp | `score` | Same |
| `PublicDailyActivity` | 24 fields | `score`, `class_5_min` | Same |
| `PublicSleepContributors` (7) | **no `required`** | all | Absent, not zero |
| `PublicActivityContributors` (6) | **no `required`** | all | Absent, not zero |
| `PublicReadinessContributors` (9) | **no `required`** | all | Absent, not zero |
| `ResilienceContributors` (3) | **all three required, non-nullable** | none | **The exception — inconsistent with its three siblings** |

**Inconsistent:**

| Field | Alphabet | Non-wear representable? |
|---|---|---|
| `class_5_min` | `'0'` non wear, `'1'` rest … `'5'` high | **Yes — `'0' = non wear`** |
| `movement_30_sec` | `'1'` no motion … `'4'` active | **No** |
| `sleep_phase_5_min` / `_30_sec` / `app_sleep_phase_5_min` | `'1'` deep … `'4'` awake | **No** |

The same condition — the ring was not on the finger — is a first-class code in one field and
**unrepresentable** in four others, in the same document, with no stated reason. Recorded as an
**intra-document inconsistency**, and `class_5_min` as the correct treatment.

**Semantics, not structure, is what is missing.** `[UNKNOWN]` what a null element *means*: the contract
distinguishes absent from zero and does not distinguish *not worn* from *sensor failed* from *rejected by
the algorithm*. Oura tells integrators to "displaying gaps in visualizations rather than connecting across
missing data" — the same instruction as `AMENDMENTS.md`:61 — and does not tell them what a gap is evidence
of.

### 4.6 Time: three encodings, four indistinguishable types

| Encoding | Where | Violates the document's own rule? |
|---|---|---|
| **ISO 8601** | The global rule; the large majority of fields; `event_time`; `WebhookSubscriptionModel.expiration_time` (`format: date-time`) | No |
| **unix seconds** | `X-RateLimit-Reset` header | N/A (header) |
| **unix milliseconds** | `PublicHeartRateRow.timestamp_unix` | **Yes — and it is on the real-time row** |

And four date/time schemas — `ISODate`, `LocalDateTime`, `LocalizedDateTime`, `UtcDateTime` — are the
**complete set** of schemas whose definition is exactly `{"type": "string"}`:

| Type | Models | Distinguishable? |
|---|---|---|
| `LocalizedDateTime` | 10 | **No** — bare string; no `format`, pattern, example or description |
| `UtcDateTime` | 3 (`PublicHeartRateRow`, `PublicRingBatteryLevelRow`, `PublicRingConfiguration`) | **No** |
| `LocalDateTime` | — | **No**, and the difference from `LocalizedDateTime` is **never stated** |
| `ISODate` | — | **No** |

Separately, the FAQ states the API interprets **date query parameters in the user's local timezone**. So
the query boundary is resolved against a per-user timezone the API does not report. The single exception
is `PublicSleepTimeWindow.day_tz` — "Timezone offset in second from GMT of the day" — the **only** UTC
offset carried anywhere in the contract.

**This is the sharpest undocumented-domain finding in the candidate, and it lands on the exact field V2
stores as "UTC epoch seconds and nothing else".**

### 4.7 Parallel implementations of the same named quantity

| Concept | Fields | What the vendor says |
|---|---|---|
| Sleep phase (5-min) | `sleep_phase_5_min`, `app_sleep_phase_5_min` | `app_...` is "aligned with what is shown in the app" and will be "removed in the future after a transition period" |
| Sleep phase (resolution) | `sleep_phase_5_min`, `sleep_phase_30_sec` | **Contradicted by the vendor's own blog** — it says the API reports "sleep stages in five-minute blocks" |
| Average heart rate | `average_heart_rate` (ecore, 30 s samples) vs the app (aggregated 5-min) | Stated in the field description |
| Lowest heart rate | `lowest_heart_rate` (ecore, 30 s) vs the app (min of 5-min) | Stated |
| Body temperature deviation | `temperature_deviation`, `temperature_trend_deviation` | **Nothing — the difference is never described** |

Oura **discloses** which implementation a field carries, in prose, in the contract. V2 forbids the
situation. Both are defensible; only V2's is machine-checkable.

### 4.8 Webhooks, rate limits and the sandbox

| Property | Documented value | Gap |
|---|---|---|
| Signature | HMAC over `timestamp + JSON.stringify(body)`, header `x-oura-timestamp` | **No replay window stated**; the vendor's own reference implementation **does not check the timestamp** |
| Retries | 10 for 4xx, 10 for 5xx, 10 for timeouts | **No backoff schedule, interval or total window** |
| Lossiness | "if your server is down for an extended period, **you might miss some events**" + "implement a reconciliation process" | **Vendor-documented** — recorded as an honest boundary, not a defect |
| Subscriptions | "separate subscriptions for each combination of event_type … data_type" | 3 × 17 = **up to 51 subscriptions per user**; the multiplication is not stated |
| Expiry | `expiration_time` (ISO `date-time`), `renew/{id}` exists | **No duration, no warning, no stated renewal requirement** |
| Rate limit | FAQ: "5000 requests per 5-minute period"; overview: a **two-layer** model | **The two-layer ceilings are not published** — discoverable only from a `429` header |
| Sandbox | 18 read-only mirrors; explicitly labelled fake data | **Correct handling** — contrast with `sleephq`'s unlabelled synthetic examples |
| Coverage mismatch | 3 data types pull-only (`heartrate`, `personal_info`, `ring_battery_level`); 1 push-only (`meal`) | `heartrate` — the most real-time-relevant field — **has no webhook** |

---

## 5. Reconciliation points — where the contract and the marketing diverge

| # | Marketing / documentation surface | Contract | Class |
|---|---|---|---|
| R1 | Blog: "Oura's public API reports sleep stages in five-minute blocks" | `sleep_phase_30_sec` exists | **Cross-artifact conflict — unresolved** |
| R2 | Quick Start: "Each endpoint includes example requests and responses" | 75 `x-codeSamples`, **0** `example`/`examples` | **Documentation defect** |
| R3 | Global rule: "Timestamps: All time-based data uses ISO 8601 format" | `timestamp_unix` in **ms** | **Self-violation** |
| R4 | Global rule: "Scores: Range from 0-100" | `PublicDailyActivity.score` "range **[1, 100]**"; `ResilienceContributors` "[0, 100]"; sleep/readiness scores **no range** | **Intra-document conflict** |
| R5 | Play listing: "over 30 biometrics" | Home page: "over 50 health metrics" | **Intra-vendor count discrepancy** |
| R6 | Play listing heading: "ILLNESS DETECTION" | Same listing: "not intended to diagnose, treat, cure, **monitor**, or prevent medical conditions or illnesses" | **Within-listing tension — the sharpest surface conflict found** |
| R7 | Trust Center: "we do not share or sell your personal data. That includes third-party AI models (LLMs)" | Privacy Policy **names LLM providers as recipients** | **Conflict — the policy governs (more conservative)** |
| R8 | Trust Center: "deleted data is fully removed from your account" | Privacy Policy ties retention to account lifetime | **Conflict — the policy governs** |
| R9 | Science page: "**Independently validated sensors**" | The gallery's HR study's "**Hannu Kinnunen, lead study author and Oura's Chief Scientist at the time**" | **Evidence-classification defect** |
| R10 | Science page: "99% Heart Rate Accuracy" | `/blog/how-accurate-is-oura/`: r² = 0.996, n = 49 | **Category mismatch — r² ≠ accuracy** |
| R11 | Play data-safety: "No data shared with third parties" | Privacy Policy names recipients | **Interpretation hazard, not a contradiction** — Google's definition of "sharing" excludes service providers; **the definition was not verified and this is not recorded as a discrepancy** |
| R12 | Membership page: "4.9 … based on 280k reviews" | Live store: 4.8633 over 308,643 **ratings** | **Not recorded as a discrepancy** — ratings ≠ written reviews; the page does not define which |
| R13 | `cloud.ouraring.com/v2/docs` is a JS shell | The spec is served from `/v2/static/json/openapi` | **Not a defect** — the docs page is a Redoc wrapper; recorded as a discovery note |

**R1, R2, R4, R6, R9 and R10 are the six findings a reader should carry forward.** R7 and R8 are the two
unresolved conflicts. R11 and R12 are recorded explicitly as **not** being discrepancies, because
recording them as such would overstate the case.

---

## 6. Runtime observation

**None. No app, no account, no ring, no request, no endpoint.**

| Observation class | Count |
|---|---|
| Requests issued to any Oura endpoint | **0** (including the sandbox) |
| Accounts created | **0** |
| OAuth flows executed | **0** |
| Webhook receivers stood up | **0** |
| App launches | **0** |
| Bluetooth captures | **0** |
| Bytes of user health data handled | **0** |

Everything in §4 and §5 is a reading of a published document. **No statement in this file describes
behaviour that was witnessed.**

---

## 7. Claim reconciliation

Every checkable claim from `oura_CLAIMED_FEATURES_AND_PURPOSE.md`, tested against the artifact named.
Classes: **match** · **documented-only** (stated, no artifact to test) · **partial** (true with a
qualifier the headline omits) · **does not match**.

| # | Claim | Tested against | Verdict |
|---|---|---|---|
| 1 | The API is free for developers | `info.description` ("free of charge"), FAQ, Home ("$5.99/mo") | **Partial** — API access is free; **user data access requires the user's membership** (`403` = expired subscription) |
| 2 | The API supports 10 users by default | `info.description` | Match |
| 3 | Rate limit is 5000 requests / 5 minutes | FAQ vs `info.description` two-layer model | **Partial** — the flat figure is the FAQ's; the real model has unpublished ceilings |
| 4 | Unauthenticated requests get 401 | FAQ | Documented-only |
| 5 | Expired membership returns 403 | FAQ | Documented-only |
| 6 | Webhooks are the preferred consumption method | `info.description` | Match — and **`heartrate` has no webhook type** |
| 7 | Webhook delivery is lossy | FAQ ("you might miss some events") | **Match — vendor-documented** |
| 8 | Event delivery arrives ~30 s after sync | FAQ | Documented-only |
| 9 | Signatures are HMAC-SHA256 over timestamp + body | Webhook docs, `x-codeSamples` | Match as documented; **no replay window stated** |
| 10 | "Each endpoint includes example requests and responses" | `openapi-1.41.json` | **Does not match** — 0 response examples |
| 11 | "All time-based data uses ISO 8601 format" | `timestamp_unix` (ms) | **Does not match** |
| 12 | "Scores: Range from 0-100" | `PublicDailyActivity.score`, `ResilienceContributors`, others | **Does not match** — three conventions |
| 13 | "**Durations**: Provided in seconds unless otherwise specified" | `*_duration`, `latency`, `stress_high`, `recovery_high`, `non_wear_time` | **Match** |
| 14 | "**IDs**: Unique identifiers for specific data objects follow a consistent format" | Only `format` on `WebhookSubscriptionModel.id` (`uuid4`) and 3 `format: date` fields; **sleep `id` has no `format`** | **Does not match as stated** — an undocumented claim about the document's own contents |
| 15 | Dates are interpreted in the user's local timezone | FAQ | Match as documented; **the timezone is not reported except `day_tz`** |
| 16 | Sleep data requires the user to open the app | `info.description`, FAQ | Match across both |
| 17 | Activity / stress / heart rate sync in the background | `info.description` | Match |
| 18 | Sandbox data is not real user data | Sandbox docs | **Match — correctly labelled** |
| 19 | Tags can be created by the user | `TagModel`, `EnhancedTagModel`, `ExtApiV2DataType` | Documented-only (creation is in-app; the API is read-only) |
| 20 | `meal` data is available | `ExtApiV2DataType` includes `meal`; **no REST path** | **Partial** — push-only, no read path |
| 21 | Sleep stages are exposed | `sleep_phase_5_min`, `sleep_phase_30_sec`, `app_sleep_phase_5_min` | Match — **at three names and two resolutions** |
| 22 | Body temperature is measured and exposed | `temperature_deviation`, `temperature_trend_deviation` (°C) | **Match as a deviation** — no absolute temperature field exists |
| 23 | SpO₂ is measured | `PublicDailySpO2.spo2_percentage`, `breathing_disturbance_index` | Match as an aggregate |
| 24 | "We don't sell or share your data to train other AI models" | Trust Center vs Privacy Policy's named LLM providers | **Does not match** — the policy names recipients |
| 25 | Deleted data is fully removed | Trust Center vs Privacy Policy retention | **Does not match** — retention is tied to account lifetime |
| 26 | SOC 2 certified | Trust Center | **Documented-only** — Type I/II not stated, no report published |
| 27 | HITRUST certified | Trust Center | **Documented-only** — no level, scope or date |
| 28 | AES-128 BLE; TLS 1.2+ transit; AES-256 at rest | Trust Center | **Documented-only** — nothing to test without a binary or capture |
| 29 | Oura is not a HIPAA Covered Entity for DTC | Privacy Policy | Match, **scoped** |
| 30 | Partner PHI is handled under HIPAA | Trust Center, B2B site | Match, **scoped** — consistent with the DTC exclusion |
| 31 | Fertile Window is CE-marked under MDR | Terms | Match; and the Terms add "**MUST NOT BE RELIED UPON TO PREVENT PREGNANCY**" |
| 32 | "99% Heart Rate Accuracy" | `/blog/how-accurate-is-oura/`: r² = 0.996, n = 49 | **Partial** — the number is an r² |
| 33 | "98% Heart Rate Variability Accuracy" | same source: r² = 0.980 | **Partial** — the number is an r² |
| 34 | "94% Ovulation Detection Accuracy" | science page | **Documented-only** — no source linked on the page |
| 35 | "79% Sleep Tracking Accuracy" | `/blog/new-sleep-staging-algorithm/` (2022-11-16) | Match — and the same page notes PSG's own 83% |
| 36 | "Independently validated sensors" | Gallery contains a vendor-authored study | **Does not match the heading** |
| 37 | "130+ peer-reviewed publications" | science page | Documented-only — counts publications *using* Oura |
| 38 | "the most scientifically validated smart ring" | science page | **Superlative — unverifiable** |
| 39 | Symptoms can be detected before they are felt | Health Radar | **Partial** — hedged with "sometimes"; **no rate published** |
| 40 | Symptom Radar / Blood Pressure Signals / Nighttime Breathing accuracy | — | **Not documented anywhere.** No false-alarm, sensitivity or specificity figure |
| 41 | Class action + independent 53% agreement | `/blog/sleep-staging-is-not-guesswork/` | **Reported by the vendor**, unresolved; Oura disputes it in the literature |
| 42 | Terms prohibit reverse engineering | Terms §II | **Match — quoted as the Phase 2A authorization basis** |
| 43 | Reverse engineering / subscription circumvention prohibited | Terms §II(ix) | Match |
| 44 | Terms effective 2026-11-04 | Terms | Match — **not yet in effect at the research date** |
| 45 | No alert path for emergencies | Terms + complete schema inventory | **Match** — no alert, escalation or caregiver concept exists |
| 46 | No PAP / AHI / leak concept | Complete schema inventory | **Match** — a genuine gap in V2's favour |
| 47 | No API write access to health data | Operation inventory | **Match — and recorded as a positive** |
| 48 | No raw sensor stream | Schema inventory + Data Processing Timeline | Match |
| 49 | Contract is internally consistent | 274 `$ref`s, 4 schemes | **Match — verified positive** |
| 50 | `ClientIdAuth`/`ClientSecretAuth` are dangling | hypothesis | **Refuted — not a finding** |

**Verdict tally — counted from the table above, not estimated.** Of **50** checkable claims:

| Verdict | Count | Which |
|---|---|---|
| **Match** | **24** | Includes items 13, 16, 17, 18, 21, 22, 23, 29, 30, 31, 35, 42–49 |
| **Documented-only** (stated, nothing to test) | **9** | 4, 5, 8, 19, 26, 27, 28, 34, 37 |
| **Partial** (true with a qualifier the headline omits) | **6** | 1, 3, 6, 32, 33, 39 |
| **Does not match** | **7** | **10, 11, 12, 14, 24, 25, 36** |
| Outside those classes, named individually | **4** | **38** a superlative with nothing to test · **40** published accuracy rates for the predictive surfaces are **absent from every surface** · **41** the class action and the 53% figure, **recorded by the vendor as the subject of litigation** · **50** a **refuted hypothesis** |

**An earlier draft of this file gave a different tally that the table did not support** ("17 match, 12

documented-only, 10 partial, 10 do not match"). It was written from impression rather than counted, and
it is **corrected here in place** because a tally a reader cannot reproduce from the table above is worse
than no tally. **The seven rows that do not match are the ones to carry forward**, and three of them
(items 10, 11, 14) are defects in the vendor's own document about the vendor's own document.

**Two classes are kept apart deliberately.** "Documented-only" means *the claim is stated and no artifact
exists to test it* — it is neither a match nor a defect, and it is the correct verdict for every
certification and every published rate with no report behind it. **Item 40 is not in that class**: a rate
that is *not stated anywhere* is an absence of documentation, and conflating it with "documented-only"
would have hidden the finding.

---

## 8. Evidence gaps

| # | Gap | Consequence |
|---|---|---|
| **I1** | **No runtime behaviour of any kind** | Latency, reliability, sync behaviour, sandbox output, the real response to a lapsed membership, off-finger behaviour — **all unobserved**, and no hypothesis about them is recorded as a finding |
| **I2** | **No endpoint called, including the sandbox** | The one lawful read that would convert a documented contract into an observation was **not taken**; it is an explicit authorization decision under this package's rules |
| **I3** | **No binary inspected** | Security posture (TLS, pinning, key storage, BLE pairing) is documented-only; the Terms prohibit the alternative |
| **I4** | **The BLE layer is entirely unpublished** | Nothing comparable to V2's `resmed-air11-ble-stream` work exists in the public record; the sensor side is a **complete unknown** |
| **I5** | **`cloud.ouraring.com/v2/docs` is a JS shell** | The rendered documentation could not be read as text; the specification was obtained from the static path the page references instead |
| **I6** | **Three pages returned retrieval failures** | `/legal`, `/science`, `/for-organizations` — **`[UNKNOWN]`**, not absent (§3.3) |
| **I7** | **The 2025 *Sleep Advances* letter and the 2025 *Scientific Reports* study were not fetched** | The strongest counter-evidence in this candidate, and its rebuttal, are recorded only through the vendor's characterisation |
| **I8** | **No court record consulted** | The class action is known only from the vendor's blog post about it |
| **I9** | **No validation white paper obtained** | The Trust Center offers one "upon request"; the request was not made |
| **I10** | **The first pass was not exhaustive, and this is recorded rather than closed** | A targeted second pass over enums, `required` lists, nullable unions, bare-type schemas, time formats and digit-string fields produced **9 new findings and 5 corrections**. That means **a further field class may still be unexamined**, and it means the first pass produced a *materially more negative* picture of Oura than the second did |
| **I11** | **The spec version is a snapshot** | `openapi-1.41.json` is one revision; field presence and enums are expected to change, and this file records the date |

**I10 is the most important gap in this file, and it is recorded as a defect in the process rather than in
the vendor.** A verification pass is supposed to firm up a draft. This one **reversed five verdicts and
softened a sixth**, because the first pass read the contract's prose and did not sweep its structured
field classes. The standing remedy is recorded in `RESEARCH_METHOD.md`: **the field-class sweep is part
of the first pass, not an audit of it.**

---

## 9. Ethical and legal limits applied

| Limit | Applied how |
|---|---|
| No DRM, authentication, licensing, encryption or platform protection bypassed | Nothing was bypassed. The only artifact read is published for third-party integrators |
| No certificate pinning defeated | No traffic was generated |
| No credentials or private data extracted | No account, no token, no personal data |
| No production login | No login of any kind |
| No destructive server action | No request was issued |
| Vendor-published artifacts treated as an authorized surface | The specification, legal pages, trust centre and platform listings are all published for public consumption |
| Documented ≠ working | **Every** contract finding is labelled *documented*; §6 states that runtime observation was zero |
| Vendor-authored research not presented as independent | The "Independently validated sensors" heading is recorded as the vendor's label, with the lead author's affiliation quoted from the vendor's own page |
| Vendor-published business numbers recorded as vendor-published | The 86% improvement figure, the review counts and the publication counts are all attributed to the vendor |
| A refuted hypothesis is not a finding | The dangling-`ClientIdAuth` hypothesis is recorded as refuted and carries no verdict |
| Correcting an earlier version in the same file | The 404 misdescription, the `expiration_time` encoding, the schema name and the timestamp-type count are all corrected in place, with the correction visible |
| No third-party developer machine paths reproduced | No published source comments were quoted in this candidate |

**No Oura endpoint, account, ring or binary was touched. This inspection consisted entirely of reading
documents the vendor publishes for the purpose of being read.**

---

## 10. Reproduction

```sh
# 0. Working directory for this candidate
mkdir -p /tmp/aw2research/oura/web && cd /tmp/aw2research/oura

# 1. Marketing, legal, trust and marketing surfaces (Chrome UA; some paths need it)
UA='Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36'
B=https://ouraring.com
curl -sSL -A "$UA" "$B/"                        -o web/home.html
curl -sSL -A "$UA" "$B/membership"              -o web/membership.html
curl -sSL -A "$UA" "$B/privacy-policy"          -o web/privacy.html
curl -sSL -A "$UA" "$B/terms-and-conditions"    -o web/terms.html
curl -sSL -A "$UA" "$B/security"                -o web/security.html
curl -sSL -A "$UA" "$B/blog/"                   -o web/blog.html
curl -sSL -A "$UA" "$B/why-oura"                -o web/why-oura.html
curl -sSL -A "$UA" "$B/science-and-research"    -o web/science-and-research.html
curl -sSL -A "$UA" "$B/health-radar"            -o web/health-radar.html
curl -sSL -A "$UA" "$B/science"                 -o web/science.html            # error shell
curl -sSL -A "$UA" "$B/legal"                   -o web/legal.html              # error shell
curl -sSL -A "$UA" "$B/for-organizations"       -o web/for-organizations.html  # error shell
curl -sSL -A "$UA" "$B/blog/white-papers/"      -o web/blog_white-papers.html  # genuine 404
curl -sSL -A "$UA" https://organizations.ouraring.com/ -o web/org_ouraring.html
curl -sSL -A "$UA" 'https://play.google.com/store/apps/details?id=com.ouraring.oura' -o web/play_oura.html
# blog articles quoted in this file
for s in how-accurate-is-oura new-sleep-staging-algorithm \
         how-oura-measures-sleep-and-validates-accuracy sleep-staging-is-not-guesswork; do
  curl -sSL -A "$UA" "$B/blog/$s/" -o "web/blog_$s.html"
done

# 2. Discover the specification the vendor's own docs page points at
curl -sSL -A "$UA" https://cloud.ouraring.com/v2/docs -o web/api_docs.html
grep -o 'spec-url="[^"]*"' web/api_docs.html        # -> /v2/static/json/openapi
curl -sSL -A "$UA" https://cloud.ouraring.com/v2/static/json/openapi-1.41.json -o web/openapi-1.41.json

# 3. Platform metadata (use bundleId=, not id=; the response also returns unrelated apps)
curl -sS 'https://itunes.apple.com/lookup?bundleId=com.ouraring.oura'    -o api/itunes_com.ouraring.oura.json
curl -sS 'https://itunes.apple.com/lookup?bundleId=com.ouraring.ouraapp' -o api/itunes_com.ouraring.ouraapp.json

# 4. Integrity checks and THE FIELD-CLASS SWEEP — run this BEFORE writing any conclusion
python3 - <<'PY'
import json
d=json.load(open('web/openapi-1.41.json')); S=d['components']['schemas']
def walk(o,acc):
    if isinstance(o,dict):
        for k,v in o.items():
            walk(v,acc) if k!='$ref' else acc.append(v)
    elif isinstance(o,list):
        for v in o: walk(v,acc)
    return acc
print('paths',len(d['paths']),'schemas',len(S),'refs',len(walk(d,[])))
print('securitySchemes',list(d['components']['securitySchemes']))
print('bare-string schemas',[k for k,v in S.items() if v=={'type':'string'}])
for n in ('PublicDailySleep','PublicDailyReadiness','PublicActivityContributors',
          'PublicSleepContributors','PublicReadinessContributors','ResilienceContributors'):
    print(n,'required',S[n].get('required'))
print('PublicSleepType',json.dumps(S['PublicSleepType']))
print('algorithm versions',S['PublicSleepAlgorithmVersion']['enum'])
print('analysis reasons',S['PublicSleepAnalysisReason']['enum'])
# the character-encoded fields — three alphabets, and only one has a non-wear code
for n,f in (('PublicDailyActivity','class_5_min'),('PublicModifiedSleepModel','movement_30_sec')):
    print(n,f,S[n]['properties'][f]['description'].split(chr(10))[0:3])
PY

# 5. Hashes
shasum -a 256 web/*.html web/openapi-1.41.json api/*.json
```

**The order matters and is the lesson of this candidate.** Step 4's field-class sweep was performed
*after* the first draft was written, and it changed nine findings and five verdicts. On the next
candidate it belongs immediately after step 2, before any conclusion is drafted.

---

## Related files

- `oura_CLAIMED_FEATURES_AND_PURPOSE.md` — Phase 1: what Oura claims, with the claim ledger O-1…O-95
- `PROMPT_2.md` — the candidate-specific second-stage brief
- `oura_COMPARISON.md` — the comparison report
- `../RESEARCH_METHOD.md` — evidence classes, authorization rules, standing sub-checks
- `../PROCESS_STATE.md` — candidate status and residual uncertainty
- `../PROMPT1.md` — the research brief this inspection executes
