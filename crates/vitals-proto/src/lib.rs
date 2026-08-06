//! The `vitalsd` socket wire format.
//!
//! Pure: framing and message types only, no I/O and no runtime. That keeps it
//! testable anywhere and lets a collector, the daemon and a future CLI share one
//! definition rather than three that drift.
//!
//! # Framing
//!
//! `[u32 little-endian length][CBOR payload]`. The length prefix is LE because
//! every host here is little-endian and this is a local Unix socket, not a
//! network protocol — unlike the `COPY` encoder, where network byte order is
//! mandated by `PostgreSQL`.
//!
//! A length prefix needs a ceiling or a malformed peer can make the reader
//! allocate arbitrarily. [`MAX_FRAME`] is that ceiling, and exceeding it is a
//! protocol error rather than an allocation.
//!
//! # Why CBOR
//!
//! Self-describing, so an older peer skips a field it does not know instead of
//! misparsing the remainder. That matters because collectors and the daemon are
//! deployed separately and will not always be the same build.

use serde::{Deserialize, Serialize};

/// Largest frame the reader will accept, in bytes.
///
/// Generous for a batch of samples, far below anything that threatens memory.
/// Without a ceiling, a peer sending `u32::MAX` makes the reader try to
/// allocate 4 GiB before discovering the frame never arrives.
pub const MAX_FRAME: usize = 8 * 1024 * 1024;

/// One measurement on the wire.
///
/// Mirrors `vitals_core::Sample` but owns its own definition: the protocol must
/// be able to change independently of the store's in-memory type, and a shared
/// struct would make every storage refactor a wire-format change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireSample {
    /// Registry source id.
    pub source: i32,
    /// Registry type id.
    pub r#type: i32,
    /// Identifies one upstream connection; unique across process restarts.
    pub session_id: i64,
    /// Stable identity within the session.
    pub seq: i64,
    /// Best-estimate event time, microseconds since the Unix epoch.
    ///
    /// Unix epoch on the wire, converted to the `PostgreSQL` epoch at the
    /// encoder. Two epochs in one system is a hazard, so the boundary is stated:
    /// **the wire is Unix, the store is `PostgreSQL`**, and exactly one place
    /// converts.
    pub timestamp_unix_micros: i64,
    /// The value, in the registry's canonical unit for `type`.
    pub value: f64,
    /// Source-specific detail. `None` is SQL `NULL`.
    pub extra: Option<Vec<u8>>,
    /// `None` means the row did not arrive through the signed ingest path.
    pub batch_id: Option<i64>,
}

/// What a client asks for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verb", rename_all = "snake_case")]
pub enum Request {
    /// Append samples to the spool.
    Append {
        /// The samples.
        samples: Vec<WireSample>,
        /// Fsync before replying (amendment 06 §2.1).
        ///
        /// Defaults to `false`, so a caller that omits it gets the batched
        /// path. Only `bp-collector` sets it: a BP frame is deleted by the
        /// device on the kernel's ATT confirmation and is reconstructible from
        /// nothing, while a night of RR is ~250 MB and ~100k appends that
        /// per-append fsync would destroy.
        #[serde(default)]
        sync: bool,
    },
    /// Read history. Requires the store, which is gated on a verified restore.
    Query {
        /// Registry source id, or `None` for all.
        source: Option<i32>,
        /// Registry type id, or `None` for all.
        r#type: Option<i32>,
        /// Inclusive lower bound, Unix microseconds.
        since_unix_micros: i64,
        /// Exclusive upper bound, Unix microseconds.
        until_unix_micros: i64,
    },
    /// Register for live samples matching a selector.
    Subscribe {
        /// Registry source id, or `None` for all.
        source: Option<i32>,
        /// Registry type id, or `None` for all.
        r#type: Option<i32>,
    },
    /// Everything the daemon knows about its own health.
    Health,
}

/// What the daemon answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "snake_case")]
pub enum Response {
    /// The append landed. `durable` is `true` only if it was fsynced before
    /// this reply — a caller that asked for `sync` must be able to tell.
    Appended {
        /// How many samples were written.
        count: usize,
        /// Whether the bytes were fsynced before this reply.
        durable: bool,
    },
    /// A live sample for a subscriber.
    Sample {
        /// The sample.
        sample: Box<WireSample>,
    },
    /// Health, with every degraded state naming its blocker.
    Health {
        /// The report.
        health: Box<Health>,
    },
    /// The request could not be served, and why.
    ///
    /// Never a bare failure: a guard that will not say what blocked it costs an
    /// hour of guessing.
    Error {
        /// Machine-readable class.
        kind: ErrorKind,
        /// Human-readable detail, naming the blocker and its value.
        detail: String,
    },
}

/// Why a request failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The frame did not parse, or a field was out of range.
    Malformed,
    /// The verb is defined but its dependency is not yet available.
    ///
    /// Distinct from `Malformed`: the caller did nothing wrong, and the detail
    /// says what is missing. `Query` returns this until the store exists.
    Unavailable,
    /// The daemon could not write.
    Io,
}

/// A per-source liveness state.
///
/// **`Stale` is not `Ok`, and neither is `NeverSeen`.** Silence is never
/// reassurance: a source that has stopped reporting must be visibly distinct
/// from one reporting normally, or a dead sensor reads as a calm night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Liveness {
    /// A sample arrived within the expected cadence.
    Live,
    /// Nothing has arrived within the cadence. Carries the age in `SourceHealth`.
    Stale,
    /// Nothing has ever arrived from this source in this process's lifetime.
    NeverSeen,
}

/// One source's liveness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceHealth {
    /// Registry source id.
    pub source: i32,
    /// Live, stale, or never seen.
    pub liveness: Liveness,
    /// Seconds since the last sample, or `None` if never seen.
    pub last_sample_age_secs: Option<u64>,
    /// Total samples appended from this source since start.
    pub samples: u64,
}

/// A subscriber that has lost samples.
///
/// Dropping is the correct behaviour under overflow — never block the collector
/// — but a detector that missed samples must *know* it missed them, so the
/// count is reported and the subscriber is named.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriberHealth {
    /// Which subscriber, by connection id.
    pub id: u64,
    /// Samples dropped because its queue was full.
    pub dropped: u64,
    /// Current queue depth.
    pub queued: usize,
}

/// The four facts the hardware dead-man switch depends on (amendment 05 §2.1).
///
/// `klaxond` pulses its pin only if all of these hold during a monitoring
/// window. A healthy `klaxond` pulsing while every collector is dead is the
/// worst false negative available, so the heartbeat attests the *chain*, not one
/// process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatInputs {
    /// Whether a monitoring window is active. Outside one, the heartbeat is
    /// unconditional — nothing is expected, so absence is not a fault.
    pub monitoring_window_active: bool,
    /// Seconds since a sample from a primary source, if any.
    pub primary_sample_age_secs: Option<u64>,
    /// Seconds since `cns-engine` last produced a tier evaluation, if ever.
    pub tier_evaluation_age_secs: Option<u64>,
    /// Whether the spool drain offset advanced, or the spool is empty.
    pub drain_progressing: bool,
}

impl HeartbeatInputs {
    /// Whether the heartbeat should pulse.
    ///
    /// Every unknown counts as a failure: `None` for either age means the
    /// evidence is missing, and missing evidence is not a pass. Indeterminate
    /// is never treated as safe.
    #[must_use]
    pub fn should_pulse(&self, max_age_secs: u64) -> bool {
        if !self.monitoring_window_active {
            return true;
        }
        let fresh = |age: Option<u64>| age.is_some_and(|a| a <= max_age_secs);
        fresh(self.primary_sample_age_secs)
            && fresh(self.tier_evaluation_age_secs)
            && self.drain_progressing
    }

    /// Which conditions are failing, for the health report.
    ///
    /// A guard that blocks must name its blocker.
    #[must_use]
    pub fn blockers(&self, max_age_secs: u64) -> Vec<String> {
        let mut out = Vec::new();
        if !self.monitoring_window_active {
            return out;
        }
        match self.primary_sample_age_secs {
            None => out.push("no primary sample has ever arrived".to_owned()),
            Some(a) if a > max_age_secs => {
                out.push(format!(
                    "primary sample is {a}s old (limit {max_age_secs}s)"
                ));
            }
            Some(_) => {}
        }
        match self.tier_evaluation_age_secs {
            None => out.push("cns-engine has never produced a tier evaluation".to_owned()),
            Some(a) if a > max_age_secs => {
                out.push(format!(
                    "tier evaluation is {a}s old (limit {max_age_secs}s)"
                ));
            }
            Some(_) => {}
        }
        if !self.drain_progressing {
            out.push("spool drain offset is not advancing".to_owned());
        }
        out
    }
}

/// The daemon's self-report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Health {
    /// Seconds since start.
    pub uptime_secs: u64,
    /// Per-source liveness.
    pub sources: Vec<SourceHealth>,
    /// Total samples appended since start.
    pub total_samples: u64,
    /// Bytes in the current spool segment.
    pub spool_bytes: u64,
    /// Bytes waiting for the `PostgreSQL` drainer.
    pub lag_bytes: u64,
    /// Age of the oldest waiting segment, or zero when empty.
    pub lag_age_seconds: u64,
    /// UTC epoch seconds when a drain offset last advanced; unknown before the
    /// first committed drain.
    pub last_advanced_at: Option<u64>,
    /// Subscribers, with their drop counts.
    pub subscribers: Vec<SubscriberHealth>,
    /// Inputs to the hardware dead-man switch.
    pub heartbeat: HeartbeatInputs,
    /// Whether the heartbeat should currently pulse.
    pub heartbeat_pulsing: bool,
    /// Every reason the daemon considers itself degraded, each naming its
    /// blocker and the value that blocked it. Empty means healthy.
    pub degraded: Vec<String>,
}

/// A framing or encoding failure.
#[derive(Debug)]
pub enum ProtoError {
    /// The frame exceeded [`MAX_FRAME`].
    FrameTooLarge {
        /// The advertised length.
        len: usize,
    },
    /// CBOR could not be produced or parsed.
    Cbor(String),
}

impl core::fmt::Display for ProtoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FrameTooLarge { len } => {
                write!(f, "frame of {len} bytes exceeds the {MAX_FRAME}-byte limit")
            }
            Self::Cbor(e) => write!(f, "CBOR: {e}"),
        }
    }
}

impl std::error::Error for ProtoError {}

/// Serialises a message into a length-prefixed frame.
///
/// # Errors
/// [`ProtoError::Cbor`] if serialisation fails, [`ProtoError::FrameTooLarge`]
/// if the payload exceeds [`MAX_FRAME`] — checked on the way out as well as the
/// way in, so this process cannot send something a conforming peer must reject.
pub fn encode_frame<T: Serialize>(msg: &T) -> Result<Vec<u8>, ProtoError> {
    let mut payload = Vec::new();
    ciborium::into_writer(msg, &mut payload).map_err(|e| ProtoError::Cbor(e.to_string()))?;
    if payload.len() > MAX_FRAME {
        return Err(ProtoError::FrameTooLarge { len: payload.len() });
    }
    let len = u32::try_from(payload.len())
        .map_err(|_| ProtoError::FrameTooLarge { len: payload.len() })?;
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Parses a CBOR payload (the frame body, without the length prefix).
///
/// # Errors
/// [`ProtoError::Cbor`] if the bytes are not a valid encoding of `T`.
pub fn decode_payload<T: for<'de> Deserialize<'de>>(payload: &[u8]) -> Result<T, ProtoError> {
    ciborium::from_reader(payload).map_err(|e| ProtoError::Cbor(e.to_string()))
}

/// Validates a length prefix before anything is allocated for it.
///
/// # Errors
/// [`ProtoError::FrameTooLarge`] if the advertised length exceeds
/// [`MAX_FRAME`]. Returning an error here rather than allocating is the point:
/// a peer sending `u32::MAX` would otherwise make the reader reserve 4 GiB for
/// a frame that never arrives.
pub fn check_frame_len(len: u32) -> Result<usize, ProtoError> {
    let len = len as usize;
    if len > MAX_FRAME {
        return Err(ProtoError::FrameTooLarge { len });
    }
    Ok(len)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
mod tests {
    use super::*;

    fn wire(seq: i64) -> WireSample {
        WireSample {
            source: 6,
            r#type: 6,
            session_id: 9,
            seq,
            timestamp_unix_micros: 1_785_783_900_000_000,
            value: 135.0,
            extra: None,
            batch_id: None,
        }
    }

    #[test]
    fn requests_round_trip() {
        for req in [
            Request::Append {
                samples: vec![wire(0), wire(1)],
                sync: true,
            },
            Request::Append {
                samples: vec![],
                sync: false,
            },
            Request::Query {
                source: Some(6),
                r#type: None,
                since_unix_micros: 0,
                until_unix_micros: 1,
            },
            Request::Subscribe {
                source: None,
                r#type: Some(2),
            },
            Request::Health,
        ] {
            let framed = encode_frame(&req).expect("encode");
            let len = u32::from_le_bytes(
                framed
                    .get(..4)
                    .expect("prefix")
                    .try_into()
                    .expect("4 bytes"),
            );
            let n = check_frame_len(len).expect("length ok");
            let back: Request =
                decode_payload(framed.get(4..4 + n).expect("body")).expect("decode");
            assert_eq!(req, back);
        }
    }

    /// `sync` must default to false when a peer omits it, so an older collector
    /// cannot accidentally opt every append into fsync — or, worse, have its
    /// omission read as `true`.
    #[test]
    fn sync_defaults_to_false_when_absent() {
        #[derive(Serialize)]
        struct Old {
            verb: &'static str,
            samples: Vec<WireSample>,
        }
        let bytes = encode_frame(&Old {
            verb: "append",
            samples: vec![wire(0)],
        })
        .expect("encode");
        let req: Request = decode_payload(bytes.get(4..).expect("body")).expect("decode");
        assert_eq!(
            req,
            Request::Append {
                samples: vec![wire(0)],
                sync: false
            }
        );
    }

    /// A length prefix must be rejected before it is used to size an
    /// allocation.
    #[test]
    fn oversized_frames_are_rejected_before_allocation() {
        assert!(check_frame_len(u32::MAX).is_err());
        let ok = u32::try_from(MAX_FRAME).expect("fits");
        assert!(check_frame_len(ok).is_ok());
        assert!(check_frame_len(ok + 1).is_err());
    }

    #[test]
    fn garbage_payloads_are_an_error_not_a_panic() {
        let r: Result<Request, _> = decode_payload(&[0xFF, 0x00, 0x13, 0x37]);
        assert!(r.is_err());
        let r: Result<Request, _> = decode_payload(&[]);
        assert!(r.is_err());
    }

    /// Outside a monitoring window nothing is expected, so absence is not a
    /// fault and the heartbeat is unconditional.
    #[test]
    fn heartbeat_is_unconditional_outside_a_monitoring_window() {
        let h = HeartbeatInputs {
            monitoring_window_active: false,
            primary_sample_age_secs: None,
            tier_evaluation_age_secs: None,
            drain_progressing: false,
        };
        assert!(h.should_pulse(30));
        assert!(h.blockers(30).is_empty());
    }

    /// The worst false negative available: a healthy daemon pulsing while every
    /// collector is dead.
    #[test]
    fn heartbeat_stops_when_any_link_in_the_chain_is_missing() {
        let good = HeartbeatInputs {
            monitoring_window_active: true,
            primary_sample_age_secs: Some(5),
            tier_evaluation_age_secs: Some(5),
            drain_progressing: true,
        };
        assert!(good.should_pulse(30));
        assert!(good.blockers(30).is_empty());

        let stale_sample = HeartbeatInputs {
            primary_sample_age_secs: Some(31),
            ..good.clone()
        };
        assert!(!stale_sample.should_pulse(30));
        assert!(
            stale_sample
                .blockers(30)
                .first()
                .is_some_and(|b| b.contains("primary sample is 31s old"))
        );

        let stale_tier = HeartbeatInputs {
            tier_evaluation_age_secs: Some(99),
            ..good.clone()
        };
        assert!(!stale_tier.should_pulse(30));
        assert!(
            stale_tier
                .blockers(30)
                .first()
                .is_some_and(|b| b.contains("tier evaluation is 99s old"))
        );

        let stuck_drain = HeartbeatInputs {
            drain_progressing: false,
            ..good.clone()
        };
        assert!(!stuck_drain.should_pulse(30));
        assert!(
            stuck_drain
                .blockers(30)
                .first()
                .is_some_and(|b| b.contains("drain offset is not advancing"))
        );
    }

    /// Missing evidence is not a pass. `None` means nothing has ever been
    /// observed, which must never read as healthy.
    #[test]
    fn never_observed_is_not_treated_as_fresh() {
        let h = HeartbeatInputs {
            monitoring_window_active: true,
            primary_sample_age_secs: None,
            tier_evaluation_age_secs: Some(1),
            drain_progressing: true,
        };
        assert!(!h.should_pulse(30));
        // The blocker must say the evidence is absent, not merely stale — the
        // operator needs to know nothing has ever arrived, not that it aged out.
        assert_eq!(
            h.blockers(30),
            vec!["no primary sample has ever arrived".to_owned()]
        );
    }

    /// Every blocker is listed, not just the first — otherwise fixing one
    /// reveals the next and the operator debugs in series.
    #[test]
    fn all_blockers_are_reported_together() {
        let h = HeartbeatInputs {
            monitoring_window_active: true,
            primary_sample_age_secs: None,
            tier_evaluation_age_secs: None,
            drain_progressing: false,
        };
        assert_eq!(h.blockers(30).len(), 3);
    }
}
