# Live vitals kiosk and source-age investigation

**Date:** 2026-08-31  
**Host:** `pi-host`  
**Kiosk:** `http://pi-host:8975/live-vitals.html?kiosk=1`

## Kiosk contract

`live-vitals.html` is a display-only page. It keeps the Polar H10, Wellue, and
A&D UA-651BLE as separate source series:

- **Polar H10:** `WORKING` only when a fresh physical-device status has
  `live: true`.
- **Wellue:** `WORKING` only when the physical link, finger state, producer
  heartbeat, and physiological sample are all fresh.
- **A&D cuff:** shows the latest durable pressure reading, with its age and
  `arrival_estimate` provenance.

The page does not fill gaps, smooth values, or use a producer heartbeat as a
physiological sample. It is responsive on phones and tablets; the 800 × 480
bedside view remains fixed when used on the panel.

A Pi-side `live-vitals-watch` process checks the page, merged status document,
source states, database query path, and service states every 60 seconds for
approximately ten hours. It logs `OK` or `DEGRADED` and atomically publishes
`health.json` beside the page. The kiosk renders sensor-source state plus the
core, spool drainer, database, and page/kiosk results in a bottom **SYSTEM
HEALTH** section. The watcher does not restart sensors or invent readings.

## What looked like a drain problem

At approximately 06:34–06:38 PDT, the merged status document showed:

- H10 `live: true` with a current sample;
- Wellue `written_at` current, but `sample_at` approximately one hour old;
- Wellue `frames` increasing while `measurements` and `stored_samples` stayed
  unchanged;
- the latest BP row present in PostgreSQL.

These timestamps have different meanings:

| Field | Meaning |
|---|---|
| `sample_at` | Arrival time of the latest **valid physiological sample** from that source |
| `written_at` | Producer status heartbeat/file publication time |
| `merged_at` | Merger heartbeat time |

Therefore a fresh `written_at` does not mean a fresh Wellue measurement. The
page now says **“Transport heartbeat is fresh; last valid sample …”** when
that distinction applies, and labels the displayed value **“LAST VALID SAMPLE”**.

## Drainer verification

The drainer was not stalled:

- `vitalsd.service` was active with no reported errors.
- `vitals-spool-drain.service` was active with no insert or database errors.
- Its recent logs showed successful insertion and offsets for each sealed
  segment.
- The current active spool segment contained H10 rows only; it contained no
  Wellue or BP backlog.
- The active segment had not yet been sealed, so the newest H10 rows were
  durably on disk but had not all become PostgreSQL rows. This is the normal
  live-segment boundary, not data loss.
- PostgreSQL contained current H10/BP history through the latest sealed
  segment and the Wellue history through its latest valid measurement.

No destructive operation, `UPDATE`, or `DELETE` was used.

## Wellue finding

The raw Wellue journal was still advancing. A review of measurement-shaped
packets found two CRC-valid variants after the last accepted sample:

```text
AA 55 0F 08 01 SS PP 00 II 00 C0 CRC
AA 55 0F 08 01 SS PP 00 II 00 80 CRC
```

The existing provisional decoder accepts the first form and requires `0xC0`
as the final reserved/trailer byte before the CRC. The later `0x80` packets
contain plausible-looking fields and valid CRCs, but their exact semantics
have not yet been independently established. They are retained in the raw
journal and are not promoted to physiological rows by assumption.

Observed at investigation time:

- raw notification/frame count continued to increase;
- valid measurement count stopped at the last accepted sample;
- the source-4 database history stopped at the same time;
- no source-4 records were waiting in the active spool.

This places the apparent one-hour age at the Wellue decode/validation boundary,
not in PostgreSQL draining. The next engineering step is a separate capture and
protocol review of the `0x80` variant; widening the decoder solely because the
values look plausible would violate the reject-and-quarantine rule.

## Current interpretation

- **H10:** live and working after Polar app repair; current values are visible
  through the live status path, with durable spool drainage continuing at
  segment boundaries.
- **Wellue:** transport is alive, but the last accepted physiological sample
  is stale until the `0x80` variant is understood or the device returns to the
  known `0xC0` form.
- **BP:** latest cuff measurement is durable and displayed independently.
- **Kiosk:** served and running at the URL above.

## Follow-up — 2026-09-04

The H10 was put back on with wet electrodes and correct placement, but it was
still sending `HR=0` notifications while BlueZ reported it connected. A
controlled Raspberry Pi reboot cleared the stale Bluetooth/GATT state. After
boot, the bonded H10 was connected again and the live status feed was advancing
with valid HR and RR samples.

The EMAY SleepO2 was also present and reporting: its status showed `link=up`,
`finger=true`, fresh SpO₂/pulse values, and an increasing frame count. The live
vitals kiosk now uses that EMAY stream instead of the stale Wellue stream for its
second primary card; the Wellue feed remains available separately but is not
required by this kiosk view.
