# Design — OpenSpec corpus preservation and disposition audit

## Context

OpenSpec 1.13.2 validates structure and content, but it does not natively represent owner
acceptance, implementation, operational verification, regression, physical acceptance, or
not-measured evidence. AnxietyWatch2 adds stricter project contracts: exact task Verify exit codes,
scoped Reads and Amends, accepted amendments, review gates, and explicit unknown states.

The current repository has no canonical `openspec/specs/` capability tree. Four active changes fail
strict validation because they have no recognized spec delta; the latest CLI also supports
`skip_specs: true` for genuinely spec-free changes. That option must be applied only after reviewing
a change's intent, not as a blanket way to silence findings.

This audit is the safe first step before migration. It records facts and proposed dispositions while
leaving the source corpus authoritative and unchanged.

## Goals / Non-Goals

**Goals:**

- Establish a recoverable inventory of OpenSpec changes, amendments, knowledge directories, evidence,
  decisions, and selected root-level guidance.
- Record structural validation, task checkbox, Verify, review, owner, implementation, operational,
  hardware, and physical-display evidence separately.
- Preserve contradictions, historical claims, dissent, sensitive-material boundaries, and unavailable
  measurements.
- Identify which material should remain reference evidence and which adopted obligations warrant a
  bounded follow-on OpenSpec change.
- Produce explicit owner decision packets for every existing change and major document group.
- Identify the responsive bedside-display work as a later capability change rather than implementing
  it here.

**Non-Goals:**

- Adding missing delta specs to existing changes.
- Setting `skip_specs: true` on any existing change without an explicit disposition.
- Editing Verify commands, task checkboxes, amendment statuses, owner decisions, or historical evidence.
- Creating canonical specs under `openspec/specs/`.
- Archiving, deleting, moving, or renaming source documents.
- Implementing code, display layout, hardware integration, governance enforcement, or deployment.
- Inspecting or copying secrets/private material into public inventory artifacts.

## Decisions

### 1. Use a bounded source inventory

The initial scope includes:

- all `openspec/changes/*` artifacts and metadata;
- `openspec/config.yaml` and the absence/presence of canonical `openspec/specs/`;
- `docs/specs/` and accepted/draft amendment records;
- `docs/architecture/`, `docs/design/`, `docs/research/`, `docs/hardware/`,
  `docs/integration/`, `docs/ops/`, `docs/reviews/`, `docs/setup-logs/`, and
  `docs/status-updates/`;
- selected root-level guidance such as `AGENTS.md`, `README.md`, and project kickoff/instruction files;
- a declared list of excluded, private, generated, cache, or not-audited locations.

The inventory records groups and provenance; it does not require every binary, cache, or private file
to be copied or disclosed.

### 2. Keep lifecycle and evidence orthogonal

The audit records lifecycle claims such as `proposed`, `reviewed`, `owner-accepted`, `implemented`,
`Verify-passed`, `operationally verified`, and `superseded`. It records `not-measured` as an
orthogonal evidence state. A historical successful capture may coexist with a current unknown or
failed state.

### 3. Treat current CLI output as structural evidence

The audit stores the exact OpenSpec version, command, exit code, JSON findings, and timestamp. A
validation pass or failure does not determine authority, implementation, or operational success.

### 4. Preserve source authority and contradictions

`docs/specs/AMENDMENTS.md`, AGENTS rules, and owner decisions remain authoritative. Conflicts are
recorded with both source paths and no inferred resolution. In particular, the contradictory claims
about whether the panel's Penumbra and Semaphore directions are built remain an explicit discrepancy
until separately resolved.

### 5. One follow-on change per coherent intent

The disposition matrix may recommend separate changes for authoring contracts, canonical capability
publication, runtime/durability, bedside display/responsive layout, legacy migrations, governance
enforcement, or archive/closure. The audit does not combine these concerns or create them silently.

## Evidence record

Each claim should identify:

`claim_id`, `source_path`, `artifact_or_requirement`, `lifecycle_state`, `evidence_state`,
`evidence_type`, `command_or_observation`, `observed_at_utc`, `exit_code_or_result`, `reviewer`,
`owner_decision`, `limitations`, and `follow_on_change`.

The evidence state must support at least `measured-good`, `measured-bad`, `not-measured`, and
`historical-only`. Missing evidence must not be coerced into a pass or fail.

## Risks / Trade-offs

- **Audit becomes a second undocumented system** → keep it simple, source-linked, reviewable, and
  bounded; use stable IDs and an explicit owner.
- **Sensitive material leaks into public artifacts** → record handling class and controlled identifier,
  not secret contents.
- **Validation repair is mistaken for progress** → preserve the raw validator result and defer every
  transformation to an owner disposition.
- **Scope expands indefinitely** → require an explicit excluded/not-audited list and route each domain
  to a separate follow-on change.
- **Historical evidence is mistaken for current truth** → store observation time and current status
  separately, including regressions and unavailable evidence.
