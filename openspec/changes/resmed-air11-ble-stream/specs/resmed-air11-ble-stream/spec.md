## ADDED Requirements

### Requirement: The collector SHALL use the authenticated AirSense BLE pipe

The collector SHALL connect only to the configured AirSense 11 target, discover
the documented FD56 service and TX/RX characteristics, and reassemble bounded
FIG packets before interpreting RPC data.

#### Scenario: A notification splits and combines FIG packets

- **GIVEN** a valid FIG packet is split across several notifications and a second
  valid packet follows in the same notification
- **WHEN** the collector feeds those fragments to the reassembler
- **THEN** it emits both packets in source order
- **AND** it validates the header and payload CRC32 before emission

#### Scenario: Hostile FIG input exceeds a protocol bound

- **GIVEN** a fragment advertises a payload larger than 7682 bytes, has a bad CRC,
  or ends before the advertised payload
- **WHEN** the decoder receives it
- **THEN** it emits no physiological value and returns a named malformed-input
  result without panicking or allocating based on the advertised size

### Requirement: Credentials and session cryptography SHALL fail closed

The production collector SHALL load only a private external credential file,
prove knowledge of its master pair key with the AirSense challenge, and derive a
fresh AES session key from each successful session nonce.

#### Scenario: A valid saved pairing reconnects

- **GIVEN** a mode-0600 credentials file contains a valid client id, normalized
  target, and 32-byte master pair key
- **WHEN** the device returns a fresh challenge and nonce
- **THEN** the collector sends the exact HMAC-SHA256 integrity response
- **AND** it derives the session key only after the integrity response succeeds
- **AND** it never writes the master key or session key to logs, status, journal,
  or samples metadata

#### Scenario: Credentials or cryptographic evidence is invalid

- **GIVEN** the credential file is absent, too permissive, malformed, or the
  challenge/proof, ciphertext length, padding, or nonce is invalid
- **WHEN** the collector starts or receives the invalid protocol item
- **THEN** it refuses numeric ingest and reports the blocker with its value and
  limit/reason
- **AND** it does not fall back to unauthenticated or plaintext application RPC

### Requirement: RPC transmission SHALL be read-only and allowlisted

The final transport send boundary SHALL reject every method except the fixed
  authentication and observation set: `RequestSession`, `CheckSessionIntegrity`,
`GetVersion`, `GetDateTime`, `SubscribeEvent`, and `StartStream`. It SHALL never
expose a generic method caller to production code.

#### Scenario: A control method is attempted

- **GIVEN** code attempts to send `Set`, `EnterTherapy`, `EnterStandby`,
  `ResetDevice`, `EraseData`, an OTA method, or an unknown method
- **WHEN** the request reaches the send boundary
- **THEN** the boundary rejects it before any GATT write
- **AND** the rejection names `method=<value>` and the active allowlist

#### Scenario: A stream request is issued after authentication

- **GIVEN** the application lane is authenticated
- **WHEN** the collector starts observation
- **THEN** it sends only the fixed requested AirSense stream ids with bounded
  sample/report intervals
- **AND** it terminates observation by disconnecting rather than sending an
  assumed `StopStream` RPC

### Requirement: Stream values SHALL use registry semantics and preserve unknowns

The collector SHALL map each supported AirSense stream id to a generated
`VitalsType`, convert to that type's canonical unit exactly once, and reject
values outside the generated range without clamping. JSON `null`, omitted data,
and unavailable stream ids SHALL contribute no numeric row.

#### Scenario: A valid value is converted once

- **GIVEN** a valid leak value in raw L/s and a valid tidal-volume value in raw L
- **WHEN** the stream builder creates samples
- **THEN** leak is stored as L/min using one ×60 conversion and tidal volume as
  mL using one ×1000 conversion
- **AND** the sample types and units come from the registry-generated enums

#### Scenario: Null oximetry and unavailable data arrive

- **GIVEN** `_HRT` or `_SAO` is JSON `null`, or `StartStream` marks an id invalid
- **WHEN** the frame is processed
- **THEN** no row with value zero is emitted
- **AND** status counts the signal as unavailable rather than measured

#### Scenario: A measured zero is present

- **GIVEN** leak, snore, or flow limitation contains numeric `0.0`
- **WHEN** the frame is processed
- **THEN** the numeric zero is retained as a measured value and is not confused
  with the null/unavailable state

### Requirement: Durable identity SHALL come from AirSense source position

Every authenticated BLE connection SHALL receive a persistent unique
`session_id`, while each row's `seq` SHALL derive from the device stream start
position and sample index. The collector SHALL never use host arrival order or
`Session::next_seq()` as AirSense sample identity.

#### Scenario: A source frame is replayed in one connection

- **GIVEN** the same `startTime`, stream id, and sample position is delivered
  twice in one authenticated connection
- **WHEN** both frames are processed
- **THEN** both observations resolve to the same `(source, type, session_id, seq)`
- **AND** the second observation creates no duplicate row

#### Scenario: The BLE link reconnects

- **GIVEN** the device disconnects and later authenticates again
- **WHEN** the collector resumes the fixed stream
- **THEN** it mints a new session id before publishing new rows
- **AND** it does not falsely deduplicate the new connection against the old one

#### Scenario: Device time cannot establish source position

- **GIVEN** `startTime` is malformed, moves backward, or is otherwise not
  monotonic within the stream
- **WHEN** the frame is received
- **THEN** the frame is journaled/quarantined with a named clock blocker
- **AND** no host-derived sequence is substituted

### Requirement: Stored timestamps SHALL expose host receipt time and clock quality

The collector SHALL store UTC host receipt time as the sample timestamp and
retain the raw device start time and an explicit untrusted-clock marker in
source metadata; it SHALL not synchronize or silently correct the AirSense
clock.

#### Scenario: The device reports a stale clock

- **GIVEN** AirSense reports a start time from an invalid/stale device clock
- **WHEN** a valid numeric sample is appended
- **THEN** its timestamp is the host receipt instant
- **AND** its metadata retains the raw device time and
  `clock_quality="untrusted"`

### Requirement: Numeric publication SHALL use the existing vitalsd path

The collector SHALL journal bounded StreamData before semantic decoding and
publish only validated `WireSample` rows through `Request::Append` to `vitalsd`;
it SHALL not open PostgreSQL or create a parallel history store.

#### Scenario: A batch is accepted by vitalsd

- **GIVEN** a batch contains only validated numeric rows
- **WHEN** `vitalsd` returns `Response::Appended`
- **THEN** the collector reports the returned append state and leaves COPY
  draining to the existing daemon/drainer
- **AND** streaming appends use the non-sync path appropriate to this source

#### Scenario: The socket or journal is unavailable

- **GIVEN** the journal write, socket connection, append response, or append
  deadline fails
- **WHEN** the collector handles the failure
- **THEN** it stops or bounds numeric publication as appropriate, names the
  failed dependency and value/limit, and reports durability as unknown
- **AND** it does not mint a replacement identity merely to retry the failure

### Requirement: Reconnect and liveness SHALL be observable

The collector SHALL use bounded connection, discovery, RPC, journal, and append
deadlines; after a disconnect it SHALL re-authenticate and reissue the stream
with bounded backoff. Its status SHALL distinguish never measured, unavailable,
quarantined, stale, authenticated, and streaming states.

#### Scenario: A connection drops during therapy

- **GIVEN** an authenticated stream loses its BLE connection
- **WHEN** the reconnect loop runs
- **THEN** it closes the old session, retries within the configured backoff
  bounds, authenticates with a fresh nonce, and reissues `StartStream`
- **AND** status names the disconnect and current retry value

#### Scenario: No valid physiology has arrived

- **GIVEN** the link is connected but all returned values are null or invalid
- **WHEN** status is published
- **THEN** it does not report the AirSense source as measured-and-good
- **AND** it names the unavailable/quarantined signal evidence

### Requirement: Runtime credentials and service privileges SHALL be confined

The collector SHALL keep credentials outside Git and outside ordinary logs,
read only the BlueZ/vitalsd/state interfaces it needs, and run under a bounded
least-privilege systemd unit. Enabling the unit SHALL remain an explicit
operator and physical acceptance gate.

#### Scenario: Address input comes from macOS tooling

- **GIVEN** a six-octet macOS address uses hyphens or no separators
- **WHEN** configuration is loaded
- **THEN** it is normalized to an uppercase colon-separated 17-character BlueZ
  MAC before any connection attempt

#### Scenario: An opaque CoreBluetooth UUID is supplied

- **GIVEN** a 128-bit CoreBluetooth UUID has no explicit Linux MAC mapping
- **WHEN** configuration is loaded
- **THEN** configuration fails closed with a blocker naming the supplied form
- **AND** the collector does not truncate, hash, scan, or connect to a guessed
  address

#### Scenario: The service is installed but not authorized

- **GIVEN** the unit file exists but physical deployment/activation has not been
  approved
- **WHEN** implementation verification runs
- **THEN** it validates the unit statically without pairing, enabling, rebooting,
  changing adapters, changing overlays, or touching the EzShare route

### Requirement: Existing EzShare ingestion SHALL remain independent

The AirSense BLE change SHALL not modify the EzShare mirror/parser/importer
contract, its files, or its private network route; the two paths SHALL be
reported as separate sources and fallbacks.

#### Scenario: BLE is unavailable while the SD path works

- **GIVEN** the AirSense BLE credentials or target are unavailable
- **WHEN** the EzShare importer runs
- **THEN** its behavior and source identity remain unchanged
- **AND** the operator can continue using the documented SD/EDF fallback

### Requirement: Evidence SHALL be replayable without private captures

Every new byte decoder SHALL have malformed/truncated tests and a cargo-fuzz
target. Checked-in fixtures SHALL be sanitized and sufficient to prove CRC,
cryptographic envelope, stream mapping, null preservation, retransmission
identity, and reconnect state without credentials, raw identifiers, or large
captures.

#### Scenario: A replay fixture contains a null and a measured zero

- **GIVEN** the fixture has JSON `null` for oximetry and numeric zero for leak
- **WHEN** the offline collector pipeline replays it
- **THEN** only the measured zero can produce a row
- **AND** the test proves the two states remain distinguishable

#### Scenario: A decoder receives arbitrary bytes

- **GIVEN** the fuzz target supplies any byte slice
- **WHEN** the target calls the decoder
- **THEN** it terminates without panic, out-of-bounds access, allocation based
  on wire length, or a fabricated physiological value
