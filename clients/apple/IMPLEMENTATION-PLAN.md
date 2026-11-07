# AnxietyWatch2 Thin Client Implementation Plan

## Overview

Implement the three core thin-client capabilities:
1. **HealthKit Proxy** — Forward source samples to Pi (no local aggregation)
2. **Person-Local Inputs** — Journal entries, medication doses, Quick Log
3. **Pi Projections Display** — Trends, status, history from Pi

## Current State (as of 2026-11-07)

### ✅ Complete
- HealthKit acquisition pipeline (26 types, anchored queries, observers, background delivery)
- Journal capture (JournalCaptureCoordinator, domain commands, song search)
- Watch Quick Log (WatchRelayQueue, WCSession relay)
- Sync engine (batch transport, P-256 signing, exponential backoff)
- Pi projections (ProjectionReadService, cache, Today surface)
- Enrollment (artifact validation, Secure Enclave key, credential storage)
- APNs registration and outcome reporting
- **MedicationsSurface** — Quick Log section, recent doses, dose-anxiety prompt sheet
- **HistorySurface** — projection visualization, metric/range pickers, gap markers
- **RandomCheckInManager** — 2/day default, 8-22 active hours, UserDefaults persistence
- **DoseFollowUpManager** — 30-minute follow-up prompts for promptable doses

### 🚧 In Progress
- Projection fetch in HistorySurface uses sample data (needs real ProjectionReadService integration)
- Medication domain commands (`medication_dose` write not yet wired through outbox)

### ❌ Not Started
- Medication definitions fetch from Pi (needs API endpoint or registry lookup)

## Implementation Tasks

### Phase 1: Medication Domain Commands (AWDomain)

1. Add `MedicationCommand` to `DomainCommands.swift`:
   - `medication_dose` record type with: id, medication_name, dose_mg, timestamp, is_prn, notes
   - `medication_definition` read-only from Pi (not a write command)

2. Add `MedicationDoseCapture` coordinator similar to `JournalCaptureCoordinator`

3. Add tests in `AWDomainTests`

### Phase 2: Medications Surface (AnxietyWatchApp) ✅

Completed in PR #89:
- Created `MedicationsSurface` view with Quick Log section, recent doses list
- Created dose-anxiety prompt sheet for meds with `promptAnxietyOnLog`
- Uses sample data pending `medication_dose` outbox integration

### Phase 3: History Surface (AnxietyWatchApp) ✅

Completed in PR #89:
- Created `HistorySurface` view with Charts framework
- Metric selector (HR, SpO2, respiratory rate, resting HR, steps)
- Date range picker (day/week/month)
- Gap markers for missing intervals
- Uses sample data pending ProjectionReadService integration

### Phase 4: Local Notifications (AnxietyWatchApp) ✅

Completed in PR #89:
- `RandomCheckInManager`: 2/day default, 8-22 active hours, UserDefaults, notification categories
- `DoseFollowUpManager`: 30-minute delay, pending follow-up tracking, dose linking
- Swift 6 concurrency with `@preconcurrency import UserNotifications`

## Dependencies

- Pi must provide `medication_definition` records via projections or a new endpoint
- Pi must accept `medication_dose` record type in `/api/v1/client/items`

## Testing Strategy

1. Unit tests for domain commands and capture coordinators
2. Integration tests for outbox → sync → Pi round-trip
3. UI tests for dose capture flow
