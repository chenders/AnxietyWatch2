# Status update — 2026-09-11T10:07:27Z

## Goal

Finish the host-safe AirSense 11 admin/kiosk console surface while keeping
credential handling, lifecycle actions, and physical activation explicitly
bounded.

## Completed in this interval

- Added a task and Verify contract for the proposed admin/kiosk OpenSpec change.
- Added host checks for console asset syntax, kiosk contract markers, SQL
  migration structure, service confinement, and public-information hygiene.
- Added tests for the private-network middleware, fixed-action authorization,
  database-unavailable mutation handling, journal severity, malformed output,
  and bounded log projection.
- Reconciled the console credential-readiness path with `ProtectHome=true` by
  using an optional systemd-loaded credential view; missing external credentials
  leave status and logs available and do not enable mutations.
- Added `.idea/` to the repository ignore rules so local IDE metadata does not
  enter the task's staged file set.

## Verification

The admin/kiosk Verify command passed on the host: OpenSpec validation,
registry generation check, Rust formatting, workspace tests, workspace Clippy
with warnings denied, asset checks, SQL/static checks, public-information
checks, and diff hygiene all exited successfully.

This is host evidence only. No credentials, Bluetooth bond, service
activation, route change, adapter change, or Pi measurement was used.

## Open governance and hardware gates

The admin/kiosk OpenSpec change remains **Proposed**. Its direct private-LAN
operator deployment must be reconciled with amendment 09's accepted
loopback/WireGuard production topology before production acceptance. The
bounded Pi live-acceptance task remains separate and blocked until explicit
authorization, an existing bond, approved credentials, and a real Pi are
available.

## Next safe step

Obtain review and the governance decision on the private-LAN deployment shape.
After that, run the exact bounded Pi command only if the operator explicitly
authorizes the physical acceptance window.
