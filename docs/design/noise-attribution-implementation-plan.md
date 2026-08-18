# Aircraft-noise attribution: implementation plan (regenerated 2026-08-17)

Regenerated after the full phx-helicopter-tracker audit
(`docs/reviews/phx-helicopter-tracker-full-audit-deltas.md`). Each item notes
its status and, where relevant, the audit finding that changed it. Scope rules
unchanged: work in this worktree, no commits, no deployment to the Pi.

## Ground rules carried from the owner's requirements

- Per-event attribution from all physical evidence (local ADS-B, FR24, pass
  log, geometry, propagation timing, class, phase, modelled noise). No
  repeated-tail requirement; ambiguous stays unassigned; synthetic never
  enters. Goal is general warning heuristics — identity is supporting history.
- Real evidence has two entry paths: bridge presses (authoritative timestamps)
  and operator reports (own timestamp metadata; corroborations never duplicate
  events). Double press = human heard AND cat scared (cat top priority).
- Everything captured at event time errs generous (full feed snapshots,
  weather); PostgreSQL is the queryable archive, JSONL spools are write-ahead.
- Human/cat history marks must be visible on browser noise-watch, kiosk
  noise-watch, and admin, including for currently-quiet aircraft.

## 1. Capture — `panel-hue-aircraft-label` (REVISED by live failure + audit)

Done: activity heartbeat (`stream_last_activity_at`, throttled), env-driven
attribution path, battery refresh throttling that never clobbers a good
measurement, exponential reconnect backoff, full-snapshot + weather capture,
schema 2 rows with explicit `outcomes`.

Remaining (this pass):
- **Lost-press fix.** Live loss at 04:09:49 UTC: the stream reconnects every
  ~90 s (no keepalives from this bridge) and the parser counts only
  `initial_press`, so a press whose `initial_press` fell in the blind window
  vanished even though its `short_release` arrived. Fix: parse ALL
  button_report events for the configured button; a `short_release` (or
  `long_press`/`repeat`) with no paired `initial_press` within a pairing
  window (by the bridge's own `updated` timestamps) is a **recovered press**.
  Log every received report (event type + bridge timestamp) so the next loss
  is diagnosable — the audit's silent-failure specimens (zeroed analysis,
  salted-hash URLs) all lacked exactly this. [audit: silent-failure class]
- New-button note: a replacement button gets a new resource id; the drop-in
  must be updated. Document in README.

## 2. Attribution — `panel-aircraft-attribute` (DONE, tests to update)

Rewritten: real provenances {hue_button, operator_report}, corroboration
attachment, audited airframe-class table, propagation-corrected timing,
confirmation floor + margin with named verdict reasons, uncapped candidate
lists with verbatim `raw`, flight_key on confirmed events.

Deferred (documented, not built now): loiter/heading-variability candidate
features using circular statistics over `aircraft_track_point`
[audit: pre-entry scoring, circular mean/std]; historic-positions recovery for
late reports with local gaps [audit: `historic/flight-positions` + timestamp].

## 3. Evidence entry — `panel-aircraft-report` + `panel-aircraft-inject` (DONE)

Report tool: new real events (`--ago/--at/--iso`, timezone-explicit, live
context only within 120 s) and `--corroborates` rows that never become second
events. Inject stays synthetic-only.

## 4. Archive — sqitch + `panel-noise-drain` (DONE, tests to add)

`noise_event` / `noise_event_corroboration` / `noise_event_candidate` /
`noise_attribution` (+ `aircraft_noise_history` view) and
`aircraft_track_point` hypertable with compression. Drain extended: labels
spool (synthetic refused twice — filter and CHECK), attribution verdict
history with change-detection, track-point batch ingest, spool rotation,
disk-free named every run. Verify scripts query every column so a missed
`sqitch deploy` fails loudly [audit: POST_MORTEM — checks must run; and
`create_all`-vs-migrations dual truth is why we have exactly one schema path].

## 5. Track capture — `panel-adsb-status` (DONE, tests to add)

1 s cadence inside 4 km, 5 s to 15 km, FR24 cache points deduped by
written_at; publish loop never blocks on DB and never dies on a full disk.
Volume justified by their 8.4M-position precedent [audit].

## 6. Definitive tracks — `panel-fr24-track-fetch` (REVISED by audit)

- **Adopt attempt bookkeeping** [audit: FlightDiscovery.track_download_attempted_at]:
  mark the attempt *before* the network call, store the error, and never
  auto-retry a failed or empty-track flight beyond a small bound — the current
  draft would re-charge ~40 credits every 15 min for a permanently-broken
  flight id. Blockers named with values.
- Roadmap (needs owner credit decision, not built now): backfill flight
  history for marked airframes via `flight-summary/light` in 14-day chunks
  (730-day reach) [audit].

## 7. Weather — `panel-weather-poll` (+ service/timer) (DONE)

METAR cache; press path reads the cache only; failures never erase the last
good observation.

## 8. Display — served JSON + noise-watch + admin (REMAINING, this pass)

- `panel-adsb-status` already attaches `noise_history` per aircraft
  (human/cat counts, first/last timestamps, event ids) and a top-level
  freshness block.
- `noise-watch.html` (browser + kiosk are the same file): distinct drawn
  human/cat marks (cat in amber — top priority; human in cyan), on map
  markers, hero, queue rows, and the desktop detail table, shown even when
  the aircraft is currently quiet. Follows the panel's existing
  drawn-SVG/colour discipline.
- `admin.html`: NOISE EVENTS table (time, outcomes, verdict + reason,
  identity, margin) and marked-airframes summary; WATCHING rows get history
  columns. FR24 deep links via the verified `/data/aircraft/<reg>#<fr24_id>`
  convention where available.
- Replay UI is deferred to the web/admin phase; when built, it follows the
  evidence-honesty rules recorded in `flight-replay-reference.md` (real
  timestamps, gap-broken paths, isolated points kept) [audit].

## 9. SQL analysis helpers (NEW from audit, small)

Add `db/deploy/noise_analysis_helpers.sql`: a `noise_event_audibility`-style
LAG query as a VIEW over `aircraft_track_point` (time each confirmed aircraft
spent within N km around an event), and a track-gap finder for archive QA
[audit: time-in-radius SQL; flights-with-gaps]. Verify script included.
(If this proves too much for this pass, it moves to the roadmap section of the
README rather than silently disappearing.)

## 10. Tests (REMAINING, this pass)

Update: attribute (new API + floor/margin/provenances/corroboration), learn
(cross-outcome controls, legacy double-press, signature change), admin
(heartbeat semantics), hue-label (activity monitor, recovered press, env
paths, battery throttle). New: report, weather, track-fetch (attempts,
parsing), drain (label/attribution/track normalisers + parameter-parity tests
for every new INSERT — the audit's field-name-drift bug is the argument),
adsb-status (history attach, track-point throttling/dedup). All offline.

## 11. Docs + recovered evidence (REMAINING, this pass)

- README: rewrite the button section (attribution flow, operator reports,
  panel-heard demoted to non-authoritative legacy for this feature), install/
  verify blocks including schema queries against every new table, retention
  honesty (raw FR24 responses are not retained; pass log + track archive are
  the record), new-button resource-id note.
- `deploy/stopgap/recovered-operator-reports-2026-08-17.md`: the chat-report
  timestamps (ISO-UTC metadata → epoch, conversion provenance), correlated
  bridge event ids, the lost 04:09:49 press, and ready-to-run
  `panel-aircraft-report --corroborates` commands for the operator.
- The learner's proposal docs adopt the findings-doc format: quantified
  baseline, rejected alternatives with data, generalization check
  [audit: RADIO_TRANSCRIPTION_FINDINGS].

## 12. Verification (final step)

`python3 -m py_compile` on every touched executable, full stopgap unittest
suite, `git diff --check`. Report files changed, tests, and remaining real
limitations only.
