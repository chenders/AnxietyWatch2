use serde::{Deserialize, Serialize};
use vitals_proto::Liveness;

// ---- Existing types (kept) ----

/// One source's health as rendered by `/api/health`.
#[derive(Serialize)]
pub struct SourceHealthView {
    /// Registry source id.
    pub id: i32,
    /// Source name, as the registry enum's debug name.
    pub name: String,
    /// Derived health state: `Live`, `Failing`, or `Unknown`.
    pub state: String,
    /// Why the state is what it is, naming the value and its threshold.
    pub reason: String,
    /// Unix epoch seconds by which the source must recover or be degraded.
    pub degrade_deadline_epoch_s: Option<i64>,
    /// Seconds since the last sample arrived, or `None` if never seen.
    pub observation_age_s: Option<u64>,
    /// The daemon's own liveness verdict, before local re-evaluation.
    pub raw_liveness: Option<Liveness>,
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
    /// Value of `GIT_DESCRIBE` at build time, when set.
    pub git: Option<&'static str>,
    /// Milestone note on what this build ships.
    pub note: &'static str,
}

// ---- Error response ----

/// A JSON error body returned with a non-2xx status.
#[derive(Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message naming the blocker.
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
    /// The client's chosen id.
    pub client_id: uuid::Uuid,
    /// base64-encoded 32-byte libsodium secretbox key
    pub secret: String,
}

/// Response body of a successful registration.
#[derive(Serialize)]
pub struct RegisterResponse {
    /// The registered client id.
    pub client_id: uuid::Uuid,
    /// `registered` or `already_registered`.
    pub status: String,
}

// ---- Envelope ----

/// Body of `POST /api/envelope`: an encrypted domain record.
#[derive(Deserialize)]
pub struct EnvelopeRequest {
    /// The registered client's id.
    pub client_id: uuid::Uuid,
    /// base64-encoded 24-byte nonce
    pub nonce: String,
    /// base64-encoded XSalsa20-Poly1305 ciphertext (tag prepended)
    pub payload: String,
}

/// The decrypted envelope payload.
#[derive(Deserialize, Serialize)]
pub struct EnvelopePayload {
    /// Envelope class; only `domain_record` is accepted in M0.
    pub class: String,
    /// The domain record being written.
    pub record: DomainRecordPayload,
}

/// A domain record as carried inside an envelope.
#[derive(Deserialize, Serialize)]
pub struct DomainRecordPayload {
    /// Stable record id; the idempotency key.
    pub id: uuid::Uuid,
    /// Record kind, e.g. `test_entry`.
    pub kind: String,
    /// Free-form record content.
    pub content: serde_json::Value,
}

/// The response for a processed domain record.
#[derive(Serialize, Deserialize)]
pub struct DomainRecordResponse {
    /// The record id.
    pub id: uuid::Uuid,
    /// The record kind.
    pub kind: String,
    /// The record content as stored.
    pub content: serde_json::Value,
    /// hex-encoded SHA-256 of canonical content JSON
    pub content_sha256: String,
    /// Created timestamp, RFC 3339 with millisecond precision.
    pub created_at: String,
    /// Last-updated timestamp, RFC 3339 with millisecond precision.
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
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Effect {
    /// Records created by this attempt.
    pub created: i32,
    /// Records updated by this attempt.
    pub updated: i32,
    /// Records that were no-op replays.
    pub replayed: i32,
    /// Records rejected by this attempt.
    pub rejected: i32,
}

// ---- Attempts ----

/// One attempt row in `/api/attempts`.
#[derive(Serialize)]
pub struct AttemptView {
    /// Attempt id, the `attempts.id` row.
    pub id: i64,
    /// Kind of work attempted, e.g. `envelope`.
    pub kind: String,
    /// What the attempt acted on, e.g. the client id.
    pub subject: String,
    /// When work started, RFC 3339.
    pub started_at: String,
    /// Seconds after which an unfinished attempt degrades to UNKNOWN.
    pub expected_duration_s: i32,
    /// When work finished; `None` if still in flight.
    pub finished_at: Option<String>,
    /// `succeeded` or `failed`.
    pub outcome: Option<String>,
    /// What the attempt actually did (created/updated/replayed/rejected).
    pub effect: Option<Effect>,
    /// Error detail when the attempt failed.
    pub detail: Option<String>,
}

/// Query parameters for `GET /api/attempts`.
#[derive(Deserialize)]
pub struct AttemptsQuery {
    /// Maximum number of rows to return; defaults to 50, capped at 500.
    pub limit: Option<i64>,
}

// ---- Plaintext test route ----

/// Body of `POST /api/envelope/plaintext-test`.
#[derive(Deserialize)]
pub struct PlaintextTestRequest {
    /// Record kind for the generated domain record.
    pub kind: String,
    /// Record content for the generated domain record.
    pub content: serde_json::Value,
}

/// Response body of the plaintext test route.
#[derive(Serialize)]
pub struct PlaintextTestResponse {
    /// `ok`, `register_failed`, or `failed (status)`.
    pub status: String,
    /// The auto-generated client id.
    pub client_id: uuid::Uuid,
    /// The auto-generated secret, base64-encoded.
    pub secret: String,
    /// The processed record, when processing succeeded.
    pub record: Option<DomainRecordResponse>,
    /// Error detail, when processing failed.
    pub error: Option<String>,
}
