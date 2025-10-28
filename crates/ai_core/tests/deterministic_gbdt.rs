//! Tests for deterministic GBDT + IPPAN Time integration
//! Verifies identical inference results across nodes with
//! different local system times but same IPPAN Time median.

use ippan_ai_core::deterministic_gbdt::{
    DeterministicGBDT, normalize_features, compute_scores, ValidatorFeatures,
    GBDTTree, DecisionNode,
};
use std::collections::HashMap;

#[test]
fn test_deterministic_inference_across_nodes() {
    // Simulate same model for all nodes (shared JSON in production)
    let model = DeterministicGBDT {
        trees: vec![GBDTTree {
            nodes: vec![
                // Simple stub tree: if latency <= 1.5 → 0.8 else 0.6
                DecisionNode {
                    feature: 1,
                    threshold: 1.5,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.8),
                },
                DecisionNode {
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
    let scores_a = compute_scores(&model, &features_a, round_hash_timer).unwrap();
    let scores_b = compute_scores(&model, &features_b, round_hash_timer).unwrap();

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
    let h1 = model.model_hash(round_hash_timer).unwrap();
    let h2 = model.model_hash(round_hash_timer).unwrap();

    // Hash must be reproducible bit-for-bit
    assert_eq!(h1, h2);
    println!("Model hash reproducibility OK: {}", h1);
}

#[test]
fn test_fixed_point_stability() {
    let model = DeterministicGBDT {
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
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
fn test_ippan_time_normalization() {
    // Test that IPPAN Time-based normalization produces consistent results
    let ippan_time = 5_000_000; // 5 seconds in microseconds
    
    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("validator1".into(), (5_000_100, 1.0, 99.5, 0.9)),
        ("validator2".into(), (4_999_900, 2.5, 98.0, 0.7)),
    ]);
    
    let features = normalize_features(&telemetry, ippan_time);
    
    // Both validators should have features
    assert!(features.contains_key("validator1"));
    assert!(features.contains_key("validator2"));
    
    // Verify feature vectors have correct length
    assert_eq!(features.get("validator1").unwrap().len(), 4);
    assert_eq!(features.get("validator2").unwrap().len(), 4);
    
    println!("IPPAN Time normalization verified: {:?}", features);
}

#[test]
fn test_deterministic_scoring_with_complex_model() {
    // More realistic GBDT model with multiple trees
    let model = DeterministicGBDT {
        trees: vec![
            GBDTTree {
                nodes: vec![
                    DecisionNode {
                        feature: 1,
                        threshold: 2.0,
                        left: Some(1),
                        right: Some(2),
                        value: None,
                    },
                    DecisionNode {
                        feature: 0,
                        threshold: 0.0,
                        left: None,
                        right: None,
                        value: Some(0.5),
                    },
                    DecisionNode {
                        feature: 0,
                        threshold: 0.0,
                        left: None,
                        right: None,
                        value: Some(0.3),
                    },
                ],
            },
            GBDTTree {
                nodes: vec![
                    DecisionNode {
                        feature: 2,
                        threshold: 95.0,
                        left: Some(1),
                        right: Some(2),
                        value: None,
                    },
                    DecisionNode {
                        feature: 0,
                        threshold: 0.0,
                        left: None,
                        right: None,
                        value: Some(0.2),
                    },
                    DecisionNode {
                        feature: 0,
                        threshold: 0.0,
                        left: None,
                        right: None,
                        value: Some(0.4),
                    },
                ],
            },
        ],
        learning_rate: 0.1,
    };
    
    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("node1".into(), (1_000_000, 1.5, 99.0, 0.8)),
        ("node2".into(), (1_000_000, 3.0, 92.0, 0.6)),
    ]);
    
    let features = normalize_features(&telemetry, 1_000_000);
    let scores = compute_scores(&model, &features, "round_100").unwrap();
    
    // Verify scores are computed
    assert!(scores.contains_key("node1"));
    assert!(scores.contains_key("node2"));
    
    // Verify determinism by computing again
    let scores2 = compute_scores(&model, &features, "round_100").unwrap();
    assert_eq!(scores, scores2);
    
    println!("Complex model scoring verified: {:?}", scores);
}

#[test]
fn test_cross_node_consensus() {
    // Simulate 3 nodes computing scores for the same round
    let model = DeterministicGBDT {
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 1,
                    threshold: 1.8,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.9),
                },
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.5),
                },
            ],
        }],
        learning_rate: 1.0,
    };
    
    let ippan_time = 10_000_000;
    let round_ht = "round_42_hashtimer_abc123";
    
    // All nodes observe same telemetry (via consensus)
    let shared_telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("val1".into(), (10_000_050, 1.2, 99.9, 0.95)),
        ("val2".into(), (10_000_100, 2.5, 98.5, 0.85)),
    ]);
    
    // Node 1 computes
    let features_node1 = normalize_features(&shared_telemetry, ippan_time);
    let scores_node1 = compute_scores(&model, &features_node1, round_ht).unwrap();
    
    // Node 2 computes (independently)
    let features_node2 = normalize_features(&shared_telemetry, ippan_time);
    let scores_node2 = compute_scores(&model, &features_node2, round_ht).unwrap();
    
    // Node 3 computes (independently)
    let features_node3 = normalize_features(&shared_telemetry, ippan_time);
    let scores_node3 = compute_scores(&model, &features_node3, round_ht).unwrap();
    
    // All nodes must arrive at identical scores
    assert_eq!(scores_node1, scores_node2);
    assert_eq!(scores_node2, scores_node3);
    
    println!("Cross-node consensus verified: {:?}", scores_node1);
}

#[test]
fn test_invalid_model_detection() {
    // Test that models with NaN values are properly rejected
    let model_with_nan = DeterministicGBDT {
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
                    threshold: f64::NAN,
                    left: None,
                    right: None,
                    value: Some(1.0),
                },
            ],
        }],
        learning_rate: 1.0,
    };

    // Model hash should fail for NaN values
    assert!(model_with_nan.model_hash("test_round").is_err());

    // Test that models with infinity values are also rejected
    let model_with_inf = DeterministicGBDT {
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
                    threshold: f64::INFINITY,
                    left: None,
                    right: None,
                    value: Some(1.0),
                },
            ],
        }],
        learning_rate: 1.0,
    };

    // Model hash should fail for infinity values
    assert!(model_with_inf.model_hash("test_round").is_err());

    // Test that compute_scores also properly propagates these errors
    let telemetry: HashMap<String, (i64, f64, f64, f64)> = HashMap::from([
        ("node1".into(), (1_000_000, 1.0, 99.0, 0.8)),
    ]);
    let features = normalize_features(&telemetry, 1_000_000);

    // Should fail when trying to compute scores with invalid model
    assert!(compute_scores(&model_with_nan, &features, "test_round").is_err());
    assert!(compute_scores(&model_with_inf, &features, "test_round").is_err());

    println!("Invalid model detection verified - NaN and inf properly rejected");
}
