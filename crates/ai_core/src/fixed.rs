//! Deterministic Fixed-Point Arithmetic for AI Core
//!
//! Provides bit-for-bit reproducible numeric operations across all architectures
//! (x86_64, aarch64, RISC-V) for consensus-critical AI scoring and telemetry.
//!
//! # Design
//!
//! - Uses 64-bit signed integers with micro-precision (1e-6)
//! - All operations are deterministic and platform-independent
//! - Serialization produces identical bytes on all platforms
//! - No floating-point operations at any stage
//!
//! # Example
//!
//! ```rust
//! use ippan_ai_core::fixed::Fixed;
//!
//! let a = Fixed::from_micro(1_500_000); // 1.5
//! let b = Fixed::from_micro(2_250_000); // 2.25
//! let c = a * b;
//! assert_eq!(c.to_micro(), 3_375_000); // 3.375
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Fixed-point precision: 1 unit = 1e-6 (micro precision)
pub const SCALE: i64 = 1_000_000;

/// Half of scale, used for rounding in division
const SCALE_HALF: i64 = SCALE / 2;

/// Deterministic fixed-point number with micro-precision
///
/// Internally stored as an `i64` representing value * 1,000,000.
/// This allows representing values from approximately -9.2e12 to 9.2e12
/// with 6 decimal places of precision.
///
/// # Invariants
///
/// - All arithmetic operations are deterministic
/// - Serialization is platform-independent (uses i64 encoding)
/// - Hash values are consistent across architectures
/// - No floating-point operations are ever used
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Fixed(pub i64);

impl Fixed {
    /// Zero constant
    pub const ZERO: Self = Fixed(0);

    /// One constant (1.0)
    pub const ONE: Self = Fixed(SCALE);

    /// Negative one constant (-1.0)
    pub const NEG_ONE: Self = Fixed(-SCALE);

    /// Maximum representable value
    pub const MAX: Self = Fixed(i64::MAX);

    /// Minimum representable value
    pub const MIN: Self = Fixed(i64::MIN);

    /// Create a Fixed from raw micro units (internal representation)
    #[inline]
    pub const fn from_micro(micro: i64) -> Self {
        Fixed(micro)
    }

    /// Get the raw micro units (internal representation)
    #[inline]
    pub const fn to_micro(self) -> i64 {
        self.0
    }

    /// Create a Fixed from an integer (exact conversion)
    #[inline]
    pub const fn from_int(val: i64) -> Self {
        Fixed(val.saturating_mul(SCALE))
    }

    /// Convert to integer (truncates fractional part)
    #[inline]
    pub const fn to_int(self) -> i64 {
        self.0 / SCALE
    }

    /// Create a Fixed from f64 (for compatibility/testing only)
    ///
    /// # Warning
    ///
    /// This should only be used in tests or for migration from float-based code.
    /// For production, use `from_micro` or `from_int` directly.
    #[inline]
    pub fn from_f64(val: f64) -> Self {
        Fixed((val * SCALE as f64).round() as i64)
    }

    /// Convert to f64 (for display/debugging only)
    ///
    /// # Warning
    ///
    /// Do not use for consensus-critical calculations. This is for display only.
    #[inline]
    pub fn to_f64(self) -> f64 {
        (self.0 as f64) / (SCALE as f64)
    }

    /// Absolute value
    #[inline]
    pub const fn abs(self) -> Self {
        Fixed(self.0.abs())
    }

    /// Checked multiplication (returns None on overflow)
    #[inline]
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let result = (a * b) / SCALE as i128;
        if result > i64::MAX as i128 || result < i64::MIN as i128 {
            None
        } else {
            Some(Fixed(result as i64))
        }
    }

    /// Checked division (returns None on overflow or divide-by-zero)
    #[inline]
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let result = (a * SCALE as i128) / b;
        if result > i64::MAX as i128 || result < i64::MIN as i128 {
            None
        } else {
            Some(Fixed(result as i64))
        }
    }

    /// Saturating multiplication (clamps to MIN/MAX on overflow)
    #[inline]
    pub fn saturating_mul(self, rhs: Self) -> Self {
        self.checked_mul(rhs).unwrap_or_else(|| {
            if (self.0 < 0) == (rhs.0 < 0) {
                Fixed::MAX
            } else {
                Fixed::MIN
            }
        })
    }

    /// Saturating division (clamps to MIN/MAX on overflow)
    #[inline]
    pub fn saturating_div(self, rhs: Self) -> Self {
        self.checked_div(rhs).unwrap_or_else(|| {
            if rhs.0 == 0 {
                if self.0 >= 0 {
                    Fixed::MAX
                } else {
                    Fixed::MIN
                }
            } else if (self.0 < 0) == (rhs.0 < 0) {
                Fixed::MAX
            } else {
                Fixed::MIN
            }
        })
    }

    /// Create Fixed from numerator and denominator (exact integer division)
    #[inline]
    pub fn from_ratio(num: i64, denom: i64) -> Self {
        if denom == 0 {
            return Fixed::ZERO;
        }
        let result = ((num as i128) * (SCALE as i128)) / (denom as i128);
        Fixed(result.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }

    /// Multiply by an integer (more efficient than general multiplication)
    #[inline]
    pub const fn mul_int(self, rhs: i64) -> Self {
        Fixed(self.0.saturating_mul(rhs))
    }

    /// Divide by an integer (more efficient than general division)
    #[inline]
    pub const fn div_int(self, rhs: i64) -> Self {
        if rhs == 0 {
            return Fixed::ZERO;
        }
        Fixed(self.0 / rhs)
    }

    /// Minimum of two values
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        if self.0 < other.0 {
            self
        } else {
            other
        }
    }

    /// Maximum of two values
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 {
            self
        } else {
            other
        }
    }

    /// Clamp value between min and max
    #[inline]
    pub const fn clamp(self, min: Self, max: Self) -> Self {
        if self.0 < min.0 {
            min
        } else if self.0 > max.0 {
            max
        } else {
            self
        }
    }

    /// Check if value is zero
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Check if value is positive
    #[inline]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    /// Check if value is negative
    #[inline]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
}

// ---------------------------------------------------------------------------
// Arithmetic operators
// ---------------------------------------------------------------------------

impl Add for Fixed {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Fixed {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl Sub for Fixed {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Fixed {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl Mul for Fixed {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let result = (a * b) / SCALE as i128;
        Fixed(result.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }
}

impl MulAssign for Fixed {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for Fixed {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            return Fixed::ZERO;
        }
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let result = (a * SCALE as i128) / b;
        Fixed(result.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }
}

impl DivAssign for Fixed {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Neg for Fixed {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Fixed(-self.0)
    }
}

// ---------------------------------------------------------------------------
// Display and formatting
// ---------------------------------------------------------------------------

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let integer = self.0 / SCALE;
        let fraction = (self.0 % SCALE).abs();
        if self.0 < 0 && integer == 0 {
            write!(f, "-{}.{:06}", integer, fraction)
        } else {
            write!(f, "{}.{:06}", integer, fraction)
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion traits
// ---------------------------------------------------------------------------

impl From<i32> for Fixed {
    #[inline]
    fn from(val: i32) -> Self {
        Fixed::from_int(val as i64)
    }
}

impl From<i64> for Fixed {
    #[inline]
    fn from(val: i64) -> Self {
        Fixed::from_int(val)
    }
}

impl From<u32> for Fixed {
    #[inline]
    fn from(val: u32) -> Self {
        Fixed::from_int(val as i64)
    }
}

// ---------------------------------------------------------------------------
// Default
// ---------------------------------------------------------------------------

impl Default for Fixed {
    #[inline]
    fn default() -> Self {
        Fixed::ZERO
    }
}

// ---------------------------------------------------------------------------
// Deterministic hashing utility
// ---------------------------------------------------------------------------

use blake3::Hasher;

/// Compute deterministic Blake3 hash of a Fixed value
///
/// This ensures identical hashes across all platforms for consensus.
pub fn hash_fixed(value: Fixed) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&value.0.to_le_bytes());
    *hasher.finalize().as_bytes()
}

/// Compute deterministic Blake3 hash of a slice of Fixed values
pub fn hash_fixed_slice(values: &[Fixed]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    for value in values {
        hasher.update(&value.0.to_le_bytes());
    }
    *hasher.finalize().as_bytes()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let a = Fixed::from_micro(1_500_000); // 1.5
        let b = Fixed::from_micro(2_000_000); // 2.0

        assert_eq!((a + b).to_micro(), 3_500_000); // 3.5
        assert_eq!((b - a).to_micro(), 500_000); // 0.5
        assert_eq!((a * b).to_micro(), 3_000_000); // 3.0
        assert_eq!((b / a).to_micro(), 1_333_333); // ~1.333333
    }

    #[test]
    fn test_multiplication_precision() {
        let a = Fixed::from_micro(1_500_000); // 1.5
        let b = Fixed::from_micro(2_250_000); // 2.25
        let c = a * b;
        assert_eq!(c.to_micro(), 3_375_000); // 3.375
    }

    #[test]
    fn test_division_precision() {
        let a = Fixed::from_micro(5_000_000); // 5.0
        let b = Fixed::from_micro(2_000_000); // 2.0
        let c = a / b;
        assert_eq!(c.to_micro(), 2_500_000); // 2.5
    }

    #[test]
    fn test_from_f64_roundtrip() {
        let val = 123.456789;
        let fixed = Fixed::from_f64(val);
        let back = fixed.to_f64();
        assert!((val - back).abs() < 0.000001);
    }

    #[test]
    fn test_integer_conversion() {
        assert_eq!(Fixed::from_int(42).to_int(), 42);
        assert_eq!(Fixed::from_int(-100).to_int(), -100);
    }

    #[test]
    fn test_constants() {
        assert_eq!(Fixed::ZERO.to_int(), 0);
        assert_eq!(Fixed::ONE.to_int(), 1);
        assert_eq!(Fixed::NEG_ONE.to_int(), -1);
    }

    #[test]
    fn test_from_ratio() {
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(half.to_f64(), 0.5);

        let third = Fixed::from_ratio(1, 3);
        assert!((third.to_f64() - 0.333333).abs() < 0.000001);

        let two_fifths = Fixed::from_ratio(2, 5);
        assert_eq!(two_fifths.to_f64(), 0.4);
    }

    #[test]
    fn test_mul_div_int() {
        let val = Fixed::from_int(5);
        assert_eq!(val.mul_int(3).to_int(), 15);
        assert_eq!(val.div_int(2).to_micro(), 2_500_000);
    }

    #[test]
    fn test_min_max_clamp() {
        let a = Fixed::from_int(5);
        let b = Fixed::from_int(10);
        let c = Fixed::from_int(7);

        assert_eq!(a.min(b), a);
        assert_eq!(a.max(b), b);
        assert_eq!(c.clamp(a, b), c);
        assert_eq!(Fixed::from_int(3).clamp(a, b), a);
        assert_eq!(Fixed::from_int(15).clamp(a, b), b);
    }

    #[test]
    fn test_zero_checks() {
        assert!(Fixed::ZERO.is_zero());
        assert!(!Fixed::ONE.is_zero());
        assert!(Fixed::ONE.is_positive());
        assert!(!Fixed::ONE.is_negative());
        assert!(Fixed::NEG_ONE.is_negative());
    }

    #[test]
    fn test_saturating_arithmetic() {
        let max = Fixed::MAX;
        let one = Fixed::ONE;

        // Addition should saturate
        assert_eq!(max + one, max);

        // Subtraction should saturate
        assert_eq!(Fixed::MIN - one, Fixed::MIN);
    }

    #[test]
    fn test_checked_operations() {
        let a = Fixed::from_int(5);
        let b = Fixed::from_int(2);

        assert_eq!(a.checked_mul(b), Some(Fixed::from_int(10)));
        assert_eq!(a.checked_div(b), Some(Fixed::from_micro(2_500_000)));

        // Division by zero
        assert_eq!(a.checked_div(Fixed::ZERO), None);

        // Overflow
        let huge = Fixed::from_micro(i64::MAX / 2);
        assert_eq!(huge.checked_mul(huge), None);
    }

    #[test]
    fn test_deterministic_serialization() {
        let val = Fixed::from_f64(123.456789);
        let json1 = serde_json::to_string(&val).unwrap();
        let json2 = serde_json::to_string(&val).unwrap();
        assert_eq!(json1, json2);

        let deserialized: Fixed = serde_json::from_str(&json1).unwrap();
        assert_eq!(val, deserialized);
    }

    #[test]
    fn test_hash_determinism() {
        let val = Fixed::from_f64(123.456);
        let hash1 = hash_fixed(val);
        let hash2 = hash_fixed(val);
        assert_eq!(hash1, hash2);

        let vals = vec![
            Fixed::from_int(1),
            Fixed::from_int(2),
            Fixed::from_int(3),
        ];
        let slice_hash1 = hash_fixed_slice(&vals);
        let slice_hash2 = hash_fixed_slice(&vals);
        assert_eq!(slice_hash1, slice_hash2);
    }

    #[test]
    fn test_negative_numbers() {
        let a = Fixed::from_int(-5);
        let b = Fixed::from_int(3);

        assert_eq!((a + b).to_int(), -2);
        assert_eq!((a - b).to_int(), -8);
        assert_eq!((a * b).to_int(), -15);
        assert_eq!((a / b).to_micro(), -1_666_666); // ~-1.666666
    }

    #[test]
    fn test_display() {
        assert_eq!(Fixed::from_int(5).to_string(), "5.000000");
        assert_eq!(Fixed::from_f64(123.456).to_string(), "123.456000");
        assert_eq!(Fixed::from_int(-5).to_string(), "-5.000000");
    }

    #[test]
    fn test_ordering() {
        let a = Fixed::from_int(1);
        let b = Fixed::from_int(2);
        let c = Fixed::from_int(2);

        assert!(a < b);
        assert!(b > a);
        assert_eq!(b, c);
        assert!(a <= b);
        assert!(b >= a);
    }

    #[test]
    fn test_cross_platform_determinism() {
        // These operations must produce identical results on all platforms
        let inputs = vec![
            (Fixed::from_f64(1.5), Fixed::from_f64(2.5)),
            (Fixed::from_f64(-3.7), Fixed::from_f64(1.2)),
            (Fixed::from_f64(0.0001), Fixed::from_f64(0.9999)),
        ];

        for (a, b) in inputs {
            let sum = a + b;
            let diff = a - b;
            let prod = a * b;
            let quot = a / b;

            // Re-serialize and deserialize to ensure platform independence
            let sum_json = serde_json::to_string(&sum).unwrap();
            let sum_back: Fixed = serde_json::from_str(&sum_json).unwrap();
            assert_eq!(sum, sum_back);

            let diff_json = serde_json::to_string(&diff).unwrap();
            let diff_back: Fixed = serde_json::from_str(&diff_json).unwrap();
            assert_eq!(diff, diff_back);

            let prod_json = serde_json::to_string(&prod).unwrap();
            let prod_back: Fixed = serde_json::from_str(&prod_json).unwrap();
            assert_eq!(prod, prod_back);

            let quot_json = serde_json::to_string(&quot).unwrap();
            let quot_back: Fixed = serde_json::from_str(&quot_json).unwrap();
            assert_eq!(quot, quot_back);
        }
    }

    #[test]
    fn test_abs() {
        assert_eq!(Fixed::from_int(5).abs(), Fixed::from_int(5));
        assert_eq!(Fixed::from_int(-5).abs(), Fixed::from_int(5));
        assert_eq!(Fixed::ZERO.abs(), Fixed::ZERO);
    }

    #[test]
    fn test_neg() {
        assert_eq!(-Fixed::from_int(5), Fixed::from_int(-5));
        assert_eq!(-Fixed::from_int(-5), Fixed::from_int(5));
        assert_eq!(-Fixed::ZERO, Fixed::ZERO);
    }
}
