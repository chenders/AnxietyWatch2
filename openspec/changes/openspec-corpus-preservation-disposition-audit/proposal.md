# Proposal — OpenSpec corpus preservation and disposition audit

**Status:** Draft — created at the owner's request; no source migration, archival, or acceptance decision is authorized by this artifact.

## Why

The repository contains OpenSpec changes, accepted amendments, architecture and design documents,
research, hardware and integration notes, operations records, reviews, setup logs, and status
updates. Their authority and lifecycle are not consistently explicit. Four active OpenSpec changes
currently fail strict structural validation, while historical task checkboxes and runtime evidence
must not be treated as current acceptance automatically.

Before changing any of these artifacts, the project needs a recoverable, source-backed inventory and
an explicit disposition for each group. Without that baseline, adding specs, setting `skip_specs`,
creating canonical specs, archiving, or moving documents could silently change project authority or
lose evidence.

## What Changes

- Preserve and inventory the in-scope OpenSpec and documentation corpus without rewriting source files.
- Record each change, document group, evidence group, owner decision, lifecycle claim, validation
  result, Verify contract, and unknown/not-measured state.
- Record the current OpenSpec 1.13.2 structural validation result and distinguish it from project
  acceptance, implementation, operational verification, and physical acceptance.
- Classify each group as retain-reference, promote, supersede, defer, archive-after-gate, private,
  or not-audited, with reasons and dependencies.
- Identify contradictions, stale claims, orphaned evidence, missing owners, and the unresolved
  bedside-panel documentation discrepancy.
- Produce owner decision packets and bounded follow-on change recommendations.
- Do not repair existing changes, alter Verify lines, publish canonical specs, archive changes, or
  implement responsive display behavior in this audit.

This is intentionally a documentation/evidence/governance audit with no new observable product
capability; `.openspec.yaml` therefore declares `skip_specs: true`.

## Capabilities

### New Capabilities

None. This is a preservation and disposition audit, not a product-capability change.

### Modified Capabilities

None.

## Impact

- Reads `openspec/`, `docs/`, and selected root-level project guidance within a declared inventory scope.
- Creates a public-safe inventory, preservation manifest, validation findings, discrepancy register,
  and disposition matrix under `docs/knowledge/` or another owner-approved evidence location.
- May add read-only inventory/checking tools in a later task.
- Creates no canonical specs and changes no existing OpenSpec artifact or implementation.
- Depends on `AGENTS.md`, `docs/specs/AMENDMENTS.md`, owner decisions, and the approved OpenSpec
  authoring guidance as authority; it does not supersede them.
