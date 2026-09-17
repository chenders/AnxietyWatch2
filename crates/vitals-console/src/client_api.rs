//! Versioned Stage 7 Apple-client HTTP contract.
//!
//! Version and registry discovery, enrollment, and authenticated status-beacon admission are
//! implemented. Domain writes and projections remain gated until their owning tasks land.

use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Extension, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{any, get, post};
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use p256::ecdsa::signature::Verifier;
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
        HealthBeaconResponse,
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

/// One-time operator response containing an enrollment token that is never returned again.
#[derive(Debug, Serialize, ToSchema)]
struct AdminEnrollmentArtifactResponse {
    api_major: u32,
    origin: String,
    server_instance_id: Uuid,
    enrollment_id: Uuid,
    server_display_label: String,
    device_label: String,
    scopes: Vec<String>,
    expires_at: String,
    token: String,
}

/// Validated request for creating a new one-time enrollment artifact.
#[derive(Debug, Deserialize)]
pub struct AdminEnrollmentArtifactRequest {
    /// Operator-visible label for the device that will claim the artifact.
    pub device_label: String,
    /// Requested least-privilege mobile scopes.
    pub scopes: Vec<String>,
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
    /// Last time the phone changed any reported status fact.
    pub changed_at: String,
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

/// Successful status-beacon admission response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HealthBeaconResponse {
    /// API contract version that processed the beacon.
    pub api_version: String,
    /// Client idempotency identifier copied from the request.
    pub beacon_id: Uuid,
    /// `accepted` for a new assertion or `replayed` for identical content.
    pub status: String,
    /// Server response instant in UTC.
    pub server_time: String,
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

const SIGNATURE_COVERED_COMPONENTS: &str = "(\"@method\" \"@authority\" \"@path\" \"content-digest\" \"content-type\" \"x-anxietywatch-api-version\" \"x-anxietywatch-client-id\" \"x-anxietywatch-key-id\" \"x-anxietywatch-request-id\")";

type MobileCredentialRow = (
    Vec<u8>,
    Vec<String>,
    chrono::DateTime<Utc>,
    Option<chrono::DateTime<Utc>>,
);

struct MobileSignature {
    client_id: Uuid,
    key_id: Uuid,
    request_id: Uuid,
    created: i64,
    expires: i64,
    content_digest: String,
    content_type: String,
    api_version: String,
    authority: String,
    signature_params: String,
    signature: Vec<u8>,
}

fn required_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, ClientApiError> {
    headers
        .get(name)
        .ok_or_else(|| {
            error(
                StatusCode::UNAUTHORIZED,
                "request_signature_missing",
                "required RFC 9421 request signature fields are absent",
                false,
                format!("{name}=missing"),
                Some(API_MAJOR),
            )
        })?
        .to_str()
        .map_err(|_| {
            error(
                StatusCode::UNAUTHORIZED,
                "request_signature_invalid",
                "RFC 9421 request signature fields are not valid text",
                false,
                format!("{name}=non_text"),
                Some(API_MAJOR),
            )
        })
}

fn parse_uuid_header(headers: &HeaderMap, name: &'static str) -> Result<Uuid, ClientApiError> {
    let value = required_header(headers, name)?;
    Uuid::parse_str(value).map_err(|_| {
        error(
            StatusCode::UNAUTHORIZED,
            "request_signature_invalid",
            "RFC 9421 request identity is not a UUID",
            false,
            format!("{name}={value}"),
            Some(API_MAJOR),
        )
    })
}

fn parse_signature_input(value: &str) -> Result<(i64, i64, String), ClientApiError> {
    let signature_params = value
        .strip_prefix("sig1=")
        .ok_or_else(|| {
            error(
                StatusCode::UNAUTHORIZED,
                "request_signature_invalid",
                "RFC 9421 signature label is invalid",
                false,
                "signature-input=missing-sig1".to_string(),
                Some(API_MAJOR),
            )
        })?
        .to_string();
    let suffix = value.strip_prefix("sig1=").ok_or_else(|| {
        error(
            StatusCode::UNAUTHORIZED,
            "request_signature_invalid",
            "RFC 9421 signature label is invalid",
            false,
            "signature-input=missing-sig1".to_string(),
            Some(API_MAJOR),
        )
    })?;
    let suffix = suffix
        .strip_prefix(SIGNATURE_COVERED_COMPONENTS)
        .ok_or_else(|| {
            error(
                StatusCode::UNAUTHORIZED,
                "request_signature_invalid",
                "RFC 9421 covered components do not match the client profile",
                false,
                "signature-input=covered-components-mismatch".to_string(),
                Some(API_MAJOR),
            )
        })?;
    let suffix = suffix.strip_prefix(";created=").ok_or_else(|| {
        error(
            StatusCode::UNAUTHORIZED,
            "request_signature_invalid",
            "RFC 9421 created parameter is missing",
            false,
            "signature-input=created-missing".to_string(),
            Some(API_MAJOR),
        )
    })?;
    let (created_text, expires_text) = suffix.split_once(";expires=").ok_or_else(|| {
        error(
            StatusCode::UNAUTHORIZED,
            "request_signature_invalid",
            "RFC 9421 expires parameter is missing",
            false,
            "signature-input=expires-missing".to_string(),
            Some(API_MAJOR),
        )
    })?;
    let created = created_text.parse::<i64>().map_err(|_| {
        error(
            StatusCode::UNAUTHORIZED,
            "request_signature_invalid",
            "RFC 9421 created parameter is not an integer",
            false,
            format!("created={created_text}"),
            Some(API_MAJOR),
        )
    })?;
    let expires = expires_text.parse::<i64>().map_err(|_| {
        error(
            StatusCode::UNAUTHORIZED,
            "request_signature_invalid",
            "RFC 9421 expires parameter is not an integer",
            false,
            format!("expires={expires_text}"),
            Some(API_MAJOR),
        )
    })?;
    Ok((created, expires, signature_params))
}

fn decode_signature(value: &str) -> Result<Vec<u8>, ClientApiError> {
    let encoded = value
        .strip_prefix("sig1=:")
        .and_then(|value| value.strip_suffix(':'))
        .ok_or_else(|| {
            error(
                StatusCode::UNAUTHORIZED,
                "request_signature_invalid",
                "RFC 9421 signature value is invalid",
                false,
                "signature=invalid-label".to_string(),
                Some(API_MAJOR),
            )
        })?;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|decode_error| {
            error(
                StatusCode::UNAUTHORIZED,
                "request_signature_invalid",
                "RFC 9421 signature value is not valid base64",
                false,
                format!("signature_encoding={decode_error}"),
                Some(API_MAJOR),
            )
        })
}

fn mobile_signature(headers: &HeaderMap, body: &[u8]) -> Result<MobileSignature, ClientApiError> {
    validate_version(headers)?;
    let client_id = parse_uuid_header(headers, "x-anxietywatch-client-id")?;
    let key_id = parse_uuid_header(headers, "x-anxietywatch-key-id")?;
    let request_id = parse_uuid_header(headers, "x-anxietywatch-request-id")?;
    let content_digest = required_header(headers, "content-digest")?.to_string();
    let content_type = required_header(headers, "content-type")?.to_string();
    let api_version = required_header(headers, VERSION_HEADER)?.to_string();
    let authority = required_header(headers, "host")?.to_string();
    let signature_input = required_header(headers, "signature-input")?;
    let (created, expires, signature_params) = parse_signature_input(signature_input)?;
    let now = Utc::now().timestamp();
    if expires < created || expires.saturating_sub(created) > 120 {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "client_signature_expired",
            "request signature lifetime is outside the permitted bound",
            false,
            format!("created={created}; expires={expires}; maximum_lifetime=120"),
            Some(API_MAJOR),
        ));
    }
    if created.saturating_sub(now).abs() > 300 || expires.saturating_sub(now) < -300 {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "client_clock_out_of_range",
            "client request clock is outside the permitted skew",
            false,
            format!("created={created}; expires={expires}; server_now={now}; skew_limit=300"),
            Some(API_MAJOR),
        ));
    }
    let expected_digest = format!(
        "sha-256=:{}:",
        base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(body))
    );
    if content_digest != expected_digest {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "content_digest_invalid",
            "request body digest does not match the signed content",
            false,
            format!("request_id={request_id}; digest=mismatch"),
            Some(API_MAJOR),
        ));
    }
    let signature = decode_signature(required_header(headers, "signature")?)?;
    Ok(MobileSignature {
        client_id,
        key_id,
        request_id,
        created,
        expires,
        content_digest,
        content_type,
        api_version,
        authority,
        signature_params,
        signature,
    })
}

/// Authenticate a signed mobile request and durably admit its request id.
///
/// The public key is loaded before body-specific work, the exact signed profile is verified, and
/// replay admission, last-used evidence, and audit evidence commit in one database transaction.
///
/// # Errors
///
/// Returns a named authentication, scope, replay, clock, or database blocker.
#[expect(
    clippy::too_many_lines,
    reason = "request admission keeps credential, signature, replay, and audit checks in one transaction boundary"
)]
async fn authenticate_mobile_request(
    database: &sqlx::PgPool,
    headers: &HeaderMap,
    body: &[u8],
    method: &str,
    path: &str,
    required_scope: &str,
) -> Result<(), ClientApiError> {
    let signature = mobile_signature(headers, body)?;
    let credential: Option<MobileCredentialRow> = sqlx::query_as(
        "SELECT public_key, scopes, expires_at, revoked_at FROM mobile_credentials\
             WHERE client_id = $1 AND key_id = $2",
    )
    .bind(signature.client_id)
    .bind(signature.key_id)
    .fetch_optional(database)
    .await
    .map_err(|database_error| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "mobile_authentication_unavailable",
            "mobile credential lookup failed",
            true,
            format!("database={database_error}"),
            Some(API_MAJOR),
        )
    })?;
    let Some((public_key, scopes, expires_at, revoked_at)) = credential else {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "credential_unknown",
            "mobile credential is not recognized",
            false,
            format!(
                "client_id={}; key_id={}",
                signature.client_id, signature.key_id
            ),
            Some(API_MAJOR),
        ));
    };
    if revoked_at.is_some() {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "credential_revoked",
            "mobile credential has been revoked",
            false,
            format!(
                "client_id={}; key_id={}; state=revoked",
                signature.client_id, signature.key_id
            ),
            Some(API_MAJOR),
        ));
    }
    if expires_at <= Utc::now() {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "credential_expired",
            "mobile credential has expired",
            false,
            format!(
                "client_id={}; key_id={}; expires_at={expires_at}",
                signature.client_id, signature.key_id
            ),
            Some(API_MAJOR),
        ));
    }
    if !scopes.iter().any(|scope| scope == required_scope) {
        return Err(error(
            StatusCode::FORBIDDEN,
            "credential_scope_denied",
            "mobile credential scope does not permit this operation",
            false,
            format!(
                "client_id={}; required_scope={required_scope}",
                signature.client_id
            ),
            Some(API_MAJOR),
        ));
    }
    let verifying_key = p256::ecdsa::VerifyingKey::from_sec1_bytes(&public_key).map_err(|_| {
        error(
            StatusCode::UNAUTHORIZED,
            "credential_invalid",
            "stored mobile public key is invalid",
            false,
            format!(
                "client_id={}; key_id={}; public_key=invalid",
                signature.client_id, signature.key_id
            ),
            Some(API_MAJOR),
        )
    })?;
    let ecdsa_signature = p256::ecdsa::Signature::from_der(&signature.signature).map_err(|_| {
        error(
            StatusCode::UNAUTHORIZED,
            "request_signature_invalid",
            "RFC 9421 signature is not a DER P-256 signature",
            false,
            format!("request_id={}", signature.request_id),
            Some(API_MAJOR),
        )
    })?;
    let signature_base = format!(
        "\"@method\": {method}\n\"@authority\": {}\n\"@path\": {path}\ncontent-digest: {}\ncontent-type: {}\nx-anxietywatch-api-version: {}\nx-anxietywatch-client-id: {}\nx-anxietywatch-key-id: {}\nx-anxietywatch-request-id: {}\n\"@signature-params\": {}",
        signature.authority,
        signature.content_digest,
        signature.content_type,
        signature.api_version,
        signature.client_id,
        signature.key_id,
        signature.request_id,
        signature.signature_params
    );
    verifying_key
        .verify(signature_base.as_bytes(), &ecdsa_signature)
        .map_err(|_| {
            error(
                StatusCode::UNAUTHORIZED,
                "request_signature_invalid",
                "RFC 9421 signature verification failed",
                false,
                format!(
                    "client_id={}; request_id={}",
                    signature.client_id, signature.request_id
                ),
                Some(API_MAJOR),
            )
        })?;
    let mut transaction = database.begin().await.map_err(|database_error| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "mobile_authentication_unavailable",
            "mobile request admission transaction failed",
            true,
            format!("database={database_error}"),
            Some(API_MAJOR),
        )
    })?;
    let admitted: Option<(Uuid,)> = sqlx::query_as(
        "INSERT INTO mobile_request_replays (client_id, request_id, request_digest, expires_at)\
         VALUES ($1, $2, $3, to_timestamp($4))\
         ON CONFLICT (client_id, request_id) DO NOTHING\
         RETURNING request_id",
    )
    .bind(signature.client_id)
    .bind(signature.request_id)
    .bind(sha2::Sha256::digest(body).as_slice())
    .bind(signature.expires)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|database_error| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "mobile_authentication_unavailable",
            "mobile request replay admission failed",
            true,
            format!("database={database_error}"),
            Some(API_MAJOR),
        )
    })?;
    if admitted.is_none() {
        return Err(error(
            StatusCode::CONFLICT,
            "request_replayed",
            "request id was already admitted",
            false,
            format!(
                "client_id={}; request_id={}",
                signature.client_id, signature.request_id
            ),
            Some(API_MAJOR),
        ));
    }
    sqlx::query("UPDATE mobile_credentials SET last_used_at = to_timestamp($3) WHERE client_id = $1 AND key_id = $2")
        .bind(signature.client_id)
        .bind(signature.key_id)
        .bind(signature.created)
        .execute(&mut *transaction)
        .await
        .map_err(|database_error| {
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "mobile_authentication_unavailable",
                "mobile credential use evidence could not be recorded",
                true,
                format!("database={database_error}"),
                Some(API_MAJOR),
            )
        })?;
    sqlx::query(
        "INSERT INTO credential_audit (client_id, key_id, action, actor, reason)\
         VALUES ($1, $2, 'used', $3, $4)",
    )
    .bind(signature.client_id)
    .bind(signature.key_id)
    .bind(format!("client-request:{}", signature.request_id))
    .bind(format!("{method} {path}"))
    .execute(&mut *transaction)
    .await
    .map_err(|database_error| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "mobile_authentication_unavailable",
            "mobile credential audit evidence could not be recorded",
            true,
            format!("database={database_error}"),
            Some(API_MAJOR),
        )
    })?;
    transaction.commit().await.map_err(|database_error| {
        error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "mobile_authentication_unavailable",
            "mobile request admission could not be committed",
            true,
            format!("database={database_error}"),
            Some(API_MAJOR),
        )
    })?;
    Ok(())
}

/// Create a single-use enrollment artifact for a mobile device.
///
/// The browser middleware has already checked the session and CSRF token; this handler additionally
/// requires password reauthentication within the bounded sensitive-action window. The response is
/// explicitly non-cacheable because it contains the only plaintext token reveal.
///
/// # Errors
///
/// Returns a named authentication, configuration, validation, or database blocker.
pub async fn admin_enrollment_artifact_handler(
    State(state): State<crate::handlers::AppState>,
    headers: HeaderMap,
    Json(request): Json<AdminEnrollmentArtifactRequest>,
) -> Result<axum::response::Response, ClientApiError> {
    state
        .security
        .require_recent_auth(&headers, Utc::now().timestamp())
        .map_err(|security_error| {
            error(
                StatusCode::UNAUTHORIZED,
                "recent_auth_required",
                "sensitive enrollment action requires recent operator reauthentication",
                false,
                format!("security={security_error:?}"),
                Some(API_MAJOR),
            )
        })?;
    let database = state.db.as_ref().ok_or_else(|| {
        unavailable(
            "enrollment_artifact_creation",
            "VITALS_CONSOLE_DATABASE_URL",
        )
    })?;
    let server_instance_id = state.server_instance_id.ok_or_else(|| {
        unavailable(
            "enrollment_artifact_creation",
            "VITALS_CONSOLE_SERVER_INSTANCE_ID",
        )
    })?;
    let origin = state.client_origin.clone().ok_or_else(|| {
        unavailable(
            "enrollment_artifact_creation",
            "VITALS_CONSOLE_CLIENT_ORIGIN",
        )
    })?;
    let artifact = crate::db::create_mobile_enrollment_artifact(
        database,
        server_instance_id,
        &request.device_label,
        &request.scopes,
        crate::security::AUTHENTICATED_ACTOR,
        Utc::now(),
    )
    .await
    .map_err(|reason| {
        let status = if reason.contains("blocked") || reason.contains("rejected") {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        error(
            status,
            "enrollment_artifact_creation_failed",
            "enrollment artifact could not be created",
            false,
            reason,
            Some(API_MAJOR),
        )
    })?;
    let mut response = Json(AdminEnrollmentArtifactResponse {
        api_major: API_MAJOR,
        origin,
        server_instance_id: artifact.server_instance_id,
        enrollment_id: artifact.enrollment_id,
        server_display_label: state.server_display_label,
        device_label: request.device_label,
        scopes: artifact.scopes,
        expires_at: artifact.expires_at.to_rfc3339(),
        token: artifact.token,
    })
    .into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    Ok(response)
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

/// Authenticate and process a bounded batch of domain-record writes.
///
/// # Errors
///
/// Returns a version, authentication, body, or persistence error. Individual record validation and
/// idempotency outcomes are returned as item acknowledgements.
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
        (status = 200, description = "Item-level domain write outcomes", body = ClientWriteResponse),
        (status = 426, description = "Incompatible API major", body = ClientErrorResponse),
        (status = 503, description = "Write database unavailable", body = ClientErrorResponse)
    )
)]
#[expect(
    clippy::too_many_lines,
    reason = "item acknowledgement mapping keeps every terminal domain outcome explicit"
)]
pub async fn client_write_handler(
    Extension(database): Extension<Option<sqlx::PgPool>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ClientWriteResponse>, ClientApiError> {
    validate_version(&headers)?;
    let database =
        database.ok_or_else(|| unavailable("item_writes", "VITALS_CONSOLE_DATABASE_URL"))?;
    authenticate_mobile_request(
        &database,
        &headers,
        body.as_ref(),
        "POST",
        "/api/v1/client/items",
        "submit_domain_records",
    )
    .await?;
    let request: ClientWriteBatch = serde_json::from_slice(&body).map_err(|decode_error| {
        error(
            StatusCode::BAD_REQUEST,
            "write_body_invalid",
            "client write body does not match the versioned contract",
            false,
            format!("body_decode={decode_error}"),
            Some(API_MAJOR),
        )
    })?;
    if request.items.is_empty() || request.items.len() > 100 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "write_batch_invalid",
            "client write batch must contain 1 through 100 items",
            false,
            format!("item_count={}; allowed=1..=100", request.items.len()),
            Some(API_MAJOR),
        ));
    }
    let client_id = parse_uuid_header(&headers, "x-anxietywatch-client-id")?;
    let actor = format!("mobile-client:{client_id}");
    let mut acknowledgements = Vec::with_capacity(request.items.len());
    for item in request.items {
        let acknowledgement =
            match serde_json::from_value::<crate::models::EnvelopePayload>(item.content) {
                Ok(envelope)
                    if envelope.class == "domain_record"
                        && item.schema_version > 0
                        && item.id == envelope.record.id =>
                {
                    match crate::db::process_domain_record(&database, &envelope.record, &actor)
                        .await
                    {
                        Ok(crate::db::DomainRecordOutcome::Created { .. }) => ItemAcknowledgement {
                            item_id: item.id,
                            status: "created".to_string(),
                            code: Some("domain_record_created".to_string()),
                            message: "domain record created".to_string(),
                            retryable: false,
                        },
                        Ok(crate::db::DomainRecordOutcome::Replayed { .. }) => {
                            ItemAcknowledgement {
                                item_id: item.id,
                                status: "replayed".to_string(),
                                code: Some("domain_record_replayed".to_string()),
                                message:
                                    "domain record was already accepted with identical content"
                                        .to_string(),
                                retryable: false,
                            }
                        }
                        Ok(crate::db::DomainRecordOutcome::Conflict { id }) => {
                            ItemAcknowledgement {
                                item_id: item.id,
                                status: "conflict".to_string(),
                                code: Some("domain_record_conflict".to_string()),
                                message: format!(
                                    "domain record {id} conflicts with existing content"
                                ),
                                retryable: false,
                            }
                        }
                        Err((_, response)) => ItemAcknowledgement {
                            item_id: item.id,
                            status: "rejected".to_string(),
                            code: Some("domain_record_rejected".to_string()),
                            message: response.0.error,
                            retryable: false,
                        },
                    }
                }
                Ok(envelope) => ItemAcknowledgement {
                    item_id: item.id,
                    status: "rejected".to_string(),
                    code: Some("item_identity_invalid".to_string()),
                    message: if envelope.class != "domain_record" {
                        format!("item class {} is not supported", envelope.class)
                    } else if item.schema_version == 0 {
                        "item schema version must be positive".to_string()
                    } else {
                        format!("item id {} does not match its domain record id", item.id)
                    },
                    retryable: false,
                },
                Err(decode_error) => ItemAcknowledgement {
                    item_id: item.id,
                    status: "rejected".to_string(),
                    code: Some("item_content_invalid".to_string()),
                    message: format!("item content is not a domain envelope: {decode_error}"),
                    retryable: false,
                },
            };
        acknowledgements.push(acknowledgement);
    }
    Ok(Json(ClientWriteResponse {
        api_version: format!("{API_MAJOR}.{API_MINOR}"),
        batch_id: request.batch_id,
        acknowledgements,
        server_time: Utc::now().to_rfc3339(),
    }))
}

/// Validate status facts without pretending that `HealthKit` exposes read authorization.
fn validate_health_beacon(request: &HealthBeaconRequest) -> Result<(), String> {
    if request.app_version.trim().is_empty() {
        return Err("app_version is empty".to_string());
    }
    if request.registry_sha256.trim().is_empty() {
        return Err("registry_sha256 is empty".to_string());
    }
    if request.types.len() > 256 {
        return Err(format!("type_count={}; maximum=256", request.types.len()));
    }
    let asserted_at = chrono::DateTime::parse_from_rfc3339(&request.asserted_at)
        .map_err(|error| format!("asserted_at is invalid: {error}"))?;
    let changed_at = chrono::DateTime::parse_from_rfc3339(&request.changed_at)
        .map_err(|error| format!("changed_at is invalid: {error}"))?;
    if changed_at > asserted_at {
        return Err(format!(
            "changed_at={}; asserted_at={}; changed_at is later than asserted_at",
            request.changed_at, request.asserted_at
        ));
    }
    for instant in [request.last_server_ack_at.as_deref()]
        .into_iter()
        .flatten()
    {
        chrono::DateTime::parse_from_rfc3339(instant)
            .map_err(|error| format!("status timestamp is invalid: {error}"))?;
    }
    for fact in &request.types {
        if fact.healthkit_type.trim().is_empty() {
            return Err("healthkit_type is empty".to_string());
        }
        let outcomes = [
            fact.authorization_request_outcome.as_deref(),
            fact.observer_registration_outcome.as_deref(),
            fact.background_delivery_outcome.as_deref(),
            fact.last_query_outcome.as_deref(),
        ];
        if outcomes
            .iter()
            .flatten()
            .any(|outcome| outcome.to_ascii_lowercase().contains("granted"))
        {
            return Err(format!(
                "healthkit_type={}; authorization outcome must not claim granted",
                fact.healthkit_type
            ));
        }
        for instant in [
            fact.last_query_at.as_deref(),
            fact.last_matching_sample_at.as_deref(),
            fact.last_pi_ack_at.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            chrono::DateTime::parse_from_rfc3339(instant)
                .map_err(|error| format!("status timestamp is invalid: {error}"))?;
        }
    }
    Ok(())
}

/// Accept an authenticated, idempotent operational phone status beacon.
///
/// # Errors
///
/// Returns a named version, authentication, validation, conflict, or persistence error.
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
        (status = 200, description = "Status beacon accepted or replayed", body = HealthBeaconResponse),
        (status = 400, description = "Missing or malformed status beacon", body = ClientErrorResponse),
        (status = 401, description = "Missing or invalid request signature", body = ClientErrorResponse),
        (status = 403, description = "Credential scope does not permit this operation", body = ClientErrorResponse),
        (status = 409, description = "Durable request or beacon id was already admitted", body = ClientErrorResponse),
        (status = 426, description = "Incompatible API major", body = ClientErrorResponse),
        (status = 503, description = "Status beacon database unavailable", body = ClientErrorResponse)
    )
)]
pub async fn client_beacon_handler(
    Extension(database): Extension<Option<sqlx::PgPool>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<HealthBeaconResponse>, ClientApiError> {
    validate_version(&headers)?;
    let database =
        database.ok_or_else(|| unavailable("status_beacon", "VITALS_CONSOLE_DATABASE_URL"))?;
    authenticate_mobile_request(
        &database,
        &headers,
        body.as_ref(),
        "POST",
        "/api/v1/client/status",
        "submit_platform_health",
    )
    .await?;
    let request: HealthBeaconRequest = serde_json::from_slice(&body).map_err(|decode_error| {
        error(
            StatusCode::BAD_REQUEST,
            "status_body_invalid",
            "status beacon body does not match the versioned contract",
            false,
            format!("body_decode={decode_error}"),
            Some(API_MAJOR),
        )
    })?;
    validate_health_beacon(&request).map_err(|reason| {
        error(
            StatusCode::BAD_REQUEST,
            "status_body_invalid",
            "status beacon contains invalid operational facts",
            false,
            reason,
            Some(API_MAJOR),
        )
    })?;
    let client_id = parse_uuid_header(&headers, "x-anxietywatch-client-id")?;
    let outcome = crate::db::store_health_beacon(&database, client_id, &request, &body)
        .await
        .map_err(|reason| {
            let status = if reason.contains("conflicts") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            error(
                status,
                "status_beacon_persistence_failed",
                "status beacon could not be stored",
                status == StatusCode::INTERNAL_SERVER_ERROR,
                reason,
                Some(API_MAJOR),
            )
        })?;
    Ok(Json(HealthBeaconResponse {
        api_version: format!("{API_MAJOR}.{API_MINOR}"),
        beacon_id: request.beacon_id,
        status: outcome,
        server_time: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    }))
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
    Extension(database): Extension<Option<sqlx::PgPool>>,
    headers: HeaderMap,
    Query(_query): Query<ProjectionQuery>,
) -> Result<Json<ProjectionPage>, ClientApiError> {
    validate_version(&headers)?;
    let database =
        database.ok_or_else(|| unavailable("projections", "VITALS_CONSOLE_DATABASE_URL"))?;
    authenticate_mobile_request(
        &database,
        &headers,
        &[],
        "GET",
        "/api/v1/client/projections",
        "read_projections",
    )
    .await?;
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

    #[test]
    fn phone_status_fixture_accepts_unknown_healthkit_read_authorization() {
        let request: HealthBeaconRequest = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/health-beacon-request.json"
        )))
        .expect("health beacon fixture decodes");
        assert!(validate_health_beacon(&request).is_ok());
        assert_eq!(request.types[0].last_matching_sample_at, None);
    }

    #[test]
    fn phone_status_rejects_a_granted_authorization_claim() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/health-beacon-request.json"
        )))
        .expect("health beacon fixture decodes");
        value["types"][0]["authorization_request_outcome"] =
            serde_json::Value::String("granted".to_string());
        let request: HealthBeaconRequest =
            serde_json::from_value(value).expect("modified beacon decodes");
        let error = validate_health_beacon(&request).expect_err("granted must be rejected");
        assert!(error.contains("must not claim granted"));
    }

    #[test]
    fn phone_status_rejects_changed_time_after_assertion() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../api/fixtures/client-v1/health-beacon-request.json"
        )))
        .expect("health beacon fixture decodes");
        value["changed_at"] = serde_json::Value::String("2025-01-15T08:11:00.000Z".to_string());
        let request: HealthBeaconRequest =
            serde_json::from_value(value).expect("modified beacon decodes");
        let error = validate_health_beacon(&request).expect_err("later changed time must fail");
        assert!(error.contains("later than asserted_at"));
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
                .is_some_and(|v| v.contains("VITALS_CONSOLE_DATABASE_URL"))
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
        let status_responses = value["paths"]["/api/v1/client/status"]["post"]["responses"]
            .as_object()
            .expect("status responses object");
        assert!(status_responses.contains_key("200"));
        let projection_responses = value["paths"]["/api/v1/client/projections"]["get"]["responses"]
            .as_object()
            .expect("projection responses object");
        assert!(
            projection_responses
                .keys()
                .all(|status| !status.starts_with('2'))
        );
        let serialized = serde_json::to_string(&value).expect("OpenAPI JSON serializes");
        assert!(!serialized.contains("VitalsType"));
        assert!(!serialized.contains("VitalsSource"));
        assert!(!serialized.contains("polar_h10"));
    }
}
