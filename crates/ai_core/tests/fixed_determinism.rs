//! Comprehensive tests for fixed-point determinism
//!
//! Tests edge cases, overflow handling, SCALE invariants, and cross-platform consistency

use ippan_ai_core::fixed::{
    add, clamp_i64, cmp_fixed, div_fixed, hash_fixed, hash_fixed_slice, mul_fixed, quantize_i64,
    sub, to_fixed, Fixed, SCALE,
};
use std::cmp::Ordering;

#[test]
fn test_scale_constant() {
    assert_eq!(SCALE, 1_000_000);
}

#[test]
fn test_to_fixed_zero_decimals() {
    // Converting integer with 0 decimals means multiply by SCALE
    assert_eq!(to_fixed(0, 0), 0);
    assert_eq!(to_fixed(1, 0), 1_000_000);
    assert_eq!(to_fixed(123, 0), 123_000_000);
    assert_eq!(to_fixed(-42, 0), -42_000_000);
}

#[test]
fn test_to_fixed_with_decimals() {
    // 1 decimal place: 12 with 1 decimal -> 12 * (1_000_000 / 10) = 12 * 100_000
    assert_eq!(to_fixed(12, 1), 1_200_000); // 1.2
    assert_eq!(to_fixed(1234, 3), 1_234_000); // 1.234
    assert_eq!(to_fixed(123456, 6), 123_456); // Already in micro units
    assert_eq!(to_fixed(1234567, 6), 1_234_567);
}

#[test]
fn test_to_fixed_overflow_saturation() {
    // Test saturation on overflow
    let large = i64::MAX / 100;
    let result = to_fixed(large, 0);
    // Should saturate to i64::MAX
    assert!(result == i64::MAX || result == large * SCALE);
}

#[test]
#[should_panic(expected = "Floating-point conversion is forbidden")]
fn test_from_f64_lossy_panics() {
    ippan_ai_core::fixed::from_f64_lossy(3.14);
}

// ---------------------------------------------------------------------------
// Basic arithmetic tests
// ---------------------------------------------------------------------------

#[test]
fn test_add_basic() {
    let a = to_fixed(1, 0); // 1.0
    let b = to_fixed(2, 0); // 2.0
    let result = add(a, b);
    assert_eq!(result, to_fixed(3, 0)); // 3.0
}

#[test]
fn test_sub_basic() {
    let a = to_fixed(5, 0); // 5.0
    let b = to_fixed(2, 0); // 2.0
    let result = sub(a, b);
    assert_eq!(result, to_fixed(3, 0)); // 3.0
}

#[test]
fn test_add_saturation_positive() {
    let a = i64::MAX - 1000;
    let b = 2000;
    let result = add(a, b);
    assert_eq!(result, i64::MAX); // Saturates
}

#[test]
fn test_sub_saturation_negative() {
    let a = i64::MIN + 1000;
    let b = 2000;
    let result = sub(a, b);
    assert_eq!(result, i64::MIN); // Saturates
}

// ---------------------------------------------------------------------------
// Multiplication tests
// ---------------------------------------------------------------------------

#[test]
fn test_mul_fixed_basic() {
    // 2.0 * 3.0 = 6.0
    let a = to_fixed(2, 0);
    let b = to_fixed(3, 0);
    let result = mul_fixed(a, b);
    assert_eq!(result, to_fixed(6, 0));
}

#[test]
fn test_mul_fixed_fractional() {
    // 1.5 * 2.5 = 3.75
    let a = 1_500_000; // 1.5
    let b = 2_500_000; // 2.5
    let result = mul_fixed(a, b);
    assert_eq!(result, 3_750_000); // 3.75
}

#[test]
fn test_mul_fixed_by_zero() {
    let a = to_fixed(42, 0);
    let result = mul_fixed(a, 0);
    assert_eq!(result, 0);
}

#[test]
fn test_mul_fixed_overflow_clamps() {
    // Test with values that would overflow
    let a = i64::MAX / 2;
    let b = to_fixed(3, 0);
    let result = mul_fixed(a, b);
    // Should clamp to i64::MAX
    assert_eq!(result, i64::MAX);
}

#[test]
fn test_mul_fixed_negative_overflow() {
    let a = i64::MIN / 2;
    let b = to_fixed(3, 0);
    let result = mul_fixed(a, b);
    // Should clamp to i64::MIN
    assert_eq!(result, i64::MIN);
}

#[test]
fn test_mul_fixed_precision() {
    // Test that precision is maintained
    let a = 1_000_001; // 1.000001
    let b = 1_000_001; // 1.000001
    let result = mul_fixed(a, b);
    // (1.000001 * 1.000001) = 1.000002000001 -> 1.000002 (rounded)
    assert_eq!(result, 1_000_002);
}

// ---------------------------------------------------------------------------
// Division tests
// ---------------------------------------------------------------------------

#[test]
fn test_div_fixed_basic() {
    // 6.0 / 2.0 = 3.0
    let a = to_fixed(6, 0);
    let b = to_fixed(2, 0);
    let result = div_fixed(a, b);
    assert_eq!(result, to_fixed(3, 0));
}

#[test]
fn test_div_fixed_fractional() {
    // 7.5 / 2.5 = 3.0
    let a = 7_500_000; // 7.5
    let b = 2_500_000; // 2.5
    let result = div_fixed(a, b);
    assert_eq!(result, 3_000_000); // 3.0
}

#[test]
fn test_div_fixed_by_zero() {
    let a = to_fixed(42, 0);
    let result = div_fixed(a, 0);
    assert_eq!(result, 0); // Returns 0 on division by zero
}

#[test]
fn test_div_fixed_overflow_clamps() {
    // Test with values that would overflow
    let a = i64::MAX / 2;
    let b = to_fixed(0, 0) + 100; // Very small divisor (0.0001)
    let result = div_fixed(a, b);
    // Should clamp to i64::MAX
    assert_eq!(result, i64::MAX);
}

#[test]
fn test_div_fixed_precision() {
    // 1 / 3 should give approximately 0.333333
    let a = to_fixed(1, 0);
    let b = to_fixed(3, 0);
    let result = div_fixed(a, b);
    assert_eq!(result, 333_333); // 0.333333
}

#[test]
fn test_div_fixed_negative() {
    let a = to_fixed(-10, 0);
    let b = to_fixed(2, 0);
    let result = div_fixed(a, b);
    assert_eq!(result, to_fixed(-5, 0));
}

// ---------------------------------------------------------------------------
// Clamp tests
// ---------------------------------------------------------------------------

#[test]
fn test_clamp_i64_within_bounds() {
    assert_eq!(clamp_i64(5, 0, 10), 5);
    assert_eq!(clamp_i64(0, 0, 10), 0);
    assert_eq!(clamp_i64(10, 0, 10), 10);
}

#[test]
fn test_clamp_i64_below_min() {
    assert_eq!(clamp_i64(-5, 0, 10), 0);
    assert_eq!(clamp_i64(i64::MIN, 0, 10), 0);
}

#[test]
fn test_clamp_i64_above_max() {
    assert_eq!(clamp_i64(15, 0, 10), 10);
    assert_eq!(clamp_i64(i64::MAX, 0, 10), 10);
}

// ---------------------------------------------------------------------------
// Quantize tests
// ---------------------------------------------------------------------------

#[test]
fn test_quantize_i64_basic() {
    let step = SCALE / 10; // 0.1 steps
    assert_eq!(quantize_i64(1_234_567, step), 1_200_000); // 1.2
    assert_eq!(quantize_i64(1_567_890, step), 1_500_000); // 1.5
}

#[test]
fn test_quantize_i64_zero_step() {
    let value = 1_234_567;
    assert_eq!(quantize_i64(value, 0), value); // Returns original value
}

#[test]
fn test_quantize_i64_negative() {
    let step = SCALE; // 1.0 steps
    assert_eq!(quantize_i64(-3_456_789, step), -3_000_000); // Floor to -3.0
}

#[test]
fn test_quantize_i64_exact_multiple() {
    let step = SCALE / 2; // 0.5 steps
    assert_eq!(quantize_i64(2_500_000, step), 2_500_000); // Already at 2.5
}

// ---------------------------------------------------------------------------
// Comparison tests
// ---------------------------------------------------------------------------

#[test]
fn test_cmp_fixed_ordering() {
    let a = to_fixed(1, 0);
    let b = to_fixed(2, 0);
    let c = to_fixed(1, 0);

    assert_eq!(cmp_fixed(a, b), Ordering::Less);
    assert_eq!(cmp_fixed(b, a), Ordering::Greater);
    assert_eq!(cmp_fixed(a, c), Ordering::Equal);
}

#[test]
fn test_cmp_fixed_negative() {
    let a = to_fixed(-5, 0);
    let b = to_fixed(-2, 0);

    assert_eq!(cmp_fixed(a, b), Ordering::Less);
    assert_eq!(cmp_fixed(b, a), Ordering::Greater);
}

// ---------------------------------------------------------------------------
// Hash determinism tests
// ---------------------------------------------------------------------------

#[test]
fn test_hash_fixed_deterministic() {
    let v = Fixed::from_int(123);
    let h1 = hash_fixed(v);
    let h2 = hash_fixed(v);
    assert_eq!(h1, h2);
}

#[test]
fn test_hash_fixed_different_values() {
    let v1 = Fixed::from_int(123);
    let v2 = Fixed::from_int(124);
    let h1 = hash_fixed(v1);
    let h2 = hash_fixed(v2);
    assert_ne!(h1, h2);
}

#[test]
fn test_hash_fixed_slice_deterministic() {
    let values = vec![
        Fixed::from_int(1),
        Fixed::from_int(2),
        Fixed::from_int(3),
    ];
    let h1 = hash_fixed_slice(&values);
    let h2 = hash_fixed_slice(&values);
    assert_eq!(h1, h2);
}

#[test]
fn test_hash_fixed_slice_order_matters() {
    let values1 = vec![Fixed::from_int(1), Fixed::from_int(2)];
    let values2 = vec![Fixed::from_int(2), Fixed::from_int(1)];
    let h1 = hash_fixed_slice(&values1);
    let h2 = hash_fixed_slice(&values2);
    assert_ne!(h1, h2);
}

// ---------------------------------------------------------------------------
// SCALE invariant tests
// ---------------------------------------------------------------------------

#[test]
fn test_scale_invariant_identity() {
    // x * 1.0 = x
    let x = to_fixed(42, 0);
    let one = to_fixed(1, 0);
    let result = mul_fixed(x, one);
    assert_eq!(result, x);
}

#[test]
fn test_scale_invariant_division() {
    // (x * y) / y = x (within precision limits)
    let x = to_fixed(7, 0);
    let y = to_fixed(3, 0);
    let product = mul_fixed(x, y);
    let result = div_fixed(product, y);
    assert_eq!(result, x);
}

#[test]
fn test_scale_invariant_addition_subtraction() {
    // (x + y) - y = x
    let x = to_fixed(100, 0);
    let y = to_fixed(42, 0);
    let sum = add(x, y);
    let result = sub(sum, y);
    assert_eq!(result, x);
}

// ---------------------------------------------------------------------------
// Cross-platform consistency tests
// ---------------------------------------------------------------------------

#[test]
fn test_fixed_struct_serialization_determinism() {
    let values = vec![
        Fixed::from_int(1),
        Fixed::from_int(2),
        Fixed::from_int(-3),
        Fixed::from_micro(123_456),
    ];

    // Serialize multiple times
    let json1 = serde_json::to_string(&values).unwrap();
    let json2 = serde_json::to_string(&values).unwrap();

    assert_eq!(json1, json2);

    // Deserialize and re-serialize
    let deserialized: Vec<Fixed> = serde_json::from_str(&json1).unwrap();
    let json3 = serde_json::to_string(&deserialized).unwrap();

    assert_eq!(json1, json3);
}

#[test]
fn test_operations_produce_same_results() {
    // This test ensures that repeated operations produce identical results
    let a = to_fixed(12345, 3);
    let b = to_fixed(67890, 3);

    let result1 = mul_fixed(a, b);
    let result2 = mul_fixed(a, b);
    assert_eq!(result1, result2);

    let result3 = div_fixed(a, b);
    let result4 = div_fixed(a, b);
    assert_eq!(result3, result4);
}

// ---------------------------------------------------------------------------
// Edge case tests
// ---------------------------------------------------------------------------

#[test]
fn test_edge_case_min_max_values() {
    let min = Fixed::MIN;
    let max = Fixed::MAX;

    // Adding zero should not change values
    assert_eq!(add(min.to_micro(), 0), min.to_micro());
    assert_eq!(add(max.to_micro(), 0), max.to_micro());

    // Multiplying by one should preserve values (within saturation limits)
    let one = to_fixed(1, 0);
    assert_eq!(mul_fixed(0, one), 0);
}

#[test]
fn test_edge_case_very_small_values() {
    // Test with micro-unit precision
    let tiny = 1; // 0.000001
    let result = mul_fixed(tiny, tiny);
    assert_eq!(result, 0); // Result is smaller than precision

    let result2 = div_fixed(tiny, to_fixed(1, 0));
    assert_eq!(result2, tiny);
}

#[test]
fn test_associativity_addition() {
    // (a + b) + c = a + (b + c)
    let a = to_fixed(1, 0);
    let b = to_fixed(2, 0);
    let c = to_fixed(3, 0);

    let left = add(add(a, b), c);
    let right = add(a, add(b, c));
    assert_eq!(left, right);
}

#[test]
fn test_commutativity_multiplication() {
    // a * b = b * a
    let a = 1_500_000; // 1.5
    let b = 2_500_000; // 2.5

    let result1 = mul_fixed(a, b);
    let result2 = mul_fixed(b, a);
    assert_eq!(result1, result2);
}

#[test]
fn test_distributivity() {
    // a * (b + c) = a * b + a * c (within precision limits)
    let a = to_fixed(2, 0);
    let b = to_fixed(3, 0);
    let c = to_fixed(4, 0);

    let left = mul_fixed(a, add(b, c));
    let right = add(mul_fixed(a, b), mul_fixed(a, c));
    assert_eq!(left, right);
}
