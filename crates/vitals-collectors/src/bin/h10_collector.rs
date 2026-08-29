//! Polar H10 Heart Rate Service collector for vitalsd.
//!
//! The sample-building policy is host-independent and lives in the crate lib, so
//! its tests run on any platform; only the `BlueZ` transport is Linux.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("h10-collector needs BlueZ and runs on the Pi");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
mod linux {
    use bluer::Address;
    use futures::StreamExt as _;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::UnixStream;
    use vitals_collectors::h10_build as build;
    use vitals_collectors::h10_reconnect::{
        LinkLoss, ReconnectBackoff, assess_link, notification_idle_timeout,
    };
    use vitals_core::SessionMinter;
    use vitals_decode::hr::decode_2a37;
    use vitals_proto::{
        Request, Response, WireSample, check_frame_len, decode_payload, encode_frame,
    };
    use vitals_registry::VitalsSource;

    const HR_SERVICE: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_180d_0000_1000_8000_0080_5f9b_34fb);
    const HR_MEASUREMENT: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_2a37_0000_1000_8000_0080_5f9b_34fb);
    /// A wedged daemon must be loud, not a silent stall (amendment 06 §2.2's
    /// visibility rule applies to the streaming path too).
    const APPEND_TIMEOUT: Duration = Duration::from_secs(30);
    /// `BlueZ` can leave a connect operation pending indefinitely. A deadline
    /// plus `Device1.Disconnect` cancellation prevents the stale
    /// `Operation already in progress` loop observed on 2026-08-28.
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
    const CONNECTION_CHECK_INTERVAL: Duration = Duration::from_secs(1);
    const CONNECTION_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
    const GATT_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(15);
    const SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(10);
    const CANCEL_CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

    pub(super) struct Config {
        target: Address,
        socket: PathBuf,
        state_dir: PathBuf,
        live_status: Option<PathBuf>,
        live_history: Option<PathBuf>,
        emulator: bool,
    }

    impl Config {
        pub(super) fn from_env() -> Result<Self, String> {
            let mac = std::env::var("H10_MAC")
                .ok()
                .or_else(|| std::env::args().nth(1))
                .ok_or_else(|| "H10_MAC is unset".to_owned())?;
            Ok(Self {
                target: mac
                    .parse()
                    .map_err(|_| format!("invalid H10 address {mac:?}"))?,
                socket: std::env::var_os("VITALSD_SOCKET")
                    .map_or_else(|| "/run/vitals/vitalsd.sock".into(), PathBuf::from),
                state_dir: std::env::var_os("H10_STATE_DIR")
                    .map_or_else(|| "/var/lib/vitals/h10".into(), PathBuf::from),
                live_status: std::env::var_os("H10_LIVE_STATUS_JSON").map(PathBuf::from),
                live_history: std::env::var_os("H10_LIVE_HISTORY_JSONL").map(PathBuf::from),
                emulator: std::env::var("H10_EMULATOR").is_ok_and(|v| v == "1"),
            })
        }
    }

    fn now_micros() -> Result<i64, String> {
        let d = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?;
        i64::try_from(d.as_micros()).map_err(|_| "timestamp overflow".to_owned())
    }

    fn now_secs() -> Result<f64, String> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs_f64())
            .map_err(|e| e.to_string())
    }

    #[derive(Clone)]
    struct LiveStatusUpdate {
        heart_rate: vitals_decode::hr::HeartRate,
        session_id: i64,
        hr_seq: i64,
    }

    struct LiveStatusPublisher {
        tx: Option<mpsc::Sender<LiveStatusUpdate>>,
        worker: Option<std::thread::JoinHandle<()>>,
    }

    impl LiveStatusPublisher {
        fn start(config: &Config) -> Result<Option<Self>, String> {
            let Some(live_status) = config.live_status.clone() else {
                return Ok(None);
            };
            let live_history = config.live_history.clone();
            let emulator = config.emulator;
            let (tx, rx) = mpsc::channel::<LiveStatusUpdate>();
            let worker = std::thread::Builder::new()
                .name("h10-live-status".to_owned())
                .spawn(move || {
                    while let Ok(update) = rx.recv() {
                        if let Err(e) = publish_live_status(
                            &live_status,
                            live_history.as_deref(),
                            emulator,
                            &update.heart_rate,
                            update.session_id,
                            update.hr_seq,
                        ) {
                            eprintln!("h10-collector: live status publish failed: {e}");
                        }
                    }
                })
                .map_err(|e| format!("could not start live status publisher: {e}"))?;
            Ok(Some(Self {
                tx: Some(tx),
                worker: Some(worker),
            }))
        }

        fn publish(
            &self,
            heart_rate: &vitals_decode::hr::HeartRate,
            session_id: i64,
            hr_seq: i64,
        ) -> Result<(), String> {
            self.tx
                .as_ref()
                .ok_or_else(|| "live status publisher is closed".to_owned())?
                .send(LiveStatusUpdate {
                    heart_rate: heart_rate.clone(),
                    session_id,
                    hr_seq,
                })
                .map_err(|_| "live status publisher stopped".to_owned())
        }
    }

    impl Drop for LiveStatusPublisher {
        fn drop(&mut self) {
            self.tx.take();
            if let Some(worker) = self.worker.take()
                && let Err(_) = worker.join()
            {
                eprintln!("h10-collector: live status publisher thread panicked");
            }
        }
    }

    fn publish_live_status(
        path: &Path,
        live_history: Option<&Path>,
        emulator: bool,
        h: &vitals_decode::hr::HeartRate,
        session_id: i64,
        hr_seq: i64,
    ) -> Result<(), String> {
        use std::io::Write as _;

        let parent = path
            .parent()
            .ok_or_else(|| format!("live status path has no parent: {}", path.display()))?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = parent.join(format!(
            ".{}.tmp",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("h10-status")
        ));
        let sample_at = now_secs()?;
        let document =
            build::live_status_json(h, session_id, hr_seq, emulator, sample_at, now_secs()?);
        let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(document.as_bytes())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;

        if let Some(history_path) = live_history {
            if let Some(history_parent) = history_path.parent() {
                std::fs::create_dir_all(history_parent).map_err(|e| e.to_string())?;
            }
            let mut history = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(history_path)
                .map_err(|e| e.to_string())?;
            history
                .write_all(document.as_bytes())
                .map_err(|e| e.to_string())?;
            history.flush().map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    async fn append(socket: &Path, samples: Vec<WireSample>) -> Result<(), String> {
        let io = async {
            let mut stream = UnixStream::connect(socket)
                .await
                .map_err(|e| e.to_string())?;
            let frame = encode_frame(&Request::Append {
                samples,
                // Never true here: RR is a streaming source, reconstructible
                // from the sensor's own buffer, and per-append fsync would
                // destroy the ingest path to protect it (amendment 06 §2.1).
                // Durability cost is paid by bp-collector and nowhere else.
                sync: false,
            })
            .map_err(|e| e.to_string())?;
            stream.write_all(&frame).await.map_err(|e| e.to_string())?;
            let mut len = [0; 4];
            stream
                .read_exact(&mut len)
                .await
                .map_err(|e| e.to_string())?;
            let n = check_frame_len(u32::from_le_bytes(len)).map_err(|e| e.to_string())?;
            let mut body = vec![0; n];
            stream
                .read_exact(&mut body)
                .await
                .map_err(|e| e.to_string())?;
            match decode_payload::<Response>(&body).map_err(|e| e.to_string())? {
                Response::Appended { count, .. } if count > 0 => Ok(()),
                Response::Error { detail, .. } => Err(detail),
                other => Err(format!("unexpected vitalsd response: {other:?}")),
            }
        };
        tokio::time::timeout(APPEND_TIMEOUT, io)
            .await
            .map_err(|_| {
                format!(
                    "vitalsd append timed out after {}s: samples not confirmed",
                    APPEND_TIMEOUT.as_secs()
                )
            })?
    }

    fn source_for(config: &Config) -> VitalsSource {
        if config.emulator {
            VitalsSource::PolarH10Emulator
        } else {
            VitalsSource::PolarH10
        }
    }

    fn jitter_seed() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() ^ u64::from(d.subsec_nanos()))
    }

    async fn cancel_connection(device: &bluer::Device) {
        match tokio::time::timeout(CANCEL_CONNECTION_TIMEOUT, device.disconnect()).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => eprintln!(
                "h10-collector: BlueZ connection cleanup returned kind={:?}, message={:?}; \
                 next attempt will still retry",
                e.kind, e.message
            ),
            Err(_) => eprintln!(
                "h10-collector: BlueZ connection cancellation timed out after {}s",
                CANCEL_CONNECTION_TIMEOUT.as_secs()
            ),
        }
    }

    async fn connect_and_find(
        device: &bluer::Device,
    ) -> Result<bluer::gatt::remote::Characteristic, String> {
        let connected = tokio::time::timeout(CONNECTION_CHECK_TIMEOUT, device.is_connected())
            .await
            .map_err(|_| {
                format!(
                    "initial BlueZ Connected status check timed out after {}s",
                    CONNECTION_CHECK_TIMEOUT.as_secs()
                )
            })?
            .map_err(|e| format!("initial BlueZ Connected status check failed: {e}"))?;
        if !connected {
            match tokio::time::timeout(CONNECT_TIMEOUT, device.connect()).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(format!("BlueZ connect failed: {e}")),
                Err(_) => {
                    return Err(format!(
                        "BlueZ connect timed out after {}s",
                        CONNECT_TIMEOUT.as_secs()
                    ));
                }
            }
        }
        tokio::time::timeout(Duration::from_secs(10), async {
            while !device.is_services_resolved().await.unwrap_or(false) {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        .map_err(|_| "ServicesResolved timeout after 10s".to_owned())?;

        tokio::time::timeout(GATT_DISCOVERY_TIMEOUT, async {
            for service in device.services().await.map_err(|e| e.to_string())? {
                if service.uuid().await.map_err(|e| e.to_string())? != HR_SERVICE {
                    continue;
                }
                for characteristic in service.characteristics().await.map_err(|e| e.to_string())? {
                    if characteristic.uuid().await.map_err(|e| e.to_string())? == HR_MEASUREMENT {
                        return Ok(characteristic);
                    }
                }
            }
            Err("H10 has no 0x2A37".to_owned())
        })
        .await
        .map_err(|_| {
            format!(
                "GATT discovery timed out after {}s",
                GATT_DISCOVERY_TIMEOUT.as_secs()
            )
        })?
    }

    async fn consume_notifications(
        config: &Config,
        live_status: Option<&LiveStatusPublisher>,
        minter: &mut SessionMinter,
        backoff: &mut ReconnectBackoff,
        device: &bluer::Device,
        characteristic: &bluer::gatt::remote::Characteristic,
    ) -> Result<LinkLoss, String> {
        let id = minter.mint().map_err(|e| e.to_string())?;
        let mut hr_seq = 0_i64;
        let mut rr_seq = 0_i64;
        let notification_stream = tokio::time::timeout(SUBSCRIBE_TIMEOUT, characteristic.notify())
            .await
            .map_err(|_| {
                format!(
                    "GATT notification subscribe timed out after {}s",
                    SUBSCRIBE_TIMEOUT.as_secs()
                )
            })?
            .map_err(|e| e.to_string())?;
        let mut notifications = Box::pin(notification_stream);
        let idle_timeout = notification_idle_timeout(source_for(config));
        let mut last_notification = tokio::time::Instant::now();
        let mut received_notification = false;
        let mut connection_check = tokio::time::interval(CONNECTION_CHECK_INTERVAL);
        connection_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        eprintln!(
            "h10-collector: subscribed, session {id}, emulator={}, notification watchdog={}s",
            config.emulator,
            idle_timeout.as_secs()
        );

        loop {
            tokio::select! {
                next = notifications.next() => {
                    let Some(raw) = next else {
                        return Ok(LinkLoss::StreamEnded);
                    };
                    // Any notification proves the complete BLE subscription path is live,
                    // even if its physiological payload is malformed or rejected.
                    last_notification = tokio::time::Instant::now();
                    backoff.reset();
                    if !received_notification {
                        received_notification = true;
                        eprintln!("h10-collector: first notification received, session {id}");
                    }
                    if let Some(h) = decode_2a37(&raw) {
                        let built = build::samples(
                            &h,
                            id,
                            &mut hr_seq,
                            &mut rr_seq,
                            now_micros()?,
                            config.emulator,
                        );
                        for reason in &built.rejected {
                            eprintln!("h10-collector: {reason}");
                        }
                        if !built.samples.is_empty() {
                            let has_hr = built
                                .samples
                                .iter()
                                .any(|sample| sample.r#type == vitals_registry::VitalsType::Hr.id());
                            append(&config.socket, built.samples).await?;
                            if has_hr
                                && let Some(publisher) = live_status
                                && let Err(e) = publisher.publish(&h, id, hr_seq.saturating_sub(1))
                            {
                                eprintln!("h10-collector: live status publish failed: {e}");
                            }
                        }
                    } else {
                        eprintln!("h10-collector: malformed frame {raw:02x?}");
                    }
                }
                _ = connection_check.tick() => {
                    let connected = tokio::time::timeout(
                        CONNECTION_CHECK_TIMEOUT,
                        device.is_connected(),
                    )
                    .await
                    .map_err(|_| {
                        format!(
                            "BlueZ Connected status check timed out after {}s",
                            CONNECTION_CHECK_TIMEOUT.as_secs()
                        )
                    })?
                    .map_err(|e| format!("BlueZ Connected status check failed: {e}"))?;
                    if let Some(loss) = assess_link(
                        connected,
                        last_notification.elapsed(),
                        idle_timeout,
                    ) {
                        return Ok(loss);
                    }
                }
            }
        }
    }

    async fn collect_once(
        config: &Config,
        live_status: Option<&LiveStatusPublisher>,
        minter: &mut SessionMinter,
        backoff: &mut ReconnectBackoff,
    ) -> Result<LinkLoss, String> {
        let session = bluer::Session::new().await.map_err(|e| e.to_string())?;
        let adapter = session.default_adapter().await.map_err(|e| e.to_string())?;
        adapter.set_powered(true).await.map_err(|e| e.to_string())?;
        let device = adapter.device(config.target).map_err(|e| e.to_string())?;
        let result = async {
            let characteristic = connect_and_find(&device).await?;
            consume_notifications(
                config,
                live_status,
                minter,
                backoff,
                &device,
                &characteristic,
            )
            .await
        }
        .await;

        // Device1.Disconnect cancels an outstanding Connect as well as closing
        // a live link. This cleanup is what prevents a failed attempt from
        // poisoning every later retry with `Operation already in progress`.
        cancel_connection(&device).await;
        result
    }

    pub async fn run(config: Config) -> Result<(), String> {
        let live_status = LiveStatusPublisher::start(&config)?;
        let mut minter = SessionMinter::open(&config.state_dir).map_err(|e| e.to_string())?;
        let mut backoff = ReconnectBackoff::default();
        loop {
            let reason = match collect_once(
                &config,
                live_status.as_ref(),
                &mut minter,
                &mut backoff,
            )
            .await
            {
                Ok(LinkLoss::StreamEnded) => "notification stream ended".to_owned(),
                Ok(LinkLoss::Disconnected) => {
                    "BlueZ Connected=false while notification stream remained open".to_owned()
                }
                Ok(LinkLoss::NotificationIdle { idle_for }) => format!(
                    "no notification for {:.1}s while BlueZ remained connected",
                    idle_for.as_secs_f64()
                ),
                Err(e) => e,
            };
            let delay = backoff.next_delay(jitter_seed());
            eprintln!(
                "h10-collector: reconnect required: {reason}; retrying in {:.1}s",
                delay.as_secs_f64()
            );
            tokio::time::sleep(delay).await;
        }
    }

    #[cfg(test)]
    #[expect(
        clippy::expect_used,
        reason = "these tests use fixed local fixtures and should fail loudly"
    )]
    mod tests {
        use super::*;
        use tempfile::tempdir;
        use vitals_decode::hr::decode_2a37;

        fn test_config(root: &Path) -> Config {
            Config {
                target: "00:00:00:00:00:00".parse().expect("valid test address"),
                socket: root.join("vitalsd.sock"),
                state_dir: root.join("state"),
                live_status: Some(root.join("status/status.json")),
                live_history: Some(root.join("history/status.jsonl")),
                emulator: false,
            }
        }

        #[test]
        fn live_status_publisher_serializes_updates() {
            let temp = tempdir().expect("tempdir");
            let config = test_config(temp.path());
            let status_path = config.live_status.clone().expect("status path");
            let history_path = config.live_history.clone().expect("history path");
            let publisher = LiveStatusPublisher::start(&config)
                .expect("publisher setup succeeds")
                .expect("publisher enabled");
            let first = decode_2a37(&[0x00, 60]).expect("first frame decodes");
            let second = decode_2a37(&[0x00, 61]).expect("second frame decodes");

            publisher
                .publish(&first, 7, 9)
                .expect("first send succeeds");
            publisher
                .publish(&second, 7, 10)
                .expect("second send succeeds");
            drop(publisher);

            let status = std::fs::read_to_string(status_path).expect("status file readable");
            assert!(status.contains("\"seq\":10"));
            assert!(status.contains("\"hr\":61"));

            let history = std::fs::read_to_string(history_path).expect("history file readable");
            let entries: Vec<_> = history.lines().collect();
            assert_eq!(entries.len(), 2);
            assert!(
                entries
                    .first()
                    .is_some_and(|entry| entry.contains("\"seq\":9"))
            );
            assert!(
                entries
                    .get(1)
                    .is_some_and(|entry| entry.contains("\"seq\":10"))
            );
        }
    }
}

#[cfg(target_os = "linux")]
fn main() {
    let config = match linux::Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("h10-collector: {e}");
            std::process::exit(64);
        }
    };
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        eprintln!("h10-collector: could not start tokio runtime");
        std::process::exit(70);
    };
    if let Err(e) = runtime.block_on(linux::run(config)) {
        eprintln!("h10-collector: {e}");
        std::process::exit(70);
    }
}
