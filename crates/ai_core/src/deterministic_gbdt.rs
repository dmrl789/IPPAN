//! Deterministic Gradient-Boosted Decision Tree inference
//! Anchored to IPPAN Time and HashTimer
//!
//! Ensures that every node in the network computes identical
//! predictions and validator rankings within a round.

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

    /// Load model from binary file (shared by all validators)
    pub fn from_binary_file<P: AsRef<Path>>(path: P) -> Result<Self, DeterministicGBDTError> {
        let path = path.as_ref();
        let data = fs::read(path)
            .map_err(|e| DeterministicGBDTError::ModelLoadError(format!("Failed to read file {:?}: {}", path, e)))?;
        
        let model: DeterministicGBDT = bincode::deserialize(&data)
            .map_err(|e| DeterministicGBDTError::ModelLoadError(format!("Failed to parse binary: {}", e)))?;
        
        model.validate()?;
        Ok(model)
    }

    /// Save model to JSON file
    pub fn save_json<P: AsRef<Path>>(&self, path: P) -> Result<(), DeterministicGBDTError> {
        let path = path.as_ref();
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| DeterministicGBDTError::SerializationError(format!("JSON serialization failed: {}", e)))?;
        
        fs::write(path, json)
            .map_err(|e| DeterministicGBDTError::SerializationError(format!("Failed to write file {:?}: {}", path, e)))?;
        
        Ok(())
    }

    /// Save model to binary file
    pub fn save_binary<P: AsRef<Path>>(&self, path: P) -> Result<(), DeterministicGBDTError> {
        let path = path.as_ref();
        let data = bincode::serialize(self)
            .map_err(|e| DeterministicGBDTError::SerializationError(format!("Binary serialization failed: {}", e)))?;
        
        fs::write(path, data)
            .map_err(|e| DeterministicGBDTError::SerializationError(format!("Failed to write file {:?}: {}", path, e)))?;
        
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_model_creation_and_validation() {
        let model = create_test_model();
        assert_eq!(model.trees.len(), 1);
        assert_eq!(model.learning_rate, 0.1);
    }

    #[test]
    fn test_deterministic_prediction() {
        let model = create_test_model();
        let features = vec![1.0, 2.0, 3.0, 4.0];
        
        let result1 = model.predict(&features);
        let result2 = model.predict(&features);
        
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_model_hash_consistency() {
        let model = create_test_model();
        let round_hash = "test_round_hash";
        
        let hash1 = model.model_hash(round_hash);
        let hash2 = model.model_hash(round_hash);
        
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_normalize_features() {
        let mut telemetry = HashMap::new();
        telemetry.insert("node1".to_string(), (100_000, 1.2, 99.9, 0.42));
        telemetry.insert("node2".to_string(), (100_080, 0.9, 99.8, 0.38));
        
        let ippan_time_median = 100_050;
        let features = normalize_features(&telemetry, ippan_time_median);
        
        assert_eq!(features.len(), 2);
        assert_eq!(features[0].node_id, "node1");
        assert_eq!(features[0].delta_time_us, -50);
        assert_eq!(features[1].node_id, "node2");
        assert_eq!(features[1].delta_time_us, 30);
    }

    #[test]
    fn test_compute_scores() {
        let model = create_test_model();
        let mut telemetry = HashMap::new();
        telemetry.insert("node1".to_string(), (100_000, 1.2, 99.9, 0.42));
        telemetry.insert("node2".to_string(), (100_080, 0.9, 99.8, 0.38));
        
        let ippan_time_median = 100_050;
        let features = normalize_features(&telemetry, ippan_time_median);
        let round_hash = "test_round";
        
        let scores = compute_scores(&model, &features, round_hash);
        
        assert_eq!(scores.len(), 2);
        assert!(scores.contains_key("node1"));
        assert!(scores.contains_key("node2"));
    }

    #[test]
    fn test_determinism_across_calls() {
        let model = create_test_model();
        let features = vec![1.0, 2.0, 3.0, 4.0];
        
        // Multiple predictions should be identical
        let results: Vec<f64> = (0..10).map(|_| model.predict(&features)).collect();
        
        for i in 1..results.len() {
            assert_eq!(results[0], results[i]);
        }
    }

    #[test]
    fn test_model_validation() {
        // Test valid model
        let valid_model = create_test_model();
        assert!(valid_model.validate().is_ok());

        // Test invalid model with empty trees
        let invalid_model = DeterministicGBDT {
            trees: vec![],
            learning_rate: 0.1,
        };
        assert!(invalid_model.validate().is_err());
    }

    #[test]
    fn test_json_serialization() {
        let model = create_test_model();
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: DeterministicGBDT = serde_json::from_str(&json).unwrap();
        
        assert_eq!(model.trees.len(), deserialized.trees.len());
        assert_eq!(model.learning_rate, deserialized.learning_rate);
    }
}