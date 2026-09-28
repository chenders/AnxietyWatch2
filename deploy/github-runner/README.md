# Native GitHub Actions runner for the Pi

This is the bootstrap contract for the `vitals-pi` runner. It is intentionally separate from the
native service deployment contract: the runner must first prove that it can execute harmless tests,
then later receive narrowly scoped staging authority, and only after the dry-run/backup gates may it
restart a service.

## Bootstrap boundary

Perform these steps on the Pi as an operator. Do not put the runner token, host address, private
network details, or production credentials in this repository.

1. In the repository's **Settings → Actions → Runners**, create a **new repository runner** for
   Linux ARM64 and obtain its short-lived registration token. Do not create a PAT for this runner.
2. Create a dedicated account and root-owned installation directory:

   ```sh
   sudo useradd --system --create-home --home-dir /var/lib/github-vitals \
     --shell /usr/sbin/nologin github-vitals
   sudo install -d -o github-vitals -g github-vitals -m 0750 \
     /opt/actions-runner-vitals /var/lib/github-vitals
   ```

3. Download the official ARM64 Actions Runner release selected by the operator, verify its
   published SHA-256 checksum, and unpack it as `github-vitals`. Do not use `latest` without a
   recorded version and checksum.
4. Configure it for this repository with labels exactly matching the smoke workflow:

   ```sh
   sudo -u github-vitals -H /opt/actions-runner-vitals/config.sh \
     --url https://github.com/chenders/AnxietyWatch2 \
     --token "$RUNNER_TOKEN" \
     --name vitals-pi \
     --labels self-hosted,linux,ARM64,vitals-pi \
     --work _work \
     --unattended \
     --replace
   unset RUNNER_TOKEN
   ```

   The token is consumed during configuration and must not be written to a file or shell history.
   Confirm the runner is visible as **Idle** before proceeding.
5. Install the runner as a systemd service. The service must run as `github-vitals`, use the fixed
   working directory, and have no Docker socket, no production `EnvironmentFile`, and no broad
   `sudo` permission. A representative unit is:

   ```ini
   [Unit]
   Description=GitHub Actions runner for AnxietyWatch2 native Pi delivery
   Wants=network-online.target
   After=network-online.target

   [Service]
   User=github-vitals
   Group=github-vitals
   WorkingDirectory=/opt/actions-runner-vitals
   ExecStart=/opt/actions-runner-vitals/run.sh
   Restart=always
   RestartSec=10
   TimeoutStopSec=5min
   UMask=0077
   # Required for the narrow sudoers rule to elevate only vitals-deploy.
   NoNewPrivileges=no
   PrivateTmp=yes
   ProtectHome=yes
   ProtectSystem=full
   MemoryHigh=1G
   CPUQuota=200%

   [Install]
   WantedBy=multi-user.target
   ```

   Install the checked-in unit and narrow sudoers rule:

   ```sh
   sudo install -o root -g root -m 0644 deploy/github-runner/github-vitals-runner.service \
     /etc/systemd/system/github-vitals-runner.service
   sudo install -o root -g root -m 0440 deploy/github-runner/github-vitals-deploy.sudoers \
     /etc/sudoers.d/github-vitals-deploy
   sudo visudo -cf /etc/sudoers.d/github-vitals-deploy
   sudo systemctl daemon-reload
   sudo systemctl enable --now github-vitals-runner
   ```

   `NoNewPrivileges=no` is deliberate: the runner cannot use `sudo` otherwise. Systemd sandbox
   options such as `RestrictNamespaces` and `LockPersonality` implicitly enable
   `NoNewPrivileges`, so they are intentionally not present here. The sudoers rule permits only
   the root-owned `vitals-deploy` helper and does not permit a shell, `systemctl`, `install`, or
   arbitrary root commands. `ProtectSystem=full` is only the bootstrap posture; the
   implementation task must verify the runner's own update/work-directory behavior before hardening
   it further. Never set `RUNNER_ALLOW_RUNASROOT`.
6. Install the pinned Rust toolchain and components as the runner user, plus only the documented
   build prerequisites:

   ```sh
   sudo -u github-vitals -H rustup toolchain install 1.97.1 \
     --component rustfmt,clippy,llvm-tools-preview
   sudo systemctl daemon-reload
   sudo systemctl enable --now github-vitals-runner
   ```

7. Manually dispatch `.github/workflows/pi-runner-smoke.yml`. The smoke job must prove Linux,
   `aarch64`, the pinned toolchain, formatting, and `vitals-decode` tests. It must report
   `Deployment performed: false`. A green GitHub status is not a substitute for checking the local
   runner account, labels, and systemd unit.

The exact unit installation and runner release checksum will be captured as an implementation
receipt by the runner-bootstrap task. This document is not permission to install a deployment
helper or to grant the runner root access.

## Staged delivery commands

The checked-in `vitals-deploy` helper has three deliberately separate modes:

- `manifest --stage-dir ...` builds a SHA-addressed immutable staging tree and never touches live paths.
- `apply --dry-run` snapshots services, database identity, runner state, live files, and spool identity; it validates every staged checksum and unit without activation.
- `apply --activate` requires root, `--backup-gate`, `VITALS_DEPLOY_ALLOW_ACTIVATE=1`, an ARM64 manifest with no blockers, and a successful preflight. It installs a new release, restarts only manifest-selected services in dependency order, checks health and spool identity, and records rollback evidence.

A backup gate must be a root-owned JSON receipt containing `backup_verified: true`, `restore_verified: true`, `off_pi: true`, a non-empty `spool_fingerprint`, `rollback_release`, and `encryption_key_location`. The encryption key itself must never be placed in the receipt, GitHub, or this repository. The activation command is intentionally not run by bootstrap or smoke workflows:

```sh
VITALS_DEPLOY_ALLOW_ACTIVATE=1 sudo --preserve-env=VITALS_DEPLOY_ALLOW_ACTIVATE \
  /usr/local/bin/vitals-deploy \
  apply --manifest /var/lib/vitals/deploy/<sha>/manifest.json \
  --activate --backup-gate /var/lib/vitals/recovery/github-deploy-gate.json \
  --receipt /var/lib/vitals/recovery/github-deploy-latest.json
```

The operator must retain the prior release and use the receipt's rollback command if health or data-path evidence is not green. A failed preflight or missing backup gate must leave the live release untouched.

## Operational rules

- The runner is repository-scoped and must not be shared with unrelated repositories.
- Only trusted `main` jobs may eventually use the deploy label. Pull requests never target it.
- The activation workflow may use a narrowly allowlisted sudoers entry for the root-owned
  `/usr/local/bin/vitals-deploy` helper only; it must not permit `systemctl`, `install`, a shell, or
  arbitrary commands. The helper validates the manifest and backup gate again as root.
- No Docker socket is mounted. The native life-safety path remains outside containers.
- Registration-token renewal, runner software upgrades, and removal are operator tasks; record the
  runner version and result without recording tokens.
- If the runner is compromised or its labels drift, disable it in GitHub and stop the service before
  investigating. Do not repair it by granting more privilege.
- The runner is not part of the alarm's runtime dependency chain. Loss of GitHub connectivity may
  stop delivery but must not stop collectors, vitalsd, the drainer, or an already-running alarm.
