# Pi GitHub Actions delivery observation

Status: **complete**. Per Amendment 18, T-R8 closes on finite evidence rather than elapsed time.

## Delivery under observation

- Host: `$PI_HOST`
- Merged source SHA: `a60304ef952575d757af4bfc6a2c551ad251f990`
- Activation workflow: [37099671541](https://github.com/chenders/AnxietyWatch2/actions/runs/37099671541)
- Target: the reviewed native release set and its selected systemd units
- Activation receipt: `/var/lib/vitals/recovery/github-deploy-latest.json`
- Result: `activated=true`, `preflight_ok=true`, `health_ok=true`, `spool_unchanged=true`,
  `backup_gate=true`
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
- `silence-detector`: `inactive` (host unit is not loaded; not treated as an unknown or failed state)
- vitalsd socket: present
- spool: one 37-byte segment, unchanged content fingerprint
- `vitals-spool-seal.timer`: `active`; the observed seal cycle completed with `count=0 durable=true`
- memory: 7.9 GiB total, 3.4 GiB available; root filesystem 2% used
- priority journal errors since activation: none

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
