//! Deterministic fixed-point arithmetic utilities for `ai_core`.
//!
//! This module provides a micro-precision (1e-6) fixed-point type that
//! guarantees bit-for-bit reproducibility across architectures. All
//! operations promote intermediate values to `i128` to avoid overflow and
//! rounding inconsistencies when multiplying or dividing.

use core::fmt::{self, Display, Formatter};
use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// The scaling factor used for the fixed-point representation (1e-6).
pub const SCALE: i64 = 1_000_000;

/// Deterministic fixed-point 64-bit number with micro precision.
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
pub struct Fixed(pub i64);

impl Fixed {
    /// Creates a fixed-point value from a raw scaled integer.
    #[inline]
    pub const fn from_scaled(raw: i64) -> Self {
        Self(raw)
    }

    /// Returns the raw scaled integer value.
    #[inline]
    pub const fn into_inner(self) -> i64 {
        self.0
    }

    /// Creates a fixed-point value from an `f64` by rounding to the nearest micro unit.
    #[inline]
    pub fn from_f64(value: f64) -> Self {
        Self((value * SCALE as f64).round() as i64)
    }

    /// Creates a fixed-point value from an `f32` by rounding to the nearest micro unit.
    #[inline]
    pub fn from_f32(value: f32) -> Self {
        Self::from_f64(value as f64)
    }

    /// Converts the fixed-point value back to an `f64`.
    #[inline]
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / SCALE as f64
    }

    /// Converts the fixed-point value back to an `f32`.
    #[inline]
    pub fn to_f32(self) -> f32 {
        self.to_f64() as f32
    }

    /// Multiplies two fixed-point numbers with rounding toward zero.
    #[inline]
    pub fn mul(self, rhs: Self) -> Self {
        let product = self.0 as i128 * rhs.0 as i128;
        Self(((product) / SCALE as i128) as i64)
    }

    /// Divides two fixed-point numbers with rounding toward zero.
    ///
    /// # Panics
    /// Panics if `rhs` is zero.
    #[inline]
    pub fn div(self, rhs: Self) -> Self {
        assert!(rhs.0 != 0, "division by zero in Fixed");
        let numerator = self.0 as i128 * SCALE as i128;
        Self((numerator / rhs.0 as i128) as i64)
    }
}

impl Default for Fixed {
    fn default() -> Self {
        Self(0)
    }
}

impl Display for Fixed {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6}", self.to_f64())
    }
}

impl From<i64> for Fixed {
    #[inline]
    fn from(value: i64) -> Self {
        Self(value * SCALE)
    }
}

impl From<i32> for Fixed {
    #[inline]
    fn from(value: i32) -> Self {
        Self::from(value as i64)
    }
}

impl From<Fixed> for f64 {
    #[inline]
    fn from(value: Fixed) -> Self {
        value.to_f64()
    }
}

impl From<Fixed> for f32 {
    #[inline]
    fn from(value: Fixed) -> Self {
        value.to_f32()
    }
}

impl Add for Fixed {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Fixed {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Fixed {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Fixed {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Mul for Fixed {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        self.mul(rhs)
    }
}

impl MulAssign for Fixed {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.mul(rhs);
    }
}

impl Div for Fixed {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        self.div(rhs)
    }
}

impl DivAssign for Fixed {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        *self = self.div(rhs);
    }
}

impl Sum for Fixed {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, x| acc + x)
    }
}

impl<'a> Sum<&'a Fixed> for Fixed {
    fn sum<I: Iterator<Item = &'a Fixed>>(iter: I) -> Self {
        iter.fold(Self::default(), |acc, x| acc + *x)
    }
}

impl Product for Fixed {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::from_scaled(SCALE), |acc, x| acc * x)
    }
}

impl<'a> Product<&'a Fixed> for Fixed {
    fn product<I: Iterator<Item = &'a Fixed>>(iter: I) -> Self {
        iter.fold(Self::from_scaled(SCALE), |acc, x| acc * *x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_roundtrip_precision() {
        let value = 123.456_789_f64;
        let fixed = Fixed::from_f64(value);
        let roundtrip = fixed.to_f64();
        assert!((value - roundtrip).abs() < 1e-6);
    }

    #[test]
    fn fixed_basic_operations() {
        let a = Fixed::from_f64(1.5);
        let b = Fixed::from_f64(2.25);

        assert_eq!((a + b).to_f64(), 3.75);
        assert_eq!((b - a).to_f64(), 0.75);
        assert_eq!((a * b).to_f64(), 3.375);
        assert_eq!((b / a).to_f64(), 1.5);
    }

    #[test]
    fn fixed_sum_and_product() {
        let values = [0.1, 0.2, 0.3, 0.4];
        let fixed_values: Vec<_> = values.iter().copied().map(Fixed::from_f64).collect();

        let sum: Fixed = fixed_values.iter().copied().sum();
        assert_eq!(sum.to_f64(), 1.0);

        let product: Fixed = fixed_values.iter().copied().product();
        assert_eq!(product.to_f64(), 0.0024);
    }

    #[test]
    fn fixed_serialization_is_deterministic() {
        let a = Fixed::from_f64(42.1337);
        let b = Fixed::from_f64(42.1337);

        let sa = serde_json::to_string(&a).unwrap();
        let sb = serde_json::to_string(&b).unwrap();

        assert_eq!(sa, sb);
        assert_eq!(sa, "42133700");
    }

    #[test]
    fn fixed_display_outputs_micro_precision() {
        let value = Fixed::from_f64(-12.345_678);
        assert_eq!(format!("{}", value), "-12.345678");
    }
}
