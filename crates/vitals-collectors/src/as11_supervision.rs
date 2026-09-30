//! Portable reconnect backoff policy for the `AirSense` stream collector.
//!
//! The policy is deliberately finite and deterministic. A failed link cannot
//! turn into a tight reconnect loop, and a successful authentication resets the
//! delay before the next independent failure.

use std::time::Duration;

/// Initial delay before retrying a failed `AirSense` connection.
pub const RECONNECT_BASE_DELAY: Duration = Duration::from_secs(1);
/// Maximum delay between reconnect attempts.
pub const RECONNECT_MAX_DELAY: Duration = Duration::from_mins(1);

/// Per-attempt protocol state. Every new connection starts unauthenticated
/// and unsubscribed, including a reconnect after an otherwise healthy stream.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectCycle {
    authenticated: bool,
    stream_started: bool,
}

impl ReconnectCycle {
    /// Creates a disconnected cycle.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            authenticated: false,
            stream_started: false,
        }
    }

    /// Begins a fresh connection attempt and clears the previous session state.
    pub const fn begin_connection(&mut self) {
        self.authenticated = false;
        self.stream_started = false;
    }

    /// Whether the authentication exchange is still required.
    #[must_use]
    pub const fn authentication_required(&self) -> bool {
        !self.authenticated
    }

    /// Records successful authentication for this connection only.
    pub const fn mark_authenticated(&mut self) {
        self.authenticated = true;
    }

    /// Whether the fixed stream subscription may be sent.
    #[must_use]
    pub const fn can_start_stream(&self) -> bool {
        self.authenticated && !self.stream_started
    }

    /// Records successful fixed stream subscription.
    pub const fn mark_stream_started(&mut self) {
        self.stream_started = true;
    }

    /// Whether this connection completed authentication.
    #[must_use]
    pub const fn authenticated(&self) -> bool {
        self.authenticated
    }
}

/// Exponential reconnect backoff with a fixed ceiling.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectBackoff {
    failures: u32,
}

impl ReconnectBackoff {
    /// Creates a backoff with no recorded failures.
    #[must_use]
    pub const fn new() -> Self {
        Self { failures: 0 }
    }

    /// Returns the next bounded delay and records one failed attempt.
    #[must_use]
    pub fn next_delay(&mut self) -> Duration {
        let shift = self.failures.min(6);
        let multiplier = 1_u64.checked_shl(shift).map_or(u64::MAX, |value| value);
        let seconds = RECONNECT_BASE_DELAY
            .as_secs()
            .saturating_mul(multiplier)
            .min(RECONNECT_MAX_DELAY.as_secs());
        self.failures = self.failures.saturating_add(1);
        Duration::from_secs(seconds)
    }

    /// Clears the failure history after a successful authenticated reconnect.
    pub const fn reset(&mut self) {
        self.failures = 0;
    }

    /// Number of failures recorded, for bounded status reporting.
    #[must_use]
    pub const fn failures(&self) -> u32 {
        self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reconnect_repeats_authentication_and_stream_subscription() {
        let mut cycle = ReconnectCycle::new();
        cycle.begin_connection();
        assert!(cycle.authentication_required());
        assert!(!cycle.can_start_stream());
        cycle.mark_authenticated();
        assert!(cycle.can_start_stream());
        cycle.mark_stream_started();
        assert!(!cycle.can_start_stream());

        cycle.begin_connection();
        assert!(cycle.authentication_required());
        assert!(!cycle.can_start_stream());
        cycle.mark_authenticated();
        assert!(cycle.can_start_stream());
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let mut backoff = ReconnectBackoff::new();
        let delays: Vec<Duration> = (0..10).map(|_| backoff.next_delay()).collect();
        assert_eq!(delays.first().copied(), Some(Duration::from_secs(1)));
        assert_eq!(delays.get(1).copied(), Some(Duration::from_secs(2)));
        assert_eq!(delays.get(5).copied(), Some(Duration::from_secs(32)));
        assert_eq!(delays.get(6).copied(), Some(RECONNECT_MAX_DELAY));
        assert!(delays.iter().all(|delay| *delay <= RECONNECT_MAX_DELAY));
        assert_eq!(backoff.failures(), 10);
    }

    #[test]
    fn authentication_reset_starts_at_the_base_delay() {
        let mut backoff = ReconnectBackoff::new();
        let _ = backoff.next_delay();
        let _ = backoff.next_delay();
        backoff.reset();
        assert_eq!(backoff.failures(), 0);
        assert_eq!(backoff.next_delay(), RECONNECT_BASE_DELAY);
    }
}
