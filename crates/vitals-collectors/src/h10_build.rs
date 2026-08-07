//! Store-row policy for the Polar H10 collector. Pure and portable.

use vitals_decode::hr::HeartRate;
use vitals_proto::WireSample;
use vitals_registry::{VitalsSource, VitalsType};

/// The rows to append plus every named rejection. The caller logs the
/// rejections; an empty list means the whole notification was stored.
pub struct Built {
    /// Rows that passed every gate.
    pub samples: Vec<WireSample>,
    /// Why anything absent from `samples` is absent. Guards name their
    /// blocker and the value that blocked them.
    pub rejected: Vec<String>,
}

/// Microseconds from a seconds value that came out of `decode_2a37`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "RR seconds derive from u16/1024 and are bounded by ±64 s, \
              eight orders of magnitude inside i64 microseconds"
)]
fn micros(seconds: f64) -> i64 {
    (seconds * 1_000_000.0).round() as i64
}

/// Builds store rows from one notification, enforcing the registry gates.
///
/// * `contact == Some(false)` discards the whole notification: the sensor
///   is asserting its own numbers are meaningless (dry or dangling strap),
///   and a motion artifact stored as a measurement is exactly the
///   measured-and-bad-read-as-good failure the store must never contain.
/// * Out-of-range values are rejected with the value and range named,
///   never clamped. Their `seq` still advances, so
///   `max(seq) - count(*)` keeps rejected beats measurable per session.
/// * Provenance is structural: emulator beats carry
///   [`VitalsSource::PolarH10Emulator`], so no query keyed on the physical
///   source can aggregate them, whatever any free-text marker says.
/// * `seq` under amendment 06 §1.2: `0x2A37` notifications carry no
///   counter, but within one connection the link layer delivers them
///   ordered and exactly once, so the delivery index *is* the sensor's
///   emission index — there is no visible retransmission for it to be
///   unstable across. A reconnect mints a new session.
#[must_use]
pub fn samples(
    h: &HeartRate,
    session_id: i64,
    hr_seq: &mut i64,
    rr_seq: &mut i64,
    arrival_micros: i64,
    emulator: bool,
) -> Built {
    let source = if emulator {
        VitalsSource::PolarH10Emulator.id()
    } else {
        VitalsSource::PolarH10.id()
    };
    let extra = Some(if emulator {
        b"test_origin=polar_h10_emulator".to_vec()
    } else {
        b"test_origin=physical_device".to_vec()
    });
    let mut built = Built {
        samples: Vec::with_capacity(1 + h.rr().len()),
        rejected: Vec::new(),
    };
    if h.contact == Some(false) {
        built.rejected.push(format!(
            "notification discarded: sensor reports no skin contact \
             (bpm={} and {} rr interval(s) not stored)",
            h.bpm,
            h.rr().len()
        ));
        return built;
    }
    if h.rr_truncated > 0 {
        built.rejected.push(format!(
            "{} RR interval(s) beyond MAX_RR were LOST at decode",
            h.rr_truncated
        ));
    }

    let (hr_lo, hr_hi) = VitalsType::Hr.range();
    let bpm = f64::from(h.bpm);
    if bpm < hr_lo || bpm > hr_hi {
        built.rejected.push(format!(
            "hr={bpm} outside registry range {hr_lo}..{hr_hi} bpm: \
             rejected, never clamped"
        ));
    } else {
        built.samples.push(WireSample {
            source,
            r#type: VitalsType::Hr.id(),
            session_id,
            seq: *hr_seq,
            timestamp_unix_micros: arrival_micros,
            value: bpm,
            extra: extra.clone(),
            batch_id: None,
        });
    }
    *hr_seq = hr_seq.saturating_add(1);

    // Back-date each beat: arrival is when the notification landed; the
    // intervals say how long before that each beat happened. The sum uses
    // every interval, valid or not — elapsed time is elapsed time.
    let (rr_lo, rr_hi) = VitalsType::RrInterval.range();
    let total: i64 = h.rr().iter().map(|v| micros(*v)).sum();
    let mut beat_time = arrival_micros.saturating_sub(total);
    for rr_s in h.rr() {
        beat_time = beat_time.saturating_add(micros(*rr_s));
        if *rr_s < rr_lo || *rr_s > rr_hi {
            built.rejected.push(format!(
                "rr_interval={rr_s}s outside registry range \
                 {rr_lo}..{rr_hi}s: rejected, never clamped"
            ));
        } else {
            built.samples.push(WireSample {
                source,
                r#type: VitalsType::RrInterval.id(),
                session_id,
                seq: *rr_seq,
                timestamp_unix_micros: beat_time,
                value: *rr_s,
                extra: extra.clone(),
                batch_id: None,
            });
        }
        *rr_seq = rr_seq.saturating_add(1);
    }
    built
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a failing assertion in a test should panic loudly on fixed data"
)]
mod tests {
    use super::*;
    use vitals_decode::hr::decode_2a37;

    fn decoded(frame: &[u8]) -> HeartRate {
        decode_2a37(frame).expect("test frame decodes")
    }

    #[test]
    fn no_contact_discards_everything_and_says_why() {
        // flags 0x14: contact supported + not in contact + RR present.
        let h = decoded(&[0x14, 82, 0x00, 0x04]);
        let mut hr = 0;
        let mut rr = 0;
        let b = samples(&h, 1, &mut hr, &mut rr, 1_000_000, false);
        assert!(b.samples.is_empty());
        assert_eq!(b.rejected.len(), 1);
        assert!(b.rejected[0].contains("no skin contact"));
        assert!(b.rejected[0].contains("bpm=82"));
    }

    #[test]
    fn dry_electrode_zero_bpm_is_rejected_not_stored() {
        let h = decoded(&[0x00, 0]);
        let mut hr = 0;
        let mut rr = 0;
        let b = samples(&h, 1, &mut hr, &mut rr, 1_000_000, false);
        assert!(b.samples.is_empty(), "0 bpm must never look like asystole");
        assert!(b.rejected[0].contains("outside registry range"));
        assert_eq!(hr, 1, "rejected beats still advance seq");
    }

    #[test]
    fn emulator_beats_carry_their_own_source_id() {
        let h = decoded(&[0x00, 60]);
        let mut hr = 0;
        let mut rr = 0;
        let real = samples(&h, 1, &mut hr, &mut rr, 0, false);
        let fake = samples(&h, 1, &mut hr, &mut rr, 0, true);
        assert_eq!(real.samples[0].source, VitalsSource::PolarH10.id());
        assert_eq!(fake.samples[0].source, VitalsSource::PolarH10Emulator.id());
        assert_ne!(real.samples[0].source, fake.samples[0].source);
    }

    #[test]
    fn rr_intervals_are_stored_in_seconds_and_backdated() {
        // Two intervals of exactly 1 s each (1024 units).
        let h = decoded(&[0x10, 60, 0x00, 0x04, 0x00, 0x04]);
        let mut hr = 0;
        let mut rr = 0;
        let arrival = 10_000_000_i64;
        let b = samples(&h, 1, &mut hr, &mut rr, arrival, false);
        let rr_rows: Vec<_> = b
            .samples
            .iter()
            .filter(|s| s.r#type == VitalsType::RrInterval.id())
            .collect();
        assert_eq!(rr_rows.len(), 2);
        assert!((rr_rows[0].value - 1.0).abs() < 1e-9);
        assert_eq!(rr_rows[0].timestamp_unix_micros, arrival - 1_000_000);
        assert_eq!(rr_rows[1].timestamp_unix_micros, arrival);
    }

    #[test]
    fn out_of_range_rr_is_rejected_while_good_ones_land() {
        // 0x1000 = 4096 units = 4.0 s, above the registry ceiling of 3 s;
        // then a valid 1.0 s interval.
        let h = decoded(&[0x10, 60, 0x00, 0x10, 0x00, 0x04]);
        let mut hr = 0;
        let mut rr = 0;
        let b = samples(&h, 1, &mut hr, &mut rr, 10_000_000, false);
        let rr_rows: Vec<_> = b
            .samples
            .iter()
            .filter(|s| s.r#type == VitalsType::RrInterval.id())
            .collect();
        assert_eq!(rr_rows.len(), 1);
        assert!(b.rejected.iter().any(|r| r.contains("rr_interval=4")));
        assert_eq!(rr, 2, "the rejected interval still consumed a seq");
    }
}
