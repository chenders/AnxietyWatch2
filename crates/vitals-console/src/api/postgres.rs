use crate::handlers::AppState;
use async_trait::async_trait;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_postgres::{Client, NoTls};

/// Query parameters for `/api/latest`.
#[derive(Deserialize)]
pub struct LatestParams {
    /// Registry source id.
    pub source: i32,
    /// Registry type id.
    pub r#type: i32,
}

/// Response body for `/api/latest`.
#[derive(Serialize)]
pub struct LatestResponse {
    /// Whether a sample exists for the requested source/type; `false` means
    /// "not measured", which is distinct from a healthy or failing value.
    pub measured: bool,
    /// Timestamp of the latest sample, Unix microseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub t_micros: Option<i64>,
    /// Why the sample is not usable, when `measured` is `false` despite a row
    /// existing (e.g. a future-dated timestamp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Latest value in the registry's canonical unit for `type`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Seconds since the latest sample was recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation_age_s: Option<i64>,
}

/// One per-day aggregate row for `/api/nightly`.
#[derive(Serialize)]
pub struct NightlyRow {
    /// Registry source id.
    pub source: i32,
    /// Registry type id.
    pub r#type: i32,
    /// UTC midnight of the bucket, Unix microseconds.
    pub day_micros: i64,
    /// Minimum value in the bucket.
    pub min: f64,
    /// Maximum value in the bucket.
    pub max: f64,
    /// Mean value in the bucket.
    pub avg: f64,
    /// Number of samples in the bucket.
    pub count: i64,
}

/// `PostgreSQL` access for the read API, injectable so handlers are testable
/// without a database.
#[async_trait]
pub trait DbOps: Send + Sync {
    /// Return the deployed Sqitch change id and pending migration count.
    ///
    /// # Errors
    /// Returns a database/Sqitch metadata error naming why identity is unavailable.
    async fn get_schema_identity(&self) -> Result<(String, u64), String>;

    /// Return the most recent `(timestamp, value)` for a source/type, or
    /// `None` if that source/type has never been measured.
    ///
    /// # Errors
    /// Returns a database error string when the query fails.
    async fn get_latest(&self, source: i32, r#type: i32) -> Result<Option<(i64, f64)>, String>;

    /// Return `(timestamp_micros, value)` rows in `[since, until)` for an
    /// optional source/type filter, oldest first. The caller bounds the window.
    ///
    /// # Errors
    /// Returns a database error string when the query fails.
    async fn get_range(
        &self,
        source: i32,
        r#type: i32,
        since_micros: i64,
        until_micros: i64,
    ) -> Result<Vec<(i64, f64)>, String>;
    /// Return per-day aggregates over the last `nights` days, optionally
    /// restricted to one source.
    ///
    /// # Errors
    /// Returns a database error string when the query fails.
    async fn get_nightly(
        &self,
        nights: u32,
        source: Option<i32>,
    ) -> Result<Vec<NightlyRow>, String>;
}

/// Production `DbOps` backed by a live `PostgreSQL` connection.
pub struct PgDbOps {
    /// `PostgreSQL` connection URL in libpq format.
    pub db_url: String,
}

impl PgDbOps {
    async fn connect(&self) -> Result<Client, String> {
        let (client, connection) = tokio_postgres::connect(&self.db_url, NoTls)
            .await
            .map_err(|e| format!("db connect: {e}"))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });
        Ok(client)
    }
}

#[async_trait]
impl DbOps for PgDbOps {
    async fn get_schema_identity(&self) -> Result<(String, u64), String> {
        let client = self.connect().await?;
        let deployed: Option<String> = client
            .query_one(
                "SELECT change_id FROM sqitch.changes ORDER BY committed_at DESC LIMIT 1",
                &[],
            )
            .await
            .map_err(|e| format!("schema identity query failed: {e}"))?
            .get(0);
        let revision = deployed.ok_or_else(|| {
            "schema identity query returned no deployed Sqitch changes".to_string()
        })?;
        let deployed_count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM sqitch.changes", &[])
            .await
            .map_err(|e| format!("schema change-count query failed: {e}"))?
            .get(0);
        let planned_count = env!("VITALS_PLANNED_SCHEMA_CHANGES")
            .parse::<u64>()
            .map_err(|e| format!("embedded planned schema change count is invalid: {e}"))?;
        let deployed_count = u64::try_from(deployed_count)
            .map_err(|_| format!("deployed schema change count is negative: {deployed_count}"))?;
        let pending = planned_count.checked_sub(deployed_count).ok_or_else(|| {
            format!(
                "deployed schema change count {deployed_count} exceeds embedded plan count {planned_count}"
            )
        })?;
        Ok((revision, pending))
    }

    async fn get_latest(&self, source: i32, r#type: i32) -> Result<Option<(i64, f64)>, String> {
        let client = self.connect().await?;
        let row_opt = client
            .query_opt(
                "SELECT (EXTRACT(epoch FROM timestamp) * 1000000)::bigint, value
             FROM samples
             WHERE source = $1 AND type = $2
             ORDER BY timestamp DESC LIMIT 1",
                &[&source, &r#type],
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(row_opt.map(|row| (row.get(0), row.get(1))))
    }

    async fn get_range(
        &self,
        source: i32,
        r#type: i32,
        since_micros: i64,
        until_micros: i64,
    ) -> Result<Vec<(i64, f64)>, String> {
        let client = self.connect().await?;
        let rows = client
            .query(
                "SELECT (EXTRACT(epoch FROM timestamp) * 1000000)::bigint, value
                 FROM samples
                 WHERE source = $1
                   AND type = $2
                   AND timestamp >= to_timestamp($3::bigint / 1000000.0)
                   AND timestamp <  to_timestamp($4::bigint / 1000000.0)
                 ORDER BY timestamp",
                &[&source, &r#type, &since_micros, &until_micros],
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.iter().map(|r| (r.get(0), r.get(1))).collect())
    }

    async fn get_nightly(
        &self,
        nights: u32,
        source: Option<i32>,
    ) -> Result<Vec<NightlyRow>, String> {
        let client = self.connect().await?;
        // A `nights` value above i32::MAX cannot be bound to the SQL `int`
        // parameter; reject it by name rather than clamping.
        let nights_int =
            i32::try_from(nights).map_err(|_| format!("nights too large: {nights}"))?;

        // OPERATOR DIRECTIVE (2024-06-25): Exclude synthetic sources (7: polar_h10_emulator)
        // unless explicitly requested by the caller.
        let (_query, rows) = if let Some(src) = source {
            let query = "
                SELECT
                    source,
                    type,
                    (EXTRACT(epoch FROM date_trunc('day', timestamp - interval '12 hours')) * 1000000)::bigint as day_micros,
                    MIN(value) as min_val,
                    MAX(value) as max_val,
                    AVG(value) as avg_val,
                    COUNT(*) as sample_count
                FROM samples
                WHERE timestamp >= date_trunc('day', now() - interval '12 hours') + interval '12 hours' - ($1::int * interval '1 day')
                  AND timestamp < date_trunc('day', now() - interval '12 hours') + interval '12 hours'
                  AND source = $2
                GROUP BY source, type, date_trunc('day', timestamp - interval '12 hours')
                ORDER BY day_micros DESC
            ";
            (
                query,
                client
                    .query(query, &[&nights_int, &src])
                    .await
                    .map_err(|e| e.to_string())?,
            )
        } else {
            let query = "
                SELECT
                    source,
                    type,
                    (EXTRACT(epoch FROM date_trunc('day', timestamp - interval '12 hours')) * 1000000)::bigint as day_micros,
                    MIN(value) as min_val,
                    MAX(value) as max_val,
                    AVG(value) as avg_val,
                    COUNT(*) as sample_count
                FROM samples
                WHERE timestamp >= date_trunc('day', now() - interval '12 hours') + interval '12 hours' - ($1::int * interval '1 day')
                  AND timestamp < date_trunc('day', now() - interval '12 hours') + interval '12 hours'
                  AND source <> 7
                GROUP BY source, type, date_trunc('day', timestamp - interval '12 hours')
                ORDER BY day_micros DESC
            ";
            (
                query,
                client
                    .query(query, &[&nights_int])
                    .await
                    .map_err(|e| e.to_string())?,
            )
        };

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            result.push(NightlyRow {
                source: row.get(0),
                r#type: row.get(1),
                day_micros: row.get(2),
                min: row.get(3),
                max: row.get(4),
                avg: row.get(5),
                count: row.get(6),
            });
        }
        Ok(result)
    }
}

/// GET /api/latest?source=&type=
///
/// Serves the most recent sample for a source/type, or an explicit
/// "not measured" marker when none exists.
///
/// # Errors
/// 500 on a database failure, or when the system clock is before the Unix
/// epoch (ages would be lies).
pub async fn latest_handler(
    State(state): State<AppState>,
    Query(params): Query<LatestParams>,
) -> Result<Json<LatestResponse>, (StatusCode, String)> {
    let row_opt = state
        .db_ops
        .get_latest(params.source, params.r#type)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    match row_opt {
        Some((t_micros, value)) => {
            // A pre-epoch clock would make the age a lie; refuse by name.
            let now_s = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("system clock is before the Unix epoch: {e}"),
                    )
                })?
                .as_secs();
            let now_s = i64::try_from(now_s).unwrap_or(i64::MAX);
            let sample_s = t_micros / 1_000_000;
            if sample_s > now_s {
                // A future timestamp is clock skew or a corrupt row; saturating
                // it to "0s old" would render an invalid sample as fresh.
                return Ok(Json(LatestResponse {
                    measured: false,
                    t_micros: Some(t_micros),
                    value: None,
                    observation_age_s: None,
                    reason: Some(format!(
                        "rejected: sample timestamp is {}s in the future — clock skew or corrupt row",
                        sample_s - now_s
                    )),
                }));
            }
            let age = now_s - sample_s;

            Ok(Json(LatestResponse {
                measured: true,
                t_micros: Some(t_micros),
                value: Some(value),
                observation_age_s: Some(age),
                reason: None,
            }))
        }
        None => Ok(Json(LatestResponse {
            measured: false,
            reason: None,
            t_micros: None,
            value: None,
            observation_age_s: None,
        })),
    }
}

/// Query parameters for `/api/nightly`.
#[derive(Deserialize)]
pub struct NightlyParams {
    /// Number of days of aggregates to return; defaults to 7.
    pub nights: Option<u32>,
    /// Restrict the result to one registry source, when set.
    pub source: Option<i32>,
}

/// GET /api/nightly?nights=&source=
///
/// Serves per-day min/max/avg/count aggregates from `samples`.
///
/// # Errors
/// 500 on a database failure, or when `nights` exceeds `i32::MAX`.
pub async fn nightly_handler(
    State(state): State<AppState>,
    Query(params): Query<NightlyParams>,
) -> Result<Json<Vec<NightlyRow>>, (StatusCode, String)> {
    let nights = params.nights.unwrap_or(7);
    let rows = state
        .db_ops
        .get_nightly(nights, params.source)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(rows))
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "unit tests use fixed fixtures and a stub DbOps; unwrap is the assertion that setup and the handler call succeed"
    )]
    #![expect(
        clippy::indexing_slicing,
        reason = "tests index serde_json::Value views of canned handler responses"
    )]

    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    struct StubDbOps {
        has_latest: bool,
    }

    #[async_trait]
    impl DbOps for StubDbOps {
        async fn get_schema_identity(&self) -> Result<(String, u64), String> {
            Ok(("fixture-change".to_string(), 0))
        }
        async fn get_latest(&self, _source: i32, _type: i32) -> Result<Option<(i64, f64)>, String> {
            if self.has_latest {
                Ok(Some((1_600_000_000_000_000, 42.0)))
            } else {
                Ok(None)
            }
        }
        async fn get_nightly(
            &self,
            _nights: u32,
            _source: Option<i32>,
        ) -> Result<Vec<NightlyRow>, String> {
            Ok(vec![])
        }
        async fn get_range(
            &self,
            _s: i32,
            _t: i32,
            _since: i64,
            _until: i64,
        ) -> Result<Vec<(i64, f64)>, String> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_latest_null_safety() {
        let db = Arc::new(StubDbOps { has_latest: false });
        let state = AppState {
            socket_path: std::path::PathBuf::new(),
            db: None,
            test_routes_enabled: false,
            db_url: String::new(),
            sys_ops: crate::api::ops::SysOps::real(),
            host_history: crate::api::host::HostHistory::default(),
            db_ops: db.clone(),
            security: std::sync::Arc::new(crate::security::SecurityState::unconfigured()),
            ops_token: None,
            server_instance_id: None,
            client_origin: None,
            server_display_label: "test server".to_string(),
        };

        let resp = latest_handler(
            State(state.clone()),
            Query(LatestParams {
                source: 1,
                r#type: 1,
            }),
        )
        .await
        .unwrap();
        let json_val = serde_json::to_value(&resp.0).unwrap();

        assert_eq!(json_val, json!({ "measured": false }));

        let db2 = Arc::new(StubDbOps { has_latest: true });
        let state2 = AppState {
            socket_path: std::path::PathBuf::new(),
            db: None,
            test_routes_enabled: false,
            db_url: String::new(),
            sys_ops: crate::api::ops::SysOps::real(),
            host_history: crate::api::host::HostHistory::default(),
            db_ops: db2,
            security: std::sync::Arc::new(crate::security::SecurityState::unconfigured()),
            ops_token: None,
            server_instance_id: None,
            client_origin: None,
            server_display_label: "test server".to_string(),
        };

        let resp2 = latest_handler(
            State(state2),
            Query(LatestParams {
                source: 1,
                r#type: 1,
            }),
        )
        .await
        .unwrap();
        let json_val2 = serde_json::to_value(&resp2.0).unwrap();

        assert_eq!(json_val2["measured"], true);
        assert_eq!(json_val2["value"], 42.0);
    }
}
