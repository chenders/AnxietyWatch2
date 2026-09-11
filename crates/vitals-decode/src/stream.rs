//! Allocation-free decoder for `AirSense` 11 `StreamData` JSON.
//!
//! The JSON is carried inside authenticated BLE notifications, but it is still
//! untrusted input: a corrupt frame, firmware change, or malicious peer must not
//! panic the collector or cause an unbounded allocation. The decoder stores only
//! bounded signal and value arrays and represents JSON `null` with an explicit
//! validity bit.

/// Maximum number of signal groups retained in one `StreamData` frame.
pub const MAX_STREAM_SIGNALS: usize = 24;
/// Maximum total number of values retained in one `StreamData` frame.
pub const MAX_STREAM_VALUES: usize = 256;
/// Maximum UTF-8 byte length retained for a source start-time string.
pub const MAX_START_TIME_BYTES: usize = 40;
/// Maximum UTF-8 byte length retained for a source signal id.
pub const MAX_SIGNAL_NAME_BYTES: usize = 48;
const MAX_INTERVAL_MS: u32 = 300_000;
const MAX_SKIP_DEPTH: u8 = 16;

/// A supported `AirSense` stream signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamSignal {
    /// Unknown or unsupported source signal.
    Unknown,
    /// 25-Hz patient flow waveform.
    PatientFlow,
    /// 25-Hz mask-pressure waveform.
    MaskPressure,
    /// Two-second mask-pressure summary.
    MaskPressureTwoSecond,
    /// Two-second inspiratory-pressure summary.
    InspiratoryPressure,
    /// Two-second expiratory-pressure summary.
    ExpiratoryPressure,
    /// Two-second leak summary.
    Leak,
    /// Two-second respiratory-rate estimate.
    RespiratoryRate,
    /// Two-second tidal-volume estimate.
    TidalVolume,
    /// Two-second minute-ventilation estimate.
    MinuteVentilation,
    /// Two-second target minute-ventilation value.
    TargetMinuteVentilation,
    /// Two-second inspiratory-to-expiratory ratio.
    IeRatio,
    /// Two-second snore index.
    SnoreIndex,
    /// Two-second flow-limitation ratio.
    FlowLimitation,
    /// Two-second inspiratory duration.
    InspiratoryDuration,
    /// 25-Hz trigger/cycle event code.
    TriggerCycleEvent,
    /// One-Hz external-oximeter heart rate.
    HeartRate,
    /// One-Hz external-oximeter oxygen saturation.
    SpO2,
}

impl StreamSignal {
    /// Maps a short or documented long `AirSense` signal id.
    #[must_use]
    pub fn from_name(name: &[u8]) -> Self {
        if name == b"_RFL" || name == b"PatientFlow" || name == b"PatientFlow-100hz" {
            Self::PatientFlow
        } else if name == b"_MKP" || name == b"MaskPressure" || name == b"MaskPressure-100hz" {
            Self::MaskPressure
        } else if name == b"_MKF" || name == b"MaskPressure-TwoSecond" {
            Self::MaskPressureTwoSecond
        } else if name == b"_MKI"
            || name == b"InspiratoryPressure-50hz"
            || name == b"InspiratoryPressure-TwoSecond"
        {
            Self::InspiratoryPressure
        } else if name == b"_MKE"
            || name == b"ExpiratoryPressure-50hz"
            || name == b"ExpiratoryPressure-TwoSecond"
        {
            Self::ExpiratoryPressure
        } else if name == b"_LKF" || name == b"Leak" || name == b"Leak-50hz" {
            Self::Leak
        } else if name == b"_RR2" || name == b"RespiratoryRate" || name == b"RespiratoryRate-50hz" {
            Self::RespiratoryRate
        } else if name == b"_TD2" || name == b"TidalVolume" || name == b"TidalVolume-50hz" {
            Self::TidalVolume
        } else if name == b"_MV2"
            || name == b"MinuteVentilation"
            || name == b"MinuteVentilation-50hz"
        {
            Self::MinuteVentilation
        } else if name == b"_TGT" || name == b"TargetMinuteVentilation" {
            Self::TargetMinuteVentilation
        } else if name == b"_IE2" || name == b"IeRatio" {
            Self::IeRatio
        } else if name == b"_SNI" || name == b"SnoreIndex" || name == b"SnoreIndex-50hz" {
            Self::SnoreIndex
        } else if name == b"_FFL" || name == b"FlowLimitation" || name == b"FlowLimitation-50hz" {
            Self::FlowLimitation
        } else if name == b"_INT" || name == b"InspiratoryDuration" {
            Self::InspiratoryDuration
        } else if name == b"_BYV" || name == b"TriggerCycleEvent" {
            Self::TriggerCycleEvent
        } else if name == b"_HRT" || name == b"HeartRate" {
            Self::HeartRate
        } else if name == b"_SAO" || name == b"SpO2" {
            Self::SpO2
        } else {
            Self::Unknown
        }
    }

    /// Native source interval in milliseconds, when known.
    #[must_use]
    pub const fn native_interval_ms(self) -> Option<u32> {
        match self {
            Self::PatientFlow | Self::MaskPressure | Self::TriggerCycleEvent => Some(40),
            Self::HeartRate | Self::SpO2 => Some(1_000),
            Self::Unknown => None,
            Self::MaskPressureTwoSecond
            | Self::InspiratoryPressure
            | Self::ExpiratoryPressure
            | Self::Leak
            | Self::RespiratoryRate
            | Self::TidalVolume
            | Self::MinuteVentilation
            | Self::TargetMinuteVentilation
            | Self::IeRatio
            | Self::SnoreIndex
            | Self::FlowLimitation
            | Self::InspiratoryDuration => Some(2_000),
        }
    }
}

/// One value in a source signal array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamValue {
    /// Whether the source supplied a number rather than JSON `null`.
    pub valid: bool,
    /// The numeric value. Ignore this field when [`Self::valid`] is false.
    pub value: f64,
}

impl StreamValue {
    const EMPTY: Self = Self {
        valid: false,
        value: 0.0,
    };
}

/// One signal group and its contiguous range in [`StreamFrame::values`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamSignalData {
    /// Mapped signal identity; `Unknown` is retained for diagnostics only.
    pub signal: StreamSignal,
    /// Original source id bytes, without JSON quotes.
    pub name: [u8; MAX_SIGNAL_NAME_BYTES],
    /// Number of bytes used in [`Self::name`].
    pub name_len: u8,
    /// Index of the first value in the frame's value array.
    pub value_offset: u16,
    /// Number of values in this signal array, including nulls.
    pub sample_count: u16,
    /// Number of valid numeric values in this signal array.
    pub valid_count: u16,
    /// Interval reported by the device for this frame.
    pub sample_interval_ms: u32,
}

impl StreamSignalData {
    const EMPTY: Self = Self {
        signal: StreamSignal::Unknown,
        name: [0; MAX_SIGNAL_NAME_BYTES],
        name_len: 0,
        value_offset: 0,
        sample_count: 0,
        valid_count: 0,
        sample_interval_ms: 0,
    };

    /// The original source id as a borrowed byte string.
    #[must_use]
    pub fn source_name(&self) -> &[u8] {
        self.name.get(..usize::from(self.name_len)).unwrap_or(&[])
    }
}

/// A decoded, bounded `AirSense` `StreamData` notification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamFrame {
    /// Source-assigned stream id.
    pub stream_id: u32,
    /// Report interval carried by the source frame.
    pub interval_ms: u32,
    /// Raw source start-time bytes, retained verbatim.
    pub start_time: [u8; MAX_START_TIME_BYTES],
    /// Number of bytes used in [`Self::start_time`].
    pub start_time_len: u8,
    /// Number of signal groups.
    pub signal_count: u8,
    /// Number of values across all signal groups.
    pub value_count: u16,
    /// Signal metadata in source order.
    pub signals: [StreamSignalData; MAX_STREAM_SIGNALS],
    /// Values in signal-group order; nulls have `valid == false`.
    pub values: [StreamValue; MAX_STREAM_VALUES],
}

impl StreamFrame {
    const EMPTY: Self = Self {
        stream_id: 0,
        interval_ms: 0,
        start_time: [0; MAX_START_TIME_BYTES],
        start_time_len: 0,
        signal_count: 0,
        value_count: 0,
        signals: [StreamSignalData::EMPTY; MAX_STREAM_SIGNALS],
        values: [StreamValue::EMPTY; MAX_STREAM_VALUES],
    };

    /// Returns the raw source start-time string.
    #[must_use]
    pub fn source_start_time(&self) -> &[u8] {
        self.start_time
            .get(..usize::from(self.start_time_len))
            .unwrap_or(&[])
    }

    /// Returns signal metadata by ordinal.
    #[must_use]
    pub fn signal(&self, index: usize) -> Option<&StreamSignalData> {
        if index >= usize::from(self.signal_count) {
            return None;
        }
        self.signals.get(index)
    }

    /// Returns a signal's values, including explicit null entries.
    #[must_use]
    pub fn values_for(&self, signal: &StreamSignalData) -> Option<&[StreamValue]> {
        let begin = usize::from(signal.value_offset);
        let end = begin.checked_add(usize::from(signal.sample_count))?;
        self.values.get(begin..end)
    }
}

/// Decodes one JSON-RPC `StreamData` envelope.
///
/// The parser accepts the documented short/long ids and ignores bounded JSON
/// members it does not need. A missing `data` member is a valid no-measurement
/// frame; malformed JSON, duplicate fields, invalid numbers, and capacity
/// overflow return `None`.
#[must_use]
pub fn decode_stream(buf: &[u8]) -> Option<StreamFrame> {
    let mut cursor = Cursor { buf, pos: 0 };
    let mut frame = StreamFrame::EMPTY;
    let mut method_seen = false;
    let mut params_seen = false;
    if !cursor.consume(b'{') {
        return None;
    }
    if cursor.consume(b'}') {
        return None;
    }

    loop {
        let mut key = [0u8; 32];
        let key_len = cursor.parse_string(&mut key)?;
        let key = key.get(..key_len)?;
        if !cursor.consume(b':') {
            return None;
        }
        if key == b"method" {
            if method_seen {
                return None;
            }
            let mut method = [0u8; 32];
            let method_len = cursor.parse_string(&mut method)?;
            if method.get(..method_len)? != b"StreamData" {
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

    if !method_seen || !params_seen || !cursor.at_end() {
        return None;
    }
    if frame.stream_id == 0 || frame.interval_ms == 0 || frame.interval_ms > MAX_INTERVAL_MS {
        return None;
    }
    if frame.start_time_len == 0 {
        return None;
    }
    let interval = frame.interval_ms;
    for signal in frame
        .signals
        .iter_mut()
        .take(usize::from(frame.signal_count))
    {
        signal.sample_interval_ms = interval;
    }
    Some(frame)
}

fn parse_params(cursor: &mut Cursor<'_>, frame: &mut StreamFrame) -> bool {
    if !cursor.consume(b'{') || cursor.consume(b'}') {
        return false;
    }
    let mut stream_id_seen = false;
    let mut interval_seen = false;
    let mut start_time_seen = false;
    let mut data_seen = false;

    loop {
        let mut key = [0u8; 32];
        let Some(key_len) = cursor.parse_string(&mut key) else {
            return false;
        };
        let Some(key) = key.get(..key_len) else {
            return false;
        };
        if !cursor.consume(b':') {
            return false;
        }
        if key == b"streamId" {
            if stream_id_seen {
                return false;
            }
            frame.stream_id = match cursor.parse_u32() {
                Some(value) if value != 0 => value,
                _ => return false,
            };
            stream_id_seen = true;
        } else if key == b"intervalMs" {
            if interval_seen {
                return false;
            }
            frame.interval_ms = match cursor.parse_u32() {
                Some(value) if value != 0 && value <= MAX_INTERVAL_MS => value,
                _ => return false,
            };
            interval_seen = true;
        } else if key == b"startTime" {
            if start_time_seen {
                return false;
            }
            let Some(len) = cursor.parse_string(&mut frame.start_time) else {
                return false;
            };
            frame.start_time_len = match u8::try_from(len) {
                Ok(value) => value,
                Err(_) => return false,
            };
            start_time_seen = true;
        } else if key == b"data" {
            if data_seen || parse_data_array(cursor, frame).is_none() {
                return false;
            }
            data_seen = true;
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

    stream_id_seen && interval_seen && start_time_seen
}

fn parse_data_array(cursor: &mut Cursor<'_>, frame: &mut StreamFrame) -> Option<()> {
    if !cursor.consume(b'[') {
        return None;
    }
    if cursor.consume(b']') {
        return Some(());
    }
    loop {
        parse_signal_object(cursor, frame)?;
        if cursor.consume(b']') {
            return Some(());
        }
        if !cursor.consume(b',') {
            return None;
        }
    }
}

fn parse_signal_object(cursor: &mut Cursor<'_>, frame: &mut StreamFrame) -> Option<()> {
    if !cursor.consume(b'{') {
        return None;
    }
    if cursor.consume(b'}') {
        return Some(());
    }
    loop {
        let mut name = [0u8; MAX_SIGNAL_NAME_BYTES];
        let name_len = cursor.parse_string(&mut name)?;
        let name = name.get(..name_len)?;
        if !cursor.consume(b':') {
            return None;
        }
        if StreamSignal::from_name(name) == StreamSignal::Unknown {
            cursor.skip_value(0)?;
        } else {
            add_signal(cursor, frame, name)?;
        }
        if cursor.consume(b'}') {
            return Some(());
        }
        if !cursor.consume(b',') {
            return None;
        }
    }
}

fn add_signal(cursor: &mut Cursor<'_>, frame: &mut StreamFrame, name: &[u8]) -> Option<()> {
    let signal = StreamSignal::from_name(name);
    let signal_count = usize::from(frame.signal_count);
    if signal_count >= MAX_STREAM_SIGNALS {
        return None;
    }
    for existing in frame.signals.iter().take(signal_count) {
        if existing.signal == signal || existing.source_name() == name {
            return None;
        }
    }
    let name_len = u8::try_from(name.len()).ok()?;
    let value_offset = frame.value_count;
    let signal_data = StreamSignalData {
        signal,
        name: copy_name(name)?,
        name_len,
        value_offset,
        sample_count: 0,
        valid_count: 0,
        sample_interval_ms: 0,
    };
    *frame.signals.get_mut(signal_count)? = signal_data;
    frame.signal_count = frame.signal_count.checked_add(1)?;
    parse_value_array(cursor, frame, signal_count)
}

fn copy_name(name: &[u8]) -> Option<[u8; MAX_SIGNAL_NAME_BYTES]> {
    let mut out = [0u8; MAX_SIGNAL_NAME_BYTES];
    out.get_mut(..name.len())?.copy_from_slice(name);
    Some(out)
}

fn parse_value_array(
    cursor: &mut Cursor<'_>,
    frame: &mut StreamFrame,
    signal_index: usize,
) -> Option<()> {
    if !cursor.consume(b'[') {
        return None;
    }
    if cursor.consume(b']') {
        return Some(());
    }
    loop {
        let value = if cursor.consume_literal(b"null") {
            StreamValue::EMPTY
        } else {
            StreamValue {
                valid: true,
                value: cursor.parse_number()?,
            }
        };
        let value_index = usize::from(frame.value_count);
        if value_index >= MAX_STREAM_VALUES {
            return None;
        }
        let slot = frame.values.get_mut(value_index)?;
        *slot = value;
        frame.value_count = frame.value_count.checked_add(1)?;
        let signal = frame.signals.get_mut(signal_index)?;
        signal.sample_count = signal.sample_count.checked_add(1)?;
        if value.valid {
            signal.valid_count = signal.valid_count.checked_add(1)?;
        }

        if cursor.consume(b']') {
            return Some(());
        }
        if !cursor.consume(b',') {
            return None;
        }
    }
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
                    if escaped == b'u' {
                        let mut code = 0u16;
                        for _ in 0..4 {
                            code = code
                                .checked_mul(16)?
                                .checked_add(u16::from(hex_digit(self.take()?)?))?;
                        }
                        if (0xD800..=0xDFFF).contains(&code) {
                            return None;
                        }
                        if code < 0x80 {
                            push(out, &mut len, u8::try_from(code).ok()?)?;
                        } else if code < 0x800 {
                            push(out, &mut len, u8::try_from(0xC0 | (code >> 6)).ok()?)?;
                            push(out, &mut len, u8::try_from(0x80 | (code & 0x3F)).ok()?)?;
                        } else {
                            push(out, &mut len, u8::try_from(0xE0 | (code >> 12)).ok()?)?;
                            push(
                                out,
                                &mut len,
                                u8::try_from(0x80 | ((code >> 6) & 0x3F)).ok()?,
                            )?;
                            push(out, &mut len, u8::try_from(0x80 | (code & 0x3F)).ok()?)?;
                        }
                    } else {
                        let decoded = match escaped {
                            b'"' | b'\\' | b'/' => escaped,
                            b'b' => 8,
                            b'f' => 12,
                            b'n' => b'\n',
                            b'r' => b'\r',
                            b't' => b'\t',
                            _ => return None,
                        };
                        push(out, &mut len, decoded)?;
                    }
                }
                value if value >= 0x20 => push(out, &mut len, value)?,
                _ => return None,
            }
        }
    }

    fn parse_u32(&mut self) -> Option<u32> {
        self.skip_ws();
        let first = self.peek()?;
        if !first.is_ascii_digit() {
            return None;
        }
        let mut value = 0u32;
        let mut digits = 0u8;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            if digits == 1 && value == 0 {
                return None;
            }
            value = value.checked_mul(10)?.checked_add(u32::from(byte - b'0'))?;
            digits = digits.checked_add(1)?;
            self.pos = self.pos.saturating_add(1);
        }
        Some(value)
    }

    fn parse_number(&mut self) -> Option<f64> {
        self.skip_ws();
        let negative = self.peek() == Some(b'-');
        if negative {
            self.pos = self.pos.saturating_add(1);
        }
        let first = self.peek()?;
        if !first.is_ascii_digit() {
            return None;
        }
        let mut value = 0.0f64;
        let mut digits = 0u8;
        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }
            if digits == 1 && value == 0.0 {
                return None;
            }
            value = value * 10.0 + f64::from(byte - b'0');
            digits = digits.checked_add(1)?;
            if digits > 19 || !value.is_finite() {
                return None;
            }
            self.pos = self.pos.saturating_add(1);
        }

        if self.peek() == Some(b'.') {
            self.pos = self.pos.saturating_add(1);
            let mut factor = 0.1f64;
            let mut fraction_digits = 0u8;
            while let Some(byte) = self.peek() {
                if !byte.is_ascii_digit() {
                    break;
                }
                value += f64::from(byte - b'0') * factor;
                factor *= 0.1;
                fraction_digits = fraction_digits.checked_add(1)?;
                if fraction_digits > 18 || !value.is_finite() {
                    return None;
                }
                self.pos = self.pos.saturating_add(1);
            }
            if fraction_digits == 0 {
                return None;
            }
        }

        let exponent = if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos = self.pos.saturating_add(1);
            let exponent_negative = match self.peek() {
                Some(b'-') => {
                    self.pos = self.pos.saturating_add(1);
                    true
                }
                Some(b'+') => {
                    self.pos = self.pos.saturating_add(1);
                    false
                }
                _ => false,
            };
            let mut exponent_value = 0u32;
            let mut exponent_digits = 0u8;
            while let Some(byte) = self.peek() {
                if !byte.is_ascii_digit() {
                    break;
                }
                exponent_value = exponent_value
                    .checked_mul(10)?
                    .checked_add(u32::from(byte - b'0'))?;
                exponent_digits = exponent_digits.checked_add(1)?;
                if exponent_digits > 2 || exponent_value > 12 {
                    return None;
                }
                self.pos = self.pos.saturating_add(1);
            }
            if exponent_digits == 0 {
                return None;
            }
            Some((exponent_negative, exponent_value))
        } else {
            None
        };

        if let Some((exponent_negative, exponent_value)) = exponent {
            for _ in 0..exponent_value {
                if exponent_negative {
                    value *= 0.1;
                } else {
                    value *= 10.0;
                }
            }
        }
        if negative {
            value = -value;
        }
        value.is_finite().then_some(value)
    }

    fn skip_value(&mut self, depth: u8) -> Option<()> {
        if depth >= MAX_SKIP_DEPTH {
            return None;
        }
        self.skip_ws();
        match self.peek()? {
            b'"' => {
                self.skip_string()?;
                Some(())
            }
            b'{' => self.skip_object(depth),
            b'[' => self.skip_array(depth),
            b't' => self.consume_literal(b"true").then_some(()),
            b'f' => self.consume_literal(b"false").then_some(()),
            b'n' => self.consume_literal(b"null").then_some(()),
            b'-' | b'0'..=b'9' => {
                self.parse_number()?;
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
            self.skip_string()?;
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

    fn skip_string(&mut self) -> Option<()> {
        self.skip_ws();
        if self.take()? != b'"' {
            return None;
        }
        loop {
            match self.take()? {
                b'"' => return Some(()),
                b'\\' => {
                    let escaped = self.take()?;
                    if escaped == b'u' {
                        for _ in 0..4 {
                            hex_digit(self.take()?)?;
                        }
                    } else if !matches!(
                        escaped,
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't'
                    ) {
                        return None;
                    }
                }
                value if value >= 0x20 => {}
                _ => return None,
            }
        }
    }
}

fn push(out: &mut [u8], len: &mut usize, value: u8) -> Option<()> {
    let slot = out.get_mut(*len)?;
    *slot = value;
    *len = len.checked_add(1)?;
    Some(())
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    // De-identified fixture: it contains no pairing material, device address,
    // serial, or raw capture bytes.
    const FRAME: &[u8] = include_bytes!("../fixtures/air11-stream-null.json");

    fn signal(frame: &StreamFrame, wanted: StreamSignal) -> &StreamSignalData {
        for index in 0..usize::from(frame.signal_count) {
            let candidate = frame.signal(index).expect("signal");
            if candidate.signal == wanted {
                return candidate;
            }
        }
        panic!("signal not found: {wanted:?}");
    }

    #[test]
    fn decodes_values_nulls_and_source_metadata() {
        let frame = decode_stream(FRAME).expect("valid stream");
        assert_eq!(frame.stream_id, 4);
        assert_eq!(frame.interval_ms, 200);
        assert_eq!(frame.source_start_time(), b"2008-01-13T22:35:03.414Z");
        let flow = signal(&frame, StreamSignal::PatientFlow);
        assert_eq!(flow.sample_count, 3);
        assert_eq!(flow.valid_count, 2);
        assert_eq!(flow.source_name(), b"_RFL");
        let values = frame.values_for(flow).expect("flow values");
        assert!(values.first().expect("first").valid);
        assert!((values.first().expect("first").value - 0.33).abs() < 1e-12);
        assert!(!values.get(2).expect("null").valid);
        let leak = signal(&frame, StreamSignal::Leak);
        let leak_value = frame
            .values_for(leak)
            .expect("leak values")
            .first()
            .expect("leak value");
        assert!(leak_value.valid);
        assert!((leak_value.value - 0.0).abs() < 1e-12);
        assert!(
            !signal(&frame, StreamSignal::HeartRate)
                .signal
                .eq(&StreamSignal::Unknown)
        );
        assert_eq!(signal(&frame, StreamSignal::HeartRate).valid_count, 0);
        assert_eq!(signal(&frame, StreamSignal::SpO2).valid_count, 0);
    }

    #[test]
    fn maps_documented_aliases_and_native_intervals() {
        assert_eq!(
            StreamSignal::from_name(b"_RR2"),
            StreamSignal::RespiratoryRate
        );
        assert_eq!(
            StreamSignal::from_name(b"InspiratoryPressure-TwoSecond"),
            StreamSignal::InspiratoryPressure
        );
        assert_eq!(
            StreamSignal::from_name(b"_MKI"),
            StreamSignal::InspiratoryPressure
        );
        assert_eq!(StreamSignal::PatientFlow.native_interval_ms(), Some(40));
        assert_eq!(
            StreamSignal::RespiratoryRate.native_interval_ms(),
            Some(2_000)
        );
        assert_eq!(StreamSignal::Unknown.native_interval_ms(), None);
    }

    #[test]
    fn accepts_a_frame_without_data_as_no_measurement() {
        let bytes = br#"{"method":"StreamData","params":{"streamId":1,"intervalMs":200,"startTime":"2008-01-01T00:00:00.000Z"}}"#;
        let frame = decode_stream(bytes).expect("valid empty stream frame");
        assert_eq!(frame.signal_count, 0);
        assert_eq!(frame.value_count, 0);
    }

    #[test]
    fn unknown_signal_is_skipped_without_becoming_a_value() {
        let bytes = br#"{"method":"StreamData","params":{"streamId":1,"intervalMs":200,"startTime":"t","data":[{"_UNKNOWN":[1.0]},{"_RR2":[12.0]}]}}"#;
        let frame = decode_stream(bytes).expect("valid unknown signal");
        assert_eq!(frame.signal_count, 1);
        assert_eq!(frame.value_count, 1);
        assert_eq!(
            frame.signal(0).expect("signal").signal,
            StreamSignal::RespiratoryRate
        );
    }

    #[test]
    fn malformed_json_and_duplicate_fields_are_rejected() {
        for bytes in [
            br#"{"method":"StreamData","params":{"streamId":1,"intervalMs":200,"startTime":"x"} too}"#.as_slice(),
            br#"{"method":"NotStreamData","params":{"streamId":1,"intervalMs":200,"startTime":"x"}}"#.as_slice(),
            br#"{"method":"StreamData","params":{"streamId":1,"intervalMs":200,"intervalMs":200,"startTime":"x"}}"#.as_slice(),
            br#"{"method":"StreamData","params":{"streamId":1,"intervalMs":200,"startTime":"x","data":[{"_RR2":[1e99]}]}}"#.as_slice(),
            br#"{"method":"StreamData","params":{"streamId":1,"intervalMs":200,"startTime":"x","data":[{"_RR2":[null,]}]}}"#.as_slice(),
        ] {
            assert_eq!(decode_stream(bytes), None);
        }
    }

    #[test]
    fn bounds_are_rejected_instead_of_truncated() {
        let mut bytes = std::vec::Vec::from(
            br#"{"method":"StreamData","params":{"streamId":1,"intervalMs":200,"startTime":"x","data":[{"_RR2":["#,
        );
        for index in 0..=MAX_STREAM_VALUES {
            if index != 0 {
                bytes.extend_from_slice(b",");
            }
            bytes.extend_from_slice(b"1");
        }
        bytes.extend_from_slice(b"]}]}}");
        assert_eq!(decode_stream(&bytes), None);
    }
}
