# Design prompt — AnxietyWatch2 progress map

Paste everything below the line into Claude with the design/artifact feature enabled.

---

Build me a single self-contained HTML page: a **progress map** for a Raspberry Pi 5 medical
monitoring system I am building. It has to work as a wall-chart — something I open, glance at, and
immediately know where the project is and what is blocking it.

## What the system is

**AnxietyWatch2** — a Raspberry Pi 5 becoming the system of record for an entire personal health
platform: physiological capture, journaling, medications, prescriptions, pharmacies, lab results,
CPAP data, and an overnight CNS-depression alarm.

It is the second generation. Generation one is an iOS + watchOS app plus a Flask/Postgres server on
a machine called *megadude*. Both are being consolidated onto the Pi. The phone survives as a
HealthKit bridge, a journal, and a viewer — it stops computing baselines, stops running detection,
and stops holding BLE sessions.

The **nocturnal CNS-depression monitor is the hardest subsystem, not the whole project.** It is
being built first for two reasons: it is the part where failure has real consequences, and every
other subsystem needs the storage and ingest foundation it establishes.

Title the page **"AnxietyWatch2 — platform migration"**, not "CNS monitor".

This is not a hobby dashboard. Design it like clinical instrumentation, not a startup landing page.

## The structure: 9 stages, 15 tasks (so far), 3 gates

Stages run 0→8. Stages 0–5 each produce an artifact you can open in a browser, and each artifact
becomes a panel of an admin console that assembles as the project progresses. That compounding is
the most interesting thing about the plan and worth showing visually.

```
Stage 0  Hardware floor    → system-report.html  → becomes the "Host" panel
Stage 1  Thesis spike      → night1.html         → becomes the "Radios" panel
Stage 2  Storage floor     → recovery.html       → becomes the "Recovery" screen
Stage 3  vitals core       → first-sample.html   → becomes the "Ingest" panel
Stage 4  BLE fleet         → the live Overview console
Stage 5  Detector + klaxon → replay-report.html + THE ALARM ACTUALLY SOUNDING
Stage 6  Migrate megadude  → journal, medications, prescriptions, pharmacies, labs,
                             songs, CPAP/AS11, Oura cloud — the Flask API and admin
                             UI move into Docker on the Pi. megadude is decommissioned
Stage 7  Thin the phone    → iOS keeps HealthKit reads, journaling and viewing;
                             stops baselines, detection, BLE. Services/ 21k → ~3k lines
Stage 8  Bedside panel     → a fullscreen 800×480 status light, a viewer that is
                             never a dependency
```

**Two things the layout should make clear.** Stage 5 is where the safety-critical spine completes —
the alarm actually sounds. Stages 6–8 are where it becomes the whole platform rather than one
alarm. Neither half is "the project"; the page should not imply the work ends at Stage 5.

Only Stages 0–3 are specified to task level today (the 15 tasks below). Stages 4–8 exist in the
design document but have no task breakdown yet — show them as planned, distinctly from the
tasks-and-gates detail.

### Tasks and current status

Use exactly these states: **done**, **in progress**, **ready** (unblocked, not started),
**blocked-by-gate**, **blocked-on-hardware**.

| Task | Stage | Status | Note |
|---|---|---|---|
| T-A1 Migrate root to NVMe | 0 | blocked-on-hardware | M.2 HAT arrives Aug 5; NVMe on hand. **Unblocks Aug 5** |
| T-A2 Audio output path | 0 | blocked-on-hardware | USB DAC + speaker arrive Aug 5–6. **Unblocks Aug 6** |
| T-A3 Bring up Sena UD100-G03 | 0 | **done** | gate G1 owner |
| T-A4 Disable onboard Bluetooth | 0 | **done** | Sena is now the only adapter |
| T-A5 Watchdog, governor, OOM | 0 | **done** | found Docker OOM defaults backwards |
| T-B1 Eight-hour Polar H10 capture | 1 | **ready** | the foundational measurement |
| T-C1 PostgreSQL on NVMe | 2 | blocked-by-gate | G2 |
| T-C2 pgBackRest, three tiers | 2 | blocked-by-gate | G2 |
| T-C3 Scheduled restore verification | 2 | blocked-by-gate | G2 · gate G3 owner |
| T-D1 Registry TOML + codegen | 3 | **done** | merged |
| T-D2 IEEE-11073 SFLOAT decoder | 3 | **done** | merged, fuzzed |
| T-D3 COPY-binary encoder + spool | 3 | **done** | round-trip verified against real Postgres |
| T-D4 vitalsd daemon + socket | 3 | **ready** | unblocked now |
| T-D5 Spool drainer | 3 | blocked-by-gate | G3 |
| T-D6 bp-collector end to end | 3 | blocked-by-gate | G3 |

### The three gates

Gates are the spine of this plan and deserve real visual weight — they are refusals to proceed, not
checkpoints. Each should read as a locked door with its reason stated.

- **G1 — "Verify the new radio before removing the old one."**
  Status: **OPEN** (passed). Disabling the onboard Bluetooth removes the only other adapter; doing
  it before the dongle was proven would have left the box with no Bluetooth at all.
- **G2 — "Postgres must never initialize on the microSD."**
  Status: **CLOSED**, opens Aug 5 when the M.2 HAT lands. Write endurance. Blocks all of Stage 2.
- **G3 — "No sample worth keeping is written before a restore has been demonstrated."**
  Status: **CLOSED**, downstream of G2. A backup that has never been restored is a hypothesis, not a
  backup. Blocks T-D5 and T-D6.

### Hardware

**Working:** Pi 5 (8 GB) · Sena UD100-G03 long-range BLE · Polar H10 · EMAY SleepO2 pulse oximeter ·
A&D UA-651BLE blood pressure cuff · Hosyond 7" DSI touchscreen · ResMed AirSense 11

**In transit** (ordered 2026-08-04): M.2 HAT + speaker → **Aug 5** · UPS + case + USB DAC → **Aug 6**
· alerting chain, shaker/piezo/MOSFETs/12 V supply → **Aug 6** · active cooler → Aug 10–13

**Still to source:** 4× 18650 cells (local) · RTC battery · TPM 2.0 module · 60 GHz respiration radar

Make it obvious that **the critical path is now a date, not a missing part**. The M.2 HAT lands Aug 5
and closes G2, which closes G3, which unblocks the last two tasks of Stage 3 — so Stage 2 and the
rest of Stage 3 all hinge on one delivery. That single dependency chain, and the fact that it
resolves this week, is the most decision-useful thing on the page.

### Milestones already reached

Give these a small, quiet "log" treatment — dated, factual, not celebratory:

- **2026-08-03** — Bluetooth stack proven: dongle verified genuine by OUI, onboard radio disabled,
  survives reboot
- **2026-08-03** — **First real physiological data through the system.** Four blood-pressure
  measurements decoded live off the A&D cuff over standard BLE `0x2A35`
- **2026-08-03** — Hardware floor: watchdog armed (15 s), CPU governor pinned, unattended reboots
  disabled
- **2026-08-03** — Six design amendments accepted; task specs reconciled against them
- **2026-08-04** — Bedside panel live on the Pi: real EMAY SpO₂ and pulse at 1 Hz, eight swipeable
  views, self-healing across cold boots and wifi drops
- **2026-08-04** — Hardware ordered against a costed plan. The two Stage-0 hardware blockers clear
  Aug 5–6

## Design direction

**Not a Gantt chart.** Time is not the useful axis here — dependency is. Something closer to a
subway map, a circuit diagram, or a pipeline with valves. I want to see *what unblocks what*.

**Palette.** Deep instrument-panel neutrals. Accent hues already validated for this project:
`#7D87D0` (periwinkle), `#B8942A` (brass), `#D93E36` (signal red). **No brown-centred palette.** Red
is reserved for the alarm path and blocking states — never decoration.

**Accessibility is a hard requirement, not a nice-to-have:**
- All text meets WCAG 2.2 AA contrast (4.5:1 body, 3:1 large). State the ratios in a comment.
- **Never encode status by colour alone** — every state needs a shape, icon, or label too. Verify the
  five states remain distinguishable under deuteranopia and protanopia.
- Base font ≥ 16px, generous line height, no text below 14px anywhere.
- Semantic HTML with real landmarks and a logical heading order.

**Tone.** Precise and slightly severe. Think aviation checklist or anaesthesia monitor. Dense with
information but never cluttered — every element earns its place. No gradients-for-decoration, no
drop shadows for depth, no rounded-corner friendliness. Hairline rules and disciplined spacing.

**Honest states.** A blocked task must not look like a failed one, and an unknown must never look
like a healthy one. That distinction is a core design rule of the underlying system — a status that
cannot express "not measured" is how a missing alarm becomes invisible. Carry it into the visual
language.

Self-contained: one HTML file, inline CSS, no external fonts or scripts. It should print legibly on
one page in monochrome.
