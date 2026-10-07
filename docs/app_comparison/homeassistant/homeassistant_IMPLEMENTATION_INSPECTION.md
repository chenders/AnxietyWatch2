# Home Assistant — authorized implementation inspection (Phase 2A)

**Slug:** `homeassistant`
**Research date:** 2026-10-06
**Phase:** 2A — authorized implementation inspection
**Outcome:** **Performed — published source.** The project's source was read at a pinned commit, and its
published documentation was read from the live site. **No binary was inspected. No instance was
installed, started, contacted or executed. No account was created. No endpoint was called.**

| Inspection mode | Available? | Used? | Notes |
|---|---|---|---|
| Published source (project repository) | Yes — Apache-2.0 | **Yes** | Sparse clone at tag `2026.9.4`; eleven files read in full or in part |
| Published documentation (project site) | Yes | **Yes** | Eight site pages plus the companion-app sensor reference |
| Published specification / API contract | Partly | No | The REST and WebSocket contracts exist in the tree; they were not read (gap G5) |
| Vendor-published datasets or API reference | n/a | No | Not applicable to this candidate |
| Third-party regulator artifact | n/a | No | No regulated claim exists to reconcile |
| App binary (static) | Lawfully obtainable for Android (F-Droid / official APK) | **No** | Not attempted — unnecessary once source was available, and it would have added an emulator, a store account and a CA story for no benefit to the questions asked |
| Runtime traffic / emulator observation | Possible for a self-hosted instance | **No** | Would have required standing up an instance and a target device; not authorised and not needed for the questions asked |

**The mode's own limit, stated up front.** Reading source proves what the code *says*; it proves nothing
about what a running system *does*. Everything in §4 and §5 is a static finding. §6 records zero runtime
observations, and no finding in this file is upgraded because the code "obviously" works.

---

## 1. Summary of outcome

**What was inspected.** A sparse clone of `home-assistant/core` at tag **2026.9.4**
(`9212531f40a0b7b23229a90d688dd79d9dfccff4`, 2026-09-27), restricted by `sparse-checkout` to the
`alert`, `bluetooth`, `recorder`, `notify`, `automation`, `mobile_app` and `sensor` components plus
`helpers/`, `util/`, `const.py`, `core.py` and `exceptions.py`. **110 Python files** were present in the
checkout; eleven were read in whole or in part and are listed with hashes in §3.

**Five findings** (the first is expanded in §3.3 and §8).

1. **`homeassistant/components/sensor/recorder.py:239` does not parse under Python 3.** The line reads
   `except ValueError, TypeError:`; `ast.parse` on the file fails with *"multiple exception types must be
   parenthesized"*. The file's SHA-256 in my checkout, in the git object database, in
   `raw.githubusercontent.com` at the tag, and in a separately downloaded copy are **all identical**
   (`ea8cf79e…`). The same line is present on the `dev` branch (`2792ec63…`). It is present in every
   commit that touched the file in the six-month window I sampled. **This is a static finding about a
   file, not a runtime observation**, and its consequence for a running instance is inferred, not
   observed (§7, item 32).
2. **The alert path ends an alert — and sends its resolution message — when the watched entity becomes
   absent.** `entity.py:98` ends alerting whenever the new state differs from the configured alarm
   state, and `unknown`/`unavailable` are just states that differ. There is no branch for either.
   §4.3.
3. **Storage flattens three distinct absences into one `String(255)` column.** `unknown` and
   `unavailable` are written as those literal strings, and a removed state as `""`
   (`db_schema.py:478-490`). No unit column; no provenance column; no quarantine. §4.4.
4. **The retention story is a scheduled hard delete for raw history and an upsert for aggregates.**
   `purge.py:55,498-511` deletes `states`, `events`, `statistics_runs` and `statistics_short_term`;
   `statistics` (hourly) is deleted only by a duplicate-cleanup migration and by an explicit user
   service. `db_schema.py:696-722` gives the aggregate tables a unique index on
   `(metadata_id, start_ts)`. §4.5.
5. **A delivery-confirmation mechanism exists and stops after one hop.** The local push channel
   requires confirmation within `PUSH_CONFIRM_TIMEOUT = 10` seconds and then re-sends through a single
   fallback (`push_notification.py:10,33-62`). No second fallback, no escalation, no persisted
   unacknowledged record. §4.6.

**One hypothesis tested and refuted.** I expected the recorder to exclude absent states by a
`STATE_UNAVAILABLE` check before writing. **It does not**: there is no reference to `STATE_UNAVAILABLE`
or `STATE_UNKNOWN` anywhere in `components/recorder/`. The exclusion happens further up, in
`sensor/recorder.py`, and it happens by *parseability* — `float(state.state)` inside a
`try/except ValueError, TypeError` (`:226-241`), with `math.isfinite` as a second filter (`:243-249`) —
not by an explicit three-state rule. Absent states are excluded from *statistics*, and stored verbatim
in *history*. Both halves are recorded; the refuted hypothesis is not recorded as a finding.

---

## 2. Authorization basis

- **Code licence: Apache-2.0.** Retrieved at the tag (`LICENSE.md`, SHA-256 `c71d239d…`) and confirmed
  to be the Apache License, Version 2.0 text. Reading, and quoting from, an Apache-2.0 repository is
  expressly permitted by the licence. No reverse engineering, no DRM, no authentication, no protection
  was circumvented — the material is published for exactly this purpose.
- **Documentation**: retrieved over plain HTTPS with a normal user-agent, as a member of the public.
- **No instance was contacted.** I did not query `demo.home-assistant.io`, a public instance, or any
  user's instance. No telemetry endpoint was used. No webhook was registered.
- **No accounts, no credentials, no personal data.** No store account, no Nabu Casa account, no forum
  account.
- **Third-party code not inspected.** The scanner-liveness logic lives in the separate `habluetooth`
  package, imported into `bluetooth/const.py:5-11`. I did not fetch or read it; its constants are
  `[UNKNOWN]` here (`G4`).

**Instances I deliberately did not inspect.** There are public Home Assistant instances with open
dashboards, and a project demo instance. Reading one would have produced runtime observations, but it
would also have meant reading someone else's data. Not done.

---

## 3. Artifact record

### 3.1 Primary artifact

| Field | Value |
|---|---|
| Repository | `https://github.com/home-assistant/core` (default branch `dev`) |
| Licence | Apache-2.0 |
| Tag inspected | **2026.9.4** |
| Commit | **`9212531f40a0b7b23229a90d688dd79d9dfccff4`** (2026-09-27 11:45:19 +0200) |
| Tag publication date | 2026-09-27 (GitHub releases API) |
| Clone method | `git clone --depth 1 --branch 2026.9.4 --filter=blob:none --sparse`, then `git sparse-checkout set` |
| Checkout size | 6.4 MB; 110 Python files |
| Working tree state | **unmodified** — `git status --short` empty at the time of inspection |

### 3.2 Files read, with SHA-256 computed from the checkout

| File | SHA-256 | Read |
|---|---|---|
| `homeassistant/const.py` | `3914e64322a5c5b3effb6cd4a1dca25f3ebca59381aea1a679d6be27bff89f8f` | in part |
| `homeassistant/core.py` | `2d92d136433488735db82b7fea044908c9689e7f3b87f5f43aac6fce319473bc` | in part |
| `homeassistant/components/sensor/recorder.py` | `ea8cf79e9367fb80f07b34594e659b92f04ecdefb2f11feb4d2be644c213de69` | in part (incl. the whole failing function) |
| `homeassistant/components/recorder/db_schema.py` | `2ae9c803ec3fc575b04ee93997a30e746d33cbd5bf44e03d74e9d2c910cac668` | in part |
| `homeassistant/components/recorder/purge.py` | `0b7ecf3c9df9f219a49fd5ca00e3f4383b21ed381bfc860db961c3a0ff703d72` | in part |
| `homeassistant/components/recorder/queries.py` | `02584000da22f4692d315dd38515432299df91d9db423f898e34207e108287fe` | in part |
| `homeassistant/components/recorder/__init__.py` | `feaa663638d62d00948b9a07737618d3c6a572c15713c2b6bb13b9e87265f362` | in part |
| `homeassistant/components/recorder/statistics.py` | (read only via grep; hash not taken) | in part |
| `homeassistant/components/alert/entity.py` | `2d077c3be8a77cd81095686209591c21d47e50d6bd0fead347fd8770bd42c0ae` | **in full** |
| `homeassistant/components/alert/__init__.py` | `9a39139651e879c8cd18cc0ef16a8b472421ba5b46f1f3637895d8b0682046b0` | listed; not read in full |
| `homeassistant/components/mobile_app/push_notification.py` | `6d4ba495a3a70414d20141a8a4aad5589b7a950a187d4d0a63dfe2a3789a5bb8` | **in full** |
| `homeassistant/components/bluetooth/const.py` | `58755c8c8283aed456673c14b21707d1a53dd456bcef5217090cbf2e3f8d4c60` | in full |
| `tests/components/sensor/test_sensor_recorder.py` | `bac5ce23e7a2f4d76a645511e2906d1b59eefad587932e1893205a76f23bd4c5` | imports the failing module (`:48-52`); 56 test functions |
| `pyproject.toml` (repo root, at the tag) | `20427298fe50422e6feeccc137bf8539337bc2cc68f58a62674f48fdccc132b5` | in part — ruff `select`, per-file-ignores, pylint MESSAGES CONTROL |

Two rows disclose incomplete hashing rather than guessing: `recorder/statistics.py` was only grepped.

### 3.3 The failing file — full verification chain

The claim "this file does not parse" is strong enough that it needs four independent confirmations, and
it got them:

| # | Read | Result |
|---|---|---|
| 1 | `shasum -a 256` of the file in the sparse checkout | `ea8cf79e9367fb80f07b34594e659b92f04ecdefb2f11feb4d2be644c213de69` |
| 2 | `git cat-file -p HEAD:homeassistant/components/sensor/recorder.py \| shasum -a 256` (the committed blob) | **same hash** — the checkout matches the object database |
| 3 | `curl https://raw.githubusercontent.com/home-assistant/core/2026.9.4/…/recorder.py \| shasum -a 256` (independently downloaded at the tag) | **same hash** — the object database matches the published tag |
| 4 | `curl https://raw.githubusercontent.com/home-assistant/core/dev/…/recorder.py` | different hash (`2792ec63…`), **same failing line** — not fixed on `dev` |

And the failing line is confirmed by parsing rather than by eye:

```
$ python3 -c "import ast; ast.parse(open('homeassistant/components/sensor/recorder.py').read())"
SyntaxError: multiple exception types must be parenthesized (line 239)
```

`git status --short` in the clone was **empty**, so this is not a local edit, a merge marker, or a
partially-applied patch. **I did not run any Home Assistant code, so I cannot report what a running
instance does with this file.** Whether the module would be imported, and by which code path, is
inference (§7 item 32).

**Three further facts that change how a reader should weigh this.**

- **The module is imported by its own test file.** `test_sensor_recorder.py:48-52` opens with
  `from homeassistant.components.sensor.recorder import (` and imports three names from it; the file
  contains **56 test functions**. Under Python's import semantics a module containing a syntax error raises
  `SyntaxError` when it is imported, so that test module cannot be collected. **That is Python's documented
  behaviour, not an observation of this repository** — I did not run pytest, and could not, because the
  package's test dependencies are not installed here.
- **The same idiom, written correctly, appears five lines below.** `_is_numeric`
  (`homeassistant/components/sensor/recorder.py:244-249`) handles the same two exception types with
  `with suppress(ValueError, TypeError):` — the valid form. The file therefore contains both the correct
  and the invalid spelling of one idiom. That is why the broken line reads as a typo rather than as a
  deliberate choice, and why I record it without any inference about intent.
- **Static-analysis configuration does not obviously excuse it.** `[tool.ruff.lint] select` includes `"E"`
  (`pyproject.toml:658`, within the `select = [` list at `:647`), ruff is pinned to
  `required-version = ">=0.16.3"` (`:644`), and there is **no `per-file-ignores` entry** covering this path
  (`pyproject.toml:862-879`). The string `"syntax-error"` does appear in the repository's lint
  configuration — but at `pyproject.toml:214`, inside `[tool.pylint."MESSAGES CONTROL"]` (`:139`), which
  disables **pylint's own** E0001. That is a different linter, and it is not a reason for ruff's E999 to
  pass.

**An unresolved tension, left unresolved.** A release whose own test module cannot import one of its own
component modules should not have passed CI. I cannot explain that, and I will not guess. Every candidate
resolution — CI configuration excluding the path, a tag cut after a failing run, a build step that
regenerates or patches the file, or an error in this inspection — is something I did not test. What I can
say is bounded and verified: **at tag `2026.9.4`, and on `dev`, as retrieved on 2026-10-06, the published
bytes do not parse under Python 3, and `ast.parse` names the reason.**

### 3.4 Documentation artifacts

Site pages and companion-app reference retrieved 2026-10-06; SHA-256 values are in
`homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` §14.2. The site displays version **2026.9.4**
(`ha_home.html`), agreeing with the tag inspected — the documentation I read and the source I read are
the same release.

---

## 4. Static findings

### 4.1 Architecture as read

```
┌─ companion app (iOS/Android, Nabu Casa, Inc) ─────────────────────────┐
│  sensors incl. health_connect_*  ──last value + last_updated──────────┼──┐
│  local push channel ──confirm ≤10 s──▶ fallback sender                │  │
└───────────────────────────────────────────────────────────────────────┘  │
                                                                           ▼
┌─ Home Assistant instance (user-owned, local) ────────────────────────────┐
│  state machine: entity_id → (state: str ≤255, attributes, timestamps)    │
│      reserved: "unknown" | "unavailable"        ← only absent states     │
│  event bus: state_changed, ...                                            │
│  ┌ alert ──────────────────────┐   ┌ automation ──────────┐              │
│  │ eq-test on watched state   │   │ triggers/conditions  │              │
│  │ no branch for unknown/     │   └──────────────────────┘              │
│  │ unavailable → send "done"  │                                          │
│  │ in-memory _firing/_ack     │                                          │
│  └────────────────────────────┘                                          │
│  ┌ recorder ────────────────────────────────────────────────────────────┐│
│  │ states  (String(255) state, no unit, no provenance)                  ││
│  │   purge: nightly, delete rows older than purge_keep_days=10          ││
│  │ statistics        hourly, unique(metadata_id,start_ts) → UPSERT      ││
│  │ statistics_short_term  5-min, purged with states                     ││
│  │ statistics_meta   units, source, name                                ││
│  └──────────────────────────────────────────────────────────────────────┘│
│  ├─ habluetooth (third-party) — scanner watchdog, not inspected (G4)     │
└───────────────────────────────────────────────────────────────────────────┘
        (dashed = existed but was not read; hatched = absent)
```

### 4.2 Absent states at the state-machine layer

- `STATE_UNKNOWN: Final = "unknown"` (`const.py:303`); `STATE_UNAVAILABLE: Final = "unavailable"`
  (`const.py:313`). Both are ordinary strings; both are valid values of `states.state`.
- `MAX_LENGTH_STATE_STATE: Final = 255` (`const.py:61`).
- **Two different policies for one condition, in one file.** `validate_state()` **raises**
  `InvalidStateError("Invalid state with length … State max length is 255 characters.")` when a state is
  too long (`core.py:202-208`), while the internal setter **substitutes** `STATE_UNKNOWN` and logs
  `"State %s for %s is longer than %s, falling back to %s"` (`core.py:2500-2507`).
  The substitution does not coerce or truncate the value — it declares the state absent, which is the
  honest choice — but the platform's public and internal paths disagree about whether an unusable value
  is an error or a substitution.
- **Positive comparator for V2:** substituting an explicit absent state rather than clamping, truncating
  or keeping the over-long value is precisely the behaviour V2's "never clamp an out-of-range value"
  rule demands. It is recorded here as a point of agreement, not a difference.

- **The same file contains a second, sharper internal contrast.** `validate_state` **raises**
  `InvalidStateError`, while `_is_numeric` (`components/sensor/recorder.py:244-249`) uses
  `suppress(ValueError, TypeError)` to *skip* a value it cannot parse, and the five lines above it use the
  invalid `except ValueError, TypeError:` instead. One file therefore demonstrates three policies for an
  unusable value — raise, skip, and a third that does not parse at all — with no rule stating which applies
  where.

### 4.3 The alert path — the finding that matters

Read in full: `homeassistant/components/alert/entity.py` (212 lines).

| Element | Evidence | Reading |
|---|---|---|
| Lifecycle | `entity.py:3` and `:33` | **"DEVELOPMENT OF THE ALERT INTEGRATION IS FROZEN."** in both the module and class docstrings |
| Trigger | `entity.py:91-99` | on a state-change event; `None` new state returns early (`:93`); begin when the state **equals** the alarm state (`:96`); **end when it does not** (`:98`) |
| State rendering | `entity.py:83-89` | `on` while firing and unacknowledged; `off` while firing **and acknowledged**; `idle` otherwise — documented identically at `ha_integrations_alert_.txt:26-33` |
| Repetition | `entity.py:128-139` | delays from a configured minute list; `min(self._next_delay + 1, len(self._delay) - 1)` **saturates** at the last value, so the alert becomes periodic |
| Acknowledgement | `entity.py:201-206` | `async_turn_off` raises `ServiceValidationError("This alert cannot be acknowledged")` when `can_acknowledge` is false |
| Notification failure | `entity.py:183-193` | catches **`ServiceNotFound` only**, logs `"Failed to call notify.%s, retrying at next notification interval"`, continues |
| Completion | `entity.py:158-168`, `:148` | `_send_done_message` is armed only after a message has been sent; `_notify_done_message` sends the configured resolution text when the alert ends |
| Persistence | `entity.py:38-72` | `_firing`, `_ack`, `_next_delay` are in-memory fields on an `Entity`; the scheduled callback is created with `cancel_on_shutdown=True` |

**The failure mode, stated precisely.** The alert's entire condition logic is an equality test against
one string. `unknown` and `unavailable` are strings that are not equal to the configured alarm state,
so an unreachable sensor is treated as a **cleared condition**: `end_alerting` runs, and if a message
had been sent earlier the platform sends the user's own "resolved" text. A user watching a leak sensor
is told *the leak is over* because the sensor dropped off the network. There is no branch for either
absent state, and the published three-state table documents only `idle`/`on`/`off`.

**Two qualifications, both of which I owe the project.** First, the alert integration is general-purpose
and its documented semantics are exactly what its code does — a user who reads the docs sees the same
three states the code implements, with no claim about absent states. Second, the integration's own
docstring declares its development frozen, which is a lifecycle fact a reader can act on. The finding is
not that the documentation lies; the finding is that **the platform's most mature alarm primitive
answers "the signal is gone" with "the problem is over."**

### 4.4 Storage — how absences are written

- `class States(Base)` (`db_schema.py:404`); the state column is
  `state: Mapped[str | None] = mapped_column(String(MAX_LENGTH_STATE_STATE))` (`:422`) — a nullable
  255-character string, with **no unit column and no provenance column**.
- `States.from_event` (`:478-490`):
  - `state is None` (the state was removed from the state machine) → **`state_value = ""`**, with
    `last_changed_ts = None` and `last_reported_ts = None` (`:481-486`);
  - otherwise → **`state_value = state.state`** (`:487`), so `"unknown"` and `"unavailable"` are stored
    as ordinary values in the same column as `"23.5"`.
- **Consequence:** a consumer of the history table cannot distinguish, from the schema, a value of
  `""`/`"unknown"`/`"unavailable"` from a real state string; the meaning is a convention. V2's decoder
  contract requires the opposite — reject with a named reason, never store a sentinel in the value.
- **Fair note:** storing the literal `unknown`/`unavailable` is at least *self-describing* — the strings
  are reserved constants and a reader sees the word. It is not the same as storing `0` or `-1` for
  missing data, which this platform does not do. The defect is the mixing of types in one untyped
  column and the collapse of three distinct absences onto three string values with no enum.

### 4.5 Retention and the aggregate layer

- **Raw history is hard-deleted on a schedule.** `purge_old_data` (`purge.py:55`) selects and deletes
  `states` and `events` older than a `purge_before` datetime, then `statistics_runs`
  (`:498-502`, excluding the newest run by design, `:366-385`) and `statistics_short_term`
  (`:504-511`). **No tombstone, no quarantine, no audit of what was deleted.**
- **Default window:** `vol.Optional(CONF_PURGE_KEEP_DAYS, default=10)` (`recorder/__init__.py:105`);
  documented as `purge_keep_days` default 10 (`ha_integrations_recorder_.txt:63`), with nightly auto-purge
  at 04:12 local and a monthly repack.
- **Long-term statistics are not purged by that routine.** The only `delete(Statistics)` in the
  component is `delete_duplicate_statistics_row` (`queries.py:708`), called from the duplicate-cleanup
  path in `migration.py`. The only `update(Statistics)` (`queries.py:676`) belongs to a schema migration
  converting `start` to `start_ts`. User-initiated clearing is a separate service
  (`core.py:557 async_clear_statistics`).
- **Aggregates are upserted.** `Statistics` carries `unique=True` on `(metadata_id, start_ts)` with
  `duration = timedelta(hours=1)` (`db_schema.py:696-710`); `StatisticsShortTerm` is the same shape at
  `timedelta(minutes=5)` (`:713-722`). Columns include `mean`, `mean_weight`, `min`, `max`, `sum`,
  `state`, `last_reset_ts`.
- **`mean_weight` is the interesting column**: a coverage weight stored next to the mean. It is how the
  fact that an hour was only half covered survives into the aggregate — a partial answer to the same
  problem V2 solves with explicit NULLs and gap-filling. Recorded as a **positive comparator**.
- **Circular statistics are implemented** — `circular_mean` SQL helpers with `weighted_sum_sin` /
  `weighted_sum_cos` (`statistics.py:144-162`). Recorded as a **positive comparator**: a numeric
  correctness concern V2's registry does not address at all.

### 4.6 Delivery confirmation

`mobile_app/push_notification.py`, read in full (79 lines):

- `PUSH_CONFIRM_TIMEOUT = 10  # seconds` (`:10`).
- `async_send_notification` (`:33-62`): when the channel supports confirmation, a `confirm_id` is
  attached, a timeout handler is scheduled, and the message is sent. On timeout, the pending confirm is
  popped, the local channel is **torn down**, and `fallback_send(data)` is invoked.
- `async_confirm_notification` (`:65-73`) returns `False` for an unknown `confirm_id` — an explicit
  receipt-does-not-match-anything-I-sent answer rather than a silent success.
- **Boundaries:** one fallback hop; no second channel; no escalation to a second recipient; no persisted
  unacknowledged record; nothing written to the database on delivery or failure.

### 4.7 Bluetooth boundary

- `bluetooth/const.py:5-11` imports `CONNECTABLE_FALLBACK_MAXIMUM_STALE_ADVERTISEMENT_SECONDS`,
  `FALLBACK_MAXIMUM_STALE_ADVERTISEMENT_SECONDS`, `SCANNER_WATCHDOG_INTERVAL`,
  `SCANNER_WATCHDOG_TIMEOUT` and `BluetoothScanningMode` from the third-party **`habluetooth`** package
  with `# noqa: F401` — i.e. re-exported for other components. **The scanner-liveness logic is not in
  this tree**; its values are `[UNKNOWN]` here (`G4`).
- `UNAVAILABLE_TRACK_SECONDS: Final = 60 * 5` (`:34`) — five minutes without an advertisement makes a
  tracked device unavailable. The constant exists; the code that consults it was not in the inspected
  subtree.
- Documentation states the platform requirement (BlueZ ≥ 5.43; ≥ 5.63 recommended) and recommends an
  ESP32 proxy over a locally attached adapter, citing that "Linux kernel updates have previously broken
  Bluetooth functionality" (`ha_integrations_bluetooth_.txt:51,70`).

---

## 5. Reconciliation points

Carry-forward points for the comparison report.

| # | Point | Evidence | Status |
|---|---|---|---|
| R1 | The platform has `unknown` and `unavailable` as first-class states | `const.py:303,313` | **verified in source** |
| R2 | The alert layer has no branch for either, and treats them as a cleared condition | `entity.py:91-99,115-126` | **verified in source** |
| R3 | Alert state does not survive a restart | `entity.py:38-72` (in-memory fields, `Entity` base, `cancel_on_shutdown=True`) | **verified in source for the entity; the component's `reproduce_state.py` was not read** (§7 item 34, gap G9) |
| R4 | Raw history is hard-deleted; no tombstone | `purge.py:55,498-511` | **verified in source** |
| R5 | Aggregates are upsert rows with a coverage weight | `db_schema.py:696-722` | **verified in source** |
| R6 | Absence is stored as a literal string in the value column | `db_schema.py:478-490` | **verified in source** |
| R7 | Over-long states are replaced by `unknown`, not clamped or truncated | `core.py:2500-2507` | **verified in source** |
| R8 | Public and internal paths disagree on whether an invalid state is an error | `core.py:202-208` vs `:2500-2507` | **verified in source** |
| R9 | Delivery confirmation with a 10-second timeout and one fallback | `push_notification.py:10,33-62` | **verified in source** |
| R10 | Scanner liveness is delegated to `habluetooth` | `bluetooth/const.py:5-11` | **verified in source**; values `[UNKNOWN]` |
| R11 | `sensor/recorder.py` does not parse | §3.3 (four-way hash agreement + `ast.parse`) | **verified as a static fact**; runtime consequence inferred |
| R12 | Statistics exclude non-finite and non-numeric states by parseability | `sensor/recorder.py:226-249` | **verified in source** — but see R11: the module containing it does not parse, so "the code that excludes them" cannot be said to run |
| R13 | Health sensors are last-value only, 30 days, no analysis | companion docs | **documented**, not verified |

**Explicitly not recorded as discrepancies.**

- `averageUserRating = 0` in the iTunes record for an app with claimed million-user scale. Recorded as
  **unresolved** (`homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` §4.1), excluded from all tallies. An
  API field reporting zero is not evidence that the app is unrated.
- Home Assistant Cloud being outside the project's privacy policy. This is a **scope statement the
  project itself makes**, not a concealment; recorded as a boundary (§9.1 of the claimed-features file).
- The "1500+ integrations" figure being single-sourced. Recorded as an evidence gap (G7), not a
  discrepancy, because the integrations index page does not claim a count at all.

---

## 6. Runtime observation

| Category | Count |
|---|---|
| Instances installed | **0** |
| Instances started | **0** |
| Instances contacted | **0** |
| Requests issued to any Home Assistant API | **0** |
| Health/state values observed | **0** |
| Notifications sent | **0** |
| Alerts triggered | **0** |
| Accounts created | **0** |
| App binaries inspected | **0** |
| Test suites executed | **0** |

**Zero in every row.** Nothing in this file is a runtime observation, and no static finding was upgraded
by assuming the code runs. In particular, the syntax error in §3.3 is a claim about a file under a
Python parser; the claim that it therefore prevents a component from loading is **inference** and is
labelled as such wherever it appears.

---

## 7. Claim reconciliation

Claims from `homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` §14.1 that could be checked against the
inspected tree. Verdict vocabulary: **match** (the source/docs agree with the claim), **documented-only**
(the claim is in the project's documentation and was not verified in source), **partial** (true with a
material qualification), **does not match**, **outside classes** (not a checkable claim — a marketing count or superlative, a store-metadata
field, an absence claim, or a refuted hypothesis).

| # | Claim | Verdict | Basis |
|---|---|---|---|
| 1 | Local-first, open source, privacy first | match | Apache-2.0 `LICENSE.md`; local instance architecture as read |
| 2 | Non-profit owner; can't be sold | documented-only | home page only; no legal check |
| 3 | 1500+ integrations | outside classes | single-sourced marketing count; index page states no count |
| 4 | Top open source project by contributors, 2025 | outside classes | superlative with no named ranking source |
| 5 | Over 1 million users | outside classes | app description claim; not verifiable here |
| 6 | `unknown` and `unavailable` are real states | match | `const.py:303,313` |
| 7 | Companion sensor "state will be unavailable if the timestamp cannot be determined" | documented-only | companion docs; no iOS/Android implementation read |
| 8 | Health sensors are last-value, 30 days | documented-only | companion docs only |
| 9 | Sleep classification "provided by Google" | documented-only | companion docs |
| 10 | 26 Health Connect sensors with per-sensor units | documented-only | companion docs table |
| 11 | One unit spelled "celsus" beside "celsius" | match | observed in the published table, `comp_sensors.txt:399,404` |
| 12 | Alert has three states, documented as idle/on/off | match | docs table agrees exactly with `entity.py:83-89` |
| 13 | Alert can be configured non-acknowledgeable and says so | match | `entity.py:201-206` |
| 14 | Notify failures are logged and retried next interval | match | `entity.py:183-193` |
| 15 | Alert repeats on a configurable interval | match | `entity.py:128-139`; saturates at the last delay |
| 16 | Alert condition ends when the watched state changes away | match | `entity.py:98` |
| 17 | — that includes a watched entity going unknown/unavailable, which produces the `done_message` | match | `entity.py:98` + `:115-126`; no branch for either state exists |
| 18 | Alert development is frozen | match | `entity.py:3,33` |
| 19 | `purge_keep_days` default 10 | match | docs + `recorder/__init__.py:105` |
| 20 | Auto purge nightly at 04:12 local | documented-only | docs only; schedule not verified |
| 21 | Monthly auto repack | documented-only | docs only |
| 22 | Purge deletes states and events | match | `purge.py:55,498-511` |
| 23 | Purge also deletes short-term statistics | match | `purge.py:504-511` |
| 24 | Long-term statistics are not purged on that schedule | match | only duplicate-cleanup and a user service delete from `statistics` |
| 25 | Raw history keeps no tombstone | match | deletes are plain row deletes; no tombstone column |
| 26 | Removed state is stored as an empty string | match | `db_schema.py:481-486` |
| 27 | `unknown`/`unavailable` are stored as literal state strings | match | `db_schema.py:487` |
| 28 | No unit column on `states` | match | `db_schema.py:404-437` |
| 29 | Statistics are upserted on `(metadata_id, start_ts)` | match | `db_schema.py:696-710` unique index |
| 30 | `mean_weight` records coverage | match | `db_schema.py:663,699`; `statistics.py:144-162` |
| 31 | Circular-mean helpers exist | match | `statistics.py:144-162` |
| 32 | `sensor/recorder.py` fails to parse, so the statistics sensor path cannot load | partial | the parse failure is verified four ways (§3.3) and the module is imported by its own test file; the *consequence* for a running instance is Python's import semantics plus inference, not an observation of a running instance |
| 33 | Over-long states are replaced by `unknown`, not clamped | match | `core.py:2500-2507` |
| 34 | Alert state does not survive a restart | partial | in-memory fields and `cancel_on_shutdown=True` are verified; the `alert` component's own state-reproduction path was not read (gap G9) |
| 35 | Local push confirms within 10 s, else one fallback | match | `push_notification.py:10,33-62` |
| 36 | There is no escalation, second fallback or persisted receipt | match | whole file read; no such mechanism exists |
| 37 | Scanner liveness is delegated to `habluetooth` | match | `bluetooth/const.py:5-11` |
| 38 | BlueZ ≥ 5.43 required, ≥ 5.63 recommended | documented-only | docs only |
| 39 | Vendor advises an ESP32 proxy over a local adapter | documented-only | docs only |
| 40 | `UNAVAILABLE_TRACK_SECONDS` = 5 minutes | match | `bluetooth/const.py:34` |
| 41 | No health-signal analysis anywhere in the platform | outside classes | an absence claim; the components read contain none, which is not proof none exists |
| 42 | Analytics are opt-in and retention is 60 days | documented-only | privacy policy only |
| 43 | Cloud is out of scope of the privacy policy | match | the policy says so in its own words |
| 44 | iOS app is free, min OS 16.4, v2026.9.3 | outside classes | third-party store metadata, not a project claim |
| 45 | `averageUserRating = 0` for a million-user app | outside classes | **unresolved ambiguity**, excluded from tallies |
| 46 | Statistics exclude absent states by parseability rather than by an explicit rule | match | `sensor/recorder.py:226-249`; no `STATE_UNAVAILABLE`/`STATE_UNKNOWN` reference anywhere in `components/recorder/` |
| 47 | The hypothesis that the recorder checks absent states explicitly | outside classes | **refuted** by R46; not recorded as a finding |
| 48 | `validate_state` raises while the internal setter substitutes | match | `core.py:202-208` vs `:2500-2507` |
| 49 | The syntax error is a long-standing condition, not a one-commit regression | match | present in all five commits touching the file from 2026-04-30 to 2026-09-03, and on `dev` |
| 50 | The documentation I read and the source I read are the same release | match | site displays 2026.9.4; tag inspected is 2026.9.4 |

**Tally — machine-counted from the table above, not written from impression.**

**50 checkable claims: 31 match · 10 documented-only · 2 partial · 7 outside classes.**

The two `partial` rows are items 32 and 34: in both, the underlying static fact is verified but the
*consequence* rests on a file this inspection did not read — the running instance's import path, and the
`alert` component's state-reproduction path. The seven `outside classes` rows are items 3, 4, 5, 41, 44,
45 and 47: a marketing count, two superlative/user-count claims, an absence claim, a third-party
store-metadata row, the unresolved ratings field, and the refuted hypothesis. **No row in this table is a
runtime observation**, because §6 contains none.

---

## 8. Evidence gaps

- **G1** No runtime observation of any kind (§6).
- **G2** Android/Play metadata not retrieved; only iOS store metadata exists for this candidate.
- **G3** Nabu Casa cloud pricing and its separate privacy notice not retrieved.
- **G4** `habluetooth` not inspected; `SCANNER_WATCHDOG_TIMEOUT`/`SCANNER_WATCHDOG_INTERVAL` values
  `[UNKNOWN]`.
- **G5** REST and WebSocket API contracts not read; the platform's external interface is `[UNKNOWN]`
  beyond its existence.
- **G6** The iOS HealthKit ingestion path was not verified; the documented health surface is Android
  Health Connect.
- **G7** "1500+ integrations" is single-sourced.
- **G8** "Top open source project by contributors in 2025" names no ranking source.
- **G9** `alert/reproduce_state.py` not read; R3/R34 rest on entity-level evidence only.
- **G10** The syntax error's **origin** is unbounded. I sampled five commits (2026-04-30 → 2026-09-03)
  plus `dev`; all contain it. I did not fetch earlier history, so I cannot say when it was introduced,
  only that it is not a recent one-commit regression and is not fixed on `dev`.
- **G11** No test run was attempted; whether the failing module is imported on a given code path is
  inference.
- **G12** `recorder/history/` (the read path) was not inspected.
- **G13** 110 files were present in the sparse checkout; **thirteen** rows in §3.2, of which eleven files
  were read in part or in full. Everything else in the platform is outside this inspection.
- **G14** **Why CI appears not to have caught a non-parsing file is unexplained and untested** (§3.3). The
  tension is recorded, not resolved. Resolving it would require the repository's CI configuration and a CI
  run, neither of which was retrieved.
- **G15** The function containing the syntax error (`_entity_history_to_float_and_state`, `:226-241`) is
  referenced by **no test** in the module's own test file — `grep` for the function name, `_is_numeric`, or
  `except ValueError` across `test_sensor_recorder.py` returns nothing. That the function is untested is
  verified; that the lack of a test is *why* the error survived is inference and is not asserted.

---

## 9. Ethical and legal limits observed

- **Nothing was bypassed.** No DRM, authentication, licence check, rate limit or access control was
  circumvented. The source is published under Apache-2.0 for reading.
- **No instance was touched.** Not a public demo, not a community instance, not a user's instance.
- **No personal data was collected, read or retained.** No account was created anywhere.
- **No device or credential is present in this file.** No Bluetooth address, hostname, IP address or
  MAC. The only machine-specific strings quoted are public project paths and upstream file paths.
- **No claim of independent validation.** This inspection is a *static reading of published source by an
  interested party*. It is not a review, an audit, a security assessment or a test result, and the
  project has not endorsed it.
- **The documentation defect and the syntax error are recorded as observations about artifacts**, with
  the artifact and the exact evidence, and without any inference about the maintainers' intent.
  A file that does not parse and marketing copy that overstates are different kinds of problem, and both
  are recorded without a motive attached.

---

## 10. Reproduction

```bash
# 1. Pin the release.
mkdir -p /tmp/aw2research/homeassistant && cd /tmp/aw2research/homeassistant
curl -sS -A aw2-research https://api.github.com/repos/home-assistant/core/releases/latest -o rel.json
python3 -c "import json;d=json.load(open('rel.json'));print(d['tag_name'],d['published_at'])"
#   expected: 2026.9.4 2026-09-27T09:51:05Z

# 2. Sparse clone the tag.
git clone --depth 1 --branch 2026.9.4 --filter=blob:none --sparse \
    https://github.com/home-assistant/core.git core
cd core
git sparse-checkout set homeassistant/components/sensor homeassistant/components/alert \
    homeassistant/components/bluetooth homeassistant/components/recorder \
    homeassistant/components/notify homeassistant/components/automation \
    homeassistant/components/mobile_app homeassistant/helpers homeassistant/const.py \
    homeassistant/core.py homeassistant/util
git log -1 --format='%H %ci'
#   expected: 9212531f40a0b7b23229a90d688dd79d9dfccff4 2026-09-27 11:45:19 +0200
git status --short          # expected: empty

# 3. The failing parse.
python3 -c "import ast; ast.parse(open('homeassistant/components/sensor/recorder.py').read())"
#   expected: SyntaxError: multiple exception types must be parenthesized (line 239)

# 4. Confirm the bytes are upstream bytes, not a local artifact.
shasum -a 256 homeassistant/components/sensor/recorder.py
#   expected: ea8cf79e9367fb80f07b34594e659b92f04ecdefb2f11feb4d2be644c213de69
curl -sS -L https://raw.githubusercontent.com/home-assistant/core/2026.9.4/homeassistant/components/sensor/recorder.py \
  | shasum -a 256
#   expected: the same value

# 5. The alert path.
grep -n "to_state.state != self._alert_state" homeassistant/components/alert/entity.py
#   expected: 98
grep -rn "STATE_UNAVAILABLE\|STATE_UNKNOWN" homeassistant/components/recorder/ | wc -l
#   expected: 0   (absent states are not handled there at all)

# 6. Absence in storage.
grep -n "state_value = \"\"\|state_value = state.state" \
    homeassistant/components/recorder/db_schema.py
#   expected: 483 and 487

# 7. Retention and aggregates.
grep -n "unique=True" homeassistant/components/recorder/db_schema.py | sed -n '1,6p'
grep -n "delete_statistics_short_term_rows\|delete_statistics_runs_rows" \
    homeassistant/components/recorder/purge.py
#   expected: 500 and 509 as the only two delete call sites

# 8. Delivery confirmation.
grep -n "PUSH_CONFIRM_TIMEOUT" homeassistant/components/mobile_app/push_notification.py
#   expected: 10 (definition) and 58 (use)
```

Documentation artifacts were retrieved with a Chrome user-agent; SHA-256 values are in the
claimed-features file §14.2, and the textual claims quoted in this package were verified against those
bytes with the same quote-verification sweep described in `../RESEARCH_METHOD.md`.

---

## Related files

- `homeassistant_CLAIMED_FEATURES_AND_PURPOSE.md` — public claims, the sensor table, the claim ledger
- `PROMPT_2.md` — the candidate-specific Prompt 2
- `homeassistant_COMPARISON.md` — the comparison report
- `../RESEARCH_METHOD.md` — inspection modes, this candidate's mode, and the standing sub-checks
- `../PROCESS_STATE.md` — candidate status and residual uncertainty
