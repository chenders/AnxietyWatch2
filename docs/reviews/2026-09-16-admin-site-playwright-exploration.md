# Admin/site Playwright exploration

**Date:** 2026-09-16 UTC
**Target:** `minidude` over the LAN (`:8480` console and `:8975` bedside panel)
**Method:** Chromium through Playwright, headless, with direct navigation through every
known console screen/layout and every static bedside page. Viewports tested were
`1280×900` and `800×480` where appropriate.

No POST, lifecycle action, credential submission, pairing operation, or therapy/control
operation was performed. The AirSense action controls were disabled because the deployed
console-admin capability is not configured.

## Executive finding

The universal admin console is real and works well as an **AirSense operational status
console**. Its navigation, three presentation layouts, read-only data boundary, service
status, credential-shape status, counters, logs, audit area, and disabled lifecycle
controls all render correctly.

It does **not yet accomplish the complete platform-admin role implied by the current
integration work**. Therapy-session history is not wired into the bedside page, the
AirSense admin screen does not expose the durable therapy/event/session view, and the
aircraft/noise tracker is still a separate site with no link from the universal console.
There are also live cross-system health discrepancies that the admin console correctly
surfaces but does not resolve.

## Routes explored

### Universal console (`:8480`)

- `/console/`
- `/console/?screen=overview`
- `/console/?screen=display`
- `/console/?screen=ingest`
- `/console/?screen=attempts`
- `/console/?screen=air11`
- Each of the five screens with `dir=a`, `dir=b`, and `dir=c`
- `/console/air11-kiosk.html`
- `/console/air11-bedside.html`

The universal console navigation exposes:

- Overview
- Display path
- Ingest
- Attempt history
- AirSense 11
- Layout: Service Mode, Preflight, and Flowpath

### Bedside/static panel (`:8975`)

- `/`
- `/index.html?kiosk=1`
- `/clinical-monitor.html`
- `/bedside-vitals.html`
- `/live-vitals.html?kiosk=1`
- `/polar-h10.html`
- `/noise-watch.html`
- `/noise-watch.html?kiosk=1`
- `/air11.html?kiosk=1`

## What works

### Universal admin shell

- All known console routes returned HTTP 200.
- All three layouts rendered the same underlying status model with the expected
  presentation differences.
- The AirSense screen shows separate link, measurement, process, credential, opt-in,
  host-receipt, counter, log, and audit sections.
- The page distinguishes `STREAMING / MEASURED`, service `ACTIVE`, credential `READY`,
  and `OPTED IN` rather than collapsing them into one health value.
- The page explicitly states that the AirSense integration is read-only and exposes no
  pairing or device-control operation.
- Credential display is bounded to presence, shape, and permissions. No key, device
  address, session key, or raw frame was rendered.
- The action controls were present but disabled because the external console-admin
  capability is not configured. The page explains that blocker and instructs operators
  not to paste the AirSense key into the page.
- Bounded service logs and append-only admin audit sections are present and clearly show
  their empty states.
- The AirSense kiosk rendered the same safe projection without an admin token field or
  lifecycle controls.

### Evidence and missing-data behavior

- The bedside and vitals pages preserve unknown/stale/unavailable states instead of
  fabricating values.
- The AirSense bedside page explicitly states that AHI is not derived from the stream.
- The aircraft page rendered live aircraft rows, range/altitude/bearing/phase, noise
  status, FlightRadar24 map/history links, dongle freshness, and credit information.
- The demo clinical panel visibly says `DEMO DATA · NOT LIVE`.
- No horizontal overflow was found at the tested 800-pixel width.

## Findings requiring follow-up

### 1. AirSense bedside history request is broken — high priority

`/console/air11-bedside.html` and the `:8975/air11.html?kiosk=1` wrapper request:

```text
GET http://minidude:8480/api/air11/history?window_hours=48
```

The endpoint returns HTTP 404. The page consequently displays:

> Stored history is not available.

This is a concrete integration defect, not merely a missing-data state. The page can
show current respiration/leak/pressure and event count, but it cannot show the stored
history it asks for.

### 2. Durable therapy/session data is not yet an admin view — high priority

The live `/api/air11` response contains therapy/event fields including:

- `event_notifications`
- `event_records`
- `event_subscription_state`
- `therapy_state`
- therapy start/stop/finalization timestamps
- `therapy_blocker`

The universal AirSense admin screen does not render those fields. The bedside page
renders only an event count and a simplified therapy state; it does not show completed
sessions, current-session details, end reason/confidence, event evidence, or a session
summary. At the time of exploration it reported `therapy_state: unknown` after a
collector restart, which is correct conservative behavior but not yet a useful session
history view.

This means the admin surface currently supports transport/collector monitoring, not the
full Amendment 12 therapy-session inspection requirement.

### 3. Aircraft/noise tracking is outside the universal admin console — medium priority

The aircraft tracker is functional at:

```text
http://minidude:8975/noise-watch.html
```

However, it is not present in the `:8480/console/` navigation and none of the universal
console screens links to it. The operator must know a separate port and URL. If the
intended product is one universal admin site, aircraft/noise needs either a console
screen or an explicit cross-link and ownership explanation.

### 4. Global admin health contains live integration mismatches — medium priority

The Overview/Display/Ingest screens surfaced these actual states during exploration:

- `vitalsd` is `UNKNOWN` with a CBOR protocol error: missing field `lag_bytes`.
- Hardware heartbeat is `UNKNOWN` because the daemon did not report heartbeat state.
- Undervoltage is `UNKNOWN` because `vcgencmd get_throttled` failed.
- HDMI outputs are `FAILING` because they report disconnected.
- Writeback connectors are `UNKNOWN`.
- The generic Ingest screen reports `ResmedAir11` as never observed while the dedicated
  AirSense screen reports a streaming/measured collector and current PostgreSQL values.

The console is right to show these as unknown/failing rather than healthy. They need
reconciliation so that the universal admin page gives a coherent appliance picture and
so that AirSense ingestion is represented consistently in the generic source view.

### 5. Some kiosk pages exceed the physical 800×480 frame — medium priority

At an 800×480 viewport, measured document heights were:

| Page | Document height |
|---|---:|
| AirSense bedside | 548 px |
| AirSense kiosk | 2,213 px |
| Live vitals kiosk | 909 px |
| Noise-watch kiosk | 480 px |
| Demo clinical panel | 480 px |
| Bedside vitals | 713 px |

There was no horizontal overflow, but the AirSense bedside page is taller than the
physical frame used by the kiosk configuration. The AirSense kiosk and live-vitals pages
are clearly long-form monitor/admin views rather than one-frame 800×480 kiosk views.
This should be made explicit in deployment, or the physical bedside route should be
reduced to the information intended to be visible without scrolling.

### 6. Missing favicon produces avoidable browser errors — low priority

Chromium reported a 404 for `/favicon.ico` on the universal-console root and the
stopgap panel root. This does not affect data or controls, but it creates noisy browser
console errors during an otherwise clean exploration.

## Acceptance assessment

| Admin responsibility | Result |
|---|---|
| One universal console with navigation | **Pass** |
| Three layouts over one status model | **Pass** |
| AirSense read-only operational status | **Pass** |
| Safe credential boundary | **Pass** |
| Disabled/authorized lifecycle actions | **Pass, not enabled in deployment** |
| Bounded logs and audit presentation | **Pass, empty in this run** |
| Explicit unknown/stale/missing states | **Pass** |
| AirSense current bedside display | **Partial** |
| AirSense stored history | **Fail: endpoint 404** |
| Durable therapy-session/event admin view | **Fail: not exposed** |
| Aircraft tracker inside universal admin | **Fail: separate site** |
| Coherent appliance-wide health projection | **Partial: live mismatches surfaced** |
| Fixed 800×480 for every kiosk route | **Partial** |

## Follow-up verification after the exploration

At 2026-09-16 02:20 UTC, the routed history handler was deployed to `minidude` and
verified directly:

- `GET /api/air11/history?window_hours=48` returned HTTP 200.
- The response was `state: observed`, with a provisional gap-bounded boundary and
  stored series data.
- A fresh Playwright load of `/console/air11-bedside.html` rendered `Stored history is
  available` and no longer reported the history endpoint as a 404.
- The remaining browser console error on that page is the unrelated missing
  `/favicon.ico` request.

The history defect is therefore fixed in the deployed console. A subsequent deployment
also added the current therapy session identity, lifecycle state, boundary timestamps,
event-subscription state, event counts, and lifecycle blocker to the AirSense admin
screen. Playwright verified those fields while the collector conservatively reported
`therapy_state: unknown` after restart. Completed-session records, deterministic
summaries, and a separate current/completed session view remain open; the UI does not
claim those are complete yet.

The aircraft-console, appliance-health, and viewport findings below remain open.

## Recommended order of work

1. Add completed/current therapy-session and event-record projections to the AirSense
   admin and bedside views, retaining conservative unknown states and evidence.
2. Decide whether aircraft/noise is a universal-console screen. If yes, add it to the
   console navigation and use an authenticated/bounded backend projection rather than
   relying on the separate static panel port.
3. Reconcile the generic Ingest source projection with the dedicated AirSense status.
4. Define which routes are true 800×480 physical kiosk pages and constrain or relabel
   the long-form pages accordingly.
5. Add a favicon or explicitly suppress the request to keep browser diagnostics clean.
