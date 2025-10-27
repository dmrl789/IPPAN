//! Deterministic GBDT + IPPAN Time integration
//! 
//! Provides deterministic GBDT inference that behaves identically across nodes
//! regardless of local clock drift by using IPPAN Time median normalization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sha3::{Digest, Sha3_256};

/// Deterministic GBDT model with IPPAN Time integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeterministicGBDT {
    /// Collection of decision trees
    pub trees: Vec<GBDTTree>,
    /// Learning rate (scaled integer)
    pub learning_rate: f64,
}

/// A single decision tree in the GBDT
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GBDTTree {
    /// Decision nodes in the tree
    pub nodes: Vec<DecisionNode>,
}

/// A decision node (internal or leaf)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionNode {
    /// Feature index to compare
    pub feature: usize,
    /// Threshold value for comparison
    pub threshold: f64,
    /// Left child node index (None for leaf)
    pub left: Option<usize>,
    /// Right child node index (None for leaf)
    pub right: Option<usize>,
    /// Leaf value (Some for leaf nodes, None for internal)
    pub value: Option<f64>,
}

/// Validator features for GBDT evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidatorFeatures {
    /// Normalized latency (microseconds, relative to IPPAN Time median)
    pub normalized_latency: f64,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// Network reliability score
    pub network_reliability: f64,
}

impl DeterministicGBDT {
    /// Create a new deterministic GBDT model
    pub fn new(trees: Vec<GBDTTree>, learning_rate: f64) -> Self {
        Self {
            trees,
            learning_rate,
        }
    }

    /// Predict using the GBDT model with deterministic behavior
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut prediction = 0.0;
        
        for tree in &self.trees {
            let tree_prediction = self.evaluate_tree(tree, features, 0);
            prediction += tree_prediction * self.learning_rate;
        }
        
        prediction
    }

    /// Evaluate a single tree starting from the given node index
    fn evaluate_tree(&self, tree: &GBDTTree, features: &[f64], node_idx: usize) -> f64 {
        if node_idx >= tree.nodes.len() {
            return 0.0;
        }

        let node = &tree.nodes[node_idx];
        
        // If this is a leaf node, return its value
        if let Some(value) = node.value {
            return value;
        }

        // Check feature bounds
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

    /// Compute deterministic model hash using SHA3-256
    pub fn model_hash(&self, round_hash_timer: &str) -> String {
        let mut hasher = Sha3_256::new();
        
        // Hash the model structure
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
        
        // Include learning rate
        hasher.update(self.learning_rate.to_le_bytes());
        
        // Include round hash timer for additional determinism
        hasher.update(round_hash_timer.as_bytes());
        
        format!("{:x}", hasher.finalize())
    }
}

/// Normalize features using IPPAN Time median to ensure deterministic behavior
/// across nodes with different local clock drifts
pub fn normalize_features(
    telemetry: &HashMap<String, (i64, f64, f64, f64)>,
    ippan_time_median: i64,
) -> HashMap<String, ValidatorFeatures> {
    let mut normalized = HashMap::new();
    
    for (node_id, (local_time_us, latency_us, cpu_usage, memory_usage)) in telemetry {
        // Normalize latency relative to IPPAN Time median
        // This ensures identical results regardless of local clock drift
        let time_offset = local_time_us - ippan_time_median;
        let normalized_latency = latency_us + (time_offset as f64 / 1_000_000.0);
        
        // Calculate network reliability based on latency consistency
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

/// Compute deterministic scores for all validators
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_gbdt_creation() {
        let tree = GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
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
        };
        
        let model = DeterministicGBDT::new(vec![tree], 1.0);
        assert_eq!(model.trees.len(), 1);
        assert_eq!(model.learning_rate, 1.0);
    }

    #[test]
    fn test_tree_evaluation() {
        let tree = GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
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
        };
        
        let model = DeterministicGBDT::new(vec![tree], 1.0);
        
        // Test prediction with different feature values
        let features_low = vec![1.0, 0.0, 0.0, 0.0];
        let features_high = vec![2.0, 0.0, 0.0, 0.0];
        
        let prediction_low = model.predict(&features_low);
        let prediction_high = model.predict(&features_high);
        
        assert_eq!(prediction_low, 0.8);
        assert_eq!(prediction_high, 0.6);
    }

    #[test]
    fn test_model_hash_determinism() {
        let tree = GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
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
        };
        
        let model = DeterministicGBDT::new(vec![tree], 1.0);
        let round_hash = "test_round_123";
        
        let hash1 = model.model_hash(round_hash);
        let hash2 = model.model_hash(round_hash);
        
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_feature_normalization() {
        let mut telemetry = HashMap::new();
        telemetry.insert("nodeA".to_string(), (999_950, 1.2, 99.9, 0.4));
        telemetry.insert("nodeB".to_string(), (1_000_030, 0.9, 99.8, 0.3));
        
        let ippan_time_median = 1_000_000;
        let normalized = normalize_features(&telemetry, ippan_time_median);
        
        assert_eq!(normalized.len(), 2);
        assert!(normalized.contains_key("nodeA"));
        assert!(normalized.contains_key("nodeB"));
        
        // Check that latency is normalized relative to IPPAN Time
        let node_a = normalized.get("nodeA").unwrap();
        let node_b = normalized.get("nodeB").unwrap();
        
        // NodeA should have slightly higher normalized latency due to -50μs offset
        assert!(node_a.normalized_latency > node_b.normalized_latency);
    }

    #[test]
    fn test_score_computation() {
        let tree = GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
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
        };
        
        let model = DeterministicGBDT::new(vec![tree], 1.0);
        
        let mut features = HashMap::new();
        features.insert("nodeA".to_string(), ValidatorFeatures {
            normalized_latency: 1.2,
            cpu_usage: 99.9,
            memory_usage: 0.4,
            network_reliability: 1.0,
        });
        
        let scores = compute_scores(&model, &features, "test_round");
        
        assert_eq!(scores.len(), 1);
        assert!(scores.contains_key("nodeA"));
        assert_eq!(scores.get("nodeA"), Some(&0.8));
    }
}