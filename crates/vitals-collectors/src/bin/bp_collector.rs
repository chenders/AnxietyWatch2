//! A&D UA-651BLE blood-pressure collector.
//!
//! The cuff is store-and-forward and advertises only briefly after a reading.
//! This process deliberately never initiates a userspace device connection: the kernel
//! auto-connect list catches that short window. That list does NOT survive a
//! reboot, and `bluetoothd` does not repopulate it for this bonded+trusted
//! cuff (proven 2026-08-07: post-reboot the cuff sat at auto-connect action 0
//! and a real reading never connected). The privileged registration is owned
//! by `vitals-bp-autoconnect.service`, which runs `btmgmt add-device -a 2`
//! before this unit on every boot; this process still asserts the trust flag
//! as belt and braces. The collector reacts once the broker reports the kernel-created
//! bonded device connected and GATT resolved, then subscribes through the broker so
//! indication CCCD `0x0002` is used.
//!
//! The sample-building policy is host-independent and lives in the crate lib so
//! its tests run on any platform; only the broker transport is Linux.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("bp-collector needs BlueZ and runs on the Pi; its decoder is portable");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
mod linux {
    use std::fs::{File, OpenOptions};
    use std::io::{self, Write as _};
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::UnixStream;
    use vitals_ble_broker::bp::bp_capabilities;
    use vitals_ble_broker::client::{BrokerClient, BrokerEvent};
    use vitals_ble_broker::{Intent, Priority, Source};
    use vitals_collectors::bp_build as build;
    use vitals_core::{Session, SessionMinter};
    use vitals_decode::bp::bp_decode_2a35;
    use vitals_proto::{
        Request, Response, WireSample, check_frame_len, decode_payload, encode_frame,
    };

    const BP_SERVICE: &str = "00001810-0000-1000-8000-00805f9b34fb";
    const BP_MEASUREMENT: &str = "00002a35-0000-1000-8000-00805f9b34fb";
    const SERVICES_TIMEOUT: Duration = Duration::from_secs(10);
    const POLL: Duration = Duration::from_millis(200);
    /// A sync append that neither succeeds nor fails is a frame in limbo and
    /// the cuff has already deleted its copy. Amendment 06 §2.2: the loss must
    /// be visible, so a wedged daemon becomes a named error, not a stall.
    const APPEND_TIMEOUT: Duration = Duration::from_secs(30);
    /// Do not subscribe to the cuff until the durable sink accepts connections.
    /// The cuff may deliver exactly one indication and then discard it.
    const VITALSD_READY_TIMEOUT: Duration = Duration::from_secs(30);
    const VITALSD_READY_POLL: Duration = Duration::from_millis(100);

    struct Config {
        target: String,
        broker_socket: PathBuf,
        adapter_selector: String,
        socket: PathBuf,
        state_dir: PathBuf,
        journal: PathBuf,
    }

    impl Config {
        fn from_env() -> Result<Self, String> {
            let mac = std::env::var("BP_CUFF_MAC")
                .ok()
                .or_else(|| std::env::args().nth(1))
                .ok_or_else(|| "BP_CUFF_MAC is unset (or pass the cuff address)".to_owned())?;
            if mac.is_empty() {
                return Err("BP_CUFF_MAC is empty".to_owned());
            }
            let broker_socket = std::env::var_os("VITALS_BLE_BROKER_SOCKET")
                .map(PathBuf::from)
                .ok_or_else(|| "VITALS_BLE_BROKER_SOCKET is unset".to_owned())?;
            let adapter_selector = std::env::var("VITALS_BLE_ADAPTER_SELECTOR")
                .map_err(|_| "VITALS_BLE_ADAPTER_SELECTOR is unset".to_owned())?;
            let state_dir = PathBuf::from(
                std::env::var_os("BP_STATE_DIR").unwrap_or_else(|| "/var/lib/vitals/bp".into()),
            );
            let journal = std::env::var_os("BP_JOURNAL")
                .map_or_else(|| state_dir.join("raw-frames.journal"), PathBuf::from);
            let socket = PathBuf::from(
                std::env::var_os("VITALSD_SOCKET")
                    .unwrap_or_else(|| "/run/vitals/vitalsd.sock".into()),
            );
            Ok(Self {
                target: mac,
                broker_socket,
                adapter_selector,
                socket,
                state_dir,
                journal,
            })
        }
    }

    struct RawJournal {
        file: File,
    }

    impl RawJournal {
        fn open(path: &Path) -> io::Result<Self> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .open(path)?;
            Ok(Self { file })
        }

        /// Durable before decode: the cuff may already have deleted its copy.
        fn append(&mut self, arrival_micros: i64, raw: &[u8]) -> io::Result<()> {
            self.file.write_all(&arrival_micros.to_le_bytes())?;
            let len = u32::try_from(raw.len())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "raw frame too large"))?;
            self.file.write_all(&len.to_le_bytes())?;
            self.file.write_all(raw)?;
            self.file.sync_data()
        }
    }

    fn now_micros() -> Result<i64, String> {
        let d = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("system clock is before Unix epoch: {e}"))?;
        i64::try_from(d.as_micros()).map_err(|_| "Unix timestamp does not fit i64".to_owned())
    }

    async fn vitalsd_health(socket: &Path) -> Result<(bool, Vec<String>), String> {
        let mut stream = UnixStream::connect(socket)
            .await
            .map_err(|e| format!("connect {}: {e}", socket.display()))?;
        let frame = encode_frame(&Request::Health).map_err(|e| format!("encode health: {e}"))?;
        stream
            .write_all(&frame)
            .await
            .map_err(|e| format!("write health: {e}"))?;
        let mut len = [0u8; 4];
        stream
            .read_exact(&mut len)
            .await
            .map_err(|e| format!("read health reply length: {e}"))?;
        let n = check_frame_len(u32::from_le_bytes(len))
            .map_err(|e| format!("health reply length: {e}"))?;
        let mut body = vec![0; n];
        stream
            .read_exact(&mut body)
            .await
            .map_err(|e| format!("read health reply: {e}"))?;
        match decode_payload::<Response>(&body).map_err(|e| format!("decode health reply: {e}"))? {
            Response::Health { health } => Ok((health.sync_append_ready, health.degraded.clone())),
            Response::Error { kind, detail } => Err(format!("vitalsd health {kind:?}: {detail}")),
            other => Err(format!("unexpected vitalsd health reply: {other:?}")),
        }
    }

    async fn wait_for_vitalsd(socket: &Path) -> Result<(), String> {
        wait_for_vitalsd_with_timeout(socket, VITALSD_READY_TIMEOUT).await
    }

    async fn wait_for_vitalsd_with_timeout(socket: &Path, timeout: Duration) -> Result<(), String> {
        let started = Instant::now();
        loop {
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(format!(
                    "vitalsd readiness timed out after {}s for {}: no health response",
                    timeout.as_secs(),
                    socket.display()
                ));
            }
            let last_error = match tokio::time::timeout(remaining, vitalsd_health(socket)).await {
                Ok(Ok((true, _))) => {
                    eprintln!("bp-collector: vitalsd durable socket is ready");
                    return Ok(());
                }
                Ok(Ok((false, degraded))) => {
                    let blocker = if degraded.is_empty() {
                        "vitalsd reported sync_append_ready=false".to_owned()
                    } else {
                        degraded.join("; ")
                    };
                    format!("vitalsd sync append blocked: {blocker}")
                }
                Ok(Err(error)) => error,
                Err(_) => format!(
                    "vitalsd health probe timed out after {}ms",
                    remaining.as_millis()
                ),
            };
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err(format!(
                    "vitalsd readiness timed out after {}s for {}: {last_error}",
                    timeout.as_secs(),
                    socket.display()
                ));
            }
            tokio::time::sleep(VITALSD_READY_POLL.min(remaining)).await;
        }
    }

    async fn append_durable(socket: &Path, samples: Vec<WireSample>) -> Result<(), String> {
        if samples.is_empty() {
            return Ok(());
        }
        let io = async {
            let mut stream = UnixStream::connect(socket)
                .await
                .map_err(|e| format!("connect {}: {e}", socket.display()))?;
            let frame = encode_frame(&Request::Append {
                samples,
                sync: true,
            })
            .map_err(|e| format!("encode append: {e}"))?;
            stream
                .write_all(&frame)
                .await
                .map_err(|e| format!("write append: {e}"))?;
            let mut len = [0u8; 4];
            stream
                .read_exact(&mut len)
                .await
                .map_err(|e| format!("read append reply length: {e}"))?;
            let n = check_frame_len(u32::from_le_bytes(len))
                .map_err(|e| format!("append reply length: {e}"))?;
            let mut body = vec![0; n];
            stream
                .read_exact(&mut body)
                .await
                .map_err(|e| format!("read append reply: {e}"))?;
            match decode_payload::<Response>(&body)
                .map_err(|e| format!("decode append reply: {e}"))?
            {
                Response::Appended { count, durable } if count > 0 && durable => Ok(()),
                Response::Appended { count, durable } => Err(format!(
                    "vitalsd violated sync append contract: count={count}, durable={durable}"
                )),
                Response::Error { kind, detail } => {
                    Err(format!("vitalsd append {kind:?}: {detail}"))
                }
                other => Err(format!("unexpected vitalsd reply: {other:?}")),
            }
        };
        tokio::time::timeout(APPEND_TIMEOUT, io)
            .await
            .map_err(|_| {
                format!(
                    "vitalsd sync append timed out after {}s: frame durability UNKNOWN, \
                     raw journal holds the bytes",
                    APPEND_TIMEOUT.as_secs()
                )
            })?
    }

    async fn capture_connection(
        config: &Config,
        client: &mut BrokerClient,
        minter: &mut SessionMinter,
        journal: &mut RawJournal,
    ) -> Result<u64, String> {
        let id = minter.mint().map_err(|e| format!("mint session id: {e}"))?;
        let mut session = Session::new(id);
        let characteristic = client
            .resolve_characteristic_bounded(BP_SERVICE, BP_MEASUREMENT, SERVICES_TIMEOUT)
            .await
            .map_err(|e| format!("broker GATT resolution failed: {e}"))?;
        client
            .subscribe_bounded(characteristic, SERVICES_TIMEOUT)
            .await
            .map_err(|e| format!("broker indication subscription failed: {e}"))?;
        eprintln!("bp-collector: subscribed to 0x2A35, session {id}");
        let mut count = 0u64;
        loop {
            let event = client
                .next_event()
                .await
                .map_err(|e| format!("broker BP event failed: {e}"))?;
            let Some(event) = event else {
                return Ok(count);
            };
            let raw = match event {
                BrokerEvent::Frame { payload, .. } => payload,
                BrokerEvent::DeviceState { connected, .. } if !connected => return Ok(count),
                BrokerEvent::DeviceState { .. } => continue,
                BrokerEvent::Disconnected { reason } => {
                    return Err(format!("broker disconnected BP: {reason}"));
                }
            };
            let arrival = now_micros()?;
            journal
                .append(arrival, &raw)
                .map_err(|e| format!("raw journal fsync failed: {e}"))?;
            // One transport delivery has one index. Type is part of sample
            // identity, so every field decoded from this indication shares it.
            let seq = session.next_seq();
            let Some(measurement) = bp_decode_2a35(&raw) else {
                eprintln!("bp-collector: journaled malformed 0x2A35 frame: {raw:02x?}");
                continue;
            };
            let built = build::samples(&measurement, session.id(), seq, arrival);
            for reason in &built.rejected {
                eprintln!("bp-collector: {reason}");
            }
            append_durable(&config.socket, built.samples).await?;
            count = count.saturating_add(1);
            eprintln!("bp-collector: frame {count} is durable in vitalsd spool");
        }
    }

    async fn run(config: Config) -> Result<(), String> {
        let mut minter = SessionMinter::open(&config.state_dir)
            .map_err(|e| format!("open session minter: {e}"))?;
        let mut journal = RawJournal::open(&config.journal)
            .map_err(|e| format!("open raw journal {}: {e}", config.journal.display()))?;
        wait_for_vitalsd(&config.socket).await?;
        let mut client = BrokerClient::connect_source(
            Source::Bp,
            &config.broker_socket,
            config.adapter_selector.clone(),
            config.target.clone(),
            bp_capabilities(),
        )
        .await
        .map_err(|e| format!("connect to BLE broker: {e}"))?;
        client
            .acquire_lease(
                Intent::KernelObserve,
                Priority::new(10),
                300,
                24 * 60 * 60,
                config.target.clone(),
                bp_capabilities(),
                "BP kernel-autoconnect observation",
            )
            .await
            .map_err(|e| format!("acquire BP observation lease: {e}"))?;
        eprintln!(
            "bp-collector: waiting for broker-observed kernel auto-connect from {} \
             (never requesting userspace Connect)",
            config.target
        );
        loop {
            let (connected, services_resolved) = client
                .observe_device_bounded(config.target.clone(), SERVICES_TIMEOUT)
                .await
                .map_err(|e| format!("observe BP device: {e}"))?;
            if !connected {
                tokio::time::sleep(POLL).await;
                continue;
            }
            if !services_resolved {
                eprintln!(
                    "bp-collector: DEGRADED — bonded cuff {} connected but ServicesResolved \
                     did not fire within {}s; encryption could not resume. Remove bond and re-pair",
                    config.target,
                    SERVICES_TIMEOUT.as_secs()
                );
                tokio::time::sleep(POLL).await;
                continue;
            }
            match capture_connection(&config, &mut client, &mut minter, &mut journal).await {
                Ok(count) => eprintln!("bp-collector: connection ended after {count} frame(s)"),
                Err(e) => eprintln!("bp-collector: connection failed: {e}"),
            }
        }
    }

    #[cfg(test)]
    #[expect(
        clippy::expect_used,
        clippy::items_after_test_module,
        reason = "Linux socket tests fail loudly and keep the entry point beside run"
    )]
    mod tests {
        use super::*;
        use tempfile::tempdir;
        use tokio::net::UnixListener;
        use vitals_proto::{Health, HeartbeatInputs};

        #[tokio::test(flavor = "current_thread")]
        async fn waits_for_vitalsd_health_readiness() {
            let directory = tempdir().expect("temporary directory");
            let socket = directory.path().join("vitalsd.sock");
            let delayed_socket = socket.clone();
            let listener = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let listener = UnixListener::bind(delayed_socket).expect("bind test socket");
                let (mut stream, _) = listener.accept().await.expect("accept health probe");
                let mut len = [0u8; 4];
                stream
                    .read_exact(&mut len)
                    .await
                    .expect("read health length");
                let n = check_frame_len(u32::from_le_bytes(len)).expect("health length");
                let mut body = vec![0; n];
                stream
                    .read_exact(&mut body)
                    .await
                    .expect("read health body");
                assert_eq!(
                    decode_payload::<Request>(&body).expect("decode request"),
                    Request::Health
                );
                let response = Response::Health {
                    health: Box::new(Health {
                        uptime_secs: 1,
                        sources: Vec::new(),
                        total_samples: 0,
                        spool_bytes: 0,
                        sync_append_ready: true,
                        lag_bytes: 0,
                        lag_age_seconds: 0,
                        last_advanced_at: None,
                        subscribers: Vec::new(),
                        heartbeat: HeartbeatInputs {
                            monitoring_window_active: false,
                            primary_sample_age_secs: None,
                            tier_evaluation_age_secs: None,
                            drain_progressing: true,
                        },
                        heartbeat_pulsing: true,
                        degraded: Vec::new(),
                    }),
                };
                stream
                    .write_all(&encode_frame(&response).expect("encode health response"))
                    .await
                    .expect("write health response");
            });

            wait_for_vitalsd(&socket)
                .await
                .expect("health becomes ready");
            listener.await.expect("listener task");
        }

        #[tokio::test(flavor = "current_thread")]
        async fn sync_append_blocker_prevents_readiness() {
            let directory = tempdir().expect("temporary directory");
            let socket = directory.path().join("vitalsd.sock");
            let listener = UnixListener::bind(&socket).expect("bind test socket");
            let task = tokio::spawn(async move {
                for _ in 0..3 {
                    let (mut stream, _) = listener.accept().await.expect("accept health probe");
                    let mut len = [0u8; 4];
                    stream
                        .read_exact(&mut len)
                        .await
                        .expect("read health length");
                    let n = check_frame_len(u32::from_le_bytes(len)).expect("health length");
                    let mut body = vec![0; n];
                    stream
                        .read_exact(&mut body)
                        .await
                        .expect("read health body");
                    assert_eq!(
                        decode_payload::<Request>(&body).expect("decode request"),
                        Request::Health
                    );
                    let response = Response::Health {
                        health: Box::new(Health {
                            uptime_secs: 1,
                            sources: Vec::new(),
                            total_samples: 0,
                            spool_bytes: 0,
                            sync_append_ready: false,
                            lag_bytes: 0,
                            lag_age_seconds: 0,
                            last_advanced_at: None,
                            subscribers: Vec::new(),
                            heartbeat: HeartbeatInputs {
                                monitoring_window_active: false,
                                primary_sample_age_secs: None,
                                tier_evaluation_age_secs: None,
                                drain_progressing: true,
                            },
                            heartbeat_pulsing: true,
                            degraded: vec!["spool rotation blocked: test blocker".to_owned()],
                        }),
                    };
                    stream
                        .write_all(&encode_frame(&response).expect("encode health response"))
                        .await
                        .expect("write health response");
                }
            });

            let error = wait_for_vitalsd_with_timeout(&socket, Duration::from_millis(220))
                .await
                .expect_err("sync append blocker must prevent readiness");
            assert!(error.contains("vitalsd readiness timed out"));
            task.abort();
        }

        #[tokio::test(flavor = "current_thread")]
        async fn health_probe_timeout_is_bounded() {
            let directory = tempdir().expect("temporary directory");
            let socket = directory.path().join("vitalsd.sock");
            let listener = UnixListener::bind(&socket).expect("bind test socket");
            let task = tokio::spawn(async move {
                let (_stream, _) = listener.accept().await.expect("accept health probe");
                tokio::time::sleep(Duration::from_secs(1)).await;
            });

            let error = wait_for_vitalsd_with_timeout(&socket, Duration::from_millis(20))
                .await
                .expect_err("unresponsive health socket must time out");
            assert!(error.contains("vitalsd readiness timed out"));
            task.abort();
        }
    }

    pub fn main() {
        let config = match Config::from_env() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("bp-collector: {e}");
                std::process::exit(64);
            }
        };
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            eprintln!("bp-collector: could not start tokio runtime");
            std::process::exit(70);
        };
        if let Err(e) = runtime.block_on(run(config)) {
            eprintln!("bp-collector: {e}");
            std::process::exit(70);
        }
    }
}

#[cfg(target_os = "linux")]
fn main() {
    linux::main();
}
