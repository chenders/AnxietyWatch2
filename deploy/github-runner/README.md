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
   TimeoutStopSec=5
   NoNewPrivileges=yes
   PrivateTmp=yes
   ProtectHome=yes
   ProtectSystem=full
   RestrictNamespaces=yes
   LockPersonality=yes
   MemoryHigh=1G
   CPUQuota=200%

   [Install]
   WantedBy=multi-user.target
   ```

   `ProtectSystem=full` is only the bootstrap posture; the implementation task must verify the
   runner's own update/work-directory behavior before hardening it further. Never set
   `RUNNER_ALLOW_RUNASROOT`.
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

## Operational rules

- The runner is repository-scoped and must not be shared with unrelated repositories.
- Only trusted `main` jobs may eventually use the deploy label. Pull requests never target it.
- No Docker socket is mounted. The native life-safety path remains outside containers.
- Registration-token renewal, runner software upgrades, and removal are operator tasks; record the
  runner version and result without recording tokens.
- If the runner is compromised or its labels drift, disable it in GitHub and stop the service before
  investigating. Do not repair it by granting more privilege.
- The runner is not part of the alarm's runtime dependency chain. Loss of GitHub connectivity may
  stop delivery but must not stop collectors, vitalsd, the drainer, or an already-running alarm.
