# Tasks: History PII/PHI purge

**Scheduled by `public-release-readiness`** (2026-11-10): T-HP-3…T-HP-9 run as its T-PR-6…T-PR-13, which also correct T-HP-5 — the `main` ruleset has no `non_fast_forward` rule, and its `required_signatures` rule rejects the unsigned rewritten commits.

Strictly ordered: each task depends on the one before it. `$PURGE` means
`$HOME/Source/aw2-history-purge` — the out-of-repo working directory that holds the manifest,
the backup and the rewritten mirror. **Nothing under `$PURGE` is ever copied into the
repository**: it contains the values being removed.

- [x] T-HP-1 — Merge PR #105 so the tip is clean · integrate · host
- [ ] T-HP-2 — Build the purge manifest from both audits · investigate · host
- [ ] T-HP-3 — Back up every ref before any rewrite · ops · host
- [ ] T-HP-4 — Rewrite a full mirror and prove the result · implement · host
- [ ] T-HP-5 — Force-push the rewritten history · ops · host · **owner action**
- [ ] T-HP-6 — Move the local repository and worktrees onto the new history · ops · host
- [ ] T-HP-7 — Re-clone the Pi's checkout · ops · pi
- [ ] T-HP-8 — Ask GitHub to purge cached pull-request refs · ops · **owner action**
- [ ] T-HP-9 — Gate every commit a PR introduces, not only its tip · implement · host

---

### T-HP-1 — Merge PR #105 so the tip is clean

**Depends:** none · **Review required:** done (local reviews recorded in the PR)

**Amends:** none.

**Reads:** PR #105.

**Do:** Wait for required checks, merge with a merge commit. The rewrite in T-HP-4 must start
from a tip that already passes every content and path gate, so that "main's tree is unchanged
by the rewrite" is a meaningful check.

**Verify:**
```sh
git fetch -q origin && test -z "$(git ls-tree -r --name-only origin/main -- firmware xe125 aw2-ios-thin-client-chatgpt-context.zip PI-KICKOFF.md)" && git cat-file -e origin/main:scripts/check-tracked-paths.py
```

**Context budget:** 1 PR.

---

### T-HP-2 — Build the purge manifest from both audits

**Depends:** T-HP-1 · **Review required:** yes

**Amends:** none.

**Reads:** `scripts/check-public-info.py` (rules), the history audit report.

**Do:** Write to `$PURGE/manifest/`:
- `paths.txt` — `git filter-repo --paths-from-file` input (with `--invert-paths`): every path
  whose whole history must go (root screenshots and recording, the snapshot ZIP, session
  prompts, the PDF export, `firmware/`, investigation dumps, local-only research packages,
  and any location-revealing tooling the audit identifies).
- `replace.txt` — `--replace-text` input: each identifier and reading the scanners and the
  audit found, mapped to the **same** placeholder or substitute the cleaned tree already uses,
  so that the rewrite does not change `main`'s final tree.

Every entry traces to a scanner finding or an audit finding. Nothing is removed on a guess.

**Verify:**
```sh
test -s "$HOME/Source/aw2-history-purge/manifest/paths.txt" && test -s "$HOME/Source/aw2-history-purge/manifest/replace.txt" && ! git ls-files --error-unmatch "$HOME/Source/aw2-history-purge" 2>/dev/null
```

**Context budget:** 2 files written, outside the repository.

---

### T-HP-3 — Back up every ref before any rewrite

**Depends:** T-HP-2 · **Review required:** no

**Amends:** none.

**Reads:** none.

**Do:** `git bundle create $PURGE/backup/pre-purge.bundle --all` from the main checkout, and
save `git stash list` and `git worktree list --porcelain` beside it. This is the only copy of
the pre-rewrite history after T-HP-6; it stays on this machine and is never pushed.

**Verify:**
```sh
git bundle verify -q "$HOME/Source/aw2-history-purge/backup/pre-purge.bundle" && test "$(git bundle list-heads "$HOME/Source/aw2-history-purge/backup/pre-purge.bundle" | wc -l)" -ge "$(git for-each-ref refs/heads refs/remotes refs/tags | wc -l)"
```

**Context budget:** none.

---

### T-HP-4 — Rewrite a full mirror and prove the result

**Depends:** T-HP-3 · **Review required:** yes

**Amends:** none.

**Reads:** `$PURGE/manifest/*`.

**Do:** Clone the main checkout with `--mirror` into `$PURGE/mirror.git` (all local branches,
remote-tracking branches and tags), run `git filter-repo --paths-from-file paths.txt
--invert-paths --replace-text replace.txt` once, then prove, in `$PURGE/verify.sh`:
1. every blob in every ref passes `check-public-info.py`'s rules and no blob contains any
   `replace.txt` source string;
2. no commit in any ref contains any `paths.txt` path;
3. the rewritten `main` tree hash equals the pre-rewrite `origin/main` tree hash (the tip was
   already clean, so the rewrite must not change it);
4. branch, tag and commit counts match, apart from commits that become empty and are pruned.

**Verify:**
```sh
bash "$HOME/Source/aw2-history-purge/verify.sh"
```

**Context budget:** the mirror.

---

### T-HP-5 — Force-push the rewritten history

**Depends:** T-HP-4 · **Review required:** yes · **Owner action**

**Amends:** none.

**Reads:** the `main` ruleset (`gh api repos/{owner}/{repo}/rules/branches/main`).

**Do:** The owner disables the ruleset's `non_fast_forward` (and, if needed, `deletion`) rule.
Force-push every rewritten branch that exists on GitHub, and delete from GitHub any branch the
rewrite emptied. The owner restores the rule immediately after. No open PR may exist during
the push: its base would be rewritten under it.

**Verify:** every GitHub branch head equals the mirror's rewritten copy of it (the mirror's
`refs/remotes/origin/<b>`; its local-only branches are never pushed), and the force-push block
is back:
```sh
git ls-remote --heads origin | while read -r sha ref; do test "$(git --git-dir="$HOME/Source/aw2-history-purge/mirror.git" rev-parse "refs/remotes/origin/${ref#refs/heads/}")" = "$sha" || { echo "mismatch: $ref"; exit 1; }; done && gh api repos/chenders/AnxietyWatch2/rules/branches/main --jq '.[].type' | grep -qx non_fast_forward
```

**Context budget:** none.

---

### T-HP-6 — Move the local repository and worktrees onto the new history

**Depends:** T-HP-5 · **Review required:** no

**Amends:** none.

**Reads:** `$PURGE/backup/` (stash and worktree lists).

**Do:** Replace the main checkout's refs with the mirror's (local branches with unpushed work
come across rewritten, so nothing is lost), re-create each worktree on its rewritten branch,
re-apply stashes from the backup bundle where they still apply, then expire the reflog and
`git gc --prune=now` so no old object remains locally. The backup bundle is the record of
anything that does not carry over.

**Verify:**
```sh
test -z "$(git rev-list --all -- health.jpg healthkit.jpg aw2-ios-thin-client-chatgpt-context.zip firmware)" && test -z "$(git log --all --format=%h -G 'readings are real, not syntheti[c]')"
```

**Context budget:** none.

---

### T-HP-7 — Re-clone the Pi's checkout

**Depends:** T-HP-5 · **Review required:** no

**Amends:** none.

**Reads:** `scripts/sync-to-pi.sh`, `deploy/github-runner/README.md`.

**Do:** Any clone on the Pi (deploy-key checkout, runner workspace) still holds the old
history. Re-clone it; do not fetch into it. Confirm services still start from the new checkout.

**Verify:**
```sh
ssh "$PI_HOST" 'cd "$PI_PATH" && test -z "$(git rev-list --all -- health.jpg aw2-ios-thin-client-chatgpt-context.zip firmware)"'
```

**Context budget:** none.

---

### T-HP-8 — Ask GitHub to purge cached pull-request refs

**Depends:** T-HP-5 · **Review required:** no · **Owner action**

**Amends:** none.

**Reads:** GitHub's documentation on removing sensitive data.

**Do:** GitHub keeps `refs/pull/*` and cached views of every PR, which a force-push cannot
change. The owner sends GitHub Support a removal request naming the repository and the
affected PRs (a draft is prepared from the audit). Record the ticket number here.

**Verify:** none automatable; the evidence is GitHub's confirmation. Recorded as:
```sh
grep -qE '^GitHub support ticket: [0-9]+' openspec/changes/history-phi-purge/tasks.md
```

**Context budget:** none.

---

### T-HP-9 — Gate every commit a PR introduces, not only its tip

**Depends:** none (independent of the rewrite) · **Review required:** yes

**Amends:** none.

**Reads:** `.github/workflows/ci.yml` (`public-info` job), `scripts/check-tracked-paths.py`,
`scripts/check-public-info.py`.

**Do:** Merges are merge commits, so every branch commit becomes a permanent ancestor of
`main`; a file added and deleted inside one PR still lands in history. Give both scripts a
mode that checks a named commit's tree (not the work tree), and run it in CI for every commit
in `origin/main..HEAD` of a PR. Self-test it with a fixture branch that adds and then deletes
a screenshot, which must fail.

**Verify:**
```sh
grep -q 'origin/main\.\.HEAD\|base\.sha' .github/workflows/ci.yml && uv run python3 scripts/check-tracked-paths.py --self-test
```

**Context budget:** 3 files.
