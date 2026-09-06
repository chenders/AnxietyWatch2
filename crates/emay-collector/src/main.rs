//! Linux wiring for the durable EMAY `SleepO2` stored-session collector.
//!
//! The collector downloads source-identified stored sessions at startup and
//! on a bounded periodic interval, appends them through `vitalsd`, then keeps
//! the live stream alive for bedside status. Live `EB` frames are never
//! assigned a durable sequence here.

#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
const DEFAULT_SOCKET: &str = "/run/vitals/vitalsd.sock";

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("emay-collector needs BlueZ and runs on the Pi; portable protocol tests run with: ");
    eprintln!("    cargo test -p emay-collector");
    #[cfg(not(test))]
    std::process::exit(64);
}

#[cfg(all(test, not(target_os = "linux")))]
#[test]
fn non_linux_entrypoint_reports_its_platform_requirement() {
    // The production entrypoint exits with a usage status. Test builds omit
    // that final process exit so coverage can execute the same evidence path.
    main();
}

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashMap;
    use std::future::Future;
    use std::path::Path;
    use std::pin::Pin;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use bluer::{Adapter, Address, Device};
    use chrono::{Datelike as _, Local, Timelike as _};
    use futures::{Stream, StreamExt as _};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::UnixStream;
    use vitals_decode::emay::{
        SESSION_DESCRIPTOR_LEN, STORED_RECORD_LEN, SessionDescriptor, StoredMetric, StoredRecord,
        cmd, decode_session_descriptor, decode_stored_record, is_empty_session_descriptor,
        next_storage_cursor, scan_frame, storage,
    };
    use vitals_proto::{
        Request, Response, WireSample, check_frame_len, decode_payload, encode_frame,
    };
    use vitals_registry::VitalsType;

    use emay_collector::{LiveIdentityState, StoredSyncState, StoredTransferState, wire_samples};

    const SERVICE: bluer::Uuid = bluer::Uuid::from_u128(0x0000_ff12_0000_1000_8000_0080_5f9b_34fb);
    const WRITE_CHAR: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_ff01_0000_1000_8000_0080_5f9b_34fb);
    const NOTIFY_CHAR: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_ff02_0000_1000_8000_0080_5f9b_34fb);
    const STORAGE_SLOTS: usize = 7;
    #[cfg(not(test))]
    const RESPONSE_TIMEOUT: Duration = Duration::from_secs(3);
    #[cfg(test)]
    const RESPONSE_TIMEOUT: Duration = Duration::from_millis(1);
    const STORAGE_TRANSFER_RETRIES: usize = 3;
    const APPEND_BATCH_SIZE: usize = 256;
    #[cfg(not(test))]
    const EMPTY_STREAM_GRACE: Duration = Duration::from_secs(1);
    #[cfg(test)]
    const EMPTY_STREAM_GRACE: Duration = Duration::from_millis(1);
    #[cfg(not(test))]
    const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(1200);
    #[cfg(test)]
    const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(1);
    #[cfg(not(test))]
    const SILENCE_LIMIT: Duration = Duration::from_mins(2);
    #[cfg(test)]
    const SILENCE_LIMIT: Duration = Duration::from_millis(10);
    /// Re-enter confirmed device storage often enough to bound the time a
    /// source cursor can remain unsynchronized, without making live display
    /// the history path.
    const STORAGE_SYNC_INTERVAL: Duration = Duration::from_mins(15);
    /// Every per-command timeout is finite, but a malformed or quiet device
    /// must also be prevented from occupying live mode indefinitely.
    #[cfg(not(test))]
    const STORAGE_SYNC_DEADLINE: Duration = Duration::from_mins(3);
    #[cfg(test)]
    const STORAGE_SYNC_DEADLINE: Duration = Duration::from_millis(10);
    type CollectorResult<T> = Result<T, String>;

    trait CharacteristicWriter {
        async fn write(&self, bytes: &[u8]) -> bluer::Result<()>;
    }

    struct BluezWriter(bluer::gatt::remote::Characteristic);

    impl CharacteristicWriter for BluezWriter {
        async fn write(&self, bytes: &[u8]) -> bluer::Result<()> {
            self.0.write(bytes).await
        }
    }

    type NotificationStream = Pin<Box<dyn Stream<Item = Vec<u8>>>>;

    trait CollectorBackend {
        type Writer: CharacteristicWriter;

        async fn connect(
            &self,
            target: Address,
        ) -> CollectorResult<(Self::Writer, NotificationStream)>;
    }

    /// Current live status, deliberately separate from durable stored rows.
    #[derive(Debug)]
    struct LiveStatus {
        written_at: f64,
        sample_at: Option<f64>,
        spo2: Option<u8>,
        pulse_bpm: Option<u8>,
        finger: bool,
        link: &'static str,
        frames: u64,
        measurements: u64,
        no_finger: u64,
        undecodable: u64,
        stored_sync: StoredSyncState,
    }

    impl LiveStatus {
        fn new(stored_sync: StoredSyncState) -> Self {
            Self {
                written_at: now_secs(),
                sample_at: None,
                spo2: None,
                pulse_bpm: None,
                finger: false,
                link: "up",
                frames: 0,
                measurements: 0,
                no_finger: 0,
                undecodable: 0,
                stored_sync,
            }
        }
    }

    fn now_secs() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |duration| duration.as_secs_f64())
    }

    fn json_option(value: Option<u8>) -> String {
        value.map_or_else(|| "null".to_owned(), |number| number.to_string())
    }

    /// Encodes externally sourced error text without allowing it to corrupt
    /// the status document. A malformed status feed is a safety failure, not
    /// a cosmetic problem.
    fn json_string(value: &str) -> String {
        use std::fmt::Write as _;

        let mut output = String::with_capacity(value.len() + 2);
        output.push('"');
        for character in value.chars() {
            match character {
                '"' => output.push_str("\\\""),
                '\\' => output.push_str("\\\\"),
                '\u{08}' => output.push_str("\\b"),
                '\u{0c}' => output.push_str("\\f"),
                '\n' => output.push_str("\\n"),
                '\r' => output.push_str("\\r"),
                '\t' => output.push_str("\\t"),
                character if character.is_control() => {
                    let _ = write!(output, "\\u{:04x}", u32::from(character));
                }
                character => output.push(character),
            }
        }
        output.push('"');
        output
    }

    fn json_option_string(value: Option<&str>) -> String {
        value.map_or_else(|| "null".to_owned(), json_string)
    }

    fn json_option_f64(value: Option<f64>) -> String {
        value.map_or_else(|| "null".to_owned(), |number| format!("{number:.3}"))
    }

    fn status_json(status: &LiveStatus) -> String {
        format!(
            concat!(
                "{{\"schema\":2,",
                "\"written_at\":{:.3},",
                "\"sample_at\":{},",
                "\"spo2\":{},",
                "\"pulse_bpm\":{},",
                "\"finger\":{},",
                "\"link\":{},",
                "\"live_identity\":{},",
                "\"live_durability\":\"status_only_not_committed\",",
                "\"frames\":{},",
                "\"measurements\":{},",
                "\"no_finger\":{},",
                "\"undecodable\":{},",
                "\"stored_transfer\":{},",
                "\"stored_identity\":{},",
                "\"stored_durability\":{},",
                "\"stored_records_transferred\":{},",
                "\"stored_rows_spooled\":{},",
                "\"stored_last_sync_at\":{},",
                "\"stored_last_error\":{}}}\n"
            ),
            status.written_at,
            json_option_f64(status.sample_at),
            json_option(status.spo2),
            json_option(status.pulse_bpm),
            status.finger,
            json_string(status.link),
            json_string(LiveIdentityState::AnonymousNotDurable.as_str()),
            status.frames,
            status.measurements,
            status.no_finger,
            status.undecodable,
            json_string(status.stored_sync.transfer.as_str()),
            json_string(status.stored_sync.identity.as_str()),
            json_string(status.stored_sync.durability.as_str()),
            status.stored_sync.last_records_transferred,
            status.stored_sync.last_rows_spooled,
            json_option_f64(status.stored_sync.last_sync_at),
            json_option_string(status.stored_sync.last_error.as_deref()),
        )
    }

    fn write_status(path: Option<&Path>, status: &mut LiveStatus) {
        let Some(path) = path else {
            return;
        };
        status.written_at = now_secs();
        let temporary = path.with_extension("json.tmp");
        if let Err(error) = std::fs::write(&temporary, status_json(status))
            .and_then(|()| std::fs::rename(&temporary, path))
        {
            eprintln!(
                "event=emay.status stage=status_publication outcome=error path={:?} error={:?}",
                path.display().to_string(),
                error.to_string()
            );
        }
    }

    async fn known_device(adapter: &Adapter, target: Address) -> CollectorResult<Device> {
        let device = adapter
            .device(target)
            .map_err(|error| format!("get known EMAY device {target}: {error}"))?;
        let connected = device
            .is_connected()
            .await
            .map_err(|error| format!("check EMAY connection {target}: {error}"))?;
        if !connected {
            device
                .connect()
                .await
                .map_err(|error| format!("connect to EMAY {target}: {error}"))?;
        }
        Ok(device)
    }

    async fn characteristics(
        device: &Device,
    ) -> CollectorResult<(
        bluer::gatt::remote::Characteristic,
        bluer::gatt::remote::Characteristic,
    )> {
        let mut write = None;
        let mut notify = None;
        for service in device
            .services()
            .await
            .map_err(|error| format!("discover EMAY services: {error}"))?
        {
            if service
                .uuid()
                .await
                .map_err(|error| format!("read EMAY service UUID: {error}"))?
                != SERVICE
            {
                continue;
            }
            for characteristic in service
                .characteristics()
                .await
                .map_err(|error| format!("discover EMAY characteristics: {error}"))?
            {
                match characteristic
                    .uuid()
                    .await
                    .map_err(|error| format!("read EMAY characteristic UUID: {error}"))?
                {
                    uuid if uuid == WRITE_CHAR => write = Some(characteristic),
                    uuid if uuid == NOTIFY_CHAR => notify = Some(characteristic),
                    _ => {}
                }
            }
        }
        match (write, notify) {
            (Some(write), Some(notify)) => Ok((write, notify)),
            _ => Err(
                "EMAY durable collector blocked: known device has no FF01 write and FF02 notify"
                    .to_owned(),
            ),
        }
    }

    async fn send_setup<W>(write: &W) -> CollectorResult<()>
    where
        W: CharacteristicWriter + ?Sized,
    {
        // These commands and their ordering are captured from the official
        // app. The clock command is a verified device protocol operation: the
        // descriptor has no timezone, so the device clock must be aligned with
        // the same host-local clock used when decoding its stored timestamps.
        let now = Local::now();
        let year = u8::try_from(
            now.year()
                .checked_sub(2000)
                .ok_or_else(|| "EMAY clock year is before the supported century".to_owned())?,
        )
        .map_err(|_| {
            format!(
                "EMAY clock year {} does not fit the device byte",
                now.year()
            )
        })?;
        let clock = storage::clock_command(
            year,
            u8::try_from(now.month()).map_err(|_| "EMAY clock month overflowed u8".to_owned())?,
            u8::try_from(now.day()).map_err(|_| "EMAY clock day overflowed u8".to_owned())?,
            u8::try_from(now.hour()).map_err(|_| "EMAY clock hour overflowed u8".to_owned())?,
            u8::try_from(now.minute()).map_err(|_| "EMAY clock minute overflowed u8".to_owned())?,
            u8::try_from(now.second()).map_err(|_| "EMAY clock second overflowed u8".to_owned())?,
        )
        .ok_or_else(|| "EMAY clock fields were outside the device range".to_owned())?;
        for command in [
            // The durable unit replaces the display-only stream. Explicitly
            // stop a prior live session so its EB notifications cannot be
            // mistaken for, or crowd out, the stored ED transfer.
            cmd::STOP_REALTIME.as_slice(),
            cmd::HELLO.as_slice(),
            cmd::DEVICE_STATE.as_slice(),
            clock.as_slice(),
            storage::PREPARE.as_slice(),
        ] {
            write
                .write(command)
                .await
                .map_err(|error| format!("EMAY setup write {}: {error}", hex(command)))?;
            setup_pause().await;
        }
        Ok(())
    }

    #[cfg(not(test))]
    async fn setup_pause() {
        tokio::time::sleep(Duration::from_millis(400)).await;
    }

    #[cfg(test)]
    fn setup_pause() -> std::future::Ready<()> {
        std::future::ready(())
    }

    fn hex(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    }

    async fn next_notification<S>(stream: &mut Pin<Box<S>>) -> CollectorResult<Vec<u8>>
    where
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        stream
            .next()
            .await
            .ok_or_else(|| "EMAY notification stream ended during storage download".to_owned())
    }

    /// Discards a valid anonymous live frame while storage responses are being
    /// assembled. Live bytes are display/journal-only and must not crowd out
    /// source-identified `EC`/`ED` records.
    fn discard_live_frame(pending: &mut Vec<u8>) -> bool {
        if pending.iter().any(|byte| *byte == 0xEC || *byte == 0xED) {
            return false;
        }
        let Some((_, used)) = scan_frame(pending) else {
            return false;
        };
        pending.drain(..used);
        true
    }

    /// Reads one EC descriptor from the mixed notification stream.
    async fn read_descriptor<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        pending: &mut Vec<u8>,
    ) -> CollectorResult<Option<SessionDescriptor>>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        write
            .write(&storage::LIST_NEXT)
            .await
            .map_err(|error| format!("request EMAY session descriptor: {error}"))?;
        loop {
            if let Some(position) = pending.iter().position(|byte| *byte == 0xEC) {
                if position > 0 {
                    pending.drain(..position);
                }
                if pending.len() >= SESSION_DESCRIPTOR_LEN {
                    let frame = pending
                        .get(..SESSION_DESCRIPTOR_LEN)
                        .ok_or_else(|| "EMAY EC descriptor truncated".to_owned())?;
                    if is_empty_session_descriptor(frame) {
                        if pending.get(SESSION_DESCRIPTOR_LEN) == Some(&0) {
                            pending.drain(..=SESSION_DESCRIPTOR_LEN);
                        } else {
                            pending.drain(..SESSION_DESCRIPTOR_LEN);
                        }
                        return Ok(None);
                    }
                    let descriptor = decode_session_descriptor(frame).ok_or_else(|| {
                        format!("EMAY EC descriptor failed validation: {}", hex(frame))
                    })?;
                    match pending.get(SESSION_DESCRIPTOR_LEN) {
                        Some(0) => {
                            pending.drain(..=SESSION_DESCRIPTOR_LEN);
                            return Ok(Some(descriptor));
                        }
                        // The live firmware can resume an EB stream without
                        // emitting the storage terminator seen in the iOS
                        // capture. The EC frame is still independently valid;
                        // preserve the EB byte for the next parser and make
                        // this framing degradation visible.
                        Some(0xEB) => {
                            eprintln!(
                                "event=emay.storage_descriptor stage=framing outcome=degraded source=emay slot={} reason=missing_zero_terminator next_opcode=0xeb",
                                descriptor.slot
                            );
                            pending.drain(..SESSION_DESCRIPTOR_LEN);
                            return Ok(Some(descriptor));
                        }
                        Some(_) => {
                            return Err(format!(
                                "EMAY EC descriptor missing stream terminator: {}",
                                hex(pending.get(..SESSION_DESCRIPTOR_LEN + 1).unwrap_or(&[]))
                            ));
                        }
                        None => {}
                    }
                }
            }
            if discard_live_frame(pending) {
                continue;
            }
            if pending.len() > (SESSION_DESCRIPTOR_LEN + 1) * 4 {
                return Err(format!(
                    "EMAY descriptor response exceeded {} buffered bytes without EC",
                    pending.len()
                ));
            }
            pending.extend_from_slice(&next_notification(stream).await?);
        }
    }

    #[derive(Debug)]
    enum MetricAttemptError {
        Timeout(Vec<StoredRecord>),
        Transport(String),
        Protocol(String),
    }

    /// Reads one metric's ED records until its zero stream terminator.
    ///
    /// A notification timeout is recoverable: the records already decoded are
    /// retained, the partial tail is discarded, and the next command resumes
    /// at the following source cursor. A host arrival counter is never used.
    async fn read_metric_attempt<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        pending: &mut Vec<u8>,
        metric: StoredMetric,
        slot: u8,
        start_cursor: u16,
    ) -> Result<Vec<StoredRecord>, MetricAttemptError>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        let command = storage::read_command(metric, slot, start_cursor).ok_or_else(|| {
            MetricAttemptError::Protocol(format!(
                "cannot build EMAY {metric:?} slot {slot} cursor {start_cursor:#06x} command"
            ))
        })?;
        write.write(&command).await.map_err(|error| {
            MetricAttemptError::Transport(format!(
                "request EMAY {metric:?} slot {slot} cursor {start_cursor:#06x}: {error}"
            ))
        })?;
        let mut records = Vec::new();
        loop {
            if pending.first() == Some(&0) {
                pending.drain(..1);
                if records.is_empty() {
                    // A descriptor-list timeout can leave its zero
                    // terminator queued in BlueZ. Do not let that stale byte
                    // make the following metric look empty; give the device a
                    // short chance to deliver the requested ED stream.
                    match tokio::time::timeout(EMPTY_STREAM_GRACE, next_notification(stream)).await
                    {
                        Ok(Ok(bytes)) => {
                            pending.extend_from_slice(&bytes);
                            continue;
                        }
                        Ok(Err(error)) => return Err(MetricAttemptError::Transport(error)),
                        Err(_) => {}
                    }
                }
                return Ok(records);
            }
            if let Some(position) = pending.iter().position(|byte| *byte == 0xED) {
                if position > 0 {
                    pending.drain(..position);
                }
                if pending.len() >= STORED_RECORD_LEN {
                    let frame = pending.get(..STORED_RECORD_LEN).ok_or_else(|| {
                        MetricAttemptError::Protocol("EMAY ED record truncated".to_owned())
                    })?;
                    let record = decode_stored_record(frame).ok_or_else(|| {
                        MetricAttemptError::Protocol(format!(
                            "EMAY ED record failed validation for {:?} slot {slot}: {}",
                            metric,
                            hex(frame)
                        ))
                    })?;
                    if record.metric != metric || record.slot != slot {
                        return Err(MetricAttemptError::Protocol(format!(
                            "EMAY ED record identity mismatch: expected {:?}/slot {}, got {:?}/slot {}",
                            metric, slot, record.metric, record.slot
                        )));
                    }
                    // A retry can leave already-delivered notifications queued
                    // in BlueZ. They are source-identifiable duplicates, so
                    // discard only cursors before this attempt's start; a jump
                    // forward remains an error below when rows are assembled.
                    if record.cursor >= start_cursor {
                        records.push(record);
                    }
                    pending.drain(..STORED_RECORD_LEN);
                    continue;
                }
            }
            if discard_live_frame(pending) {
                continue;
            }
            if pending.len() > STORED_RECORD_LEN * 4 {
                let preview_len = pending.len().min(128);
                return Err(MetricAttemptError::Protocol(format!(
                    "EMAY {metric:?} slot {slot} response exceeded {} buffered bytes without ED: {}",
                    pending.len(),
                    hex(pending.get(..preview_len).unwrap_or(&[]))
                )));
            }
            match tokio::time::timeout(RESPONSE_TIMEOUT, next_notification(stream)).await {
                Ok(Ok(bytes)) => pending.extend_from_slice(&bytes),
                Ok(Err(error)) => return Err(MetricAttemptError::Transport(error)),
                Err(_) => return Err(MetricAttemptError::Timeout(records)),
            }
        }
    }

    fn append_metric_records(
        all: &mut Vec<StoredRecord>,
        new_records: &[StoredRecord],
        metric: StoredMetric,
        slot: u8,
        expected_cursor: u16,
    ) -> CollectorResult<()> {
        let mut expected = expected_cursor;
        for record in new_records {
            if record.metric != metric || record.slot != slot {
                return Err(format!(
                    "EMAY {:?} slot {slot} retry returned {:?}/slot {}",
                    metric, record.metric, record.slot
                ));
            }
            if record.cursor != expected {
                return Err(format!(
                    "EMAY {:?} slot {slot} cursor jump: expected {expected:#06x}, got {:#06x}",
                    metric, record.cursor
                ));
            }
            all.push(*record);
            expected = next_storage_cursor(record.cursor).ok_or_else(|| {
                format!(
                    "EMAY {:?} slot {slot} cursor {:#06x} has no representable successor",
                    metric, record.cursor
                )
            })?;
        }
        Ok(())
    }

    #[derive(Debug, Clone, Copy)]
    struct StoredCursorPair {
        spo2: Option<u16>,
        pulse: Option<u16>,
    }

    impl Default for StoredCursorPair {
        fn default() -> Self {
            Self {
                spo2: Some(0),
                pulse: Some(0),
            }
        }
    }

    fn next_cursor_after(
        initial: Option<u16>,
        records: &[StoredRecord],
        metric: StoredMetric,
        slot: u8,
    ) -> CollectorResult<Option<u16>> {
        let Some(record) = records.last() else {
            return Ok(initial);
        };
        next_storage_cursor(record.cursor)
            .ok_or_else(|| {
                format!(
                    "EMAY {metric:?} slot {slot} cursor {:#06x} has no representable successor",
                    record.cursor
                )
            })
            .map(Some)
    }

    async fn read_metric_if_needed<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        pending: &mut Vec<u8>,
        metric: StoredMetric,
        slot: u8,
        start_cursor: Option<u16>,
    ) -> CollectorResult<Vec<StoredRecord>>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        let Some(start_cursor) = start_cursor else {
            return Ok(Vec::new());
        };
        read_metric(write, stream, pending, metric, slot, start_cursor).await
    }

    async fn read_metric<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        pending: &mut Vec<u8>,
        metric: StoredMetric,
        slot: u8,
        start_cursor: u16,
    ) -> CollectorResult<Vec<StoredRecord>>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        let mut records = Vec::new();
        let mut cursor = start_cursor;
        let mut retries = 0usize;
        loop {
            match read_metric_attempt(write, stream, pending, metric, slot, cursor).await {
                Ok(chunk) => {
                    append_metric_records(&mut records, &chunk, metric, slot, cursor)?;
                    return Ok(records);
                }
                Err(MetricAttemptError::Timeout(chunk)) => {
                    append_metric_records(&mut records, &chunk, metric, slot, cursor)?;
                    let next = match records.last() {
                        None => cursor,
                        Some(record) => next_storage_cursor(record.cursor).ok_or_else(|| {
                            format!(
                                "EMAY {:?} slot {slot} cursor {:#06x} has no representable successor after timeout",
                                metric, record.cursor
                            )
                        })?,
                    };
                    if retries >= STORAGE_TRANSFER_RETRIES {
                        return Err(format!(
                            "EMAY {:?} slot {slot} timed out after {} retries at cursor {next:#06x}; {} records received",
                            metric,
                            STORAGE_TRANSFER_RETRIES,
                            records.len()
                        ));
                    }
                    // Only the timed-out partial tail is discarded. A
                    // completed stream terminator may be followed by the
                    // next metric's ED records in the same BLE notification,
                    // and those bytes must remain queued for that metric.
                    pending.clear();
                    retries += 1;
                    cursor = next;
                    eprintln!(
                        "event=emay.storage_transfer stage=retry outcome=retry metric={metric:?} slot={slot} attempt={retries} max_attempts={STORAGE_TRANSFER_RETRIES} source_cursor={cursor:#06x}"
                    );
                }
                Err(MetricAttemptError::Transport(error) | MetricAttemptError::Protocol(error)) => {
                    return Err(error);
                }
            }
        }
    }

    fn pair_records(
        descriptor: &SessionDescriptor,
        spo2: &[StoredRecord],
        pulse: &[StoredRecord],
    ) -> CollectorResult<Vec<StoredRecord>> {
        if spo2.len() != pulse.len() {
            return Err(format!(
                "EMAY slot {} metric record counts differ: SpO2={}, pulse={}",
                descriptor.slot,
                spo2.len(),
                pulse.len()
            ));
        }
        for (spo2_record, pulse_record) in spo2.iter().zip(pulse.iter()) {
            if spo2_record.cursor != pulse_record.cursor {
                return Err(format!(
                    "EMAY slot {} metric cursor mismatch: SpO2={:#06x}, pulse={:#06x}",
                    descriptor.slot, spo2_record.cursor, pulse_record.cursor
                ));
            }
        }
        let mut records = Vec::with_capacity(spo2.len() + pulse.len());
        records.extend_from_slice(spo2);
        records.extend_from_slice(pulse);
        Ok(records)
    }

    #[derive(Debug, Clone, Copy)]
    struct AppendReceipt {
        count: usize,
        durable: bool,
    }

    #[derive(Debug)]
    struct StorageSyncProgress {
        records_transferred: usize,
        rows_spooled: usize,
        all_fsynced: bool,
    }

    impl StorageSyncProgress {
        fn new() -> Self {
            Self {
                records_transferred: 0,
                rows_spooled: 0,
                all_fsynced: true,
            }
        }

        fn record_batch(&mut self, receipt: AppendReceipt) -> CollectorResult<()> {
            self.rows_spooled = self
                .rows_spooled
                .checked_add(receipt.count)
                .ok_or_else(|| "EMAY spooled-row count overflowed usize".to_owned())?;
            self.all_fsynced &= receipt.durable;
            Ok(())
        }

        fn record_source_records(&mut self, count: usize) -> CollectorResult<()> {
            self.records_transferred = self
                .records_transferred
                .checked_add(count)
                .ok_or_else(|| "EMAY transferred-record count overflowed usize".to_owned())?;
            Ok(())
        }
    }

    #[derive(Debug)]
    struct StorageSyncReceipt {
        records_transferred: usize,
        rows_spooled: usize,
        all_fsynced: bool,
    }

    #[derive(Debug)]
    struct StorageSyncFailure {
        detail: String,
        records_transferred: usize,
        rows_spooled: usize,
        all_fsynced: bool,
    }

    fn storage_sync_receipt(progress: &StorageSyncProgress) -> StorageSyncReceipt {
        StorageSyncReceipt {
            records_transferred: progress.records_transferred,
            rows_spooled: progress.rows_spooled,
            all_fsynced: progress.all_fsynced,
        }
    }

    fn storage_sync_failure(progress: &StorageSyncProgress, detail: String) -> StorageSyncFailure {
        StorageSyncFailure {
            detail,
            records_transferred: progress.records_transferred,
            rows_spooled: progress.rows_spooled,
            all_fsynced: progress.all_fsynced,
        }
    }

    fn finish_storage_sync<E>(
        result: Result<(), String>,
        exit: Result<(), E>,
        progress: &StorageSyncProgress,
        updated_cursors: Vec<(i64, StoredCursorPair)>,
        cursors: &mut HashMap<i64, StoredCursorPair>,
    ) -> Result<StorageSyncReceipt, StorageSyncFailure>
    where
        E: std::fmt::Display,
    {
        match (result, exit) {
            (Ok(()), Ok(())) => {
                for (identity, cursor) in updated_cursors {
                    cursors.insert(identity, cursor);
                }
                Ok(storage_sync_receipt(progress))
            }
            (Ok(()), Err(error)) => Err(storage_sync_failure(
                progress,
                format!("leave EMAY storage mode: {error}"),
            )),
            (Err(error), Ok(())) => Err(storage_sync_failure(progress, error)),
            (Err(error), Err(exit_error)) => Err(storage_sync_failure(
                progress,
                format!("{error}; leave EMAY storage mode also failed: {exit_error}"),
            )),
        }
    }

    async fn append_rows(
        socket_path: &Path,
        rows: &[WireSample],
    ) -> CollectorResult<AppendReceipt> {
        if rows.is_empty() {
            return Ok(AppendReceipt {
                count: 0,
                durable: true,
            });
        }
        let mut socket = UnixStream::connect(socket_path)
            .await
            .map_err(|error| format!("connect to vitalsd at {}: {error}", socket_path.display()))?;
        let request = Request::Append {
            samples: rows.to_vec(),
            // EMAY retains the source records, so a reconnect can safely
            // re-download this batch. Leave fsync to vitalsd's periodic spool
            // sync; per-append fsync is reserved for unrecoverable BP frames.
            sync: false,
        };
        let frame =
            encode_frame(&request).map_err(|error| format!("encode EMAY append: {error}"))?;
        socket
            .write_all(&frame)
            .await
            .map_err(|error| format!("write EMAY append to vitalsd: {error}"))?;
        let mut prefix = [0u8; 4];
        socket
            .read_exact(&mut prefix)
            .await
            .map_err(|error| format!("read vitalsd append response length: {error}"))?;
        let length = u32::from_le_bytes(prefix);
        let length =
            check_frame_len(length).map_err(|error| format!("vitalsd response: {error}"))?;
        let mut body = vec![0u8; length];
        socket
            .read_exact(&mut body)
            .await
            .map_err(|error| format!("read vitalsd append response body: {error}"))?;
        match decode_payload::<Response>(&body)
            .map_err(|error| format!("decode vitalsd response: {error}"))?
        {
            Response::Appended { count, durable } if count == rows.len() => {
                eprintln!(
                    "event=emay.append stage=spooled outcome=ok source=emay type_first={} type_last={} session_id={} seq_first={} seq_last={} row_count={} fsynced={} identity=source_descriptor_cursor",
                    rows.first().map_or(0, |row| row.r#type),
                    rows.last().map_or(0, |row| row.r#type),
                    rows.first().map_or(0, |row| row.session_id),
                    rows.first().map_or(0, |row| row.seq),
                    rows.last().map_or(0, |row| row.seq),
                    count,
                    durable
                );
                Ok(AppendReceipt { count, durable })
            }
            Response::Appended { count, .. } => Err(format!(
                "vitalsd appended {count} EMAY rows, but collector sent {}",
                rows.len()
            )),
            Response::Error { kind, detail } => {
                Err(format!("vitalsd rejected EMAY append ({kind:?}): {detail}"))
            }
            other => Err(format!("unexpected vitalsd append response: {other:?}")),
        }
    }

    async fn append_rows_in_batches(
        socket_path: &Path,
        rows: &[WireSample],
        progress: &mut StorageSyncProgress,
    ) -> CollectorResult<()> {
        for batch in rows.chunks(APPEND_BATCH_SIZE) {
            let receipt = append_rows(socket_path, batch).await?;
            progress.record_batch(receipt)?;
        }
        Ok(())
    }

    async fn read_descriptors<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        pending: &mut Vec<u8>,
    ) -> CollectorResult<Vec<SessionDescriptor>>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        let mut descriptors = Vec::with_capacity(STORAGE_SLOTS);
        for _ in 0..STORAGE_SLOTS {
            match tokio::time::timeout(RESPONSE_TIMEOUT, read_descriptor(write, stream, pending))
                .await
            {
                Ok(Ok(Some(descriptor))) => {
                    if descriptors
                        .iter()
                        .any(|seen: &SessionDescriptor| seen.slot == descriptor.slot)
                    {
                        return Err(format!(
                            "EMAY stored-session descriptor repeated slot {}",
                            descriptor.slot
                        ));
                    }
                    descriptors.push(descriptor);
                }
                Ok(Ok(None)) => {
                    pending.clear();
                    if descriptors.is_empty() {
                        eprintln!(
                            "event=emay.storage_sync stage=descriptor_list outcome=empty source=emay reason=device_reported_no_stored_sessions"
                        );
                    }
                    break;
                }
                Ok(Err(error)) => {
                    pending.clear();
                    return Err(format!(
                        "EMAY stored-session descriptor list unavailable after {} descriptors: {error}",
                        descriptors.len()
                    ));
                }
                Err(_) => {
                    pending.clear();
                    return Err(format!(
                        "EMAY stored-session descriptor {} timed out after {}s",
                        descriptors.len() + 1,
                        RESPONSE_TIMEOUT.as_secs()
                    ));
                }
            }
        }
        Ok(descriptors)
    }

    async fn storage_sync<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        pending: &mut Vec<u8>,
        socket_path: &Path,
        cursors: &mut HashMap<i64, StoredCursorPair>,
    ) -> Result<StorageSyncReceipt, StorageSyncFailure>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        let mut progress = StorageSyncProgress::new();
        let mut updated_cursors = Vec::with_capacity(STORAGE_SLOTS);
        let result = tokio::time::timeout(STORAGE_SYNC_DEADLINE, async {
            send_setup(write).await?;
            let descriptors = read_descriptors(write, stream, pending).await?;
            for descriptor in descriptors {
                let identity = emay_collector::session_id(&descriptor);
                let cursor = cursors.get(&identity).copied().unwrap_or_default();
                eprintln!(
                    "event=emay.storage_session stage=received outcome=ok source=emay session_id={identity} slot={} identity=source_descriptor_cursor spo2_start_cursor={:?} pulse_start_cursor={:?}",
                    descriptor.slot, cursor.spo2, cursor.pulse
                );
                let spo2 = read_metric_if_needed(
                    write,
                    stream,
                    pending,
                    StoredMetric::Spo2,
                    descriptor.slot,
                    cursor.spo2,
                )
                .await?;
                let pulse = read_metric_if_needed(
                    write,
                    stream,
                    pending,
                    StoredMetric::Pulse,
                    descriptor.slot,
                    cursor.pulse,
                )
                .await?;
                let records = pair_records(&descriptor, &spo2, &pulse)?;
                progress.record_source_records(records.len())?;
                eprintln!(
                    "event=emay.stored_records stage=received outcome=accepted source=emay session_id={identity} type_first={} type_last={} seq_first={} seq_last={} records={} identity=source_descriptor_cursor",
                    records.first().map_or(0, |record| record.metric.code()),
                    records.last().map_or(0, |record| record.metric.code()),
                    records.first().map_or(0, |record| record.cursor),
                    records.last().map_or(0, |record| record.cursor),
                    records.len()
                );
                let rows = wire_samples(&descriptor, &records)?;
                append_rows_in_batches(socket_path, &rows, &mut progress).await?;
                let next = StoredCursorPair {
                    spo2: next_cursor_after(
                        cursor.spo2,
                        &spo2,
                        StoredMetric::Spo2,
                        descriptor.slot,
                    )?,
                    pulse: next_cursor_after(
                        cursor.pulse,
                        &pulse,
                        StoredMetric::Pulse,
                        descriptor.slot,
                    )?,
                };
                // Do not advance the in-process cursor until the whole
                // exit acknowledgement fails, the next attempt must replay
                // the source records rather than trusting an incomplete
                // transfer; source identity makes that replay idempotent.
                updated_cursors.push((identity, next));
                eprintln!(
                    "event=emay.storage_session stage=spooled outcome=ok source=emay session_id={identity} slot={} records={} rows={} next_spo2_cursor={:?} next_pulse_cursor={:?}",
                    descriptor.slot,
                    records.len(),
                    rows.len(),
                    next.spo2,
                    next.pulse
                );
            }
            Ok(())
        })
        .await;
        let result = match result {
            Ok(result) => result,
            Err(_) => Err(format!(
                "EMAY stored-session transfer exceeded {}s deadline",
                STORAGE_SYNC_DEADLINE.as_secs()
            )),
        };

        finish_storage_sync(
            result,
            write.write(&storage::EXIT).await,
            &progress,
            updated_cursors,
            cursors,
        )
    }

    fn drain_live(
        bytes: &[u8],
        pending: &mut Vec<u8>,
        status: &mut LiveStatus,
        status_path: Option<&Path>,
    ) {
        pending.extend_from_slice(bytes);
        let mut consumed = 0usize;
        while let Some((reading, used)) = pending.get(consumed..).and_then(scan_frame) {
            consumed += used;
            status.frames = status.frames.saturating_add(1);
            if reading.is_measurement() {
                status.measurements = status.measurements.saturating_add(1);
            } else {
                status.no_finger = status.no_finger.saturating_add(1);
            }
            let sample_at = now_secs();
            status.sample_at = Some(sample_at);
            status.spo2 = reading.spo2;
            status.pulse_bpm = reading.pulse_bpm;
            status.finger = reading.is_measurement();
            eprintln!(
                "event=emay.live stage=received outcome=accepted source=emay type_first={} type_last={} session_id=none seq=none event_time_unix_secs={sample_at:.3} measurement={} spo2={:?} pulse_bpm={:?} durability=status_only_not_committed received_total={}",
                VitalsType::Spo2.id(),
                VitalsType::Hr.id(),
                reading.is_measurement(),
                reading.spo2,
                reading.pulse_bpm,
                status.frames
            );
            write_status(status_path, status);
        }
        pending.drain(..consumed);
        if pending.len() > 32 {
            status.undecodable = status.undecodable.saturating_add(1);
            eprintln!(
                "event=emay.live stage=decode outcome=rejected source=emay identity=anonymous_not_durable bytes={} rejected_frames={}",
                pending.len(),
                status.undecodable
            );
            pending.clear();
            write_status(status_path, status);
        }
    }

    fn apply_storage_sync_result(
        status: &mut LiveStatus,
        result: Result<StorageSyncReceipt, StorageSyncFailure>,
    ) -> CollectorResult<()> {
        match result {
            Ok(receipt) => {
                let records = u64::try_from(receipt.records_transferred)
                    .map_err(|_| "EMAY transferred-record count exceeded u64".to_owned())?;
                let rows = u64::try_from(receipt.rows_spooled)
                    .map_err(|_| "EMAY spooled-row count exceeded u64".to_owned())?;
                status
                    .stored_sync
                    .complete(now_secs(), records, rows, receipt.all_fsynced);
                eprintln!(
                    "event=emay.storage_sync stage=transfer outcome=complete state={} records_transferred={} rows_spooled={} all_fsynced={} durability={}",
                    status.stored_sync.transfer.as_str(),
                    records,
                    rows,
                    receipt.all_fsynced,
                    status.stored_sync.durability.as_str()
                );
            }
            Err(failure) => {
                let records = u64::try_from(failure.records_transferred)
                    .map_err(|_| "EMAY transferred-record count exceeded u64".to_owned())?;
                let rows = u64::try_from(failure.rows_spooled)
                    .map_err(|_| "EMAY spooled-row count exceeded u64".to_owned())?;
                eprintln!(
                    "event=emay.storage_sync stage=transfer outcome=failed state={} records_transferred={} rows_spooled={} all_fsynced={} error={:?}",
                    StoredTransferState::Failed.as_str(),
                    records,
                    rows,
                    failure.all_fsynced,
                    failure.detail
                );
                status.stored_sync.fail(
                    now_secs(),
                    records,
                    rows,
                    failure.all_fsynced,
                    failure.detail,
                );
            }
        }
        Ok(())
    }

    async fn run_storage_sync<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        pending: &mut Vec<u8>,
        socket_path: &Path,
        status_path: Option<&Path>,
        status: &mut LiveStatus,
        cursors: &mut HashMap<i64, StoredCursorPair>,
    ) -> CollectorResult<()>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        status.stored_sync.begin();
        eprintln!(
            "event=emay.storage_sync stage=transfer outcome=started state={} identity={}",
            status.stored_sync.transfer.as_str(),
            status.stored_sync.identity.as_str()
        );
        write_status(status_path, status);
        let result = storage_sync(write, stream, pending, socket_path, cursors).await;
        apply_storage_sync_result(status, result)?;
        write_status(status_path, status);
        Ok(())
    }

    async fn start_live<W>(write: &W) -> CollectorResult<()>
    where
        W: CharacteristicWriter + ?Sized,
    {
        for command in [
            cmd::HELLO.as_slice(),
            cmd::DEVICE_STATE.as_slice(),
            cmd::START_REALTIME.as_slice(),
            cmd::BATTERY.as_slice(),
        ] {
            write.write(command).await.map_err(|error| {
                format!("start live EMAY stream with {}: {error}", hex(command))
            })?;
            setup_pause().await;
        }
        Ok(())
    }

    async fn live<W, S>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        status_path: Option<&Path>,
        socket_path: &Path,
        status: &mut LiveStatus,
        pending: &mut Vec<u8>,
        cursors: &mut HashMap<i64, StoredCursorPair>,
    ) -> CollectorResult<()>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
    {
        live_with_shutdown(
            write,
            stream,
            status_path,
            socket_path,
            status,
            pending,
            cursors,
            tokio::signal::ctrl_c(),
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the live loop keeps all mutable protocol state at one boundary"
    )]
    async fn live_with_shutdown<W, S, F>(
        write: &W,
        stream: &mut Pin<Box<S>>,
        status_path: Option<&Path>,
        socket_path: &Path,
        status: &mut LiveStatus,
        pending: &mut Vec<u8>,
        cursors: &mut HashMap<i64, StoredCursorPair>,
        shutdown: F,
    ) -> CollectorResult<()>
    where
        W: CharacteristicWriter + ?Sized,
        S: Stream<Item = Vec<u8>> + ?Sized,
        F: Future<Output = std::io::Result<()>>,
    {
        let mut shutdown = Box::pin(shutdown);
        if let Err(error) = start_live(write).await {
            status.link = "lost";
            write_status(status_path, status);
            return Err(format!("start live EMAY stream: {error}"));
        }
        write_status(status_path, status);
        let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
        // The first interval tick is immediate. The connection just completed
        // its setup, so wait for the first real keepalive period instead of
        // issuing a duplicate command in the same turn.
        keepalive.tick().await;
        let mut storage_sync_tick = tokio::time::interval(STORAGE_SYNC_INTERVAL);
        storage_sync_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // `interval` ticks immediately on its first call. The initial sync was
        // completed by `run`, so consume that tick before waiting for the
        // bounded periodic transfer interval.
        storage_sync_tick.tick().await;
        let mut last_frame = tokio::time::Instant::now();
        loop {
            tokio::select! {
                item = next_notification(stream) => {
                    let bytes = match item {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            status.link = "lost";
                            write_status(status_path, status);
                            return Err(error);
                        }
                    };
                    let before = status.frames;
                    drain_live(&bytes, pending, status, status_path);
                    if status.frames != before {
                        last_frame = tokio::time::Instant::now();
                    }
                }
                _ = keepalive.tick() => {
                    if let Err(error) = write.write(&cmd::KEEPALIVE).await {
                        status.link = "lost";
                        write_status(status_path, status);
                        return Err(format!(
                            "EMAY live keepalive failed; link is gone: {error}"
                        ));
                    }
                    write_status(status_path, status);
                    if last_frame.elapsed() > SILENCE_LIMIT {
                        status.link = "lost";
                        write_status(status_path, status);
                        return Err(format!(
                            "EMAY live stream stale for {}s while keepalives succeeded",
                            last_frame.elapsed().as_secs()
                        ));
                    }
                }
                _ = storage_sync_tick.tick() => {
                    run_storage_sync(
                        write,
                        stream,
                        pending,
                        socket_path,
                        status_path,
                        status,
                        cursors,
                    ).await?;
                    start_live(write).await.map_err(|error| {
                        status.link = "lost";
                        write_status(status_path, status);
                        format!("resume live EMAY stream after storage sync: {error}")
                    })?;
                    last_frame = tokio::time::Instant::now();
                    write_status(status_path, status);
                }
                interrupted = &mut shutdown => {
                    interrupted.map_err(|error| format!("wait for EMAY shutdown signal: {error}"))?;
                    let _ = write.write(&cmd::STOP_REALTIME).await;
                    status.link = "lost";
                    write_status(status_path, status);
                    return Ok(());
                }
            }
        }
    }

    struct BluezBackend;

    impl CollectorBackend for BluezBackend {
        type Writer = BluezWriter;

        async fn connect(
            &self,
            target: Address,
        ) -> CollectorResult<(Self::Writer, NotificationStream)> {
            let session = bluer::Session::new()
                .await
                .map_err(|error| format!("open BlueZ session: {error}"))?;
            let adapter = session
                .default_adapter()
                .await
                .map_err(|error| format!("get default BlueZ adapter: {error}"))?;
            adapter
                .set_powered(true)
                .await
                .map_err(|error| format!("power BlueZ adapter {}: {error}", adapter.name()))?;
            let device = known_device(&adapter, target).await?;
            let (write, notify) = characteristics(&device).await?;
            let stream = Box::pin(
                notify
                    .notify()
                    .await
                    .map_err(|error| format!("subscribe to EMAY FF02: {error}"))?,
            );
            Ok((BluezWriter(write), stream))
        }
    }

    async fn run_with_backend<B>(
        backend: &B,
        mac: &str,
        socket_path: &Path,
        status_path: Option<&Path>,
    ) -> CollectorResult<()>
    where
        B: CollectorBackend,
    {
        let mut status = LiveStatus::new(StoredSyncState::new());
        status.link = "connecting";
        write_status(status_path, &mut status);
        let target = mac
            .parse::<Address>()
            .map_err(|error| format!("{mac:?} is not a Bluetooth address: {error}"))?;
        let (write, mut stream) = backend.connect(target).await?;
        let mut pending = Vec::with_capacity(256);
        let mut cursors = HashMap::new();
        status.link = "up";
        write_status(status_path, &mut status);
        run_storage_sync(
            &write,
            &mut stream,
            &mut pending,
            socket_path,
            status_path,
            &mut status,
            &mut cursors,
        )
        .await?;
        live(
            &write,
            &mut stream,
            status_path,
            socket_path,
            &mut status,
            &mut pending,
            &mut cursors,
        )
        .await
    }

    pub async fn run(
        mac: &str,
        socket_path: &Path,
        status_path: Option<&Path>,
    ) -> CollectorResult<()> {
        run_with_backend(&BluezBackend, mac, socket_path, status_path).await
    }

    #[cfg(test)]
    #[expect(
        clippy::expect_used,
        reason = "Linux collector fixtures must fail loudly when malformed"
    )]
    mod tests {
        use super::*;
        use futures::stream;
        use std::cell::RefCell;
        use std::path::PathBuf;

        struct TestWriter {
            calls: RefCell<Vec<Vec<u8>>>,
            fail: bool,
            fail_after: Option<usize>,
        }

        impl TestWriter {
            fn ok() -> Self {
                Self {
                    calls: RefCell::new(Vec::new()),
                    fail: false,
                    fail_after: None,
                }
            }

            fn failing() -> Self {
                Self {
                    calls: RefCell::new(Vec::new()),
                    fail: true,
                    fail_after: None,
                }
            }

            fn fail_after(writes: usize) -> Self {
                Self {
                    calls: RefCell::new(Vec::new()),
                    fail: false,
                    fail_after: Some(writes),
                }
            }
        }

        impl CharacteristicWriter for TestWriter {
            async fn write(&self, bytes: &[u8]) -> bluer::Result<()> {
                let failed = self.fail
                    || self
                        .fail_after
                        .is_some_and(|limit| self.calls.borrow().len() >= limit);
                self.calls.borrow_mut().push(bytes.to_vec());
                if failed {
                    Err(bluer::Error {
                        kind: bluer::ErrorKind::Failed,
                        message: "test writer failure".to_owned(),
                    })
                } else {
                    Ok(())
                }
            }
        }

        struct TestBackend {
            fail: bool,
        }

        impl CollectorBackend for TestBackend {
            type Writer = TestWriter;

            async fn connect(
                &self,
                _target: Address,
            ) -> CollectorResult<(Self::Writer, NotificationStream)> {
                if self.fail {
                    return Err("test backend blocked: no verified radio".to_owned());
                }
                Ok((
                    TestWriter::ok(),
                    Box::pin(stream::iter([empty_descriptor()])),
                ))
            }
        }

        fn checksum(frame: &[u8]) -> u8 {
            u8::try_from(frame.iter().fold(0u32, |sum, byte| sum + u32::from(*byte)) & 0x7f)
                .expect("masked checksum fits u8")
        }

        fn test_directory(label: &str) -> PathBuf {
            let path = std::env::temp_dir()
                .join(format!("anxiety-watch-emay-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir(&path).expect("test directory");
            path
        }

        fn wire_sample() -> WireSample {
            WireSample {
                source: 1,
                r#type: 1,
                session_id: 1,
                seq: 1,
                timestamp_unix_micros: 0,
                value: 1.0,
                extra: None,
                batch_id: None,
            }
        }

        async fn serve_append(
            listener: tokio::net::UnixListener,
            response: Response,
            expected_count: usize,
        ) -> Result<(), String> {
            use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

            let (mut socket, _) = listener
                .accept()
                .await
                .map_err(|error| format!("accept test socket: {error}"))?;
            let mut prefix = [0u8; 4];
            socket
                .read_exact(&mut prefix)
                .await
                .map_err(|error| format!("read test request length: {error}"))?;
            let length = u32::from_le_bytes(prefix);
            let length =
                check_frame_len(length).map_err(|error| format!("test request: {error}"))?;
            let mut body = vec![0u8; length];
            socket
                .read_exact(&mut body)
                .await
                .map_err(|error| format!("read test request body: {error}"))?;
            let request = decode_payload::<Request>(&body)
                .map_err(|error| format!("decode test request: {error}"))?;
            match request {
                Request::Append { samples, sync } if samples.len() == expected_count && !sync => {}
                other => return Err(format!("unexpected test request: {other:?}")),
            }
            let frame = encode_frame(&response)
                .map_err(|error| format!("encode test response: {error}"))?;
            socket
                .write_all(&frame)
                .await
                .map_err(|error| format!("write test response: {error}"))?;
            Ok(())
        }

        async fn append_with_response(
            label: &str,
            response: Response,
            rows: Vec<WireSample>,
        ) -> CollectorResult<AppendReceipt> {
            let dir = test_directory(label);
            let socket = dir.join("vitalsd.sock");
            let listener = tokio::net::UnixListener::bind(&socket)
                .map_err(|error| format!("bind test socket: {error}"))?;
            let server = serve_append(listener, response, rows.len());
            let (result, server_result) = tokio::join!(append_rows(&socket, &rows), server);
            server_result?;
            result
        }

        fn descriptor(slot: u8, terminator: Option<u8>) -> Vec<u8> {
            let mut frame = vec![0u8; SESSION_DESCRIPTOR_LEN];
            *frame.get_mut(0).expect("opcode") = 0xec;
            *frame.get_mut(1).expect("flags") = 0x01;
            *frame.get_mut(2).expect("selector") = 0x01;
            *frame.get_mut(3).expect("slot") = slot;
            *frame.get_mut(4).expect("year") = 26;
            *frame.get_mut(5).expect("month") = 9;
            *frame.get_mut(6).expect("day") = 5;
            *frame.get_mut(7).expect("hour") = 18;
            *frame.get_mut(8).expect("minute") = 0;
            *frame.get_mut(9).expect("second") = 0;
            let cks = checksum(frame.get(..20).expect("descriptor payload"));
            *frame.last_mut().expect("descriptor checksum") = cks;
            if let Some(byte) = terminator {
                frame.push(byte);
            }
            frame
        }

        fn empty_descriptor() -> Vec<u8> {
            let mut frame = vec![0u8; SESSION_DESCRIPTOR_LEN];
            *frame.get_mut(0).expect("opcode") = 0xec;
            let cks = checksum(frame.get(..20).expect("payload"));
            *frame.last_mut().expect("checksum") = cks;
            frame.push(0);
            frame
        }

        fn record(metric: StoredMetric, slot: u8, cursor: u16, value: u8) -> Vec<u8> {
            let mut frame = vec![0u8; STORED_RECORD_LEN];
            *frame.get_mut(0).expect("opcode") = 0xed;
            *frame.get_mut(1).expect("version") = 0x01;
            *frame.get_mut(2).expect("metric") = metric.code();
            *frame.get_mut(3).expect("selector") = 0x01;
            *frame.get_mut(4).expect("slot") = slot;
            let [lo, hi] = cursor.to_le_bytes();
            *frame.get_mut(5).expect("cursor low") = lo;
            *frame.get_mut(6).expect("cursor high") = hi;
            *frame.get_mut(9).expect("value") = value;
            *frame.last_mut().expect("record checksum") =
                checksum(frame.get(..STORED_RECORD_LEN - 1).expect("record payload"));
            frame
        }

        fn live_frame(pulse: u8, spo2: u8) -> Vec<u8> {
            let mut frame = vec![0xeb, 0x01, 0x05, pulse, spo2, 0x7f, 0x00, 0x00];
            let cks = checksum(frame.get(..7).expect("live payload"));
            *frame.last_mut().expect("live checksum") = cks;
            frame
        }

        fn decoded_descriptor(slot: u8) -> SessionDescriptor {
            decode_session_descriptor(
                descriptor(slot, None)
                    .get(..SESSION_DESCRIPTOR_LEN)
                    .expect("descriptor bytes"),
            )
            .expect("valid descriptor")
        }

        fn decoded_record(metric: StoredMetric, slot: u8, cursor: u16) -> StoredRecord {
            decode_stored_record(
                record(metric, slot, cursor, 96)
                    .get(..STORED_RECORD_LEN)
                    .expect("record bytes"),
            )
            .expect("valid record")
        }

        #[test]
        fn status_and_live_frames_keep_presence_explicit() {
            let mut status = LiveStatus::new(StoredSyncState::new());
            let json = status_json(&status);
            assert!(json.contains("\"spo2\":null"), "{json}");
            assert!(
                json.contains("\"stored_identity\":\"source_descriptor_cursor\""),
                "{json}"
            );

            let temp = test_directory("status");
            let path = temp.join("status.json");
            write_status(Some(&path), &mut status);
            assert!(path.is_file());
            write_status(None, &mut status);
            write_status(Some(&temp), &mut status);
            status.stored_sync.last_error =
                Some("quote=\" slash=\\ line=\n tab=\t back=\u{0008} form=\u{000c} carriage=\r ctrl=\u{0001}".to_owned());
            let escaped = status_json(&status);
            assert!(escaped.contains("\\\""), "{escaped}");
            assert!(escaped.contains("\\\\"), "{escaped}");
            assert!(escaped.contains("\\n"), "{escaped}");
            assert!(escaped.contains("\\t"), "{escaped}");
            assert!(escaped.contains("\\b"), "{escaped}");
            assert!(escaped.contains("\\f"), "{escaped}");
            assert!(escaped.contains("\\r"), "{escaped}");
            assert!(escaped.contains("\\u0001"), "{escaped}");

            let mut pending = live_frame(58, 97);
            pending.extend_from_slice(&live_frame(0, 0));
            drain_live(&pending, &mut Vec::new(), &mut status, None);
            assert_eq!(status.frames, 2);
            assert_eq!(status.measurements, 1);
            assert_eq!(status.no_finger, 1);
            assert_eq!(status.spo2, None);
            assert_eq!(status.pulse_bpm, None);

            let mut garbage = vec![0xaa; 33];
            drain_live(&garbage, &mut Vec::new(), &mut status, None);
            assert_eq!(status.undecodable, 1);
            garbage.clear();
            assert_eq!(hex(&[0x01, 0xab, 0xff]), "01:ab:ff");
        }

        #[tokio::test(flavor = "current_thread")]
        async fn setup_and_live_start_report_writer_failures() {
            let writer = TestWriter::ok();
            send_setup(&writer).await.expect("setup writes");
            assert_eq!(writer.calls.borrow().len(), 5);
            start_live(&writer).await.expect("live start writes");
            assert_eq!(writer.calls.borrow().len(), 9);

            let failing = TestWriter::failing();
            let error = send_setup(&failing).await.expect_err("setup must fail");
            assert!(error.contains("EMAY setup write"), "{error}");
            let error = start_live(&failing)
                .await
                .expect_err("live start must fail");
            assert!(error.contains("start live EMAY stream"), "{error}");
        }

        #[tokio::test(flavor = "current_thread")]
        async fn descriptor_reading_handles_terminators_live_bytes_and_errors() {
            let writer = TestWriter::ok();
            let mut no_terminator = empty_descriptor();
            no_terminator.pop();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_descriptor(&writer, &mut stream, &mut no_terminator)
                    .await
                    .expect("empty descriptor without terminator")
                    .is_none()
            );
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_descriptor(&TestWriter::failing(), &mut stream, &mut Vec::new())
                    .await
                    .is_err()
            );

            let mut pending = descriptor(1, Some(0));
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let found = read_descriptor(&writer, &mut stream, &mut pending)
                .await
                .expect("descriptor");
            assert_eq!(found.expect("descriptor").slot, 1);
            assert!(pending.is_empty());

            let mut pending = vec![0xaa];
            pending.extend_from_slice(&descriptor(4, Some(0)));
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert_eq!(
                read_descriptor(&writer, &mut stream, &mut pending)
                    .await
                    .expect("descriptor after prefix")
                    .expect("descriptor")
                    .slot,
                4
            );

            let mut malformed = descriptor(1, Some(0));
            if let Some(checksum) = malformed.get_mut(SESSION_DESCRIPTOR_LEN - 1) {
                *checksum ^= 1;
            }
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_descriptor(&writer, &mut stream, &mut malformed)
                    .await
                    .expect_err("malformed descriptor")
                    .contains("failed validation")
            );

            let mut pending = descriptor(2, Some(0xeb));
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert_eq!(
                read_descriptor(&writer, &mut stream, &mut pending)
                    .await
                    .expect("degraded descriptor")
                    .expect("descriptor")
                    .slot,
                2
            );

            let mut pending = empty_descriptor();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_descriptor(&writer, &mut stream, &mut pending)
                    .await
                    .expect("empty descriptor")
                    .is_none()
            );

            let mut pending = descriptor(3, Some(1));
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let error = read_descriptor(&writer, &mut stream, &mut pending)
                .await
                .expect_err("bad terminator");
            assert!(error.contains("missing stream terminator"), "{error}");

            let mut pending = live_frame(60, 98);
            let mut stream = Box::pin(stream::iter([descriptor(4, Some(0))]));
            assert_eq!(
                read_descriptor(&writer, &mut stream, &mut pending)
                    .await
                    .expect("descriptor after live")
                    .expect("descriptor")
                    .slot,
                4
            );
            assert!(pending.is_empty());

            let mut pending = vec![0xaa; (SESSION_DESCRIPTOR_LEN + 1) * 4 + 1];
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let error = read_descriptor(&writer, &mut stream, &mut pending)
                .await
                .expect_err("oversized response");
            assert!(error.contains("exceeded"), "{error}");

            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let error = read_descriptor(&writer, &mut stream, &mut pending)
                .await
                .expect_err("ended response");
            assert!(error.contains("notification stream ended"), "{error}");
        }

        #[tokio::test(flavor = "current_thread")]
        async fn descriptor_lists_stop_at_empty_and_reject_duplicates() {
            let writer = TestWriter::ok();
            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::iter([descriptor(1, Some(0)), empty_descriptor()]));
            let descriptors = read_descriptors(&writer, &mut stream, &mut pending)
                .await
                .expect("descriptor list");
            assert_eq!(descriptors.len(), 1);

            let mut pending = descriptor(1, Some(0));
            let mut stream = Box::pin(stream::iter([descriptor(1, Some(0))]));
            let error = read_descriptors(&writer, &mut stream, &mut pending)
                .await
                .expect_err("duplicate descriptor");
            assert!(error.contains("repeated slot"), "{error}");

            let mut malformed = descriptor(2, Some(0));
            if let Some(checksum) = malformed.get_mut(SESSION_DESCRIPTOR_LEN - 1) {
                *checksum ^= 1;
            }
            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::iter([malformed]));
            let error = read_descriptors(&writer, &mut stream, &mut pending)
                .await
                .expect_err("malformed descriptor list");
            assert!(error.contains("descriptor list unavailable"), "{error}");

            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::pending::<Vec<u8>>());
            let error = read_descriptors(&writer, &mut stream, &mut pending)
                .await
                .expect_err("descriptor timeout");
            assert!(error.contains("timed out"), "{error}");
        }

        #[tokio::test(flavor = "current_thread")]
        #[expect(
            clippy::too_many_lines,
            reason = "the test exercises the complete metric retry and validation matrix"
        )]
        async fn metric_reading_preserves_source_cursor_and_rejects_bad_identity() {
            let writer = TestWriter::ok();
            let mut pending = record(StoredMetric::Spo2, 1, 0, 96);
            pending.push(0);
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let records =
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 0)
                    .await
                    .expect("metric");
            assert_eq!(records.len(), 1);
            assert_eq!(records.first().expect("record").cursor, 0);

            let mut pending = record(StoredMetric::Pulse, 1, 0, 60);
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let error =
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 0)
                    .await
                    .expect_err("metric identity");
            assert!(matches!(error, MetricAttemptError::Protocol(_)));

            let mut pending = vec![0xaa; STORED_RECORD_LEN * 4 + 1];
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let error =
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 0)
                    .await
                    .expect_err("oversized metric response");
            assert!(matches!(error, MetricAttemptError::Protocol(_)));

            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let error =
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 0)
                    .await
                    .expect_err("ended metric response");
            assert!(matches!(error, MetricAttemptError::Transport(_)));

            let mut pending = record(StoredMetric::Spo2, 1, 0, 96);
            pending.push(0);
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let records = read_metric_if_needed(
                &writer,
                &mut stream,
                &mut pending,
                StoredMetric::Spo2,
                1,
                Some(0),
            )
            .await
            .expect("metric wrapper");
            assert_eq!(records.len(), 1);

            let mut pending = vec![0];
            let mut stream = Box::pin(stream::iter([{
                let mut bytes = record(StoredMetric::Spo2, 1, 0, 96);
                bytes.push(0);
                bytes
            }]));
            let records =
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 0)
                    .await
                    .expect("stale terminator before metric");
            assert_eq!(records.len(), 1);

            let mut prefixed = vec![0xaa];
            prefixed.extend_from_slice(&record(StoredMetric::Spo2, 1, 0, 96));
            prefixed.push(0);
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let records = read_metric_attempt(
                &writer,
                &mut stream,
                &mut prefixed,
                StoredMetric::Spo2,
                1,
                0,
            )
            .await
            .expect("metric after garbage prefix");
            assert_eq!(records.len(), 1);

            let mut malformed = record(StoredMetric::Spo2, 1, 0, 96);
            if let Some(checksum) = malformed.last_mut() {
                *checksum ^= 1;
            }
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(matches!(
                read_metric_attempt(
                    &writer,
                    &mut stream,
                    &mut malformed,
                    StoredMetric::Spo2,
                    1,
                    0,
                )
                .await,
                Err(MetricAttemptError::Protocol(_))
            ));

            let mut pending = record(StoredMetric::Spo2, 1, 0, 96);
            pending.push(0);
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(matches!(
                read_metric_attempt(
                    &TestWriter::failing(),
                    &mut stream,
                    &mut pending,
                    StoredMetric::Spo2,
                    1,
                    0,
                )
                .await,
                Err(MetricAttemptError::Transport(_))
            ));

            let mut pending = record(StoredMetric::Spo2, 1, 0, 96);
            pending.push(0);
            let mut stream = Box::pin(stream::iter([
                record(StoredMetric::Spo2, 1, 1, 96),
                vec![0],
            ]));
            let records =
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 1)
                    .await
                    .expect("old source cursor is discarded");
            assert_eq!(records.len(), 1);
            assert_eq!(records.first().expect("new record").cursor, 1);

            let first = record(StoredMetric::Spo2, 1, 0, 96);
            let mut stream = Box::pin(stream::iter([first]).chain(stream::pending::<Vec<u8>>()));
            let error = read_metric(
                &writer,
                &mut stream,
                &mut Vec::new(),
                StoredMetric::Spo2,
                1,
                0,
            )
            .await
            .expect_err("repeated source timeout");
            assert!(error.contains("timed out after 3 retries"), "{error}");

            let error = read_metric(
                &TestWriter::failing(),
                &mut Box::pin(stream::empty::<Vec<u8>>()),
                &mut Vec::new(),
                StoredMetric::Spo2,
                1,
                0,
            )
            .await
            .expect_err("metric transport error");
            assert!(error.contains("request EMAY"), "{error}");

            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_metric_if_needed(
                    &writer,
                    &mut stream,
                    &mut pending,
                    StoredMetric::Spo2,
                    1,
                    None,
                )
                .await
                .expect("missing cursor")
                .is_empty()
            );

            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::iter([{
                let mut bytes = record(StoredMetric::Spo2, 1, 0, 96);
                bytes.push(0);
                bytes
            }]));
            let records =
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 0)
                    .await
                    .expect("stream metric");
            assert_eq!(records.len(), 1);

            let mut pending = live_frame(60, 98);
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_metric_attempt(&writer, &mut stream, &mut pending, StoredMetric::Spo2, 1, 0,)
                    .await
                    .is_err()
            );

            let mut pending = descriptor(1, None);
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_descriptor(&writer, &mut stream, &mut pending)
                    .await
                    .is_err()
            );

            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            assert!(
                read_metric_attempt(
                    &writer,
                    &mut stream,
                    &mut pending,
                    StoredMetric::Spo2,
                    1,
                    0x0080,
                )
                .await
                .is_err()
            );
        }

        #[test]
        fn record_pairing_progress_and_finish_name_each_failure() {
            let descriptor = decoded_descriptor(1);
            let spo2 = [decoded_record(StoredMetric::Spo2, 1, 0)];
            let pulse = [decoded_record(StoredMetric::Pulse, 1, 0)];
            assert_eq!(
                pair_records(&descriptor, &spo2, &pulse)
                    .expect("paired")
                    .len(),
                2
            );
            assert!(pair_records(&descriptor, &spo2, &[]).is_err());
            let wrong_cursor = [decoded_record(StoredMetric::Pulse, 1, 1)];
            assert!(pair_records(&descriptor, &spo2, &wrong_cursor).is_err());

            let mut all = Vec::new();
            append_metric_records(&mut all, &spo2, StoredMetric::Spo2, 1, 0)
                .expect("append records");
            assert_eq!(all.len(), 1);
            assert!(append_metric_records(&mut all, &pulse, StoredMetric::Spo2, 1, 1).is_err());
            assert!(append_metric_records(&mut all, &spo2, StoredMetric::Spo2, 1, 1).is_err());

            assert_eq!(
                next_cursor_after(Some(7), &[], StoredMetric::Spo2, 1).expect("initial"),
                Some(7)
            );
            assert_eq!(
                next_cursor_after(None, &spo2, StoredMetric::Spo2, 1).expect("next"),
                Some(1)
            );
            let reserved = StoredRecord {
                metric: StoredMetric::Spo2,
                slot: 1,
                cursor: 0x007f,
                value: None,
                payload: [0; 16],
            };
            assert!(next_cursor_after(None, &[reserved], StoredMetric::Spo2, 1).is_ok());
            let exhausted = [decoded_record(StoredMetric::Spo2, 1, 0xff7f)];
            let mut all = Vec::new();
            assert!(
                append_metric_records(&mut all, &exhausted, StoredMetric::Spo2, 1, 0xff7f).is_err()
            );
            assert!(next_cursor_after(None, &exhausted, StoredMetric::Spo2, 1).is_err());

            let mut progress = StorageSyncProgress::new();
            progress.record_source_records(2).expect("records");
            progress
                .record_batch(AppendReceipt {
                    count: 2,
                    durable: false,
                })
                .expect("batch");
            assert_eq!(progress.records_transferred, 2);
            assert_eq!(progress.rows_spooled, 2);
            assert!(!progress.all_fsynced);
            let receipt = storage_sync_receipt(&progress);
            assert_eq!(receipt.rows_spooled, 2);
            let failure = storage_sync_failure(&progress, "blocked".to_owned());
            assert_eq!(failure.detail, "blocked");

            let mut cursors = HashMap::new();
            assert!(
                finish_storage_sync::<String>(
                    Ok(()),
                    Ok(()),
                    &progress,
                    vec![(7, StoredCursorPair::default())],
                    &mut cursors
                )
                .is_ok()
            );
            assert_eq!(cursors.len(), 1);
            assert!(
                finish_storage_sync::<String>(
                    Ok(()),
                    Err("exit".to_owned()),
                    &progress,
                    Vec::new(),
                    &mut cursors
                )
                .is_err()
            );
            assert!(
                finish_storage_sync::<String>(
                    Err("transfer".to_owned()),
                    Ok(()),
                    &progress,
                    Vec::new(),
                    &mut cursors
                )
                .is_err()
            );
            assert!(
                finish_storage_sync::<String>(
                    Err("transfer".to_owned()),
                    Err("exit".to_owned()),
                    &progress,
                    Vec::new(),
                    &mut cursors
                )
                .is_err()
            );
        }

        #[tokio::test(flavor = "current_thread")]
        async fn socket_append_decodes_receipts_and_rejections() {
            let receipt = append_with_response(
                "append-ok",
                Response::Appended {
                    count: 1,
                    durable: true,
                },
                vec![wire_sample()],
            )
            .await
            .expect("matching append receipt");
            assert_eq!(receipt.count, 1);
            assert!(receipt.durable);

            let error = append_with_response(
                "append-count",
                Response::Appended {
                    count: 2,
                    durable: false,
                },
                vec![wire_sample()],
            )
            .await
            .expect_err("count mismatch");
            assert!(error.contains("but collector sent 1"), "{error}");

            let error = append_with_response(
                "append-error",
                Response::Error {
                    kind: vitals_proto::ErrorKind::Io,
                    detail: "spool blocked".to_owned(),
                },
                vec![wire_sample()],
            )
            .await
            .expect_err("daemon rejection");
            assert!(error.contains("spool blocked"), "{error}");

            let error = append_with_response(
                "append-unexpected",
                Response::Sample {
                    sample: Box::new(wire_sample()),
                },
                vec![wire_sample()],
            )
            .await
            .expect_err("unexpected response");
            assert!(
                error.contains("unexpected vitalsd append response"),
                "{error}"
            );
        }

        #[tokio::test(flavor = "current_thread")]
        async fn storage_sync_transfers_confirmed_records_and_updates_cursors() {
            let writer = TestWriter::ok();
            let dir = test_directory("storage-sync-full");
            let socket = dir.join("vitalsd.sock");
            let listener = tokio::net::UnixListener::bind(&socket).expect("bind test socket");
            let server = serve_append(
                listener,
                Response::Appended {
                    count: 2,
                    durable: true,
                },
                2,
            );
            let mut spo2 = record(StoredMetric::Spo2, 1, 0, 96);
            spo2.push(0);
            let mut pulse = record(StoredMetric::Pulse, 1, 0, 58);
            pulse.push(0);
            let mut stream = Box::pin(stream::iter([
                descriptor(1, Some(0)),
                empty_descriptor(),
                spo2,
                pulse,
            ]));
            let mut pending = Vec::new();
            let mut cursors = HashMap::new();
            let transfer = storage_sync(&writer, &mut stream, &mut pending, &socket, &mut cursors);
            let (result, server_result) = tokio::join!(transfer, server);
            server_result.expect("append server");
            let receipt = result.expect("full storage sync");
            assert_eq!(receipt.records_transferred, 2);
            assert_eq!(receipt.rows_spooled, 2);
            assert!(receipt.all_fsynced);
            assert_eq!(cursors.len(), 1);
            assert!(
                writer
                    .calls
                    .borrow()
                    .iter()
                    .any(|call| call == &storage::EXIT)
            );
        }

        #[tokio::test(flavor = "current_thread")]
        async fn storage_sync_wrapper_publishes_success_and_failure() {
            let writer = TestWriter::ok();
            let mut stream = Box::pin(stream::iter([empty_descriptor()]));
            let mut pending = Vec::new();
            let mut status = LiveStatus::new(StoredSyncState::new());
            let mut cursors = HashMap::new();
            run_storage_sync(
                &writer,
                &mut stream,
                &mut pending,
                Path::new("/not/a/socket"),
                None,
                &mut status,
                &mut cursors,
            )
            .await
            .expect("successful empty wrapper");
            assert_eq!(status.stored_sync.transfer, StoredTransferState::Succeeded);

            let writer = TestWriter::failing();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let mut pending = Vec::new();
            run_storage_sync(
                &writer,
                &mut stream,
                &mut pending,
                Path::new("/not/a/socket"),
                None,
                &mut status,
                &mut cursors,
            )
            .await
            .expect("failure is published, not propagated");
            assert_eq!(status.stored_sync.transfer, StoredTransferState::Failed);
        }

        #[tokio::test(flavor = "current_thread")]
        async fn storage_sync_empty_device_is_a_complete_named_transfer() {
            let writer = TestWriter::ok();
            let dir = test_directory("storage-sync");
            let mut pending = Vec::new();
            let mut stream = Box::pin(stream::iter([empty_descriptor()]));
            let mut cursors = HashMap::new();
            let receipt = storage_sync(&writer, &mut stream, &mut pending, &dir, &mut cursors)
                .await
                .expect("empty storage sync");
            assert_eq!(receipt.records_transferred, 0);
            assert_eq!(receipt.rows_spooled, 0);
            assert!(receipt.all_fsynced);
            assert_eq!(writer.calls.borrow().last(), Some(&storage::EXIT.to_vec()));
        }

        #[test]
        fn append_and_apply_paths_fail_closed_without_a_socket() {
            let result = futures::executor::block_on(append_rows(Path::new("/not/a/socket"), &[]));
            assert_eq!(result.expect("empty append").count, 0);
            let result = futures::executor::block_on(append_rows(
                Path::new("/not/a/socket"),
                &[wire_sample()],
            ));
            assert!(
                result
                    .expect_err("missing socket")
                    .contains("connect to vitalsd")
            );
            let mut progress = StorageSyncProgress::new();
            futures::executor::block_on(append_rows_in_batches(
                Path::new("/not/a/socket"),
                &[],
                &mut progress,
            ))
            .expect("empty batches");
            assert!(
                futures::executor::block_on(append_rows_in_batches(
                    Path::new("/not/a/socket"),
                    &[wire_sample()],
                    &mut progress
                ))
                .is_err()
            );

            let mut status = LiveStatus::new(StoredSyncState::new());
            apply_storage_sync_result(
                &mut status,
                Ok(StorageSyncReceipt {
                    records_transferred: 2,
                    rows_spooled: 2,
                    all_fsynced: true,
                }),
            )
            .expect("successful status");
            apply_storage_sync_result(
                &mut status,
                Err(StorageSyncFailure {
                    detail: "test failure".to_owned(),
                    records_transferred: 1,
                    rows_spooled: 1,
                    all_fsynced: false,
                }),
            )
            .expect("failed status");
        }

        #[tokio::test(flavor = "current_thread")]
        #[expect(
            clippy::too_many_lines,
            reason = "the test exercises live frames, stale detection, shutdown, and cleanup"
        )]
        async fn live_reports_an_ended_stream_after_startup() {
            let writer = TestWriter::ok();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let dir = test_directory("live");
            let mut status = LiveStatus::new(StoredSyncState::new());
            let mut pending = Vec::new();
            let mut cursors = HashMap::new();
            let error = live(
                &writer,
                &mut stream,
                None,
                &dir,
                &mut status,
                &mut pending,
                &mut cursors,
            )
            .await
            .expect_err("ended live stream");
            assert!(error.contains("notification stream ended"), "{error}");

            let writer = TestWriter::failing();
            let mut stream = Box::pin(stream::empty::<Vec<u8>>());
            let mut status = LiveStatus::new(StoredSyncState::new());
            let mut pending = Vec::new();
            let error = live(
                &writer,
                &mut stream,
                None,
                Path::new("/not/a/socket"),
                &mut status,
                &mut pending,
                &mut HashMap::new(),
            )
            .await
            .expect_err("live startup failure");
            assert!(error.contains("start live EMAY stream"), "{error}");
            assert_eq!(status.link, "lost");

            let writer = TestWriter::ok();
            let mut stream = Box::pin(stream::iter([live_frame(60, 98)]));
            let dir = test_directory("live-frame");
            let path = dir.join("status.json");
            let mut status = LiveStatus::new(StoredSyncState::new());
            let mut pending = Vec::new();
            let error = live(
                &writer,
                &mut stream,
                Some(&path),
                &dir,
                &mut status,
                &mut pending,
                &mut HashMap::new(),
            )
            .await
            .expect_err("live stream end after a frame");
            assert!(error.contains("notification stream ended"), "{error}");
            assert_eq!(status.frames, 1);
            assert!(path.is_file());

            let writer = TestWriter::fail_after(4);
            let mut stream = Box::pin(stream::pending::<Vec<u8>>());
            let mut status = LiveStatus::new(StoredSyncState::new());
            let error = tokio::time::timeout(
                Duration::from_secs(1),
                live(
                    &writer,
                    &mut stream,
                    None,
                    &dir,
                    &mut status,
                    &mut Vec::new(),
                    &mut HashMap::new(),
                ),
            )
            .await
            .expect("keepalive failure must not hang")
            .expect_err("keepalive failure");
            assert!(error.contains("keepalive failed"), "{error}");
            assert_eq!(status.link, "lost");

            let writer = TestWriter::ok();
            let mut stream = Box::pin(stream::pending::<Vec<u8>>());
            let mut status = LiveStatus::new(StoredSyncState::new());
            let error = tokio::time::timeout(
                Duration::from_secs(1),
                live(
                    &writer,
                    &mut stream,
                    None,
                    &dir,
                    &mut status,
                    &mut Vec::new(),
                    &mut HashMap::new(),
                ),
            )
            .await
            .expect("silence failure must not hang")
            .expect_err("silence failure");
            assert!(error.contains("stream stale"), "{error}");
            assert_eq!(status.link, "lost");

            let writer = TestWriter::ok();
            let mut stream = Box::pin(stream::pending::<Vec<u8>>());
            let mut status = LiveStatus::new(StoredSyncState::new());
            live_with_shutdown(
                &writer,
                &mut stream,
                None,
                &dir,
                &mut status,
                &mut Vec::new(),
                &mut HashMap::new(),
                std::future::ready(Ok(())),
            )
            .await
            .expect("shutdown is clean");
            assert_eq!(status.link, "lost");
            assert_eq!(
                writer.calls.borrow().last(),
                Some(&cmd::STOP_REALTIME.to_vec())
            );

            let writer = TestWriter::ok();
            let mut stream = Box::pin(stream::pending::<Vec<u8>>());
            let error = live_with_shutdown(
                &writer,
                &mut stream,
                None,
                &dir,
                &mut LiveStatus::new(StoredSyncState::new()),
                &mut Vec::new(),
                &mut HashMap::new(),
                std::future::ready(Err(std::io::Error::other("shutdown test failure"))),
            )
            .await
            .expect_err("shutdown failure");
            assert!(error.contains("wait for EMAY shutdown signal"), "{error}");
        }

        #[tokio::test(flavor = "current_thread")]
        async fn run_rejects_an_invalid_address_before_opening_bluez() {
            let dir = test_directory("run");
            let error = run("not-an-address", &dir, None)
                .await
                .expect_err("invalid address");
            assert!(error.contains("not a Bluetooth address"), "{error}");

            let blocked = TestBackend { fail: true };
            let error = run_with_backend(
                &blocked,
                "D9:80:A3XX:XX:XX",
                Path::new("/not/a/socket"),
                None,
            )
            .await
            .expect_err("backend error");
            assert!(error.contains("no verified radio"), "{error}");

            let fake = TestBackend { fail: false };
            let error =
                run_with_backend(&fake, "D9:80:A3XX:XX:XX", Path::new("/not/a/socket"), None)
                    .await
                    .expect_err("ended notification stream");
            assert!(error.contains("notification stream ended"), "{error}");
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_entrypoint_with_runtime<B, R>(
    environment: Option<&str>,
    socket: Option<&str>,
    status: Option<&Path>,
    build_runtime: B,
    run: R,
) -> std::process::ExitCode
where
    B: FnOnce() -> std::io::Result<tokio::runtime::Runtime>,
    R: FnOnce(tokio::runtime::Runtime, &str, &Path, Option<&Path>) -> Result<(), String>,
{
    let Some(mac) = environment.filter(|value| !value.is_empty()) else {
        eprintln!(
            "usage: EMAY_MAC=<address> [VITALS_SOCKET=/run/vitals/vitalsd.sock] \
             [EMAY_STATUS_JSON=<path>] emay-collector"
        );
        return std::process::ExitCode::from(64);
    };
    let socket = socket
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SOCKET);
    let Ok(runtime) = build_runtime() else {
        eprintln!("emay-collector: create runtime");
        return std::process::ExitCode::from(70);
    };
    match run(runtime, mac, Path::new(socket), status) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("emay-collector: {error}");
            std::process::ExitCode::from(70)
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_main(
    environment: Option<&str>,
    socket: Option<&str>,
    status: Option<&Path>,
) -> std::process::ExitCode {
    linux_entrypoint_with_runtime(
        environment,
        socket,
        status,
        || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
        },
        |runtime, mac, socket, status| runtime.block_on(linux::run(mac, socket, status)),
    )
}

#[cfg(target_os = "linux")]
#[rustfmt::skip]
fn main() -> std::process::ExitCode { linux_main(std::env::var("EMAY_MAC").ok().as_deref(), std::env::var("VITALS_SOCKET").ok().as_deref(), std::env::var("EMAY_STATUS_JSON").ok().filter(|value| !value.is_empty()).map(PathBuf::from).as_deref()) }

#[cfg(all(test, target_os = "linux"))]
mod entrypoint_tests {
    use super::*;

    #[test]
    fn entrypoint_maps_configuration_runtime_and_collector_outcomes() {
        let build = || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
        };
        assert_eq!(
            linux_entrypoint_with_runtime(None, None, None, build, |_, _, _, _| Ok(())),
            std::process::ExitCode::from(64)
        );
        assert_eq!(
            linux_entrypoint_with_runtime(Some(""), None, None, build, |_, _, _, _| Ok(())),
            std::process::ExitCode::from(64)
        );
        assert_eq!(
            linux_entrypoint_with_runtime(
                Some("AA:BB:CC:DD:EE:FF"),
                None,
                None,
                || Err(std::io::Error::other("runtime test failure")),
                |_, _, _, _| Ok(()),
            ),
            std::process::ExitCode::from(70)
        );
        assert_eq!(
            linux_entrypoint_with_runtime(
                Some("AA:BB:CC:DD:EE:FF"),
                Some("/tmp/test-vitals.sock"),
                Some(Path::new("/tmp/emay-status.json")),
                build,
                |_, mac, socket, status| {
                    assert_eq!(mac, "AA:BB:CC:DD:EE:FF");
                    assert_eq!(socket, Path::new("/tmp/test-vitals.sock"));
                    assert_eq!(status, Some(Path::new("/tmp/emay-status.json")));
                    Ok(())
                },
            ),
            std::process::ExitCode::SUCCESS
        );
        assert_eq!(
            linux_entrypoint_with_runtime(
                Some("AA:BB:CC:DD:EE:FF"),
                None,
                None,
                build,
                |_, _, _, _| Err("collector test failure".to_owned()),
            ),
            std::process::ExitCode::from(70)
        );
        assert_eq!(
            linux_main(Some("invalid"), None, None),
            std::process::ExitCode::from(70)
        );
    }
}
