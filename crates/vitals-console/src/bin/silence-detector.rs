//! Supervised off-page source silence detector.
//!
//! This process is intentionally separate from the HTTP server. It reads
//! immutable attempt evidence, evaluates every source through the shared core
//! evaluator, then persists status and notification evidence before sleeping.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::env;
use std::time::Duration;
use vitals_console::silence::{
    NotificationEvent, SilenceDetector, SourcePolicy, StatusObservation,
};
use vitals_core::liveness::{Evidence, HealthState, Inflight};
use vitals_registry::{ALL_SOURCES, VitalsSource};

#[derive(Debug, FromRow)]
struct AttemptRow {
    started_at: DateTime<Utc>,
    expected_duration_s: i32,
    finished_at: Option<DateTime<Utc>>,
    outcome: Option<String>,
    detail: Option<String>,
}

#[derive(Debug, FromRow)]
struct SavedStatus {
    state: String,
    reported_at: DateTime<Utc>,
    changed_at: DateTime<Utc>,
    next_repeat_at: Option<DateTime<Utc>>,
    transition_id: String,
    declared_absent: bool,
    muted_until: Option<DateTime<Utc>>,
}

fn epoch(timestamp: DateTime<Utc>) -> i64 {
    timestamp.timestamp()
}

fn parse_state(value: &str) -> Option<HealthState> {
    match value {
        "live" => Some(HealthState::Live),
        "failing" => Some(HealthState::Failing),
        "unknown" => Some(HealthState::Unknown),
        _ => None,
    }
}

fn evidence_from_attempts(attempts: &[AttemptRow], latest_sample_epoch_s: Option<i64>) -> Evidence {
    let latest = attempts.first();
    let latest_attempt_epoch_s = latest.map(|attempt| epoch(attempt.started_at));
    let last_attempt_failure = latest.and_then(|attempt| {
        if attempt.finished_at.is_some() && attempt.outcome.as_deref() != Some("succeeded") {
            Some(
                attempt
                    .detail
                    .clone()
                    .unwrap_or_else(|| "attempt did not succeed".to_string()),
            )
        } else {
            None
        }
    });
    let last_success_epoch_s = attempts.iter().find_map(|attempt| {
        (attempt.outcome.as_deref() == Some("succeeded"))
            .then(|| attempt.finished_at.map(epoch))
            .flatten()
    });
    let inflight = latest.and_then(|attempt| {
        attempt.finished_at.is_none().then(|| {
            Some(Inflight {
                started_epoch_s: epoch(attempt.started_at),
                expected_duration_s: u32::try_from(attempt.expected_duration_s).ok()?,
            })
        })?
    });
    // The sample timestamp goes in its own field. It used to be substituted
    // into `last_attempt_epoch_s` whenever it was newer, which compared a
    // sample age against an attempt cadence — and because `attempts` has never
    // held a single row, that substitution was the *only* path that ever
    // produced evidence. For a source declaring one sample per second, the
    // ordinary lag between collection and commit then read as stale (measured
    // 2026-11-09, n=36: p50 35s, max 44s, on a ~30s commit cycle), so a ten-day
    // outage produced no state transition to notice.
    Evidence {
        last_attempt_epoch_s: latest_attempt_epoch_s,
        last_attempt_failure,
        last_success_epoch_s,
        inflight,
        last_sample_epoch_s: latest_sample_epoch_s,
        // `last_link_epoch_s` and `activity` come from the collector's status
        // file, which this binary does not read yet. Defaulting them is safe in
        // the only direction that matters: an unobserved activity gate never
        // licenses escalation, and an absent contact timestamp contributes
        // nothing. It does mean the AirSense cannot yet escalate on transport
        // silence, which is the gap that leaves the original outage undetected
        // -- see max_link_silence_s in registry.toml.
        ..Default::default()
    }
}

async fn load_attempts(
    pool: &PgPool,
    source: VitalsSource,
) -> Result<Vec<AttemptRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT started_at, expected_duration_s, finished_at, outcome, detail \
         FROM attempts WHERE kind = 'source' AND subject = $1 \
         ORDER BY started_at DESC LIMIT 128",
    )
    .bind(source.id().to_string())
    .fetch_all(pool)
    .await
}

async fn load_latest_sample_epoch(
    pool: &PgPool,
    source: VitalsSource,
) -> Result<Option<i64>, sqlx::Error> {
    let latest: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT timestamp FROM samples WHERE source = $1 ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(source.id())
    .fetch_optional(pool)
    .await?
    .flatten();
    Ok(latest.map(epoch))
}

async fn load_saved_status(
    pool: &PgPool,
    source: VitalsSource,
) -> Result<Option<StatusObservation>, sqlx::Error> {
    let saved = sqlx::query_as::<_, SavedStatus>(
        "SELECT state, reported_at, changed_at, next_repeat_at, transition_id, \
                declared_absent, muted_until FROM status_observations WHERE subject = $1",
    )
    .bind(source.id().to_string())
    .fetch_optional(pool)
    .await?;
    Ok(saved.and_then(|saved| {
        parse_state(&saved.state).map(|state| StatusObservation {
            source,
            state,
            reason: String::new(),
            reported_at_epoch_s: epoch(saved.reported_at),
            changed_at_epoch_s: epoch(saved.changed_at),
            next_repeat_at_epoch_s: saved.next_repeat_at.map(epoch),
            transition_id: saved.transition_id,
            declared_absent: saved.declared_absent,
            muted_until_epoch_s: saved.muted_until.map(epoch),
        })
    }))
}

fn state_name(state: HealthState) -> &'static str {
    match state {
        HealthState::Live => "live",
        HealthState::Failing => "failing",
        HealthState::Unknown => "unknown",
    }
}

async fn save_status(pool: &PgPool, status: &StatusObservation) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO status_observations \
         (subject, state, reason, reported_at, changed_at, next_repeat_at, transition_id, declared_absent, muted_until) \
         VALUES ($1, $2, $3, to_timestamp($4), to_timestamp($5), \
                 CASE WHEN $6::bigint IS NULL THEN NULL ELSE to_timestamp($6) END, $7, $8, \
                 CASE WHEN $9::bigint IS NULL THEN NULL ELSE to_timestamp($9) END) \
         ON CONFLICT (subject) DO UPDATE SET state = EXCLUDED.state, reason = EXCLUDED.reason, \
           reported_at = EXCLUDED.reported_at, changed_at = EXCLUDED.changed_at, \
           next_repeat_at = EXCLUDED.next_repeat_at, transition_id = EXCLUDED.transition_id, \
           declared_absent = EXCLUDED.declared_absent, muted_until = EXCLUDED.muted_until",
    )
    .bind(status.source.id().to_string())
    .bind(state_name(status.state))
    .bind(&status.reason)
    .bind(status.reported_at_epoch_s)
    .bind(status.changed_at_epoch_s)
    .bind(status.next_repeat_at_epoch_s)
    .bind(&status.transition_id)
    .bind(status.declared_absent)
    .bind(status.muted_until_epoch_s)
    .execute(pool)
    .await
    .map(|_| ())
}

async fn save_event(pool: &PgPool, event: &NotificationEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO notification_events \
         (subject, transition_id, state, reason, emitted_at, suppressed_by_mute) \
         VALUES ($1, $2, $3, $4, to_timestamp($5), $6)",
    )
    .bind(event.source.id().to_string())
    .bind(&event.transition_id)
    .bind(state_name(event.state))
    .bind(&event.reason)
    .bind(event.emitted_at_epoch_s)
    .bind(event.suppressed_by_mute)
    .execute(pool)
    .await
    .map(|_| ())
}

fn jitter_seconds() -> i64 {
    let mut byte = [0_u8; 1];
    getrandom::getrandom(&mut byte).map_or(0, |()| i64::from(byte[0] % 60))
}

async fn run_once(pool: &PgPool, detector: &mut SilenceDetector, now_epoch_s: i64) {
    for source in ALL_SOURCES {
        let saved = match load_saved_status(pool, source).await {
            Ok(saved) => saved,
            Err(error) => {
                eprintln!("silence detector: status load failed for {source:?}: {error}");
                None
            }
        };
        if let Some(saved) = &saved {
            detector.restore(saved);
        }
        let attempts = match load_attempts(pool, source).await {
            Ok(attempts) => attempts,
            Err(error) => {
                eprintln!("silence detector: attempt load failed for {source:?}: {error}");
                continue;
            }
        };
        let latest_sample_epoch_s = match load_latest_sample_epoch(pool, source).await {
            Ok(latest) => latest,
            Err(error) => {
                eprintln!("silence detector: sample load failed for {source:?}: {error}");
                continue;
            }
        };
        let evidence = evidence_from_attempts(&attempts, latest_sample_epoch_s);
        let policy = SourcePolicy {
            source,
            // A retired source is absent by declaration -- the device is gone,
            // so it is not failing and must not emit. The operator flag means
            // the same thing for a device that is present but deliberately
            // unmonitored, so they fold together here rather than becoming two
            // suppression paths that can disagree.
            declared_absent: source.is_retired()
                || saved.as_ref().is_some_and(|saved| saved.declared_absent),
            mute_until_epoch_s: saved.as_ref().and_then(|saved| saved.muted_until_epoch_s),
        };
        let (status, event) = detector.observe(now_epoch_s, policy, &evidence, jitter_seconds());
        if let Err(error) = save_status(pool, &status).await {
            eprintln!("silence detector: status save failed for {source:?}: {error}");
            continue;
        }
        if let Some(event) = event
            && let Err(error) = save_event(pool, &event).await
        {
            eprintln!("silence detector: event save failed for {source:?}: {error}");
        }
    }
}

fn database_url_is_supported(value: &str) -> bool {
    value.starts_with("postgres://") || value.starts_with("postgresql://")
}

const STATUS_SCHEMA_PROBE: &str = "SELECT subject, state, reason, reported_at, changed_at, next_repeat_at, transition_id, \
            declared_absent, muted_until FROM status_observations WHERE FALSE";
const NOTIFICATION_SCHEMA_PROBE: &str = "SELECT subject, transition_id, state, reason, emitted_at, suppressed_by_mute \
     FROM notification_events WHERE FALSE";
const SAMPLE_SCHEMA_PROBE: &str = "SELECT source, timestamp FROM samples WHERE FALSE";

async fn validate_detector_schema(pool: &PgPool) -> Result<(), String> {
    for (object, query) in [
        ("status_observations detector columns", STATUS_SCHEMA_PROBE),
        ("notification_events table", NOTIFICATION_SCHEMA_PROBE),
        ("samples source timestamp", SAMPLE_SCHEMA_PROBE),
    ] {
        sqlx::query(query)
            .execute(pool)
            .await
            .map_err(|error| format!("{object}: {error}"))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let database_url = match env::var("DATABASE_URL") {
        Ok(value) if database_url_is_supported(&value) => value,
        Ok(_) => {
            eprintln!(
                "silence detector blocked: DATABASE_URL must be a postgres:// or postgresql:// URL"
            );
            std::process::exit(64);
        }
        Err(_) => {
            eprintln!("silence detector blocked: DATABASE_URL is unset");
            std::process::exit(64);
        }
    };
    let pool = match PgPool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("silence detector blocked: database connection failed: {error}");
            std::process::exit(70);
        }
    };
    if let Err(error) = validate_detector_schema(&pool).await {
        eprintln!("silence detector blocked: schema validation failed: {error}");
        std::process::exit(70);
    }
    let mut detector = SilenceDetector::default();
    loop {
        let now_epoch_s = Utc::now().timestamp();
        run_once(&pool, &mut detector, now_epoch_s).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptRow, NOTIFICATION_SCHEMA_PROBE, SAMPLE_SCHEMA_PROBE, STATUS_SCHEMA_PROBE,
        database_url_is_supported, evidence_from_attempts,
    };
    use chrono::{DateTime, TimeDelta, Utc};
    use sqlx::postgres::PgConnectOptions;
    use std::str::FromStr;

    #[test]
    fn accepts_sqlx_postgres_url_schemes() {
        assert!(database_url_is_supported(
            "postgresql://vitals@localhost/vitals?host=%2Fvar%2Frun%2Fpostgresql"
        ));
        assert!(database_url_is_supported(
            "postgres://vitals@localhost/vitals"
        ));
    }

    #[test]
    fn accepts_the_configured_unix_socket_url() {
        assert!(
            PgConnectOptions::from_str(
                "postgresql://vitals@localhost/vitals?host=%2Fvar%2Frun%2Fpostgresql"
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_libpq_keyword_connection_strings() {
        assert!(!database_url_is_supported(
            "host=/var/run/postgresql user=vitals dbname=vitals"
        ));
        assert!(!database_url_is_supported(""));
    }

    // The four tests below used to pin the substitution rather than catch it.
    // Their old names said so -- "supersedes missing source attempts",
    // "remains authoritative until a new sample", "clears the failure" -- and
    // two of them passed only because their sample happened to be *older* than
    // their attempt, so the substitution never fired. That is why a suite of
    // 208 tests stayed green through a ten-day outage: it was asserting the
    // defect. Each now pins the separation instead.

    /// An empty `attempts` table means no attempt was observed. Reporting a
    /// sample timestamp as an attempt timestamp invents a measurement, and
    /// `attempts` has never held a row, so this was the live case.
    #[test]
    fn sample_evidence_does_not_fabricate_an_attempt() {
        let evidence = evidence_from_attempts(&[], Some(1_700_000_000));
        assert_eq!(evidence.last_attempt_epoch_s, None);
        assert_eq!(evidence.last_success_epoch_s, None);
        assert_eq!(evidence.last_sample_epoch_s, Some(1_700_000_000));
        assert!(evidence.last_attempt_failure.is_none());
        assert!(evidence.inflight.is_none());
    }

    /// Data arriving after a failed attempt does not retract the failure.
    /// §2.5 puts FAILING above LIVE precisely because the drainer can flush a
    /// spool while the collector is still broken, so arriving samples are not
    /// evidence that collection recovered. A real recovery writes a success
    /// row; until collectors write attempt rows at all, `attempts` stays empty
    /// and there are no failure rows to retract either way.
    #[test]
    fn a_recorded_failure_survives_a_newer_sample() {
        let attempt = AttemptRow {
            started_at: DateTime::<Utc>::UNIX_EPOCH + TimeDelta::seconds(1_700_000_000),
            expected_duration_s: 30,
            finished_at: Some(DateTime::<Utc>::UNIX_EPOCH + TimeDelta::seconds(1_700_000_010)),
            outcome: Some("failed".to_string()),
            detail: Some("connection failed".to_string()),
        };
        let evidence = evidence_from_attempts(&[attempt], Some(1_700_000_020));
        assert_eq!(
            evidence.last_attempt_failure.as_deref(),
            Some("connection failed")
        );
        assert_eq!(evidence.last_attempt_epoch_s, Some(1_700_000_000));
        assert_eq!(evidence.last_sample_epoch_s, Some(1_700_000_020));
    }

    #[test]
    fn a_failed_attempt_and_a_sample_occupy_separate_fields() {
        let started_at = DateTime::<Utc>::UNIX_EPOCH + TimeDelta::seconds(1_700_000_000);
        let attempt = AttemptRow {
            started_at,
            expected_duration_s: 30,
            finished_at: Some(started_at + TimeDelta::seconds(10)),
            outcome: Some("failed".to_string()),
            detail: Some("connection failed".to_string()),
        };
        let evidence = evidence_from_attempts(&[attempt], Some(1_700_000_005));
        assert_eq!(
            evidence.last_attempt_failure.as_deref(),
            Some("connection failed")
        );
        assert_eq!(evidence.last_attempt_epoch_s, Some(1_700_000_000));
        assert_eq!(evidence.last_sample_epoch_s, Some(1_700_000_005));
    }

    #[test]
    fn a_successful_attempt_and_a_sample_occupy_separate_fields() {
        let attempt = AttemptRow {
            started_at: DateTime::<Utc>::UNIX_EPOCH + TimeDelta::seconds(1_700_000_000),
            expected_duration_s: 30,
            finished_at: Some(DateTime::<Utc>::UNIX_EPOCH + TimeDelta::seconds(1_700_000_010)),
            outcome: Some("succeeded".to_string()),
            detail: None,
        };
        let evidence = evidence_from_attempts(&[attempt], Some(1_700_000_005));
        assert_eq!(evidence.last_attempt_epoch_s, Some(1_700_000_000));
        assert_eq!(evidence.last_success_epoch_s, Some(1_700_000_010));
        assert_eq!(evidence.last_sample_epoch_s, Some(1_700_000_005));
    }

    #[test]
    fn schema_probes_cover_persisted_detector_state() {
        assert!(SAMPLE_SCHEMA_PROBE.contains("timestamp"));
        assert!(STATUS_SCHEMA_PROBE.contains("subject"));
        assert!(STATUS_SCHEMA_PROBE.contains("reported_at"));
        assert!(STATUS_SCHEMA_PROBE.contains("next_repeat_at"));
        assert!(STATUS_SCHEMA_PROBE.contains("muted_until"));
        assert!(NOTIFICATION_SCHEMA_PROBE.contains("notification_events"));
        assert!(NOTIFICATION_SCHEMA_PROBE.contains("emitted_at"));
        assert!(NOTIFICATION_SCHEMA_PROBE.contains("suppressed_by_mute"));
    }
}
