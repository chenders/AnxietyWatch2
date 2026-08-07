//! Source-health policy evaluator.
//!
//! Implements the liveness logic defined in the spec.
//! This module is pure and contains no I/O, clocks, or side-effects.

/// The expected execution cadence for a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    /// A source expected to run automatically on an interval.
    EverySeconds(u32),
    /// A source that only runs when triggered by an external event.
    OnDemand,
}

/// Information about a currently running (inflight) attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inflight {
    /// Started timestamp in epoch seconds.
    pub started_epoch_s: i64,
    /// Expected duration of the attempt in seconds.
    pub expected_duration_s: u32,
}

/// The recorded evidence about a source's attempts and successes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// Timestamp of the last attempt in epoch seconds.
    pub last_attempt_epoch_s: Option<i64>,
    /// `Some(reason)` when the most recent finished attempt failed.
    /// `None` implies the most recent finished attempt succeeded.
    pub last_attempt_failure: Option<String>,
    /// Timestamp of the last successful attempt in epoch seconds.
    pub last_success_epoch_s: Option<i64>,
    /// An attempt with no outcome yet.
    pub inflight: Option<Inflight>,
    /// Learned advertising interval; may TIGHTEN only.
    pub learned_interval_s: Option<u32>,
}

/// Computed health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// The source is operating within expected parameters.
    Live,
    /// The source has experienced a failure on its most recent attempt.
    Failing,
    /// The source's health cannot be determined (e.g. stale, never run, running too long).
    Unknown,
}

/// The outcome of evaluating a source's health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assertion {
    /// The evaluated health state.
    pub state: HealthState,
    /// The reason for the health state, naming values and thresholds.
    pub reason: String,
    /// The time at which a currently `Live` source will degrade to `Unknown` if no new attempt occurs.
    pub degrade_deadline_epoch_s: Option<i64>,
}

/// Evaluates the current health of a source based on its configured cadence and recorded evidence.
///
/// Rules applied (spec sections refer to SPEC-admin-console.md):
/// 1. Never observed -> Unknown.
/// 2. Precedence is FAILING > UNKNOWN > LIVE (§2.5). A failing attempt overrides staleness.
/// 3. Interval sources: Cadence measures ATTEMPTS. `effective = min(declared, learned)`.
///    Age > effective is Unknown. Boundary `age == effective` is Live.
/// 4. `OnDemand` sources: Never time-degraded (§2.5).
/// 5. Inflight attempts past their expected duration override Live to Unknown (§9 criterion 3).
/// 6. Every reason names the value and the threshold that produced it.
#[must_use]
pub fn evaluate(now_epoch_s: i64, declared: Cadence, evidence: &Evidence) -> Assertion {
    // 1. Never observed
    if evidence.last_attempt_epoch_s.is_none()
        && evidence.last_success_epoch_s.is_none()
        && evidence.inflight.is_none()
    {
        return Assertion {
            state: HealthState::Unknown,
            reason: "never observed".to_string(),
            degrade_deadline_epoch_s: None,
        };
    }

    let (mut state, mut reason, mut deadline) = match declared {
        Cadence::EverySeconds(decl) => {
            let effective = match evidence.learned_interval_s {
                Some(learned) if learned < decl => learned,
                _ => decl,
            };

            if let Some(last_attempt) = evidence.last_attempt_epoch_s {
                let age = now_epoch_s.saturating_sub(last_attempt);
                let stale = age > i64::from(effective);
                let staleness_str = if stale {
                    format!("no attempt for {age}s, expect <= {effective}s")
                } else {
                    format!("{age}s ago, expect <= {effective}s")
                };

                if let Some(ref fail_msg) = evidence.last_attempt_failure {
                    let st = HealthState::Failing;
                    let r = if stale {
                        format!("{fail_msg} (and stale: {staleness_str})")
                    } else {
                        format!("{fail_msg} ({staleness_str})")
                    };
                    (st, r, None)
                } else if stale {
                    (HealthState::Unknown, staleness_str, None)
                } else {
                    (
                        HealthState::Live,
                        staleness_str,
                        Some(last_attempt.saturating_add(i64::from(effective))),
                    )
                }
            } else {
                (
                    HealthState::Unknown,
                    "never completed an attempt".to_string(),
                    None,
                )
            }
        }
        Cadence::OnDemand => {
            if let Some(last_attempt) = evidence.last_attempt_epoch_s {
                let age = now_epoch_s.saturating_sub(last_attempt);
                if let Some(ref fail_msg) = evidence.last_attempt_failure {
                    (
                        HealthState::Failing,
                        format!("failed {age}s ago: {fail_msg}"),
                        None,
                    )
                } else {
                    (HealthState::Live, format!("succeeded {age}s ago"), None)
                }
            } else {
                (
                    HealthState::Unknown,
                    "never completed an attempt".to_string(),
                    None,
                )
            }
        }
    };

    if let Some(ref inf) = evidence.inflight {
        let inflight_age = now_epoch_s.saturating_sub(inf.started_epoch_s);
        let expected_dur = inf.expected_duration_s;

        if inflight_age > i64::from(expected_dur) {
            if state != HealthState::Failing {
                state = HealthState::Unknown;
                reason = format!("attempt running {inflight_age}s, expected <= {expected_dur}s");
                deadline = None;
            }
        } else if state == HealthState::Live {
            let inf_deadline = inf.started_epoch_s.saturating_add(i64::from(expected_dur));
            deadline = Some(deadline.map_or(inf_deadline, |d| d.min(inf_deadline)));
        }
    }

    Assertion {
        state,
        reason,
        degrade_deadline_epoch_s: deadline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 10000;

    #[test]
    fn test_never_observed() {
        let evidence = Evidence {
            last_attempt_epoch_s: None,
            last_attempt_failure: None,
            last_success_epoch_s: None,
            inflight: None,
            learned_interval_s: None,
        };
        let assertion = evaluate(NOW, Cadence::EverySeconds(60), &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "never observed");
    }

    #[test]
    fn test_interval_boundary() {
        let declared = 60;
        let mut evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 60), // age == 60
            last_attempt_failure: None,
            last_success_epoch_s: Some(NOW - 60),
            inflight: None,
            learned_interval_s: None,
        };

        let assertion = evaluate(NOW, Cadence::EverySeconds(declared), &evidence);
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "60s ago, expect <= 60s");
        assert_eq!(assertion.degrade_deadline_epoch_s, Some(NOW));

        evidence.last_attempt_epoch_s = Some(NOW - 61); // age == 61
        let assertion2 = evaluate(NOW, Cadence::EverySeconds(declared), &evidence);
        assert_eq!(assertion2.state, HealthState::Unknown);
        assert_eq!(assertion2.reason, "no attempt for 61s, expect <= 60s");
        assert_eq!(assertion2.degrade_deadline_epoch_s, None);
    }

    #[test]
    fn test_learned_interval_tightens() {
        // declared 300, learned 60, age 120 -> Unknown
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 120),
            last_attempt_failure: None,
            last_success_epoch_s: Some(NOW - 120),
            inflight: None,
            learned_interval_s: Some(60),
        };
        let assertion = evaluate(NOW, Cadence::EverySeconds(300), &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "no attempt for 120s, expect <= 60s");
    }

    #[test]
    fn test_learned_interval_never_loosens() {
        // declared 60, learned 300, age 120 -> Unknown
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 120),
            last_attempt_failure: None,
            last_success_epoch_s: Some(NOW - 120),
            inflight: None,
            learned_interval_s: Some(300),
        };
        let assertion = evaluate(NOW, Cadence::EverySeconds(60), &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "no attempt for 120s, expect <= 60s");
    }

    #[test]
    fn test_failing_beats_stale() {
        // failed 120s ago, expect <= 60s
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 120),
            last_attempt_failure: Some("connection refused".to_string()),
            last_success_epoch_s: Some(NOW - 1000),
            inflight: None,
            learned_interval_s: None,
        };
        let assertion = evaluate(NOW, Cadence::EverySeconds(60), &evidence);
        assert_eq!(assertion.state, HealthState::Failing);
        assert_eq!(
            assertion.reason,
            "connection refused (and stale: no attempt for 120s, expect <= 60s)"
        );
    }

    #[test]
    fn test_ondemand_old_success_stays_live() {
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 86400 * 30), // 30 days ago
            last_attempt_failure: None,
            last_success_epoch_s: Some(NOW - 86400 * 30),
            inflight: None,
            learned_interval_s: None,
        };
        let assertion = evaluate(NOW, Cadence::OnDemand, &evidence);
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "succeeded 2592000s ago");
        assert_eq!(assertion.degrade_deadline_epoch_s, None);
    }

    #[test]
    fn test_inflight_overdue_beats_live_but_not_failing() {
        // Base state is Live: attempt 10s ago, expect <= 60s.
        // Inflight started 40s ago, expected duration 30s.
        let mut evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 10),
            last_attempt_failure: None,
            last_success_epoch_s: Some(NOW - 10),
            inflight: Some(Inflight {
                started_epoch_s: NOW - 40,
                expected_duration_s: 30,
            }),
            learned_interval_s: None,
        };

        let assertion = evaluate(NOW, Cadence::EverySeconds(60), &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "attempt running 40s, expected <= 30s");

        // Now base state is Failing. Overdue inflight should not override Failing.
        evidence.last_attempt_failure = Some("OOM killed".to_string());
        let assertion2 = evaluate(NOW, Cadence::EverySeconds(60), &evidence);
        assert_eq!(assertion2.state, HealthState::Failing);
        assert_eq!(assertion2.reason, "OOM killed (10s ago, expect <= 60s)");
    }

    #[test]
    fn test_inflight_within_duration_deadline_clamping() {
        // Base state Live. Attempt 10s ago, expect <= 60s. So rotation deadline is NOW + 50.
        // Inflight started 5s ago, expected 20s. So inflight deadline is NOW + 15.
        // Min should be NOW + 15.
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 10),
            last_attempt_failure: None,
            last_success_epoch_s: Some(NOW - 10),
            inflight: Some(Inflight {
                started_epoch_s: NOW - 5,
                expected_duration_s: 20,
            }),
            learned_interval_s: None,
        };

        let assertion = evaluate(NOW, Cadence::EverySeconds(60), &evidence);
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "10s ago, expect <= 60s");
        assert_eq!(assertion.degrade_deadline_epoch_s, Some(NOW + 15));
    }
}
