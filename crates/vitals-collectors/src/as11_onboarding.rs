//! Bounded, operator-authorized `AirSense` 11 discovery and first pairing.
//!
//! This module is separate from the production read-only collector. The
//! pairing surface contains only the two documented first-pair messages and
//! the discovery surface contains only the documented FD56 service filter.

use std::time::Duration;

#[cfg(any(target_os = "linux", test))]
use serde_json::{Value, json};
#[cfg(any(target_os = "linux", test))]
use zeroize::{Zeroize, Zeroizing};

use crate::{
    as11_config::{CredentialWriteError, CredentialsError},
    as11_crypto::CryptoError,
};
#[cfg(any(target_os = "linux", test))]
use crate::{
    as11_crypto::{MAX_JSON_RESPONSE_BYTES, MAX_SALT_BYTES, SRP_PAD_BYTES, encode_hex},
    as11_rpc::{MAX_CLIENT_ID_BYTES, RpcError},
};

/// The `AirSense` 11 service used for application pairing and streaming.
pub const AIR11_SERVICE_UUID: &str = "0000fd56-0000-1000-8000-00805f9b34fb";
/// The shortest accepted discovery duration in seconds.
pub const MIN_DISCOVERY_SECONDS: u64 = 5;
/// The longest accepted discovery duration in seconds.
pub const MAX_DISCOVERY_SECONDS: u64 = 120;
/// The default discovery duration in seconds.
pub const DEFAULT_DISCOVERY_SECONDS: u64 = 30;
/// The maximum time allowed for each pairing GATT operation.
pub const PAIRING_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
/// The maximum time allowed for a pairing response notification.
pub const PAIRING_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

/// An onboarding failure that is safe to show to an operator.
#[derive(Debug)]
pub enum OnboardingError {
    /// The supplied target was not a valid six-octet address.
    InvalidAddress,
    /// The discovery duration was outside the fixed bound.
    InvalidDuration {
        /// The requested duration.
        seconds: u64,
        /// The shortest accepted duration.
        min: u64,
        /// The longest accepted duration.
        max: u64,
    },
    /// The adapter has no usable power state.
    AdapterNotPowered,
    /// Another discovery operation already owns the adapter.
    DiscoveryAlreadyActive,
    /// A `BlueZ` operation failed.
    #[cfg(target_os = "linux")]
    Bluez {
        /// The bounded `BlueZ` operation that failed.
        operation: &'static str,
    },
    /// A `BlueZ` operation exceeded its bounded deadline.
    #[cfg(target_os = "linux")]
    BluezDeadline {
        /// The operation that exceeded its deadline.
        operation: &'static str,
    },
    /// Discovery found no eligible device or an ambiguous number of devices.
    CandidateCount {
        /// The number of eligible candidates.
        count: usize,
    },
    /// An explicitly selected address was not among the eligible candidates.
    SelectedCandidateMissing {
        /// The number of eligible candidates.
        count: usize,
    },
    /// Discovery or pairing cleanup could not be confirmed.
    CleanupFailed {
        /// The cleanup operation that failed.
        operation: &'static str,
    },
    /// A pairing request or response violated a fixed protocol shape.
    PairingProtocol {
        /// The failed protocol invariant.
        reason: &'static str,
    },
    /// The device returned a JSON-RPC error during first pairing.
    PairingDeviceError {
        /// The device error code.
        code: i64,
    },
    /// The pairing transport failed before a response was received.
    PairingTransport,
    /// The pairing response did not arrive before its deadline.
    PairingResponseTimeout,
    /// The portable pairing cryptography rejected an input.
    Crypto(CryptoError),
    /// A typed pairing request could not be constructed.
    #[cfg(any(target_os = "linux", test))]
    Rpc(RpcError),
    /// Existing credentials could not be validated.
    Credentials(CredentialsError),
    /// A private credential-file update failed.
    CredentialWrite(CredentialWriteError),
}

impl core::fmt::Display for OnboardingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidAddress => f.write_str("onboarding target address invalid"),
            Self::InvalidDuration { seconds, min, max } => write!(
                f,
                "onboarding duration={seconds} outside limit={min}..{max} seconds"
            ),
            Self::AdapterNotPowered => f.write_str("onboarding adapter powered=false"),
            Self::DiscoveryAlreadyActive => {
                f.write_str("onboarding discovery blocked: adapter discovery already active")
            }
            #[cfg(target_os = "linux")]
            Self::Bluez { operation } => {
                write!(f, "onboarding BlueZ operation failed: {operation}")
            }
            #[cfg(target_os = "linux")]
            Self::BluezDeadline { operation } => {
                write!(f, "onboarding BlueZ deadline expired: {operation}")
            }
            Self::CandidateCount { count } => {
                write!(
                    f,
                    "onboarding candidate_count={count}; explicit target required"
                )
            }
            Self::SelectedCandidateMissing { count } => write!(
                f,
                "onboarding selected target was not discovered; candidate_count={count}"
            ),
            Self::CleanupFailed { operation } => {
                write!(f, "onboarding cleanup failed: operation={operation}")
            }
            Self::PairingProtocol { reason } => {
                write!(f, "onboarding pairing protocol rejected: {reason}")
            }
            Self::PairingDeviceError { code } => {
                write!(f, "onboarding pairing device error code={code}")
            }
            Self::PairingTransport => f.write_str("onboarding pairing GATT write failed"),
            Self::PairingResponseTimeout => {
                f.write_str("onboarding pairing response deadline expired")
            }
            Self::Crypto(error) => write!(f, "onboarding pairing crypto: {error}"),
            #[cfg(any(target_os = "linux", test))]
            Self::Rpc(error) => write!(f, "onboarding pairing request: {error}"),
            Self::Credentials(error) => write!(f, "onboarding credentials: {error}"),
            Self::CredentialWrite(error) => write!(f, "onboarding credential update: {error}"),
        }
    }
}

impl std::error::Error for OnboardingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Crypto(error) => Some(error),
            #[cfg(any(target_os = "linux", test))]
            Self::Rpc(error) => Some(error),
            Self::Credentials(error) => Some(error),
            Self::CredentialWrite(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CryptoError> for OnboardingError {
    fn from(error: CryptoError) -> Self {
        Self::Crypto(error)
    }
}

#[cfg(any(target_os = "linux", test))]
impl From<RpcError> for OnboardingError {
    fn from(error: RpcError) -> Self {
        Self::Rpc(error)
    }
}

impl From<CredentialsError> for OnboardingError {
    fn from(error: CredentialsError) -> Self {
        Self::Credentials(error)
    }
}

impl From<CredentialWriteError> for OnboardingError {
    fn from(error: CredentialWriteError) -> Self {
        Self::CredentialWrite(error)
    }
}

#[cfg(target_os = "linux")]
const PAIRING_REQUEST_VCID: u16 = 0x0393;
#[cfg(any(target_os = "linux", test))]
const PAIRING_RESPONSE_VCID: u16 = 0x0392;

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PairingMethod {
    StartKeyExchange,
    ConfirmKeyExchange,
}

#[cfg(any(target_os = "linux", test))]
impl PairingMethod {
    const fn as_str(self) -> &'static str {
        match self {
            Self::StartKeyExchange => "StartKeyExchange",
            Self::ConfirmKeyExchange => "ConfirmKeyExchange",
        }
    }
}

#[cfg(any(target_os = "linux", test))]
/// A typed first-pair request. There is no generic onboarding RPC request.
pub struct PairingRequest {
    id: u64,
    method: PairingMethod,
    payload: Zeroizing<Vec<u8>>,
}

#[cfg(any(target_os = "linux", test))]
impl PairingRequest {
    /// Builds the documented `StartKeyExchange` request.
    ///
    /// # Errors
    /// Returns an error when the public key is not the fixed-width hexadecimal
    /// SRP value or the bounded JSON body cannot be constructed.
    pub fn start_key_exchange(id: u64, public_key_hex: &str) -> Result<Self, OnboardingError> {
        if public_key_hex.len() != SRP_PAD_BYTES * 2
            || !public_key_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(OnboardingError::PairingProtocol {
                reason: "client_public_key_shape",
            });
        }
        let params = json!({"clientPk": public_key_hex});
        Self::build(id, PairingMethod::StartKeyExchange, &params)
    }

    /// Builds the documented `ConfirmKeyExchange` request.
    ///
    /// # Errors
    /// Returns an error when the bounded JSON body cannot be constructed.
    pub fn confirm_key_exchange(id: u64, client_proof: &[u8; 32]) -> Result<Self, OnboardingError> {
        let params = json!({"clientConfirmation": encode_hex(client_proof)});
        Self::build(id, PairingMethod::ConfirmKeyExchange, &params)
    }

    fn build(id: u64, method: PairingMethod, params: &Value) -> Result<Self, OnboardingError> {
        let value = json!({
            "jsonrpc": "2.0",
            "method": method.as_str(),
            "id": id,
            "params": params,
        });
        let payload = serde_json::to_vec(&value).map_err(|_| OnboardingError::PairingProtocol {
            reason: "request_serialization",
        })?;
        if payload.len() > crate::as11_crypto::MAX_JSON_REQUEST_BYTES {
            return Err(OnboardingError::PairingProtocol {
                reason: "request_size",
            });
        }
        Ok(Self {
            id,
            method,
            payload: Zeroizing::new(payload),
        })
    }

    const fn id(&self) -> u64 {
        self.id
    }

    const fn method(&self) -> PairingMethod {
        self.method
    }

    pub(crate) fn payload(&self) -> &[u8] {
        &self.payload
    }
}

#[cfg(any(target_os = "linux", test))]
/// The bounded response to `StartKeyExchange`.
pub struct PairingStart {
    server_public_key: String,
    salt: String,
}

#[cfg(any(target_os = "linux", test))]
impl PairingStart {
    /// Returns the fixed-width server public key for the SRP calculation.
    #[must_use]
    pub fn server_public_key(&self) -> &str {
        &self.server_public_key
    }

    /// Returns the bounded SRP salt.
    #[must_use]
    pub fn salt(&self) -> &str {
        &self.salt
    }
}

#[cfg(any(target_os = "linux", test))]
impl Drop for PairingStart {
    fn drop(&mut self) {
        self.server_public_key.zeroize();
        self.salt.zeroize();
    }
}

#[cfg(any(target_os = "linux", test))]
/// The bounded response to `ConfirmKeyExchange`.
pub struct PairingConfirmation {
    client_id: String,
    server_confirmation: String,
    nonce: String,
}

#[cfg(any(target_os = "linux", test))]
impl PairingConfirmation {
    /// Returns the device-assigned reusable client identifier.
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Returns the device's SRP server proof.
    #[must_use]
    pub fn server_confirmation(&self) -> &str {
        &self.server_confirmation
    }

    /// Returns the bounded one-session nonce.
    #[must_use]
    pub fn nonce(&self) -> &str {
        &self.nonce
    }
}

#[cfg(any(target_os = "linux", test))]
impl Drop for PairingConfirmation {
    fn drop(&mut self) {
        self.client_id.zeroize();
        self.server_confirmation.zeroize();
        self.nonce.zeroize();
    }
}

#[cfg(any(target_os = "linux", test))]
/// A response from one of the two typed first-pair methods.
pub enum PairingResponse {
    /// Response to `StartKeyExchange`.
    Start(PairingStart),
    /// Response to `ConfirmKeyExchange`.
    Confirmation(PairingConfirmation),
}

#[cfg(any(target_os = "linux", test))]
fn parse_pairing_response(
    request: &PairingRequest,
    response_vcid: u16,
    payload: &[u8],
) -> Result<PairingResponse, OnboardingError> {
    if response_vcid != PAIRING_RESPONSE_VCID {
        return Err(OnboardingError::PairingProtocol {
            reason: "response_vcid",
        });
    }
    if payload.len() > MAX_JSON_RESPONSE_BYTES {
        return Err(OnboardingError::PairingProtocol {
            reason: "response_size",
        });
    }
    let value: Value =
        serde_json::from_slice(payload).map_err(|_| OnboardingError::PairingProtocol {
            reason: "response_json",
        })?;
    let object = value.as_object().ok_or(OnboardingError::PairingProtocol {
        reason: "response_object",
    })?;
    if object.get("method").is_some() {
        return Err(OnboardingError::PairingProtocol {
            reason: "response_notification",
        });
    }
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(OnboardingError::PairingProtocol {
            reason: "response_version",
        });
    }
    if object.get("id").and_then(Value::as_u64) != Some(request.id()) {
        return Err(OnboardingError::PairingProtocol {
            reason: "response_id",
        });
    }
    match (object.get("result"), object.get("error")) {
        (Some(_), Some(_)) => Err(OnboardingError::PairingProtocol {
            reason: "result_and_error",
        }),
        (None, Some(error)) => {
            let code = error
                .as_object()
                .and_then(|error| error.get("code"))
                .and_then(Value::as_i64)
                .ok_or(OnboardingError::PairingProtocol {
                    reason: "device_error_code",
                })?;
            Err(OnboardingError::PairingDeviceError { code })
        }
        (Some(result), None) => parse_pairing_result(request.method(), result),
        (None, None) => Err(OnboardingError::PairingProtocol {
            reason: "result_or_error",
        }),
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_pairing_result(
    method: PairingMethod,
    result: &Value,
) -> Result<PairingResponse, OnboardingError> {
    let result = result.as_object().ok_or(OnboardingError::PairingProtocol {
        reason: "result_object",
    })?;
    match method {
        PairingMethod::StartKeyExchange => {
            let server_public_key = required_string(result, "serverPk")?;
            let salt = required_string(result, "salt")?;
            if server_public_key.len() != SRP_PAD_BYTES * 2
                || !server_public_key
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(OnboardingError::PairingProtocol {
                    reason: "server_public_key_shape",
                });
            }
            if salt.is_empty()
                || salt.len() > MAX_SALT_BYTES * 2
                || !salt.len().is_multiple_of(2)
                || !salt.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(OnboardingError::PairingProtocol {
                    reason: "salt_shape",
                });
            }
            Ok(PairingResponse::Start(PairingStart {
                server_public_key,
                salt,
            }))
        }
        PairingMethod::ConfirmKeyExchange => {
            let client_id = required_string(result, "clientId")?;
            let server_confirmation = required_string(result, "serverConfirmation")?;
            let nonce = required_string(result, "nonce")?;
            if client_id.is_empty() || client_id.len() > MAX_CLIENT_ID_BYTES {
                return Err(OnboardingError::PairingProtocol {
                    reason: "client_id_shape",
                });
            }
            if server_confirmation.len() != 64
                || !server_confirmation
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(OnboardingError::PairingProtocol {
                    reason: "server_confirmation_shape",
                });
            }
            if nonce.is_empty()
                || nonce.len() > crate::as11_crypto::MAX_NONCE_BYTES * 2
                || !nonce.len().is_multiple_of(2)
                || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(OnboardingError::PairingProtocol {
                    reason: "nonce_shape",
                });
            }
            Ok(PairingResponse::Confirmation(PairingConfirmation {
                client_id,
                server_confirmation,
                nonce,
            }))
        }
    }
}

#[cfg(any(target_os = "linux", test))]
fn required_string(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<String, OnboardingError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or(OnboardingError::PairingProtocol { reason: field })
}

#[cfg(target_os = "linux")]
mod linux {
    use std::{collections::HashSet, future::Future};

    use bluer::{
        AdapterEvent, Address, DiscoveryFilter, DiscoveryTransport, Session, Uuid,
        gatt::{WriteOp, remote::Characteristic},
    };
    use futures::{StreamExt, stream::BoxStream};
    use tokio::time::timeout;
    use vitals_decode::fig::{FIG_MAX_PAYLOAD, FIG_PREFIX_LENGTH, FigReassembler, FigTake};

    use super::{
        AIR11_SERVICE_UUID, OnboardingError, PAIRING_OPERATION_TIMEOUT, PAIRING_REQUEST_VCID,
        PAIRING_RESPONSE_TIMEOUT, PairingConfirmation, PairingRequest, PairingResponse,
        PairingStart, parse_pairing_response,
    };
    use crate::as11_bluez::{self, BluezError};
    use crate::as11_config::normalize_linux_target;

    const DEVICE_NAME_PREFIX: &str = "ResMed";

    /// Discovers one `AirSense` target using a finite FD56-filtered scan.
    ///
    /// An optional address is an explicit operator selection used only to
    /// disambiguate multiple eligible devices.
    ///
    /// # Errors
    /// Returns a bounded adapter, discovery, candidate, or cleanup error.
    pub async fn discover_target(
        adapter_name: &str,
        duration: std::time::Duration,
        selected: Option<&str>,
    ) -> Result<String, OnboardingError> {
        let seconds = duration.as_secs();
        if !(super::MIN_DISCOVERY_SECONDS..=super::MAX_DISCOVERY_SECONDS).contains(&seconds) {
            return Err(OnboardingError::InvalidDuration {
                seconds,
                min: super::MIN_DISCOVERY_SECONDS,
                max: super::MAX_DISCOVERY_SECONDS,
            });
        }
        let selected = selected
            .map(|value| {
                value
                    .parse::<Address>()
                    .map_err(|_| OnboardingError::InvalidAddress)
            })
            .transpose()?;
        let session = bounded("dbus_session", Session::new()).await?;
        let adapter = session
            .adapter(adapter_name)
            .map_err(|_| OnboardingError::Bluez {
                operation: "adapter_lookup",
            })?;
        if !bounded("adapter_powered", adapter.is_powered()).await? {
            return Err(OnboardingError::AdapterNotPowered);
        }
        if bounded("discovery_state", adapter.is_discovering()).await? {
            return Err(OnboardingError::DiscoveryAlreadyActive);
        }
        let previous_filter = adapter.discovery_filter().await;
        let service_uuid =
            Uuid::parse_str(AIR11_SERVICE_UUID).map_err(|_| OnboardingError::Bluez {
                operation: "service_uuid",
            })?;
        let mut uuids = HashSet::new();
        uuids.insert(service_uuid);
        adapter
            .set_discovery_filter(DiscoveryFilter {
                uuids,
                transport: DiscoveryTransport::Le,
                duplicate_data: true,
                ..DiscoveryFilter::default()
            })
            .await
            .map_err(|_| OnboardingError::Bluez {
                operation: "set_discovery_filter",
            })?;

        let discovery_result = discover_candidates(&adapter, duration, service_uuid).await;
        let restore_result = adapter.set_discovery_filter(previous_filter).await;
        if restore_result.is_err() {
            return Err(OnboardingError::CleanupFailed {
                operation: "restore_discovery_filter",
            });
        }
        let candidates = discovery_result?;
        if let Some(selected) = selected {
            if !candidates.contains(&selected) {
                return Err(OnboardingError::SelectedCandidateMissing {
                    count: candidates.len(),
                });
            }
            return normalize_linux_target(&selected.to_string(), None)
                .map_err(|_| OnboardingError::InvalidAddress);
        }
        if candidates.len() != 1 {
            return Err(OnboardingError::CandidateCount {
                count: candidates.len(),
            });
        }
        let address = candidates
            .into_iter()
            .next()
            .ok_or(OnboardingError::CandidateCount { count: 0 })?;
        normalize_linux_target(&address.to_string(), None)
            .map_err(|_| OnboardingError::InvalidAddress)
    }

    async fn discover_candidates(
        adapter: &bluer::Adapter,
        duration: std::time::Duration,
        service_uuid: Uuid,
    ) -> Result<HashSet<Address>, OnboardingError> {
        let mut events = adapter
            .discover_devices()
            .await
            .map_err(|_| OnboardingError::Bluez {
                operation: "start_discovery",
            })?;
        let result = timeout(duration, async {
            let mut candidates = HashSet::new();
            while let Some(event) = events.next().await {
                match event {
                    AdapterEvent::DeviceAdded(address) => {
                        if is_air11_candidate(adapter, address, service_uuid).await {
                            candidates.insert(address);
                        }
                    }
                    AdapterEvent::DeviceRemoved(address) => {
                        candidates.remove(&address);
                    }
                    AdapterEvent::PropertyChanged(_) => {}
                }
            }
            candidates
        })
        .await;
        match result {
            Ok(candidates) => Ok(candidates),
            Err(_) => Err(OnboardingError::BluezDeadline {
                operation: "discovery",
            }),
        }
    }

    async fn is_air11_candidate(
        adapter: &bluer::Adapter,
        address: Address,
        service_uuid: Uuid,
    ) -> bool {
        let Ok(device) = adapter.device(address) else {
            return false;
        };
        let Ok(Some(uuids)) = device.uuids().await else {
            return false;
        };
        if !uuids.contains(&service_uuid) {
            return false;
        }
        let Ok(Some(_rssi)) = device.rssi().await else {
            return false;
        };
        match device.name().await {
            Ok(Some(name)) if !name.is_empty() => name.starts_with(DEVICE_NAME_PREFIX),
            Ok(_) | Err(_) => true,
        }
    }

    async fn bounded<T, F>(operation: &'static str, future: F) -> Result<T, OnboardingError>
    where
        F: Future<Output = bluer::Result<T>>,
    {
        match timeout(PAIRING_OPERATION_TIMEOUT, future).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(_)) => Err(OnboardingError::Bluez { operation }),
            Err(_) => Err(OnboardingError::BluezDeadline { operation }),
        }
    }

    /// A connected GATT link restricted to the two first-pair methods.
    pub struct Air11PairingLink {
        device: bluer::Device,
        tx: Characteristic,
        mtu: usize,
        write_op: WriteOp,
        notifications: BoxStream<'static, Vec<u8>>,
        fig: FigReassembler,
        next_request_id: u64,
    }

    impl Air11PairingLink {
        /// Connects to a discovered target without requiring saved application
        /// credentials or invoking a generic `BlueZ` pairing method.
        ///
        /// # Errors
        /// Returns a bounded BlueZ/GATT error when the FD56 pipe is unavailable.
        pub async fn connect(adapter_name: &str, target: &str) -> Result<Self, OnboardingError> {
            let address = target
                .parse::<Address>()
                .map_err(|_| OnboardingError::InvalidAddress)?;
            let session = bounded("dbus_session", Session::new()).await?;
            let adapter = session
                .adapter(adapter_name)
                .map_err(|_| OnboardingError::Bluez {
                    operation: "adapter_lookup",
                })?;
            let device = adapter
                .device(address)
                .map_err(|_| OnboardingError::Bluez {
                    operation: "device_lookup",
                })?;
            if !bounded("connection_state", device.is_connected()).await? {
                bounded("gatt_connect", device.connect()).await?;
            }
            let (tx, rx, mtu, write_op) =
                as11_bluez::resolve_pipe(&device)
                    .await
                    .map_err(|_| OnboardingError::Bluez {
                        operation: "gatt_service_resolution",
                    })?;
            let notifications = bounded("rx_subscribe", rx.notify()).await?;
            Ok(Self {
                device,
                tx,
                mtu,
                write_op,
                notifications: Box::pin(notifications),
                fig: FigReassembler::new(),
                next_request_id: 1,
            })
        }

        /// Sends only `StartKeyExchange` and returns its bounded SRP inputs.
        ///
        /// # Errors
        /// Returns a named protocol, transport, or response error when the
        /// request cannot be sent or the response is not valid.
        pub async fn start_key_exchange(
            &mut self,
            public_key_hex: &str,
        ) -> Result<PairingStart, OnboardingError> {
            let request = PairingRequest::start_key_exchange(self.next_id()?, public_key_hex)?;
            match self.send_request(&request).await? {
                PairingResponse::Start(response) => Ok(response),
                PairingResponse::Confirmation(_) => Err(OnboardingError::PairingProtocol {
                    reason: "unexpected_start_response",
                }),
            }
        }

        /// Sends only `ConfirmKeyExchange` and returns its bounded result.
        ///
        /// # Errors
        /// Returns a named protocol, transport, or response error when the
        /// request cannot be sent or the response is not valid.
        pub async fn confirm_key_exchange(
            &mut self,
            client_proof: &[u8; 32],
        ) -> Result<PairingConfirmation, OnboardingError> {
            let request = PairingRequest::confirm_key_exchange(self.next_id()?, client_proof)?;
            match self.send_request(&request).await? {
                PairingResponse::Confirmation(response) => Ok(response),
                PairingResponse::Start(_) => Err(OnboardingError::PairingProtocol {
                    reason: "unexpected_confirmation_response",
                }),
            }
        }

        /// Disconnects the temporary pairing link.
        ///
        /// # Errors
        /// Returns a named `BlueZ` error when the temporary link cannot be
        /// disconnected before its deadline.
        pub async fn disconnect(&mut self) -> Result<(), OnboardingError> {
            let notifications =
                std::mem::replace(&mut self.notifications, Box::pin(futures::stream::empty()));
            drop(notifications);
            bounded("gatt_disconnect", self.device.disconnect()).await
        }

        fn next_id(&mut self) -> Result<u64, OnboardingError> {
            let id = self.next_request_id;
            self.next_request_id =
                self.next_request_id
                    .checked_add(1)
                    .ok_or(OnboardingError::PairingProtocol {
                        reason: "request_id_exhausted",
                    })?;
            Ok(id)
        }

        async fn send_request(
            &mut self,
            request: &PairingRequest,
        ) -> Result<PairingResponse, OnboardingError> {
            let packet = as11_bluez::encode_fig(PAIRING_REQUEST_VCID, request.payload())
                .map_err(|_| OnboardingError::PairingTransport)?;
            match timeout(
                PAIRING_OPERATION_TIMEOUT,
                as11_bluez::write_packet(&self.tx, &packet, self.mtu, self.write_op),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(_)) => return Err(OnboardingError::PairingTransport),
                Err(_) => {
                    return Err(OnboardingError::BluezDeadline {
                        operation: "pairing_write",
                    });
                }
            }
            self.next_response(request).await
        }

        async fn next_response(
            &mut self,
            request: &PairingRequest,
        ) -> Result<PairingResponse, OnboardingError> {
            loop {
                let fragment = timeout(PAIRING_RESPONSE_TIMEOUT, self.notifications.next())
                    .await
                    .map_err(|_| OnboardingError::PairingResponseTimeout)?
                    .ok_or(OnboardingError::PairingProtocol {
                        reason: "notification_stream_ended",
                    })?;
                if fragment.len() > FIG_PREFIX_LENGTH + FIG_MAX_PAYLOAD {
                    return Err(OnboardingError::PairingProtocol {
                        reason: "notification_size",
                    });
                }
                if !self.fig.feed(&fragment) {
                    return Err(OnboardingError::PairingProtocol {
                        reason: "reassembly_buffer",
                    });
                }
                match self.fig.take() {
                    FigTake::NeedMore => {}
                    FigTake::Packet(packet) => {
                        return parse_pairing_response(request, packet.vcid, packet.payload);
                    }
                    FigTake::HeaderCrcMismatch => {
                        return Err(OnboardingError::PairingProtocol {
                            reason: "header_crc",
                        });
                    }
                    FigTake::PayloadTooLarge => {
                        return Err(OnboardingError::PairingProtocol {
                            reason: "payload_size",
                        });
                    }
                    FigTake::PayloadCrcMismatch => {
                        return Err(OnboardingError::PairingProtocol {
                            reason: "payload_crc",
                        });
                    }
                    FigTake::BufferFull => {
                        return Err(OnboardingError::PairingProtocol {
                            reason: "reassembly_buffer_full",
                        });
                    }
                }
            }
        }
    }

    impl Drop for Air11PairingLink {
        fn drop(&mut self) {
            // The caller performs the bounded async disconnect. Dropping the
            // notification stream still releases its BlueZ subscription.
        }
    }

    impl From<BluezError> for OnboardingError {
        fn from(_: BluezError) -> Self {
            Self::Bluez {
                operation: "gatt_operation",
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux::{Air11PairingLink, discover_target};

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "synthetic pairing protocol fixtures are assertion inputs"
)]
mod tests {
    use super::*;

    #[test]
    fn pairing_requests_are_fixed_to_two_methods() {
        let public_key = "AB".repeat(SRP_PAD_BYTES);
        let start = PairingRequest::start_key_exchange(1, &public_key).expect("start");
        let start_json: Value = serde_json::from_slice(start.payload()).expect("start JSON");
        assert_eq!(start_json["method"], "StartKeyExchange");
        assert_eq!(start_json["params"]["clientPk"], public_key);

        let proof = [0x11_u8; 32];
        let confirm = PairingRequest::confirm_key_exchange(2, &proof).expect("confirm");
        let confirm_json: Value = serde_json::from_slice(confirm.payload()).expect("confirm JSON");
        assert_eq!(confirm_json["method"], "ConfirmKeyExchange");
        assert_eq!(
            confirm_json["params"]["clientConfirmation"],
            "11".repeat(32)
        );
    }

    #[test]
    fn malformed_pairing_response_is_rejected_without_secret_details() {
        let public_key = "AB".repeat(SRP_PAD_BYTES);
        let request = PairingRequest::start_key_exchange(7, &public_key).expect("request");
        let error = parse_pairing_response(&request, PAIRING_RESPONSE_VCID, br"{}")
            .err()
            .expect("empty response");
        assert!(matches!(error, OnboardingError::PairingProtocol { .. }));
    }

    #[test]
    fn pairing_response_requires_fixed_start_fields() {
        let public_key = "AB".repeat(SRP_PAD_BYTES);
        let request = PairingRequest::start_key_exchange(7, &public_key).expect("request");
        let response = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "result": {
                "serverPk": public_key,
                "salt": "00112233445566778899AABBCCDDEEFF"
            }
        });
        let response = parse_pairing_response(
            &request,
            PAIRING_RESPONSE_VCID,
            serde_json::to_string(&response)
                .expect("response")
                .as_bytes(),
        )
        .expect("response");
        assert!(matches!(response, PairingResponse::Start(_)));
    }
}
