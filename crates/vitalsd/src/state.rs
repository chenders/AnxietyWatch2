//! Everything the daemon knows, with no I/O and no runtime.
//!
//! Split out deliberately: liveness, backpressure and the heartbeat conditions
//! are where the interesting failures live, and none of them should require a
//! socket to test. A rule that can only be exercised by standing up a daemon is
//! a rule that gets exercised rarely.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use vitals_proto::{Health, HeartbeatInputs, Liveness, SourceHealth, SubscriberHealth, WireSample};

// DEFAULT_STALE_AFTER was here: a hardcoded, source-independent two-minute
// staleness bound. It is deleted rather than retuned, because a single number
// is right for at most one source. Staleness now comes from the source's
// declared `max_sample_arrival_s` in the registry, and a source that declares
// none is reported `Unbounded` rather than being judged against an invented
// threshold. See AGENTS.md: the unit and valid range come from
// vitals/registry.toml, never hardcoded.

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

    /// Set by the monitoring-window controller. Defaults to **false**: the
    /// heartbeat is unconditional outside a window, so defaulting to `true`
    /// would arm the dead-man switch before anything is expected to report and
    /// fire on an idle box.
    pub monitoring_window_active: bool,
    /// Last tier evaluation from `cns-engine`. `None` until one arrives.
    pub last_tier_evaluation: Option<Instant>,
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
            monitoring_window_active: false,
            last_tier_evaluation: None,
            primary_sources,
        }
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

    /// The declared arrival bound for a source, or `None` when it has not been
    /// measured — including for an id the registry does not know.
    ///
    /// An unknown id yields `None` rather than a default, so a sample claiming
    /// a source that does not exist cannot be judged fresh.
    fn arrival_bound(source: i32) -> Option<Duration> {
        vitals_registry::VitalsSource::from_id(source)
            .and_then(vitals_registry::VitalsSource::max_sample_arrival_s)
            .map(|secs| Duration::from_secs(u64::from(secs)))
    }

    fn liveness_of(
        source: i32,
        st: &SourceState,
        now: Instant,
    ) -> (Liveness, Option<u64>, Option<u64>) {
        match st.last_sample {
            None => (Liveness::NeverSeen, None, None),
            Some(t) => {
                let age = now.saturating_duration_since(t);
                let Some(limit) = Self::arrival_bound(source) else {
                    // Measured, but nothing declares how fresh is fresh. Saying
                    // `Live` would reassure on no evidence and `Stale` would
                    // accuse on none.
                    return (Liveness::Unbounded, Some(age.as_secs()), None);
                };
                let l = if age > limit {
                    Liveness::Stale
                } else {
                    Liveness::Live
                };
                (l, Some(age.as_secs()), Some(limit.as_secs()))
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
            // The daemon overlays actual spool lag before publishing health.
            drain_progressing: true,
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
                let (liveness, age, limit) = Self::liveness_of(source, st, now);
                if liveness == Liveness::Stale {
                    degraded.push(format!(
                        "source {source} is stale: {}s since last sample (limit {}s)",
                        age.unwrap_or(0),
                        limit.unwrap_or(0)
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
            sync_append_ready: true,
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
    /// `resmed_air11` — the one source with a measured arrival bound (300s).
    const AIR11: i32 = 5;

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

    /// Staleness is judged against the source's own declared bound, and the
    /// message names that bound rather than a process-wide constant.
    #[test]
    fn silence_becomes_stale_and_names_the_declared_limit() {
        let mut s = state();
        let t0 = Instant::now();
        s.record_appended(&[sample(AIR11, 1)], 0, t0);

        // AirSense declares max_sample_arrival_s = 300, measured from its real
        // store-arrival lag (p50 35s, max 44s).
        let h = s.health(t0 + Duration::from_secs(301));
        assert_eq!(h.sources.first().map(|s| s.liveness), Some(Liveness::Stale));
        assert!(h.degraded.iter().any(|d| d.contains("source 5 is stale")));
        assert!(
            h.degraded.iter().any(|d| d.contains("limit 300s")),
            "the limit must be the source's declared bound, not a constant"
        );
    }

    #[test]
    fn a_source_within_its_declared_bound_is_live_and_not_degraded() {
        let mut s = state();
        let t0 = Instant::now();
        s.record_appended(&[sample(AIR11, 1)], 0, t0);
        let h = s.health(t0 + Duration::from_secs(299));
        assert_eq!(h.sources.first().map(|s| s.liveness), Some(Liveness::Live));
        assert!(h.degraded.is_empty());
    }

    /// A source with no measured arrival bound is neither fresh nor stale.
    ///
    /// Before this, a hardcoded two-minute constant labelled every such source
    /// `Stale` past 120s — including on the same card where the console, using
    /// the declared 300s, said `Live`. Two numbers for one quantity.
    #[test]
    fn a_source_with_no_declared_bound_is_unbounded_not_stale() {
        let mut s = state();
        let t0 = Instant::now();
        // polar_h10 declares expected_cadence_s but no max_sample_arrival_s.
        s.record_appended(&[sample(HR, 1)], 0, t0);

        for age in [1_u64, 121, 3_600, 921_331] {
            let h = s.health(t0 + Duration::from_secs(age));
            assert_eq!(
                h.sources.first().map(|s| s.liveness),
                Some(Liveness::Unbounded),
                "age {age}s must stay Unbounded without a declared bound"
            );
            assert!(
                h.degraded.is_empty(),
                "an unjudgeable source must not be reported degraded at {age}s"
            );
        }
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
        let mut h = s.health(Instant::now());
        h.heartbeat.drain_progressing = false;
        h.degraded.extend(
            h.heartbeat
                .blockers(HEARTBEAT_MAX_AGE.as_secs())
                .into_iter()
                .map(|b| format!("heartbeat blocked: {b}")),
        );
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
