# Building AnxietyWatch2 with herdr + pi

Everything you need to take this project from an empty skeleton to a Pi that detects and alarms on
its own. Read §1–§4 once; §5 onward is reference.

| Doc | What it is |
|---|---|
| `docs/specs/2026-08-03-pi5-vitals-core-design.md` | Architecture and rationale. The *why*. |
| `openspec/changes/pi5-vitals-core/proposal.md` | Scope of the first change (Stages 0–3) |
| `openspec/changes/pi5-vitals-core/design.md` | Execution contract — tiers, verification, gates |
| `openspec/changes/pi5-vitals-core/tasks.md` | 15 dispatchable task briefs |
| `openspec/changes/pi5-vitals-core/RUNBOOK.md` | herdr operator reference |
| **this file** | How it all fits together, plus recommendations |

---

## 1. The one thing to understand first

**The Mac is the editor. The Pi is the computer.**

This project targets Linux on arm64. Your Mac can compile and test a meaningful *subset* — and that
subset is worth having, because it iterates in seconds — but a Mac build produces Mach-O, which will
not execute on the Pi at all.

The split is encoded in `vitals/CMakeLists.txt` as two libraries, and it maps exactly onto the
`host:` tag already on every task:

| Library | Contains | Builds on | Task tag |
|---|---|---|---|
| `vitals_core` | SFLOAT decode, `0x2A35` / `0x2A37` decode, HLC | macOS **and** Linux | `host: either` |
| `vitals_linux` | SQLite store, `vitalsd`, epoll, BlueZ, WAL tail | **Linux only** | `host: pi` |

One genuine piece of luck: Apple Silicon and the Pi 5 are both **arm64 little-endian**, so byte
layout, struct packing, and integer widths agree. A decoder that passes on your Mac behaves
identically on the Pi. That is not true of most cross-platform C work and it is why splitting the
pure logic out is worth the small extra structure.

Configure on a Mac and the Linux-only targets are simply absent — CMake prints a note and moves on.
Nothing is faked. A mocked BlueZ would only ever test the mock.

**Anything Linux-specific — including debugging — happens on the Pi.** §4 makes that a one-key
operation rather than a chore.

---

## 2. First-time setup

### 2.1 Verify the harness

```sh
herdr --version     # 0.7.5
pi --version        # 0.83.0
claude --version    # 2.1.220
```

Both machines have all three.

### 2.2 The gateway

One LiteLLM gateway, on the Pi, because the Pi is the box that is always on. A sleeping laptop must
not be brokering model calls for an overnight capture.

```sh
curl -s http://192.168.1.48:4000/health/readiness    # {"status":"healthy","db":"connected"}
```

Your Mac's `~/.pi/agent/models.json` already points there and shares the Pi's worker key.

**Add provider keys on the Pi only** — both machines then get them:

```sh
ssh chris@minidude && cd ~/litellm && ./add-provider-key.sh
```

Running that on the Mac edits a `.env` no gateway reads. It reports success and changes nothing.

### 2.3 Build locally, once

```sh
cd ~/Source/AnxietyWatch2
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build && ctest --test-dir build --output-on-failure
```

The `toolchain` test exists to prove exactly this before any real task runs: C11 compiles,
warnings-as-errors passes, sanitizers link, CTest reports an exit code. If it fails, no task result
can be trusted.

### 2.4 Build on the Pi, once

```sh
scripts/sync-to-pi.sh --test
```

Verified working, producing `ELF 64-bit LSB pie executable, ARM aarch64`.

---

## 3. Model tiers

Tasks name a **capability tier**, never a vendor. This table is the only place models appear — when
they change, edit here and nothing else. All IDs live-tested through the Pi's gateway.

| Tier | Model | Used for |
|---|---|---|
| `mechanical` | `deepseek/deepseek-v4-flash` | Config, systemd units, codegen. Cheapest thing that follows a literal spec. |
| `implement-standard` | `openrouter/qwen/qwen3-coder` | Coding against a written spec with tests supplied. Was AUTHOR-Swift on the prior redesign. |
| `implement-complex` | `gemini/gemini-3.1-pro-preview` | Binary decode, HLC ordering, single-writer discipline. |
| `review` | `claude` CLI | Adversarial verification. Runs on subscription — costs nothing against metered credits. |
| `human` | — | Screwdrivers. |

Escalation when a tier stalls or a provider rate-limits: `openai/gpt-5.6-sol` ·
`deepseek/deepseek-v4-pro` · `xai/grok-4.5` · `openrouter/moonshotai/kimi-k2`.

Verify an ID before a long run — the catalog in `~/.pi/agent/models.json` is the authority, not
memory:

```sh
pi --model deepseek/deepseek-v4-flash -p "reply OK"
```

---

## 4. Remote debugging — the part that matters

For `host: pi` work you want breakpoints, not print statements. Two ways.

### 4.1 CLion remote toolchain (recommended)

Edit on the Mac, build/run/debug on the Pi, with full IDE debugging. Set up once:

1. **Settings → Build, Execution, Deployment → Toolchains → + → Remote Host**
   Credentials: `chris@minidude`, key-based. CLion auto-detects `gcc`, `gdb`, `cmake`, `ninja`
   (all installed).
2. **Settings → Deployment** — the mapping is already in `.idea/deployment.xml`:
   `$PROJECT_DIR$ → /home/chris/AnxietyWatch2`, with `build*`, `.git`, `.idea` excluded.
3. **Settings → CMake** — add a profile using the remote toolchain.

Now `Run → Debug` compiles on the Pi, launches under `gdbserver`, and gives you breakpoints,
watches, and stepping in the IDE while the process runs on the Pi with real radios attached. This is
the only way to debug a BLE collector, because the bug needs the hardware.

### 4.2 Command line

```sh
scripts/sync-to-pi.sh --test        # sync, build, ctest
ssh chris@minidude
cd ~/AnxietyWatch2 && gdb ./build/vitals/test_bp_decode
```

Or drive `claude` / `pi` directly on the Pi where the logs and radios are local — see §6 Mode B.

**Never** try to reproduce a BlueZ, epoll, or systemd problem on macOS. The APIs do not exist there.

---

## 5. Test coverage

**Gate: 95% on pure logic, 85% on store/daemon logic, hardware seam measured but not gated.**

```sh
scripts/coverage.sh            # report
scripts/coverage.sh --check    # enforce; nonzero exit on failure
```

A single flat number would be the wrong shape here, in both directions. The BlueZ/D-Bus glue cannot
be exercised without real radios, so gating on it would either block merges forever or push someone
to write tests that assert nothing. Meanwhile the decoders are pure functions with no state and no
I/O — 90% there would be *too low*, because the untested 10% is where a malformed radio frame walks
off the end of a buffer.

So the bar is highest exactly where testing is cheapest and the consequences are worst:

| Layer | Gate | Why |
|---|---|---|
| Decoders, HLC | **95%** | Pure functions. No excuse, and hostile input lands here. |
| Store, daemon logic | **85%** | Testable against `:memory:` SQLite. |
| BLE / systemd glue | measured only | Needs real radios. Covered by the T-B1 overnight run instead. |

**Coverage is a floor, not the goal.** Every byte-parsing function also gets a libFuzzer target,
because coverage tells you a line ran, not that it survived a truncated frame. CI runs each fuzzer
for 60 s per push.

---

## 6. Running the work

```sh
cd ~/Source/AnxietyWatch2
herdr                                   # or: herdr --remote chris@minidude
herdr plugin action invoke spinup --plugin chris.opsx-dev-layout
```

Conductor + 3 workers + a shell pane showing the 15-task index.

**Mode A — Mac pool** (default). Workers on the Mac. Every `Verify` in `tasks.md` is written as
`ssh chris@minidude '…'`, so `host: pi` tasks still execute remotely.

**Mode B — Pi-local.** `herdr --remote chris@minidude`. Panes run *on the Pi*, survive
disconnection, and are still there when you reattach from a phone. Use for all of Group A and the
overnight T-B1 capture.

Dispatch — the `send_task` function is in `RUNBOOK.md` §5:

```sh
send_task T-D2 worker-1 gemini/gemini-3.1-pro-preview
herdr agent wait worker-1 --status idle
```

Then **run the Verify command yourself.** A worker reporting success is a claim; the exit code is
the evidence. Tick the box only on your own `exit=0`.

### 6.1 Adding a task's files to the build

Each `T-D*` task names the files it creates. `vitals/CMakeLists.txt` has commented placeholders for
every one — uncomment the matching lines and nothing else changes:

```cmake
set(VITALS_CORE_SOURCES
    src/version.c
    src/sfloat.c        # T-D2   ← uncomment when the task lands
)
vitals_test(sfloat)     # T-D2   ← and its test
vitals_fuzz(sfloat)     # T-D2   ← and its fuzzer
```

Each task's Verify line then works unchanged: `ctest -R sfloat` runs exactly that task's suite.

---

## 7. Recommendations

**Start with T-D1 and T-D2 tonight.** Neither needs hardware. T-D1 (registry codegen) unblocks
everything downstream; T-D2 (SFLOAT decoder) is the subtlest code in the change and the easiest to
prove correct. Two workers, in parallel, right now.

**Do T-B1 before you trust any of this.** The eight-hour Polar H10 capture is deliberately placed
before the infrastructure. The whole project rests on the Pi holding a BLE connection all night — if
the gap histogram is bad, better to know before Postgres, `vitalsd`, and the klaxon are built on top
of it. It costs one night and it is the single highest-information thing you can do.

**Respect gate G1.** `dtoverlay=disable-bt` before the Sena is verified as `hci1` leaves the box
with no Bluetooth at all, and you will be fixing it over a keyboard attached to the panel.

**Order the two missing parts.** M.2 HAT + NVMe (Postgres, and gate G2 blocks on it), and a USB DAC
+ amp + speaker. The Pi 5 has no analog audio, so today the alarm has no way to make a sound — the
one component the entire project exists to trigger.

**Write the decoder tests before the decoders.** They are pure `(bytes, len) → result` functions,
which is the easiest possible thing to test-drive, and the task briefs already contain the test
cases. Uncomment the `vitals_test(...)` line first and watch it fail to link.

**Let review be generous.** It runs on the Claude subscription, not metered credits, so an extra
adversarial pass costs nothing. Prompt reviewers to *refute*, never to confirm — "find an input that
breaks this" produces different behavior than "check this."

**Keep the registry the single source of truth.** The moment a `source` or `type` integer is typed
by hand somewhere instead of coming from `registry.toml` codegen, the drift class of bug is back.
CI's `registry-is-generated` job exists to make a hand-edited generated file fail the build.

**Record T-B1's histogram verbatim** in `tasks.md` under `## Implementation notes`. It is the
project's foundational measurement and you will want to compare against it later.

---

## 8. Repository layout

```
AnxietyWatch2/
├── CMakeLists.txt              root build; sanitizers, coverage, fuzz options
├── vitals/
│   ├── CMakeLists.txt          the portable/Linux split; task placeholders
│   ├── include/vitals.h        public surface
│   ├── src/                    implementation
│   ├── tests/                  one CTest target per task
│   ├── fuzz/                   one libFuzzer target per radio decoder
│   ├── collectors/             per-radio daemons (Linux only)
│   ├── tools/                  gen_registry.py (T-D1)
│   ├── generated/              codegen output — committed, never hand-edited
│   └── registry.toml           THE source of truth for sources, types, units
├── deploy/
│   ├── systemd/                unit files
│   └── postgres/               schema, tuning, pgBackRest config
├── scripts/
│   ├── sync-to-pi.sh           push + build + test on minidude
│   └── coverage.sh             instrumented build, report, threshold gate
├── docs/
│   ├── HERDR-GUIDE.md          this file
│   ├── specs/                  architecture
│   └── design/                 UI design + prompts, bedside panel mockup
├── openspec/changes/pi5-vitals-core/
│   ├── proposal.md · design.md · tasks.md · RUNBOOK.md
└── .github/workflows/ci.yml    build+test · registry-is-generated · fuzz · coverage
```

---

## 9. Quick reference

```sh
# local
cmake -B build -DCMAKE_BUILD_TYPE=Debug && cmake --build build
ctest --test-dir build --output-on-failure
ctest --test-dir build -R sfloat            # one task's suite

# on the Pi
scripts/sync-to-pi.sh --test

# coverage
scripts/coverage.sh --check

# fuzz
cmake -B build-fuzz -DVITALS_FUZZ=ON -DVITALS_SANITIZE=OFF -DCMAKE_C_COMPILER=clang
cmake --build build-fuzz && build-fuzz/vitals/fuzz_sfloat -max_total_time=60

# gates
ssh chris@minidude 'hciconfig hci1 | grep -q "UP RUNNING"'          && echo G1 OPEN
ssh chris@minidude 'findmnt -no SOURCE / | grep -q nvme'            && echo G2 OPEN
ssh chris@minidude 'jq -e ".ok==true" /var/lib/vitals/restore-verify.json' && echo G3 OPEN

# harness
herdr --remote chris@minidude
herdr plugin action invoke spinup --plugin chris.opsx-dev-layout
herdr agent list
```
