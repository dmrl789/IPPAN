#![cfg(feature = "deterministic_math")]

use ippan_ai_core::{Fixed, FIXED_SCALE};
use std::cmp::Ordering;

#[test]
fn mul_and_div_fixed_roundtrip() {
    let a = Fixed::from_ratio(3, 2); // 1.5
    let b = Fixed::from_ratio(5, 1); // 5.0

    let product = (a * b).to_micro();
    assert_eq!(product, Fixed::from_ratio(15, 2).to_micro());

    let quotient = Fixed::from_micro(product) / b;
    assert_eq!(quotient.to_micro(), a.to_micro());
}

#[test]
fn clamp_behaviour_matches_bounds() {
    let value = Fixed::from_int(150);
    let clamped = value.clamp(Fixed::ZERO, Fixed::from_int(100));
    assert_eq!(clamped.to_micro(), 100 * FIXED_SCALE);

    let negative = Fixed::from_int(-5);
    assert_eq!(negative.clamp(Fixed::NEG_ONE, Fixed::ONE), Fixed::NEG_ONE);
}

#[test]
fn scaled_unit_conversion() {
    assert_eq!(Fixed::from_scaled_units(123_456, 3).to_micro(), 123_456_000);
    assert_eq!(Fixed::from_scaled_units(-5, 0).to_micro(), -5 * FIXED_SCALE);
    assert_eq!(Fixed::from_scaled_units(1, 6).to_micro(), 1);
}

#[test]
fn ordering_matches_raw_values() {
    let a = Fixed::from_int(2);
    let b = Fixed::from_ratio(5, 2);
    assert_eq!(a.cmp(&b), Ordering::Less);
    assert_eq!(b.cmp(&b), Ordering::Equal);
    assert_eq!(b.cmp(&a), Ordering::Greater);
}

#[test]
fn decimal_parsing_round_trips() {
    let parsed = Fixed::from_decimal_str("12.345678").unwrap();
    assert_eq!(parsed.to_micro(), 12_345_678);
    assert!(Fixed::from_decimal_str("abc").is_none());
}

#[test]
fn saturating_subtraction_keeps_bounds() {
    let min = Fixed::MIN;
    let result = min - Fixed::from_micro(1);
    assert_eq!(result, Fixed::MIN);
}
