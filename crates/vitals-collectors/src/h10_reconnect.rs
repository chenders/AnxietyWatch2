//! Pure reconnect policy for the Polar H10 transport.
//!
//! `BlueZ` is Linux-only, but the rules deciding when and how quickly to
//! reconnect are host-independent so the failure modes remain unit-testable.

use std::time::Duration;
use vitals_registry::{Cadence, VitalsSource};

/// A live H10 sends approximately one notification per second. Missing this
/// many registry-declared cadences means the notification stream is wedged,
/// even if `BlueZ` has not closed it and still reports the process as healthy.
const MISSED_CADENCES_BEFORE_RECONNECT: u32 = 15;

/// The collector retries quickly for a bedside dropout, then stops hammering
/// `BlueZ`. The small jitter prevents several collectors from synchronizing after
/// a shared controller restart.
const INITIAL_BACKOFF_SECONDS: u64 = 1;
const MAX_BACKOFF_SECONDS: u64 = 30;

/// Why a subscribed link must be torn down and re-established.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkLoss {
    /// `BlueZ` explicitly ended the GATT notification stream.
    StreamEnded,
    /// `BlueZ`'s `Device1.Connected` property is false even though the
    /// notification stream has not ended.
    Disconnected,
    /// `BlueZ` still reports connected, but no notification has arrived within
    /// the source-derived deadline.
    NotificationIdle {
        /// Elapsed time since the most recent notification.
        idle_for: Duration,
    },
}

/// Derive the notification watchdog from the registry rather than duplicating
/// the H10 cadence in the collector.
#[must_use]
pub fn notification_idle_timeout(source: VitalsSource) -> Duration {
    match source.expected_cadence() {
        Cadence::EverySeconds(seconds) => Duration::from_secs(u64::from(
            seconds.saturating_mul(MISSED_CADENCES_BEFORE_RECONNECT),
        )),
        // The H10 variants are periodic, so this branch is defensive for a
        // future caller and deliberately fails on a short, visible deadline.
        Cadence::OnDemand => Duration::from_secs(30),
    }
}

/// Assess a subscribed link independently of whether the notification stream
/// happened to yield `None`. A stale stream was the production failure: `BlueZ`
/// reported disconnected for nine hours while `notifications.next()` remained
/// pending forever.
#[must_use]
pub fn assess_link(
    connected: bool,
    idle_for: Duration,
    idle_timeout: Duration,
) -> Option<LinkLoss> {
    if !connected {
        Some(LinkLoss::Disconnected)
    } else if idle_for >= idle_timeout {
        Some(LinkLoss::NotificationIdle { idle_for })
    } else {
        None
    }
}

/// Capped exponential reconnect delay with bounded positive jitter.
#[derive(Debug, Clone)]
pub struct ReconnectBackoff {
    next_base_seconds: u64,
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self {
            next_base_seconds: INITIAL_BACKOFF_SECONDS,
        }
    }
}

impl ReconnectBackoff {
    /// Return the next delay. `jitter_seed` is supplied by the transport so the
    /// pure policy needs no RNG or platform dependency.
    pub fn next_delay(&mut self, jitter_seed: u64) -> Duration {
        let base = self.next_base_seconds;
        let jitter_room = base / 4;
        let jitter = if jitter_room == 0 {
            0
        } else {
            jitter_seed % (jitter_room + 1)
        };
        let seconds = base.saturating_add(jitter).min(MAX_BACKOFF_SECONDS);
        self.next_base_seconds = base.saturating_mul(2).min(MAX_BACKOFF_SECONDS);
        Duration::from_secs(seconds)
    }

    /// A received notification proves the whole BLE subscription path works;
    /// only that evidence resets the backoff.
    pub fn reset(&mut self) {
        self.next_base_seconds = INITIAL_BACKOFF_SECONDS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnected_property_breaks_a_still_pending_stream() {
        assert_eq!(
            assess_link(false, Duration::ZERO, Duration::from_secs(15)),
            Some(LinkLoss::Disconnected)
        );
    }

    #[test]
    fn connected_but_idle_link_is_reconnected() {
        let timeout = Duration::from_secs(15);
        assert_eq!(
            assess_link(true, Duration::from_millis(14_999), timeout),
            None
        );
        assert_eq!(
            assess_link(true, timeout, timeout),
            Some(LinkLoss::NotificationIdle { idle_for: timeout })
        );
    }

    #[test]
    fn h10_watchdog_comes_from_registry_cadence() {
        assert_eq!(
            notification_idle_timeout(VitalsSource::PolarH10),
            Duration::from_secs(15)
        );
        assert_eq!(
            notification_idle_timeout(VitalsSource::PolarH10Emulator),
            Duration::from_secs(15)
        );
        assert_eq!(
            notification_idle_timeout(VitalsSource::Derived),
            Duration::from_secs(30),
            "a future accidental on-demand caller must retain a finite watchdog"
        );
    }

    #[test]
    fn reconnect_backoff_is_exponential_capped_and_resettable() {
        let mut backoff = ReconnectBackoff::default();
        let delays: Vec<_> = (0..8).map(|_| backoff.next_delay(0)).collect();
        assert_eq!(
            delays,
            [1, 2, 4, 8, 16, 30, 30, 30].map(Duration::from_secs)
        );

        backoff.reset();
        assert_eq!(backoff.next_delay(0), Duration::from_secs(1));
    }

    #[test]
    fn jitter_never_exceeds_the_cap() {
        let mut backoff = ReconnectBackoff::default();
        for _ in 0..20 {
            assert!(backoff.next_delay(u64::MAX) <= Duration::from_secs(MAX_BACKOFF_SECONDS));
        }
    }
}
