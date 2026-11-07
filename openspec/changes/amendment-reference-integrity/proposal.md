# Make amendment citations resolve

## Why

`AGENTS.md` makes `docs/specs/AMENDMENTS.md` mandatory reading for every task, and says plainly
that a task brief contradicting an accepted amendment is wrong — the amendment wins. The rule
exists because four amendments once changed the implementation language, the storage engine, the
schema and the primary key while every task brief still described the superseded system.

That rule assumes one thing it never states: that following a citation lands the reader on the
decision the citing text describes. Two things in this repository break that assumption, and they
are different problems needing different treatment.

### The index carries two parallel numbering series

`10-A11` (AirSense acceptance scope) and `11-A11` (H10 live gate) were written on 2026-09-13 and
are **still named `amendment-10-...` and `amendment-11-...` on disk**. Two days later the main
series created its own Amendment 10 (first-draft private-LAN transport), and on 2026-09-16 its own
Amendment 11 (optional direct lyrics). The index disambiguated the earlier pair with an `-A11`
suffix; the text citing them was never updated.

So `docs/status-updates/2026-09-13T05-20-57Z.md` says *"Added Amendment 10: AirSense live
acceptance gates only the AirSense path"* — and a reader following that citation today arrives at a
document about LAN HTTP transport. Ten citations of a colliding number exist; six of them give the
reader nothing to disambiguate with, and two of those six name the wrong document outright.

This is structural and decidable without judgement. It is what the new gate blocks on.

### A citation can also attribute a requirement its target does not contain

Three files cite "Amendment 12" as the source of a durable therapy-session-record requirement.
Amendment 12 is *Provider-aware song search and provider-independent identity*; it contains the
word "therapy" **zero times**, and no accepted amendment specifies a therapy-session record at
all — `openspec/specs` has no mention of one either.

The consequential one is `crates/vitals-console/src/api/air11.rs`, a `///` doc comment telling a
future worker that `/api/air11/history` "must not pretend that a transport or time window is a
finalized therapy session" *because* Amendment 12 "does not yet provide a durable therapy-session
record." A worker obeying the mandatory-reading rule would read Amendment 12, find song search,
find no such constraint, and could reasonably conclude the guard was stale and remove it. The
guard is correct; its stated authority is not.

### Why the obvious gates do not work

Worth recording, because each was tried and measured against the real tree:

| Rule | Findings | Verdict |
|---|---|---|
| Does the cited amendment exist? | 0 | Passes the defect. Amendment 12 exists and is indexed. |
| Must every citation carry a title or link? | 111 | A migration, not a gate. A bare number is the house style for ordinary cross-references, and a gate demanding 111 rewrites gets turned off. |
| Must a requirement-asserting citation share distinctive vocabulary with its target? | 25 | Mostly terse-but-correct. A `**Amends:** Amendment 05` line has almost no context words, so it can never reach any threshold. Vocabulary cannot separate terse from wrong. |
| Must a bare citation of a *colliding* number say which it means? | 6 | Bounded, decidable, zero judgement. |

The vocabulary test ships as **advisory**: reported, never gating. Calling a heuristic with known
false positives a gate means either editing correct text to satisfy it or switching it off. Its job
is to put candidates in front of a human, and it earns its place by having found all three
therapy-session citations and two wrong-series ones.

### A gate that announced a verdict CI disagreed with

Separately, and found while shipping this: `.githooks/pre-push` does not run
`cargo fmt --all --check`, which `.github/workflows/ci.yml` gates on. On 2026-11-06 two branches
pushed with `✓ all gates passed` while rustfmt-dirty, and both would have failed CI. A hook that
announces a verdict CI disagrees with is worse than no hook, because it is believed.

## What Changes

- **ADD** `scripts/check-amendment-refs.py`, a two-tier checker with a `--self-test`:
  - blocking: a citation naming an unindexed number, or a bare citation of a number that a
    suffixed sibling makes ambiguous
  - advisory: a citation asserting a requirement its target may not discuss; never gating
- **ADD** both tiers to `.githooks/pre-push` and `.github/workflows/ci.yml`
- **ADD** `cargo fmt --all --check` to `.githooks/pre-push`, closing the gap against CI
- **MODIFY** six ambiguous citations to name which amendment they mean, two of which were pointing
  at the wrong document
- **MODIFY** the three citations attributing a therapy-session requirement to Amendment 12, so they
  state that no accepted amendment specifies it rather than naming one that does not

## Impact

- Affected specs: `amendment-reference-integrity` (new)
- Affected code: `scripts/`, `.githooks/pre-push`, `.github/workflows/ci.yml`,
  `crates/vitals-console/src/api/air11.rs` (doc comment only), six documents
- No runtime behaviour changes. The `air11.rs` edit is a `///` comment; the guard it describes is
  unchanged, because the guard was right.

## Open question for the owner

**Should a durable therapy-session record be written as an accepted amendment?**

This change removes the false attribution but deliberately does not invent the requirement. Three
independent places believed such a requirement existed and shaped behaviour around it — the history
endpoint refuses to finalize sessions, the BLE stream collector keeps its boundary provisional, and
the admin console review treats session inspection as an unmet obligation. That consistency
suggests a real decision was made and never written down, but which is true — a decision that
escaped the index, or three documents that drifted into agreement — is not determinable from the
tree. It needs the owner's intent.

Until then the three sites say the requirement is unwritten, which is accurate and leaves the
conservative behaviour in place. "Unknown is not OK" applies to provenance as much as to readings:
a requirement whose source cannot be found is not thereby satisfied, and it is not thereby void.
