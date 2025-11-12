#![cfg(feature = "deterministic_math")]

use ippan_ai_core::{
    clamp_i64, cmp_fixed, div_fixed, from_f64_lossy, mul_fixed, quantize_i64, sub, to_fixed, Fixed,
    FIXED_SCALE,
};
use std::cmp::Ordering;

#[test]
fn mul_and_div_fixed_roundtrip() {
    let a = Fixed::from_ratio(3, 2); // 1.5
    let b = Fixed::from_ratio(5, 1); // 5.0

    let product = mul_fixed(a.to_micro(), b.to_micro());
    assert_eq!(product, Fixed::from_ratio(15, 2).to_micro());

    let quotient = div_fixed(product, b.to_micro());
    assert_eq!(quotient, a.to_micro());
}

#[test]
fn quantize_and_clamp_behaviour() {
    let value = 9_876_543;
    assert_eq!(quantize_i64(value, 1_000), 9_876_000);
    assert_eq!(quantize_i64(-value, 1_000), -9_877_000);

    let clamped = clamp_i64(150 * FIXED_SCALE, 0, 100 * FIXED_SCALE);
    assert_eq!(clamped, 100 * FIXED_SCALE);
}

#[test]
fn to_fixed_handles_various_decimals() {
    assert_eq!(to_fixed(123_456, 3), 123_456_000); // 123.456 -> micro
    assert_eq!(to_fixed(-5, 0), -5 * FIXED_SCALE);
    assert_eq!(to_fixed(1, 6), 1);
}

#[test]
fn cmp_fixed_orders_values() {
    let a = Fixed::from_int(2).to_micro();
    let b = Fixed::from_ratio(5, 2).to_micro();
    assert_eq!(cmp_fixed(a, b), Ordering::Less);
    assert_eq!(cmp_fixed(b, b), Ordering::Equal);
    assert_eq!(cmp_fixed(b, a), Ordering::Greater);
}

#[test]
#[should_panic(expected = "floating-point inputs are forbidden")]
fn from_f64_lossy_panics() {
    let _ = from_f64_lossy(1.23);
}

#[test]
fn checked_sub_overflow_panics() {
    let min = Fixed::MIN.to_micro();
    let result = std::panic::catch_unwind(|| sub(min, 1));
    assert!(result.is_err(), "expected subtraction overflow to panic");
}
