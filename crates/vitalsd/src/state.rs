//! Everything the daemon knows, with no I/O and no runtime.
//!
//! Split out deliberately: liveness, backpressure and the heartbeat conditions
//! are where the interesting failures live, and none of them should require a
//! socket to test. A rule that can only be exercised by standing up a daemon is
//! a rule that gets exercised rarely.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use vitals_proto::{
    DRAIN_HEALTH_SCHEMA, DrainHealth, Health, HeartbeatInputs, Liveness, SourceHealth,
    SubscriberHealth, WireSample,
};

/// How long a source may be silent before it is `Stale`.
///
/// Per-source cadences belong in the registry once sources differ enough to
/// need them; one conservative default is honest until then.
pub const DEFAULT_STALE_AFTER: Duration = Duration::from_mins(2);

/// Freshness limit for the heartbeat's inputs (amendment 05 §2.1).
pub const HEARTBEAT_MAX_AGE: Duration = Duration::from_secs(30);

/// A drainer snapshot older than this cannot prove that the database path is
/// still advancing. Missing or stale evidence is a safety fault, not an idle
/// success.
pub const DRAIN_HEALTH_MAX_AGE: Duration = Duration::from_secs(5);

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
    received_samples: u64,
    total_samples: u64,
    fsynced_samples: u64,
    published_samples: u64,
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
    /// Whether the validated drain snapshot proves progress, or proves that
    /// no rows are pending. Defaults to **false**: without an independent
    /// drainer snapshot the daemon cannot claim storage health.
    pub drain_progressing: bool,
    /// Last validated drainer snapshot. `available = false` is explicit
    /// unknown evidence and is rendered as degraded.
    pub drain: DrainHealth,
    /// The exact blocker derived from the last snapshot, if any. Keeping this
    /// at the state boundary lets the daemon report stale evidence even when
    /// there are no pending bytes.
    drain_blocker: Option<String>,
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
            received_samples: 0,
            total_samples: 0,
            fsynced_samples: 0,
            published_samples: 0,
            spool_bytes: 0,
            subscribers: Vec::new(),
            stale_after: DEFAULT_STALE_AFTER,
            monitoring_window_active: false,
            last_tier_evaluation: None,
            drain_progressing: false,
            drain: DrainHealth::default(),
            drain_blocker: Some(format!(
                "drain health unavailable: available=false schema={DRAIN_HEALTH_SCHEMA} error=not published"
            )),
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

    /// Records observations accepted at the daemon boundary.
    pub fn record_received(&mut self, count: usize) {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        self.received_samples = self.received_samples.saturating_add(count);
    }

    /// Records that samples were appended to the spool.
    pub fn record_appended(&mut self, samples: &[WireSample], spool_bytes: u64, now: Instant) {
        for s in samples {
            let e = self.sources.entry(s.source).or_default();
            e.last_sample = Some(now);
            e.samples = e.samples.saturating_add(1);
        }
        let count = u64::try_from(samples.len()).unwrap_or(u64::MAX);
        self.total_samples = self.total_samples.saturating_add(count);
        self.spool_bytes = spool_bytes;
    }

    /// Records rows made durable by an explicit or periodic fsync.
    pub fn record_fsynced(&mut self, count: u64) {
        self.fsynced_samples = self.fsynced_samples.saturating_add(count);
    }

    /// Records samples admitted to subscriber queues. This is `published`, not
    /// proof that a panel or browser rendered the value.
    pub fn record_published(&mut self, count: u64) {
        self.published_samples = self.published_samples.saturating_add(count);
    }

    /// Replaces the last drainer snapshot and derives the conservative
    /// heartbeat input from it.
    ///
    /// The timestamp is supplied by the caller so this pure state transition
    /// can be tested with fixed values. A future-dated, stale, unavailable, or
    /// errored snapshot never satisfies the storage requirement.
    pub fn record_drain_health(&mut self, health: DrainHealth, now_unix_secs: u64) {
        let blocker = if !health.available {
            Some(format!(
                "drain health unavailable: available={} schema={} error={}",
                health.available,
                health.schema,
                health.last_error.as_deref().unwrap_or("not published")
            ))
        } else if health.schema != DRAIN_HEALTH_SCHEMA {
            Some(format!(
                "drain health blocked: schema={} expected={DRAIN_HEALTH_SCHEMA}",
                health.schema
            ))
        } else if health.generated_at_unix_secs > now_unix_secs {
            Some(format!(
                "drain health blocked: generated_at={} is newer than now={now_unix_secs}",
                health.generated_at_unix_secs
            ))
        } else if now_unix_secs.saturating_sub(health.generated_at_unix_secs)
            > DRAIN_HEALTH_MAX_AGE.as_secs()
        {
            Some(format!(
                "drain health stale: generated_at={} age_secs={} limit_secs={}",
                health.generated_at_unix_secs,
                now_unix_secs.saturating_sub(health.generated_at_unix_secs),
                DRAIN_HEALTH_MAX_AGE.as_secs()
            ))
        } else if let Some(error) = &health.last_error {
            Some(format!("drain blocked: {error}"))
        } else if health
            .last_success_unix_secs
            .is_some_and(|last| last > now_unix_secs)
        {
            Some(format!(
                "drain health blocked: last_success={} is newer than now={now_unix_secs}",
                health.last_success_unix_secs.unwrap_or(now_unix_secs)
            ))
        } else {
            let no_pending = health.pending_bytes == 0 && health.pending_segments == 0;
            let recent_success = health.last_success_unix_secs.is_some_and(|last| {
                last <= now_unix_secs
                    && now_unix_secs.saturating_sub(last) <= DRAIN_HEALTH_MAX_AGE.as_secs()
            });
            if no_pending || recent_success {
                None
            } else {
                Some(format!(
                    "drain blocked: pending_bytes={} pending_segments={} oldest_age_secs={}",
                    health.pending_bytes,
                    health.pending_segments,
                    health
                        .oldest_pending_age_secs
                        .map_or_else(|| "unknown".to_owned(), |age| age.to_string())
                ))
            }
        };
        self.drain_progressing = blocker.is_none();
        self.drain = health;
        self.drain_blocker = blocker;
    }

    /// The last explicit drainer blocker, if the storage evidence is not safe.
    #[must_use]
    pub fn drain_blocker(&self) -> Option<&str> {
        self.drain_blocker.as_deref()
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

    fn liveness_of(&self, st: Option<&SourceState>, now: Instant) -> (Liveness, Option<u64>) {
        match st.and_then(|state| state.last_sample) {
            None => (Liveness::NeverSeen, None),
            Some(t) => {
                let age = now.saturating_duration_since(t);
                let liveness = if age > self.stale_after {
                    Liveness::Stale
                } else {
                    Liveness::Live
                };
                (liveness, Some(age.as_secs()))
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

        let mut source_ids = self.primary_sources.clone();
        source_ids.extend(self.sources.keys().copied());
        source_ids.sort_unstable();
        source_ids.dedup();
        let sources: Vec<SourceHealth> = source_ids
            .into_iter()
            .map(|source| {
                let state = self.sources.get(&source);
                let (liveness, age) = self.liveness_of(state, now);
                if liveness == Liveness::Stale {
                    let age = age.unwrap_or(0);
                    degraded.push(format!(
                        "source {source} is stale: {age}s since last sample (limit {}s)",
                        self.stale_after.as_secs()
                    ));
                } else if liveness == Liveness::NeverSeen {
                    degraded.push(format!("source {source} has never produced a sample"));
                }
                SourceHealth {
                    source,
                    liveness,
                    last_sample_age_secs: age,
                    samples: state.map_or(0, |value| value.samples),
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

        if let Some(blocker) = &self.drain_blocker {
            degraded.push(blocker.clone());
        }

        let heartbeat = self.heartbeat_inputs(now);
        let max_age = HEARTBEAT_MAX_AGE.as_secs();
        for b in heartbeat.blockers(max_age) {
            degraded.push(format!("heartbeat blocked: {b}"));
        }

        Health {
            uptime_secs: now.saturating_duration_since(self.started).as_secs(),
            sources,
            received_samples: self.received_samples,
            spooled_samples: self.total_samples,
            fsynced_samples: self.fsynced_samples,
            published_samples: self.published_samples,
            displayed_samples: None,
            total_samples: self.total_samples,
            spool_bytes: self.spool_bytes,
            subscribers,
            drain: self.drain.clone(),
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

    fn healthy_drain() -> DrainHealth {
        DrainHealth {
            available: true,
            generated_at_unix_secs: 100,
            ..DrainHealth::default()
        }
    }

    #[test]
    fn existing_spool_bytes_are_reported_at_startup() {
        let s = State::new_with_spool_bytes(vec![SPO2], 4096);
        let h = s.health(Instant::now());
        assert_eq!(h.spool_bytes, 4096);
        assert_eq!(h.received_samples, 0);
    }

    #[test]
    fn pipeline_counters_keep_each_stage_distinct() {
        let mut s = state();
        let now = Instant::now();
        s.record_received(3);
        s.record_appended(&[sample(SPO2, 1), sample(HR, 1)], 512, now);
        s.record_fsynced(1);
        s.record_published(1);
        let h = s.health(now);
        assert_eq!(h.received_samples, 3);
        assert_eq!(h.spooled_samples, 2);
        assert_eq!(h.fsynced_samples, 1);
        assert_eq!(h.published_samples, 1);
        assert_eq!(h.displayed_samples, None);
        assert_eq!(h.total_samples, h.spooled_samples);
    }

    #[test]
    fn a_source_that_never_reported_is_not_healthy() {
        let mut s = state();
        let now = Instant::now();
        s.record_appended(&[sample(HR, 1)], 0, now);
        let h = s.health(now);
        // SpO2 has never been seen, so it is represented explicitly as
        // NeverSeen rather than omitted and mistaken for a quiet healthy night.
        assert_eq!(
            h.sources
                .iter()
                .find(|source| source.source == SPO2)
                .map(|source| source.liveness),
            Some(Liveness::NeverSeen)
        );
        assert_eq!(h.heartbeat.primary_sample_age_secs, None);
        assert!(h.degraded.iter().any(|d| d.contains("source 2 has never")));
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
        s.record_drain_health(healthy_drain(), 100);
        let t0 = Instant::now();
        s.record_appended(&[sample(HR, 1), sample(SPO2, 2)], 0, t0);
        let h = s.health(t0 + Duration::from_secs(1));
        assert!(
            h.sources
                .iter()
                .all(|source| source.liveness == Liveness::Live)
        );
        assert!(h.degraded.is_empty());
    }

    /// HR and HRV corroborate but must never launder a missing primary into
    /// "all clear" (amendment 05 §2.1).
    #[test]
    fn a_corroborating_source_does_not_satisfy_the_primary_requirement() {
        let mut s = state();
        s.record_drain_health(healthy_drain(), 100);
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
        assert!(
            h.degraded
                .iter()
                .any(|d| d.contains("drain health unavailable")),
            "missing storage evidence remains degraded outside a monitoring window"
        );
    }

    #[test]
    fn fresh_empty_drain_health_proves_storage_path() {
        let mut s = state();
        s.record_drain_health(healthy_drain(), 100);
        let t0 = Instant::now();
        s.record_appended(&[sample(SPO2, 2)], 0, t0);
        let h = s.health(t0);
        assert!(s.drain_progressing);
        assert!(s.drain_blocker().is_none());
        assert!(h.degraded.is_empty());
    }

    #[test]
    fn stale_drain_health_names_age_and_limit() {
        let mut s = state();
        s.record_drain_health(healthy_drain(), 106);
        let h = s.health(Instant::now());
        assert!(!s.drain_progressing);
        assert!(
            h.degraded
                .iter()
                .any(|d| d.contains("drain health stale") && d.contains("age_secs=6"))
        );
        assert!(h.degraded.iter().any(|d| d.contains("limit_secs=5")));
    }

    #[test]
    fn future_and_schema_mismatched_drain_health_fail_closed() {
        let mut future = healthy_drain();
        future.generated_at_unix_secs = 101;
        let mut s = state();
        s.record_drain_health(future, 100);
        assert!(
            s.drain_blocker()
                .is_some_and(|d| d.contains("newer than now=100"))
        );

        let mut wrong_schema = healthy_drain();
        wrong_schema.schema += 1;
        s.record_drain_health(wrong_schema, 100);
        assert!(
            s.drain_blocker()
                .is_some_and(|d| d.contains("schema=2") && d.contains("expected=1"))
        );
    }

    #[test]
    fn pending_without_recent_success_names_the_pending_values() {
        let mut future_success = healthy_drain();
        future_success.last_success_unix_secs = Some(101);
        let mut s = state();
        s.record_drain_health(future_success, 100);
        assert!(
            s.drain_blocker()
                .is_some_and(|d| d.contains("last_success=101"))
        );

        let mut recent_success = healthy_drain();
        recent_success.pending_segments = 2;
        recent_success.pending_bytes = 4096;
        recent_success.last_success_unix_secs = Some(99);
        let mut s = state();
        s.record_drain_health(recent_success, 100);
        assert!(s.drain_blocker().is_none());

        let mut health = healthy_drain();
        health.pending_segments = 2;
        health.pending_bytes = 4096;
        health.oldest_pending_age_secs = Some(9);
        let mut s = state();
        s.record_drain_health(health, 100);
        let h = s.health(Instant::now());
        assert!(h.degraded.iter().any(|d| d.contains("pending_bytes=4096")
            && d.contains("pending_segments=2")
            && d.contains("oldest_age_secs=9")));
    }

    #[test]
    fn drainer_error_and_unavailable_evidence_are_named() {
        let mut errored = healthy_drain();
        errored.last_error = Some("PostgreSQL blocked drain: connection refused".to_owned());
        let mut s = state();
        s.record_drain_health(errored, 100);
        assert!(
            s.drain_blocker()
                .is_some_and(|d| d.contains("connection refused"))
        );

        let unavailable = DrainHealth {
            last_error: Some("health file malformed".to_owned()),
            ..DrainHealth::default()
        };
        s.record_drain_health(unavailable, 100);
        assert!(
            s.drain_blocker()
                .is_some_and(|d| d.contains("available=false") && d.contains("malformed"))
        );
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
