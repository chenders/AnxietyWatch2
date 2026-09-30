# ResMed AirSense 11 BLE therapy stream

This document is the operational and product-facing guide for the read-only
AirSense 11 collector. It describes the data path, safety boundary, service,
acceptance evidence, and the admin-page/kiosk contract. It does not authorize
service activation or live acceptance. First-time target discovery and
application pairing are handled by the separate, bounded
`resmed-air11-onboard` operation. For the operator's complete setup and usage
sequence, see [`resmed-air11-user-guide.md`](resmed-air11-user-guide.md).

## Scope and non-goals

The integration observes the short live therapy stream exposed by an AirSense
11 over its authenticated BLE application pipe. It uses credentials provisioned
by an operator and sends only the fixed observation request. It does not:

- pair, collect a passkey, create or rotate credentials, or change the device;
- send therapy, settings, reset, erase, time-setting, OTA, CAN, or arbitrary
  RPCs;
- read historical therapy data from BLE; or
- replace the independent EzShare SD/EDF archive path.

A BLE connection is not a physiological measurement. Authentication, stream
subscription, receipt of a frame, and receipt of a usable numeric value are
separate facts and are reported separately. Onboarding only establishes the
Linux target and application credential; it does not imply collection or
measurement.

## Data path

AirSense data follows the existing durable ingestion path:

```text
AirSense 11
    │ authenticated FD56 BLE notifications
    ▼
resmed-air11-collector
    │ bounded journal before semantic decode
    │ Request::Append { sync: false }
    ▼
vitalsd Unix socket
    │ PostgreSQL binary COPY spool
    ▼
vitals-spool-drain
    ▼
PostgreSQL samples
```

The collector never opens PostgreSQL and never creates a second history store.
The raw journal is a recovery input, not an admin history API. A journal write,
append acknowledgement, or status publication failure stops or degrades
numeric publication rather than claiming that data is durable.

EzShare remains independent:

```text
AirSense SD card → EzShare HTTP/EDF importer → its existing ingest path
```

BLE failure does not disable or change EzShare, and EzShare route changes are
not part of AirSense activation.

## BLE and protocol boundary

The Linux transport connects only to the configured target and resolves the
documented service and characteristics. If BlueZ has not materialized the
configured target, it first tries a direct connect and then performs one
bounded LE discovery filtered to the FD56 service and target address. It never
pairs or scans indefinitely. It authenticates through the AirSense application
protocol before requesting the fixed stream; a separate BlueZ bond is not
required:

| Item | Value |
|---|---|
| Service | `0000fd56-0000-1000-8000-00805f9b34fb` |
| TX characteristic | `a6220002-35f1-4b20-afae-cb089d2044aa` |
| RX characteristic | `a6220003-35f1-4b20-afae-cb089d2044aa` |

`StartStream` requests a 40-ms sample interval and a 200-ms report interval.
The tested firmware reports `StreamData.intervalMs=40`; this is the observed
spacing of values in the frame, not a required echo of the requested report
interval. Source positions therefore use the reported frame interval, while
native signal cadence remains the registry-backed metadata.

FIG fragments are reassembled with fixed bounds and CRC checks before the
payload is considered an RPC envelope. The decoder rejects truncation,
oversize payload claims, invalid CRCs, malformed JSON, invalid UTF-8, and bad
padding without allocating according to an attacker-controlled length.

The production RPC allowlist is enforced at the final send boundary. Only
these methods can be sent:

1. `RequestSession`;
2. `CheckSessionIntegrity`;
3. `GetVersion`;
4. `GetDateTime`;
5. `SubscribeEvent`; and
6. `StartStream` with the fixed stream ids and bounded intervals.

`Set`, therapy transitions, reset, erase, pairing, OTA, time-setting,
arbitrary `Get`, and unknown methods are rejected before a GATT write. There is
no assumed `StopStream`; disconnecting the BLE session ends observation.

## Credentials and identity

Credentials are externally provisioned at:

```text
~/.air11/air11-credentials.json
```

For the systemd unit, `~` means the home directory of the `vitals` service
account. The file must be regular, owner-readable, and inaccessible to group
and other users. It contains a client id, an explicit Linux target or a target
plus an explicit Linux mapping, and a 32-byte master pair key. No credential is stored
in Git, an environment file, an ordinary command-line argument, a status
record, a journal record, a sample extra field, or a log line.

The AirSense unit uses `LoadCredential` to copy the private source into a
runtime-only credential directory. The collector receives only that runtime
path; `ProtectHome` then hides the source home directory from the service. The
console unit uses a separate optional `LoadCredential` view of the same source
for readiness inspection; if the source is absent, the console remains
available for status and logs while mutations stay disabled. Neither process
returns the credential contents. Invalid permissions, malformed JSON, an invalid key length, an opaque
CoreBluetooth UUID without a Linux mapping, or an all-zero target fails closed.

Each successful authenticated connection receives a fresh persistent
`session_id`. AirSense sample `seq` values come from device source position,
not host arrival order and not `Session::next_seq()`. A repeated source
position in one connection is therefore deduplicable; a new authenticated
connection is not falsely deduplicated against the previous connection.

The device clock is provenance only. Stored sample timestamps are host receipt
time in UTC epoch units. Metadata retains the raw device `startTime` and
`clock_quality="untrusted"`. A malformed, backward, or unusable source
position is journaled/quarantined and never replaced with a host-derived
sequence.

## Registry-backed stream mapping

All ids, units, ranges, and conversions come from `vitals/registry.toml` and
its generated artifacts. The supported stream map is:

| AirSense id | Registry type | Stored unit | Conversion |
|---|---|---|---|
| `_RFL` | `patient_flow` | L/s | direct |
| `_MKP` | `mask_pressure` | cmH2O | direct |
| `_MKF` | `mask_pressure_2s` | cmH2O | direct |
| `_MKI` | `inspiratory_pressure` | cmH2O | direct |
| `_MKE` | `expiratory_pressure` | cmH2O | direct |
| `_LKF` | `leak` | L/min | raw L/s × 60 |
| `_RR2` | `respiratory_rate` | bpm | direct |
| `_TD2` | `tidal_volume` | mL | raw L × 1000 |
| `_MV2` | `minute_ventilation` | L/min | direct |
| `_TGT` | `target_minute_ventilation` | L/min | direct |
| `_IE2` | `ie_ratio` | ratio | direct |
| `_SNI` | `snore_index` | index | direct |
| `_FFL` | `flow_limitation` | ratio | direct |
| `_INT` | `inspiratory_duration` | seconds | direct |
| `_BYV` | `trigger_cycle_event` | event code | direct |
| `_HRT` | existing `hr` | bpm | direct |
| `_SAO` | existing `spo2` | percent | direct |

Invalid values are rejected, never clamped. JSON `null`, omitted data, and an
invalid `StartStream` group produce no numeric row and increment unavailable
evidence. A measured numeric `0.0` remains a measured zero.

The collector accepts the observed frame interval independently of the fixed
request's report interval; a mismatch with 200 ms is not, by itself, a bad
measurement.

### Event subscription and summary paths

The collector now sends a fixed, read-only `SubscribeEvent` request alongside
`StartStream`. It requests the four AirCANNect-compatible base selectors plus
`TherapyEvents-RespiratoryEvents`. The response must return a non-zero
subscription id and `valid: true` for every fixed selector; partial coverage is
reported as unavailable with the selector named. Event notifications are
journaled on the same encrypted application lane and decoded into bounded
lifecycle and respiratory event records. Event counters and the last data id
are exposed in the operator-safe status document; event records are not yet persisted as
therapy-session rows or used to calculate AHI.

AirCANnect's independently verified source recognizes hypopnea, central apnea,
obstructive apnea, unclassified apnea, RERA, and CSR events. Its separate
Summary/report path contains a numeric AHI field. Therefore AHI is **not a
`StartStream` signal** and is not derived from live respiration, leak, pressure,
or flow-limitation samples. The next stage must persist event records, prove
therapy-session boundaries and event coverage, and then compare a derived
provisional AHI with the device Summary report.

### Post-stop lifecycle policy

The protocol does not document a ResMed-specific hose-clearing duration. The
reference AirCANnect report path documents a 30-second post-therapy summary
delay and a 180-second maximum report-settling wait; these are operational
settling bounds, not clinical claims about hose clearing. The Rust collector
therefore records `TherapyStop`, `MaskOff`, or `StandbyStarted` as a
**provisional** boundary, permits finalization after 30 seconds only when no
post-stop stream envelope arrived, and waits until 180 seconds when stream
activity continued after the stop. A new start during settling resumes the
current session rather than creating a duplicate. This in-memory projection is
not yet the durable therapy-session record required by Amendment 12.

Native cadence is retained in metadata. The collector does not turn repeated
high-rate copies of a lower-rate signal into false clinical resolution.

## Journal and durability

A bounded decrypted `StreamData` envelope is journaled before semantic decode.
Authentication messages, master keys, session keys, target identifiers, and
raw device captures are not journaled. Journal segments rotate by finite size,
record count, and age; writes are private and fsynced according to the fixed
policy in the implementation.

Rows are appended to `vitalsd` with `sync=false`, which is the intended policy
for this high-rate source. The collector validates the returned acknowledgement
count. If the socket, response, or append deadline fails, it retains the
journal, sets durability-unknown state, and does not retry through a replacement
identity. The existing daemon and spool drainer remain responsible for durable
COPY ingestion.

## Systemd service

The unit is `deploy/vitals-air11.service`. It is intentionally installed but
not enabled by this change. Activation is a separate operator and physical
acceptance gate.

The service:

- runs as the unprivileged `vitals` account with a restrictive umask;
- starts after and requires `bluetooth.service` and `vitalsd.service`;
- exposes only the BlueZ/D-Bus and Unix-socket address families needed by the
  collector;
- grants writable state and runtime directories only through
  `StateDirectory` and `RuntimeDirectory`;
- loads the credential through a private systemd credential path rather than an
  environment file or secret argument;
- uses `ProtectSystem=strict`, `ProtectHome`, `NoNewPrivileges`, private
  devices and temporary storage, a blank capability set, and native syscall
  architecture confinement;
- applies `OOMScoreAdjust=-1000`, `MemoryMin`, and `MemoryHigh`; and
- uses explicit restart, timeout, and start-rate limits.

The service does not pair, scan indefinitely, modify the Bluetooth adapter,
enable itself, write PostgreSQL, or run an AirSense control RPC. Its bounded
fallback discovery only resolves the already configured target.

## Status contract and admin page

The collector atomically publishes its operator-safe status at:

```text
/run/vitals-air11/status.json
```

The status file is a backend input. A browser is not granted direct access to
the runtime directory or credential directory. The Rust admin API reads this
safe document and exposes only its facts and blockers. The same API exposes a
small fixed set of operator actions—opt-in/out and start/stop/restart of the
collector—behind the private-LAN allowlist, an external console-admin
capability, a per-process action token, explicit confirmation, and append-only
audit. It has no browser filesystem route and no AirSense device-control route.

### Link and measurement are separate

The admin page must render `link_state` and `measurement_state` as two distinct
indicators. It must not label the source `LIVE`, healthy, or measured merely
because BLE is connected or authenticated.

| Link state | Operator meaning |
|---|---|
| `starting` | The collector is preparing a bounded connection attempt. |
| `connected` | BlueZ and the exact GATT pipe are connected. |
| `authenticated` | Session integrity succeeded; a usable value may still be absent. |
| `streaming` | The fixed observation subscription was accepted. |
| `disconnected` | The link ended and bounded reconnect is pending. |
| `degraded` | A dependency or protocol failure is blocking truthful publication. |

| Measurement state | Admin presentation |
|---|---|
| `never_measured` | No usable numeric value has ever arrived. |
| `measured` | At least one validated numeric row was acknowledged by `vitalsd`. |
| `unavailable` | Frames arrived but values were null or unavailable. |
| `quarantined` | Input or source position was rejected and retained for diagnosis. |
| `stale` | A previously measured source has gone silent or disconnected. |

Recommended combined labels include:

- `Streaming — measured`;
- `Streaming — connected, no usable measurement`;
- `Authenticated — waiting for usable physiology`;
- `Disconnected — reconnect pending`;
- `Degraded — durability unknown; journal retained`; and
- `Stale — last measurement is not current`.

The page must preserve the following semantics:

- `null` and unavailable counts are displayed as unavailable evidence, never as
  numeric zero;
- `durability_unknown=true` is a warning and is never shown as durable or
  complete;
- a non-empty blocker is shown with its state/value context rather than hidden
  behind a generic red badge;
- `last_host_receipt_unix_micros` is the freshness field for stored observations;
- `last_device_start_time` is labeled as raw, untrusted device provenance and
  is not used as the displayed event time;
- `session_id`, counters, and source evidence may be displayed as diagnostics,
  but no full target address, serial, client id, master key, session key, or raw
  payload may be rendered; and
- no admin control is added for pairing, therapy state, settings, reset, erase,
  OTA, or an assumed stream-stop RPC.

The admin page should show counters for received envelopes, decoded frames,
published samples, unavailable values, duplicates, quarantined values, dropped
envelopes, retries, and reconnects. This makes a quiet therapy stream
observable without converting quietness into an alarm or a false healthy state.

### Admin and kiosk surfaces

The admin screen is available through the Rust console at
`/console/?screen=air11&dir=a|b|c`; all three directions consume the same safe
view-model. The dedicated read-only monitor is
`/console/air11-kiosk.html`. It shows the current status, service state,
counters, blocker precedence, and bounded sanitized journal entries, but no
lifecycle buttons or credential inputs. The physiology dashboard links to this
monitor without merging AirSense machine state into physiological measurements.

The admin page never accepts or returns the AirSense master pair key. It checks
external-file presence, owner-only permissions, and credential shape. The
separate console-admin capability is used only for console lifecycle actions;
it is not an AirSense credential. Missing admin authorization leaves the status,
last blocker, and logs readable while leaving all mutations disabled.

### Admin-page failure precedence

When multiple facts are present, use this order for the primary operator
message:

1. `durability_unknown` or a journal/append blocker;
2. `degraded`, `disconnected`, or stale link state;
3. `quarantined` or `unavailable` measurement evidence;
4. `never_measured`; then
5. measured evidence with its current receipt time.

This ordering prevents old rows or a connected-but-null stream from reassuring
the operator. It also keeps infrastructure failure distinct from physiological
badness.

## Offline evidence

Host-only evidence is provided by the workspace tests and the CI gates:

- FIG fragmentation, coalescing, CRC, truncation, and oversize rejection;
- StreamData values, nulls, measured zero, aliases, unknown ids, and malformed
  JSON;
- registry-generated ids and units;
- credential/address validation and session crypto vectors;
- source-position retransmission identity and reconnect session identity;
- journal round-trip, rotation, torn-tail recovery, and replay identity;
- `vitalsd` append framing and acknowledgement validation;
- status atomicity and private-field exclusion; and
- the final read-only RPC allowlist, including zero writes for forbidden methods.

The checked-in fixtures are sanitized and bounded. CI rejects raw captures,
private identifiers, credential artifacts, copied embedded runtime code, stale
registry output, missing decoder fuzz targets, and allowlist regressions.

## Bounded Pi acceptance

The acceptance harness is:

```text
scripts/air11-pi-acceptance.sh
```

It must be run on the Pi from an interactive terminal. The required invocation
uses the installed H10 implementation as an independent live-value gate:

```sh
scripts/air11-pi-acceptance.sh --duration 60 --require-postgres --require-h10
```

The harness asks the operator to type a duration-specific confirmation (for
example, `AIR11-LIVE-60`) before starting or restarting the AirSense unit. It
never pairs, scans, enables the unit, changes an adapter, edits firmware,
changes a route, or sends a control RPC. If it started the collector, it stops
it again during cleanup; an already running collector is not silently disabled.

A successful run requires all of the following:

1. the `vitals` credential file exists, is private, and contains a valid
   explicit Linux target and key shape without printing the key;
2. the configured target was resolved by bounded FD56 discovery; no pairing
   command is attempted by the collector;
3. the PostgreSQL connection and `public.samples` table are available;
4. the collector reaches authenticated streaming and measured states;
5. at least one unavailable/null observation is preserved in status;
6. host receipt time and raw device start-time provenance are present;
7. two distinct persistent sessions are observed across the bounded restart;
8. source/type rows with untrusted-clock metadata reach PostgreSQL through the
   existing ingest path; and
9. H10 remains active and produces a fresh registry-valid HR or RR interval
   row. EMAY and EzShare are separate optional integrations and are not
   required by this AirSense acceptance.

The harness removes its temporary evidence on exit and prints only aggregate
counts and named blockers. A nonzero exit is the acceptance result; a status
report copied into chat is not a substitute for the exit code.

## Evidence interpretation

A passing offline suite proves portable logic and confinement only. A passing
Pi run proves one bounded live window and its explicitly requested unaffected
source checks. Neither proves overnight reliability, universal device firmware
compatibility, trustworthy device wall time, or support for oximetry when
`_HRT`/`_SAO` remain null.

If credentials, the configured target, PostgreSQL, a source row, host-time
metadata, reconnect sessions, or unaffected-source health is missing, the result is
failed or unknown—not successful by inference. Keep EzShare enabled as the
separate durable/archive fallback while BLE evidence is still limited.
