# Change: CPAP data ingestion from ez Share SD

## Why

ResMed AirSense 11 therapy data is not available over its BLE interface. The machine does write
OSCAR-compatible EDF files to SD, and the installed ez Share adapter exposes those files over its
private Wi-Fi access point. Without a supervised local importer, CPAP data remains a manual or cloud-
dependent feed and a core input to the sleep-apnea/anxiety thesis can fail silently.

## What

Build a supervised, resumable batch path that mirrors new EDF files from the ez Share HTTP interface,
parses ResMed semantics out of process, and appends provenance-preserving samples to PostgreSQL.
Expose each attempt, its row effect, clock-transform evidence, and source health to the console.

The work is deliberately staged: transport and fixtures first; clinical mapping and identity contract
before database writes; deployment only after an operator verifies the card and route on real hardware.

## In scope

- A bounded, atomic ez Share HTTP mirror with content-hash manifest and replay tests.
- Clean-room ResMed EDF parsing using permissively licensed libraries; OSCAR is formats-only.
- Registry-backed CPAP source/type definitions and stable append-only ingest identity.
- Versioned CPAP clock correction applied at read time, never by rewriting stored timestamps.
- Attempt-ledger integration, systemd supervision, least-privilege networking, and console evidence.
- An authenticated admin setup screen for the card SSID and Wi-Fi password, including a safe connection test and password rotation.
- A manual EDF upload path may reuse the parser, but is not required by this change.

## Out of scope

- AirSense BLE therapy-data access or control.
- myAir cloud ingestion.
- Copying OSCAR GPL code.
- Alarm decisions based solely on delayed nightly CPAP imports.
- Reformatting the card or changing Pi network configuration without operator approval.

## Gates

Deployment tasks are blocked until the operator verifies the ez Share card API and Pi route. Parser and
transport work use recorded/synthetic fixtures and can proceed without hardware.
