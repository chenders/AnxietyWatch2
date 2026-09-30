//! Versioned, bounded worker protocol and deterministic fake transport.

use std::fmt;

use crate::lease::{Capability, CapabilitySet, Generation, Intent, LeaseId, Priority, Source};

/// The current worker protocol version.
pub const PROTOCOL_VERSION: u16 = 1;
/// The maximum encoded frame size, including the fixed header.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
const MAX_STRING_BYTES: usize = 4096;
const HEADER_BYTES: usize = 11;

/// A worker handshake.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hello {
    /// The protocol version requested by the worker.
    pub protocol_version: u16,
    /// The source identity of the worker.
    pub source: Source,
    /// A stable configured adapter selector, not an `hciN` alias.
    pub adapter_selector: String,
    /// An optional opaque configured peer selector.
    pub device_selector: Option<String>,
    /// Capabilities requested by the worker.
    pub capabilities: CapabilitySet,
}

/// A lease request sent over the worker protocol.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Acquire {
    /// The requesting source.
    pub source: Source,
    /// The desired broker operation.
    pub intent: Intent,
    /// The scheduler priority.
    pub priority: Priority,
    /// The maximum number of monotonic seconds from broker receipt before the request expires.
    pub deadline: u64,
    /// The maximum lease hold time.
    pub max_hold: u64,
    /// Whether the operation may be preempted at a safe boundary.
    pub preemptible: bool,
    /// An optional opaque peer selector.
    pub device_selector: Option<String>,
    /// Capabilities needed by the operation.
    pub capabilities: CapabilitySet,
    /// A bounded human-readable reason.
    pub reason: String,
}

/// A typed operation permitted by a lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    /// Observe a device created by kernel autoconnect.
    ObserveDevice,
    /// Discover a configured target.
    DiscoverTarget,
    /// Initiate a permitted device connection.
    Connect,
    /// Resolve a service and characteristic set.
    Resolve,
    /// Subscribe to notifications or indications.
    Subscribe,
    /// Write a bounded GATT payload.
    Write,
}

/// A typed broker operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    /// The lease authorizing this operation.
    pub lease_id: LeaseId,
    /// A worker-generated operation correlation id.
    pub operation_id: u64,
    /// The typed operation kind.
    pub kind: OperationKind,
    /// A configured target or characteristic selector.
    pub target: String,
    /// A bounded payload, used only by typed operations that accept one.
    pub payload: Vec<u8>,
}

/// A lease release message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Release {
    /// The lease being released.
    pub lease_id: LeaseId,
    /// The named reason for release.
    pub reason: String,
}

/// An operation cancellation message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancel {
    /// The lease owning the operation.
    pub lease_id: LeaseId,
    /// The operation to cancel.
    pub operation_id: u64,
    /// The named cancellation reason.
    pub reason: String,
}

/// A client-to-broker protocol message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientMessage {
    /// Establish worker identity and capabilities.
    Hello(Hello),
    /// Request a lease.
    Acquire(Acquire),
    /// Execute one typed operation under a lease.
    Operation(Operation),
    /// Release a lease.
    Release(Release),
    /// Cancel an operation.
    Cancel(Cancel),
    /// Request a health snapshot.
    Health,
}

/// A broker-to-worker response or event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerMessage {
    /// Handshake accepted for the broker generation.
    HelloAccepted {
        /// Current broker generation.
        generation: Generation,
    },
    /// Lease granted immediately.
    LeaseGranted {
        /// Granted lease identifier.
        lease_id: LeaseId,
        /// Generation attached to the lease.
        generation: Generation,
        /// Monotonic lease expiry.
        expires_at: u64,
    },
    /// Lease is queued behind another owner.
    LeaseQueued {
        /// Queue ticket.
        ticket: u64,
    },
    /// Typed operation accepted.
    OperationAccepted {
        /// Operation correlation identifier.
        operation_id: u64,
    },
    /// GATT resolution returned an opaque broker handle.
    Resolved {
        /// Operation correlation identifier.
        operation_id: u64,
        /// Opaque characteristic handle for later typed operations.
        handle: String,
    },
    /// Current device transport state.
    DeviceState {
        /// Operation correlation identifier.
        operation_id: u64,
        /// Whether `BlueZ` reports the peer connected.
        connected: bool,
        /// Whether `BlueZ` reports services resolved.
        services_resolved: bool,
    },
    /// A bounded notification or indication payload.
    Frame {
        /// Lease that owns the frame stream.
        lease_id: LeaseId,
        /// Subscription operation correlation identifier.
        operation_id: u64,
        /// Whether the source frame is an indication.
        indication: bool,
        /// Raw GATT payload.
        payload: Vec<u8>,
    },
    /// The broker observed a device or stream disconnection.
    Disconnected {
        /// Operation correlation identifier.
        operation_id: u64,
        /// Redacted named reason.
        reason: String,
    },
    /// Lease release accepted.
    Released {
        /// Released lease identifier.
        lease_id: LeaseId,
    },
    /// Health snapshot.
    Health {
        /// Broker generation.
        generation: Generation,
        /// Current queue depth.
        queue_depth: usize,
    },
    /// A typed operation failed.
    Error {
        /// Operation correlation identifier, or zero for handshake errors.
        operation_id: u64,
        /// Named protocol/BlueZ failure code.
        code: u8,
        /// Redacted detail.
        detail: String,
    },
    /// A pending operation was cancelled at a safe broker boundary.
    Cancelled {
        /// Operation correlation identifier.
        operation_id: u64,
    },
}

/// A response produced by the deterministic fake transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FakeReply {
    /// The handshake was accepted.
    HelloAccepted {
        /// The broker generation used for subsequent operations.
        generation: Generation,
    },
    /// A non-health message was accepted into the bounded fake queue.
    Accepted,
    /// A health snapshot from the fake broker.
    Health {
        /// The current broker generation.
        generation: Generation,
        /// Number of accepted queued messages.
        queue_depth: usize,
    },
}

/// Protocol decoding, validation, or fake-transport failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    /// The frame exceeds the configured maximum.
    FrameTooLarge {
        /// The observed size.
        length: usize,
        /// The allowed maximum.
        max: usize,
    },
    /// The frame ended before its declared content was present.
    Truncated,
    /// The four-byte frame magic was invalid.
    BadMagic,
    /// The frame version is unsupported.
    UnsupportedVersion(u16),
    /// The message kind is not defined by this protocol.
    UnknownKind(u8),
    /// The broker response kind is not defined by this protocol.
    UnknownServerKind(u8),
    /// The payload contains invalid or trailing bytes.
    InvalidPayload,
    /// A string was not valid UTF-8.
    InvalidUtf8,
    /// The capability bitset contains an unknown bit.
    UnknownCapabilityBits(u8),
    /// A message other than `Hello` arrived before handshake.
    HandshakeRequired,
    /// A second `Hello` arrived on an established transport.
    HelloAlreadyReceived,
    /// BP requested userspace connection authority.
    BpConnectForbidden,
    /// The fake transport's bounded queue is full.
    QueueFull {
        /// The configured queue capacity.
        capacity: usize,
    },
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameTooLarge { length, max } => {
                write!(formatter, "frame too large: length={length}, max={max}")
            }
            Self::Truncated => formatter.write_str("truncated protocol frame"),
            Self::BadMagic => formatter.write_str("invalid protocol frame magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported protocol version: {version}")
            }
            Self::UnknownKind(kind) => write!(formatter, "unknown protocol message kind: {kind}"),
            Self::UnknownServerKind(kind) => {
                write!(formatter, "unknown broker response kind: {kind}")
            }
            Self::InvalidPayload => formatter.write_str("invalid protocol payload"),
            Self::InvalidUtf8 => formatter.write_str("protocol string is not UTF-8"),
            Self::UnknownCapabilityBits(bits) => {
                write!(formatter, "unknown capability bits: {bits:#04x}")
            }
            Self::HandshakeRequired => formatter.write_str("protocol handshake required"),
            Self::HelloAlreadyReceived => {
                formatter.write_str("protocol handshake already received")
            }
            Self::BpConnectForbidden => {
                formatter.write_str("BP worker cannot request userspace connection")
            }
            Self::QueueFull { capacity } => {
                write!(formatter, "fake transport queue full: capacity={capacity}")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Encode one client message into a complete bounded frame.
///
/// # Errors
///
/// Returns an error if a field is too large, a capability set is invalid, or
/// the encoded message exceeds [`MAX_FRAME_BYTES`].
pub fn encode(message: &ClientMessage) -> Result<Vec<u8>, ProtocolError> {
    let (kind, payload) = encode_payload(message)?;
    let frame_size =
        HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(ProtocolError::FrameTooLarge {
                length: usize::MAX,
                max: MAX_FRAME_BYTES,
            })?;
    if frame_size > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: frame_size,
            max: MAX_FRAME_BYTES,
        });
    }
    let payload_length =
        u32::try_from(payload.len()).map_err(|_| ProtocolError::FrameTooLarge {
            length: payload.len(),
            max: u32::MAX as usize,
        })?;
    let mut frame = Vec::with_capacity(frame_size);
    frame.extend_from_slice(b"VBT1");
    frame.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    frame.push(kind);
    frame.extend_from_slice(&payload_length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Decode one complete client frame.
///
/// # Errors
///
/// Returns an error for malformed headers, unsupported versions, invalid
/// payloads, unknown capabilities, or trailing bytes.
pub fn decode(frame: &[u8]) -> Result<ClientMessage, ProtocolError> {
    if frame.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: frame.len(),
            max: MAX_FRAME_BYTES,
        });
    }
    let mut reader = Reader::new(frame);
    if reader.take::<4>()? != *b"VBT1" {
        return Err(ProtocolError::BadMagic);
    }
    let version = reader.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(version));
    }
    let kind = reader.u8()?;
    let payload_length =
        usize::try_from(reader.u32()?).map_err(|_| ProtocolError::InvalidPayload)?;
    let payload = reader.bytes(payload_length)?;
    if reader.remaining() != 0 {
        return Err(ProtocolError::InvalidPayload);
    }
    let mut payload_reader = Reader::new(payload);
    let message = decode_payload(kind, &mut payload_reader)?;
    if payload_reader.remaining() != 0 {
        return Err(ProtocolError::InvalidPayload);
    }
    Ok(message)
}

/// Encode one broker response or event into a bounded frame.
///
/// # Errors
///
/// Returns an error when a bounded field exceeds the protocol limits.
pub fn encode_server(message: &ServerMessage) -> Result<Vec<u8>, ProtocolError> {
    let mut writer = Writer::default();
    let kind = match message {
        ServerMessage::HelloAccepted { generation } => {
            writer.u64(generation.value());
            0
        }
        ServerMessage::LeaseGranted {
            lease_id,
            generation,
            expires_at,
        } => {
            writer.u64(lease_id.value());
            writer.u64(generation.value());
            writer.u64(*expires_at);
            1
        }
        ServerMessage::LeaseQueued { ticket } => {
            writer.u64(*ticket);
            2
        }
        ServerMessage::OperationAccepted { operation_id } => {
            writer.u64(*operation_id);
            3
        }
        ServerMessage::Resolved {
            operation_id,
            handle,
        } => {
            writer.u64(*operation_id);
            writer.string(handle)?;
            4
        }
        ServerMessage::DeviceState {
            operation_id,
            connected,
            services_resolved,
        } => {
            writer.u64(*operation_id);
            writer.u8(u8::from(*connected));
            writer.u8(u8::from(*services_resolved));
            5
        }
        ServerMessage::Frame {
            lease_id,
            operation_id,
            indication,
            payload,
        } => {
            writer.u64(lease_id.value());
            writer.u64(*operation_id);
            writer.u8(u8::from(*indication));
            writer.bytes(payload)?;
            6
        }
        ServerMessage::Disconnected {
            operation_id,
            reason,
        } => {
            writer.u64(*operation_id);
            writer.string(reason)?;
            7
        }
        ServerMessage::Released { lease_id } => {
            writer.u64(lease_id.value());
            8
        }
        ServerMessage::Health {
            generation,
            queue_depth,
        } => {
            writer.u64(generation.value());
            writer.u64(u64::try_from(*queue_depth).map_err(|_| ProtocolError::InvalidPayload)?);
            9
        }
        ServerMessage::Error {
            operation_id,
            code,
            detail,
        } => {
            writer.u64(*operation_id);
            writer.u8(*code);
            writer.string(detail)?;
            10
        }
        ServerMessage::Cancelled { operation_id } => {
            writer.u64(*operation_id);
            11
        }
    };
    encode_frame(kind, &writer.finish())
}

/// Decode one broker response or event frame.
///
/// # Errors
///
/// Returns an error for malformed or unknown response frames.
pub fn decode_server(frame: &[u8]) -> Result<ServerMessage, ProtocolError> {
    let payload = decode_frame(frame)?;
    let mut reader = Reader::new(payload.1);
    match payload.0 {
        0 => Ok(ServerMessage::HelloAccepted {
            generation: Generation::new(reader.u64()?),
        }),
        1 => Ok(ServerMessage::LeaseGranted {
            lease_id: LeaseId::new(reader.u64()?),
            generation: Generation::new(reader.u64()?),
            expires_at: reader.u64()?,
        }),
        2 => Ok(ServerMessage::LeaseQueued {
            ticket: reader.u64()?,
        }),
        3 => Ok(ServerMessage::OperationAccepted {
            operation_id: reader.u64()?,
        }),
        4 => Ok(ServerMessage::Resolved {
            operation_id: reader.u64()?,
            handle: reader.string()?,
        }),
        5 => Ok(ServerMessage::DeviceState {
            operation_id: reader.u64()?,
            connected: decode_bool(reader.u8()?)?,
            services_resolved: decode_bool(reader.u8()?)?,
        }),
        6 => Ok(ServerMessage::Frame {
            lease_id: LeaseId::new(reader.u64()?),
            operation_id: reader.u64()?,
            indication: reader.u8()? != 0,
            payload: reader.bytes_field()?,
        }),
        7 => Ok(ServerMessage::Disconnected {
            operation_id: reader.u64()?,
            reason: reader.string()?,
        }),
        8 => Ok(ServerMessage::Released {
            lease_id: LeaseId::new(reader.u64()?),
        }),
        9 => Ok(ServerMessage::Health {
            generation: Generation::new(reader.u64()?),
            queue_depth: usize::try_from(reader.u64()?)
                .map_err(|_| ProtocolError::InvalidPayload)?,
        }),
        10 => Ok(ServerMessage::Error {
            operation_id: reader.u64()?,
            code: reader.u8()?,
            detail: reader.string()?,
        }),
        11 => Ok(ServerMessage::Cancelled {
            operation_id: reader.u64()?,
        }),
        kind => Err(ProtocolError::UnknownServerKind(kind)),
    }
    .and_then(|message| {
        if reader.remaining() == 0 {
            Ok(message)
        } else {
            Err(ProtocolError::InvalidPayload)
        }
    })
}

fn encode_frame(kind: u8, payload: &[u8]) -> Result<Vec<u8>, ProtocolError> {
    let frame_size =
        HEADER_BYTES
            .checked_add(payload.len())
            .ok_or(ProtocolError::FrameTooLarge {
                length: usize::MAX,
                max: MAX_FRAME_BYTES,
            })?;
    if frame_size > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: frame_size,
            max: MAX_FRAME_BYTES,
        });
    }
    let payload_length =
        u32::try_from(payload.len()).map_err(|_| ProtocolError::FrameTooLarge {
            length: payload.len(),
            max: MAX_FRAME_BYTES,
        })?;
    let mut frame = Vec::with_capacity(frame_size);
    frame.extend_from_slice(b"VBT1");
    frame.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    frame.push(kind);
    frame.extend_from_slice(&payload_length.to_be_bytes());
    frame.extend_from_slice(payload);
    Ok(frame)
}

fn decode_frame(frame: &[u8]) -> Result<(u8, &[u8]), ProtocolError> {
    if frame.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: frame.len(),
            max: MAX_FRAME_BYTES,
        });
    }
    let mut reader = Reader::new(frame);
    if reader.take::<4>()? != *b"VBT1" {
        return Err(ProtocolError::BadMagic);
    }
    let version = reader.u16()?;
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(version));
    }
    let kind = reader.u8()?;
    let payload_length =
        usize::try_from(reader.u32()?).map_err(|_| ProtocolError::InvalidPayload)?;
    let payload = reader.bytes(payload_length)?;
    if reader.remaining() != 0 {
        return Err(ProtocolError::InvalidPayload);
    }
    Ok((kind, payload))
}

/// A deterministic in-memory transport used by protocol and capability tests.
#[derive(Clone, Debug)]
pub struct FakeTransport {
    generation: Generation,
    capacity: usize,
    hello: Option<Hello>,
    accepted: Vec<ClientMessage>,
}

impl FakeTransport {
    /// Construct a fake transport with a bounded accepted-message queue.
    #[must_use]
    pub const fn new(generation: Generation, capacity: usize) -> Self {
        Self {
            generation,
            capacity,
            hello: None,
            accepted: Vec::new(),
        }
    }

    /// Send an encoded frame to the fake broker.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed frames, invalid handshake ordering,
    /// forbidden BP capabilities, or queue saturation.
    pub fn send(&mut self, frame: &[u8]) -> Result<FakeReply, ProtocolError> {
        let message = decode(frame)?;
        match &message {
            ClientMessage::Hello(hello) => {
                if self.hello.is_some() {
                    return Err(ProtocolError::HelloAlreadyReceived);
                }
                if hello.protocol_version != PROTOCOL_VERSION {
                    return Err(ProtocolError::UnsupportedVersion(hello.protocol_version));
                }
                if hello.source == Source::Bp && hello.capabilities.contains(Capability::Connect) {
                    return Err(ProtocolError::BpConnectForbidden);
                }
                self.hello = Some(hello.clone());
                Ok(FakeReply::HelloAccepted {
                    generation: self.generation,
                })
            }
            ClientMessage::Health => {
                if self.hello.is_none() {
                    return Err(ProtocolError::HandshakeRequired);
                }
                Ok(FakeReply::Health {
                    generation: self.generation,
                    queue_depth: self.accepted.len(),
                })
            }
            ClientMessage::Acquire(acquire) => {
                self.require_handshake()?;
                if acquire.source == Source::Bp
                    && (acquire.intent == Intent::Connect
                        || acquire.capabilities.contains(Capability::Connect))
                {
                    return Err(ProtocolError::BpConnectForbidden);
                }
                self.accept(message)
            }
            ClientMessage::Operation(operation) => {
                self.require_handshake()?;
                if operation.kind == OperationKind::Connect
                    && self
                        .hello
                        .as_ref()
                        .is_some_and(|hello| hello.source == Source::Bp)
                {
                    return Err(ProtocolError::BpConnectForbidden);
                }
                self.accept(message)
            }
            ClientMessage::Release(_) | ClientMessage::Cancel(_) => {
                self.require_handshake()?;
                self.accept(message)
            }
        }
    }

    /// Return accepted messages in arrival order.
    #[must_use]
    pub fn accepted(&self) -> &[ClientMessage] {
        &self.accepted
    }

    /// Return the negotiated worker handshake, if any.
    #[must_use]
    pub fn hello(&self) -> Option<&Hello> {
        self.hello.as_ref()
    }

    fn require_handshake(&self) -> Result<(), ProtocolError> {
        if self.hello.is_some() {
            Ok(())
        } else {
            Err(ProtocolError::HandshakeRequired)
        }
    }

    fn accept(&mut self, message: ClientMessage) -> Result<FakeReply, ProtocolError> {
        if self.accepted.len() >= self.capacity {
            return Err(ProtocolError::QueueFull {
                capacity: self.capacity,
            });
        }
        self.accepted.push(message);
        Ok(FakeReply::Accepted)
    }
}

fn encode_payload(message: &ClientMessage) -> Result<(u8, Vec<u8>), ProtocolError> {
    let mut writer = Writer::default();
    let kind = match message {
        ClientMessage::Hello(hello) => {
            writer.u16(hello.protocol_version);
            writer.u8(source_code(hello.source));
            writer.string(&hello.adapter_selector)?;
            writer.optional_string(hello.device_selector.as_deref())?;
            writer.u8(hello.capabilities.bits());
            0
        }
        ClientMessage::Acquire(acquire) => {
            writer.u8(source_code(acquire.source));
            writer.u8(intent_code(acquire.intent));
            writer.u8(acquire.priority.value());
            writer.u64(acquire.deadline);
            writer.u64(acquire.max_hold);
            writer.u8(u8::from(acquire.preemptible));
            writer.optional_string(acquire.device_selector.as_deref())?;
            writer.u8(acquire.capabilities.bits());
            writer.string(&acquire.reason)?;
            1
        }
        ClientMessage::Operation(operation) => {
            writer.u64(operation.lease_id.value());
            writer.u64(operation.operation_id);
            writer.u8(operation_code(operation.kind));
            writer.string(&operation.target)?;
            writer.bytes(&operation.payload)?;
            2
        }
        ClientMessage::Release(release) => {
            writer.u64(release.lease_id.value());
            writer.string(&release.reason)?;
            3
        }
        ClientMessage::Cancel(cancel) => {
            writer.u64(cancel.lease_id.value());
            writer.u64(cancel.operation_id);
            writer.string(&cancel.reason)?;
            4
        }
        ClientMessage::Health => 5,
    };
    Ok((kind, writer.finish()))
}

fn decode_payload(kind: u8, reader: &mut Reader<'_>) -> Result<ClientMessage, ProtocolError> {
    match kind {
        0 => Ok(ClientMessage::Hello(Hello {
            protocol_version: reader.u16()?,
            source: source_from_code(reader.u8()?)?,
            adapter_selector: reader.string()?,
            device_selector: reader.optional_string()?,
            capabilities: capabilities_from_bits(reader.u8()?)?,
        })),
        1 => Ok(ClientMessage::Acquire(Acquire {
            source: source_from_code(reader.u8()?)?,
            intent: intent_from_code(reader.u8()?)?,
            priority: Priority::new(reader.u8()?),
            deadline: reader.u64()?,
            max_hold: reader.u64()?,
            preemptible: reader.u8()? != 0,
            device_selector: reader.optional_string()?,
            capabilities: capabilities_from_bits(reader.u8()?)?,
            reason: reader.string()?,
        })),
        2 => Ok(ClientMessage::Operation(Operation {
            lease_id: LeaseId::from_wire(reader.u64()?),
            operation_id: reader.u64()?,
            kind: operation_from_code(reader.u8()?)?,
            target: reader.string()?,
            payload: reader.bytes_field()?,
        })),
        3 => Ok(ClientMessage::Release(Release {
            lease_id: LeaseId::from_wire(reader.u64()?),
            reason: reader.string()?,
        })),
        4 => Ok(ClientMessage::Cancel(Cancel {
            lease_id: LeaseId::from_wire(reader.u64()?),
            operation_id: reader.u64()?,
            reason: reader.string()?,
        })),
        5 => Ok(ClientMessage::Health),
        other => Err(ProtocolError::UnknownKind(other)),
    }
}

fn decode_bool(value: u8) -> Result<bool, ProtocolError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProtocolError::InvalidPayload),
    }
}

fn capabilities_from_bits(bits: u8) -> Result<CapabilitySet, ProtocolError> {
    CapabilitySet::from_bits(bits).ok_or(ProtocolError::UnknownCapabilityBits(bits))
}

fn source_code(source: Source) -> u8 {
    match source {
        Source::Bp => 0,
        Source::H10 => 1,
        Source::SleepO2 => 2,
        Source::AirSense => 3,
        Source::Wellue => 4,
        Source::Diagnostics => 5,
    }
}

fn source_from_code(code: u8) -> Result<Source, ProtocolError> {
    match code {
        0 => Ok(Source::Bp),
        1 => Ok(Source::H10),
        2 => Ok(Source::SleepO2),
        3 => Ok(Source::AirSense),
        4 => Ok(Source::Wellue),
        5 => Ok(Source::Diagnostics),
        _ => Err(ProtocolError::InvalidPayload),
    }
}

fn intent_code(intent: Intent) -> u8 {
    match intent {
        Intent::KernelObserve => 0,
        Intent::Discover => 1,
        Intent::Connect => 2,
        Intent::Gatt => 3,
        Intent::Stream => 4,
        Intent::PowerRecovery => 5,
    }
}

fn intent_from_code(code: u8) -> Result<Intent, ProtocolError> {
    match code {
        0 => Ok(Intent::KernelObserve),
        1 => Ok(Intent::Discover),
        2 => Ok(Intent::Connect),
        3 => Ok(Intent::Gatt),
        4 => Ok(Intent::Stream),
        5 => Ok(Intent::PowerRecovery),
        _ => Err(ProtocolError::InvalidPayload),
    }
}

fn operation_code(operation: OperationKind) -> u8 {
    match operation {
        OperationKind::ObserveDevice => 0,
        OperationKind::DiscoverTarget => 1,
        OperationKind::Connect => 2,
        OperationKind::Resolve => 3,
        OperationKind::Subscribe => 4,
        OperationKind::Write => 5,
    }
}

fn operation_from_code(code: u8) -> Result<OperationKind, ProtocolError> {
    match code {
        0 => Ok(OperationKind::ObserveDevice),
        1 => Ok(OperationKind::DiscoverTarget),
        2 => Ok(OperationKind::Connect),
        3 => Ok(OperationKind::Resolve),
        4 => Ok(OperationKind::Subscribe),
        5 => Ok(OperationKind::Write),
        _ => Err(ProtocolError::InvalidPayload),
    }
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn string(&mut self, value: &str) -> Result<(), ProtocolError> {
        let bytes = value.as_bytes();
        let length = bounded_length(bytes.len())?;
        self.u16(length);
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn optional_string(&mut self, value: Option<&str>) -> Result<(), ProtocolError> {
        match value {
            Some(value) => {
                self.u8(1);
                self.string(value)?;
            }
            None => self.u8(0),
        }
        Ok(())
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), ProtocolError> {
        let length = u32::try_from(value.len()).map_err(|_| ProtocolError::FrameTooLarge {
            length: value.len(),
            max: MAX_FRAME_BYTES,
        })?;
        self.bytes.extend_from_slice(&length.to_be_bytes());
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn bounded_length(length: usize) -> Result<u16, ProtocolError> {
    if length > MAX_STRING_BYTES || length > usize::from(u16::MAX) {
        return Err(ProtocolError::FrameTooLarge {
            length,
            max: MAX_STRING_BYTES,
        });
    }
    u16::try_from(length).map_err(|_| ProtocolError::FrameTooLarge {
        length,
        max: MAX_STRING_BYTES,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], ProtocolError> {
        let end = self.offset.checked_add(N).ok_or(ProtocolError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ProtocolError::Truncated)?;
        self.offset = end;
        value.try_into().map_err(|_| ProtocolError::Truncated)
    }

    fn u8(&mut self) -> Result<u8, ProtocolError> {
        let [value] = self.take::<1>()?;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, ProtocolError> {
        Ok(u16::from_be_bytes(self.take::<2>()?))
    }

    fn u32(&mut self) -> Result<u32, ProtocolError> {
        Ok(u32::from_be_bytes(self.take::<4>()?))
    }

    fn u64(&mut self) -> Result<u64, ProtocolError> {
        Ok(u64::from_be_bytes(self.take::<8>()?))
    }

    fn bytes(&mut self, length: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(ProtocolError::Truncated)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(ProtocolError::Truncated)?;
        self.offset = end;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, ProtocolError> {
        let length = usize::from(self.u16()?);
        if length > MAX_STRING_BYTES {
            return Err(ProtocolError::FrameTooLarge {
                length,
                max: MAX_STRING_BYTES,
            });
        }
        String::from_utf8(self.bytes(length)?.to_vec()).map_err(|_| ProtocolError::InvalidUtf8)
    }

    fn optional_string(&mut self) -> Result<Option<String>, ProtocolError> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.string().map(Some),
            _ => Err(ProtocolError::InvalidPayload),
        }
    }

    fn bytes_field(&mut self) -> Result<Vec<u8>, ProtocolError> {
        let length = usize::try_from(self.u32()?).map_err(|_| ProtocolError::InvalidPayload)?;
        Ok(self.bytes(length)?.to_vec())
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(source: Source, capabilities: CapabilitySet) -> ClientMessage {
        ClientMessage::Hello(Hello {
            protocol_version: PROTOCOL_VERSION,
            source,
            adapter_selector: "verified-sena".to_owned(),
            device_selector: Some("bp".to_owned()),
            capabilities,
        })
    }

    #[test]
    fn round_trip_preserves_typed_operation_and_payload() {
        let message = ClientMessage::Operation(Operation {
            lease_id: LeaseId::from_wire(4),
            operation_id: 9,
            kind: OperationKind::Write,
            target: "rx-characteristic".to_owned(),
            payload: vec![0, 1, 2, 255],
        });
        let frame_result = encode(&message);
        assert!(frame_result.is_ok());
        let Ok(frame) = frame_result else {
            return;
        };
        assert_eq!(decode(&frame), Ok(message));
    }

    #[test]
    fn server_events_round_trip_with_bounded_payload() {
        let message = ServerMessage::Frame {
            lease_id: LeaseId::new(7),
            operation_id: 9,
            indication: false,
            payload: vec![1, 2, 3],
        };
        let frame_result = encode_server(&message);
        assert!(frame_result.is_ok());
        let Ok(frame) = frame_result else {
            return;
        };
        assert_eq!(decode_server(&frame), Ok(message));
    }

    #[test]
    fn malformed_and_trailing_frames_fail_closed() {
        assert_eq!(decode(b"VBT1"), Err(ProtocolError::Truncated));
        assert_eq!(
            decode(b"NOPE\0\x01\0\0\0\0\0"),
            Err(ProtocolError::BadMagic)
        );
        let frame_result = encode(&ClientMessage::Health);
        assert!(frame_result.is_ok());
        let Ok(mut frame) = frame_result else {
            return;
        };
        frame.push(1);
        assert_eq!(decode(&frame), Err(ProtocolError::InvalidPayload));
    }

    #[test]
    fn unknown_capability_bits_are_rejected() {
        let Ok(mut frame) = encode(&hello(Source::H10, CapabilitySet::empty())) else {
            return;
        };
        let Some(capability_byte) = frame.last_mut() else {
            return;
        };
        *capability_byte = 0x80;
        assert_eq!(
            decode(&frame),
            Err(ProtocolError::UnknownCapabilityBits(0x80))
        );
    }

    #[test]
    fn fake_transport_requires_handshake_and_bounds_queue() {
        let mut transport = FakeTransport::new(Generation::new(3), 1);
        let health_result = encode(&ClientMessage::Health);
        assert!(health_result.is_ok());
        let Ok(health) = health_result else {
            return;
        };
        assert_eq!(
            transport.send(&health),
            Err(ProtocolError::HandshakeRequired)
        );
        let handshake_result = encode(&hello(Source::H10, CapabilitySet::empty()));
        assert!(handshake_result.is_ok());
        let Ok(handshake) = handshake_result else {
            return;
        };
        assert_eq!(
            transport.send(&handshake),
            Ok(FakeReply::HelloAccepted {
                generation: Generation::new(3)
            })
        );
        let acquire = ClientMessage::Acquire(Acquire {
            source: Source::H10,
            intent: Intent::Connect,
            priority: Priority::new(2),
            deadline: 100,
            max_hold: 10,
            preemptible: false,
            device_selector: Some("h10".to_owned()),
            capabilities: CapabilitySet::from_capability(Capability::Connect),
            reason: "test".to_owned(),
        });
        let acquire_result = encode(&acquire);
        assert!(acquire_result.is_ok());
        let Ok(acquire_frame) = acquire_result else {
            return;
        };
        assert_eq!(transport.send(&acquire_frame), Ok(FakeReply::Accepted));
        assert_eq!(
            transport.send(&acquire_frame),
            Err(ProtocolError::QueueFull { capacity: 1 })
        );
    }

    #[test]
    fn fake_transport_rejects_bp_connect_at_handshake_and_operation() {
        let mut transport = FakeTransport::new(Generation::new(1), 4);
        let handshake = hello(
            Source::Bp,
            CapabilitySet::from_capability(Capability::Connect),
        );
        let frame_result = encode(&handshake);
        assert!(frame_result.is_ok());
        let Ok(frame) = frame_result else {
            return;
        };
        assert_eq!(
            transport.send(&frame),
            Err(ProtocolError::BpConnectForbidden)
        );

        let good_handshake = hello(
            Source::Bp,
            CapabilitySet::from_capability(Capability::KernelObserve),
        );
        let frame_result = encode(&good_handshake);
        assert!(frame_result.is_ok());
        let Ok(frame) = frame_result else {
            return;
        };
        assert!(matches!(
            transport.send(&frame),
            Ok(FakeReply::HelloAccepted { .. })
        ));
        let operation = ClientMessage::Operation(Operation {
            lease_id: LeaseId::from_wire(1),
            operation_id: 1,
            kind: OperationKind::Connect,
            target: "bp".to_owned(),
            payload: Vec::new(),
        });
        let frame_result = encode(&operation);
        assert!(frame_result.is_ok());
        let Ok(frame) = frame_result else {
            return;
        };
        assert_eq!(
            transport.send(&frame),
            Err(ProtocolError::BpConnectForbidden)
        );
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "the complete typed protocol fixture is intentionally exhaustive"
    )]
    fn every_client_and_server_variant_round_trips() {
        let capabilities = CapabilitySet::empty()
            .with(Capability::Connect)
            .with(Capability::Resolve)
            .with(Capability::Subscribe)
            .with(Capability::Write);
        let acquire = Acquire {
            source: Source::H10,
            intent: Intent::Connect,
            priority: Priority::new(4),
            deadline: 20,
            max_hold: 30,
            preemptible: false,
            device_selector: Some("target".to_owned()),
            capabilities,
            reason: "round trip".to_owned(),
        };
        let client_messages = vec![
            hello(Source::H10, capabilities),
            ClientMessage::Acquire(acquire),
            ClientMessage::Release(Release {
                lease_id: LeaseId::new(1),
                reason: "release".to_owned(),
            }),
            ClientMessage::Cancel(Cancel {
                lease_id: LeaseId::new(1),
                operation_id: 2,
                reason: "cancel".to_owned(),
            }),
            ClientMessage::Health,
        ];
        for message in client_messages {
            let frame_result = encode(&message);
            assert!(frame_result.is_ok());
            let Ok(frame) = frame_result else {
                return;
            };
            assert_eq!(decode(&frame), Ok(message));
        }
        for (operation_id, kind) in [
            (1, OperationKind::ObserveDevice),
            (2, OperationKind::DiscoverTarget),
            (3, OperationKind::Connect),
            (4, OperationKind::Resolve),
            (5, OperationKind::Subscribe),
            (6, OperationKind::Write),
        ] {
            let message = ClientMessage::Operation(Operation {
                lease_id: LeaseId::new(1),
                operation_id,
                kind,
                target: "target".to_owned(),
                payload: vec![1, 2],
            });
            let frame_result = encode(&message);
            assert!(frame_result.is_ok());
            let Ok(frame) = frame_result else {
                return;
            };
            assert_eq!(decode(&frame), Ok(message));
        }

        let server_messages = vec![
            ServerMessage::HelloAccepted {
                generation: Generation::new(1),
            },
            ServerMessage::LeaseGranted {
                lease_id: LeaseId::new(1),
                generation: Generation::new(1),
                expires_at: 10,
            },
            ServerMessage::LeaseQueued { ticket: 2 },
            ServerMessage::OperationAccepted { operation_id: 3 },
            ServerMessage::Resolved {
                operation_id: 4,
                handle: "handle".to_owned(),
            },
            ServerMessage::DeviceState {
                operation_id: 5,
                connected: true,
                services_resolved: false,
            },
            ServerMessage::Frame {
                lease_id: LeaseId::new(1),
                operation_id: 6,
                indication: true,
                payload: vec![1, 2],
            },
            ServerMessage::Disconnected {
                operation_id: 7,
                reason: "reason".to_owned(),
            },
            ServerMessage::Released {
                lease_id: LeaseId::new(1),
            },
            ServerMessage::Health {
                generation: Generation::new(1),
                queue_depth: 2,
            },
            ServerMessage::Error {
                operation_id: 8,
                code: 2,
                detail: "detail".to_owned(),
            },
            ServerMessage::Cancelled { operation_id: 9 },
        ];
        for message in server_messages {
            let frame_result = encode_server(&message);
            assert!(frame_result.is_ok());
            let Ok(frame) = frame_result else {
                return;
            };
            assert_eq!(decode_server(&frame), Ok(message));
        }
    }

    #[test]
    fn private_protocol_codecs_reject_invalid_values_and_bounds() {
        assert_eq!(decode_bool(2), Err(ProtocolError::InvalidPayload));
        assert_eq!(source_from_code(99), Err(ProtocolError::InvalidPayload));
        assert_eq!(intent_from_code(99), Err(ProtocolError::InvalidPayload));
        assert_eq!(operation_from_code(99), Err(ProtocolError::InvalidPayload));
        assert_eq!(
            capabilities_from_bits(0x80),
            Err(ProtocolError::UnknownCapabilityBits(0x80))
        );
        assert!(matches!(
            bounded_length(MAX_STRING_BYTES + 1),
            Err(ProtocolError::FrameTooLarge { .. })
        ));
        let mut writer = Writer::default();
        assert!(writer.string(&"x".repeat(MAX_STRING_BYTES + 1)).is_err());
        assert!(writer.bytes(&[1, 2, 3]).is_ok());
        let mut reader = Reader::new(&[1, 2]);
        assert_eq!(reader.take::<4>(), Err(ProtocolError::Truncated));
        let mut reader = Reader::new(&[0, 0, 0, 2, 1]);
        assert_eq!(reader.bytes_field(), Err(ProtocolError::Truncated));
        let mut reader = Reader::new(&[1, 0]);
        assert_eq!(reader.optional_string(), Err(ProtocolError::Truncated));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn codec_helpers_cover_success_and_frame_rejection_paths() {
        for source in [
            Source::Bp,
            Source::H10,
            Source::SleepO2,
            Source::AirSense,
            Source::Wellue,
            Source::Diagnostics,
        ] {
            assert_eq!(source_from_code(source_code(source)), Ok(source));
        }
        for intent in [
            Intent::KernelObserve,
            Intent::Discover,
            Intent::Connect,
            Intent::Gatt,
            Intent::Stream,
            Intent::PowerRecovery,
        ] {
            assert_eq!(intent_from_code(intent_code(intent)), Ok(intent));
        }
        for kind in [
            OperationKind::ObserveDevice,
            OperationKind::DiscoverTarget,
            OperationKind::Connect,
            OperationKind::Resolve,
            OperationKind::Subscribe,
            OperationKind::Write,
        ] {
            assert_eq!(operation_from_code(operation_code(kind)), Ok(kind));
        }
        assert_eq!(decode_bool(0), Ok(false));
        assert_eq!(decode_bool(1), Ok(true));

        let mut writer = Writer::default();
        writer.u8(7);
        writer.u16(8);
        writer.u64(9);
        writer.optional_string(None).expect("test writer");
        writer.optional_string(Some("ok")).expect("test writer");
        writer.bytes(&[1, 2]).expect("test writer");
        let bytes = writer.finish();
        let mut reader = Reader::new(&bytes);
        assert_eq!(reader.u8(), Ok(7));
        assert_eq!(reader.u16(), Ok(8));
        assert_eq!(reader.u64(), Ok(9));
        assert_eq!(reader.optional_string(), Ok(None));
        assert_eq!(reader.optional_string(), Ok(Some("ok".to_owned())));
        assert_eq!(reader.bytes_field(), Ok(vec![1, 2]));
        assert_eq!(reader.remaining(), 0);

        assert!(matches!(
            encode(&ClientMessage::Hello(Hello {
                protocol_version: PROTOCOL_VERSION,
                source: Source::H10,
                adapter_selector: "x".repeat(MAX_STRING_BYTES + 1),
                device_selector: None,
                capabilities: CapabilitySet::empty(),
            })),
            Err(ProtocolError::FrameTooLarge { .. })
        ));
        assert_eq!(
            decode(b"VBT1\0\x02\0\0\0\0\0"),
            Err(ProtocolError::UnsupportedVersion(2))
        );
        let mut unknown_client = b"VBT1\0\x01\x7f\0\0\0\0".to_vec();
        assert_eq!(
            decode(&unknown_client),
            Err(ProtocolError::UnknownKind(0x7f))
        );
        if let Some(kind) = unknown_client.get_mut(6) {
            *kind = 0;
        }
        assert_eq!(decode(&unknown_client), Err(ProtocolError::Truncated));
        assert!(matches!(
            encode_server(&ServerMessage::Error {
                operation_id: 1,
                code: 1,
                detail: "x".repeat(MAX_STRING_BYTES + 1),
            }),
            Err(ProtocolError::FrameTooLarge { .. })
        ));

        let mut transport = FakeTransport::new(Generation::new(4), 4);
        assert!(transport.accepted().is_empty());
        assert!(transport.hello().is_none());
        let handshake = encode(&hello(Source::H10, CapabilitySet::empty())).expect("test frame");
        assert!(transport.send(&handshake).is_ok());
        assert!(transport.hello().is_some());
        assert_eq!(transport.accepted().len(), 0);
    }

    #[test]
    fn protocol_error_display_names_each_blocker() {
        let errors = [
            ProtocolError::FrameTooLarge { length: 2, max: 1 },
            ProtocolError::Truncated,
            ProtocolError::BadMagic,
            ProtocolError::UnsupportedVersion(2),
            ProtocolError::UnknownKind(99),
            ProtocolError::UnknownServerKind(99),
            ProtocolError::InvalidPayload,
            ProtocolError::InvalidUtf8,
            ProtocolError::UnknownCapabilityBits(128),
            ProtocolError::HandshakeRequired,
            ProtocolError::HelloAlreadyReceived,
            ProtocolError::BpConnectForbidden,
            ProtocolError::QueueFull { capacity: 1 },
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}
