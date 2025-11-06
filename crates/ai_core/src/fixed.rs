//! Deterministic fixed-point arithmetic module
//!
//! Floating-point operations can produce non-deterministic results across CPU
//! architectures and compiler settings. To ensure deterministic AI inference
//! and validator scoring, this module implements a simple fixed-point type
//! backed by a 64-bit integer with micro (1e-6) precision.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Scaling factor: 1 unit = 1e-6
const SCALE: i64 = 1_000_000;

/// Deterministic fixed-point number with 6 decimal places of precision.
///
/// Internally represented as an `i64`, where 1.0 == 1_000_000.
/// Example:
/// ```
/// use ippan_ai_core::fixed::Fixed;
/// let x = Fixed::from_f64(1.234567);
/// assert_eq!(x.to_f64(), 1.234567);
/// ```
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fixed(pub i64);

impl Fixed {
    /// Construct from a raw integer value.
    #[inline]
    pub fn new(raw: i64) -> Self {
        Fixed(raw)
    }

    /// Convert an f64 value into fixed-point.
    #[inline]
    pub fn from_f64(value: f64) -> Self {
        Fixed((value * SCALE as f64).round() as i64)
    }

    /// Convert fixed-point back to f64.
    #[inline]
    pub fn to_f64(self) -> f64 {
        (self.0 as f64) / SCALE as f64
    }

    /// Zero constant.
    #[inline]
    pub const fn zero() -> Self {
        Fixed(0)
    }

    /// One constant (1.0)
    #[inline]
    pub const fn one() -> Self {
        Fixed(SCALE)
    }

    /// Absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        Fixed(self.0.abs())
    }
}

impl Neg for Fixed {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Fixed(-self.0)
    }
}

impl Add for Fixed {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Fixed(self.0 + rhs.0)
    }
}

impl Sub for Fixed {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Fixed(self.0 - rhs.0)
    }
}

impl Mul for Fixed {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Fixed(((self.0 as i128 * rhs.0 as i128) / SCALE as i128) as i64)
    }
}

impl Div for Fixed {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Fixed(((self.0 as i128 * SCALE as i128) / rhs.0 as i128) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_basic_operations() {
        let a = Fixed::from_f64(1.5);
        let b = Fixed::from_f64(2.25);
        let res = a * b;
        assert!((res.to_f64() - 3.375).abs() < 1e-6);
    }

    #[test]
    fn test_addition_subtraction() {
        let a = Fixed::from_f64(5.0);
        let b = Fixed::from_f64(2.5);
        assert_eq!((a + b).to_f64(), 7.5);
        assert_eq!((a - b).to_f64(), 2.5);
    }

    #[test]
    fn test_serialization_determinism() {
        let x = Fixed::from_f64(123.456789);
        let y = Fixed::from_f64(123.456789);
        let sx = serde_json::to_string(&x).unwrap();
        let sy = serde_json::to_string(&y).unwrap();
        assert_eq!(sx, sy);
    }

    #[test]
    fn test_blake3_hash_consistency() {
        let x = Fixed::from_f64(1.234567);
        let y = Fixed::from_f64(1.234567);
        let hx = blake3::hash(&serde_json::to_vec(&x).unwrap());
        let hy = blake3::hash(&serde_json::to_vec(&y).unwrap());
        assert_eq!(hx, hy);
    }
}
