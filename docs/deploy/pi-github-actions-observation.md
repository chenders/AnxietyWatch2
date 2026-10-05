# Pi GitHub Actions delivery observation

Status: **GitHub deployment observation remains historical; manual silence-detector acceptance is complete**. Per Amendment 18, T-R8 closes on finite evidence rather than elapsed time.

## Delivery under observation

- Host: `$PI_HOST`
- Merged source SHA: `bbc4cf85dc4d1124261c61c0365d628ba36eb5ab`
- Activation workflow: [37113131668](https://github.com/chenders/AnxietyWatch2/actions/runs/37113131668)
- Target: the reviewed native release set and its selected systemd units
- Activation receipt: `/var/lib/vitals/recovery/github-deploy-latest.json`
- Deployment receipt: `activated=true`, `preflight_ok=true`, `health_ok=true`,
  `spool_unchanged=true`, `backup_gate=true`; the GitHub workflow did not perform the separate
  operator-gated silence-detector schema/runtime acceptance recorded below
- Backup label: `20261002-213707F`
- Recovery gate spool fingerprint: `b94eb3f373c9fdaed40e624c3fe36a21d85694925078575ad0e99f3de73ea564`
- Rollback release retained: `/usr/local/lib/vitals/releases/2c30fc371eb4b1e7eb47347d7a82d01eac6f3093`
- Root helper installed from the reviewed ARM64 artifact: SHA-256
  `8183674d80389c6a434f089c33911eb0e6d5a8fe526d703ffac44d783757754c`

The two earlier activation attempts were retained as evidence: `37096166778` failed closed on a
stale spool identity, and `37097541745` passed preflight but rolled back when the old filename-based
fingerprint changed across writer restart. Neither was reported as successful deployment.

## Recovery and gate evidence

The fresh full pgBackRest backup and non-vacuous isolated restore verification used label
`20261002-213707F`, checked 31 tables with no mismatches, and completed `pg_amcheck`. The encrypted
backup set and the pending spool were copied to the operator-controlled off-Pi recovery directory
`$OFF_PI_RECOVERY_PATH/20261002-213707F-github-deploy`; the end-to-end checksum comparison passed
for 2,212 files. The encryption key remains only in the root-only Pi configuration.

The spool-seal timer was stopped only for gate creation and activation, then restarted after the
result. The reviewed helper's content-based fingerprint remained stable across the controlled writer
restart while preserving the one 37-byte pending segment.

## Post-deploy evidence

A read-only snapshot was taken at `2026-10-03T05:33:42Z`, after a scheduled spool-seal cycle:

- active release: `a60304ef952575d757af4bfc6a2c551ad251f990`
- PostgreSQL: `ready`, schema `public`, `samples` present
- GitHub runner: `active`
- `vitalsd`, `vitals-console`, `vitals-spool-drain`, `vitals-h10`, and `vitals-bp`: `active`
- `vitals-bp-autoconnect`: `inactive` (oneshot unit after completion)
- `silence-detector`: `inactive` and `disabled`; the unit and binary are present, but the service was
  deliberately left off pending the runtime fix below
- vitalsd socket: present
- spool: one 37-byte segment, unchanged content fingerprint
- `vitals-spool-seal.timer`: `active`; the observed seal cycle completed with `count=0 durable=true`
- memory: 7.9 GiB total, 3.4 GiB available; root filesystem 2% used
- priority journal errors since activation: none

## Silence-detector incident and remediation

The earliest Pi journal evidence is `2026-09-27T19:52:03-07:00`, immediately after the appliance
integration merge `79201d3` introduced the daemon and unit. The daemon exited with status 70 because
the unit supplied a libpq keyword connection string to SQLx, which requires a PostgreSQL URL.

On `$PI_HOST`, workflow `37092938522` first exposed a separate artifact-mode failure at
`2026-10-02T20:28:57-07:00`: the restored binary was mode `0700` and systemd reported `203/EXEC`.
After the executable-mode fix, workflow `37099671541` started the binary at
`2026-10-02T22:30:56-07:00`; it then reached the database connection and repeatedly reported
`relative URL without a base`. The operator stopped the resulting restart loop at
`2026-10-02T22:31:21-07:00`.

The corrected unit uses a SQLx PostgreSQL URL for the Unix socket, with doubled percent signs so
systemd passes `%2F` to SQLx rather than interpreting it as a unit specifier. The deployment helper
waits for three consecutive stable post-restart observations, requires structured successful states
for long-running units, preserves the completed BP oneshot exception, and records per-service state,
result, exit status, restart count, and health in activation receipts. The first post-fix activation
attempt, workflow `37109378859`, failed closed in preflight because the downloaded udev rule retained
mode `0600` while the manifest required `0644`; no live release or service state was mutated. The
next activation, workflow `37111157822`, exposed the systemd percent-specifier defect: the service
entered an auto-restart loop and the helper's single transient health sample incorrectly passed.
The activation receipt for corrected release `bbc4cf85dc4d1124261c61c0365d628ba36eb5ab` was green,
but it did not contain the new per-service `services` evidence: the workflow invoked the
root-owned helper installed before that release, so the strengthened helper was not actually the
live activation controller. Direct runtime observation then found the live database lacks the
`status_observations.next_repeat_at` column and `notification_events` table required by the daemon;
all persistence queries failed. The repository's `silence_detector` Sqitch change exists, while automatic schema migration from
the GitHub activation workflow is intentionally a non-goal. The separate
`silence-detector-live-activation` change now provides operator-gated migration and helper-refresh
procedures; the service was stopped and remains disabled until those receipts and runtime evidence
pass. No successful silence-detector evaluation or notification delivery is inferred from any green
activation receipt.

On `2026-10-04`, the operator-gated schema procedure applied the migration and repaired the `vitals`
role grants for `notification_events`; the helper refresh receipt and schema receipt both verified.
The H10, BLE broker, and Bluetooth prerequisite units were started for acceptance. The detector initially
found no `attempts.kind = 'source'` rows because collectors persist durable samples rather than console
operation attempts. The detector was corrected to use the newest persisted sample as successful source
evidence when it is newer than an attempt failure. After deploying that correction, the final runtime
receipt recorded three 30-second cycles, 8 persisted status observations, 8 notification events, and
unchanged `NRestarts=0`; `silence-detector` is now `enabled`, `active/running`, with `Result=success`
and `ExecMainStatus=0`. The final deployed detector was built from source commit `8b169e3` and
has SHA-256 `3597bcb9362cfb053852952fb38e1794a96f468cb63d881e5a3f4628e713d9ad` at
`/usr/local/bin/silence-detector`; the protected runtime receipt remains at
`/var/lib/vitals/recovery/silence-detector-runtime-latest.json`.

## Manual rollback rehearsal

Receipt: `/var/lib/vitals/recovery/manual-rollback-rehearsal-20261002.json`.

At `2026-10-03T05:32:33Z`, the operator switched from release
`a60304ef952575d757af4bfc6a2c551ad251f990` to the prior full release
`2c30fc371eb4b1e7eb47347d7a82d01eac6f3093`, daemon-reloaded, and restarted the selected services.
The rollback exit code was `0`; release, database, runner, socket, service, and spool checks passed.
The deployed release was restored with exit code `0`, the same checks passed, spool identity remained
unchanged, and the spool-seal timer was restored active.

## Operator decision

Retain `scripts/sync-to-pi.sh` as the emergency/manual recovery path. It is not called by the
GitHub workflow and is not being retired by this change.
