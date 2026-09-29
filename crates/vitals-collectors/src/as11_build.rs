//! Registry-backed `AirSense` 11 source identity and sample construction.
//!
//! This is the boundary where decoded stream values become canonical samples.
//! Device wall time is used only as an untrusted source position; the stored
//! timestamp is always the host receipt time.  No value uses
//! `Session::next_seq()`: a retransmitted source position must retain the same
//! identity, while a bad position stops publication instead of silently
//! inventing a host sequence.

use std::{
    str,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::json;
use vitals_decode::stream::{StreamFrame, StreamSignal};
use vitals_proto::WireSample;
use vitals_registry::{VitalsSource, VitalsType};

use crate::as11_rpc::AIR11_SAMPLE_INTERVAL_MS;

/// Maximum metadata size attached to one emitted sample.
pub const MAX_AIR11_EXTRA_BYTES: usize = 512;
/// The number of supported stream signals, including the two nullable
/// external-oximeter signals.
pub const AIR11_SIGNAL_SLOTS: usize = 17;

/// The result of building one bounded `StreamData` frame.
#[derive(Debug, Default)]
pub struct Air11Batch {
    /// Valid numeric samples.  The vector is bounded by the decoder's value
    /// capacity and never contains a row for a JSON `null`.
    pub samples: Vec<WireSample>,
    /// Number of unavailable/null values or no-data frames observed.
    pub unavailable: u32,
    /// Number of source positions already emitted in this connection.
    pub duplicates: u32,
}

/// A source-position or value policy failure.  No variant carries a secret.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Air11BuildError {
    /// The connection identity was not a positive persistent session id.
    InvalidSessionId {
        /// The rejected session id.
        session_id: i64,
    },
    /// The host receipt timestamp cannot be represented as Unix microseconds.
    HostTimestampInvalid,
    /// The source start time was malformed or outside the supported epoch.
    SourceStartTimeInvalid,
    /// A later frame moved the device source clock backwards.
    SourceStartTimeBackwards {
        /// The previous parsed source start time in microseconds.
        previous: i64,
        /// The rejected current parsed source start time in microseconds.
        current: i64,
    },
    /// A source sample position moved backwards within one signal.
    SourcePositionBackwards {
        /// The signal whose source position moved backwards.
        signal: StreamSignal,
        /// The last accepted source position.
        previous: i64,
        /// The rejected source position.
        current: i64,
    },
    /// A known signal did not have a registry mapping.
    UnsupportedSignal {
        /// The unmapped signal.
        signal: StreamSignal,
    },
    /// A numeric value was not finite after canonical-unit conversion.
    NonFiniteValue {
        /// The signal whose conversion was non-finite.
        signal: StreamSignal,
    },
    /// A converted value was outside its registry range and was not clamped.
    ValueOutOfRange {
        /// The signal whose value was out of range.
        signal: StreamSignal,
        /// The converted value that was rejected.
        value: f64,
    },
    /// Source metadata could not be encoded within its finite bound.
    MetadataTooLarge {
        /// The encoded metadata length.
        len: usize,
        /// The maximum metadata length.
        max: usize,
    },
}

impl core::fmt::Display for Air11BuildError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSessionId { session_id } => {
                write!(f, "air11 session_id={session_id} is not positive")
            }
            Self::HostTimestampInvalid => f.write_str("air11 host receipt timestamp invalid"),
            Self::SourceStartTimeInvalid => f.write_str("air11 source startTime invalid"),
            Self::SourceStartTimeBackwards { previous, current } => write!(
                f,
                "air11 startTime moved backwards: previous={previous}, current={current}"
            ),
            Self::SourcePositionBackwards {
                signal,
                previous,
                current,
            } => write!(
                f,
                "air11 source position moved backwards: signal={signal:?}, \
                 previous={previous}, current={current}"
            ),
            Self::UnsupportedSignal { signal } => {
                write!(f, "air11 signal={signal:?} has no registry mapping")
            }
            Self::NonFiniteValue { signal } => {
                write!(f, "air11 signal={signal:?} converted to a non-finite value")
            }
            Self::ValueOutOfRange { signal, value } => {
                write!(
                    f,
                    "air11 signal={signal:?} value={value} outside registry range"
                )
            }
            Self::MetadataTooLarge { len, max } => {
                write!(f, "air11 metadata length={len} exceeds limit={max}")
            }
        }
    }
}

impl std::error::Error for Air11BuildError {}

/// Stateful identity and sample builder for one authenticated `AirSense`
/// connection.
#[derive(Debug)]
pub struct Air11SampleBuilder {
    session_id: i64,
    last_start_time_micros: Option<i64>,
    first_positions: [Option<i64>; AIR11_SIGNAL_SLOTS],
    last_positions: [Option<i64>; AIR11_SIGNAL_SLOTS],
}

impl Air11SampleBuilder {
    /// Starts a builder for one already-minted persistent session.
    ///
    /// # Errors
    /// Returns [`Air11BuildError::InvalidSessionId`] for zero or negative
    /// identities; the builder never creates an identity of its own.
    pub fn new(session_id: i64) -> Result<Self, Air11BuildError> {
        if session_id <= 0 {
            return Err(Air11BuildError::InvalidSessionId { session_id });
        }
        Ok(Self {
            session_id,
            last_start_time_micros: None,
            first_positions: [None; AIR11_SIGNAL_SLOTS],
            last_positions: [None; AIR11_SIGNAL_SLOTS],
        })
    }

    /// Builds all rows from one decoded stream frame.
    ///
    /// Source positions are absolute source sample ordinals: the parsed
    /// `startTime` in microseconds is advanced by the interval reported in the
    /// frame for each bounded array index, then divided by the signal's native
    /// cadence.  The stored `seq` is that ordinal relative to the first
    /// position for the signal in this session.  Repeated positions are
    /// ignored; a backwards position is an error.  State is committed only
    /// after the whole frame succeeds, so a quarantined frame cannot partially
    /// advance identity.
    ///
    /// # Errors
    /// Returns a named error for a bad host timestamp, source clock, source
    /// position, conversion, registry range, or metadata bound.
    pub fn build_frame(
        &mut self,
        frame: &StreamFrame,
        host_receipt: SystemTime,
    ) -> Result<Air11Batch, Air11BuildError> {
        let timestamp_unix_micros = host_timestamp_micros(host_receipt)?;
        let start_time_micros = parse_source_start_time(frame.source_start_time())
            .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
        if let Some(previous) = self.last_start_time_micros
            && start_time_micros < previous
        {
            return Err(Air11BuildError::SourceStartTimeBackwards {
                previous,
                current: start_time_micros,
            });
        }

        let mut first_positions = self.first_positions;
        let mut next_positions = self.last_positions;
        let mut batch = Air11Batch {
            samples: Vec::new(),
            unavailable: 0,
            duplicates: 0,
        };
        if frame.signal_count == 0 {
            batch.unavailable = 1;
        }

        let reported_interval_micros = i64::from(frame.interval_ms)
            .checked_mul(1_000)
            .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
        let context = BuildContext {
            start_time_micros,
            timestamp_unix_micros,
            reported_interval_micros,
            same_start_time: self.last_start_time_micros == Some(start_time_micros),
            session_id: self.session_id,
        };
        for signal_index in 0..usize::from(frame.signal_count) {
            build_signal(
                frame,
                signal_index,
                &context,
                &mut first_positions,
                &mut next_positions,
                &mut batch,
            )?;
        }

        self.last_start_time_micros = Some(start_time_micros);
        self.first_positions = first_positions;
        self.last_positions = next_positions;
        Ok(batch)
    }

    /// The persistent session id used by this builder.
    #[must_use]
    pub const fn session_id(&self) -> i64 {
        self.session_id
    }
}

struct BuildContext {
    start_time_micros: i64,
    timestamp_unix_micros: i64,
    reported_interval_micros: i64,
    same_start_time: bool,
    session_id: i64,
}

fn build_signal(
    frame: &StreamFrame,
    signal_index: usize,
    context: &BuildContext,
    first_positions: &mut [Option<i64>; AIR11_SIGNAL_SLOTS],
    next_positions: &mut [Option<i64>; AIR11_SIGNAL_SLOTS],
    batch: &mut Air11Batch,
) -> Result<(), Air11BuildError> {
    let signal_data = frame
        .signal(signal_index)
        .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
    let source_signal = signal_data.signal;
    let slot = signal_slot(source_signal).ok_or(Air11BuildError::UnsupportedSignal {
        signal: source_signal,
    })?;
    let native_interval_ms =
        source_signal
            .native_interval_ms()
            .ok_or(Air11BuildError::UnsupportedSignal {
                signal: source_signal,
            })?;
    let values = frame
        .values_for(signal_data)
        .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
    let (r#type, multiplier) =
        registry_mapping(source_signal).ok_or(Air11BuildError::UnsupportedSignal {
            signal: source_signal,
        })?;
    let source_name = str::from_utf8(signal_data.source_name())
        .map_err(|_| Air11BuildError::SourceStartTimeInvalid)?;
    let native_interval_micros = i64::from(native_interval_ms)
        .checked_mul(1_000)
        .ok_or(Air11BuildError::SourceStartTimeInvalid)?;

    for (value_index, value) in values.iter().enumerate() {
        let value_index =
            i64::try_from(value_index).map_err(|_| Air11BuildError::SourceStartTimeInvalid)?;
        let sample_offset = context
            .reported_interval_micros
            .checked_mul(value_index)
            .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
        let source_time_micros = context
            .start_time_micros
            .checked_add(sample_offset)
            .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
        let source_ordinal = source_time_micros
            .checked_div(native_interval_micros)
            .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
        let previous = *next_positions
            .get(slot)
            .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
        if let Some(previous) = previous {
            if source_ordinal < previous {
                if context.same_start_time {
                    batch.duplicates = batch.duplicates.saturating_add(1);
                    continue;
                }
                return Err(Air11BuildError::SourcePositionBackwards {
                    signal: source_signal,
                    previous,
                    current: source_ordinal,
                });
            }
            if source_ordinal == previous {
                batch.duplicates = batch.duplicates.saturating_add(1);
                continue;
            }
        }
        let sequence =
            record_source_position(source_ordinal, slot, first_positions, next_positions)?;
        if !value.valid {
            batch.unavailable = batch.unavailable.saturating_add(1);
            continue;
        }
        let converted = value.value * multiplier;
        if !converted.is_finite() {
            return Err(Air11BuildError::NonFiniteValue {
                signal: source_signal,
            });
        }
        let (minimum, maximum) = r#type.range();
        if converted < minimum || converted > maximum {
            return Err(Air11BuildError::ValueOutOfRange {
                signal: source_signal,
                value: converted,
            });
        }
        let extra = metadata(frame, source_name, native_interval_ms, source_ordinal)?;
        batch.samples.push(WireSample {
            source: VitalsSource::ResmedAir11.id(),
            r#type: r#type.id(),
            session_id: context.session_id,
            seq: sequence,
            timestamp_unix_micros: context.timestamp_unix_micros,
            value: converted,
            extra: Some(extra),
            batch_id: None,
        });
    }
    Ok(())
}

fn record_source_position(
    source_ordinal: i64,
    slot: usize,
    first_positions: &mut [Option<i64>; AIR11_SIGNAL_SLOTS],
    next_positions: &mut [Option<i64>; AIR11_SIGNAL_SLOTS],
) -> Result<i64, Air11BuildError> {
    let first_position = first_positions
        .get(slot)
        .copied()
        .ok_or(Air11BuildError::SourceStartTimeInvalid)?
        .unwrap_or(source_ordinal);
    let sequence = source_ordinal
        .checked_sub(first_position)
        .ok_or(Air11BuildError::SourceStartTimeInvalid)?;
    *first_positions
        .get_mut(slot)
        .ok_or(Air11BuildError::SourceStartTimeInvalid)? = Some(first_position);
    *next_positions
        .get_mut(slot)
        .ok_or(Air11BuildError::SourceStartTimeInvalid)? = Some(source_ordinal);
    Ok(sequence)
}

/// Parses the bounded source clock representation retained by `StreamData`.
///
/// It accepts the observed RFC-3339 form `YYYY-MM-DDTHH:MM:SS[.fraction]Z` and
/// numeric offsets.  Fractions are required to contain at most six digits so
/// conversion to microseconds never silently truncates precision.
#[must_use]
pub fn parse_source_start_time(input: &[u8]) -> Option<i64> {
    if input.len() < 20 {
        return None;
    }
    let year = i64::from(decimal(input, 0, 4)?);
    let month = decimal(input, 5, 2)?;
    let day = decimal(input, 8, 2)?;
    if input.get(4) != Some(&b'-') || input.get(7) != Some(&b'-') || input.get(10) != Some(&b'T') {
        return None;
    }
    let hour = decimal(input, 11, 2)?;
    let minute = decimal(input, 14, 2)?;
    let second = decimal(input, 17, 2)?;
    if hour > 23 || minute > 59 || second > 59 || year < 1970 || !valid_date(year, month, day) {
        return None;
    }
    if input.get(13) != Some(&b':') || input.get(16) != Some(&b':') {
        return None;
    }

    let mut position = 19_usize;
    let mut micros = 0_u32;
    if input.get(position) == Some(&b'.') {
        position = position.checked_add(1)?;
        let fraction_start = position;
        while input.get(position).is_some_and(u8::is_ascii_digit) {
            position = position.checked_add(1)?;
        }
        let fraction_len = position.checked_sub(fraction_start)?;
        if fraction_len == 0 || fraction_len > 6 {
            return None;
        }
        let fraction = decimal(input, fraction_start, fraction_len)?;
        let scale = 10_u32.checked_pow(u32::try_from(6 - fraction_len).ok()?)?;
        micros = fraction.checked_mul(scale)?;
    }

    let offset_seconds = match input.get(position).copied()? {
        b'Z' if position.checked_add(1)? == input.len() => 0_i64,
        b'+' | b'-' => {
            let zone_hour = decimal(input, position.checked_add(1)?, 2)?;
            let zone_minute = decimal(input, position.checked_add(4)?, 2)?;
            if zone_hour > 23
                || zone_minute > 59
                || input.get(position.checked_add(3)?) != Some(&b':')
                || position.checked_add(6)? != input.len()
            {
                return None;
            }
            let seconds = i64::from(zone_hour)
                .checked_mul(3_600)?
                .checked_add(i64::from(zone_minute).checked_mul(60)?)?;
            if input.get(position) == Some(&b'+') {
                seconds
            } else {
                seconds.checked_neg()?
            }
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day)?;
    let local_seconds = days
        .checked_mul(86_400)?
        .checked_add(i64::from(hour).checked_mul(3_600)?)?
        .checked_add(i64::from(minute).checked_mul(60)?)?
        .checked_add(i64::from(second))?;
    let utc_seconds = local_seconds.checked_sub(offset_seconds)?;
    utc_seconds
        .checked_mul(1_000_000)?
        .checked_add(i64::from(micros))
}

fn host_timestamp_micros(timestamp: SystemTime) -> Result<i64, Air11BuildError> {
    let duration = timestamp
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Air11BuildError::HostTimestampInvalid)?;
    let seconds =
        i64::try_from(duration.as_secs()).map_err(|_| Air11BuildError::HostTimestampInvalid)?;
    seconds
        .checked_mul(1_000_000)
        .and_then(|value| value.checked_add(i64::from(duration.subsec_micros())))
        .ok_or(Air11BuildError::HostTimestampInvalid)
}

fn registry_mapping(signal: StreamSignal) -> Option<(VitalsType, f64)> {
    Some(match signal {
        StreamSignal::PatientFlow => (VitalsType::PatientFlow, 1.0),
        StreamSignal::MaskPressure => (VitalsType::MaskPressure, 1.0),
        StreamSignal::MaskPressureTwoSecond => (VitalsType::MaskPressure2s, 1.0),
        StreamSignal::InspiratoryPressure => (VitalsType::InspiratoryPressure, 1.0),
        StreamSignal::ExpiratoryPressure => (VitalsType::ExpiratoryPressure, 1.0),
        StreamSignal::Leak => (VitalsType::Leak, 60.0),
        StreamSignal::RespiratoryRate => (VitalsType::RespiratoryRate, 1.0),
        StreamSignal::TidalVolume => (VitalsType::TidalVolume, 1_000.0),
        StreamSignal::MinuteVentilation => (VitalsType::MinuteVentilation, 1.0),
        StreamSignal::TargetMinuteVentilation => (VitalsType::TargetMinuteVentilation, 1.0),
        StreamSignal::IeRatio => (VitalsType::IeRatio, 1.0),
        StreamSignal::SnoreIndex => (VitalsType::SnoreIndex, 1.0),
        StreamSignal::FlowLimitation => (VitalsType::FlowLimitation, 1.0),
        StreamSignal::InspiratoryDuration => (VitalsType::InspiratoryDuration, 1.0),
        StreamSignal::TriggerCycleEvent => (VitalsType::TriggerCycleEvent, 1.0),
        StreamSignal::HeartRate => (VitalsType::Hr, 1.0),
        StreamSignal::SpO2 => (VitalsType::Spo2, 1.0),
        StreamSignal::Unknown => return None,
    })
}

fn signal_slot(signal: StreamSignal) -> Option<usize> {
    Some(match signal {
        StreamSignal::PatientFlow => 0,
        StreamSignal::MaskPressure => 1,
        StreamSignal::MaskPressureTwoSecond => 2,
        StreamSignal::InspiratoryPressure => 3,
        StreamSignal::ExpiratoryPressure => 4,
        StreamSignal::Leak => 5,
        StreamSignal::RespiratoryRate => 6,
        StreamSignal::TidalVolume => 7,
        StreamSignal::MinuteVentilation => 8,
        StreamSignal::TargetMinuteVentilation => 9,
        StreamSignal::IeRatio => 10,
        StreamSignal::SnoreIndex => 11,
        StreamSignal::FlowLimitation => 12,
        StreamSignal::InspiratoryDuration => 13,
        StreamSignal::TriggerCycleEvent => 14,
        StreamSignal::HeartRate => 15,
        StreamSignal::SpO2 => 16,
        StreamSignal::Unknown => return None,
    })
}

fn metadata(
    frame: &StreamFrame,
    source_name: &str,
    native_interval_ms: u32,
    source_ordinal: i64,
) -> Result<Vec<u8>, Air11BuildError> {
    let device_start_time = str::from_utf8(frame.source_start_time())
        .map_err(|_| Air11BuildError::SourceStartTimeInvalid)?;
    let value = json!({
        "schema": 1,
        "clock_quality": "untrusted",
        "device_start_time": device_start_time,
        "stream_id": frame.stream_id,
        "signal": source_name,
        "native_interval_ms": native_interval_ms,
        "source_ordinal": source_ordinal,
        "requested_sample_interval_ms": AIR11_SAMPLE_INTERVAL_MS,
        "reported_interval_ms": frame.interval_ms,
    });
    let bytes = serde_json::to_vec(&value).map_err(|_| Air11BuildError::MetadataTooLarge {
        len: usize::MAX,
        max: MAX_AIR11_EXTRA_BYTES,
    })?;
    if bytes.len() > MAX_AIR11_EXTRA_BYTES {
        return Err(Air11BuildError::MetadataTooLarge {
            len: bytes.len(),
            max: MAX_AIR11_EXTRA_BYTES,
        });
    }
    Ok(bytes)
}

fn decimal(input: &[u8], begin: usize, length: usize) -> Option<u32> {
    let end = begin.checked_add(length)?;
    let bytes = input.get(begin..end)?;
    if bytes.is_empty() {
        return None;
    }
    let mut value = 0_u32;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .checked_mul(10)?
            .checked_add(u32::from(*byte - b'0'))?;
    }
    Some(value)
}

fn valid_date(year: i64, month: u32, day: u32) -> bool {
    if !(1..=12).contains(&month) {
        return false;
    }
    let days = match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days).contains(&day)
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    let adjusted_year = year.checked_sub(i64::from(month <= 2))?;
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let month_i64 = i64::from(month);
    let month_prime = month_i64 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "synthetic stream fixtures are fixed assertion inputs"
)]
mod tests {
    use super::*;
    use std::time::Duration;
    use vitals_decode::stream::decode_stream;

    const START: &str = "2026-09-10T01:02:03.000Z";

    fn receipt() -> SystemTime {
        UNIX_EPOCH + Duration::from_micros(1_800_000_000_123_456)
    }

    fn frame() -> StreamFrame {
        decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":40,"startTime":"2026-09-10T01:02:03.000Z","data":[{"_RFL":[1.5,-1.0,0.0]},{"_LKF":[1.0]},{"_TD2":[0.5]},{"_HRT":[null]}]}}"#,
        )
        .expect("valid frame")
    }

    #[test]
    fn parses_source_time_and_offsets_without_normalising_bad_dates() {
        let utc = parse_source_start_time(START.as_bytes()).expect("UTC");
        let offset = parse_source_start_time(b"2026-09-10T02:02:03.000+01:00").expect("offset");
        assert_eq!(utc, offset);
        assert!(parse_source_start_time(b"2026-02-29T00:00:00Z").is_none());
        assert!(parse_source_start_time(b"2026-09-10T01:02:03.1234567Z").is_none());
        assert!(parse_source_start_time(b"1969-12-31T23:59:59Z").is_none());
    }

    #[test]
    fn builds_registry_rows_with_host_time_and_untrusted_metadata() {
        let mut builder = Air11SampleBuilder::new(42).expect("session");
        let batch = builder.build_frame(&frame(), receipt()).expect("batch");
        assert_eq!(batch.samples.len(), 5);
        assert_eq!(batch.unavailable, 1);
        assert_eq!(batch.duplicates, 0);
        let flow = batch
            .samples
            .iter()
            .find(|sample| sample.r#type == VitalsType::PatientFlow.id())
            .expect("flow");
        assert!((flow.value - 1.5).abs() < 1e-9);
        assert_eq!(flow.source, VitalsSource::ResmedAir11.id());
        assert_eq!(flow.session_id, 42);
        assert_eq!(flow.timestamp_unix_micros, 1_800_000_000_123_456);
        let extra = flow.extra.as_deref().expect("metadata");
        let metadata: serde_json::Value = serde_json::from_slice(extra).expect("metadata JSON");
        assert_eq!(metadata["clock_quality"], "untrusted");
        assert_eq!(metadata["signal"], "_RFL");
        assert_eq!(metadata["native_interval_ms"], 40);

        let leak = batch
            .samples
            .iter()
            .find(|sample| sample.r#type == VitalsType::Leak.id())
            .expect("leak");
        assert!((leak.value - 60.0).abs() < 1e-9);
        let tidal = batch
            .samples
            .iter()
            .find(|sample| sample.r#type == VitalsType::TidalVolume.id())
            .expect("tidal volume");
        assert!((tidal.value - 500.0).abs() < 1e-9);
    }

    #[test]
    fn maps_every_fixed_stream_signal_through_registry_types() {
        let frame = decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":40,"startTime":"2026-09-10T01:02:03.000Z","data":[{"_RFL":[1.0]},{"_MKP":[2.0]},{"_MKF":[3.0]},{"_MKI":[4.0]},{"_MKE":[5.0]},{"_LKF":[1.0]},{"_RR2":[12.0]},{"_TD2":[0.5]},{"_MV2":[2.0]},{"_TGT":[3.0]},{"_IE2":[1.2]},{"_SNI":[0.0]},{"_FFL":[0.0]},{"_INT":[1.0]},{"_BYV":[0.0]},{"_HRT":[60.0]},{"_SAO":[98.0]}]}}"#,
        )
        .expect("all fixed signals");
        let mut builder = Air11SampleBuilder::new(6).expect("session");
        let batch = builder.build_frame(&frame, receipt()).expect("batch");
        assert_eq!(batch.samples.len(), AIR11_SIGNAL_SLOTS);
        assert_eq!(batch.unavailable, 0);
        assert!(
            (batch
                .samples
                .iter()
                .find(|sample| sample.r#type == VitalsType::Leak.id())
                .expect("leak")
                .value
                - 60.0)
                .abs()
                < 1e-9
        );
        assert!(
            (batch
                .samples
                .iter()
                .find(|sample| sample.r#type == VitalsType::TidalVolume.id())
                .expect("tidal volume")
                .value
                - 500.0)
                .abs()
                < 1e-9
        );
        for sample in &batch.samples {
            assert_eq!(sample.source, VitalsSource::ResmedAir11.id());
            assert!(VitalsType::from_id(sample.r#type).is_some());
        }
    }

    #[test]
    fn measured_zero_is_kept_and_null_is_not_a_zero_row() {
        let mut builder = Air11SampleBuilder::new(1).expect("session");
        let batch = builder.build_frame(&frame(), receipt()).expect("batch");
        assert!(batch.samples.iter().any(|sample| {
            sample.r#type == VitalsType::PatientFlow.id() && sample.value.abs() < 1e-9
        }));
        assert!(
            !batch
                .samples
                .iter()
                .any(|sample| sample.r#type == VitalsType::Hr.id())
        );
    }

    #[test]
    fn retransmitted_source_positions_are_deduplicated() {
        let mut builder = Air11SampleBuilder::new(3).expect("session");
        let first = builder.build_frame(&frame(), receipt()).expect("first");
        let second = builder.build_frame(&frame(), receipt()).expect("duplicate");
        assert_eq!(first.samples.len(), 5);
        assert_eq!(second.samples.len(), 0);
        assert_eq!(second.duplicates, 6);
    }

    #[test]
    fn backward_positions_and_range_fail_without_a_host_sequence_fallback() {
        let mut builder = Air11SampleBuilder::new(4).expect("session");
        let first = builder.build_frame(&frame(), receipt()).expect("first");
        assert_eq!(first.samples.len(), 5);
        let backward = decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":40,"startTime":"2026-09-09T01:02:03.000Z","data":[{"_RFL":[1.0]}]}}"#,
        )
        .expect("backward fixture");
        assert!(matches!(
            builder.build_frame(&backward, receipt()),
            Err(Air11BuildError::SourceStartTimeBackwards { .. })
        ));
        let out_of_range = decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":40,"startTime":"2026-09-10T01:02:05.200Z","data":[{"_LKF":[6.0]}]}}"#,
        )
        .expect("range fixture");
        assert!(matches!(
            builder.build_frame(&out_of_range, receipt()),
            Err(Air11BuildError::ValueOutOfRange { .. })
        ));
        let good = decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":40,"startTime":"2026-09-10T01:02:03.200Z","data":[{"_RFL":[1.0]}]}}"#,
        )
        .expect("good fixture");
        let recovered = builder.build_frame(&good, receipt()).expect("good");
        assert_eq!(recovered.samples.len(), 1);
        assert_eq!(recovered.samples[0].seq, 5);
    }

    #[test]
    fn reported_frame_interval_drives_source_positions() {
        let frame = decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":40,"startTime":"2026-09-10T01:02:03.000Z","data":[{"_RFL":[1.0,2.0]}]}}"#,
        )
        .expect("observed interval frame");
        let mut builder = Air11SampleBuilder::new(5).expect("session");
        let samples = builder
            .build_frame(&frame, receipt())
            .expect("batch")
            .samples;
        let mut positions = samples.iter().map(|sample| sample.seq);
        let first = positions.next().expect("first position");
        let second = positions.next().expect("second position");
        assert_eq!(first, 0);
        assert_eq!(second - first, 1);

        let report_interval_frame = decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":200,"startTime":"2026-09-10T01:02:03.000Z","data":[{"_RFL":[1.0,2.0]}]}}"#,
        )
        .expect("report interval frame");
        let mut other_builder = Air11SampleBuilder::new(6).expect("session");
        let samples = other_builder
            .build_frame(&report_interval_frame, receipt())
            .expect("batch")
            .samples;
        let mut positions = samples.iter().map(|sample| sample.seq);
        let first = positions.next().expect("first position");
        let second = positions.next().expect("second position");
        assert_eq!(second - first, 5);
    }

    #[test]
    fn fixed_interval_and_timestamp_guards_are_named() {
        let mut builder = Air11SampleBuilder::new(5).expect("session");
        let observed_device_interval = decode_stream(
            br#"{"method":"StreamData","params":{"streamId":7,"intervalMs":40,"startTime":"2026-09-10T01:02:03.000Z","data":[]}}"#,
        )
        .expect("observed interval fixture");
        assert_eq!(
            builder
                .build_frame(&observed_device_interval, receipt())
                .expect("observed interval is accepted")
                .unavailable,
            1
        );
        assert!(matches!(
            builder.build_frame(&frame(), UNIX_EPOCH - Duration::from_secs(1)),
            Err(Air11BuildError::HostTimestampInvalid)
        ));
        assert!(matches!(
            Air11SampleBuilder::new(0),
            Err(Air11BuildError::InvalidSessionId { .. })
        ));
    }
}
