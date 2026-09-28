# Pi GitHub Actions delivery observation

Status: **complete**. Per Amendment 18, T-R8 closes on finite evidence rather than elapsed time.

## Delivery under observation

- Host: `$PI_HOST`
- Branch: `chore/github-runner-deployment-plan`
- Activated source SHA: `9b4fff2091be39a5f8517ab52a99cfd764e6658b`
- Target: console binary, console assets, and `vitals-console.service` only
- Activation receipt: `/var/lib/vitals/recovery/github-deploy-latest.json`
- Result: `activated=true`, `preflight_ok=true`, `health_ok=true`, `spool_unchanged=true`,
  `backup_gate=true`
- Rollback release retained: `/usr/local/lib/vitals/releases/pre-runner-20260927`
- Rollback command retained in the activation receipt; no rollback was needed for the successful
  minor activation.

The protected activation job in `.github/workflows/pi-deploy.yml` is restricted to `main`. The
approved branch validation therefore used the same root helper and gates directly on the Pi; the
branch workflow was exercised only in build-only mode.

## Evidence collected

Build-only workflow run `36371955874` completed successfully for commit
`a76f0f8e60c35a349d1aad71db67064a28c663c7`:
<https://github.com/chenders/AnxietyWatch2/actions/runs/36371955874>

A second, manually dispatched build-only run `36372549404` also completed successfully for the
same commit; its `activate=false` input skipped the production job:
<https://github.com/chenders/AnxietyWatch2/actions/runs/36372549404>

These deliveries are supporting evidence only, not a mandatory count or elapsed-time gate.

At the post-deployment observation sample on 2026-09-28 03:08 UTC:

- active release: `9b4fff2091be39a5f8517ab52a99cfd764e6658b`
- PostgreSQL: `ready`
- GitHub runner: `active`
- `vitalsd`, `vitals-console`, and `vitals-spool-drain`: `active`
- spool: one 37-byte segment; drainer repeatedly reported offset 37 and `inserted=0`
- `vitals-spool-seal.timer`: `active`
- `vitalsd` socket and deployment lock: present/clear

The scheduled spool-seal/drainer idle path continued to run after activation without changing the
release or database state. Three read-only samples 65 seconds apart retained release
`9b4fff2091be39a5f8517ab52a99cfd764e6658b`, PostgreSQL `ready`, runner `active`, one 37-byte spool
segment, and active `vitalsd`/console/drainer services. The segment fingerprint rotated as the seal
timer published fresh empty segments; byte count and database state remained unchanged.

## Manual rollback rehearsal

Receipt: `/var/lib/vitals/recovery/manual-rollback-rehearsal.json`

On 2026-09-28 08:05 UTC, the operator manually switched from release
`9b4fff2091be39a5f8517ab52a99cfd764e6658b` to `pre-runner-20260927`, daemon-reloaded, and restarted
`vitals-console`. The rollback exit code was `0`, the rollback state checks passed, and the
`vitalsd` socket, database/runner observability, and spool evidence remained valid. The operator
then restored the deployed release with exit code `0`; the restore state checks passed and
`spool_unchanged` was `true`.

## Operator decision

Retain `scripts/sync-to-pi.sh` as the emergency/manual recovery path. It is not called by the
GitHub workflow and is not being retired by this change.

## Known host conditions

These were observed before the successful console-only activation and are outside its selected
targets: `silence-detector.service` is not installed on the legacy host, and `vitals-bp` is in its
known retry loop because the BLE broker socket is absent. `NetworkManager-wait-online.service` and
`nvmf-autoconnect.service` are also listed as failed host units. They remain named observations,
not evidence that the console activation changed the data path.

## Migration decision

`scripts/sync-to-pi.sh` remains installed as an **emergency/manual rollback and recovery path**.
It is not called by the GitHub workflow. The post-deploy evidence, manual rollback rehearsal, and
operator retain decision are now recorded.
