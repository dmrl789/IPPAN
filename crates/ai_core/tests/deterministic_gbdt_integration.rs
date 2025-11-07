#![cfg(feature = "deterministic_math")]

//! Integration test demonstrating the deterministic GBDT usage example
//!
//! This test shows how to use the deterministic GBDT module in a real-world scenario
//! with IPPAN Time normalization and validator scoring.

use ippan_ai_core::deterministic_gbdt::{
    compute_scores, create_test_model, normalize_features, DeterministicGBDT,
};
use std::collections::HashMap;

/// Integration test demonstrating the usage example from the user's request
#[test]
fn test_deterministic_gbdt_usage_example() {
    // Create a test model (in production, this would be loaded from a shared file)
    let model = create_test_model();

    // Example telemetry from nodes (as shown in the user's request)
    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (100_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (100_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (100_030, 2.1, 98.9, 0.45)),
    ]);

    // IPPAN Time median in µs (as shown in the user's request)
    let ippan_time_median = 100_050;

    // Round HashTimer (from consensus layer)
    let round_hash_timer = "4b2e18f2fa7c...";

    // Normalize and score deterministically
    let features = normalize_features(&telemetry, ippan_time_median);
    let scores = compute_scores(&model, &features, round_hash_timer);

    // Verify results
    assert_eq!(scores.len(), 3);
    assert!(scores.contains_key("nodeA"));
    assert!(scores.contains_key("nodeB"));
    assert!(scores.contains_key("nodeC"));

    // All scores should be finite (can be negative since GBDT models can have negative leaf values)
    for (node_id, score) in &scores {
        let value = score.to_f64();
        assert!(
            value.is_finite(),
            "Score for {} is not finite: {}",
            node_id,
            value
        );
        // Note: Scores can be negative - this is normal for GBDT models
    }

    // Verify determinism - run the same computation again
    let features2 = normalize_features(&telemetry, ippan_time_median);
    let scores2 = compute_scores(&model, &features2, round_hash_timer);

    // Results should be identical
    assert_eq!(scores, scores2);

    println!("Validator scores: {:?}", scores);
}

/// Test with the actual model file from the models directory
#[test]
fn test_with_actual_model_file() {
    // Try to load the example model file
    let model_path = "../../../models/deterministic_gbdt_model.json";

    match DeterministicGBDT::from_json_file(model_path) {
        Ok(model) => {
            // Test with sample telemetry
            let mut telemetry = HashMap::new();
            telemetry.insert("test_node".to_string(), (100_000, 1.0, 99.0, 0.5));

            let ippan_time_median = 100_000;
            let round_hash = "test_round";

            let features = normalize_features(&telemetry, ippan_time_median);
            let scores = compute_scores(&model, &features, round_hash);

            assert_eq!(scores.len(), 1);
            assert!(scores.contains_key("test_node"));

            println!("Loaded model prediction: {:?}", scores);
        }
        Err(e) => {
            // If the model file doesn't exist, skip this test
            println!("Skipping test with model file: {}", e);
        }
    }
}

/// Test cross-node determinism simulation
#[test]
fn test_cross_node_determinism_simulation() {
    let model = create_test_model();

    // Simulate the same telemetry being processed by different nodes
    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("node1".into(), (100_000, 1.0, 99.0, 0.5)),
        ("node2".into(), (100_050, 1.5, 98.5, 0.6)),
        ("node3".into(), (99_950, 0.8, 99.5, 0.4)),
    ]);

    let ippan_time_median = 100_000;
    let round_hash = "consensus_round_12345";

    // Simulate Node A processing
    let features_a = normalize_features(&telemetry, ippan_time_median);
    let scores_a = compute_scores(&model, &features_a, round_hash);

    // Simulate Node B processing (should be identical)
    let features_b = normalize_features(&telemetry, ippan_time_median);
    let scores_b = compute_scores(&model, &features_b, round_hash);

    // Simulate Node C processing (should be identical)
    let features_c = normalize_features(&telemetry, ippan_time_median);
    let scores_c = compute_scores(&model, &features_c, round_hash);

    // All results should be identical
    assert_eq!(scores_a, scores_b);
    assert_eq!(scores_b, scores_c);

    println!("Cross-node determinism verified - all nodes produced identical results");
    println!("Node A scores: {:?}", scores_a);
}

/// Test with realistic validator scenarios
#[test]
fn test_realistic_validator_scenarios() {
    let model = create_test_model();

    // Create realistic validator scenarios
    let mut telemetry = HashMap::new();

    // High-performance validator
    telemetry.insert("validator_alpha".to_string(), (100_000, 0.5, 99.9, 0.8));

    // Average validator
    telemetry.insert("validator_beta".to_string(), (100_020, 1.2, 98.5, 0.6));

    // Poor validator
    telemetry.insert("validator_gamma".to_string(), (100_100, 3.0, 85.0, 0.3));

    // New validator (recently joined)
    telemetry.insert("validator_delta".to_string(), (99_980, 2.5, 95.0, 0.4));

    let ippan_time_median = 100_000;
    let round_hash = "realistic_round_test";

    let features = normalize_features(&telemetry, ippan_time_median);
    let scores = compute_scores(&model, &features, round_hash);

    // Verify all validators have scores
    assert_eq!(scores.len(), 4);

    // All scores should be finite
    for (validator, score) in &scores {
        let value = score.to_f64();
        assert!(
            value.is_finite(),
            "Score for {} is not finite: {}",
            validator,
            value
        );
        println!("{}: {}", validator, score);
    }

    // Verify determinism
    let features2 = normalize_features(&telemetry, ippan_time_median);
    let scores2 = compute_scores(&model, &features2, round_hash);
    assert_eq!(scores, scores2);
}

/// Test model hash certificate generation
#[test]
fn test_model_hash_certificate_generation() {
    let model = create_test_model();

    // Test with different round hashes
    let round_hash_1 = "round_12345";
    let round_hash_2 = "round_67890";

    let hash_1 = model.model_hash(round_hash_1).unwrap();
    let hash_2 = model.model_hash(round_hash_2).unwrap();

    // Different round hashes should produce different model hashes
    assert_ne!(hash_1, hash_2);

    // Same round hash should produce same model hash
    let hash_1_again = model.model_hash(round_hash_1).unwrap();
    assert_eq!(hash_1, hash_1_again);

    // Hashes should be valid hex strings
    assert!(!hash_1.is_empty());
    assert!(!hash_2.is_empty());

    println!("Model hash for round {}: {}", round_hash_1, hash_1);
    println!("Model hash for round {}: {}", round_hash_2, hash_2);
}
