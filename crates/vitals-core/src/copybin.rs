//! `PostgreSQL` binary `COPY` encoder.
//!
//! Pure and portable: no allocation beyond the caller's buffer, no I/O, no
//! database handle. That is why it lives here rather than in the Linux-only
//! crate, and why it unit-tests on macOS as well as on the Pi.
//!
//! # Two ways to be silently wrong
//!
//! **Byte order.** Every integer in the binary format is network byte order.
//! Both hosts are little-endian arm64, so a native-endian write produces a
//! value `Postgres` accepts without complaint and interprets as garbage. Every
//! integer here goes through `to_be_bytes`.
//!
//! **The epoch.** `TIMESTAMPTZ` is microseconds since **2000-01-01 UTC**, not
//! the Unix epoch. An encoder written from the specification alone will very
//! plausibly emit Unix-epoch microseconds, which `Postgres` stores happily and
//! reads back thirty years off. This was confirmed against a real server rather
//! than derived: `'2000-01-01 00:00:00+00'` round-trips as exactly zero.
//!
//! Neither failure produces an error. Both produce a number that looks like a
//! measurement.

use crate::{NodeId, Sample};
use core::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

/// `PGCOPY\n\377\r\n\0` — the 11-byte file signature.
const SIGNATURE: [u8; 11] = [
    0x50, 0x47, 0x43, 0x4F, 0x50, 0x59, 0x0A, 0xFF, 0x0D, 0x0A, 0x00,
];

/// Columns in `samples`, in declaration order. Binary `COPY` is positional:
/// the encoder and `db/deploy/samples.sql` must agree exactly, and a mismatch
/// is not detectable from the bytes alone.
pub const FIELD_COUNT: i16 = 9;

/// Seconds between the Unix epoch and the `Postgres` epoch (2000-01-01 UTC).
const PG_EPOCH_OFFSET_SECS: i64 = 946_684_800;

/// Field length marking SQL `NULL`.
const NULL_LEN: i32 = -1;

/// Why a row could not be encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// A timestamp outside the range `TIMESTAMPTZ` can represent.
    ///
    /// Returned rather than saturated: a clamped timestamp is indistinguishable
    /// from a real one downstream, and this system quarantines with a reason
    /// instead of clamping.
    TimestampOutOfRange {
        /// The offending time, as seconds from the Unix epoch.
        unix_secs: i64,
    },
    /// A `BYTEA` longer than a field length can describe.
    FieldTooLarge {
        /// Which column.
        field: &'static str,
        /// Its length in bytes.
        len: usize,
    },
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TimestampOutOfRange { unix_secs } => {
                write!(
                    f,
                    "timestamp {unix_secs}s from the Unix epoch is outside TIMESTAMPTZ range"
                )
            }
            Self::FieldTooLarge { field, len } => {
                write!(
                    f,
                    "{field} is {len} bytes, too large for a COPY field length"
                )
            }
        }
    }
}

impl std::error::Error for EncodeError {}

/// The 19-byte file header: signature, a zero flags word, a zero
/// header-extension length.
#[must_use]
pub fn copy_header() -> [u8; 19] {
    let mut out = [0u8; 19];
    let (sig, rest) = out.split_at_mut(SIGNATURE.len());
    sig.copy_from_slice(&SIGNATURE);
    // flags and extension length are already zero.
    debug_assert_eq!(rest.len(), 8);
    out
}

/// The file trailer: a field count of `-1`.
#[must_use]
pub fn copy_trailer() -> [u8; 2] {
    (-1i16).to_be_bytes()
}

/// Converts a `SystemTime` to `Postgres` microseconds since 2000-01-01 UTC.
///
/// Separate and public so it can be tested directly against values captured
/// from a real server — the epoch shift is the single easiest thing here to get
/// wrong in a way nothing downstream would notice.
///
/// # Errors
/// [`EncodeError::TimestampOutOfRange`] if the instant cannot be represented.
pub fn pg_timestamp_micros(t: SystemTime) -> Result<i64, EncodeError> {
    let (unix_secs, sub_micros) = match t.duration_since(UNIX_EPOCH) {
        Ok(d) => (
            i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
            i64::from(d.subsec_micros()),
        ),
        Err(e) => {
            // Before 1970. Valid for TIMESTAMPTZ; the duration is just negative.
            let d: Duration = e.duration();
            let secs = i64::try_from(d.as_secs()).unwrap_or(i64::MAX);
            let micros = i64::from(d.subsec_micros());
            if micros == 0 {
                (-secs, 0)
            } else {
                (-secs - 1, 1_000_000 - micros)
            }
        }
    };
    unix_secs
        .checked_sub(PG_EPOCH_OFFSET_SECS)
        .and_then(|s| s.checked_mul(1_000_000))
        .and_then(|us| us.checked_add(sub_micros))
        .ok_or(EncodeError::TimestampOutOfRange { unix_secs })
}

fn put_field(out: &mut Vec<u8>, bytes: &[u8], field: &'static str) -> Result<(), EncodeError> {
    let len = i32::try_from(bytes.len()).map_err(|_| EncodeError::FieldTooLarge {
        field,
        len: bytes.len(),
    })?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

fn put_null(out: &mut Vec<u8>) {
    out.extend_from_slice(&NULL_LEN.to_be_bytes());
}

/// Encodes one row, appending to `out`. Returns the number of bytes written.
///
/// `node_id` is passed rather than carried on [`Sample`]: it is constant for
/// the life of the process, and the column is `NOT NULL`, so a `Sample` alone
/// cannot describe a valid row.
///
/// # Errors
/// [`EncodeError`] if the timestamp is unrepresentable or a `BYTEA` is too long.
/// On error `out` is truncated back to its original length, so a failed row
/// never leaves a partial record in the buffer — a half-written row would
/// desynchronise every subsequent row in the stream.
pub fn encode_row(out: &mut Vec<u8>, s: &Sample, node_id: &NodeId) -> Result<usize, EncodeError> {
    let start = out.len();
    match encode_row_inner(out, s, node_id) {
        Ok(()) => Ok(out.len() - start),
        Err(e) => {
            out.truncate(start);
            Err(e)
        }
    }
}

fn encode_row_inner(out: &mut Vec<u8>, s: &Sample, node_id: &NodeId) -> Result<(), EncodeError> {
    let micros = pg_timestamp_micros(s.timestamp)?;
    out.extend_from_slice(&FIELD_COUNT.to_be_bytes());
    put_field(out, &s.source.to_be_bytes(), "source")?;
    put_field(out, &s.r#type.to_be_bytes(), "type")?;
    put_field(out, &s.session_id.to_be_bytes(), "session_id")?;
    put_field(out, &s.seq.to_be_bytes(), "seq")?;
    put_field(out, &micros.to_be_bytes(), "timestamp")?;
    put_field(out, &s.value.to_be_bytes(), "value")?;
    match &s.extra {
        Some(b) => put_field(out, b, "extra")?,
        None => put_null(out),
    }
    put_field(out, node_id.as_bytes(), "node_id")?;
    match s.batch_id {
        // NULL means the row did not arrive through the signed ingest path.
        // That is the detection mechanism, not an error state to hide.
        Some(id) => put_field(out, &id.to_be_bytes(), "batch_id")?,
        None => put_null(out),
    }
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
#[expect(
    clippy::duration_suboptimal_units,
    reason = "epoch seconds; expressing them in hours would obscure the instant"
)]
mod tests {
    use super::*;

    /// Captured from `PostgreSQL` 16 with:
    ///
    /// ```sql
    /// COPY (SELECT * FROM samples ORDER BY session_id) TO STDOUT (FORMAT binary)
    /// ```
    ///
    /// against the table in `db/deploy/samples.sql`. Ground truth, not this
    /// encoder's own output — self-consistency is not correctness, and an
    /// encoder tested against itself agrees with itself about the wrong epoch.
    const REFERENCE: &str = concat!(
        "5047434f50590aff0d0a000000000000000000",
        "0009",
        "00000004",
        "00000006",
        "00000004",
        "00000006",
        "00000008",
        "0000000000000001",
        "00000008",
        "0000000000000000",
        "00000008",
        "0002fb27fbc96f00",
        "00000008",
        "4060e00000000000",
        "ffffffff",
        "00000010",
        "000102030405060708090a0b0c0d0e0f",
        "ffffffff",
        "0009",
        "00000004",
        "00000001",
        "00000004",
        "00000005",
        "00000008",
        "000000000000002a",
        "00000008",
        "00000000000004d2",
        "00000008",
        "0000000000000000",
        "00000008",
        "3feb400000000000",
        "00000004",
        "deadbeef",
        "00000010",
        "000102030405060708090a0b0c0d0e0f",
        "00000008",
        "000000000000004d",
        "ffff",
    );

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .filter_map(|i| s.get(i..i + 2))
            .filter_map(|b| u8::from_str_radix(b, 16).ok())
            .collect()
    }

    fn node() -> NodeId {
        let mut b = [0u8; 16];
        for (i, slot) in b.iter_mut().enumerate() {
            *slot = u8::try_from(i).unwrap_or(0);
        }
        NodeId::from_bytes(b)
    }

    fn at(unix_secs: i64) -> SystemTime {
        if unix_secs >= 0 {
            UNIX_EPOCH + Duration::from_secs(u64::try_from(unix_secs).unwrap_or(0))
        } else {
            UNIX_EPOCH - Duration::from_secs(u64::try_from(-unix_secs).unwrap_or(0))
        }
    }

    /// The whole point of this test file: byte-for-byte agreement with a real
    /// server, header and trailer included.
    #[test]
    fn matches_bytes_captured_from_postgres() {
        let n = node();
        let rows = [
            Sample {
                source: 6,
                r#type: 6,
                session_id: 1,
                seq: 0,
                timestamp: at(1_785_783_900), // 2026-08-03T19:05:00Z
                value: 135.0,
                extra: None,
                batch_id: None,
            },
            Sample {
                source: 1,
                r#type: 5,
                session_id: 42,
                seq: 1234,
                timestamp: at(946_684_800), // 2000-01-01T00:00:00Z — the PG epoch
                value: 0.851_562_5,
                extra: Some(vec![0xDE, 0xAD, 0xBE, 0xEF]),
                batch_id: Some(77),
            },
        ];

        let mut out = Vec::new();
        out.extend_from_slice(&copy_header());
        for s in &rows {
            encode_row(&mut out, s, &n).expect("row encodes");
        }
        out.extend_from_slice(&copy_trailer());

        let want = hex(REFERENCE);
        assert_eq!(
            out.len(),
            want.len(),
            "length differs: got {} want {}",
            out.len(),
            want.len()
        );
        assert_eq!(
            out, want,
            "encoded stream differs from the captured reference"
        );
    }

    /// The epoch is the failure this file exists to prevent. Unix-epoch micros
    /// would make this `946_684_800_000_000` instead of 0, and `Postgres` would
    /// accept it.
    #[test]
    fn postgres_epoch_is_2000_not_1970() {
        assert_eq!(pg_timestamp_micros(at(946_684_800)), Ok(0));
        assert_eq!(pg_timestamp_micros(UNIX_EPOCH), Ok(-946_684_800_000_000));
        // The captured row-0 value, independently.
        assert_eq!(
            pg_timestamp_micros(at(1_785_783_900)),
            Ok(839_099_100_000_000)
        );
    }

    /// Times before 1970 are legal `TIMESTAMPTZ` values, and `duration_since`
    /// returns an error rather than a negative duration for them — an easy way
    /// to end up encoding a large positive number.
    #[test]
    fn pre_unix_epoch_times_encode_negative() {
        let t = UNIX_EPOCH - Duration::from_hours(24); // 1969-12-31
        let us = pg_timestamp_micros(t).expect("representable");
        assert_eq!(us, (-86_400 - 946_684_800) * 1_000_000);
        assert!(us < 0);
    }

    #[test]
    fn sub_second_precision_survives() {
        let t = UNIX_EPOCH + Duration::from_secs(946_684_800) + Duration::from_micros(123_456);
        assert_eq!(pg_timestamp_micros(t), Ok(123_456));
    }

    #[test]
    fn header_and_trailer_are_exact() {
        assert_eq!(copy_header().len(), 19);
        assert_eq!(&copy_header()[..11], &SIGNATURE);
        assert_eq!(copy_header()[11..], [0u8; 8]);
        assert_eq!(copy_trailer(), [0xFF, 0xFF]);
    }

    /// Every integer must be big-endian. On this little-endian host a native
    /// write would pass any test that only round-trips through this encoder.
    #[test]
    fn integers_are_network_byte_order() {
        let mut out = Vec::new();
        let s = Sample {
            source: 0x0102_0304,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp: at(946_684_800),
            value: 1.0,
            extra: None,
            batch_id: None,
        };
        encode_row(&mut out, &s, &node()).expect("encodes");
        // i16 count, then source's i32 length, then source itself.
        assert_eq!(out.get(6..10), Some(&[0x01, 0x02, 0x03, 0x04][..]));
    }

    /// A failed row must leave nothing behind. A partial record desynchronises
    /// every row after it, and the reader has no way to detect the boundary.
    #[test]
    fn a_failed_row_leaves_the_buffer_untouched() {
        let mut out = vec![0xAA, 0xBB];
        let before = out.clone();
        let s = Sample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            // Far outside TIMESTAMPTZ range; the multiply overflows.
            timestamp: UNIX_EPOCH + Duration::from_secs(u64::MAX / 2),
            value: 1.0,
            extra: None,
            batch_id: None,
        };
        assert!(encode_row(&mut out, &s, &node()).is_err());
        assert_eq!(out, before, "a failed row must not leave a partial record");
    }

    /// NULL is a length of -1 with no payload, distinct from a zero-length
    /// BYTEA. Conflating them would turn "did not arrive signed" into
    /// "arrived signed with an empty batch".
    #[test]
    fn null_and_empty_bytea_are_distinct() {
        let n = node();
        let base = Sample {
            source: 1,
            r#type: 1,
            session_id: 1,
            seq: 1,
            timestamp: at(946_684_800),
            value: 1.0,
            extra: None,
            batch_id: None,
        };
        let mut with_null = Vec::new();
        encode_row(&mut with_null, &base, &n).expect("encodes");

        let mut with_empty = Vec::new();
        encode_row(
            &mut with_empty,
            &Sample {
                extra: Some(vec![]),
                ..base.clone()
            },
            &n,
        )
        .expect("encodes");

        assert_ne!(with_null, with_empty);
        assert_eq!(
            with_null.len(),
            with_empty.len(),
            "both are 4 bytes of length"
        );
        // -1 versus 0.
        assert!(with_null.windows(4).any(|w| w == (-1i32).to_be_bytes()));
    }
}
