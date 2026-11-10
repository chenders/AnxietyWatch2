//! IEEE-11073 16-bit SFLOAT.
//!
//! **Not** an IEEE-754 half float. 4-bit signed exponent in the high nibble,
//! 12-bit signed mantissa below. A naive `f16` reinterpretation produces
//! plausible-looking garbage, which is the worst possible failure in a medical
//! value: 118/74 and 141/92 are both believable.

/// Reserved mantissa values. These mean **absent**, never a number.
///
/// Coercing any of these to a numeric value is the exact failure the whole
/// design forbids — a NaN sentinel silently becoming `0.0` is indistinguishable
/// from a real reading by the time it reaches the detection engine.
mod reserved {
    pub const NAN: u16 = 0x07FF;
    pub const NRES: u16 = 0x0800;
    pub const POS_INF: u16 = 0x07FE;
    pub const NEG_INF: u16 = 0x0802;
    pub const RESERVED: u16 = 0x0801;
}

/// Decodes an IEEE-11073 SFLOAT.
///
/// Returns `None` for `NaN`, `NRes`, ±INFINITY and Reserved — all of which mean the
/// measurement is absent.
///
/// # Examples
///
/// ```
/// use vitals_decode::decode_sfloat;
/// // 118 mmHg, exponent 0
/// assert_eq!(decode_sfloat(0x0076), Some(118.0));
/// // exponent -1, mantissa 74 → 7.4
/// assert_eq!(decode_sfloat(0xF04A), Some(7.4));
/// // NaN sentinel is absent, not zero
/// assert_eq!(decode_sfloat(0x07FF), None);
/// ```
#[must_use]
pub fn decode_sfloat(raw: u16) -> Option<f64> {
    let mantissa_bits = raw & 0x0FFF;

    match mantissa_bits {
        reserved::NAN
        | reserved::NRES
        | reserved::POS_INF
        | reserved::NEG_INF
        | reserved::RESERVED => return None,
        _ => {}
    }

    // Sign-extend the 4-bit exponent and the 12-bit mantissa. Doing this with
    // explicit arithmetic rather than casts keeps clippy's cast lints — which
    // are `deny` in this workspace — meaningful.
    let exponent = {
        let e = i32::from(raw >> 12);
        if e >= 0x8 { e - 0x10 } else { e }
    };
    let mantissa = {
        let m = i32::from(mantissa_bits);
        if m >= 0x800 { m - 0x1000 } else { m }
    };

    Some(f64::from(mantissa) * libm_powi10(exponent))
}

/// `10^exp` for the small exponent range SFLOAT can express (-8..=7).
///
/// A lookup rather than `powf` so the crate stays `no_std` with no libm
/// dependency — and so the C build's "libm is not linked on glibc" class of bug
/// cannot exist here at all.
fn libm_powi10(exp: i32) -> f64 {
    const POW10: [f64; 16] = [
        1e-8, 1e-7, 1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1, 1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7,
    ];
    // SFLOAT's exponent is 4-bit signed, so -8..=7 is the whole domain and the
    // index is always in bounds. `get` rather than `[]` anyway: clippy cannot
    // prove the clamp, and a panicking index inside a decoder is precisely the
    // pattern this crate forbids. `1.0` is unreachable, not a silent fallback.
    let idx = usize::try_from(exp + 8).unwrap_or(8);
    POW10.get(idx).copied().unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn decodes_typical_blood_pressure_values() {
        assert!(close(decode_sfloat(0x0076).unwrap(), 118.0));
        assert!(close(decode_sfloat(0x004A).unwrap(), 74.0));
        assert!(close(decode_sfloat(0x004C).unwrap(), 76.0));
    }

    #[test]
    fn applies_negative_exponent() {
        // exponent 0xF = -1, mantissa 0x04A = 74  →  7.4
        assert!(close(decode_sfloat(0xF04A).unwrap(), 7.4));
        // The T-D2 task brief asserted 0xF14A → 7.4, which is wrong: mantissa
        // 0x14A is 330, so the value is 33.0. The C worker caught the error and
        // documented it in a comment; the brief has since been corrected.
        assert!(close(decode_sfloat(0xF14A).unwrap(), 33.0));
    }

    #[test]
    fn reserved_values_are_absent_not_numbers() {
        for raw in [0x07FF, 0x0800, 0x07FE, 0x0802, 0x0801] {
            assert_eq!(
                decode_sfloat(raw),
                None,
                "raw {raw:#06X} must decode as absent"
            );
        }
    }

    #[test]
    fn mantissa_0xfff_is_not_a_sentinel() {
        // 0x0FFF has mantissa 0xFFF, which is NOT the 0x7FF NaN pattern.
        // Treating it as reserved would silently discard a real reading.
        assert!(decode_sfloat(0x0FFF).is_some());
    }

    #[test]
    fn decodes_negative_mantissa() {
        // 0x0FFF = mantissa 0xFFF = -1 after sign extension, exponent 0.
        assert!(close(decode_sfloat(0x0FFF).unwrap(), -1.0));
    }

    #[test]
    fn every_input_is_finite_or_absent() {
        // Exhaustive: the whole 16-bit domain. A C fuzzer needs 30M random runs
        // to approximate what an exhaustive loop proves in milliseconds here.
        for raw in 0u16..=u16::MAX {
            if let Some(v) = decode_sfloat(raw) {
                assert!(v.is_finite(), "raw {raw:#06X} produced non-finite {v}");
            }
        }
    }
}
