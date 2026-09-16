//! Admin-console authentication and request admission policy.
//!
//! This module protects browser/admin routes only. The versioned mobile client
//! routes are deliberately separate and use device signatures rather than
//! browser cookies.

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Maximum idle time for an authenticated admin session.
pub const SESSION_IDLE_SECONDS: i64 = 30 * 60;
/// Maximum absolute lifetime for an authenticated admin session.
pub const SESSION_ABSOLUTE_SECONDS: i64 = 8 * 60 * 60;
/// Window in which a password reauthentication satisfies sensitive actions.
pub const RECENT_AUTH_SECONDS: i64 = 5 * 60;
/// Maximum buffered request body, enforced by axum before handlers run.
pub const MAX_REQUEST_BODY_BYTES: usize = 1_048_576;
const MAX_LOGIN_FAILURES: u32 = 5;
const THROTTLE_SECONDS: i64 = 60;
const SESSION_COOKIE: &str = "aw_admin_session";
const CSRF_HEADER: &str = "x-anxietywatch-csrf";
/// Stable actor id for the single configured operator identity.
pub const AUTHENTICATED_ACTOR: &str = "operator";

#[derive(Debug, Clone, PartialEq, Eq)]
/// Admission failures are explicit so callers can render a named blocker.
pub enum SecurityError {
    /// No operator verifier is configured.
    NotConfigured,
    /// The supplied password did not verify.
    InvalidCredentials,
    /// This source is temporarily blocked after repeated failures.
    Throttled {
        /// Source identifier used for independent throttling.
        source: String,
        /// Seconds until this source may retry.
        retry_after: i64,
    },
    /// No session cookie was supplied.
    MissingSession,
    /// The session exceeded its idle or absolute lifetime.
    ExpiredSession,
    /// The state-changing request has no matching CSRF token.
    InvalidCSRF,
    /// The request host is not the configured private admin origin.
    InvalidOrigin(String),
    /// The in-memory security state could not be accessed.
    Internal(String),
}

impl SecurityError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotConfigured | Self::Internal(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::InvalidCredentials
            | Self::MissingSession
            | Self::ExpiredSession
            | Self::InvalidCSRF
            | Self::InvalidOrigin(_) => StatusCode::UNAUTHORIZED,
            Self::Throttled { .. } => StatusCode::TOO_MANY_REQUESTS,
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotConfigured => {
                "admin authentication unavailable: password verifier is not configured".to_string()
            }
            Self::InvalidCredentials => {
                "login rejected: password did not match the configured verifier".to_string()
            }
            Self::Throttled {
                source,
                retry_after,
            } => {
                format!(
                    "login blocked: source {source} exceeded {MAX_LOGIN_FAILURES} failures; retry after {retry_after}s"
                )
            }
            Self::MissingSession => "admin request blocked: session cookie is missing".to_string(),
            Self::ExpiredSession => {
                "admin request blocked: session expired; sign in again".to_string()
            }
            Self::InvalidCSRF => {
                "admin request blocked: CSRF token is missing or invalid".to_string()
            }
            Self::InvalidOrigin(value) => {
                format!("admin request blocked: origin {value} is not allowlisted")
            }
            Self::Internal(value) => format!("admin security unavailable: {value}"),
        }
    }
}

/// In-memory operator security state. Sessions are intentionally invalidated on process restart.
#[derive(Clone)]
pub struct SecurityState {
    password_hash: Option<String>,
    admin_host: Option<String>,
    tls_terminated: bool,
    trusted_proxy: bool,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    throttles: Arc<Mutex<HashMap<String, Throttle>>>,
}

#[derive(Clone)]
struct Session {
    csrf: String,
    created_at: i64,
    last_seen_at: i64,
    recent_auth_at: i64,
}

#[derive(Clone, Default)]
struct Throttle {
    failures: u32,
    blocked_until: i64,
}

/// Non-secret values returned after a successful login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginReceipt {
    /// Opaque session-cookie value.
    pub session_cookie: String,
    /// CSRF token bound to this session.
    pub csrf_token: String,
    /// Absolute session expiry as UTC epoch seconds.
    pub expires_at: i64,
}

/// Non-sensitive security facts rendered by the admin diagnostics surface.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SecurityView {
    /// Whether the supplied request has a valid session.
    pub admin_authenticated: bool,
    /// Human-readable transport mode.
    pub transport: &'static str,
    /// Whether Caddy reports TLS termination for this process.
    pub tls_terminated_at_proxy: bool,
    /// Configured idle lifetime.
    pub session_idle_seconds: i64,
    /// Configured absolute lifetime.
    pub session_absolute_seconds: i64,
    /// Recent-auth reauthentication window.
    pub recent_auth_window_seconds: i64,
    /// Whether recent password authentication evidence exists.
    pub recent_auth_evidence: bool,
}

impl SecurityState {
    /// Load the operator verifier and proxy/origin facts from the environment.
    #[must_use]
    pub fn from_env() -> Self {
        Self::new(
            std::env::var("VITALS_CONSOLE_ADMIN_PASSWORD_HASH").ok(),
            std::env::var("VITALS_CONSOLE_TLS_TERMINATED").is_ok_and(|value| value == "1"),
            std::env::var("VITALS_CONSOLE_TRUST_PROXY").is_ok_and(|value| value == "1"),
            Some(
                std::env::var("VITALS_CONSOLE_ADMIN_HOST")
                    .unwrap_or_else(|_| "admin.example.invalid".to_string()),
            ),
        )
    }

    /// Construct security state with an optional Argon2id verifier and admin host.
    #[must_use]
    pub fn new(
        password_hash: Option<String>,
        tls_terminated: bool,
        trusted_proxy: bool,
        admin_host: Option<String>,
    ) -> Self {
        Self {
            password_hash,
            admin_host: admin_host.map(|host| host.to_ascii_lowercase()),
            tls_terminated,
            trusted_proxy,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            throttles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Construct a fail-closed state for tests and installations awaiting setup.
    #[must_use]
    pub fn unconfigured() -> Self {
        Self::new(None, false, false, None)
    }

    /// Verify a password and create a bounded session for one source.
    ///
    /// # Errors
    ///
    /// Returns an explicit configuration, credential, throttle, or state-lock failure.
    pub fn login(
        &self,
        source: &str,
        password: &str,
        now: i64,
    ) -> Result<LoginReceipt, SecurityError> {
        let password_hash = self
            .password_hash
            .as_deref()
            .ok_or(SecurityError::NotConfigured)?;
        {
            let throttles = self.throttles.lock().map_err(|_| {
                SecurityError::Internal("login throttle state lock is poisoned".to_string())
            })?;
            if let Some(throttle) = throttles.get(source)
                && throttle.blocked_until > now
            {
                return Err(SecurityError::Throttled {
                    source: source.to_string(),
                    retry_after: throttle.blocked_until - now,
                });
            }
        }

        let parsed = PasswordHash::new(password_hash).map_err(|_| {
            SecurityError::Internal("configured Argon2id verifier is malformed".to_string())
        })?;
        let valid = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        if !valid {
            let mut throttles = self.throttles.lock().map_err(|_| {
                SecurityError::Internal("login throttle state lock is poisoned".to_string())
            })?;
            let throttle = throttles.entry(source.to_string()).or_default();
            throttle.failures = throttle.failures.saturating_add(1);
            if throttle.failures >= MAX_LOGIN_FAILURES {
                throttle.blocked_until = now.saturating_add(THROTTLE_SECONDS);
            }
            return Err(SecurityError::InvalidCredentials);
        }

        let session_id = Uuid::new_v4().to_string();
        let csrf = Uuid::new_v4().to_string();
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SecurityError::Internal("session state lock is poisoned".to_string()))?;
        sessions.insert(
            session_id.clone(),
            Session {
                csrf: csrf.clone(),
                created_at: now,
                last_seen_at: now,
                recent_auth_at: now,
            },
        );
        drop(sessions);
        if let Ok(mut throttles) = self.throttles.lock() {
            throttles.remove(source);
        }
        Ok(LoginReceipt {
            session_cookie: session_id,
            csrf_token: csrf,
            expires_at: now.saturating_add(SESSION_ABSOLUTE_SECONDS),
        })
    }

    /// Validate the session, origin, and CSRF token for a request.
    ///
    /// # Errors
    ///
    /// Returns the named session, origin, CSRF, expiry, or state-lock blocker.
    pub fn authenticate(
        &self,
        headers: &HeaderMap,
        method: &Method,
        now: i64,
    ) -> Result<(), SecurityError> {
        self.validate_host(headers)?;
        let session_id =
            cookie_value(headers, SESSION_COOKIE).ok_or(SecurityError::MissingSession)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SecurityError::Internal("session state lock is poisoned".to_string()))?;
        let session = sessions
            .get_mut(&session_id)
            .ok_or(SecurityError::MissingSession)?;
        if now.saturating_sub(session.created_at) > SESSION_ABSOLUTE_SECONDS
            || now.saturating_sub(session.last_seen_at) > SESSION_IDLE_SECONDS
        {
            sessions.remove(&session_id);
            return Err(SecurityError::ExpiredSession);
        }
        if matches!(
            *method,
            Method::POST | Method::PUT | Method::PATCH | Method::DELETE
        ) {
            let supplied = headers
                .get(CSRF_HEADER)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            if supplied != session.csrf {
                return Err(SecurityError::InvalidCSRF);
            }
        }
        session.last_seen_at = now;
        Ok(())
    }

    /// Revoke the session represented by the request cookie.
    ///
    /// # Errors
    ///
    /// Returns an explicit missing-cookie or state-lock failure.
    pub fn logout(&self, headers: &HeaderMap) -> Result<(), SecurityError> {
        let session_id =
            cookie_value(headers, SESSION_COOKIE).ok_or(SecurityError::MissingSession)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SecurityError::Internal("session state lock is poisoned".to_string()))?;
        sessions.remove(&session_id);
        Ok(())
    }

    /// Report visible transport and recent-auth evidence without exposing secrets.
    #[must_use]
    pub fn security_view(&self, headers: &HeaderMap, now: i64) -> SecurityView {
        let authenticated = self.authenticate(headers, &Method::GET, now).is_ok();
        let recent_auth_evidence = cookie_value(headers, SESSION_COOKIE)
            .and_then(|session_id| self.sessions.lock().ok()?.get(&session_id).cloned())
            .is_some_and(|session| {
                now.saturating_sub(session.recent_auth_at) <= RECENT_AUTH_SECONDS
            });
        SecurityView {
            admin_authenticated: authenticated,
            transport: if self.tls_terminated {
                "https_via_caddy"
            } else {
                "loopback_plaintext_test_only"
            },
            tls_terminated_at_proxy: self.tls_terminated,
            session_idle_seconds: SESSION_IDLE_SECONDS,
            session_absolute_seconds: SESSION_ABSOLUTE_SECONDS,
            recent_auth_window_seconds: RECENT_AUTH_SECONDS,
            recent_auth_evidence,
        }
    }

    fn source_from_request(&self, request: &Request<Body>) -> String {
        let peer = request
            .extensions()
            .get::<axum::extract::ConnectInfo<SocketAddr>>()
            .map_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)), |peer| peer.0);
        if self.trusted_proxy
            && peer.ip().is_loopback()
            && let Some(source) = request
                .headers()
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .and_then(|value| value.trim().parse::<IpAddr>().ok())
        {
            return source.to_string();
        }
        peer.ip().to_string()
    }

    fn validate_host(&self, headers: &HeaderMap) -> Result<(), SecurityError> {
        let Some(expected) = &self.admin_host else {
            return Ok(());
        };
        let actual = headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("missing-host")
            .to_ascii_lowercase();
        if &actual == expected {
            Ok(())
        } else {
            Err(SecurityError::InvalidOrigin(actual))
        }
    }
}

/// Protect browser/admin routes while leaving versioned mobile routes to device authentication.
pub async fn security_middleware(
    State(security): State<Arc<SecurityState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path.starts_with("/api/v1/client/") || matches!(path, "/api/health" | "/api/version") {
        return next.run(request).await;
    }
    if path == "/api/admin/login" {
        if let Err(error) = security.validate_host(request.headers()) {
            return (
                error.status(),
                Json(crate::models::ErrorResponse {
                    error: error.message(),
                }),
            )
                .into_response();
        }
        return next.run(request).await;
    }
    if path.starts_with("/console") || path.starts_with("/api/") {
        let now = unix_now();
        if let Err(error) = security.authenticate(request.headers(), request.method(), now) {
            return (
                error.status(),
                Json(crate::models::ErrorResponse {
                    error: error.message(),
                }),
            )
                .into_response();
        }
    }
    next.run(request).await
}

#[derive(Debug, serde::Deserialize)]
/// Password body accepted only by the login route.
pub struct LoginRequest {
    /// Operator password; it is never logged or echoed.
    pub password: String,
}

/// Authenticate the operator and issue an HttpOnly/SameSite session (Secure via Caddy).
///
/// # Errors
///
/// Returns a named malformed-body, throttle, configuration, or verifier blocker.
pub async fn login_handler(
    State(state): State<crate::handlers::AppState>,
    request: Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, Json<crate::models::ErrorResponse>)> {
    let source = state.security.source_from_request(&request);
    let body = axum::body::to_bytes(request.into_body(), MAX_REQUEST_BODY_BYTES)
        .await
        .map_err(|error| {
            security_reply(&SecurityError::Internal(format!(
                "login body rejected: {error}"
            )))
        })?;
    let login: LoginRequest = serde_json::from_slice(&body)
        .map_err(|_| security_reply(&SecurityError::InvalidCredentials))?;
    let receipt = state
        .security
        .login(&source, &login.password, unix_now())
        .map_err(|error| security_reply(&error))?;
    let cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; Max-Age={SESSION_ABSOLUTE_SECONDS}{}; HttpOnly; SameSite=Strict",
        receipt.session_cookie,
        if state.security.tls_terminated {
            "; Secure"
        } else {
            ""
        }
    );
    let mut response = Json(serde_json::json!({
        "authenticated": true,
        "csrf_token": receipt.csrf_token,
        "expires_at": receipt.expires_at
    }))
    .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie.parse().map_err(|_| {
            security_reply(&SecurityError::Internal(
                "session cookie construction failed".to_string(),
            ))
        })?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-store, max-age=0"
            .parse()
            .unwrap_or_else(|_| header::HeaderValue::from_static("no-store")),
    );
    Ok(response)
}

/// Revoke the current browser session.
#[allow(
    clippy::needless_pass_by_value,
    clippy::unused_async,
    reason = "axum handler and HeaderMap extractors own request parts"
)]
///
/// # Errors
///
/// Returns the named cookie or state-lock blocker.
pub async fn logout_handler(
    State(state): State<crate::handlers::AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<crate::models::ErrorResponse>)> {
    state
        .security
        .logout(&headers)
        .map_err(|error| security_reply(&error))?;
    Ok(Json(serde_json::json!({ "authenticated": false })))
}

/// Return visible transport and session-policy evidence.
#[allow(
    clippy::needless_pass_by_value,
    clippy::unused_async,
    reason = "axum handler and HeaderMap extractors own request parts"
)]
pub async fn security_handler(
    State(state): State<crate::handlers::AppState>,
    headers: HeaderMap,
) -> Json<SecurityView> {
    Json(state.security.security_view(&headers, unix_now()))
}

/// Convert an admission error to the common JSON response shape.
pub(crate) fn security_reply(
    error: &SecurityError,
) -> (StatusCode, Json<crate::models::ErrorResponse>) {
    (
        error.status(),
        Json(crate::models::ErrorResponse {
            error: error.message(),
        }),
    )
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (key, value) = cookie.trim().split_once('=')?;
                (key == name).then(|| value.to_string())
            })
        })
}

/// Return current UTC epoch seconds for request policy checks.
pub(crate) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        reason = "security unit tests use fixed header and verifier fixtures"
    )]

    use super::*;
    use argon2::password_hash::{PasswordHasher, SaltString};
    use axum::Router;
    use axum::routing::get;
    use tower::ServiceExt;
    fn configured() -> SecurityState {
        let salt = SaltString::from_b64("dGVzdHNhbHQ").expect("fixed test salt");
        let hash = Argon2::default()
            .hash_password(b"correct horse", &salt)
            .expect("test hash")
            .to_string();
        SecurityState::new(
            Some(hash),
            true,
            true,
            Some("admin.example.test".to_string()),
        )
    }

    fn headers(receipt: &LoginReceipt, csrf: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, "admin.example.test".parse().unwrap());
        headers.insert(
            header::COOKIE,
            format!("{SESSION_COOKIE}={}", receipt.session_cookie)
                .parse()
                .unwrap(),
        );
        if csrf {
            headers.insert(CSRF_HEADER, receipt.csrf_token.parse().unwrap());
        }
        headers
    }

    #[test]
    fn security_login_success_and_csrf() {
        let security = configured();
        let receipt = security
            .login("127.0.0.1", "correct horse", 1_000)
            .expect("login");
        assert!(
            security
                .authenticate(&headers(&receipt, false), &Method::GET, 1_001)
                .is_ok()
        );
        assert_eq!(
            security.authenticate(&headers(&receipt, false), &Method::POST, 1_001),
            Err(SecurityError::InvalidCSRF)
        );
        assert!(
            security
                .authenticate(&headers(&receipt, true), &Method::POST, 1_001)
                .is_ok()
        );
    }

    #[test]
    fn security_login_throttles_per_source() {
        let security = configured();
        for _ in 0..MAX_LOGIN_FAILURES {
            assert_eq!(
                security.login("10.0.0.1", "wrong", 2_000),
                Err(SecurityError::InvalidCredentials)
            );
        }
        assert!(matches!(
            security.login("10.0.0.1", "correct horse", 2_001),
            Err(SecurityError::Throttled { .. })
        ));
        assert!(security.login("10.0.0.2", "correct horse", 2_001).is_ok());
    }

    #[test]
    fn security_session_expiry_and_origin_are_visible() {
        let security = configured();
        let receipt = security
            .login("127.0.0.1", "correct horse", 3_000)
            .expect("login");
        assert_eq!(
            security.authenticate(
                &headers(&receipt, true),
                &Method::GET,
                3_000 + SESSION_IDLE_SECONDS + 1
            ),
            Err(SecurityError::ExpiredSession)
        );
        let mut wrong_host = headers(&receipt, true);
        wrong_host.insert(header::HOST, "client.example.test".parse().unwrap());
        assert_eq!(
            security.authenticate(&wrong_host, &Method::GET, 3_001),
            Err(SecurityError::InvalidOrigin(
                "client.example.test".to_string()
            ))
        );
    }

    #[test]
    fn security_transport_state_is_visible() {
        let security = configured();
        let view = security.security_view(&HeaderMap::new(), 4_000);
        assert_eq!(view.transport, "https_via_caddy");
        assert!(view.tls_terminated_at_proxy);
        assert!(!view.admin_authenticated);
        assert_eq!(view.recent_auth_window_seconds, RECENT_AUTH_SECONDS);
    }

    #[test]
    fn security_throttle_source_uses_only_validated_proxy_address() {
        let security = configured();
        let forwarded = Request::builder()
            .header("x-forwarded-for", "192.0.2.10, 127.0.0.1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(security.source_from_request(&forwarded), "192.0.2.10");
        let malformed = Request::builder()
            .header("x-forwarded-for", "not-an-ip")
            .body(Body::empty())
            .unwrap();
        assert_eq!(security.source_from_request(&malformed), "127.0.0.1");
    }

    #[test]
    fn security_body_limit_is_bounded_before_handler_buffering() {
        assert_eq!(MAX_REQUEST_BODY_BYTES, 1_048_576);
    }

    #[tokio::test]
    async fn security_public_client_route_is_separate_from_admin_cookie_auth() {
        let security = Arc::new(configured());
        let app = Router::new()
            .route("/api/v1/client/version", get(|| async { StatusCode::OK }))
            .route("/api/admin/login", get(|| async { StatusCode::OK }))
            .route("/api/health", get(|| async { StatusCode::OK }))
            .route("/api/vitals", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                security,
                security_middleware,
            ));
        let public_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/client/version")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(public_response.status(), StatusCode::OK);
        let login_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/login")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
        let health_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health_response.status(), StatusCode::OK);
        let admin_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/vitals")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(admin_response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn security_unconfigured_fails_closed() {
        let security = SecurityState::unconfigured();
        assert_eq!(
            security.login("local", "anything", 4_000),
            Err(SecurityError::NotConfigured)
        );
    }
}
