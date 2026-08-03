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

## Git

- **Never commit to `main`.** Branch first: `git checkout -b <name> main`.
- Use `git pull --rebase`. Never `git reset --hard`, `git checkout -B`, or `git switch -C`.
- Stage specific files by name. Never `git add -A` or `git add .`.
- Commit after every task.

## C

- C11. `-Wall -Wextra -Werror -Wshadow -Wconversion`. **A new warning is a failing test.**
- Every function that parses bytes off a radio is pure: `(const uint8_t *, size_t) -> result`.
  No allocation, no I/O, no global state. Bounds-check every read — `len` is attacker-controlled.
- Every such function also gets a libFuzzer target in `vitals/fuzz/`.
- No `UPDATE`, no `DELETE` against `samples`. It is append-only; removal is a tombstone row.
- Never clamp an out-of-range value. Reject it, name the reason, quarantine it. A clamped reading is
  indistinguishable from a real one downstream.
- Invalid or absent data contributes nothing. Never coerce a sentinel to a number — IEEE-11073
  SFLOAT `0x07FF`/`0x0800` mean *absent*, not zero.

## Portability

Two libraries, and the split is load-bearing:

| Target | Contains | Builds on |
|---|---|---|
| `vitals_core` | decoders, HLC — pure C11 | macOS and Linux |
| `vitals_linux` | SQLite store, `vitalsd`, epoll, BlueZ, systemd | **Linux only** |

Do not add a Linux header to `vitals_core`. Do not mock BlueZ to make something build on macOS — a
mock only tests the mock. Linux-specific work, including debugging, runs on the Pi
(`scripts/sync-to-pi.sh`).

## Time and units

- The store holds **UTC epoch seconds** and nothing else. Local time exists only in presentation.
- One canonical unit per type, converted **once**, at the decoder. Never at a later layer.
- The unit and valid range come from `vitals/registry.toml`. Never hardcode either.

## The registry

`vitals/registry.toml` is the single source of truth for sources, types, units, ranges, and
thresholds. Files in `vitals/generated/` are produced from it and are **never hand-edited** — CI
regenerates and diffs. If you need a new source or type, add it to the TOML and regenerate.

Source and type identifiers are integers everywhere in code. A string like `"polar_h10"` appears in
exactly one file.

## Tests

- Every new or changed behavior gets a test. Coverage gates: 95% pure logic, 85% store/daemon.
- Never assert float equality. Use a tolerance: `fabs(a - b) < 1e-9`.
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
