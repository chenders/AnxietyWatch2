//! Read-only Linux `AirSense` 11 stream collector.
//!
//! The portable configuration, journal, status, identity, and sample policy
//! live in the library. This binary owns only `BlueZ`, the bounded raw journal,
//! and the existing `vitalsd` socket path. It never opens `PostgreSQL` and never
//! sends a control, pairing, reset, erase, OTA, or assumed `StopStream` RPC.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("resmed-air11-collector unavailable: this binary requires Linux BlueZ; no host mock");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "linux")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(target_os = "linux")]
use tokio::net::UnixStream;
#[cfg(target_os = "linux")]
use tokio::signal::unix::{SignalKind, signal};
#[cfg(target_os = "linux")]
use vitals_collectors::{
    as11_bluez::Air11BleTransport,
    as11_build::Air11SampleBuilder,
    as11_config::{self, Air11Credentials},
    as11_journal::RawJournal,
    as11_session::{TherapyJournal, TherapyLifecycle},
    as11_status::{
        Air11LatestValue, Air11Status, EventSubscriptionState, LinkState, MeasurementState,
    },
    as11_supervision::{ReconnectBackoff, ReconnectCycle},
    as11_vitalsd::{encode_stream_append, validate_stream_append},
};
#[cfg(target_os = "linux")]
use vitals_decode::{
    events::{EventFrame, decode_event},
    stream::decode_stream,
};
#[cfg(target_os = "linux")]
use vitals_proto::{Response, WireSample, check_frame_len, decode_payload};
#[cfg(target_os = "linux")]
use vitals_registry::VitalsType;

#[cfg(target_os = "linux")]
const DEFAULT_VITALSD_SOCKET: &str = "/run/vitals/vitalsd.sock";
#[cfg(target_os = "linux")]
const DEFAULT_STATE_DIR: &str = "/var/lib/vitals/air11";
#[cfg(target_os = "linux")]
const APPEND_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(target_os = "linux")]
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);

#[cfg(target_os = "linux")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("resmed-air11-collector: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
struct Options {
    credentials_path: PathBuf,
    adapter: String,
    state_dir: PathBuf,
    socket: PathBuf,
    status_path: PathBuf,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct FatalFailure {
    detail: String,
    durability_unknown: bool,
}

#[cfg(target_os = "linux")]
impl FatalFailure {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            durability_unknown: false,
        }
    }
}

#[cfg(target_os = "linux")]
impl core::fmt::Display for FatalFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.detail)
    }
}

#[cfg(target_os = "linux")]
impl std::error::Error for FatalFailure {}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct AppendFailure {
    detail: String,
    durability_unknown: bool,
}

#[cfg(target_os = "linux")]
impl AppendFailure {
    fn unknown(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
            durability_unknown: true,
        }
    }
}

#[cfg(target_os = "linux")]
struct ConnectionLost {
    detail: String,
    reconnect_succeeded: bool,
}

#[cfg(target_os = "linux")]
enum ConnectionOutcome {
    Stopped,
    Lost(ConnectionLost),
}

#[cfg(target_os = "linux")]
type Shutdown = std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), std::io::Error>>>>;

#[cfg(target_os = "linux")]
struct Collector {
    options: Options,
    credentials: Air11Credentials,
    journal: RawJournal,
    therapy_journal: TherapyJournal,
    session_minter: vitals_core::SessionMinter,
    status: Air11Status,
    backoff: ReconnectBackoff,
    reconnect_cycle: ReconnectCycle,
    therapy: TherapyLifecycle,
    attempted: bool,
    shutdown: Shutdown,
}

#[cfg(target_os = "linux")]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_options().map_err(std::io::Error::other)?;
    let mut collector = Collector::new(options)?;
    collector.supervise().await
}

#[cfg(target_os = "linux")]
async fn shutdown_signal() -> Result<(), std::io::Error> {
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}

#[cfg(target_os = "linux")]
impl Collector {
    fn new(options: Options) -> Result<Self, Box<dyn std::error::Error>> {
        let mut status = Air11Status::default();
        status
            .write_atomic(&options.status_path)
            .map_err(|error| format_status_error(&options.status_path, &error))?;
        let credentials = match as11_config::load_credentials(&options.credentials_path) {
            Ok(credentials) => credentials,
            Err(error) => {
                status.link_state = LinkState::Degraded;
                status.blocker = Some(error.to_string());
                let _ = status.write_atomic(&options.status_path);
                return Err(Box::new(error));
            }
        };
        let journal = match RawJournal::open(&options.state_dir) {
            Ok(journal) => journal,
            Err(error) => {
                status.link_state = LinkState::Degraded;
                status.blocker = Some(error.to_string());
                let _ = status.write_atomic(&options.status_path);
                return Err(Box::new(error));
            }
        };
        let therapy_journal = match TherapyJournal::open(&options.state_dir) {
            Ok(value) => value,
            Err(error) => {
                status.link_state = LinkState::Degraded;
                status.blocker = Some(format!("therapy lifecycle unavailable: {error}"));
                let _ = status.write_atomic(&options.status_path);
                return Err(Box::new(error));
            }
        };
        let mut therapy_journal = therapy_journal;
        let mut therapy = therapy_journal.lifecycle();
        if therapy.mark_process_restart() {
            therapy_journal.append(therapy).map_err(|error| {
                format!("therapy restart boundary could not be journaled: {error}")
            })?;
        }
        let session_minter = match vitals_core::SessionMinter::open(&options.state_dir) {
            Ok(minter) => minter,
            Err(error) => {
                status.link_state = LinkState::Degraded;
                status.blocker = Some(format!("session identity unavailable: {error}"));
                let _ = status.write_atomic(&options.status_path);
                return Err(Box::new(error));
            }
        };
        let mut collector = Self {
            options,
            credentials,
            journal,
            therapy_journal,
            session_minter,
            status,
            backoff: ReconnectBackoff::new(),
            reconnect_cycle: ReconnectCycle::new(),
            therapy,
            attempted: false,
            shutdown: Box::pin(shutdown_signal()),
        };
        collector.sync_therapy_status();
        collector
            .status
            .write_atomic(&collector.options.status_path)
            .map_err(|error| format_status_error(&collector.options.status_path, &error))?;
        Ok(collector)
    }

    async fn supervise(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            if self.attempted {
                self.status.reconnect_count = self.status.reconnect_count.saturating_add(1);
            }
            self.attempted = true;
            self.reconnect_cycle.begin_connection();
            self.status.link_state = LinkState::Starting;
            self.status.session_id = None;
            self.status.blocker = None;
            self.status.event_subscription_state = EventSubscriptionState::NotAttempted;
            self.status.event_blocker = None;
            self.status.last_event_data_id = None;
            self.publish()?;

            match self.collect_connection().await {
                Ok(ConnectionOutcome::Stopped) => {
                    self.journal.sync()?;
                    self.status.link_state = LinkState::Disconnected;
                    self.status.blocker = Some("collector stopped by operator".to_owned());
                    self.publish()?;
                    return Ok(());
                }
                Ok(ConnectionOutcome::Lost(connection)) => {
                    if connection.reconnect_succeeded {
                        self.backoff.reset();
                    }
                    self.status.link_state = LinkState::Disconnected;
                    if self.status.published_samples > 0 {
                        self.status.measurement_state = MeasurementState::Stale;
                    }
                    self.status.blocker = Some(connection.detail);
                }
                Err(mut failure) => {
                    if let Err(error) = self.journal.sync() {
                        failure.detail =
                            format!("{}; journal sync also failed: {error}", failure.detail);
                    }
                    self.status.link_state = LinkState::Degraded;
                    self.status.durability_unknown |= failure.durability_unknown;
                    self.status.blocker = Some(failure.detail.clone());
                    let _ = self.publish();
                    return Err(Box::new(failure));
                }
            }
            if self.wait_for_retry().await? {
                return Ok(());
            }
        }
    }

    async fn wait_for_retry(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let delay = self.backoff.next_delay();
        self.status.retry_count = self.status.retry_count.saturating_add(1);
        let reason = self
            .status
            .blocker
            .as_deref()
            .map_or("connection ended without a reason", |value| value);
        self.status.blocker = Some(format!(
            "reconnect pending: {reason}; retry_delay_seconds={}",
            delay.as_secs()
        ));
        self.publish()?;
        tokio::select! {
            () = tokio::time::sleep(delay) => Ok(false),
            signal = self.shutdown.as_mut() => {
                signal.map_err(|error| std::io::Error::other(format!("shutdown signal failed: {error}")))?;
                self.journal.sync()?;
                self.status.link_state = LinkState::Disconnected;
                self.status.blocker = Some("collector stopped by operator".to_owned());
                self.publish()?;
                Ok(true)
            }
        }
    }

    fn publish(&self) -> Result<(), FatalFailure> {
        publish_status(&self.options.status_path, &self.status)
    }
}

#[cfg(target_os = "linux")]
fn format_status_error(path: &Path, error: &std::io::Error) -> std::io::Error {
    std::io::Error::new(
        error.kind(),
        format!("status publish failed for {}: {error}", path.display()),
    )
}

#[cfg(target_os = "linux")]
fn publish_status(path: &Path, status: &Air11Status) -> Result<(), FatalFailure> {
    status
        .write_atomic(path)
        .map_err(|error| FatalFailure::new(format_status_error(path, &error).to_string()))
}

#[cfg(target_os = "linux")]
impl Collector {
    async fn collect_connection(&mut self) -> Result<ConnectionOutcome, FatalFailure> {
        let mut transport =
            match Air11BleTransport::connect(&self.credentials, &self.options.adapter).await {
                Ok(transport) => transport,
                Err(error) => {
                    return Ok(ConnectionOutcome::Lost(ConnectionLost {
                        detail: format!("AirSense connection failed: {error}"),
                        reconnect_succeeded: false,
                    }));
                }
            };
        self.status.link_state = LinkState::Connected;
        self.status.blocker = None;
        self.publish()?;
        let result = self.run_authenticated(&mut transport).await;
        let _ = tokio::time::timeout(DISCONNECT_TIMEOUT, transport.disconnect()).await;
        result
    }

    async fn run_authenticated(
        &mut self,
        transport: &mut Air11BleTransport,
    ) -> Result<ConnectionOutcome, FatalFailure> {
        if self.reconnect_cycle.authentication_required() {
            if let Err(error) = transport.authenticate(self.credentials.client_id()).await {
                return Ok(ConnectionOutcome::Lost(ConnectionLost {
                    detail: format!("AirSense authentication failed: {error}"),
                    reconnect_succeeded: false,
                }));
            }
            self.reconnect_cycle.mark_authenticated();
        }
        self.status.link_state = LinkState::Authenticated;
        self.status.blocker = None;
        self.publish()?;

        let session_id = self
            .session_minter
            .mint()
            .map_err(|error| FatalFailure::new(format!("session identity mint failed: {error}")))?;
        let mut builder = Air11SampleBuilder::new(session_id).map_err(|error| {
            FatalFailure::new(format!("sample builder rejected session: {error}"))
        })?;
        self.status.session_id = Some(session_id);
        self.publish()?;

        if !self.reconnect_cycle.can_start_stream() {
            return Err(FatalFailure::new(
                "AirSense stream start blocked: authentication is not complete",
            ));
        }
        if let Err(error) = transport.start_stream().await {
            return Ok(ConnectionOutcome::Lost(ConnectionLost {
                detail: format!("AirSense stream subscription failed: {error}"),
                reconnect_succeeded: false,
            }));
        }
        self.reconnect_cycle.mark_stream_started();
        match transport.subscribe_events().await {
            Ok(()) => {
                self.status.event_subscription_state = EventSubscriptionState::Subscribed;
                self.status.event_blocker = None;
            }
            Err(error) => {
                self.status.event_subscription_state = EventSubscriptionState::Unavailable;
                self.status.event_blocker =
                    Some(format!("AirSense event subscription unavailable: {error}"));
            }
        }
        self.status.link_state = LinkState::Streaming;
        self.status.blocker = None;
        self.publish()?;
        self.stream_loop(transport, &mut builder).await
    }

    async fn stream_loop(
        &mut self,
        transport: &mut Air11BleTransport,
        builder: &mut Air11SampleBuilder,
    ) -> Result<ConnectionOutcome, FatalFailure> {
        let mut lifecycle_tick = tokio::time::interval(Duration::from_secs(1));
        lifecycle_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = lifecycle_tick.tick() => {
                    let (_, now_unix_micros) = host_receipt()
                        .map_err(|error| FatalFailure::new(format!("therapy lifecycle clock failed: {error}")))?;
                    if self.advance_therapy(now_unix_micros)? {
                        self.publish()?;
                    }
                }
                signal = self.shutdown.as_mut() => {
                    signal.map_err(|error| FatalFailure::new(format!("shutdown signal failed: {error}")))?;
                    return Ok(ConnectionOutcome::Stopped);
                }
                envelope = transport.next_stream_envelope() => {
                    let envelope = match envelope {
                        Ok(envelope) => envelope,
                        Err(error) => {
                            return Ok(ConnectionOutcome::Lost(ConnectionLost {
                                detail: format!("AirSense stream ended: {error}"),
                                reconnect_succeeded: true,
                            }));
                        }
                    };
                    self.process_envelope(envelope, builder).await?;
                }
            }
        }
    }

    async fn process_envelope(
        &mut self,
        envelope: Vec<u8>,
        builder: &mut Air11SampleBuilder,
    ) -> Result<(), FatalFailure> {
        let (receipt, receipt_unix_micros) = host_receipt()
            .map_err(|error| FatalFailure::new(format!("host receipt failed: {error}")))?;
        self.journal
            .append(receipt_unix_micros, &envelope)
            .map_err(|error| {
                FatalFailure::new(format!("raw journal blocked publication: {error}"))
            })?;
        self.status.received_envelopes = self.status.received_envelopes.saturating_add(1);
        self.status.last_host_receipt_unix_micros = Some(receipt_unix_micros);
        self.advance_therapy(receipt_unix_micros)?;
        self.publish()?;

        if let Some(frame) = decode_event(&envelope) {
            self.process_event_frame(&frame, receipt_unix_micros)?;
            return self.publish();
        }
        let Some(frame) = decode_stream(&envelope) else {
            self.quarantine(
                "AirSense application envelope decode invalid; envelope journaled".to_owned(),
            )?;
            return Ok(());
        };
        self.status.decoded_frames = self.status.decoded_frames.saturating_add(1);
        let therapy_before_stream = self.therapy;
        self.therapy.observe_stream();
        if self.therapy != therapy_before_stream {
            self.persist_therapy()?;
            self.sync_therapy_status();
        }
        self.status.last_device_start_time = std::str::from_utf8(frame.source_start_time())
            .ok()
            .map(str::to_owned);
        let batch = match builder.build_frame(&frame, receipt) {
            Ok(batch) => batch,
            Err(error) => {
                self.quarantine(error.to_string())?;
                return Ok(());
            }
        };
        self.status.unavailable = self
            .status
            .unavailable
            .saturating_add(u64::from(batch.unavailable));
        self.status.duplicates = self
            .status
            .duplicates
            .saturating_add(u64::from(batch.duplicates));
        let sample_count = batch.samples.len();
        if sample_count > 0 {
            self.publish_batch(batch.samples).await?;
        } else if self.status.published_samples == 0 {
            if batch.unavailable > 0 {
                self.status.measurement_state = MeasurementState::Unavailable;
                self.status.blocker = Some(format!(
                    "no usable AirSense measurement: unavailable_total={}",
                    self.status.unavailable
                ));
            } else if self.status.measurement_state != MeasurementState::Quarantined {
                self.status.measurement_state = MeasurementState::NeverMeasured;
            }
        }
        self.publish()
    }

    fn process_event_frame(
        &mut self,
        frame: &EventFrame,
        receipt_unix_micros: i64,
    ) -> Result<(), FatalFailure> {
        self.status.event_notifications = self.status.event_notifications.saturating_add(1);
        self.status.event_records = self
            .status
            .event_records
            .saturating_add(u64::from(frame.event_count));
        if frame.truncated {
            self.status.event_truncated_notifications =
                self.status.event_truncated_notifications.saturating_add(1);
        }
        let therapy_before_event = self.therapy;
        for event in frame.events.iter().take(usize::from(frame.event_count)) {
            self.therapy
                .observe_event(event.name(), receipt_unix_micros)
                .map_err(|error| {
                    FatalFailure::new(format!("therapy lifecycle update failed: {error}"))
                })?;
        }
        if self.therapy != therapy_before_event {
            self.persist_therapy()?;
        }
        self.status.last_event_data_id =
            std::str::from_utf8(frame.data_id()).ok().map(str::to_owned);
        self.sync_therapy_status();
        Ok(())
    }

    fn advance_therapy(&mut self, now_unix_micros: i64) -> Result<bool, FatalFailure> {
        let changed = self.therapy.advance(now_unix_micros);
        if changed {
            self.persist_therapy()?;
            self.sync_therapy_status();
        }
        Ok(changed)
    }

    fn persist_therapy(&mut self) -> Result<(), FatalFailure> {
        self.therapy_journal.append(self.therapy).map_err(|error| {
            FatalFailure::new(format!("therapy lifecycle journal blocked: {error}"))
        })
    }

    fn sync_therapy_status(&mut self) {
        self.status.therapy_session_id =
            (self.therapy.session_id() > 0).then_some(self.therapy.session_id());
        self.status.therapy_state = self.therapy.state();
        self.status.therapy_start_host_unix_micros = self.therapy.start_host_unix_micros();
        self.status.therapy_stop_host_unix_micros = self.therapy.stop_host_unix_micros();
        self.status.therapy_finalized_host_unix_micros = self.therapy.finalized_host_unix_micros();
        self.status.therapy_blocker = match self.therapy.state() {
            vitals_collectors::as11_session::TherapyState::Provisional => Some(format!(
                "therapy boundary provisional: stop_host_unix_micros={:?}; settle_min_micros={}; settle_max_micros={}",
                self.therapy.stop_host_unix_micros(),
                vitals_collectors::as11_session::POST_STOP_SETTLE_MICROS,
                vitals_collectors::as11_session::POST_STOP_SETTLE_MAX_MICROS,
            )),
            vitals_collectors::as11_session::TherapyState::Unknown
                if self.therapy.start_host_unix_micros().is_some() => Some(
                "therapy state unknown after collector restart; no stop/start evidence was observed during downtime".to_owned(),
            ),
            _ => None,
        };
    }

    async fn publish_batch(&mut self, samples: Vec<WireSample>) -> Result<(), FatalFailure> {
        let sample_count = samples.len();
        match append_samples(&self.options.socket, samples.clone()).await {
            Ok(count) => {
                self.status.published_samples = self
                    .status
                    .published_samples
                    .saturating_add(u64::try_from(count).map_or(u64::MAX, |value| value));
                self.update_latest(&samples);
                self.status.measurement_state = MeasurementState::Measured;
                self.status.blocker = None;
                if count != sample_count {
                    return Err(FatalFailure::new(format!(
                        "vitalsd append count={count}, expected={sample_count}"
                    )));
                }
                Ok(())
            }
            Err(error) => {
                let detail = error.detail.clone();
                self.status.durability_unknown |= error.durability_unknown;
                self.status.link_state = LinkState::Degraded;
                self.status.blocker = Some(detail.clone());
                if let Err(status_error) = self.publish() {
                    return Err(FatalFailure {
                        detail: format!("{detail}; status publish failed: {status_error}"),
                        durability_unknown: true,
                    });
                }
                Err(FatalFailure {
                    detail,
                    durability_unknown: error.durability_unknown,
                })
            }
        }
    }

    fn update_latest(&mut self, samples: &[WireSample]) {
        for sample in samples {
            let Some(value_type) = VitalsType::from_id(sample.r#type) else {
                continue;
            };
            let latest = Air11LatestValue {
                value: sample.value,
                sample_at_unix_micros: sample.timestamp_unix_micros,
            };
            match value_type {
                VitalsType::RespiratoryRate => self.status.latest.respiratory_rate = Some(latest),
                VitalsType::Leak => self.status.latest.leak_lpm = Some(latest),
                VitalsType::MaskPressure | VitalsType::MaskPressure2s => {
                    self.status.latest.pressure_cmh2o = Some(latest);
                }
                _ => {}
            }
        }
    }

    fn quarantine(&mut self, detail: String) -> Result<(), FatalFailure> {
        self.status.quarantined = self.status.quarantined.saturating_add(1);
        self.status.measurement_state = MeasurementState::Quarantined;
        self.status.blocker = Some(detail);
        self.publish()
    }
}

#[cfg(target_os = "linux")]
fn host_receipt() -> Result<(SystemTime, i64), std::io::Error> {
    let receipt = SystemTime::now();
    let duration = receipt.duration_since(UNIX_EPOCH).map_err(|error| {
        std::io::Error::other(format!("system clock is before Unix epoch: {error}"))
    })?;
    let micros = i64::try_from(duration.as_micros())
        .map_err(|_| std::io::Error::other("host receipt timestamp exceeds i64"))?;
    Ok((receipt, micros))
}

#[cfg(target_os = "linux")]
async fn append_samples(socket: &Path, samples: Vec<WireSample>) -> Result<usize, AppendFailure> {
    if samples.is_empty() {
        return Ok(0);
    }
    let operation = async {
        let mut stream = UnixStream::connect(socket).await.map_err(|error| {
            AppendFailure::unknown(format!(
                "vitalsd socket connect {} failed: {error}",
                socket.display()
            ))
        })?;
        let (frame, expected) = encode_stream_append(samples).map_err(|error| {
            AppendFailure::unknown(format!("vitalsd append encode failed: {error}"))
        })?;
        stream.write_all(&frame).await.map_err(|error| {
            AppendFailure::unknown(format!("vitalsd append write failed: {error}"))
        })?;
        let mut length = [0_u8; 4];
        stream.read_exact(&mut length).await.map_err(|error| {
            AppendFailure::unknown(format!("vitalsd append reply length failed: {error}"))
        })?;
        let body_len = check_frame_len(u32::from_le_bytes(length)).map_err(|error| {
            AppendFailure::unknown(format!("vitalsd append reply length invalid: {error}"))
        })?;
        let mut body = vec![0_u8; body_len];
        stream.read_exact(&mut body).await.map_err(|error| {
            AppendFailure::unknown(format!("vitalsd append reply failed: {error}"))
        })?;
        let response = decode_payload::<Response>(&body).map_err(|error| {
            AppendFailure::unknown(format!("vitalsd append reply decode failed: {error}"))
        })?;
        validate_stream_append(response, expected)
            .map_err(|error| AppendFailure::unknown(error.to_string()))
    };
    tokio::time::timeout(APPEND_TIMEOUT, operation)
        .await
        .map_err(|_| {
            AppendFailure::unknown(format!(
                "vitalsd append deadline expired after {}s: durability UNKNOWN; journal retained",
                APPEND_TIMEOUT.as_secs()
            ))
        })?
}

#[cfg(target_os = "linux")]
fn parse_options() -> Result<Options, String> {
    let mut credentials_path =
        as11_config::default_credentials_path().map_err(|error| error.to_string())?;
    let mut adapter = String::from("hci0");
    let mut state_dir = PathBuf::from(DEFAULT_STATE_DIR);
    let mut socket = PathBuf::from(DEFAULT_VITALSD_SOCKET);
    let mut status_path = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "command-line option is not UTF-8".to_owned())?;
        match argument.as_str() {
            "--credentials" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--credentials requires a path".to_owned())?;
                credentials_path = PathBuf::from(path);
            }
            "--adapter" => {
                let name = args
                    .next()
                    .ok_or_else(|| "--adapter requires a name".to_owned())?;
                adapter = name
                    .into_string()
                    .map_err(|_| "--adapter name is not UTF-8".to_owned())?;
                if adapter.is_empty() {
                    return Err("--adapter cannot be empty".to_owned());
                }
            }
            "--state-dir" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--state-dir requires a path".to_owned())?;
                state_dir = PathBuf::from(path);
            }
            "--socket" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--socket requires a path".to_owned())?;
                socket = PathBuf::from(path);
            }
            "--status" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--status requires a path".to_owned())?;
                status_path = Some(PathBuf::from(path));
            }
            "--help" => {
                println!(
                    "usage: resmed-air11-collector [--credentials PATH] [--adapter NAME] [--state-dir PATH] [--socket PATH] [--status PATH]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option={argument}")),
        }
    }
    let status_path = status_path.unwrap_or_else(|| state_dir.join("status.json"));
    Ok(Options {
        credentials_path,
        adapter,
        state_dir,
        socket,
        status_path,
    })
}
