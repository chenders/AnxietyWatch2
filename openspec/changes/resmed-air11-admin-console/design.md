# Design — ResMed AirSense 11 admin and kiosk console

## Governing decisions

This change follows amendments 07, 08, and 09. The API and admin surface are
Rust. One AirSense view-model feeds Service Mode, Preflight, and Flowpath
renderings. The console is private to the configured local network; it is not a
public phone API. The AirSense BLE device remains read-only.

## Backend projection

`GET /api/air11` is the only browser data feed for this capability. The handler
reads the atomically published `/run/vitals-air11/status.json`, selected
`systemctl show` properties, bounded `journalctl` JSON output, external
credential-file metadata/shape, and the non-secret PostgreSQL config/audit
records. It returns a typed safe projection:

- `link_state` and `measurement_state` remain separate;
- process state and restart count remain separate from health;
- host receipt age is distinct from raw untrusted device time;
- zero, false, null/unavailable, and not-observed remain distinguishable;
- status blockers, service blockers, journal-read blockers, and durability
  uncertainty are retained and promoted; and
- no target address, serial, client id, master key, session key, raw frame, or
  unbounded journal text is returned.

The backend selects only `MESSAGE`, `PRIORITY`, and timestamp fields from the
system journal, limits entry count and message length, and redacts private
address/key-shaped tokens before returning them. It never exposes raw journal
JSON or a general command/log endpoint.

## Credential and configuration boundary

The AirSense credential remains at the external operator-managed path used by
the collector. Under the deployed `ProtectHome=true` sandbox, systemd gives the
console an optional private `LoadCredential` copy for its bounded
shape/permission check; a missing source must not stop read-only status/log
serving. The console returns only readiness booleans and a named state. It
never accepts, stores, returns, logs, or displays the AirSense key. PostgreSQL stores:

- one `air11_integration_config` singleton with `opted_in`, created, and
  updated timestamps; and
- append-only `air11_admin_audit` rows containing a fixed action, outcome, and
  non-secret detail.

A separate operator-managed console-admin capability authorizes mutations. It is
read from a private file at process startup, never returned by the API, and is
not the AirSense credential. The page holds a supplied token only in tab memory.
A per-process random action token prevents cross-site requests from invoking a
fixed action without first reading the same-origin page.

## LAN and authorization boundary

The listener binds to a LAN-capable address (the code default is
`0.0.0.0:8480`). The AirSense API route uses `ConnectInfo<SocketAddr>` and
admits only configured private CIDRs, defaulting to loopback, RFC1918 IPv4, and
IPv6 unique-local space. An operator may narrow this to the appliance's exact
local CIDR. A peer outside the list or a missing peer identity is rejected with
a named blocker.

Read-only status and logs are available to an allowed private-LAN peer. POST
operations require all of:

1. the service policy flag enabling AirSense actions;
2. the per-process action token;
3. the separate console-admin bearer capability; and
4. an exact confirmation phrase for opt-in/out/start/stop/restart.

The POST handler contains no user-selected program, unit, verb, path, or RPC.
It can only write the config row or invoke `systemctl {start|stop|restart}
vitals-air11.service`. A narrow polkit rule is supplied for the service user;
no permission is granted for another unit. A request is audited before a
systemd action, and the result is appended afterward. If the result cannot be
audited, the response says that the outcome is unknown.

Opt-out is refused while the service is actively running. Start is refused
unless the database says opted-in and the external credential check is ready.
Stopping/restarting the service disconnects BLE and never sends an AirSense
`StopStream` or therapy command.

## Admin UI

The existing console `model()` seam gains an `air11` screen. Each direction
renders the same model:

- Service Mode uses a dense evidence-card hierarchy;
- Preflight uses an explicit checklist/row layout; and
- Flowpath uses horizontally ordered evidence nodes.

All directions show the same primary attention area, with `FAILING` blockers
above `UNKNOWN`, and `NOT CONFIGURED` outside the health palette. The page
contains:

- summary facts for link, measurement, process, credential, opt-in, and last
  host receipt;
- full status counters and clock-provenance facts;
- credential readiness and external-provisioning instructions;
- lifecycle controls with typed confirmation and an in-tab admin-token field;
- bounded sanitized service logs with errors/warnings clearly promoted;
- append-only AirSense admin audit history; and
- explicit stale/unknown/error/empty states.

The page does not show a physiology value, infer a health result from a BLE
connection, or provide any device-control/pairing UI.

## Dedicated kiosk

`/console/air11-kiosk.html` is a separate read-only monitor. It uses the same
`/api/air11` projection, refreshes on a bounded interval, renders large link,
measurement, process, and credential-readiness facts, and keeps the last known
content visible behind a page-level stale warning. Status blockers and error
journal entries are promoted above routine counters. The kiosk contains no
admin token input or lifecycle control. The existing `/dashboard` links to it,
but continues to own physiological display and interpretation.

## Verification

Host verification covers Rust formatting, console tests and Clippy,
JavaScript syntax, SQL structure, public-information scans, and `git diff
--check`. Tests cover safe projection/redaction, credential readiness without
secret leakage, CIDR parsing/fail-closed behavior, systemd parsing, journal
severity, confirmation phrases, and stale/unknown rendering contracts. Live
service operation remains behind the separate explicit Pi acceptance gate.
