//! A&D UA-651BLE blood-pressure collector.
//!
//! The cuff is store-and-forward and advertises only briefly after a reading.
//! This process deliberately never calls `Device1.Connect`: the kernel
//! auto-connect list catches that short window. `bluetoothd` repopulates that
//! list on every boot for devices that are **paired and trusted** (verified
//! against `/sys/kernel/debug/bluetooth/hci0/device_list` on the Pi), so this
//! collector asserts the trust flag at startup rather than shelling out to a
//! privileged `btmgmt`. The collector reacts once `BlueZ` reports the bonded
//! device connected and GATT resolved, then subscribes through `BlueZ` so
//! indication CCCD `0x0002` is used.
//!
//! The sample-building policy is host-independent and lives in the crate lib so
//! its tests run on any platform; only the `BlueZ` transport is Linux.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("bp-collector needs BlueZ and runs on the Pi; its decoder is portable");
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
mod linux {
    use bluer::{Address, Device};
    use futures::StreamExt as _;
    use std::fs::{File, OpenOptions};
    use std::io::{self, Write as _};
    use std::os::unix::fs::OpenOptionsExt as _;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::UnixStream;
    use vitals_collectors::bp_build as build;
    use vitals_core::{Session, SessionMinter};
    use vitals_decode::bp::bp_decode_2a35;
    use vitals_proto::{
        Request, Response, WireSample, check_frame_len, decode_payload, encode_frame,
    };

    const BP_SERVICE: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_1810_0000_1000_8000_0080_5f9b_34fb);
    const BP_MEASUREMENT: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_2a35_0000_1000_8000_0080_5f9b_34fb);
    const SERVICES_TIMEOUT: Duration = Duration::from_secs(10);
    const POLL: Duration = Duration::from_millis(200);
    /// A sync append that neither succeeds nor fails is a frame in limbo and
    /// the cuff has already deleted its copy. Amendment 06 §2.2: the loss must
    /// be visible, so a wedged daemon becomes a named error, not a stall.
    const APPEND_TIMEOUT: Duration = Duration::from_secs(30);

    struct Config {
        target: Address,
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
            let target = mac
                .parse()
                .map_err(|_| format!("{mac:?} is not a Bluetooth address"))?;
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
                target,
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

    async fn resolved(device: &Device) -> bool {
        tokio::time::timeout(SERVICES_TIMEOUT, async {
            loop {
                if device.is_services_resolved().await.unwrap_or(false) {
                    break;
                }
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .is_ok()
    }

    async fn measurement_char(
        device: &Device,
    ) -> bluer::Result<Option<bluer::gatt::remote::Characteristic>> {
        for service in device.services().await? {
            if service.uuid().await? != BP_SERVICE {
                continue;
            }
            for characteristic in service.characteristics().await? {
                if characteristic.uuid().await? == BP_MEASUREMENT {
                    return Ok(Some(characteristic));
                }
            }
        }
        Ok(None)
    }

    async fn capture_connection(
        config: &Config,
        device: &Device,
        minter: &mut SessionMinter,
        journal: &mut RawJournal,
    ) -> Result<u64, String> {
        let id = minter.mint().map_err(|e| format!("mint session id: {e}"))?;
        let mut session = Session::new(id);
        let characteristic = measurement_char(device)
            .await
            .map_err(|e| format!("enumerate GATT: {e}"))?
            .ok_or_else(|| "resolved cuff exposes no 0x2A35 under service 0x1810".to_owned())?;
        // BlueZ chooses indication bit 0x0002 from the characteristic flags.
        let mut indications = Box::pin(
            characteristic
                .notify()
                .await
                .map_err(|e| format!("StartNotify 0x2A35: {e}"))?,
        );
        eprintln!("bp-collector: subscribed to 0x2A35, session {id}");
        let mut count = 0u64;
        while let Some(raw) = indications.next().await {
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
        Ok(count)
    }

    async fn run(config: Config) -> Result<(), String> {
        let mut minter = SessionMinter::open(&config.state_dir)
            .map_err(|e| format!("open session minter: {e}"))?;
        let mut journal = RawJournal::open(&config.journal)
            .map_err(|e| format!("open raw journal {}: {e}", config.journal.display()))?;
        let bluez = bluer::Session::new().await.map_err(|e| e.to_string())?;
        let adapter = bluez.default_adapter().await.map_err(|e| e.to_string())?;
        adapter.set_powered(true).await.map_err(|e| e.to_string())?;
        let device = adapter.device(config.target).map_err(|e| e.to_string())?;
        if !device.is_paired().await.unwrap_or(false) {
            return Err(format!(
                "{} is not paired; pair it so BlueZ stores the IRK before collecting",
                config.target
            ));
        }
        // bluetoothd repopulates the kernel auto-connect list on boot only
        // for trusted devices. Without this flag the cuff's 5–10 s window
        // after a reading passes with no LE Create Connection — which looks
        // exactly like a quiet night.
        if !device.is_trusted().await.unwrap_or(false) {
            device
                .set_trusted(true)
                .await
                .map_err(|e| format!("could not set {} trusted: {e}", config.target))?;
            eprintln!(
                "bp-collector: set {} trusted so bluetoothd re-registers kernel \
                 auto-connect on every boot",
                config.target
            );
        }
        eprintln!(
            "bp-collector: waiting for kernel auto-connect from {} (never calling Connect)",
            config.target
        );
        loop {
            while !device.is_connected().await.unwrap_or(false) {
                tokio::time::sleep(POLL).await;
            }
            if !resolved(&device).await {
                eprintln!(
                    "bp-collector: DEGRADED — bonded cuff {} connected but ServicesResolved did \
                     not fire within {}s; encryption could not resume. Remove bond and re-pair",
                    config.target,
                    SERVICES_TIMEOUT.as_secs()
                );
                while device.is_connected().await.unwrap_or(false) {
                    tokio::time::sleep(POLL).await;
                }
                continue;
            }
            match capture_connection(&config, &device, &mut minter, &mut journal).await {
                Ok(count) => eprintln!("bp-collector: connection ended after {count} frame(s)"),
                Err(e) => eprintln!("bp-collector: connection failed: {e}"),
            }
            while device.is_connected().await.unwrap_or(false) {
                tokio::time::sleep(POLL).await;
            }
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
