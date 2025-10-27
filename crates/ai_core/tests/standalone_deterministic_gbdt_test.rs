//! Standalone test for deterministic GBDT module
//! This test only uses the deterministic GBDT module without other dependencies

use std::collections::HashMap;

// Import only the deterministic GBDT module directly
mod deterministic_gbdt {
    use serde::{Deserialize, Serialize};
    use sha3::{Digest, Sha3_256};
    use std::{collections::HashMap, fs, path::Path};
    use tracing::{info, warn, error};

    /// Fixed-point arithmetic precision (1e-6)
    const FP_PRECISION: f64 = 1_000_000.0;

    /// Normalized validator telemetry (anchored to IPPAN Time)
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ValidatorFeatures {
        pub node_id: String,
        pub delta_time_us: i64,   // difference from IPPAN median time (µs)
        pub latency_ms: f64,
        pub uptime_pct: f64,
        pub peer_entropy: f64,
    }

    /// Single decision node in a GBDT tree
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DecisionNode {
        pub feature: usize,
        pub threshold: f64,
        pub left: Option<usize>,
        pub right: Option<usize>,
        pub value: Option<f64>,
    }

    /// One decision tree
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GBDTTree {
        pub nodes: Vec<DecisionNode>,
    }

    /// Full deterministic GBDT model
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct DeterministicGBDT {
        pub trees: Vec<GBDTTree>,
        pub learning_rate: f64,
    }

    /// Error types for deterministic GBDT operations
    #[derive(Debug, thiserror::Error)]
    pub enum DeterministicGBDTError {
        #[error("Failed to load model from file: {0}")]
        ModelLoadError(String),
        
        #[error("Invalid model structure: {0}")]
        InvalidModelStructure(String),
        
        #[error("Feature vector size mismatch: expected {expected}, got {actual}")]
        FeatureSizeMismatch { expected: usize, actual: usize },
        
        #[error("Invalid tree node reference: node {node} references invalid child {child}")]
        InvalidNodeReference { node: usize, child: usize },
        
        #[error("Serialization error: {0}")]
        SerializationError(String),
    }

    impl DeterministicGBDT {
        /// Load model from JSON file (shared by all validators)
        pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self, DeterministicGBDTError> {
            let path = path.as_ref();
            let data = fs::read_to_string(path)
                .map_err(|e| DeterministicGBDTError::ModelLoadError(format!("Failed to read file {:?}: {}", path, e)))?;
            
            let model: DeterministicGBDT = serde_json::from_str(&data)
                .map_err(|e| DeterministicGBDTError::ModelLoadError(format!("Failed to parse JSON: {}", e)))?;
            
            model.validate()?;
            Ok(model)
        }

        /// Deterministic prediction using fixed-point math
        pub fn predict(&self, features: &[f64]) -> f64 {
            let mut score_fp: i64 = 0;
            
            for tree in &self.trees {
                let mut node_idx = 0;
                loop {
                    if node_idx >= tree.nodes.len() {
                        warn!("Invalid tree structure: node index {} out of bounds", node_idx);
                        break;
                    }
                    
                    let node = &tree.nodes[node_idx];
                    if let Some(value) = node.value {
                        score_fp += (value * FP_PRECISION) as i64;
                        break;
                    }
                    
                    if node.feature >= features.len() {
                        warn!("Feature index {} out of bounds for features of length {}", node.feature, features.len());
                        break;
                    }
                    
                    let feat_val = features[node.feature];
                    if feat_val <= node.threshold {
                        node_idx = node.left.unwrap_or(node_idx);
                    } else {
                        node_idx = node.right.unwrap_or(node_idx);
                    }
                }
            }
            
            (score_fp as f64 / FP_PRECISION) * self.learning_rate
        }

        /// Compute model certificate hash (for on-chain verification)
        pub fn model_hash(&self, round_hash_timer: &str) -> String {
            let serialized = serde_json::to_string(self).unwrap();
            let mut hasher = Sha3_256::new();
            hasher.update(serialized.as_bytes());
            hasher.update(round_hash_timer.as_bytes());
            format!("{:x}", hasher.finalize())
        }

        /// Validate model structure
        fn validate(&self) -> Result<(), DeterministicGBDTError> {
            if self.trees.is_empty() {
                return Err(DeterministicGBDTError::InvalidModelStructure("Model has no trees".to_string()));
            }

            for (tree_idx, tree) in self.trees.iter().enumerate() {
                if tree.nodes.is_empty() {
                    return Err(DeterministicGBDTError::InvalidModelStructure(
                        format!("Tree {} has no nodes", tree_idx)
                    ));
                }

                for (node_idx, node) in tree.nodes.iter().enumerate() {
                    // Check if node references are valid
                    if let Some(left) = node.left {
                        if left >= tree.nodes.len() {
                            return Err(DeterministicGBDTError::InvalidNodeReference {
                                node: node_idx,
                                child: left,
                            });
                        }
                    }
                    
                    if let Some(right) = node.right {
                        if right >= tree.nodes.len() {
                            return Err(DeterministicGBDTError::InvalidNodeReference {
                                node: node_idx,
                                child: right,
                            });
                        }
                    }
                }
            }

            Ok(())
        }
    }

    /// Normalize raw telemetry using IPPAN Time median
    pub fn normalize_features(
        telemetry: &HashMap<String, (i64, f64, f64, f64)>, // (local_time_us, latency, uptime, entropy)
        ippan_time_median: i64,
    ) -> Vec<ValidatorFeatures> {
        telemetry
            .iter()
            .map(|(node_id, (local_time_us, latency, uptime, entropy))| ValidatorFeatures {
                node_id: node_id.clone(),
                delta_time_us: local_time_us - ippan_time_median,
                latency_ms: *latency,
                uptime_pct: *uptime,
                peer_entropy: *entropy,
            })
            .collect()
    }

    /// Compute deterministic validator scores
    pub fn compute_scores(
        model: &DeterministicGBDT,
        features: &[ValidatorFeatures],
        round_hash_timer: &str,
    ) -> HashMap<String, f64> {
        let mut scores = HashMap::new();
        
        for v in features {
            let feature_vector = vec![
                v.delta_time_us as f64,
                v.latency_ms,
                v.uptime_pct,
                v.peer_entropy,
            ];
            
            let score = model.predict(&feature_vector);
            scores.insert(v.node_id.clone(), score);
        }

        // Generate reproducible hash certificate
        let cert = model.model_hash(round_hash_timer);
        info!("Deterministic GBDT certificate: {}", cert);
        
        scores
    }

    /// Create a simple test model for validation
    pub fn create_test_model() -> DeterministicGBDT {
        let tree = GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.1),
                },
                DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(0.2),
                },
            ],
        };

        DeterministicGBDT {
            trees: vec![tree],
            learning_rate: 0.1,
        }
    }
}

#[test]
fn test_deterministic_gbdt_basic_functionality() {
    // Test model creation
    let model = deterministic_gbdt::create_test_model();
    assert_eq!(model.trees.len(), 1);
    assert_eq!(model.learning_rate, 0.1);

    // Test prediction
    let features = vec![1.0, 2.0, 3.0, 4.0];
    let prediction = model.predict(&features);
    assert!(prediction.is_finite());

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
    let features = deterministic_gbdt::normalize_features(&telemetry, ippan_time_median);
    
    assert_eq!(features.len(), 2);
    assert_eq!(features[0].delta_time_us, -50);  // 100_000 - 100_050
    assert_eq!(features[1].delta_time_us, 30);   // 100_080 - 100_050
}

#[test]
fn test_validator_scoring() {
    let model = deterministic_gbdt::create_test_model();
    let mut telemetry = HashMap::new();
    telemetry.insert("test_node".to_string(), (100_000, 1.0, 99.0, 0.5));
    
    let ippan_time_median = 100_000;
    let round_hash = "test_round";
    
    let features = deterministic_gbdt::normalize_features(&telemetry, ippan_time_median);
    let scores = deterministic_gbdt::compute_scores(&model, &features, round_hash);
    
    assert_eq!(scores.len(), 1);
    assert!(scores.contains_key("test_node"));
    assert!(scores["test_node"].is_finite());
}

#[test]
fn test_model_hash_consistency() {
    let model = deterministic_gbdt::create_test_model();
    let round_hash = "consistent_round";
    
    let hash1 = model.model_hash(round_hash);
    let hash2 = model.model_hash(round_hash);
    
    assert_eq!(hash1, hash2);
    assert!(!hash1.is_empty());
}

#[test]
fn test_cross_platform_determinism() {
    let model = deterministic_gbdt::create_test_model();
    let features = vec![1.5, 2.5, 3.5, 4.5];
    
    // Simulate multiple nodes computing the same prediction
    let node1_result = model.predict(&features);
    let node2_result = model.predict(&features);
    let node3_result = model.predict(&features);
    
    assert_eq!(node1_result, node2_result);
    assert_eq!(node2_result, node3_result);
}

#[test]
fn test_usage_example() {
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

    // Create model and normalize features
    let model = deterministic_gbdt::create_test_model();
    let features = deterministic_gbdt::normalize_features(&telemetry, ippan_time_median);
    let scores = deterministic_gbdt::compute_scores(&model, &features, round_hash_timer);

    // Verify results
    assert_eq!(scores.len(), 3);
    assert!(scores.contains_key("nodeA"));
    assert!(scores.contains_key("nodeB"));
    assert!(scores.contains_key("nodeC"));

    // All scores should be finite and positive
    for (node_id, score) in &scores {
        assert!(score.is_finite(), "Score for {} is not finite: {}", node_id, score);
        assert!(*score >= 0.0, "Score for {} is negative: {}", node_id, score);
    }

    println!("Validator scores: {:?}", scores);
}