//! Source-health policy evaluator.
//!
//! Implements the liveness logic defined in the spec.
//! This module is pure and contains no I/O, clocks, or side-effects.

use core::fmt::Write as _;

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

/// The bounds a source is judged against.
///
/// Attempt cadence and sample arrival are **different physical quantities** and
/// are declared separately. Conflating them caused a ten-day outage to render
/// identically to healthy operation: see the module comment on
/// `sample_arrival` below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// How often the source is expected to *attempt* work.
    pub attempt_cadence: Cadence,
    /// Beyond this age, the newest sample no longer establishes liveness.
    ///
    /// `None` means no honest bound has been measured for this source, and
    /// sample arrival then contributes **nothing** — it does not fall back to
    /// `attempt_cadence`. That fallback was tried and rejected during review:
    /// `expected_cadence_s` is the sample *rate* for the 1 Hz sources, so
    /// falling back to it reproduced exactly the comparison this module exists
    /// to delete, while reading as if it had been fixed.
    pub max_sample_arrival_s: Option<u32>,
    /// Beyond this age, sample silence is positive evidence of failure.
    ///
    /// This is the bound whose absence let 921,331 seconds of silence score as
    /// `Unknown`. It must be far above `max_sample_arrival_s` so a transient
    /// gap degrades to `Unknown` first and only sustained silence escalates.
    ///
    /// Invalid on an [`Cadence::OnDemand`] source — §2.5 says on-demand is
    /// never time-degraded *to failing*, and `gen_registry.py` rejects the
    /// declaration by name rather than relying on this type to be read.
    pub sample_silence_failure_s: Option<u32>,
    /// Beyond this age, an on-demand source's last event no longer certifies
    /// it, and the state becomes `Unknown` — never `Failing`.
    ///
    /// A **separate quantity** from `max_sample_arrival_s`, which bounds the
    /// arrival of a *stream*. This one bounds the age of a discrete event: a
    /// cuff reading, an SD-card import. The two were briefly the same field,
    /// which meant a sample-arrival bound was compared against an attempt age
    /// — the conflation this module exists to remove, at a fresh site.
    ///
    /// `None` means no time degradation at all, which is §2.5's original rule
    /// and is what every on-demand source does today. The consequence is that
    /// an on-demand source reports `Live` on arbitrarily old evidence; closing
    /// that needs a declared number per source, which is an owner decision and
    /// is recorded as open in `SPEC-admin-console.md` §2.5 rather than invented
    /// here.
    pub max_event_age_s: Option<u32>,
    /// Beyond this age with no transport-level contact at all, the source is
    /// `Failing` — **regardless of whether it should be producing data**.
    ///
    /// The only bound that can escalate while a source is observed idle, which
    /// makes it the fallback for a device that genuinely stops producing. The
    /// `AirSense` does not need it — measurement showed it streams
    /// continuously — so `sample_silence_failure_s` carries detection there and
    /// this stays undeclared until a device needs it.
    ///
    /// Measured against [`Evidence::last_link_epoch_s`], which is a different
    /// quantity from every sample timestamp in this struct — and the whole
    /// reason this field exists rather than reusing one of them.
    pub max_link_silence_s: Option<u32>,
}

impl Bounds {
    /// Attempt cadence only — sample arrival contributes nothing.
    #[must_use]
    pub const fn from_cadence(attempt_cadence: Cadence) -> Self {
        Self {
            attempt_cadence,
            max_sample_arrival_s: None,
            sample_silence_failure_s: None,
            max_event_age_s: None,
            max_link_silence_s: None,
        }
    }
}

/// Whether a source is currently expected to be producing data.
///
/// Exists for devices that are *legitimately* silent some of the time — an
/// SD-card import, a cuff, a therapy device that only streams while in use.
/// Judging those against an arrival bound would turn ordinary non-use into an
/// alarm.
///
/// Only an **observed** [`Activity::Idle`] suppresses a silence verdict.
/// [`Activity::Unknown`] does not, and getting that backwards is a defect this
/// type already shipped once: treating "not measured" as equivalent to "idle"
/// let a missing gate veto a declared bound and measured silence, which is the
/// escalation that detects a dead collector. The three-state rule says a
/// missing signal never escalates *on its own* — it does not say a missing
/// signal may silence evidence that does exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Activity {
    /// Samples should be arriving now, so their absence means something.
    Expected,
    /// **Observed** to be legitimately idle. Silence is not a fault and arrival
    /// bounds do not apply; a *fresh* sample still proves production.
    Idle,
    /// Not measured. Falls back to the registry declaration: declaring
    /// `sample_silence_failure_s` *is* the statement that this source produces
    /// continuously, so silence past it is a fault. A device that is genuinely
    /// intermittent declares no silence bound, or reports its activity.
    #[default]
    Unknown,
}

impl Activity {
    /// Whether silence may be read as evidence of failure.
    const fn silence_is_meaningful(self) -> bool {
        !matches!(self, Self::Idle)
    }
}

/// The recorded evidence about a source's attempts and successes.
///
/// [`Default`] is every channel absent and [`Activity::Unknown`] — "nothing was
/// measured", which is the safe reading of a field nobody filled in. Tests
/// construct the one or two channels under test and default the rest; adding a
/// channel then cannot silently change what an existing test asserts, because
/// absent contributes nothing by construction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
    /// Timestamp of the newest stored sample in epoch seconds.
    ///
    /// Its own field, because it is not an attempt. The silence detector used
    /// to substitute this value into `last_attempt_epoch_s` whenever it was
    /// newer, which compared a sample timestamp against an attempt cadence —
    /// and for a source declaring one sample per second, the ordinary lag
    /// between collection and commit then read as stale. Five of eight had
    /// consequently never once reported `Live`, so a real ten-day outage
    /// produced no state transition to notice.
    pub last_sample_epoch_s: Option<i64>,
    /// Timestamp of the last transport-level contact with the device, in epoch
    /// seconds — for the `AirSense`, the host receipt time of the last BLE
    /// envelope.
    ///
    /// Not a sample timestamp. An envelope arrives whether or not it carries
    /// anything storable, so this channel keeps running even when a device
    /// stores nothing. That makes it the only evidence that separates "idle and
    /// healthy" from "collector dead" for a device that stops producing
    /// legitimately. The `AirSense` is not one: it streams continuously.
    pub last_link_epoch_s: Option<i64>,
    /// Whether samples should be arriving right now.
    ///
    /// For the `AirSense` this is the collector's therapy lifecycle state.
    /// Defaults to [`Activity::Unknown`], which never licenses escalation.
    pub activity: Activity,
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

/// How far a stored timestamp may sit ahead of `now` before it is unusable.
///
/// This is a tolerance for *granularity*, not for bad clocks. The silence
/// detector takes one `now` per sweep and then makes several round-trips per
/// source across nine sources, so a 1 Hz source routinely stores a row
/// timestamped after that snapshot; truncating to whole seconds turns that into
/// an age of −1. One second absorbs it. Anything beyond is a clock fault, and
/// a clock fault gets named rather than absorbed.
const CLOCK_GRANULARITY_S: i64 = 1;

/// The age of a stored timestamp, or `Err(skew)` when it is too far ahead of
/// `now` to be usable.
///
/// A future-dated timestamp must never be clamped into a plausible age. A
/// negative age read as "fresh" pinned the `AirSense` `Live` for a full day
/// with its collector dead, and disabled the only escalation path while it did
/// so — a clamped reading is indistinguishable from a real one downstream.
fn age_of(now_epoch_s: i64, then_epoch_s: i64) -> Result<i64, i64> {
    let age = now_epoch_s.saturating_sub(then_epoch_s);
    if age < -CLOCK_GRANULARITY_S {
        Err(age.saturating_neg())
    } else {
        // Within granularity only: "this second" is age 0, not age −1.
        Ok(age.max(0))
    }
}

/// What sample arrival can say about an interval source.
///
/// Total by construction, so no case can be reached without having been named.
/// This replaced three mutable out-parameters whose correctness depended on an
/// unwritten coupling between them — review showed the natural next change to
/// that code would have produced a `Live` state carrying a stale-sample reason
/// with no test failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SampleVerdict {
    /// No sample stored, or no arrival bound measured: the channel is silent
    /// and contributes nothing. **Not** a fallback to the attempt cadence.
    Silent,
    /// Within the arrival bound — direct proof the source produced data.
    Fresh {
        /// Age of the newest sample, in seconds.
        age_s: i64,
        /// The declared bound it came in under.
        limit_s: u32,
        /// Timestamp of that sample, for the degrade deadline.
        sample_epoch_s: i64,
    },
    /// Past the arrival bound but short of any declared failure bound.
    Stale {
        /// Age of the newest sample, in seconds.
        age_s: i64,
        /// The declared bound it exceeded.
        limit_s: u32,
    },
    /// Past the arrival bound while the source was not expected to produce.
    ///
    /// Distinct from [`SampleVerdict::Stale`] because the operator reading the
    /// card needs to know *which*. Rendering legitimate non-use as staleness is
    /// how a card nobody believes gets built. Reached only on an **observed**
    /// idle state, never on an unmeasured one.
    Idle {
        /// Age of the newest sample, in seconds.
        age_s: i64,
    },
    /// Past the declared failure bound: *evidence of absence*, not absence of
    /// evidence. This is the state whose absence let 921,331 seconds of
    /// silence score the same as a brief gap.
    GrossSilence {
        /// How long the source has been silent, in seconds.
        age_s: i64,
        /// The declared failure bound it exceeded.
        limit_s: u32,
    },
    /// Timestamped further into the future than clock granularity explains.
    FutureDated {
        /// How far ahead of `now` the sample claims to be, in seconds.
        skew_s: i64,
    },
}

/// Classifies the newest stored sample against this source's declared bounds.
///
/// Order matters and is load-bearing:
/// 1. A **fresh** sample is proof of production whatever the activity gate
///    says, so freshness is tested before the gate.
/// 2. Past the bound, the gate decides whether silence means anything. An idle
///    source is not a stale one.
/// 3. Only then can silence escalate, and only for interval sources.
fn sample_verdict(now_epoch_s: i64, bounds: Bounds, evidence: &Evidence) -> SampleVerdict {
    let Some(sample) = evidence.last_sample_epoch_s else {
        return SampleVerdict::Silent;
    };
    let age = match age_of(now_epoch_s, sample) {
        Ok(age) => age,
        Err(skew_s) => return SampleVerdict::FutureDated { skew_s },
    };

    // No measured arrival bound means the channel stays silent. It does *not*
    // fall back to `attempt_cadence`: for the 1 Hz sources that field is the
    // sample rate, so the fallback reproduced the original defect while
    // reading as if it had been fixed.
    let Some(arrival_limit) = bounds.max_sample_arrival_s else {
        return SampleVerdict::Silent;
    };
    if age <= i64::from(arrival_limit) {
        return SampleVerdict::Fresh {
            age_s: age,
            limit_s: arrival_limit,
            sample_epoch_s: sample,
        };
    }

    // Past the bound. Silence stops being evidence only when the source is
    // *observed* idle. An unmeasured gate falls through to the declaration:
    // declaring a silence bound is the assertion that silence means something,
    // and letting `Unknown` suppress it disabled the escalation entirely,
    // because nothing reports activity today.
    if !evidence.activity.silence_is_meaningful() {
        return SampleVerdict::Idle { age_s: age };
    }

    // Escalation is confined to interval sources: §2.5 says an on-demand
    // source is never time-degraded *to failing*. `gen_registry.py` also
    // rejects the declaration on an on-demand source by name, so this is belt
    // and braces — the trap was baited and unguarded in the first version of
    // this change.
    if let (Cadence::EverySeconds(_), Some(failure_limit)) =
        (bounds.attempt_cadence, bounds.sample_silence_failure_s)
        && age > i64::from(failure_limit)
    {
        return SampleVerdict::GrossSilence {
            age_s: age,
            limit_s: failure_limit,
        };
    }

    SampleVerdict::Stale {
        age_s: age,
        limit_s: arrival_limit,
    }
}

/// What transport-level contact can say about a source.
///
/// Separate from [`SampleVerdict`] because it measures a separate thing, and
/// because it is the only channel that may escalate while a source is idle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkVerdict {
    /// No contact recorded, or no bound declared: contributes nothing.
    Silent,
    /// Contact within the declared bound.
    Contact,
    /// No contact for longer than the bound allows. The device is unreachable
    /// whether or not it was due to produce anything, so this escalates on an
    /// idle source where no sample-derived rule can.
    Lost {
        /// How long since the last contact, in seconds.
        age_s: i64,
        /// The declared bound it exceeded.
        limit_s: u32,
    },
    /// Contact timestamped further into the future than granularity explains.
    FutureDated {
        /// How far ahead of `now` the contact claims to be, in seconds.
        skew_s: i64,
    },
}

/// Classifies transport contact against this source's declared link bound.
fn link_verdict(now_epoch_s: i64, bounds: Bounds, evidence: &Evidence) -> LinkVerdict {
    let (Some(last), Some(limit)) = (evidence.last_link_epoch_s, bounds.max_link_silence_s) else {
        return LinkVerdict::Silent;
    };
    match age_of(now_epoch_s, last) {
        Err(skew_s) => LinkVerdict::FutureDated { skew_s },
        Ok(age) if age > i64::from(limit) => LinkVerdict::Lost {
            age_s: age,
            limit_s: limit,
        },
        Ok(_) => LinkVerdict::Contact,
    }
}

/// The state of an on-demand source, derived from its newest *event*.
///
/// An event is an attempt **or** a stored sample, whichever is newer: a cuff
/// reading that reached `samples` is an event even though nothing writes
/// attempt rows. Treating only attempts as events made a blood-pressure
/// reading taken one second ago render as "never completed an attempt" for
/// four of nine sources.
///
/// Never time-degrades to `Failing` (§2.5). Past `max_event_age_s` — a bound on
/// *event age*, not on stream arrival — it reports `Unknown` rather than
/// `Live`, because §2.5 requires only that an old timestamp not read as a
/// *fault*, which is not a licence to read as healthy. No source declares that
/// bound today, so this is mechanism without an invented number.
fn on_demand_state(
    now_epoch_s: i64,
    bounds: Bounds,
    evidence: &Evidence,
) -> (HealthState, String, Option<i64>) {
    // A recorded failure stands until a later attempt replaces it. A newer
    // sample does not retract it: the drainer can flush a spool while the
    // collector is broken, and §2.5 puts FAILING above LIVE.
    if let (Some(last_attempt), Some(fail_msg)) = (
        evidence.last_attempt_epoch_s,
        evidence.last_attempt_failure.as_ref(),
    ) {
        let age = now_epoch_s.saturating_sub(last_attempt);
        return (
            HealthState::Failing,
            format!("failed {age}s ago: {fail_msg}"),
            None,
        );
    }

    let newest = match (evidence.last_attempt_epoch_s, evidence.last_sample_epoch_s) {
        (Some(attempt), Some(sample)) => Some((attempt.max(sample), "event")),
        (Some(attempt), None) => Some((attempt, "succeeded")),
        (None, Some(sample)) => Some((sample, "sample")),
        (None, None) => None,
    };
    let Some((event_epoch_s, noun)) = newest else {
        return (
            HealthState::Unknown,
            "never observed an event".to_string(),
            None,
        );
    };

    let age = match age_of(now_epoch_s, event_epoch_s) {
        Ok(age) => age,
        Err(skew_s) => {
            return (
                HealthState::Unknown,
                format!("{noun} timestamped {skew_s}s in the future: unusable"),
                None,
            );
        }
    };
    match bounds.max_event_age_s {
        Some(limit) if age > i64::from(limit) => (
            HealthState::Unknown,
            format!("{noun} {age}s ago, beyond {limit}s; on-demand so not a fault"),
            None,
        ),
        _ => (HealthState::Live, format!("{noun} {age}s ago"), None),
    }
}

/// The attempt-derived state for an interval source.
///
/// Extracted so `evaluate` stays readable as a precedence fold rather than a
/// nest of conditions.
fn interval_state(
    now_epoch_s: i64,
    declared_s: u32,
    evidence: &Evidence,
) -> (HealthState, String, Option<i64>) {
    let effective = match evidence.learned_interval_s {
        // Learned intervals may TIGHTEN only.
        Some(learned) if learned < declared_s => learned,
        _ => declared_s,
    };
    let Some(last_attempt) = evidence.last_attempt_epoch_s else {
        return (
            HealthState::Unknown,
            "never completed an attempt".to_string(),
            None,
        );
    };

    let age = now_epoch_s.saturating_sub(last_attempt);
    let stale = age > i64::from(effective);
    let staleness = if stale {
        format!("no attempt for {age}s, expect <= {effective}s")
    } else {
        format!("{age}s ago, expect <= {effective}s")
    };

    if let Some(ref fail_msg) = evidence.last_attempt_failure {
        let reason = if stale {
            format!("{fail_msg} (and stale: {staleness})")
        } else {
            format!("{fail_msg} ({staleness})")
        };
        (HealthState::Failing, reason, None)
    } else if stale {
        (HealthState::Unknown, staleness, None)
    } else {
        (
            HealthState::Live,
            staleness,
            Some(last_attempt.saturating_add(i64::from(effective))),
        )
    }
}

/// Evaluates the current health of a source based on its configured cadence and recorded evidence.
///
/// Rules applied (spec sections refer to SPEC-admin-console.md):
/// 1. Never observed -> Unknown.
/// 2. Precedence is FAILING > UNKNOWN > LIVE (§2.5). A failing attempt overrides staleness.
/// 3. Interval sources: Cadence measures ATTEMPTS. `effective = min(declared, learned)`.
///    Age > effective is Unknown. Boundary `age == effective` is Live.
/// 4. `OnDemand` sources: Never time-degraded *to failing* (§2.5). Their state
///    comes from the newest **event** — attempt or sample, whichever is newer —
///    and degrades to `Unknown` only past a declared `max_event_age_s`.
/// 5. Inflight attempts past their expected duration override Live to Unknown (§9 criterion 3).
/// 6. Every reason names the value and the threshold that produced it, and
///    names *which quantity* it measured.
/// 7. **Sample arrival is independent evidence.** A stored sample is direct
///    proof the source produced data, so a sample within a declared
///    `max_sample_arrival_s` establishes `Live` even when no attempt was ever
///    recorded — which is the state every source is in today, because nothing
///    writes attempts. Conversely, silence beyond `sample_silence_failure_s`
///    is *evidence of absence* rather than absence of evidence, and reports
///    `Failing`. There is **no fallback** when the bound is undeclared: the
///    channel stays silent.
/// 8. **Silence only counts while the source should be producing.** A CPAP
///    stores nothing between therapy sessions, so [`Activity`] gates every
///    silence-derived verdict. An unobserved gate never licenses escalation.
/// 9. **Transport contact is a third channel** and the only one that may
///    escalate on an *observed* idle source, because the device answers on the
///    radio even when it stores nothing. Needed only for a device that stops
///    producing legitimately; the `AirSense` streams continuously, so its
///    sample bound carries detection.
#[must_use]
pub fn evaluate(now_epoch_s: i64, bounds: Bounds, evidence: &Evidence) -> Assertion {
    // 1. Never observed. A sample or a contact counts as observation — without
    //    this a source with data but no attempt row reads "never observed".
    if evidence.last_attempt_epoch_s.is_none()
        && evidence.last_success_epoch_s.is_none()
        && evidence.inflight.is_none()
        && evidence.last_sample_epoch_s.is_none()
        && evidence.last_link_epoch_s.is_none()
    {
        return Assertion {
            state: HealthState::Unknown,
            reason: "never observed".to_string(),
            degrade_deadline_epoch_s: None,
        };
    }

    let sample = sample_verdict(now_epoch_s, bounds, evidence);
    let link = link_verdict(now_epoch_s, bounds, evidence);

    // 9a. Lost transport contact outranks everything. It is the only verdict
    //     that holds while a source is legitimately idle, and it carries any
    //     recorded attempt failure along so the cause is not discarded.
    if let LinkVerdict::Lost { age_s, limit_s } = link {
        let mut reason = format!("no device contact for {age_s}s, failing beyond {limit_s}s");
        if let Some(ref fail_msg) = evidence.last_attempt_failure {
            let _ = write!(reason, " (last attempt failed: {fail_msg})");
        }
        return Assertion {
            state: HealthState::Failing,
            reason,
            degrade_deadline_epoch_s: None,
        };
    }

    // 7a. Gross sample silence is positive evidence of failure, evaluated
    //     before anything else can soften it to Unknown. The attempt-failure
    //     detail rides along for the same reason as above.
    if let SampleVerdict::GrossSilence { age_s, limit_s } = sample {
        let mut reason = format!("no sample for {age_s}s, failing beyond {limit_s}s");
        if let Some(ref fail_msg) = evidence.last_attempt_failure {
            let _ = write!(reason, " (last attempt failed: {fail_msg})");
        }
        return Assertion {
            state: HealthState::Failing,
            reason,
            degrade_deadline_epoch_s: None,
        };
    }

    let (mut state, mut reason, mut deadline) = match bounds.attempt_cadence {
        Cadence::EverySeconds(declared_s) => interval_state(now_epoch_s, declared_s, evidence),
        Cadence::OnDemand => on_demand_state(now_epoch_s, bounds, evidence),
    };

    // 7b/8. Fold the sample channel in by precedence. `Failing` is never
    //       overridden: data can flow while the collection process is broken,
    //       and §2.5 puts FAILING above LIVE.
    if state != HealthState::Failing {
        match sample {
            // Already handled above, and `Silent` says nothing by definition.
            SampleVerdict::GrossSilence { .. } | SampleVerdict::Silent => {}
            SampleVerdict::FutureDated { skew_s } => {
                state = HealthState::Unknown;
                reason = format!("sample timestamped {skew_s}s in the future: unusable");
                deadline = None;
            }
            SampleVerdict::Fresh {
                age_s,
                limit_s,
                sample_epoch_s,
            } => {
                // Promotion only where the attempt channel is silent. With a
                // recorded-but-stale attempt, §2.5's FAILING > UNKNOWN > LIVE
                // order means Unknown wins — a fresh sample must not launder
                // a collector that has stopped attempting.
                if evidence.last_attempt_epoch_s.is_none() {
                    state = HealthState::Live;
                    reason = format!("sample {age_s}s ago, expect <= {limit_s}s");
                    deadline = Some(sample_epoch_s.saturating_add(i64::from(limit_s)));
                }
            }
            SampleVerdict::Stale { age_s, limit_s } => {
                if evidence.last_attempt_epoch_s.is_none() {
                    reason = format!("no sample for {age_s}s, expect <= {limit_s}s");
                }
            }
            SampleVerdict::Idle { age_s } => {
                if evidence.last_attempt_epoch_s.is_none() {
                    reason = format!("no sample for {age_s}s; source idle, not a fault");
                }
            }
        }
    }

    // 9b. A future-dated contact timestamp is named rather than ignored, but
    //     must not mask a failure the attempt channel already established.
    if let (LinkVerdict::FutureDated { skew_s }, false) = (
        link,
        state == HealthState::Failing || state == HealthState::Live,
    ) {
        reason = format!("device contact timestamped {skew_s}s in the future: unusable");
        deadline = None;
    }

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
            ..Default::default()
        };
        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(60)),
            &evidence,
        );
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
            ..Default::default()
        };

        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(declared)),
            &evidence,
        );
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "60s ago, expect <= 60s");
        assert_eq!(assertion.degrade_deadline_epoch_s, Some(NOW));

        evidence.last_attempt_epoch_s = Some(NOW - 61); // age == 61
        let assertion2 = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(declared)),
            &evidence,
        );
        assert_eq!(assertion2.state, HealthState::Unknown);
        assert_eq!(assertion2.reason, "no attempt for 61s, expect <= 60s");
        assert_eq!(assertion2.degrade_deadline_epoch_s, None);
    }

    /// The incident, as a test. 2026-10-29 to 2026-11-08: the `AirSense`
    /// collector died, samples stopped, and the detector reported `Unknown`
    /// for ten days because staleness mapped to Unknown no matter its size.
    /// 921,331s against a 1,800s failure bound is evidence of absence.
    #[test]
    fn gross_sample_silence_is_failing_not_unknown() {
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 921_331),
            // Escalation requires an observed activity gate: without this the
            // verdict is `Unknown`, which is the honest answer for a source
            // that may simply be idle.
            activity: Activity::Expected,
            ..Default::default()
        };
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Failing);
        assert_eq!(
            assertion.reason,
            "no sample for 921331s, failing beyond 1800s"
        );
    }

    /// The other half of the incident: healthy operation also read `Unknown`,
    /// so there was no transition to notice. A 24s-old sample against a
    /// one-per-second cadence is ordinary commit lag, not staleness.
    #[test]
    fn a_fresh_sample_is_live_even_with_no_attempt_ever_recorded() {
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 24),
            ..Default::default()
        };
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "sample 24s ago, expect <= 300s");
        assert_eq!(assertion.degrade_deadline_epoch_s, Some(NOW - 24 + 300));
    }

    /// Between the two bounds is the grace band: stale enough not to reassure,
    /// not yet proof of failure. Without this a transient drain stall would
    /// page immediately.
    #[test]
    fn sample_between_the_bounds_is_unknown() {
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 600),
            ..Default::default()
        };
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        assert_eq!(evaluate(NOW, bounds, &evidence).state, HealthState::Unknown);
    }

    /// Boundaries, because an off-by-one here is a missed alarm or a false one.
    #[test]
    fn the_arrival_bounds_are_inclusive_at_the_edge() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let at = |age: i64| {
            let evidence = Evidence {
                last_sample_epoch_s: Some(NOW - age),
                activity: Activity::Expected,
                ..Default::default()
            };
            evaluate(NOW, bounds, &evidence).state
        };
        assert_eq!(at(300), HealthState::Live, "age == arrival bound is live");
        assert_eq!(at(301), HealthState::Unknown, "one past is unknown");
        assert_eq!(at(1_800), HealthState::Unknown, "age == failure bound");
        assert_eq!(at(1_801), HealthState::Failing, "one past is failing");
    }

    /// A failing attempt still wins over fresh data: the collection process
    /// can be broken while samples flow, and §2.5 puts FAILING above LIVE.
    #[test]
    fn a_failing_attempt_outranks_a_fresh_sample() {
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 10),
            last_attempt_failure: Some("credential rejected".to_string()),
            last_sample_epoch_s: Some(NOW - 1),
            ..Default::default()
        };
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(60))
        };
        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Failing);
        assert!(assertion.reason.contains("credential rejected"));
    }

    /// A source with no declared arrival bound keeps the old attempt-only
    /// behaviour, so sources without measured evidence are not given invented
    /// thresholds.
    #[test]
    fn without_declared_bounds_sample_evidence_does_not_decide() {
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 900_000),
            ..Default::default()
        };
        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(1)),
            &evidence,
        );
        assert_eq!(assertion.state, HealthState::Unknown);
    }

    /// Sources 4 and 6 were reporting `Live` on successes 52.8 days old.
    /// On-demand must not become a fault on age (§2.5) but must not reassure
    /// either — Unknown is the honest third state.
    #[test]
    fn on_demand_stops_reporting_live_on_ancient_evidence() {
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 4_564_409),
            last_success_epoch_s: Some(NOW - 4_564_409),
            ..Default::default()
        };
        let bounds = Bounds {
            max_event_age_s: Some(86_400),
            ..Bounds::from_cadence(Cadence::OnDemand)
        };
        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert!(assertion.reason.contains("not a fault"));
    }

    /// And on-demand with recent evidence is still live, so the change above
    /// does not make every on-demand source permanently unknown.
    #[test]
    fn on_demand_with_recent_evidence_is_still_live() {
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 60),
            last_success_epoch_s: Some(NOW - 60),
            ..Default::default()
        };
        let bounds = Bounds {
            max_event_age_s: Some(86_400),
            ..Bounds::from_cadence(Cadence::OnDemand)
        };
        assert_eq!(evaluate(NOW, bounds, &evidence).state, HealthState::Live);
    }

    /// A source with neither attempts nor samples is still "never observed" —
    /// adding sample evidence must not turn absence into a positive claim.
    #[test]
    fn no_evidence_at_all_is_still_never_observed() {
        let evidence = Evidence {
            ..Default::default()
        };
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "never observed");
    }

    #[test]
    fn test_learned_interval_tightens() {
        // declared 300, learned 60, age 120 -> Unknown
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 120),
            last_success_epoch_s: Some(NOW - 120),
            learned_interval_s: Some(60),
            ..Default::default()
        };
        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(300)),
            &evidence,
        );
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "no attempt for 120s, expect <= 60s");
    }

    #[test]
    fn test_learned_interval_never_loosens() {
        // declared 60, learned 300, age 120 -> Unknown
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 120),
            last_success_epoch_s: Some(NOW - 120),
            learned_interval_s: Some(300),
            ..Default::default()
        };
        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(60)),
            &evidence,
        );
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
            ..Default::default()
        };
        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(60)),
            &evidence,
        );
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
            ..Default::default()
        };
        let assertion = evaluate(NOW, Bounds::from_cadence(Cadence::OnDemand), &evidence);
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
            last_success_epoch_s: Some(NOW - 10),
            inflight: Some(Inflight {
                started_epoch_s: NOW - 40,
                expected_duration_s: 30,
            }),
            ..Default::default()
        };

        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(60)),
            &evidence,
        );
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "attempt running 40s, expected <= 30s");

        // Now base state is Failing. Overdue inflight should not override Failing.
        evidence.last_attempt_failure = Some("OOM killed".to_string());
        let assertion2 = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(60)),
            &evidence,
        );
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
            last_success_epoch_s: Some(NOW - 10),
            inflight: Some(Inflight {
                started_epoch_s: NOW - 5,
                expected_duration_s: 20,
            }),
            ..Default::default()
        };

        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(60)),
            &evidence,
        );
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "10s ago, expect <= 60s");
        assert_eq!(assertion.degrade_deadline_epoch_s, Some(NOW + 15));
    }

    /// Measured-and-stale is not never-measured. With the substitution gone,
    /// a source with samples but no attempt rows used to report "never
    /// completed an attempt" — true of attempts, false of the source, and
    /// naming neither a value nor a threshold. The sample channel has to
    /// speak in both directions, not only when it can raise to `Live`.
    #[test]
    fn a_stale_sample_names_its_age_and_bound() {
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 1000),
            activity: Activity::Expected,
            ..Default::default()
        };
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };

        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "no sample for 1000s, expect <= 300s");
        assert_eq!(assertion.degrade_deadline_epoch_s, None);
    }

    /// With no measured arrival bound the sample channel stays silent — it does
    /// **not** fall back to the attempt cadence. That fallback shipped in the
    /// first version of this change and reproduced the original defect for the
    /// two 1 Hz sources, whose `expected_cadence_s` is a sample rate: review
    /// executed both branches and got the same `Unknown` with only the noun
    /// changed. The honest result for an undeclared source is that the attempt
    /// channel speaks alone.
    #[test]
    fn an_undeclared_arrival_bound_does_not_fall_back_to_the_cadence() {
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 60),
            activity: Activity::Expected,
            ..Default::default()
        };
        let bounds = Bounds::from_cadence(Cadence::EverySeconds(1));

        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "never completed an attempt");
        // And critically: a 60s-old sample against a 1s cadence must not be
        // called stale, because 1s was never a statement about arrival.
        assert!(!assertion.reason.contains("expect <= 1s"));
    }

    /// A source observed to be legitimately idle is not stale, and silence
    /// while idle is not a fault. The `AirSense` turns out not to need this —
    /// it streams 24/7 — but an SD-card import or a cuff does.
    #[test]
    fn an_idle_source_is_not_stale_and_never_escalates() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let at = |activity| {
            let evidence = Evidence {
                last_sample_epoch_s: Some(NOW - 50_000),
                activity,
                ..Default::default()
            };
            evaluate(NOW, bounds, &evidence)
        };

        let idle = at(Activity::Idle);
        assert_eq!(idle.state, HealthState::Unknown);
        assert_eq!(
            idle.reason,
            "no sample for 50000s; source idle, not a fault"
        );

        // Both of these must escalate. `Unknown` falls back to the registry
        // declaration rather than suppressing it — which is what makes the
        // 2026-10-30 outage detectable within half an hour instead of never.
        // This test previously asserted `Unknown` behaves like `Idle`, which
        // disabled the escalation entirely: nothing reports activity today, so
        // every source sat at the `Unknown` default and no declared silence
        // bound could ever fire.
        assert_eq!(at(Activity::Expected).state, HealthState::Failing);
        assert_eq!(at(Activity::Unknown).state, HealthState::Failing);
    }

    /// The outage, replayed end to end against the measured configuration.
    ///
    /// The `AirSense` writes ~84,000 rows/hour on its 25 Hz channels, every
    /// hour of every day — measured over two full pre-outage days, 48 of 48
    /// hours populated, minimum 79,636. So sample silence is unambiguous
    /// evidence rather than a therapy gap, and nothing reports activity, so
    /// this runs at the `Unknown` default: the configuration that must fire.
    #[test]
    fn the_outage_is_detected_within_the_failure_bound() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let after = |silence_s: i64| {
            let evidence = Evidence {
                last_sample_epoch_s: Some(NOW - silence_s),
                ..Default::default()
            };
            evaluate(NOW, bounds, &evidence).state
        };

        assert_eq!(after(35), HealthState::Live, "ordinary commit lag");
        assert_eq!(
            after(301),
            HealthState::Unknown,
            "past arrival, not a fault yet"
        );
        assert_eq!(
            after(1_801),
            HealthState::Failing,
            "half an hour of silence"
        );
        // The real thing: 921,331s, which scored `Unknown` for ten days.
        assert_eq!(after(921_331), HealthState::Failing);
    }

    /// Transport contact is the only channel that may escalate on an observed
    /// idle source: the device answers on the radio even when it stores
    /// nothing, so legitimate sample silence still has a heartbeat underneath
    /// it.
    #[test]
    fn lost_transport_contact_escalates_even_when_idle() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            max_link_silence_s: Some(300),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 50_000),
            last_link_epoch_s: Some(NOW - 900),
            activity: Activity::Idle,
            ..Default::default()
        };

        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Failing);
        assert_eq!(
            assertion.reason,
            "no device contact for 900s, failing beyond 300s"
        );

        // Contact within the bound leaves the idle verdict standing.
        let contactable = Evidence {
            last_link_epoch_s: Some(NOW - 10),
            ..evidence.clone()
        };
        let assertion = evaluate(NOW, bounds, &contactable);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert!(assertion.reason.contains("idle, not a fault"));
    }

    /// A recorded attempt failure rides along with an escalation instead of
    /// being discarded. The state was already right; the cause was not.
    #[test]
    fn an_escalation_carries_the_attempt_failure_detail() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 5_000),
            last_attempt_failure: Some("bluez: le-connection-abort-by-local".to_string()),
            activity: Activity::Expected,
            ..Default::default()
        };

        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Failing);
        assert_eq!(
            assertion.reason,
            "no sample for 5000s, failing beyond 1800s \
             (last attempt failed: bluez: le-connection-abort-by-local)"
        );
    }

    /// A future-dated sample is rejected and named, never clamped into a
    /// plausible age. Clamped, it pinned the source `Live` for the whole clock
    /// excursion and disabled the only escalation path while it did so.
    #[test]
    fn a_future_dated_sample_is_rejected_by_name() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW + 86_400),
            activity: Activity::Expected,
            ..Default::default()
        };

        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(
            assertion.reason,
            "sample timestamped 86400s in the future: unusable"
        );
        assert_eq!(assertion.degrade_deadline_epoch_s, None);
    }

    /// One second of forward skew is clock granularity, not a fault: the
    /// detector takes one `now` per sweep and a 1 Hz source writes rows after
    /// it. That case is routine and must stay `Live`.
    #[test]
    fn one_second_of_forward_skew_is_granularity_not_a_fault() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            ..Bounds::from_cadence(Cadence::EverySeconds(1))
        };
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW + 1),
            activity: Activity::Expected,
            ..Default::default()
        };

        let assertion = evaluate(NOW, bounds, &evidence);
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "sample 0s ago, expect <= 300s");
    }

    /// An on-demand source's newest *event* is an attempt or a sample,
    /// whichever is newer. Counting only attempts made a cuff reading taken
    /// one second ago render as "never completed an attempt" for four of nine
    /// sources — measured-and-fresh read as never-measured.
    #[test]
    fn an_on_demand_sample_is_an_event() {
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 1),
            ..Default::default()
        };
        let assertion = evaluate(NOW, Bounds::from_cadence(Cadence::OnDemand), &evidence);
        assert_eq!(assertion.state, HealthState::Live);
        assert_eq!(assertion.reason, "sample 1s ago");
    }

    /// On-demand never escalates on silence, even where a failure bound was
    /// wrongly declared. §2.5 says on-demand is never time-degraded *to
    /// failing*; `gen_registry.py` rejects the declaration, and this is the
    /// second lock on the same door.
    #[test]
    fn on_demand_never_escalates_on_silence() {
        let bounds = Bounds {
            max_sample_arrival_s: Some(300),
            sample_silence_failure_s: Some(1_800),
            ..Bounds::from_cadence(Cadence::OnDemand)
        };
        let evidence = Evidence {
            last_sample_epoch_s: Some(NOW - 50_000),
            activity: Activity::Expected,
            ..Default::default()
        };
        assert_ne!(evaluate(NOW, bounds, &evidence).state, HealthState::Failing);
    }

    /// When attempts *are* recorded, their reason is the one that survives: it
    /// reports against the declared cadence contract, which is the stronger
    /// statement. The sample reason only fills the gap where the attempt
    /// channel is silent.
    #[test]
    fn an_attempt_reason_outranks_a_stale_sample_reason() {
        let evidence = Evidence {
            last_attempt_epoch_s: Some(NOW - 100),
            last_success_epoch_s: Some(NOW - 100),
            last_sample_epoch_s: Some(NOW - 1000),
            ..Default::default()
        };

        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(60)),
            &evidence,
        );
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "no attempt for 100s, expect <= 60s");
    }

    /// An overrunning attempt still outranks a stale sample: a collector stuck
    /// mid-attempt is a more specific statement about why nothing arrived.
    #[test]
    fn an_overrunning_attempt_outranks_a_stale_sample_reason() {
        let evidence = Evidence {
            inflight: Some(Inflight {
                started_epoch_s: NOW - 90,
                expected_duration_s: 30,
            }),
            last_sample_epoch_s: Some(NOW - 1000),
            ..Default::default()
        };

        let assertion = evaluate(
            NOW,
            Bounds::from_cadence(Cadence::EverySeconds(1)),
            &evidence,
        );
        assert_eq!(assertion.state, HealthState::Unknown);
        assert_eq!(assertion.reason, "attempt running 90s, expected <= 30s");
    }
}
