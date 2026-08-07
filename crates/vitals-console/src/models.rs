use serde::{Deserialize, Serialize};
use vitals_proto::Liveness;

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

#[derive(Serialize)]
pub struct ErrorResponse {
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

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub client_id: uuid::Uuid,
    /// base64-encoded 32-byte libsodium secretbox key
    pub secret: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub client_id: uuid::Uuid,
    pub status: String,
}

// ---- Envelope ----

#[derive(Deserialize)]
pub struct EnvelopeRequest {
    pub client_id: uuid::Uuid,
    /// base64-encoded 24-byte nonce
    pub nonce: String,
    /// base64-encoded XSalsa20-Poly1305 ciphertext (tag prepended)
    pub payload: String,
}

/// The decrypted envelope payload.
#[derive(Deserialize, Serialize)]
pub struct EnvelopePayload {
    pub class: String,
    pub record: DomainRecordPayload,
}

#[derive(Deserialize, Serialize)]
pub struct DomainRecordPayload {
    pub id: uuid::Uuid,
    pub kind: String,
    pub content: serde_json::Value,
}

/// The response for a processed domain record.
#[derive(Serialize, Deserialize)]
pub struct DomainRecordResponse {
    pub id: uuid::Uuid,
    pub kind: String,
    pub content: serde_json::Value,
    /// hex-encoded SHA-256 of canonical content JSON
    pub content_sha256: String,
    pub created_at: String,
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
    pub created: i32,
    pub updated: i32,
    pub replayed: i32,
    pub rejected: i32,
}

// ---- Attempts ----

#[derive(Serialize)]
pub struct AttemptView {
    pub id: i64,
    pub kind: String,
    pub subject: String,
    pub started_at: String,
    pub expected_duration_s: i32,
    pub finished_at: Option<String>,
    pub outcome: Option<String>,
    pub effect: Option<Effect>,
    pub detail: Option<String>,
}

#[derive(Deserialize)]
pub struct AttemptsQuery {
    pub limit: Option<i64>,
}

// ---- Plaintext test route ----

#[derive(Deserialize)]
pub struct PlaintextTestRequest {
    pub kind: String,
    pub content: serde_json::Value,
}

#[derive(Serialize)]
pub struct PlaintextTestResponse {
    pub status: String,
    pub client_id: uuid::Uuid,
    pub secret: String,
    pub record: Option<DomainRecordResponse>,
    pub error: Option<String>,
}
