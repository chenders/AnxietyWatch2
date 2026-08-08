use serde::{Deserialize, Serialize};
use vitals_proto::Liveness;

/// One source's health as rendered by `/api/health`.
#[derive(Serialize)]
pub struct SourceHealthView {
    /// Registry source id.
    pub id: i32,
    /// Source name, as the registry enum's debug name.
    pub name: String,
    /// Derived health state: `Live`, `Failing`, `Unknown`, or `NOT_CONFIGURED`.
    pub state: String,
    /// Why the state is what it is, naming the value and its threshold.
    pub reason: String,
    /// Unix epoch seconds by which the source must recover or be degraded.
    pub degrade_deadline_epoch_s: Option<i64>,
    /// Seconds since the last sample arrived, or `None` if never seen.
    pub observation_age_s: Option<u64>,
    /// The daemon's own liveness verdict, before local re-evaluation.
    pub raw_liveness: Option<Liveness>,
    /// Whether this source is expected to exist.
    pub is_configured: bool,
    /// If muted, the ISO 8601 expiry timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted_until: Option<String>,
    /// If muted, who muted it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted_by: Option<String>,
}

/// Whether the `vitalsd` daemon answered the health request.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonReachable {
    /// The daemon answered; per-source health follows.
    Reachable,
    /// The daemon did not answer; the string names why.
    Unreachable(String),
}

/// The `/api/health` response body.
#[derive(Serialize)]
pub struct HealthView {
    /// Whether `vitalsd` itself answered.
    pub vitalsd: DaemonReachable,
    /// Daemon uptime in seconds, when reachable.
    pub uptime: Option<u64>,
    /// Named degradations reported by the daemon.
    pub degraded: Vec<String>,
    /// Whether the hardware dead-man's-switch heartbeat is pulsing.
    pub heartbeat_pulsing: Option<bool>,
    /// One entry per registry source.
    pub sources: Vec<SourceHealthView>,
}

/// The `/api/version` response body.
#[derive(Serialize)]
pub struct VersionView {
    /// Crate version from `Cargo.toml`.
    pub version: &'static str,
    /// Exact Git commit embedded by `build.rs`, or `UNKNOWN` if Git was unavailable.
    pub git_commit: &'static str,
    /// Build time as UTC epoch seconds, or `UNKNOWN` if the build clock failed.
    pub build_epoch_s: &'static str,
    /// Sqitch's currently deployed change id, when measured from `PostgreSQL`.
    pub schema_revision: Option<String>,
    /// Number of planned Sqitch changes absent from the deployed schema.
    pub pending_migrations: Option<u64>,
    /// Why schema identity is absent, when it could not be measured.
    pub schema_unmeasured_reason: Option<String>,
}

// ---- Error response ----

/// JSON error body returned on non-2xx responses.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Human-readable reason, naming the value that blocked the action.
    pub error: String,
}

/// Helper: construct a status-code + JSON error response.
pub fn error_reply(
    status: axum::http::StatusCode,
    msg: impl Into<String>,
) -> (axum::http::StatusCode, axum::Json<ErrorResponse>) {
    (status, axum::Json(ErrorResponse { error: msg.into() }))
}

// ---- Registration ----

/// Body of `POST /api/envelope/register`.
#[derive(Deserialize)]
pub struct RegisterRequest {
    /// Client identifier to register or re-register.
    pub client_id: uuid::Uuid,
    /// base64-encoded 32-byte libsodium secretbox key
    pub secret: String,
}

/// Successful `POST /api/envelope/register` response body.
#[derive(Serialize)]
pub struct RegisterResponse {
    /// The registered client identifier.
    pub client_id: uuid::Uuid,
    /// `registered` (new) or `already_registered` (idempotent).
    pub status: String,
}

// ---- Envelope ----

/// Body of `POST /api/envelope`.
#[derive(Deserialize)]
pub struct EnvelopeRequest {
    /// Client identifier whose registered secret decrypts `payload`.
    pub client_id: uuid::Uuid,
    /// base64-encoded 24-byte nonce
    pub nonce: String,
    /// base64-encoded XSalsa20-Poly1305 ciphertext (tag prepended)
    pub payload: String,
}

/// The decrypted envelope payload.
#[derive(Deserialize, Serialize)]
pub struct EnvelopePayload {
    /// Envelope class; only `domain_record` is supported in M0.
    pub class: String,
    /// The domain record carried by this envelope.
    pub record: DomainRecordPayload,
}

/// A domain record inside an envelope, before idempotency processing.
#[derive(Deserialize, Serialize)]
pub struct DomainRecordPayload {
    /// Client-assigned record id; this identity is never reused for edits.
    pub id: uuid::Uuid,
    /// Record kind, e.g. `journal_entry` or `song_occurrence`.
    pub kind: String,
    /// Version of the kind-specific content contract.
    pub schema_version: u32,
    /// iPhone capture time, never a server receipt-time substitute.
    pub recorded_at: String,
    /// New client identity that edits an earlier record.
    pub supersedes_id: Option<uuid::Uuid>,
    /// New client identity that tombstones an earlier record.
    pub tombstone_target_id: Option<uuid::Uuid>,
    /// Record payload as arbitrary JSON.
    pub content: serde_json::Value,
}

/// The response for a processed domain record.
#[derive(Serialize, Deserialize)]
pub struct DomainRecordResponse {
    /// Record id as stored.
    pub id: uuid::Uuid,
    /// Record kind as stored.
    pub kind: String,
    /// Version of the kind-specific content contract.
    pub schema_version: u32,
    /// iPhone capture time stored with the record.
    pub recorded_at: String,
    /// New client identity that edits an earlier record.
    pub supersedes_id: Option<uuid::Uuid>,
    /// New client identity that tombstones an earlier record.
    pub tombstone_target_id: Option<uuid::Uuid>,
    /// Record payload as stored.
    pub content: serde_json::Value,
    /// hex-encoded SHA-256 of canonical content JSON
    pub content_sha256: String,
    /// `YYYY-MM-DDTHH:MM:SS.fffZ` creation timestamp.
    pub created_at: String,
    /// Compatibility receipt timestamp; accepted records are never updated.
    pub updated_at: String,
    /// true if this request was a no-op replay
    pub replayed: bool,
}

// ---- Effect breakdown ----

/// Breakdown of what an envelope attempt actually did.
///
/// This is stored as JSONB in the attempts.effect column and returned in
/// attempt views so that "the sync succeeded" is not mistaken for progress
/// when every record was a no-op replay (spec §2.3b).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Effect {
    /// Records newly created by this attempt.
    pub created: i32,
    /// Records updated by this attempt.
    pub updated: i32,
    /// Records that were no-op replays of existing content.
    pub replayed: i32,
    /// Records rejected (e.g. idempotency conflicts) by this attempt.
    pub rejected: i32,
}

// ---- Attempts ----

/// One attempt row as rendered by `GET /api/attempts`.
#[derive(Serialize)]
pub struct AttemptView {
    /// Attempt id from the `attempts` table.
    pub id: i64,
    /// Attempt kind, e.g. `envelope`.
    pub kind: String,
    /// Subject of the attempt, e.g. the client id.
    pub subject: String,
    /// `YYYY-MM-DDTHH:MM:SS.fffZ` start time.
    pub started_at: String,
    /// Expected duration in seconds; a `NULL` outcome past this time
    /// degrades to UNKNOWN, never to success.
    pub expected_duration_s: i32,
    /// `YYYY-MM-DDTHH:MM:SS.fffZ` finish time, when finished.
    pub finished_at: Option<String>,
    /// `succeeded` or `failed`, when finished.
    pub outcome: Option<String>,
    /// Effect breakdown, when finished.
    pub effect: Option<Effect>,
    /// Free-form failure detail, when failed.
    pub detail: Option<String>,
}

/// Query parameters for `GET /api/attempts`.
#[derive(Deserialize)]
pub struct AttemptsQuery {
    /// Maximum number of rows to return; clamped to `1..=500`.
    pub limit: Option<i64>,
}

// ---- Plaintext test route ----

/// Body of `POST /api/envelope/plaintext-test`.
#[derive(Deserialize)]
pub struct PlaintextTestRequest {
    /// Record kind for the generated domain record.
    pub kind: String,
    /// Record payload as arbitrary JSON.
    pub content: serde_json::Value,
}

/// Response of `POST /api/envelope/plaintext-test`.
#[derive(Serialize)]
pub struct PlaintextTestResponse {
    /// `ok`, `failed`, or `register_failed`.
    pub status: String,
    /// The auto-generated client id.
    pub client_id: uuid::Uuid,
    /// The auto-generated base64-encoded secret, so the page can reuse it.
    pub secret: String,
    /// The processed record, when the envelope path succeeded.
    pub record: Option<DomainRecordResponse>,
    /// Failure detail, when the request did not fully succeed.
    pub error: Option<String>,
}
