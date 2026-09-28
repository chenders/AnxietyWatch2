# Amendment 18 — Deployment acceptance is evidence-based, not time-based

**Date:** 2026-09-28

**Status:** Accepted by operator

**Supersedes:** The undefined “observation period”, “defined period”, and “agreed observation
period” language in the Pi GitHub Actions deployment proposal, design, tasks, traceability, and
observation report. It does not weaken the backup, rollback, health, spool, or owner-approval gates.

**Affects:** `openspec/changes/pi-github-actions-deployment/` proposal, design, tasks, and
traceability; `docs/deploy/pi-github-actions-observation.md`.

## Decision

There is no mandatory elapsed-time observation period and no arbitrary required count of subsequent
main deliveries. Time alone is not evidence of deployment safety, and the existing plan did not
specify a duration or a meaningful stopping rule.

T-R8 is instead an evidence-and-decision task. It is complete only when the repository contains:

1. the green T-R7 activation receipt;
2. a read-only post-deploy snapshot taken after a scheduled idle/spool-seal cycle, covering the
   release identity, runner, database, vitalsd socket, service states, drainer progress, and spool
   evidence;
3. a manual rollback rehearsal performed without GitHub Actions, with its command and exit result
   recorded; and
4. an explicit operator decision to retain `scripts/sync-to-pi.sh` as an emergency path or retire it.

The old release remains installed until the rollback decision is recorded. A later main delivery is
valuable operational evidence but is not a substitute for the required receipt, snapshot, rollback
rehearsal, or operator decision. If new evidence is collected after T-R8, it is recorded as a normal
operational update rather than waiting for an arbitrary clock threshold.

## Required wording changes

- Replace “observation period”, “observation window”, and “defined/agreed period” with
  “post-deploy evidence set” wherever they describe a deployment acceptance gate.
- Keep “observation” as the name of the evidence activity; it does not imply a minimum duration.
- Do not retire the emergency sync path merely because a clock elapsed. Retirement still requires the
  explicit operator decision and the manual rollback evidence above.

## Rationale

A fixed duration was never selected, and a duration without a stated failure criterion would make a
passing clock indistinguishable from a passing deployment. The release receipt, data-path snapshot,
rollback exit code, and explicit operator decision are auditable facts and provide a finite gate
without pretending that an arbitrary wait proves safety.
