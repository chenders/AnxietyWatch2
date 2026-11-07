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

### 🚧 In Progress
- Medications surface (placeholder → needs domain commands + UI)
- History surface (placeholder → needs projection rendering)

### ❌ Not Started
- Local notification infrastructure (random check-ins, dose follow-ups)
- Medication domain commands (`medication_dose`, `medication_definition`)

## Implementation Tasks

### Phase 1: Medication Domain Commands (AWDomain)

1. Add `MedicationCommand` to `DomainCommands.swift`:
   - `medication_dose` record type with: id, medication_name, dose_mg, timestamp, is_prn, notes
   - `medication_definition` read-only from Pi (not a write command)

2. Add `MedicationDoseCapture` coordinator similar to `JournalCaptureCoordinator`

3. Add tests in `AWDomainTests`

### Phase 2: Medications Surface (AnxietyWatchApp)

1. Create `MedicationsSurface` view with:
   - Quick Log section (one-tap dose logging per active med)
   - Recent doses list
   - Pi-provided medication definitions

2. Create `DoseAnxietyPromptView` for meds with `promptAnxietyOnLog`

3. Wire to outbox and sync engine

### Phase 3: History Surface (AnxietyWatchApp)

1. Create `HistorySurface` view with:
   - Projection charts (HR, SpO2, etc.)
   - Explicit gap markers
   - Date range navigation

2. Use `ProjectionReadService` with time windows

### Phase 4: Local Notifications (AnxietyWatchApp)

1. Create `RandomCheckInManager`:
   - Configurable frequency (default 2/day)
   - Active hours (default 8-22)
   - UserDefaults persistence

2. Create `DoseFollowUpManager`:
   - 30-minute delay after promptable medications
   - Links to triggering dose

3. Wire notifications to journal capture

## Dependencies

- Pi must provide `medication_definition` records via projections or a new endpoint
- Pi must accept `medication_dose` record type in `/api/v1/client/items`

## Testing Strategy

1. Unit tests for domain commands and capture coordinators
2. Integration tests for outbox → sync → Pi round-trip
3. UI tests for dose capture flow
