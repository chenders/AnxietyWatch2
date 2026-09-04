//! EMAY `SleepO2` real-time BLE protocol.
//!
//! Reverse-engineered and verified against the device on 2026-07-09. No public
//! decoder for the live protocol existed; this was derived from the vendor
//! Android app plus a captured iOS trace.
//!
//! Vendor-serial over `0xFF12`: write `0xFF01`, notify `0xFF02`. The device
//! connects with **no pairing, no bonding and no encryption**, which is why it
//! cannot hit the SMP Security-Request failure that blocks a bonded reconnect on
//! kernel 6.12.x (amendment 06 §3) — there is no bond to resume.
//!
//! The official app's stored-session mode uses the same transport. `EC`
//! descriptors identify device sessions and `ED` records carry the metric,
//! storage slot, and source-provided cursor used for durable identity. The live
//! `EB` notification remains anonymous and is deliberately kept separate.
//!
//! # The checksum gotcha, and why testing cannot catch it
//!
//! `checksum = sum(payload) & 0x7F` — masked with **0x7F, not 0xFF**.
//!
//! The two masks agree whenever bit 7 of the payload sum is clear, which for
//! this frame layout means they agree across most of the *waking* range and
//! disagree across **43% of the physiologically possible space** (4998 of 11526
//! PR×SpO₂ combinations). The disagreement region is the one that matters:
//!
//! ```text
//! PR 40-48 · SpO₂ 95-100   masks DISAGREE   <- sleeping
//! PR 70    · SpO₂ 97       masks agree      <- awake, sitting up
//! ```
//!
//! So an `0xFF`-masked decoder works perfectly during setup and testing, then
//! **begins silently rejecting frames as the wearer falls asleep and their
//! heart rate drops into the 40s** — failing precisely when the device is doing
//! the job it is worn for.
//!
//! This was nearly missed twice. The single reference frame in the protocol
//! notes and a live 27-frame capture on
//! 2026-08-03 both fall in the agreeing region and verify
//! identically under either mask. **`0x7F` is correct because it came from
//! decompiling the vendor application, not because any captured frame
//! demonstrated it** — and no frame captured from an awake subject ever will.
//!
//! # Why `SpO₂` absence must be absence
//!
//! This is a **primary** signal: amendment 05 §2.1 allows only `SpO₂` and
//! respiratory rate to raise a tier past watch or to clear a raised one. A
//! no-finger reading coerced to a number would be a primary signal fabricated
//! out of nothing, which is the single worst input this system can receive. So
//! out-of-range values are absent, never clamped and never zero.

/// The only physically impossible `SpO₂` value: saturation cannot exceed 100%.
///
/// **There is deliberately no lower bound, and no pulse range at all.**
///
/// An earlier version of this decoder used 50–100% for `SpO₂` and 25–250 bpm
/// for pulse, invented from a summary note. A published spec for this device
/// suggests 30–220 bpm. Both are wrong for *this* application, and the shipped
/// Swift implementation (`EMAYRealtimeService.swift`) says so explicitly:
///
/// > A plausibility *range* is deliberately NOT used: narrowing it would
/// > [drop genuine extremes].
///
/// The asymmetry is what settles it. A pulse of 25 bpm or an `SpO₂` of 45% is
/// **exactly the event this system exists to detect** — profound bradycardia
/// and severe desaturation during CNS depression. Discarding one as
/// "implausible" is silent and permanent. Accepting a corrupted byte is neither:
/// it must still pass the checksum, the fixed header and the fixed trailer, and
/// anything that survives all three is caught downstream by cross-signal
/// agreement.
///
/// Filtering here would trade a visible, recoverable error for an invisible,
/// unrecoverable one, on the single signal the alarm acts on.
const SPO2_MAX: u8 = 100;

/// Sentinel bytes meaning "no finger" in either field.
///
/// Checked **before** anything else: `0x00` is always "no reading", never a
/// genuine 0%.
const SENTINELS: [u8; 2] = [0x00, 0xFF];

/// Length of a real-time data frame.
pub const FRAME_LEN: usize = 8;

/// Opcode of a real-time data notification.
const OP_DATA: u8 = 0xEB;

/// One real-time reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// `SpO₂` in percent, or `None` when the device reported no finger.
    pub spo2: Option<u8>,
    /// Pulse rate in bpm, or `None` when the device reported no finger.
    pub pulse_bpm: Option<u8>,
}

impl Reading {
    /// Whether either field carries a usable measurement.
    ///
    /// A frame with neither is a valid *frame* reporting no *reading* — the
    /// device is connected and saying "no finger", which is information, not an
    /// error, and it is still evidence the link is alive.
    #[must_use]
    pub const fn is_measurement(&self) -> bool {
        self.spo2.is_some() || self.pulse_bpm.is_some()
    }
}

/// Verifies the trailing checksum.
///
/// The mask is `0x7F`. Using `0xFF` compares a byte the device never sets.
#[must_use]
pub fn checksum_ok(frame: &[u8]) -> bool {
    let Some((&given, payload)) = frame.split_last() else {
        return false;
    };
    if payload.is_empty() {
        return false;
    }
    let sum = payload.iter().fold(0u32, |a, b| a + u32::from(*b));
    // try_from rather than `as`: the mask makes the value provably small, and
    // saying so with a fallible conversion needs no #[expect] that a later edit
    // could invalidate.
    u8::try_from(sum & 0x7F).is_ok_and(|want| given == want)
}

/// Decodes a real-time data notification.
///
/// Layout: `EB 01 05 <PR> <SPO2> 7F 00 <cks>`, exactly 8 bytes. The device
/// also emits the same-sized `EB 01 07 7F 7F 7F 00 70` transition frame while
/// moving to or from its no-reading state. That frame is valid link evidence,
/// but it carries no measurement and must not be turned into a numeric row.
///
/// Validates length, header, trailer (`7F 00`) and checksum before reading any
/// value. Genuine extremes are preserved — only the sentinels and a physically
/// impossible `SpO₂` above 100% are dropped. See [`SPO2_MAX`] for why there is
/// no lower bound.
#[must_use]
pub fn decode_data(frame: &[u8]) -> Option<Reading> {
    if frame.len() != FRAME_LEN {
        return None;
    }
    // Header and trailer are fixed. Checking them rejects a misaligned read
    // that the checksum alone might let through.
    if *frame.first()? != OP_DATA || *frame.get(1)? != 0x01 {
        return None;
    }
    let layout = *frame.get(2)?;
    if layout != 0x05 && layout != 0x07 {
        return None;
    }
    if *frame.get(5)? != 0x7F || *frame.get(6)? != 0x00 {
        return None;
    }
    if !checksum_ok(frame) {
        return None;
    }
    // `0x07` is an observed transition format, not a second measurement
    // layout. Accept only its exact all-sentinel payload so an unknown variant
    // cannot smuggle values into the live path.
    if layout == 0x07 {
        if !frame.get(3..6)?.iter().all(|byte| *byte == 0x7F) {
            return None;
        }
        return Some(Reading {
            spo2: None,
            pulse_bpm: None,
        });
    }
    Some(Reading {
        // Pulse: sentinel check only. No range — see SPO2_MAX's note.
        pulse_bpm: sentinel_or(*frame.get(3)?),
        // SpO2: sentinel, then the one genuine impossibility.
        spo2: sentinel_or(*frame.get(4)?).filter(|v| *v <= SPO2_MAX),
    })
}

/// `None` for a no-finger sentinel, otherwise the value itself.
///
/// Never clamped: a clamped physiological value is indistinguishable from a
/// real one downstream.
const fn sentinel_or(v: u8) -> Option<u8> {
    if v == SENTINELS[0] || v == SENTINELS[1] {
        None
    } else {
        Some(v)
    }
}

/// Finds the next valid data frame in a byte stream.
///
/// Returns the decoded reading and how many bytes to consume, or `None` if no
/// complete frame is present yet.
///
/// # Why a scanner and not a length check
///
/// BLE notifications are **not** frame-aligned. The device emits 8-byte
/// records, but the transport delivers whatever fits the negotiated MTU: at the
/// 23-byte default each notification carries one record, and at a larger MTU
/// they arrive coalesced, with records straddling notification boundaries.
///
/// Observed live on 2026-08-03 — one connection delivered exactly one record
/// per notification and looked perfect, while the next delivered 20-byte
/// notifications carrying 2.5 records each. A decoder that requires exactly 8
/// bytes silently discarded about 45% of that second run, and **which
/// behaviour you get depends on the MTU the connection happens to negotiate**.
/// A single successful test run cannot distinguish the two.
///
/// The frame is self-synchronising: fixed length, known opcode, and a checksum.
/// So on a mismatch this advances one byte and retries rather than discarding
/// the buffer — a partial record at the head must not eat the good records
/// behind it.
#[must_use]
pub fn scan_frame(buf: &[u8]) -> Option<(Reading, usize)> {
    let mut i = 0usize;
    while let Some(window) = buf.get(i..i + FRAME_LEN) {
        if let Some(r) = decode_data(window) {
            return Some((r, i + FRAME_LEN));
        }
        i += 1;
    }
    None
}

/// Length of an `EC` stored-session descriptor, excluding the trailing stream
/// terminator byte.
pub const SESSION_DESCRIPTOR_LEN: usize = 21;

/// Length of one `ED` stored-session record.
pub const STORED_RECORD_LEN: usize = 24;

const OP_SESSION_DESCRIPTOR: u8 = 0xEC;
const OP_STORED_RECORD: u8 = 0xED;

/// The two metric streams in a stored session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredMetric {
    /// Oxygen saturation, in percent.
    Spo2,
    /// Pulse rate, in beats per minute.
    Pulse,
}

impl StoredMetric {
    /// The metric selector used by the device's storage command and record.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Spo2 => 1,
            Self::Pulse => 2,
        }
    }
}

/// The session identity and device-local start time returned by `EC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionDescriptor {
    /// Device status flags echoed in the descriptor.
    pub flags: u8,
    /// Device storage slot. The observed device exposes slots 1 through 7.
    pub slot: u8,
    /// Two-digit year, interpreted by the collector's configured century.
    pub year: u8,
    /// Month, 1 through 12.
    pub month: u8,
    /// Day of month, 1 through 31.
    pub day: u8,
    /// Hour, 0 through 23, in the device's local clock.
    pub hour: u8,
    /// Minute, 0 through 59, in the device's local clock.
    pub minute: u8,
    /// Second, 0 through 59, in the device's local clock.
    pub second: u8,
    /// Uninterpreted descriptor bytes, retained for deterministic identity.
    metadata: [u8; 11],
}

impl SessionDescriptor {
    /// The descriptor metadata after the six date/time bytes.
    #[must_use]
    pub const fn metadata(&self) -> &[u8; 11] {
        &self.metadata
    }
}

/// One source-provided record from an EMAY stored session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredRecord {
    /// Whether the value is `SpO₂` or pulse.
    pub metric: StoredMetric,
    /// Device storage slot echoed by the record.
    pub slot: u8,
    /// Device storage cursor. This is the durable `seq` input, not arrival
    /// order. The low byte's bit 7 is reserved by the device's page layout.
    pub cursor: u16,
    /// The metric value, or `None` for a stored no-reading sentinel.
    pub value: Option<u8>,
    /// The sixteen uninterpreted record payload bytes, including `value` at
    /// payload offset 2. Keeping these allows provenance in `extra` without
    /// reparsing the radio frame.
    pub payload: [u8; 16],
}

/// Maps the device's page-encoded cursor to its logical one-Hz ordinal.
///
/// The device emits cursor values `0..0x007f`, then `0x0100..0x017f`, and so
/// on. Bit 7 of the low byte is a reserved page marker and is not a sample.
/// The raw cursor remains the identity; this mapping is only for event-time
/// reconstruction and gap accounting.
#[must_use]
pub fn storage_ordinal(cursor: u16) -> Option<u32> {
    if cursor & 0x0080 != 0 {
        return None;
    }
    let page = u32::from(cursor >> 8);
    let offset = u32::from(cursor & 0x007f);
    Some(page * 128 + offset)
}

/// Returns the source cursor immediately after a valid stored cursor.
///
/// The low-byte page marker is reserved, so `0x007f` is followed by `0x0100`,
/// not by the invalid raw value `0x0080`. `None` means the cursor is reserved
/// or the representable device cursor space is exhausted.
#[must_use]
pub fn next_storage_cursor(cursor: u16) -> Option<u16> {
    let ordinal = storage_ordinal(cursor)?.checked_add(1)?;
    let page = ordinal / 128;
    let offset = ordinal % 128;
    let raw = page.checked_mul(0x100)?.checked_add(offset)?;
    u16::try_from(raw).ok()
}

/// Recognizes the device's end-of-session-list marker.
///
/// An empty slot is returned as a checksummed `EC` frame with selector and slot
/// zero and no descriptor payload. It is a protocol marker, not a session and
/// must never become a durable row.
#[must_use]
pub fn is_empty_session_descriptor(frame: &[u8]) -> bool {
    let Some(payload) = frame.get(4..20) else {
        return false;
    };
    frame.len() == SESSION_DESCRIPTOR_LEN
        && frame.first() == Some(&OP_SESSION_DESCRIPTOR)
        && frame.get(2) == Some(&0)
        && frame.get(3) == Some(&0)
        && payload.iter().all(|byte| *byte == 0)
        && checksum_ok(frame)
}

/// Decodes one `EC` session descriptor.
///
/// The BLE stream appends one zero terminator after the 21-byte descriptor;
/// that terminator is framing, not part of this function's input.
#[must_use]
pub fn decode_session_descriptor(frame: &[u8]) -> Option<SessionDescriptor> {
    if frame.len() != SESSION_DESCRIPTOR_LEN {
        return None;
    }
    if *frame.first()? != OP_SESSION_DESCRIPTOR || *frame.get(2)? != 0x01 || !checksum_ok(frame) {
        return None;
    }
    let flags = *frame.get(1)?;
    let slot = *frame.get(3)?;
    if !(1..=7).contains(&slot) {
        return None;
    }
    let year = *frame.get(4)?;
    let month = *frame.get(5)?;
    let day = *frame.get(6)?;
    let hour = *frame.get(7)?;
    let minute = *frame.get(8)?;
    let second = *frame.get(9)?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let mut metadata = [0u8; 11];
    for (i, byte) in metadata.iter_mut().enumerate() {
        *byte = *frame.get(10 + i)?;
    }
    Some(SessionDescriptor {
        flags,
        slot,
        year,
        month,
        day,
        hour,
        minute,
        second,
        metadata,
    })
}

/// Decodes one `ED` stored-session metric record.
///
/// The record checksum uses the same `sum & 0x7f` algorithm as the live
/// protocol. `0x00`, `0x7f`, and `0xff` are absent values; `0x7f` is the
#[must_use]
pub fn decode_stored_record(frame: &[u8]) -> Option<StoredRecord> {
    if frame.len() != STORED_RECORD_LEN {
        return None;
    }
    if *frame.first()? != OP_STORED_RECORD || *frame.get(1)? != 0x01 || *frame.get(3)? != 0x01 {
        return None;
    }
    let metric = match *frame.get(2)? {
        0x01 => StoredMetric::Spo2,
        0x02 => StoredMetric::Pulse,
        _ => return None,
    };
    let slot = *frame.get(4)?;
    if !(1..=7).contains(&slot) {
        return None;
    }
    let cursor = u16::from_le_bytes([*frame.get(5)?, *frame.get(6)?]);
    if storage_ordinal(cursor).is_none() || !checksum_ok(frame) {
        return None;
    }
    let mut payload = [0u8; 16];
    for (i, byte) in payload.iter_mut().enumerate() {
        *byte = *frame.get(7 + i)?;
    }
    let value = match *frame.get(9)? {
        0x00 | 0x7f | 0xff => None,
        v => Some(v),
    };
    if matches!(metric, StoredMetric::Spo2) && value.is_some_and(|v| v > SPO2_MAX) {
        return None;
    }
    Some(StoredRecord {
        metric,
        slot,
        cursor,
        value,
        payload,
    })
}

/// Commands and framing helpers for the official app's stored-session mode.
pub mod storage {
    use super::StoredMetric;

    /// Set the device-local clock before reading stored session timestamps.
    /// The six time fields are decimal bytes, as observed in the official app.
    #[must_use]
    pub fn clock_command(
        year: u8,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Option<[u8; 10]> {
        if !(1..=12).contains(&month)
            || !(1..=31).contains(&day)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return None;
        }
        let mut frame = [0x83, year, month, day, hour, minute, second, 0, 0, 0];
        let sum = frame
            .get(..9)?
            .iter()
            .fold(0u32, |total, byte| total + u32::from(*byte));
        *frame.last_mut()? = u8::try_from(sum & 0x7f).ok()?;
        Some(frame)
    }

    /// Enter stored-session mode after the device clock has been set.
    pub const PREPARE: [u8; 3] = [0x90, 0x06, 0x16];
    /// Ask for the next stored-session descriptor. The device answers with an
    /// `EC` descriptor followed by a zero stream terminator.
    pub const LIST_NEXT: [u8; 3] = [0x9c, 0x01, 0x1d];
    /// Leave stored-session mode after a download.
    pub const EXIT: [u8; 8] = [0x9d, 0x7f, 0x7f, 0x7f, 0x7f, 0x00, 0x00, 0x19];

    /// Builds a metric download command beginning at a device cursor.
    ///
    /// The command's cursor is a source cursor, so callers may resume a
    /// partial transfer without inventing a host position.
    #[must_use]
    pub fn read_command(metric: StoredMetric, slot: u8, cursor: u16) -> Option<[u8; 8]> {
        if !(1..=7).contains(&slot) || super::storage_ordinal(cursor).is_none() {
            return None;
        }
        let mut frame = [0u8; 8];
        *frame.get_mut(0)? = 0x9d;
        *frame.get_mut(1)? = 0x01;
        *frame.get_mut(2)? = metric.code();
        *frame.get_mut(3)? = 0x01;
        *frame.get_mut(4)? = slot;
        let [lo, hi] = cursor.to_le_bytes();
        *frame.get_mut(5)? = lo;
        *frame.get_mut(6)? = hi;
        let sum = frame
            .get(..7)?
            .iter()
            .fold(0u32, |total, byte| total + u32::from(*byte));
        *frame.last_mut()? = u8::try_from(sum & 0x7f).ok()?;
        Some(frame)
    }
}

/// Commands the host writes to `0xFF01`.
///
/// Kept here beside the decoder so the protocol lives in one file rather than
/// being split between a spec document and a binary nobody can test.
pub mod cmd {
    /// Hello. Device replies with a `0xF9` frame.
    pub const HELLO: [u8; 2] = [0x89, 0x09];
    /// Device state. Replies `0xFE`.
    pub const DEVICE_STATE: [u8; 3] = [0x8E, 0x05, 0x13];
    /// Start the real-time stream. Data begins immediately on `0xFF02`.
    pub const START_REALTIME: [u8; 3] = [0x9B, 0x01, 0x1C];
    /// Battery level. Replies `0xF6 <batt> <cks>`.
    pub const BATTERY: [u8; 2] = [0x86, 0x06];
    /// Keepalive. **Must be resent about every 1.5 s** or the stream stops.
    pub const KEEPALIVE: [u8; 2] = [0x9A, 0x1A];
    /// Stop the real-time stream, before disconnecting.
    pub const STOP_REALTIME: [u8; 3] = [0x9B, 0x7F, 0x1A];
}

/// A response opcode is its command opcode **XOR** `0x70`.
///
/// Not addition. For every `0x8x` command the two agree — `0x89 + 0x70` and
/// `0x89 ^ 0x70` are both `0xF9` — so a note derived from those alone records
/// "+ 0x70" and is wrong exactly once: `0x9B + 0x70` wraps to `0x0B`, while the
/// device answers `0xEB`. `0x9B` is `START_REALTIME`, the command whose reply
/// carries every reading.
#[must_use]
pub const fn response_opcode(command: u8) -> u8 {
    command ^ 0x70
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "a failing assertion in a test should panic loudly"
)]
mod tests {
    use super::*;

    /// The device's frame layout with substituted readings: PR 70, `SpO2` 97.
    const DEVICE_LAYOUT_FRAME: [u8; 8] = [0xEB, 0x01, 0x05, 0x46, 0x61, 0x7F, 0x00, 0x17];

    /// Captured from the device during a no-reading transition.
    const NO_READING_TRANSITION: [u8; 8] = [0xEB, 0x01, 0x07, 0x7F, 0x7F, 0x7F, 0x00, 0x70];
    /// A transition-shaped frame carrying a value must not become a reading.
    const INVALID_TRANSITION_VALUE: [u8; 8] = [0xEB, 0x01, 0x07, 0x01, 0x7F, 0x7F, 0x00, 0x72];

    /// One `EC` descriptor in the official app's storage-download layout; date substituted.
    /// The stream's trailing zero terminator is intentionally not included.
    const STORED_SESSION: [u8; 21] = [
        0xEC, 0x00, 0x01, 0x01, 0x1A, 0x01, 0x0F, 0x0A, 0x28, 0x30, 0x78, 0x43, 0x00, 0x00, 0x75,
        0x73, 0x65, 0x72, 0x00, 0x00, 0x74,
    ];

    /// First `SpO₂` `ED` record for storage slot 1.
    /// Empty-session marker returned after the last available slot.
    const EMPTY_SESSION: [u8; 21] = [
        0xEC, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x2C,
    ];

    const STORED_SPO2: [u8; 24] = [
        0xED, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x5F, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x51,
    ];

    /// First pulse `ED` record paired with [`STORED_SPO2`].
    const STORED_PULSE: [u8; 24] = [
        0xED, 0x01, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x43, 0x00, 0x00, 0x01, 0x09, 0x00,
        0x00, 0x09, 0x09, 0x00, 0x00, 0x00, 0x00, 0x10, 0x61,
    ];

    /// Final no-reading record from slot 1. Its cursor is page 2, offset 0x42.
    const STORED_ABSENT: [u8; 24] = [
        0xED, 0x01, 0x01, 0x01, 0x01, 0x42, 0x02, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F,
        0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x25,
    ];

    #[test]
    fn decodes_a_stored_session_descriptor() {
        let d = decode_session_descriptor(&STORED_SESSION).expect("descriptor");
        assert_eq!(d.flags, 0);
        assert_eq!(d.slot, 1);
        assert_eq!((d.year, d.month, d.day), (0x1A, 7, 9));
        assert_eq!((d.hour, d.minute, d.second), (3, 0x29, 0x36));
        assert_eq!(d.metadata().first(), Some(&0x78));
        assert_eq!(d.metadata().get(10), Some(&0x74));
    }

    #[test]
    fn recognizes_the_empty_session_list_marker() {
        assert!(is_empty_session_descriptor(&EMPTY_SESSION));
        assert!(decode_session_descriptor(&EMPTY_SESSION).is_none());
        let mut corrupt = EMPTY_SESSION;
        if let Some(last) = corrupt.last_mut() {
            *last ^= 1;
        }
        assert!(!is_empty_session_descriptor(&corrupt));
    }

    #[test]
    fn decodes_paired_stored_metric_records() {
        let spo2 = decode_stored_record(&STORED_SPO2).expect("SpO2 record");
        assert_eq!(spo2.metric, StoredMetric::Spo2);
        assert_eq!(spo2.slot, 1);
        assert_eq!(spo2.cursor, 0);
        assert_eq!(storage_ordinal(spo2.cursor), Some(0));
        assert_eq!(spo2.value, Some(95));
        assert_eq!(spo2.payload.get(2), Some(&95));

        let pulse = decode_stored_record(&STORED_PULSE).expect("pulse record");
        assert_eq!(pulse.metric, StoredMetric::Pulse);
        assert_eq!(pulse.slot, spo2.slot);
        assert_eq!(pulse.cursor, spo2.cursor);
        assert_eq!(pulse.value, Some(67));
    }

    #[test]
    fn stored_page_cursor_maps_to_logical_ordinal() {
        assert_eq!(storage_ordinal(0x007F), Some(127));
        assert_eq!(storage_ordinal(0x0100), Some(128));
        assert_eq!(storage_ordinal(0x0242), Some(322));
        assert_eq!(storage_ordinal(0x0080), None);
        assert_eq!(next_storage_cursor(0x007f), Some(0x0100));
        assert_eq!(next_storage_cursor(0x0100), Some(0x0101));
        assert_eq!(next_storage_cursor(0x0080), None);
        assert_eq!(next_storage_cursor(0xff7f), None);
        let r = decode_stored_record(&STORED_ABSENT).expect("absent record");
        assert_eq!(r.value, None);
        assert_eq!(storage_ordinal(r.cursor), Some(322));
    }

    #[test]
    fn storage_clock_command_matches_the_captured_app_command() {
        assert_eq!(
            storage::clock_command(0x1a, 9, 4, 4, 9, 0x12),
            Some([0x83, 0x1a, 0x09, 0x04, 0x04, 0x09, 0x12, 0x00, 0x00, 0x49])
        );
        assert_eq!(storage::clock_command(0x1a, 0, 4, 4, 9, 0x12), None);
        assert_eq!(storage::clock_command(0x1a, 9, 4, 4, 60, 0x12), None);
    }

    #[test]
    fn storage_commands_use_the_device_checksum() {
        assert_eq!(
            storage::read_command(StoredMetric::Spo2, 1, 0),
            Some([0x9D, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x21])
        );
        assert_eq!(
            storage::read_command(StoredMetric::Pulse, 1, 0),
            Some([0x9D, 0x01, 0x02, 0x01, 0x01, 0x00, 0x00, 0x22])
        );
        assert_eq!(storage::read_command(StoredMetric::Spo2, 0, 0), None);
        assert_eq!(storage::read_command(StoredMetric::Spo2, 1, 0x0080), None);
    }

    #[test]
    fn malformed_stored_records_and_descriptors_are_rejected() {
        assert!(decode_session_descriptor(STORED_SESSION.get(..20).expect("short")).is_none());
        let mut bad_descriptor = STORED_SESSION;
        if let Some(second) = bad_descriptor.get_mut(8) {
            *second = 60;
        }
        assert!(decode_session_descriptor(&bad_descriptor).is_none());
        let mut bad_descriptor_checksum = STORED_SESSION;
        if let Some(last) = bad_descriptor_checksum.last_mut() {
            *last ^= 1;
        }
        assert!(decode_session_descriptor(&bad_descriptor_checksum).is_none());

        let mut bad_record = STORED_SPO2;
        if let Some(last) = bad_record.last_mut() {
            *last ^= 1;
        }
        assert!(decode_stored_record(&bad_record).is_none());
        assert!(decode_stored_record(STORED_SPO2.get(..23).expect("short")).is_none());
    }

    #[test]
    fn stored_spo2_above_physical_maximum_is_rejected() {
        let mut frame = STORED_SPO2;
        if let Some(value) = frame.get_mut(9) {
            *value = 101;
        }
        let sum = frame
            .get(..23)
            .expect("payload")
            .iter()
            .fold(0u32, |total, byte| total + u32::from(*byte));
        if let Some(last) = frame.last_mut() {
            *last = u8::try_from(sum & 0x7F).expect("masked checksum");
        }
        assert!(decode_stored_record(&frame).is_none());
    }

    #[test]
    fn decodes_a_frame_in_the_device_layout() {
        let r = decode_data(&DEVICE_LAYOUT_FRAME).expect("decodes");
        assert_eq!(r.pulse_bpm, Some(70));
        assert_eq!(r.spo2, Some(97));
        assert!(r.is_measurement());
    }

    #[test]
    fn recognizes_the_no_reading_transition_frame() {
        let r = decode_data(&NO_READING_TRANSITION).expect("transition frame");
        assert_eq!(r.spo2, None);
        assert_eq!(r.pulse_bpm, None);
        assert!(!r.is_measurement());
        let (scanned, used) = scan_frame(&NO_READING_TRANSITION).expect("scanned transition");
        assert_eq!(scanned, r);
        assert_eq!(used, FRAME_LEN);
        assert!(decode_data(&INVALID_TRANSITION_VALUE).is_none());
    }

    /// The gotcha this decoder exists to get right. With a 0xFF mask the
    /// captured frame's checksum would not verify at all.
    #[test]
    fn checksum_is_masked_with_0x7f_not_0xff() {
        assert!(checksum_ok(&DEVICE_LAYOUT_FRAME));

        // The captured frame cannot demonstrate the difference: its payload
        // sums to 0x217, and 0x217 & 0x7F == 0x217 & 0xFF == 0x17. The masks
        // only diverge when bit 7 of the sum is set, which is why a decoder
        // written with 0xFF can pass a hand-check against one real frame and
        // still be wrong.
        let diverging = [0xEB, 0x01, 0x05, 0x3E, 50, 0x7F, 0x00, 0x60];
        let sum: u32 = diverging
            .get(..7)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        assert_eq!(sum, 0x1E0);
        assert_eq!(sum & 0x7F, 0x60, "the device's mask");
        assert_eq!(
            sum & 0xFF,
            0xE0,
            "an 0xFF mask would compute a different byte"
        );
        assert!(checksum_ok(&diverging), "0x7F is the correct mask");

        let mut wrong = diverging;
        if let Some(last) = wrong.last_mut() {
            *last = 0xE0; // what an 0xFF-masked encoder would emit
        }
        assert!(!checksum_ok(&wrong), "an 0xFF-masked frame must not verify");
    }

    #[test]
    fn a_corrupt_checksum_is_rejected() {
        let mut bad = DEVICE_LAYOUT_FRAME;
        if let Some(last) = bad.last_mut() {
            *last ^= 0x01;
        }
        assert!(decode_data(&bad).is_none());
    }

    #[test]
    fn a_corrupt_payload_is_rejected() {
        let mut bad = DEVICE_LAYOUT_FRAME;
        if let Some(b) = bad.get_mut(4) {
            *b = 0x60; // SpO2 96 without fixing the checksum
        }
        assert!(decode_data(&bad).is_none());
    }

    /// No-finger and fault values must be absent, never coerced. This is a
    /// primary signal: a fabricated `SpO2` could clear a raised alarm tier.
    #[test]
    fn out_of_range_values_are_absent_not_clamped() {
        // 0xFF in both fields, checksum recomputed so the frame itself is valid.
        let mut f = [0xEB, 0x01, 0x05, 0xFF, 0xFF, 0x7F, 0x00, 0x00];
        let sum: u32 = f
            .get(..7)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        if let Some(last) = f.last_mut() {
            *last = u8::try_from(sum & 0x7F).unwrap_or(0);
        }
        let r = decode_data(&f).expect("the frame is well-formed");
        assert_eq!(r.spo2, None, "0xFF must not become 255%");
        assert_eq!(r.pulse_bpm, None);
        assert!(!r.is_measurement(), "a valid frame reporting no reading");
    }

    #[test]
    fn zero_is_absent_too() {
        let mut f = [0xEB, 0x01, 0x05, 0x00, 0x00, 0x7F, 0x00, 0x00];
        let sum: u32 = f
            .get(..7)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        if let Some(last) = f.last_mut() {
            *last = u8::try_from(sum & 0x7F).unwrap_or(0);
        }
        let r = decode_data(&f).expect("well-formed");
        assert_eq!(r.spo2, None, "0% SpO2 is a fault code, not a measurement");
        assert_eq!(r.pulse_bpm, None);
    }

    /// Build a well-formed frame with the given values.
    fn frame(pr: u8, spo2: u8) -> [u8; 8] {
        let mut f = [0xEB, 0x01, 0x05, pr, spo2, 0x7F, 0x00, 0x00];
        let sum: u32 = f
            .get(..7)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        if let Some(last) = f.last_mut() {
            *last = u8::try_from(sum & 0x7F).unwrap_or(0);
        }
        f
    }

    /// **The bug this range exists to prevent.** An earlier version used a 50%
    /// `SpO₂` floor, invented from a summary note. A desaturation to 45% — a
    /// severe, alarm-worthy event — would have been silently discarded as "out
    /// of range", on a monitor whose entire purpose is detecting desaturation.
    #[test]
    fn a_severe_desaturation_survives_decoding() {
        for spo2 in [70u8, 60, 50, 45, 30, 10, 1] {
            let r = decode_data(&frame(60, spo2))
                .unwrap_or_else(|| panic!("SpO2 {spo2}% must decode, not be filtered"));
            assert_eq!(r.spo2, Some(spo2), "SpO2 {spo2}%");
        }
    }

    /// Same hazard on the other signal: a real bradycardia must not be
    /// filtered for looking implausible.
    #[test]
    fn a_real_bradycardia_survives_decoding() {
        for pr in [50u8, 40, 35, 30] {
            let r = decode_data(&frame(pr, 95)).unwrap_or_else(|| panic!("pulse {pr} must decode"));
            assert_eq!(r.pulse_bpm, Some(pr));
        }
    }

    /// Above 100% is the one genuine impossibility, so that field is absent —
    /// but the pulse riding alongside it survives, because dropping a real
    /// reading is the more expensive error.
    #[test]
    fn spo2_above_100_is_absent_and_the_pulse_survives() {
        let r = decode_data(&frame(60, 122)).expect("frame is well-formed");
        assert_eq!(r.spo2, None, "122% is not physically possible");
        assert_eq!(r.pulse_bpm, Some(60), "the pulse beside it is still real");
    }

    /// **No lower bound anywhere.** A pulse of 25 and an `SpO2` of 45 are exactly
    /// the events this system exists to catch; a decoder that filtered them as
    /// "implausible" would fail silently at the only moment that matters.
    #[test]
    fn profound_bradycardia_and_desaturation_are_never_filtered() {
        let r = decode_data(&frame(25, 45)).expect("frame is well-formed");
        assert_eq!(r.pulse_bpm, Some(25), "25 bpm must reach the detector");
        assert_eq!(r.spo2, Some(45), "45% must reach the detector");

        // The extreme edge of what a byte can carry, either side of a sentinel.
        let r = decode_data(&frame(1, 1)).expect("frame is well-formed");
        assert_eq!(r.pulse_bpm, Some(1));
        assert_eq!(r.spo2, Some(1));
        let r = decode_data(&frame(254, 100)).expect("frame is well-formed");
        assert_eq!(r.pulse_bpm, Some(254));
    }

    /// Header and trailer are fixed. A misaligned read can still satisfy the
    /// checksum, so these are checked too.
    #[test]
    fn a_wrong_header_or_trailer_is_rejected() {
        for (idx, val) in [(1usize, 0x02u8), (2, 0x06), (5, 0x7E), (6, 0x01)] {
            let mut f = frame(60, 95);
            if let Some(b) = f.get_mut(idx) {
                *b = val;
            }
            // Repair the checksum so only the header/trailer check can reject.
            let sum: u32 = f
                .get(..7)
                .expect("payload")
                .iter()
                .map(|b| u32::from(*b))
                .sum();
            if let Some(last) = f.last_mut() {
                *last = u8::try_from(sum & 0x7F).unwrap_or(0);
            }
            assert!(
                decode_data(&f).is_none(),
                "byte {idx} = {val:#04x} must reject the frame"
            );
        }
    }

    #[test]
    fn non_data_opcodes_are_not_readings() {
        // A battery reply is a valid frame but not a measurement.
        let mut f = [0xF6, 0x64, 0x00];
        let sum: u32 = f
            .get(..2)
            .expect("payload")
            .iter()
            .map(|b| u32::from(*b))
            .sum();
        if let Some(last) = f.last_mut() {
            *last = u8::try_from(sum & 0x7F).unwrap_or(0);
        }
        assert!(decode_data(&f).is_none());
    }

    #[test]
    fn wrong_length_is_rejected() {
        assert!(decode_data(&[]).is_none());
        assert!(decode_data(&[0xEB]).is_none());
        assert!(decode_data(DEVICE_LAYOUT_FRAME.get(..7).expect("short")).is_none());
        let mut long = DEVICE_LAYOUT_FRAME.to_vec();
        long.push(0x00);
        assert!(decode_data(&long).is_none());
    }

    #[test]
    fn response_opcodes_are_command_plus_0x70() {
        assert_eq!(response_opcode(0x89), 0xF9);
        assert_eq!(response_opcode(0x8E), 0xFE);
        assert_eq!(response_opcode(0x9B), 0xEB);
        assert_eq!(response_opcode(0x86), 0xF6);
    }

    /// Layout captured live on 2026-08-03, readings substituted: one notification carrying two whole records
    /// and the head of a third. A decoder requiring exactly 8 bytes rejects the
    /// entire thing.
    const COALESCED: [u8; 20] = [
        0xEB, 0x01, 0x05, 0x48, 0x60, 0x7F, 0x00, 0x18, // PR 72, SpO2 96
        0xEB, 0x01, 0x05, 0x48, 0x5F, 0x7F, 0x00, 0x17, // PR 72, SpO2 95
        0xEB, 0x01, 0x05, 0x48, // partial
    ];

    #[test]
    fn a_coalesced_notification_yields_every_whole_frame() {
        let (first, used) = scan_frame(&COALESCED).expect("first frame");
        assert_eq!(used, 8);
        assert_eq!(first.pulse_bpm, Some(0x48));
        assert_eq!(first.spo2, Some(0x60));

        let rest = COALESCED.get(used..).expect("rest");
        let (second, used2) = scan_frame(rest).expect("second frame");
        assert_eq!(used2, 8);
        assert_eq!(second.spo2, Some(0x5F));

        // The trailing partial record yields nothing yet, and must not be
        // discarded — its remaining bytes arrive in the next notification.
        let tail = rest.get(used2..).expect("tail");
        assert_eq!(tail.len(), 4);
        assert!(scan_frame(tail).is_none());
    }

    /// A record split across two notifications must survive reassembly.
    #[test]
    fn a_frame_split_across_notifications_is_recovered() {
        let (head, tail) = DEVICE_LAYOUT_FRAME.split_at(3);
        assert!(scan_frame(head).is_none(), "not enough bytes yet");
        let mut buf = [0u8; 8];
        if let Some(d) = buf.get_mut(..3) {
            d.copy_from_slice(head);
        }
        if let Some(d) = buf.get_mut(3..) {
            d.copy_from_slice(tail);
        }
        let (r, used) = scan_frame(&buf).expect("reassembled");
        assert_eq!(used, 8);
        assert_eq!(r.pulse_bpm, Some(70));
    }

    /// Leading garbage must be skipped, not allowed to hide the good records
    /// behind it.
    #[test]
    fn the_scanner_resynchronises_past_junk() {
        let mut buf = [0u8; 3 + 8];
        buf[0] = 0xAA;
        buf[1] = 0xBB;
        if let Some(dst) = buf.get_mut(3..) {
            dst.copy_from_slice(&DEVICE_LAYOUT_FRAME);
        }
        let (r, used) = scan_frame(&buf).expect("found past the junk");
        assert_eq!(used, 3 + 8);
        assert_eq!(r.spo2, Some(97));
    }

    #[test]
    fn the_scanner_finds_nothing_in_a_short_buffer() {
        assert!(scan_frame(&[]).is_none());
        assert!(scan_frame(&[0xEB, 0x01]).is_none());
    }

    /// No input may panic; every byte comes off a radio.
    #[test]
    fn no_input_panics() {
        for len in 0..12usize {
            for b in 0u16..=0xFF {
                let mut buf = [0u8; 12];
                for (i, slot) in buf.iter_mut().enumerate() {
                    *slot = u8::try_from((b as usize + i) % 256).unwrap_or(0);
                }
                let _ = decode_data(buf.get(..len).unwrap_or(&[]));
                let _ = checksum_ok(buf.get(..len).unwrap_or(&[]));
            }
        }
    }
}
