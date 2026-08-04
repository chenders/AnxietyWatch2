# Runbook — driving this change with herdr

Operator's guide. How to actually execute `openspec/changes/pi5-vitals-core/` using herdr + pi,
on the Mac, on the-pi, or both.

Read `design.md` §2 (tiers), §3 (verification), §5 (gates) first — this file is the *how*, that one
is the *why*.

---

## 1. Which machine

| Task group | Machine | Why |
|---|---|---|
| **Group A** (hardware) | **the-pi** | You are physically at the box with a screwdriver. `dmesg`, `lsusb`, `hciconfig` are local. |
| **T-B1** (8-hour capture) | **the-pi** | Must outlive an SSH session. Runs overnight under `systemd-run`. |
| **Group C** (Postgres) | either | Every `Verify` is already `ssh $PI_HOST '…'`, so Mac works fine. |
| **Group D** (code) | **Mac** | Pure code. Bigger worker pool, your editor, the repo is here. |

Rule of thumb: **hardware and overnight work on the-pi, code on the Mac.** Both have herdr 0.7.5
and pi 0.83.0, so either works for anything — this is preference, not capability.

Attach to the Pi's session from your Mac terminal without leaving your desk:

```sh
herdr --remote $PI_HOST
```

That drives the Pi's own herdr server. Panes execute *on the Pi*, survive disconnection, and are
still there when you reattach from a phone at 3 a.m.

---

## 2. One-time setup

### 2.1 Both machines — confirm the harness

```sh
herdr --version          # expect 0.7.5
pi --version             # expect 0.83.0
claude --version         # expect 2.1.220
```

On the-pi, `pi` needs its bundled node on PATH. Already done — `~/.bashrc` and `~/.profile` carry
`~/.local/share/pi-node/node-v22.23.2-linux-arm64/bin`. If `pi` ever reports
`/usr/bin/env: 'node': No such file or directory`, that line went missing.

### 2.2 The gateway

LiteLLM now runs **on the-pi**, not the Mac. Verify from wherever you are:

```sh
curl -s http://pi-host:4000/health/readiness     # {"status":"healthy","db":"connected"}
```

Its `config.yaml` uses passthrough wildcards, so any model under these prefixes routes through with
no config change: `openai/*` · `gemini/*` · `groq/*` · `openrouter/*` · `xai/*` · `deepseek/*` ·
`mistral/*` · `perplexity/*`.

**`anthropic/*` is commented out.** Claude models do not route through the gateway — the `review`
tier uses the `claude` CLI directly on its subscription, which is also why review costs nothing
against your topped-up credits.

**There is exactly one gateway, and it lives on the Pi.** The Mac's stack was stopped and its
`~/.pi/agent/models.json` repointed at `http://pi-host:4000/v1` (backup at `models.json.bak`).
Both machines share one worker key, one budget, one cost dashboard. The Pi is the right host because
it is the box that is always on — a laptop that sleeps should not be brokering model calls for an
overnight capture running on the Pi.

**Consequence for adding provider keys:** add them **on the Pi only**, and both machines get them.

```sh
ssh $PI_HOST && cd ~/litellm && ./add-provider-key.sh
```

Running that on the Mac now edits a `.env` no gateway reads — it will report success and change
nothing.

### 2.3 Budget-capped worker key

Workers get a spend-capped key so a runaway loop cannot drain an account:

```sh
cd ~/Source/ai-harness-stack
litellm/create-worker-key.sh 5 25          # $5/day, $25/month
export LITELLM_WORKER_KEY=<printed key>    # add to ~/.bashrc on whichever box runs workers
```

### 2.4 The layout plugin

```sh
herdr plugin config-dir example.opsx-dev-layout    # prints the config dir
```

Edit `config.env` there:

```sh
CONDUCTOR_CMD="claude"                     # conductor reads specs, dispatches, verifies
WORKER_CMD="srt pi"                        # sandboxed workers; model set per-dispatch
WORKERS=3                                  # T-D1, T-D2, T-D3 can run concurrently
START_WORKERS=1
SEND_KICKOFF=1
```

`srt` sandboxes workers at the OS level (filesystem + network). Keep it — these agents run shell
commands against a box you care about.

Conductor as `claude` rather than `pi` is deliberate: the conductor's job is reading specs, judging
diffs, and refusing to cross gates. That is the most judgment-heavy seat and the one place to spend
the strongest model.

---

## 3. Tier → model

`design.md` names capability tiers, never vendors. This is the mapping. **Edit this table when
models change; never edit the tasks.**

All IDs below were live-tested end to end from the Mac through the Pi's gateway on 2026-08-03.

| Tier | Model | Rationale |
|---|---|---|
| `mechanical` | `deepseek/deepseek-v4-flash` | Config files, systemd units, codegen. Cheapest thing that reliably follows a literal spec. |
| `implement-standard` | `openrouter/qwen/qwen3-coder` | Normal coding against a written spec with tests supplied. This was AUTHOR-Swift on the prior redesign — proven on this codebase. |
| `implement-complex` | `gemini/gemini-3.1-pro-preview` | Binary decode, identity/ordering, single-writer discipline. Your pi default. |
| `review` | `claude` (CLI, not gateway) | Adversarial verification. `anthropic/*` is commented out of the gateway config, so this runs on the CLI subscription and costs nothing against metered credits. |
| `human` | — | Screwdrivers. |

**Escalation / fallback**, when a tier stalls or a provider rate-limits mid-run. Same task text,
different `--model`, nothing else changes:

`openai/gpt-5.6-sol` · `deepseek/deepseek-v4-pro` · `xai/grok-4.5` · `openrouter/moonshotai/kimi-k2`

Full catalog in `~/.pi/agent/models.json` (15 models): the five `openai/gpt-5.x` variants, two
Gemini, two DeepSeek, three Groq, two OpenRouter, and `xai/grok-4.5`.

**Verify an ID before a long run** rather than discovering it at dispatch time — the catalog is the
authority, not memory:

```sh
pi --model deepseek/deepseek-v4-flash -p "reply OK"
```

---

## 4. Spin up

```sh
cd ~/Source/AnxietyWatch
git checkout design/pi5-vitals-core
herdr                                                            # or: herdr --remote $PI_HOST
# then, inside a pane in this workspace:
herdr plugin action invoke spinup --plugin example.opsx-dev-layout
```

You get a tab labeled `opsx:pi5-vitals-core`:

```
┌──────────────────────┬──────────────┐
│ conductor (claude)   │ worker-1     │
│                      ├──────────────┤
├──────────────────────┤ worker-2     │
│ shell — task list,   ├──────────────┤
│ git status           │ worker-3     │
└──────────────────────┴──────────────┘
```

The shell pane greps `^- \[` out of `tasks.md`, so the 15-task index renders there as your board.

The kickoff prompt is **typed but not submitted** into the conductor — read it, then press Enter.

---

## 5. The dispatch loop

Each `### T-XX` block in `tasks.md` is a complete standalone prompt. Extract and send:

```sh
send_task() {                     # send_task T-D2 worker-1 gemini/gemini-3.1-pro-preview
  local id=$1 target=$2 model=$3
  local body
  body=$(awk "/^### $id /{f=1} f&&/^### T-/&&!/^### $id /{exit} f" \
         openspec/changes/pi5-vitals-core/tasks.md)
  herdr agent send "$target" "You are executing one task. Read ONLY the files in its Reads: list.
Acceptance is the Verify command's exit code, not your assessment. Model: $model

$body"
}

send_task T-D2 worker-1 gemini/gemini-3.1-pro-preview
herdr agent wait worker-1 --status idle
herdr agent read worker-1
```

Drop this in `~/.bashrc` on whichever box you drive from.

To pin a worker's model for a whole session instead, launch its pane with it:

```sh
herdr pane run <pane-id> "srt pi --model deepseek/deepseek-chat"
```

### 5.1 Verify yourself

**A worker reporting success is a claim. The exit code is the evidence.** Run the `Verify` line from
the task in the shell pane before ticking its box:

```sh
ssh $PI_HOST 'cd ~/AnxietyWatch2 && cargo test -p vitals-decode sfloat'
echo "exit=$?"
```

Only on `exit=0` do you tick `- [ ]` → `- [x]` in the task index.

### 5.2 Review gate

Tasks marked `Review required: yes` get a second pass before the box is ticked:

```sh
herdr agent send worker-3 "Adversarially review the working-tree diff for T-D2 against
openspec/changes/pi5-vitals-core/tasks.md section T-D2. Try to find an input that breaks it.
Report PASS, or a specific failing input. Do not confirm — attempt to refute."
```

Run that worker as `claude`, not `pi` — refutation is the one job worth the strongest model, and it
is free on subscription.

### 5.3 When a task fails

1. Re-dispatch to the **same** worker with the failure output appended — it has the context.
2. After two failures, escalate one tier and dispatch **fresh** (a stuck context rarely unsticks).
3. After escalation fails, the task is mis-scoped. Split it and record the split under
   `## Implementation notes`.

---

## 6. Gates

Three points where dispatching early destroys something. **Cross them only on a green verify you ran
yourself, never on a worker's report.**

**G1 — T-A4 needs T-A3 green.** `dtoverlay=disable-bt` removes the onboard controller. If the Sena
is not verified up as `hci1` first, the box ends with no Bluetooth at all.

```sh
ssh $PI_HOST 'hciconfig hci1 | grep -q "UP RUNNING"' && echo "G1 OPEN" || echo "G1 CLOSED"
```

**G2 — Group C needs T-A1 green.** Postgres must never initialize on the microSD.

```sh
ssh $PI_HOST 'findmnt -no SOURCE / | grep -q nvme' && echo "G2 OPEN" || echo "G2 CLOSED"
```

**G3 — T-D6 needs T-C3 green.** No sample worth keeping is written before a restore is demonstrated.

```sh
ssh $PI_HOST 'jq -e ".ok == true" /var/lib/vitals/restore-verify.json' && echo "G3 OPEN"
```

---

## 7. A realistic first session

Nothing in Group A can start until the NVMe and DAC arrive, but **Group D needs no hardware.**

```sh
# Mac, three workers in parallel — no hardware dependency
send_task T-D1 worker-1 openai/gpt-5.3-codex              # registry TOML + codegen
send_task T-D2 worker-2 gemini/gemini-3.1-pro-preview     # SFLOAT decoder + fuzz
herdr agent wait worker-1 --status idle
herdr agent wait worker-2 --status idle

# verify both yourself
python3 vitals/tools/gen_registry.py --check                       ; echo "D1 exit=$?"
cargo test -p vitals-decode sfloat                                 ; echo "D2 exit=$?"

# T-D3 consumes D1's generated header — dispatch after D1 is green
send_task T-D3 worker-3 gemini/gemini-3.1-pro-preview
```

That is Stage 3's foundation done before a single package arrives.

When the hardware lands: `herdr --remote $PI_HOST`, spin the layout there, run Group A in
order, and dispatch **T-B1 the moment T-A4 goes green** — it occupies a whole night, so start it and
let everything else proceed around it.

---

## 8. Cost control

- Workers use `LITELLM_WORKER_KEY` with hard daily/monthly caps. A runaway loop hits the cap, not
  your balance.
- Every task declares a **context budget** and an explicit `Reads:` list. The prior redesign lost a
  worker to a Groq 12k TPM cap on oversized prompts. If a worker starts reading the whole repo, the
  task is mis-scoped — stop it.
- `review` runs on the `claude` subscription and does not touch metered credits, so do not skimp on
  review passes.
- Watch spend: `http://pi-host:4000/ui`

---

## 9. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `pi` → `env: 'node': No such file` | bundled node off PATH | re-add the `pi-node/.../bin` line to `~/.bashrc` |
| Worker idles immediately, no output | model ID rejected by gateway | `pi --model <id> -p "reply OK"` |
| Shell pane task list empty | wrong workspace cwd | pane must be in `~/Source/AnxietyWatch` |
| `spinup` → "refusing to run" | `HERDR_ENV != 1` | invoke from *inside* a herdr pane, not a plain terminal |
| Gateway 401, `token_not_found_in_db` | the key was minted against a *different* LiteLLM database | Keys live in the gateway's own Postgres. A key from the Mac's DB is unknown to the Pi's. Mint one where the gateway runs: `cd ~/litellm && ./create-worker-key.sh 5 25` |
| Gateway 401 on the **master** key itself | `general_settings.master_key` in `config.yaml` is a literal and overrides `.env` | Set it to `os.environ/LITELLM_MASTER_KEY` so `.env` is the single source of truth, then `docker compose up -d --force-recreate litellm` |
| Provider says `bad-credentials` but the key is set | the env var exists but is **empty** — a failed terminal paste writes `VAR=` silently | `V=$(sed -n "s/^XAI_API_KEY=//p" ~/litellm/.env); echo "len ${#V}"` — zero means re-enter. Capture with `read -rs -p "key: " K; echo "captured ${#K} chars"` so a failed paste is visible immediately |
| Layout picks the wrong change | most-recently-modified wins | `OPSX_CHANGE=pi5-vitals-core herdr plugin action invoke spinup …` |
| Pi session gone after reboot | herdr server not enabled at boot | reattach with `herdr --remote $PI_HOST`; panes restore |

---

## 10. Conductor rules

Paste into the conductor if you override the default kickoff:

> You are the conductor for OpenSpec change `pi5-vitals-core`. Read `proposal.md`, `design.md`, and
> `tasks.md` in `openspec/changes/pi5-vitals-core/`.
>
> 1. Dispatch one task per worker. Never two concurrently to one worker.
> 2. **Run each task's `Verify` command yourself.** A worker's report is a claim; the exit code is
>    the evidence. Tick `- [ ]` → `- [x]` only after your own verify exits 0.
> 3. Never dispatch across gates G1, G2, or G3 (design.md §5) on a report — only on a green verify.
>    G1 in particular: dispatching T-A4 early leaves the box with no Bluetooth.
> 4. Tasks marked `Review required: yes` get an adversarial review pass before the box is ticked.
>    Prompt the reviewer to refute, not to confirm.
> 5. Implement a task yourself only if its tier is `mechanical`.
> 6. Stop and ask before anything destructive: repartitioning, `dtoverlay` edits, `pgbackrest`
>    restore into a live cluster.
> 7. Record scope deltas, measured results, and decisions under `## Implementation notes` in
>    `tasks.md` as they happen. T-B1's gap histogram goes there verbatim.
