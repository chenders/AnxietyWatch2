//! Safe AirSense 11 admin projection and lifecycle actions.
//!
//! The browser can inspect link, measurement, credential-readiness, service,
//! and journal facts, and can request the fixed lifecycle actions. The
//! AirSense master key is never returned, stored in PostgreSQL, or accepted in
//! an HTTP body. PostgreSQL stores only opt-in state and an audit trail.

#![expect(
    missing_docs,
    reason = "serialized response fields are documented by their containing API projection"
)]

use axum::{
    Json,
    body::Body,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env, fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use crate::handlers::AppState;
use vitals_registry::VitalsSource;

pub const AIR11_SERVICE: &str = "vitals-air11.service";
pub const AIR11_STATUS_PATH: &str = "/run/vitals-air11/status.json";
const CREDENTIAL_FILE: &str = ".air11/air11-credentials.json";
const DEFAULT_ADMIN_TOKEN_FILE: &str = "/etc/vitals/console-admin-token";
const MAX_CREDENTIAL_BYTES: u64 = 4096;
const MAX_STATUS_BYTES: u64 = 64 * 1024;
const MAX_BLOCKER_BYTES: usize = 256;
const MAX_LOG_LINE_CHARS: usize = 512;
const MAX_JOURNAL_OUTPUT_BYTES: usize = 256 * 1024;
const MAX_ADMIN_TOKEN_BYTES: u64 = 4096;
const MAX_ADMIN_TOKEN_LEN: usize = 4096;
const ACTION_HEADER: &str = "x-air11-action-token";
const AUTHORIZATION_HEADER: &str = "authorization";

/// A private-network CIDR accepted by the `AirSense` console listener.
#[derive(Debug, Clone)]
pub struct AllowedNetwork {
    address: IpAddr,
    prefix: u8,
}

impl AllowedNetwork {
    /// Returns whether an address belongs to this network.
    #[must_use]
    pub fn contains(&self, address: IpAddr) -> bool {
        match (self.address, address) {
            (IpAddr::V4(network), IpAddr::V4(candidate)) => {
                let prefix = u32::from(self.prefix);
                if prefix == 0 {
                    true
                } else {
                    let mask = u32::MAX << (32 - prefix);
                    (u32::from(network) & mask) == (u32::from(candidate) & mask)
                }
            }
            (IpAddr::V6(network), IpAddr::V6(candidate)) => {
                let prefix = u32::from(self.prefix);
                if prefix == 0 {
                    true
                } else {
                    let mask = u128::MAX << (128 - prefix);
                    (u128::from(network) & mask) == (u128::from(candidate) & mask)
                }
            }
            _ => false,
        }
    }
}

/// Parses a comma-separated list of IPv4/IPv6 networks.
///
/// An address without a slash is treated as a single-host network. Empty
/// entries and prefixes wider than the address family are rejected so a typo
/// cannot silently widen the listener.
///
/// # Errors
///
/// Returns an error naming the malformed entry when an address, prefix, or
/// list entry is invalid.
pub fn parse_allowed_networks(raw: &str) -> Result<Vec<AllowedNetwork>, String> {
    let mut networks = Vec::new();
    for item in raw.split(',') {
        let item = item.trim();
        if item.is_empty() {
            return Err("local-network allowlist contains an empty entry".to_string());
        }
        let (address_text, prefix_text) = item
            .split_once('/')
            .map_or((item, None), |(address, prefix)| (address, Some(prefix)));
        let address = address_text.parse::<IpAddr>().map_err(|_| {
            format!("local-network allowlist entry is not an IP address: {address_text}")
        })?;
        let maximum: u8 = match address {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        let prefix = match prefix_text {
            Some(text) => text
                .parse::<u8>()
                .map_err(|_| format!("local-network allowlist prefix is invalid: {item}"))?,
            None => maximum,
        };
        if prefix > maximum {
            return Err(format!(
                "local-network allowlist prefix is too wide: {item}"
            ));
        }
        networks.push(AllowedNetwork { address, prefix });
    }
    if networks.is_empty() {
        return Err("local-network allowlist is empty".to_string());
    }
    Ok(networks)
}

/// Default allowlist: loopback plus RFC1918 and IPv6 unique-local space.
#[must_use]
pub fn default_allowed_networks() -> Vec<AllowedNetwork> {
    parse_allowed_networks("127.0.0.0/8,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,::1/128,fd00::/8")
        .unwrap_or_default()
}

/// Whether a peer is allowed to reach the `AirSense` API.
#[must_use]
pub fn peer_allowed(networks: &[AllowedNetwork], peer: IpAddr) -> bool {
    networks.iter().any(|network| network.contains(peer))
}

/// Rejects `AirSense` API requests from peers outside the private-network policy.
pub async fn air11_network_guard(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip());
    let Some(peer) = peer else {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::models::ErrorResponse {
                error: "AirSense page blocked: peer address was not provided by the server"
                    .to_string(),
            }),
        )
            .into_response();
    };
    if !peer_allowed(&state.air11_allowed_networks, peer) {
        return (
            StatusCode::FORBIDDEN,
            Json(crate::models::ErrorResponse {
                error: format!(
                    "AirSense page blocked: peer={peer} is outside the configured local-network allowlist"
                ),
            }),
        )
            .into_response();
    }
    next.run(request).await
}

/// Loads the separate console-admin capability from an operator-managed file.
///
/// The value is never logged or serialized. Missing credentials disable
/// mutation routes while keeping read-only status available.
///
/// # Errors
///
/// Returns a named error when the configured token file is unsafe, malformed,
/// unreadable, or outside its size bound.
pub fn load_admin_token() -> Result<Option<String>, &'static str> {
    let path = env::var_os("VITALS_CONSOLE_ADMIN_TOKEN_FILE")
        .map_or_else(|| PathBuf::from(DEFAULT_ADMIN_TOKEN_FILE), PathBuf::from);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("admin token metadata could not be read"),
    };
    if !metadata.is_file() {
        return Err("admin token path is not a regular file");
    }
    if !metadata.permissions().mode_is_private() {
        return Err("admin token permissions are not owner-only");
    }
    if metadata.len() == 0 || metadata.len() > MAX_ADMIN_TOKEN_BYTES {
        return Err("admin token file length is outside the bounded range");
    }
    let bytes = fs::read(path).map_err(|_| "admin token file could not be read")?;
    let token = String::from_utf8(bytes).map_err(|_| "admin token file is not UTF-8")?;
    let token = token.trim();
    if token.is_empty() || token.len() > MAX_ADMIN_TOKEN_LEN {
        return Err("admin token value is empty or exceeds the bounded range");
    }
    Ok(Some(token.to_string()))
}

/// A safe status projection for the admin and kiosk pages. It intentionally
/// omits all target and credential material.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Air11StatusView {
    pub link_state: Option<String>,
    pub measurement_state: Option<String>,
    pub session_id: Option<i64>,
    pub reconnect_count: Option<u64>,
    pub retry_count: Option<u64>,
    pub received_envelopes: Option<u64>,
    pub decoded_frames: Option<u64>,
    pub published_samples: Option<u64>,
    pub unavailable: Option<u64>,
    pub duplicates: Option<u64>,
    pub quarantined: Option<u64>,
    pub dropped: Option<u64>,
    pub durability_unknown: Option<bool>,
    pub last_host_receipt_unix_micros: Option<i64>,
    pub last_device_start_time: Option<String>,
    pub blocker: Option<String>,
    pub event_subscription_state: Option<String>,
    pub event_notifications: Option<u64>,
    pub event_records: Option<u64>,
    pub event_truncated_notifications: Option<u64>,
    pub last_event_data_id: Option<String>,
    pub event_blocker: Option<String>,
    pub therapy_session_id: Option<u64>,
    pub therapy_state: Option<String>,
    pub therapy_start_host_unix_micros: Option<i64>,
    pub therapy_stop_host_unix_micros: Option<i64>,
    pub therapy_finalized_host_unix_micros: Option<i64>,
    pub therapy_blocker: Option<String>,
    pub status_state: String,
}

/// Credential readiness without a secret or device identifier.
#[derive(Debug, Clone, Serialize)]
pub struct CredentialView {
    pub state: String,
    pub client_id_present: bool,
    pub target_present: bool,
    pub master_key_present: bool,
    pub permissions: String,
    pub blocker: Option<String>,
}

/// Process state for the supervised `AirSense` unit.
#[derive(Debug, Clone, Serialize)]
pub struct Air11ServiceView {
    pub active_state: String,
    pub sub_state: String,
    pub restart_count: Option<u32>,
    pub started_at: Option<String>,
    pub blocker: Option<String>,
}

/// Non-secret opt-in state held in `PostgreSQL`.
#[derive(Debug, Clone, Serialize)]
pub struct Air11ConfigView {
    pub state: String,
    pub opted_in: Option<bool>,
    pub blocker: Option<String>,
}

/// One safe lifecycle audit record.
#[derive(Debug, Clone, Serialize)]
pub struct Air11AuditView {
    pub id: i64,
    pub action: String,
    pub outcome: String,
    pub detail: String,
    pub created_at: String,
}

/// One sanitized system journal entry.
#[derive(Debug, Clone, Serialize)]
pub struct Air11LogView {
    pub level: String,
    pub observed_at: String,
    pub message: String,
}

/// Journal collection state and entries.
#[derive(Debug, Clone, Serialize)]
pub struct Air11LogsView {
    pub state: String,
    pub blocker: Option<String>,
    pub entries: Vec<Air11LogView>,
}

/// One latest stored `AirSense` measurement in its registry unit.
#[derive(Debug, Clone, Serialize)]
pub struct Air11LatestView {
    pub type_id: i32,
    pub name: String,
    pub unit: String,
    pub t_micros: i64,
    pub value: f64,
}

/// Query for bounded stored `AirSense` history.
#[derive(Debug, Deserialize)]
pub struct Air11HistoryQuery {
    /// Stored-history lookback in hours. The route rejects zero and values over 48.
    pub window_hours: Option<u32>,
}

/// One one-minute stored `AirSense` history bucket.
#[derive(Debug, Clone, Serialize)]
pub struct Air11HistoryPoint {
    pub t_micros: i64,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub count: i64,
}

/// One registry type in the stored `AirSense` history projection.
#[derive(Debug, Clone, Serialize)]
pub struct Air11HistorySeries {
    pub name: String,
    pub unit: String,
    pub points: Vec<Air11HistoryPoint>,
}

/// Stored `AirSense` history used by the bedside visual field.
#[derive(Debug, Clone, Serialize)]
pub struct Air11HistoryView {
    pub state: String,
    pub blocker: Option<String>,
    pub window_hours: u32,
    pub boundary: String,
    pub series: Vec<Air11HistorySeries>,
}

/// Combined response for `GET /api/air11`.
#[derive(Debug, Clone, Serialize)]
pub struct Air11View {
    pub actions_enabled: bool,
    pub admin_auth_configured: bool,
    pub action_token: Option<String>,
    pub config: Air11ConfigView,
    pub credentials: CredentialView,
    pub service: Air11ServiceView,
    pub status: Air11StatusView,
    pub logs: Air11LogsView,
    pub audit_state: String,
    pub audit_blocker: Option<String>,
    pub audit: Vec<Air11AuditView>,
    pub latest_state: String,
    pub latest_blocker: Option<String>,
    pub latest: Vec<Air11LatestView>,
}

/// Fixed actions supported by the page. There is no arbitrary command route.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Air11Action {
    OptIn,
    OptOut,
    Start,
    Stop,
    Restart,
    CheckCredentials,
}

#[derive(Debug, Deserialize)]
pub struct Air11ActionRequest {
    pub action: Air11Action,
    pub confirmation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Air11ActionResponse {
    pub action: String,
    pub message: String,
    pub view: Air11View,
}

fn error_reply(
    status: StatusCode,
    message: impl Into<String>,
) -> (StatusCode, Json<crate::models::ErrorResponse>) {
    (
        status,
        Json(crate::models::ErrorResponse {
            error: message.into(),
        }),
    )
}

fn credentials_path() -> PathBuf {
    if let Some(path) = env::var_os("AIR11_CREDENTIALS_PATH") {
        return PathBuf::from(path);
    }
    env::var_os("HOME").map_or_else(
        || PathBuf::from(CREDENTIAL_FILE),
        |home| PathBuf::from(home).join(CREDENTIAL_FILE),
    )
}

fn is_hex_byte_pair(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_linux_mac(value: &str) -> bool {
    let value = value.trim();
    if value.len() == 12 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return true;
    }
    let separator = if value.contains(':') { ':' } else { '-' };
    let parts = value.split(separator);
    let mut count = 0;
    for part in parts {
        if !is_hex_byte_pair(part) {
            return false;
        }
        count += 1;
    }
    count == 6
}

fn is_nonzero_linux_mac(value: &str) -> bool {
    is_linux_mac(value)
        && value
            .bytes()
            .filter(u8::is_ascii_hexdigit)
            .any(|byte| !matches!(byte, b'0'))
}

fn is_nonzero_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        && value.bytes().any(|byte| !matches!(byte, b'0'))
}

fn exceeds_byte_bound(len: usize, maximum: u64) -> bool {
    match u64::try_from(len) {
        Ok(len) => len > maximum,
        Err(_) => true,
    }
}

fn value_string<'a>(document: &'a Value, names: &[&str]) -> Option<&'a str> {
    names.iter().find_map(|name| document.get(*name)?.as_str())
}

fn valid_target_present(document: &Value) -> bool {
    [
        "linux_address",
        "linuxAddress",
        "linux_mac",
        "linuxMac",
        "address",
        "device_address",
        "target",
    ]
    .iter()
    .any(|name| {
        document
            .get(*name)
            .and_then(Value::as_str)
            .is_some_and(is_nonzero_linux_mac)
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "credential validation keeps every fail-closed file and shape gate explicit"
)]
fn credential_view(path: &Path) -> CredentialView {
    let missing = || CredentialView {
        state: "missing".to_string(),
        client_id_present: false,
        target_present: false,
        master_key_present: false,
        permissions: "not_checked".to_string(),
        blocker: Some("credential file is absent; provision it outside the console".to_string()),
    };
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return missing(),
        Err(_) => {
            return CredentialView {
                state: "unknown".to_string(),
                client_id_present: false,
                target_present: false,
                master_key_present: false,
                permissions: "not_checked".to_string(),
                blocker: Some("credential file metadata could not be read".to_string()),
            };
        }
    };
    if !metadata.is_file() {
        return CredentialView {
            state: "invalid".to_string(),
            client_id_present: false,
            target_present: false,
            master_key_present: false,
            permissions: "not_regular".to_string(),
            blocker: Some("credential path is not a regular file".to_string()),
        };
    }
    if !metadata.permissions().mode_is_private() {
        return CredentialView {
            state: "insecure".to_string(),
            client_id_present: false,
            target_present: false,
            master_key_present: false,
            permissions: "group_or_world_accessible".to_string(),
            blocker: Some(
                "credential file permissions are not owner-only; expected mode 0600 or stricter"
                    .to_string(),
            ),
        };
    }
    if metadata.len() > MAX_CREDENTIAL_BYTES {
        return CredentialView {
            state: "invalid".to_string(),
            client_id_present: false,
            target_present: false,
            master_key_present: false,
            permissions: "owner_only".to_string(),
            blocker: Some("credential file exceeds the 4096-byte bound".to_string()),
        };
    }
    let Ok(bytes) = fs::read(path) else {
        return CredentialView {
            state: "unknown".to_string(),
            client_id_present: false,
            target_present: false,
            master_key_present: false,
            permissions: "owner_only".to_string(),
            blocker: Some("credential file could not be read".to_string()),
        };
    };
    if exceeds_byte_bound(bytes.len(), MAX_CREDENTIAL_BYTES) {
        return CredentialView {
            state: "invalid".to_string(),
            client_id_present: false,
            target_present: false,
            master_key_present: false,
            permissions: "owner_only".to_string(),
            blocker: Some("credential file exceeds the 4096-byte bound".to_string()),
        };
    }
    let document: Value = match serde_json::from_slice(&bytes) {
        Ok(document) => document,
        Err(_) => {
            return CredentialView {
                state: "invalid".to_string(),
                client_id_present: false,
                target_present: false,
                master_key_present: false,
                permissions: "owner_only".to_string(),
                blocker: Some("credential JSON is malformed".to_string()),
            };
        }
    };
    let address_present = value_string(&document, &["address", "device_address", "target"])
        .is_some_and(|value| !value.is_empty());
    let client_id_present = value_string(&document, &["client_id", "clientId"])
        .is_some_and(|value| !value.is_empty() && value.len() <= 128);
    let target_present = valid_target_present(&document);
    let master_key_present = value_string(
        &document,
        &[
            "master_pair_key",
            "masterPairKey",
            "master_key",
            "master_key_hex",
        ],
    )
    .is_some_and(|value| is_nonzero_hex(value, 64));
    let valid = document.is_object()
        && address_present
        && client_id_present
        && target_present
        && master_key_present;
    CredentialView {
        state: if valid { "ready" } else { "invalid" }.to_string(),
        client_id_present,
        target_present,
        master_key_present,
        permissions: "owner_only".to_string(),
        blocker: (!valid).then_some(
            "credential JSON must contain a bounded client id, explicit Linux target, and 32-byte master key"
                .to_string(),
        ),
    }
}

fn truncate(value: &str, limit: usize) -> String {
    let mut output: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        output.push('…');
    }
    output
}

fn safe_blocker(value: Option<String>) -> Option<String> {
    value.map(|value| {
        let sanitized = safe_log_line(&value);
        if sanitized.len() > MAX_BLOCKER_BYTES {
            "status blocker exceeded the 256-byte display bound".to_string()
        } else {
            sanitized
        }
    })
}

fn is_private_token(value: &str) -> bool {
    let trimmed = value.trim_matches(|character: char| {
        !character.is_ascii_hexdigit() && character != ':' && character != '-'
    });
    is_linux_mac(trimmed)
        || (trimmed.len() == 64 && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn contains_private_token(value: &str) -> bool {
    is_private_token(value)
        || value
            .split(|character: char| {
                !character.is_ascii_hexdigit() && character != ':' && character != '-'
            })
            .any(is_private_token)
}

fn safe_log_line(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if lower.contains("masterpairkey")
        || lower.contains("master_pair_key")
        || lower.contains("master_key")
        || lower.contains("sessionkey")
        || lower.contains("session_key")
        || lower.contains("client_id")
        || lower.contains("client=")
        || lower.contains("device_address")
        || lower.contains("address")
        || lower.contains("target")
    {
        return "private collector detail was suppressed".to_string();
    }
    let mut output = String::new();
    for (index, token) in value.split_whitespace().enumerate() {
        if index != 0 {
            output.push(' ');
        }
        if contains_private_token(token) {
            output.push_str("[private address or key redacted]");
        } else {
            output.push_str(token);
        }
    }
    truncate(&output, MAX_LOG_LINE_CHARS)
}

fn status_view(path: &Path) -> Air11StatusView {
    let missing = || Air11StatusView {
        status_state: "not_observed".to_string(),
        blocker: Some(
            "status file is absent; no current AirSense observation is available".to_string(),
        ),
        ..Air11StatusView::default()
    };
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return missing(),
        Err(_) => {
            return Air11StatusView {
                status_state: "unknown".to_string(),
                blocker: Some("status file metadata could not be read".to_string()),
                ..Air11StatusView::default()
            };
        }
    };
    if !metadata.is_file() || metadata.len() > MAX_STATUS_BYTES {
        return Air11StatusView {
            status_state: "invalid".to_string(),
            blocker: Some(
                "status file is not a regular file within the 65536-byte bound".to_string(),
            ),
            ..Air11StatusView::default()
        };
    }
    let Ok(bytes) = fs::read(path) else {
        return Air11StatusView {
            status_state: "unknown".to_string(),
            blocker: Some("status file could not be read".to_string()),
            ..Air11StatusView::default()
        };
    };
    if exceeds_byte_bound(bytes.len(), MAX_STATUS_BYTES) {
        return Air11StatusView {
            status_state: "invalid".to_string(),
            blocker: Some(
                "status file is not a regular file within the 65536-byte bound".to_string(),
            ),
            ..Air11StatusView::default()
        };
    }
    let mut status: Air11StatusView = match serde_json::from_slice(&bytes) {
        Ok(status) => status,
        Err(_) => {
            return Air11StatusView {
                status_state: "invalid".to_string(),
                blocker: Some("status JSON is malformed".to_string()),
                ..Air11StatusView::default()
            };
        }
    };
    status.status_state = "observed".to_string();
    status.blocker = safe_blocker(status.blocker);
    status.last_device_start_time = status.last_device_start_time.map(|value| {
        if value.len() <= 128 {
            value
        } else {
            "untrusted device time exceeded the display bound".to_string()
        }
    });
    status
}

fn parse_systemctl(output: &str) -> Air11ServiceView {
    let mut view = Air11ServiceView {
        active_state: "unknown".to_string(),
        sub_state: "unknown".to_string(),
        restart_count: None,
        started_at: None,
        blocker: None,
    };
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("ActiveState=") {
            view.active_state = truncate(value, 64);
        } else if let Some(value) = line.strip_prefix("SubState=") {
            view.sub_state = truncate(value, 64);
        } else if let Some(value) = line.strip_prefix("NRestarts=") {
            view.restart_count = value.parse().ok();
        } else if let Some(value) = line.strip_prefix("ExecMainStartTimestamp=")
            && !value.is_empty()
        {
            view.started_at = Some(truncate(value, 128));
        }
    }
    view
}

async fn service_view(state: &AppState) -> Air11ServiceView {
    match (state.sys_ops.cmd)(
        "systemctl",
        &[
            "show",
            "-p",
            "ActiveState,SubState,NRestarts,ExecMainStartTimestamp",
            AIR11_SERVICE,
        ],
    )
    .await
    {
        Ok(output) => parse_systemctl(&output),
        Err(_) => Air11ServiceView {
            active_state: "unknown".to_string(),
            sub_state: "unknown".to_string(),
            restart_count: None,
            started_at: None,
            blocker: Some("systemd service inspection failed".to_string()),
        },
    }
}

fn log_level(document: &Value) -> &'static str {
    let priority = document.get("PRIORITY").and_then(|value| {
        value.as_str().or_else(|| {
            value.as_i64().and_then(|number| match number {
                0 => Some("0"),
                1 => Some("1"),
                2 => Some("2"),
                3 => Some("3"),
                4 => Some("4"),
                5 => Some("5"),
                _ => None,
            })
        })
    });
    match priority {
        Some("0" | "1" | "2" | "3") => "error",
        Some("4") => "warning",
        Some("5") => "notice",
        _ => "info",
    }
}

fn journal_view_output(output: &str) -> Air11LogsView {
    if output.len() > MAX_JOURNAL_OUTPUT_BYTES {
        return Air11LogsView {
            state: "invalid".to_string(),
            blocker: Some("journal output exceeded the 262144-byte display bound".to_string()),
            entries: Vec::new(),
        };
    }
    let mut entries = Vec::new();
    let mut malformed = false;
    for line in output.lines().take(80) {
        let Ok(document) = serde_json::from_str::<Value>(line) else {
            malformed = true;
            continue;
        };
        let Some(message) = document.get("MESSAGE").and_then(Value::as_str) else {
            continue;
        };
        if message.is_empty() {
            continue;
        }
        let observed_at = document
            .get("__REALTIME_TIMESTAMP")
            .and_then(Value::as_str)
            .map_or_else(|| "unknown".to_string(), |value| truncate(value, 32));
        entries.push(Air11LogView {
            level: log_level(&document).to_string(),
            observed_at,
            message: safe_log_line(message),
        });
    }
    if malformed && entries.is_empty() {
        Air11LogsView {
            state: "invalid".to_string(),
            blocker: Some("journal output could not be parsed safely".to_string()),
            entries,
        }
    } else {
        Air11LogsView {
            state: if entries.is_empty() {
                "empty"
            } else {
                "observed"
            }
            .to_string(),
            blocker: malformed.then_some(
                "some journal entries were malformed and were omitted safely".to_string(),
            ),
            entries,
        }
    }
}

async fn logs_view(state: &AppState) -> Air11LogsView {
    match (state.sys_ops.cmd)(
        "journalctl",
        &[
            "-u",
            AIR11_SERVICE,
            "-n",
            "80",
            "--since=-10min",
            "--no-pager",
            "-o",
            "json",
            "--output-fields=MESSAGE,PRIORITY,__REALTIME_TIMESTAMP",
        ],
    )
    .await
    {
        Ok(output) => journal_view_output(&output),
        Err(_) => Air11LogsView {
            state: "unavailable".to_string(),
            blocker: Some("service journal is unavailable to the console user".to_string()),
            entries: Vec::new(),
        },
    }
}

async fn config_view(state: &AppState) -> Air11ConfigView {
    let Some(pool) = state.db.as_ref() else {
        return Air11ConfigView {
            state: "unavailable".to_string(),
            opted_in: None,
            blocker: Some("database unavailable; opt-in state cannot be read".to_string()),
        };
    };
    match sqlx::query_scalar::<_, bool>(
        "SELECT opted_in FROM air11_integration_config WHERE singleton = TRUE",
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(opted_in)) => Air11ConfigView {
            state: if opted_in { "opted_in" } else { "not_opted_in" }.to_string(),
            opted_in: Some(opted_in),
            blocker: None,
        },
        Ok(None) => Air11ConfigView {
            state: "not_configured".to_string(),
            opted_in: None,
            blocker: Some(
                "AirSense opt-in row is absent; database migration is incomplete".to_string(),
            ),
        },
        Err(_) => Air11ConfigView {
            state: "unavailable".to_string(),
            opted_in: None,
            blocker: Some(
                "AirSense config table is unavailable; run the database migration".to_string(),
            ),
        },
    }
}

async fn latest_view(state: &AppState) -> (Vec<Air11LatestView>, String, Option<String>) {
    let Some(pool) = state.db.as_ref() else {
        return (
            Vec::new(),
            "unavailable".to_string(),
            Some("database unavailable; latest AirSense measurements cannot be read".to_string()),
        );
    };
    let source = VitalsSource::ResmedAir11.id();
    let rows = sqlx::query_as::<_, (i32, String, String, i64, f64)>(
        "SELECT latest.type,
                registry.name,
                registry.unit,
                (EXTRACT(epoch FROM latest.timestamp) * 1000000)::bigint,
                latest.value
         FROM (
             SELECT DISTINCT ON (s.type)
                    s.type, s.timestamp, s.value
             FROM samples AS s
             WHERE s.source = $1
             ORDER BY s.type, s.timestamp DESC
         ) AS latest
         JOIN vitals_type AS registry ON registry.id = latest.type
         ORDER BY latest.type",
    )
    .bind(source)
    .fetch_all(pool)
    .await;
    match rows {
        Ok(rows) if rows.is_empty() => (
            Vec::new(),
            "empty".to_string(),
            Some("no stored AirSense measurements are available".to_string()),
        ),
        Ok(rows) => (
            rows.into_iter()
                .map(|(type_id, name, unit, t_micros, value)| Air11LatestView {
                    type_id,
                    name,
                    unit,
                    t_micros,
                    value,
                })
                .collect(),
            "observed".to_string(),
            None,
        ),
        Err(_) => (
            Vec::new(),
            "unavailable".to_string(),
            Some("latest AirSense measurement query failed".to_string()),
        ),
    }
}

async fn audit_view(state: &AppState) -> (Vec<Air11AuditView>, String, Option<String>) {
    let Some(pool) = state.db.as_ref() else {
        return (
            Vec::new(),
            "unavailable".to_string(),
            Some("database unavailable; audit history cannot be read".to_string()),
        );
    };
    match sqlx::query_as::<_, (i64, String, String, String, String)>(
        "SELECT id, action, outcome, detail, created_at::text \
         FROM air11_admin_audit ORDER BY id DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => (
            rows.into_iter()
                .map(|(id, action, outcome, detail, created_at)| Air11AuditView {
                    id,
                    action: truncate(&action, 64),
                    outcome: truncate(&outcome, 64),
                    detail: safe_log_line(&detail),
                    created_at: truncate(&created_at, 64),
                })
                .collect(),
            "observed".to_string(),
            None,
        ),
        Err(_) => (
            Vec::new(),
            "unavailable".to_string(),
            Some("AirSense audit history is unavailable; run the database migration".to_string()),
        ),
    }
}

fn actions_enabled(state: &AppState) -> bool {
    state.air11_actions_enabled && state.air11_admin_token.is_some()
}

fn admin_token_matches(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(expected) = state.air11_admin_token.as_ref() else {
        return false;
    };
    headers
        .get(AUTHORIZATION_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        == Some(expected.as_str())
}

async fn build_view(state: &AppState, headers: &HeaderMap) -> Air11View {
    let credentials = credential_view(&credentials_path());
    let (config, service, logs, audit_result, latest_result) = tokio::join!(
        config_view(state),
        service_view(state),
        logs_view(state),
        audit_view(state),
        latest_view(state)
    );
    let (audit, audit_state, audit_blocker) = audit_result;
    let (latest, latest_state, latest_blocker) = latest_result;
    let enabled = actions_enabled(state);
    Air11View {
        actions_enabled: enabled,
        admin_auth_configured: state.air11_admin_token.is_some(),
        action_token: if enabled && admin_token_matches(state, headers) {
            state
                .air11_action_token
                .as_ref()
                .map(|token| token.as_ref().clone())
        } else {
            None
        },
        config,
        credentials,
        service,
        status: status_view(Path::new(AIR11_STATUS_PATH)),
        logs,
        audit_state,
        audit_blocker,
        audit,
        latest_state,
        latest_blocker,
        latest,
    }
}

/// GET `/api/air11`.
pub async fn air11_handler(State(state): State<AppState>, headers: HeaderMap) -> Json<Air11View> {
    Json(build_view(&state, &headers).await)
}

/// GET `/api/air11/history?window_hours=48`.
///
/// Returns bounded, one-minute `PostgreSQL` buckets rather than browser-session
/// samples. The boundary is deliberately gap-bounded and provisional: Amendment 12
/// does not yet provide a durable therapy-session record, so this endpoint must
/// not pretend that a transport or time window is a finalized therapy session.
#[expect(
    clippy::too_many_lines,
    reason = "the bounded history projection keeps validation, query, and response states together"
)]
pub async fn air11_history_handler(
    State(state): State<AppState>,
    Query(query): Query<Air11HistoryQuery>,
) -> Json<Air11HistoryView> {
    let window_hours = match query.window_hours {
        None => 48,
        Some(hours) if (1..=48).contains(&hours) => hours,
        Some(hours) => {
            return Json(Air11HistoryView {
                state: "invalid".to_string(),
                blocker: Some(format!(
                    "history window rejected: window_hours={hours}; expected 1..=48"
                )),
                window_hours: hours,
                boundary: "unknown".to_string(),
                series: Vec::new(),
            });
        }
    };
    let Some(pool) = state.db.as_ref() else {
        return Json(Air11HistoryView {
            state: "unavailable".to_string(),
            blocker: Some(
                "database unavailable; stored AirSense history cannot be read".to_string(),
            ),
            window_hours,
            boundary: "unknown".to_string(),
            series: Vec::new(),
        });
    };
    let source = VitalsSource::ResmedAir11.id();
    let rows = sqlx::query_as::<_, (String, String, i64, f64, f64, f64, i64)>(
        "WITH minute_presence AS (
             SELECT time_bucket(INTERVAL '1 minute', timestamp) AS bucket
             FROM samples
             WHERE source = $1
               AND timestamp >= now() - ($2::int * INTERVAL '1 hour')
             GROUP BY bucket
         ), ordered_presence AS (
             SELECT bucket,
                    lag(bucket) OVER (ORDER BY bucket) AS previous_bucket
             FROM minute_presence
         ), latest_window AS (
             SELECT COALESCE(
                        MAX(bucket) FILTER (
                            WHERE previous_bucket IS NULL
                               OR bucket - previous_bucket > INTERVAL '15 minutes'
                        ),
                        MIN(bucket)
                    ) AS start_bucket
             FROM ordered_presence
         )
         SELECT registry.name,
                registry.unit,
                (EXTRACT(epoch FROM time_bucket(INTERVAL '1 minute', s.timestamp)) * 1000000)::bigint,
                AVG(s.value),
                MIN(s.value),
                MAX(s.value),
                COUNT(*)::bigint
         FROM samples AS s
         JOIN vitals_type AS registry ON registry.id = s.type
         CROSS JOIN latest_window
         WHERE s.source = $1
           AND latest_window.start_bucket IS NOT NULL
           AND s.timestamp >= latest_window.start_bucket
         GROUP BY registry.name, registry.unit, time_bucket(INTERVAL '1 minute', s.timestamp)
         ORDER BY registry.name, time_bucket(INTERVAL '1 minute', s.timestamp)"
    )
    .bind(source)
    .bind(u8::try_from(window_hours).map_or(48, i32::from))
    .fetch_all(pool)
    .await;
    let Ok(rows) = rows else {
        return Json(Air11HistoryView {
            state: "unavailable".to_string(),
            blocker: Some("stored AirSense history query failed".to_string()),
            window_hours,
            boundary: "unknown".to_string(),
            series: Vec::new(),
        });
    };
    let mut series: Vec<Air11HistorySeries> = Vec::new();
    for (name, unit, t_micros, value, min, max, count) in rows {
        let target = if let Some(target) = series.iter_mut().find(|item| item.name == name) {
            target
        } else {
            series.push(Air11HistorySeries {
                name: name.clone(),
                unit: unit.clone(),
                points: Vec::new(),
            });
            let Some(target) = series.last_mut() else {
                continue;
            };
            target
        };
        target.points.push(Air11HistoryPoint {
            t_micros,
            value,
            min,
            max,
            count,
        });
    }
    Json(Air11HistoryView {
        state: if series.is_empty() {
            "empty".to_string()
        } else {
            "observed".to_string()
        },
        blocker: if series.is_empty() {
            Some("no stored AirSense history is available".to_string())
        } else {
            None
        },
        window_hours,
        boundary: "latest_gap_bounded_window_provisional_until_therapy_sessions_exist".to_string(),
        series,
    })
}

fn action_name(action: &Air11Action) -> &'static str {
    match action {
        Air11Action::OptIn => "opt_in",
        Air11Action::OptOut => "opt_out",
        Air11Action::Start => "start",
        Air11Action::Stop => "stop",
        Air11Action::Restart => "restart",
        Air11Action::CheckCredentials => "credential_check",
    }
}

fn required_confirmation(action: &Air11Action) -> Option<&'static str> {
    match action {
        Air11Action::OptIn => Some("OPT_IN_AIR11"),
        Air11Action::OptOut => Some("OPT_OUT_AIR11"),
        Air11Action::Start => Some("START_AIR11"),
        Air11Action::Stop => Some("STOP_AIR11"),
        Air11Action::Restart => Some("RESTART_AIR11"),
        Air11Action::CheckCredentials => None,
    }
}

fn check_action_token(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<crate::models::ErrorResponse>)> {
    if !state.air11_actions_enabled {
        return Err(error_reply(
            StatusCode::FORBIDDEN,
            "AirSense action blocked: admin actions are disabled by service policy",
        ));
    }
    let Some(expected_action_token) = state.air11_action_token.as_ref() else {
        return Err(error_reply(
            StatusCode::FORBIDDEN,
            "AirSense action blocked: action capability is unavailable",
        ));
    };
    let supplied_action_token = headers
        .get(ACTION_HEADER)
        .and_then(|value| value.to_str().ok());
    if supplied_action_token != Some(expected_action_token.as_str()) {
        return Err(error_reply(
            StatusCode::FORBIDDEN,
            "AirSense action blocked: action token is missing or invalid",
        ));
    }
    if state.air11_admin_token.is_none() {
        return Err(error_reply(
            StatusCode::FORBIDDEN,
            "AirSense action blocked: external console-admin credential is unavailable",
        ));
    }
    if !admin_token_matches(state, headers) {
        return Err(error_reply(
            StatusCode::UNAUTHORIZED,
            "AirSense action blocked: console-admin authorization is missing or invalid",
        ));
    }
    Ok(())
}

fn require_pool(
    state: &AppState,
) -> Result<&sqlx::PgPool, (StatusCode, Json<crate::models::ErrorResponse>)> {
    state.db.as_ref().ok_or_else(|| {
        error_reply(
            StatusCode::SERVICE_UNAVAILABLE,
            "AirSense action blocked: database unavailable; opt-in and audit state cannot be stored",
        )
    })
}

async fn record_audit(
    pool: &sqlx::PgPool,
    action: &str,
    outcome: &str,
    detail: &str,
) -> Result<(), ()> {
    sqlx::query("INSERT INTO air11_admin_audit (action, outcome, detail) VALUES ($1, $2, $3)")
        .bind(action)
        .bind(outcome)
        .bind(detail)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|_| ())
}

async fn set_opt_in(pool: &sqlx::PgPool, opted_in: bool) -> Result<(), ()> {
    let mut transaction = pool.begin().await.map_err(|_| ())?;
    sqlx::query("SELECT set_air11_opt_in($1)")
        .bind(opted_in)
        .execute(&mut *transaction)
        .await
        .map_err(|_| ())?;
    transaction.commit().await.map_err(|_| ())
}

async fn run_systemctl(
    state: &AppState,
    action: &Air11Action,
    pool: &sqlx::PgPool,
) -> Result<(), (StatusCode, Json<crate::models::ErrorResponse>)> {
    let verb = action_name(action);
    if record_audit(
        pool,
        verb,
        "requested",
        "fixed AirSense service action requested",
    )
    .await
    .is_err()
    {
        return Err(error_reply(
            StatusCode::SERVICE_UNAVAILABLE,
            "AirSense action blocked: request could not be recorded before systemd control",
        ));
    }
    let result = (state.sys_ops.cmd)("systemctl", &[verb, AIR11_SERVICE]).await;
    if result.is_ok() {
        if record_audit(
            pool,
            verb,
            "succeeded",
            "fixed AirSense service action completed",
        )
        .await
        .is_err()
        {
            return Err(error_reply(
                StatusCode::SERVICE_UNAVAILABLE,
                "AirSense action result is unknown: systemd completed but audit append failed",
            ));
        }
        Ok(())
    } else {
        let _ = record_audit(
            pool,
            verb,
            "failed",
            "systemd did not complete the fixed action",
        )
        .await;
        Err(error_reply(
            StatusCode::BAD_GATEWAY,
            "AirSense action failed: systemd did not complete the fixed service action",
        ))
    }
}

/// POST `/api/air11`.
///
/// # Errors
///
/// Returns a named HTTP error when authorization, confirmation, database
/// preconditions, credential readiness, systemd control, or audit persistence
/// fails.
#[expect(
    clippy::too_many_lines,
    reason = "the fixed action dispatcher keeps all preconditions visible at the HTTP boundary"
)]
pub async fn air11_action_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<Air11ActionRequest>,
) -> Result<Json<Air11ActionResponse>, (StatusCode, Json<crate::models::ErrorResponse>)> {
    check_action_token(&state, &headers)?;
    let action_label = action_name(&request.action).to_string();
    if let Some(expected) = required_confirmation(&request.action)
        && request.confirmation.as_deref() != Some(expected)
    {
        return Err(error_reply(
            StatusCode::BAD_REQUEST,
            format!(
                "AirSense action blocked: confirmation value did not match action={action_label}"
            ),
        ));
    }

    match request.action {
        Air11Action::CheckCredentials => {
            let pool = require_pool(&state)?;
            let credentials = credential_view(&credentials_path());
            if credentials.state != "ready" {
                record_audit(
                    pool,
                    "credential_check",
                    "blocked",
                    "credential readiness is not ready",
                )
                .await
                .map_err(|()| {
                    error_reply(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "AirSense credential check result is unknown: audit append failed",
                    )
                })?;
                return Err(error_reply(
                    StatusCode::CONFLICT,
                    format!(
                        "AirSense credential check blocked: state={}; blocker={}",
                        credentials.state,
                        credentials
                            .blocker
                            .as_deref()
                            .unwrap_or("credential readiness is not ready")
                    ),
                ));
            }
            record_audit(
                pool,
                "credential_check",
                "succeeded",
                "credential shape checked without exposing values",
            )
            .await
            .map_err(|()| {
                error_reply(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "AirSense credential check result is unknown: audit append failed",
                )
            })?;
        }
        Air11Action::OptIn => {
            let pool = require_pool(&state)?;
            set_opt_in(pool, true).await.map_err(|()| {
                error_reply(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "AirSense opt-in failed: database state or audit append did not complete",
                )
            })?;
        }
        Air11Action::OptOut => {
            let pool = require_pool(&state)?;
            let service = service_view(&state).await;
            if !matches!(service.active_state.as_str(), "inactive" | "failed") {
                let status = if service.active_state == "unknown" {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::CONFLICT
                };
                return Err(error_reply(
                    status,
                    format!(
                        "AirSense opt-out blocked: active_state={}; service is not stopped",
                        service.active_state
                    ),
                ));
            }
            set_opt_in(pool, false).await.map_err(|()| {
                error_reply(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "AirSense opt-out failed: database state or audit append did not complete",
                )
            })?;
        }
        Air11Action::Start => {
            let pool = require_pool(&state)?;
            let config = config_view(&state).await;
            if config.state == "unavailable" {
                return Err(error_reply(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "AirSense start blocked: opt-in state is unavailable",
                ));
            }
            if config.opted_in != Some(true) {
                return Err(error_reply(
                    StatusCode::CONFLICT,
                    format!(
                        "AirSense start blocked: opted_in={}; expected true",
                        config
                            .opted_in
                            .map_or_else(|| "unknown".to_string(), |value| value.to_string())
                    ),
                ));
            }
            if credential_view(&credentials_path()).state != "ready" {
                return Err(error_reply(
                    StatusCode::CONFLICT,
                    "AirSense start blocked: external credentials are not ready; no key was accepted by this API",
                ));
            }
            run_systemctl(&state, &Air11Action::Start, pool).await?;
        }
        Air11Action::Stop | Air11Action::Restart => {
            let pool = require_pool(&state)?;
            run_systemctl(&state, &request.action, pool).await?;
        }
    }

    Ok(Json(Air11ActionResponse {
        action: action_label,
        message: "AirSense action completed; refresh status for current service evidence"
            .to_string(),
        view: build_view(&state, &headers).await,
    }))
}

trait PrivateMode {
    fn mode_is_private(&self) -> bool;
}

impl PrivateMode for std::fs::Permissions {
    fn mode_is_private(&self) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            self.mode().trailing_zeros() >= 6 && self.mode() & 0o400 != 0
        }
        #[cfg(not(unix))]
        {
            let _ = self;
            false
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "tests use bounded temporary fixtures and assertion setup"
)]
mod tests {
    use super::*;
    use axum::{Router, middleware, routing::get};
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::NamedTempFile;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        AppState {
            socket_path: PathBuf::new(),
            db: None,
            test_routes_enabled: false,
            db_url: String::new(),
            sys_ops: crate::api::ops::SysOps::real(),
            host_history: crate::api::host::HostHistory::default(),
            db_ops: std::sync::Arc::new(crate::api::postgres::PgDbOps {
                db_url: String::new(),
            }),
            security: std::sync::Arc::new(crate::security::SecurityState::unconfigured()),
            ops_token: None,
            server_instance_id: None,
            client_origin: None,
            server_display_label: "test server".to_string(),
            air11_actions_enabled: true,
            air11_action_token: Some(std::sync::Arc::new("csrf-token".to_string())),
            air11_admin_token: Some(std::sync::Arc::new("admin-token".to_string())),
            air11_allowed_networks: std::sync::Arc::new(default_allowed_networks()),
        }
    }

    #[test]
    fn credential_shape_rejects_all_zero_target_and_master_key() {
        assert!(!is_nonzero_linux_mac(&"0".repeat(12)));
        assert!(is_nonzero_linux_mac(&format!("{}1", "0".repeat(11))));
        assert!(!is_nonzero_hex("0".repeat(64).as_str(), 64));
        assert!(is_nonzero_hex(&format!("{}1", "0".repeat(63)), 64));
    }

    #[test]
    fn credential_projection_never_returns_key_or_target() {
        let mut file = NamedTempFile::new().expect("temp file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))
                .expect("private mode");
        }
        write!(
            file,
            "{{\"address\":\"00112233-4455-6677-8899-aabbccddeeff\",\"linux_address\":\"AA:BB:CC:DD:EE:FF\",\"client_id\":\"collector\",\"master_pair_key\":\"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\"}}"
        )
        .expect("JSON");
        let view = credential_view(file.path());
        assert_eq!(view.state, "ready");
        assert!(view.client_id_present);
        assert!(view.target_present);
        assert!(view.master_key_present);
        let encoded = serde_json::to_string(&view).expect("view JSON");
        assert!(!encoded.contains("00010203"));
        assert!(!encoded.contains("AA:BB:CC:DD:EE:FF"));
    }

    #[test]
    fn credential_projection_rejects_insecure_and_oversized_files() {
        let file = NamedTempFile::new().expect("temp file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o644))
                .expect("permissions");
            assert_eq!(credential_view(file.path()).state, "insecure");
        }
        let mut oversized = NamedTempFile::new().expect("oversized file");
        #[cfg(unix)]
        oversized
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .expect("private mode");
        oversized
            .write_all(&vec![
                b'x';
                usize::try_from(MAX_CREDENTIAL_BYTES)
                    .unwrap_or(usize::MAX)
                    + 1
            ])
            .expect("oversized content");
        assert_eq!(credential_view(oversized.path()).state, "invalid");
    }

    #[test]
    fn status_projection_sanitizes_private_blocker_text() {
        let mut file = NamedTempFile::new().expect("temp file");
        write!(
            file,
            "{{\"link_state\":\"degraded\",\"blocker\":\"masterPairKey leaked\",\"last_host_receipt_unix_micros\":123}}"
        )
        .expect("JSON");
        let view = status_view(file.path());
        assert_eq!(view.status_state, "observed");
        assert_eq!(
            view.blocker.as_deref(),
            Some("private collector detail was suppressed")
        );
    }

    #[test]
    fn systemctl_output_is_parsed_without_claiming_missing_fields() {
        let view = parse_systemctl(
            "ActiveState=active\nSubState=running\nNRestarts=2\nExecMainStartTimestamp=now\n",
        );
        assert_eq!(view.active_state, "active");
        assert_eq!(view.sub_state, "running");
        assert_eq!(view.restart_count, Some(2));
        assert_eq!(view.started_at.as_deref(), Some("now"));
        let empty = parse_systemctl("");
        assert_eq!(empty.active_state, "unknown");
        assert!(empty.restart_count.is_none());
    }

    #[test]
    fn journal_projection_marks_errors_and_redacts_private_tokens() {
        let logs = journal_view_output(
            r#"{"PRIORITY":"3","__REALTIME_TIMESTAMP":"123","MESSAGE":"connection failed for AA:BB:CC:DD:EE:FF"}
{"PRIORITY":"6","MESSAGE":"streaming"}"#,
        );
        assert_eq!(logs.state, "observed");
        let first = logs.entries.first().expect("first log");
        let second = logs.entries.get(1).expect("second log");
        assert_eq!(first.level, "error");
        assert!(first.message.contains("redacted"));
        assert_eq!(second.level, "info");
        assert_eq!(
            safe_log_line("address=AA:BB:CC:DD:EE:FF"),
            "private collector detail was suppressed"
        );
        let key = "a".repeat(64);
        let redacted = safe_log_line(&format!(r#"{{"key":"{key}"}}"#));
        assert!(!redacted.contains(&key));
        let numeric = journal_view_output(r#"{"PRIORITY":3,"MESSAGE":"numeric error"}"#);
        let numeric_error = numeric.entries.first().expect("numeric log");
        assert_eq!(numeric_error.level, "error");
        let partial = journal_view_output("{\"MESSAGE\":\"ok\"}\nnot-json");
        assert!(partial.blocker.is_some());
        let oversized = journal_view_output(&"x".repeat(MAX_JOURNAL_OUTPUT_BYTES + 1));
        assert_eq!(oversized.state, "invalid");
    }

    #[test]
    fn local_network_allowlist_handles_lan_and_rejects_public_peer() {
        let networks = parse_allowed_networks("192.168.0.0/16,fd00::/8").expect("CIDRs");
        assert!(peer_allowed(
            &networks,
            "192.168.1.25".parse().expect("LAN IP")
        ));
        assert!(!peer_allowed(
            &networks,
            "203.0.113.10".parse().expect("public IP")
        ));
        assert!(peer_allowed(&networks, "fd12::10".parse().expect("ULA IP")));
    }

    #[test]
    fn malformed_local_network_allowlist_fails_closed() {
        assert!(parse_allowed_networks("192.168.1.0/33").is_err());
        assert!(parse_allowed_networks(",192.168.1.0/24").is_err());
    }

    #[test]
    fn action_capability_is_visible_only_with_the_external_admin_token() {
        let state = test_state();
        let empty = HeaderMap::new();
        assert!(!admin_token_matches(&state, &empty));
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION_HEADER,
            axum::http::HeaderValue::from_static("Bearer wrong-token"),
        );
        assert!(!admin_token_matches(&state, &headers));
        headers.insert(
            AUTHORIZATION_HEADER,
            axum::http::HeaderValue::from_static("Bearer admin-token"),
        );
        assert!(admin_token_matches(&state, &headers));
    }

    #[test]
    fn fixed_actions_require_both_capabilities() {
        let state = test_state();
        let mut headers = HeaderMap::new();
        headers.insert(
            ACTION_HEADER,
            axum::http::HeaderValue::from_static("csrf-token"),
        );
        assert!(check_action_token(&state, &headers).is_err());
        headers.insert(
            AUTHORIZATION_HEADER,
            axum::http::HeaderValue::from_static("Bearer admin-token"),
        );
        assert!(check_action_token(&state, &headers).is_ok());
    }

    #[tokio::test]
    async fn network_guard_rejects_missing_and_public_peers() {
        let state = test_state();
        let app = Router::new()
            .route("/", get(|| async { StatusCode::NO_CONTENT }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                air11_network_guard,
            ))
            .with_state(state);

        let missing = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request");
        let response = app.clone().oneshot(missing).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let mut public = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request");
        public
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([203, 0, 113, 10], 8480))));
        let response = app.clone().oneshot(public).await.expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let mut lan = Request::builder()
            .uri("/")
            .body(Body::empty())
            .expect("request");
        lan.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([192, 168, 1, 25], 8480))));
        let response = app.oneshot(lan).await.expect("response");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn history_rejects_an_unbounded_window_without_clamping() {
        let view = air11_history_handler(
            State(test_state()),
            Query(Air11HistoryQuery {
                window_hours: Some(49),
            }),
        )
        .await
        .0;
        assert_eq!(view.state, "invalid");
        assert_eq!(view.window_hours, 49);
        assert!(
            view.blocker
                .as_deref()
                .is_some_and(|message| message.contains("window_hours=49"))
        );
    }

    #[tokio::test]
    async fn action_handler_names_database_unavailability() {
        let state = test_state();
        let mut headers = HeaderMap::new();
        headers.insert(
            ACTION_HEADER,
            axum::http::HeaderValue::from_static("csrf-token"),
        );
        headers.insert(
            AUTHORIZATION_HEADER,
            axum::http::HeaderValue::from_static("Bearer admin-token"),
        );
        let request = Air11ActionRequest {
            action: Air11Action::OptIn,
            confirmation: Some("OPT_IN_AIR11".to_string()),
        };
        let error = air11_action_handler(State(state), headers, Json(request))
            .await
            .expect_err("missing database must block mutation");
        assert_eq!(error.0, StatusCode::SERVICE_UNAVAILABLE);
        assert!(error.1.0.error.contains("database unavailable"));
    }
}
