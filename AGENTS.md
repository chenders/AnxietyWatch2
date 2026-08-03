# AGENTS.md — instructions for agentic workers

Read this before touching anything. `CLAUDE.md` is a symlink to this file; both Claude Code and pi
read the same rules.

## The one rule that matters

**Acceptance is a shell command's exit code, never your own assessment.**

Every task in `openspec/changes/*/tasks.md` carries a `Verify:` line. Run it. If it exits nonzero,
the task is not done, regardless of how the code looks. Do not report success on a command you did
not run.

## Scope

Work one task at a time. Read **only** the files in that task's `Reads:` list — not the whole repo.
Every task declares a context budget; if you find yourself reading far beyond the list, the task is
mis-scoped. Say so rather than proceeding.

### Always in scope, regardless of `Reads:`

**`docs/specs/AMENDMENTS.md`** is mandatory reading for every task. It is the index of decisions that
supersede the original design, and a task brief that contradicts an accepted amendment is **wrong —
the amendment wins.** Report the contradiction rather than implementing either version.

This exists because the rule above once caused a real defect. Four amendments changed the
implementation language, the storage engine, the schema and the primary key, while every task brief
still described the superseded system. No `Reads:` list referenced an amendment, so a worker obeying
the scoping rule *correctly* could not see them — and the committed SQL proved it had already
happened. The optimisation that keeps context small silently broke the invariant that a task brief
describes the current system.

Each task header also carries an `**Amends:**` field naming the amendments that touch it. Read those
in full before starting.

## Git

- **Never commit to `main`.** Branch first: `git checkout -b <name> main`.
- Use `git pull --rebase`. Never `git reset --hard`, `git checkout -B`, or `git switch -C`.
- Stage specific files by name. Never `git add -A` or `git add .`.
- Commit after every task.

## Rust

- Rust, not C. `cargo clippy -- -D warnings`. **A new warning is a failing test.**
- Every function that parses bytes off a radio is pure: `fn decode_x(buf: &[u8]) -> Option<T>`.
  No allocation, no I/O, no global state. Use `get()`, never `buf[i]` — `indexing_slicing` is denied
  and the input is attacker-controlled.
- Return `Option`/`Result`, never a `bool` plus an out-parameter. A caller must not be able to read a
  value that is not there. That is the reason this project is in Rust.
- Every such function also gets a `cargo-fuzz` target in `fuzz/fuzz_targets/`.
- No `unwrap()`, `expect()`, or `panic!` outside tests. `panic = "abort"` in release, so a panic is a
  process death, and a dead collector is a silent alarm.
- No `UPDATE`, no `DELETE` against `samples`. It is append-only; removal is a tombstone row.
- Never clamp an out-of-range value. Reject it, name the reason, quarantine it. A clamped reading is
  indistinguishable from a real one downstream.
- Invalid or absent data contributes nothing. Never coerce a sentinel to a number — IEEE-11073
  SFLOAT `0x07FF`/`0x0800` mean *absent*, not zero.

## Crate boundaries

The split is load-bearing:

| Crate | Contains | Constraint |
|---|---|---|
| `vitals-registry` | generated types and sources | `no_std`, no dependencies |
| `vitals-decode` | pure decoders | `no_std`, `forbid(unsafe_code)`, no I/O |
| `vitals-core` | shared value types, COPY encoder, spool | no BlueZ, no systemd |
| `vitalsd`, collectors | BlueZ, tokio, postgres, systemd | Linux only |

Do not add a BlueZ, SQL, or systemd dependency to `vitals-decode` or `vitals-registry`. Do not mock
BlueZ to make something build on macOS — a mock only tests the mock. Linux-specific work, including
debugging, runs on the Pi (`scripts/sync-to-pi.sh`).

## Time and units

- The store holds **UTC epoch seconds** and nothing else. Local time exists only in presentation.
- One canonical unit per type, converted **once**, at the decoder. Never at a later layer.
- The unit and valid range come from `vitals/registry.toml`. Never hardcode either.

## The registry

`vitals/registry.toml` is the single source of truth for sources, types, units, ranges, and
thresholds. Files in `vitals/generated/` are produced from it and are **never hand-edited** — CI
regenerates and diffs. If you need a new source or type, add it to the TOML and regenerate.

Source and type identifiers are the **generated enums** `VitalsSource` and `VitalsType` everywhere in
code — never a bare integer, never a string. An `i32` that can hold any value is what lets a source
id and a type id be transposed at a call site without the compiler noticing. A string like
`"polar_h10"` appears in exactly one file.

## Tests

- Every new or changed behavior gets a test. Coverage gates: 90% decoders, 85% store/daemon.
- Never assert float equality. Use a tolerance: `(a - b).abs() < 1e-9`.
- Fixed reference timestamps. No wall clock in assertions.
- Fixing a failing test is always in scope, related to your task or not.

## Guards must say why

Any check that blocks an action names its blocker and the value that blocked it. Failing closed is
correct; failing closed silently is not. `"restore blocked: HealthSnapshot=91"`, never
`"restore blocked"`.

## Unknown is not OK

Three states, always: measured-and-good, measured-and-bad, and **not measured**. The third is never
rendered, logged, or scored as the first. A missing signal holds state; it never reassures and never
escalates on its own.

## Destructive actions

Stop and ask before: repartitioning, `dtoverlay` edits, `pgbackrest restore` into a live cluster,
dropping tables, or anything that touches `/boot/firmware`.

Gate G1 is absolute: **never apply `dtoverlay=disable-bt` until the Sena dongle is verified UP as
`hci1`.** It removes the onboard controller and leaves the box with no Bluetooth at all.

## Keeping docs in sync

If you change a threshold, a sort order, a filter, or an interface, update every `///` comment and
doc in the same commit. A plan doc that does not reflect what shipped is a defect.

## Workflow — every change ships this way

**Nothing lands on `main` except through a reviewed PR.** The pre-push hook blocks direct pushes.

```
worktree → branch → commit → push → PR → /respond-to-copilot → squash-merge
```

### One worktree per task

Concurrent agents must not share a working tree. Two agents in one checkout race on shared files,
and a commit from one lands on the other's branch. Both happened here on 2026-08-03.

```sh
git worktree add -b task/t-d3-libvitals ../aw2-t-d3 main
cd ../aw2-t-d3
```

Clean up after merge: `git worktree remove ../aw2-t-d3`

### Review

Open the PR, then run `/respond-to-copilot` and work the loop until Copilot has no actionable
findings. Address every comment — fix it, or reply with the reason and the rule. Silence reads as
agreement and the next round re-raises it.

Before opening the PR, dispatch `c-pre-pr-reviewer` on the diff (and
`physio-accuracy-reviewer` if it touches a decoder, the registry, or the CNS pipeline). Catching a
finding there costs one agent run; catching it in Copilot costs a full round trip.

### Commit trailers

Agent-authored commits carry a `Co-Authored-By:` trailer. The repo's git config signs every commit
as the owner, so without it agent work is indistinguishable from human work in `git log`. The
pre-push hook enforces this on any `type(T-XX):` commit.

### Verify commands are contracts

Never edit a `**Verify:**` line in `tasks.md` so your implementation passes. Satisfy it as written.
If the command is genuinely wrong, say so in the PR and change it deliberately as its own decision.
The pre-push hook blocks silent edits — this has already happened once.

### Ticking a box

`- [ ]` → `- [x]` in `tasks.md` only after **you** ran that task's Verify command and saw exit 0.
A worker's report is a claim; the exit code is the evidence.
