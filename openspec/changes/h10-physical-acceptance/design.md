# Design — Polar H10 physical acceptance

This follow-up validates the already-brokered H10 transport; it does not redesign the broker.

## Controlled run

1. Confirm the approved H10 identity is present and paired; do not pair, remove bonds, or infer an
   identity from a runtime `hciN` alias.
2. Confirm the broker is the sole application-level BlueZ owner and Wellue is inactive.
3. Start a finite `btmon` capture before starting the H10 collector.
4. Wake/wear the H10 and collect fresh HR/RR notifications.
5. Compare the packet trace, broker events, collector logs, raw/status evidence, and `vitalsd`
   replies. A `Connected` event without a fresh nonzero measurement is insufficient.
6. Stop the collector and prove lease/subscription release. Repeat only with explicit authorization
   for restart/reconnect or coexistence phases.

## Failure handling

- An unavailable sensor, connect timeout, notification stall, missing RR, malformed frame, or
  ambiguous controller is named `unknown`/`degraded`; it is never scored as healthy.
- Do not retry indefinitely, reset the adapter, or enable Wellue to make the test pass.
- If the gate fails, stop H10 and retain the bounded evidence bundle for the follow-up review.
