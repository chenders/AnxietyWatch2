//! Linux `BlueZ` transport for the read-only `AirSense` 11 stream.
//!
//! This module is deliberately the only `AirSense` code that knows about `BlueZ`.
//! It connects to the configured address, resolves the exact FD56 service and
//! characteristics, subscribes to RX notifications, and sends only the
//! typed requests accepted by [`crate::as11_rpc::ReadOnlyRpcLink`].  There is
//! no discovery or pairing path here.

use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

use bluer::{
    Address, Session, Uuid,
    gatt::{
        WriteOp,
        remote::{Characteristic, CharacteristicWriteRequest},
    },
};
use futures::{StreamExt, stream::BoxStream};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::timeout,
};
use vitals_decode::{
    fig::{FIG_MAX_PAYLOAD, FIG_PREFIX_LENGTH, FIG_SYNC, FigReassembler, FigTake},
    stream::{StreamFrame, decode_stream},
};
use zeroize::Zeroize;

use crate::{
    as11_config::Air11Credentials,
    as11_crypto::{CryptoError, EnvelopeError, SessionCrypto},
    as11_rpc::{
        ENCRYPTED_REQUEST_VCID, ENCRYPTED_RESPONSE_VCID, PLAINTEXT_REQUEST_VCID, ReadOnlyRpcLink,
        RpcError, RpcRequest, RpcResponse, RpcResponseError, RpcSendError, RpcTransport,
    },
};

const SERVICE_UUID: &str = "0000fd56-0000-1000-8000-00805f9b34fb";
const TX_UUID: &str = "a6220002-35f1-4b20-afae-cb089d2044aa";
const RX_UUID: &str = "a6220003-35f1-4b20-afae-cb089d2044aa";
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const NOTIFICATION_TIMEOUT: Duration = Duration::from_secs(5);
const GATT_WRITE_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_QUEUE_CAPACITY: usize = 1;

/// A bounded `BlueZ` operation failure.  It contains operation names and limits,
/// never a target address, credential, session key, or device payload.
#[derive(Debug)]
pub enum BluezError {
    /// A `BlueZ` or D-Bus operation returned an error.
    Operation {
        /// The operation that failed.
        operation: &'static str,
    },
    /// A `BlueZ` operation did not complete before its deadline.
    Deadline {
        /// The operation that exceeded its deadline.
        operation: &'static str,
    },
    /// The configured adapter name was invalid or unavailable.
    AdapterUnavailable,
    /// The normalized target could not be converted to a `BlueZ` address.
    AddressInvalid,
    /// The required service was not resolved.
    ServiceMissing,
    /// A required characteristic was not resolved.
    CharacteristicMissing {
        /// The missing characteristic name.
        name: &'static str,
    },
    /// Two matching objects were resolved, which is ambiguous and unsafe.
    DuplicateCharacteristic {
        /// The ambiguous characteristic name.
        name: &'static str,
    },
    /// The TX characteristic does not support a write operation.
    TxNotWritable,
    /// The RX characteristic has no notification or indication capability.
    RxNotNotifiable,
    /// The negotiated MTU cannot carry an ATT value.
    MtuInvalid {
        /// The invalid negotiated MTU.
        mtu: usize,
    },
    /// The notification stream ended before the pending operation completed.
    NotificationEnded,
    /// A bounded FIG notification could not be retained or decoded.
    FigProtocol {
        /// The protocol invariant that failed.
        reason: &'static str,
    },
    /// A decrypted response was not a valid bounded `StreamData` object.
    StreamDataInvalid,
    /// The device returned a JSON-RPC error; only its code is retained.
    DeviceRpc {
        /// The device error code.
        code: i64,
    },
    /// The device response omitted a required authentication field.
    RpcResultInvalid {
        /// The missing result field.
        field: &'static str,
    },
    /// The typed request failed the final read-only gate.
    RpcRejected(RpcError),
    /// The final transport queue or GATT write failed.
    Transport(TransportError),
    /// The correlated response was malformed or arrived on the wrong lane.
    Response(RpcResponseError),
    /// Portable crypto rejected a configured or received value.
    Crypto(CryptoError),
    /// Portable envelope encryption/decryption failed.
    Envelope(EnvelopeError),
    /// The request-id counter reached its fixed integer ceiling.
    RequestIdExhausted,
}

impl core::fmt::Display for BluezError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Operation { operation } => write!(f, "bluez operation failed: {operation}"),
            Self::Deadline { operation } => write!(f, "bluez deadline expired: {operation}"),
            Self::AdapterUnavailable => f.write_str("bluez adapter unavailable"),
            Self::AddressInvalid => f.write_str("bluez target address invalid"),
            Self::ServiceMissing => f.write_str("air11 service fd56 not resolved"),
            Self::CharacteristicMissing { name } => {
                write!(f, "air11 characteristic missing: {name}")
            }
            Self::DuplicateCharacteristic { name } => {
                write!(f, "air11 characteristic ambiguous: {name}")
            }
            Self::TxNotWritable => f.write_str("air11 TX characteristic is not writable"),
            Self::RxNotNotifiable => f.write_str("air11 RX characteristic is not notifiable"),
            Self::MtuInvalid { mtu } => write!(f, "air11 negotiated MTU={mtu} is too small"),
            Self::NotificationEnded => f.write_str("air11 notification stream ended"),
            Self::FigProtocol { reason } => write!(f, "air11 FIG protocol error: {reason}"),
            Self::StreamDataInvalid => f.write_str("air11 StreamData payload invalid"),
            Self::DeviceRpc { code } => write!(f, "air11 device RPC error code={code}"),
            Self::RpcResultInvalid { field } => {
                write!(f, "air11 RPC result missing field={field}")
            }
            Self::RpcRejected(error) => write!(f, "air11 RPC rejected: {error}"),
            Self::Transport(error) => write!(f, "air11 transport: {error}"),
            Self::Response(error) => write!(f, "air11 response: {error}"),
            Self::Crypto(error) => write!(f, "air11 crypto: {error}"),
            Self::Envelope(error) => write!(f, "air11 envelope: {error}"),
            Self::RequestIdExhausted => f.write_str("air11 RPC request id exhausted"),
        }
    }
}

impl std::error::Error for BluezError {}

/// Failure of the final queued GATT writer.  Payloads are intentionally not
/// retained in any variant.
#[derive(Debug)]
pub enum TransportError {
    /// The crypto mutex was poisoned after a task failure.
    CryptoStatePoisoned,
    /// A request attempted to use an unsupported FIG lane.
    InvalidVcid {
        /// The rejected virtual circuit id.
        vcid: u16,
    },
    /// The FIG payload exceeded its fixed protocol capacity.
    FigPayloadTooLarge {
        /// Observed payload length.
        len: usize,
        /// Maximum accepted payload length.
        max: usize,
    },
    /// Another request is still waiting for its GATT write acknowledgement.
    WritePending,
    /// The bounded writer queue is full.
    WriteQueueFull,
    /// The writer task has no receiver.
    WriterStopped,
    /// The waiter state was poisoned.
    WaiterStatePoisoned,
    /// The caller attempted to await a write that was not reserved.
    WriteNotPending,
    /// The characteristic write failed.
    GattWriteFailed,
    /// The characteristic write exceeded its deadline.
    GattWriteDeadline,
    /// The MTU leaves no room for an ATT value.
    MtuTooSmall {
        /// The rejected MTU.
        mtu: usize,
    },
    /// The random-IV envelope could not be produced.
    Envelope(EnvelopeError),
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CryptoStatePoisoned => f.write_str("crypto state poisoned"),
            Self::InvalidVcid { vcid } => write!(f, "unsupported request VCID=0x{vcid:04x}"),
            Self::FigPayloadTooLarge { len, max } => {
                write!(f, "FIG payload length={len} exceeds limit={max}")
            }
            Self::WritePending => f.write_str("a GATT write is already pending"),
            Self::WriteQueueFull => f.write_str("GATT write queue is full"),
            Self::WriterStopped => f.write_str("GATT writer stopped"),
            Self::WaiterStatePoisoned => f.write_str("GATT waiter state poisoned"),
            Self::WriteNotPending => f.write_str("no GATT write acknowledgement is pending"),
            Self::GattWriteFailed => f.write_str("GATT write failed"),
            Self::GattWriteDeadline => f.write_str("GATT write deadline expired"),
            Self::MtuTooSmall { mtu } => write!(f, "negotiated MTU={mtu} is too small"),
            Self::Envelope(error) => write!(f, "envelope encryption failed: {error}"),
        }
    }
}

impl std::error::Error for TransportError {}

struct WriteItem {
    packet: Vec<u8>,
    acknowledgement: Option<oneshot::Sender<Result<(), TransportError>>>,
}

impl Drop for WriteItem {
    fn drop(&mut self) {
        self.packet.zeroize();
    }
}

type WriteReceiver = oneshot::Receiver<Result<(), TransportError>>;

#[derive(Clone)]
struct WriteWaiter {
    pending: Arc<Mutex<Option<WriteReceiver>>>,
}

impl WriteWaiter {
    fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(None)),
        }
    }

    fn reserve(&self) -> Result<oneshot::Sender<Result<(), TransportError>>, TransportError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| TransportError::WaiterStatePoisoned)?;
        if pending.is_some() {
            return Err(TransportError::WritePending);
        }
        let (sender, receiver) = oneshot::channel();
        *pending = Some(receiver);
        Ok(sender)
    }

    fn cancel(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.take();
        }
    }

    async fn wait(&self) -> Result<(), TransportError> {
        let receiver = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| TransportError::WaiterStatePoisoned)?;
            pending.take().ok_or(TransportError::WriteNotPending)?
        };
        receiver.await.map_err(|_| TransportError::WriterStopped)?
    }
}

struct BluezRpcTransport {
    queue: mpsc::Sender<WriteItem>,
    waiter: WriteWaiter,
    crypto: Arc<Mutex<SessionCrypto>>,
}

impl BluezRpcTransport {
    fn new(
        queue: mpsc::Sender<WriteItem>,
        crypto: Arc<Mutex<SessionCrypto>>,
    ) -> (Self, WriteWaiter) {
        let waiter = WriteWaiter::new();
        (
            Self {
                queue,
                waiter: waiter.clone(),
                crypto,
            },
            waiter,
        )
    }
}

impl RpcTransport for BluezRpcTransport {
    type Error = TransportError;

    fn write(&mut self, vcid: u16, payload: &[u8]) -> Result<(), Self::Error> {
        let wire_payload = match vcid {
            PLAINTEXT_REQUEST_VCID => payload.to_vec(),
            ENCRYPTED_REQUEST_VCID => {
                let crypto = self
                    .crypto
                    .lock()
                    .map_err(|_| TransportError::CryptoStatePoisoned)?;
                crypto.encrypt(payload).map_err(TransportError::Envelope)?
            }
            _ => return Err(TransportError::InvalidVcid { vcid }),
        };
        let packet = encode_fig(vcid, &wire_payload)?;
        let acknowledgement = self.waiter.reserve()?;
        let item = WriteItem {
            packet,
            acknowledgement: Some(acknowledgement),
        };
        match self.queue.try_send(item) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.waiter.cancel();
                Err(TransportError::WriteQueueFull)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.waiter.cancel();
                Err(TransportError::WriterStopped)
            }
        }
    }
}

/// A connected, authenticated-capable Linux `AirSense` BLE link.
///
/// Construction requires a validated external credential. Application-level
/// authentication supplies session security; this link has no generic BlueZ
/// pairing, scanning, or control-RPC method.
pub struct Air11BleTransport {
    device: bluer::Device,
    rpc: ReadOnlyRpcLink<BluezRpcTransport>,
    waiter: WriteWaiter,
    crypto: Arc<Mutex<SessionCrypto>>,
    notifications: BoxStream<'static, Vec<u8>>,
    fig: FigReassembler,
    writer_task: JoinHandle<()>,
    next_request_id: u64,
}

impl Air11BleTransport {
    /// Connects to the configured target and resolves the exact `AirSense`
    /// service and TX/RX characteristics.
    ///
    /// # Errors
    /// Returns a named bounded transport error when `BlueZ` cannot resolve the
    /// pipe.
    pub async fn connect(
        credentials: &Air11Credentials,
        adapter_name: &str,
    ) -> Result<Self, BluezError> {
        let address: Address = credentials
            .target()
            .parse()
            .map_err(|_| BluezError::AddressInvalid)?;
        let session = with_deadline("dbus_session", Session::new()).await?;
        let adapter = session
            .adapter(adapter_name)
            .map_err(|_| BluezError::AdapterUnavailable)?;
        let device = adapter
            .device(address)
            .map_err(|_| BluezError::AddressInvalid)?;
        let connected = with_deadline("connection_check", device.is_connected()).await?;
        if !connected {
            with_deadline("ble_connect", device.connect()).await?;
        }
        let (tx, rx, mtu, write_op) = resolve_pipe(&device).await?;
        let notification_stream = with_deadline("rx_subscribe", rx.notify()).await?;
        let crypto = Arc::new(Mutex::new(
            SessionCrypto::from_master_key_hex(credentials.master_pair_key())
                .map_err(BluezError::Crypto)?,
        ));
        let (queue, writes) = mpsc::channel(WRITE_QUEUE_CAPACITY);
        let writer_task = tokio::spawn(write_loop(writes, tx, mtu, write_op));
        let (transport, waiter) = BluezRpcTransport::new(queue, Arc::clone(&crypto));
        Ok(Self {
            device,
            rpc: ReadOnlyRpcLink::new(transport),
            waiter,
            crypto,
            notifications: Box::pin(notification_stream),
            fig: FigReassembler::new(),
            writer_task,
            next_request_id: 1,
        })
    }

    /// Completes the reconnect authentication exchange using the external
    /// client identifier.  Session-key derivation happens only after the
    /// device's integrity response has correlated successfully.
    ///
    /// # Errors
    /// Returns a named crypto, RPC, response, or bounded transport error.
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
    /// Returns a named error when the fixed request cannot be sent or the
    /// device returns a JSON-RPC error.
    pub async fn start_stream(&mut self) -> Result<(), BluezError> {
        let request = RpcRequest::start_stream(self.next_id()?).map_err(BluezError::RpcRejected)?;
        let response = self.send_request(&request).await?;
        ensure_success(&response)
    }

    /// Waits for one encrypted `StreamData`-lane envelope without decoding it.
    ///
    /// The caller must journal the returned bounded plaintext before passing it
    /// to [`decode_stream`]. Authentication and application responses are
    /// consumed by [`Self::authenticate`] and [`Self::start_stream`], so this
    /// method is only valid after the fixed stream has been started.
    ///
    /// # Errors
    /// Returns a bounded FIG, envelope, or notification error.
    pub async fn next_stream_envelope(&mut self) -> Result<Vec<u8>, BluezError> {
        let (vcid, payload) = self.next_fig().await?;
        if vcid != ENCRYPTED_RESPONSE_VCID {
            return Err(BluezError::FigProtocol {
                reason: "stream_vcid",
            });
        }
        self.decrypt(&payload)
    }

    /// Waits for and decodes one encrypted `StreamData` notification.
    ///
    /// # Errors
    /// Returns a bounded FIG, envelope, notification, or `StreamData` error.
    pub async fn next_stream_frame(&mut self) -> Result<StreamFrame, BluezError> {
        let plaintext = self.next_stream_envelope().await?;
        decode_stream(&plaintext).ok_or(BluezError::StreamDataInvalid)
    }

    /// Disconnects the BLE link without sending a device `StopStream` RPC.
    /// Dropping the notification subscription is the stream termination path.
    ///
    /// # Errors
    /// Returns a bounded `BlueZ` disconnect failure.
    pub async fn disconnect(&mut self) -> Result<(), BluezError> {
        self.writer_task.abort();
        with_deadline("ble_disconnect", self.device.disconnect()).await
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
        match self.rpc.send(request) {
            Ok(()) => {}
            Err(RpcSendError::Rejected(error)) => return Err(BluezError::RpcRejected(error)),
            Err(RpcSendError::Transport(error)) => return Err(BluezError::Transport(error)),
        }
        self.waiter.wait().await.map_err(BluezError::Transport)?;
        loop {
            let (vcid, payload) = self.next_fig().await?;
            let plaintext = if vcid == ENCRYPTED_RESPONSE_VCID {
                self.decrypt(&payload)?
            } else {
                payload
            };
            match crate::as11_rpc::correlate_response(request, vcid, &plaintext) {
                Ok(response) => return Ok(response),
                Err(RpcResponseError::Notification) => {}
                Err(error) => return Err(BluezError::Response(error)),
            }
        }
    }

    async fn next_fig(&mut self) -> Result<(u16, Vec<u8>), BluezError> {
        loop {
            let fragment = timeout(NOTIFICATION_TIMEOUT, self.notifications.next())
                .await
                .map_err(|_| BluezError::Deadline {
                    operation: "rx_notification",
                })?
                .ok_or(BluezError::NotificationEnded)?;
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

impl Drop for Air11BleTransport {
    fn drop(&mut self) {
        self.writer_task.abort();
    }
}

pub(crate) async fn resolve_pipe(
    device: &bluer::Device,
) -> Result<(Characteristic, Characteristic, usize, WriteOp), BluezError> {
    let service_uuid = Uuid::parse_str(SERVICE_UUID).map_err(|_| BluezError::ServiceMissing)?;
    let tx_uuid = Uuid::parse_str(TX_UUID)
        .map_err(|_| BluezError::CharacteristicMissing { name: "TX_UUID" })?;
    let rx_uuid = Uuid::parse_str(RX_UUID)
        .map_err(|_| BluezError::CharacteristicMissing { name: "RX_UUID" })?;
    let services = with_deadline("gatt_service_resolution", device.services()).await?;
    let mut service = None;
    for candidate in services {
        let uuid = with_deadline("gatt_service_uuid", candidate.uuid()).await?;
        if uuid == service_uuid {
            if service.is_some() {
                return Err(BluezError::DuplicateCharacteristic { name: "FD56" });
            }
            service = Some(candidate);
        }
    }
    let service = service.ok_or(BluezError::ServiceMissing)?;
    let characteristics =
        with_deadline("gatt_characteristic_resolution", service.characteristics()).await?;
    let mut tx = None;
    let mut rx = None;
    for candidate in characteristics {
        let uuid = with_deadline("gatt_characteristic_uuid", candidate.uuid()).await?;
        if uuid == tx_uuid {
            if tx.is_some() {
                return Err(BluezError::DuplicateCharacteristic { name: "TX" });
            }
            tx = Some(candidate);
        } else if uuid == rx_uuid {
            if rx.is_some() {
                return Err(BluezError::DuplicateCharacteristic { name: "RX" });
            }
            rx = Some(candidate);
        }
    }
    let tx = tx.ok_or(BluezError::CharacteristicMissing { name: "TX" })?;
    let rx = rx.ok_or(BluezError::CharacteristicMissing { name: "RX" })?;
    let tx_flags = with_deadline("tx_capabilities", tx.flags()).await?;
    let write_op = if tx_flags.write_without_response {
        WriteOp::Command
    } else if tx_flags.write {
        WriteOp::Request
    } else {
        return Err(BluezError::TxNotWritable);
    };
    let rx_flags = with_deadline("rx_capabilities", rx.flags()).await?;
    if !(rx_flags.notify || rx_flags.indicate) {
        return Err(BluezError::RxNotNotifiable);
    }
    let mtu = with_deadline("tx_mtu", tx.mtu()).await?;
    if mtu == 0 {
        return Err(BluezError::MtuInvalid { mtu });
    }
    Ok((tx, rx, mtu, write_op))
}

async fn with_deadline<T, F>(operation: &'static str, future: F) -> Result<T, BluezError>
where
    F: Future<Output = bluer::Result<T>>,
{
    match timeout(OPERATION_TIMEOUT, future).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_)) => Err(BluezError::Operation { operation }),
        Err(_) => Err(BluezError::Deadline { operation }),
    }
}

async fn write_loop(
    mut writes: mpsc::Receiver<WriteItem>,
    tx: Characteristic,
    mtu: usize,
    write_op: WriteOp,
) {
    while let Some(mut item) = writes.recv().await {
        let result = write_packet(&tx, &item.packet, mtu, write_op).await;
        let stop = result.is_err();
        if let Some(acknowledgement) = item.acknowledgement.take() {
            let _ = acknowledgement.send(result);
        }
        if stop {
            break;
        }
    }
}

pub(crate) async fn write_packet(
    tx: &Characteristic,
    packet: &[u8],
    mtu: usize,
    write_op: WriteOp,
) -> Result<(), TransportError> {
    // `Characteristic::mtu()` is already the maximum characteristic value
    // accepted by bluer (including its platform workaround); do not subtract
    // the ATT header a second time.
    let chunk_size = mtu;
    if chunk_size == 0 {
        return Err(TransportError::MtuTooSmall { mtu });
    }
    let write_request = CharacteristicWriteRequest {
        op_type: write_op,
        ..CharacteristicWriteRequest::default()
    };
    let result = timeout(GATT_WRITE_TIMEOUT, async {
        for chunk in packet.chunks(chunk_size) {
            tx.write_ext(chunk, &write_request).await.map_err(|_| ())?;
        }
        Ok::<(), ()>(())
    })
    .await;
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(())) => Err(TransportError::GattWriteFailed),
        Err(_) => Err(TransportError::GattWriteDeadline),
    }
}

pub(crate) fn encode_fig(vcid: u16, payload: &[u8]) -> Result<Vec<u8>, TransportError> {
    if payload.len() > FIG_MAX_PAYLOAD || payload.len() > usize::from(u16::MAX) {
        return Err(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        });
    }
    let payload_length =
        u16::try_from(payload.len()).map_err(|_| TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?;
    let payload_crc = crc32_ieee(payload);
    let mut header = [0_u8; 8];
    header
        .get_mut(..2)
        .ok_or(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?
        .copy_from_slice(&vcid.to_le_bytes());
    header
        .get_mut(2..4)
        .ok_or(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?
        .copy_from_slice(&payload_length.to_le_bytes());
    header
        .get_mut(4..8)
        .ok_or(TransportError::FigPayloadTooLarge {
            len: payload.len(),
            max: FIG_MAX_PAYLOAD,
        })?
        .copy_from_slice(&payload_crc.to_le_bytes());
    let header_crc = crc32_ieee(&header);
    let mut packet = Vec::with_capacity(FIG_PREFIX_LENGTH + payload.len());
    packet.extend_from_slice(&FIG_SYNC);
    packet.extend_from_slice(&header);
    packet.extend_from_slice(&header_crc.to_le_bytes());
    packet.extend_from_slice(payload);
    Ok(packet)
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn result_string(response: &RpcResponse, field: &'static str) -> Result<String, BluezError> {
    let result = response.result().ok_or_else(|| device_error(response))?;
    result
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or(BluezError::RpcResultInvalid { field })
}

fn ensure_success(response: &RpcResponse) -> Result<(), BluezError> {
    if response.result().is_some() {
        Ok(())
    } else {
        Err(device_error(response))
    }
}

fn ensure_confirmation(response: &RpcResponse) -> Result<(), BluezError> {
    let result = response.result().ok_or_else(|| device_error(response))?;
    if result
        .get("confirmation")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err(BluezError::RpcResultInvalid {
            field: "confirmation",
        })
    }
}

fn device_error(response: &RpcResponse) -> BluezError {
    response
        .device_error()
        .map_or(BluezError::RpcResultInvalid { field: "result" }, |error| {
            BluezError::DeviceRpc { code: error.code }
        })
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the FIG test uses fixed synthetic protocol bytes"
)]
mod tests {
    use super::*;
    use vitals_decode::fig::decode_fig;

    #[test]
    fn encoded_fig_round_trips_through_the_hostile_decoder() {
        let packet = encode_fig(ENCRYPTED_REQUEST_VCID, br#"{"id":1}"#).expect("FIG");
        let decoded = decode_fig(&packet).expect("decoded FIG");
        assert_eq!(decoded.vcid, ENCRYPTED_REQUEST_VCID);
        assert_eq!(decoded.payload, br#"{"id":1}"#);
    }

    #[test]
    fn oversized_fig_payload_is_rejected_before_allocation() {
        let payload = vec![0_u8; FIG_MAX_PAYLOAD + 1];
        assert!(matches!(
            encode_fig(PLAINTEXT_REQUEST_VCID, &payload),
            Err(TransportError::FigPayloadTooLarge { .. })
        ));
    }
}
