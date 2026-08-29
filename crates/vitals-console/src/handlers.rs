use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::{Json, response::Html};
use base64::Engine;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use vitals_core::liveness::{Evidence, evaluate};
use vitals_registry::ALL_SOURCES;

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
    /// `PostgreSQL` access for the read API, injected for testability.
    pub db_ops: std::sync::Arc<dyn crate::api::postgres::DbOps>,
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

/// GET /api/health
///
/// Fetches the daemon's health over its Unix socket and derives a per-source
/// state with the local liveness evaluator.
pub async fn health_handler(State(state): State<AppState>) -> Json<HealthView> {
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

        let mut evidence = Evidence {
            last_attempt_epoch_s: None,
            last_attempt_failure: None,
            last_success_epoch_s: None,
            inflight: None,
            learned_interval_s: None,
        };

        let mut observation_age_s = None;
        let mut raw_liveness = None;

        if let Some(info) = daemon_info {
            raw_liveness = Some(info.liveness);
            if let Some(age) = info.last_sample_age_secs {
                // An age that cannot convert is a malformed report, and a
                // malformed report must not evaluate as a fresh sample.
                if let Ok(age_i) = i64::try_from(age) {
                    observation_age_s = Some(age);
                    let timestamp = now_s.saturating_sub(age_i);
                    evidence.last_attempt_epoch_s = Some(timestamp);
                    evidence.last_success_epoch_s = Some(timestamp);
                } else {
                    evidence.last_attempt_failure = Some(format!(
                        "daemon reported an impossible sample age ({age}s): treated as unmeasurable"
                    ));
                }
            }
        }

        let assertion = evaluate(now_s, core_cadence, &evidence);

        let state_str = match assertion.state {
            vitals_core::liveness::HealthState::Live => "Live",
            vitals_core::liveness::HealthState::Failing => "Failing",
            vitals_core::liveness::HealthState::Unknown => "Unknown",
        };

        source_views.push(SourceHealthView {
            id: source.id(),
            name: format!("{source:?}"),
            state: state_str.to_string(),
            reason: assertion.reason,
            degrade_deadline_epoch_s: assertion.degrade_deadline_epoch_s,
            observation_age_s,
            raw_liveness,
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
/// Renders the crate version, `GIT_DESCRIBE` when built from a tag, and a
/// milestone note.
#[expect(
    clippy::unused_async,
    reason = "axum handlers must be async to satisfy the Handler trait"
)]
pub async fn version_handler() -> Json<VersionView> {
    Json(VersionView {
        version: env!("CARGO_PKG_VERSION"),
        git: option_env!("GIT_DESCRIBE"),
        note: "attempts schema: deployed in this milestone",
    })
}

// ---- New handlers ----

/// POST /api/envelope/register
///
/// Register a client for the envelope write API. The bind default is
/// 127.0.0.1 and the auth posture is an open decision (spec §9c) — see the
/// design document for the bind and auth posture decision deferred to the
/// point of use.
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
    let content_hash = db::content_sha256(&record.content);

    // Apply idempotency rules (spec §2.3b).
    let outcome =
        db::process_domain_record(db, record.id, &record.kind, &record.content, &content_hash)
            .await?;

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

    struct DummyDbOps;
    #[async_trait::async_trait]
    impl crate::api::postgres::DbOps for DummyDbOps {
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

    fn test_state(socket_path: PathBuf) -> AppState {
        AppState {
            socket_path,
            db: None,
            test_routes_enabled: false,
            db_url: String::new(),
            sys_ops: crate::api::ops::SysOps::real(),
            db_ops: std::sync::Arc::new(DummyDbOps),
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
        assert!(
            !sources.is_empty(),
            "must not return an empty list of sources"
        );
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

        // Find Emay (a)
        let emay = sources.iter().find(|s| s["name"] == "Emay").unwrap();
        assert_eq!(emay["state"], "Live");
        assert_eq!(emay["observation_age_s"], 10);
        assert!(emay["degrade_deadline_epoch_s"].is_number());
        assert_eq!(emay["reason"], "10s ago, expect <= 60s"); // (e) reason strings name value + threshold

        // Find PolarH10 (b)
        let polar = sources.iter().find(|s| s["name"] == "PolarH10").unwrap();
        assert_eq!(polar["state"], "Unknown"); // never FAILING
        assert_eq!(polar["observation_age_s"], 1000);
        assert_eq!(polar["reason"], "no attempt for 1000s, expect <= 1s"); // (e)

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
