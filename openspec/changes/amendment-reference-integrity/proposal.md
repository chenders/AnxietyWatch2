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

## Where the therapy-session citation came from

An earlier draft of this proposal said three independent places believed the requirement existed,
and that the consistency suggested a real decision had escaped the index. **That was wrong, and
the history settles it:**

- All three citations were introduced in **one commit** — `2c30fc3`, *feat(air11): integrate ResMed
  AirSense BLE and bedside evidence* (#64). Not three documents agreeing; one belief written three
  times by one author in one sitting.
- At `2c30fc3`, `docs/specs/AMENDMENTS.md` **already listed** `12` as *Provider-aware song search*
  and `13` as *Platform-sample envelope and sub-provenance*. The correct index was in front of the
  author.
- `docs/specs/2026-09-16-amendment-12-provider-aware-song-search.md` was added once, in #52, and
  has **never been renamed**. No renumbering explains the citation, unlike the `10`/`11` collision
  above.

So the citation was wrong when it was written. The remaining question is therefore not *"was this
decided?"* — it was not — but *"should durable therapy-session records exist?"*, which is a
roadmap question. No repo evidence can answer it, because the repository records what was decided
and never what ought to be. That half is the owner's and nothing here is blocked on it: the three
sites now say the requirement is unwritten, the conservative behaviour stands on the accepted
"Unknown is not OK" rule rather than on a missing amendment, and `#64`'s own log shows the therapy
lifecycle work that does exist (`persist therapy lifecycle snapshots`, `invalidate active therapy
across collector restart`).

## What the advisory tier costs, measured

All 18 advisory findings on this tree were triaged by hand against their cited amendments. **All
18 are correct citations.** With three true positives, precision is 3/21 — about **14%**.

The failures are instructive, because they rule out a fix that looks obvious:

- `openspec/changes/admin-console-m0/tasks.md:25` *directly quotes* Amendment 01 — "samples is a
  hypertable, compressed, with a continuous aggregate" — and is still flagged.
- `docs/status-updates/2026-09-18-reboot-handoff.md:95` says "APNs is never used to report phone
  silence to the silent phone" while citing *Phone-silence escalation*.
- `docs/design/SPEC-ios-client.md:347` says "versioned Rust `axum` API required by Amendment 07"
  while citing *The HTTP API and admin console are Rust, not Python*.

The last is the clearest: `rust`, `api` and `axum` are common across amendments, so they are
stripped as non-distinctive and a perfect citation shares nothing measurable with its target. **No
threshold fixes this** — the discriminating information is not present in a bare number plus
surrounding prose.

This is also why the advisory tier is **not** given an `.cargo/audit.toml`-style baseline where
each entry carries a dated reason. That pattern fits a decision to ship with a known risk; these
are false positives of a weak heuristic. Annotating 18 of them would add no information, would tax
every future author, and would not move the question to authoring time — it would only make the
gate resented and eventually bypassed.

## What would prevent recurrence

The triage points at the real fix, and it is a convention rather than an algorithm. Six of the
eighteen citations already follow it:

```
Amendment 06 §2.2: the loss must be visible
Amendment 05 §4.6
Amendment 05 §2.1 permits only SpO₂ and respiratory rate to raise a tier
Amendment 01's "samples is a hypertable, compressed, with a continuous aggregate"
```

Each carries a **section reference or a quoted fragment**, which makes the claim checkable by
whoever reads it, with no heuristic involved. The three therapy-session citations carried neither —
and an author who had to write `§N` would have had to open Amendment 12 to find the section, which
is the moment the error becomes impossible.

Requiring that of requirement-asserting citations is mechanically checkable at near-zero false
positive rate. It is **not proposed here**, because it is a twelve-site migration and that is a
cost decision rather than a correctness one. Recorded so the option is costed rather than
rediscovered.

## Still open

**Should a durable therapy-session record be written as an accepted amendment?** Roadmap, owner's
call, nothing blocked on it. "Unknown is not OK" applies to provenance as much as to readings: a
requirement whose source cannot be found is not thereby satisfied, and not thereby void.
