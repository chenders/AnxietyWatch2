//! Policy, protocol, and Linux transport boundary for the `BlueZ` broker.
//!
//! The lease policy remains deterministic and is exercised with caller-supplied
//! monotonic ticks. Linux-only adapter ownership and Unix-socket client transport
//! are isolated behind the `linux` feature and never expose arbitrary D-Bus calls.
#![forbid(unsafe_code)]

pub mod bluez;
pub mod bp;
#[cfg(all(feature = "linux", target_os = "linux"))]
pub mod client;
pub mod lease;
mod protocol;
#[cfg(all(feature = "linux", target_os = "linux"))]
pub mod server;
#[cfg(all(feature = "linux", target_os = "linux"))]
mod telemetry;

pub use lease::{
    AcquireResult, Capability, CapabilitySet, DeviceId, Generation, Intent, Lease, LeaseId,
    LeaseRequest, LeaseScheduler, NamedBlocker, PolicyError, Priority, QueueTicket, ReleaseOutcome,
    SchedulerEvent, Source, Tick,
};

pub use protocol::{
    Acquire, Cancel, ClientMessage, FakeReply, FakeTransport, Hello, MAX_FRAME_BYTES, Operation,
    OperationKind, PROTOCOL_VERSION, ProtocolError, Release, ServerMessage, decode, decode_server,
    encode, encode_server,
};

#[cfg(all(feature = "linux", target_os = "linux"))]
pub use client::{BrokerClient, BrokerEvent, ClientError as BrokerClientError};
