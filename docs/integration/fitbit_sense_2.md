# Integrating the Fitbit Sense 2 with AnxietyWatch2: Technical Implementation Plan

## TL;DR
- **Real-time is impossible via any Fitbit path for the Sense 2.** The cloud API only exposes data after a device→phone→cloud sync; the Sense 2 runs a locked-down OS that blocks all third-party on-device apps; and it does NOT support the standard-BLE Heart Rate broadcast. Fitbit must therefore be a **HISTORICAL/baseline input only** — never a real-time safety input to the CNS-depression early-warning fusion engine.
- **The valuable, achievable integration is the cloud API for overnight/historical correlation:** daily/nightly HRV (RMSSD), SpO2 trends, breathing rate, skin-temperature deviation, sleep stages, and intraday heart rate. **EDA/cEDA and the Stress Management Score are NOT available through any API.**
- **Build on the Google Health API, not the legacy Fitbit Web API.** The legacy API is deprecated. Use `health.googleapis.com/v4` with Google OAuth 2.0. In AnxietyWatch2, this fits into `vitalsd` (or a dedicated integration worker) as a delayed, polling/webhook-driven cloud source feeding the `samples` table with historical timestamps.

## Integration Architecture in AnxietyWatch2

In the AnxietyWatch2 backend, all physiological data is written to a PostgreSQL `samples` table. Because Fitbit data is heavily delayed (traveling from Watch → Phone → Cloud → `vitalsd`), it must be handled differently than real-time BLE streams:

1. **OAuth Handling:** `vitalsd` (or an auxiliary service) must manage Google OAuth 2.0 flows, storing rotating refresh tokens securely in the database.
2. **Polling / Webhooks:** Utilize Google Health webhooks (or schedule periodic polling) to fetch new data.
3. **Data Mapping (vitals-registry):**
   - Intraday HR maps to `VitalsType::HeartRate` with a `source` of `FitbitSense2`.
   - HRV maps to appropriate overnight RMSSD metrics.
   - SpO2 maps to `VitalsType::SpO2` (flagged as a 5-min EMA, not raw 1Hz continuous).
4. **Append-Only Immutable Storage:** Downloaded samples are converted to UTC epoch seconds and written to Postgres. They must never trigger the real-time CNS-depression alarm paths.

## Key Findings

- **Latency:** Data automatically syncs to the Google Health app throughout the day only when in Bluetooth range. Cloud processing adds further delay. This chain makes cloud data structurally unusable for a real-time alert.
- **No Direct BLE:** The Sense 2 does not expose `0x180D` Heart Rate Service to phones, unlike the Charge 6 or Pixel Watches. `vitalsd` cannot connect to it directly via BlueZ.
- **Tokens do not transfer:** Users must consent to the Google Health API natively. Access tokens are short-lived, and refresh tokens are single-use and rotating.

## Security & Scopes

- **Restricted Scopes:** `googlehealth.*` scopes are Restricted, requiring Google's Trust & Safety privacy/security review and an annual third-party CASA security assessment.
- **Data Model:** Ensure that any historical data backfilled by this integration carries a distinct `latency_class` or is explicitly excluded from the `vitals-core` real-time alerting Spool. The alarm engine must only trigger off low-latency `vitalsd` BlueZ sources.
