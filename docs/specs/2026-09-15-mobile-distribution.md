# First-draft mobile distribution decision

**Status:** Accepted by the owner for the first iPhone draft.

Install the first draft directly on the owner's physical iPhone through Xcode/XcodeBuildMCP using the
existing Apple Development team and managed signing identity. Do not add a TestFlight or Ad Hoc
artifact pipeline yet.

TestFlight can be added after the Pi contracts, enrollment, offline outbox behavior, and
physical-device acceptance tests stabilize. Distribution credentials and App Store Connect secrets
must remain outside the repository and CI logs.
