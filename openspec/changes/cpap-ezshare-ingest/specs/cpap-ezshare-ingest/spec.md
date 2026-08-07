# CPAP ez Share ingestion capability

## ADDED Requirements

### Requirement: Confined and bounded card mirroring

The system SHALL mirror files only from one configured ez Share HTTP origin, SHALL treat listings and
file bytes as untrusted input, and SHALL enforce finite timeout, response-size, traversal-depth, and
file-count limits. It SHALL reject path escape and cross-origin redirects.

#### Scenario: Hostile listing cannot escape or exhaust the mirror

- **WHEN** the card returns traversal names, an oversized body, excessive recursion, or a redirect to a
  different origin
- **THEN** the run exits nonzero, records the rejected value and applicable bound, writes no file outside
  the mirror, and does not contact the redirected origin

### Requirement: Immutable resumable raw evidence

The system SHALL stream downloads into durable temporary files, hash them, atomically publish immutable
content, and atomically maintain a versioned manifest. Reprocessing unchanged evidence SHALL not
redownload or alter it.

#### Scenario: Interrupted and replayed downloads preserve evidence

- **WHEN** a response truncates or manifest replacement is interrupted and the same run is retried
- **THEN** no partial file is treated as complete, the prior manifest remains usable, and a subsequent
  unchanged replay performs zero file downloads

### Requirement: Clean-room validated EDF decoding

The system SHALL decode supported ResMed EDF channels without copying OSCAR GPL implementation code,
map values through generated registry source/type definitions, convert units once, and reject absent,
invalid, sentinel, or out-of-range values rather than coercing or clamping them.

#### Scenario: Malformed or absent physiology contributes no sample

- **WHEN** an EDF is truncated, a channel is unsupported, or a value is sentinel/out of registry range
- **THEN** the parser records a named rejection and emits no numeric sample for that input

### Requirement: Stable append-only identity and replay

The system SHALL derive deterministic CPAP session and sequence identity from immutable device/file
session evidence, never arrival time, and SHALL append using `(source,type,session_id,seq)` without
updating or deleting existing samples.

#### Scenario: Importing identical files twice is a no-op replay

- **WHEN** one mirrored EDF set is imported successfully and then imported again byte-for-byte
- **THEN** the second attempt succeeds with a replay effect and creates zero additional sample rows

### Requirement: Versioned device-clock interpretation

The system SHALL preserve raw device time and store clock correction as versioned metadata with its
derivation evidence. Corrections SHALL be applied at read time and SHALL never rewrite samples.

#### Scenario: Correcting a clock transform preserves raw history

- **WHEN** an operator supersedes a CPAP clock transform
- **THEN** a new metadata version is appended, affected-night preview is available, and stored sample
  identity and raw device time remain unchanged

### Requirement: Observable supervised imports

The system SHALL create an attempt before each poll/import and distinguish success, failure, timeout,
unfinished work, no-op replay, and rejected rows. The operator console SHALL show last attempt, last
success, row effect, failure reason, uniquely owned data, and clock-transform evidence.

#### Scenario: Card disappears during a scheduled import

- **WHEN** the card becomes unreachable after an attempt begins
- **THEN** the attempt finishes failed or remains detectably overdue, the console names card reachability
  as the blocker, and no old sample is presented as evidence that the attempt succeeded

### Requirement: Private least-privilege card networking

The system SHALL route card HTTP only over the dedicated card interface and subnet, SHALL NOT expose or
forward the card interface to the home LAN, and SHALL run the poller with bounded resources and runtime.

#### Scenario: Arbitrary target URL cannot use the poller as a proxy

- **WHEN** configuration or card content supplies a URL outside the configured card origin
- **THEN** the poller rejects it before connection and records the blocked origin
