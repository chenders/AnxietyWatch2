# Amendment 10 — AirSense acceptance scope

**Date:** 2026-09-13  
**Supersedes:** the unrelated host-health portion of T-A11-6's live acceptance gate  
**Affects:** T-A11-6 and `scripts/air11-pi-acceptance.sh`

## Decision

The AirSense live acceptance gate proves the AirSense path itself:

- operator authorization and private credential/target validation;
- authenticated streaming and measured status;
- host receipt and device-clock provenance;
- AirSense source/type rows through vitalsd into PostgreSQL; and
- bounded reconnect/restart behavior.

H10, EMAY, and EzShare are separate integrations. They are not prerequisites for
accepting AirSense and must not be required by the AirSense task's Verify command.
EzShare is a separate store-and-forward fallback, and its unit is not present on
the deployment covered by this change. The AirSense collector must not modify
those integrations, but their health is not evidence required to accept this
collector.

The acceptance harness may retain explicit optional host-health flags for a
separate operator-requested regression run when the corresponding implementation
is installed and active. Optional flags are not part of T-A11-6's required gate.

## Required verification

T-A11-6's required command is:

```text
scripts/air11-pi-acceptance.sh --duration 60 --require-postgres
```

A run using `--require-h10`, `--require-emay`, or
`--require-ezshare-unchanged` is additional cross-integration evidence, not an
AirSense acceptance prerequisite.
