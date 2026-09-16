# Specification — AnxietyWatch2 Apple client (Stage 7)

**Status:** implementation target; subordinate to accepted amendments
**Scope:** eventual iPhone app, companion watchOS Quick Log, and their HTTP contracts with the Pi.
The first draft is iPhone-only and follows [Amendment 10](../specs/2026-09-15-amendment-10-first-draft-lan-http.md)
for private-LAN transport and deferred features.
**Implementation home:** `clients/apple/` in this repository
**Related:** [`GEN1-REASSESSMENT.md`](GEN1-REASSESSMENT.md) ·
[`HOMEASSISTANT-PRIOR-ART.md`](HOMEASSISTANT-PRIOR-ART.md) ·
[Amendment 07](../specs/2026-08-05-amendment-07-api-in-rust.md) ·
[Stage 7 change plan](../../openspec/changes/ios-client-stage7/proposal.md)

This document turns the architecture's phrase **“HealthKit bridge in, journal in, viewer out”** into
an implementable client contract. If this document conflicts with an accepted amendment, the
amendment wins and implementation stops until the conflict is resolved.

---

## 1. Purpose and boundary

The Apple client exists because some capabilities are device-local or person-local:

1. HealthKit can be read only on an Apple device.
2. Anxiety, pain, medication doses, and check-in responses must be captured where the person is.
3. The owner needs a portable viewer and report reader.
4. A watch can provide a sub-five-second Quick Log during panic.

The Pi remains the system of record, analysis host, integration hub, and independent overnight
monitor. The app is useful when the Pi is temporarily unreachable, but it is never part of the
local detector-to-klaxon path. In the first draft, HealthKit, watchOS, reminders/notifications, and
non-journal domain features are deferred; the app captures journal/anxiety records and reads Pi
projections and prior records.

### 1.1 The three jobs

| Job | Phone responsibility | Pi responsibility |
|---|---|---|
| HealthKit bridge in | request narrowly scoped read access, query, normalize transport payloads, enqueue, retry | authenticate, map to registry types/units, validate, store, expose receiving state |
| Journal in | capture reports and doses, schedule local prompts, enqueue immutable versions | own authoritative records, conflict history, policy, analysis, export |
| Viewer out | render server projections and explicit stale/unknown states | compute projections, analysis, reports, and evidence ages |

### 1.2 Explicit non-goals

The Stage 7 app does **not**:

- hold Polar H10, EMAY, cuff, or Oura BLE sessions;
- compute authoritative baselines, CNS risk tiers, correlations, or clinical reports;
- write any data back to HealthKit;
- maintain a second authoritative health database;
- clear or suppress the Pi's local alarm path;
- equate a running app, successful request, or granted-looking toggle with fresh data;
- restore by replacing the Pi's store or use a wall-clock sync cursor;
- ship advertising, tracking, or general-purpose analytics SDKs.

---

## 2. Binding product decisions

1. **Server authority.** The Pi owns authoritative durable truth. Local persistence includes a durable
   temporary write outbox, server-issued read cursors, settings, bounded physiological display caches,
   and a permanent Data-Protected journal/anxiety mirror for offline reading and reconciliation. The
   journal mirror is not authoritative and is never used for local analysis.
2. **Upload at the earliest opportunity.** A new journal/song/rating/dose/check-in or HealthKit
   observer result triggers a drain immediately when iOS permits network work. Foreground entry,
   connectivity restoration, app activation, HealthKit background delivery, and scheduled background
   refresh are all drain triggers; none waits for a daily sync window.
3. **One-way writes, ordinary reads.** Phone-to-Pi is an idempotent enrichment push. Pi-to-phone is a
   query/projection API. Generation one's bidirectional CRDT is not ported.
4. **Batch transport, not batch delay.** HealthKit and domain writes use bounded HTTPS batches to
   reduce overhead, but the first item starts a drain immediately. A live socket is a later
   optimization only after a measured latency requirement has a consumer.
5. **Watch relays through iPhone.** The first watchOS client uses `WatchConnectivity`; it holds no Pi
   credential. Direct watch-to-Pi communication is deferred unless measured delivery failures prove
   the relay inadequate.
6. **Registry-driven platform data.** The app reads versioned `registry.json`; it never hardcodes
   numeric source/type identifiers, canonical units, valid ranges, or server thresholds. The server
   remains the final validator.
7. **No generated Swift registry.** Registry JSON is decoded into client models and cached with its
   version. Unknown entries are preserved for diagnostics and not guessed into known types.
8. **No BLE compatibility mode.** Generation-one BLE services are removed rather than hidden behind
   a default-off switch.
9. **HealthKit reads only.** The app requests only enabled read types and has no HealthKit share/write
   authorization.
10. **Facts, not permission guesses.** HealthKit does not reveal whether read permission was granted.
   The app reports request outcome, query outcome, last matching sample, and last acknowledged upload
   separately; it never labels read access “granted” from absence of an error.
11. **Existing app knowledge, new narrow tree.** Selected generation-one UI and behavior are imported
    into `clients/apple/` with provenance. The old `Services/` tree is not copied wholesale.
12. **Native-first implementation.** Swift, SwiftUI, structured concurrency, `URLSession`, HealthKit,
    `BGTaskScheduler`, UserNotifications, and WatchConnectivity are the defaults. A third-party
    dependency requires a license, privacy, maintenance, and necessity record.
13. **Missing is never zero.** This applies to HealthKit values, journal fields, check-ins, graph
    gaps, and server state.

---

## 3. User-visible information architecture

The app has six primary surfaces. Navigation labels may evolve, but responsibilities may not move
across the phone/Pi boundary silently.

### 3.1 Today

- Latest server-projected physiological summary with observation age.
- Current monitoring state as reported by the Pi, including `UNKNOWN`/unreachable; never recomputed
  locally.
- Promoted data-coverage gaps and pending check-ins.
- A persistent stale banner when the last successful read exceeds the endpoint's declared cadence.
- No empty-state illustration that resembles a healthy state.

### 3.2 Journal

- Anxiety severity using the generation-one 1–10 scale and named bands.
- Optional note, tags, and song. The entry time is captured by the iPhone when the user saves it;
  user-entered backdating is not supported. Search uses the Pi Genius proxy when its key is available,
  or direct Genius when the owner has explicitly provisioned the key to the iPhone; otherwise it uses
  the no-login/no-secret Apple/iTunes Search API. Genius failures fall back to iTunes with the
  provider shown. The mirror retains title, artist, album, provider metadata, artwork URL, and a small
  local thumbnail for offline display. A manual free-form song stub remains available offline. The Pi
  may enrich the synced record with Genius.com lyrics using its key, controlled by a Pi-admin global
  setting that defaults to automatic; without a key, it is off and disabled with a guided setup note.
  Public-release credentials, licensing, disclosure, and optional AI analysis require a separate
  future decision.
- Optional pain only after its scale and missing-value behavior are accepted in the domain schema.
- Express Mode preserves one-tap anxiety capture. Optional secondary fields cannot delay the save.
- Entry detail shows subject time, recorded time, sync state, and version/conflict state.
- Editing creates a new immutable version; deletion creates a tombstone request.

### 3.3 Songs

- Provide a dedicated Songs section alongside Journal.
- List songs referenced by journal entries with their available title, artist, album, artwork,
  provider ID, lyrics state, and synchronization state.
- Tapping a song opens a detail surface containing the complete metadata snapshot available to the
  Pi/iPhone, the artwork, lyrics when present, lyrics provider and retrieval evidence, and explicit
  pending/unavailable/unknown states when lyrics are not available.
- A song detail surface never invents lyrics or treats a missing lyrics response as an empty lyric.
- Journal entries link to this same song detail surface; the song record is not duplicated into a
  second independent history.

### 3.4 Medications and check-ins

- Log a dose against a server-owned medication definition.
- Show last-taken time to reduce accidental duplicate dosing, while clearly identifying stale data.
- Schedule local dose follow-ups from server policy.
- Record prompt outcomes separately: answered, declined, expired unanswered, and never delivered.
- Random check-ins use server policy (enabled, active hours, frequency) but are scheduled locally.
- Logging a safety-relevant dose must be acknowledged by the Pi before the app claims the server's
  monitoring window is armed. “Saved on this phone” and “arming acknowledged” are separate states.

### 3.5 History and reports

- Query server projections using opaque server cursors and explicit UTC windows.
- Preserve null gaps; never interpolate physiological data or coerce absent values to zero.
- Render server-produced analysis and clinical PDFs; the phone does not regenerate them.
- Export/share uses a server-produced artifact and the standard iOS share sheet.

### 3.6 Health access and sync

Per HealthKit type, show these independent facts:

- configured on/off;
- authorization request attempted and its result;
- observer registered and background delivery request result;
- last query attempted, query result, and anchor age;
- last matching HealthKit sample time, if one exists;
- pending outbox count and oldest age;
- last batch acknowledged by the Pi;
- Pi's last receiving observation for that type.

A type with no matching sample is not automatically broken: it may be denied, unmeasured, or simply
absent. The UI states exactly which fact is known.

### 3.7 Settings and diagnostics

- Server endpoint, authenticated device label, credential creation/revocation state, and TLS state.
- Registry/API/app build versions.
- Bounded redacted sync diagnostics with a user-initiated export.
- HealthKit type selection and rationale before authorization.
- Notification policy and system authorization state.
- Privacy screen, journal-mirror backup inclusion choice, and a destructive local reset that does not
  imply server deletion. The backup choice is asked on first launch before journal use and remains
  changeable in General Settings.

---

## 4. Data and identity contracts

### 4.1 Domain records

Every phone-created record has a client-minted UUID (UUIDv7 when the platform API permits; UUIDv4 is
acceptable until then). Identity is never a timestamp or content hash.

- New ID + content creates a record.
- Same ID + same canonical content is an acknowledged no-op replay.
- Same ID + different content is a conflict, never an overwrite.
- An edit has a new ID plus `supersedes` pointing to the previous version.
- A removal is an append-only tombstone with its own ID and target ID.
- Subject time is the iPhone capture instant in the first draft. The Pi receipt instant is retained
  separately as transport evidence and never replaces the entry time.

The app retains an outbox item until it receives an item-level terminal acknowledgement. Batch-level
HTTP success alone is insufficient.

### 4.2 HealthKit samples

The final Stage 7 target is **generation-one HealthKit coverage parity**, not merely the two types in
today's registry. T-I0 produces a checked parity ledger from generation one's authorization set,
sample mirror, anchored observers, and snapshot inputs. That ledger currently includes the general
families below:

- heart rate, resting heart rate, HRV SDNN, respiratory rate, and oxygen saturation;
- sleep analysis and sleeping wrist/body temperature;
- systolic/diastolic blood pressure, blood glucose, body mass, and demographics used for context;
- steps, active energy, exercise time, workouts, physical effort, and time in daylight;
- VO₂ max, walking heart rate, speed, step length, steadiness, double-support, and asymmetry;
- environmental/headphone audio exposure and atrial-fibrillation burden.

Parity means proxying source samples and their provenance, **not porting generation one's snapshots,
trend thresholds, aggregation, scoring, or analysis**. Any excluded generation-one type gets a named
reason in the ledger; it cannot disappear because the current v2 registry is smaller.

Implementation remains registry-gated: each parity type first receives a canonical registry type,
unit/range, HealthKit mapping, and server receiver contract. The client sends only types declared for
`source.healthkit` by the fetched registry, so registry expansion and client rollout proceed together
without the app inventing server semantics.

Each transport sample includes, subject to the server contract established by T-X3:

- HealthKit sample UUID;
- HealthKit type identifier;
- start and end instants in UTC;
- original value and unit identifier, before server-side canonical conversion;
- source revision identifiers (bundle identifier, source name, product type/version when available);
- device metadata when available;
- a strict allowlist of HealthKit metadata fields;
- query/observer batch identity and client record time.

Sub-provenance is mandatory. A Watch measurement, manually entered value, and third-party-app value
must not collapse into an undifferentiated `healthkit` row. The server defines stable ingest identity
and canonical conversion; the phone does not invent `(session_id, seq)`.

### 4.3 Status beacon

The status beacon is operational evidence, stored separately from physiological samples. It contains
no measurement values. It reports per-type facts from §3.5, app/build/registry version, outbox count
and oldest age, last successful server acknowledgement, and timestamp of the current assertion.

The beacon cannot claim the app is healthy after it stops running. The Pi owns silence detection and
degrades the last beacon by cadence.

### 4.4 Read models

- Read APIs return server-computed projections with `observed_at`, `generated_at`, and explicit
  coverage/gap metadata.
- Pagination uses opaque server cursors, never client-computed timestamps.
- Physiological projection cache entries carry endpoint, authenticated client, schema version,
  response generation time, and expiration. A cache from another server/account is never reused.
- Physiological cached content can remain visible while stale only with a persistent stale state and
  its age. Journal/anxiety history is held in the permanent local mirror and reconciled from the Pi.

---

## 5. Durable delivery and background behavior

### 5.1 Outbox transaction

Capture and enqueue are one local transaction. A UI success means **durably queued on this device**,
not accepted by the Pi. The outbox stores the immutable payload, record ID, attempt count, next
eligible attempt, and last classified error.

The outbox is not a general mirror:

- an acknowledged raw HealthKit payload is deleted from local persistence in the same transaction
  that records its terminal acknowledgement;
- acknowledged journal/anxiety payloads are reconciled into the permanent local mirror after terminal
  acknowledgement; the mirror stores the content needed for offline history and version reconciliation;
- a bounded receipt may retain only record ID, outcome, acknowledgement time, and server attempt ID
  — never HealthKit values or journal content outside the protected mirror;
- physiological server read responses use bounded, expiring caches; journal/anxiety history uses the
  protected local mirror and is reconciled from the Pi when reachable;
- unacknowledged items are the unavoidable exception: they remain until acknowledged or the user
  explicitly accepts a named loss. They are never age-evicted to satisfy a storage limit;
- logout, reset, and server replacement always keep unsent records; queued items are never discarded
  by these flows;
- uploaded records can be deleted normally by creating a tombstone, which remains part of the Pi's
  authoritative audit trail.

There is no local HealthKit browser, local report archive, or local analysis over the outbox or
journal mirror. Once the Pi acknowledges a write, the Pi remains the authoritative long-term owner;
the iPhone's journal mirror is a recoverable convenience replica.

### 5.2 Retry

- Attempt an immediate drain whenever a new item arrives and iOS permits it.
- Retry transport failures, timeouts, 408, 429, and retryable 5xx responses with bounded jittered
  exponential backoff.
- Honor `Retry-After` within a configured ceiling.
- Do not retry validation errors or identity conflicts automatically.
- Cancellation must cancel the request without deleting its outbox item.
- Manual Sync Now bypasses the wait once; it does not create a parallel drain.
- At most one drain owns a given client outbox.

### 5.3 Background execution

HealthKit observer callbacks enqueue work and complete promptly. They do not perform unbounded
network transfers. `BGAppRefreshTask` and, where justified, `BGProcessingTask` drain bounded batches.
Foreground activation and explicit user actions also trigger a drain.

No one-second timers, permanent sockets, background audio, location abuse, or BLE keep-alive are
permitted. iOS background execution is best effort; the Pi's status-beacon silence detector is the
source of truth about whether delivery is happening.

### 5.4 HealthKit anchoring

Each type has its own committed `HKQueryAnchor`. The next anchor is persisted only in the same local
transaction that durably enqueues every result from that query page. A crash before commit replays
the old anchor and idempotence absorbs duplicates. Advancing an anchor before enqueue is forbidden.

Deletes reported by anchored queries become explicit deletion/tombstone events only after the server
contract defines them; they are never silently ignored or translated into SQL deletion.

---

## 6. API and enrollment

All production communication uses the versioned Rust `axum` API required by Amendment 07. The API
publishes OpenAPI and representative fixtures; generated Swift transport types may consume that API
schema, but physiological registry types still come from `registry.json`.

### 6.1 Enrollment contract

[Amendment 09](../specs/2026-08-30-amendment-09-mobile-enrollment-and-tls.md) defines the eventual
production authentication/TLS decision. [Amendment 10](../specs/2026-09-15-amendment-10-first-draft-lan-http.md)
supersedes its public/TLS requirements for the first draft only. The currently shipped
`/api/envelope/register` remains test-only and is **not** a remote enrollment or compatibility flow.

The first draft uses a manually configured private-LAN Pi address over HTTP and has no away-from-home
operation. It still requires a ten-minute single-use artifact created after recent operator
authentication, a non-exportable Secure Enclave P-256 key, scoped per-device credentials, RFC 9421
signatures, content digests, bounded creation/expiry times, and durable replay IDs. Rotation signs a
new key with the current key and never rewrites credential-neutral queued payloads; revocation/recovery
preserves every unacknowledged item.

Before public or away-from-home use, the Amendment 09 Caddy HTTPS origin, ATS system trust, hostname
validation, and no-cleartext gate must be restored.

The app never embeds a production endpoint, bearer token, symmetric key, private key, or certificate
exception in source or configuration committed to git.

### 6.2 Versioning and errors

- API paths or media types carry a major version.
- Every write item returns created, replayed, rejected, conflict, or retryable with a stable code and
  human-readable named blocker.
- Unknown response fields are ignored; unknown enum values are preserved as unknown, not mapped to a
  healthy/default case.
- Incompatible major versions block writes while preserving the outbox and allowing diagnostics.
- Server time and client time are both recorded; server receipt time never rewrites subject/event
  time.

---

## 7. Security and privacy requirements

These requirements apply OWASP MASVS-AUTH, STORAGE, NETWORK, CODE, PLATFORM, and PRIVACY controls to
this single-owner health client.

### 7.1 Credentials and local protection

- Device credentials live in Keychain with a `ThisDeviceOnly` accessibility class appropriate for
  background delivery after first unlock; never `UserDefaults`, source, logs, or an export.
- The durable outbox and physiological projection cache use iOS Data Protection and are excluded from
  device backups. The permanent journal/anxiety mirror also uses Data Protection, with backup inclusion
  controlled by the user's first-launch choice and General Settings. The selected protection class must
  permit the documented background behavior and is verified on a locked physical device.
- An optional Face ID/Touch ID foreground privacy lock is separate from the background transport
  credential; enabling it must not turn a biometric boolean into the cryptographic access control.
- Sensitive screens are obscured in the app-switcher snapshot.
- Diagnostic exports require explicit action, contain no credentials, and redact free text and
  physiological values unless the user selects an identified data export flow.

### 7.2 Network

- The eventual production target keeps App Transport Security enabled with no cleartext exception.
  The first draft is the explicit Amendment 10 exception: HTTP is allowed only to the configured
  private-LAN address, and the UI labels it first-draft LAN-only transport.
- Device signatures, scoped credentials, replay protection, and private-address validation remain
  mandatory in LAN mode. HTTPS system trust and hostname validation return before public or
  away-from-home use.
- Requests have bounded connect/resource timeouts, body limits, and response decoding limits.
- Secrets and health payloads never appear in URLs, analytics, crash breadcrumbs, or OSLog.

### 7.3 Authorization and minimization

- HealthKit access is opt-in per type with a plain-language purpose shown before the system sheet.
- The app requests read-only access to the enabled subset and remains useful when any subset is
  denied or absent.
- No tracking permission, contacts, location, microphone, camera, or Bluetooth permission is added
  unless a separately specified feature requires it. OCR may request camera/photo access only at
  point of use.
- Third-party code is pinned and inventoried; an SBOM and privacy-manifest review gate every release.
- The privacy screen identifies what stays on device, what is sent to the Pi, and what server export
  or deletion operations exist.

---

## 8. Accessibility, panic-path, and presentation

- Express Mode and watch Quick Log require no more than one tap after opening the capture surface and
  target a median completion under five seconds on a physical device.
- Touch targets are at least 44×44 points; Dynamic Type works through accessibility sizes; VoiceOver
  has meaningful labels/order/actions; color is never the only carrier of severity or sync state.
- Reduce Motion and Increase Contrast are respected.
- Values always include units. Date/time presentation uses the user's locale, while stored/transmitted
  instants remain UTC.
- Graphs expose a textual summary and explicit missing intervals.
- Destructive actions name the affected record/outbox count and require proportionate confirmation.
- Notifications contain minimal lock-screen content by default and offer a privacy setting for detail.

---

## 9. Notifications and watchOS

### 9.1 Local prompts

Reminders and notifications are deferred from the first draft. The first draft schedules no journal,
dose, check-in, or sync reminders. A later release may add person-facing prompts under a separately
accepted notification policy; unknown delivery must never be converted to declined.

### 9.2 Remote alarm information

Remote notifications may report the Pi's alarm state when the M6 channel contract exists, but they
are a redundant convenience, not proof that the local alarm worked. Notification receipt, display,
and acknowledgement are separate evidence. The app cannot silently clear the local tier.

### 9.3 Watch Quick Log

- The watch captures severity with the existing color-graded grid and accessibility equivalents.
- It assigns a stable client ID before transfer and retains the item until iPhone acknowledgement.
- The phone preserves that ID into its own outbox and acknowledges the watch only after durable local
  enqueue, not after server delivery.
- Duplicate transfers replay as no-ops.
- The watch displays queued-on-watch, queued-on-phone, and server-acknowledged as distinct facts when
  available.

---

## 10. Observability and failure semantics

The app and server use a shared failure taxonomy:

| State | Meaning | Required presentation |
|---|---|---|
| live | affirmative evidence within declared cadence | evidence age and expectation available |
| failing | an attempt ran and failed | named operation, reason, and last attempt time |
| unknown | no fresh affirmative evidence | never rendered as good or zero |
| not configured | deliberately absent | quiet and distinct from broken |
| queued | durable locally, not acknowledged | count and oldest age |
| conflict | same identity, different content | preserve both facts; explicit resolution |

The app exposes sync attempt history with operation class, start/end, outcome, created/replayed/
rejected/conflict counts, and sanitized reason. A successful no-op replay is success but not progress.
The oldest pending outbox item is the write high-water blocker and must remain visible.

---

## 11. Migration from generation one

Migration is a one-time, measured import—not normal bidirectional sync.

1. Inventory generation-one capabilities and committed local data by class.
2. Export immutable source fixtures before deleting services.
3. Import through the same idempotent server contracts used in production, with a dedicated migration
   provenance marker and attempt ledger.
4. Re-run the import and prove it is a no-op.
5. Compare per-class counts, min/max dates, and known sentinel records; count parity alone is not
   enough.
6. Keep the generation-one app read-only until every retained capability has an accepted owner and
   the verified migration report is archived.
7. Never claim migration complete while unsent phone/watch records or unmodeled classes remain.

The old HealthKit mirror table, snapshot repair controls, BLE services, local analysis, and local PDF
generator are not migrated as runtime architecture. Their data, clinical rules, and test fixtures are
retained where the reassessment requires them.

---

## 12. Testing and release gates

### 12.1 Automated layers

- Pure Swift package tests: registry decoding, unit mapping inputs, status model, retry policy,
  canonical envelope serialization, cursor/cache rules, and no absent-to-zero conversion.
- Contract tests: OpenAPI compatibility and shared JSON fixtures tested by Rust and Swift.
- Persistence tests: crash at every anchor/outbox boundary, replay, conflict, cancellation, logout,
  and credential rotation.
- UI tests: onboarding, partial HealthKit access, offline capture, stale viewer, Express Mode,
  Dynamic Type, VoiceOver labels, reduced motion, and destructive confirmations.
- Security checks: committed-secret scan, ATS configuration, Keychain accessibility, backup
  exclusion, log redaction, dependency audit, privacy manifest, and release entitlement diff.
- Performance/energy: bounded history rendering, no polling loop, observer debounce, background batch
  size, launch responsiveness, and overnight battery impact.

### 12.2 Physical-device gates

Simulator tests do not prove HealthKit background delivery, notification behavior, locked-device data
protection, WatchConnectivity, energy use, or credential persistence. Milestone acceptance therefore
runs on the owner's iPhone and Apple Watch using fixed reference fixtures plus explicitly identified
personal-data checks that are never committed.

### 12.3 Stage 7 completion

Stage 7 is complete only when:

1. The generation-one HealthKit parity ledger has an implemented registry/receiver mapping or a
   named, accepted exclusion for every type; one sample of every enabled type is acknowledged by the
   Pi with preserved sub-provenance and appears in the server receiving-state view.
2. Denied, absent, stale, malformed, and out-of-range HealthKit paths remain distinct and never
   become zero or healthy.
3. A journal Quick Log made offline survives termination/reboot, replays after connectivity, and a
   second replay is a server no-op.
4. A same-ID/different-content conflict is visible and neither version is silently overwritten.
5. A watch Quick Log survives a disconnected phone and preserves identity through eventual server
   acknowledgement.
6. Local check-in outcomes distinguish answered, declined, expired, and delivery unknown.
7. The viewer presents stale caches as stale and preserves graph gaps.
8. The eventual production networking target has no cleartext/ATS bypass; the first draft's explicit
   Amendment 10 LAN-only HTTP exception is private-address validated and retains signed requests.
   Credentials are revocable and absent from logs, backups, and diagnostic exports.
9. Accessibility and panic-path timing pass on physical devices.
10. With the iPhone powered off, Pi collection, detection, and local alert acceptance remain green.
11. The verified generation-one migration report has no unowned retained capability or unexplained
    record-count/date discrepancy.

---

## 13. Dependency gates

| Gate | Blocks | Owner |
|---|---|---|
| G-IOS-AUTH — LAN/HTTPS/enrollment decision | production enrollment and writes; first-draft LAN writes follow Amendment 10 | admin roadmap T-M2a + Stage 7 T-I1 |
| G-IOS-PROVENANCE — platform sub-provenance and identity | HealthKit payload/receiver | admin roadmap T-X3/T-X4 |
| G-IOS-BEACON — status-beacon server contract | phone health beacon | admin roadmap T-X1/T-X2 |
| G-IOS-RECORDS — domain schema/version ownership | journal, medication, check-in writes | admin roadmap T-M5a/T-M5b |
| G-IOS-REPORTS — read/report APIs | complete viewer | admin roadmap M3/M4 |
| G-IOS-MIGRATION — verified backfill contract | gen-1 decommission | admin roadmap T-X5/T-X6 |
| G-IOS-ALARM-PUSH — remote channel protocol | remote alarm presentation only | admin roadmap M6 |

Blocked features remain absent or explicitly unavailable. They are never mocked into a production
success state to make the app look complete.
