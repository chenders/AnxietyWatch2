use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::{Json, response::Html};
use base64::Engine;
use chrono::Utc;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use vitals_core::liveness::{Evidence, evaluate};
use vitals_registry::{ALL_SOURCES, VitalsSource};

use crate::client_api::HealthTypeFact;

use crate::client::get_health;
use crate::crypto;
use crate::db;
use crate::models::{
    AttemptsQuery, DaemonReachable, DomainRecordResponse, Effect, EnvelopePayload, EnvelopeRequest,
    ErrorResponse, HealthView, PlaintextTestRequest, PlaintextTestResponse, RegisterRequest,
    SourceHealthView, VersionView, error_reply,
};
use crate::test_page::TEST_PAGE_HTML;

/// Dashboard HTML embedded from the design artifact so deployment remains a
/// single binary and every API request is same-origin.
const DASHBOARD_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/design/web-dashboard/index.html"
));

/// Shared application state for all console routes.
#[derive(Clone)]
pub struct AppState {
    /// Path to the `vitalsd` Unix socket.
    pub socket_path: PathBuf,
    /// Write-API pool (envelope, attempts). `None` serves reads only.
    pub db: Option<sqlx::PgPool>,
    /// Whether the guarded plaintext test routes are mounted.
    pub test_routes_enabled: bool,
    /// `PostgreSQL` connection URL in libpq format, used by `/api/recovery`.
    pub db_url: String,
    /// Injected system operations for the host-status collectors.
    pub sys_ops: crate::api::ops::SysOps,
    /// Bounded host history used for disk projections.
    pub host_history: crate::api::host::HostHistory,
    /// `PostgreSQL` access for the read API, injected for testability.
    pub db_ops: std::sync::Arc<dyn crate::api::postgres::DbOps>,
    /// Operator authentication state; mobile client routes use a separate protocol.
    pub security: std::sync::Arc<crate::security::SecurityState>,
    /// Second capability required for destructive service operations.
    pub ops_token: Option<String>,
    /// Stable server identity included in enrollment artifacts.
    pub server_instance_id: Option<Uuid>,
    /// Manually configured first-draft client origin included in enrollment artifacts.
    pub client_origin: Option<String>,
    /// Operator-visible server label included in enrollment artifacts.
    pub server_display_label: String,
    /// Whether the fixed `AirSense` admin actions are enabled by policy.
    pub air11_actions_enabled: bool,
    /// Per-process CSRF capability for fixed `AirSense` actions.
    pub air11_action_token: Option<std::sync::Arc<String>>,
    /// External console-admin capability; never sent to the browser.
    pub air11_admin_token: Option<std::sync::Arc<String>>,
    /// Peer networks permitted to reach the `AirSense` admin API.
    pub air11_allowed_networks: std::sync::Arc<Vec<crate::api::air11::AllowedNetwork>>,
}

/// Require a database pool; return a named 503 when absent.
fn require_db(state: &AppState) -> Result<&sqlx::PgPool, (StatusCode, Json<ErrorResponse>)> {
    state.db.as_ref().ok_or_else(|| {
        error_reply(
            StatusCode::SERVICE_UNAVAILABLE,
            "database unavailable — VITALS_CONSOLE_DATABASE_URL not set or connection failed",
        )
    })
}

#[derive(Debug, PartialEq, Eq)]
enum PhoneFactState {
    Live,
    Unknown(String),
    Failing(String),
}

fn phone_fact_state(facts: &[HealthTypeFact]) -> PhoneFactState {
    if facts.is_empty() {
        return PhoneFactState::Unknown("no per-type HealthKit status was reported".to_string());
    }
    let mut has_unmeasured = false;
    for fact in facts {
        let outcomes = [
            fact.authorization_request_outcome.as_deref(),
            fact.observer_registration_outcome.as_deref(),
            fact.background_delivery_outcome.as_deref(),
        ];
        if let Some(outcome) = outcomes.iter().flatten().find(|outcome| {
            let lower = outcome.to_ascii_lowercase();
            lower.contains("denied") || lower.contains("failed") || lower.contains("failure")
        }) {
            return PhoneFactState::Failing(format!(
                "healthkit_type={}; outcome={outcome}",
                fact.healthkit_type
            ));
        }
        if fact.last_matching_sample_at.is_none() {
            has_unmeasured = true;
        }
        match fact.last_query_outcome.as_deref() {
            Some(outcome) if outcome.to_ascii_lowercase().contains("failed") => {
                return PhoneFactState::Failing(format!(
                    "healthkit_type={}; query_outcome={outcome}",
                    fact.healthkit_type
                ));
            }
            Some(outcome) => {
                let lower = outcome.to_ascii_lowercase();
                if lower.contains("no_matching") || !lower.contains("success") {
                    has_unmeasured = true;
                }
            }
            None => has_unmeasured = true,
        }
    }
    if has_unmeasured {
        PhoneFactState::Unknown(
            "HealthKit status has no matching sample or completed query".to_string(),
        )
    } else {
        PhoneFactState::Live
    }
}

/// GET /api/health
///
/// Fetches the daemon's health over its Unix socket and derives a per-source
/// state with the local liveness evaluator.
#[allow(clippy::too_many_lines)]
#[expect(
    clippy::collapsible_if,
    reason = "easier to read without nesting lets on stable rust"
)]
pub async fn health_handler(State(state): State<AppState>) -> Json<HealthView> {
    let mut config_and_mutes = std::collections::HashMap::new();
    if let Some(ref db) = state.db {
        if let Ok(map) = crate::db::get_config_and_mutes(db).await {
            config_and_mutes = map;
        }
    }

    let daemon_health = get_health(&state.socket_path).await;

    let (vitalsd, uptime, degraded, heartbeat_pulsing, daemon_sources) = match daemon_health {
        Ok(h) => (
            DaemonReachable::Reachable,
            Some(h.uptime_secs),
            h.degraded,
            Some(h.heartbeat_pulsing),
            Some(h.sources),
        ),
        Err(e) => (
            DaemonReachable::Unreachable(e.to_string()),
            None,
            vec![],
            None,
            None,
        ),
    };

    let latest_phone_beacon = match state.db.as_ref() {
        Some(database) => crate::db::latest_health_beacon(database)
            .await
            .ok()
            .flatten(),
        None => None,
    };

    // A pre-epoch clock must not fall back to zero: every age would render
    // as fresh, which is the reassuring lie the three-state rule forbids.
    let now_s: i64 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(e) => {
            return Json(HealthView {
                vitalsd: DaemonReachable::Unreachable(format!(
                    "system clock is before the Unix epoch: {e}"
                )),
                uptime: None,
                degraded: vec![format!("host clock unusable: {e}")],
                heartbeat_pulsing: None,
                sources: vec![],
            });
        }
    };

    let mut source_views = Vec::new();
    for source in ALL_SOURCES {
        let expected_cadence = source.expected_cadence();
        let core_cadence = match expected_cadence {
            vitals_registry::Cadence::EverySeconds(s) => {
                vitals_core::liveness::Cadence::EverySeconds(s)
            }
            vitals_registry::Cadence::OnDemand => vitals_core::liveness::Cadence::OnDemand,
        };
        let daemon_info = daemon_sources
            .as_ref()
            .and_then(|sources| sources.iter().find(|s| s.source == source.id()));

        // `activity` and `last_link_epoch_s` are left at their "not measured"
        // defaults. Both are available from the collector's status file -- the
        // air11 API already reads `therapy_state` and
        // `last_host_receipt_unix_micros` from it -- but plumbing them here is
        // its own change, and defaulting them cannot produce a false alarm:
        // an unobserved activity gate never licenses escalation and an absent
        // contact timestamp contributes nothing.
        let mut evidence = Evidence::default();

        let mut observation_age_s = None;
        let mut raw_liveness = None;

        if let Some(info) = daemon_info {
            raw_liveness = Some(info.liveness);
            if let Some(age) = info.last_sample_age_secs {
                // An age that cannot convert is a malformed report, and a
                // malformed report must not evaluate as a fresh sample.
                if let Ok(age_i) = i64::try_from(age) {
                    observation_age_s = Some(age);
                    // A sample age is sample evidence. It used to be
                    // written into last_attempt_epoch_s and
                    // last_success_epoch_s, which compared it against the
                    // attempt cadence -- the same unit error the silence
                    // detector made, at a second site.
                    evidence.last_sample_epoch_s = Some(now_s.saturating_sub(age_i));
                } else {
                    evidence.last_attempt_failure = Some(format!(
                        "daemon reported an impossible sample age ({age}s): treated as unmeasurable"
                    ));
                }
            }
        } else if source == VitalsSource::Healthkit {
            if let Some((received_at, facts)) = latest_phone_beacon.as_ref() {
                let received_s = received_at.timestamp();
                evidence.last_attempt_epoch_s = Some(received_s);
                match phone_fact_state(facts) {
                    PhoneFactState::Live => evidence.last_success_epoch_s = Some(received_s),
                    PhoneFactState::Unknown(_) => {}
                    PhoneFactState::Failing(reason) => {
                        evidence.last_attempt_failure = Some(reason);
                    }
                }
            }
        }

        let bounds = vitals_core::liveness::Bounds {
            max_sample_arrival_s: source.max_sample_arrival_s(),
            sample_silence_failure_s: source.sample_silence_failure_s(),
            max_event_age_s: source.max_event_age_s(),
            max_link_silence_s: source.max_link_silence_s(),
            ..vitals_core::liveness::Bounds::from_cadence(core_cadence)
        };
        let mut assertion = evaluate(now_s, bounds, &evidence);
        if source == VitalsSource::Healthkit {
            if let Some((_, facts)) = latest_phone_beacon.as_ref() {
                match phone_fact_state(facts) {
                    PhoneFactState::Live => {}
                    PhoneFactState::Unknown(reason) => {
                        assertion.state = vitals_core::liveness::HealthState::Unknown;
                        assertion.reason = reason;
                        assertion.degrade_deadline_epoch_s = None;
                    }
                    PhoneFactState::Failing(reason) => {
                        assertion.state = vitals_core::liveness::HealthState::Failing;
                        assertion.reason = reason;
                        assertion.degrade_deadline_epoch_s = None;
                    }
                }
            }
        }

        let cm = config_and_mutes.get(&source.id());
        let is_configured = cm.is_none_or(|c| c.is_configured);
        let muted_until = cm
            .and_then(|c| c.muted_until.as_ref())
            .map(chrono::DateTime::to_rfc3339);
        let muted_by = cm.and_then(|c| c.muted_by.clone());

        // A retired source stays in the view -- nothing may silently vanish
        // from monitoring -- but it is labelled rather than evaluated. It has
        // no device behind it, so `Unknown` would be a health claim about
        // something that is not there, and a permanently-Unknown card is how
        // an operator learns to stop reading cards.
        let state_str = if source.is_retired() {
            "RETIRED"
        } else if is_configured {
            match assertion.state {
                vitals_core::liveness::HealthState::Live => "Live",
                vitals_core::liveness::HealthState::Failing => "Failing",
                vitals_core::liveness::HealthState::Unknown => "Unknown",
            }
        } else {
            "NOT_CONFIGURED"
        };
        if source.is_retired() {
            assertion.reason =
                "retired: declared only so historical rows resolve to it".to_string();
            assertion.degrade_deadline_epoch_s = None;
        }

        source_views.push(SourceHealthView {
            id: source.id(),
            name: format!("{source:?}"),
            state: state_str.to_string(),
            reason: assertion.reason,
            degrade_deadline_epoch_s: assertion.degrade_deadline_epoch_s,
            observation_age_s,
            raw_liveness,
            is_configured,
            muted_until,
            muted_by,
        });
    }

    Json(HealthView {
        vitalsd,
        uptime,
        degraded,
        heartbeat_pulsing,
        sources: source_views,
    })
}

/// GET `/dashboard` and `/dashboard/`
///
/// Serves the real-data dashboard from the same origin as its `/api/*` feeds.
#[allow(
    clippy::unused_async,
    reason = "axum handlers must be async to satisfy the Handler trait;               whether the lint fires varies by target"
)]
pub async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// GET /api/version
///
/// Renders build identity embedded by `build.rs` and schema identity measured
/// from the deployed Sqitch metadata.
pub async fn version_handler(State(state): State<AppState>) -> Json<VersionView> {
    let (schema_revision, pending_migrations, schema_unmeasured_reason) =
        match state.db_ops.get_schema_identity().await {
            Ok((revision, pending)) => (Some(revision), Some(pending), None),
            Err(reason) => (None, None, Some(reason)),
        };
    Json(VersionView {
        version: env!("CARGO_PKG_VERSION"),
        git_commit: env!("VITALS_BUILD_COMMIT"),
        build_epoch_s: env!("VITALS_BUILD_EPOCH_S"),
        schema_revision,
        pending_migrations,
        schema_unmeasured_reason,
    })
}

// ---- New handlers ----

/// POST /api/envelope/register
///
/// Register a legacy client for the loopback-only envelope test API. Production
/// uses the versioned signed-device client routes; this handler is mounted only
/// when `VITALS_CONSOLE_TEST_ROUTES=1`.
///
/// # Errors
/// 400 on invalid base64 or a secret that is not 32 bytes; 503 when the
/// database is unavailable; 409 on a takeover attempt; 500 on database
/// failure.
pub async fn register_handler(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let db = require_db(&state)?;

    // Decode base64 secret -> 32 bytes.
    let secret_bytes: [u8; 32] = {
        let v = base64::engine::general_purpose::STANDARD
            .decode(&req.secret)
            .map_err(|e| {
                error_reply(
                    StatusCode::BAD_REQUEST,
                    format!("invalid base64 secret: {e}"),
                )
            })?;
        let len = v.len();
        v.try_into().map_err(|_| {
            error_reply(
                StatusCode::BAD_REQUEST,
                format!("secret must be 32 bytes after base64 decode, got {len}"),
            )
        })?
    };

    db::register_client(db, req.client_id, &secret_bytes).await
}

/// POST /api/envelope
///
/// Accept an encrypted envelope carrying a domain record. Decrypts with
/// XSalsa20-Poly1305 using the registered secret, applies idempotency rules
/// (spec §2.3b), and records the attempt with an effect breakdown.
///
/// # Errors
/// 503 when the database is unavailable; 400 on malformed base64, bad
/// nonce/ciphertext, or an unsupported envelope class; 401 for an unknown
/// client; 409 on an idempotency conflict; 500 on database failure.
pub async fn envelope_handler(
    State(state): State<AppState>,
    Json(req): Json<EnvelopeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let db = require_db(&state)?;

    // ---- Create attempt record BEFORE work (spec §2.3, criterion 3) ----
    let attempt_id = db::create_attempt(
        db,
        "envelope",
        &req.client_id.to_string(),
        30, // expected_duration_s
    )
    .await?;

    // ---- Do the work ----
    let result = process_envelope(db, &req).await;

    // ---- Build effect breakdown and finish attempt ----
    let (outcome, effect, detail) = match &result {
        Ok((_, _, e)) => ("succeeded", e.clone(), None::<String>),
        Err((_, err_resp)) => ("failed", Effect::default(), Some(err_resp.error.clone())),
    };

    let _ = db::finish_attempt(db, attempt_id, outcome, &effect, detail.as_deref()).await;

    result.map(|(s, b, _)| (s, Json(b)))
}

/// Core envelope processing logic, separated so we can control when the
/// attempt is finished.
async fn process_envelope(
    db: &sqlx::PgPool,
    req: &EnvelopeRequest,
) -> Result<(StatusCode, serde_json::Value, Effect), (StatusCode, Json<ErrorResponse>)> {
    // Look up registered secret for this client.
    let secret_row: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT secret FROM envelope_clients WHERE client_id = $1")
            .bind(req.client_id)
            .fetch_optional(db)
            .await
            .map_err(|e| {
                error_reply(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("db error looking up client: {e}"),
                )
            })?;

    let stored_secret = secret_row.ok_or_else(|| {
        error_reply(
            StatusCode::UNAUTHORIZED,
            format!("unknown client_id: {}", req.client_id),
        )
    })?;

    let secret_bytes: [u8; 32] = stored_secret.0.try_into().map_err(|_| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            "stored secret is wrong length".to_string(),
        )
    })?;

    // Decode nonce (base64 -> 24 bytes) and payload (base64 -> ciphertext).
    let nonce_bytes: [u8; 24] = {
        let v = base64::engine::general_purpose::STANDARD
            .decode(&req.nonce)
            .map_err(|e| {
                error_reply(
                    StatusCode::BAD_REQUEST,
                    format!("invalid nonce base64: {e}"),
                )
            })?;
        let len = v.len();
        v.try_into().map_err(|_| {
            error_reply(
                StatusCode::BAD_REQUEST,
                format!("nonce must be 24 bytes after base64 decode, got {len}"),
            )
        })?
    };

    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(&req.payload)
        .map_err(|e| {
            error_reply(
                StatusCode::BAD_REQUEST,
                format!("invalid payload base64: {e}"),
            )
        })?;

    // Decrypt.
    let plaintext = crypto::decrypt(&ciphertext, &secret_bytes, &nonce_bytes).map_err(|e| {
        error_reply(
            StatusCode::BAD_REQUEST,
            format!("decryption failed (bad key/nonce/ciphertext?): {e}"),
        )
    })?;

    // Parse the envelope payload.
    let envelope: EnvelopePayload = serde_json::from_slice(&plaintext).map_err(|e| {
        error_reply(
            StatusCode::BAD_REQUEST,
            format!("invalid envelope JSON: {e}"),
        )
    })?;

    // Check class - only "domain_record" is supported in M0.
    if envelope.class != "domain_record" {
        return Err(error_reply(
            StatusCode::BAD_REQUEST,
            format!("unknown envelope class: {}", envelope.class),
        ));
    }

    let record = envelope.record;
    let actor = format!("client:{}", req.client_id);

    // Apply immutable version and conflict rules.
    let outcome = db::process_domain_record(db, &record, &actor).await?;

    match outcome {
        db::DomainRecordOutcome::Created { response } => {
            let body = serde_json::to_value(&response).unwrap_or_default();
            let effect = Effect {
                created: 1,
                ..Default::default()
            };
            Ok((StatusCode::CREATED, body, effect))
        }
        db::DomainRecordOutcome::Replayed { response } => {
            let body = serde_json::to_value(&response).unwrap_or_default();
            let effect = Effect {
                replayed: 1,
                ..Default::default()
            };
            Ok((StatusCode::OK, body, effect))
        }
        db::DomainRecordOutcome::Conflict { id } => Err(error_reply(
            StatusCode::CONFLICT,
            format!("conflict: record {id} already exists with different content"),
        )),
    }
}

/// GET /api/attempts?limit=N
///
/// List the most recent attempt rows with their effect breakdowns visible.
///
/// # Errors
/// 503 when the database is unavailable; 500 on database failure.
pub async fn attempts_handler(
    State(state): State<AppState>,
    Query(query): Query<AttemptsQuery>,
) -> Result<Json<Vec<crate::models::AttemptView>>, (StatusCode, Json<ErrorResponse>)> {
    let db = require_db(&state)?;
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let attempts = db::list_attempts(db, limit).await?;
    Ok(Json(attempts))
}

/// GET /test
///
/// Crude test page: one form that sends a plaintext domain record via the
/// server-side test route. Deliberately not part of the console (spec §9).
#[expect(
    clippy::unused_async,
    reason = "axum handlers must be async to satisfy the Handler trait"
)]
pub async fn test_page_handler() -> Html<&'static str> {
    Html(TEST_PAGE_HTML)
}

/// POST /api/envelope/plaintext-test
///
/// Server-side test endpoint guarded by `VITALS_CONSOLE_TEST_ROUTES=1`.
/// Auto-generates a `client_id` + secret, registers the client, encrypts the
/// domain record with XSalsa20-Poly1305, and submits it through the envelope
/// processing path. Chosen over the client-side JS approach because it avoids
/// bundling a `NaCl` secretbox implementation in the page.
///
/// # Errors
/// 404 when test routes are disabled; 503 when the database is unavailable;
/// 400/401/409 from envelope processing (bad ciphertext, unknown client,
/// idempotency conflict); 500 on RNG, serialization, encryption, or database
/// failure.
pub async fn plaintext_test_handler(
    State(state): State<AppState>,
    Json(req): Json<PlaintextTestRequest>,
) -> Result<Json<PlaintextTestResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !state.test_routes_enabled {
        return Err(error_reply(
            StatusCode::NOT_FOUND,
            "test routes disabled - set VITALS_CONSOLE_TEST_ROUTES=1",
        ));
    }

    let db = require_db(&state)?;

    // Generate random credentials.
    let client_id = Uuid::now_v7();
    let mut secret = [0u8; 32];
    getrandom::getrandom(&mut secret)
        .map_err(|e| error_reply(StatusCode::INTERNAL_SERVER_ERROR, format!("RNG error: {e}")))?;
    let secret_b64 = base64::engine::general_purpose::STANDARD.encode(secret);

    // Register the client.
    match db::register_client(db, client_id, &secret).await {
        Ok(_) => {}
        Err(e) => {
            return Ok(Json(PlaintextTestResponse {
                status: "register_failed".to_string(),
                client_id,
                secret: secret_b64,
                record: None,
                error: Some(e.1.error.clone()),
            }));
        }
    }

    // Build a domain record.
    let record_id = Uuid::now_v7();

    // Create the envelope payload.
    let envelope = EnvelopePayload {
        class: "domain_record".to_string(),
        record: crate::models::DomainRecordPayload {
            id: record_id,
            kind: req.kind.clone(),
            schema_version: crate::domain::CURRENT_SCHEMA_VERSION,
            recorded_at: Utc::now().to_rfc3339(),
            supersedes_id: None,
            tombstone_target_id: None,
            content: req.content.clone(),
        },
    };
    let payload_bytes = serde_json::to_vec(&envelope).map_err(|e| {
        error_reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("serialization error: {e}"),
        )
    })?;

    // Generate a random nonce.
    let mut nonce_bytes = [0u8; 24];
    getrandom::getrandom(&mut nonce_bytes)
        .map_err(|e| error_reply(StatusCode::INTERNAL_SERVER_ERROR, format!("RNG error: {e}")))?;

    // Encrypt.
    let ciphertext = crypto::encrypt(&payload_bytes, &secret, &nonce_bytes)
        .map_err(|e| error_reply(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Build the envelope request and process it.
    let envelope_req = EnvelopeRequest {
        client_id,
        nonce: base64::engine::general_purpose::STANDARD.encode(nonce_bytes),
        payload: base64::engine::general_purpose::STANDARD.encode(&ciphertext),
    };

    // Create attempt before work.
    let attempt_id = db::create_attempt(db, "envelope", &client_id.to_string(), 30).await?;

    let result = process_envelope(db, &envelope_req).await;

    let (status_str, record, error) = match &result {
        Ok((_status, body, _effect)) => {
            let record: Option<DomainRecordResponse> = serde_json::from_value(body.clone()).ok();
            ("ok".to_string(), record, None)
        }
        Err((_code, err_resp)) => ("failed".to_string(), None, Some(err_resp.error.clone())),
    };

    // Compute effect for finish
    let finish_effect = match &result {
        Ok((_, _, e)) => e.clone(),
        Err(_) => Effect {
            rejected: 1,
            ..Default::default()
        },
    };
    let (outcome, detail) = match &result {
        Ok(_) => ("succeeded", None),
        Err((_, err_resp)) => ("failed", Some(err_resp.error.as_str())),
    };
    let _ = db::finish_attempt(db, attempt_id, outcome, &finish_effect, detail).await;

    match result {
        Ok(_) => Ok(Json(PlaintextTestResponse {
            status: status_str,
            client_id,
            secret: secret_b64,
            record,
            error,
        })),
        Err((code, err_resp)) => {
            // Return 200 even on processing error so the test page can display the result
            Ok(Json(PlaintextTestResponse {
                status: format!("failed ({code})"),
                client_id,
                secret: secret_b64,
                record,
                error: Some(err_resp.error.clone()),
            }))
        }
    }
}

#[expect(
    clippy::unused_async,
    reason = "axum handlers must be async to satisfy the Handler trait"
)]
/// POST /api/health/mutes
///
/// Sets a time-bounded mute on a health condition. Mutes expire to underlying health.
/// Currently fails closed as auth is not available.
pub async fn health_mute_handler() -> (StatusCode, Json<ErrorResponse>) {
    error_reply(
        StatusCode::FORBIDDEN,
        "mutation blocked: auth interface not yet available",
    )
}

#[expect(
    clippy::unused_async,
    reason = "axum handlers must be async to satisfy the Handler trait"
)]
/// POST /api/config/declarations
///
/// Declares whether an optional source is configured.
/// Currently fails closed as auth is not available.
pub async fn source_declaration_handler() -> (StatusCode, Json<ErrorResponse>) {
    error_reply(
        StatusCode::FORBIDDEN,
        "mutation blocked: auth interface not yet available",
    )
}

#[expect(
    clippy::unused_async,
    reason = "axum handlers must be async to satisfy the Handler trait"
)]
/// POST /api/config/overrides
///
/// Sets operator display overrides for registry values. Upgrades do not overwrite values.
/// Currently fails closed as auth is not available.
pub async fn operator_override_handler() -> (StatusCode, Json<ErrorResponse>) {
    error_reply(
        StatusCode::FORBIDDEN,
        "mutation blocked: auth interface not yet available",
    )
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "unit tests use fixed fixtures and a local socket; unwrap is the assertion that setup and the canned daemon reply succeed"
    )]
    #![expect(
        clippy::indexing_slicing,
        reason = "tests index serde_json::Value views of canned handler responses"
    )]

    use super::*;
    use axum::{Router, body::Body, http::Request as AxumRequest, routing::get};
    use http_body_util::BodyExt; // for `collect`
    use serde_json::Value;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;
    use tower::util::ServiceExt; // for `oneshot`
    use vitals_proto::{Health, Liveness, Response, SourceHealth, check_frame_len, encode_frame};
    use vitals_registry::VitalsSource;

    #[test]
    fn phone_status_no_matching_sample_is_unknown() {
        let fact = HealthTypeFact {
            healthkit_type: "HKQuantityTypeIdentifierHeartRate".to_string(),
            configured: true,
            authorization_request_outcome: Some("request_completed_unknown_read_grant".to_string()),
            observer_registration_outcome: Some("registered".to_string()),
            background_delivery_outcome: Some("requested".to_string()),
            last_query_outcome: Some("no_matching_sample".to_string()),
            last_query_at: None,
            last_matching_sample_at: None,
            last_pi_ack_at: None,
        };
        assert_eq!(
            phone_fact_state(&[fact]),
            PhoneFactState::Unknown(
                "HealthKit status has no matching sample or completed query".to_string()
            )
        );
    }

    #[test]
    fn phone_status_denial_is_failing() {
        let fact = HealthTypeFact {
            healthkit_type: "HKQuantityTypeIdentifierHeartRate".to_string(),
            configured: true,
            authorization_request_outcome: Some("denied".to_string()),
            observer_registration_outcome: None,
            background_delivery_outcome: None,
            last_query_outcome: None,
            last_query_at: None,
            last_matching_sample_at: None,
            last_pi_ack_at: None,
        };
        assert!(matches!(
            phone_fact_state(&[fact]),
            PhoneFactState::Failing(_)
        ));
    }

    #[test]
    fn phone_status_success_requires_a_successful_query() {
        let fact = HealthTypeFact {
            healthkit_type: "HKQuantityTypeIdentifierHeartRate".to_string(),
            configured: true,
            authorization_request_outcome: Some("request_completed_unknown_read_grant".to_string()),
            observer_registration_outcome: Some("registered".to_string()),
            background_delivery_outcome: Some("requested".to_string()),
            last_query_outcome: Some("success_with_samples".to_string()),
            last_query_at: None,
            last_matching_sample_at: Some("2025-01-15T08:09:00.000Z".to_string()),
            last_pi_ack_at: None,
        };
        assert_eq!(phone_fact_state(&[fact]), PhoneFactState::Live);
    }

    struct DummyDbOps;
    #[async_trait::async_trait]
    impl crate::api::postgres::DbOps for DummyDbOps {
        async fn get_schema_identity(&self) -> Result<(String, u64), String> {
            Ok(("fixture-change".to_string(), 0))
        }
        async fn get_latest(&self, _s: i32, _t: i32) -> Result<Option<(i64, f64)>, String> {
            Ok(None)
        }
        async fn get_nightly(
            &self,
            _n: u32,
            _source: Option<i32>,
        ) -> Result<Vec<crate::api::postgres::NightlyRow>, String> {
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

    fn spawn_stub_daemon(socket_path: PathBuf, canned_health: Health) {
        tokio::spawn(async move {
            let listener = UnixListener::bind(&socket_path).unwrap();
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).await.is_ok() {
                    let len = check_frame_len(u32::from_le_bytes(len_buf)).unwrap();
                    let mut payload = vec![0u8; len];
                    if stream.read_exact(&mut payload).await.is_ok() {
                        let resp = Response::Health {
                            health: Box::new(canned_health),
                        };
                        let out = encode_frame(&resp).unwrap();
                        let _ = stream.write_all(&out).await;
                    }
                }
            }
        });
    }

    fn default_health() -> Health {
        Health {
            uptime_secs: 100,
            sources: vec![],
            total_samples: 0,
            spool_bytes: 0,
            sync_append_ready: true,
            lag_bytes: 0,
            lag_age_seconds: 0,
            last_advanced_at: None,
            subscribers: vec![],
            heartbeat: vitals_proto::HeartbeatInputs {
                monitoring_window_active: true,
                primary_sample_age_secs: Some(10),
                tier_evaluation_age_secs: Some(5),
                drain_progressing: true,
            },
            heartbeat_pulsing: true,
            degraded: vec![],
        }
    }

    fn assert_registry_census(sources: &[Value]) {
        assert_eq!(
            sources.len(),
            ALL_SOURCES.len(),
            "health view must left-join every registry source"
        );
        for source in ALL_SOURCES {
            let view = sources
                .iter()
                .find(|view| view["id"] == source.id() && view["name"] == format!("{source:?}"));
            assert!(
                view.is_some(),
                "registry source {source:?} disappeared from health view"
            );
            // Retired sources stay visible but are never given a health state:
            // there is no device to assert anything about.
            let state = view.map(|view| view["state"].clone()).unwrap_or_default();
            if source.is_retired() {
                assert_eq!(state, "RETIRED", "{source:?} must render retired");
            } else {
                assert_ne!(state, "RETIRED", "{source:?} is not retired");
            }
        }
    }

    fn test_state(socket_path: PathBuf) -> AppState {
        AppState {
            socket_path,
            db: None,
            test_routes_enabled: false,
            db_url: String::new(),
            sys_ops: crate::api::ops::SysOps::real(),
            host_history: crate::api::host::HostHistory::default(),
            db_ops: std::sync::Arc::new(DummyDbOps),
            security: std::sync::Arc::new(crate::security::SecurityState::unconfigured()),
            ops_token: None,
            server_instance_id: None,
            client_origin: None,
            server_display_label: "test server".to_string(),
            air11_actions_enabled: false,
            air11_action_token: None,
            air11_admin_token: None,
            air11_allowed_networks: std::sync::Arc::new(
                crate::api::air11::default_allowed_networks(),
            ),
        }
    }

    #[tokio::test]
    async fn test_dashboard_is_served_same_origin() {
        let app = Router::new().route("/dashboard", get(dashboard_handler));
        let response = app
            .oneshot(
                AxumRequest::builder()
                    .uri("/dashboard")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/html; charset=utf-8"
        );
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let html = std::str::from_utf8(&body).unwrap();
        assert!(html.contains("/api/health"));
        assert!(html.contains("/api/nightly"));
    }

    #[tokio::test]
    async fn test_version_reports_embedded_build_and_measured_schema_identity() {
        let Json(version) = version_handler(State(test_state(PathBuf::new()))).await;
        assert_eq!(version.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(version.git_commit, env!("VITALS_BUILD_COMMIT"));
        assert_eq!(version.build_epoch_s, env!("VITALS_BUILD_EPOCH_S"));
        assert_eq!(version.schema_revision.as_deref(), Some("fixture-change"));
        assert_eq!(version.pending_migrations, Some(0));
        assert_eq!(version.schema_unmeasured_reason, None);
    }

    #[tokio::test]
    async fn test_unreachable_socket() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("vitalsd.sock");

        let app = Router::new()
            .route("/api/health", get(health_handler))
            .with_state(test_state(socket_path));

        let req = AxumRequest::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        // (d) unreachable socket produces the unreachable marker, not an empty list
        assert!(json["vitalsd"].get("unreachable").is_some());

        let sources = json["sources"].as_array().unwrap();
        assert_registry_census(sources);
    }

    #[tokio::test]
    async fn test_source_mapping() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("vitalsd.sock");

        let mut health = default_health();
        health.sources = vec![
            SourceHealth {
                source: VitalsSource::Emay.id(),
                liveness: Liveness::Live,
                last_sample_age_secs: Some(10),
                samples: 100,
            },
            SourceHealth {
                source: VitalsSource::PolarH10.id(),
                liveness: Liveness::Stale,
                last_sample_age_secs: Some(1000),
                samples: 50,
            },
            SourceHealth {
                source: VitalsSource::Healthkit.id(),
                liveness: Liveness::NeverSeen,
                last_sample_age_secs: None,
                samples: 0,
            },
        ];

        spawn_stub_daemon(socket_path.clone(), health);
        // give the listener a moment to bind
        tokio::time::sleep(Duration::from_millis(50)).await;

        let app = Router::new()
            .route("/api/health", get(health_handler))
            .with_state(test_state(socket_path));

        let req = AxumRequest::builder()
            .uri("/api/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["vitalsd"], "reachable");

        let sources = json["sources"].as_array().unwrap();
        assert_registry_census(sources);

        // Find Emay (a)
        //
        // `Unknown`, not `Live`, and that is the honest answer rather than a
        // regression. The daemon reports only `last_sample_age_secs`. That age
        // used to be written into `last_attempt_epoch_s` and compared against
        // `expected_cadence_s` -- reporting against a quantity that had not
        // been measured, since `attempts` has never held a row. Emay has no
        // measured `max_sample_arrival_s`, so the sample channel now says
        // nothing and the attempt channel says what is true: no attempt was
        // ever recorded.
        //
        // In production this changes nothing: Emay's newest row is from
        // 2026-10-30, so it was already `Unknown` on age. What is removed is
        // the case where a fresh sample would have reported `Live` on behalf
        // of a collector that had not run. Declaring an arrival bound (needs
        // measurement, and Emay has produced nothing for ten days to measure)
        // or writing attempt rows -- the next task -- restores a useful
        // answer. Contract (e) still holds for every reason that names a
        // threshold at all.
        let emay = sources.iter().find(|s| s["name"] == "Emay").unwrap();
        assert_eq!(emay["state"], "Unknown");
        assert_eq!(emay["observation_age_s"], 10);
        assert_eq!(emay["reason"], "never completed an attempt");

        // Find PolarH10 (b)
        let polar = sources.iter().find(|s| s["name"] == "PolarH10").unwrap();
        assert_eq!(polar["state"], "Unknown"); // never FAILING
        assert_eq!(polar["observation_age_s"], 1000);
        // Critically *not* "expect <= 1s": that 1 is a sample rate, and
        // judging a sample age against it is the defect being removed.
        assert_eq!(polar["reason"], "never completed an attempt");
        assert!(!polar["reason"].as_str().unwrap().contains("1s"));

        // Find Healthkit (b)
        let hk = sources.iter().find(|s| s["name"] == "Healthkit").unwrap();
        assert_eq!(hk["state"], "Unknown");
        assert_eq!(hk["reason"], "never observed");

        // Find AdUa651ble (c) registry source absent from reply
        let ad = sources.iter().find(|s| s["name"] == "AdUa651ble").unwrap();
        assert_eq!(ad["state"], "Unknown");
        assert_eq!(ad["reason"], "never observed");
    }

    #[tokio::test]
    async fn test_test_routes_disabled_by_default() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("vitalsd.sock");
        let state = test_state(socket_path);

        let app = Router::new()
            .route(
                "/api/envelope/plaintext-test",
                get(plaintext_test_handler).post(plaintext_test_handler),
            )
            .with_state(state);

        let req = AxumRequest::builder()
            .method("POST")
            .uri("/api/envelope/plaintext-test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"kind":"test","content":{"x":1}}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 404);
    }
}
