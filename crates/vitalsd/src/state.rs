//! Everything the daemon knows, with no I/O and no runtime.
//!
//! Split out deliberately: liveness, backpressure and the heartbeat conditions
//! are where the interesting failures live, and none of them should require a
//! socket to test. A rule that can only be exercised by standing up a daemon is
//! a rule that gets exercised rarely.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use vitals_proto::{Health, HeartbeatInputs, Liveness, SourceHealth, SubscriberHealth, WireSample};

/// How long a source may be silent before it is `Stale`.
///
/// Per-source cadences belong in the registry once sources differ enough to
/// need them; one conservative default is honest until then.
pub const DEFAULT_STALE_AFTER: Duration = Duration::from_mins(2);

/// Freshness limit for the heartbeat's inputs (amendment 05 §2.1).
pub const HEARTBEAT_MAX_AGE: Duration = Duration::from_secs(30);

/// Per-subscriber queue depth before samples are dropped.
///
/// Bounded because an unbounded queue converts a slow subscriber into unbounded
/// memory growth in the one process that must never be OOM-killed.
pub const SUBSCRIBER_QUEUE_DEPTH: usize = 1024;

/// A selector a subscriber registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selector {
    /// Registry source id, or `None` for all.
    pub source: Option<i32>,
    /// Registry type id, or `None` for all.
    pub r#type: Option<i32>,
}

impl Selector {
    /// Whether a sample matches.
    #[must_use]
    pub fn matches(&self, s: &WireSample) -> bool {
        self.source.is_none_or(|x| x == s.source) && self.r#type.is_none_or(|x| x == s.r#type)
    }
}

/// One connected subscriber.
#[derive(Debug)]
pub struct Subscriber {
    /// Connection id, used to name it in health output.
    pub id: u64,
    /// What it asked for.
    pub selector: Selector,
    /// Samples dropped because the queue was full.
    pub dropped: u64,
    /// Current depth, as reported by the sending side.
    pub queued: usize,
}

/// Per-source counters.
#[derive(Debug, Default)]
struct SourceState {
    last_sample: Option<Instant>,
    samples: u64,
}

/// The daemon's state.
#[derive(Debug)]
pub struct State {
    started: Instant,
    sources: BTreeMap<i32, SourceState>,
    total_samples: u64,
    spool_bytes: u64,
    subscribers: Vec<Subscriber>,
    stale_after: Duration,

    /// Set by the monitoring-window controller. Defaults to **false**: the
    /// heartbeat is unconditional outside a window, so defaulting to `true`
    /// would arm the dead-man switch before anything is expected to report and
    /// fire on an idle box.
    pub monitoring_window_active: bool,
    /// Last tier evaluation from `cns-engine`. `None` until one arrives.
    pub last_tier_evaluation: Option<Instant>,
    /// Whether the drain offset advanced, or the spool is empty.
    ///
    /// Defaults to **true** so a system with nothing yet spooled is not
    /// reported as stuck; the drainer sets it false when it observes a
    /// non-empty spool that is not advancing.
    pub drain_progressing: bool,
    /// Sources whose samples count as "primary" for the heartbeat: `SpO2` and
    /// respiratory rate. HR and HRV corroborate but must never launder a
    /// missing primary into "all clear" (amendment 05 §2.1).
    pub primary_sources: Vec<i32>,
}

impl State {
    /// A fresh state at `now`.
    ///
    /// `spool_bytes` starts at whatever the segment already holds — a new
    /// segment carries a header, so reporting zero until the first append would
    /// describe a file that does not exist.
    #[must_use]
    pub fn new_with_spool_bytes(primary_sources: Vec<i32>, spool_bytes: u64) -> Self {
        let mut s = Self::new(primary_sources);
        s.spool_bytes = spool_bytes;
        s
    }

    /// A fresh state at `now`.
    #[must_use]
    pub fn new(primary_sources: Vec<i32>) -> Self {
        Self {
            started: Instant::now(),
            sources: BTreeMap::new(),
            total_samples: 0,
            spool_bytes: 0,
            subscribers: Vec::new(),
            stale_after: DEFAULT_STALE_AFTER,
            monitoring_window_active: false,
            last_tier_evaluation: None,
            drain_progressing: true,
            primary_sources,
        }
    }

    /// Overrides the staleness threshold.
    ///
    /// Used by tests today; the real cadence belongs per-source in the registry
    /// once sources differ enough to need one.
    #[cfg(test)]
    pub(crate) const fn set_stale_after(&mut self, d: Duration) {
        self.stale_after = d;
    }

    /// Records that samples were appended.
    pub fn record_appended(&mut self, samples: &[WireSample], spool_bytes: u64, now: Instant) {
        for s in samples {
            let e = self.sources.entry(s.source).or_default();
            e.last_sample = Some(now);
            e.samples += 1;
        }
        self.total_samples += samples.len() as u64;
        self.spool_bytes = spool_bytes;
    }

    /// Registers a subscriber.
    pub fn add_subscriber(&mut self, id: u64, selector: Selector) {
        self.subscribers.push(Subscriber {
            id,
            selector,
            dropped: 0,
            queued: 0,
        });
    }

    /// Removes a subscriber that disconnected.
    pub fn remove_subscriber(&mut self, id: u64) {
        self.subscribers.retain(|s| s.id != id);
    }

    /// Records a drop against a subscriber.
    ///
    /// Dropping is correct — never block the collector — but the count is kept
    /// and reported, because a detector that missed samples must know it missed
    /// them. The count feeds the same "can't assess" path as a sensor gap.
    pub fn record_drop(&mut self, id: u64, dropped: u64, queued: usize) {
        if let Some(s) = self.subscribers.iter_mut().find(|s| s.id == id) {
            s.dropped += dropped;
            s.queued = queued;
        }
    }

    /// Subscribers whose selector matches, for fan-out.
    pub fn matching_subscribers<'a>(&'a self, s: &'a WireSample) -> impl Iterator<Item = u64> + 'a {
        self.subscribers
            .iter()
            .filter(move |sub| sub.selector.matches(s))
            .map(|sub| sub.id)
    }

    fn liveness_of(&self, st: &SourceState, now: Instant) -> (Liveness, Option<u64>) {
        match st.last_sample {
            None => (Liveness::NeverSeen, None),
            Some(t) => {
                let age = now.saturating_duration_since(t);
                let l = if age > self.stale_after {
                    Liveness::Stale
                } else {
                    Liveness::Live
                };
                (l, Some(age.as_secs()))
            }
        }
    }

    /// The current heartbeat inputs.
    #[must_use]
    pub fn heartbeat_inputs(&self, now: Instant) -> HeartbeatInputs {
        let primary_age = self
            .primary_sources
            .iter()
            .filter_map(|id| self.sources.get(id))
            .filter_map(|st| st.last_sample)
            .map(|t| now.saturating_duration_since(t).as_secs())
            .min();
        HeartbeatInputs {
            monitoring_window_active: self.monitoring_window_active,
            primary_sample_age_secs: primary_age,
            tier_evaluation_age_secs: self
                .last_tier_evaluation
                .map(|t| now.saturating_duration_since(t).as_secs()),
            drain_progressing: self.drain_progressing,
        }
    }

    /// The full health report.
    #[must_use]
    pub fn health(&self, now: Instant) -> Health {
        let mut degraded = Vec::new();

        let sources: Vec<SourceHealth> = self
            .sources
            .iter()
            .map(|(&source, st)| {
                let (liveness, age) = self.liveness_of(st, now);
                if liveness == Liveness::Stale {
                    degraded.push(format!(
                        "source {source} is stale: {}s since last sample (limit {}s)",
                        age.unwrap_or(0),
                        self.stale_after.as_secs()
                    ));
                }
                SourceHealth {
                    source,
                    liveness,
                    last_sample_age_secs: age,
                    samples: st.samples,
                }
            })
            .collect();

        let subscribers: Vec<SubscriberHealth> = self
            .subscribers
            .iter()
            .map(|s| {
                if s.dropped > 0 {
                    degraded.push(format!(
                        "subscriber {} has dropped {} sample(s) — it missed data and must \
                         treat that window as unassessable",
                        s.id, s.dropped
                    ));
                }
                SubscriberHealth {
                    id: s.id,
                    dropped: s.dropped,
                    queued: s.queued,
                }
            })
            .collect();

        let heartbeat = self.heartbeat_inputs(now);
        let max_age = HEARTBEAT_MAX_AGE.as_secs();
        for b in heartbeat.blockers(max_age) {
            degraded.push(format!("heartbeat blocked: {b}"));
        }

        Health {
            uptime_secs: now.saturating_duration_since(self.started).as_secs(),
            sources,
            total_samples: self.total_samples,
            spool_bytes: self.spool_bytes,
            // T-D5 derives these from locked-segment offsets in the daemon
            // loop; unknown must not be presented as healthy progress.
            lag_bytes: 0,
            lag_age_seconds: 0,
            last_advanced_at: None,
            subscribers,
            heartbeat_pulsing: heartbeat.should_pulse(max_age),
            heartbeat,
            degraded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPO2: i32 = 2;
    const HR: i32 = 1;

    fn sample(source: i32, r#type: i32) -> WireSample {
        WireSample {
            source,
            r#type,
            session_id: 1,
            seq: 0,
            timestamp_unix_micros: 0,
            value: 98.0,
            extra: None,
            batch_id: None,
        }
    }

    fn state() -> State {
        State::new(vec![SPO2])
    }

    #[test]
    fn a_source_that_never_reported_is_not_healthy() {
        let mut s = state();
        let now = Instant::now();
        s.record_appended(&[sample(HR, 1)], 0, now);
        let h = s.health(now);
        // SpO2 has never been seen, so it does not appear at all rather than
        // appearing as fine — and the heartbeat notices.
        assert!(h.sources.iter().all(|x| x.source != SPO2));
        assert_eq!(h.heartbeat.primary_sample_age_secs, None);
    }

    #[test]
    fn silence_becomes_stale_and_names_itself() {
        let mut s = state();
        s.set_stale_after(Duration::from_secs(10));
        let t0 = Instant::now();
        s.record_appended(&[sample(HR, 1)], 0, t0);

        let later = t0 + Duration::from_secs(11);
        let h = s.health(later);
        assert_eq!(h.sources.first().map(|s| s.liveness), Some(Liveness::Stale));
        assert!(h.degraded.iter().any(|d| d.contains("source 1 is stale")));
        assert!(
            h.degraded.iter().any(|d| d.contains("limit 10s")),
            "must name the limit too"
        );
    }

    #[test]
    fn a_live_source_is_not_degraded() {
        let mut s = state();
        let t0 = Instant::now();
        s.record_appended(&[sample(HR, 1)], 0, t0);
        let h = s.health(t0 + Duration::from_secs(1));
        assert_eq!(h.sources.first().map(|s| s.liveness), Some(Liveness::Live));
        assert!(h.degraded.is_empty());
    }

    /// HR and HRV corroborate but must never launder a missing primary into
    /// "all clear" (amendment 05 §2.1).
    #[test]
    fn a_corroborating_source_does_not_satisfy_the_primary_requirement() {
        let mut s = state();
        s.monitoring_window_active = true;
        let t0 = Instant::now();
        s.last_tier_evaluation = Some(t0);
        s.record_appended(&[sample(HR, 1)], 0, t0);

        let h = s.health(t0);
        assert!(
            !h.heartbeat_pulsing,
            "HR alone must not satisfy the heartbeat"
        );
        assert!(h.degraded.iter().any(|d| d.contains("no primary sample")));

        s.record_appended(&[sample(SPO2, 2)], 0, t0);
        let h = s.health(t0);
        assert!(h.heartbeat_pulsing, "with SpO2 present the chain is intact");
    }

    #[test]
    fn drops_are_counted_and_the_subscriber_is_named() {
        let mut s = state();
        s.add_subscriber(
            42,
            Selector {
                source: None,
                r#type: None,
            },
        );
        s.record_drop(42, 7, 1024);
        let h = s.health(Instant::now());
        assert_eq!(h.subscribers.first().map(|s| s.dropped), Some(7));
        assert!(
            h.degraded
                .iter()
                .any(|d| d.contains("subscriber 42 has dropped 7"))
        );
    }

    #[test]
    fn selectors_filter_fan_out() {
        let mut s = state();
        s.add_subscriber(
            1,
            Selector {
                source: None,
                r#type: None,
            },
        );
        s.add_subscriber(
            2,
            Selector {
                source: Some(SPO2),
                r#type: None,
            },
        );
        s.add_subscriber(
            3,
            Selector {
                source: Some(HR),
                r#type: Some(99),
            },
        );

        let ids: Vec<u64> = s.matching_subscribers(&sample(SPO2, 2)).collect();
        assert_eq!(ids, vec![1, 2]);
        let ids: Vec<u64> = s.matching_subscribers(&sample(HR, 1)).collect();
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn a_disconnected_subscriber_stops_being_reported() {
        let mut s = state();
        s.add_subscriber(
            1,
            Selector {
                source: None,
                r#type: None,
            },
        );
        s.remove_subscriber(1);
        assert!(s.health(Instant::now()).subscribers.is_empty());
    }

    /// Defaulting the window to active would arm the dead-man switch before
    /// anything is expected to report, and fire on an idle box — the
    /// false-alarm-on-boot shape this project has designed in three times.
    #[test]
    fn monitoring_window_defaults_to_inactive() {
        let s = state();
        let h = s.health(Instant::now());
        assert!(!h.heartbeat.monitoring_window_active);
        assert!(h.heartbeat_pulsing, "unconditional outside a window");
        assert!(h.degraded.is_empty());
    }

    #[test]
    fn every_blocker_appears_in_degraded() {
        let mut s = state();
        s.monitoring_window_active = true;
        s.drain_progressing = false;
        let h = s.health(Instant::now());
        assert!(h.degraded.iter().any(|d| d.contains("no primary sample")));
        assert!(
            h.degraded
                .iter()
                .any(|d| d.contains("never produced a tier evaluation"))
        );
        assert!(
            h.degraded
                .iter()
                .any(|d| d.contains("drain offset is not advancing"))
        );
    }
}
