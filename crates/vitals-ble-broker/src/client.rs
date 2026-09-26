//! Typed Unix-socket client for source workers.
//!
//! The client exposes only broker protocol operations. It contains no BlueZ,
//! D-Bus object paths, adapter power controls, or arbitrary method calls.

use std::{fmt, path::Path, time::Duration};

use crate::protocol::{
    Acquire, ClientMessage, Hello, Operation, OperationKind, ProtocolError, Release, ServerMessage,
    decode_server, encode,
};
use crate::{CapabilitySet, Generation, LeaseId, Priority, Source};

/// A broker client failure with a named blocker.
#[derive(Debug)]
pub enum ClientError {
    /// Unix-socket or framing I/O failed.
    Io(std::io::Error),
    /// The broker sent an invalid response.
    Protocol(ProtocolError),
    /// The broker rejected an operation.
    Broker { code: u8, detail: String },
    /// A response did not match the typed request.
    UnexpectedResponse(&'static str),
    /// The broker queued an operation that this bounded client cannot hold.
    Queued { ticket: u64 },
    /// An operation exceeded its deadline after broker cancellation.
    DeadlineExceeded { operation: &'static str },
    /// The broker did not acknowledge cancellation after a deadline.
    CancellationFailed {
        /// Operation that could not be cancelled.
        operation: &'static str,
        /// Named cancellation blocker.
        detail: String,
    },
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "broker socket I/O failed: {error}"),
            Self::Protocol(error) => write!(formatter, "broker protocol failed: {error}"),
            Self::Broker { code, detail } => {
                write!(
                    formatter,
                    "broker operation rejected: code={code}, detail={detail}"
                )
            }
            Self::UnexpectedResponse(operation) => {
                write!(formatter, "broker response blocked: expected={operation}")
            }
            Self::Queued { ticket } => {
                write!(formatter, "broker lease queued: ticket={ticket}")
            }
            Self::DeadlineExceeded { operation } => {
                write!(
                    formatter,
                    "broker operation deadline exceeded: operation={operation}"
                )
            }
            Self::CancellationFailed { operation, detail } => write!(
                formatter,
                "broker cancellation failed: operation={operation}, detail={detail}"
            ),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<std::io::Error> for ClientError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ProtocolError> for ClientError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

/// A granted worker lease.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrokerLease {
    /// Lease identifier used in typed operations.
    pub id: LeaseId,
    /// Generation that must match all operations.
    pub generation: Generation,
    /// Monotonic expiry supplied by the broker.
    pub expires_at: u64,
}

/// A typed event from a broker-owned H10 subscription.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrokerEvent {
    /// Raw notification or indication payload.
    Frame {
        /// Raw GATT bytes.
        payload: Vec<u8>,
        /// Whether the payload was an indication.
        indication: bool,
    },
    /// Current transport properties.
    DeviceState {
        /// Whether the broker observes the peer connected.
        connected: bool,
        /// Whether GATT services are resolved.
        services_resolved: bool,
    },
    /// The broker ended the stream.
    Disconnected {
        /// Redacted named reason.
        reason: String,
    },
}

/// A bounded H10/source-worker broker client.
pub struct BrokerClient {
    reader: tokio::net::unix::OwnedReadHalf,
    writer: tokio::net::unix::OwnedWriteHalf,
    lease: BrokerLease,
    next_operation: u64,
}

impl BrokerClient {
    /// Connect and authenticate as one H10 worker.
    ///
    /// # Errors
    ///
    /// Returns a named socket, handshake, or broker rejection error.
    pub async fn connect(
        socket: impl AsRef<Path>,
        adapter_selector: impl Into<String>,
        device_selector: impl Into<String>,
        capabilities: CapabilitySet,
    ) -> Result<Self, ClientError> {
        let stream = tokio::net::UnixStream::connect(socket).await?;
        let (reader, writer) = stream.into_split();
        let mut client = Self {
            reader,
            writer,
            lease: BrokerLease {
                id: LeaseId::new(0),
                generation: Generation::new(0),
                expires_at: 0,
            },
            next_operation: 1,
        };
        client
            .send(ClientMessage::Hello(Hello {
                protocol_version: crate::PROTOCOL_VERSION,
                source: Source::H10,
                adapter_selector: adapter_selector.into(),
                device_selector: Some(device_selector.into()),
                capabilities,
            }))
            .await?;
        match client.receive().await? {
            ServerMessage::HelloAccepted { generation } => {
                client.lease.generation = generation;
                Ok(client)
            }
            ServerMessage::Error { code, detail, .. } => Err(ClientError::Broker { code, detail }),
            _ => Err(ClientError::UnexpectedResponse("HelloAccepted")),
        }
    }

    /// Request a bounded connect lease.
    pub async fn acquire_connect(
        &mut self,
        priority: Priority,
        deadline: u64,
        max_hold: u64,
        device_selector: impl Into<String>,
    ) -> Result<BrokerLease, ClientError> {
        self.send(ClientMessage::Acquire(Acquire {
            source: Source::H10,
            intent: crate::Intent::Connect,
            priority,
            deadline,
            max_hold,
            preemptible: false,
            device_selector: Some(device_selector.into()),
            capabilities: CapabilitySet::empty()
                .with(crate::Capability::Connect)
                .with(crate::Capability::Resolve)
                .with(crate::Capability::Subscribe),
            reason: "H10 broker transport connect".to_owned(),
        }))
        .await?;
        match self.receive().await? {
            ServerMessage::LeaseGranted {
                lease_id,
                generation,
                expires_at,
            } => {
                self.lease = BrokerLease {
                    id: lease_id,
                    generation,
                    expires_at,
                };
                Ok(self.lease)
            }
            ServerMessage::LeaseQueued { ticket } => Err(ClientError::Queued { ticket }),
            ServerMessage::Error { code, detail, .. } => Err(ClientError::Broker { code, detail }),
            _ => Err(ClientError::UnexpectedResponse("LeaseGranted")),
        }
    }

    /// Initiate a broker-owned connection for the acquired lease.
    pub async fn connect_device(&mut self, target: impl Into<String>) -> Result<(), ClientError> {
        let operation_id = self.next_operation();
        self.send_operation(operation_id, OperationKind::Connect, target, Vec::new())
            .await?;
        self.expect_operation(operation_id).await
    }

    /// Initiate a broker-owned connection with cancellation on deadline.
    pub async fn connect_device_bounded(
        &mut self,
        target: impl Into<String>,
        deadline: Duration,
    ) -> Result<(), ClientError> {
        let operation_id = self.next_operation();
        self.send_operation(operation_id, OperationKind::Connect, target, Vec::new())
            .await?;
        match tokio::time::timeout(deadline, self.expect_operation(operation_id)).await {
            Ok(result) => result,
            Err(_) => Err(self.cancel_after_deadline(operation_id, "connect").await),
        }
    }

    /// Resolve the configured H10 service and measurement characteristic.
    pub async fn resolve_characteristic(
        &mut self,
        service_uuid: impl Into<String>,
        characteristic_uuid: impl Into<String>,
    ) -> Result<String, ClientError> {
        self.resolve_characteristic_bounded(
            service_uuid,
            characteristic_uuid,
            Duration::from_secs(30),
        )
        .await
    }

    /// Resolve a characteristic with cancellation on deadline.
    pub async fn resolve_characteristic_bounded(
        &mut self,
        service_uuid: impl Into<String>,
        characteristic_uuid: impl Into<String>,
        deadline: Duration,
    ) -> Result<String, ClientError> {
        let operation_id = self.next_operation();
        let target = format!("{}|{}", service_uuid.into(), characteristic_uuid.into());
        self.send_operation(operation_id, OperationKind::Resolve, target, Vec::new())
            .await?;
        match tokio::time::timeout(deadline, self.receive()).await {
            Ok(ServerMessage::Resolved {
                operation_id: response_id,
                handle,
            }) if response_id == operation_id => Ok(handle),
            Ok(ServerMessage::Error { code, detail, .. }) => {
                Err(ClientError::Broker { code, detail })
            }
            Ok(Err(error)) => Err(error),
            Ok(_) => Err(ClientError::UnexpectedResponse("Resolved")),
            Err(_) => Err(self.cancel_after_deadline(operation_id, "resolve").await),
        }
    }

    /// Subscribe to the resolved characteristic.
    pub async fn subscribe(&mut self, handle: impl Into<String>) -> Result<(), ClientError> {
        self.subscribe_bounded(handle, Duration::from_secs(30))
            .await
    }

    /// Subscribe with cancellation on deadline.
    pub async fn subscribe_bounded(
        &mut self,
        handle: impl Into<String>,
        deadline: Duration,
    ) -> Result<(), ClientError> {
        let operation_id = self.next_operation();
        self.send_operation(operation_id, OperationKind::Subscribe, handle, Vec::new())
            .await?;
        match tokio::time::timeout(deadline, self.expect_operation(operation_id)).await {
            Ok(result) => result,
            Err(_) => Err(self.cancel_after_deadline(operation_id, "subscribe").await),
        }
    }

    /// Read the next broker event from the bounded subscription stream.
    pub async fn next_event(&mut self) -> Result<Option<BrokerEvent>, ClientError> {
        loop {
            match self.receive().await? {
                ServerMessage::Frame {
                    payload,
                    indication,
                    ..
                } => {
                    return Ok(Some(BrokerEvent::Frame {
                        payload,
                        indication,
                    }));
                }
                ServerMessage::DeviceState {
                    connected,
                    services_resolved,
                    ..
                } => {
                    return Ok(Some(BrokerEvent::DeviceState {
                        connected,
                        services_resolved,
                    }));
                }
                ServerMessage::Disconnected { reason, .. } => {
                    return Ok(Some(BrokerEvent::Disconnected { reason }));
                }
                ServerMessage::Error { code, detail, .. } => {
                    return Err(ClientError::Broker { code, detail });
                }
                _ => {}
            }
        }
    }

    /// Release the worker lease after the source stream has ended.
    pub async fn release(&mut self, reason: impl Into<String>) -> Result<(), ClientError> {
        self.send(ClientMessage::Release(Release {
            lease_id: self.lease.id,
            reason: reason.into(),
        }))
        .await?;
        match self.receive().await? {
            ServerMessage::Released { lease_id } if lease_id == self.lease.id => Ok(()),
            ServerMessage::Error { code, detail, .. } => Err(ClientError::Broker { code, detail }),
            _ => Err(ClientError::UnexpectedResponse("Released")),
        }
    }

    async fn send(&mut self, message: ClientMessage) -> Result<(), ClientError> {
        let frame = encode(&message)?;
        tokio::io::AsyncWriteExt::write_all(&mut self.writer, &frame).await?;
        Ok(())
    }

    async fn send_operation(
        &mut self,
        operation_id: u64,
        kind: OperationKind,
        target: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<(), ClientError> {
        self.send(ClientMessage::Operation(Operation {
            lease_id: self.lease.id,
            operation_id,
            kind,
            target: target.into(),
            payload,
        }))
        .await
    }

    async fn expect_operation(&mut self, operation_id: u64) -> Result<(), ClientError> {
        match self.receive().await? {
            ServerMessage::OperationAccepted { operation_id: id } if id == operation_id => Ok(()),
            ServerMessage::Error { code, detail, .. } => Err(ClientError::Broker { code, detail }),
            _ => Err(ClientError::UnexpectedResponse("OperationAccepted")),
        }
    }

    async fn cancel_after_deadline(
        &mut self,
        operation_id: u64,
        operation: &'static str,
    ) -> ClientError {
        let cancel = self
            .send(ClientMessage::Cancel(crate::Cancel {
                lease_id: self.lease.id,
                operation_id,
                reason: format!("{operation} deadline exceeded"),
            }))
            .await;
        let cancel = match cancel {
            Ok(()) => match self.receive().await {
                Ok(ServerMessage::Cancelled { operation_id: id }) if id == operation_id => Ok(()),
                Ok(ServerMessage::Error { code, detail, .. }) => {
                    Err(ClientError::Broker { code, detail })
                }
                Ok(_) => Err(ClientError::UnexpectedResponse("Cancelled")),
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        match cancel {
            Ok(()) => ClientError::DeadlineExceeded { operation },
            Err(error) => ClientError::CancellationFailed {
                operation,
                detail: error.to_string(),
            },
        }
    }

    async fn receive(&mut self) -> Result<ServerMessage, ClientError> {
        let mut header = [0_u8; 11];
        tokio::io::AsyncReadExt::read_exact(&mut self.reader, &mut header).await?;
        let payload_length = usize::try_from(u32::from_be_bytes([
            header[7], header[8], header[9], header[10],
        ]))
        .map_err(|_| ClientError::Protocol(ProtocolError::InvalidPayload))?;
        let frame_length =
            11_usize
                .checked_add(payload_length)
                .ok_or(ProtocolError::FrameTooLarge {
                    length: usize::MAX,
                    max: crate::MAX_FRAME_BYTES,
                })?;
        if frame_length > crate::MAX_FRAME_BYTES {
            return Err(ClientError::Protocol(ProtocolError::FrameTooLarge {
                length: frame_length,
                max: crate::MAX_FRAME_BYTES,
            }));
        }
        let mut frame = Vec::with_capacity(frame_length);
        frame.extend_from_slice(&header);
        let mut payload = vec![0_u8; payload_length];
        tokio::io::AsyncReadExt::read_exact(&mut self.reader, &mut payload).await?;
        frame.extend_from_slice(&payload);
        Ok(decode_server(&frame)?)
    }

    fn next_operation(&mut self) -> u64 {
        let operation = self.next_operation;
        self.next_operation = self.next_operation.saturating_add(1);
        operation
    }
}
