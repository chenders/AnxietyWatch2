# SleepHQ — implementation inspection

Candidate 5 of 10. Inspection performed **2026-10-06**.
Companion to `sleephq_CLAIMED_FEATURES_AND_PURPOSE.md`.

---

## 1. Summary of outcome

**Inspection mode: documented-contract inspection. Performed for the published API reference; not
performed for the app binaries or for any running service.**

| Question | Answer |
|---|---|
| Was a published interface specification read? | **Yes** — the vendor's OpenAPI document and rendered API reference, in full |
| Was source code read? | **No.** No vendor source is published (see §3.3) |
| Was an application binary obtained, decompiled or executed? | **No** |
| Was any vendor endpoint called? | **No** — not one request |
| Was an account created or a trial started? | **No** |
| Runtime observations | **Zero** |

**What this mode can and cannot establish.** The OpenAPI document names 21 paths, 11 response models and
the exact attributes of each. That is genuine evidence about **the interface SleepHQ publishes and is
therefore obliged to keep working** — an interface contract is a real artifact, deliberately written for
third-party consumers, and its field names are a commitment. It is **not** evidence about what the
service does internally, how the analysis layer computes a value, what is inside the opaque summary
blocks, or how the app behaves. Those remain unobserved.

Every finding below is labelled **[DOCUMENTED]** (stated by the vendor's own machine-readable
specification), **[STATIC]** (a reading of a published artifact, not of behaviour), or **[INFERENCE]**.
None is labelled "verified" or "working", because nothing was verified and nothing was run.

**OpenAPI metadata as fetched:** `swagger: "2.0"`, `host: "sleephq.com"`, `basePath: "/api"`,
`title: "SleepHQ"`, **21 paths, 11 definitions**. Reference UI at
`https://sleephq.com/api-docs/index.html`; specification at `https://sleephq.com/api/swagger.json`
(SHA-256 `9a4305e663ddc9b65c9b471e8fd04e1085f382814c3540193d85cbd8660536a4`).

---

## 2. Authorization basis

Same basis as every other candidate in this package:

- **Read** — the specification and reference page are published unauthenticated by the vendor. Anyone can
  fetch them. Reading them requires no authorization from SleepHQ beyond publication.
- **Not exercised** — the API is documented as usable with an OAuth2 bearer token or a browser session.
  Obtaining either requires an account and, for the API path, an OAuth client. **No account was created
  and no client was registered**, so the write surface is described from the specification and was never
  touched. This is the deliberate limit: reading a contract is not using it.
- **Not attempted** — no attempt was made to obtain, decompile or execute the iOS or Android application.
  No lawful route to those binaries was identified for this exercise.

For the fourth time in this package the practical constraint is the same: the artifact that explains most
about a closed commercial product — its running binary — cannot be inspected lawfully here. SleepHQ's
published API is the only implementation-grade surface available, so it is the one that was read.

---

## 3. Artifact record

### 3.1 Inspected

| Artifact | URL | SHA-256 | Note |
|---|---|---|---|
| API specification | `https://sleephq.com/api/swagger.json` | `9a4305e6…536a4` | The primary artifact. 21 paths, 11 models, OpenAPI 2.0 |
| API reference UI | `https://sleephq.com/api-docs/index.html` | `f889631f…fb2ac` | Auth narrative, code samples, the MD5 worked example |
| Privacy policy | `https://www.sleephq.com/privacy-policy` | `c3290ee6…42dbf` | Text extract |
| Terms of service | `https://www.sleephq.com/terms-of-service` | `2437aab4…34a97` | Text extract |
| Supported devices | `https://www.sleephq.com/supported-devices` | `83afae86…65fd2` | Text extract |
| App Store metadata | iTunes Lookup API, id 6451459781 | `10f3a130…7c242` | iOS record |
| Play listing | `https://play.google.com/store/apps/details?id=com.sleephq.android` | `4425b84b…18d09` | Includes data-safety panel and release notes |
| Marketing site | `https://sleephq.com/` | `79c38261…d559` | Pricing matrix, hardware, feature claims |
| Shop product pages | `https://shop.sleephq.com/products/{o2-ring-pro,magic-uploader-pro}` | — | JS-rendered; prices read from embedded JSON |
| Founder articles | `/articles/myair`, `/articles/` | — | AHI accuracy claims |
| Enterprise page + case articles | `/enterprise`, four clinic articles | — | B2B posture |

### 3.2 Not inspected

| Artifact | Why not |
|---|---|
| iOS application binary (`com.sleephq.ios`) | No lawful route identified; Apple's distribution does not expose it |
| Android application package (`com.sleephq.android`) | No lawful route identified; the Play listing does not authorise retrieval of the APK |
| The running API service | Would require an account, an OAuth client and — for any ingestion path — uploading a real SD card. Out of scope for this exercise |
| The clinic/enterprise API surface | Documented as existing, explicitly **not published** in the reference |
| The `jsonb` summary block schemas | Not published anywhere |
| A share link | Not created; the de-identification claim therefore remains unverified |

### 3.3 Why there is no source inspection

For `oscar` (candidate 4) the source-inspection mode was available because OSCAR is GPLv3 software. For
SleepHQ it is not: **no SleepHQ-authored code repository was found** on GitHub, GitLab or elsewhere, and
the vendor publishes no source. The third-party projects that exist (a Raspberry Pi uploader, an API
client) are **unaffiliated user work**, and reading them would tell us about those authors, not about
SleepHQ's implementation. They were not treated as vendor evidence.

This is the second candidate to be inspected through a published interface specification rather than
source — `exist` (candidate 3) was the first — and the second for which the inspection mode is
"documented contract" rather than "performed on source".

---

## 4. Static findings from the published interface specification

### 4.1 The documented resource model `[DOCUMENTED]`

21 paths, clustered into nine tags: **Masks, Current User, Teams, Machines, Machine Dates, Imports,
Import Files, Journals, Devices**. Responses use a JSON:API-shaped envelope — `data.id`, `data.type`,
`data.attributes`, `data.relationships` — and every model carries `created_at` and `updated_at`.

The resource graph, as documented:

```
User ── relationships ──> teams, memberships
Team ── relationships ──> machines        (attributes: time_zone, locale, energy_unit, teams_type)
  ├── Machines      (brand, model, serial_number, name)
  ├── Machine Dates (one per night)
  │     └── Sessions (one per mask-on period)
  ├── Journals      (one per day)
  ├── Masks         (with aggregate therapy statistics)
  ├── Imports       (upload jobs; link to machine_id, device_id, mask_id, uploader_id)
  │     └── Import Files (content_hash, fingerprint_base64, path, download_url)
  └── Devices       (id, name, brand, device_type)
```

Three structural observations:

1. **The team is the root of the data model, not the user.** Machines, journals, masks, imports and files
   are all addressed under `/v1/teams/{team_id}/…`. This is a shared-account design: one team can hold
   multiple members and multiple machines. AnxietyWatch2's model is the inverse — a single owner, no
   tenancy, no sharing. The difference explains why SleepHQ needs `time_zone` per *team* as well as per
   *user*, and why it can serve clinics and AnxietyWatch2 cannot.
2. **The "machine date" is a first-class nightly record**, between the machine and its sessions, carrying
   its own eleven summary blocks, its own `time_zone` and `time_offset`, and a `large_leak` flag. Sessions
   hang off it. So SleepHQ's night is an explicit object with an editable time identity — closer to
   AnxietyWatch2's per-session model than to a flat row store, and reached without a hypertable.
3. **The mask is a modelled entity with its own statistics.** `Api_V1_MaskSerializer` publishes
   `average_ahi`, `average_pressure`, `average_leak_rate`, `average_usage_per_night` and `last_used_on`.
   So the platform ties therapy outcome to the physical mask a session used, and an import records which
   `mask_id` produced it. *Did the new mask improve my numbers* is answerable from the documented model.
   Neither AnxietyWatch version models a mask.

### 4.2 The documented derived-value surface — what SleepHQ computes `[DOCUMENTED]`

`Api_V1_SessionSerializer` carries **35 attributes**, of which these are derived rather than measured:

| Group | Fields |
|---|---|
| Apnoea event scoring | **`ahi`**, `oa_count`, `ca_count`, `h_count`, `ua_count` |
| Pressure percentiles | `pressure_set` and **`pressure_set_95`**, `ipap_set` and **`ipap_set_95`**, `epap_set` and **`epap_set_95`** |
| Flow-limitation percentiles | `flow_limit`, **`flow_limit_95`** |
| Leak | `time_over_leak_threshhold` |
| Desaturation durations | **`duration_90`**, **`drop_90`**, **`drop_3`**, **`drop_4`** |
| Oximetry score | **`o2_score`** |
| Fidelity marker | **`summary_calculated`** (boolean) |
| Omission marker | `omitted` (string) |
| Correction provenance | `moved_from_machine_date_id`, `moved_from_date` |
| Identity | `machine_session_id`, `device_session_key`, `machine_date_id`, `machine_id` |

This is the single most important static finding of the inspection, and it is the point of contact with
AnxietyWatch2's design decisions:

- **SleepHQ computes and publishes an AHI per session, plus the four component counts.** AnxietyWatch2
  computes no AHI at all. SleepHQ's own founder publishes that the device's AHI under-counts hypopnoeas
  (`sleephq_CLAIMED_FEATURES_AND_PURPOSE.md` §14) while the platform surfaces that same quantity as a
  headline field. Both statements are the vendor's own.
- **SleepHQ computes percentiles and V2 does not.** `*_95` on pressure, IPAP, EPAP and flow limitation, in
  addition to a scalar. The suffix `_95` is the only indication of *which* percentile — the specification
  does not state the estimator, the interpolation, the window, or whether the value is over the session or
  over the night. It is **labelled p95 by naming convention, not by documented definition**.
- **`duration_90`, `drop_90`, `drop_3` and `drop_4` are oximetry-derived** and are **undefined in the
  specification**. The naming convention (`_90`, `_3`, `_4`) suggests an SpO₂ threshold and a duration, but
  the specification says nothing: no units, no thresholds, no equations. A consumer integrating against
  these fields is integrating against a name. **This is a documented contract with an undocumented value
  domain** — the same class of hazard as OSCAR's 27-column positional header (candidate 4), arrived at
  from the opposite direction: OSCAR's meaning drifted under a stable name; SleepHQ's names were never
  defined.
- **`o2_score` is an integer with no published definition at all.** An integer score over oximetry, named
  and shipped, with no documented method. This is precisely the kind of figure this project's rules treat
  as dangerous: a number that looks like a measurement and is actually an unstated algorithm.
- **`summary_calculated` is a fidelity marker.** The presence of a boolean recording whether the summary
  was computed is a small but real design virtue: the record itself can say "this summary is not there
  yet". It is the closest thing in SleepHQ's model to this project's "unknown is not OK" rule, though the
  specification does not say what the summary fields contain when the flag is false.

### 4.3 The documented write surface, and what is missing from it `[DOCUMENTED]`

Eight write operations are published:

| Operation | Writable fields |
|---|---|
| `POST /v1/me` | `first_name`, `last_name`, `email` |
| `POST /v1/teams/{team_id}/imports` | `programatic`, `device_id`, `mask_id`, `name` |
| `POST /v1/imports/{import_id}/files` | `file`, `name`, `path` (SD-root-relative), **`content_hash` (required)** |
| `POST /v1/imports/{id}/process_files` | — (triggers processing) |
| `POST /v1/teams/{team_id}/journals` | `date`, `step_count`, `weight_grams`, `feeling_score`, `active_energy_joules`, `notes`, `sleep_stages` |
| `PUT /v1/journals/{id}` | same set |
| `PUT /v1/machine_dates/{id}` | **`time_offset` only** |
| `DELETE /v1/imports/{id}`, `DELETE /v1/imports/files/{id}` | — |

Four findings from that table:

1. **There is no write path to any session, and none to any derived value.** A consumer cannot set an AHI,
   correct a leak figure, or amend a session. The derived layer is read-only through the API, and the only
   correction the API permits on the time axis is the night's `time_offset`. So the *only* user-visible
   correction path for a wrong night is to re-date the night, and the only record that a session moved is
   the pair `moved_from_machine_date_id` / `moved_from_date`, which the server sets.
2. **Uploads are two-stage: upload, then `process_files`.** Ingestion work is explicitly triggered rather
   than implicit. AnxietyWatch2's equivalent principle — "attempts precede work" in the `cpap-ezshare-ingest`
   change — is the same instinct, expressed as a ledger rather than as an explicit step.
3. **Deletion exists and is not a tombstone.** `DELETE /v1/imports/{id}?purge_files=true` removes an
   import and its files. AnxietyWatch2 forbids `DELETE` against `samples` and requires a tombstone row.
   SleepHQ documents deletion. Recorded as a divergence, not a defect — the sampling granularities and the
   retention obligations are not comparable.
4. **Subjective data is accepted daily and only daily.** A consumer may write `feeling_score` and `notes`
   per *date*. There is **no documented path to write a symptom, a panic event, a medication dose, or a
   time-stamped feeling**. This is exactly the axis on which AnxietyWatch v1's panic-time capture is
   different, and the documented contract makes the difference unambiguous rather than inferential.

**One documented asymmetry worth naming.** `active_energy_joules` is accepted by both journal write
operations but **does not appear in `Api_V1_JournalSerializer`'s attribute list**. As documented, that
field is writable and never returned. It is one line in a specification, and the most likely explanation
is an omission in the published model rather than an intentional write-only field — but the *documented
contract* is that an integrator can send a value they cannot read back. This is recorded as a
documentation defect of the same family as the misspelled `time_over_leak_threshhold`, and both are
consumer-facing.

### 4.4 Undocumented value domains `[DOCUMENTED]` + `[STATIC]`

Four fields are published with a type of `string` or `integer` and **no enumeration, no description and no
range**:

| Field | Type | What is documented | What is not |
|---|---|---|---|
| `ImportSerializer.status` | integer | nothing | the meaning of any code |
| `ImportSerializer.progress` | integer | nothing | the scale (percent? 0–1? 0–1000?) |
| `MachineDateSerializer.usage` | string | nothing | whether it is a duration, a ratio, or a compliance band |
| `SessionSerializer.omitted` | string | nothing | whether "omitted" means excluded, quarantined, or never recorded |
| `TeamSerializer.teams_type` | string | nothing | the set of team kinds |
| `DeviceSerializer.device_type` | string | nothing | the set of device kinds |

An integrator polling an import for `status == 3` is guessing. The fields exist, are typed, and are
unusable without out-of-band knowledge. **This is the most consequential weakness found in the documented
contract**: three published status/state fields with undefined domains. It is also the reason this file
does not report *SleepHQ distinguishes not-measured from measured-and-good* — a field called `omitted`
that might carry that meaning exists, and the specification declines to say.

### 4.5 Provenance at the file level `[DOCUMENTED]`

| Field | Reading |
|---|---|
| `content_hash` | **Documented as MD5 of (file content + file name), path excluded**, with a worked example and a self-test endpoint |
| `fingerprint_base64` | A second file identifier, algorithm **not disclosed** |
| `path` | Required on upload, **relative to the SD-card root** |
| `size` | Byte size |
| `added_by_id` / `added_by_type` | Who added the file |
| `created_at` / `updated_at` | Timestamps |
| `download_url` | **"a signed URL that will expire in 5 minutes"** |
| `ImportSerializer` | `status`, `progress`, `failed_reason`, `programatic`, `processed_at`, plus `machine_id`, `device_id`, `mask_id`, `uploader_id` |

Two readings.

**The provenance model is file-oriented and honest about failure.** An import record carries an explicit
`failed_reason` and a `processed_at`, and the file record carries both a content hash and a second,
undisclosed fingerprint. An upload that fails is a record with a reason, not a silence. That is the same
commitment AnxietyWatch2 makes with its attempt ledger — reached independently, at a different layer
(SleepHQ records the *file*; V2 plans to record the *attempt*).

**The published hash algorithm is a real commitment.** The vendor publishes the exact algorithm, a worked
example, **and** a validation endpoint, then warns consumers not to use the endpoint in production. That
is the most integration-friendly piece of documentation in the entire candidate set: an algorithm stated so
precisely that a third-party implementation can be checked against it. It is also MD5 — recorded as a
documented choice, with the caveat that MD5's collision weakness is not the property that matters for
deduplicating one user's own SD-card files, and that the specification does not claim otherwise.

### 4.6 Authentication `[DOCUMENTED]`

| Item | Finding |
|---|---|
| Schemes | OAuth2 bearer token, **or** a logged-in browser session |
| Precedence | **"The user's session takes priority if both are present."** |
| OAuth flow | **`password` grant** — `POST /oauth/token` with `client_id`, `client_secret`, `grant_type=password`, `scope` |
| Scopes | `read`, `write`, `delete` |
| Token lifetime | **`expires_in: 7200`** (two hours) |
| Refresh | **"the OAuth spec does not allow the password grant to use refresh tokens. Even though you get a refresh token back in the response, you will need to generate a new token each time"** |
| Rate limits | Only `POST /v1/teams/{team_id}/imports` is capped (20/hour, 100/24 h per team). **Every** operation lists a `429`, because sign-in and client registration are limited by a mechanism not in the reference |

Three readings, all labelled.

**The password grant is a deprecated OAuth 2.0 flow.** It requires the consumer to handle the user's
password and cannot use refresh tokens, so a client must hold credentials and re-authenticate every two
hours. The vendor documents this limitation clearly and correctly. The consequence for an integrator is
real: **there is no way to obtain long-lived, revocable, refreshable access.**

**Session-takes-priority is a subtle hazard.** If a request carries both a session cookie and a bearer
token, the session wins. In a browser context an integrator can therefore believe they are testing API
credentials while actually acting as the logged-in user. The reference says this plainly, which is why it
is recorded as a documented behaviour and not a discovered flaw — but the vendor's own reference UI has to
warn that "to test API credentials, you need to log out of your SleepHQ account".

**The rate-limit disclosure is unusually candid and is recorded as a positive.** The reference explains
that buckets are fixed rather than rolling, that a straddling request can be allowed "up to twice a limit
in quick succession", that the 24-hour bucket reopens at UTC midnight, and that `retry-after` may therefore
be much shorter than the window implies. It also documents that the cap counts *requests*, not the dozens
of child imports a single accepted request can create, and that a card is split server-side into one child
import per calendar month (or per Philips P-Series folder). This is the level of self-description that
makes an interface usable, and it is the most honest limit documentation found in this candidate set.

### 4.7 A note on the specification's examples `[STATIC]`

Every model in the specification carries an `example` block. They are **synthetic**. The machine example
reads `"brand": "ad"`, `"model": "eos"`, `"serial_number": "quos"`, `"name": "magni"` — Latin filler — and
its `created_at`/`updated_at` are the timestamp of the fetch itself (`"2026-10-06 05:28:49 UTC"`), i.e. the
moment the specification was generated for this request. The examples are therefore produced by a
placeholder generator at render time, not captured from a real account.

Recorded because it matters for evidence discipline: **a reader could mistake these for real records**, and
the timestamps change on every fetch. Nothing in this file treats any example value as data about a real
user.

---

## 5. Runtime observation

**None. Zero.** No endpoint was called, no account was created, no trial was started, no app was
installed, no binary was executed, no share link was created.

Consequently the following remain **unobserved** and must not be reported as findings:

- what any endpoint actually returns, including whether the documented fields appear
- whether `summary_calculated` is ever false in practice
- whether `omitted` carries a three-state meaning
- what is inside the eleven `jsonb` summary blocks
- what the import `status` codes and `progress` scale mean
- whether the free tier's 30-day limit hides or deletes
- what a share link contains
- whether the mobile apps behave as their listings describe

---

## 6. Claim reconciliation

| # | Public claim | Documented contract | Reconciliation |
|---|---|---|---|
| 1 | "Track your AHI (apnea-hypopnea index)" | `SessionSerializer.ahi` (decimal) plus `oa_count`/`ca_count`/`h_count`/`ua_count` | **Consistent.** The claim is backed by a published field. Whether the value is re-derived by SleepHQ or relayed from the device is **not documented** |
| 2 | "Breath-by-breath… zoom to analyse airflow limitation and respiratory arousals" | `flow_limit`, `flow_limit_95` per session; **no documented waveform or high-resolution series endpoint** | **Not reconciled.** `flow_limit` is a scalar and its p95. No documented resource carries breath-level data, so how the breath-by-breath view is served is **not explained by the public contract**. Possibly served by an unpublished clinic/enterprise endpoint, or by the apps directly — **unknown** |
| 3 | "Detailed statistics on your therapy for every night" | Eleven `jsonb` summary blocks on `MachineDateSerializer` | **Consistent in existence, silent in content.** The blocks exist; their contents are undocumented |
| 4 | "Track your feelings with fun emojis & create custom flags" | `JournalSerializer.feeling_score` (integer), `notes` (string), `sleep_stages` (text); writable daily | **Partially reconciled.** `feeling_score` is a single daily integer with no documented range. A `sleep_stages` text field exists. **No field exists for "flags"**, and none for a time-of-day feeling |
| 5 | "Share your data… no personally identifiable information is shared" | No documented share-link resource | **Cannot be checked.** The de-identification claim is unfalsifiable from the public contract |
| 6 | "SleepHQ is not a medical device" | AHI, event counts and an undefined `o2_score` are published per session | **Compatible but worth naming.** The platform derives and publishes clinical-looking indices while disclaiming medical-device status. This is the same posture as OSCAR and the same one this project takes: surface the numbers, refuse the diagnosis |
| 7 | "Custom PDF reports with AI Insights" | **No documented endpoint, field or description of any AI feature** | **Not reconciled.** The API surface contains no AI resource. That does not mean the feature is absent — it may be a server-side report generator with no API — but the documented contract provides no evidence of it either way |
| 8 | "Automatically syncs… every 10 minutes" (App Store) | No documented sync schedule; no push/webhook resource | **Unreconciled.** A 10-minute cadence is a claim about the uploader hardware and the apps, neither of which is inspectable here |
| 9 | "set personalised alerts to receive notifications for specific oxygen & pulse rate level events" | No documented alert, threshold or notification resource | **Not reconciled.** The API exposes no alerting surface. The alerts are a device/app feature outside the public API |
| 10 | "only SleepHQ branded devices are compatible" (oximetry) | Events and SpO₂-derived fields are named for a single integrated sensor; `ImportFile` requires an SD-root-relative `path`, implying a card-oriented source | **Consistent.** The data model is built around card files and one integrated oximeter. There is no generic "import SpO₂ from any device" operation |
| 11 | "You can download all data from your SleepHQ account" | `download_url` — a **signed URL expiring in 5 minutes** — per import file | **Reconciled, with a caveat.** Portability is real and documented. The expiry means a downloader must fetch promptly; there is no documented bulk-archive endpoint |
| 12 | "All SleepHQ data is encrypted both at rest and in flight" | The specification says nothing about transport or storage security | **Outside the artifact.** No security property can be checked from an API specification. The claim rests on the FAQ and is contradicted in strength by the privacy policy and the Play panel (§10.1 of the claims file) |
| 13 | "Connect with registered CPAP professionals" | `TeamSerializer.teams_type` exists, unenumerated; clinical accounts have **unpublished** endpoints | **Consistent in shape, unverifiable in detail.** A team kind exists that could represent a clinic; its domain is undisclosed |
| 14 | "Advanced statistics… see how your pressure changes over time" | `pressure_set(_95)`, `ipap_set(_95)`, `epap_set(_95)` per session | **Consistent.** The per-session pressure and its p95 are published |

**Net result of reconciliation: the documented contract corroborates most of the analysis-related claims in
shape, contradicts none, and is silent on the three claims that matter most for this project — the AI
feature, the alerting feature and the breath-by-breath view.** The two claims that cannot be reconciled at
all (breath-by-breath, alerts) are exactly the ones a consumer would use to decide between this product and
a self-hosted alternative.

---

## 7. Evidence gaps

| # | Gap | Why it matters here |
|---|---|---|
| G1 | Contents of the eleven `jsonb` summary blocks | The per-night analysis depth is unknowable; the comparison cannot score it |
| G2 | Definition of `duration_90`, `drop_90`, `drop_3`, `drop_4` | Four oximetry-derived fields exist without units, thresholds or equations |
| G3 | Definition of `o2_score` | An integer score with no published method |
| G4 | `ImportSerializer.status` / `progress` codes | Polling an import is guesswork |
| G5 | Meaning of `omitted` | Cannot determine whether SleepHQ distinguishes not-measured from measured |
| G6 | Whether the free tier deletes or hides data past 30 days | A privacy and portability difference from a "deletes" policy like Welltory's |
| G7 | Whether the AHI filed on a session is relayed from the device or re-derived | Determines whether the founder's published criticism applies to the platform's own number |
| G8 | What "AI Insights" is | The privacy implication of an unnamed AI feature in a health platform |
| G9 | Whether the sleep alarm is supervised, and whether it distinguishes unauthorised or unmeasured from normal | This is the exact category AnxietyWatch2 specifies |
| G10 | What a share link actually contains | The PII claim rests entirely on vendor assertion |
| G11 | The `fingerprint_base64` algorithm | A second identifier with no stated derivation |
| G12 | Whether `active_energy_joules` is returned anywhere | A documented write-only field |
| G13 | Whether `_95` means a percentile estimator, and which | Percentile values are published with only a naming convention behind them |

None of these is closable by further reading of the same artifacts. G1, G4, G5 and G12 would become
visible from a single authorised API call; G2, G3 and G13 would require documentation or a support
statement; G7 and G8 require the vendor; G6 requires the vendor; G9, G10 and the app-level claims require
the binary or the hardware.

---

## 8. Ethical and legal limits applied

- The specification is published unauthenticated by the vendor; reading it required no bypass of any
  control.
- **No endpoint was called.** No authentication was obtained, no client registered, no rate limit
  encountered.
- No account, trial, purchase or subscription.
- No application binary was obtained, decompiled, patched or executed.
- No attempt was made to defeat authentication, TLS, or any platform protection.
- No personal data was involved; nothing was uploaded.
- No share link was created, so no third-party's data could be exposed.
- No credential, token or key was captured or transmitted.
- The clinic/enterprise API surface is documented as unpublished and was not probed.

---

## 9. Reproduction

```sh
# 1. The primary artifact
curl -sS -o swagger.json https://sleephq.com/api/swagger.json
shasum -a 256 swagger.json
#   9a4305e663ddc9b65c9b471e8fd04e1085f382814c3540193d85cbd8660536a4
python3 -c "import json;d=json.load(open('swagger.json'));print(d['swagger'],d['host'],d['basePath'],len(d['paths']),len(d['definitions']))"
#   2.0 sleephq.com /api 21 11

# 2. Enumerate the documented derived fields
python3 -c "
import json
d=json.load(open('swagger.json'))
a=d['definitions']['Api_V1_SessionSerializer']['properties']['data']['properties']['attributes']['properties']
print(len(a)); print([k for k in a])"

# 3. The documented hash algorithm
curl -sS https://sleephq.com/api-docs/index.html | grep -o 'MD5 hash of the file.s content and name joined together'

# 4. The date drift in the examples (they are generated at fetch time)
python3 -c "
import json
d=json.load(open('swagger.json'))
print(d['definitions']['Api_V1_MachineSerializer']['properties']['data']['example']['attributes'])"
```

The URLs above are public and require no credential. **The reproduction confirms only what the contract
says.** It cannot confirm what the service returns; no method in this file calls the service.

---

## 10. Related files

- `docs/app_comparison/sleephq/sleephq_CLAIMED_FEATURES_AND_PURPOSE.md` — the public claims
- `docs/app_comparison/sleephq/PROMPT_2.md` — the candidate-specific second-stage prompt
- `docs/app_comparison/sleephq/sleephq_COMPARISON.md` — the comparison report
- `docs/app_comparison/exist/exist_IMPLEMENTATION_INSPECTION.md` — the other documented-contract inspection
- `docs/app_comparison/oscar/oscar_IMPLEMENTATION_INSPECTION.md` — the source-inspection mode
- `docs/app_comparison/RESEARCH_METHOD.md` — inspection modes and evidence classes
