# Flight-path replay: reference implementation in phx-helicopter-tracker

**Purpose of this note.** The owner's sibling project at
`~/Source/phx-helicopter-tracker` (also on GitHub: `chenders/phx-helicopter-tracker`)
contains a mature, production-tested flight replay and visualisation stack
built on exactly the data this project now archives in `aircraft_track_point`.
When the web dashboard or admin console grows a "replay this noise event"
view, **start from that code — do not reinvent it.** It represents months of
iteration (190 commits), including a lot of paid-for dead ends.

## What it has that we will want

### 3D replay (Cesium + Google photorealistic tiles)

- `frontend/src/components/FlightVisualization3DCesium*.tsx` — several
  generations; `FlightVisualization3DCesiumNPM.tsx` and
  `...CesiumFixed.tsx` are the ones that shipped. `frontend/src/config/cesiumConfig.ts`
  holds tokens/terrain config.
- `FlightVisualization3DPhotorealistic.tsx` — Google Maps photorealistic 3D
  tiles variant, with a pilot-view camera animated by `requestAnimationFrame`
  along the recorded positions; camera speed scales with recorded ground
  speed, tilt with altitude (lines ~208–247).
- Third-person chase camera + helicopter model/icon with bearing indicator
  (commits `063db782`, `8b7bbfe9`).
- `FlightHUD.tsx` — draggable HUD showing speed / altitude / heading /
  timestamp during replay.
- Ground/street labels in 3D: `ScreenSpaceLabels.tsx`,
  `HolographicStreetLabels.tsx`, `StreetLabelList.tsx` — camera-based label
  placement with extensive Playwright visual tests
  (`frontend/tests/*label*.spec.ts`). This was hard-won; the tests encode the
  failure modes.
- `PhoenixMinimap.tsx` — 2D minimap synced to the 3D camera.

### 2D replay and analysis views

- Google Maps polyline flight paths with start/end markers, hover-location
  markers (surveillance analysis), animated playback, bounds-fitting.
- Heatmaps of position density; zone overlays.
- Jump-to-closest-approach, slider scrubbing, animation-speed controls —
  all covered by Playwright specs under `frontend/tests/` (e.g.
  `test-jump-to-closest.spec.ts`, `animation-speed.spec.ts`,
  `test-slider-navigation.spec.ts`). Read the tests first; they document the
  interaction design.

### Backend patterns already borrowed (2026-08-17)

These are noted here so nobody re-learns them the expensive way:

- `flight-tracks?flight_id=` response shape and the 10 s request-spacing
  floor → `deploy/stopgap/panel-fr24-track-fetch`.
- `historic/flight-positions/light` with a `timestamp` parameter can recover
  "what was overhead at time T" **retroactively** — the fallback if an
  operator report arrives hours late and the local record has a gap. Not yet
  wired here; see `backend/app/services/flightradar24_api_service.py`
  (`get_historical_positions`).
- Redis-backed rate limiter with per-minute/hour/day/month windows and
  adaptive backoff on 429 (`fr24_rate_limiter.py`); our single-process scale
  uses the simpler ledger, but if polling ever multiplies, copy theirs.
- Rebuild-stable flight identity: `public_id = sha256(UPPER(reg)|departure
  ISO-UTC)[:16]`, mirrored byte-for-byte in a Postgres trigger
  (`backend/app/core/flight_identity.py`). Vendor flight ids are
  rebuild-fragile; ours are stored alongside the registration for this
  reason.
- Hover detection: consecutive-point displacement < ~0.0005° (≈50 m) marks
  `is_hovering` and accumulates duration (`flight_tracker.py`
  `process_landed_flight`). Circling/orbit detection for the police workload
  should start from their `_analyze_complete_path`.
- Post-mortem (`POST_MORTEM.md`): schema drift silently blocked ingestion
  for two weeks because migrations were never applied and nothing checked.
  Our equivalent guard: the README verify step queries every noise table so
  a missed `sqitch deploy` fails loudly, not silently.

## What NOT to copy

- The multiple abandoned Cesium components (`...Native`, `...Experimental`,
  `...Debug`) are kept there as history; pick the shipped one.
- Their Celery/Redis worker topology — this project's scale is one Pi and a
  systemd timer.
