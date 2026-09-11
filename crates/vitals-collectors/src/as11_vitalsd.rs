//! `AirSense`'s bounded `vitalsd` append contract.
//!
//! This module owns no socket and no storage. It keeps the Linux client wire
//! construction and acknowledgement validation testable on a host while the
//! binary supplies only the Unix-stream I/O.

use vitals_proto::{ProtoError, Request, Response, WireSample, encode_frame};

/// A bounded append acknowledgement failure.
#[derive(Debug, PartialEq, Eq)]
pub enum AppendResponseError {
    /// The daemon acknowledged a different number of rows.
    CountMismatch {
        /// Number acknowledged by the daemon.
        actual: usize,
        /// Number sent by this request.
        expected: usize,
    },
    /// The daemon rejected the append and named its blocker.
    Daemon {
        /// Machine-readable error class.
        kind: vitals_proto::ErrorKind,
        /// Human-readable blocker.
        detail: String,
    },
    /// The daemon returned a response for another operation.
    UnexpectedResponse,
}

impl core::fmt::Display for AppendResponseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CountMismatch { actual, expected } => {
                write!(f, "vitalsd append count={actual}, expected={expected}")
            }
            Self::Daemon { kind, detail } => write!(f, "vitalsd append {kind:?}: {detail}"),
            Self::UnexpectedResponse => {
                f.write_str("vitalsd append returned an unexpected response")
            }
        }
    }
}

impl std::error::Error for AppendResponseError {}

/// Encodes the non-fsync append used by the streaming path.
///
/// The returned count is kept beside the frame so a caller cannot validate a
/// reply against a later, accidentally different batch.
///
/// # Errors
/// Returns the protocol encoder's bounded framing error.
pub fn encode_stream_append(samples: Vec<WireSample>) -> Result<(Vec<u8>, usize), ProtoError> {
    let expected = samples.len();
    let frame = encode_frame(&Request::Append {
        samples,
        sync: false,
    })?;
    Ok((frame, expected))
}

/// Validates one `vitalsd` response against the append count.
///
/// # Errors
/// Rejects a short/over-count acknowledgement, a daemon error, or a response
/// belonging to a different request.
pub fn validate_stream_append(
    response: Response,
    expected: usize,
) -> Result<usize, AppendResponseError> {
    match response {
        Response::Appended { count, .. } if count == expected => Ok(count),
        Response::Appended { count, .. } => Err(AppendResponseError::CountMismatch {
            actual: count,
            expected,
        }),
        Response::Error { kind, detail } => Err(AppendResponseError::Daemon { kind, detail }),
        Response::Sample { .. } | Response::Health { .. } => {
            Err(AppendResponseError::UnexpectedResponse)
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "wire-contract tests use fixed local values"
)]
mod tests {
    use super::*;

    fn sample(seq: i64) -> WireSample {
        WireSample {
            source: 5,
            r#type: 9,
            session_id: 73,
            seq,
            timestamp_unix_micros: 1_800_000_000_000_000,
            value: 12.0,
            extra: None,
            batch_id: None,
        }
    }

    #[test]
    fn stream_append_is_framed_as_non_fsync_append() {
        let (frame, expected) = encode_stream_append(vec![sample(4), sample(5)]).expect("encode");
        let length = u32::from_le_bytes(frame.get(..4).expect("length").try_into().expect("array"));
        let body = frame.get(4..).expect("body");
        assert_eq!(expected, 2);
        assert_eq!(
            usize::try_from(length).expect("length conversion"),
            body.len()
        );
        let request: Request = vitals_proto::decode_payload(body).expect("decode request");
        assert_eq!(
            request,
            Request::Append {
                samples: vec![sample(4), sample(5)],
                sync: false,
            }
        );
    }

    #[test]
    fn append_acknowledgement_must_match_the_batch() {
        let response = Response::Appended {
            count: 1,
            durable: false,
        };
        assert!(matches!(
            validate_stream_append(response, 2),
            Err(AppendResponseError::CountMismatch {
                actual: 1,
                expected: 2
            })
        ));
    }

    #[test]
    fn daemon_error_and_wrong_response_are_not_success() {
        let error = validate_stream_append(
            Response::Error {
                kind: vitals_proto::ErrorKind::Unavailable,
                detail: "spool is not ready: restore gate=closed".to_owned(),
            },
            1,
        )
        .expect_err("daemon error");
        assert!(matches!(error, AppendResponseError::Daemon { .. }));
        assert_eq!(
            validate_stream_append(
                Response::Health {
                    health: Box::new(vitals_proto::Health {
                        uptime_secs: 0,
                        sources: Vec::new(),
                        total_samples: 0,
                        spool_bytes: 0,
                        lag_bytes: 0,
                        lag_age_seconds: 0,
                        last_advanced_at: None,
                        subscribers: Vec::new(),
                        heartbeat: vitals_proto::HeartbeatInputs {
                            monitoring_window_active: false,
                            primary_sample_age_secs: None,
                            tier_evaluation_age_secs: None,
                            drain_progressing: true,
                        },
                        heartbeat_pulsing: true,
                        degraded: Vec::new(),
                    }),
                },
                1,
            ),
            Err(AppendResponseError::UnexpectedResponse)
        );
    }
}
