//! Portable pieces of the durable EMAY `SleepO2` collector.
//!
//! The BLE connection and `vitalsd` socket live in the Linux binary. Session
//! identity, event-time conversion, registry validation, and wire-row
//! construction live here so they can be tested without a radio or database.

use chrono::{Local, LocalResult, NaiveDate, TimeZone};
use vitals_decode::emay::{SessionDescriptor, StoredMetric, StoredRecord, storage_ordinal};
use vitals_proto::WireSample;
use vitals_registry::{VitalsSource, VitalsType};

/// The byte-level version used in the source-specific `extra` field.
const EXTRA_VERSION: u8 = 1;

/// A deterministic session identity for one device-stored recording.
///
/// The complete descriptor is the source's session identity. FNV-1a is used
/// only as a compact, deterministic encoding into the existing `i64` schema
/// field; the raw descriptor is retained in each row's `extra` so a collision
/// or future decoder change is observable and repairable. The configured
/// collector represents one physical EMAY source, so the descriptor's slot,
/// flags, start time, and metadata are sufficient to distinguish its stored
/// sessions.
#[must_use]
pub fn session_id(descriptor: &SessionDescriptor) -> i64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let header = [
        descriptor.flags,
        descriptor.slot,
        descriptor.year,
        descriptor.month,
        descriptor.day,
        descriptor.hour,
        descriptor.minute,
        descriptor.second,
    ];
    for byte in header.iter().chain(descriptor.metadata().iter()) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    i64::from_be_bytes(hash.to_be_bytes())
}

/// Converts a descriptor's device-local start time using the Pi's configured
/// local timezone and returns Unix epoch microseconds.
///
/// A local time inside a DST gap or fold is rejected rather than silently
/// choosing one occurrence. The device has no timezone field; the app sets its
/// clock from the phone's local time, so the collector's host timezone must
/// match the device timezone.
///
/// # Errors
/// Returns a named error for an invalid calendar value or an ambiguous/non-
/// existent local time.
pub fn session_start_unix_micros(descriptor: &SessionDescriptor) -> Result<i64, String> {
    session_start_unix_micros_in(descriptor, &Local)
}

fn session_start_unix_micros_in<Tz>(
    descriptor: &SessionDescriptor,
    timezone: &Tz,
) -> Result<i64, String>
where
    Tz: TimeZone,
{
    let year = i32::from(descriptor.year)
        .checked_add(2000)
        .ok_or_else(|| "EMAY session year overflowed the supported century".to_owned())?;
    let date =
        NaiveDate::from_ymd_opt(year, u32::from(descriptor.month), u32::from(descriptor.day))
            .ok_or_else(|| {
                format!(
                    "EMAY session has invalid date {:02}-{:02}-{:02}",
                    descriptor.year, descriptor.month, descriptor.day
                )
            })?;
    let naive = date
        .and_hms_opt(
            u32::from(descriptor.hour),
            u32::from(descriptor.minute),
            u32::from(descriptor.second),
        )
        .ok_or_else(|| {
            format!(
                "EMAY session has invalid time {:02}:{:02}:{:02}",
                descriptor.hour, descriptor.minute, descriptor.second
            )
        })?;
    let local = match timezone.from_local_datetime(&naive) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(_, _) => {
            return Err(format!(
                "EMAY session local time is ambiguous: {naive} — refusing to choose a DST fold"
            ));
        }
        LocalResult::None => {
            return Err(format!(
                "EMAY session local time does not exist: {naive} — it falls in a DST gap"
            ));
        }
    };
    Ok(local.timestamp_micros())
}

/// Returns the event timestamp for a stored record.
///
/// The device's page-encoded cursor is converted to the one-Hz logical ordinal
/// by the decoder. The addition and multiplication are checked so malformed
/// source metadata cannot wrap an event into a different date.
///
/// # Errors
/// Returns an error if the cursor is reserved or the timestamp arithmetic
/// overflows.
pub fn record_timestamp_unix_micros(
    descriptor: &SessionDescriptor,
    record: &StoredRecord,
) -> Result<i64, String> {
    let start = session_start_unix_micros(descriptor)?;
    let ordinal = storage_ordinal(record.cursor).ok_or_else(|| {
        format!(
            "EMAY storage cursor {:#06x} uses a reserved page bit",
            record.cursor
        )
    })?;
    let offset = i64::from(ordinal)
        .checked_mul(1_000_000)
        .ok_or_else(|| format!("EMAY storage ordinal {ordinal} overflowed microseconds"))?;
    start
        .checked_add(offset)
        .ok_or_else(|| format!("EMAY storage timestamp overflowed at ordinal {ordinal}"))
}

/// Converts a decoded stored record into its durable socket row.
///
/// `None` means the device explicitly reported no reading. It must not become
/// a zero-valued row. Values are checked against the generated registry range;
/// no value is clamped.
///
/// # Errors
/// Returns a named error for a slot mismatch, reserved cursor, or registry
/// range violation.
pub fn wire_sample(
    descriptor: &SessionDescriptor,
    record: &StoredRecord,
) -> Result<Option<WireSample>, String> {
    if record.slot != descriptor.slot {
        return Err(format!(
            "EMAY stored record slot {} disagrees with descriptor slot {}",
            record.slot, descriptor.slot
        ));
    }
    let Some(value) = record.value else {
        return Ok(None);
    };
    let value_f64 = f64::from(value);
    let (r#type, range) = match record.metric {
        StoredMetric::Spo2 => (VitalsType::Spo2, VitalsType::Spo2.range()),
        StoredMetric::Pulse => (VitalsType::Hr, VitalsType::Hr.range()),
    };
    if value_f64 < range.0 || value_f64 > range.1 {
        return Err(format!(
            "EMAY {:?} value {value} is outside registry range {}..{} {}",
            record.metric,
            range.0,
            range.1,
            r#type.unit()
        ));
    }
    let timestamp_unix_micros = record_timestamp_unix_micros(descriptor, record)?;
    let mut extra = Vec::with_capacity(2 + 16 + 8 + 11);
    extra.extend_from_slice(&[EXTRA_VERSION, record.metric.code()]);
    extra.extend_from_slice(&record.cursor.to_le_bytes());
    extra.extend_from_slice(&record.payload);
    extra.extend_from_slice(&[
        descriptor.flags,
        descriptor.slot,
        descriptor.year,
        descriptor.month,
        descriptor.day,
        descriptor.hour,
        descriptor.minute,
        descriptor.second,
    ]);
    extra.extend_from_slice(descriptor.metadata());
    Ok(Some(WireSample {
        source: VitalsSource::Emay.id(),
        r#type: r#type.id(),
        session_id: session_id(descriptor),
        seq: i64::from(record.cursor),
        timestamp_unix_micros,
        value: value_f64,
        extra: Some(extra),
        batch_id: None,
    }))
}

/// Converts a batch of records from one descriptor, preserving source order.
///
/// A metric stream must be strictly increasing in its device cursor. The page
/// encoding permits a jump at each 128-record boundary; any other regression
/// or duplicate is rejected before it can create ambiguous rows.
///
/// # Errors
/// Returns the first named source, identity, time, or registry error.
pub fn wire_samples(
    descriptor: &SessionDescriptor,
    records: &[StoredRecord],
) -> Result<Vec<WireSample>, String> {
    let mut out = Vec::with_capacity(records.len());
    let mut previous: Option<(StoredMetric, u16)> = None;
    for record in records {
        if let Some((previous_metric, previous_cursor)) = previous
            && previous_metric == record.metric
            && record.cursor <= previous_cursor
        {
            return Err(format!(
                "EMAY {:?} cursor {:#06x} is not after {:#06x}",
                record.metric, record.cursor, previous_cursor
            ));
        }
        previous = Some((record.metric, record.cursor));
        if let Some(sample) = wire_sample(descriptor, record)? {
            out.push(sample);
        }
    }
    Ok(out)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "test fixtures should fail loudly when their protocol shape is wrong"
)]
mod tests {
    use super::*;
    use chrono::FixedOffset;
    use vitals_decode::emay::{decode_session_descriptor, decode_stored_record};

    const SESSION: [u8; 21] = [
        0xEC, 0x00, 0x01, 0x01, 0x1A, 0x01, 0x0F, 0x0A, 0x28, 0x30, 0x78, 0x43, 0x00, 0x00, 0x75,
        0x73, 0x65, 0x72, 0x00, 0x00, 0x74,
    ];
    const SPO2: [u8; 24] = [
        0xED, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x5F, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x51,
    ];
    const PULSE: [u8; 24] = [
        0xED, 0x01, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x01, 0x09, 0x00,
        0x00, 0x09, 0x09, 0x00, 0x00, 0x00, 0x00, 0x10, 0x61,
    ];

    #[test]
    fn session_identity_is_deterministic_and_descriptor_sensitive() {
        let descriptor = decode_session_descriptor(&SESSION).expect("session");
        assert_eq!(session_id(&descriptor), session_id(&descriptor));
        let mut changed = SESSION;
        if let Some(metadata) = changed.get_mut(10) {
            *metadata = metadata.saturating_add(1);
        }
        let sum = changed
            .get(..20)
            .expect("descriptor payload")
            .iter()
            .fold(0u32, |total, byte| total + u32::from(*byte));
        if let Some(checksum) = changed.last_mut() {
            *checksum = u8::try_from(sum & 0x7f).expect("checksum");
        }
        let other = decode_session_descriptor(&changed).expect("other session");
        assert_ne!(session_id(&descriptor), session_id(&other));
    }

    #[test]
    fn fixed_timezone_conversion_is_explicit_and_stable() {
        let descriptor = decode_session_descriptor(&SESSION).expect("session");
        let utc = FixedOffset::east_opt(0).expect("UTC");
        let west = FixedOffset::west_opt(7 * 60 * 60).expect("PDT");
        let utc_micros = session_start_unix_micros_in(&descriptor, &utc).expect("UTC time");
        let west_micros = session_start_unix_micros_in(&descriptor, &west).expect("west time");
        assert_eq!(west_micros - utc_micros, 7 * 60 * 60 * 1_000_000);
    }

    #[test]
    fn wire_rows_use_registry_identity_and_device_cursor() {
        let descriptor = decode_session_descriptor(&SESSION).expect("session");
        let spo2 = decode_stored_record(&SPO2).expect("SpO2");
        let pulse = decode_stored_record(&PULSE).expect("pulse");
        let rows = wire_samples(&descriptor, &[spo2, pulse]).expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.first().map(|row| row.source),
            Some(VitalsSource::Emay.id())
        );
        assert_eq!(
            rows.first().map(|row| row.r#type),
            Some(VitalsType::Spo2.id())
        );
        assert_eq!(rows.get(1).map(|row| row.r#type), Some(VitalsType::Hr.id()));
        assert_eq!(rows.first().map(|row| row.seq), Some(0));
        assert_eq!(rows.first().map(|row| row.value), Some(95.0));
        assert!(rows.first().and_then(|row| row.extra.as_ref()).is_some());
    }

    #[test]
    fn absent_storage_value_produces_no_numeric_row() {
        let descriptor = decode_session_descriptor(&SESSION).expect("session");
        let mut absent = SPO2;
        if let Some(value) = absent.get_mut(9) {
            *value = 0x7f;
        }
        let sum = absent
            .get(..23)
            .expect("payload")
            .iter()
            .fold(0u32, |total, byte| total + u32::from(*byte));
        if let Some(last) = absent.last_mut() {
            *last = u8::try_from(sum & 0x7f).expect("checksum");
        }
        let record = decode_stored_record(&absent).expect("absent record");
        assert_eq!(wire_sample(&descriptor, &record).expect("absence"), None);
    }

    #[test]
    fn registry_out_of_range_is_named_and_not_clamped() {
        let descriptor = decode_session_descriptor(&SESSION).expect("session");
        let mut low = PULSE;
        if let Some(value) = low.get_mut(9) {
            *value = 1;
        }
        let sum = low
            .get(..23)
            .expect("payload")
            .iter()
            .fold(0u32, |total, byte| total + u32::from(*byte));
        if let Some(last) = low.last_mut() {
            *last = u8::try_from(sum & 0x7f).expect("checksum");
        }
        let record = decode_stored_record(&low).expect("low pulse");
        let error = wire_sample(&descriptor, &record).expect_err("must reject");
        assert!(error.contains("outside registry range"), "{error}");
        assert!(error.contains('1'), "{error}");
    }

    #[test]
    fn cursor_regression_is_rejected_per_metric() {
        let descriptor = decode_session_descriptor(&SESSION).expect("session");
        let first = decode_stored_record(&SPO2).expect("first");
        let mut second = SPO2;
        if let Some(cursor) = second.get_mut(5) {
            *cursor = 0;
        }
        if let Some(cursor) = second.get_mut(6) {
            *cursor = 0;
        }
        if let Some(value) = second.get_mut(9) {
            *value = 94;
        }
        let sum = second
            .get(..23)
            .expect("payload")
            .iter()
            .fold(0u32, |total, byte| total + u32::from(*byte));
        if let Some(last) = second.last_mut() {
            *last = u8::try_from(sum & 0x7f).expect("checksum");
        }
        let duplicate = decode_stored_record(&second).expect("second");
        let error = wire_samples(&descriptor, &[first, duplicate]).expect_err("duplicate");
        assert!(error.contains("is not after"), "{error}");
    }
}
