# Tasks: Host-neutral naming (deferred)

All tasks remain unchecked. This change records later work, not completed renames.
Work in order and commit after each task. No live deployment is authorized here.

- [ ] T-HN-1 — Inventory semantic Pi names and freeze the migration map
- [ ] T-HN-2 — Add neutral helper names with safe compatibility
- [ ] T-HN-3 — Migrate active plan/display references and reader-facing labels

## T-HN-1 — Inventory semantic Pi names and freeze the migration map

**Depends:** none · **Review required:** yes

**Amends:** none; preserves accepted decisions and historical evidence.

**Reads:** this change's `proposal.md` and `specs/host-neutral-naming/spec.md`, `AGENTS.md`,
`README.md`, `rust-toolchain.toml`. Filename and reference discovery with `git ls-files`
and `git grep` is permitted. Before classification, declare in the inventory up to six candidate
implementation/caller body reads per bounded group, then read that group and record its evidence.
If more groups are needed, split discovery/classification into separately scoped tasks before
freezing the map. Also enumerate downstream body reads before dispatching T-HN-2 or T-HN-3.
Do not classify solely from grep output or expand into an unbounded repository-body read.

**Do:** Create `inventory.md` here with a row for each semantic Pi identifier/path and its
callers, canonical replacement or intentional exception, compatibility strategy, and verification.
Include scripts, `PI_HOST`/`PI_PATH`, workflows, `scripts/pi/`, active OpenSpec plans,
`docs/design/pi-panel/`, and agent/integration documentation. Distinguish reusable interfaces
from board-specific firmware/configuration and historical measurements. Preserve identifiers
containing unrelated substrings (`api`, `spike`, PII). Assign bounded file lists and freeze
exact reference/compatibility tests for downstream tasks before implementation. Record unresolved
platform or architecture assumptions without claiming support or changing accepted amendments.

**Verify:** `test -s openspec/changes/host-neutral-naming/inventory.md && openspec validate host-neutral-naming --strict --no-interactive && git diff --check`

**Context budget:** 5 initial files plus filename/reference search results and one declared group
of at most 6 candidate implementation/caller body reads. Additional groups require task splitting;
downstream body reads must be declared individually in the inventory.

## T-HN-2 — Add neutral helper names with safe compatibility

**Depends:** T-HN-1 · **Review required:** yes

**Amends:** none; installation and operator gates remain unchanged.

**Reads:** `inventory.md`, this change's spec, and the helper/test/CI/guide/agent-rule files
explicitly assigned to this task by T-HN-1.

**Do:** Implement the approved canonical helper and environment names. Keep legacy names as
thin compatibility entry points/aliases, document precedence, and reject conflicting old/new
values before I/O. The current installer test stops before SSH; create an isolated fake-`ssh`/
`scp` harness for installer/sync aliases, both variable forms, conflict rejection, identical
allowlists, permissions, and `/dev/null` checks. Separately create a pseudo-TTY/fake-command
harness (or an explicit self-test seam) for the local acceptance wrapper's finite duration and
confirmation gates. The eventual exact Verify must run both harnesses and prove no real remote
or lifecycle command runs. No actual SSH, deployment, pairing, or service action belongs here. Update guides, hooks and current callers together; do not bypass
the guarded artifact installer. Keep existing Verify contracts working.

**Verify:** must be replaced with an exact command before dispatch; task is not dispatchable until then.

**Context budget:** freeze an explicit helper/caller file list in T-HN-1; split this task if
that list exceeds 12 body reads.

## T-HN-3 — Migrate active plan/display references and reader-facing labels

**Depends:** T-HN-1, T-HN-2 · **Review required:** yes

**Amends:** none; accepted amendment text and physical-hardware evidence remain historical.

**Reads:** `inventory.md`, this change's spec, and the active plan/display/reference files
explicitly assigned to this task by T-HN-1.

**Do:** Move reusable active directories to the inventoried neutral paths, preserving task IDs,
checkboxes and evidence. Update current links, CI, hooks and read lists; keep compatibility for
existing exact Verify paths or record a deliberate, separately justified Verify change under
the repository procedure. Add an automated reference check using the inventory's old/new map
and exception list, including links to active plans and display assets. Preserve board-specific
configuration and historical evidence. Run existing relevant test suites plus the new reference
check. Do not claim a detector, alarm, or new platform is implemented merely because names changed.

**Verify:** must be replaced with an exact command before dispatch; task is not dispatchable until then.

**Context budget:** freeze explicit path groups in T-HN-1; split into separately verified tasks
if a group exceeds 12 body reads.
