use serde::{Deserialize, Serialize};
use vitals_proto::Liveness;

// ---- Existing types (kept) ----

#[derive(Serialize)]
pub struct SourceHealthView {
    pub id: i32,
    pub name: String,
    pub state: String,
    pub reason: String,
    pub degrade_deadline_epoch_s: Option<i64>,
    pub observation_age_s: Option<u64>,
    pub raw_liveness: Option<Liveness>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonReachable {
    Reachable,
    Unreachable(String),
}

#[derive(Serialize)]
pub struct HealthView {
    pub vitalsd: DaemonReachable,
    pub uptime: Option<u64>,
    pub degraded: Vec<String>,
    pub heartbeat_pulsing: Option<bool>,
    pub sources: Vec<SourceHealthView>,
}

#[derive(Serialize)]
pub struct VersionView {
    pub version: &'static str,
    pub git: Option<&'static str>,
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
