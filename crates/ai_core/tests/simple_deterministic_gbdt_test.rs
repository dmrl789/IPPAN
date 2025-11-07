#![cfg(feature = "deterministic_math")]

//! Simple test for deterministic GBDT module
//! This test focuses only on the deterministic GBDT functionality

#![cfg(feature = "deterministic_math")]

use ippan_ai_core::deterministic_gbdt::{compute_scores, create_test_model, normalize_features};
use ippan_ai_core::Fixed;
use std::collections::HashMap;

fn fp(value: f64) -> Fixed {
    Fixed::from_f64(value)
}

#[test]
fn test_deterministic_gbdt_basic_functionality() {
    // Test model creation
    let model = create_test_model();
    assert_eq!(model.trees.len(), 1);
    assert_eq!(model.learning_rate, fp(0.1));

    // Test prediction
    let features = vec![
        Fixed::from_int(1),
        Fixed::from_int(2),
        Fixed::from_int(3),
        Fixed::from_int(4),
    ];
    let prediction = model.predict(&features);

    // Test determinism
    let prediction2 = model.predict(&features);
    assert_eq!(prediction, prediction2);
}

#[test]
fn test_ippan_time_normalization() {
    let mut telemetry = HashMap::new();
    telemetry.insert("node1".to_string(), (100_000, 1.2, 99.9, 0.42));
    telemetry.insert("node2".to_string(), (100_080, 0.9, 99.8, 0.38));

    let ippan_time_median = 100_050;
    let features = normalize_features(&telemetry, ippan_time_median);

    assert_eq!(features.len(), 2);

    let by_id: HashMap<_, _> = features
        .into_iter()
        .map(|f| (f.node_id.clone(), f))
        .collect();

    assert_eq!(by_id["node1"].delta_time_us, -50); // 100_000 - 100_050
    assert_eq!(by_id["node2"].delta_time_us, 30); // 100_080 - 100_050
}

#[test]
fn test_validator_scoring() {
    let model = create_test_model();
    let mut telemetry = HashMap::new();
    telemetry.insert("test_node".to_string(), (100_000, 1.0, 99.0, 0.5));

    let ippan_time_median = 100_000;
    let round_hash = "test_round";

    let features = normalize_features(&telemetry, ippan_time_median);
    let scores = compute_scores(&model, &features, round_hash);

    assert_eq!(scores.len(), 1);
    assert!(scores.contains_key("test_node"));
    let score_value = scores["test_node"].to_f64();
    assert!(score_value.is_finite());
}

#[test]
fn test_model_hash_consistency() {
    let model = create_test_model();
    let round_hash = "consistent_round";

    let hash1 = model.model_hash(round_hash).unwrap();
    let hash2 = model.model_hash(round_hash).unwrap();

    assert_eq!(hash1, hash2);
    assert!(!hash1.is_empty());
}

#[test]
fn test_cross_platform_determinism() {
    let model = create_test_model();
    let features = vec![fp(1.5), fp(2.5), fp(3.5), fp(4.5)];

    // Simulate multiple nodes computing the same prediction
    let node1_result = model.predict(&features);
    let node2_result = model.predict(&features);
    let node3_result = model.predict(&features);

    assert_eq!(node1_result, node2_result);
    assert_eq!(node2_result, node3_result);
}
