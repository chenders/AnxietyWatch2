//! Database operations for the vitals-console HTTP service.
//!
//! All queries use `sqlx::query` (runtime-checked), not `sqlx::query!`
//! (compile-time macros), so the build does not need a database.

use axum::Json;
use axum::http::StatusCode;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AttemptView, DomainRecordResponse, Effect, ErrorResponse, error_reply};

/// Register a client (upsert on `client_id`).
///
/// - New `client_id` → insert, return 201.
/// - Existing `client_id` with matching secret → idempotent 200.
/// - Existing `client_id` with different secret → 409 conflict (takeover
///   attempt refused by name, spec §2.3b).
///
/// # Errors
/// 500 on a database failure; 409 when re-registering an existing `client_id`
/// with a different secret (the takeover attempt is refused).
pub async fn register_client(
    pool: &PgPool,
    client_id: Uuid,
    secret: &[u8],
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // Check if this client_id already exists.
    let existing: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT secret FROM envelope_clients WHERE client_id = $1")
            .bind(client_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                error_reply(StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}"))
            })?;

    if let Some((stored_secret,)) = existing {
        if stored_secret == secret {
            // Idempotent re-registration with same secret.
            let _ = sqlx::query(
                "UPDATE envelope_clients SET last_seen_at = now() WHERE client_id = $1",
            )
            .bind(client_id)
            .execute(pool)
            .await;
            let body = serde_json::json!({
                "client_id": client_id,
                "status": "already_registered"
            });
            return Ok((StatusCode::OK, Json(body)));
        }
        // Different secret → takeover attempt refused.
        return Err(error_reply(
            StatusCode::CONFLICT,
            format!("re-registration with different secret refused for client_id={client_id}"),
        ));
    }

    // New client.
    sqlx::query("INSERT INTO envelope_clients (client_id, secret) VALUES ($1, $2)")
        .bind(client_id)
        .bind(secret)
        .execute(pool)
        .await
        .map_err(|e| {
            error_reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error on insert: {e}"),
            )
        })?;

    let body = serde_json::json!({"client_id": client_id, "status": "registered"});
    Ok((StatusCode::CREATED, Json(body)))
}

/// Outcome of a domain-record write attempt.
pub enum DomainRecordOutcome {
    /// New record created.
    Created {
        /// The stored record, with hashes and timestamps.
        response: DomainRecordResponse,
    },
    /// No-op replay — same id, same hash.
    Replayed {
        /// The stored record, marked `replayed: true`.
        response: DomainRecordResponse,
    },
    /// Conflict — same id, different hash.
    Conflict {
        /// The id whose content already differs.
        id: Uuid,
    },
}

/// Process a domain record, applying the idempotency rules (spec §2.3b).
///
/// `content_sha256` must be the SHA-256 of `serde_json::to_vec(&content)`.
/// Canonicalization is byte-equality of `serde_json` serialization, which is
/// sufficient for a single writer today (one phone, deterministic serializer).
/// If multiple concurrent writers are added, a robust canonicalization scheme
/// (e.g. JCS) would be needed.
///
/// # Errors
/// 500 on a database failure.
pub async fn process_domain_record(
    pool: &PgPool,
    id: Uuid,
    kind: &str,
    content: &serde_json::Value,
    content_sha256: &[u8; 32],
) -> Result<DomainRecordOutcome, (StatusCode, Json<ErrorResponse>)> {
    let existing: Option<(
        Vec<u8>,
        String,
        serde_json::Value,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        "SELECT content_sha256, kind, content, created_at FROM domain_records WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| error_reply(StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;

    if let Some((stored_hash, _stored_kind, _stored_content, stored_created_at)) = existing {
        if stored_hash == content_sha256.as_slice() {
            // Same id, same hash → no-op replay (spec §2.3b).
            let hex_hash = hex::encode(content_sha256);
            let response = DomainRecordResponse {
                id,
                kind: kind.to_string(),
                content: content.clone(),
                content_sha256: hex_hash,
                created_at: stored_created_at
                    .format("%Y-%m-%dT%H:%M:%S%.fZ")
                    .to_string(),
                updated_at: stored_created_at
                    .format("%Y-%m-%dT%H:%M:%S%.fZ")
                    .to_string(),
                replayed: true,
            };
            return Ok(DomainRecordOutcome::Replayed { response });
        }
        // Same id, different hash → conflict, never overwrite (spec §2.3b).
        return Ok(DomainRecordOutcome::Conflict { id });
    }

    // New record.
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO domain_records (id, kind, content, content_sha256, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $5)",
    )
    .bind(id)
    .bind(kind)
    .bind(content)
    .bind(content_sha256.as_slice())
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error on insert: {e}"),
        )
    })?;

    let hex_hash = hex::encode(content_sha256);
    let response = DomainRecordResponse {
        id,
        kind: kind.to_string(),
        content: content.clone(),
        content_sha256: hex_hash,
        created_at: now.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        updated_at: now.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
        replayed: false,
    };
    Ok(DomainRecordOutcome::Created { response })
}

/// Create an attempt row **before** work begins.
///
/// Returns the attempt id so the caller can finish it later. A handler crash
/// after this call leaves `outcome` NULL, which the liveness evaluator
/// degrades to UNKNOWN past `expected_duration_s` (spec §2.3, criterion 3).
///
/// # Errors
/// 500 on a database failure.
pub async fn create_attempt(
    pool: &PgPool,
    kind: &str,
    subject: &str,
    expected_duration_s: i32,
) -> Result<i64, (StatusCode, Json<ErrorResponse>)> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO attempts (kind, subject, expected_duration_s) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(kind)
    .bind(subject)
    .bind(expected_duration_s)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error creating attempt: {e}"),
        )
    })?;
    Ok(row.0)
}

/// Finish a previously-created attempt with its outcome and effect breakdown.
///
/// # Errors
/// 500 on a database failure.
pub async fn finish_attempt(
    pool: &PgPool,
    attempt_id: i64,
    outcome: &str,
    effect: &Effect,
    detail: Option<&str>,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    sqlx::query(
        "UPDATE attempts SET finished_at = now(), outcome = $2, effect = $3, detail = $4 \
         WHERE id = $1",
    )
    .bind(attempt_id)
    .bind(outcome)
    .bind(serde_json::to_value(effect).unwrap_or_default())
    .bind(detail)
    .execute(pool)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error finishing attempt: {e}"),
        )
    })?;
    Ok(())
}

/// Compute SHA-256 of the canonical JSON representation of a value.
///
/// Canonicalization is byte-equality of `serde_json::to_vec` output. This is
/// sufficient for a single writer today because there is exactly one client
/// sending each kind of record, and `serde_json` serialization is deterministic
/// for a given input value. If multiple concurrent writers are added, a more
/// robust canonicalization (like JCS) would be needed.
#[must_use]
pub fn content_sha256(content: &serde_json::Value) -> [u8; 32] {
    let bytes = serde_json::to_vec(content).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hasher.finalize().into()
}

type AttemptRow = (
    i64,
    String,
    String,
    chrono::DateTime<chrono::Utc>,
    i32,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<String>,
    Option<serde_json::Value>,
    Option<String>,
);

/// List the most recent attempt rows.
///
/// # Errors
/// 500 on a database failure.
pub async fn list_attempts(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<AttemptView>, (StatusCode, Json<ErrorResponse>)> {
    let rows: Vec<AttemptRow> = sqlx::query_as(
        "SELECT id, kind, subject, started_at, expected_duration_s, \
         finished_at, outcome, effect, detail \
         FROM attempts ORDER BY started_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error listing attempts: {e}"),
        )
    })?;

    let views = rows
        .into_iter()
        .map(
            |(
                id,
                kind,
                subject,
                started_at,
                expected_duration_s,
                finished_at,
                outcome,
                effect,
                detail,
            )| {
                let effect: Option<Effect> = effect.and_then(|v| serde_json::from_value(v).ok());
                AttemptView {
                    id,
                    kind,
                    subject,
                    started_at: started_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
                    expected_duration_s,
                    finished_at: finished_at.map(|t| t.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string()),
                    outcome,
                    effect,
                    detail,
                }
            },
        )
        .collect();

    Ok(views)
}
