# Tasks — OpenSpec corpus preservation and disposition audit

**Status:** Draft; all tasks are intentionally unchecked. This change does not authorize source
migration, archive, canonicalization, deployment, or implementation.

**Amends:** `docs/specs/AMENDMENTS.md` (mandatory authority index); the owner-approved OpenSpec
authoring guidance under `openspec/changes/repository-recovery-and-lifecycle/reviews/`; no accepted
physiological, storage, alerting, or hardware amendment is changed.

## Task index

- [x] T-OCPA1 — Establish the bounded preservation baseline
- [ ] T-OCPA2 — Inventory changes, amendments, documents, and evidence
- [ ] T-OCPA3 — Record validation, lifecycle, and discrepancy findings
- [ ] T-OCPA4 — Produce disposition and follow-on change packets
- [ ] T-OCPA5 — Review and obtain owner dispositions

## 1. Preservation baseline

### T-OCPA1 — Establish the bounded preservation baseline

**Tier:** implement-complex · **Host:** either · **Context budget:** ~8k · **Depends:** none ·
**Review required:** yes

**Reads:** `AGENTS.md`, `docs/specs/AMENDMENTS.md`, `openspec/config.yaml`,
`openspec/changes/openspec-corpus-preservation-disposition-audit/proposal.md`,
`openspec/changes/openspec-corpus-preservation-disposition-audit/design.md`, and bounded directory
inventories for the declared scope.

**Creates:** a preservation manifest, observation cutoff, source inventory root, and public-safe
handling rules for excluded/private/generated material.

**Modifies:** none of the source corpus.

**Do:** record the current branch/worktree context, source roots, file/group identity, and explicit
excluded or not-audited boundaries. Preserve original files and existing Verify lines verbatim.
Do not copy credentials, private keys, or sensitive contents into the public manifest.

**Verify:** `python3 tools/verify_openspec_preservation.py --manifest docs/knowledge/openspec-preservation-manifest.json`

**Expect:** exit 0 only when every declared root has a manifest entry, the observation cutoff is
recorded, excluded/private boundaries are explicit, and no source file was modified.

## 2. Corpus and evidence inventory

### T-OCPA2 — Inventory changes, amendments, documents, and evidence

**Tier:** implement-complex · **Host:** either · **Context budget:** ~10k · **Depends:** T-OCPA1 ·
**Review required:** yes

**Reads:** the T-OCPA1 manifest; `openspec/changes/`; `openspec/config.yaml`; `docs/specs/`;
`docs/architecture/`; `docs/design/`; `docs/research/`; `docs/hardware/`; `docs/integration/`;
`docs/ops/`; `docs/reviews/`; `docs/setup-logs/`; `docs/status-updates/`; and selected root-level
project guidance named by the manifest.

**Creates:** a sanitized inventory with stable IDs, source paths, category, purpose, authority,
provenance, lifecycle claim, evidence state, owner/reviewer, related change/capability, and
sensitivity class.

**Modifies:** none of the inventoried source documents.

**Do:** inventory OpenSpec changes, amendment records, design/research/hardware/integration/operations
material, reviews, setup logs, status updates, and the bounded display slice. Record the absence of
canonical specs and the distinction between historical evidence and current evidence.

**Verify:** `python3 tools/verify_openspec_inventory.py --manifest docs/knowledge/openspec-preservation-manifest.json --inventory docs/knowledge/openspec-corpus-inventory.json`

**Expect:** exit 0 only when every in-scope group has a stable inventory record or explicit
excluded/not-audited disposition, and sensitive material is not disclosed.

### T-OCPA3 — Record validation, lifecycle, and discrepancy findings

**Tier:** implement-standard · **Host:** either · **Context budget:** ~8k · **Depends:** T-OCPA2 ·
**Review required:** yes

**Reads:** the T-OCPA2 inventory; current `openspec --version`; `openspec doctor --json`;
`openspec list --specs`; `openspec validate --all --strict --no-interactive --json`; all existing
change task files; owner decisions; review records; and evidence references identified by the inventory.

**Creates:** raw validator output, a lifecycle/evidence ledger, and a discrepancy register covering
validation failures, status contradictions, missing owners, stale claims, and the panel-direction
conflict.

**Modifies:** none of the existing changes or source records.

**Do:** keep structural validity, proposal/review/owner acceptance, implementation, exact Verify-passed,
operational verification, historical-only, and not-measured states separate. Record the four current
no-delta failures without deciding whether they should become spec-bearing or `skip_specs: true`.

**Verify:** `python3 tools/verify_openspec_findings.py --inventory docs/knowledge/openspec-corpus-inventory.json --findings docs/knowledge/openspec-findings.json`

**Expect:** exit 0 when every finding has a source, observation time, evidence type, limitation, and
state; no validator result is presented as implementation or operational acceptance.

## 3. Disposition and closure

### T-OCPA4 — Produce disposition and follow-on change packets

**Tier:** review · **Host:** either · **Context budget:** ~8k · **Depends:** T-OCPA3 · **Review required:** yes

**Reads:** the inventory, findings, discrepancy register, lifecycle/evidence ledger, accepted
amendments, owner decisions, and `openspec/changes/project-knowledge-architecture/`.

**Creates:** one disposition packet for every existing OpenSpec change and each major documentation
slice, plus a dependency-ordered list of bounded follow-on proposals.

**Modifies:** none of the source corpus or existing change contracts.

**Do:** propose one of `retain-reference`, `promote`, `supersede`, `defer`, `archive-after-gate`,
`private`, or `not-audited`, with rationale, owner, dependencies, excluded material, and required
review. Include separate candidates for authoring contracts, canonical capability publication,
runtime/durability, bedside display/responsive layout, legacy migration, governance enforcement, and
archive/closure only where evidence justifies them.

**Verify:** `python3 tools/verify_openspec_dispositions.py --findings docs/knowledge/openspec-findings.json --dispositions docs/knowledge/openspec-dispositions.json --follow-ons docs/knowledge/openspec-follow-on-plan.json`

**Expect:** exit 0 only when every in-scope group has exactly one proposed disposition or explicit
owner-decision-needed state, and every promoted obligation has a bounded destination.

### T-OCPA5 — Review and obtain owner dispositions

**Tier:** review/human · **Host:** either · **Context budget:** ~7k · **Depends:** T-OCPA4 ·
**Review required:** yes

**Reads:** all T-OCPA1–T-OCPA4 artifacts; `AGENTS.md`; `docs/specs/AMENDMENTS.md`; OpenSpec
instructions and validation output; reviewer findings; and owner decisions.

**Creates:** complete review/comment dispositions, owner decisions for each change/slice, and an
explicit open or closed state for this audit.

**Modifies:** no source artifact unless a separate owner-authorized follow-on change is opened.

**Do:** run current OpenSpec structural validation, project-specific evidence checks, privacy review,
and required specialist review. Record that `skip_specs: true` is valid only for genuinely spec-free
changes. Do not archive, canonicalize, or repair any existing change as part of this task.

**Verify:** `openspec validate openspec-corpus-preservation-disposition-audit --type change --strict --no-interactive --json`

**Expect:** exit 0 for structural validity only; closure additionally requires reviewed dispositions
and explicit owner decisions, with unknown/unavailable evidence left visible.
