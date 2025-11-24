//! Deterministic helpers for fixed-point calculations inside the AI service.

use ippan_ai_core::{Fixed, FIXED_SCALE};

fn clamp_to_fixed(raw: i128) -> Fixed {
    let clamped = raw.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    Fixed::from_micro(clamped as i64)
}

/// Compute the average of the provided values.
pub fn mean(values: &[Fixed]) -> Fixed {
    if values.is_empty() {
        return Fixed::ZERO;
    }

    let sum = values.iter().fold(Fixed::ZERO, |acc, value| acc + *value);
    sum / Fixed::from_int(values.len() as i64)
}

/// Compute the variance using deterministic fixed-point arithmetic.
pub fn variance(values: &[Fixed], mean: Fixed) -> Fixed {
    if values.is_empty() {
        return Fixed::ZERO;
    }

    let mean_micro = i128::from(mean.to_micro());
    let sum = values.iter().fold(0i128, |acc, value| {
        let diff = i128::from(value.to_micro()) - mean_micro;
        let squared = (diff * diff) / i128::from(FIXED_SCALE);
        acc.saturating_add(squared)
    });

    clamp_to_fixed(sum / i128::from(values.len() as i64))
}

/// Integer square root for non-negative `i128` values using binary search.
fn integer_sqrt(value: i128) -> i128 {
    if value <= 0 {
        return 0;
    }

    let mut left = 0i128;
    let mut right = value;
    while left <= right {
        let mid = left + (right - left) / 2;
        let mid_sq = mid.saturating_mul(mid);
        if mid_sq == value {
            return mid;
        }
        if mid_sq < value {
            left = mid + 1;
        } else {
            right = mid - 1;
        }
    }
    right
}

/// Deterministically compute the square root of a fixed-point value.
pub fn sqrt_fixed(value: Fixed) -> Fixed {
    if value.is_negative() {
        return Fixed::ZERO;
    }

    let scaled = i128::from(value.to_micro()) * i128::from(FIXED_SCALE);
    let root = integer_sqrt(scaled);
    clamp_to_fixed(root)
}

/// Compute the Pearson correlation coefficient using fixed-point math.
pub fn correlation(x: &[Fixed], y: &[Fixed]) -> Fixed {
    if x.len() != y.len() || x.is_empty() {
        return Fixed::ZERO;
    }

    let n = Fixed::from_int(x.len() as i64);
    let sum_x = x.iter().copied().fold(Fixed::ZERO, |acc, v| acc + v);
    let sum_y = y.iter().copied().fold(Fixed::ZERO, |acc, v| acc + v);
    let sum_xy = x
        .iter()
        .zip(y.iter())
        .fold(Fixed::ZERO, |acc, (a, b)| acc + (*a * *b));
    let sum_x2 = x.iter().fold(Fixed::ZERO, |acc, v| acc + (*v * *v));
    let sum_y2 = y.iter().fold(Fixed::ZERO, |acc, v| acc + (*v * *v));

    let numerator = n * sum_xy - sum_x * sum_y;
    let denom_x = n * sum_x2 - sum_x * sum_x;
    let denom_y = n * sum_y2 - sum_y * sum_y;
    let denominator = sqrt_fixed(denom_x * denom_y);

    if denominator.is_zero() {
        Fixed::ZERO
    } else {
        numerator / denominator
    }
}

/// Convert a ratio of unsigned integers into a fixed-point value.
pub fn ratio_to_fixed(numerator: u64, denominator: u64) -> Fixed {
    if denominator == 0 {
        return Fixed::ZERO;
    }
    let num = i128::from(numerator) * i128::from(FIXED_SCALE);
    let denom = i128::from(denominator);
    clamp_to_fixed(num / denom)
}

/// Helper to create a fixed-point value from a decimal environment string.
pub fn parse_decimal_env(value: &str, default: Fixed) -> Fixed {
    Fixed::from_decimal_str(value.trim()).unwrap_or(default)
}
