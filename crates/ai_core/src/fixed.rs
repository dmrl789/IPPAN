//! Deterministic fixed-point arithmetic with micro precision (1e-6).
//!
//! The [`Fixed`] type wraps a signed 64-bit integer whose value is interpreted
//! as `raw / SCALE`. All arithmetic uses saturating integer math so that the
//! results are deterministic across targets and do not rely on floating point
//! behaviour.

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Number of fractional units stored per whole number (micro precision).
pub const SCALE: i64 = 1_000_000;

/// Fixed-point number with deterministic, saturating arithmetic.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fixed(i64);

impl Fixed {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(SCALE);
    pub const NEG_ONE: Self = Self(-SCALE);
    pub const MAX: Self = Self(i64::MAX);
    pub const MIN: Self = Self(i64::MIN);

    /// Create a fixed value from the underlying micro units.
    #[inline]
    pub const fn from_micro(raw: i64) -> Self {
        Self(raw)
    }

    /// Return the raw micro units backing this value.
    #[inline]
    pub const fn to_micro(self) -> i64 {
        self.0
    }

    /// Construct from an integer (treated as whole units).
    pub fn from_int(value: i64) -> Self {
        let scaled = (value as i128) * (SCALE as i128);
        Self::from_i128(scaled)
    }

    /// Construct from a rational value `(numerator / denominator)`.
    pub fn from_ratio(numerator: i64, denominator: i64) -> Self {
        if denominator == 0 {
            return Self::ZERO;
        }
        let scaled = (i128::from(numerator) * i128::from(SCALE)) / i128::from(denominator);
        Self::from_i128(scaled)
    }

    /// Construct from a decimal string such as `"12.3456"` or `"-0.001"`.
    pub fn from_decimal_str(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let (negative, digits) = match trimmed.as_bytes()[0] {
            b'-' => (true, &trimmed[1..]),
            b'+' => (false, &trimmed[1..]),
            _ => (false, trimmed),
        };

        if digits.is_empty() {
            return None;
        }

        let mut split = digits.split('.');
        let int_part = split.next().unwrap_or("0");
        let frac_part = split.next().unwrap_or("");
        if split.next().is_some() {
            return None;
        }

        if !int_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if !frac_part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let int_value: i128 = if int_part.is_empty() {
            0
        } else {
            int_part.parse().ok()?
        };

        let frac_digits = frac_part.chars().take(6).collect::<String>();
        let frac_len = frac_digits.len() as u32;
        let frac_value: i128 = if frac_len == 0 {
            0
        } else {
            let parsed: i128 = frac_digits.parse().ok()?;
            let padding = 6 - frac_len;
            parsed * i128::from(10_i64.pow(padding))
        };

        let mut raw = int_value
            .checked_mul(i128::from(SCALE))
            .and_then(|base| base.checked_add(frac_value))?;
        if negative {
            raw = -raw;
        }

        Some(Self::from_i128(raw))
    }

    /// Construct from a value expressed with `decimals` digits of precision.
    ///
    /// Example: `from_scaled_units(118, 2)` => `1.18`.
    pub fn from_scaled_units(value: i64, decimals: u32) -> Self {
        if decimals == 0 {
            return Self::from_int(value);
        }
        let scale = i128::from(10_i64.pow(decimals.min(18)));
        let raw = (i128::from(value) * i128::from(SCALE)) / scale;
        Self::from_i128(raw)
    }

    /// True if the value is exactly zero.
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Absolute value (saturating at `i64::MAX`).
    pub fn abs(self) -> Self {
        if self.0 == i64::MIN {
            Self::MAX
        } else {
            Self(self.0.abs())
        }
    }

    /// Clamp between `min` and `max`.
    pub fn clamp(self, min: Self, max: Self) -> Self {
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }

    /// Multiply by an integer scalar with saturation.
    pub fn mul_int(self, rhs: i64) -> Self {
        let product = i128::from(self.0) * i128::from(rhs);
        Self::from_i128(product)
    }

    /// Divide by an integer scalar (returns zero if the divisor is zero).
    pub fn div_int(self, rhs: i64) -> Self {
        if rhs == 0 {
            return Self::ZERO;
        }
        Self::from_i128(i128::from(self.0) / i128::from(rhs))
    }

    /// Whether the value is strictly negative.
    #[inline]
    pub const fn is_negative(self) -> bool {
        self.0.is_negative()
    }

    fn from_i128(value: i128) -> Self {
        let clamped = value.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
        Self(clamped as i64)
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_part = self.0 / SCALE;
        let frac = (self.0 % SCALE).abs();
        write!(f, "{int_part}.{frac:06}")
    }
}

impl fmt::Debug for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed({self})")
    }
}

impl Add for Fixed {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for Fixed {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Fixed {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl Mul for Fixed {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        if self.is_zero() || rhs.is_zero() {
            return Self::ZERO;
        }
        let product = (i128::from(self.0) * i128::from(rhs.0)) / i128::from(SCALE);
        Self::from_i128(product)
    }
}

impl MulAssign for Fixed {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for Fixed {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_zero() {
            return Self::ZERO;
        }
        let quotient = (i128::from(self.0) * i128::from(SCALE)) / i128::from(rhs.0);
        Self::from_i128(quotient)
    }
}

impl DivAssign for Fixed {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Neg for Fixed {
    type Output = Self;

    fn neg(self) -> Self::Output {
        if self.0 == i64::MIN {
            Self::MAX
        } else {
            Self(-self.0)
        }
    }
}

impl From<Fixed> for i64 {
    fn from(value: Fixed) -> Self {
        value.to_micro() / SCALE
    }
}

/// Compute the BLAKE3 hash of a single fixed-point value.
pub fn hash_fixed(value: Fixed) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&value.to_micro().to_le_bytes());
    *hasher.finalize().as_bytes()
}

/// Compute the BLAKE3 hash of a sequence of fixed-point values.
pub fn hash_fixed_slice(values: &[Fixed]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    for value in values {
        hasher.update(&value.to_micro().to_le_bytes());
    }
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition_and_subtraction() {
        let a = Fixed::from_int(2);
        let b = Fixed::from_ratio(1, 2);
        assert_eq!((a + b).to_micro(), 2_500_000);
        assert_eq!((a - b).to_micro(), 1_500_000);
    }

    #[test]
    fn multiplication_and_division() {
        let a = Fixed::from_ratio(3, 2); // 1.5
        let b = Fixed::from_ratio(5, 1); // 5.0
        assert_eq!((a * b).to_micro(), 7_500_000);
        assert_eq!((a * b / b).to_micro(), a.to_micro());
    }

    #[test]
    fn decimal_parsing() {
        let val = Fixed::from_decimal_str("12.345678").unwrap();
        assert_eq!(val.to_micro(), 12_345_678);
        let neg = Fixed::from_decimal_str("-0.500001").unwrap();
        assert_eq!(neg.to_micro(), -500_001);
        assert!(Fixed::from_decimal_str("abc").is_none());
    }

    #[test]
    fn scaled_units() {
        let latency = Fixed::from_scaled_units(118, 2);
        assert_eq!(latency.to_micro(), 1_180_000);
    }

    #[test]
    fn hashing_slice() {
        let hash = hash_fixed_slice(&[Fixed::from_int(1), Fixed::from_int(2)]);
        let hash_again = hash_fixed_slice(&[Fixed::from_int(1), Fixed::from_int(2)]);
        assert_eq!(hash, hash_again);
    }
}
