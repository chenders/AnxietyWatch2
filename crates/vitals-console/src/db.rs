//! Database operations for the vitals-console HTTP service.
//!
//! All queries use `sqlx::query` (runtime-checked), not `sqlx::query!`
//! (compile-time macros), so the build does not need a database.

use axum::Json;
use axum::http::StatusCode;
use base64::Engine;
use sha2::Digest;
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::client_api::HealthBeaconRequest;
use crate::credentials::{
    CREDENTIAL_LIFETIME_S, DeviceCredential, fingerprint_public_key, hash_enrollment_token,
    validate_mobile_label, validate_mobile_public_key, validate_mobile_scopes,
};
use crate::domain;
use crate::models::{
    AttemptView, DomainRecordPayload, DomainRecordResponse, Effect, ErrorResponse, error_reply,
};

/// Return the most recent received status beacon for the Ingest health projection.
///
/// The receive time, rather than the client clock, is the freshness evidence used by the core
/// liveness policy.
///
/// # Errors
///
/// Returns the database or stored type-facts decoding error.
pub async fn latest_health_beacon(
    pool: &PgPool,
) -> Result<
    Option<(
        chrono::DateTime<chrono::Utc>,
        Vec<crate::client_api::HealthTypeFact>,
    )>,
    sqlx::Error,
> {
    let row: Option<(chrono::DateTime<chrono::Utc>, serde_json::Value)> = sqlx::query_as(
        "SELECT received_at, type_facts FROM mobile_status_beacons ORDER BY received_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    row.map(|(received_at, type_facts)| {
        serde_json::from_value(type_facts)
            .map(|facts| (received_at, facts))
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))
    })
    .transpose()
}

/// Store a status beacon without mixing operational facts into physiological samples.
///
/// The beacon id is client-scoped and content-addressed. Repeating the same beacon is a no-op;
/// reusing its id for different content is a named conflict.
///
/// # Errors
///
/// Returns a persistence or timestamp conversion error.
pub async fn store_health_beacon(
    pool: &PgPool,
    client_id: Uuid,
    request: &HealthBeaconRequest,
    body: &[u8],
) -> Result<String, String> {
    let digest = sha2::Sha256::digest(body);
    let existing: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT body_sha256 FROM mobile_status_beacons WHERE client_id = $1 AND beacon_id = $2",
    )
    .bind(client_id)
    .bind(request.beacon_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| format!("status beacon lookup failed: {error}"))?;
    if let Some(existing) = existing {
        if existing.as_slice() == digest.as_slice() {
            return Ok("replayed".to_string());
        }
        return Err(format!(
            "status beacon {} conflicts with previously stored content",
            request.beacon_id
        ));
    }

    let asserted_at = chrono::DateTime::parse_from_rfc3339(&request.asserted_at)
        .map_err(|error| format!("status asserted_at conversion failed: {error}"))?
        .with_timezone(&chrono::Utc);
    let changed_at = chrono::DateTime::parse_from_rfc3339(&request.changed_at)
        .map_err(|error| format!("status changed_at conversion failed: {error}"))?
        .with_timezone(&chrono::Utc);
    let last_server_ack_at = request
        .last_server_ack_at
        .as_deref()
        .map(chrono::DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|error| format!("status last_server_ack_at conversion failed: {error}"))?
        .map(|instant| instant.with_timezone(&chrono::Utc));
    let outbox_count = i64::try_from(request.outbox_count).map_err(|_| {
        format!(
            "status outbox_count={} exceeds BIGINT",
            request.outbox_count
        )
    })?;
    let oldest_outbox_age_s = request
        .oldest_outbox_age_s
        .map(i64::try_from)
        .transpose()
        .map_err(|_| "status oldest_outbox_age_s exceeds BIGINT".to_string())?;
    let types = serde_json::to_value(&request.types)
        .map_err(|error| format!("status type facts serialization failed: {error}"))?;
    sqlx::query(
        "INSERT INTO mobile_status_beacons \
         (client_id, beacon_id, body_sha256, asserted_at, changed_at, app_version, registry_sha256,\
          outbox_count, oldest_outbox_age_s, last_server_ack_at, type_facts)\
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(client_id)
    .bind(request.beacon_id)
    .bind(digest.as_slice())
    .bind(asserted_at)
    .bind(changed_at)
    .bind(&request.app_version)
    .bind(&request.registry_sha256)
    .bind(outbox_count)
    .bind(oldest_outbox_age_s)
    .bind(last_server_ack_at)
    .bind(types)
    .execute(pool)
    .await
    .map_err(|error| format!("status beacon insert failed: {error}"))?;
    Ok("accepted".to_string())
}

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

type EnrollmentArtifactRow = (
    Uuid,
    Vec<u8>,
    Vec<String>,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
);

/// Plaintext enrollment artifact returned only to the authenticated creator once.
pub struct MobileEnrollmentArtifact {
    /// Single-use enrollment identifier.
    pub enrollment_id: Uuid,
    /// Server identity bound into the artifact.
    pub server_instance_id: Uuid,
    /// Artifact expiry in UTC.
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Allowlisted device scopes.
    pub scopes: Vec<String>,
    /// Opaque token that is never stored in plaintext.
    pub token: String,
}

/// Create a ten-minute enrollment artifact and persist only its token hash.
///
/// # Errors
///
/// Returns a validation, live-artifact limit, random-token, or database error.
pub async fn create_mobile_enrollment_artifact(
    pool: &PgPool,
    server_instance_id: Uuid,
    label: &str,
    scopes: &[String],
    actor: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<MobileEnrollmentArtifact, String> {
    validate_mobile_label(label)?;
    let scope_refs: Vec<&str> = scopes.iter().map(String::as_str).collect();
    let scopes = validate_mobile_scopes(&scope_refs)?;
    let mut token_bytes = [0_u8; 32];
    getrandom::getrandom(&mut token_bytes)
        .map_err(|error| format!("enrollment token generation failed: {error}"))?;
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes);
    let expires_at = now
        .checked_add_signed(chrono::Duration::seconds(
            crate::credentials::ENROLLMENT_LIFETIME_S,
        ))
        .ok_or_else(|| {
            "enrollment artifact creation blocked: expiry overflowed UTC range".to_string()
        })?;
    let enrollment_id = Uuid::new_v4();
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| format!("enrollment artifact transaction failed: {error}"))?;
    let live_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM enrollment_artifacts WHERE consumed_at IS NULL AND expires_at > $1",
    )
    .bind(now)
    .fetch_one(&mut *transaction)
    .await
    .map_err(|error| format!("enrollment artifact limit lookup failed: {error}"))?;
    if live_count >= 3 {
        return Err(format!(
            "enrollment artifact creation blocked: live_artifact_count={live_count}; maximum=3"
        ));
    }
    sqlx::query(
        "INSERT INTO enrollment_artifacts\
         (enrollment_id, server_instance_id, token_hash, scopes, expires_at, created_by)\
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(enrollment_id)
    .bind(server_instance_id)
    .bind(hash_enrollment_token(&token).as_slice())
    .bind(&scopes)
    .bind(expires_at)
    .bind(actor)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("enrollment artifact insert failed: {error}"))?;
    sqlx::query(
        "INSERT INTO credential_audit (action, actor, reason)\
         VALUES ('enrollment_created', $1, $2)",
    )
    .bind(actor)
    .bind(format!("enrollment_id={enrollment_id}; label={label}"))
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("enrollment artifact audit failed: {error}"))?;
    transaction
        .commit()
        .await
        .map_err(|error| format!("enrollment artifact commit failed: {error}"))?;
    Ok(MobileEnrollmentArtifact {
        enrollment_id,
        server_instance_id,
        expires_at,
        scopes,
        token,
    })
}

/// Claim a single-use enrollment artifact and persist a public-key credential atomically.
///
/// The caller supplies the uncompressed SEC1 public key decoded from the phone's SPKI value.
/// `PostgreSQL` stores only the token hash and public key; the plaintext token is never persisted.
///
/// # Errors
///
/// Returns a named validation, artifact-state, or database failure. The transaction consumes the
/// artifact only if the credential insert and audit row both succeed.
pub async fn claim_mobile_credential(
    pool: &PgPool,
    enrollment_id: Uuid,
    token: &str,
    server_instance_id: Uuid,
    public_key: &[u8],
    label: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<DeviceCredential, String> {
    validate_mobile_label(label)?;
    validate_mobile_public_key(public_key)?;
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| format!("mobile enrollment database transaction failed: {error}"))?;
    let artifact: Option<EnrollmentArtifactRow> = sqlx::query_as(
        "SELECT server_instance_id, token_hash, scopes, expires_at, consumed_at\
         FROM enrollment_artifacts WHERE enrollment_id = $1 FOR UPDATE",
    )
    .bind(enrollment_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| format!("mobile enrollment artifact lookup failed: {error}"))?;
    let Some((artifact_server_id, token_hash, scopes, expires_at, consumed_at)) = artifact else {
        return Err(format!(
            "enrollment blocked: enrollment_id={enrollment_id} is unknown"
        ));
    };
    if artifact_server_id != server_instance_id {
        return Err(format!(
            "enrollment blocked: server_instance_id={server_instance_id} does not match artifact"
        ));
    }
    if expires_at <= now {
        return Err(format!(
            "enrollment blocked: enrollment_id={enrollment_id} expired_at={expires_at}"
        ));
    }
    if consumed_at.is_some() {
        return Err(format!(
            "enrollment blocked: enrollment_id={enrollment_id} was already consumed"
        ));
    }
    let expected_hash = hash_enrollment_token(token);
    if !bool::from(token_hash.as_slice().ct_eq(expected_hash.as_slice())) {
        return Err(format!(
            "enrollment blocked: enrollment_id={enrollment_id} token did not match"
        ));
    }
    let scope_refs: Vec<&str> = scopes.iter().map(String::as_str).collect();
    let artifact_scopes = validate_mobile_scopes(&scope_refs)?;
    let client_id = Uuid::new_v4();
    let key_id = Uuid::new_v4();
    let expires_at = now
        .checked_add_signed(chrono::Duration::seconds(CREDENTIAL_LIFETIME_S))
        .ok_or_else(|| "enrollment blocked: credential expiry overflowed UTC range".to_string())?;
    sqlx::query(
        "INSERT INTO mobile_credentials\
         (client_id, key_id, label, public_key, public_key_fingerprint, scopes, issued_at, expires_at)\
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(client_id)
    .bind(key_id)
    .bind(label)
    .bind(public_key)
    .bind(fingerprint_public_key(public_key))
    .bind(&artifact_scopes)
    .bind(now)
    .bind(expires_at)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("mobile credential insert failed: {error}"))?;
    sqlx::query(
        "UPDATE enrollment_artifacts SET consumed_at = $2 WHERE enrollment_id = $1 AND consumed_at IS NULL",
    )
    .bind(enrollment_id)
    .bind(now)
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("mobile enrollment consume failed: {error}"))?;
    sqlx::query(
        "INSERT INTO credential_audit (client_id, key_id, action, actor, reason)\
         VALUES ($1, $2, 'enrolled', $3, $4)",
    )
    .bind(client_id)
    .bind(key_id)
    .bind(format!("enrollment:{enrollment_id}"))
    .bind("mobile public-key enrollment")
    .execute(&mut *transaction)
    .await
    .map_err(|error| format!("mobile credential audit insert failed: {error}"))?;
    transaction
        .commit()
        .await
        .map_err(|error| format!("mobile enrollment commit failed: {error}"))?;
    Ok(DeviceCredential {
        client_id,
        key_id,
        label: label.to_string(),
        public_key_fingerprint: fingerprint_public_key(public_key),
        scopes: artifact_scopes,
        issued_at_epoch_s: now.timestamp(),
        expires_at_epoch_s: expires_at.timestamp(),
        last_used_at_epoch_s: None,
        revoked_at_epoch_s: None,
        overlap_until_epoch_s: None,
    })
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

/// Process one complete `HealthKit` anchored page in one database transaction.
///
/// All samples are validated before the transaction starts. A duplicate page is a replay when no
/// sample or deletion tombstone was newly inserted; `PostgreSQL` identity constraints make retries
/// safe without using a timestamp as identity.
///
/// # Errors
///
/// Returns a named 400 quarantine reason for invalid platform data, or a 500 persistence error.
#[expect(
    clippy::too_many_lines,
    reason = "the transactional HealthKit ingest path keeps validation and append-only commit together"
)]
pub async fn process_healthkit_batch(
    pool: &PgPool,
    payload: &crate::healthkit::HealthKitBatchPayload,
    client_id: Uuid,
) -> Result<bool, (StatusCode, Json<ErrorResponse>)> {
    if payload.class != "platform_sample_batch" {
        return Err(error_reply(
            StatusCode::BAD_REQUEST,
            format!("healthkit batch class is not supported: {}", payload.class),
        ));
    }
    if payload.schema_version != 1 {
        return Err(error_reply(
            StatusCode::BAD_REQUEST,
            format!(
                "healthkit batch schema_version is unsupported: {}",
                payload.schema_version
            ),
        ));
    }
    if payload.samples.is_empty() && payload.deletions.is_empty() {
        return Err(error_reply(
            StatusCode::BAD_REQUEST,
            "healthkit batch contains no samples or deletion facts",
        ));
    }

    let mut mapped_samples = Vec::with_capacity(payload.samples.len());
    for sample in &payload.samples {
        let mapped = crate::healthkit::map_healthkit_sample(sample).map_err(|quarantine| {
            error_reply(
                StatusCode::BAD_REQUEST,
                format!(
                    "HealthKit sample quarantined: {} ({})",
                    quarantine.code, quarantine.detail
                ),
            )
        })?;
        let timestamp = chrono::DateTime::from_timestamp_micros(mapped.timestamp_unix_micros)
            .ok_or_else(|| {
                error_reply(
                    StatusCode::BAD_REQUEST,
                    format!(
                        "HealthKit sample timestamp is outside PostgreSQL range: {}",
                        sample.start_at
                    ),
                )
            })?;
        mapped_samples.push((mapped, timestamp));
    }

    let mut mapped_deletions = Vec::with_capacity(payload.deletions.len());
    for deletion in &payload.deletions {
        let registry_type =
            crate::healthkit::registry_type_for_healthkit_identifier(&deletion.healthkit_type)
                .ok_or_else(|| {
                    error_reply(
                        StatusCode::BAD_REQUEST,
                        format!(
                            "HealthKit deletion type is unsupported: {}",
                            deletion.healthkit_type
                        ),
                    )
                })?;
        let recorded_at = chrono::DateTime::parse_from_rfc3339(&deletion.recorded_at)
            .map(|value| value.with_timezone(&chrono::Utc))
            .map_err(|error| {
                error_reply(
                    StatusCode::BAD_REQUEST,
                    format!("HealthKit deletion recorded_at is not RFC3339: {error}"),
                )
            })?;
        let extra = serde_json::to_vec(&serde_json::json!({
            "schema": 1,
            "platform": "healthkit",
            "sample_id": deletion.healthkit_sample_uuid,
            "healthkit_type": deletion.healthkit_type,
            "deletion": true,
        }))
        .map_err(|error| {
            error_reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("HealthKit deletion provenance encoding failed: {error}"),
            )
        })?;
        mapped_deletions.push((deletion, registry_type, recorded_at, extra));
    }

    let mut tx = pool.begin().await.map_err(|error| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error beginning HealthKit batch transaction: {error}"),
        )
    })?;
    let node_id = client_id.as_bytes().to_vec();
    let mut inserted = false;
    for (sample, timestamp) in mapped_samples {
        let result = sqlx::query(
            "INSERT INTO samples (source, type, session_id, seq, timestamp, value, extra, node_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
        )
        .bind(sample.source.id())
        .bind(sample.r#type.id())
        .bind(sample.session_id)
        .bind(sample.seq)
        .bind(timestamp)
        .bind(sample.value)
        .bind(sample.extra)
        .bind(&node_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            error_reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error inserting HealthKit sample: {error}"),
            )
        })?;
        inserted |= result.rows_affected() == 1;
    }
    for (deletion, registry_type, recorded_at, extra) in mapped_deletions {
        let result = sqlx::query(
            "INSERT INTO healthkit_sample_tombstones \
             (healthkit_sample_uuid, healthkit_type, registry_type, deleted_at, query_batch_id, recorded_at, node_id, extra) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
        )
        .bind(deletion.healthkit_sample_uuid)
        .bind(&deletion.healthkit_type)
        .bind(registry_type.id())
        .bind(recorded_at)
        .bind(deletion.query_batch_id)
        .bind(recorded_at)
        .bind(&node_id)
        .bind(extra)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            error_reply(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("db error inserting HealthKit deletion tombstone: {error}"),
            )
        })?;
        inserted |= result.rows_affected() == 1;
    }
    tx.commit().await.map_err(|error| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error committing HealthKit batch: {error}"),
        )
    })?;
    Ok(inserted)
}

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

/// Values recorded for one immutable service-operation audit event.
pub struct ServiceOperationAudit<'a> {
    /// Authenticated actor label.
    pub actor: &'a str,
    /// Exact deployment-manifest unit.
    pub target: &'a str,
    /// Allowlisted operation.
    pub action: &'a str,
    /// UI confirmation class.
    pub confirmation_class: &'a str,
    /// Linked attempt id.
    pub attempt_id: i64,
    /// Final operation outcome.
    pub outcome: &'a str,
    /// Sanitised executor detail.
    pub detail: Option<&'a str>,
}

/// Finish the operation attempt and audit it in one database transaction.
///
/// The external `systemd` call cannot be made atomic with `PostgreSQL`, but once
/// it returns this keeps the attempt and immutable audit evidence together.
///
/// # Errors
///
/// 500 on database failure or when the attempt id is not exactly one row.
pub async fn finish_service_operation_attempt(
    pool: &PgPool,
    audit: &ServiceOperationAudit<'_>,
    effect: &Effect,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let mut tx = pool.begin().await.map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error beginning service-operation finalization: {e}"),
        )
    })?;
    let updated = sqlx::query(
        "UPDATE attempts SET finished_at = now(), outcome = $2, effect = $3, detail = $4 \
         WHERE id = $1 AND outcome IS NULL",
    )
    .bind(audit.attempt_id)
    .bind(audit.outcome)
    .bind(serde_json::to_value(effect).unwrap_or_default())
    .bind(audit.detail)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error finishing service-operation attempt: {e}"),
        )
    })?;
    if updated.rows_affected() != 1 {
        return Err(error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "service-operation audit blocked: attempt_id={} was not open",
                audit.attempt_id
            ),
        ));
    }
    sqlx::query(
        "INSERT INTO service_operation_audit \
         (actor, target, action, confirmation_class, attempt_id, outcome, detail) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(audit.actor)
    .bind(audit.target)
    .bind(audit.action)
    .bind(audit.confirmation_class)
    .bind(audit.attempt_id)
    .bind(audit.outcome)
    .bind(audit.detail)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error recording service-operation audit: {e}"),
        )
    })?;
    tx.commit().await.map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("db error committing service-operation audit: {e}"),
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

/// Health config and mute info for a source.
pub struct SourceConfigAndMute {
    /// Whether the source is configured.
    pub is_configured: bool,
    /// The timestamp until which the source is muted.
    pub muted_until: Option<chrono::DateTime<chrono::Utc>>,
    /// The actor who muted the source.
    pub muted_by: Option<String>,
}

/// Fetch config and active mutes for all sources.
///
/// # Errors
/// Returns `sqlx::Error` on query failure.
pub async fn get_config_and_mutes(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, SourceConfigAndMute>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT s.id as source_id,
                sd.is_configured,
                hm.muted_until,
                hm.muted_by
         FROM vitals_source s
         LEFT JOIN source_declarations sd ON s.id = sd.source_id
         LEFT JOIN (
             SELECT source_id, muted_until, muted_by
             FROM health_mutes
             WHERE muted_until > NOW()
             ORDER BY muted_until DESC
         ) hm ON s.id = hm.source_id",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::new();
    for row in rows {
        let is_configured: Option<bool> = row.try_get("is_configured").unwrap_or(None);
        map.insert(
            row.try_get("source_id")?,
            SourceConfigAndMute {
                is_configured: is_configured.unwrap_or(true),
                muted_until: row.try_get("muted_until").unwrap_or(None),
                muted_by: row.try_get("muted_by").unwrap_or(None),
            },
        );
    }
    Ok(map)
}
