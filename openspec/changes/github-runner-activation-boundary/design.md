# Design: GitHub runner activation write boundary

## Observed failure

Workflow run `37092323616` passed runner identity, toolchain, build, manifest generation, and
fault rehearsal. Its protected activation then failed with:

```text
vitals-deploy: Read-only file system (os error 30)
```

The runner service has `ProtectSystem=full`. The helper's activation path writes the release tree
under `/usr/local/lib/vitals`, binaries under `/usr/local/bin`, console assets under
`/usr/local/share/vitals-console/assets`, and checked-in units/rules under `/etc/systemd/system`
and `/etc/udev/rules.d`. `sudo` does not escape the service mount namespace.

## Boundary

Add one `ReadWritePaths=` directive to the runner unit containing exactly those five existing target
roots. Keep `ProtectSystem=full`, `ProtectHome=yes`, `PrivateTmp=yes`, CPU/memory bounds, and
`NoNewPrivileges=no` as the deliberate prerequisite for the narrow sudoers rule.

The runner user still has no write permission through normal Unix ownership/mode checks. Only
`/usr/local/bin/vitals-deploy` may run as root, and the helper continues to validate manifest paths,
checksums, target kinds, backup gate, spool identity, health, and rollback.

## Verification ladder

1. Validate the OpenSpec change and unit text locally.
2. Install the unit on `minidude`, run `systemd-analyze verify`, reload, and restart the runner.
3. Confirm the runner is active, the runner account cannot directly write the target roots, and the
   helper's staged activation can proceed through the existing protected workflow.
4. Confirm the deployment receipt reports activation or an explicit named blocker, and restart the
   spool-seal timer after the maintenance window.

No direct write test is performed against live targets by the unprivileged account. The protected
helper remains the only activation writer.
