# Design — Pi 5 Vitals Core

Architecture lives in `docs/superpowers/specs/2026-08-03-pi5-vitals-core-design.md`. This document
is the **execution contract**: how tasks in `tasks.md` are dispatched, by which class of model, and
how their output is verified without a human in the loop.

---

## 1. Execution model

Herdr `opsx-dev-layout` spins a conductor + N workers for this change:

```
herdr plugin action invoke spinup --plugin <layout-plugin-id>
```

- **Conductor** — reads `proposal.md`, `design.md`, `tasks.md`; dispatches; reviews; maintains
  checkboxes. Does not implement except where a task is marked `tier: mechanical`.
- **Workers** — `srt pi` (sandboxed). Each receives one task brief and nothing else.
- **Shell pane** — `grep -n '^- \[' tasks.md`, `git status`.

Dispatch loop, per task:

```sh
herdr agent send   worker-1 "$(sed -n '/^### T03 /,/^### T04 /p' openspec/changes/pi5-vitals-core/tasks.md)"
herdr agent wait   worker-1 --status idle
herdr agent read   worker-1
```

The task text **is** the prompt. A task that cannot be pasted into a fresh agent and executed
without conversation history is a defective task.

---

## 2. Model tiers

Tasks name a **capability tier**, never a vendor. One table maps tiers to today's models; model
churn edits this table and nothing else.

| Tier | Use | Current model | Where |
|---|---|---|---|
| `mechanical` | Codegen, config files, systemd units, boilerplate. No design judgment. | `pi --model gemini-3.5-flash` | any |
| `implement-standard` | Normal coding against a written spec with tests supplied. | `pi --model qwen3-coder` | any |
| `implement-complex` | Binary protocol decode, concurrency, single-writer discipline, anything correctness-critical. | `pi --model gemini-3.1-pro` | any |
| `review` | Adversarial verification. Tries to break the deliverable, not confirm it. | `pi --model claude-opus-5` | any |
| `human` | Physical hardware. Screwdrivers and cables. | — | bench |

**Rate-limit note.** The prior redesign lost AUTHOR-Fast to a Groq 12k TPM cap on large prompts.
Every task therefore declares a **context budget** and an explicit **Reads:** list. A worker reads
those files and no others. A task whose brief plus its `Reads:` list exceeds its budget is
mis-scoped and must be split, not squeezed.

---

## 3. Verification protocol

**Every task's acceptance is a shell command with an exit code.** Not a description, not a
judgment, not "looks correct."

This is a load-bearing constraint, not a style preference. Workers span a wide capability range, and
a weaker model's self-assessment is not evidence. A command that exits 0 is evidence regardless of
which model ran it. It also means the conductor verifies by running, not by reading.

Task acceptance blocks are literal:

```
**Verify:** `ssh "$PI_HOST" 'cd ~/AnxietyWatch2 && cargo test -p vitals-decode sfloat'`
**Expect:** exit 0, "Ok: 12" in output
```

Where a deliverable is genuinely not machine-checkable (hardware seated correctly, a physical cable),
the task is `tier: human` and its verify step is a command that *observes* the result —
`lsblk | grep nvme0n1`, `lsusb | grep -i cambridge` — never an assertion that the human did it.

### 3.1 Review gate

Tasks marked `review-required: yes` get a second dispatch to a `review` worker after the
implementing worker reports idle:

```sh
herdr agent send worker-2 "Adversarially review the diff for T03 against
openspec/changes/pi5-vitals-core/tasks.md §T03. Try to find an input that breaks it.
Report PASS or a specific failing input. Do not confirm — attempt to refute."
```

Applied to: anything decoding bytes off a radio, anything writing to the store, anything in the
backup path.

---

## 4. Host targeting

Tasks declare `host:` because the two machines differ in what they can do.

| `host:` | Meaning |
|---|---|
| `pi` | Runs on the Pi. Hardware, radios, systemd, Postgres, logs. |
| `mac` | Runs in this repo. Specs, registry TOML, generated headers, plan bookkeeping. |
| `either` | Pure code with no hardware dependency. Cross-compiles or runs in CI. |

### 4.1 Measured harness inventory (2026-08-03)

| | development host | Pi |
|---|---|---|
| `herdr` | `~/.local/bin/herdr` | **0.7.5, server live** (`~/.config/herdr/herdr.sock`) |
| `pi` | `~/.nvm/.../bin/pi` | **0.83.0**, at `~/.local/share/pi-node/node-v22.23.2-linux-arm64/bin/pi` |
| `claude` | this session | 2.1.220 at `~/.local/bin/claude` |
| Worker pool | multi-model via `pi --model …` | multi-model, **after the PATH fix below** |

**Prerequisite 1 — pi's bundled node on PATH. ✅ DONE 2026-08-03.** `pi` on the Pi ships its own
Node runtime beside it and `node` is not otherwise installed, so `pi` failed with
`/usr/bin/env: 'node': No such file or directory` and a herdr `WORKER_CMD="pi --model …"` would have
died on launch. `~/.local/share/pi-node/node-v22.23.2-linux-arm64/bin` is now on PATH via `.bashrc`
and `.profile`; `bash -lc "pi --version"` returns 0.83.0.

**Prerequisite 2 — LiteLLM gateway relocated to the Pi. ✅ DONE 2026-08-03.** The Pi's
`~/.pi/agent/models.json` sets `defaultProvider: litellm` at `http://localhost:4000/v1`, but the
gateway ran in Docker **on the Mac**, bound to `127.0.0.1:4000` — verified UNREACHABLE from the Pi,
so `pi` there could not reach any model at all.

Exposing the Mac's gateway would have worked, but it was the wrong fix: it makes the Pi's overnight
capture depend on a laptop staying awake to broker its model calls, which is the exact dependency
this project exists to remove. **The gateway is infrastructure and belongs on the box that does not
sleep.** So it moved instead:

- Docker 29.7.1 + Compose v5.3.1 installed on the Pi (`get.docker.com`).
- `litellm/` stack copied to `~/litellm` on the Pi; `.env` transferred via scp at mode `600`.
- Port binding changed `127.0.0.1:4000` → `0.0.0.0:4000` so both hosts share one gateway. Auth is
  enforced by LiteLLM master/virtual keys; the LAN is trusted.
- `ghcr.io/berriai/litellm-database:latest` confirmed to publish `linux/arm64`.

Verified: `{"status":"healthy","db":"connected"}` from the Pi *and* from the Mac at
`http://pi-host:4000`, and `pi -p` on the Pi completes a real model call end to end.

| Host | `models.json` api base | State |
|---|---|---|
| Pi | `http://localhost:4000/v1` | correct as-is — the gateway is now local |
| Mac | `http://localhost:4000/v1` | still points at its own gateway |

**Remaining, optional:** the Mac still runs a second gateway. Consolidating — stop the Mac's stack
and repoint its `models.json` at `http://pi-host:4000/v1` — restores a single budget cap and
one cost dashboard. Not required by any task in this change.

Budget caps and the cost dashboard are preserved; only the host changed. This is the one place
Docker appears before Stage 6, and it is correct there — a model gateway is a restartable
convenience, never on the life-safety path.

With both prerequisites done, the full §2 tier table is available on **both** hosts. Two dispatch
modes, both valid:

**Mode A — Mac pool, remote execution (default).** Workers run on the Mac; every `Verify` in
`tasks.md` is already written as `ssh "$PI_HOST" '…'`, so `host: pi` tasks execute remotely with
no change. Default for everything except long-running hardware work.

**Mode B — Pi-local session.** For work where being on the box matters — the overnight T-B1 capture,
`dmesg`/`journalctl` tailing, `bluetoothctl` interaction, anything that must outlive an SSH
connection — attach to the Pi's own herdr from the Mac:

```sh
herdr --remote "$PI_HOST"
```

Workers execute locally on the Pi with the same model tiers. Prefer Mode B for all of Group A and
T-B1; Mode A for Group D's pure-code tasks.

**Repo split.** This repo is authoritative for `proposal/design/tasks`, the registry TOML, and
generated headers — those are committed here. Pi-resident source (`~/vitals`) is a working tree that
consumes them. Generated artifacts are committed, never regenerated ad hoc on the Pi, so both hosts
agree on what a `source` id means.

---

## 5. Ordering and gates

Tasks are mostly sequential. Three hard gates, where proceeding early destroys something:

**G1 — `T-A4` (disable onboard Bluetooth) requires `T-A3` green.**
`dtoverlay=disable-bt` removes `hci0`. If the Sena has not been physically installed and verified
scanning as `hci1`, this leaves the box with no Bluetooth at all. `T-A3` must show a successful scan
on `hci1` before `T-A4` may run. **The conductor must not dispatch T-A4 on a report; only on a
verify command that exited 0.**

**G2 — `T-C*` (Postgres) requires `T-A1` green.** Postgres must never initialize on the microSD. If
`PGDATA` lands on `mmcblk0`, moving it later means a dump/restore rather than a mount point.

**G3 — `T-D6` (bp-collector) requires `T-C3` green.** No sample worth keeping is written before a
restore has been demonstrated to work.

Everything else in a group may run concurrently across workers.

---

## 6. Task anatomy

Every task in `tasks.md` carries this header. Fields are not optional — a missing field is a
defective task.

```
### T-XX — <title>

**Tier:** <mechanical|implement-standard|implement-complex|review|human>
**Host:** <pi|mac|either>
**Context budget:** <~Nk tokens>
**Depends:** <task ids, or none>
**Review required:** <yes|no>

**Reads:** exact paths. The worker reads these and nothing else.
**Creates:** exact paths.
**Modifies:** exact paths with line ranges where known.

**Do:** numbered steps, each 2–5 minutes, with literal commands and literal code.

**Verify:** one shell command.
**Expect:** exact exit code and output substring.

**Commit:** literal git command with the message.
```

---

## 7. Conductor operating rules

1. Dispatch one task per worker. Never two tasks to one worker concurrently.
2. Run the `Verify` command yourself. A worker reporting success is a claim; the exit code is the
   evidence.
3. On a failing verify, re-dispatch to the *same* worker with the failure output appended. After two
   failures, escalate one tier and re-dispatch fresh.
4. Update the `- [ ]` checkbox in `tasks.md` only after your own verify exits 0.
5. Never dispatch across a gate (§5) on a report. Only on a green verify.
6. Anything destructive — repartitioning, `dtoverlay` edits, `pgbackrest` restore into a live
   cluster — stops and asks the human.
7. Record scope deltas in `tasks.md` under `## Implementation notes` as they happen, per the
   CLAUDE.md plan-doc rule.
