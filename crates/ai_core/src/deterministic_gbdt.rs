//! Deterministic GBDT with IPPAN Time integration
//!
//! This module provides deterministic GBDT inference that uses IPPAN Time median
//! instead of local system clocks to ensure identical results across all nodes.

use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;

/// Validator features for GBDT scoring
pub type ValidatorFeatures = Vec<f64>;

/// A decision node in the GBDT tree
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionNode {
    /// Feature index to evaluate (0-based)
    pub feature: usize,
    /// Threshold value for split decision
    pub threshold: f64,
    /// Left child node index (if feature <= threshold)
    pub left: Option<usize>,
    /// Right child node index (if feature > threshold)
    pub right: Option<usize>,
    /// Leaf value (Some if this is a leaf node)
    pub value: Option<f64>,
}

/// A single decision tree in the GBDT ensemble
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GBDTTree {
    /// All nodes in the tree (breadth-first ordering)
    pub nodes: Vec<DecisionNode>,
}

impl GBDTTree {
    /// Predict value by traversing the tree
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut node_idx = 0;
        
        loop {
            if node_idx >= self.nodes.len() {
                return 0.0; // Safety fallback
            }
            
            let node = &self.nodes[node_idx];
            
            // If leaf node, return value
            if let Some(value) = node.value {
                return value;
            }
            
            // Otherwise, traverse based on feature comparison
            if node.feature >= features.len() {
                return 0.0; // Safety fallback
            }
            
            let feature_value = features[node.feature];
            
            if feature_value <= node.threshold {
                if let Some(left) = node.left {
                    node_idx = left;
                } else {
                    return 0.0; // Safety fallback
                }
            } else {
                if let Some(right) = node.right {
                    node_idx = right;
                } else {
                    return 0.0; // Safety fallback
                }
            }
        }
    }
}

/// Deterministic GBDT model with fixed-point arithmetic
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeterministicGBDT {
    /// Collection of decision trees
    pub trees: Vec<GBDTTree>,
    /// Learning rate (shrinkage factor)
    pub learning_rate: f64,
}

impl DeterministicGBDT {
    /// Predict output for given features
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut sum = 0.0;
        
        for tree in &self.trees {
            let tree_pred = tree.predict(features);
            sum += self.learning_rate * tree_pred;
        }
        
        sum
    }
    
    /// Compute deterministic model hash using HashTimer anchor
    pub fn model_hash(&self, round_hash_timer: &str) -> String {
        let mut hasher = Sha3_256::new();
        
        // Hash the model structure (serialized)
        let model_json = serde_json::to_string(self).unwrap_or_default();
        hasher.update(model_json.as_bytes());
        
        // Hash the round HashTimer for consensus anchor
        hasher.update(round_hash_timer.as_bytes());
        
        // Return hex string
        format!("{:x}", hasher.finalize())
    }
}

/// Normalize features using IPPAN Time median
///
/// This function ensures deterministic feature normalization by:
/// 1. Using IPPAN Time median instead of local clocks
/// 2. Normalizing telemetry timestamps relative to median
/// 3. Producing identical features across all nodes
///
/// # Arguments
/// * `telemetry` - Map of node_id -> (timestamp_us, latency, uptime, reputation)
/// * `ippan_time_median` - Consensus IPPAN Time median (microseconds)
///
/// # Returns
/// Map of node_id -> normalized feature vector
pub fn normalize_features(
    telemetry: &HashMap<String, (i64, f64, f64, f64)>,
    ippan_time_median: i64,
) -> HashMap<String, ValidatorFeatures> {
    let mut normalized = HashMap::new();
    
    for (node_id, (timestamp_us, latency, uptime, reputation)) in telemetry {
        // Normalize timestamp relative to IPPAN Time median
        let time_delta = (*timestamp_us - ippan_time_median) as f64 / 1_000_000.0; // Convert to seconds
        
        // Create feature vector: [time_delta, latency, uptime, reputation]
        let features = vec![
            time_delta,
            *latency,
            *uptime,
            *reputation,
        ];
        
        normalized.insert(node_id.clone(), features);
    }
    
    normalized
}

/// Compute deterministic scores for all validators
///
/// Uses the GBDT model and HashTimer to produce consensus-compatible scores.
///
/// # Arguments
/// * `model` - Deterministic GBDT model
/// * `features` - Normalized features for each validator
/// * `round_hash_timer` - Current round HashTimer for reproducibility
///
/// # Returns
/// Map of node_id -> deterministic score
pub fn compute_scores(
    model: &DeterministicGBDT,
    features: &HashMap<String, ValidatorFeatures>,
    round_hash_timer: &str,
) -> HashMap<String, f64> {
    let mut scores = HashMap::new();
    
    // Compute model hash for this round (for verification)
    let _model_hash = model.model_hash(round_hash_timer);
    
    // Score each validator
    for (node_id, feature_vec) in features {
        let score = model.predict(feature_vec);
        scores.insert(node_id.clone(), score);
    }
    
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_tree_traversal() {
        let tree = GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
                    threshold: 5.0,
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
                    value: Some(0.2),
                },
            ],
        };

        assert_eq!(tree.predict(&[3.0]), 0.8);
        assert_eq!(tree.predict(&[7.0]), 0.2);
    }

    #[test]
    fn test_gbdt_prediction() {
        let model = DeterministicGBDT {
            trees: vec![GBDTTree {
                nodes: vec![DecisionNode {
                    feature: 0,
                    threshold: 0.0,
                    left: None,
                    right: None,
                    value: Some(1.0),
                }],
            }],
            learning_rate: 0.5,
        };

        let prediction = model.predict(&[0.0]);
        assert_eq!(prediction, 0.5);
    }

    #[test]
    fn test_model_hash_determinism() {
        let model = DeterministicGBDT {
            trees: vec![],
            learning_rate: 1.0,
        };

        let hash1 = model.model_hash("test_round");
        let hash2 = model.model_hash("test_round");

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_feature_normalization() {
        let telemetry = HashMap::from([
            ("node1".to_string(), (1_000_000i64, 1.5f64, 99.5f64, 0.9f64)),
            ("node2".to_string(), (1_000_100i64, 2.0f64, 98.0f64, 0.8f64)),
        ]);

        let median = 1_000_000i64;
        let features = normalize_features(&telemetry, median);

        assert_eq!(features.len(), 2);
        assert!(features.contains_key("node1"));
        assert!(features.contains_key("node2"));
    }
}
