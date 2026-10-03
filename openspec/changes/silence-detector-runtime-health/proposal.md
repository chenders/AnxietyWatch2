# Change: make silence-detector activation-safe

## Why

The Pi's `silence-detector` binary was shipped with a libpq keyword connection string even though
its `sqlx::PgPool::connect` entry point requires a PostgreSQL URL. It therefore exited with status
70. The deployment helper also sampled service state immediately after `systemctl restart`, which
could observe `active` during a restart loop and report a green activation receipt.

## What changes

- configure the daemon with a valid Unix-socket PostgreSQL URL and reject unsupported URL syntax at
  process startup;
- normalize systemd unit artifacts to readable mode `0644` while retaining `0755` for binaries;
- collect structured systemd service state, exit status, and restart count;
- wait through the post-restart settling window and require stable successful long-running services;
- retain the completed `vitals-bp-autoconnect` oneshot exception;
- record named service health blockers and restart-count regressions in activation failures;
- document the incident timeline and perform a controlled Pi verification before enabling the unit.

## Non-goals

- No database schema or notification-policy changes.
- No automatic enabling of the service without the operator-controlled Pi verification.
- No changes to Bluetooth, boot firmware, the spool, or PostgreSQL backup policy.
