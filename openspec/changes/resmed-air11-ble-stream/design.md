# Design — read-only ResMed AirSense 11 BLE therapy streaming

## Boundary and evidence

This is a new Rust capability, not a port of AirCANnect's ESP32 runtime. The
AirCANnect and airbreak-plus trees are protocol references only. The implementation
must remain inside the existing crate boundaries: hostile wire decoders in
`vitals-decode`, host-independent sample builders in `vitals-collectors`, and
BlueZ/tokio/socket code in the Linux collector binary. No BlueZ, SQL, or systemd
dependency may enter `vitals-decode` or `vitals-registry`.

The evidence consists of authenticated GATT sessions, the documented FD56
service and characteristics, and sanitized JSONL capture summaries. It proves
short live delivery, not all-night reliability or trustworthy device wall time.
The collector therefore records what it measured and what it could not measure;
it never turns a successful GATT connection into a healthy physiology claim.

## Data and registry contract

`vitals/registry.toml` remains the only source of ids, units, ranges, and
precision. Proposed permanent additions are assigned after the existing ids and
are generated into Rust, SQL reference data, and JSON; `generated.rs` is never
hand-edited. The source is `resmed_air11`, transport `ble`, with a one-second
liveness expectation while the collector is configured. A stream can be quiet
because therapy is not active, so status must distinguish connected,
authenticated, streaming, and no usable measurement.

The fixed stream mapping is:

| Air11 id | Registry type | Canonical unit and conversion | Native cadence |
|---|---|---|---:|
| `_RFL` | `patient_flow` | L/s, direct | 40 ms |
| `_MKP` | `mask_pressure` | cmH2O, direct | 40 ms |
| `_MKF` | `mask_pressure_2s` | cmH2O, direct | 2000 ms |
| `_MKI` | `inspiratory_pressure` | cmH2O, direct | 2000 ms |
| `_MKE` | `expiratory_pressure` | cmH2O, direct | 2000 ms |
| `_LKF` | `leak` | L/min, raw L/s × 60 | 2000 ms |
| `_RR2` | `respiratory_rate` | bpm, direct | 2000 ms |
| `_TD2` | `tidal_volume` | mL, raw L × 1000 | 2000 ms |
| `_MV2` | `minute_ventilation` | L/min, direct | 2000 ms |
| `_TGT` | `target_minute_ventilation` | L/min, direct | 2000 ms |
| `_IE2` | `ie_ratio` | ratio, direct | 2000 ms |
| `_SNI` | `snore_index` | index, direct | 2000 ms |
| `_FFL` | `flow_limitation` | ratio, direct | 2000 ms |
| `_INT` | `inspiratory_duration` | seconds, direct | 2000 ms |
| `_BYV` | `trigger_cycle_event` | event code, direct | 40 ms |
| `_HRT` | existing `hr` | bpm, direct | 1000 ms |
| `_SAO` | existing `spo2` | percent, direct | 1000 ms |

The collector must not store the repeated 40-ms copies of a native 1-Hz or
0.5-Hz signal as if they were new clinical resolution. It emits a source-time
position once per signal's native cadence. The cadence table is protocol
metadata; every numeric range and unit check still comes from the generated
registry enum. A signal-specific range violation is quarantined with the signal
name and value, never clamped.

`null`, an omitted data group, and a `StartStream` validity result of `false` are
not measurements. They produce no numeric row and increment an unavailable
counter. A non-null `_HRT` or `_SAO` is accepted only after the normal registry
range check; the tested captures had both entirely null, so the implementation
must not claim oximetry support merely because the ids were accepted.

## Protocol layers

### GATT and FIG

BlueZ discovers service
`0000fd56-0000-1000-8000-00805f9b34fb`, writes to TX
`a6220002-35f1-4b20-afae-cb089d2044aa`, and subscribes to notifications on RX
`a6220003-35f1-4b20-afae-cb089d2044aa`. The portable FIG decoder accepts arbitrary
fragment boundaries, searches for `BE BA FE CA`, enforces the 7682-byte payload
ceiling, validates IEEE CRC32 over both the eight-byte header body and payload,
and yields every complete packet in order. It rejects truncated, oversized,
corrupt, and trailing data without indexing or allocation. The Linux reassembler
keeps only a bounded incomplete packet and reports a named protocol error.

### Authentication and encryption

Production starts only with an externally supplied `clientId` and 32-byte
`masterPairKey`. It sends plaintext `RequestSession`, checks the response
challenge with HMAC-SHA256, sends plaintext `CheckSessionIntegrity`, and derives
`SHA256(masterPairKey || nonce_raw)` only from the fresh nonce after the device
accepts the proof. AES-256-CBC uses a random 16-byte IV and the observed
`[uint16 little-endian JSON length][UTF-8 JSON][zero padding]` format. Decryption
rejects invalid block lengths, lengths beyond the plaintext, non-zero padding,
empty JSON, and invalid UTF-8. No session key or master key is written to a
journal, status file, log, command line, or database extra field.

The SRP-6a-like 2048-bit calculation and proof verification are retained as a
portable, fixed-buffer protocol test implementation so the observed pairing
vectors remain reproducible. The production collector does not initiate
`StartKeyExchange`/`ConfirmKeyExchange` and does not save credentials. A future
pairing tool would require a separate approved change.

### JSON-RPC and safety allowlist

The request builder has no generic method escape hatch. The only production
methods are `RequestSession`, `CheckSessionIntegrity`, `GetVersion`,
`GetDateTime`, and `StartStream`; the first two are fixed authentication
messages, the next two are read-only diagnostics, and `StartStream` is emitted
with the fixed id list and bounded 40/200-ms intervals. Responses are matched by
request id and VCID; notifications such as `StreamData` cannot satisfy a
pending request. Any method not in the allowlist is rejected before a GATT
write, with an error naming `method=<value>` and `allowlist=...`. Tests place a
recording transport around the final `send()` boundary and assert that control
and OTA names never reach it.

There is no `StopStream` assumption. On shutdown, disconnecting the BLE session
terminates the observation stream; the collector must not invent a stop RPC.

## Identity, time, and metadata

Each successful authenticated BLE connection mints a fresh persistent
`session_id` using `SessionMinter`. `SessionMinter` is for connection identity
only; Air11 sample `seq` is never `Session::next_seq()` and never host arrival
order. The source position is derived from the device-provided `startTime`, the
reported sample index, the fixed requested interval, and the signal's native
cadence. A duplicate source position in the same connection produces the same
`seq` and is ignored; a new authenticated connection gets a new session even if
the device repeats old values.

The device time is parsed only enough to establish a relative, monotonic source
position. A malformed, backward, or non-monotonic `startTime` quarantines the
frame; the implementation must not fall back to a host counter. The stored
`timestamp` is the UTC host receipt time in Unix microseconds. The row's versioned
`extra` metadata carries the raw device `startTime`, raw stream id, short data id,
native interval, source ordinal, and `clock_quality="untrusted"`. The observed
device clock (`2008-01-01T00:00:00.000Z`) is evidence for this policy, not a
value to be corrected or synchronized by this change.

## Collector and durability path

Before semantic decoding, the collector appends each bounded decrypted
`StreamData` envelope to a mode-0600 raw journal under its state directory. The
journal is length-delimited, atomically rotated at a finite size, and fsynced at
a fixed interval; it contains no authentication messages or credentials. A
journal write/rotation failure stops numeric publication and exposes the
blocker rather than claiming replayability. Sanitized fixtures, never raw
captures or identifiers, are committed to Git.

Valid rows are sent in bounded batches as `vitals_proto::Request::Append` with
`sync=false`, because this is a high-rate streaming source. The collector waits
for and validates `Response::Appended`; it never writes PostgreSQL directly and
never changes the COPY schema. A socket timeout reports durability as unknown,
retains the journal, and does not silently retry with a new identity. The
existing daemon and drainer remain the only path to the spool and database.

A status document is atomically replaced and contains only state, counts,
last host receipt, last raw device timestamp, session id, reconnect count,
quarantine/unavailable counts, and a blocker when degraded. It contains no
credential, session key, device serial, or full target address. Health is not
`LIVE` merely because a connection exists or old rows exist.

## Reconnect and deployment

The Linux transport uses bounded connect, service-resolution, notification, RPC,
and append deadlines. It connects to the explicitly resolved target without
requiring a separate BlueZ bond; AirSense application authentication remains the
security gate. It does not pair or scan indefinitely. A disconnect tears down the
session, mints a new identity only after the next authentication succeeds,
re-authenticates, reissues `StartStream`, and resets exponential backoff. A
failed connect, authentication, journal, decode, append, or stale stream names
the exact state/value/limit in status and logs only non-secret identifiers.

Credential loading defaults to `~/.air11/air11-credentials.json`, requires a
regular private file and exact key lengths, and keeps the file outside the
repository. The normalized Linux target is always an uppercase, colon-separated
17-character MAC. Six-octet macOS forms (`AA-BB-CC-DD-EE-FF` or contiguous hex)
are normalized to that form. A CoreBluetooth 128-bit UUID is opaque and has no
recoverable MAC; it must be accompanied by an explicitly operator-supplied
Linux MAC and is otherwise rejected rather than truncated or hashed into a fake
address.

`vitals-air11.service` runs as the least-privilege vitals account, starts after
`vitalsd`, and is restricted to BlueZ/D-Bus plus the vitalsd Unix socket and its
state/status paths. It uses `OOMScoreAdjust=-1000`, bounded memory, private
credentials, `ProtectSystem=strict`, no privilege escalation, no network
socket, and explicit restart/backoff limits. Installing or enabling it is a
physical/governance gate. No `dtoverlay`, adapter replacement, pairing, reboot,
EzShare route change, or other destructive action is part of implementation.

## Alternatives rejected

- Continuing to state that BLE is provisioning-only contradicts authenticated
  live evidence and loses a useful low-latency path.
- Porting the ESP32/CAN client would import Arduino allocation, control RPCs,
  and a different transport boundary; it is reference material, not a Linux
  collector.
- Host arrival sequence or timestamp cannot provide retransmission-stable
  identity. If source-time identity cannot be established on a real firmware
  stream, numeric ingest stops and the discrepancy is documented rather than
  assigning `Session::next_seq()`.
- A new SQLite/file history would violate the accepted COPY-spool architecture.
  EzShare remains the separate store-and-forward/archive fallback.
