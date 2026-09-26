# Amendment 14 — Phone-silence escalation

**Status:** Accepted 2026-09-17

**Decision:** Pi-owned phone-silence detection escalates through the configured outbound email
channel, not through APNs or any notification delivered only to the phone whose silence is being
reported.

**Phone-silence escalation decision:** After two consecutive expected phone intervals without a
qualifying status beacon or acknowledged delivery, the Pi records the condition and sends one email
on the transition to `FAILING`. While the condition remains unresolved, reminders are coalesced to at
most one per hour. Recovery sends one email when the phone returns to an acknowledged state. The
console and bedside display remain the local surfaces for the condition.

This path is independent enough because the detector and decision live on the Pi, while outbound
email uses the operator's separately configured SMTP account and can reach destinations other than
the phone. APNs is deliberately excluded from this condition: sending a phone-silence notice to the
same phone would create a notification loop and cannot report a phone that is dark, offline, or not
being carried.

Email is an operational notice, never the CNS alarm path. SMTP failures remain visible as channel
failures and do not block ingest, liveness evaluation, or local alarm behavior. T-M2e owns detection,
transition identity, and coalescing events; T-M2f owns delivery, rate limiting, and test evidence.
