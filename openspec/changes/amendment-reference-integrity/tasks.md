# Tasks: Make amendment citations resolve

Acceptance is each `Verify:` command's exit code. Do not tick a box before running its command.

- [x] T-ARI-1 — Gate amendment citations on resolvability · implement-standard · host
- [x] T-ARI-2 — Make pre-push fail on anything CI fails · implement-simple · host
- [x] T-ARI-3 — Stop attributing the therapy-session requirement to Amendment 12 · docs

## T-ARI-1 — Gate amendment citations on resolvability

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `docs/specs/AMENDMENTS.md`, `proposal.md`,
`specs/amendment-reference-integrity/spec.md`, `.githooks/pre-push` (the public-info gate, for the
interpreter-resolution convention), `.github/workflows/ci.yml` (the `public-info` job), and
`scripts/check-public-info.py` (for the `--self-test` convention).

**Modifies:** `scripts/check-amendment-refs.py` (new), `.githooks/pre-push`,
`.github/workflows/ci.yml`, `.github/instructions/swift.instructions.md`,
`docs/design/SPEC-ios-client.md`, `docs/status-updates/2026-09-13T05-20-57Z.md`,
`docs/status-updates/2026-09-13T05-40-43Z.md`.

**Do:** Add a checker with two tiers. The blocking tier rejects a citation naming a number absent
from the index, and a bare citation of a number that a suffixed sibling makes ambiguous — accepting
a link to the amendment's file, its slug, or two of its title words as identification. The advisory
tier reports requirement-asserting citations that share fewer than two distinctive words with their
target, and never changes the exit status.

Compute distinctiveness as "in the amendment's own title, or appearing in no other amendment."
Do **not** use a fraction-of-corpus threshold: that is not scale-invariant, and it fails silently
in the direction that matters. At three amendments it excludes any word in two of them, but at
twenty it only excludes words in eleven — so generic words survive, and two of them were enough to
pass the `air11.rs` citation this gate exists for. A three-amendment fixture cannot reproduce that,
so the self-test checks distinctiveness against a corpus of eight.

Then disambiguate the six citations the blocking tier reports. Four mean the main series and need
only their title named. Two are pointing at the wrong document — the 2026-09-13 status updates mean
`10-A11` and `11-A11`, which were called `10` and `11` on the day they were written. Correct the
identifier and say why it changed, so the record explains itself rather than appearing to have
always said this.

**Verify:** `scripts/check-amendment-refs.py --self-test && scripts/check-amendment-refs.py && grep -q 'check-amendment-refs' .githooks/pre-push && grep -q 'check-amendment-refs' .github/workflows/ci.yml`

## T-ARI-2 — Make pre-push fail on anything CI fails

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `.githooks/pre-push`, `.github/workflows/ci.yml`.

**Modifies:** `.githooks/pre-push`.

**Do:** Add `cargo fmt --all --check` beside the clippy gate. CI gates on it and the hook did not,
so on 2026-11-06 two branches pushed with `✓ all gates passed` while rustfmt-dirty and would have
failed CI. Record the incident in the comment: a hook that announces a verdict CI disagrees with is
worse than no hook, because it is believed.

**Verify:** `grep -q 'cargo fmt --all --check' .githooks/pre-push && bash -n .githooks/pre-push && cargo fmt --all --check`

## T-ARI-3 — Stop attributing the therapy-session requirement to Amendment 12

**Depends:** T-ARI-1 · **Review required:** yes

**Amends:** none. Amendment 12 is *Provider-aware song search and provider-independent identity*
and does not touch therapy sessions; that is the defect, not an amendment that governs this task.

**Reads:** `docs/specs/AMENDMENTS.md`,
`docs/specs/2026-09-16-amendment-12-provider-aware-song-search.md`,
`crates/vitals-console/src/api/air11.rs` (the `air11_history_handler` doc comment),
`docs/integration/resmed-air11-ble-stream.md`,
`docs/reviews/2026-09-16-admin-site-playwright-exploration.md`.

**Do:** Three places name Amendment 12 as the source of a durable therapy-session-record
requirement. No accepted amendment specifies one, and `openspec/specs` has no mention of it either.
Remove the false attribution without inventing the requirement: say that no accepted amendment
specifies it, and point at this change's open question.

Leave the behaviour alone. The history endpoint's refusal to finalize a session, and the
collector's provisional boundary, are both correct and stay exactly as they are — what was wrong
was the authority cited for them, not the conduct. Deleting a false citation does not require
knowing what the true one should be, which is why this task can close while the open question
cannot.

**Verify:** `scripts/check-amendment-refs.py && ! scripts/check-amendment-refs.py 2>&1 | grep -q 'Amendment 12 — asserts'`

> **This Verify line was changed during the task, deliberately and as its own decision.** The
> first version grepped for the literal strings `required by Amendment 12` and
> `Amendment 12 therapy`. It failed — not because the attribution survived, but because the
> correction note explaining the old claim contains the words it searched for. A substring search
> cannot tell an assertion from a text discussing that assertion, so it would have forced the
> correction to be written obscurely, or silently to be left out.
>
> The replacement asserts the property instead of the spelling: Amendment 12 no longer appears in
> the checker's advisory list, which is precisely "no citation attributes a requirement to
> Amendment 12." It is strictly stronger — it covers phrasings the grep would have missed — and it
> is checked by the same tool the rest of this change ships. Recorded here rather than quietly
> edited, because a Verify line is a contract and this is the second time in this repository a
> Verify tested a spelling rather than a behaviour.
