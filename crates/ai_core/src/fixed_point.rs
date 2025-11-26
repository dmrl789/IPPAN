use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Fixed-point numeric representation using 1e-6 precision.
///
/// All deterministic AI computations operate on this integer-backed type to
/// avoid architecture-specific floating-point behaviour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct FixedPoint(i64);

impl FixedPoint {
    /// Scaling factor: 1 unit = 10^-6 in real numbers.
    pub const SCALE: i64 = 1_000_000;

    /// Creates a new fixed-point value from the underlying scaled integer.
    #[inline]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// Returns the underlying scaled integer.
    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Zero value.
    #[inline]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Create a fixed-point value from an integer (treated as whole units).
    #[inline]
    pub fn from_integer(value: i64) -> Self {
        Self(value.saturating_mul(Self::SCALE))
    }

    /// Create a fixed-point value from a numerator/denominator pair.
    ///
    /// This avoids floating point usage in deterministic code paths.
    #[inline]
    pub fn from_ratio(numerator: i64, denominator: i64) -> Self {
        debug_assert!(denominator != 0, "denominator must be non-zero");
        if denominator == 0 {
            return Self::zero();
        }
        let scaled = (i128::from(numerator) * i128::from(Self::SCALE)) / i128::from(denominator);
        Self(scaled as i64)
    }

    /// Convert the fixed-point value back to an integer, truncating towards zero.
    #[inline]
    pub fn to_integer(self) -> i64 {
        self.0 / Self::SCALE
    }
}

impl Add for FixedPoint {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for FixedPoint {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl Mul for FixedPoint {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        if self.0 == 0 || rhs.0 == 0 {
            return Self::zero();
        }
        let product = (i128::from(self.0) * i128::from(rhs.0)) / i128::from(Self::SCALE);
        Self(product as i64)
    }
}

impl Div for FixedPoint {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        debug_assert!(rhs.0 != 0, "division by zero in FixedPoint");
        if rhs.0 == 0 {
            return Self::zero();
        }
        let quotient = (i128::from(self.0) * i128::from(Self::SCALE)) / i128::from(rhs.0);
        Self(quotient as i64)
    }
}

impl Neg for FixedPoint {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self(self.0.saturating_neg())
    }
}

impl fmt::Display for FixedPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_part = self.0 / Self::SCALE;
        let frac = (self.0 % Self::SCALE).abs();
        write!(f, "{int_part}.{frac:06}")
    }
}

impl From<i64> for FixedPoint {
    #[inline]
    fn from(value: i64) -> Self {
        Self::from_integer(value)
    }
}

impl From<FixedPoint> for i64 {
    #[inline]
    fn from(value: FixedPoint) -> Self {
        value.to_integer()
    }
}

#[cfg(test)]
mod tests {
    use super::FixedPoint;

    #[test]
    fn fixed_point_add_mul() {
        let a = FixedPoint::from_ratio(1, 2); // 0.5
        let b = FixedPoint::from_ratio(3, 4); // 0.75
        let sum = a + b;
        assert_eq!(sum.raw(), 1_250_000); // 1.25
        let product = a * b;
        assert_eq!(product.raw(), 375_000); // 0.375
    }

    #[test]
    fn fixed_point_division() {
        let a = FixedPoint::from_ratio(3, 1); // 3.0
        let b = FixedPoint::from_ratio(2, 1); // 2.0
        let res = a / b;
        assert_eq!(res.raw(), 1_500_000); // 1.5
    }
}
