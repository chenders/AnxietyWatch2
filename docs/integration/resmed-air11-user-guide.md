# AirSense 11 integration user guide

This guide covers the complete operator path for the **read-only** ResMed
AirSense 11 BLE stream: preparation, external credential provisioning,
installation, opt-in, service start, monitoring, kiosk use, logs, and bounded
acceptance. It does not authorize pairing, passkey collection, scanning,
therapy control, adapter changes, route changes, firmware changes, or changes
under `/boot/firmware`.

## 1. What this integration does

The AirSense collector receives the complete authenticated live therapy stream
requested from the device, validates every signal observation, and forwards
every valid observation through the existing ingestion path:

```text
AirSense BLE notifications
  → resmed-air11-collector
  → vitalsd Unix socket
  → PostgreSQL COPY spool
  → vitals-spool-drain
  → PostgreSQL samples
```

It does not open PostgreSQL, create a second history store, or import the
independent EzShare SD/EDF archive. Host receipt time is authoritative. Device
time is retained only as `clock_quality="untrusted"` provenance. Missing and
`null` physiology remains unavailable; it is never converted to zero.

## 2. Before installation

Confirm all of the following before changing a service or starting a live run:

- The host is Linux with BlueZ and a working `vitalsd` socket.
- The AirSense is already bonded to the intended Bluetooth adapter, including
  its existing IRK where applicable. This integration does not pair or scan.
- The existing H10, EMAY, and EzShare paths have an operator-approved baseline.
- PostgreSQL and the existing COPY spool/drain path are available.
- The operator has an approved credential source and is authorized to run a
  bounded live test on the device.

OpenSpec approval alone is not physical authorization. If any prerequisite is
unknown, stop and record the blocker rather than enabling a unit.

## 3. Provision the AirSense credential externally

The collector expects the operator-managed file:

```text
~/.air11/air11-credentials.json
```

For the `vitals` service account, `~` is that account's home directory. The
file must be a regular owner-readable file with no group or other permissions
(mode `0600` is the usual choice). It contains the client id, an explicit
six-octet Linux target (or an opaque source address plus a separate explicit
Linux mapping), and the 32-byte master pair key.

Use the approved credential source to create the file. Do not generate,
reveal, paste, or rotate the key in this web page. Do not put it in a shell
argument, environment file, Git checkout, issue, chat, screenshot, journal, or
ordinary log. A shape-only example is:

```json
{
  "address": "<approved AirSense address or opaque source address>",
  "linux_address": "<approved six-octet Linux address when needed>",
  "client_id": "<approved client id>",
  "master_pair_key": "<64 hexadecimal characters>"
}
```

The collector validates the address, key width, JSON shape, and permissions
before authentication. The admin page displays only readiness facts: file
presence, permissions, client-id presence, target presence, and key shape.
It never displays those values.

## 4. Install the service and database change

Install the built collector and `deploy/vitals-air11.service` using the normal
release procedure. Keep the unit installed but disabled until the bounded
acceptance gate is authorized.

Deploy the Sqitch change before using opt-in or audit controls:

```sh
sqitch deploy
sqitch verify
```

Use the deployment's approved Sqitch target rather than placing a database
password in a command copied into a document. The migration creates only:

- `air11_integration_config`: one non-secret opted-in/not-opted-in row; and
- `air11_admin_audit`: append-only fixed-action history.

It does **not** store the AirSense address, client id, master pair key, session
key, raw BLE payload, or physiological samples.

Install `deploy/vitals-console.service` for the Rust console. The unit
optionally loads the same operator-managed AirSense file into a private
systemd credential directory solely for readiness inspection; `ProtectHome=true`
continues to hide the rest of the home directories. If the source file is
absent, the console still serves status and logs. To enable its fixed lifecycle
actions, provision a separate console-admin capability at:

```text
/etc/vitals/console-admin-token
```

Create it outside the page, keep it owner-readable with mode `0600`, and never
reuse the AirSense credential. If it is absent or unreadable, the console
continues serving status and logs but remains read-only. The optional polkit
rule in `deploy/vitals-console-air11.rules` grants the `vitals` console user
only `start`, `stop`, and `restart` for `vitals-air11.service`; install it
through the host's normal privileged configuration process if lifecycle
buttons are required.

## 5. Network access and URLs

The console is intended for the local private network, not the public
Internet. The supplied unit binds its listener to all interfaces so another
machine on the LAN can reach it, while the Rust API applies a private-network
allowlist. The unit's default allowlist covers RFC1918 IPv4 and IPv6
unique-local networks. Set `VITALS_CONSOLE_ALLOWED_NETWORKS` to the exact
operator network when a narrower boundary is required; for example,
`192.168.1.0/16` expresses a `/16` network and is normalized by the service.
Use the host firewall as a second boundary.

With the private address substituted locally, the useful URLs are:

| Purpose | URL |
|---|---|
| AirSense admin page, Service Mode | `http://<private-host-address>:8480/console/?screen=air11&dir=a` |
| AirSense admin page, Preflight | `http://<private-host-address>:8480/console/?screen=air11&dir=b` |
| AirSense admin page, Flowpath | `http://<private-host-address>:8480/console/?screen=air11&dir=c` |
| Dedicated monitoring kiosk | `http://<private-host-address>:8480/console/air11-kiosk.html` |
| Safe JSON projection | `http://<private-host-address>:8480/api/air11` |
| Existing physiology dashboard | `http://<private-host-address>:8480/dashboard` |

The three admin directions use one backend view-model and differ only in
presentation. The dashboard links to the dedicated monitor without merging
machine-control state into physiological tiles. The kiosk has no lifecycle
buttons or credential inputs and is suitable for a read-only fullscreen tab.

If a reverse proxy is used, keep the backend listener private and preserve the
proxy's private-network/TLS policy. Do not expose `/api/air11` or `/console`
through a public route.

## 6. First setup in the admin page

1. Open the AirSense 11 admin page from the private network.
2. Confirm the **Credential readiness** card. It must say `READY`, with
   owner-only permissions and all three shape checks present. A missing,
   malformed, insecure, or unreadable file is a blocker.
3. Confirm the database/configuration card. `NOT CONFIGURED` means opt-in has
   not been declared; `UNKNOWN` means the database or migration cannot be
   checked. Neither is healthy.
4. If lifecycle actions are enabled, enter the separate console-admin token in
   the page. It is held only in memory for that tab and is not the AirSense
   key. The page does not send it until a fixed action is submitted.
5. Choose **Opt in** and type the displayed confirmation phrase. Opt-in
   records non-secret intent only; it does not pair, scan, or start the device.
6. Recheck the status and logs. Resolve every red blocker before starting.
7. Choose **Start collector** and type the displayed confirmation phrase.
   The backend checks opt-in and credential readiness, records the request,
   and invokes only `systemctl start vitals-air11.service`.
8. Wait for service `active/running`, link `STREAMING`, and measurement
   `MEASURED`. A connected or authenticated link without a usable measurement
   is not a successful stream.

The page never offers a pairing button, passkey field, scan action, therapy
setting, reset, erase, OTA, or arbitrary RPC. Disconnecting the collector
ends observation; there is no assumed device `StopStream` command.

## 7. Monitoring the integration

The admin page and kiosk show these separate axes:

- **Link state:** starting, connected, authenticated, streaming, disconnected,
  or degraded.
- **Measurement state:** never measured, measured, unavailable, quarantined, or
  stale.
- **Process state:** the systemd active/substate and restart count.
- **Configuration:** opted in, not configured, or unavailable.
- **Freshness:** age from the last host receipt, with the host clock named as
  authoritative.

Important display rules are deliberate:

- `UNKNOWN` is a question, not a healthy result.
- `NOT CONFIGURED` is a declared absence and is not promoted as a fault.
- A connected/authenticated device does not prove measured physiology.
- `null`, unavailable, and a measured `0` are different facts.
- `durability unknown` is prominently shown and says that the raw journal was
  retained; it is not reported as durable.
- Raw device start time is labeled untrusted provenance and is not used as the
  stored sample timestamp.
- Session, frame, sample, unavailable, duplicate, quarantine, dropped,
  reconnect, and retry counters remain visible.

A stale console feed adds a page-level `UNKNOWN` overlay while retaining the
last known status and reason. It does not rewrite an old failure into a fresh
success.

## 8. Last status, errors, and logs

The **Last status and service logs** section shows the latest bounded status
facts and sanitized entries from the `vitals-air11.service` system journal.
Important error entries are red, marked as errors, and promoted into the page's
attention area. The kiosk uses the same precedence and keeps the last known
content visible if its API feed becomes stale.

The backend reads only selected journal fields and sanitizes device addresses,
long hexadecimal private values, protocol-key labels, and oversized messages.
It does not expose raw journal JSON, credentials, session keys, raw BLE frames,
or arbitrary system logs.

For host-side diagnosis, an authorized operator may inspect the same unit's
journal with the normal privileged procedure:

```sh
journalctl -u vitals-air11.service --no-pager -n 80
```

Do not copy unredacted output into tickets or documentation. If the page says
that the journal is unavailable, that is an `UNKNOWN` log-read result—not an
empty or healthy log. Check the service account's journal-read permission using
the host's standard policy, without widening the page into a general shell.

The page also shows the append-only AirSense admin audit: opt-in/out,
credential checks, and fixed service action requests/results. Audit unavailability
is shown as a blocker; it is not silently rendered as an empty history.

## 9. Troubleshooting by named blocker

| Page evidence | Meaning and next safe check |
|---|---|
| `credential state=MISSING` | Provision the external file for the `vitals` account; do not paste the key into the page. |
| `permissions=GROUP_OR_WORLD_ACCESSIBLE` | Change the file to owner-only mode and recheck; do not weaken the collector policy. |
| `credential state=INVALID` | Validate the JSON shape, explicit Linux target/mapping, client-id bound, and 32-byte key shape without printing values. |
| `config table unavailable` | Deploy/verify the AirSense Sqitch change or restore database access; do not infer opt-in. |
| `active_state=failed` | Read the highlighted status blocker and sanitized journal errors before restarting. |
| `link_state=disconnected` | Check the existing bond/adapter and service dependencies; do not pair or scan from this integration. |
| `measurement_state=unavailable` | Frames arrived without usable numeric values; preserve that unknown evidence and do not call it zero. |
| `durability unknown` | The append result was not confirmed; retain and replay the journal according to the collector procedure. |
| `logs unavailable` | The console cannot read the unit journal; use authorized host diagnostics and fix the read policy. |
| lifecycle action unauthorized | Provision the separate console-admin capability and the narrowly scoped polkit rule; do not use the AirSense key as the admin token. |
| peer outside allowlist | Reach the page from the configured private LAN or set the allowlist to the intended private CIDR; do not open it publicly. |

For any blocker not covered here, leave the service stopped and report the
exact named state. Do not turn an unknown check into a success by looking at an
older sample row.

## 10. Stopping, opting out, and restarting

- **Stop collector:** use the fixed Stop action after its confirmation. This
  stops the process and disconnects BLE; it does not send a therapy command.
- **Restart collector:** use the fixed Restart action after its confirmation.
  This forces a new authentication/session cycle and is useful after an
  external credential or dependency fix.
- **Opt out:** stop the collector first, then use Opt out and confirm. The
  backend refuses opt-out while the service state is actively running.
- **Disable for maintenance:** stop the unit through the approved service
  procedure. Do not enable it at boot as part of this integration change.

Every fixed action has an audit request/result where the database is available.
If the action result or audit append is uncertain, the page says so and the
operator must inspect service state rather than repeat blindly.

## 11. Bounded live acceptance

After explicit operator authorization and only on the real Pi, run the exact
bounded evidence command:

```sh
scripts/air11-pi-acceptance.sh --duration 60 --require-postgres --require-h10 --require-emay --require-ezshare-unchanged
```

The harness requires its duration-specific interactive authorization. It
checks the existing bond, private credentials, measured/null evidence,
host/device provenance, PostgreSQL rows, distinct sessions, reconnect behavior,
and unchanged H10/EMAY/EzShare routes. It does not pair, scan, change an
adapter, enable overlays, change routes, or enable the AirSense unit. A nonzero
exit code is failed acceptance; a chat report is not a substitute.

Do not mark live acceptance complete based on the admin page alone. The page is
an operational monitor; the harness is the bounded acceptance contract.

## 12. Future cross-path validation against EzShare EDF

It is theoretically possible—and desirable—to compare a complete EzShare EDF
file with BLE data from the same AirSense therapy session. This is a validation
exercise, not a second BLE ingest path:

1. retain the complete EDF locally;
2. run the BLE collector over the overlapping therapy period and retain its
   bounded journal and derived rows locally;
3. map equivalent EDF channels and BLE stream ids through the registry;
4. align the two timelines using session landmarks, without rewriting stored
   host timestamps or treating device time as trusted; and
5. compare coverage, declared cadence, nulls, missing intervals, and values
   using per-signal tolerances.

The result is not expected to be byte-for-byte identical. EDF and BLE may differ
in cadence, quantization, filtering, or derived calculations. The comparison
should distinguish transport loss, genuine unavailable values, representation
or timing differences, and actual value disagreement. A BLE gap is evidence to
investigate, not proof that the EDF value should be copied into the live path.

Keep the EDF, raw BLE journal, and comparison output outside Git and redact
identifiers before sharing any summary.

## 13. Removal and recovery boundary

Stopping the unit leaves the external credential file and journal untouched.
Do not delete the journal, drop the admin tables, or remove the credential as a
first troubleshooting step. Preserve evidence, record the blocker, and use
the normal reviewed recovery procedure. The AirSense admin migration contains
no physiological rows and no secret values, so database backup/restore does
not replace backup of the separately managed credential file.
