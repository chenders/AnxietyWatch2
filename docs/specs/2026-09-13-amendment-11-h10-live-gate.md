# Amendment 11 — H10 live gate

**Date:** 2026-09-13  
**Supersedes:** Amendment 10's treatment of H10 as optional for T-A11-6  
**Affects:** T-A11-6 and `scripts/air11-pi-acceptance.sh`

## Decision

The current H10 implementation is installed, active, and producing fresh
validated rows on the deployment. T-A11-6 therefore uses H10 as the selected
independent regression gate in addition to the AirSense and PostgreSQL gates.

The H10 gate is not satisfied by a running systemd process alone. It requires:

- `vitals-h10.service` to be active; and
- at least one fresh H10 row during the bounded window whose type is the
  registry's `hr` or `rr_interval` type and whose value is within that type's
  generated registry range.

EMAY remains optional because its installed service is not currently producing
fresh values, and EzShare remains out of scope because its implementation is not
installed on this deployment. Neither is required by T-A11-6.

## Required verification

T-A11-6's required command is now:

```text
scripts/air11-pi-acceptance.sh --duration 60 --require-postgres --require-h10
```

The H10 value check is evidence that the independent collector is actually
admitting validated measurements, not merely that its process exists.
