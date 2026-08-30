## ADDED Requirements

### Requirement: Thin-client authority boundary

The Apple client SHALL act only as a Pi-backed viewer, subjective-input capture client, and
HealthKit proxy. It SHALL NOT become a long-term system of record, run authoritative physiological
analysis, hold sensor BLE sessions, write to HealthKit, or participate in the Pi's local
sensor-to-klaxon dependency path.

#### Scenario: The phone is powered off during monitoring

- **GIVEN** the Pi monitoring stack is otherwise healthy
- **WHEN** the iPhone is powered off
- **THEN** collection, detection, and local alarm operation continue
- **AND** the Pi reports the optional phone source as absent or stale rather than treating it as
  healthy

#### Scenario: A viewer projection is unavailable

- **GIVEN** the app cannot obtain a current projection from the Pi
- **WHEN** it renders the corresponding surface
- **THEN** it shows a stale cache with age or an explicit unknown state
- **AND** it does not compute a replacement value locally

### Requirement: Temporary durable outbox with immediate upload

The client SHALL atomically enqueue every person-entered command and HealthKit proxy record before
reporting local capture success. It SHALL request an upload at the earliest iOS-permitted opportunity,
retain unacknowledged content without age eviction, and delete raw content from local persistence in
the same transaction that records terminal Pi acknowledgement.

#### Scenario: Journal entry is made while the Pi is unreachable

- **GIVEN** the Pi cannot be reached
- **WHEN** the user saves a journal rating
- **THEN** the immutable command and client ID are durably queued before success feedback
- **AND** the app shows it as queued rather than server-acknowledged

#### Scenario: Connectivity returns

- **GIVEN** one or more durable outbox items and no drain in progress
- **WHEN** network execution becomes available
- **THEN** the client starts one bounded drain without waiting for a scheduled daily window
- **AND** retries remain idempotent

#### Scenario: The Pi acknowledges an item

- **GIVEN** an outbox payload receives an item-level terminal acknowledgement
- **WHEN** the acknowledgement transaction commits
- **THEN** the raw HealthKit value, journal text, song text, or dose content is removed locally
- **AND** any bounded receipt contains identifiers/outcome/times only

### Requirement: Subjective input parity

The client SHALL preserve generation-one anxiety ratings, notes, tags, song attachment, backdating,
Express Mode, medication dose capture, dose follow-ups, random check-ins, and watch Quick Log,
subject to accepted server domain contracts. The Pi SHALL own authoritative records, catalogs,
policy, enrichment, analysis, and history.

#### Scenario: Express Mode capture

- **GIVEN** Express Mode is enabled
- **WHEN** the user taps one severity value
- **THEN** rating, subject time, record time, origin, and stable ID are durably queued in one action
- **AND** optional note, pain, tags, or song do not block capture

#### Scenario: Song is added offline

- **GIVEN** the Pi song catalog is unreachable
- **WHEN** the user attaches a song
- **THEN** the client queues either a cached server song ID or a free-form song stub
- **AND** all external catalog search/enrichment is deferred to the Pi

#### Scenario: An acknowledged entry is edited

- **GIVEN** an authoritative journal entry already exists on the Pi
- **WHEN** the user edits it
- **THEN** the client creates a new immutable command with a new ID and `supersedes` reference
- **AND** it does not mutate a local or server history row in place

### Requirement: Generation-one HealthKit proxy parity

The Stage 7 target SHALL account for every HealthKit type used by generation one's authorization,
observer, sample-mirror, snapshot-input, and UI sets. Each SHALL be mapped as a source sample/category,
explicitly context-only, blocked on a named registry decision, or excluded with an accepted reason.
The phone SHALL proxy source data and provenance but SHALL NOT port generation-one snapshots,
aggregation, trend thresholds, precedence, scoring, or analysis.

#### Scenario: A generation-one HealthKit type is not in the current registry

- **GIVEN** the parity ledger identifies a used HealthKit type without a canonical v2 type
- **WHEN** Stage 7 implementation reaches that type
- **THEN** it remains blocked with a named registry owner or receives an accepted exclusion
- **AND** the app does not silently omit it or invent an identifier/unit/range

#### Scenario: HealthKit produces a supported sample

- **GIVEN** the registry and server mapper support the HealthKit type
- **WHEN** an anchored query returns a sample
- **THEN** the client enqueues its UUID, original identifier/unit/value, times, source revision,
  device, and allowlisted metadata
- **AND** the server performs canonical conversion and validation

### Requirement: HealthKit anchoring and permission truthfulness

The client SHALL request read-only access only for enabled types. It SHALL keep a separate anchored
query cursor per type and persist the next anchor only in the transaction that enqueues every result
from that query page. Because HealthKit read permission is not introspectable, the client SHALL
report authorization request, query, matching sample, and Pi acknowledgement as separate facts.

#### Scenario: A query transaction crashes

- **GIVEN** HealthKit returned additions and a next anchor
- **WHEN** the app terminates before additions and anchor commit together
- **THEN** the prior committed anchor is used on the next query
- **AND** server idempotence absorbs replayed sample UUIDs

#### Scenario: No matching sample is returned

- **GIVEN** authorization was requested for a type
- **WHEN** a query succeeds with no matching sample
- **THEN** the app reports successful query and absent sample separately
- **AND** it does not claim permission was granted, denied, healthy, or value zero

### Requirement: Registry-driven client mapping

The client SHALL fetch versioned `registry.json` and SHALL NOT hardcode numeric source/type IDs,
canonical units, valid ranges, or thresholds. OpenAPI may generate transport shapes but SHALL NOT
replace the registry as physiological source of truth.

#### Scenario: The registry contains an unknown type

- **GIVEN** the client fetches a registry version containing an unknown entry
- **WHEN** it decodes that registry
- **THEN** it preserves the entry for diagnostics and treats it as unsupported
- **AND** it does not map it to a known/default type

### Requirement: Background proxy evidence

HealthKit observer callbacks SHALL enqueue bounded query work and complete promptly. Foreground,
network recovery, app activation, HealthKit background delivery, and scheduled background tasks
SHALL request bounded drains. The client SHALL emit a measurement-free status beacon, and the Pi
SHALL own cadence degradation and silence detection.

#### Scenario: Observer registration succeeds but no data is uploaded

- **GIVEN** an observer and background-delivery request succeeded
- **WHEN** no matching sample or Pi acknowledgement follows within expected cadence
- **THEN** the status model preserves the successful registration fact
- **AND** receiving state becomes unknown/stale rather than healthy

#### Scenario: iOS does not wake the app

- **GIVEN** the app is suspended or force-quit and sends no new beacon
- **WHEN** the last beacon exceeds its cadence
- **THEN** the Pi detects and reports silence
- **AND** the last beacon does not continue asserting health indefinitely

### Requirement: Secure enrollment and protected transport

Production communication SHALL use the accepted versioned Rust API over authenticated TLS with a
single-use enrollment flow and per-device revocable credentials. Credentials SHALL use Keychain with
an accepted `ThisDeviceOnly` accessibility class and SHALL NOT appear in source, UserDefaults, logs,
backups, URLs, or diagnostic exports. Production SHALL have no cleartext ATS bypass or trust-all
handler.

#### Scenario: A credential rotates with pending writes

- **GIVEN** the outbox contains unacknowledged records
- **WHEN** the per-device credential is rotated
- **THEN** queued identities and payloads remain deliverable under the accepted rotation protocol
- **AND** old credential material is removed after successful transition

#### Scenario: TLS trust fails

- **GIVEN** hostname or certificate validation fails
- **WHEN** the app attempts a read or write
- **THEN** it blocks transport with a named trust error
- **AND** it neither bypasses validation nor deletes queued data

### Requirement: Explicit stale, unknown, and conflict states

The client SHALL preserve live, failing, unknown, not-configured, queued, and conflict as distinct
states. Missing or malformed data SHALL NOT be coerced to zero, empty success, or healthy. Same-ID
same-content writes SHALL replay as no-ops; same-ID different-content writes SHALL surface conflict
without overwriting either fact.

#### Scenario: Same identity carries different content

- **GIVEN** the Pi already stores a record ID
- **WHEN** the client sends different content under that ID
- **THEN** the Pi returns conflict and the client preserves the rejected content for resolution
- **AND** neither record is silently overwritten

### Requirement: Watch Quick Log relay

The initial watchOS client SHALL mint a stable ID and durably queue Quick Log on the watch, relay it
through iPhone using WatchConnectivity, and retain it until the phone acknowledges durable local
enqueue. The watch SHALL NOT hold a Pi credential or run analysis.

#### Scenario: Phone is disconnected during Quick Log

- **GIVEN** the watch cannot reach the phone
- **WHEN** the user taps a severity
- **THEN** the watch durably queues the stable command within the panic-path interaction
- **AND** relays it when the phone becomes reachable without creating a new identity

### Requirement: Accessible panic-path interaction

Express Mode and watch Quick Log SHALL be operable in one tap after opening their capture surface,
with a measured median completion below five seconds on the owner's physical devices. All client
surfaces SHALL support Dynamic Type, VoiceOver, 44-point targets, reduced motion, sufficient contrast,
and non-color status labels.

#### Scenario: VoiceOver user opens Quick Log

- **GIVEN** VoiceOver is active
- **WHEN** the severity grid appears
- **THEN** every severity has a meaningful label, value, and action in logical order
- **AND** queued/acknowledged state is not communicated by color alone

### Requirement: Verified migration without runtime mirroring

Generation-one retained records SHALL be migrated once through idempotent production contracts with
migration provenance and attempt evidence. A second run SHALL be a no-op. Generation-one remains
read-only until per-class count/date/provenance and sentinel checks pass; its long-term local mirror
SHALL NOT become Stage 7 runtime architecture.

#### Scenario: Migration replay

- **GIVEN** a completed first migration run
- **WHEN** the same export is imported again
- **THEN** accepted records replay as no-ops and conflicts/rejections are explicit
- **AND** no duplicate authoritative history is created

### Requirement: Local alarm independence

The Apple client MAY display remote alarm information after the M6 channel contract exists, but it
SHALL NOT be required for Pi detection or local alarm output and SHALL NOT silently clear the local
alarm state.

#### Scenario: Remote notification is not delivered

- **GIVEN** the Pi enters an alarm state and APNs or the phone is unavailable
- **WHEN** remote delivery fails
- **THEN** the Pi local alarm path continues independently
- **AND** remote delivery evidence is failing/unknown rather than inferred successful
