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

fn evidence_from_attempts(attempts: &[AttemptRow]) -> Evidence {
    let latest = attempts.first();
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
    Evidence {
        last_attempt_epoch_s: latest.map(|attempt| epoch(attempt.started_at)),
        last_attempt_failure,
        last_success_epoch_s,
        inflight,
        learned_interval_s: None,
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
        let evidence = evidence_from_attempts(&attempts);
        let policy = SourcePolicy {
            source,
            declared_absent: saved.as_ref().is_some_and(|saved| saved.declared_absent),
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
    let mut detector = SilenceDetector::default();
    loop {
        let now_epoch_s = Utc::now().timestamp();
        run_once(&pool, &mut detector, now_epoch_s).await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::database_url_is_supported;
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
}
