//! T-B1 substitute — overnight EMAY `SleepO2` capture.
//!
//! T-B1 asks one question: **can the Pi hold a BLE link all night?** The brief
//! names the Polar H10 because its RR intervals are dense, but the H10's HR and
//! RR are *corroborating* signals. Amendment 05 §2.1 allows only `SpO₂` and
//! respiratory rate to raise a tier past watch or clear a raised one, so this
//! device answers the same question while carrying the signal the alarm
//! actually acts on.
//!
//! It is also a harder test. The EMAY stops streaming unless the host writes a
//! keepalive roughly every 1.5 s, so this exercises **bidirectional** traffic
//! for eight hours rather than passive notifications. And it needs no pairing at
//! all, so it cannot hit the SMP Security-Request failure that blocks a bonded
//! reconnect on kernel 6.12.x — there is no bond to resume.
//!
//! Output is one line per frame, in the format `tools/gap_histogram.py` already
//! reads:
//!
//! ```text
//! <unix_epoch_seconds_float> <pulse_bpm> <spo2_or_empty>
//! ```
//!
//! The third field is where the H10 put RR intervals. The EMAY sends no
//! interval data, so drift is genuinely not computable from this device and the
//! histogram will say so rather than inventing a number.
//!
//! **Coalesced records share a timestamp.** When several records arrive in one
//! notification they all carry that notification's arrival time, because that is
//! genuinely when they arrived — inventing intermediate timestamps would be
//! worse. The effect is extra zero-second entries in the histogram's `<2s`
//! bucket. Duration and longest-gap, the two numbers this run exists to
//! produce, are unaffected: they measure when the link was up, and a coalesced
//! batch is one moment of liveness, not several.
//!
//! The protocol lives in `vitals-decode::emay`, tested and fuzzed. This file is
//! only the wiring.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!(
        "emay-spike needs BlueZ and runs on the Pi. The protocol it uses lives in \
         vitals-decode and is tested on every platform:\n    cargo test -p vitals-decode emay"
    );
    std::process::exit(64);
}

#[cfg(target_os = "linux")]
mod linux {
    use bluer::{Adapter, AdapterEvent, Address, Device};
    use futures::StreamExt as _;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use vitals_decode::emay::{FRAME_LEN, cmd, scan_frame};

    const SERVICE: bluer::Uuid = bluer::Uuid::from_u128(0x0000_ff12_0000_1000_8000_0080_5f9b_34fb);
    const WRITE_CHAR: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_ff01_0000_1000_8000_0080_5f9b_34fb);
    const NOTIFY_CHAR: bluer::Uuid =
        bluer::Uuid::from_u128(0x0000_ff02_0000_1000_8000_0080_5f9b_34fb);

    /// Verified live: the stream stops without this. 1.2 s leaves margin under
    /// the ~1.5 s the device tolerates.
    const KEEPALIVE_INTERVAL: Duration = Duration::from_millis(1200);

    /// How long to look for the device before calling it out of range.
    ///
    /// The EMAY powers down when not worn and does not advertise at all, so
    /// "not found" almost always means "not on a finger".
    const DISCOVERY_TIMEOUT: Duration = Duration::from_mins(1);

    /// Longest silence tolerated before the run is declared dead.
    ///
    /// The device sends about one frame a second. Two minutes of nothing while
    /// the keepalive is still being written means the link is gone, and saying
    /// so is the whole point — silence must never read as a quiet night.
    const SILENCE_LIMIT: Duration = Duration::from_mins(2);

    fn now_secs() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64())
    }

    /// One capture line, flushed immediately: an eight-hour run that buffers
    /// loses everything if the process is killed.
    fn emit(pulse: Option<u8>, spo2: Option<u8>) {
        use std::io::Write as _;
        // Absent stays absent on the wire too. An empty field is "no reading",
        // which is different from a zero and must stay different.
        let p = pulse.map_or_else(|| "-".to_owned(), |v| v.to_string());
        let s = spo2.map_or_else(String::new, |v| v.to_string());
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{:.3} {p} {s}", now_secs());
        let _ = out.flush();
    }

    async fn find(adapter: &Adapter, target: Address) -> bluer::Result<Option<Device>> {
        if let Ok(d) = adapter.device(target)
            && d.is_connected().await.unwrap_or(false)
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

    /// Totals a run reports at the end.
    struct Totals {
        frames: u64,
        measurements: u64,
        no_finger: u64,
        undecodable: u64,
    }

    async fn capture(
        write: bluer::gatt::remote::Characteristic,
        notify: bluer::gatt::remote::Characteristic,
    ) -> bluer::Result<Totals> {
        // Not Unpin, so it must be pinned before `.next()` can be used in
        // select!. Building on the Pi is what surfaced this; macOS compiles
        // only the non-Linux stub.
        let mut stream = Box::pin(notify.notify().await?);

        for c in [
            cmd::HELLO.as_slice(),
            cmd::DEVICE_STATE.as_slice(),
            cmd::START_REALTIME.as_slice(),
            cmd::BATTERY.as_slice(),
        ] {
            write.write(c).await?;
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        eprintln!("emay-spike: start sequence sent — capturing until interrupted");

        let mut t = Totals {
            frames: 0,
            measurements: 0,
            no_finger: 0,
            undecodable: 0,
        };
        // Reassembly buffer: records straddle notification boundaries.
        let mut pending: Vec<u8> = Vec::with_capacity(FRAME_LEN * 8);
        let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
        let mut last_frame = tokio::time::Instant::now();

        loop {
            tokio::select! {
                item = stream.next() => {
                    let Some(buf) = item else {
                        eprintln!("emay-spike: notification stream ended after {} frames", t.frames);
                        break;
                    };
                    last_frame = tokio::time::Instant::now();

                    // BLE notifications are NOT frame-aligned: at a larger MTU
                    // the device's 8-byte records arrive coalesced and straddle
                    // notification boundaries. Observed live -- one connection
                    // delivered one record per notification and looked perfect,
                    // the next delivered 20-byte notifications carrying 2.5
                    // records each, and a length-checking decoder silently lost
                    // about 45% of that run. Buffer and scan.
                    pending.extend_from_slice(&buf);
                    let mut consumed = 0usize;
                    while let Some((r, used)) =
                        pending.get(consumed..).and_then(scan_frame)
                    {
                        consumed += used;
                        t.frames += 1;
                        if r.is_measurement() {
                            t.measurements += 1;
                        } else {
                            // A valid frame reporting no finger. Recorded, not
                            // discarded: it is evidence the link was up, which
                            // is exactly what this run measures.
                            t.no_finger += 1;
                        }
                        emit(r.pulse_bpm, r.spo2);
                    }
                    pending.drain(..consumed);

                    // A partial record at the tail is normal and waits for the
                    // next notification. Anything larger than that is garbage
                    // accumulating, which is a different problem and must be
                    // visible rather than growing quietly.
                    if pending.len() > FRAME_LEN * 4 {
                        t.undecodable += 1;
                        eprintln!(
                            "emay-spike: {} unparsed bytes buffered ({} so far); dropping: {:02x?}",
                            pending.len(),
                            t.undecodable,
                            pending.as_slice()
                        );
                        pending.clear();
                    }
                }
                _ = keepalive.tick() => {
                    // The device stops streaming without this. A write failure
                    // means the link is gone; say so rather than looping
                    // silently against a dead connection.
                    if let Err(e) = write.write(&cmd::KEEPALIVE).await {
                        eprintln!("emay-spike: keepalive write failed, link is gone: {e}");
                        break;
                    }
                    if last_frame.elapsed() > SILENCE_LIMIT {
                        eprintln!(
                            "emay-spike: NO FRAMES for {}s while keepalives were still being \
                             written. The link is dead; this is not a quiet night.",
                            last_frame.elapsed().as_secs()
                        );
                        break;
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("emay-spike: interrupted after {} frames", t.frames);
                    break;
                }
            }
        }

        // Leave the device in a sane state so the next run does not inherit a
        // stream it did not start.
        let _ = write.write(&cmd::STOP_REALTIME).await;
        Ok(t)
    }

    pub async fn run(mac: &str) -> bluer::Result<i32> {
        let target: Address = mac.parse().map_err(|_| bluer::Error {
            kind: bluer::ErrorKind::InvalidArguments,
            message: format!("{mac:?} is not a Bluetooth address"),
        })?;

        let session = bluer::Session::new().await?;
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;
        eprintln!("emay-spike: adapter {} · target {target}", adapter.name());

        let Some(device) = find(&adapter, target).await? else {
            eprintln!(
                "emay-spike: NOT FOUND — {target} did not appear within {}s. The EMAY powers \
                 down when not worn and does not advertise at all; put it on a finger.",
                DISCOVERY_TIMEOUT.as_secs()
            );
            return Ok(3);
        };

        if !device.is_connected().await.unwrap_or(false) {
            // No pairing, no bonding, no encryption — verified live against
            // this device on 2026-08-03.
            device.connect().await?;
        }

        let mut write = None;
        let mut notify = None;
        for service in device.services().await? {
            if service.uuid().await? != SERVICE {
                continue;
            }
            for c in service.characteristics().await? {
                match c.uuid().await? {
                    u if u == WRITE_CHAR => write = Some(c),
                    u if u == NOTIFY_CHAR => notify = Some(c),
                    _ => {}
                }
            }
        }
        let (Some(write), Some(notify)) = (write, notify) else {
            eprintln!("emay-spike: {target} does not expose 0xFF01/0xFF02 under service 0xFF12");
            return Ok(6);
        };

        let t = capture(write, notify).await?;
        eprintln!(
            "emay-spike: {} frames · {} measurements · {} no-finger · {} undecodable",
            t.frames, t.measurements, t.no_finger, t.undecodable
        );
        // A run that captured nothing must not exit 0, or the acceptance script
        // reads an empty file as a clean night.
        Ok(i32::from(t.frames == 0))
    }
}

#[cfg(target_os = "linux")]
fn main() {
    let mac = std::env::var("EMAY_MAC").unwrap_or_else(|_| {
        std::env::args().nth(1).unwrap_or_else(|| {
            eprintln!("usage: EMAY_MAC=<addr> emay-spike   (or: emay-spike <addr>)");
            std::process::exit(64);
        })
    });

    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        eprintln!("emay-spike: could not start the runtime");
        std::process::exit(70);
    };
    match rt.block_on(linux::run(&mac)) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("emay-spike: {e}");
            std::process::exit(70);
        }
    }
}
