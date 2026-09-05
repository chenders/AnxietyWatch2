//! T-B1 — eight-hour Polar H10 capture.
//!
//! The entire project rests on the Pi holding a BLE connection all night. This
//! measures that before anything is built on top of it, which is why the task
//! sits ahead of the storage and core work.
//!
//! The `BlueZ` adapter is deliberately kept at the bottom of this file. The
//! argument, state, notification, and exit decisions above it are testable
//! without a radio; tests never manufacture a successful Bluetooth link.

use std::process::ExitCode;
use vitals_decode::hr::{HeartRate, decode_2a37};

// These decisions are compiled on every host so they can be tested without
// hardware; on non-Linux the production entrypoint is intentionally only the
// platform diagnostic, so the Pi consumer is not visible to dead-code lint.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetInputError {
    Missing,
    Empty,
}

/// Selects the configured address without touching `BlueZ`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn target_from_inputs<'a>(
    environment: Option<&'a str>,
    argument: Option<&'a str>,
) -> Result<&'a str, TargetInputError> {
    let target = environment.or(argument).ok_or(TargetInputError::Missing)?;
    if target.is_empty() {
        return Err(TargetInputError::Empty);
    }
    Ok(target)
}

/// States that the hardware boundary may report before capture starts.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceState {
    NotFound,
    NotPaired,
    ServicesUnresolved,
    CharacteristicMissing,
    Capturing,
}

/// Counts observations without deciding that Bluetooth hardware succeeded.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, Default, PartialEq, Eq)]
struct CaptureStats {
    decoded: u64,
    undecodable: u64,
    rr_truncated: u64,
}

/// Records one notification for the capture decision layer.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn record_notification(stats: &mut CaptureStats, buf: &[u8]) -> Option<HeartRate> {
    if let Some(heart_rate) = decode_2a37(buf) {
        stats.decoded = stats.decoded.saturating_add(1);
        let truncated = u64::try_from(heart_rate.rr_truncated).map_or(u64::MAX, |value| value);
        stats.rr_truncated = stats.rr_truncated.saturating_add(truncated);
        Some(heart_rate)
    } else {
        stats.undecodable = stats.undecodable.saturating_add(1);
        None
    }
}

/// Maps a completed state to the process status used by the overnight log.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn capture_exit_code(state: DeviceState, decoded_notifications: u64) -> i32 {
    match state {
        DeviceState::NotFound => 3,
        DeviceState::NotPaired => 4,
        DeviceState::ServicesUnresolved => 5,
        DeviceState::CharacteristicMissing => 6,
        DeviceState::Capturing => i32::from(decoded_notifications == 0),
    }
}

#[cfg(not(target_os = "linux"))]
fn unsupported_platform() -> ExitCode {
    eprintln!(
        "hr-spike needs BlueZ and runs on the Pi. The 0x2A37 decoder it uses lives in \
         vitals-decode and is tested on every platform:\n    cargo test -p vitals-decode hr"
    );
    ExitCode::from(64)
}

#[cfg(target_os = "linux")]
mod bluez_boundary {
    //! Hardware-only BlueZ adapter boundary.
    //!
    //! This module is the Pi acceptance surface: only it opens a BlueZ session,
    //! discovers the H10, connects, resolves GATT, and subscribes. The generic
    //! notification loop and state decisions are exercised with deterministic
    //! streams in unit tests, never with a pretend successful radio.

    use super::{CaptureStats, DeviceState, HeartRate, capture_exit_code, record_notification};
    use bluer::{Adapter, AdapterEvent, Address, Device};
    use futures::{Future, Stream, StreamExt as _};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// Heart Rate Measurement.
    const HR_MEASUREMENT: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_2a37_0000_1000_8000_0080_5f9b_34fb);
    /// Heart Rate Service.
    const HR_SERVICE: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_180d_0000_1000_8000_0080_5f9b_34fb);

    /// How long to wait for GATT after the link comes up.
    ///
    /// Past this, the connection is up but unusable — amendment 06 §3. A longer
    /// timeout would not help: the failure is a rejected SMP Security Request,
    /// and it does not resolve itself.
    const SERVICES_RESOLVED_TIMEOUT: Duration = Duration::from_secs(10);

    /// How long to look for the device before calling it out of range.
    const DISCOVERY_TIMEOUT: Duration = Duration::from_mins(1);

    fn now_secs() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64())
    }

    /// One line of capture. Written to stdout and flushed per notification: an
    /// eight-hour run that buffers loses everything if the process is killed,
    /// and the whole point is surviving the night.
    fn emit(bpm: u16, rr_ms: &[f64]) {
        use std::io::Write as _;
        let rr = rr_ms
            .iter()
            .map(|v| format!("{v:.3}"))
            .collect::<Vec<_>>()
            .join(",");
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{:.3} {bpm} {rr}", now_secs());
        let _ = out.flush();
    }

    async fn wait_for_target<S>(mut events: S, target: Address, timeout: Duration) -> bool
    where
        S: Stream<Item = AdapterEvent> + Unpin,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return false;
            }
            match tokio::time::timeout(remaining, events.next()).await {
                Ok(Some(AdapterEvent::DeviceAdded(address))) if address == target => return true,
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => return false,
            }
        }
    }

    async fn find(adapter: &Adapter, target: Address) -> bluer::Result<Option<Device>> {
        if let Ok(device) = adapter.device(target)
            && device.is_paired().await.unwrap_or(false)
        {
            return Ok(Some(device));
        }
        let events = adapter.discover_devices().await?;
        if wait_for_target(events, target, DISCOVERY_TIMEOUT).await {
            return Ok(Some(adapter.device(target)?));
        }
        Ok(None)
    }

    async fn wait_for_services<F, Fut>(timeout: Duration, mut is_resolved: F) -> bool
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = bool>,
    {
        tokio::time::timeout(timeout, async {
            loop {
                if is_resolved().await {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        .is_ok()
    }

    async fn consume_notifications<S, C, E>(
        mut notifications: S,
        stop: C,
        mut emit_decoded: E,
    ) -> CaptureStats
    where
        S: Stream<Item = Vec<u8>> + Unpin,
        C: Future,
        E: FnMut(&HeartRate),
    {
        let mut stop = Box::pin(stop);
        let mut stats = CaptureStats::default();

        loop {
            tokio::select! {
                item = notifications.next() => {
                    let Some(buf) = item else {
                        eprintln!(
                            "hr-spike: notification stream ended after {} notifications",
                            stats.decoded
                        );
                        break;
                    };
                    if let Some(heart_rate) = record_notification(&mut stats, &buf) {
                        emit_decoded(&heart_rate);
                        if heart_rate.rr_truncated > 0 {
                            eprintln!(
                                "hr-spike: dropped {} RR interval(s) past MAX_RR",
                                heart_rate.rr_truncated
                            );
                        }
                    } else {
                        eprintln!(
                            "hr-spike: undecodable notification ({} so far): {buf:02x?}",
                            stats.undecodable
                        );
                    }
                }
                _ = &mut stop => {
                    eprintln!(
                        "hr-spike: interrupted after {} notifications",
                        stats.decoded
                    );
                    break;
                }
            }
        }
        stats
    }

    async fn capture(hr_char: bluer::gatt::remote::Characteristic) -> bluer::Result<CaptureStats> {
        let notifications = Box::pin(hr_char.notify().await?);
        Ok(
            consume_notifications(notifications, tokio::signal::ctrl_c(), |heart_rate| {
                emit(heart_rate.bpm, heart_rate.rr())
            })
            .await,
        )
    }

    fn terminal_code(target: Address, state: DeviceState) -> i32 {
        match state {
            DeviceState::NotFound => {
                eprintln!(
                    "hr-spike: NOT FOUND — {target} did not appear within {}s. For an H10 this \
                     usually means the strap is dry or not worn; it does not advertise otherwise.",
                    DISCOVERY_TIMEOUT.as_secs()
                );
                capture_exit_code(state, 0)
            }
            DeviceState::NotPaired => {
                eprintln!(
                    "hr-spike: NOT PAIRED — pair once out of band so the IRK is stored:\n    \
                     sudo bluetoothctl pair {target}\n    sudo btmgmt add-device -a 2 -t 1 {target}\n\
                     `trust` alone is not the mechanism; a device implementing LE Privacy rotates \
                     its address and static-MAC trust fails silently."
                );
                capture_exit_code(state, 0)
            }
            DeviceState::ServicesUnresolved => {
                eprintln!(
                    "hr-spike: BONDED BUT UNUSABLE — connected to {target}, but ServicesResolved \
                     never fired within {}s. Encryption could not resume, so GATT never opened. \
                     On kernel 6.12.x this presents as `unexpected SMP command 0x0b`. This looks \
                     exactly like a quiet night and is not one. Recovery:\n    \
                     sudo bluetoothctl remove {target}   # then re-pair",
                    SERVICES_RESOLVED_TIMEOUT.as_secs()
                );
                capture_exit_code(state, 0)
            }
            DeviceState::CharacteristicMissing => {
                eprintln!("hr-spike: {target} exposes no 0x2A37 under service 0x180D");
                capture_exit_code(state, 0)
            }
            DeviceState::Capturing => capture_exit_code(state, 0),
        }
    }

    pub async fn run(mac: &str) -> bluer::Result<i32> {
        let target: Address = mac.parse().map_err(|_| bluer::Error {
            kind: bluer::ErrorKind::InvalidArguments,
            message: format!("{mac:?} is not a Bluetooth address"),
        })?;

        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;
        eprintln!("hr-spike: adapter {} · target {target}", adapter.name());

        let Some(device) = find(&adapter, target).await? else {
            return Ok(terminal_code(target, DeviceState::NotFound));
        };

        if !device.is_paired().await.unwrap_or(false) {
            return Ok(terminal_code(target, DeviceState::NotPaired));
        }

        if !device.is_connected().await.unwrap_or(false) {
            device.connect().await?;
        }

        // The state amendment 06 §3 exists for. A bonded, connected device
        // whose services never resolve is indistinguishable from a quiet night
        // unless something says so out loud.
        let resolved = wait_for_services(SERVICES_RESOLVED_TIMEOUT, || async {
            device.is_services_resolved().await.unwrap_or(false)
        })
        .await;

        if !resolved {
            return Ok(terminal_code(target, DeviceState::ServicesUnresolved));
        }

        let mut hr_char = None;
        for service in device.services().await? {
            if service.uuid().await? != HR_SERVICE {
                continue;
            }
            for characteristic in service.characteristics().await? {
                if characteristic.uuid().await? == HR_MEASUREMENT {
                    hr_char = Some(characteristic);
                    break;
                }
            }
        }
        let Some(hr_char) = hr_char else {
            return Ok(terminal_code(target, DeviceState::CharacteristicMissing));
        };

        eprintln!("hr-spike: subscribed to 0x2A37 — capturing until interrupted");
        let stats = capture(hr_char).await?;
        eprintln!(
            "hr-spike: {} notifications, {} undecodable",
            stats.decoded, stats.undecodable
        );
        Ok(capture_exit_code(DeviceState::Capturing, stats.decoded))
    }

    #[cfg(test)]
    #[expect(
        clippy::expect_used,
        reason = "BlueZ decision tests should fail loudly on malformed fixtures"
    )]
    mod tests {
        use super::*;
        use futures::{future, stream};
        use vitals_decode::hr::{MAX_RR, decode_2a37};

        fn address(text: &str) -> Address {
            text.parse().expect("Bluetooth address")
        }

        #[tokio::test(flavor = "current_thread")]
        async fn discovery_decision_accepts_target_and_rejects_other_streams() {
            let target = address("AA:BB:CC:DD:EE:FF");
            let other = address("11:22:33:44:55:66");
            assert!(
                wait_for_target(
                    stream::iter([
                        AdapterEvent::DeviceAdded(other),
                        AdapterEvent::DeviceAdded(target)
                    ]),
                    target,
                    Duration::from_secs(1),
                )
                .await
            );
            assert!(
                !wait_for_target(
                    stream::iter([AdapterEvent::DeviceAdded(other)]),
                    target,
                    Duration::from_secs(1),
                )
                .await
            );
            assert!(!wait_for_target(stream::empty(), target, Duration::from_secs(1),).await);
        }

        #[tokio::test(flavor = "current_thread")]
        async fn service_resolution_and_capture_stop_decisions_are_testable() {
            assert!(wait_for_services(Duration::from_secs(1), || async { true }).await);
            assert!(!wait_for_services(Duration::ZERO, || async { false }).await);

            let mut emitted = Vec::new();
            let oversized = {
                let mut bytes = vec![0x10, 60];
                for _ in 0..(MAX_RR + 2) {
                    bytes.extend_from_slice(&1024_u16.to_le_bytes());
                }
                bytes
            };
            let stats = consume_notifications(
                stream::iter([vec![0, 72], vec![0], oversized]),
                future::pending::<()>(),
                |heart_rate| emitted.push(heart_rate.bpm),
            )
            .await;
            assert_eq!(stats.decoded, 2);
            assert_eq!(stats.undecodable, 1);
            assert_eq!(stats.rr_truncated, 2);
            assert_eq!(emitted, [72, 60]);

            let interrupted =
                consume_notifications(stream::pending::<Vec<u8>>(), async {}, |_| {}).await;
            assert_eq!(interrupted, CaptureStats::default());
        }

        #[test]
        fn terminal_messages_cover_each_non_capture_state() {
            let target = address("AA:BB:CC:DD:EE:FF");
            assert_eq!(terminal_code(target, DeviceState::NotFound), 3);
            assert_eq!(terminal_code(target, DeviceState::NotPaired), 4);
            assert_eq!(terminal_code(target, DeviceState::ServicesUnresolved), 5);
            assert_eq!(terminal_code(target, DeviceState::CharacteristicMissing), 6);
            emit(72, &[1000.0]);
            assert!(decode_2a37(&[0, 72]).is_some());
        }
    }
}

#[cfg(target_os = "linux")]
fn exit_code_from_i32(code: i32) -> ExitCode {
    u8::try_from(code).map_or(ExitCode::FAILURE, ExitCode::from)
}

#[cfg(target_os = "linux")]
fn linux_entrypoint(environment: Option<&str>, argument: Option<&str>) -> ExitCode {
    let mac = match target_from_inputs(environment, argument) {
        Ok(mac) => mac,
        Err(TargetInputError::Missing) => {
            eprintln!("usage: H10_MAC=<addr> hr-spike   (or: hr-spike <addr>)");
            return ExitCode::from(64);
        }
        Err(TargetInputError::Empty) => {
            eprintln!("hr-spike: H10_MAC is empty; provide a Bluetooth address");
            return ExitCode::from(64);
        }
    };

    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        eprintln!("hr-spike: could not start the runtime");
        return ExitCode::from(70);
    };
    match runtime.block_on(bluez_boundary::run(mac)) {
        Ok(code) => exit_code_from_i32(code),
        Err(error) => {
            eprintln!("hr-spike: {error}");
            ExitCode::from(70)
        }
    }
}

#[cfg(target_os = "linux")]
fn main() -> ExitCode {
    let mut arguments = std::env::args();
    let _program = arguments.next();
    let environment = std::env::var("H10_MAC").ok();
    let argument = arguments.next();
    linux_entrypoint(environment.as_deref(), argument.as_deref())
}

#[cfg(not(target_os = "linux"))]
fn main() -> ExitCode {
    unsupported_platform()
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "argument and state fixtures should fail loudly"
)]
mod tests {
    use super::*;

    #[test]
    fn target_selection_has_explicit_precedence_and_validation() {
        assert_eq!(
            target_from_inputs(Some("environment"), Some("argument")),
            Ok("environment")
        );
        assert_eq!(target_from_inputs(None, Some("argument")), Ok("argument"));
        assert_eq!(
            target_from_inputs(Some(""), Some("argument")),
            Err(TargetInputError::Empty)
        );
        assert_eq!(
            target_from_inputs(None, None),
            Err(TargetInputError::Missing)
        );
    }

    #[test]
    fn capture_decision_distinguishes_every_device_state_and_empty_capture() {
        assert_eq!(capture_exit_code(DeviceState::NotFound, 0), 3);
        assert_eq!(capture_exit_code(DeviceState::NotPaired, 0), 4);
        assert_eq!(capture_exit_code(DeviceState::ServicesUnresolved, 0), 5);
        assert_eq!(capture_exit_code(DeviceState::CharacteristicMissing, 0), 6);
        assert_eq!(capture_exit_code(DeviceState::Capturing, 0), 1);
        assert_eq!(capture_exit_code(DeviceState::Capturing, 1), 0);
    }

    #[test]
    fn notification_accounting_keeps_decoded_and_bad_input_separate() {
        let mut stats = CaptureStats::default();
        let decoded = record_notification(&mut stats, &[0, 72]).expect("valid notification");
        assert_eq!(decoded.bpm, 72);
        assert!(record_notification(&mut stats, &[0]).is_none());
        assert_eq!(
            stats,
            CaptureStats {
                decoded: 1,
                undecodable: 1,
                rr_truncated: 0,
            }
        );
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn unsupported_platform_is_an_explicit_non_success() {
        assert_eq!(unsupported_platform(), ExitCode::from(64));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_entrypoint_validates_before_touching_bluetooth() {
        assert_eq!(linux_entrypoint(None, None), ExitCode::from(64));
        assert_eq!(linux_entrypoint(Some(""), None), ExitCode::from(64));
        assert_eq!(
            linux_entrypoint(None, Some("not-a-bluetooth-address")),
            ExitCode::from(70)
        );
    }
}
