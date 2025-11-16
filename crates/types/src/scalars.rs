//! Deterministic fixed-point helpers for representing ratios without floats.
//!
//! Ratios are stored as micro-units (`RATIO_SCALE = 1_000_000`) so that
//! `1.0 == 1_000_000` and `0.5 == 500_000`. This allows runtime code to
//! perform percentage and probability math deterministically while still
//! offering human-readable formatting helpers.

use core::fmt;

/// Fixed-point representation of ratios using 6 decimal places.
pub type RatioMicros = u64;

/// Scale applied to ratio values (one million micro-units == 1.0).
pub const RATIO_SCALE: RatioMicros = 1_000_000;

/// Compute a ratio from two integers using deterministic fixed-point math.
///
/// Returns a clamped value between `0` and `RATIO_SCALE`.
pub fn ratio_from_parts(numerator: u128, denominator: u128) -> RatioMicros {
    if denominator == 0 {
        return 0;
    }

    let scaled = (numerator.saturating_mul(RATIO_SCALE as u128)) / denominator;
    scaled.min(RATIO_SCALE as u128) as RatioMicros
}

/// Clamp the provided ratio to the valid `[0, RATIO_SCALE]` range.
pub fn clamp_ratio(value: RatioMicros) -> RatioMicros {
    value.min(RATIO_SCALE)
}

/// Convert basis points (0-10_000) into ratio micro units.
pub fn ratio_from_bps(bps: u32) -> RatioMicros {
    let value = (bps as u128) * (RATIO_SCALE as u128) / 10_000u128;
    value.min(RATIO_SCALE as u128) as RatioMicros
}

/// Convert ratio micro units into basis points (0-10_000).
pub fn ratio_to_bps(ratio: RatioMicros) -> u32 {
    ((ratio as u128) * 10_000u128 / (RATIO_SCALE as u128)) as u32
}

/// Format a ratio as a string with up to six decimal places without using floats.
pub fn format_ratio(ratio: RatioMicros) -> RatioDisplay {
    RatioDisplay { ratio }
}

/// Display helper returned by [`format_ratio`].
pub struct RatioDisplay {
    ratio: RatioMicros,
}

impl fmt::Display for RatioDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.ratio / RATIO_SCALE;
        let fractional = self.ratio % RATIO_SCALE;

        if fractional == 0 {
            write!(f, "{}", whole)
        } else {
            // Trim trailing zeros for cleaner formatting.
            let mut frac_str = format!("{fractional:06}");
            while frac_str.ends_with('0') {
                frac_str.pop();
            }
            write!(f, "{}.{}", whole, frac_str)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratio_from_parts() {
        assert_eq!(ratio_from_parts(1, 2), 500_000);
        assert_eq!(ratio_from_parts(3, 3), RATIO_SCALE);
        assert_eq!(ratio_from_parts(0, 5), 0);
        assert_eq!(ratio_from_parts(5, 0), 0);
    }

    #[test]
    fn test_ratio_from_bps() {
        assert_eq!(ratio_from_bps(0), 0);
        assert_eq!(ratio_from_bps(10_000), RATIO_SCALE);
        assert_eq!(ratio_from_bps(6_700), 670_000);
    }

    #[test]
    fn test_ratio_to_bps() {
        assert_eq!(ratio_to_bps(0), 0);
        assert_eq!(ratio_to_bps(RATIO_SCALE), 10_000);
        assert_eq!(ratio_to_bps(250_000), 2_500);
    }

    #[test]
    fn test_ratio_display() {
        assert_eq!(format!("{}", format_ratio(0)), "0");
        assert_eq!(format!("{}", format_ratio(123_000)), "0.123");
        assert_eq!(format!("{}", format_ratio(1_000_000)), "1");
        assert_eq!(format!("{}", format_ratio(1_500_500)), "1.5005");
    }
}
