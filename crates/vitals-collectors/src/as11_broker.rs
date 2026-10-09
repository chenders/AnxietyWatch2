//! Broker-owned Linux transport for the read-only `AirSense` 11 stream.
//!
//! This module contains no `BlueZ` or D-Bus access. The broker owns connection,
//! discovery, GATT resolution, subscription, writes, and disconnect cleanup;
//! this worker owns only the `AirSense` application protocol and its crypto.

use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::Mutex as AsyncMutex;
use vitals_ble_broker::{
    BrokerClient, BrokerEvent, Capability, CapabilitySet, Intent, Priority, Source,
};
use vitals_decode::fig::{FIG_MAX_PAYLOAD, FIG_PREFIX_LENGTH, FigReassembler, FigTake};

use crate::{
    as11_common::{
        BluezError, TransportError, encode_fig, ensure_confirmation, ensure_event_subscription,
        ensure_success, result_string,
    },
    as11_config::Air11Credentials,
    as11_crypto::{CryptoError, EnvelopeError, SessionCrypto},
    as11_rpc::{
        AsyncRpcTransport, ENCRYPTED_REQUEST_VCID, ENCRYPTED_RESPONSE_VCID, PLAINTEXT_REQUEST_VCID,
        ReadOnlyRpcLink, RpcRequest, RpcResponse, RpcSendError,
    },
};

const SERVICE_UUID: &str = "0000fd56-0000-1000-8000-00805f9b34fb";
const TX_UUID: &str = "a6220002-35f1-4b20-afae-cb089d2044aa";
const RX_UUID: &str = "a6220003-35f1-4b20-afae-cb089d2044aa";
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const NOTIFICATION_TIMEOUT: Duration = Duration::from_secs(5);
const GATT_WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const LEASE_DEADLINE: u64 = 300;
const LEASE_HOLD: u64 = 180;

struct BrokerRpcTransport {
    broker: Arc<AsyncMutex<BrokerClient>>,
    tx_handle: String,
    crypto: Arc<Mutex<SessionCrypto>>,
}

impl AsyncRpcTransport for BrokerRpcTransport {
    type Error = TransportError;

    fn write<'a>(
        &'a mut self,
        vcid: u16,
        payload: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'a>> {
        let crypto = Arc::clone(&self.crypto);
        let broker = Arc::clone(&self.broker);
        let tx_handle = self.tx_handle.clone();
        Box::pin(async move {
            let wire_payload = match vcid {
                PLAINTEXT_REQUEST_VCID => payload.to_vec(),
                ENCRYPTED_REQUEST_VCID => {
                    let crypto = crypto
                        .lock()
                        .map_err(|_| TransportError::CryptoStatePoisoned)?;
                    crypto.encrypt(payload).map_err(TransportError::Envelope)?
                }
                _ => return Err(TransportError::InvalidVcid { vcid }),
            };
            let packet = encode_fig(vcid, &wire_payload)?;
            broker
                .lock()
                .await
                .write_bounded(&tx_handle, &packet, GATT_WRITE_TIMEOUT)
                .await
                .map_err(|error| TransportError::BrokerWriteFailed {
                    detail: error.to_string(),
                })
        })
    }
}

/// A connected, application-authentication-capable `AirSense` BLE link.
///
/// The link owns one broker lease. It never opens `BlueZ`, performs discovery,
/// or sends a control RPC outside the existing `AirSense` read-only allowlist.
pub struct Air11BleTransport {
    broker: Arc<AsyncMutex<BrokerClient>>,
    rpc: ReadOnlyRpcLink<BrokerRpcTransport>,
    crypto: Arc<Mutex<SessionCrypto>>,
    fig: FigReassembler,
    next_request_id: u64,
}

impl Air11BleTransport {
    /// Connects through the broker, resolving the exact `AirSense` pipe.
    ///
    /// # Errors
    /// Returns a named broker, protocol, crypto, or bounded transport error.
    pub async fn connect(
        credentials: &Air11Credentials,
        broker_socket: impl AsRef<Path>,
        adapter_selector: &str,
    ) -> Result<Self, BluezError> {
        let target = credentials.target().to_owned();
        let socket = broker_socket.as_ref();
        let mut broker = connect_broker(socket, adapter_selector, &target).await?;
        if let Err(error) = broker
            .connect_device_bounded(target.clone(), OPERATION_TIMEOUT)
            .await
        {
            if !error.is_device_missing() {
                return Err(BluezError::Broker(error));
            }
            broker
                .release("AirSense target missing before bounded discovery")
                .await
                .map_err(BluezError::Broker)?;
            discover_target(socket, adapter_selector, &target).await?;
            broker = connect_broker(socket, adapter_selector, &target).await?;
            broker
                .connect_device_bounded(target, OPERATION_TIMEOUT)
                .await
                .map_err(BluezError::Broker)?;
        }

        let tx_handle = broker
            .resolve_characteristic_bounded(SERVICE_UUID, TX_UUID, OPERATION_TIMEOUT)
            .await
            .map_err(BluezError::Broker)?;
        let rx_handle = broker
            .resolve_characteristic_bounded(SERVICE_UUID, RX_UUID, OPERATION_TIMEOUT)
            .await
            .map_err(BluezError::Broker)?;
        broker
            .subscribe_bounded(rx_handle, OPERATION_TIMEOUT)
            .await
            .map_err(BluezError::Broker)?;

        let crypto = Arc::new(Mutex::new(
            SessionCrypto::from_master_key_hex(credentials.master_pair_key())
                .map_err(BluezError::Crypto)?,
        ));
        let broker = Arc::new(AsyncMutex::new(broker));
        let transport = BrokerRpcTransport {
            broker: Arc::clone(&broker),
            tx_handle,
            crypto: Arc::clone(&crypto),
        };
        Ok(Self {
            broker,
            rpc: ReadOnlyRpcLink::new(transport),
            crypto,
            fig: FigReassembler::new(),
            next_request_id: 1,
        })
    }

    /// Completes the reconnect authentication exchange.
    ///
    /// # Errors
    /// Returns a named crypto, RPC, response, or bounded broker error.
    pub async fn authenticate(&mut self, client_id: &str) -> Result<(), BluezError> {
        let request = RpcRequest::request_session(self.next_id()?, client_id)
            .map_err(BluezError::RpcRejected)?;
        let response = self.send_request(&request).await?;
        let challenge = result_string(&response, "challenge")?;
        let nonce = result_string(&response, "nonce")?;
        let integrity = {
            let crypto = self
                .crypto
                .lock()
                .map_err(|_| BluezError::Crypto(CryptoError::MasterKeyMissing))?;
            crypto
                .integrity_response(&challenge)
                .map_err(BluezError::Crypto)?
        };
        let request = RpcRequest::check_session_integrity(self.next_id()?, &integrity)
            .map_err(BluezError::RpcRejected)?;
        let response = self.send_request(&request).await?;
        ensure_confirmation(&response)?;
        self.crypto
            .lock()
            .map_err(|_| BluezError::Crypto(CryptoError::MasterKeyMissing))?
            .derive_session_key(&nonce)
            .map_err(BluezError::Crypto)
    }

    /// Sends the fixed read-only stream subscription after authentication.
    ///
    /// # Errors
    /// Returns a named RPC or broker error.
    pub async fn start_stream(&mut self) -> Result<(), BluezError> {
        let request = RpcRequest::start_stream(self.next_id()?).map_err(BluezError::RpcRejected)?;
        let response = self.send_request(&request).await?;
        ensure_success(&response)
    }

    /// Sends the fixed read-only lifecycle and respiratory event subscription.
    ///
    /// # Errors
    /// Returns a named RPC or broker error.
    pub async fn subscribe_events(&mut self) -> Result<(), BluezError> {
        let request =
            RpcRequest::subscribe_events(self.next_id()?).map_err(BluezError::RpcRejected)?;
        let response = self.send_request(&request).await?;
        ensure_event_subscription(&response)
    }

    /// Waits for one encrypted stream envelope without decoding it.
    ///
    /// # Errors
    /// Returns a bounded FIG, envelope, notification, or broker error.
    pub async fn next_stream_envelope(&mut self) -> Result<Vec<u8>, BluezError> {
        let (vcid, payload) = self.next_fig().await?;
        if vcid != ENCRYPTED_RESPONSE_VCID {
            return Err(BluezError::FigProtocol {
                reason: "stream_vcid",
            });
        }
        self.decrypt(&payload)
    }

    /// Disconnects by releasing the broker lease.
    ///
    /// # Errors
    /// Returns a named broker release error.
    pub async fn disconnect(&mut self) -> Result<(), BluezError> {
        self.broker
            .lock()
            .await
            .release("AirSense transport disconnect")
            .await
            .map_err(BluezError::Broker)
    }

    fn next_id(&mut self) -> Result<u64, BluezError> {
        let id = self.next_request_id;
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or(BluezError::RequestIdExhausted)?;
        Ok(id)
    }

    async fn send_request(&mut self, request: &RpcRequest) -> Result<RpcResponse, BluezError> {
        match self.rpc.send_async(request).await {
            Ok(()) => {}
            Err(RpcSendError::Rejected(error)) => return Err(BluezError::RpcRejected(error)),
            Err(RpcSendError::Transport(error)) => return Err(BluezError::Transport(error)),
        }
        loop {
            let (vcid, payload) = self.next_fig().await?;
            let plaintext = if vcid == ENCRYPTED_RESPONSE_VCID {
                self.decrypt(&payload)?
            } else {
                payload
            };
            match crate::as11_rpc::correlate_response(request, vcid, &plaintext) {
                Ok(response) => return Ok(response),
                Err(error) if crate::as11_rpc::is_response_noise(&error) => {}
                Err(error) => return Err(BluezError::Response(error)),
            }
        }
    }

    async fn next_fig(&mut self) -> Result<(u16, Vec<u8>), BluezError> {
        loop {
            match self.fig.take() {
                FigTake::NeedMore => {}
                FigTake::Packet(packet) => return Ok((packet.vcid, packet.payload.to_vec())),
                FigTake::HeaderCrcMismatch => {
                    return Err(BluezError::FigProtocol {
                        reason: "header_crc",
                    });
                }
                FigTake::PayloadTooLarge => {
                    return Err(BluezError::FigProtocol {
                        reason: "payload_too_large",
                    });
                }
                FigTake::PayloadCrcMismatch => {
                    return Err(BluezError::FigProtocol {
                        reason: "payload_crc",
                    });
                }
                FigTake::BufferFull => {
                    return Err(BluezError::FigProtocol {
                        reason: "reassembly_buffer_full",
                    });
                }
            }

            let event =
                tokio::time::timeout(NOTIFICATION_TIMEOUT, self.broker.lock().await.next_event())
                    .await
                    .map_err(|_| BluezError::Deadline {
                        operation: "rx_notification",
                    })?
                    .map_err(BluezError::Broker)?;
            let Some(event) = event else {
                return Err(BluezError::NotificationEnded);
            };
            let fragment = match event {
                BrokerEvent::Frame { payload, .. } => payload,
                BrokerEvent::DeviceState { connected, .. } if !connected => {
                    return Err(BluezError::NotificationEnded);
                }
                BrokerEvent::Disconnected { .. } => return Err(BluezError::NotificationEnded),
                BrokerEvent::DeviceState { .. } => continue,
            };
            if fragment.len() > FIG_PREFIX_LENGTH + FIG_MAX_PAYLOAD {
                return Err(BluezError::FigProtocol {
                    reason: "notification_too_large",
                });
            }
            if !self.fig.feed(&fragment) {
                return Err(BluezError::FigProtocol {
                    reason: "reassembly_buffer_full",
                });
            }
        }
    }

    fn decrypt(&self, payload: &[u8]) -> Result<Vec<u8>, BluezError> {
        self.crypto
            .lock()
            .map_err(|_| BluezError::Envelope(EnvelopeError::SessionKeyMissing))?
            .decrypt(payload)
            .map_err(BluezError::Envelope)
    }
}

async fn connect_broker(
    socket: &Path,
    adapter_selector: &str,
    target: &str,
) -> Result<BrokerClient, BluezError> {
    let capabilities = CapabilitySet::empty()
        .with(Capability::Connect)
        .with(Capability::Resolve)
        .with(Capability::Subscribe)
        .with(Capability::Write);
    let mut broker = BrokerClient::connect_source(
        Source::AirSense,
        socket,
        adapter_selector,
        target,
        capabilities,
    )
    .await
    .map_err(BluezError::Broker)?;
    broker
        .acquire_lease(
            Intent::Connect,
            Priority::new(2),
            LEASE_DEADLINE,
            LEASE_HOLD,
            target,
            capabilities,
            "AirSense authenticated read-only stream",
        )
        .await
        .map_err(BluezError::Broker)?;
    Ok(broker)
}

async fn discover_target(
    socket: &Path,
    adapter_selector: &str,
    target: &str,
) -> Result<(), BluezError> {
    let capabilities = CapabilitySet::empty().with(Capability::Discover);
    let mut broker = BrokerClient::connect_source(
        Source::AirSense,
        socket,
        adapter_selector,
        target,
        capabilities,
    )
    .await
    .map_err(BluezError::Broker)?;
    broker
        .acquire_discovery(Priority::new(2), LEASE_DEADLINE, LEASE_HOLD, target)
        .await
        .map_err(BluezError::Broker)?;
    broker
        .discover_target_bounded(target, OPERATION_TIMEOUT)
        .await
        .map_err(BluezError::Broker)?;
    broker
        .release("AirSense bounded target discovery complete")
        .await
        .map_err(BluezError::Broker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitals_decode::fig::decode_fig;

    #[test]
    fn broker_transport_keeps_fig_encoder_compatible() -> Result<(), &'static str> {
        let packet = encode_fig(ENCRYPTED_REQUEST_VCID, br#"{"id":1}"#).map_err(|_| "FIG")?;
        let decoded = decode_fig(&packet).ok_or("decoded FIG")?;
        assert_eq!(decoded.vcid, ENCRYPTED_REQUEST_VCID);
        assert_eq!(decoded.payload, br#"{"id":1}"#);
        Ok(())
    }
}
