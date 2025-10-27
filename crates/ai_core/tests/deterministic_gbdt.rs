//! Tests for deterministic GBDT + IPPAN Time integration
//! Verifies identical inference results across nodes with
//! different local system times but same IPPAN Time median.

use ai_core::deterministic_gbdt::{
    DeterministicGBDT, normalize_features, compute_scores, ValidatorFeatures,
};
use std::collections::HashMap;

#[test]
fn test_deterministic_inference_across_nodes() {
    // Simulate same model for all nodes (shared JSON in production)
    let model = DeterministicGBDT {
        trees: vec![ai_core::deterministic_gbdt::GBDTTree {
            nodes: vec![
                // Simple stub tree: if latency <= 1.5 → 0.8 else 0.6
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 1,
                    threshold: 1.5,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.8),
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.6),
                },
            ],
        }],
        learning_rate: 1.0,
    };

    // Simulate IPPAN Time median (microseconds)
    let ippan_time_median: i64 = 1_000_000;

    // Telemetry from nodes with slightly different local times
    let telemetry_a: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (999_950, 1.2, 99.9, 0.4)), // local clock -50µs
        ("nodeB".into(), (1_000_030, 0.9, 99.8, 0.3)), // local clock +30µs
    ]);

    let telemetry_b: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("nodeA".into(), (1_999_950, 1.2, 99.9, 0.4)), // offset by +1s
        ("nodeB".into(), (2_000_030, 0.9, 99.8, 0.3)), // offset by +1s
    ]);

    // Normalize features for both nodes using the *same* IPPAN Time
    let features_a = normalize_features(&telemetry_a, ippan_time_median);
    let features_b = normalize_features(&telemetry_b, ippan_time_median + 1_000_000); // same median offset

    // Deterministic HashTimer anchor (same for both)
    let round_hash_timer = "hashtimer_example_123";

    // Compute deterministic scores
    let scores_a = compute_scores(&model, &features_a, round_hash_timer);
    let scores_b = compute_scores(&model, &features_b, round_hash_timer);

    // NodeA and NodeB scores must be identical across both runs
    assert_eq!(scores_a.get("nodeA"), scores_b.get("nodeA"));
    assert_eq!(scores_a.get("nodeB"), scores_b.get("nodeB"));

    // Scores must be deterministic regardless of local clock offsets
    println!("Deterministic scores verified: {:?}", scores_a);
}

#[test]
fn test_model_hash_reproducibility() {
    let model = DeterministicGBDT {
        trees: vec![],
        learning_rate: 1.0,
    };
    let round_hash_timer = "round_42_ht";
    let h1 = model.model_hash(round_hash_timer);
    let h2 = model.model_hash(round_hash_timer);

    // Hash must be reproducible bit-for-bit
    assert_eq!(h1, h2);
    println!("Model hash reproducibility OK: {}", h1);
}

#[test]
fn test_fixed_point_stability() {
    let model = DeterministicGBDT {
        trees: vec![ai_core::deterministic_gbdt::GBDTTree {
            nodes: vec![
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.123456),
                },
            ],
        }],
        learning_rate: 1.0,
    };

    let features = vec![0.0, 0.0, 0.0, 0.0];
    let y1 = model.predict(&features);
    let y2 = model.predict(&features);

    // Ensure fixed-point prediction is numerically identical
    assert!((y1 - y2).abs() < 1e-12);
    println!("Fixed-point stability verified: {}", y1);
}

#[test]
fn test_ippan_time_normalization_consistency() {
    // Test that IPPAN Time normalization produces consistent results
    // regardless of local clock drift
    let ippan_time_median = 1_000_000;
    
    // Simulate different local clock drifts
    let scenarios = vec![
        ("no_drift", 1_000_000),
        ("positive_drift", 1_000_100),
        ("negative_drift", 999_900),
        ("large_positive_drift", 1_100_000),
        ("large_negative_drift", 900_000),
    ];
    
    let mut telemetry = HashMap::new();
    telemetry.insert("test_node".to_string(), (0, 1.5, 50.0, 0.5));
    
    let mut normalized_results = Vec::new();
    
    for (scenario_name, local_time) in scenarios {
        let mut scenario_telemetry = HashMap::new();
        scenario_telemetry.insert("test_node".to_string(), (local_time, 1.5, 50.0, 0.5));
        
        let normalized = normalize_features(&scenario_telemetry, ippan_time_median);
        let node_features = normalized.get("test_node").unwrap();
        
        normalized_results.push((scenario_name, node_features.normalized_latency));
    }
    
    // All scenarios should produce the same normalized latency
    // because they all have the same actual latency (1.5) and same IPPAN Time median
    for i in 1..normalized_results.len() {
        assert!((normalized_results[0].1 - normalized_results[i].1).abs() < 1e-10,
                "Scenario {} produced different normalized latency: {} vs {}",
                normalized_results[i].0, normalized_results[0].1, normalized_results[i].1);
    }
    
    println!("IPPAN Time normalization consistency verified across {} scenarios", scenarios.len());
}

#[test]
fn test_deterministic_scoring_with_clock_drift() {
    // Test that scoring remains deterministic even with significant clock drift
    let model = DeterministicGBDT {
        trees: vec![ai_core::deterministic_gbdt::GBDTTree {
            nodes: vec![
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0, // normalized_latency
                    threshold: 1.0,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.9), // High score for low latency
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.3), // Low score for high latency
                },
            ],
        }],
        learning_rate: 1.0,
    };
    
    let ippan_time_median = 1_000_000;
    let round_hash_timer = "deterministic_test_round";
    
    // Test with different clock drifts but same actual performance
    let test_cases = vec![
        ("no_drift", 1_000_000, 0.8), // Low latency
        ("positive_drift", 1_000_500, 0.8), // Same latency, +500μs drift
        ("negative_drift", 999_500, 0.8), // Same latency, -500μs drift
        ("high_latency_no_drift", 1_000_000, 1.8), // High latency
        ("high_latency_with_drift", 1_000_300, 1.8), // Same high latency, +300μs drift
    ];
    
    let mut scores = Vec::new();
    
    for (case_name, local_time, actual_latency) in test_cases {
        let mut telemetry = HashMap::new();
        telemetry.insert("test_node".to_string(), (local_time, actual_latency, 50.0, 0.5));
        
        let features = normalize_features(&telemetry, ippan_time_median);
        let node_scores = compute_scores(&model, &features, round_hash_timer);
        
        let score = node_scores.get("test_node").unwrap();
        scores.push((case_name, *score));
    }
    
    // Low latency cases should have identical scores regardless of clock drift
    assert!((scores[0].1 - scores[1].1).abs() < 1e-10, 
            "Low latency scores differ with clock drift: {} vs {}", scores[0].1, scores[1].1);
    assert!((scores[0].1 - scores[2].1).abs() < 1e-10, 
            "Low latency scores differ with clock drift: {} vs {}", scores[0].1, scores[2].1);
    
    // High latency cases should have identical scores regardless of clock drift
    assert!((scores[3].1 - scores[4].1).abs() < 1e-10, 
            "High latency scores differ with clock drift: {} vs {}", scores[3].1, scores[4].1);
    
    // Low latency should score higher than high latency
    assert!(scores[0].1 > scores[3].1, 
            "Low latency score {} should be higher than high latency score {}", scores[0].1, scores[3].1);
    
    println!("Deterministic scoring with clock drift verified:");
    for (case_name, score) in scores {
        println!("  {}: {:.6}", case_name, score);
    }
}

#[test]
fn test_model_hash_with_different_rounds() {
    // Test that model hash changes with different round hash timers
    let model = DeterministicGBDT {
        trees: vec![ai_core::deterministic_gbdt::GBDTTree {
            nodes: vec![
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 1.0,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.5),
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.7),
                },
            ],
        }],
        learning_rate: 0.8,
    };
    
    let hash1 = model.model_hash("round_1");
    let hash2 = model.model_hash("round_2");
    let hash3 = model.model_hash("round_1"); // Same as hash1
    
    // Different rounds should produce different hashes
    assert_ne!(hash1, hash2);
    
    // Same round should produce identical hashes
    assert_eq!(hash1, hash3);
    
    println!("Model hash round dependency verified:");
    println!("  Round 1: {}", hash1);
    println!("  Round 2: {}", hash2);
    println!("  Round 1 (repeat): {}", hash3);
}

#[test]
fn test_edge_cases_and_error_handling() {
    // Test edge cases and error handling
    let model = DeterministicGBDT {
        trees: vec![ai_core::deterministic_gbdt::GBDTTree {
            nodes: vec![
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(1.0),
                },
            ],
        }],
        learning_rate: 1.0,
    };
    
    // Test with empty feature vector
    let empty_features = vec![];
    let prediction_empty = model.predict(&empty_features);
    assert_eq!(prediction_empty, 0.0);
    
    // Test with single feature
    let single_feature = vec![0.5];
    let prediction_single = model.predict(&single_feature);
    assert_eq!(prediction_single, 1.0);
    
    // Test with many features (should ignore extra ones)
    let many_features = vec![0.5, 1.0, 2.0, 3.0, 4.0];
    let prediction_many = model.predict(&many_features);
    assert_eq!(prediction_many, 1.0);
    
    // Test with extreme values
    let extreme_features = vec![f64::INFINITY, f64::NEG_INFINITY, f64::NAN];
    let prediction_extreme = model.predict(&extreme_features);
    // Should handle extreme values gracefully
    assert!(prediction_extreme.is_finite());
    
    println!("Edge cases and error handling verified");
}

#[test]
fn test_performance_consistency() {
    // Test that performance remains consistent across multiple evaluations
    let model = DeterministicGBDT {
        trees: vec![ai_core::deterministic_gbdt::GBDTTree {
            nodes: vec![
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.5,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.3),
                },
                ai_core::deterministic_gbdt::DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.7),
                },
            ],
        }],
        learning_rate: 1.0,
    };
    
    let features = vec![0.6, 0.4, 0.8, 0.2];
    
    // Run multiple evaluations and ensure consistency
    let mut predictions = Vec::new();
    for _ in 0..100 {
        predictions.push(model.predict(&features));
    }
    
    // All predictions should be identical
    let first_prediction = predictions[0];
    for (i, prediction) in predictions.iter().enumerate() {
        assert!((prediction - first_prediction).abs() < 1e-15,
                "Prediction {} differs from first: {} vs {}", i, prediction, first_prediction);
    }
    
    println!("Performance consistency verified: {} identical predictions", predictions.len());
}