# Change: pi-github-actions-deployment — controlled GitHub Actions delivery to the Pi

## Why

`../AnxietyWatch` already uses a self-hosted GitHub Actions runner: GitHub-hosted CI builds and
publishes an image, then a repository-scoped runner on the server pulls it, starts Compose, and
checks container health. This repository is different. Its production path is native Rust under
systemd, with BLE, PostgreSQL, a durable COPY spool, and safety-path services that must not depend
on Docker. `scripts/sync-to-pi.sh` currently copies a mutable working tree over SSH and builds in
place; it has no deployment identity, staging boundary, backup gate, health/rollback contract, or
repeatable dry run.

## What changes

Introduce a native ARM64 GitHub Actions runner on the Pi and a staged, fail-closed deployment path:

- a manual runner smoke workflow proves the runner, architecture, pinned Rust toolchain, and test
  execution before it receives any deployment authority;
- CI and the Pi runner build/test the Rust workspace using the repository's toolchain and native
  Linux utilities rather than Docker or macOS cross-build assumptions;
- every deployment is represented by a commit SHA, an artifact manifest, checksums, and a release
  directory; live binaries and service files are switched atomically by a narrow root-owned helper;
- dry-run mode validates the complete artifact, service, permissions, systemd, backup, health, and
  rollback plan without changing binaries, units, database, spool, or service state;
- repeated fault-injection rehearsals establish that a failed preflight or post-restart health
  check leaves the prior release running and names the blocking value;
- the first real update is gated on an off-Pi backup and verified restore evidence, then is a small,
  coordinated update evaluated against fixed post-deploy acceptance evidence;
- `sync-to-pi.sh` remains available until the post-deploy evidence, manual rollback rehearsal, and
  explicit operator decision are recorded; no arbitrary elapsed-time observation period is required.

## Scope

This change covers runner bootstrap, workflow security, native Rust artifact delivery, dry-run and
rollback evidence, backup gating, and retirement of the old sync path. It does not install the
runner on the Pi, create credentials, alter Bluetooth, change PostgreSQL schema, or deploy a real
update as part of this planning branch. Those actions are explicit later tasks requiring the
operator, the Pi, or a backup/rollback decision.

## Hard safety gates

1. The runner is repository-scoped, labelled for this Pi, and runs as an unprivileged dedicated
   account. It has no Docker socket, no broad deploy key, and no production secret.
2. Only trusted `main` pushes or an explicitly approved workflow dispatch may reach deployment; pull
   requests and arbitrary workflow inputs never run on the deployment runner.
3. No root privilege is granted to the runner shell. A root-owned helper accepts only a validated
   manifest from a controlled staging directory and performs an allowlisted operation.
4. A dry run must prove the complete target set. The current repository contains service units that
   refer to binaries not yet present in the workspace; this mismatch is a blocker, not a reason to
   skip targets silently.
5. No real deploy occurs until the encrypted off-Pi backup, restore/readability evidence, spool
   state, and rollback release are recorded.
6. `sync-to-pi.sh` is not removed until the post-deploy evidence set is green, an operator has a
   tested manual rollback, and the operator explicitly decides whether retirement is appropriate.

## Non-goals

- Running life-safety services in containers.
- Making GitHub Actions, GitHub's network, or the Pi's runner a dependency of the alarm at runtime.
- Automatic schema migrations, Bluetooth changes, destructive cleanup, or unattended rollback that
  could hide a data or hardware fault.
- Treating a green process exit, a systemd `active` state, or a copied backup file as proof of
  physiological data safety.
