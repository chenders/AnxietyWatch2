# OpenSpec corpus audit review record

**Change:** `openspec-corpus-preservation-disposition-audit`
**Task:** T-OCPA5
**State:** OPEN — owner dispositions required; not closed, archived, canonicalized, or accepted.

## Evidence completed

The audit implementation produced and verified:

- T-OCPA1 preservation manifest: exit 0
- T-OCPA2 metadata-only corpus inventory: exit 0; 320 records
- T-OCPA3 validation/lifecycle/discrepancy findings: exit 0; 8 changes and 8 discrepancies
- T-OCPA4 disposition packets/follow-on plan: exit 0; 17 groups and 6 bounded follow-ons
- Audit change structural validation under OpenSpec 1.13.2: exit 0

The repository-wide structural validation remains a separate finding:

```text
openspec validate --all --strict --no-interactive --json --report full
exit 1
```

Three changes pass and four changes fail for absent recognized spec deltas. This is structural
validator evidence only; it is not an implementation, approval, or safety verdict.

## Review dispositions

| Review input | Disposition | Consequence |
|---|---|---|
| Project expert inventory | Accepted into T-OCPA1–T-OCPA4 | Preserve authority, provenance, unknown state, and bounded scope |
| OpenSpec methodology review | Accepted into T-OCPA3/T-OCPA4 | Keep structural validity separate from lifecycle and operational evidence |
| Panel-direction contradiction | Open discrepancy | Do not rewrite either source; resolve in a later owner-approved reconciliation |
| Four no-delta validation failures | Open disposition | Do not add specs or set `skip_specs` automatically |
| OpenSpec 1.6.0 guidance vs installed 1.13.2 | Version-drift finding | Re-probe version-specific guidance before any update; do not rewrite silently |
| Private/generated/untracked boundaries | Explicitly excluded or not-audited | No sensitive contents copied into public inventory |

No independent specialist review has been claimed. The bedside-display and hardware acceptance
questions remain outside this audit's evidence and are `not-measured`.

## Owner decisions required

1. For each existing OpenSpec change, choose retain, promote, supersede, defer, archive-after-gate,
   private, or another explicit disposition:
   - `99-last-specs`
   - `emay-durable-ingestion`
   - `pi5-vitals-core`
   - `project-knowledge-architecture`
   - `repository-recovery-and-lifecycle`
   - `safety-hardening`
   - `xe125-respiratory-rate`
   - `openspec-corpus-preservation-disposition-audit`
2. Decide whether the four no-delta changes are genuinely spec-free (`skip_specs: true`) or require
   real capability deltas/companion changes.
3. Decide whether the audit should lead to canonical specs after disposition.
4. Approve or revise the lifecycle/evidence vocabulary used by the audit.
5. Resolve whether the panel README is stale or whether the implementation claim is overstated.
6. Decide whether OpenSpec 1.13.2 becomes the documented project baseline.
7. Approve, reject, or defer each proposed follow-on change in
   `docs/knowledge/openspec-follow-on-plan.json`.

Until these decisions are recorded, the audit remains open and its task checkbox must remain
unchecked.

## Closure rule

Closure requires the exact T-OCPA5 Verify command to exit 0, complete reviewer comment dispositions,
explicit owner decisions for every group, and a statement that no source artifact was modified by
the audit. Owner decisions do not authorize implementation, archive, deployment, or canonicalization
unless they say so explicitly and the applicable follow-on gates pass.
