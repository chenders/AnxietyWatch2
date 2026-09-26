# Domain records placement and ownership decision

**Status:** Accepted by the owner during the iPhone-first MVP walkthrough.
**Applies to:** `T-M5a`, `T-M5b`, `T-I12`, and `T-I13`.

Domain schema placement: **the same PostgreSQL database as physiological data, in a separate `domain`
schema**. Domain records do not belong in the `samples` hypertable. Encrypted appliance backups include
the domain schema, while logical domain export and recovery remain separately supported. Journal and
anxiety records are not automatically deleted on the physiological-sample retention schedule.

Records surface owner: **the iPhone is the primary user-facing Records surface** for journal and anxiety
capture, editing, history, and deletion. The admin console remains the operational surface for conflict
review, recovery, audit, and server administration; it is not a second full user-facing editor.

The first iPhone release includes journal/anxiety records only: a required 1–10 anxiety rating and
optional notes, tags, and song reference. Medication, check-in, and watch records are deferred.

The Pi is the long-term source of truth. The iPhone keeps a durable temporary outbox until item-level
acknowledgement, then reconciles the acknowledged record into a permanent Data-Protected local mirror
for journal/anxiety history. The mirror is a replica for offline reading, not a second authority or
analysis store. Physiological projection caches remain bounded and expiring.

The canonical record shape is an append-only `domain_records` envelope with strict kind-specific
validation. A correction creates a new ID with `supersedes`; deletion creates a new tombstone ID. A
same-ID/same-content retry is a no-op replay. A same-ID/different-content submission is a surfaced
conflict and never silently overwrites either version. Queued entries are never discarded by logout,
reset, or server replacement. The first-draft entry time is the iPhone capture time; the Pi receipt
instant is retained separately as transport evidence.
