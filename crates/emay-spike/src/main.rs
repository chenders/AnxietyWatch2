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

/// The live-panel status file.
///
/// A stopgap so the bedside panel can show real numbers before the storage and
/// query path lands (T-C1..T-C3). Deliberately not a protocol: one small JSON
/// object rewritten in place, carrying the latest reading plus enough context
/// that a reader can tell *measured* from *not measured* without guessing.
///
/// Serialisation lives out here, outside the Linux-only wiring, so it compiles
/// and is tested on every platform. The panel parses this by field name; a
/// field renamed by accident is a panel that silently shows nothing, which is
/// the one outcome this whole design exists to prevent.
pub mod status {
    /// What the link is doing. A reader must never have to infer this from a
    /// missing field.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum Link {
        /// Connected, start sequence sent, frames expected.
        Up,
        /// The process is alive and still writing, but the device has stopped
        /// reporting. This is NOT the same as the file going stale, which means
        /// the process itself died — and the panel says something different for
        /// each, because the fix is different.
        Lost,
    }

    impl Link {
        const fn as_str(self) -> &'static str {
            match self {
                Self::Up => "up",
                Self::Lost => "lost",
            }
        }
    }

    /// One snapshot. Every physiological field is `Option` because absent and
    /// zero are different things, and they have to stay different all the way
    /// to the screen.
    #[derive(Clone, Copy, Debug)]
    pub struct Snapshot {
        /// When this file was written. Lets a reader distinguish "the process
        /// is alive and the device is quiet" from "the process is gone".
        pub written_at: f64,
        /// When the most recent frame actually arrived. `None` until the first.
        pub sample_at: Option<f64>,
        /// Oxygen saturation, percent. `None` means not measured — never zero.
        pub spo2: Option<u8>,
        /// Pulse rate, bpm. `None` means not measured — never zero.
        pub pulse_bpm: Option<u8>,
        /// A valid frame reporting no finger is still evidence the link is up.
        pub finger: bool,
        /// Whether the device is still reporting. See [`Link`].
        pub link: Link,
        /// Valid frames decoded this run, including no-finger frames.
        pub frames: u64,
        /// Frames carrying an actual reading.
        pub measurements: u64,
        /// Valid frames reporting no finger on the sensor.
        pub no_finger: u64,
        /// Times the reassembly buffer was dropped as garbage. A rising count
        /// means the link is delivering noise, which is a different problem
        /// from delivering nothing.
        pub undecodable: u64,
    }

    fn num<T: core::fmt::Display>(v: Option<T>) -> String {
        v.map_or_else(|| "null".to_owned(), |x| x.to_string())
    }

    /// Hand-rolled rather than pulling `serde` into a spike binary. Every value
    /// is a number, a bool, or a fixed string chosen in this module, so there
    /// is nothing that needs escaping.
    #[must_use]
    pub fn to_json(s: &Snapshot) -> String {
        format!(
            concat!(
                "{{\"schema\":1,",
                "\"written_at\":{:.3},",
                "\"sample_at\":{},",
                "\"spo2\":{},",
                "\"pulse_bpm\":{},",
                "\"finger\":{},",
                "\"link\":\"{}\",",
                "\"frames\":{},",
                "\"measurements\":{},",
                "\"no_finger\":{},",
                "\"undecodable\":{}}}\n"
            ),
            s.written_at,
            s.sample_at
                .map_or_else(|| "null".to_owned(), |v| format!("{v:.3}")),
            num(s.spo2),
            num(s.pulse_bpm),
            s.finger,
            s.link.as_str(),
            s.frames,
            s.measurements,
            s.no_finger,
            s.undecodable,
        )
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!(
        "emay-spike needs BlueZ and runs on the Pi. The protocol it uses lives in \
         vitals-decode and is tested on every platform:\n    cargo test -p vitals-decode emay"
    );
    std::process::exit(64);
}

#[cfg(test)]
mod tests {
    use super::status::{Link, Snapshot, to_json};

    fn snap() -> Snapshot {
        Snapshot {
            written_at: 1_700_000_000.5,
            sample_at: Some(1_700_000_000.25),
            spo2: Some(96),
            pulse_bpm: Some(58),
            finger: true,
            link: Link::Up,
            frames: 10,
            measurements: 9,
            no_finger: 1,
            undecodable: 0,
        }
    }

    #[test]
    fn a_measurement_carries_its_value() {
        let j = to_json(&snap());
        assert!(j.contains("\"spo2\":96"), "{j}");
        assert!(j.contains("\"pulse_bpm\":58"), "{j}");
        assert!(j.contains("\"link\":\"up\""), "{j}");
    }

    /// The invariant the whole panel rests on. A missing reading must serialise
    /// as `null`, never as `0` — a zero would render as a real measurement of
    /// zero saturation, which is both false and alarming.
    #[test]
    fn an_absent_reading_is_null_and_never_zero() {
        let s = Snapshot {
            spo2: None,
            pulse_bpm: None,
            finger: false,
            ..snap()
        };
        let j = to_json(&s);
        assert!(j.contains("\"spo2\":null"), "{j}");
        assert!(j.contains("\"pulse_bpm\":null"), "{j}");
        assert!(
            !j.contains("\"spo2\":0"),
            "absent must not become zero: {j}"
        );
        assert!(j.contains("\"finger\":false"), "{j}");
    }

    /// Before the first frame there is no sample time. `null` rather than 0,
    /// which would read as 1970 and make the reading look infinitely stale
    /// instead of absent.
    #[test]
    fn no_sample_yet_is_null_not_epoch_zero() {
        let s = Snapshot {
            sample_at: None,
            ..snap()
        };
        assert!(to_json(&s).contains("\"sample_at\":null"));
    }

    /// A dead link is stated, not implied by silence.
    #[test]
    fn a_lost_link_says_so() {
        let s = Snapshot {
            link: Link::Lost,
            ..snap()
        };
        assert!(to_json(&s).contains("\"link\":\"lost\""));
    }

    #[test]
    fn the_object_is_one_line_and_closed() {
        let j = to_json(&snap());
        assert!(j.starts_with('{'), "{j}");
        assert!(j.ends_with("}\n"), "{j}");
        assert_eq!(
            j.matches('\n').count(),
            1,
            "one line, so a partial read is obvious: {j}"
        );
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use crate::status::{Link, Snapshot, to_json};
    use bluer::{Adapter, AdapterEvent, Address, Device};
    use futures::StreamExt as _;
    use std::path::{Path, PathBuf};
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

    /// Rewrite the status file atomically: write a sibling temp, then rename.
    /// A panel polling this at 1 Hz must never catch a half-written object, and
    /// a truncated parse is exactly the kind of thing that would quietly render
    /// as a plausible reading.
    ///
    /// Failure is deliberately soft. A status file this run cannot write must
    /// never take down a capture that is otherwise working — it goes to stderr
    /// and the file simply stops changing, which the panel already treats as
    /// "cannot assess". Losing the display is recoverable; losing the night is
    /// not.
    fn write_status(path: &Path, snap: &Snapshot) {
        let tmp = path.with_extension("json.tmp");
        if let Err(e) =
            std::fs::write(&tmp, to_json(snap)).and_then(|()| std::fs::rename(&tmp, path))
        {
            eprintln!("emay-spike: status write to {} failed: {e}", path.display());
        }
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

    /// Decode everything a notification carried, then say so.
    ///
    /// Extracted from `capture` because that function crossed clippy's
    /// too-many-lines threshold once status publishing was added — and a
    /// select! arm that has grown past a screen is worth splitting on its own
    /// merits, not just to satisfy the lint.
    fn drain_notification(
        buf: &[u8],
        pending: &mut Vec<u8>,
        t: &mut Totals,
        snap: &mut Snapshot,
        publish: &impl Fn(&mut Snapshot, &Totals, Link),
    ) {
        // BLE notifications are NOT frame-aligned: at a larger MTU the device's
        // 8-byte records arrive coalesced and straddle notification boundaries.
        // Observed live -- one connection delivered one record per notification
        // and looked perfect, the next delivered 20-byte notifications carrying
        // 2.5 records each, and a length-checking decoder silently lost about
        // 45% of that run. Buffer and scan.
        pending.extend_from_slice(buf);
        let mut consumed = 0usize;
        while let Some((r, used)) = pending.get(consumed..).and_then(scan_frame) {
            consumed += used;
            t.frames += 1;
            if r.is_measurement() {
                t.measurements += 1;
            } else {
                // A valid frame reporting no finger. Recorded, not discarded:
                // it is evidence the link was up, which is what this measures.
                t.no_finger += 1;
            }
            emit(r.pulse_bpm, r.spo2);
            // Absent stays absent here too. A no-finger frame clears the
            // readings rather than leaving the last good ones on screen
            // pretending to be current.
            snap.sample_at = Some(now_secs());
            snap.spo2 = r.spo2;
            snap.pulse_bpm = r.pulse_bpm;
            snap.finger = r.is_measurement();
            publish(snap, t, Link::Up);
        }
        pending.drain(..consumed);

        // A partial record at the tail is normal and waits for the next
        // notification. Anything larger is garbage accumulating, which is a
        // different problem and must be visible rather than growing quietly.
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

    async fn capture(
        write: bluer::gatt::remote::Characteristic,
        notify: bluer::gatt::remote::Characteristic,
        status: Option<&Path>,
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
        // Mirrors `t`, plus the latest reading. Written on every frame AND on
        // every keepalive tick: the tick is what lets a reader tell a quiet
        // device (file fresh, link "lost") from a dead process (file stale).
        let mut snap = Snapshot {
            written_at: now_secs(),
            sample_at: None,
            spo2: None,
            pulse_bpm: None,
            finger: false,
            link: Link::Up,
            frames: 0,
            measurements: 0,
            no_finger: 0,
            undecodable: 0,
        };
        let publish = |snap: &mut Snapshot, t: &Totals, link: Link| {
            if let Some(path) = status {
                snap.written_at = now_secs();
                snap.link = link;
                snap.frames = t.frames;
                snap.measurements = t.measurements;
                snap.no_finger = t.no_finger;
                snap.undecodable = t.undecodable;
                write_status(path, snap);
            }
        };
        publish(&mut snap, &t, Link::Up);
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

                    drain_notification(&buf, &mut pending, &mut t, &mut snap, &publish);
                }
                _ = keepalive.tick() => {
                    // The device stops streaming without this. A write failure
                    // means the link is gone; say so rather than looping
                    // silently against a dead connection.
                    if let Err(e) = write.write(&cmd::KEEPALIVE).await {
                        eprintln!("emay-spike: keepalive write failed, link is gone: {e}");
                        publish(&mut snap, &t, Link::Lost);
                        break;
                    }
                    // Proof of life even when the device is silent.
                    publish(&mut snap, &t, Link::Up);
                    if last_frame.elapsed() > SILENCE_LIMIT {
                        eprintln!(
                            "emay-spike: NO FRAMES for {}s while keepalives were still being \
                             written. The link is dead; this is not a quiet night.",
                            last_frame.elapsed().as_secs()
                        );
                        // Say so in the file before leaving, so the panel gets
                        // a reason rather than only a stale timestamp.
                        publish(&mut snap, &t, Link::Lost);
                        break;
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("emay-spike: interrupted after {} frames", t.frames);
                    publish(&mut snap, &t, Link::Lost);
                    break;
                }
            }
        }

        // Leave the device in a sane state so the next run does not inherit a
        // stream it did not start.
        let _ = write.write(&cmd::STOP_REALTIME).await;
        Ok(t)
    }

    pub async fn run(mac: &str, status: Option<PathBuf>) -> bluer::Result<i32> {
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

        let t = capture(write, notify, status.as_deref()).await?;
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
            eprintln!(
                "usage: EMAY_MAC=<addr> [EMAY_STATUS_JSON=<path>] emay-spike   (or: emay-spike <addr>)"
            );
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
    // Env rather than a flag: this binary takes no argument parser, and systemd
    // sets Environment= anyway. Unset means "write no status file", which is
    // exactly the overnight-capture behaviour that already exists.
    let status = std::env::var("EMAY_STATUS_JSON")
        .ok()
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from);
    if let Some(p) = status.as_ref() {
        eprintln!("emay-spike: writing live status to {}", p.display());
    }

    match rt.block_on(linux::run(&mac, status)) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("emay-spike: {e}");
            std::process::exit(70);
        }
    }
}
