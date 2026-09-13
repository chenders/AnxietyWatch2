//! Conservative `AirSense` therapy-session boundary policy.
//!
//! This module distinguishes device therapy lifecycle evidence from the BLE
//! transport session. It intentionally keeps a stopped session provisional
//! while post-stop stream activity may still represent hose clearing or device
//! shutdown. Durable session records are a separate persistence concern.

/// Minimum post-stop settling interval before a quiet stream may finalize.
pub const POST_STOP_SETTLE_MICROS: i64 = 30_000_000;
/// Maximum post-stop settling interval before finalization is allowed.
pub const POST_STOP_SETTLE_MAX_MICROS: i64 = 180_000_000;

/// Conservative therapy-session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TherapyState {
    /// No lifecycle boundary has been observed.
    #[default]
    Unknown,
    /// Explicit start evidence has been observed.
    Active,
    /// Stop evidence has been observed but settling is incomplete.
    Provisional,
    /// The post-stop boundary passed without a new start.
    Complete,
}

/// Host-receipt lifecycle tracker for one current therapy session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TherapyLifecycle {
    state: TherapyState,
    start_host_unix_micros: Option<i64>,
    stop_host_unix_micros: Option<i64>,
    finalized_host_unix_micros: Option<i64>,
    stream_after_stop: bool,
}

impl TherapyLifecycle {
    /// Current lifecycle state.
    #[must_use]
    pub const fn state(self) -> TherapyState {
        self.state
    }

    /// Host receipt of the first start evidence.
    #[must_use]
    pub const fn start_host_unix_micros(self) -> Option<i64> {
        self.start_host_unix_micros
    }

    /// Host receipt of stop evidence.
    #[must_use]
    pub const fn stop_host_unix_micros(self) -> Option<i64> {
        self.stop_host_unix_micros
    }

    /// Host receipt at which the conservative boundary was finalized.
    #[must_use]
    pub const fn finalized_host_unix_micros(self) -> Option<i64> {
        self.finalized_host_unix_micros
    }

    /// Whether stream evidence arrived after stop evidence.
    #[must_use]
    pub const fn stream_after_stop(self) -> bool {
        self.stream_after_stop
    }

    /// Records one lifecycle event by its protocol name.
    ///
    /// `TherapyStart` and `MaskOn` establish activity. `TherapyStop`,
    /// `MaskOff`, and `StandbyStarted` begin the provisional post-stop window.
    /// A start during that window resumes the same current session rather than
    /// silently creating a second one.
    pub fn observe_event(&mut self, name: &[u8], host_unix_micros: i64) {
        if matches!(name, b"TherapyStart" | b"MaskOn") {
            if self.state == TherapyState::Complete || self.start_host_unix_micros.is_none() {
                self.start_host_unix_micros = Some(host_unix_micros);
            }
            self.state = TherapyState::Active;
            self.stop_host_unix_micros = None;
            self.finalized_host_unix_micros = None;
            self.stream_after_stop = false;
        } else if matches!(name, b"TherapyStop" | b"MaskOff" | b"StandbyStarted")
            && self.state != TherapyState::Complete
        {
            if self.start_host_unix_micros.is_none() {
                self.start_host_unix_micros = Some(host_unix_micros);
            }
            self.stop_host_unix_micros = Some(host_unix_micros);
            self.finalized_host_unix_micros = None;
            self.stream_after_stop = false;
            self.state = TherapyState::Provisional;
        }
    }

    /// Records a stream envelope received after a stop boundary.
    pub fn observe_stream(&mut self) {
        if self.state == TherapyState::Provisional {
            self.stream_after_stop = true;
        }
    }

    /// Advances the conservative post-stop boundary.
    ///
    /// Returns `true` when the public state changed. A quiet post-stop period
    /// may finalize after 30 seconds; any stream activity after the stop keeps
    /// the state provisional until the 180-second maximum.
    pub fn advance(&mut self, now_unix_micros: i64) -> bool {
        if self.state != TherapyState::Provisional {
            return false;
        }
        let Some(stop) = self.stop_host_unix_micros else {
            return false;
        };
        let elapsed = now_unix_micros.saturating_sub(stop);
        let can_finalize = if self.stream_after_stop {
            elapsed >= POST_STOP_SETTLE_MAX_MICROS
        } else {
            elapsed >= POST_STOP_SETTLE_MICROS
        };
        if !can_finalize {
            return false;
        }
        self.state = TherapyState::Complete;
        self.finalized_host_unix_micros = Some(now_unix_micros);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_start_and_stop_are_distinct_from_transport_state() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle.observe_event(b"TherapyStart", 1_000_000);
        assert_eq!(lifecycle.state(), TherapyState::Active);
        lifecycle.observe_event(b"TherapyStop", 2_000_000);
        assert_eq!(lifecycle.state(), TherapyState::Provisional);
        assert!(!lifecycle.advance(2_000_000 + POST_STOP_SETTLE_MICROS - 1));
        assert!(lifecycle.advance(2_000_000 + POST_STOP_SETTLE_MICROS));
        assert_eq!(lifecycle.state(), TherapyState::Complete);
    }

    #[test]
    fn post_stop_stream_activity_uses_the_maximum_settle_window() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle.observe_event(b"MaskOn", 10_000_000);
        lifecycle.observe_event(b"MaskOff", 20_000_000);
        lifecycle.observe_stream();
        assert!(!lifecycle.advance(20_000_000 + POST_STOP_SETTLE_MICROS));
        assert!(lifecycle.advance(20_000_000 + POST_STOP_SETTLE_MAX_MICROS));
        assert_eq!(lifecycle.state(), TherapyState::Complete);
    }

    #[test]
    fn a_new_start_during_settling_resumes_without_duplication() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle.observe_event(b"TherapyStart", 100);
        lifecycle.observe_event(b"TherapyStop", 200);
        lifecycle.observe_event(b"TherapyStart", 300);
        assert_eq!(lifecycle.state(), TherapyState::Active);
        assert_eq!(lifecycle.start_host_unix_micros(), Some(100));
        assert_eq!(lifecycle.stop_host_unix_micros(), None);
    }

    #[test]
    fn unrelated_events_do_not_create_a_session_boundary() {
        let mut lifecycle = TherapyLifecycle::default();
        lifecycle.observe_event(b"ValueChange", 100);
        lifecycle.observe_event(b"Leak", 200);
        assert_eq!(lifecycle.state(), TherapyState::Unknown);
    }
}
