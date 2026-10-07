# Home Assistant — claimed features and purpose

**Candidate:** Home Assistant (candidate 9 of the shortlist)
**Slug:** `homeassistant`
**Research date:** 2026-10-06
**Vendor / project:** Home Assistant, a project of the **Open Home Foundation** (non-profit). The
iOS/Android companion apps are published by **Nabu Casa, Inc**.
**Inspection mode for this candidate:** **performed — published source**, plus published docs. See
`homeassistant_IMPLEMENTATION_INSPECTION.md`.

> **Why this candidate is in the package.** Home Assistant is not a competitor for the health-sensing
> layer and does not claim to be. It is the closest available benchmark for the layer V2 actually has to
> build well: an always-on local collector, a local database with retention, a local alarm path, and an
> operator-facing console — on a Raspberry Pi, offline, with a large integration fleet.
>
> **Evidence classes used in this file.** `[PUBLIC CLAIM]` — a statement the project makes about itself.
> `[FOUND IN SOURCE]` — read directly from the pinned source tree. `[DOCUMENTED]` — stated in the
> project's own documentation, not verified by running anything. `[THIRD PARTY]` — an external record.
> `[INFERENCE]` — my reasoning, labelled as such. `[UNKNOWN]` — not established. Nothing in this file is
> a runtime observation; nothing was executed. See §14.3.

---

## 1. Executive summary

Seven findings, in order of consequence. Each is expanded later in the file.

1. **The platform's own alert path treats an unreachable sensor as a resolved condition, and announces
   it.** The alert entity ends the alert whenever the watched entity's state is not the configured
   alarm state — including `unknown` and `unavailable` — and on that transition sends the configured
   `done_message` ("Garage is closed"). `[FOUND IN SOURCE]` The published state table documents three
   states and does not mention either absent state. See §12.1 — this is the single sharpest contrast
   with V2's "a missing signal never reassures" rule in the whole package.
2. **Home Assistant does have a three-state honesty model at the state-machine layer, and it is
   documented.** `STATE_UNKNOWN` and `STATE_UNAVAILABLE` are first-class constants
   (`homeassistant/const.py:303,313`), and the companion-app documentation states plainly that a sensor's
   "state will be unavailable if the timestamp cannot be determined" — `[DOCUMENTED]`. **This refutes the
   shortlist's hypothesis that a general-purpose hub has no three-state honesty model
   (`CANDIDATE_SHORTLIST.md` §9).** The model exists; the loss is downstream, in the alert path and in
   storage. That correction is recorded in §14.4 and was written back into the shortlist entry itself on
   2026-10-06.
3. **In storage, all three absences collapse into one string column.** A state of `unknown` or
   `unavailable` is written to `states.state` as that literal string; a *removed* state is written as the
   **empty string** `""`, with `last_changed_ts` and `last_reported_ts` set to `None`
   (`db_schema.py:478-490`). `[FOUND IN SOURCE]` Whether a value means "nobody measured this" or "the
   measurement was zero" is carried by convention on a `String(255)` column, not by the schema. There is
   no unit column on `states` at all.
4. **Raw history is hard-deleted on a schedule.** The recorder's nightly purge deletes `states` and
   `events` rows older than `purge_keep_days` (**default 10**), and deletes short-term (5-minute)
   statistics in the same pass (`purge.py:55,498-511`; `__init__.py:105`). There is no tombstone and no
   quarantine record: a purged state change is gone. Long-term hourly statistics are **not** purged by
   that routine — a user service and a duplicate-cleanup migration are the only two code paths that
   delete from `statistics`. `[FOUND IN SOURCE]`
5. **The aggregate layer is an upsert, not an append.** `Statistics` carries
   `Index(..., unique=True)` on `(metadata_id, start_ts)` with `duration = timedelta(hours=1)`, and
   siblings at 5 minutes; the row for an entity-hour is rewritten as more data arrives
   (`db_schema.py:696-722`). `mean_weight` is stored alongside `mean` — a coverage weight, which is how
   partial-hour coverage survives into the aggregate. `[FOUND IN SOURCE]` This is a legitimate design; it
   is simply not V2's append-only `samples` layer, and the comparison has to say that rather than implying
   equivalence.
6. **The companion app already ingests a broad health signal set — as current values, for 30 days, with
   no analysis.** **26 `health_connect_*` sensors** are documented, including blood glucose (mg/dL),
   systolic/diastolic pressure (mmHg), oxygen saturation (%), respiratory rate, HRV (ms), VO2 max, basal
   body temperature, sleep duration and weight. `[DOCUMENTED]` The same page says that unless otherwise
   noted "only the last 30 days of data is used", and every description begins "The last recorded …" —
   these are **last-value entities**, not measurement series. `[DOCUMENTED]` Units are per-sensor and
   include non-SI units, with conversion left to the front end. Documenting the same page, the
   basal-temperature row's unit is spelled **"celsus"** where the sibling body-temperature row is spelled
   "celsius" — a directly observable documentation defect (§14.2, H-31).
7. **The release I inspected does not parse.** `homeassistant/components/sensor/recorder.py:239` reads
   `except ValueError, TypeError:`, which Python 3 rejects; `ast.parse` fails with "multiple exception
   types must be parenthesized". Byte-identical at the tag, on `dev`, and in the working tree. Recorded as
   a **static finding** with the full verification chain in the implementation-inspection file — it is not
   a runtime observation, and I did not run Home Assistant.
   The module is imported by its own test file (`test_sensor_recorder.py:48-52`, 56 tests), and **the same
   idiom is written correctly five lines below** (`:244-249`, `with suppress(ValueError, TypeError):`) —
   which is why it reads as a typo rather than a deliberate choice. The repository's lint configuration
   selects ruff rule `E` with no ignore for this path (`pyproject.toml:658,647,862-879`); the only
   `"syntax-error"` entry is in the separate **pylint** section (`:214`). **Why CI appears not to have
   caught this is unexplained and is recorded as an open tension, not resolved.** It remains a static
   finding about a file: it is not a runtime observation, and I did not run Home Assistant.

**One-sentence purpose statement.** Home Assistant is a local-first, open-source home-automation platform
whose stated purpose is local control and privacy; its overlap with AnxietyWatch2 is entirely
architectural — always-on edge collection, local storage and retention, local alerting, and an
operator console.

---

## 2. Positioning

- **Self-description:** "Open source home automation that puts local control and privacy first."
  `[PUBLIC CLAIM]` (`ha_home.txt:5`)
- "Powered by a worldwide community of tinkerers and DIY enthusiasts." `[PUBLIC CLAIM]`
- "Browse 1500+ integrations" on the home page. `[PUBLIC CLAIM]` (`ha_home.txt:8`) — note that the
  integrations index page I retrieved does not itself state a count; the number appears on the home page
  only (`[UNKNOWN]` whether the two surfaces agree).
- "We are a non-profit and can't be sold or acquired." `[PUBLIC CLAIM]` (`ha_home.txt:29`)
- "We want privacy and local control to be the standard." `[PUBLIC CLAIM]` (`ha_home.txt:130`)
- "Home Assistant is a project of the Open Home Foundation: the non-profit organization that fights for
  the fundamental principles of privacy, choice, and sustainability for smart homes."
  `[PUBLIC CLAIM]` (`ha_home.txt:146`)
- "Top open source project by contributors in 2025." `[PUBLIC CLAIM]` (`ha_home.txt:6`) — no ranking
  source named on the page; recorded as a claim, not as a measurement.
- The iOS App Store description says: "Join the over 1 million users and empower your home with better
  privacy, choice and sustainability." `[PUBLIC CLAIM]` (iTunes lookup record, §14.2 H-2)

**What the marketing surfaces do not claim.** No physiological measurement, no clinical function, no
wellness or screening claim, no health scores, no illness detection. The health-adjacent surface is the
*Companion app's sensor list*, which reports what other apps already store on the phone. There is no
Home Assistant algorithm that interprets it. `[INFERENCE, from the full absence of any such claim on the
retrieved surfaces]`

---

## 3. Governance, corporate and legal

| Item | Value | Class |
|---|---|---|
| Project owner | **Open Home Foundation** (non-profit) | `[PUBLIC CLAIM]` |
| Companion-app publisher (iOS) | **Nabu Casa, Inc** (`artistName`, `sellerName`) | `[THIRD PARTY — iTunes Lookup]` |
| Commercial cloud | **Home Assistant Cloud**, operated by Nabu Casa; described as a separate legal entity | `[DOCUMENTED]` (`ha_privacy_.txt:12`) |
| Code licence | **Apache-2.0** | `[THIRD PARTY — GitHub API]`; `LICENSE.md` retrieved and read to confirm it is the Apache 2.0 text |
| Repository | `home-assistant/core`, default branch **`dev`**, created 2013-09-17 | `[THIRD PARTY — GitHub API]` |
| Repository scale (as retrieved) | 91,266 stars · 38,804 forks · 3,586 open issues · 1,362 watchers | `[THIRD PARTY — GitHub API]` |
| Latest release at research date | **2026.9.4**, published 2026-09-27 | `[THIRD PARTY — GitHub releases API]` |
| Website's displayed version | **2026.9.4** | `[PUBLIC CLAIM]` (`ha_home.txt:3`) — the site and the tag I inspected agree |

**A governance boundary worth recording.** The project is a non-profit and says so prominently, but the
**companion apps are published by a for-profit company** (Nabu Casa, Inc) and the **cloud service is
excluded from the project's own privacy policy** (§9). The "non-profit" claim attaches to the project
and the Foundation, not to every surface a user touches. Both facts are the project's own statements.

---

## 4. Availability, platforms and hardware

- **Self-hosted hub**: Home Assistant OS, Container, Core, or Supervised; the iOS description names
  "a device like the Home Assistant Green or Raspberry Pi". `[PUBLIC CLAIM]`
- **First-party hardware**: Home Assistant Green, Connect ZBT-2 (Zigbee/Thread), Connect ZWA-2
  (Z-Wave), Voice Preview Edition. `[PUBLIC CLAIM]` (`ha_home.txt:36-45`)
- **Companion apps**: iOS (`io.robbie.HomeAssistant`, v**2026.9.3**, released 2026-09-30, 271,976,448 B,
  minimum OS **16.4**, price **free**) and Android. `[THIRD PARTY — iTunes Lookup]`
- **Voice**: "Our private, insanely fast, voice assistant running on affordable hardware. Now supporting
  wake words." `[PUBLIC CLAIM]`
- **Energy**: "Home Energy Management" is a first-class dashboard feature. `[PUBLIC CLAIM]`

### 4.1 Ratings — an unresolved ambiguity, not a discrepancy

The iTunes lookup for `io.robbie.HomeAssistant` returns `averageUserRating = 0`,
`averageUserRatingForCurrentVersion = 0`, and a zero rating count, on an app first released 2017-04-15
with more than a million claimed users. `[THIRD PARTY]` I did **not** observe the App Store page itself,
and I did not retrieve the Play Store listing. **This is recorded as unresolved: an API field reporting
zero is not evidence that the app is unrated, and I am not recording it as a discrepancy.** It is the
same class as the Oura ratings-vs-reviews ambiguity, and it is excluded from every tally in this package.

---

## 5. Pricing

There is no first-party price for the software: the platform is free to self-host and the apps are free
to download. The paid surface is the **optional cloud**. `[PUBLIC CLAIM]` and `[DOCUMENTED]`

- The privacy policy states that Home Assistant Cloud is operated by Nabu Casa and "governed by the
  respective provider's privacy policy", and is "outside the scope of this policy". `[DOCUMENTED]`
  (`ha_privacy_.txt:12`)
- The iOS description states: "If you would like to access this app while away from home, the most
  secure and simple way to get started is Home Assistant Cloud." `[PUBLIC CLAIM]` (iTunes record) —
  i.e. the project itself recommends the paid cloud for the remote-access case, which is a
  self-limiting statement about how far "it runs locally" extends.
- **I did not retrieve Nabu Casa's pricing page**, so the cloud price is `[UNKNOWN]` here rather than
  quoted from memory.

---

## 6. Measured signals

### 6.1 The state model

- Every entity has exactly one **current state**: a string, up to 255 characters
  (`homeassistant/const.py:61`). `[FOUND IN SOURCE]`
- Two reserved states exist for absence: `unknown` and `unavailable` (`const.py:303,313`).
  `[FOUND IN SOURCE]`
- History is a log of **state changes**, not a series of measurements: the recorder's `States` row is
  created from a `state_changed` event (`db_schema.py:478`). Two measurements of the same quantity from
  two different devices at the same instant are two entity IDs, and nothing in the row records which
  device produced a value beyond the entity ID. `[FOUND IN SOURCE]`

### 6.2 The companion app's health sensors

`[DOCUMENTED]` — Companion app sensor reference, `comp_sensors.txt:391-416`. **26 sensors** in the
Health Connect family, with the unit column exactly as published:

| Sensor | Published unit |
|---|---|
| `health_connect_active_calories_burned` | kilocalories |
| `health_connect_basal_body_temperature` | **celsus** |
| `health_connect_basal_metabolic_rate` | kilocalories per day |
| `health_connect_blood_glucose` | milligrams per deciliter |
| `health_connect_body_fat` | percent |
| `health_connect_body_temperature` | celsius |
| `health_connect_body_water_mass` | grams |
| `health_connect_bone_mass` | grams |
| `health_connect_daily_distance` | meters |
| `health_connect_daily_elevation_gained` | meters |
| `health_connect_daily_floors` | floors |
| `health_connect_daily_hydration` | milliliters |
| `health_connect_daily_steps` | steps |
| `health_connect_diastolic_blood_pressure` | millimeters of Mercury |
| `health_connect_heart_rate` | beats per minute |
| `health_connect_heart_rate_variability` | milliseconds |
| `health_connect_height` | meters |
| `health_connect_lean_body_mass` | grams |
| `health_connect_oxygen_saturation` | percent |
| `health_connect_respiratory_rate` | breaths per minute |
| `health_connect_resting_heart_rate` | beats per minute |
| `health_connect_sleep_duration` | minutes |
| `health_connect_systolic_blood_pressure` | millimeters of Mercury |
| `health_connect_total_calories_burned` | kilocalories |
| `health_connect_vo2_max` | milliliters per minute per kilogram |
| `health_connect_weight` | grams |

Three facts about this list, all from the same page:

1. **They are last-value sensors.** Every description begins "The last recorded …" and the page states:
   "These sensors will reflect health and fitness data stored by other apps on your device in Health
   Connect. Unless otherwise noted, only the last 30 days of data is used." `[DOCUMENTED]`
2. **The list is broader than V2's registry in *types* and far narrower in *depth*.** Blood glucose,
   blood pressure and SpO2 appear here as single current values; V2's registry models each as a typed
   measurement with a range, a unit and a validity decision at the decoder. `[INFERENCE, grounded in the
   quoted descriptions]`
3. **Units are published per sensor and are mixed SI/non-SI** — mmHg, mg/dL, kcal, and "steps" and
   "floors" are not physical units at all. `[DOCUMENTED]` Conversion is a front-end concern; there is no
   "one canonical unit per type converted once at the decoder" rule anywhere in the retrieved material.

### 6.3 Other sensor families relevant to the comparison

- **Activity sensors** "See Below — The current activity type, sleep confidence and sleep segment as
  computed by Google. Requires activity recognition permissions on supported devices."
  `[DOCUMENTED]` — the sleep classification is performed by the **operating system**, not by Home
  Assistant.
- **Sleep Confidence / Sleep Segment**: "updates about once a day" / "every 10 minutes"; "All data is
  provided by Google." `[DOCUMENTED]` (`comp_sensors.txt:146`)
- **`last_reboot`**: "The state will be unavailable if the timestamp cannot be determined."
  `[DOCUMENTED]` (`comp_sensors.txt:432`) — the same page's clearest statement of the unavailable state.
- **Battery sensors** expose `battery_health`, `battery_level`, `battery_power`,
  `battery_temperature`, plus a documented `low_battery_alert` update rule.
  `[DOCUMENTED]` (`comp_sensors.txt:209`) — a device-health surface V2 has no equivalent of.

---

## 7. Derived measures and analysis

- **Long-term statistics** are the platform's derived layer: hourly rows retaining `mean`, `min`, `max`,
  `sum`, `state` and `last_reset_ts`, with `mean_weight` for coverage (`db_schema.py:696-710`).
  `[FOUND IN SOURCE]`
- **Circular statistics exist** — `circular_mean` SQL helpers are implemented (`statistics.py:144-162`).
  `[FOUND IN SOURCE]` That is a genuine numerical-correctness feature V2's registry has no analogue for.
- **No analysis of the health signals themselves.** Nothing in the retrieved material or the inspected
  component set scores, correlates or interprets the health sensors. Home Assistant surfaces them; it
  does not analyse them. `[INFERENCE from the absence of any such claim and from the component scope
  I read]` — flagged in §14.5 as a negative claim established by absence, which is the weakest class of
  evidence in this package.

---

## 8. Published interfaces

- **`recorder.purge` / `recorder.purge_entities`** services, documented with `keep_days`
  (`ATTR_KEEP_DAYS`, `recorder/const.py:47`). `[DOCUMENTED]` + `[FOUND IN SOURCE]`
- **Long-term statistics retrieval** is a documented service ("Retrieves long-term statistics for one or
  more entities", `ha_integrations_recorder_.txt:179`). `[DOCUMENTED]`
- **REST and WebSocket APIs** exist (the recorder ships `websocket_api.py` and
  `basic_websocket_api.py`). `[FOUND IN SOURCE]` I did **not** read the API schemas, so the API's
  contract is `[UNKNOWN]` here beyond its existence.
- **Config-entry config flows** are the onboarding mechanism; `bluetooth/config_flow.py` exists.
  `[FOUND IN SOURCE]`

---

## 9. Privacy

Source: the project's privacy policy, retrieved 2026-10-06, with its own revision history.

- "We do not sell or rent personal data." `[PUBLIC CLAIM]` (`ha_privacy_.txt:48`)
- Analytics: "Nothing is sent unless the user opts in, and the sharing settings can be changed or
  disabled at any time." `[PUBLIC CLAIM]` (`ha_privacy_.txt:43`) — **opt-in, not opt-out**.
- Analytics retention: "stored in Cloudflare's Key-Value store for a maximum of 60 days since the last
  update." `[PUBLIC CLAIM]` (`ha_privacy_.txt:43`)
- Analytics stack: **self-hosted Plausible**; "No data is shared with Plausible Insights OÜ, the
  Plausible company, or any other third party." `[PUBLIC CLAIM]` (`ha_privacy_.txt:22`)
- Cloudflare Analytics is **disabled**, though Cloudflare still processes operational data:
  "Cloudflare necessarily processes certain operational data, such as IP addresses and user-agent
  information". `[PUBLIC CLAIM]` (`ha_privacy_.txt:19`) — the project states both halves rather than
  claiming zero third-party processing.
- Companion apps: "sensor, health, and location data … is sent directly to the configured Home Assistant
  instance and is **not logged by the Open Home Foundation** or shared by the Open Home Foundation with
  third parties as part of that direct transmission." `[PUBLIC CLAIM]` (`ha_privacy_.txt:56`)
- Push and crash reporting: "Certain separate app features may rely on third-party infrastructure,
  including services used for push notifications and crash or diagnostic reporting."
  `[PUBLIC CLAIM]` (`ha_privacy_.txt:57`) — **the third parties are not named on this page.**
- Retention: a general rule only — "We retain personal data only for as long as necessary … Retention
  periods vary by service and data category." `[PUBLIC CLAIM]` (`ha_privacy_.txt:59`), i.e. **no
  per-category retention table**.
- Revision history is published with dates: 2020-10-19 (Companion applications), 2024-10-30 (health
  data added to Companion applications), **2026-08-27** (comprehensive update). `[PUBLIC CLAIM]`
  (`ha_privacy_.txt:66-68`)

### 9.1 Two scope facts, recorded as scope

1. **The privacy policy governs the websites and services, not the instance you run.** Its scope
   sentence names the websites, the forum, status pages and the companion apps
   (`ha_privacy_.txt:11`). The data on a user's own hub is governed by the user.
2. **Home Assistant Cloud is explicitly carved out.** "Commercial services associated with Home
   Assistant that are operated by separate legal entities, including Home Assistant Cloud operated by
   Nabu Casa, are governed by the respective provider's privacy policy and are outside the scope of this
   policy." (`ha_privacy_.txt:12`) This is an honest boundary rather than a defect; it does mean the
   strongest privacy claim ("it is yours") and the weakest-governed surface (the cloud) sit on adjacent
   pages, and the reader has to follow a second document to see the whole picture.

---

## 10. Security and reliability surfaces

- **Bluetooth platform requirement**: "The Bluetooth adapter must be accessible to D-Bus and running
  BlueZ >= 5.43. It is highly recommended to use BlueZ >= 5.63 as older versions have been reported to
  be unreliable." `[DOCUMENTED]` (`ha_integrations_bluetooth_.txt:70`)
- **A vendor-published caution about Linux Bluetooth**: "In many cases, a better approach than a
  directly connected adapter or card is to use a Bluetooth proxy using an ESP32 since Linux kernel
  updates have previously broken Bluetooth functionality and Bluetooth driver support Linux generally
  falls behind other operating systems for newer adapters." `[DOCUMENTED]`
  (`ha_integrations_bluetooth_.txt:51`)
  — Home Assistant, on its own authority, recommends against the architecture V2 is building (Pi + local
  adapter + BlueZ) for reliability reasons. That is independent corroboration of a risk V2 already
  tracks, from a project with a far larger Bluetooth fleet. It is **not** evidence about V2's specific
  implementation.
- Scanner liveness is delegated: the scanner watchdog constants are imported from the third-party
  `habluetooth` package (`bluetooth/const.py:5-11`, `# noqa: F401` on the import). `[FOUND IN SOURCE]`
  The values themselves are outside the inspected tree → `[UNKNOWN]`.
- Device staleness: `UNAVAILABLE_TRACK_SECONDS = 60 * 5` (`bluetooth/const.py:34`) — a device with no
  advertisement for five minutes goes unavailable. `[FOUND IN SOURCE]`
- Local push has a **confirmation protocol with a 10-second timeout and an automatic fallback**
  (`mobile_app/push_notification.py:10,58`). `[FOUND IN SOURCE]` See §12.2.

---

## 11. Clinical, regulatory and validation posture

- **None.** No medical device, no CE marking or FDA clearance claim, no clinical validation claim, no
  accuracy statistic, and no health-outcome statistic appears on any retrieved surface.
  `[INFERENCE from the absence of any such claim on the retrieved surfaces]`
- Home Assistant is not a regulated product and does not present itself as one. For this package that is
  a **positive**: it is the one candidate that makes no claim needing regulatory reconciliation.
- There is therefore no `[REGULATOR RECORD]` evidence class in this file at all.

---

## 12. What Home Assistant does not do — in the places that matter to V2

### 12.1 The alert path treats an unreachable sensor as a resolved condition

`[FOUND IN SOURCE]` — `homeassistant/components/alert/entity.py`.

- The alert has three states, documented verbatim: `idle` = "The condition for the alert is false.";
  `on` = "The condition for the alert is true."; `off` = "The condition for the alert is true but it was
  acknowledged." `[DOCUMENTED]` (`ha_integrations_alert_.txt:26-33`)
- The code: a state change on the watched entity ends the alert whenever the new state is **not** the
  configured alarm state — `if to_state.state != self._alert_state and self._firing: await
  self.end_alerting()` (`entity.py:98-99`), and a `None` new state returns early (`entity.py:93`).
- `end_alerting` clears the firing flag and, if a message had already been sent, calls
  `_notify_done_message` (`entity.py:115-126`, with `_send_done_message = True` set at `:148`).
- **Consequence:** when the watched sensor goes `unknown` or `unavailable` — a routine, documented
  condition in this platform — the alert transitions from `on` to `idle` and emits the configured
  resolution message. The user is told, in the project's own configured words, that the problem is over.
- **There is no `unknown`/`unavailable` branch in the alert entity**, and the published state table does
  not mention the two absent states.
- **Recorded as a code path, not as observed behaviour.** I did not run Home Assistant. This is a static
  finding about a branch that does not exist, verified by reading the whole file.

**Why this is the sharpest finding in the package.** V2's rule is "a missing signal holds state; it never
reassures". Home Assistant's alarm layer — in a mature, widely deployed, non-commercial platform with a
documented `unavailable` concept — does the opposite at exactly the moment the signal disappears. The
comparison is not that V2 asserts a better rule; it is that V2's rule is enforced by a state machine
that has a third state, and HA's rule is enforced by an equality test that has two.

### 12.2 There is no escalation, no persisted alarm state, and no receipt

- The alert schedules repeats from a configured list of minute delays and **saturates at the last
  entry** (`_next_delay = min(self._next_delay + 1, len(self._delay) - 1)`, `entity.py:128-139`), so the
  alert becomes periodic. `[FOUND IN SOURCE]`
- Acknowledgement is supported (`can_acknowledge`, default `true`), and `async_turn_off` raises
  `ServiceValidationError("This alert cannot be acknowledged")` when it is disabled
  (`entity.py:201-206`). `[FOUND IN SOURCE]` — an alert that cannot be dutifully acknowledged **and
  says so** is a genuine positive: it names the blocker.
- When a notifier is missing, the alert logs the failure and continues: `except ServiceNotFound:` →
  "Failed to call notify.%s, retrying at next notification interval" (`entity.py:183-193`).
  `[FOUND IN SOURCE]` There is no health state, no escalation to another channel, and no record that
  delivery failed.
- **Alert state does not survive a restart.** `AlertEntity` derives from `Entity` and holds `_firing`,
  `_ack` and `_next_delay` in memory; it does not use the restore-state mixin, and the scheduled
  callback is registered with `cancel_on_shutdown=True` (`entity.py:128-139`). `[FOUND IN SOURCE]`
  A restart while an alert is firing therefore returns it to `idle` until the watched entity changes
  again. **There is no dead-man switch on the alarm path.**
- The module and class docstrings both read **"DEVELOPMENT OF THE ALERT INTEGRATION IS FROZEN."**
  (`entity.py:3,33`). `[FOUND IN SOURCE]` A lifecycle fact about the platform's alarm layer, quoted as
  such.

### 12.3 Push delivery: a real confirmation protocol, and where it stops

`[FOUND IN SOURCE]` — `mobile_app/push_notification.py`:

- The local (LAN) push channel requires the app to confirm receipt; `PUSH_CONFIRM_TIMEOUT = 10` seconds
  (`:10`). On timeout, the local channel is torn down and the message is re-sent through
  `fallback_send` (`:33-62`). `[FOUND IN SOURCE]`
- **What exists that V2's spec wants:** a delivery confirmation, a bounded timeout, and a documented
  automatic fallback channel.
- **What does not exist:** a second fallback, an escalation to a second recipient, a persisted
  unacknowledged record, or any receipt written to the database. The fallback is one hop.
- **Fair framing for the comparison:** V2's supervised alarm path is *specified* and not built; HA's is
  *built* and stops after one confirmation and one fallback. Neither is a finished answer to the same
  question, and the comparison should say that rather than scoring a win.

### 12.4 Other absences, recorded as absences

- No three-state concept in the *alert* layer (§12.1) even though the platform has one in the state
  layer (§6.1).
- No per-record algorithm version, no provenance field naming why a value changed, and no quarantine
  mechanism anywhere in the recorder. `[FOUND IN SOURCE]` — the recorder's columns are `state_id`,
  `metadata_id`, `state`, timestamps and a context ID (`db_schema.py:404-437`). There is nothing like
  V2's reject-with-named-reason.
- No unit column on `states` (`String(255)` state, no unit); units live in `statistics_meta` for
  statistics and in the attributes JSON otherwise. `[FOUND IN SOURCE]`
- No tombstone for deleted history: purge deletes rows (§1.4). `[FOUND IN SOURCE]`

---

## 13. Relationship to AnxietyWatch v1 and v2

- **To v1:** no overlap. v1 is a SwiftUI iOS app with HealthKit, a local store and a CNS risk pipeline.
  Home Assistant does not read HealthKit on iOS (the health surface is Android Health Connect, plus
  Apple Health via third-party integrations not inspected here). `[INFERENCE, from the retrieved
  surfaces]`
- **To v2:** the overlap is the *platform layer*, not the product: an always-on collector, a local
  database with a retention policy, a local alarm path, an operator console, and a Raspberry Pi target.
  V2's `pi5-vitals-core`, `linux-bluez-broker`, `admin-console-roadmap` and
  `pi-prototype-storage-degraded-mode` changes are all exercised, in some form, by Home Assistant at a
  scale V2 will never reach. `[INFERENCE from the V2 change set already read for this package]`
- **The most useful thing this candidate does for the package** is to show that a mature, non-commercial,
  privacy-first platform with a documented absent-state model **still** fails the "missing signal never
  reassures" test in its alarm layer. That makes V2's three-state rule defensible on evidence rather than
  on taste — and it makes the rule's *coverage* the thing to check, because Home Assistant shows how a
  correct core can be defeated by one equality test downstream.

---

## 14. Evidence posture

### 14.1 Claim ledger

| # | Claim | Source surface | Class | Status |
|---|---|---|---|---|
| H-1 | "Open source home automation that puts local control and privacy first." | home page | `[PUBLIC CLAIM]` | consistent with Apache-2.0 source and the retrieved policy |
| H-2 | "over 1 million users" | iOS App Store description | `[PUBLIC CLAIM]` | not independently verifiable here |
| H-3 | "Browse 1500+ integrations" | home page | `[PUBLIC CLAIM]` | integrations index page retrieved; **no count stated there** → the number is single-sourced |
| H-4 | "We are a non-profit and can't be sold or acquired." | home page | `[PUBLIC CLAIM]` | consistent with Open Home Foundation; not a legal check |
| H-5 | "Top open source project by contributors in 2025." | home page | `[PUBLIC CLAIM]` | **no ranking source named on the page** |
| H-6 | "Nothing is sent unless the user opts in." | privacy policy | `[PUBLIC CLAIM]` | opt-in stated twice (analytics + diagnostics) |
| H-7 | "We do not sell or rent personal data." | privacy policy | `[PUBLIC CLAIM]` | — |
| H-8 | Cloudflare Analytics disabled | privacy policy | `[PUBLIC CLAIM]` | stated with its own limitation in the next sentence |
| H-9 | Companion app data "is not logged by the Open Home Foundation" | privacy policy | `[PUBLIC CLAIM]` | scoped to the direct transmission |
| H-10 | Push/crash reporting "may rely on third-party infrastructure" | privacy policy | `[PUBLIC CLAIM]` | **third parties not named on that page** |
| H-11 | Cloud is out of scope of the privacy policy | privacy policy | `[PUBLIC CLAIM]` | scope fact, §9.1 |
| H-12 | Health Connect sensors use "only the last 30 days of data" | companion docs | `[DOCUMENTED]` | matches the "last recorded …" descriptions |
| H-13 | Sleep confidence/segment "provided by Google" | companion docs | `[DOCUMENTED]` | the platform attributes the inference |
| H-14 | `last_reboot` "will be unavailable if the timestamp cannot be determined" | companion docs | `[DOCUMENTED]` | clearest published absent-state statement |
| H-15 | Alert `off` = "condition … true but it was acknowledged" | alert docs | `[DOCUMENTED]` | matches `entity.py:83-89` exactly |
| H-16 | BlueZ ">= 5.43", "highly recommended … >= 5.63" | bluetooth docs | `[DOCUMENTED]` | — |
| H-17 | "Linux kernel updates have previously broken Bluetooth functionality" | bluetooth docs | `[DOCUMENTED]` | vendor caution; corroborates a V2 risk, proves nothing about V2 |
| H-18 | ESP32 proxy recommended over a local adapter | bluetooth docs | `[DOCUMENTED]` | — |
| H-19 | `purge_keep_days` default 10 | recorder docs + source | `[DOCUMENTED]` + `[FOUND IN SOURCE]` | default confirmed at `__init__.py:105` |
| H-20 | Auto purge nightly "at 04:12 local time" | recorder docs | `[DOCUMENTED]` | schedule time not verified in source |
| H-21 | "Automatically repack … second Sunday of each month" | recorder docs | `[DOCUMENTED]` | not verified in source |
| H-22 | Long-term statistics retrievable via a service | recorder docs | `[DOCUMENTED]` | — |
| H-23 | iOS app free, minimum OS 16.4, v2026.9.3 | iTunes lookup | `[THIRD PARTY]` | retrieved 2026-10-06 |
| H-24 | Repo Apache-2.0, 91,266 stars | GitHub API | `[THIRD PARTY]` | retrieved 2026-10-06 |
| H-25 | Release 2026.9.4 published 2026-09-27 | GitHub releases API | `[THIRD PARTY]` | tag inspected |
| H-26 | Three alert states, documented in a table | alert docs | `[DOCUMENTED]` | *not* a three-state absent/measured model (§12.1) |
| H-27 | Alert cannot be acknowledged when `can_acknowledge: false` | source | `[FOUND IN SOURCE]` | positive: the failure names its blocker |
| H-28 | Local push confirms within 10 s else falls back | source | `[FOUND IN SOURCE]` | §12.3 |
| H-29 | `unknown`/`unavailable` are first-class states | source | `[FOUND IN SOURCE]` | refutes the shortlist hypothesis |
| H-30 | Removed state stored as `""`; `unknown`/`unavailable` stored as literal strings | source | `[FOUND IN SOURCE]` | §1.3 |
| H-31 | Companion docs spell one unit "celsus" beside "celsius" | companion docs | `[DOCUMENTED]` | documentation defect, quoted verbatim |
| H-32 | `sensor/recorder.py:239` does not parse; the module is imported by its own test file (56 tests) | source | `[FOUND IN SOURCE]` | verification chain in the inspection file; the CI tension is **unresolved** |
| H-33 | `statistics` rows are upserted via a unique index | source | `[FOUND IN SOURCE]` | §1.5 |
| H-34 | Circular-mean SQL helpers exist | source | `[FOUND IN SOURCE]` | positive |
| H-35 | Purge deletes only `statistics_runs` and `statistics_short_term` | source | `[FOUND IN SOURCE]` | the only other `statistics` delete is duplicate cleanup in migration |

### 14.2 Artifact record

Website and API artifacts retrieved 2026-10-06 with a Chrome user-agent. SHA-256:

| Artifact | SHA-256 |
|---|---|
| `web/ha_home.html` | `71105f0ed761a06aa18b8570c53a955d27f35951d07aff415f36fd02a5e64c5e` |
| `web/ha_installation_.html` | `bda16e59559f78aa2e3c6d9705a1277ac0679e31e5ef72681a5e411795c70a7b` |
| `web/ha_integrations_.html` | `137981f1c954ab5a11c5094f5f5eb6b3274d1cd9b1819102081925a383e465cf` |
| `web/ha_integrations_alert_.html` | `3b686822185f424959072b7f032636955fcdc8cd48a656b5d911779c2689ce53` |
| `web/ha_integrations_bluetooth_.html` | `6826f37ca45dae23e01825575213748353bc3f1820b384dc60c60154b5d1078c` |
| `web/ha_integrations_recorder_.html` | `6de3f299b5ef5cbf60f7c8a3a640cc99562956c7f310aa60acb518128541aaf2` |
| `web/ha_privacy_.html` | `7dd8b76e3eeb48f2255a0ef54c0a90811f2135c926c03682816e13619ba8a068` |
| `web/ha_dashboards_.html` | `7cf38a0be8243d0838bedea3ae0a18440f253e75db2f728769787fd0dd8c4563` |
| `web/comp_sensors.html` | `a41c65432c6881698f5dcabdb8ed03148e9ce5ef442db777dd8c4323d117e92e` |
| `api/itunes_ha.json` | `b43206450e8d687234f374acbf171b7ade5ba561d5fa6bbdb098b08ca124f340` |
| `api/repo.json` | `ad9af7158ed1f15b60277a824a8fb69d40d516bc91681e30e049cef88c751cec` |
| `rel.json` (releases API) | `9a4781bfa4ba06c91553e0b0f5c85a7a6b7d255ce92807397c6467bc9d09e087` |
| `LICENSE.md` | `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4` |
| `raw_sensor_recorder.py` (tag 2026.9.4, from `raw.githubusercontent.com`) | `ea8cf79e9367fb80f07b34594e659b92f04ecdefb2f11feb4d2be644c213de69` |
| `raw_dev_sensor_recorder.py` (`dev` branch) | `2792ec6387d7c72bab05ec44affa3796460afdbf87dc8919da3b42c44abd5013` |

Text extractions (`web/*.txt`) were produced from the HTML by a local tag-stripping script; their
hashes are recorded in the inspection file's reproduction section rather than repeated here.

### 14.3 What this file is not

- **Not a runtime observation.** Nothing was installed, started or executed. `[FOUND IN SOURCE]` means
  the source was read.
- **Not a security assessment.** I did not review the platform's authentication, its HTTP layer, or its
  update mechanism.
- **Not an integration survey.** 1,500+ integrations exist; I read one (`alert`) closely, parts of three
  others (`recorder`, `bluetooth`, `mobile_app`), and nothing else.

### 14.4 Correction carried into this file

The shortlist asserted, as an uncertainty, that a general-purpose hub has no three-state honesty
model (`CANDIDATE_SHORTLIST.md` §9). **That is wrong, and the correction is recorded here and in
`PROCESS_STATE.md`.** The shortlist entry itself was corrected in place on 2026-10-06, so the file no
longer carries the old wording; the superseded phrasing is preserved only in the correction block there.
Home Assistant has two first-class absent states, documents one of them in the
companion-app reference, and has a `validate_state`/`async_set` policy for rejecting unusable values.
The accurate statement is narrower and more useful: *the three-state model exists at the state-machine
layer and is lost downstream — in the alert path, which has no branch for it, and in storage, which
flattens both absences into the same string column as the values.*

### 14.5 Evidence gaps

- **G1** No runtime behaviour of any kind was observed (`[UNKNOWN]` for behaviour).
- **G2** The Play Store listing was not retrieved; Android rating and download figures are `[UNKNOWN]`.
- **G3** Nabu Casa's cloud pricing and its privacy notice were not retrieved.
- **G4** The `habluetooth` package's watchdog values are outside the inspected tree.
- **G5** The REST/WebSocket API contracts were not read.
- **G6** Apple Health / HealthKit ingestion on iOS was not verified; the documented health surface is
  Android Health Connect.
- **G7** The claim "1500+ integrations" is single-sourced to the home page.
- **G8** "Top open source project by contributors in 2025" has no named ranking source.
- **G9** The `alert` integration's docs were read, but the **automation** trigger/condition semantics
  (the other half of a real alarm path) were not; only the component's file list was confirmed.
- **G10** The syntax error's *origin* was not bounded: it is present in every commit touching that file
  in the six months I sampled (2026-04-30 → 2026-09-03) and on `dev`, so I can say it is not a
  one-commit regression in that window and nothing more.
- **G11** No test run can be attempted here; the practical consequence of G10 for a user is inferred, not
  observed.

### 14.6 Weaknesses and omissions in this file

1. The health-sensor section leans on one documentation page; the underlying Android implementation was
   not read.
2. The absence claim that Home Assistant performs no analysis of health signals (`G6`/§7) is the weakest
   evidence class here.
3. I did not read the recorder's `history/` subpackage, so the *query* side of the retention story is
   unexamined.
4. The `alert` component's `reproduce_state.py` — which decides what happens to alerts across a
   restart — was listed but not read; §12.2's restart conclusion rests on the entity's inheritance and
   the in-memory fields. **This is the weakest of the file's source-derived claims and is flagged as
   such.**
5. No claim in this file was reconciled against a user-visible behaviour, because there is no runtime
   evidence.
6. The candidate's own shortlist entry was wrong in one respect (§14.4); **the entry has since been
   corrected in place in `CANDIDATE_SHORTLIST.md` §9** (2026-10-06), with the superseded phrasing kept in
   the correction block rather than deleted.

---

## Related files

- `homeassistant_IMPLEMENTATION_INSPECTION.md` — Phase 2A record: pinned tree, artifact hashes, the
  syntax-error verification chain, and the claim reconciliation
- `PROMPT_2.md` — the candidate-specific Prompt 2
- `homeassistant_COMPARISON.md` — the comparison report
- `../CANDIDATE_SHORTLIST.md` — entry 9 (note §14.4 above)
- `../RESEARCH_METHOD.md` — the inspection modes and standing sub-checks this file follows
- `../PROCESS_STATE.md` — candidate status
