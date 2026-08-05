# Home Assistant architecture — patterns to adopt, machinery to avoid

**Date:** 2026-08-05 · **Status:** research findings; the normative subset is folded into
[`SPEC-admin-console.md`](SPEC-admin-console.md) §8b
**Companion:** [`HOMEASSISTANT-PRIOR-ART.md`](HOMEASSISTANT-PRIOR-ART.md) covers *which integrations*
to translate; this document covers *how HA is built* and what of that transfers to a single-user
Rust health appliance.

**Provenance.** Claims marked *[verified]* were read directly from a sparse clone of
`home-assistant/core` @ `9c8ae126` (dev, 2026-08-04, v2026.9.0.dev0) and the
`home-assistant/architecture` ADR repo. Counts marked *[computed]* were calculated over those
clones. Everything else is from developers.home-assistant.io or linked discussions.

---

## 1. The headline conclusion

HA's architecture is a tax paid for openness — 1,481 integrations, 735 contributors, 662,275
strangers running code on hardware nobody controls. Almost every subsystem is **mechanism
substituting for trust**: the manifest/hassfest apparatus, the quality scale, the runtime blocking-IO
watchdog, the hardcoded custom-integration kill-list, the deprecation-shim treadmill (225 live
`breaks_in_ha_version` warnings *[computed]*). This project has trust — it writes all its own code.

**Copy HA's measurements and its scars. Copy almost none of its machinery.** The measurements: BLE
constants, the 16 dB source-switch threshold, the learned-interval staleness algorithm, the 10%
counter-reset rule, the 16 KiB attribute cap. The scars: typed values, retention decided at schema
time, one database engine, bounded queues, freshness as data.

---

## 2. Availability and state semantics — the part that bears on the governing law

This was the primary research question, and the answer is sharper than HA's docs suggest.

### 2.1 The rule: transport vs content

- **`unavailable`** = "I cannot reach the thing." Set by the integration via `available → False`.
- **`unknown`** = "I reached the thing; this particular datum is absent." Set by returning `None`.

Implemented in one function *[verified, `helpers/entity.py:1063`]*: `available` is checked first and
short-circuits — an unavailable entity can never report `unknown`, and its value property is never
consulted. When unavailable, all *dynamic* attributes are dropped but *descriptive* ones
(unit, device_class, friendly_name) survive — keep what describes the entity, drop what measures it.
Corollary for any consumer: never key "is this alive" off attribute presence.

### 2.2 Automatic staleness detection: there is none

Confirmed three ways *[verified]*: core never sets availability itself; no timer in the entity
helpers relates to availability; and **a failed poll writes no state at all** — it logs and returns,
leaving the previous value in the state machine **looking perfectly healthy, forever**. Across 1,481
integrations, exactly **two** implement an expiry timeout: `mqtt` (`expire_after`) and `lacrosse`.

This is precisely the defect the admin-console spec's §2.1 indicts in gen-1 — the largest
home-telemetry project in the world ships it as its default behaviour. Independent confirmation that
"liveness as a computed property with a declared cadence" has to be built into the core, because
left to integrations it appears 2 times in 1,481.

### 2.3 What HA does have, and what to take from each

**(a) Three timestamps** *[verified, `core.py`]*:

| Field | Advances when |
|---|---|
| `last_changed` | the state value changes |
| `last_updated` | state or attributes change |
| `last_reported` | the state is written at all, **even byte-identical** |

`last_reported` was added in 2024 because HA used to *discard* no-op writes — a sensor reporting
21.0 °C every 30 s for an hour produced one row, and "steady" was indistinguishable from "dead."
**`now − last_reported` is the correct staleness metric**: the last time the producer affirmatively
asserted the value. And because a failed poll writes nothing, it degrades correctly.
→ **Adopt:** `reported_at` distinct from `changed_at`, first-class on every reading and attempt.
Skip the archaeology; make freshness a required field, not a derived query.

**(b) The learned-interval engine** — the only real staleness engine in HA, living in the Bluetooth
stack, not the core. `AdvertisementTracker` buffers inter-arrival times until it has 16 samples,
stores the **max** observed gap as the device's interval, and calls the device unavailable at
`interval + 5 s` wobble — falling back to 15 min only when the interval is unknown. Three details
make it work: **clear the timing data whenever the radio pauses to connect** (else the connection
gap is measured as a huge advertising interval), **reset on source change**, and a per-device
override — BTHome's `sleepy_device` flag short-circuits unavailability entirely for devices whose
normal state is silence. → **Adopt for BLE collectors** as a refinement layered on the declared
cadence (the registry cadence remains the floor; a learned interval may tighten it, never loosen it
past declaration). The sleepy override is exactly what an overnight oximeter needs.

**(c) Transition logging discipline.** Every failure log is gated on `if last_update_success:` —
one line on the way down, **total silence while the failure persists**, one line on recovery. The
same guard appears in four independent places in core. → **Adopt everywhere.** Cheapest
observability win available; a journal that repeats a known failure at every poll buries the next
new one.

**(d) Startup placeholder state**: before an integration's first update, every known entity is
`unavailable` + `restored: true` — never `unknown`. The flag discriminates "placeholder" from "the
producer is up and says it's unreachable." → Same distinction the console needs between "never
observed" and "check itself did not run"; both are `UNKNOWN`, but the reason must be preserved.

### 2.4 Where the spec's model is strictly stronger

HA has two words for five facts. For a klaxon-grade consumer, "sensor says 88%", "sensor says
nothing", "sensor said 96% forty minutes ago", "finger not on sensor", and "never seen" are five
different states; HA collapses them into two. The CNS quality-gate asymmetry (indeterminate ≠ safe ≠
dangerous) goes beyond what HA ships. In Rust the state should be a sum type —
`Fresh(v) | Stale(v, age) | Invalid(reason) | Unreachable | NeverSeen` — not a string vocabulary.
(Non-normative: the console's health axis remains the spec's three states; the richer sum type is
collector-side representation that the evaluator collapses.)

---

## 3. The coordinator pattern — worth reading, partially worth taking

`DataUpdateCoordinator` solves "one endpoint serving many entities" — one periodical poll, fan-out
to subscribers. Specifics worth keeping *[verified]*:

- **Jitter on every scheduled refresh** (0.05–0.50 s random) so N pollers don't resonate. Config-entry
  retry backoff jitters too. → Adopt on every scheduled repeat.
- **Debounced refresh requests**: first fires immediately, further requests within 10 s collapse into
  one trailing execution.
- **Polling is listener-gated**: a coordinator with zero subscribers does not poll.
- **The coordinator itself does NOT back off** — a 30 s coordinator against a dead device polls every
  30 s forever. Backoff lives one level up, in config-entry setup retry: `min(2^tries × 5, 600)` s.
  Auth failure stops polling dead until reauth. → The separation is right: *fetch cadence* is a
  declared property; *setup retry* is where exponential backoff belongs.
- **Evidence beats a timer**: if an entry is in setup-retry backoff and a discovery source sees the
  device, HA schedules an **immediate** reload instead of waiting out the timer. → Adopt: data
  arriving from a source in backoff short-circuits the backoff.
- **Write suppression**: on the second consecutive failure, entities are not re-notified; listeners
  fire on success-state *transitions* or data changes.
- Exception → state mapping is explicit and typed: transient (`ConfigEntryNotReady`) retries with
  backoff; auth failure starts reauth; `ConfigEntryError` is terminal. → Adopt the taxonomy: every
  collector failure is transient / needs-credentials / terminal, and they are handled differently.

**Not worth taking:** the machinery itself (listener registry, context narrowing, Python inheritance
shape). The collector layer decision is deferred to Stage 4 (spec §9c) — the recommendation this
research supports is *borrow the behaviours above; do not model the layer on HA's shape*.

---

## 4. The registry pattern worth stealing outright

`RegistryEntry` stores **user overrides separately from integration-supplied values** *[verified]*:
`name` vs `original_name`, `device_class` vs `original_device_class`, `display_precision` vs
`suggested_display_precision`. The producer writes only the `original_*`/`suggested_*` side; the
operator writes the bare side; display resolves `user_value or original_value`. A source can change
its defaults in a future version **without stomping operator customisation**, and a rename survives
every upgrade. Costs one extra column per overridable field; removes a whole class of upgrade
regressions. → Adopt for operator-editable display metadata (source display names, units,
precision) in the console's Config section.

Also noted: HA's `event` platform models momentary occurrences (state = timestamp of last event,
payload as attributes) — added specifically because modelling events as level-sensors was a
recurring mistake. A desaturation episode, an apnea, a klaxon fire are events with timestamps, not
sensors with levels.

---

## 5. Statistics semantics — the four decisions to copy (not the system)

*[verified in `recorder/`]*

1. **Time-weighted means, not simple averages** — each value weighted by its duration until the next
   change. Correct for irregular sampling; the right default for overnight SpO₂.
2. **Non-numeric states contribute nothing.** `unavailable`/`unknown` are skipped, not zeroed — an
   outage leaves a **gap**; it does not drag the mean or register as a counter reset. (The same
   invariant the CNS pipeline already encodes; HA arrived at it independently.)
3. **Counter reset detection is a 10% heuristic**: a drop <10% is noise (warn only); ≥10% is a real
   reset; negative raises. New zero-point after a reset is 0, not the dropped value.
4. **Retention asymmetry decided at the schema level**: raw states purge at 10 days; hourly
   aggregates are kept forever, and the purge job *does not know the aggregate table exists*.
   (Amendment 01 already made the equivalent choice: raw 2-year retention + continuous aggregates.)

Two storage-layer numbers worth keeping: intern hot strings (HA's `states_meta` normalisation cut
the hottest table 18% and index keys from ~1 KB to 14 bytes), and hard-cap any variable-size blob
before it reaches storage (HA: 16 KiB, chosen because legitimate payloads maxed at 1,760 bytes).

---

## 6. The Bluetooth stack — the most valuable subsystem, no Rust equivalent exists

`btleplug` ≈ `bleak`: transport only. Everything below is operational knowledge HA's stack encodes
above the transport, and it will otherwise be relearned failure by failure. *[all verified]*

### 6.1 The adapter recovery ladder

`bluetooth-auto-recovery`, in order: (1) resolve the adapter **by MAC, not `hciN`** — indices move;
(2) rfkill check/unblock, distinguishing soft block (recoverable) from hard block, then **poll** for
the kernel to clear it (grace 4.5 s, poll 1.5 s); (3) power-cycle over the BlueZ **management
socket**, not D-Bus (off 2 s, on 3 s); (4) if the adapter went silent, **USB port reset** forcing
re-enumeration; (5) poll for reappearance (3 attempts × 2 s — a Pi can exceed a single wait), then
re-check rfkill because a reset can re-block it. Budget 120 s for firmware-load fallback on some
kernels. → Port this ladder to the Pi collector supervisor.

### 6.2 The wedge detector

Scanner watchdog: 90 s timeout, 30 s check interval, sized against BlueZ's 180 s device-removal
window. Critically: **empty advertisements do not count as detections** — a wedged adapter emits
empty adverts, which would otherwise feed the watchdog forever.

### 6.3 Connection management

- Two independent retry budgets: **4 attempts for real failures, 9 for transient** — classified by
  error string (bleak has no typed errors).
- **Backoff is error-classified, not exponential**: out-of-slots 4 s · transient 0.25–1 s ·
  generic 0.1 s · clean disconnect 0 s (retry immediately).
- **Slot exhaustion presents as "device not found"** on BlueZ — the device object vanishes from
  D-Bus. Naive code reports "device gone" forever. Measured: CSR8510 = 5 slots, BCM20702 = 7;
  there is no way to query the limit.
- **Wait for the peer to actually disconnect before reconnecting** (watch the `Connected` property,
  5 s timeout) — reconnecting into a half-torn-down link reliably manufactures
  `le-connection-abort-by-local`.
- **Reconnection is on-demand and advertisement-triggered. There is no reconnect loop.** Connect
  when an advertisement proves the device is awake and in range; never poll a BLE device on a wall
  clock.

### 6.4 Multi-adapter arbitration (relevant once the Pi has USB adapters + onboard)

Per-(address, source) EWMA on RSSI (α = 0.3); a challenger must beat the owner by **16 dB**, and a
just-demoted source needs 16 + 6 dB to reclaim (asymmetric hysteresis kills ping-pong). **Missing
RSSI is never folded into the average** — it would manufacture phantom weak readings. A stale owner
loses the device only on *evidence*, not on one missed advertisement ("a missed advertisement is a
per-scanner reception lottery, not evidence the device moved").

### 6.5 The architectural point

**BLE scanning is a singleton system resource.** Two scanners in one process fight over BlueZ
discovery state. HA enforces this by monkey-patching the library globally; in Rust it comes free —
one owned scanner task with a `broadcast::Sender<Advertisement>`, and no public API that can start a
second scan.

### 6.6 Passive scanning

BlueZ `AdvertisementMonitor1` with hardware-offloaded filters needs kernel ≥ 5.15.62 and BlueZ
≥ 5.63 (experimental). A monitor with zero patterns won't start — ship a match-all pattern set.
Trap: a passive capture of a scan-response-only sensor is a *valid advertisement carrying stale
data*; active scanning is required for anything the peripheral puts in `SCAN_RSP`.

---

## 7. The phone-ingestion shape (validates the write-API design)

The HA companion app is not a special case — it is a webhook. Register once (authenticated) →
receive an id → thereafter batched unauthenticated-but-encrypted posts. **Sensors are declared once
(`register_sensor`: name, type, unit, device_class frozen at registration) and thereafter updated in
batch where only the value and attributes are mutable.** The register-once / update-many split keeps
the receiving schema stable, and libsodium `secretbox` covers confidentiality over plain HTTP. This
is the envelope `HOMEASSISTANT-PRIOR-ART.md` §1 recommends reimplementing (~200 lines server-side).

---

## 8. What NOT to copy — each with the receipt

1. **The stringly-typed state machine.** `state` is always a string capped at 255 chars; an
   over-length value silently becomes `unknown` — data loss dressed as a log line. The cap exists
   because every state change fans out to every websocket client. Downstream scars: a 16 KiB
   attribute guillotine, hash-deduplicated attribute storage (82–88% of payloads were byte-identical),
   three overlapping timestamps that integrations use wrong. For a health appliance this is actively
   dangerous: `invalid`, `missing`, and `stale` collapse into the same two magic words. → Typed
   values, sum-type status, units and provenance as fields. Already the design; keep it that way.
2. **UI-driven configuration (`.storage/`).** ADR-0010 was correct *for HA* — 662k non-technical
   users can't edit YAML. The cost: configuration became a machine-owned JSON graph with cross-file
   foreign keys that can never be selectively version-controlled or partially restored (HA
   maintainers' own verdict: "the juice not worth the squeeze"). A TOML registry in git gives diffs,
   review, blame, and rollback — properties HA traded away and cannot recover. 20.5% of HA
   integrations are still YAML-only six years later.
3. **A generic event-log recorder over pluggable SQL backends with retention decided afterward.**
   53 schema migrations, three generations of dead columns that can't be dropped, an ADR conceding
   multi-backend support "is impossible to test," and — the disqualifying one — **under load the
   recorder silently stops persisting to protect the process.** For an appliance where the data is
   the product, that is exactly backwards: bound the queue and apply backpressure or spill to disk;
   never go silently mute. (One engine, typed schema, hand-written migrations, retention at design
   time — all already chosen in Amendment 01.)
4. **The third-party-safety apparatus.** Quality tiers (only 12 of 1,481 integrations reach Gold;
   26.9% have zero code owners *[computed]*), the runtime blocking-IO watchdog, the kill-list of
   custom integrations that have crashed production, dependency fiat (1,133 exact-pinned packages,
   conflicts resolved by decree), four install methods since amputated to two. Every one exists
   because HA cannot trust the code it runs. Irrelevant here.

One HA rule that *does* transfer despite being trust-machinery: **protocol parsing lives in a
separate library with zero framework imports and zero I/O** — bytes in, typed values out. The HA
reason is reviewability; the reason it transfers is testability and blast radius (a parser fix is a
version bump, not a core change). The Rust form: decoder crates (`oura-ble`, `emay-sleepo2`,
`polar-h10`) are `no_std`-ish pure `&[u8] → Result<Reading, ParseError>` with the appliance holding
the only dependency edge. This is already the workspace shape; HA is confirmation, not a change.
