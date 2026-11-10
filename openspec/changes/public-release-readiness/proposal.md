# Proposal: Make the repository safe to publish on GitHub

Making a repository public publishes **every object reachable from any ref**, not the current
tree. As of 2026-11-10 the tree is clean (`scripts/check-public-info.py` passes on `main`), but
history is not, and several things outside git also change meaning when the repository becomes
public. This change is the complete list of what has to be true first, in the order it has to
happen. It reuses `history-phi-purge` for the rewrite itself and adds what that plan does not
cover.

## What blocks publication

1. **Health data and identifiers in history.** Everything `history-phi-purge` enumerates:
   screenshots including Health/HealthKit screens, a screen recording, a snapshot ZIP, real
   readings in fixtures, clinical detail, device identifiers, home paths, the Pi's hostname and
   address. Only that plan's tip clean-up (#105, #107–#114) has landed; its rewrite has not.
2. **Medication names still on the tip.** The purge manifest already replaces two medication
   names in the iOS client's history, but the same names, and one dose, remain in the three
   admin-console design mock-ups under `docs/design/admin-console/claude-design/`. If the iOS
   values were the owner's regimen, these are too. They are removed from the tip first, so the
   rewrite's "main's final tree is unchanged" proof still holds.
3. **Withheld code in history.** `firmware/` is in history from PR #22 and pending the owner's
   legal review. It is already in the purge's path list, so the rewrite removes it; publication
   no longer depends on the review's outcome.
4. **The `main` ruleset requires signed commits.** `required_signatures` is active with no
   bypass actor. `git filter-repo` drops signatures, so a force-push of the rewritten history is
   rejected as it stands. `history-phi-purge` T-HP-5 assumed a `non_fast_forward` rule, which the
   ruleset does not have, and did not account for signatures. Either the rewritten commits are
   re-signed — which requires rewriting the committer of commits that GitHub, Dependabot or
   Copilot committed, because a signature is verified only against the committer — or the owner
   lifts the rule for the one push and the rewritten history stays unsigned. **That is an owner
   decision**, and the only one this plan needs before the push.
5. **Pull-request refs.** GitHub keeps `refs/pull/*/head` for every PR. A force-push cannot
   change them, so publishing after a perfect rewrite would still expose the old commits through
   PR pages. Only GitHub Support can purge them, and that must finish **before** publication.
6. **Self-hosted runner.** `pi-deploy.yml` and `pi-runner-smoke.yml` run on a self-hosted runner
   on the Pi. Today only `push` and `workflow_dispatch` trigger them, which need write access.
   Nothing stops a future workflow from targeting that runner from a trigger a fork can fire,
   which on a public repository would run a stranger's code on the Pi. A CI gate makes that
   impossible to merge.
7. **The personal-term check is off on this machine.** The pre-push gate warns
   `no .githooks/pii-denylist.local.txt — personal-term checking is off` on every push.
8. **Publication settings.** Fork-PR workflow approval cannot even be set while the repository
   is private; secret scanning, push protection and private vulnerability reporting should be on
   from the first public minute. A `SECURITY.md` says where to report.

## Non-goals

- The open hardening backlog (#103, `prototype-hardening-backlog`). This is a LAN-only
  prototype; publishing the code does not expose the LAN. Those gaps stay tracked there.
- The Apple Team ID in `clients/apple/Config/Shared.xcconfig`. It is embedded in every signed
  build Apple distributes and is not a secret; it stays.

## Order

Land or close every open PR → fix the tip → enable the local gates → re-run the purge on the
final tip and prove it → **owner: signatures decision, then push** → move every clone onto the
new history → **owner: GitHub Support purge** → prove a fresh clone from GitHub is clean →
**owner: make public** → turn on the publication settings. Publication is last and is the only
step that cannot be taken back.
