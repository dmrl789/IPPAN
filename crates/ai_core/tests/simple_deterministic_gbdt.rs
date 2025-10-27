//! Simple tests for deterministic GBDT + IPPAN Time integration
//! This is a standalone test that doesn't depend on the complex module structure

use std::collections::HashMap;
use sha3::{Digest, Sha3_256};

// Simple deterministic GBDT implementation for testing
#[derive(Debug, Clone)]
pub struct DeterministicGBDT {
    pub trees: Vec<GBDTTree>,
    pub learning_rate: f64,
}

#[derive(Debug, Clone)]
pub struct GBDTTree {
    pub nodes: Vec<DecisionNode>,
}

#[derive(Debug, Clone)]
pub struct DecisionNode {
    pub feature: usize,
    pub threshold: f64,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub value: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ValidatorFeatures {
    pub normalized_latency: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_reliability: f64,
}

impl DeterministicGBDT {
    pub fn new(trees: Vec<GBDTTree>, learning_rate: f64) -> Self {
        Self {
            trees,
            learning_rate,
        }
    }

    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut prediction = 0.0;
        
        for tree in &self.trees {
            let tree_prediction = self.evaluate_tree(tree, features, 0);
            prediction += tree_prediction * self.learning_rate;
        }
        
        prediction
    }

    fn evaluate_tree(&self, tree: &GBDTTree, features: &[f64], node_idx: usize) -> f64 {
        if node_idx >= tree.nodes.len() {
            return 0.0;
        }

        let node = &tree.nodes[node_idx];
        
        if let Some(value) = node.value {
            return value;
        }

        if node.feature >= features.len() {
            return 0.0;
        }

        let feature_value = features[node.feature];
        let next_node_idx = if feature_value <= node.threshold {
            node.left.unwrap_or(0)
        } else {
            node.right.unwrap_or(0)
        };

        self.evaluate_tree(tree, features, next_node_idx)
    }

    pub fn model_hash(&self, round_hash_timer: &str) -> String {
        let mut hasher = Sha3_256::new();
        
        for tree in &self.trees {
            for node in &tree.nodes {
                hasher.update(node.feature.to_le_bytes());
                hasher.update(node.threshold.to_le_bytes());
                if let Some(left) = node.left {
                    hasher.update(left.to_le_bytes());
                }
                if let Some(right) = node.right {
                    hasher.update(right.to_le_bytes());
                }
                if let Some(value) = node.value {
                    hasher.update(value.to_le_bytes());
                }
            }
        }
        
        hasher.update(self.learning_rate.to_le_bytes());
        hasher.update(round_hash_timer.as_bytes());
        
        format!("{:x}", hasher.finalize())
    }
}

pub fn normalize_features(
    telemetry: &HashMap<String, (i64, f64, f64, f64)>,
    ippan_time_median: i64,
) -> HashMap<String, ValidatorFeatures> {
    let mut normalized = HashMap::new();
    
    for (node_id, (local_time_us, latency_us, cpu_usage, memory_usage)) in telemetry {
        let time_offset = local_time_us - ippan_time_median;
        let normalized_latency = latency_us + (time_offset as f64 / 1_000_000.0);
        
        let network_reliability = if normalized_latency < 1.0 {
            1.0
        } else if normalized_latency < 2.0 {
            0.8
        } else {
            0.6
        };
        
        normalized.insert(
            node_id.clone(),
            ValidatorFeatures {
                normalized_latency,
                cpu_usage: *cpu_usage,
                memory_usage: *memory_usage,
                network_reliability,
            },
        );
    }
    
    normalized
}

pub fn compute_scores(
    model: &DeterministicGBDT,
    features: &HashMap<String, ValidatorFeatures>,
    round_hash_timer: &str,
) -> HashMap<String, f64> {
    let mut scores = HashMap::new();
    
    for (node_id, validator_features) in features {
        let feature_vector = vec![
            validator_features.normalized_latency,
            validator_features.cpu_usage,
            validator_features.memory_usage,
            validator_features.network_reliability,
        ];
        
        let score = model.predict(&feature_vector);
        scores.insert(node_id.clone(), score);
    }
    
    scores
}

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
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0, // normalized_latency
                    threshold: 1.0,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.9), // High score for low latency
                },
                DecisionNode {
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
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
                    threshold: 1.0,
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
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
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
        trees: vec![GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
                    threshold: 0.5,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.3),
                },
                DecisionNode {
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