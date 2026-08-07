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
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    use tokio::net::UnixStream;
    use vitals_collectors::h10_build as build;
    use vitals_core::SessionMinter;
    use vitals_decode::hr::decode_2a37;
    use vitals_proto::{
        Request, Response, WireSample, check_frame_len, decode_payload, encode_frame,
    };

    const HR_SERVICE: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_180d_0000_1000_8000_0080_5f9b_34fb);
    const HR_MEASUREMENT: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_2a37_0000_1000_8000_0080_5f9b_34fb);
    /// A wedged daemon must be loud, not a silent stall (amendment 06 §2.2's
    /// visibility rule applies to the streaming path too).
    const APPEND_TIMEOUT: Duration = Duration::from_secs(30);

    pub(super) struct Config {
        target: Address,
        socket: PathBuf,
        state_dir: PathBuf,
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

    pub async fn run(config: Config) -> Result<(), String> {
        let mut minter = SessionMinter::open(&config.state_dir).map_err(|e| e.to_string())?;
        loop {
            let session = bluer::Session::new().await.map_err(|e| e.to_string())?;
            let adapter = session.default_adapter().await.map_err(|e| e.to_string())?;
            adapter.set_powered(true).await.map_err(|e| e.to_string())?;
            let device = adapter.device(config.target).map_err(|e| e.to_string())?;
            if !device.is_connected().await.unwrap_or(false) {
                device.connect().await.map_err(|e| e.to_string())?;
            }
            tokio::time::timeout(Duration::from_secs(10), async {
                while !device.is_services_resolved().await.unwrap_or(false) {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            })
            .await
            .map_err(|_| "ServicesResolved timeout".to_owned())?;
            let mut found = None;
            for service in device.services().await.map_err(|e| e.to_string())? {
                if service.uuid().await.map_err(|e| e.to_string())? != HR_SERVICE {
                    continue;
                }
                for c in service.characteristics().await.map_err(|e| e.to_string())? {
                    if c.uuid().await.map_err(|e| e.to_string())? == HR_MEASUREMENT {
                        found = Some(c);
                        break;
                    }
                }
            }
            let characteristic = found.ok_or_else(|| "H10 has no 0x2A37".to_owned())?;
            let id = minter.mint().map_err(|e| e.to_string())?;
            let mut hr_seq = 0_i64;
            let mut rr_seq = 0_i64;
            let mut notifications =
                Box::pin(characteristic.notify().await.map_err(|e| e.to_string())?);
            eprintln!(
                "h10-collector: subscribed, session {id}, emulator={}",
                config.emulator
            );
            while let Some(raw) = notifications.next().await {
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
                        append(&config.socket, built.samples).await?;
                    }
                } else {
                    eprintln!("h10-collector: malformed frame {raw:02x?}");
                }
            }
            eprintln!("h10-collector: disconnected; retrying");
            tokio::time::sleep(Duration::from_secs(2)).await;
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
