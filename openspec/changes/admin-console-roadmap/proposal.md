# Change: admin-console-roadmap — executable queue for M1 closeout through M6

## Why

`docs/design/SPEC-admin-console.md` is a functional design, not an execution queue. M0 and most of
M1 shipped, but the remaining design had no task headers, scoped `Reads:`, amendment declarations,
dependency order, or shell acceptance contracts. That made the next label (`T-M2b`) inferable but
not executable under `AGENTS.md`.

This change translates every remaining commitment in the design into queued work. It does not alter
behavioural requirements. Where the design leaves a human decision open, the queue creates an
explicit decision task or gate instead of silently choosing. Where substrate does not exist, the
queue names that dependency and puts contract work before UI work.

## What Changes

- Close the M1 gaps and perform the live direction determination required by Amendment 08.
- Queue all M2 operations work, including audited service controls, security S1–S5, silence
  detection, email, optional telemetry, logs, host history, configuration, and milestone acceptance.
- Queue M3 data integrity, scoped export, uploads, quarantine/data-loss readers, and versioned CPAP
  time transforms, while explicitly gating Timescale-dependent work on the unresolved identity
  contradiction.
- Queue M4 jobs, statistics, cost/egress accounting, AI review, and clinical PDF generation.
- Queue M5 versioned domain records and recovery surfaces.
- Queue M6 alarm-channel contracts, evidence, independent tests, and its final screen.
- Queue cross-plan dependencies named by the design: phone health-status beacon, platform-sample
  envelope/provenance, historical backfill, and migration/decommission evidence.
- Provide a traceability table from every normative design section, security control, acceptance
  criterion, migration-risk item, and open question to a task or explicit decision gate.
- Preserve a scoped generation-1 admin behaviour reference so implementation workers can reproduce
  useful operator workflows without treating the Flask application or its defects as a porting
  mandate.

## Impact

- Adds planning and acceptance contracts only; production code and deployment are unchanged.
- Establishes 45 independently dispatchable tasks across M1 closeout, M2–M6, and cross-plan work.
- Records the expected generation-1 admin workflows and their explicit generation-2 owners/drops.
- Makes unresolved operator decisions and architecture contradictions explicit blockers rather than
  implicit assumptions.
- Gives the existing admin-console design an auditable design-to-task traceability ledger.

## Non-goals

- No implementation in this change.
- No early resolution of bind/auth posture, watch topology, SQL-pane inclusion, retention ownership,
  records placement, or phone-silence escalation.
- No claim that blocked M3–M6 work is currently buildable.
- No change to the physiological sample identity or Timescale design; that contradiction requires
  its own amendment decision.

## Execution rules

1. Tasks run in dependency order within a milestone; tasks marked `BLOCKED` do not dispatch until
   their gate is resolved.
2. Every task gets its own worktree, branch, commit, independent review, PR, and conductor-run
   `Verify:` command.
3. Every worker reads `AGENTS.md` and `docs/specs/AMENDMENTS.md` in addition to its declared reads.
4. The Rust amendment governs every HTTP, socket-facing, and console task. Python remains allowed
   only for out-of-process batch parsers and analysis jobs under Amendment 07's carve-out.
5. UI tasks preserve Amendment 08's one-view-model/three-renderer boundary until the operator's
   live direction determination is recorded and the losing templates are deliberately pruned.
6. Missing, malformed, stale, or never-tested evidence fails closed to `UNKNOWN`; no task may invent
   successful evidence to make a screen complete.

## Gates and decisions

| Gate | Blocks | Resolution |
|---|---|---|
| G-M1-DIR live direction choice | template pruning only | T-M1d is operator-judgment work after real use |
| G-AUTH bind/auth posture | state-changing LAN exposure | T-M2a records and implements the decision at point of use |
| G-TS identity vs hypertable uniqueness | Timescale-backed M3 queries/retention | T-M3a creates an amendment; no silent schema workaround |
| G-RETENTION ownership | retention controls in M3 | T-M3b decision task |
| G-RECORDS placement | M5 schema | T-M5a decision task |
| G-PHONE silence escalation | outbound silence notification | T-X1 decision task |
| G-SQL-PANE inclusion | optional SQL surface | T-M3h decision; default is no SQL pane |
| G-ALARM contracts/daemons | M6 UI and controls | T-M6a contract task plus the relevant core-plan tasks |

## Completion

This roadmap is complete when every traceability row is either verified and merged, explicitly
superseded by an accepted amendment, or recorded as operator-deferred with its blocker and owner.
A checked design box without the task's `Verify:` exit 0 is not completion.
