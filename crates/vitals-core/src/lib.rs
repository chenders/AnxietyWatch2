//! Shared value types, the `PostgreSQL` binary `COPY` encoder, and the
//! append-only spool.
//!
//! Implemented by T-D3. The `COPY` encoder is pure and portable — no database
//! handle, no I/O — so it unit-tests on macOS as well as on the Pi.
//!
//! # Identity
//!
//! A sample is identified by `(source, type, session_id, seq)`. `timestamp` is a
//! best-estimate event time and is **not** part of the key: several RR intervals
//! arrive in one BLE notification, so keying on arrival makes them collide and
//! `ON CONFLICT DO NOTHING` discards real measurements while every health metric
//! reads normal (amendment 05 §1).
//!
//! `seq` derives from whatever the source makes stable across retransmission —
//! the sensor's own data for a streaming source, the delivery index for a
//! confirmed store-and-forward one (amendment 06 §1.2). A source that is
//! neither cannot be given a correct identity and must not be ingested.
#![forbid(unsafe_code)]

pub mod copybin;
pub mod session;
pub mod spool;

pub use session::{NodeId, Session, SessionMinter};
pub use spool::{SchemaFingerprint, Spool};

use std::time::SystemTime;

/// One measurement, ready to encode.
///
/// `node_id` is deliberately absent: it is constant for the life of the process
/// and the column is `NOT NULL`, so carrying it per sample would waste 16 bytes
/// each for a value that never varies. It is passed to
/// [`copybin::encode_row`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    /// `VitalsSource` id from the registry. Never a bare literal at a call site.
    pub source: i32,
    /// `VitalsType` id from the registry.
    pub r#type: i32,
    /// Identifies one upstream connection. **Unique across process restarts** —
    /// see [`SessionMinter`] for why that is load-bearing.
    pub session_id: i64,
    /// Stable identity within the session.
    pub seq: i64,
    /// Best-estimate event time. May be an estimate and may be revised, which
    /// is only safe because it is not part of the key.
    pub timestamp: SystemTime,
    /// The value, in the registry's canonical unit for `type`. Converted once,
    /// at the decoder.
    pub value: f64,
    /// Source-specific detail. `None` is SQL `NULL`, which is distinct from
    /// `Some(vec![])`.
    pub extra: Option<Vec<u8>>,
    /// `None` means the row did not arrive through the signed ingest path. That
    /// is the detection mechanism, not an error state to hide (amendment 02
    /// §2.2).
    pub batch_id: Option<i64>,
}
