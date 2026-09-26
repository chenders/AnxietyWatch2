# Design — native ARM64 runner and staged Pi delivery

## 1. Evidence from the existing AnxietyWatch deployment

`../AnxietyWatch/.github/workflows/deploy.yml` has this shape:

1. A GitHub-hosted `ci` reusable workflow runs Python lint, Bandit, PostgreSQL-backed tests, and a
   Docker build.
2. A GitHub-hosted job builds and pushes `ghcr.io/chenders/anxietywatch-server:latest` and a
   commit-SHA tag.
3. A job on the self-hosted `anxietywatch` runner checks out the repository, logs into GHCR, copies
   the production Compose file to `/opt/anxietywatch`, pulls, runs `docker compose up -d --wait`,
   polls the container health state, prints Compose status, and prunes old images.
4. `docker-compose.runners.yml` runs an ephemeral `myoung34/github-runner` container with the host
   Docker socket and `/opt/anxietywatch` mounted. `SERVER_SETUP.md` bootstraps it with a repo PAT
   having Administration read/write permission.

That is a useful delivery precedent, but not a safe copy for this project. Its deploy authority is
root-equivalent through the Docker socket; it deploys the mutable `latest` tag; it has no release
rollback or database backup gate; and its health proof is container health rather than an end-to-end
application and data-path assertion. Those are acceptable risks to revisit for a self-hosted sync
server, not defaults for a local physiological appliance.

## 2. Existing AnxietyWatch2 deployment boundary

The current project has native systemd units under `deploy/`, a pinned `rust-toolchain.toml`,
PostgreSQL/Sqitch/pgBackRest operations, and no container in the life-safety path. The current
`scripts/sync-to-pi.sh`:

- requires operator-provided `PI_HOST` and `PI_PATH` and uses SSH/rsync;
- deletes the remote checkout contents except selected build and VCS directories;
- builds or tests in that checkout only after the copy;
- has no SHA or artifact manifest, staged release, checksum, backup, health gate, rollback, or
  concurrency lock; and
- is also used by existing task verification commands, so it cannot disappear until those contracts
  are migrated.

The checked-in units currently name `vitalsd`, `vitals-spool-drain`, `vitals-console`, H10/BP
collectors, and a silence detector. The workspace currently exposes several libraries and binaries,
but the collector and silence-detector service targets must be reconciled with actual build outputs
before a full-fleet deployment can be claimed. The deployment tool must fail with the missing target
name and expected package, never omit it.

## 3. Proposed topology

### 3.1 Runner

Install the official GitHub Actions runner natively on the Pi at a fixed root-owned path such as
`/opt/actions-runner-vitals`, configured only for this repository. Run it as a dedicated, unprivileged
`github-vitals` account under systemd. Label it with `self-hosted`, `linux`, `ARM64`, and a project
specific label (for example `vitals-pi`); the deployment workflow must require all labels.

The runner account gets only the tools needed to build and stage: Git, Rustup with the pinned
stable toolchain plus `rustfmt`/`clippy`, Cargo, and the small POSIX utilities used by the scripts.
It does **not** get the Docker socket, database credentials, BLE credentials, the production
configuration directory, or a general-purpose `sudo` rule. Runner registration uses a short-lived
repository runner token obtained during bootstrap, not a long-lived PAT stored in the repository.
The token is deleted after configuration. Runner updates are pinned and checksum-verified during a
planned maintenance task rather than silently downloading arbitrary code in a job.

A runner container is deliberately not used: Docker is not part of the native path, and exposing a
socket would give source executed by a trusted `main` job host-root-equivalent control. GitHub is
used only to deliver a trusted commit and record evidence; the Pi's services continue to run when
GitHub or the network is unavailable.

### 3.2 Workflow trust boundary

- Pull requests run only on GitHub-hosted runners.
- The runner smoke workflow is `workflow_dispatch` only until the operator confirms the labels and
  toolchain.
- Build/deploy runs only for a push to `main` after ordinary CI succeeds, or for a manually approved
  dispatch from a selected trusted ref. No workflow input becomes a shell fragment or path.
- The deploy job uses `permissions: contents: read`, a serialized concurrency group, a short timeout,
  and a protected `production` environment approval before any deploy authority is exercised.
- Checkout uses `persist-credentials: false`. Every artifact is tied to `GITHUB_SHA`; the workflow
  refuses a dirty tree and refuses a SHA mismatch between checkout, manifest, and deploy helper.
- The runner work directory is not the live installation directory. Job cleanup must not touch
  `/usr/local/bin`, `/etc/systemd/system`, `/var/lib/vitals`, PostgreSQL, or the spool.

### 3.3 Build and artifact contract

The Pi builds natively with the pinned toolchain, because BlueZ/systemd and ARM64 runtime behavior
are part of the acceptance surface. Hosted CI remains responsible for fast portable checks; the Pi
job runs the ARM64 checks that cannot be honestly simulated on macOS.

The build produces a staging directory outside the checkout containing:

- only an explicit allowlist of binaries that actually exist and are required by checked-in units;
- service units, rules, and other static files selected by the same allowlist;
- the commit SHA, toolchain version, target architecture, build time, and source tree hash;
- SHA-256 checksums, file modes/owners as intended, and a schema/registry fingerprint where relevant;
- a versioned manifest with no environment values, tokens, private keys, or physiological data.

The manifest is validated before it is handed to the root helper. Missing binaries, unexpected files,
symlinks, path traversal, non-root-owned unit sources, or service references outside the allowlist
fail closed and identify the exact name/value that blocked deployment.

### 3.4 Staging, activation, and rollback

Use an immutable release layout owned by root, for example:

```text
/usr/local/lib/vitals/releases/<commit-sha>/bin/...
/usr/local/lib/vitals/releases/<commit-sha>/deploy/...
/usr/local/lib/vitals/releases/<commit-sha>/manifest.json
/usr/local/lib/vitals/current -> <commit-sha>
```

The exact paths are to be finalized by the implementation task after checking every `ExecStart`.
The root-owned helper performs only these ordered operations:

1. acquire a deployment lock and reject a concurrent run;
2. validate the manifest, target architecture, checksums, and current release;
3. copy into a new release directory, never overwrite the current directory;
4. run `systemd-analyze verify` against staged unit files and verify service-to-binary mapping;
5. in dry-run mode, stop here and emit a receipt with `would_change` entries;
6. for a real deploy, atomically update the `current` pointer and install the allowlisted units;
7. daemon-reload and restart only the services whose binaries/units changed, in dependency order;
8. evaluate health and data-path checks; on failure, restore the previous pointer/units, daemon-reload,
   restart the previous service set, and emit `rollback blocked` or `rollback completed` with the
   failing value;
9. retain the previous release until the observation and backup policy permits cleanup.

The helper must not run Sqitch, change PostgreSQL, clear the spool, change Bluetooth pairing, or
modify `/boot/firmware`. Database migrations, when eventually needed, are a separate explicitly
approved operation with the existing spool-empty and backup gates.

### 3.5 Health and data-path proof

A deploy is not green because `systemctl is-active` is green. The acceptance contract must include:

- exact release/manifest SHA reported by the helper;
- `systemd-analyze verify` success;
- expected units active and no unexpected restart loop;
- vitalsd socket health and a non-mutating daemon health response;
- spool fingerprint/byte/count evidence unchanged by the deploy unless a tested fixture explicitly
  exercises a write;
- drainer progress and PostgreSQL readiness where those services are in the target set;
- console/API readiness only for the console target, without treating the console as the safety path;
- no new journal errors attributable to the update during the observation window.

Unknown health is a block, not success. A stale or missing health document names the document and
age. The helper leaves an auditable JSON receipt in a root-owned recovery directory; receipts contain
metadata and metrics, not physiological rows or secrets.

## 4. Dry-run ladder

The ladder intentionally moves from no host mutation to a real, small update:

1. **Runner smoke:** checkout, architecture, toolchain, formatting, and a small decoder test.
2. **Artifact dry run:** build and manifest generation; validate every target without root or service
   changes.
3. **Host staging dry run:** copy to a disposable staging root and run checksum, path, permissions,
   and systemd validation; prove that live paths, units, database, and spool have zero changes before
   and after.
4. **Fault rehearsal:** inject missing artifact, checksum mismatch, interrupted copy, old-release
   health failure, and service-start failure. Each must fail closed or roll back to the old release.
5. **Full rehearsal:** exercise the exact workflow against the Pi with production paths redirected to a
   disposable root, while running backup/restore and spool evidence checks in read-only mode.
6. **First real deployment:** after a fresh encrypted backup and verified restore, choose a minor
   update with a fixed maintenance window, operator present, and a prewritten rollback command.
7. **Observation:** retain the old sync path and release for a defined period, collect fixed evidence,
   then decide whether to retire `sync-to-pi.sh`.

## 5. Backup and first-update gate

Before the first real update, create an encrypted off-Pi recovery set containing the database backup,
backup metadata/checksums, restore-verification result, and all pending spool data or a documented
spool fingerprint/recovery boundary. A copied file is not enough: the existing restore verifier must
produce successful, non-vacuous evidence. The workflow checks the freshness and identity of that
receipt but never receives the encryption key as a GitHub secret.

The first update is intentionally minor and should change one low-risk binary or static asset, not a
schema, collector identity, Bluetooth configuration, or boot configuration. The operator watches the
local health surface and journal, confirms the fixed acceptance commands, and keeps SSH/manual
rollback available. Any unknown state stops the rollout.

## 6. Relationship to `sync-to-pi.sh`

During migration, the script remains the emergency/manual path and is not called by the GitHub
workflow. Existing task `Verify` commands that depend on it must first be converted to use either a
local native build or the new staged helper. Only the final retirement task may delete it or replace
it with a message pointing to the runbook; that task requires a green observation receipt and a
manual rollback rehearsal.
