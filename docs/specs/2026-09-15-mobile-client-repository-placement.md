# Mobile client repository placement decision

**Status:** Accepted by the owner for the first draft.

The iPhone client remains in this repository under `clients/apple/`. A future Android client should
be added under `clients/android/`, with shared versioned OpenAPI/domain fixtures under the existing
contract surface rather than copied between repositories.

Keeping both clients beside the Pi API, domain schema, fixtures, and verification gates minimizes
contract drift while the enrollment, journal/anxiety records, outbox semantics, tombstones, and read
projections are still evolving. Platform-specific CI and signing boundaries remain separate.

A separate mobile repository may be reconsidered when mobile ownership or release cadence requires it,
but only alongside automated compatibility checks against the versioned server contracts.
