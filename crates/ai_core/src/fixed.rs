//! Deterministic Fixed-Point Arithmetic for AI Core
//!
//! Provides bit-for-bit reproducible numeric operations across all architectures
//! (x86_64, aarch64, RISC-V) for consensus-critical AI scoring and telemetry.
//!
//! # Design
//! - Uses 64-bit signed integers with micro-precision (1e-6)
//! - All operations are deterministic and platform-independent
//! - Serialization produces identical bytes on all platforms
//! - No floating-point operations are ever used

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Fixed-point precision: 1 unit = 1e-6 (micro precision)
pub const SCALE: i64 = 1_000_000;

/// Deterministic fixed-point number with micro-precision.
/// Internally stored as `i64`, representing value * 1_000_000.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Fixed(pub i64);

impl Default for Fixed {
    fn default() -> Self {
        Fixed::ZERO
    }
}

// Custom serialization to support both JSON floats and integers
impl Serialize for Fixed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Always serialize as integer for deterministic output
        serializer.serialize_i64(self.0)
    }
}

impl<'de> Deserialize<'de> for Fixed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FixedVisitor;

        impl<'de> Visitor<'de> for FixedVisitor {
            type Value = Fixed;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a fixed-point value encoded as integer micro-units or decimal number",
                )
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Fixed::from_micro(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value > i64::MAX as u64 {
                    return Err(de::Error::custom("fixed-point value out of range"));
                }
                Ok(Fixed::from_micro(value as i64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if !value.is_finite() {
                    return Err(de::Error::custom("non-finite fixed-point value"));
                }
                let scaled = (value * SCALE as f64).round();
                if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
                    return Err(de::Error::custom("fixed-point value out of range"));
                }
                Ok(Fixed::from_micro(scaled as i64))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(int_micro) = value.parse::<i64>() {
                    return self.visit_i64(int_micro);
                }
                let float_val = value
                    .parse::<f64>()
                    .map_err(|_| de::Error::custom("invalid fixed-point string"))?;
                self.visit_f64(float_val)
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_any(FixedVisitor)
        } else {
            deserializer.deserialize_i64(FixedVisitor)
        }
    }
}

impl Fixed {
    pub const ZERO: Self = Fixed(0);
    pub const ONE: Self = Fixed(SCALE);
    pub const NEG_ONE: Self = Fixed(-SCALE);
    pub const MAX: Self = Fixed(i64::MAX);
    pub const MIN: Self = Fixed(i64::MIN);

    /// Create a Fixed from raw micro units
    #[inline]
    pub const fn from_micro(micro: i64) -> Self {
        Fixed(micro)
    }

    /// Return raw micro units
    #[inline]
    pub const fn to_micro(self) -> i64 {
        self.0
    }

    /// Create from integer
    #[inline]
    pub const fn from_int(val: i64) -> Self {
        Fixed(val.saturating_mul(SCALE))
    }

    /// Convert to integer (truncating)
    #[inline]
    pub const fn to_int(self) -> i64 {
        self.0 / SCALE
    }

    /// Create from f64 (for testing / debugging only)
    #[inline]
    pub fn from_f64(val: f64) -> Self {
        Fixed((val * SCALE as f64).round() as i64)
    }

    /// Convert to f64 (for display/debug only)
    #[inline]
    pub fn to_f64(self) -> f64 {
        (self.0 as f64) / SCALE as f64
    }

    /// Absolute value
    #[inline]
    pub const fn abs(self) -> Self {
        Fixed(self.0.abs())
    }

    /// Checked multiplication
    #[inline]
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let r = (a * b) / SCALE as i128;
        if r > i64::MAX as i128 || r < i64::MIN as i128 {
            None
        } else {
            Some(Fixed(r as i64))
        }
    }

    /// Checked division
    #[inline]
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        let a = self.0 as i128;
        let b = rhs.0 as i128;
        let r = (a * SCALE as i128) / b;
        if r > i64::MAX as i128 || r < i64::MIN as i128 {
            None
        } else {
            Some(Fixed(r as i64))
        }
    }

    /// Saturating multiplication
    #[inline]
    pub fn saturating_mul(self, rhs: Self) -> Self {
        self.checked_mul(rhs).unwrap_or({
            if (self.0 < 0) == (rhs.0 < 0) {
                Fixed::MAX
            } else {
                Fixed::MIN
            }
        })
    }

    /// Saturating division
    #[inline]
    pub fn saturating_div(self, rhs: Self) -> Self {
        self.checked_div(rhs).unwrap_or({
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

    /// Create Fixed from integer ratio
    #[inline]
    pub fn from_ratio(num: i64, denom: i64) -> Self {
        if denom == 0 {
            return Fixed::ZERO;
        }
        let r = ((num as i128) * SCALE as i128) / denom as i128;
        Fixed(r.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }

    #[inline]
    pub const fn mul_int(self, rhs: i64) -> Self {
        Fixed(self.0.saturating_mul(rhs))
    }

    #[inline]
    pub const fn div_int(self, rhs: i64) -> Self {
        if rhs == 0 {
            return Fixed::ZERO;
        }
        Fixed(self.0 / rhs)
    }

    #[inline]
    pub const fn min(self, other: Self) -> Self {
        if self.0 < other.0 {
            self
        } else {
            other
        }
    }

    #[inline]
    pub const fn max(self, other: Self) -> Self {
        if self.0 > other.0 {
            self
        } else {
            other
        }
    }

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

    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }

    #[inline]
    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }
}

// ---------------------------------------------------------------------------
// Arithmetic traits
// ---------------------------------------------------------------------------

impl Add for Fixed {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_add(rhs.0))
    }
}
impl Sub for Fixed {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Fixed(self.0.saturating_sub(rhs.0))
    }
}
impl Mul for Fixed {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let r = ((self.0 as i128) * (rhs.0 as i128)) / SCALE as i128;
        Fixed(r.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }
}
impl Div for Fixed {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            return Fixed::ZERO;
        }
        let r = ((self.0 as i128) * SCALE as i128) / rhs.0 as i128;
        Fixed(r.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }
}
impl Neg for Fixed {
    type Output = Self;
    fn neg(self) -> Self {
        Fixed(-self.0)
    }
}

impl AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}
impl SubAssign for Fixed {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}
impl MulAssign for Fixed {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
impl DivAssign for Fixed {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

// ---------------------------------------------------------------------------
// Display / Debug
// ---------------------------------------------------------------------------

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_part = self.0 / SCALE;
        let frac = (self.0 % SCALE).abs();
        write!(f, "{}.{:06}", int_part, frac)
    }
}

// ---------------------------------------------------------------------------
// Deterministic Hashing
// ---------------------------------------------------------------------------

use blake3::Hasher;

/// Deterministic Blake3 hash of a single fixed value
pub fn hash_fixed(val: Fixed) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&val.0.to_le_bytes());
    *hasher.finalize().as_bytes()
}

/// Deterministic Blake3 hash of multiple values
pub fn hash_fixed_slice(values: &[Fixed]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    for v in values {
        hasher.update(&v.0.to_le_bytes());
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
    fn test_basic_arithmetic() {
        let a = Fixed::from_f64(1.5);
        let b = Fixed::from_f64(2.25);
        let c = a * b;
        assert_eq!(c.to_micro(), 3_375_000);
        assert_eq!((a + b).to_micro(), 3_750_000);
    }

    #[test]
    fn test_ratio_and_roundtrip() {
        let half = Fixed::from_ratio(1, 2);
        assert!((half.to_f64() - 0.5).abs() < 1e-6);
        let x = Fixed::from_f64(123.456789);
        assert!((x.to_f64() - 123.456789).abs() < 1e-6);
    }

    #[test]
    fn test_hash_determinism() {
        let x = Fixed::from_f64(1.234567);
        let h1 = hash_fixed(x);
        let h2 = hash_fixed(x);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_display_and_signs() {
        assert_eq!(Fixed::from_int(5).to_string(), "5.000000");
        assert_eq!(Fixed::from_int(-5).to_string(), "-5.000000");
    }

    #[test]
    fn test_cross_platform_determinism() {
        let vals = vec![Fixed::from_int(1), Fixed::from_int(2), Fixed::from_int(3)];
        let s1 = serde_json::to_string(&vals).unwrap();
        let s2 = serde_json::to_string(&vals).unwrap();
        assert_eq!(s1, s2);
    }
}
