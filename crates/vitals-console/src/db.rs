//! Database operations for the vitals-console HTTP service.
//!
//! All queries use `sqlx::query` (runtime-checked), not `sqlx::query!`
//! (compile-time macros), so the build does not need a database.

use axum::Json;
use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain;
use crate::models::{
    AttemptView, DomainRecordPayload, DomainRecordResponse, Effect, ErrorResponse, error_reply,
};

/// Register a client (upsert on `client_id`).
///
/// - New `client_id` → insert, return 201.
/// - Existing `client_id` with matching secret → idempotent 200.
/// - Existing `client_id` with different secret → 409 conflict (takeover
///   attempt refused by name, spec §2.3b).
///
/// # Errors
/// 409 when re-registering an existing `client_id` with a different secret
/// (takeover attempt refused); 500 on database failure.
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
        /// The record as stored, with `replayed: false`.
        response: DomainRecordResponse,
    },
    /// No-op replay — same id, same hash.
    Replayed {
        /// The record as stored, with `replayed: true`.
        response: DomainRecordResponse,
    },
    /// Conflict — same id, different hash.
    Conflict {
        /// The id whose existing content differs from the request.
        id: Uuid,
    },
}

type DomainRecordRow = (
    Vec<u8>,
    String,
    i32,
    serde_json::Value,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
    Option<Uuid>,
    Option<Uuid>,
);

/// Process a domain record, applying immutable version and conflict rules.
///
/// Same identity and hash is an audited no-op replay. Same identity with a
/// different hash is copied to `domain.record_conflicts`; the accepted row is
/// never overwritten. Edits and tombstones are new client identities.
///
/// # Errors
/// 400 when immutable metadata or a lineage target is invalid; 500 on
/// database failure. Idempotency conflicts are returned as an outcome rather
/// than a database error.
#[expect(
    clippy::too_many_lines,
    reason = "the transactional write path keeps validation, conflict capture, audit, and commit together"
)]
pub async fn process_domain_record(
    pool: &PgPool,
    record: &DomainRecordPayload,
    actor: &str,
) -> Result<DomainRecordOutcome, (StatusCode, Json<ErrorResponse>)> {
    let id = record.id;
    let kind = record.kind.as_str();
    let schema_version = record.schema_version;
    let recorded_at = chrono::DateTime::parse_from_rfc3339(&record.recorded_at)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|error| {
            error_reply(
                StatusCode::BAD_REQUEST,
                format!("recorded_at is not RFC3339 UTC: {error}"),
            )
        })?;
    let supersedes_id = record.supersedes_id;
    let tombstone_target_id = record.tombstone_target_id;
    let content = &record.content;
    let content_sha256 = domain::content_sha256(content);
    domain::validate_record_metadata(
        id,
        kind,
        schema_version,
        content,
        supersedes_id,
        tombstone_target_id,
    )
    .map_err(|reason| {
        error_reply(
            StatusCode::BAD_REQUEST,
            format!("domain record rejected: {reason}"),
        )
    })?;
    let db_schema_version = i32::try_from(schema_version).map_err(|_| {
        error_reply(
            StatusCode::BAD_REQUEST,
            format!(
                "domain record schema_version={schema_version} is outside PostgreSQL integer range"
            ),
        )
    })?;

    let mut tx = pool.begin().await.map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error beginning domain record transaction: {e}"),
        )
    })?;

    let existing: Option<DomainRecordRow> = sqlx::query_as(
        "SELECT content_sha256, kind, schema_version, content, recorded_at, created_at, \
                supersedes_id, tombstone_target_id \
           FROM domain.records WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| error_reply(StatusCode::INTERNAL_SERVER_ERROR, format!("db error: {e}")))?;

    if let Some((
        stored_hash,
        stored_kind,
        stored_schema_version,
        stored_content,
        stored_recorded_at,
        stored_created_at,
        stored_supersedes_id,
        stored_tombstone_target_id,
    )) = existing
    {
        if stored_hash == content_sha256.as_slice() {
            sqlx::query(
                "INSERT INTO domain.record_audit (record_id, event_kind, actor, detail) \
                 VALUES ($1, 'replayed', $2, $3)",
            )
            .bind(id)
            .bind(actor)
            .bind(serde_json::json!({"content_sha256": hex::encode(content_sha256)}))
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                error_reply(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("db error auditing replay: {e}"),
                )
            })?;
            tx.commit().await.map_err(|e| {
                error_reply(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("db error committing replay: {e}"),
                )
            })?;
            let response = DomainRecordResponse {
                id,
                kind: stored_kind,
                schema_version: u32::try_from(stored_schema_version).unwrap_or(0),
                recorded_at: stored_recorded_at.to_rfc3339(),
                supersedes_id: stored_supersedes_id,
                tombstone_target_id: stored_tombstone_target_id,
                content: stored_content,
                content_sha256: hex::encode(content_sha256),
                created_at: stored_created_at.to_rfc3339(),
                updated_at: stored_created_at.to_rfc3339(),
                replayed: true,
            };
            return Ok(DomainRecordOutcome::Replayed { response });
        }

        sqlx::query(
            "INSERT INTO domain.record_conflicts \
                (record_id, incoming_kind, incoming_schema_version, incoming_content, \
                 incoming_content_sha256, incoming_supersedes_id, incoming_tombstone_target_id, actor) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (record_id, incoming_content_sha256) DO NOTHING",
        )
        .bind(id)
        .bind(kind)
        .bind(db_schema_version)
        .bind(content)
        .bind(content_sha256.as_slice())
        .bind(supersedes_id)
        .bind(tombstone_target_id)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(|e| error_reply(StatusCode::INTERNAL_SERVER_ERROR, format!("db error recording conflict: {e}")))?;
        let conflict_id: i64 = sqlx::query_scalar(
            "SELECT id FROM domain.record_conflicts \
             WHERE record_id = $1 AND incoming_content_sha256 = $2",
        )
        .bind(id)
        .bind(content_sha256.as_slice())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            error_reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error finding conflict: {e}"),
            )
        })?;
        sqlx::query(
            "INSERT INTO domain.record_audit (record_id, conflict_id, event_kind, actor, detail) \
             VALUES ($1, $2, 'conflict', $3, $4)",
        )
        .bind(id)
        .bind(conflict_id)
        .bind(actor)
        .bind(serde_json::json!({"content_sha256": hex::encode(content_sha256)}))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            error_reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error auditing conflict: {e}"),
            )
        })?;
        tx.commit().await.map_err(|e| {
            error_reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error committing conflict: {e}"),
            )
        })?;
        return Ok(DomainRecordOutcome::Conflict { id });
    }

    for target in [supersedes_id, tombstone_target_id].into_iter().flatten() {
        let present: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM domain.records WHERE id = $1")
                .bind(target)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| {
                    error_reply(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("db error checking record lineage: {e}"),
                    )
                })?;
        if present.is_none() {
            return Err(error_reply(
                StatusCode::BAD_REQUEST,
                format!("domain record {id} references missing lineage target {target}"),
            ));
        }
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO domain.records \
            (id, kind, schema_version, recorded_at, supersedes_id, tombstone_target_id, \
             actor, content, content_sha256, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)",
    )
    .bind(id)
    .bind(kind)
    .bind(db_schema_version)
    .bind(recorded_at)
    .bind(supersedes_id)
    .bind(tombstone_target_id)
    .bind(actor)
    .bind(content)
    .bind(content_sha256.as_slice())
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error on domain record insert: {e}"),
        )
    })?;

    let event_kind = if tombstone_target_id.is_some() {
        "tombstoned"
    } else if supersedes_id.is_some() {
        "versioned"
    } else {
        "created"
    };
    sqlx::query(
        "INSERT INTO domain.record_audit (record_id, event_kind, actor, detail) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(event_kind)
    .bind(actor)
    .bind(serde_json::json!({"content_sha256": hex::encode(content_sha256)}))
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error auditing domain record: {e}"),
        )
    })?;
    tx.commit().await.map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error committing domain record: {e}"),
        )
    })?;

    let response = DomainRecordResponse {
        id,
        kind: kind.to_string(),
        schema_version,
        recorded_at: recorded_at.to_rfc3339(),
        supersedes_id,
        tombstone_target_id,
        content: content.clone(),
        content_sha256: hex::encode(content_sha256),
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
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
/// 500 on database failure.
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
/// 500 on database failure.
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

/// Compute SHA-256 of canonical JSON content for legacy callers.
#[must_use]
pub fn content_sha256(content: &serde_json::Value) -> [u8; 32] {
    domain::content_sha256(content)
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
/// 500 on database failure.
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
