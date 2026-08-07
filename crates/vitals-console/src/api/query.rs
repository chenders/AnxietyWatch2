use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;
use vitals_proto::{Request, Response, check_frame_len, decode_payload, encode_frame};

use crate::handlers::AppState;

/// Query parameters for `/api/query`.
#[derive(Deserialize)]
pub struct QueryParams {
    /// Registry source id, or `None` for all sources.
    pub source: Option<i32>,
    /// Registry type id, or `None` for all types.
    pub r#type: Option<i32>,
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
/// Streams samples in the requested window from the `vitalsd` Unix socket.
///
/// # Errors
/// 400 for an empty or inverted time window or one exceeding 24 hours; 503
/// when the daemon socket cannot be connected; 504 on a 30-second timeout;
/// 500 on frame or serialization errors, or a daemon error reply.
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

    let req = Request::Query {
        source: params.source,
        r#type: params.r#type,
        since_unix_micros: params.since_micros,
        until_unix_micros: params.until_micros,
    };

    let req_bytes =
        encode_frame(&req).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let future = async {
        let mut stream = UnixStream::connect(&state.socket_path).await.map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("connect failed: {e}"),
            )
        })?;

        stream
            .write_all(&req_bytes)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut rows = Vec::new();

        loop {
            let mut len_buf = [0u8; 4];
            let read_res = stream.read_exact(&mut len_buf).await;
            if let Err(e) = read_res {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    break;
                }
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
            }

            let len = check_frame_len(u32::from_le_bytes(len_buf))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let mut payload = vec![0u8; len];
            stream
                .read_exact(&mut payload)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let resp: Response = decode_payload(&payload)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            match resp {
                Response::Sample { sample } => {
                    rows.push(QueryRow {
                        t_micros: sample.timestamp_unix_micros,
                        value: sample.value,
                    });
                }
                Response::Error { detail, .. } => {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, detail));
                }
                _ => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "unexpected response type".into(),
                    ));
                }
            }
        }

        Ok(rows)
    };

    let rows = timeout(Duration::from_secs(30), future)
        .await
        .map_err(|_| {
            (
                StatusCode::GATEWAY_TIMEOUT,
                "timeout waiting for daemon".into(),
            )
        })??;

    Ok(Json(rows))
}
