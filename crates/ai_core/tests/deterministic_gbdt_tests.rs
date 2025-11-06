//! Comprehensive unit tests for deterministic GBDT module
//!
//! Tests cover:
//! - Model loading and validation
//! - Deterministic prediction consistency
//! - IPPAN Time normalization
//! - Hash certificate generation
//! - Cross-platform determinism

use ippan_ai_core::deterministic_gbdt::{
    compute_scores, create_test_model, normalize_features, DecisionNode, DeterministicGBDT,
    DeterministicGBDTError, GBDTTree, ValidatorFeatures,
};
use ippan_ai_core::FixedPoint;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

fn fp(value: f64) -> FixedPoint {
    FixedPoint::from_f64(value)
}

/// Test model loading from JSON file
#[test]
fn test_model_loading_from_json() {
    let temp_dir = TempDir::new().unwrap();
    let model_path = temp_dir.path().join("test_model.json");

    // Create a test model
    let model = create_test_model();
    model.save_json(&model_path).unwrap();

    // Load the model back
    let loaded_model = DeterministicGBDT::from_json_file(&model_path).unwrap();

    assert_eq!(model.trees.len(), loaded_model.trees.len());
    assert_eq!(model.learning_rate, loaded_model.learning_rate);
}

/// Test model loading from binary file
#[test]
fn test_model_loading_from_binary() {
    let temp_dir = TempDir::new().unwrap();
    let model_path = temp_dir.path().join("test_model.bin");

    // Create a test model
    let model = create_test_model();
    model.save_binary(&model_path).unwrap();

    // Load the model back
    let loaded_model = DeterministicGBDT::from_binary_file(&model_path).unwrap();

    assert_eq!(model.trees.len(), loaded_model.trees.len());
    assert_eq!(model.learning_rate, loaded_model.learning_rate);
}

/// Test deterministic prediction consistency
#[test]
fn test_deterministic_prediction_consistency() {
    let model = create_test_model();
    let features = vec![
        FixedPoint::from_integer(1),
        FixedPoint::from_integer(2),
        FixedPoint::from_integer(3),
        FixedPoint::from_integer(4),
    ];

    // Run prediction multiple times
    let predictions: Vec<FixedPoint> = (0..100).map(|_| model.predict(&features)).collect();

    // All predictions should be identical
    for i in 1..predictions.len() {
        assert_eq!(
            predictions[0], predictions[i],
            "Prediction {} differs from first",
            i
        );
    }
}

/// Test IPPAN Time normalization
#[test]
fn test_ippan_time_normalization() {
    let mut telemetry = HashMap::new();
    telemetry.insert("node1".to_string(), (100_000, 1.2, 99.9, 0.42));
    telemetry.insert("node2".to_string(), (100_080, 0.9, 99.8, 0.38));
    telemetry.insert("node3".to_string(), (99_950, 2.1, 98.9, 0.45));

    let ippan_time_median = 100_050;
    let features = normalize_features(&telemetry, ippan_time_median);

    assert_eq!(features.len(), 3);

    let by_id: HashMap<_, _> = features
        .into_iter()
        .map(|f| (f.node_id.clone(), f))
        .collect();

    // Check delta time calculations
    assert_eq!(by_id["node1"].delta_time_us, -50); // 100_000 - 100_050
    assert_eq!(by_id["node2"].delta_time_us, 30); // 100_080 - 100_050
    assert_eq!(by_id["node3"].delta_time_us, -100); // 99_950 - 100_050

    // Check other features are preserved
    assert_eq!(by_id["node1"].latency_ms, fp(1.2));
    assert_eq!(by_id["node1"].uptime_pct, fp(99.9));
    assert_eq!(by_id["node1"].peer_entropy, fp(0.42));
}

/// Normalization should depend on relative (not absolute) IPPAN time
#[test]
fn test_normalize_features_clock_offset_invariance() {
    let telemetry_a: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (100_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (100_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (100_030, 2.1, 98.9, 0.45)),
    ]);
    let telemetry_b: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (105_000, 1.2, 99.9, 0.42)),
        ("nodeB".into(), (105_080, 0.9, 99.8, 0.38)),
        ("nodeC".into(), (105_030, 2.1, 98.9, 0.45)),
    ]);

    let features_a = normalize_features(&telemetry_a, 100_050);
    let features_b = normalize_features(&telemetry_b, 105_050);

    let map = |features: Vec<ValidatorFeatures>| -> HashMap<String, (i64, FixedPoint, FixedPoint, FixedPoint)> {
        features
            .into_iter()
            .map(|f| {
                (
                    f.node_id,
                    (f.delta_time_us, f.latency_ms, f.uptime_pct, f.peer_entropy),
                )
            })
            .collect()
    };

    assert_eq!(map(features_a), map(features_b));
}

/// Test validator scoring with different scenarios
#[test]
fn test_validator_scoring_scenarios() {
    let model = create_test_model();

    // Test case 1: Good validator (low latency, high uptime, good entropy)
    let mut telemetry_good = HashMap::new();
    telemetry_good.insert("good_node".to_string(), (100_000, 0.5, 99.9, 0.8));

    // Test case 2: Poor validator (high latency, low uptime, poor entropy)
    let mut telemetry_poor = HashMap::new();
    telemetry_poor.insert("poor_node".to_string(), (100_000, 5.0, 85.0, 0.2));

    let ippan_time_median = 100_000;
    let round_hash = "test_round_123";

    let features_good = normalize_features(&telemetry_good, ippan_time_median);
    let features_poor = normalize_features(&telemetry_poor, ippan_time_median);

    let scores_good = compute_scores(&model, &features_good, round_hash).unwrap();
    let scores_poor = compute_scores(&model, &features_poor, round_hash).unwrap();

    assert_eq!(scores_good.len(), 1);
    assert_eq!(scores_poor.len(), 1);

    // Both should have valid scores
    assert!(scores_good.contains_key("good_node"));
    assert!(scores_poor.contains_key("poor_node"));
}

/// Test model hash consistency
#[test]
fn test_model_hash_consistency() {
    let model = create_test_model();
    let round_hash = "consistent_round_hash";

    // Generate hash multiple times
    let hashes: Vec<String> = (0..10)
        .map(|_| model.model_hash(round_hash).unwrap())
        .collect();

    // All hashes should be identical
    for i in 1..hashes.len() {
        assert_eq!(hashes[0], hashes[i], "Hash {} differs from first", i);
    }

    // Different round hashes should produce different model hashes
    let different_hash = model.model_hash("different_round_hash").unwrap();
    assert_ne!(hashes[0], different_hash);
}

/// Test model validation with invalid structures
#[test]
fn test_model_validation_invalid_structures() {
    // Test empty trees
    let empty_model = DeterministicGBDT {
        trees: vec![],
        learning_rate: fp(0.1),
    };
    assert!(empty_model.validate().is_err());

    // Test tree with invalid node references
    let invalid_tree = GBDTTree {
        nodes: vec![
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: Some(5), // Invalid reference
                right: Some(2),
                value: None,
            },
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: None,
                right: None,
                value: Some(fp(0.1)),
            },
        ],
    };

    let invalid_model = DeterministicGBDT {
        trees: vec![invalid_tree],
        learning_rate: fp(0.1),
    };
    assert!(invalid_model.validate().is_err());
}

/// Test feature vector size validation
#[test]
fn test_feature_vector_size_validation() {
    let model = create_test_model();

    // Test with correct feature size
    let correct_features = vec![
        FixedPoint::from_integer(1),
        FixedPoint::from_integer(2),
        FixedPoint::from_integer(3),
        FixedPoint::from_integer(4),
    ];
    let result = model.predict(&correct_features);
    let repeat = model.predict(&correct_features);
    assert_eq!(result, repeat);

    // Test with incorrect feature size (should not panic, but may warn)
    let incorrect_features = vec![FixedPoint::from_integer(1), FixedPoint::from_integer(2)];
    let result = model.predict(&incorrect_features);
    let repeat = model.predict(&incorrect_features);
    assert_eq!(result, repeat);
}

/// Test cross-platform determinism simulation
#[test]
fn test_cross_platform_determinism_simulation() {
    let model = create_test_model();
    let features = vec![fp(1.5), fp(2.5), fp(3.5), fp(4.5)];
    let round_hash = "deterministic_round";

    // Simulate different "nodes" computing the same prediction
    let node1_result = model.predict(&features);
    let node2_result = model.predict(&features);
    let node3_result = model.predict(&features);

    // All should be identical
    assert_eq!(node1_result, node2_result);
    assert_eq!(node2_result, node3_result);

    // Hash should also be identical
    let hash1 = model.model_hash(round_hash).unwrap();
    let hash2 = model.model_hash(round_hash).unwrap();
    assert_eq!(hash1, hash2);
}

/// Test edge cases in prediction
#[test]
fn test_prediction_edge_cases() {
    let model = create_test_model();

    // Test with zero features
    let zero_features = vec![FixedPoint::zero(); 4];
    let result = model.predict(&zero_features);
    let repeat = model.predict(&zero_features);
    assert_eq!(result, repeat);

    // Test with negative features
    let negative_features = vec![fp(-1.0), fp(-2.0), fp(-3.0), fp(-4.0)];
    let result = model.predict(&negative_features);
    let repeat = model.predict(&negative_features);
    assert_eq!(result, repeat);

    // Test with very large features
    let large_features = vec![
        FixedPoint::from_integer(1_000_000),
        FixedPoint::from_integer(2_000_000),
        FixedPoint::from_integer(3_000_000),
        FixedPoint::from_integer(4_000_000),
    ];
    let result = model.predict(&large_features);
    let repeat = model.predict(&large_features);
    assert_eq!(result, repeat);
}

/// Test model serialization round-trip
#[test]
fn test_model_serialization_round_trip() {
    let original_model = create_test_model();

    // Test JSON round-trip
    let temp_dir = TempDir::new().unwrap();
    let json_path = temp_dir.path().join("model.json");

    original_model.save_json(&json_path).unwrap();
    let loaded_model = DeterministicGBDT::from_json_file(&json_path).unwrap();

    // Test that predictions are identical
    let features = vec![
        FixedPoint::from_integer(1),
        FixedPoint::from_integer(2),
        FixedPoint::from_integer(3),
        FixedPoint::from_integer(4),
    ];
    let original_prediction = original_model.predict(&features);
    let loaded_prediction = loaded_model.predict(&features);

    assert_eq!(original_prediction, loaded_prediction);

    // Test binary round-trip
    let bin_path = temp_dir.path().join("model.bin");

    original_model.save_binary(&bin_path).unwrap();
    let loaded_bin_model = DeterministicGBDT::from_binary_file(&bin_path).unwrap();

    let bin_prediction = loaded_bin_model.predict(&features);
    assert_eq!(original_prediction, bin_prediction);
}

/// Test complex multi-tree model
#[test]
fn test_complex_multi_tree_model() {
    let tree1 = GBDTTree {
        nodes: vec![
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: Some(1),
                right: Some(2),
                value: None,
            },
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: None,
                right: None,
                value: Some(fp(0.1)),
            },
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: None,
                right: None,
                value: Some(fp(0.2)),
            },
        ],
    };

    let tree2 = GBDTTree {
        nodes: vec![
            DecisionNode {
                feature: 1,
                threshold: FixedPoint::from_integer(1),
                left: Some(1),
                right: Some(2),
                value: None,
            },
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: None,
                right: None,
                value: Some(fp(0.05)),
            },
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: None,
                right: None,
                value: Some(fp(0.15)),
            },
        ],
    };

    let model = DeterministicGBDT {
        trees: vec![tree1, tree2],
        learning_rate: fp(0.1),
    };

    let features = vec![fp(0.5), fp(1.5), fp(2.5), fp(3.5)];
    let prediction = model.predict(&features);

    assert!(prediction.raw() > 0);
}

/// Test error handling for file operations
#[test]
fn test_file_operation_errors() {
    // Test loading from non-existent file
    let result = DeterministicGBDT::from_json_file("/non/existent/path.json");
    assert!(matches!(
        result,
        Err(DeterministicGBDTError::ModelLoadError(_))
    ));

    // Test loading from invalid JSON
    let temp_dir = TempDir::new().unwrap();
    let invalid_json_path = temp_dir.path().join("invalid.json");
    fs::write(&invalid_json_path, "invalid json content").unwrap();

    let result = DeterministicGBDT::from_json_file(&invalid_json_path);
    assert!(matches!(
        result,
        Err(DeterministicGBDTError::ModelLoadError(_))
    ));
}

/// Test performance with large feature sets
#[test]
fn test_performance_large_feature_sets() {
    let model = create_test_model();

    // Test with many validators
    let mut telemetry = HashMap::new();
    for i in 0..1000 {
        telemetry.insert(
            format!("node_{}", i),
            (100_000 + i as i64, 1.0 + (i as f64) * 0.001, 99.0, 0.5),
        );
    }

    let ippan_time_median = 100_500;
    let features = normalize_features(&telemetry, ippan_time_median);
    let round_hash = "performance_test_round";

    let start = std::time::Instant::now();
    let scores = compute_scores(&model, &features, round_hash).unwrap();
    let duration = start.elapsed();

    assert_eq!(scores.len(), 1000);
    assert!(duration.as_millis() < 1000); // Should complete within 1 second
}

/// Test deterministic behavior with floating point precision
#[test]
fn test_floating_point_precision_consistency() {
    let model = create_test_model();

    // Test with values that might cause floating point precision issues
    let features1 = vec![fp(0.1), fp(0.2), fp(0.3), fp(0.4)];
    let features2 = vec![fp(0.1), fp(0.2), fp(0.3), fp(0.4)];

    let prediction1 = model.predict(&features1);
    let prediction2 = model.predict(&features2);

    // Should be identical due to fixed-point arithmetic
    assert_eq!(prediction1, prediction2);
}
