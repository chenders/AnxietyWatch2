//! Off-page source silence detection.
//!
//! The detector owns scheduling, transition identity, and notification
//! coalescing, but delegates the health decision itself to
//! `vitals_core::liveness::evaluate`. It has no clock or I/O; callers provide
//! the observation time and persist the returned status/event.

use std::collections::HashMap;
use vitals_core::liveness::{Bounds, Cadence, Evidence, HealthState, evaluate};
use vitals_registry::VitalsSource;

/// Minimum time between repeated notifications for one unchanged state.
pub const DEFAULT_REPEAT_INTERVAL_S: i64 = 900;

/// A source's operator configuration for one detector observation.
#[derive(Debug, Clone, Copy)]
pub struct SourcePolicy {
    /// Registry source whose cadence is evaluated.
    pub source: VitalsSource,
    /// Standing declaration that this source is intentionally absent.
    pub declared_absent: bool,
    /// An operator mute expiry, in UTC epoch seconds.
    pub mute_until_epoch_s: Option<i64>,
}

/// Persisted status observation. `reported_at` changes on every observation;
/// `changed_at` changes only when the evaluated health state changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusObservation {
    /// Stable source id.
    pub source: VitalsSource,
    /// Underlying evaluated state; mute and configuration never rewrite it.
    pub state: HealthState,
    /// Evaluator reason, including cadence and observed values.
    pub reason: String,
    /// Time this state was reported.
    pub reported_at_epoch_s: i64,
    /// Time this state began.
    pub changed_at_epoch_s: i64,
    /// Next eligible repeat time, with caller-supplied jitter included.
    pub next_repeat_at_epoch_s: Option<i64>,
    /// Stable identity for this state transition.
    pub transition_id: String,
    /// Whether the source is declared absent.
    pub declared_absent: bool,
    /// Active mute expiry, if any.
    pub muted_until_epoch_s: Option<i64>,
}

/// A notification candidate for the outbound channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEvent {
    /// Source that changed or is due a jittered repeat.
    pub source: VitalsSource,
    /// Same identity is coalesced by downstream channels.
    pub transition_id: String,
    /// Underlying state, never `Muted` or `NotConfigured`.
    pub state: HealthState,
    /// Reason at the time of emission.
    pub reason: String,
    /// Event creation time.
    pub emitted_at_epoch_s: i64,
    /// Whether an operator mute suppressed this event.
    pub suppressed_by_mute: bool,
}

#[derive(Debug, Clone)]
struct StoredObservation {
    state: HealthState,
    changed_at_epoch_s: i64,
    next_repeat_at_epoch_s: Option<i64>,
    transition_id: String,
    muted_until_epoch_s: Option<i64>,
}

/// Pure detector state. Persist `StatusObservation` and `NotificationEvent`
/// after each call so process restart does not turn silence into a new alert.
#[derive(Debug, Clone)]
pub struct SilenceDetector {
    statuses: HashMap<VitalsSource, StoredObservation>,
    repeat_interval_s: i64,
}

impl Default for SilenceDetector {
    fn default() -> Self {
        Self::new(DEFAULT_REPEAT_INTERVAL_S)
    }
}

impl SilenceDetector {
    /// Create a detector with a positive repeat interval.
    #[must_use]
    pub fn new(repeat_interval_s: i64) -> Self {
        Self {
            statuses: HashMap::new(),
            repeat_interval_s: repeat_interval_s.max(1),
        }
    }

    /// Restore a previously persisted status before the next observation.
    pub fn restore(&mut self, status: &StatusObservation) {
        self.statuses.insert(
            status.source,
            StoredObservation {
                state: status.state,
                changed_at_epoch_s: status.changed_at_epoch_s,
                next_repeat_at_epoch_s: status.next_repeat_at_epoch_s,
                transition_id: status.transition_id.clone(),
                muted_until_epoch_s: status.muted_until_epoch_s,
            },
        );
    }

    /// Evaluate one source and return its status plus an optional notification.
    ///
    /// `jitter_s` must be non-negative and is applied only to a repeat; a
    /// transition is immediate. `evaluate` is called directly so cadence and
    /// precedence cannot drift between the console and this daemon.
    #[expect(
        clippy::too_many_lines,
        reason = "the observation transaction keeps state, event, and repeat scheduling atomic"
    )]
    pub fn observe(
        &mut self,
        now_epoch_s: i64,
        policy: SourcePolicy,
        evidence: &Evidence,
        jitter_s: i64,
    ) -> (StatusObservation, Option<NotificationEvent>) {
        let bounds = Bounds {
            max_sample_arrival_s: policy.source.max_sample_arrival_s(),
            sample_silence_failure_s: policy.source.sample_silence_failure_s(),
            max_event_age_s: policy.source.max_event_age_s(),
            max_link_silence_s: policy.source.max_link_silence_s(),
            ..Bounds::from_cadence(match policy.source.expected_cadence() {
                vitals_registry::Cadence::EverySeconds(seconds) => Cadence::EverySeconds(seconds),
                vitals_registry::Cadence::OnDemand => Cadence::OnDemand,
            })
        };
        let assertion = evaluate(now_epoch_s, bounds, evidence);
        let previous = self.statuses.get(&policy.source).cloned();
        let state_changed = previous
            .as_ref()
            .is_none_or(|previous| previous.state != assertion.state);
        let changed_at = if state_changed {
            now_epoch_s
        } else {
            previous
                .as_ref()
                .map_or(now_epoch_s, |previous| previous.changed_at_epoch_s)
        };
        let transition_id = previous
            .as_ref()
            .filter(|_previous| !state_changed)
            .map_or_else(
                || {
                    format!(
                        "{}:{}:{}",
                        policy.source.id(),
                        state_name(assertion.state),
                        changed_at
                    )
                },
                |previous| previous.transition_id.clone(),
            );
        let muted = policy
            .mute_until_epoch_s
            .is_some_and(|until| until > now_epoch_s);
        let unmuted_after_previous = previous
            .as_ref()
            .and_then(|previous| previous.muted_until_epoch_s)
            .is_some_and(|until| until <= now_epoch_s)
            && !muted;
        let transition_event = !policy.declared_absent
            && (state_changed && previous.is_some() || unmuted_after_previous)
            && !muted;
        let initial_bad_event = !policy.declared_absent
            && previous.is_none()
            && assertion.state != HealthState::Live
            && !muted;
        let should_emit = transition_event || initial_bad_event;
        let repeat_due = !state_changed
            && !policy.declared_absent
            && !muted
            && assertion.state != HealthState::Live
            && previous
                .as_ref()
                .and_then(|previous| previous.next_repeat_at_epoch_s)
                .is_some_and(|next| now_epoch_s >= next);
        let event = if should_emit || repeat_due {
            Some(NotificationEvent {
                source: policy.source,
                transition_id: transition_id.clone(),
                state: assertion.state,
                reason: assertion.reason.clone(),
                emitted_at_epoch_s: now_epoch_s,
                suppressed_by_mute: false,
            })
        } else if (state_changed || repeat_due) && muted && assertion.state != HealthState::Live {
            Some(NotificationEvent {
                source: policy.source,
                transition_id: transition_id.clone(),
                state: assertion.state,
                reason: assertion.reason.clone(),
                emitted_at_epoch_s: now_epoch_s,
                suppressed_by_mute: true,
            })
        } else {
            None
        };
        let next_repeat = if assertion.state == HealthState::Live || muted {
            None
        } else if event.is_some() || state_changed {
            Some(
                now_epoch_s
                    .saturating_add(self.repeat_interval_s)
                    .saturating_add(jitter_s.max(0)),
            )
        } else {
            previous
                .as_ref()
                .and_then(|previous| previous.next_repeat_at_epoch_s)
        };
        let status = StatusObservation {
            source: policy.source,
            state: assertion.state,
            reason: assertion.reason,
            reported_at_epoch_s: now_epoch_s,
            changed_at_epoch_s: changed_at,
            next_repeat_at_epoch_s: next_repeat,
            transition_id,
            declared_absent: policy.declared_absent,
            muted_until_epoch_s: policy
                .mute_until_epoch_s
                .filter(|until| *until > now_epoch_s),
        };
        self.statuses.insert(
            policy.source,
            StoredObservation {
                state: status.state,
                changed_at_epoch_s: status.changed_at_epoch_s,
                next_repeat_at_epoch_s: status.next_repeat_at_epoch_s,
                transition_id: status.transition_id.clone(),
                muted_until_epoch_s: status.muted_until_epoch_s,
            },
        );
        (status, event)
    }
}

fn state_name(state: HealthState) -> &'static str {
    match state {
        HealthState::Live => "live",
        HealthState::Failing => "failing",
        HealthState::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 10_000;

    fn evidence(attempt: i64, failure: Option<&str>) -> Evidence {
        // These tests exercise the detector's scheduling and transition
        // identity, not the arrival rule, so the sample, contact and activity
        // channels are left absent on purpose — the sources they use declare
        // no arrival bounds, and absent contributes nothing.
        Evidence {
            last_attempt_epoch_s: Some(attempt),
            last_attempt_failure: failure.map(str::to_string),
            last_success_epoch_s: Some(attempt),
            ..Default::default()
        }
    }

    #[test]
    fn silence_silent_phone_emits_once_then_jittered_repeat() {
        let mut detector = SilenceDetector::new(100);
        let stale = evidence(NOW - 61, None);
        let (first, first_event) = detector.observe(
            NOW,
            SourcePolicy {
                source: VitalsSource::Emay,
                declared_absent: false,
                mute_until_epoch_s: None,
            },
            &stale,
            7,
        );
        assert_eq!(first.state, HealthState::Unknown);
        assert!(first_event.is_some());
        assert_eq!(first.next_repeat_at_epoch_s, Some(NOW + 107));

        let (_, repeated_too_soon) = detector.observe(
            NOW + 99,
            SourcePolicy {
                source: VitalsSource::Emay,
                declared_absent: false,
                mute_until_epoch_s: None,
            },
            &stale,
            7,
        );
        assert!(repeated_too_soon.is_none());
        let (_, repeated) = detector.observe(
            NOW + 107,
            SourcePolicy {
                source: VitalsSource::Emay,
                declared_absent: false,
                mute_until_epoch_s: None,
            },
            &stale,
            7,
        );
        assert!(repeated.is_some());
    }

    #[test]
    fn silence_failing_attempt_beats_staleness_and_recovers() {
        let mut detector = SilenceDetector::default();
        let policy = SourcePolicy {
            source: VitalsSource::Emay,
            declared_absent: false,
            mute_until_epoch_s: None,
        };
        let (failed, event) =
            detector.observe(NOW, policy, &evidence(NOW - 61, Some("timeout")), 0);
        assert_eq!(failed.state, HealthState::Failing);
        assert!(event.is_some());
        let (live, recovery) = detector.observe(NOW + 1, policy, &evidence(NOW, None), 0);
        assert_eq!(live.state, HealthState::Live);
        assert!(recovery.is_some());
        assert_eq!(live.changed_at_epoch_s, NOW + 1);
    }

    #[test]
    fn silence_mute_and_declared_absent_do_not_relabel_state() {
        let mut detector = SilenceDetector::default();
        let muted = SourcePolicy {
            source: VitalsSource::Emay,
            declared_absent: false,
            mute_until_epoch_s: Some(NOW + 100),
        };
        let (status, event) = detector.observe(NOW, muted, &evidence(NOW - 61, None), 0);
        assert_eq!(status.state, HealthState::Unknown);
        assert!(event.is_some_and(|event| event.suppressed_by_mute));
        assert_eq!(status.muted_until_epoch_s, Some(NOW + 100));

        let absent = SourcePolicy {
            source: VitalsSource::Emay,
            declared_absent: true,
            mute_until_epoch_s: None,
        };
        let (absent_status, absent_event) =
            detector.observe(NOW + 1, absent, &evidence(NOW - 61, None), 0);
        assert_eq!(absent_status.state, HealthState::Unknown);
        assert!(absent_status.declared_absent);
        assert!(absent_event.is_none());
    }

    #[test]
    fn silence_process_restart_restores_transition_identity() {
        let policy = SourcePolicy {
            source: VitalsSource::Emay,
            declared_absent: false,
            mute_until_epoch_s: None,
        };
        let stale = evidence(NOW - 61, None);
        let mut first = SilenceDetector::default();
        let (saved, event) = first.observe(NOW, policy, &stale, 0);
        assert!(event.is_some());
        let mut restarted = SilenceDetector::default();
        restarted.restore(&saved);
        let (again, no_duplicate) = restarted.observe(NOW + 1, policy, &stale, 0);
        assert_eq!(again.transition_id, saved.transition_id);
        assert!(no_duplicate.is_none());
    }
}
