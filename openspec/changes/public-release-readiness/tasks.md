# Tasks: Public-release readiness

Ordered; each task depends on the one before it unless it says otherwise. `$PURGE` means
`$HOME/Source/aw2-history-purge`, the out-of-repo directory `history-phi-purge` uses. **Nothing
under `$PURGE` is ever copied into the repository.** Three tasks are **owner actions**; every
other task is reversible or local and proceeds without asking.

- [ ] T-PR-1 — Land or close every open pull request · integrate · host
- [ ] T-PR-2 — Remove the medication names from the design mock-ups · implement-simple · host
- [ ] T-PR-3 — Reject self-hosted runners on fork-triggerable workflows · implement · host
- [ ] T-PR-4 — Add SECURITY.md · implement-simple · host
- [ ] T-PR-5 — Turn the personal-term check on locally · ops · host
- [ ] T-PR-6 — Re-run the purge on the final tip and prove it · implement · host
- [ ] T-PR-7 — Choose how the push satisfies `required_signatures` · **owner decision**
- [ ] T-PR-8 — Force-push the rewritten history · ops · host · **owner action**
- [ ] T-PR-9 — Move every clone onto the new history · ops · host, pi
- [ ] T-PR-10 — Ask GitHub to purge cached pull-request refs · ops · **owner action**
- [ ] T-PR-11 — Prove a fresh clone from GitHub is clean · investigate · host
- [ ] T-PR-12 — Make the repository public and turn on publication settings · ops · **owner action**
- [ ] T-PR-13 — Gate every commit a PR introduces (history-phi-purge T-HP-9) · implement · host

---

### T-PR-1 — Land or close every open pull request

**Depends:** none · **Review required:** per PR

**Amends:** none.

**Reads:** `gh pr list --state open`.

**Do:** A force-push rewrites the base of every open PR under it. Merge each open PR that has
passed review and CI; close the rest with a comment saying why (Dependabot re-opens its own
after the push). Do not open a new PR between this task and T-PR-8 except for T-PR-2…T-PR-4,
which merge before T-PR-6 starts.

**Verify:**
```sh
test "$(gh pr list --state open --json number --jq length)" = 0
```

**Context budget:** the open PRs.

---

### T-PR-2 — Remove the medication names from the design mock-ups

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `docs/design/admin-console/claude-design/{a,b,c}/records.html`,
`$PURGE/manifest/replace-health.txt`.

**Do:** Replace every medication name and dose in the three mock-ups with the same neutral
placeholders the purge already uses for the iOS client, so tip and rewritten history agree. Add
matching pairs to `$PURGE/manifest/replace-health.txt` for every historical form, and a pattern to
`$PURGE/residuals.sh`. Name no medication in the commit message, the PR or this file.

**Verify:**
```sh
git fetch -q origin && ! git grep -qiE -f "$HOME/Source/aw2-history-purge/manifest/medication-terms.txt" origin/main
```

**Context budget:** 3 files, 2 out-of-repo files.

---

### T-PR-3 — Reject self-hosted runners on fork-triggerable workflows

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `.github/workflows/*.yml`, `.github/workflows/ci.yml` (`public-info` job).

**Do:** Add `scripts/check-workflow-runners.py`: for every workflow under `.github/workflows/`,
if its triggers include anything other than `push`, `workflow_dispatch` or `schedule`, every
job's `runs-on` must be a GitHub-hosted label. A violation names the file, the job and the
trigger. `--self-test` must fail a `pull_request` workflow on `[self-hosted, …]` and pass the
repository's current workflows. Run both in `public-info`, and allowlist the script in
`scripts/check-tracked-paths.py` if its policy requires it.

**Verify:**
```sh
uv run python3 scripts/check-workflow-runners.py --self-test && uv run python3 scripts/check-workflow-runners.py && grep -q 'check-workflow-runners.py' .github/workflows/ci.yml
```

**Context budget:** 3 files.

---

### T-PR-4 — Add SECURITY.md

**Depends:** none · **Review required:** yes

**Amends:** none.

**Reads:** `README.md`, `scripts/check-tracked-paths.py` (root-file allowlist).

**Do:** State that this is a LAN-only personal prototype, not a medical device; that
vulnerabilities go through GitHub private vulnerability reporting (enabled in T-PR-12), not
public issues; and that the known hardening gaps live in `openspec/changes/prototype-hardening-backlog`.
Add `SECURITY.md` to the root-file allowlist in the same commit.

**Verify:**
```sh
test -s SECURITY.md && uv run python3 scripts/check-tracked-paths.py
```

**Context budget:** 2 files.

---

### T-PR-5 — Turn the personal-term check on locally

**Depends:** none · **Review required:** no

**Amends:** none.

**Reads:** `.githooks/pre-push` (denylist section), `.githooks/pii-denylist.local.txt.example`,
`$PURGE/manifest/replace-identifiers.txt`.

**Do:** Create `.githooks/pii-denylist.local.txt` (git-ignored) from the personal terms the purge
manifest replaces — names, account names, device names, hostnames — in the format the example
gives. It never enters the repository. The owner may add terms; none is required from him.

**Verify:**
```sh
test -s .githooks/pii-denylist.local.txt && git check-ignore -q .githooks/pii-denylist.local.txt && ! git ls-files --error-unmatch .githooks/pii-denylist.local.txt 2>/dev/null
```

**Context budget:** 1 local file.

---

### T-PR-6 — Re-run the purge on the final tip and prove it

**Depends:** T-PR-1…T-PR-5 · **Review required:** yes

**Amends:** history-phi-purge T-HP-3, T-HP-4 (re-run, unchanged in substance).

**Reads:** `$PURGE/final.sh`, `$PURGE/verify.py`, `$PURGE/residuals.sh`.

**Do:** Run `$PURGE/final.sh`: fresh `--all` backup bundle, manifest regenerated against the
current `origin/main`, full-mirror rewrite, `verify.sh`, `residuals.sh`. Then scan every ref of
the mirror with gitleaks. Regenerate `$PURGE/push.sh` with leases pinned to GitHub's current
branch SHAs, and the branch list as it now is.

**Verify:**
```sh
P="$HOME/Source/aw2-history-purge"; bash "$P/verify.sh" && bash "$P/residuals.sh" "$P/mirror.git" && gitleaks git --no-banner --redact --log-opts=--all "$P/mirror.git"
```

**Context budget:** the mirror.

---

### T-PR-7 — Choose how the push satisfies `required_signatures`

**Depends:** T-PR-6 · **Owner decision**

**Amends:** history-phi-purge T-HP-5 (it assumed a `non_fast_forward` rule that does not exist and
did not account for signatures).

**Reads:** `gh api repos/chenders/AnxietyWatch2/rulesets/20287061`.

**Do:** Present the two options with their cost:
- **Re-sign.** Sign every rewritten commit with the owner's key, setting the owner as committer on
  commits that GitHub, Dependabot or Copilot committed (a signature verifies only against the
  committer). The ruleset is never touched; those commits' committer field changes.
- **Lift the rule.** The owner disables `required_signatures` for the one push and re-enables it
  after. Metadata is untouched; the rewritten history is permanently unsigned.

Record the choice here as `Signatures: re-sign` or `Signatures: lifted`. If re-signing, apply it
to the mirror and re-run T-PR-6's Verify.

**Verify:**
```sh
grep -qE '^Signatures: (re-sign|lifted)$' openspec/changes/public-release-readiness/tasks.md
```

**Context budget:** none.

---

### T-PR-8 — Force-push the rewritten history

**Depends:** T-PR-7 · **Review required:** yes · **Owner action**

**Amends:** history-phi-purge T-HP-5.

**Reads:** `$PURGE/push.sh`.

**Do:** With no open PR, run `$PURGE/push.sh` (atomic, `--force-with-lease` per branch). If the
owner chose to lift the rule, he restores it immediately after; the Verify checks it is back.

**Verify:**
```sh
git ls-remote --heads origin | while read -r sha ref; do test "$(git --git-dir="$HOME/Source/aw2-history-purge/mirror.git" rev-parse "refs/aw2-remotes/origin/${ref#refs/heads/}")" = "$sha" || { echo "mismatch: $ref"; exit 1; }; done && gh api repos/chenders/AnxietyWatch2/rulesets/20287061 --jq '.rules[].type' | grep -qx required_signatures
```

**Context budget:** none.

---

### T-PR-9 — Move every clone onto the new history

**Depends:** T-PR-8 · **Review required:** no

**Amends:** history-phi-purge T-HP-6, T-HP-7.

**Reads:** `$PURGE/backup/` (stash and worktree lists), `scripts/sync-to-pi.sh`.

**Do:** As T-HP-6 (this checkout and its worktrees, reflog expired, `gc --prune=now`) and T-HP-7
(re-clone, never fetch into, any checkout on the Pi). If the Pi is powered down, this task waits
for it and nothing after it does: the Pi's clone is not served by GitHub.

**Verify:**
```sh
test -z "$(git rev-list --all -- firmware aw2-ios-thin-client-chatgpt-context.zip)" && test "$(git count-objects -v | awk '/^count:/ {print $2}')" -lt 100
```

**Context budget:** none.

---

### T-PR-10 — Ask GitHub to purge cached pull-request refs

**Depends:** T-PR-8 · **Owner action**

**Amends:** history-phi-purge T-HP-8.

**Reads:** GitHub's documentation on removing sensitive data from a repository.

**Do:** A request is drafted (repository, PR numbers whose refs reach removed objects, the
rewrite date) and the owner sends it from his account. Record the ticket number, then GitHub's
confirmation, here.

**Verify:**
```sh
grep -qE '^GitHub support ticket: [0-9]+ \(purge confirmed [0-9]{4}-[0-9]{2}-[0-9]{2}\)$' openspec/changes/public-release-readiness/tasks.md
```

**Context budget:** none.

---

### T-PR-11 — Prove a fresh clone from GitHub is clean

**Depends:** T-PR-10 · **Review required:** yes

**Amends:** none.

**Reads:** `$PURGE/verify.py`, `$PURGE/residuals.sh`.

**Do:** `git clone --mirror` from GitHub into a new directory (this fetches `refs/pull/*` too),
then run the content rules over every blob, the residual check, the removed-path check and
gitleaks against it. A clone, not the local mirror, is what the public would get.

**Verify:**
```sh
bash "$HOME/Source/aw2-history-purge/verify-github-clone.sh"
```

**Context budget:** a fresh mirror clone.

---

### T-PR-12 — Make the repository public and turn on publication settings

**Depends:** T-PR-11 · **Owner action**

**Amends:** none.

**Reads:** none.

**Do:** The owner changes visibility to public. Immediately after: secret scanning and push
protection on, private vulnerability reporting on, fork-PR workflow approval set to all
external contributors (settable only once public).

**Verify:**
```sh
gh repo view chenders/AnxietyWatch2 --json visibility --jq .visibility | grep -qx PUBLIC && gh api repos/chenders/AnxietyWatch2 --jq '.security_and_analysis.secret_scanning.status, .security_and_analysis.secret_scanning_push_protection.status' | grep -c enabled | grep -qx 2 && gh api repos/chenders/AnxietyWatch2/private-vulnerability-reporting --jq .enabled | grep -qx true && gh api repos/chenders/AnxietyWatch2/actions/permissions/fork-pr-contributor-approval --jq .approval_policy | grep -qx all_external_contributors
```

**Context budget:** none.

---

### T-PR-13 — Gate every commit a PR introduces

**Depends:** T-PR-9 (built on the new history) · **Review required:** yes

**Amends:** none — this is history-phi-purge T-HP-9, unchanged, scheduled here.

**Reads:** as T-HP-9.

**Do:** As T-HP-9. Not a publication blocker; it keeps the result from regressing.

**Verify:** as T-HP-9.

**Context budget:** 3 files.
