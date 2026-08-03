---
description: Loop responding to GitHub Copilot review comments on a PR until none remain
---

# respond-to-copilot

Work through Copilot's review comments on a PR until the bot has nothing further. Every task in this
project ships through this cycle.

**Usage:** `/respond-to-copilot [PR#]` — with no argument, uses the PR for the current branch.

## The cycle

```
push → Copilot reviews → address every comment → push → re-request → repeat until clean
```

## Steps

**1. Find the PR and read the review.**

```sh
PR=${1:-$(gh pr view --json number --jq .number)}
gh pr view $PR --json title,headRefName,url
gh pr view $PR --comments
gh api repos/{owner}/{repo}/pulls/$PR/comments --jq '.[] | "\(.path):\(.line)\n\(.body)\n---"'
```

**2. Triage each comment into one of three buckets. Do not skip this.**

| Bucket | Meaning | Action |
|---|---|---|
| **Correct** | A real defect or a genuine improvement | Fix it |
| **Wrong** | Copilot misread the code or the context | Reply explaining why, do not change code |
| **Out of scope** | Valid, but belongs to a different task | Reply, and record it in `openspec/changes/*/tasks.md` under `## Implementation notes` |

A comment you disagree with still needs a reply. Silence reads as agreement and the next reviewer
re-raises it.

**3. Weigh findings against project rules, not general taste.**

`.github/copilot-instructions.md` and `.github/instructions/*.instructions.md` are the standard.
Where Copilot's generic advice conflicts with a documented invariant here, the invariant wins — say
so in the reply and cite the rule. Examples that recur:

- Copilot suggests clamping an out-of-range value. **No** — this project quarantines with a reason,
  because a clamped reading is indistinguishable from a real one downstream.
- Copilot suggests defaulting a missing value to `0` or "OK". **No** — unknown is never OK here.
- Copilot suggests adding a mutex to the daemon. **No** — `vitalsd` is single-writer,
  single-threaded by design; that is why there is no locking.
- Copilot suggests a mock so a Linux-only path builds on macOS. **No** — a mocked BlueZ tests the
  mock. That code runs on the Pi.

**4. Fix, verify, commit.**

Address findings in one commit per theme rather than one per comment. Then run the task's own
`Verify` command — **unmodified** — plus the suite:

```sh
cmake --build build && ctest --test-dir build --output-on-failure
```

Never edit a `Verify:` line in `tasks.md` to make it pass. The pre-push hook blocks that, and it is
blocking a real failure mode: it has already happened once here.

**5. Reply to each thread.**

```sh
gh api repos/{owner}/{repo}/pulls/$PR/comments/<comment_id>/replies \
  -f body="Fixed in <sha>. <one line on what changed and why>"
```

For rejections, give the reason and the rule:

> Not changing this. `vitalsd` is single-writer by design — see
> `.github/instructions/c.instructions.md` § Concurrency. Adding a mutex would suggest concurrent
> writers are supported, which they are not.

**6. Push and re-request.**

```sh
git push
gh pr comment $PR --body "@copilot review"
```

**7. Repeat** until a review round produces no actionable comments.

## Stopping

Stop when Copilot's latest review has no actionable findings. If a round produces only comments you
have already rejected with reasons, stop and say so — do not loop.

If a round takes more than three iterations, that usually means the PR is too large or the task was
mis-scoped. Say that rather than grinding.

## After the loop

- Update `- [ ]` → `- [x]` in `openspec/changes/*/tasks.md` **only** after running that task's
  Verify command yourself and seeing exit 0.
- Record scope deltas and deferred findings under `## Implementation notes`.
- Merge with `gh pr merge --squash --delete-branch`, then remove the worktree:
  `git worktree remove ../aw2-<task>`.
