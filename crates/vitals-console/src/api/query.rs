use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::handlers::AppState;

/// Query parameters for `/api/query`.
#[derive(Deserialize)]
pub struct QueryParams {
    /// Registry source id.
    pub source: i32,
    /// Registry type id.
    pub r#type: i32,
    /// Inclusive lower bound, Unix microseconds.
    pub since_micros: i64,
    /// Exclusive upper bound, Unix microseconds.
    pub until_micros: i64,
}

/// One sample returned by `/api/query`.
#[derive(Serialize)]
pub struct QueryRow {
    /// Sample timestamp, Unix microseconds.
    pub t_micros: i64,
    /// Sample value in the registry's canonical unit for `type`.
    pub value: f64,
}

/// GET `/api/query`
///
/// Returns samples in the requested window from the `PostgreSQL` store.
///
/// # Errors
/// 400 for an inverted time window or one exceeding 24 hours; 500 with the
/// database error named when the store query fails.
pub async fn query_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<QueryRow>>, (StatusCode, String)> {
    let window_micros = params.until_micros.saturating_sub(params.since_micros);
    if window_micros < 0 {
        return Err((StatusCode::BAD_REQUEST, "invalid time window".into()));
    }
    let max_window_micros = 24 * 3600 * 1_000_000_i64;
    if window_micros > max_window_micros {
        return Err((StatusCode::BAD_REQUEST, "time window exceeds 24h".into()));
    }

    // The daemon's Query verb is a deliberate G3-era stub ("query needs the
    // PostgreSQL store"). The store exists and the console owns read access,
    // so this endpoint reads PostgreSQL directly through the injected DbOps —
    // one query surface, same as /api/latest and /api/nightly.
    let rows = state
        .db_ops
        .get_range(
            params.source,
            params.r#type,
            params.since_micros,
            params.until_micros,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("store query: {e}"),
            )
        })?;
    Ok(Json(
        rows.into_iter()
            .map(|(t_micros, value)| QueryRow { t_micros, value })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_query_params_require_source_and_type() {
        // Missing source
        let bad1 = json!({
            "type": 1,
            "since_micros": 0,
            "until_micros": 1000
        });
        assert!(serde_json::from_value::<QueryParams>(bad1).is_err());

        // Missing type
        let bad2 = json!({
            "source": 1,
            "since_micros": 0,
            "until_micros": 1000
        });
        assert!(serde_json::from_value::<QueryParams>(bad2).is_err());

        // Both present
        let good = json!({
            "source": 1,
            "type": 2,
            "since_micros": 0,
            "until_micros": 1000
        });
        assert!(serde_json::from_value::<QueryParams>(good).is_ok());
    }
}
