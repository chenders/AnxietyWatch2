//! Read-only `AirSense` 11 JSON-RPC messages and correlation.
//!
//! Request construction is typed so production code has no arbitrary method
//! entry point.  The final transport boundary validates the serialized body
//! again, which is deliberate defense in depth: an allowed request must also
//! have the fixed parameter shape, and a forbidden method must be rejected
//! before the transport is called.

use std::{future::Future, pin::Pin};

use serde::{
    Deserializer,
    de::{self, MapAccess, Visitor},
};
use serde_json::{Value, json};
use zeroize::Zeroize;

use crate::as11_crypto::{MAX_JSON_REQUEST_BYTES, MAX_JSON_RESPONSE_BYTES, encode_hex};

/// Plaintext host-to-device FIG VCID used during session authentication.
pub const PLAINTEXT_REQUEST_VCID: u16 = 0x0393;
/// Plaintext device-to-host FIG VCID used during session authentication.
pub const PLAINTEXT_RESPONSE_VCID: u16 = 0x0392;
/// Encrypted host-to-device FIG VCID used for application RPCs.
pub const ENCRYPTED_REQUEST_VCID: u16 = 0x0397;
/// Encrypted device-to-host FIG VCID used for application RPCs.
pub const ENCRYPTED_RESPONSE_VCID: u16 = 0x0396;
/// The fixed requested `AirSense` stream identifiers.
pub const AIR11_STREAM_IDS: [&str; 17] = [
    "_RFL", "_MKP", "_MKF", "_MKI", "_MKE", "_LKF", "_RR2", "_TD2", "_MV2", "_TGT", "_IE2", "_SNI",
    "_FFL", "_INT", "_BYV", "_HRT", "_SAO",
];
/// The fixed read-only event selectors sent in `SubscribeEvent`.
pub const AIR11_EVENT_IDS: [&str; 5] = [
    "SystemActivityEvents-FrequentActivityEvents",
    "SystemActivityEvents-SporadicActivityEvents",
    "UsageEvents-TherapyStatusEvents",
    "SettingsHistoryChangeCount",
    "TherapyEvents-RespiratoryEvents",
];
/// The fixed source sample interval sent in `StartStream`.
pub const AIR11_SAMPLE_INTERVAL_MS: u64 = 40;
/// The fixed source report interval sent in `StartStream`.
pub const AIR11_REPORT_INTERVAL_MS: u64 = 200;
/// A bounded client identifier size for the session request.
pub const MAX_CLIENT_ID_BYTES: usize = 128;
/// The methods that may reach the final send boundary.
pub const READ_ONLY_ALLOWLIST: &str =
    "RequestSession,CheckSessionIntegrity,GetVersion,GetDateTime,StartStream,SubscribeEvent";

/// A method that production `AirSense` code is permitted to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum As11RpcMethod {
    /// Begin reconnect authentication on the plaintext lane.
    RequestSession,
    /// Complete reconnect authentication on the plaintext lane.
    CheckSessionIntegrity,
    /// Read the device version on the encrypted lane.
    GetVersion,
    /// Read the device date/time on the encrypted lane.
    GetDateTime,
    /// Start the fixed therapy observation stream on the encrypted lane.
    StartStream,
    /// Subscribe to fixed lifecycle and respiratory event notifications.
    SubscribeEvent,
}

impl As11RpcMethod {
    /// Returns the exact method spelling required by `AirSense`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequestSession => "RequestSession",
            Self::CheckSessionIntegrity => "CheckSessionIntegrity",
            Self::GetVersion => "GetVersion",
            Self::GetDateTime => "GetDateTime",
            Self::StartStream => "StartStream",
            Self::SubscribeEvent => "SubscribeEvent",
        }
    }

    const fn request_version(self) -> &'static str {
        match self {
            Self::RequestSession | Self::CheckSessionIntegrity | Self::GetVersion => "2.0",
            Self::GetDateTime | Self::StartStream | Self::SubscribeEvent => "1.0",
        }
    }

    const fn request_vcid(self) -> u16 {
        match self {
            Self::RequestSession | Self::CheckSessionIntegrity => PLAINTEXT_REQUEST_VCID,
            Self::GetVersion | Self::GetDateTime | Self::StartStream | Self::SubscribeEvent => {
                ENCRYPTED_REQUEST_VCID
            }
        }
    }

    const fn response_vcid(self) -> u16 {
        match self {
            Self::RequestSession | Self::CheckSessionIntegrity => PLAINTEXT_RESPONSE_VCID,
            Self::GetVersion | Self::GetDateTime | Self::StartStream | Self::SubscribeEvent => {
                ENCRYPTED_RESPONSE_VCID
            }
        }
    }

    fn from_wire(value: &str) -> Option<Self> {
        match value {
            "RequestSession" => Some(Self::RequestSession),
            "CheckSessionIntegrity" => Some(Self::CheckSessionIntegrity),
            "GetVersion" => Some(Self::GetVersion),
            "GetDateTime" => Some(Self::GetDateTime),
            "StartStream" => Some(Self::StartStream),
            "SubscribeEvent" => Some(Self::SubscribeEvent),
            _ => None,
        }
    }
}

/// An error constructing or validating a read-only request.
///
/// The error never includes a serialized request, credential, proof, or key.
#[derive(Debug, PartialEq, Eq)]
pub enum RpcError {
    /// The method was not in the final send allowlist.
    MethodNotAllowed {
        /// The rejected method spelling.
        method: String,
    },
    /// A request parameter was empty, malformed, or otherwise not fixed.
    InvalidParameter {
        /// The parameter name.
        name: &'static str,
        /// The named reason.
        reason: &'static str,
    },
    /// A parameter exceeded its fixed bound.
    ParameterTooLarge {
        /// The parameter name.
        name: &'static str,
        /// The observed length.
        len: usize,
        /// The largest accepted length.
        max: usize,
    },
    /// JSON serialization failed.
    Serialization,
    /// A serialized request exceeded the bounded RPC body.
    RequestTooLarge {
        /// The observed length.
        len: usize,
        /// The largest accepted length.
        max: usize,
    },
    /// A serialized request no longer matched its typed request handle.
    RequestMismatch,
    /// A request body was not a valid JSON-RPC request.
    InvalidRequest {
        /// The named validation failure.
        reason: &'static str,
    },
}

impl core::fmt::Display for RpcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MethodNotAllowed { method } => write!(
                f,
                "method={method} rejected; allowlist={READ_ONLY_ALLOWLIST}"
            ),
            Self::InvalidParameter { name, reason } => write!(f, "{name}={reason}"),
            Self::ParameterTooLarge { name, len, max } => {
                write!(f, "{name} length {len} exceeds {max}-byte limit")
            }
            Self::Serialization => f.write_str("rpc_json_serialization_failed"),
            Self::RequestTooLarge { len, max } => {
                write!(f, "rpc request length {len} exceeds {max}-byte limit")
            }
            Self::RequestMismatch => f.write_str("rpc_request_handle_mismatch"),
            Self::InvalidRequest { reason } => write!(f, "rpc_request_{reason}"),
        }
    }
}

impl std::error::Error for RpcError {}

/// A typed, serialized `AirSense` JSON-RPC request.
///
/// This type intentionally does not implement `Debug`, `Display`, or
/// serialization because authentication requests contain client material.
pub struct RpcRequest {
    id: u64,
    method: As11RpcMethod,
    payload: Vec<u8>,
}

impl Drop for RpcRequest {
    fn drop(&mut self) {
        self.payload.zeroize();
    }
}

impl RpcRequest {
    /// Builds a reconnect `RequestSession` request.
    ///
    /// # Errors
    /// Returns [`RpcError`] if `client_id` is empty, too large, or the fixed
    /// JSON body cannot be produced.
    pub fn request_session(id: u64, client_id: &str) -> Result<Self, RpcError> {
        if client_id.is_empty() {
            return Err(RpcError::InvalidParameter {
                name: "client_id",
                reason: "empty",
            });
        }
        if client_id.len() > MAX_CLIENT_ID_BYTES {
            return Err(RpcError::ParameterTooLarge {
                name: "client_id",
                len: client_id.len(),
                max: MAX_CLIENT_ID_BYTES,
            });
        }
        Self::build(
            id,
            As11RpcMethod::RequestSession,
            Some(json!({ "clientId": client_id })),
        )
    }

    /// Builds a reconnect `CheckSessionIntegrity` request from a 32-byte HMAC.
    ///
    /// # Errors
    /// Returns [`RpcError::Serialization`] or [`RpcError::RequestTooLarge`]
    /// if the fixed JSON body cannot be emitted within the protocol bound.
    pub fn check_session_integrity(id: u64, response: &[u8; 32]) -> Result<Self, RpcError> {
        Self::build(
            id,
            As11RpcMethod::CheckSessionIntegrity,
            Some(json!({ "response": encode_hex(response) })),
        )
    }

    /// Builds an encrypted read-only `GetVersion` request.
    ///
    /// # Errors
    /// Returns [`RpcError`] if the fixed JSON body cannot be emitted within
    /// the protocol bound.
    pub fn get_version(id: u64) -> Result<Self, RpcError> {
        Self::build(id, As11RpcMethod::GetVersion, None)
    }

    /// Builds an encrypted read-only `GetDateTime` request.
    ///
    /// # Errors
    /// Returns [`RpcError`] if the fixed JSON body cannot be emitted within
    /// the protocol bound.
    pub fn get_date_time(id: u64) -> Result<Self, RpcError> {
        Self::build(id, As11RpcMethod::GetDateTime, None)
    }

    /// Builds the only permitted `StartStream` request.
    ///
    /// # Errors
    /// Returns [`RpcError`] if the fixed JSON body cannot be emitted within
    /// the protocol bound.
    pub fn start_stream(id: u64) -> Result<Self, RpcError> {
        let data_ids = Value::Array(
            AIR11_STREAM_IDS
                .iter()
                .map(|name| Value::String((*name).to_owned()))
                .collect(),
        );
        Self::build(
            id,
            As11RpcMethod::StartStream,
            Some(json!({
                "dataIds": data_ids,
                "sampleIntervalMs": AIR11_SAMPLE_INTERVAL_MS,
                "reportIntervalMs": AIR11_REPORT_INTERVAL_MS,
            })),
        )
    }

    /// Builds the fixed read-only lifecycle and respiratory event subscription.
    ///
    /// # Errors
    /// Returns [`RpcError`] if the fixed JSON body cannot be emitted within the
    /// protocol bound.
    pub fn subscribe_events(id: u64) -> Result<Self, RpcError> {
        let data_ids = Value::Array(
            AIR11_EVENT_IDS
                .iter()
                .map(|name| Value::String((*name).to_owned()))
                .collect(),
        );
        Self::build(
            id,
            As11RpcMethod::SubscribeEvent,
            Some(json!({ "dataIds": data_ids })),
        )
    }

    fn build(id: u64, method: As11RpcMethod, params: Option<Value>) -> Result<Self, RpcError> {
        let value = match params {
            Some(params) => json!({
                "jsonrpc": method.request_version(),
                "method": method.as_str(),
                "id": id,
                "params": params,
            }),
            None => json!({
                "jsonrpc": method.request_version(),
                "method": method.as_str(),
                "id": id,
            }),
        };
        let payload = serde_json::to_vec(&value).map_err(|_| RpcError::Serialization)?;
        if payload.len() > MAX_JSON_REQUEST_BYTES {
            return Err(RpcError::RequestTooLarge {
                len: payload.len(),
                max: MAX_JSON_REQUEST_BYTES,
            });
        }
        let request = Self {
            id,
            method,
            payload,
        };
        validate_wire_request(&request.payload, Some(method), Some(id))?;
        Ok(request)
    }

    /// Returns the request identifier used for response correlation.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the allowlisted method represented by this request.
    #[must_use]
    pub const fn method(&self) -> As11RpcMethod {
        self.method
    }

    /// Returns the FIG VCID on which this request must be sent.
    #[must_use]
    pub const fn request_vcid(&self) -> u16 {
        self.method.request_vcid()
    }

    /// Returns the FIG VCID on which this request's response must arrive.
    #[must_use]
    pub const fn response_vcid(&self) -> u16 {
        self.method.response_vcid()
    }

    /// Returns the serialized JSON-RPC body for the FIG encoder.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// A transport that can perform one final FIG/GATT write.
pub trait RpcTransport {
    /// The transport-specific failure type.
    type Error;

    /// Writes one already-framed-lane JSON body.
    ///
    /// # Errors
    /// Returns the transport's error when the underlying GATT/FIG write fails.
    fn write(&mut self, vcid: u16, payload: &[u8]) -> Result<(), Self::Error>;
}

/// A transport that can perform one final asynchronous FIG/GATT write.
pub trait AsyncRpcTransport {
    /// The transport-specific failure type.
    type Error;

    /// Writes one already-framed-lane JSON body.
    ///
    /// # Errors
    /// Returns the transport-specific failure when the underlying GATT write fails.
    fn write<'a>(
        &'a mut self,
        vcid: u16,
        payload: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>>;
}

/// A final send boundary that enforces the read-only method allowlist.
pub struct ReadOnlyRpcLink<T> {
    transport: T,
}

impl<T> ReadOnlyRpcLink<T> {
    /// Wraps a transport with the final `AirSense` read-only gate.
    #[must_use]
    pub const fn new(transport: T) -> Self {
        Self { transport }
    }

    /// Returns the wrapped transport after the link is no longer needed.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.transport
    }
}

/// The result of a final send boundary operation.
#[derive(Debug, PartialEq, Eq)]
pub enum RpcSendError<E> {
    /// The request was rejected before the transport was called.
    Rejected(RpcError),
    /// The transport rejected the write.
    Transport(E),
}

impl<T> ReadOnlyRpcLink<T>
where
    T: RpcTransport,
{
    /// Validates and sends one typed, allowlisted request.
    ///
    /// # Errors
    /// Returns [`RpcSendError::Rejected`] before any transport call when the
    /// request body is malformed, changed, or not in the read-only allowlist.
    /// Returns [`RpcSendError::Transport`] when the transport write fails.
    pub fn send(&mut self, request: &RpcRequest) -> Result<(), RpcSendError<T::Error>> {
        validate_wire_request(&request.payload, Some(request.method), Some(request.id))
            .map_err(RpcSendError::Rejected)?;
        self.transport
            .write(request.request_vcid(), &request.payload)
            .map_err(RpcSendError::Transport)
    }

    #[cfg(test)]
    fn send_wire_for_test(
        &mut self,
        vcid: u16,
        payload: &[u8],
    ) -> Result<(), RpcSendError<T::Error>> {
        let (method, _) =
            validate_wire_request(payload, None, None).map_err(RpcSendError::Rejected)?;
        self.transport
            .write(vcid, payload)
            .map_err(RpcSendError::Transport)
            .map(|()| {
                let _ = method;
            })
    }
}

impl<T> ReadOnlyRpcLink<T>
where
    T: AsyncRpcTransport,
{
    /// Validates and asynchronously sends one typed, allowlisted request.
    ///
    /// # Errors
    /// Returns [`RpcSendError::Rejected`] before any transport call when the
    /// request body is malformed or not allowlisted. Returns
    /// [`RpcSendError::Transport`] when the asynchronous transport fails.
    pub async fn send_async(&mut self, request: &RpcRequest) -> Result<(), RpcSendError<T::Error>> {
        validate_wire_request(&request.payload, Some(request.method), Some(request.id))
            .map_err(RpcSendError::Rejected)?;
        self.transport
            .write(request.request_vcid(), &request.payload)
            .await
            .map_err(RpcSendError::Transport)
    }
}

struct RootObjectVisitor;

impl<'de> Visitor<'de> for RootObjectVisitor {
    type Value = serde_json::Map<String, Value>;

    fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("a JSON object without duplicate members")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        while let Some(key) = access.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON object member"));
            }
            let value = access.next_value::<Value>()?;
            object.insert(key, value);
        }
        Ok(object)
    }
}

fn parse_root_object(payload: &[u8]) -> Result<serde_json::Map<String, Value>, ()> {
    let mut deserializer = serde_json::Deserializer::from_slice(payload);
    let object = deserializer
        .deserialize_map(RootObjectVisitor)
        .map_err(|_| ())?;
    deserializer.end().map_err(|_| ())?;
    Ok(object)
}

fn value_object<'a>(
    value: &'a Value,
    reason: &'static str,
) -> Result<&'a serde_json::Map<String, Value>, RpcError> {
    value.as_object().ok_or(RpcError::InvalidRequest { reason })
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    reason: &'static str,
) -> Result<&'a str, RpcError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or(RpcError::InvalidRequest { reason })
}

fn required_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    reason: &'static str,
) -> Result<u64, RpcError> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(RpcError::InvalidRequest { reason })
}

fn validate_event_params(params: Option<&Value>) -> Result<(), RpcError> {
    let params = params.ok_or(RpcError::InvalidRequest {
        reason: "event_params_missing",
    })?;
    let params = value_object(params, "params_not_object")?;
    if params.len() != 1 {
        return Err(RpcError::InvalidRequest {
            reason: "event_params_shape",
        });
    }
    let ids = params
        .get("dataIds")
        .and_then(Value::as_array)
        .ok_or(RpcError::InvalidRequest {
            reason: "event_ids_missing",
        })?;
    if ids.len() != AIR11_EVENT_IDS.len()
        || !AIR11_EVENT_IDS
            .iter()
            .zip(ids)
            .all(|(expected, actual)| actual.as_str().is_some_and(|value| value == *expected))
    {
        return Err(RpcError::InvalidRequest {
            reason: "event_ids_not_fixed",
        });
    }
    Ok(())
}

fn validate_params_shape(params: Option<&Value>, method: As11RpcMethod) -> Result<(), RpcError> {
    match method {
        As11RpcMethod::GetVersion | As11RpcMethod::GetDateTime => {
            if params.is_some() {
                return Err(RpcError::InvalidRequest {
                    reason: "unexpected_params",
                });
            }
        }
        As11RpcMethod::RequestSession => {
            let params = params.ok_or(RpcError::InvalidRequest {
                reason: "client_id_missing",
            })?;
            let params = value_object(params, "params_not_object")?;
            if params.len() != 1 {
                return Err(RpcError::InvalidRequest {
                    reason: "client_id_shape",
                });
            }
            let client_id = required_string(params, "clientId", "client_id_missing")?;
            if client_id.is_empty() {
                return Err(RpcError::InvalidParameter {
                    name: "client_id",
                    reason: "empty",
                });
            }
            if client_id.len() > MAX_CLIENT_ID_BYTES {
                return Err(RpcError::ParameterTooLarge {
                    name: "client_id",
                    len: client_id.len(),
                    max: MAX_CLIENT_ID_BYTES,
                });
            }
        }
        As11RpcMethod::CheckSessionIntegrity => {
            let params = params.ok_or(RpcError::InvalidRequest {
                reason: "response_missing",
            })?;
            let params = value_object(params, "params_not_object")?;
            if params.len() != 1 {
                return Err(RpcError::InvalidRequest {
                    reason: "response_shape",
                });
            }
            let response = required_string(params, "response", "response_missing")?;
            if response.len() != 64 || !response.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(RpcError::InvalidParameter {
                    name: "response",
                    reason: "must_be_64_hex_chars",
                });
            }
        }
        As11RpcMethod::StartStream => {
            let params = params.ok_or(RpcError::InvalidRequest {
                reason: "stream_params_missing",
            })?;
            let params = value_object(params, "params_not_object")?;
            if params.len() != 3 {
                return Err(RpcError::InvalidRequest {
                    reason: "stream_params_shape",
                });
            }
            let ids = params.get("dataIds").and_then(Value::as_array).ok_or(
                RpcError::InvalidRequest {
                    reason: "stream_ids_missing",
                },
            )?;
            if ids.len() != AIR11_STREAM_IDS.len()
                || !AIR11_STREAM_IDS.iter().zip(ids).all(|(expected, actual)| {
                    actual.as_str().is_some_and(|value| value == *expected)
                })
            {
                return Err(RpcError::InvalidRequest {
                    reason: "stream_ids_not_fixed",
                });
            }
            let sample_interval =
                required_u64(params, "sampleIntervalMs", "sample_interval_missing")?;
            let report_interval =
                required_u64(params, "reportIntervalMs", "report_interval_missing")?;
            if sample_interval != AIR11_SAMPLE_INTERVAL_MS {
                return Err(RpcError::InvalidParameter {
                    name: "sampleIntervalMs",
                    reason: "not_40",
                });
            }
            if report_interval != AIR11_REPORT_INTERVAL_MS {
                return Err(RpcError::InvalidParameter {
                    name: "reportIntervalMs",
                    reason: "not_200",
                });
            }
        }
        As11RpcMethod::SubscribeEvent => validate_event_params(params)?,
    }
    Ok(())
}

fn validate_wire_request(
    payload: &[u8],
    expected_method: Option<As11RpcMethod>,
    expected_id: Option<u64>,
) -> Result<(As11RpcMethod, u64), RpcError> {
    if payload.len() > MAX_JSON_REQUEST_BYTES {
        return Err(RpcError::RequestTooLarge {
            len: payload.len(),
            max: MAX_JSON_REQUEST_BYTES,
        });
    }
    let object = parse_root_object(payload).map_err(|()| RpcError::InvalidRequest {
        reason: "json_invalid",
    })?;
    let method_text = required_string(&object, "method", "method_missing")?;
    let Some(method) = As11RpcMethod::from_wire(method_text) else {
        return Err(RpcError::MethodNotAllowed {
            method: method_text.to_owned(),
        });
    };
    let version = required_string(&object, "jsonrpc", "jsonrpc_missing")?;
    if version != method.request_version() {
        return Err(RpcError::InvalidRequest {
            reason: "jsonrpc_version",
        });
    }
    if expected_method.is_some_and(|expected| expected != method) {
        return Err(RpcError::RequestMismatch);
    }
    let id = required_u64(&object, "id", "id_missing_or_invalid")?;
    if expected_id.is_some_and(|expected| expected != id) {
        return Err(RpcError::RequestMismatch);
    }
    validate_params_shape(object.get("params"), method)?;
    Ok((method, id))
}

/// Errors found while correlating a response to one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcResponseError {
    /// The response body exceeded the bounded JSON capacity.
    ResponseTooLarge {
        /// The observed length.
        len: usize,
        /// The largest accepted length.
        max: usize,
    },
    /// The response was not valid JSON-RPC.
    InvalidResponse {
        /// The named validation failure.
        reason: &'static str,
    },
    /// A notification or `StreamData` event cannot satisfy a pending request.
    Notification,
    /// The response arrived on the wrong FIG lane.
    VcidMismatch {
        /// The expected response VCID.
        expected: u16,
        /// The observed response VCID.
        actual: u16,
    },
    /// The response identifier did not match the pending request.
    IdMismatch {
        /// The pending request identifier.
        expected: u64,
        /// The received identifier, if it was numeric.
        actual: Option<u64>,
    },
}

impl core::fmt::Display for RpcResponseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ResponseTooLarge { len, max } => {
                write!(f, "rpc response length {len} exceeds {max}-byte limit")
            }
            Self::InvalidResponse { reason } => write!(f, "rpc_response_{reason}"),
            Self::Notification => f.write_str("rpc_notification_not_a_response"),
            Self::VcidMismatch { expected, actual } => {
                write!(f, "response vcid=0x{actual:04x}, expected=0x{expected:04x}")
            }
            Self::IdMismatch { expected, actual } => {
                write!(f, "response id={actual:?}, expected={expected}")
            }
        }
    }
}

impl std::error::Error for RpcResponseError {}

/// Returns whether a response-lane packet should be ignored while waiting for
/// the correlated response. `AirSense` can interleave notifications, another
/// FIG lane, or a stale response with a request response.
#[cfg(any(test, target_os = "linux"))]
#[must_use]
pub(crate) const fn is_response_noise(error: &RpcResponseError) -> bool {
    matches!(
        error,
        RpcResponseError::Notification
            | RpcResponseError::VcidMismatch { .. }
            | RpcResponseError::IdMismatch { .. }
    )
}

/// A device error returned for a correlated request.
///
/// This type has no `Debug` implementation because the optional device data is
/// arbitrary JSON and must not be casually written to logs.
pub struct RpcDeviceError {
    /// Device-defined error code.
    pub code: i64,
    /// Device-defined human-readable message.
    pub message: String,
    /// Optional device error data.
    pub data: Option<Value>,
}

/// A response that matched both the expected FIG lane and JSON-RPC id.
///
/// This type deliberately has no `Debug` implementation: a result may contain
/// device metadata that should not be logged by default.
pub enum RpcResponse {
    /// A successful JSON-RPC result.
    Result {
        /// Correlated request id.
        id: u64,
        /// Device result value.
        result: Value,
    },
    /// A correlated JSON-RPC device error.
    Error {
        /// Correlated request id.
        id: u64,
        /// Device error object.
        error: RpcDeviceError,
    },
}

impl RpcResponse {
    /// Returns the correlated request id.
    #[must_use]
    pub const fn id(&self) -> u64 {
        match self {
            Self::Result { id, .. } | Self::Error { id, .. } => *id,
        }
    }

    /// Returns the result value, if the device reported success.
    #[must_use]
    pub fn result(&self) -> Option<&Value> {
        match self {
            Self::Result { result, .. } => Some(result),
            Self::Error { .. } => None,
        }
    }

    /// Returns the device error, if the device reported failure.
    #[must_use]
    pub const fn device_error(&self) -> Option<&RpcDeviceError> {
        match self {
            Self::Result { .. } => None,
            Self::Error { error, .. } => Some(error),
        }
    }
}

/// Parses one response and requires the exact request id and response VCID.
///
/// # Errors
/// Returns [`RpcResponseError`] for an oversized, malformed, uncorrelated,
/// wrong-lane, or notification response.
pub fn correlate_response(
    request: &RpcRequest,
    response_vcid: u16,
    payload: &[u8],
) -> Result<RpcResponse, RpcResponseError> {
    if response_vcid != request.response_vcid() {
        return Err(RpcResponseError::VcidMismatch {
            expected: request.response_vcid(),
            actual: response_vcid,
        });
    }
    if payload.len() > MAX_JSON_RESPONSE_BYTES {
        return Err(RpcResponseError::ResponseTooLarge {
            len: payload.len(),
            max: MAX_JSON_RESPONSE_BYTES,
        });
    }
    let object = parse_root_object(payload).map_err(|()| RpcResponseError::InvalidResponse {
        reason: "json_invalid",
    })?;
    if object.get("method").is_some() || object.get("id").is_none() {
        return Err(RpcResponseError::Notification);
    }
    let version =
        object
            .get("jsonrpc")
            .and_then(Value::as_str)
            .ok_or(RpcResponseError::InvalidResponse {
                reason: "jsonrpc_missing",
            })?;
    if version != "2.0" {
        return Err(RpcResponseError::InvalidResponse {
            reason: "jsonrpc_version",
        });
    }
    let actual_id = object.get("id").and_then(Value::as_u64);
    if actual_id != Some(request.id()) {
        return Err(RpcResponseError::IdMismatch {
            expected: request.id(),
            actual: actual_id,
        });
    }
    match (object.get("result"), object.get("error")) {
        (Some(result), None) => Ok(RpcResponse::Result {
            id: request.id(),
            result: result.clone(),
        }),
        (None, Some(error)) => {
            let error = error.as_object().ok_or(RpcResponseError::InvalidResponse {
                reason: "error_not_object",
            })?;
            let code = error.get("code").and_then(Value::as_i64).ok_or(
                RpcResponseError::InvalidResponse {
                    reason: "error_code_missing",
                },
            )?;
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .filter(|message| !message.is_empty())
                .ok_or(RpcResponseError::InvalidResponse {
                    reason: "error_message_missing",
                })?
                .to_owned();
            Ok(RpcResponse::Error {
                id: request.id(),
                error: RpcDeviceError {
                    code,
                    message,
                    data: error.get("data").cloned(),
                },
            })
        }
        (Some(_), Some(_)) => Err(RpcResponseError::InvalidResponse {
            reason: "result_and_error_both_present",
        }),
        (None, None) => Err(RpcResponseError::InvalidResponse {
            reason: "result_or_error_missing",
        }),
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions inspect fixed synthetic JSON only"
)]
mod tests {
    use super::*;

    struct RecordingTransport {
        writes: usize,
        vcids: Vec<u16>,
        payloads: Vec<Vec<u8>>,
    }

    impl RecordingTransport {
        fn new() -> Self {
            Self {
                writes: 0,
                vcids: Vec::new(),
                payloads: Vec::new(),
            }
        }
    }

    impl RpcTransport for RecordingTransport {
        type Error = &'static str;

        fn write(&mut self, vcid: u16, payload: &[u8]) -> Result<(), Self::Error> {
            self.writes += 1;
            self.vcids.push(vcid);
            self.payloads.push(payload.to_vec());
            Ok(())
        }
    }

    #[test]
    fn typed_requests_use_the_expected_lanes_and_shapes() {
        let request = RpcRequest::request_session(7, "synthetic-client").expect("request");
        assert_eq!(request.method(), As11RpcMethod::RequestSession);
        assert_eq!(request.request_vcid(), PLAINTEXT_REQUEST_VCID);
        assert_eq!(request.response_vcid(), PLAINTEXT_RESPONSE_VCID);
        let value: Value = serde_json::from_slice(request.payload()).expect("JSON");
        assert_eq!(value["params"]["clientId"], "synthetic-client");

        let date_time = RpcRequest::get_date_time(8).expect("date-time request");
        let date_value: Value = serde_json::from_slice(date_time.payload()).expect("JSON");
        assert_eq!(date_value["jsonrpc"], "1.0");

        let stream = RpcRequest::start_stream(9).expect("stream request");
        assert_eq!(stream.request_vcid(), ENCRYPTED_REQUEST_VCID);
        let stream_value: Value = serde_json::from_slice(stream.payload()).expect("JSON");
        assert_eq!(stream_value["jsonrpc"], "1.0");
        assert_eq!(
            stream_value["params"]["sampleIntervalMs"],
            AIR11_SAMPLE_INTERVAL_MS
        );
        assert_eq!(
            stream_value["params"]["reportIntervalMs"],
            AIR11_REPORT_INTERVAL_MS
        );
        assert_eq!(
            stream_value["params"]["dataIds"].as_array().map(Vec::len),
            Some(17)
        );

        let events = RpcRequest::subscribe_events(10).expect("event request");
        assert_eq!(events.method(), As11RpcMethod::SubscribeEvent);
        assert_eq!(events.request_vcid(), ENCRYPTED_REQUEST_VCID);
        let event_value: Value = serde_json::from_slice(events.payload()).expect("JSON");
        assert_eq!(event_value["jsonrpc"], "1.0");
        assert_eq!(
            event_value["params"]["dataIds"].as_array().map(Vec::len),
            Some(AIR11_EVENT_IDS.len())
        );
        assert_eq!(
            event_value["params"]["dataIds"][4],
            "TherapyEvents-RespiratoryEvents"
        );
    }

    #[test]
    fn forbidden_method_is_rejected_before_any_transport_write() {
        let mut link = ReadOnlyRpcLink::new(RecordingTransport::new());
        let forbidden = br#"{"jsonrpc":"2.0","method":"Set","id":1,"params":{}}"#;
        let error = link
            .send_wire_for_test(ENCRYPTED_REQUEST_VCID, forbidden)
            .expect_err("Set must be rejected");
        assert!(matches!(
            &error,
            RpcSendError::Rejected(RpcError::MethodNotAllowed { .. })
        ));
        if let RpcSendError::Rejected(RpcError::MethodNotAllowed { method }) = error {
            assert_eq!(method, "Set");
        }
        assert_eq!(link.transport.writes, 0);
        assert!(
            link.send_wire_for_test(
                ENCRYPTED_REQUEST_VCID,
                br#"{"jsonrpc":"2.0","method":"EnterTherapy","id":2}"#
            )
            .is_err()
        );
        assert_eq!(link.transport.writes, 0);
    }

    #[test]
    fn typed_allowed_request_reaches_transport_once() {
        let mut link = ReadOnlyRpcLink::new(RecordingTransport::new());
        let request = RpcRequest::get_version(12).expect("request");
        link.send(&request).expect("transport write");
        assert_eq!(link.transport.writes, 1);
        assert_eq!(link.transport.vcids, vec![ENCRYPTED_REQUEST_VCID]);
    }

    #[test]
    fn response_correlation_rejects_notifications_and_wrong_ids() {
        let request = RpcRequest::get_version(4).expect("request");
        let notification = br#"{"jsonrpc":"2.0","method":"StreamData","params":{}}"#;
        let notification_error =
            correlate_response(&request, ENCRYPTED_RESPONSE_VCID, notification)
                .err()
                .expect("notification");
        assert_eq!(notification_error, RpcResponseError::Notification);
        assert!(is_response_noise(&notification_error));
        let wrong_id = br#"{"jsonrpc":"2.0","id":5,"result":true}"#;
        let wrong_id_error = correlate_response(&request, ENCRYPTED_RESPONSE_VCID, wrong_id)
            .err()
            .expect("wrong id");
        assert_eq!(
            wrong_id_error,
            RpcResponseError::IdMismatch {
                expected: 4,
                actual: Some(5),
            }
        );
        assert!(is_response_noise(&wrong_id_error));
        let wrong_vcid_error = correlate_response(&request, PLAINTEXT_RESPONSE_VCID, br"{}")
            .err()
            .expect("wrong vcid");
        assert_eq!(
            wrong_vcid_error,
            RpcResponseError::VcidMismatch {
                expected: ENCRYPTED_RESPONSE_VCID,
                actual: PLAINTEXT_RESPONSE_VCID,
            }
        );
        assert!(is_response_noise(&wrong_vcid_error));
    }

    #[test]
    fn duplicate_response_members_are_rejected_before_correlation() {
        let request = RpcRequest::get_version(4).expect("request");
        let duplicate = br#"{"jsonrpc":"2.0","id":4,"id":4,"result":true}"#;
        assert_eq!(
            correlate_response(&request, ENCRYPTED_RESPONSE_VCID, duplicate)
                .err()
                .expect("duplicate member"),
            RpcResponseError::InvalidResponse {
                reason: "json_invalid",
            }
        );
    }

    #[test]
    fn correlated_device_error_is_not_confused_with_success() {
        let request = RpcRequest::get_date_time(9).expect("request");
        let response = correlate_response(
            &request,
            ENCRYPTED_RESPONSE_VCID,
            br#"{"jsonrpc":"2.0","id":9,"error":{"code":-1,"message":"unavailable"}}"#,
        )
        .expect("correlated error");
        assert!(response.result().is_none());
        let error = response.device_error().expect("device error");
        assert_eq!(error.code, -1);
        assert_eq!(error.message, "unavailable");
    }

    #[test]
    fn every_named_control_and_ota_method_is_blocked_at_the_gate() {
        let mut link = ReadOnlyRpcLink::new(RecordingTransport::new());
        for method in [
            "Set",
            "EnterTherapy",
            "EnterStandby",
            "ResetDevice",
            "EraseData",
            "ApplyUpgrade",
            "ApplyAuthenticatedUpgrade",
            "InitiateUpgrade",
            "UpgradeDataBlock",
            "CheckUpgradeFile",
            "DiscardPairKey",
            "StartKeyExchange",
            "ConfirmKeyExchange",
            "UnknownMethod",
        ] {
            let payload = format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\",\"id\":1}}");
            let result = link.send_wire_for_test(ENCRYPTED_REQUEST_VCID, payload.as_bytes());
            assert!(matches!(
                result,
                Err(RpcSendError::Rejected(RpcError::MethodNotAllowed { .. }))
            ));
        }
        assert_eq!(link.transport.writes, 0);
    }

    #[test]
    fn start_stream_shape_cannot_be_mutated_at_the_gate() {
        let mut link = ReadOnlyRpcLink::new(RecordingTransport::new());
        let altered = br#"{"jsonrpc":"2.0","method":"StartStream","id":1,"params":{"dataIds":["_RFL"],"sampleIntervalMs":40,"reportIntervalMs":200}}"#;
        assert!(
            link.send_wire_for_test(ENCRYPTED_REQUEST_VCID, altered)
                .is_err()
        );
        assert_eq!(link.transport.writes, 0);
    }
}
