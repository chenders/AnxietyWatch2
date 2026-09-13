//! Allocation-free decoder for `AirSense` `EventNotification` JSON.
//!
//! Event notifications share the encrypted application lane with `StreamData`.
//! The decoder retains bounded event names and timestamps so malformed or
//! unexpectedly large device payloads cannot allocate or panic.

/// Maximum event records retained from one notification.
pub const MAX_EVENTS: usize = 16;
/// Maximum UTF-8 byte length retained for an event data id.
pub const MAX_DATA_ID_BYTES: usize = 64;
/// Maximum UTF-8 byte length retained for an event name.
pub const MAX_EVENT_NAME_BYTES: usize = 40;
/// Maximum UTF-8 byte length retained for an event report time.
pub const MAX_REPORT_TIME_BYTES: usize = 40;
const MAX_SKIP_DEPTH: u8 = 16;

/// Event classification used by the `AirSense` event/report paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// A scored hypopnea.
    Hypopnea,
    /// A scored central apnea.
    CentralApnea,
    /// A scored obstructive apnea.
    ObstructiveApnea,
    /// A scored apnea without a more specific class.
    UnclassifiedApnea,
    /// A respiratory-effort-related arousal; contributes to RDI, not AHI.
    Rera,
    /// Start of Cheyne–Stokes respiration.
    CsrStart,
    /// End of Cheyne–Stokes respiration.
    CsrEnd,
    /// A lifecycle/activity event not classified by this decoder.
    Other,
}

impl EventKind {
    fn from_name(name: &[u8]) -> Self {
        match name {
            b"HypopneaEnd" => Self::Hypopnea,
            b"CentralApneaEnd" => Self::CentralApnea,
            b"ObstructiveApneaEnd" => Self::ObstructiveApnea,
            b"ApneaEnd" => Self::UnclassifiedApnea,
            b"ReraEnd" => Self::Rera,
            b"CsrStart" => Self::CsrStart,
            b"CsrEnd" => Self::CsrEnd,
            _ => Self::Other,
        }
    }
}

/// One bounded event record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventRecord {
    /// Classified event kind.
    pub kind: EventKind,
    /// Original event name, without JSON quotes.
    pub name: [u8; MAX_EVENT_NAME_BYTES],
    /// Number of bytes used in [`Self::name`].
    pub name_len: u8,
    /// Optional device report time, without JSON quotes.
    pub report_time: [u8; MAX_REPORT_TIME_BYTES],
    /// Number of bytes used in [`Self::report_time`].
    pub report_time_len: u8,
    /// Optional event duration in milliseconds.
    pub duration_ms: Option<u32>,
    /// Optional lifecycle value.
    pub value: Option<i32>,
    /// Optional device backdate in milliseconds.
    pub backdate_ms: Option<u32>,
}

impl EventRecord {
    const EMPTY: Self = Self {
        kind: EventKind::Other,
        name: [0; MAX_EVENT_NAME_BYTES],
        name_len: 0,
        report_time: [0; MAX_REPORT_TIME_BYTES],
        report_time_len: 0,
        duration_ms: None,
        value: None,
        backdate_ms: None,
    };

    /// Original event name.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        self.name.get(..usize::from(self.name_len)).unwrap_or(&[])
    }

    /// Device report time, when supplied.
    #[must_use]
    pub fn report_time(&self) -> &[u8] {
        self.report_time
            .get(..usize::from(self.report_time_len))
            .unwrap_or(&[])
    }
}

/// One bounded `AirSense` event notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventFrame {
    /// Event subscription identifier.
    pub subscription_id: u32,
    /// Event data id, without JSON quotes.
    pub data_id: [u8; MAX_DATA_ID_BYTES],
    /// Number of bytes used in [`Self::data_id`].
    pub data_id_len: u8,
    /// Retained event records.
    pub events: [EventRecord; MAX_EVENTS],
    /// Number of retained records.
    pub event_count: u8,
    /// More records were present than the bounded retention capacity.
    pub truncated: bool,
}

impl EventFrame {
    const EMPTY: Self = Self {
        subscription_id: 0,
        data_id: [0; MAX_DATA_ID_BYTES],
        data_id_len: 0,
        events: [EventRecord::EMPTY; MAX_EVENTS],
        event_count: 0,
        truncated: false,
    };

    /// Original event data id.
    #[must_use]
    pub fn data_id(&self) -> &[u8] {
        self.data_id
            .get(..usize::from(self.data_id_len))
            .unwrap_or(&[])
    }
}

/// Decodes one `EventNotification` JSON envelope.
#[must_use]
pub fn decode_event(buf: &[u8]) -> Option<EventFrame> {
    let mut cursor = Cursor { buf, pos: 0 };
    let mut frame = EventFrame::EMPTY;
    let mut method_seen = false;
    let mut params_seen = false;
    if !cursor.consume(b'{') || cursor.consume(b'}') {
        return None;
    }
    loop {
        let mut key = [0_u8; 32];
        let key_len = cursor.parse_string(&mut key)?;
        let key = key.get(..key_len)?;
        if !cursor.consume(b':') {
            return None;
        }
        if key == b"method" {
            if method_seen {
                return None;
            }
            let mut method = [0_u8; 32];
            if cursor.parse_string(&mut method)? != b"EventNotification".len()
                || method.get(..b"EventNotification".len())? != b"EventNotification"
            {
                return None;
            }
            method_seen = true;
        } else if key == b"params" {
            if params_seen || !parse_params(&mut cursor, &mut frame) {
                return None;
            }
            params_seen = true;
        } else {
            cursor.skip_value(0)?;
        }
        if cursor.consume(b'}') {
            break;
        }
        if !cursor.consume(b',') {
            return None;
        }
    }
    if !method_seen || !params_seen || frame.subscription_id == 0 || !cursor.at_end() {
        return None;
    }
    Some(frame)
}

fn parse_params(cursor: &mut Cursor<'_>, frame: &mut EventFrame) -> bool {
    if !cursor.consume(b'{') || cursor.consume(b'}') {
        return false;
    }
    let mut subscription_seen = false;
    let mut data_id_seen = false;
    let mut events_seen = false;
    loop {
        let mut key = [0_u8; 32];
        let Some(key_len) = cursor.parse_string(&mut key) else {
            return false;
        };
        let Some(key) = key.get(..key_len) else {
            return false;
        };
        if !cursor.consume(b':') {
            return false;
        }
        if key == b"subscriptionId" {
            if subscription_seen {
                return false;
            }
            frame.subscription_id = match cursor.parse_u32() {
                Some(value) if value != 0 => value,
                _ => return false,
            };
            subscription_seen = true;
        } else if key == b"dataId" {
            if data_id_seen {
                return false;
            }
            let Some(len) = cursor.parse_string(&mut frame.data_id) else {
                return false;
            };
            frame.data_id_len = match u8::try_from(len) {
                Ok(value) => value,
                Err(_) => return false,
            };
            data_id_seen = true;
        } else if key == b"events" {
            if events_seen || !parse_events(cursor, frame) {
                return false;
            }
            events_seen = true;
        } else if cursor.skip_value(0).is_none() {
            return false;
        }
        if cursor.consume(b'}') {
            break;
        }
        if !cursor.consume(b',') {
            return false;
        }
    }
    subscription_seen && data_id_seen && events_seen
}

fn parse_events(cursor: &mut Cursor<'_>, frame: &mut EventFrame) -> bool {
    if !cursor.consume(b'[') {
        return false;
    }
    if cursor.consume(b']') {
        return true;
    }
    loop {
        let Some(record) = parse_event_record(cursor, usize::from(frame.event_count) >= MAX_EVENTS)
        else {
            return false;
        };
        match record {
            ParsedEvent::Record(record) => {
                let index = usize::from(frame.event_count);
                let Some(slot) = frame.events.get_mut(index) else {
                    return false;
                };
                *slot = record;
                frame.event_count = match frame.event_count.checked_add(1) {
                    Some(value) => value,
                    None => return false,
                };
            }
            ParsedEvent::Discarded => frame.truncated = true,
        }
        if cursor.consume(b']') {
            return true;
        }
        if !cursor.consume(b',') {
            return false;
        }
    }
}

enum ParsedEvent {
    Record(EventRecord),
    Discarded,
}

fn parse_event_record(cursor: &mut Cursor<'_>, discard: bool) -> Option<ParsedEvent> {
    if !cursor.consume(b'{') || cursor.consume(b'}') {
        return None;
    }
    let mut record = EventRecord::EMPTY;
    let mut event_seen = false;
    let mut report_time_seen = false;
    let mut duration_ms = None;
    let mut duration_seen = false;
    let mut value = None;
    let mut value_seen = false;
    let mut backdate_ms = None;
    let mut backdate_seen = false;
    loop {
        let mut key = [0_u8; 40];
        let key_len = cursor.parse_string(&mut key)?;
        let key = key.get(..key_len)?;
        if !cursor.consume(b':') {
            return None;
        }
        if key == b"event" {
            if event_seen {
                return None;
            }
            let name_len = cursor.parse_string(&mut record.name)?;
            record.name_len = u8::try_from(name_len).ok()?;
            record.kind = EventKind::from_name(record.name.get(..name_len)?);
            event_seen = true;
        } else if key == b"reportTime" {
            if report_time_seen {
                return None;
            }
            let time_len = cursor.parse_string(&mut record.report_time)?;
            record.report_time_len = u8::try_from(time_len).ok()?;
            report_time_seen = true;
        } else if key == b"value" {
            if value_seen {
                return None;
            }
            value = Some(cursor.parse_i32()?);
            value_seen = true;
        } else if matches!(
            key,
            b"durationMs" | b"duration_ms" | b"durationMilliseconds"
        ) {
            if duration_seen {
                return None;
            }
            let value = cursor.parse_u32()?;
            duration_ms = Some(value);
            duration_seen = true;
        } else if matches!(key, b"durationSeconds" | b"duration_s") {
            if duration_seen {
                return None;
            }
            let value = cursor.parse_u32()?;
            duration_ms = Some(value.checked_mul(1000)?);
            duration_seen = true;
        } else if key == b"backdateSeconds" {
            if backdate_seen {
                return None;
            }
            let value = cursor.parse_u32()?;
            backdate_ms = Some(value.checked_mul(1000)?);
            backdate_seen = true;
        } else {
            cursor.skip_value(0)?;
        }
        if cursor.consume(b'}') {
            break;
        }
        if !cursor.consume(b',') {
            return None;
        }
    }
    if !event_seen {
        return None;
    }
    record.duration_ms = duration_ms;
    record.value = value;
    record.backdate_ms = backdate_ms;
    if discard {
        return Some(ParsedEvent::Discarded);
    }
    Some(ParsedEvent::Record(record))
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn at_end(&mut self) -> bool {
        self.skip_ws();
        self.pos == self.buf.len()
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let value = self.peek()?;
        self.pos = self.pos.saturating_add(1);
        Some(value)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos = self.pos.saturating_add(1);
        }
    }

    fn consume(&mut self, expected: u8) -> bool {
        self.skip_ws();
        if self.peek() != Some(expected) {
            return false;
        }
        self.pos = self.pos.saturating_add(1);
        true
    }

    fn consume_literal(&mut self, literal: &[u8]) -> bool {
        self.skip_ws();
        let Some(end) = self.pos.checked_add(literal.len()) else {
            return false;
        };
        if self.buf.get(self.pos..end) != Some(literal) {
            return false;
        }
        self.pos = end;
        true
    }

    fn parse_string(&mut self, out: &mut [u8]) -> Option<usize> {
        self.skip_ws();
        if self.take()? != b'"' {
            return None;
        }
        let mut len = 0usize;
        loop {
            match self.take()? {
                b'"' => return Some(len),
                b'\\' => {
                    let escaped = self.take()?;
                    let decoded = match escaped {
                        b'"' | b'\\' | b'/' => escaped,
                        b'b' => 8,
                        b'f' => 12,
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        _ => return None,
                    };
                    let slot = out.get_mut(len)?;
                    *slot = decoded;
                    len = len.checked_add(1)?;
                }
                value if value >= 0x20 => {
                    let slot = out.get_mut(len)?;
                    *slot = value;
                    len = len.checked_add(1)?;
                }
                _ => return None,
            }
        }
    }

    fn parse_u32(&mut self) -> Option<u32> {
        self.skip_ws();
        if !self.peek()?.is_ascii_digit() {
            return None;
        }
        let mut value = 0u32;
        let mut digits = 0u8;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            value = value.checked_mul(10)?.checked_add(u32::from(byte - b'0'))?;
            digits = digits.checked_add(1)?;
            self.pos = self.pos.saturating_add(1);
        }
        (digits > 0).then_some(value)
    }

    fn parse_i32(&mut self) -> Option<i32> {
        self.skip_ws();
        let negative = self.consume(b'-');
        let magnitude = self.parse_u32()?;
        if negative {
            if magnitude == 2_147_483_648 {
                Some(i32::MIN)
            } else {
                i32::try_from(magnitude).ok().map(|value| -value)
            }
        } else {
            i32::try_from(magnitude).ok()
        }
    }

    fn skip_value(&mut self, depth: u8) -> Option<()> {
        if depth >= MAX_SKIP_DEPTH {
            return None;
        }
        self.skip_ws();
        match self.peek()? {
            b'"' => {
                self.parse_string(&mut [0; MAX_EVENT_NAME_BYTES])?;
                Some(())
            }
            b'{' => self.skip_object(depth),
            b'[' => self.skip_array(depth),
            b't' => self.consume_literal(b"true").then_some(()),
            b'f' => self.consume_literal(b"false").then_some(()),
            b'n' => self.consume_literal(b"null").then_some(()),
            b'0'..=b'9' => {
                self.parse_u32()?;
                Some(())
            }
            _ => None,
        }
    }

    fn skip_object(&mut self, depth: u8) -> Option<()> {
        if !self.consume(b'{') {
            return None;
        }
        if self.consume(b'}') {
            return Some(());
        }
        loop {
            self.parse_string(&mut [0; 40])?;
            if !self.consume(b':') {
                return None;
            }
            self.skip_value(depth.checked_add(1)?)?;
            if self.consume(b'}') {
                return Some(());
            }
            if !self.consume(b',') {
                return None;
            }
        }
    }

    fn skip_array(&mut self, depth: u8) -> Option<()> {
        if !self.consume(b'[') {
            return None;
        }
        if self.consume(b']') {
            return Some(());
        }
        loop {
            self.skip_value(depth.checked_add(1)?)?;
            if self.consume(b']') {
                return Some(());
            }
            if !self.consume(b',') {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::{format, string::String};

    #[test]
    fn decodes_respiratory_events_and_metadata() {
        let frame = decode_event(br#"{"jsonrpc":"1.0","method":"EventNotification","params":{"subscriptionId":7,"dataId":"TherapyEvents-RespiratoryEvents","events":[{"event":"HypopneaEnd","reportTime":"2026-09-14T01:02:03.000Z","durationMs":12000},{"event":"ReraEnd"}]}}"#).expect("event frame");
        assert_eq!(frame.subscription_id, 7);
        assert_eq!(frame.data_id(), b"TherapyEvents-RespiratoryEvents");
        assert_eq!(frame.event_count, 2);
        assert_eq!(
            frame.events.first().expect("first").kind,
            EventKind::Hypopnea
        );
        assert_eq!(
            frame.events.first().expect("first").duration_ms,
            Some(12000)
        );
        assert_eq!(
            frame.events.first().expect("first").report_time(),
            b"2026-09-14T01:02:03.000Z"
        );
        assert_eq!(frame.events.get(1).expect("second").kind, EventKind::Rera);
    }

    #[test]
    fn decodes_lifecycle_values_without_coercion() {
        let frame = decode_event(
            br#"{"method":"EventNotification","params":{"subscriptionId":1,"dataId":"UsageEvents-TherapyStatusEvents","events":[{"event":"ValueChange","value":-1}]}}"#,
        )
        .expect("lifecycle value");
        assert_eq!(frame.events.first().expect("first").value, Some(-1));
        assert!(decode_event(
            br#"{"method":"EventNotification","params":{"subscriptionId":1,"dataId":"x","events":[{"event":"ValueChange","value":2147483648}]}}"#,
        )
        .is_none());
    }

    #[test]
    fn retains_lifecycle_events_and_marks_overflow() {
        let mut events = String::from("[");
        for index in 0..17 {
            if index != 0 {
                events.push(',');
            }
            events.push_str(r#"{"event":"TherapyStarted"}"#);
        }
        events.push(']');
        let json = format!(
            r#"{{"method":"EventNotification","params":{{"subscriptionId":1,"dataId":"UsageEvents-TherapyStatusEvents","events":{events}}}}}"#
        );
        let frame = decode_event(json.as_bytes()).expect("lifecycle frame");
        assert_eq!(frame.event_count, 16);
        assert!(frame.truncated);
        assert_eq!(frame.events.first().expect("first").kind, EventKind::Other);
    }

    #[test]
    fn malformed_or_duplicate_fields_are_rejected() {
        assert!(decode_event(br#"{"method":"EventNotification","params":{"subscriptionId":1,"subscriptionId":2,"dataId":"x","events":[]}}"#).is_none());
        assert!(decode_event(br#"{"method":"EventNotification","params":{"subscriptionId":1,"dataId":"x","events":[{"event":"x","durationSeconds":4294967295}]}}"#).is_none());
        assert!(decode_event(br#"{"method":"EventNotification","params":{"subscriptionId":1,"dataId":"x","events":[{"event":"x","durationMs":1,"durationSeconds":1}]}}"#).is_none());
    }
}
