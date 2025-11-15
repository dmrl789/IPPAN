//! Deterministic fixed-point utilities for AI core computations.
//!
//! All arithmetic operates on 64-bit signed integers that represent values in
//! micro precision (`1 unit == 10^-6`).  The helpers in this module avoid
//! floating point usage entirely and clamp/validate intermediate results to
//! guarantee reproducibility across platforms.

use blake3::Hasher;
use core::cmp::Ordering;
use num_traits::float::Float;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Fixed-point precision: 1 unit = 1e-6 (micro precision).
pub const SCALE: i64 = 1_000_000;

const MAX_DECIMALS: u32 = 18;

#[inline]
fn pow10(decimals: u32) -> Option<i128> {
    if decimals > MAX_DECIMALS {
        return None;
    }
    let mut value: i128 = 1;
    for _ in 0..decimals {
        value = value.checked_mul(10)?;
    }
    Some(value)
}

/// Convert `x` that is encoded with `decimals` fractional digits into the
/// micro-precision representation (`SCALE` is 1e-6).
///
/// Example: `to_fixed(12345, 2)` -> `123.45` -> `123_450_000` micro units.
pub fn to_fixed(x: i64, decimals: u32) -> i64 {
    let scale_factor = pow10(decimals).expect("too many decimals for to_fixed");
    let numerator = i128::from(x)
        .checked_mul(i128::from(SCALE))
        .expect("overflow while scaling fixed-point numerator");
    let result = numerator
        .checked_div(scale_factor)
        .expect("division by zero in to_fixed");
    i64::try_from(result).expect("scaled value exceeds i64 range")
}

/// Panic on any attempt to ingest floats into deterministic paths.
pub fn reject_float_input<T: Float>(_x: T) -> ! {
    panic!("floating-point inputs are forbidden in deterministic fixed-point math");
}

/// Checked addition of two fixed-point integers.
pub fn add(a: i64, b: i64) -> i64 {
    a.checked_add(b)
        .expect("fixed-point addition overflowed i64 range")
}

/// Checked subtraction of two fixed-point integers.
pub fn sub(a: i64, b: i64) -> i64 {
    a.checked_sub(b)
        .expect("fixed-point subtraction overflowed i64 range")
}

/// Multiply two fixed-point values, automatically rescaling by `SCALE`.
fn mul_fixed_checked(a: i64, b: i64) -> Option<i64> {
    if a == 0 || b == 0 {
        return Some(0);
    }
    let product = i128::from(a).checked_mul(i128::from(b))?;
    let scaled = product.checked_div(i128::from(SCALE))?;
    if scaled < i128::from(i64::MIN) || scaled > i128::from(i64::MAX) {
        None
    } else {
        Some(scaled as i64)
    }
}

pub fn mul_fixed(a: i64, b: i64) -> i64 {
    mul_fixed_checked(a, b).expect("fixed-point multiplication overflowed i64 range")
}

/// Divide two fixed-point values, automatically rescaling by `SCALE`.
fn div_fixed_checked(a: i64, b: i64) -> Option<i64> {
    if b == 0 {
        return None;
    }
    if a == 0 {
        return Some(0);
    }
    let numerator = i128::from(a).checked_mul(i128::from(SCALE))?;
    let result = numerator.checked_div(i128::from(b))?;
    if result < i128::from(i64::MIN) || result > i128::from(i64::MAX) {
        None
    } else {
        Some(result as i64)
    }
}

pub fn div_fixed(a: i64, b: i64) -> i64 {
    div_fixed_checked(a, b).expect("fixed-point division overflowed i64 range or divided by zero")
}

/// Clamp an `i64` between the provided `[min, max]` bounds.
pub fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    assert!(min <= max, "clamp_i64 requires min <= max");
    value.max(min).min(max)
}

/// Quantise a value down to the nearest multiple of `step`.
///
/// The result is always rounded *towards negative infinity* (mathematical floor),
/// matching deterministic rounding for both positive and negative inputs.
pub fn quantize_i64(value: i64, step: i64) -> i64 {
    assert!(step > 0, "quantize_i64 requires strictly positive step");
    let remainder = value % step;
    if remainder == 0 {
        value
    } else if value >= 0 {
        value - remainder
    } else {
        value - (remainder + step)
    }
}

/// Comparator helper for branchless comparisons on fixed-point integers.
#[inline]
pub fn cmp_fixed(a: i64, b: i64) -> Ordering {
    a.cmp(&b)
}

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

impl Serialize for Fixed {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
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
                formatter.write_str("a deterministic fixed-point value encoded as integer micro-units or decimal string")
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

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if let Ok(int_micro) = value.parse::<i64>() {
                    return self.visit_i64(int_micro);
                }
                Fixed::from_decimal_str(value)
                    .ok_or_else(|| de::Error::custom("invalid fixed-point decimal string"))
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

    #[inline]
    pub const fn from_micro(micro: i64) -> Self {
        Fixed(micro)
    }

    #[inline]
    pub const fn to_micro(self) -> i64 {
        self.0
    }

    #[inline]
    pub fn from_int(value: i64) -> Self {
        Fixed(
            value
                .checked_mul(SCALE)
                .expect("fixed-point integer conversion overflowed"),
        )
    }

    /// Convert an integer that encodes `decimals` fractional digits.
    #[inline]
    pub fn from_scaled_units(value: i64, decimals: u32) -> Self {
        Fixed(to_fixed(value, decimals))
    }

    /// Parse from a decimal string (up to 6 fractional digits).
    pub fn from_decimal_str(value: &str) -> Option<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        let negative = trimmed.starts_with('-');
        let positive = if trimmed.starts_with(['-', '+']) {
            &trimmed[1..]
        } else {
            trimmed
        };

        if positive.is_empty() {
            return Some(Fixed::ZERO);
        }

        let mut parts = positive.splitn(2, '.');
        let int_part = parts.next().unwrap_or("0");
        let frac_part = parts.next().unwrap_or("");

        let mut int_value: i128 = if int_part.is_empty() {
            0
        } else {
            match int_part.parse::<i128>() {
                Ok(v) => v,
                Err(_) => return None,
            }
        };

        let (fraction_micro, carry) = parse_fraction_component(frac_part)?;
        if carry {
            int_value = int_value.checked_add(1)?;
        }

        let mut total_micro = int_value
            .checked_mul(SCALE as i128)?
            .checked_add(fraction_micro as i128)?;

        if negative {
            total_micro = -total_micro;
        }

        if total_micro > i64::MAX as i128 || total_micro < i64::MIN as i128 {
            return None;
        }

        Some(Fixed(total_micro as i64))
    }

    #[inline]
    pub const fn to_int(self) -> i64 {
        self.0 / SCALE
    }

    #[inline]
    pub const fn abs(self) -> Self {
        Fixed(self.0.abs())
    }

    #[inline]
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        mul_fixed_checked(self.0, rhs.0).map(Fixed)
    }

    #[inline]
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        div_fixed_checked(self.0, rhs.0).map(Fixed)
    }

    #[inline]
    pub fn saturating_mul(self, rhs: Self) -> Self {
        match self.checked_mul(rhs) {
            Some(result) => result,
            None => {
                if (self.0 < 0) == (rhs.0 < 0) {
                    Fixed::MAX
                } else {
                    Fixed::MIN
                }
            }
        }
    }

    #[inline]
    pub fn saturating_div(self, rhs: Self) -> Self {
        if rhs.0 == 0 {
            return if self.0 >= 0 { Fixed::MAX } else { Fixed::MIN };
        }
        match self.checked_div(rhs) {
            Some(result) => result,
            None => {
                if (self.0 < 0) == (rhs.0 < 0) {
                    Fixed::MAX
                } else {
                    Fixed::MIN
                }
            }
        }
    }

    #[inline]
    pub fn from_ratio(numerator: i64, denominator: i64) -> Self {
        if denominator == 0 {
            return Fixed::ZERO;
        }
        let ratio = (i128::from(numerator) * i128::from(SCALE)) / i128::from(denominator);
        Fixed(ratio.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
    }

    #[inline]
    pub const fn mul_int(self, rhs: i64) -> Self {
        Fixed(self.0.saturating_mul(rhs))
    }

    #[inline]
    pub const fn div_int(self, rhs: i64) -> Self {
        if rhs == 0 {
            Fixed::ZERO
        } else {
            Fixed(self.0 / rhs)
        }
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
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Fixed(clamp_i64(self.0, min.0, max.0))
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

impl Add for Fixed {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Fixed(add(self.0, rhs.0))
    }
}

impl Sub for Fixed {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Fixed(sub(self.0, rhs.0))
    }
}

impl Mul for Fixed {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Fixed(mul_fixed(self.0, rhs.0))
    }
}

impl Div for Fixed {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        if rhs.0 == 0 {
            Fixed::ZERO
        } else {
            Fixed(div_fixed(self.0, rhs.0))
        }
    }
}

impl Neg for Fixed {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Fixed(self.0.saturating_neg())
    }
}

impl AddAssign for Fixed {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = add(self.0, rhs.0);
    }
}

impl SubAssign for Fixed {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 = sub(self.0, rhs.0);
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

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int_part = self.0 / SCALE;
        let frac = (self.0 % SCALE).abs();
        write!(f, "{}.{:06}", int_part, frac)
    }
}

fn parse_fraction_component(part: &str) -> Option<(i64, bool)> {
    if part.is_empty() {
        return Some((0, false));
    }

    let mut digits = String::new();
    let mut should_round_up = false;

    for (idx, ch) in part.chars().enumerate() {
        if !ch.is_ascii_digit() {
            return None;
        }
        if idx < 6 {
            digits.push(ch);
        } else if idx == 6 {
            if ch >= '5' {
                should_round_up = true;
            }
        } else if !should_round_up && ch != '0' {
            should_round_up = true;
        }
    }

    while digits.len() < 6 {
        digits.push('0');
    }

    let mut micro = digits.parse::<i64>().ok()?;
    let mut carry = false;

    if should_round_up {
        micro += 1;
        if micro >= SCALE {
            micro -= SCALE;
            carry = true;
        }
    }

    Some((micro, carry))
}

/// Deterministic Blake3 hash of a single fixed value.
pub fn hash_fixed(val: Fixed) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(&val.0.to_le_bytes());
    *hasher.finalize().as_bytes()
}

/// Deterministic Blake3 hash of multiple values.
pub fn hash_fixed_slice(values: &[Fixed]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    for v in values {
        hasher.update(&v.0.to_le_bytes());
    }
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_conversion_matches_expected() {
        assert_eq!(to_fixed(12345, 2), 123_450_000);
        assert_eq!(to_fixed(-5, 0), -5_000_000);
        assert_eq!(to_fixed(1, 6), 1);
    }

    #[test]
    fn fixed_arithmetic_matches_helpers() {
        let a = Fixed::from_ratio(3, 2); // 1.5
        let b = Fixed::from_ratio(9, 4); // 2.25
        let product = a * b;
        assert_eq!(product.to_micro(), mul_fixed(a.to_micro(), b.to_micro()));
        let sum = a + b;
        assert_eq!(sum.to_micro(), add(a.to_micro(), b.to_micro()));
    }

    #[test]
    fn quantize_handles_negative_values() {
        assert_eq!(quantize_i64(9_999_999, 1_000), 9_999_000);
        assert_eq!(quantize_i64(-9_999_999, 1_000), -10_000_000);
    }

    #[test]
    fn cmp_fixed_orders_correctly() {
        assert_eq!(cmp_fixed(1, 2), Ordering::Less);
        assert_eq!(cmp_fixed(5, 5), Ordering::Equal);
        assert_eq!(cmp_fixed(10, -10), Ordering::Greater);
    }

    #[test]
    fn serialization_is_stable() {
        let values = vec![Fixed::from_int(1), Fixed::from_ratio(1, 10)];
        let encoded = serde_json::to_string(&values).unwrap();
        let decoded: Vec<Fixed> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(values, decoded);
    }
}
