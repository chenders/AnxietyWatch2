//! T-B1 — eight-hour Polar H10 capture.
//!
//! The entire project rests on the Pi holding a BLE connection all night. This
//! measures that before anything is built on top of it, which is why the task
//! sits ahead of the storage and core work.
//!
//! Throwaway code; the number it produces is not. Output is one line per
//! notification, for `tools/gap_histogram.py`:
//!
//! ```text
//! <unix_epoch_seconds_float> <hr_bpm> <rr_ms,rr_ms,...>
//! ```
//!
//! # What this program refuses to do
//!
//! **Report a silent link as a working one.** Amendment 06 §3 named a third
//! device state after the blood-pressure work found it: *bonded, connected, but
//! `ServicesResolved` never fired* — encryption could not resume, GATT never
//! opened, and it looks exactly like a quiet night. This program times that out
//! explicitly and says so.
//!
//! **Rely on `bluetoothctl`.** Its `connect` was observed issuing no
//! `LE Create Connection` at all while the device advertised 269 times, and its
//! `scan on` emits nothing when piped. Pairing and the kernel accept-list are
//! done once, out of band, by the commands in the task brief.
//!
//! The `0x2A37` decoder is in `vitals-decode`, not here: it is the part with
//! the bugs, and it must be testable and fuzzable without a radio.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!(
        "hr-spike needs BlueZ and runs on the Pi. The 0x2A37 decoder it uses lives in \
         vitals-decode and is tested on every platform:\n    cargo test -p vitals-decode hr"
    );
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
mod linux {
    use bluer::{Adapter, AdapterEvent, Address, Device};
    use futures::StreamExt as _;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use vitals_decode::hr::decode_2a37;

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
    fn emit(bpm: u16, rr_s: &[f64]) {
        use std::io::Write as _;
        // The decoder yields canonical seconds; this capture file format is
        // documented as milliseconds, so the presentation converts.
        let rr = rr_s
            .iter()
            .map(|v| format!("{:.3}", v * 1000.0))
            .collect::<Vec<_>>()
            .join(",");
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{:.3} {bpm} {rr}", now_secs());
        let _ = out.flush();
    }

    async fn find(adapter: &Adapter, target: Address) -> bluer::Result<Option<Device>> {
        if let Ok(d) = adapter.device(target)
            && d.is_paired().await.unwrap_or(false)
        {
            return Ok(Some(d));
        }
        let mut events = adapter.discover_devices().await?;
        let deadline = tokio::time::Instant::now() + DISCOVERY_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }
            match tokio::time::timeout(remaining, events.next()).await {
                Ok(Some(AdapterEvent::DeviceAdded(addr))) if addr == target => {
                    return Ok(Some(adapter.device(addr)?));
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => return Ok(None),
            }
        }
    }

    /// The notification loop. Returns (decoded, undecodable).
    async fn capture(hr_char: bluer::gatt::remote::Characteristic) -> bluer::Result<(u64, u64)> {
        // bluer's notify() stream is not Unpin, so `.next()` cannot be used
        // inside select! without pinning it first. Box::pin rather than
        // pin_mut! because the stream is held across the whole loop.
        let mut notifications = Box::pin(hr_char.notify().await?);
        let mut count: u64 = 0;
        let mut undecodable: u64 = 0;

        loop {
            tokio::select! {
                item = notifications.next() => {
                    let Some(buf) = item else {
                        eprintln!(
                            "hr-spike: notification stream ended after {count} notifications"
                        );
                        break;
                    };
                    if let Some(h) = decode_2a37(&buf) {
                        count += 1;
                        emit(h.bpm, h.rr());
                        if h.rr_truncated > 0 {
                            eprintln!(
                                "hr-spike: dropped {} RR interval(s) past MAX_RR",
                                h.rr_truncated
                            );
                        }
                    } else {
                        // Counted, not ignored: a rising count means the link
                        // is delivering garbage, which is a different problem
                        // from delivering nothing.
                        undecodable += 1;
                        eprintln!(
                            "hr-spike: undecodable notification ({undecodable} so far): {buf:02x?}"
                        );
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("hr-spike: interrupted after {count} notifications");
                    break;
                }
            }
        }
        Ok((count, undecodable))
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
            // Three states, not two (amendment 06 §3). This is "out of range",
            // which for an H10 usually means it is not being worn — the strap
            // needs skin contact and moisture to advertise at all.
            eprintln!(
                "hr-spike: NOT FOUND — {target} did not appear within {}s. For an H10 this \
                 usually means the strap is dry or not worn; it does not advertise otherwise.",
                DISCOVERY_TIMEOUT.as_secs()
            );
            return Ok(3);
        };

        if !device.is_paired().await.unwrap_or(false) {
            eprintln!(
                "hr-spike: NOT PAIRED — pair once out of band so the IRK is stored:\n    \
                 sudo bluetoothctl pair {target}\n    sudo btmgmt add-device -a 2 -t 1 {target}\n\
                 `trust` alone is not the mechanism; a device implementing LE Privacy rotates \
                 its address and static-MAC trust fails silently."
            );
            return Ok(4);
        }

        if !device.is_connected().await.unwrap_or(false) {
            device.connect().await?;
        }

        // The state amendment 06 §3 exists for. A bonded, connected device
        // whose services never resolve is indistinguishable from a quiet night
        // unless something says so out loud.
        let resolved = tokio::time::timeout(SERVICES_RESOLVED_TIMEOUT, async {
            loop {
                if device.is_services_resolved().await.unwrap_or(false) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        .is_ok();

        if !resolved {
            eprintln!(
                "hr-spike: BONDED BUT UNUSABLE — connected to {target}, but ServicesResolved \
                 never fired within {}s. Encryption could not resume, so GATT never opened. \
                 On kernel 6.12.x this presents as `unexpected SMP command 0x0b`. This looks \
                 exactly like a quiet night and is not one. Recovery:\n    \
                 sudo bluetoothctl remove {target}   # then re-pair",
                SERVICES_RESOLVED_TIMEOUT.as_secs()
            );
            return Ok(5);
        }

        let mut hr_char = None;
        for service in device.services().await? {
            if service.uuid().await? != HR_SERVICE {
                continue;
            }
            for c in service.characteristics().await? {
                if c.uuid().await? == HR_MEASUREMENT {
                    hr_char = Some(c);
                    break;
                }
            }
        }
        let Some(hr_char) = hr_char else {
            eprintln!("hr-spike: {target} exposes no 0x2A37 under service 0x180D");
            return Ok(6);
        };

        eprintln!("hr-spike: subscribed to 0x2A37 — capturing until interrupted");
        let (count, undecodable) = capture(hr_char).await?;
        eprintln!("hr-spike: {count} notifications, {undecodable} undecodable");

        // A run that captured nothing must not exit 0: the acceptance script
        // would otherwise read an empty file as a clean night.
        Ok(i32::from(count == 0))
    }
}

#[cfg(target_os = "linux")]
fn main() {
    let mac = std::env::var("H10_MAC").unwrap_or_else(|_| {
        std::env::args().nth(1).unwrap_or_else(|| {
            eprintln!("usage: H10_MAC=<addr> hr-spike   (or: hr-spike <addr>)");
            std::process::exit(64);
        })
    });

    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        eprintln!("hr-spike: could not start the runtime");
        std::process::exit(70);
    };
    match rt.block_on(linux::run(&mac)) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("hr-spike: {e}");
            std::process::exit(70);
        }
    }
}
