# Proposal: GitHub runner activation write boundary

**Status:** Proposed; created after the first protected activation attempt failed closed.

## Why

The repository-scoped `github-vitals` runner is intentionally sandboxed with
`ProtectSystem=full` and may invoke only the root-owned `vitals-deploy` helper. The first
protected activation reached that helper but failed with `Read-only file system` before
installing the staged release. The sandbox currently makes the helper's documented, allowlisted
release and service targets read-only even after the narrow sudo elevation.

## What changes

- Grant the runner service namespace write access only to the helper's existing deployment target
  directories: the immutable release tree, selected binaries/assets, and systemd/udev unit trees.
- Keep the runner account unprivileged and preserve the existing single-command sudoers rule.
- Verify that the runner can execute the helper's protected activation path without gaining a shell,
  Docker socket, database access, spool access, or arbitrary root writes.
- Preserve the fail-closed backup, spool-identity, manifest, health, and rollback gates.

## Non-goals

This change does not alter artifact allowlists, deployment ordering, backup semantics, spool
identity, PostgreSQL, Bluetooth, `/boot/firmware`, or the GitHub workflow trust boundary.

## Capabilities

### New Capabilities

- `github-runner-activation-boundary`: a sandbox-compatible write boundary for the existing
  root-owned deployment helper.

## Impact

The checked-in runner unit changes, plus a bounded Pi installation/restart and protected activation
verification. No application runtime code changes are expected.
