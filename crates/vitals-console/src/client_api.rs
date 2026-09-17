//! Versioned Stage 7 Apple-client HTTP contract.
//!
//! Version and registry discovery plus database-backed enrollment are implemented. Writes, status
//! beacons, and projections return stable unavailable errors until their owning tasks land.

use axum::Json;
use axum::Router;
use axum::extract::{Extension, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{any, get, post};
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::pkcs8::DecodePublicKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

const API_MAJOR: u32 = 1;
const API_MINOR: u32 = 0;
const VERSION_HEADER: &str = "x-anxietywatch-api-version";
const REGISTRY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vitals/generated/registry.json"
));

/// The generated `OpenAPI` document for `/api/v1/client/*`.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "AnxietyWatch Client API",
        version = "1.0.0",
        description = "Versioned contract for the thin Apple client; registry identifiers remain in registry.json"
    ),
    paths(
        client_version_handler,
        client_openapi_handler,
        client_registry_handler,
        client_enroll_handler,
        client_write_handler,
        client_beacon_handler,
        client_projections_handler
    ),
    components(schemas(
        ClientAuthHeaders,
        ClientVersionResponse,
        RegistryDiscoveryResponse,
        EnrollmentClaimRequest,
        EnrollmentCredentialResponse,
        ClientWriteBatch,
        ClientWriteItem,
        ClientWriteResponse,
        ItemAcknowledgement,
        HealthBeaconRequest,
        HealthTypeFact,
        ProjectionQuery,
        ProjectionPage,
        ProjectionPoint,
        ClientErrorResponse
    )),
    tags((name = "apple-client", description = "Versioned thin Apple-client contract"))
)]
pub struct ClientApiDoc;

/// Build the routes mounted by the Rust HTTP service.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v1/client/version", get(client_version_handler))
        .route("/api/v1/client/openapi.json", get(client_openapi_handler))
        .route("/api/v1/client/registry", get(client_registry_handler))
        .route("/api/v1/client/enroll", post(client_enroll_handler))
        .route("/api/v1/client/items", post(client_write_handler))
        .route("/api/v1/client/status", post(client_beacon_handler))
        .route(
            "/api/v1/client/projections",
            get(client_projections_handler),
        )
        .route("/api/v1/client/*path", any(client_not_found_handler))
}

/// RFC 9421 request-authentication fields fixed by Amendment 09.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientAuthHeaders {
    /// API major sent as `X-AnxietyWatch-API-Version`.
    pub api_version: String,
    /// Per-device client id.
    pub client_id: Uuid,
    /// Active P-256 key id.
    pub key_id: Uuid,
    /// Fresh UUID for durable replay rejection.
    pub request_id: Uuid,
    /// RFC 9530 digest of the request body.
    pub content_digest: Option<String>,
    /// RFC 9421 signature input.
    pub signature_input: String,
    /// RFC 9421 P-256 signature.
    pub signature: String,
}

/// API compatibility response. This endpoint is safe before enrollment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientVersionResponse {
    /// Contract version implemented by this server.
    pub api_version: String,
    /// Supported path major.
    pub supported_major: u32,
    /// Current additive minor version.
    pub supported_minor: u32,
    /// Oldest client major accepted for writes.
    pub minimum_client_major: u32,
    /// Authentication profile required once implemented.
    pub authentication_profile: String,
    /// Server observation time in UTC.
    pub server_time: String,
}

/// Registry discovery wraps the generated JSON without restating its enums in `OpenAPI`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegistryDiscoveryResponse {
    /// API contract version carrying this registry.
    pub api_version: String,
    /// SHA-256 of the exact generated registry JSON.
    pub registry_sha256: String,
    /// Registry JSON decoded as an open object; identifiers remain registry-owned.
    #[schema(value_type = Object)]
    pub document: serde_json::Value,
    /// Server observation time in UTC.
    pub server_time: String,
}

/// Future one-time enrollment claim. Unknown JSON fields are intentionally ignored.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnrollmentClaimRequest {
    /// API major from the enrollment artifact.
    pub api_major: u32,
    /// Single-use enrollment row id.
    pub enrollment_id: Uuid,
    /// Opaque 256-bit token, base64url encoded.
    pub token: String,
    /// Stable server instance expected by the artifact.
    pub server_instance_id: Uuid,
    /// User-visible device label.
    pub device_label: String,
    /// P-256 public key encoded as SPKI DER and then base64.
    pub public_key_spki_der_b64: String,
    /// Must be `ecdsa-p256-sha256` in Amendment 09.
    pub algorithm: String,
    /// Client build string for compatibility evidence.
    pub app_version: String,
}

/// Future successful enrollment result, included as a shared schema but not advertised as success.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnrollmentCredentialResponse {
    /// API contract version that issued the credential.
    pub api_version: String,
    /// Stable per-device client identifier.
    pub client_id: Uuid,
    /// Identifier of the enrolled public key.
    pub key_id: Uuid,
    /// Server-enforced least-privilege scopes.
    pub scopes: Vec<String>,
    /// Credential issue instant in UTC.
    pub issued_at: String,
    /// Credential expiry instant in UTC.
    pub expires_at: String,
    /// Server response instant in UTC.
    pub server_time: String,
}

/// A bounded batch of credential-neutral outbox items.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientWriteBatch {
    /// Client-minted transport batch identifier.
    pub batch_id: Uuid,
    /// Instant the client formed the batch, distinct from item subject time.
    pub recorded_at: String,
    /// Credential-neutral immutable outbox items.
    pub items: Vec<ClientWriteItem>,
}

/// One immutable outbox item. Class remains a string so unknown future classes survive decoding.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientWriteItem {
    /// Stable client-minted idempotency identifier.
    pub id: Uuid,
    /// Open item class owned by the server contracts.
    pub class: String,
    /// Item payload schema version.
    pub schema_version: u32,
    /// Class-specific payload; never a precomputed request signature.
    #[schema(value_type = Object)]
    pub content: serde_json::Value,
}

/// Future item-level write result. HTTP success alone never deletes outbox payloads.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientWriteResponse {
    /// API contract version that processed the batch.
    pub api_version: String,
    /// Batch identifier copied from the request.
    pub batch_id: Uuid,
    /// One terminal or retryable outcome per submitted item.
    pub acknowledgements: Vec<ItemAcknowledgement>,
    /// Server response instant in UTC.
    pub server_time: String,
}

/// One item outcome. `status` is open for forward-compatible unknown values.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ItemAcknowledgement {
    /// Submitted item identifier.
    pub item_id: Uuid,
    /// Open status; current values include created, replayed, rejected, conflict, and retryable.
    pub status: String,
    /// Stable machine-readable reason when status needs detail.
    pub code: Option<String>,
    /// Human-readable outcome or named blocker.
    pub message: String,
    /// Whether a later transport attempt may make progress.
    pub retryable: bool,
}

/// Operational phone evidence; it carries no measurement value.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthBeaconRequest {
    /// Client-minted idempotency identifier for this assertion.
    pub beacon_id: Uuid,
    /// Instant at which the phone made the assertion.
    pub asserted_at: String,
    /// App build/version supplying the evidence.
    pub app_version: String,
    /// Digest of the registry used by the client.
    pub registry_sha256: String,
    /// Number of unacknowledged outbox items.
    pub outbox_count: u64,
    /// Age of the oldest unacknowledged item, when one exists.
    pub oldest_outbox_age_s: Option<u64>,
    /// Most recent item-level server acknowledgement instant.
    pub last_server_ack_at: Option<String>,
    /// Per-HealthKit-type operational facts without measurements.
    pub types: Vec<HealthTypeFact>,
}

/// Independent facts for one `HealthKit` identifier; no field claims read access was granted.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthTypeFact {
    /// Original `HealthKit` type identifier, not a generated physiological enum.
    pub healthkit_type: String,
    /// Whether this type is enabled in client settings.
    pub configured: bool,
    /// Outcome of requesting authorization; never a claim that read access is granted.
    pub authorization_request_outcome: Option<String>,
    /// Outcome of registering the observer.
    pub observer_registration_outcome: Option<String>,
    /// Outcome of requesting iOS background delivery.
    pub background_delivery_outcome: Option<String>,
    /// Last anchored-query outcome.
    pub last_query_outcome: Option<String>,
    /// Last query attempt instant.
    pub last_query_at: Option<String>,
    /// Last matching `HealthKit` sample instant, when observed.
    pub last_matching_sample_at: Option<String>,
    /// Last Pi item acknowledgement for this type.
    pub last_pi_ack_at: Option<String>,
}

/// Projection query uses UTC windows and an opaque server cursor.
#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ProjectionQuery {
    /// Registry-owned metric name.
    pub metric: Option<String>,
    /// Inclusive UTC window start.
    pub window_start: Option<String>,
    /// Exclusive UTC window end.
    pub window_end: Option<String>,
    /// Opaque server-issued pagination cursor.
    pub cursor: Option<String>,
}

/// Future server projection page, included for fixture compatibility only.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProjectionPage {
    /// API contract version carrying the projection.
    pub api_version: String,
    /// Registry-owned metric name.
    pub metric: String,
    /// Most recent source observation represented, when one exists.
    pub observed_at: Option<String>,
    /// Instant at which the Pi generated this projection.
    pub generated_at: String,
    /// Projection points with explicit null values for gaps.
    pub points: Vec<ProjectionPoint>,
    /// UTC start/end pairs naming missing coverage.
    pub missing_intervals: Vec<Vec<String>>,
    /// Opaque cursor for the next page.
    pub next_cursor: Option<String>,
}

/// Missing physiological data is represented by `null`, never zero or interpolation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProjectionPoint {
    /// Point timestamp in UTC.
    pub timestamp: String,
    /// Server-projected value, or null when not measured.
    pub value: Option<f64>,
}

/// Stable client API error envelope.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientErrorResponse {
    /// API version of this error envelope.
    pub api_version: String,
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable error summary without sensitive payload content.
    pub message: String,
    /// Whether retrying later without user action may progress.
    pub retryable: bool,
    /// Named capability, task, version, or value that blocked the operation.
    pub blocker: String,
    /// Client-requested major when one was supplied.
    pub requested_major: Option<u32>,
    /// Server-supported major.
    pub supported_major: u32,
    /// Server response instant in UTC.
    pub server_time: String,
}

/// HTTP status plus a boxed stable client error body.
#[derive(Debug)]
pub struct ClientApiError {
    status: StatusCode,
    body: Box<ClientErrorResponse>,
}

impl IntoResponse for ClientApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(*self.body)).into_response()
    }
}

fn utc_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn error(
    status: StatusCode,
    code: &str,
    message: &str,
    retryable: bool,
    blocker: String,
    requested_major: Option<u32>,
) -> ClientApiError {
    ClientApiError {
        status,
        body: Box::new(ClientErrorResponse {
            api_version: format!("{API_MAJOR}.{API_MINOR}"),
            code: code.to_string(),
            message: message.to_string(),
            retryable,
            blocker,
            requested_major,
            supported_major: API_MAJOR,
            server_time: utc_now(),
        }),
    }
}

fn validate_version(headers: &HeaderMap) -> Result<(), ClientApiError> {
    let raw = headers.get(VERSION_HEADER).ok_or_else(|| {
        error(
            StatusCode::BAD_REQUEST,
            "api_version_missing",
            "required API version header is absent",
            false,
            format!("{VERSION_HEADER}=missing; supported_major={API_MAJOR}"),
            None,
        )
    })?;
    let text = raw.to_str().map_err(|_| {
        error(
            StatusCode::BAD_REQUEST,
            "api_version_invalid",
            "API version header is not valid text",
            false,
            format!("{VERSION_HEADER}=non_text; supported_major={API_MAJOR}"),
            None,
        )
    })?;
    let major_text = text.split('.').next().unwrap_or_default();
    let major = major_text.parse::<u32>().map_err(|_| {
        error(
            StatusCode::BAD_REQUEST,
            "api_version_invalid",
            "API version header must begin with a numeric major",
            false,
            format!("{VERSION_HEADER}={text}; supported_major={API_MAJOR}"),
            None,
        )
    })?;
    if major != API_MAJOR {
        return Err(error(
            StatusCode::UPGRADE_REQUIRED,
            "api_version_incompatible",
            "client API major is incompatible; queued writes must be preserved",
            false,
            format!("requested_major={major}; supported_major={API_MAJOR}"),
            Some(major),
        ));
    }
    Ok(())
}

fn unavailable(capability: &str, owner: &str) -> ClientApiError {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "capability_unavailable",
        "operation is gated and has not been implemented",
        false,
        format!("capability={capability}; blocked_on={owner}"),
        Some(API_MAJOR),
    )
}

/// Discover API compatibility before enrollment.
#[utoipa::path(
    get,
    path = "/api/v1/client/version",
    tag = "apple-client",
    responses(
        (status = 200, description = "API compatibility facts", body = ClientVersionResponse)
    )
)]
#[allow(clippy::unused_async, reason = "axum handlers must be async")]
pub async fn client_version_handler() -> Json<ClientVersionResponse> {
    Json(ClientVersionResponse {
        api_version: format!("{API_MAJOR}.{API_MINOR}"),
        supported_major: API_MAJOR,
        supported_minor: API_MINOR,
        minimum_client_major: API_MAJOR,
        authentication_profile: "rfc9421-ecdsa-p256-sha256".to_string(),
        server_time: utc_now(),
    })
}

/// Return the generated `OpenAPI` document.
#[utoipa::path(
    get,
    path = "/api/v1/client/openapi.json",
    tag = "apple-client",
    responses((status = 200, description = "Generated OpenAPI", body = Object))
)]
#[allow(clippy::unused_async, reason = "axum handlers must be async")]
pub async fn client_openapi_handler() -> Json<utoipa::openapi::OpenApi> {
    Json(ClientApiDoc::openapi())
}

/// Return registry JSON exactly as generated, wrapped with its digest.
///
/// # Errors
///
/// Returns a stable error for a missing/malformed/incompatible API version or malformed generated
/// registry JSON.
#[utoipa::path(
    get,
    path = "/api/v1/client/registry",
    tag = "apple-client",
    params(("X-AnxietyWatch-API-Version" = String, Header, description = "Supported API major")),
    responses(
        (status = 200, description = "Generated registry discovery", body = RegistryDiscoveryResponse),
        (status = 400, description = "Missing or malformed API version", body = ClientErrorResponse),
        (status = 426, description = "Incompatible API major", body = ClientErrorResponse),
        (status = 500, description = "Generated registry is malformed", body = ClientErrorResponse)
    )
)]
#[allow(clippy::unused_async, reason = "axum handlers must be async")]
pub async fn client_registry_handler(
    headers: HeaderMap,
) -> Result<Json<RegistryDiscoveryResponse>, ClientApiError> {
    validate_version(&headers)?;
    let document = serde_json::from_str(REGISTRY_JSON).map_err(|parse_error| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "registry_invalid",
            "generated registry JSON could not be decoded",
            false,
            format!("registry_parse={parse_error}"),
            Some(API_MAJOR),
        )
    })?;
    let digest = Sha256::digest(REGISTRY_JSON.as_bytes());
    Ok(Json(RegistryDiscoveryResponse {
        api_version: format!("{API_MAJOR}.{API_MINOR}"),
        registry_sha256: hex::encode(digest),
        document,
        server_time: utc_now(),
    }))
}

/// Claim an operator-created enrollment artifact and issue a scoped public-key credential.
///
/// The request is intentionally not signed: the single-use artifact is the enrollment proof and
/// is transported only to the configured origin. The database transaction consumes the artifact
/// and inserts the credential together, so a lost response cannot make the artifact reusable.
///
/// # Errors
///
/// Returns a named version, validation, database, or consumed-artifact blocker.
#[utoipa::path(
    post,
    path = "/api/v1/client/enroll",
    tag = "apple-client",
    request_body = EnrollmentClaimRequest,
    responses(
        (status = 201, description = "Device credential issued", body = EnrollmentCredentialResponse),
        (status = 400, description = "Malformed enrollment claim", body = ClientErrorResponse),
        (status = 409, description = "Enrollment artifact already consumed", body = ClientErrorResponse),
        (status = 426, description = "Incompatible enrollment major", body = ClientErrorResponse),
        (status = 503, description = "Enrollment database unavailable", body = ClientErrorResponse)
    )
)]
#[expect(
    clippy::too_many_lines,
    reason = "the enrollment boundary keeps decode, validation, persistence, and response mapping together"
)]
pub async fn client_enroll_handler(
    Extension(database): Extension<Option<sqlx::PgPool>>,
    Json(request): Json<EnrollmentClaimRequest>,
) -> Result<(StatusCode, Json<EnrollmentCredentialResponse>), ClientApiError> {
    if request.api_major != API_MAJOR {
        return Err(error(
            StatusCode::UPGRADE_REQUIRED,
            "api_version_incompatible",
            "enrollment API major is incompatible",
            false,
            format!(
                "requested_major={}; supported_major={API_MAJOR}",
                request.api_major
            ),
            Some(request.api_major),
        ));
    }
    if request.algorithm != "ecdsa-p256-sha256" {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "enrollment_algorithm_invalid",
            "enrollment algorithm is not supported",
            false,
            format!(
                "algorithm={}; required=ecdsa-p256-sha256",
                request.algorithm
            ),
            Some(API_MAJOR),
        ));
    }
    let database =
        database.ok_or_else(|| unavailable("enrollment", "VITALS_CONSOLE_DATABASE_URL"))?;
    let der = base64::engine::general_purpose::STANDARD
        .decode(&request.public_key_spki_der_b64)
        .map_err(|decode_error| {
            error(
                StatusCode::BAD_REQUEST,
                "enrollment_public_key_invalid",
                "enrollment public key is not valid base64",
                false,
                format!("public_key_encoding={decode_error}"),
                Some(API_MAJOR),
            )
        })?;
    let public_key = p256::PublicKey::from_public_key_der(&der).map_err(|key_error| {
        error(
            StatusCode::BAD_REQUEST,
            "enrollment_public_key_invalid",
            "enrollment public key is not a valid P-256 SPKI value",
            false,
            format!("public_key_der={key_error}"),
            Some(API_MAJOR),
        )
    })?;
    let public_key_sec1 = public_key.to_encoded_point(false);
    let credential = crate::db::claim_mobile_credential(
        &database,
        request.enrollment_id,
        &request.token,
        request.server_instance_id,
        public_key_sec1.as_bytes(),
        &request.device_label,
        Utc::now(),
    )
    .await
    .map_err(|reason| {
        let status = if reason.contains("already consumed") {
            StatusCode::CONFLICT
        } else if reason.contains("database")
            || reason.contains("insert")
            || reason.contains("commit")
        {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::BAD_REQUEST
        };
        error(
            status,
            "enrollment_rejected",
            "enrollment claim was not accepted",
            false,
            reason,
            Some(API_MAJOR),
        )
    })?;
    let issued_at =
        chrono::DateTime::from_timestamp(credential.issued_at_epoch_s, 0).ok_or_else(|| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "enrollment_time_invalid",
                "issued credential time could not be represented",
                false,
                format!("issued_at_epoch_s={}", credential.issued_at_epoch_s),
                Some(API_MAJOR),
            )
        })?;
    let expires_at = chrono::DateTime::from_timestamp(credential.expires_at_epoch_s, 0)
        .ok_or_else(|| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "enrollment_time_invalid",
                "credential expiry time could not be represented",
                false,
                format!("expires_at_epoch_s={}", credential.expires_at_epoch_s),
                Some(API_MAJOR),
            )
        })?;
    Ok((
        StatusCode::CREATED,
        Json(EnrollmentCredentialResponse {
            api_version: format!("{API_MAJOR}.{API_MINOR}"),
            client_id: credential.client_id,
            key_id: credential.key_id,
            scopes: credential.scopes,
            issued_at: issued_at.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            server_time: Utc::now().to_rfc3339(),
        }),
    ))
}

/// Item-write placeholder with item-level response schemas but no success advertisement.
///
/// # Errors
///
/// Returns a version error or the named domain/platform contract gate.
#[utoipa::path(
    post,
    path = "/api/v1/client/items",
    tag = "apple-client",
    params(
        ("X-AnxietyWatch-API-Version" = String, Header, description = "Supported API major"),
        ("X-AnxietyWatch-Client-ID" = String, Header, description = "Enrolled client UUID"),
        ("X-AnxietyWatch-Key-ID" = String, Header, description = "Active P-256 key UUID"),
        ("X-AnxietyWatch-Request-ID" = String, Header, description = "Fresh replay-protection UUID"),
        ("Content-Digest" = String, Header, description = "RFC 9530 body digest"),
        ("Signature-Input" = String, Header, description = "RFC 9421 covered components and times"),
        ("Signature" = String, Header, description = "RFC 9421 ecdsa-p256-sha256 signature")
    ),
    request_body = ClientWriteBatch,
    responses(
        (status = 400, description = "Missing or malformed API version", body = ClientErrorResponse),
        (status = 401, description = "Missing or invalid request signature", body = ClientErrorResponse),
        (status = 403, description = "Credential scope does not permit this operation", body = ClientErrorResponse),
        (status = 409, description = "Durable request id was already admitted", body = ClientErrorResponse),
        (status = 426, description = "Incompatible API major", body = ClientErrorResponse),
        (status = 503, description = "Writes are gated on domain/platform contracts", body = ClientErrorResponse)
    )
)]
#[allow(clippy::unused_async, reason = "axum handlers must be async")]
pub async fn client_write_handler(
    headers: HeaderMap,
    Json(_request): Json<ClientWriteBatch>,
) -> Result<Json<ClientWriteResponse>, ClientApiError> {
    validate_version(&headers)?;
    Err(unavailable("item_writes", "T-M5b_T-X2_T-X3_T-X4"))
}

/// Status-beacon placeholder. A valid request still returns a named gate.
///
/// # Errors
///
/// Returns a version error or the named T-X2 capability gate.
#[utoipa::path(
    post,
    path = "/api/v1/client/status",
    tag = "apple-client",
    params(
        ("X-AnxietyWatch-API-Version" = String, Header, description = "Supported API major"),
        ("X-AnxietyWatch-Client-ID" = String, Header, description = "Enrolled client UUID"),
        ("X-AnxietyWatch-Key-ID" = String, Header, description = "Active P-256 key UUID"),
        ("X-AnxietyWatch-Request-ID" = String, Header, description = "Fresh replay-protection UUID"),
        ("Content-Digest" = String, Header, description = "RFC 9530 body digest"),
        ("Signature-Input" = String, Header, description = "RFC 9421 covered components and times"),
        ("Signature" = String, Header, description = "RFC 9421 ecdsa-p256-sha256 signature")
    ),
    request_body = HealthBeaconRequest,
    responses(
        (status = 400, description = "Missing or malformed API version", body = ClientErrorResponse),
        (status = 401, description = "Missing or invalid request signature", body = ClientErrorResponse),
        (status = 403, description = "Credential scope does not permit this operation", body = ClientErrorResponse),
        (status = 409, description = "Durable request id was already admitted", body = ClientErrorResponse),
        (status = 426, description = "Incompatible API major", body = ClientErrorResponse),
        (status = 503, description = "Status beacon is gated on T-X2", body = ClientErrorResponse)
    )
)]
#[allow(clippy::unused_async, reason = "axum handlers must be async")]
pub async fn client_beacon_handler(
    headers: HeaderMap,
    Json(_request): Json<HealthBeaconRequest>,
) -> Result<StatusCode, ClientApiError> {
    validate_version(&headers)?;
    Err(unavailable("status_beacon", "T-X2"))
}

/// Projection placeholder. Its success schema is present but no 2xx response is advertised.
///
/// # Errors
///
/// Returns a version error or the named M3/M4 capability gate.
#[utoipa::path(
    get,
    path = "/api/v1/client/projections",
    tag = "apple-client",
    params(
        ProjectionQuery,
        ("X-AnxietyWatch-API-Version" = String, Header, description = "Supported API major"),
        ("X-AnxietyWatch-Client-ID" = String, Header, description = "Enrolled client UUID"),
        ("X-AnxietyWatch-Key-ID" = String, Header, description = "Active P-256 key UUID"),
        ("X-AnxietyWatch-Request-ID" = String, Header, description = "Fresh replay-protection UUID"),
        ("Signature-Input" = String, Header, description = "RFC 9421 covered components and times"),
        ("Signature" = String, Header, description = "RFC 9421 ecdsa-p256-sha256 signature")
    ),
    responses(
        (status = 400, description = "Missing or malformed API version", body = ClientErrorResponse),
        (status = 401, description = "Missing or invalid request signature", body = ClientErrorResponse),
        (status = 403, description = "Credential scope does not permit this operation", body = ClientErrorResponse),
        (status = 409, description = "Durable request id was already admitted", body = ClientErrorResponse),
        (status = 426, description = "Incompatible API major", body = ClientErrorResponse),
        (status = 503, description = "Projections are gated on M3/M4", body = ClientErrorResponse)
    )
)]
#[allow(clippy::unused_async, reason = "axum handlers must be async")]
pub async fn client_projections_handler(
    headers: HeaderMap,
    Query(_query): Query<ProjectionQuery>,
) -> Result<Json<ProjectionPage>, ClientApiError> {
    validate_version(&headers)?;
    Err(unavailable("projections", "admin_M3_M4"))
}

#[allow(clippy::unused_async, reason = "axum handlers must be async")]
async fn client_not_found_handler() -> impl IntoResponse {
    error(
        StatusCode::NOT_FOUND,
        "client_route_not_found",
        "requested client API route does not exist",
        false,
        "route=unknown; supported_prefix=/api/v1/client".to_string(),
        Some(API_MAJOR),
    )
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "contract tests fail loudly on malformed static requests, fixtures, or generated OpenAPI"
)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn json_response(request: Request<Body>) -> (StatusCode, serde_json::Value) {
        let response = router::<()>()
            .layer(axum::Extension(None::<sqlx::PgPool>))
            .oneshot(request)
            .await
            .expect("router responds");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("response body collects")
            .to_bytes();
        let value = serde_json::from_slice(&bytes).expect("response is JSON");
        (status, value)
    }

    #[tokio::test]
    async fn client_contract_version_and_registry_are_available_before_enrollment() {
        let version_request = Request::builder()
            .uri("/api/v1/client/version")
            .body(Body::empty())
            .expect("request builds");
        let (version_status, version) = json_response(version_request).await;
        assert_eq!(version_status, StatusCode::OK);
        assert_eq!(version["supported_major"], 1);

        let registry_request = Request::builder()
            .uri("/api/v1/client/registry")
            .header(VERSION_HEADER, "1")
            .body(Body::empty())
            .expect("request builds");
        let (registry_status, registry) = json_response(registry_request).await;
        assert_eq!(registry_status, StatusCode::OK);
        assert!(registry["document"]["type"].is_object());
    }

    #[tokio::test]
    async fn client_contract_enrollment_fixture_covers_gate_and_incompatible_major() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/enrollment-request.json"
        ));
        let claim: EnrollmentClaimRequest =
            serde_json::from_str(fixture).expect("enrollment fixture decodes");
        assert_eq!(claim.api_major, API_MAJOR);
        assert_eq!(claim.algorithm, "ecdsa-p256-sha256");

        let gated_request = Request::builder()
            .method("POST")
            .uri("/api/v1/client/enroll")
            .header("content-type", "application/json")
            .body(Body::from(fixture))
            .expect("request builds");
        let (gated_status, gated_body) = json_response(gated_request).await;
        assert_eq!(gated_status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(gated_body["code"], "capability_unavailable");
        assert!(
            gated_body["blocker"]
                .as_str()
                .is_some_and(|value| value.contains("VITALS_CONSOLE_DATABASE_URL"))
        );

        let incompatible_fixture = fixture.replacen("\"api_major\": 1", "\"api_major\": 2", 1);
        let incompatible_request = Request::builder()
            .method("POST")
            .uri("/api/v1/client/enroll")
            .header("content-type", "application/json")
            .body(Body::from(incompatible_fixture))
            .expect("request builds");
        let (incompatible_status, incompatible_body) = json_response(incompatible_request).await;
        assert_eq!(incompatible_status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(incompatible_body["code"], "api_version_incompatible");
        assert_eq!(incompatible_body["requested_major"], 2);
    }

    #[tokio::test]
    async fn client_contract_gated_write_returns_named_unavailable_without_success() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/write-batch-request.json"
        ));
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/client/items")
            .header("content-type", "application/json")
            .header(VERSION_HEADER, "1.7")
            .body(Body::from(fixture))
            .expect("request builds");
        let (status, body) = json_response(request).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["code"], "capability_unavailable");
        assert_eq!(body["retryable"], false);
        assert!(
            body["blocker"]
                .as_str()
                .is_some_and(|v| v.contains("T-M5b"))
        );
    }

    #[tokio::test]
    async fn client_contract_incompatible_major_preserves_writes_and_names_versions() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/write-batch-request.json"
        ));
        let request = Request::builder()
            .method("POST")
            .uri("/api/v1/client/items")
            .header("content-type", "application/json")
            .header(VERSION_HEADER, "2")
            .body(Body::from(fixture))
            .expect("request builds");
        let (status, body) = json_response(request).await;
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(body["code"], "api_version_incompatible");
        assert_eq!(body["requested_major"], 2);
        assert!(
            body["message"]
                .as_str()
                .is_some_and(|v| v.contains("preserved"))
        );
    }

    #[test]
    fn client_contract_shared_fixtures_decode_and_ignore_unknown_fields() {
        let write: ClientWriteBatch = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/write-batch-request.json"
        )))
        .expect("write fixture decodes");
        assert_eq!(write.items.len(), 1);

        let ack: ClientWriteResponse = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/write-batch-ack-response.json"
        )))
        .expect("ack fixture decodes");
        assert_eq!(
            ack.acknowledgements
                .first()
                .map(|item| item.status.as_str()),
            Some("created")
        );

        let error: ClientErrorResponse = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/error-version-incompatible.json"
        )))
        .expect("error fixture with unknown field decodes");
        assert_eq!(error.code, "api_version_incompatible");

        let authentication: ClientErrorResponse = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/error-authentication-required.json"
        )))
        .expect("authentication fixture decodes");
        assert_eq!(authentication.code, "request_signature_missing");

        let replay: ClientErrorResponse = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/error-request-replayed.json"
        )))
        .expect("replay fixture decodes");
        assert_eq!(replay.code, "request_replayed");
    }

    #[test]
    fn client_contract_openapi_never_advertises_gated_successes_or_registry_enums() {
        let value = serde_json::to_value(ClientApiDoc::openapi()).expect("OpenAPI serializes");
        let enrollment_responses = value["paths"]["/api/v1/client/enroll"]["post"]["responses"]
            .as_object()
            .expect("enrollment responses object");
        assert!(enrollment_responses.contains_key("201"));
        for path in [
            "/api/v1/client/items",
            "/api/v1/client/status",
            "/api/v1/client/projections",
        ] {
            let operations = value["paths"][path]
                .as_object()
                .expect("path has operation");
            for operation in operations.values() {
                let responses = operation["responses"]
                    .as_object()
                    .expect("responses object");
                assert!(responses.keys().all(|status| !status.starts_with('2')));
            }
        }
        let serialized = serde_json::to_string(&value).expect("OpenAPI JSON serializes");
        assert!(!serialized.contains("VitalsType"));
        assert!(!serialized.contains("VitalsSource"));
        assert!(!serialized.contains("polar_h10"));
    }
}
