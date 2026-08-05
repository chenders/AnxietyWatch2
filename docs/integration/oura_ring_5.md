# Oura Ring 5 Integration — Technical Implementation Plan for AnxietyWatch2

## TL;DR
- **The recommended path is the Oura Cloud API V2.** It uniquely exposes raw beat-to-beat IBI/RR via `/v2/usercollection/interbeat_interval`, giving true HRV parity with the Polar H10 without needing BLE.
- **Direct BLE is reserved as an optional Phase 3 for on-demand/real-time raw signals**, but is deprioritized due to Oura's closed ecosystem, key extraction problems, and single-client contention.
- **The Cloud API requires OAuth2 and an active Oura Membership for Gen3+ users.**

## Integration Architecture in AnxietyWatch2

In the AnxietyWatch2 backend (`vitalsd`), Oura data is fetched via the Cloud API and written to the PostgreSQL `samples` table. Because Oura data is inherently delayed (requiring a sync to the Oura mobile app and then to the cloud), it feeds only the historical baseline and long-term correlations, never the real-time CNS depression klaxon.

### 1. Data Mapping to `vitals-registry`

- **Beat-to-beat IBI/RR:** `/v2/usercollection/interbeat_interval` returns individual IBI values in milliseconds. This is decoded by `vitalsd` workers and stored in `samples` identically to Polar H10 RR intervals, but flagged with `latency_class=delayed`.
- **Heart rate (BPM):** `/v2/usercollection/heartrate`
- **SpO2:** `/v2/usercollection/daily_spo2` returns a nightly aggregate.
- **Temperature deviation:** `daily_readiness`

### 2. Backend (Rust / tokio) Implementation

- **OAuth2 token storage:** Store `access_token`, `refresh_token`, and `expires_at` securely in PostgreSQL. Implement an async task in `vitalsd` to refresh tokens before expiry.
- **Webhook receiver:** A hyper/axum HTTP endpoint to receive `/v2/webhook/subscription` events. Verify the `x-oura-signature` HMAC. Because webhooks indicate new data is available, this endpoint triggers the asynchronous fetch workers.
- **Polling fallback:** A nightly backfill job to catch missed webhooks.

### 3. Real-time Exclusion

Data fetched via the Oura API must NOT trigger the real-time CNS klaxon in `vitals-core`. Oura SpO2 is a nightly average, not a live continuous stream like the EMAY. Ensure that the `vitals-core` Spool correctly segregates Oura samples.

## Known Limits

- **Membership dependency is a hard gate:** Any Ring 5 user without an active Oura Membership yields zero API data.
- **Webhooks depend on app sync:** Data only flows after the user's ring syncs to the Oura mobile app on their phone.
- **Legal:** Oura's API Agreement contains anti-competition and no-AI-training clauses.
