//! Pure policy for the Linux `BlueZ` broker.
//!
//! This crate deliberately has no `BlueZ`, D-Bus, filesystem, clock, or Tokio
//! dependency. The policy is deterministic and is exercised with caller-supplied
//! monotonic ticks. The Linux adapter owner and worker protocol are added by later
//! tasks.
#![forbid(unsafe_code)]

pub mod bluez;
pub mod lease;
mod protocol;

pub use lease::{
    AcquireResult, Capability, CapabilitySet, DeviceId, Generation, Intent, Lease, LeaseId,
    LeaseRequest, LeaseScheduler, NamedBlocker, PolicyError, Priority, QueueTicket, ReleaseOutcome,
    SchedulerEvent, Source, Tick,
};

pub use protocol::{
    Acquire, Cancel, ClientMessage, FakeReply, FakeTransport, Hello, MAX_FRAME_BYTES, Operation,
    OperationKind, PROTOCOL_VERSION, ProtocolError, Release, decode, encode,
};
