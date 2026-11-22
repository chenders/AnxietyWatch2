# Proposal: Host-neutral names for reusable monitoring software

## Why

AnxietyWatch2 does not require a Raspberry Pi. Reusable helpers, environment variables,
planning directories, and reader-facing labels still use `pi` names inherited from the
prototype deployment. These names obscure the distinction between a Linux monitoring host
and a particular physical board.

The owner requested this follow-up after making the GitHub description, topics, and README
host-neutral. This PR records deferred work; it does not rename executable interfaces,
move existing plans, deploy anything, or claim support on untested platforms.

## What changes later

- Inventory semantic Pi references and their callers before renaming anything.
- Give reusable deployment and acceptance helpers host-neutral canonical names, preserving
  safe compatibility entry points and environment-variable aliases.
- Move active reusable planning/display paths to neutral names with checked references.
- Keep genuinely board-specific configuration and historical evidence clearly identified.

## Non-goals

No blanket replacement of the substring `pi` (for example, `api`, `spike`, and privacy/PII
identifiers are unrelated). No Windows/macOS collector support, hardware deployment, service
activation, schema changes, alarm implementation, or rewriting of accepted amendments.
No removal of Bluetooth, Linux/systemd, architecture, operator-authorization, or artifact
installation safety requirements. No changes to clinical claims or thresholds.

## Impact

Candidate identifiers include `scripts/install-air11-pi-artifact.sh`,
`scripts/air11-pi-acceptance.sh`, `scripts/sync-to-pi.sh`, `PI_HOST`, `PI_PATH`,
`scripts/tests/test-install-pi-artifact.sh`, `openspec/changes/pi5-vitals-core/`, and
`docs/design/pi-panel/`. Workflow and deployment paths require classification: a workflow
that actually targets a specific Raspberry Pi may keep its board-specific name.

The list is a starting point, not an instruction to rename every entry. Task contracts,
CI, hooks, agent rules, and integration guides are part of the reference graph.
