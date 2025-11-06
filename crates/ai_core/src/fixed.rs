//! Deterministic fixed-point arithmetic module
//!
//! Floating-point operations can produce non-deterministic results across CPU
//! architectures and compiler settings. To ensure deterministic AI inference
//! and validator scoring, this module implements a simple fixed-point type
//! backed by a 64-bit integer with micro (1e-6) precision.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Scaling factor: 1 unit = 1e-6
pub const SCALE: i64 = 1_000_000;

const SCALE_I128: i128 = SCALE as i128;

/// Deterministic fixed-point number with 6 decimal places of precision.
///
/// Internally represented as an `i64`, where 1.0 == 1_000_000.
/// Example:
/// ```
/// use ai_core::fixed::Fixed;
/// let x = Fixed::from_f64(1.234567);
/// assert!((x.to_f64() - 1.234567).abs() < 1e-12);
/// ```
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Default,
)]
pub struct Fixed(pub i64);

/// Errors that can occur during fixed-point operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FixedError {
    #[error("fixed-point overflow")]
    Overflow,
    #[error("fixed-point division by zero")]
    DivisionByZero,
    #[error("non-finite floating point value")]
    NonFinite,
}

impl Fixed {
    /// Construct from a raw integer value representing micros.
    #[inline]
    pub const fn new(raw: i64) -> Self {
        Fixed(raw)
    }

    /// Retrieve the raw micro value.
    #[inline]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// Construct from whole units (e.g. integer value `5` becomes `5.0`).
    #[inline]
    pub fn from_int(value: i64) -> Self {
        Self::try_from_int(value).expect("Fixed::from_int overflow")
    }

    /// Fallible constructor from whole units.
    #[inline]
    pub fn try_from_int(value: i64) -> Result<Self, FixedError> {
        value
            .checked_mul(SCALE)
            .map(Fixed)
            .ok_or(FixedError::Overflow)
    }

    /// Construct from (integer, micro) parts where micro is already scaled [0, SCALE).
    #[inline]
    pub fn from_parts(integer: i64, micro: i64) -> Self {
        Self::try_from_parts(integer, micro).expect("Fixed::from_parts overflow")
    }

    /// Fallible constructor from (integer, micro) parts where micro is already scaled [0, SCALE).
    #[inline]
    pub fn try_from_parts(integer: i64, micro: i64) -> Result<Self, FixedError> {
        let micro_abs = micro.checked_abs().ok_or(FixedError::Overflow)?;
        if micro_abs >= SCALE {
            return Err(FixedError::Overflow);
        }
        let base = integer.checked_mul(SCALE).ok_or(FixedError::Overflow)?;
        let raw = if integer >= 0 {
            base.checked_add(micro)
        } else {
            base.checked_sub(micro)
        };
        raw.map(Fixed).ok_or(FixedError::Overflow)
    }

    /// Convert an f64 value into fixed-point.
    #[inline]
    pub fn from_f64(value: f64) -> Self {
        Self::try_from_f64(value).expect("Fixed::from_f64 overflow")
    }

    /// Fallible conversion from f64 into fixed-point.
    #[inline]
    pub fn try_from_f64(value: f64) -> Result<Self, FixedError> {
        if !value.is_finite() {
            return Err(FixedError::NonFinite);
        }
        let scaled = (value * SCALE as f64).round();
        if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
            return Err(FixedError::Overflow);
        }
        Ok(Fixed(scaled as i64))
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
        self.checked_abs().expect("Fixed::abs overflow")
    }

    /// Checked absolute value.
    #[inline]
    pub fn checked_abs(self) -> Result<Self, FixedError> {
        self.0.checked_abs().map(Fixed).ok_or(FixedError::Overflow)
    }

    /// Checked addition.
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Result<Self, FixedError> {
        self.0
            .checked_add(rhs.0)
            .map(Fixed)
            .ok_or(FixedError::Overflow)
    }

    /// Checked subtraction.
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Result<Self, FixedError> {
        self.0
            .checked_sub(rhs.0)
            .map(Fixed)
            .ok_or(FixedError::Overflow)
    }

    /// Checked multiplication.
    #[inline]
    pub fn checked_mul(self, rhs: Self) -> Result<Self, FixedError> {
        let product = (self.0 as i128) * (rhs.0 as i128);
        let scaled = product / SCALE_I128;
        i128_to_i64(scaled).map(Fixed).ok_or(FixedError::Overflow)
    }

    /// Checked division.
    #[inline]
    pub fn checked_div(self, rhs: Self) -> Result<Self, FixedError> {
        if rhs.0 == 0 {
            return Err(FixedError::DivisionByZero);
        }
        let numerator = (self.0 as i128) * SCALE_I128;
        let quotient = numerator / (rhs.0 as i128);
        i128_to_i64(quotient).map(Fixed).ok_or(FixedError::Overflow)
    }
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let integer = self.0 / SCALE;
        let fractional = (self.0 % SCALE).abs();
        write!(f, "{}.{:06}", integer, fractional)
    }
}

impl Neg for Fixed {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fixed(self.0.checked_neg().expect("Fixed neg overflow"))
    }
}

impl Add for Fixed {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("Fixed addition overflow")
    }
}

impl Sub for Fixed {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("Fixed subtraction overflow")
    }
}

impl Mul for Fixed {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs)
            .expect("Fixed multiplication overflow")
    }
}

impl Div for Fixed {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).expect("Fixed division error")
    }
}

impl From<Fixed> for i64 {
    #[inline]
    fn from(value: Fixed) -> Self {
        value.raw()
    }
}

impl TryFrom<i64> for Fixed {
    type Error = FixedError;

    #[inline]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Fixed::try_from_int(value)
    }
}

impl TryFrom<f64> for Fixed {
    type Error = FixedError;

    #[inline]
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Fixed::try_from_f64(value)
    }
}

#[inline]
fn i128_to_i64(value: i128) -> Option<i64> {
    if value < i64::MIN as i128 || value > i64::MAX as i128 {
        None
    } else {
        Some(value as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(((a + b).to_f64() - 7.5).abs() < 1e-9);
        assert!(((a - b).to_f64() - 2.5).abs() < 1e-9);
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

    #[test]
    fn test_overflow_check() {
        let max = Fixed::new(i64::MAX);
        assert!(matches!(
            max.checked_add(Fixed::one()),
            Err(FixedError::Overflow)
        ));
    }

    #[test]
    fn test_division_by_zero() {
        let a = Fixed::from_f64(10.0);
        assert!(matches!(
            a.checked_div(Fixed::zero()),
            Err(FixedError::DivisionByZero)
        ));
    }

    #[test]
    fn test_try_from_int_overflow() {
        assert!(matches!(
            Fixed::try_from_int(i64::MAX),
            Err(FixedError::Overflow)
        ));
    }

    #[test]
    fn test_try_from_f64_non_finite() {
        assert!(matches!(
            Fixed::try_from_f64(f64::INFINITY),
            Err(FixedError::NonFinite)
        ));
    }
}
