//! Deterministic Gradient-Boosted Decision Tree (GBDT) inference
//! Anchored to IPPAN Time and HashTimer for consensus-safe validator scoring.
//!
//! Ensures identical predictions, rankings, and hashes across all validator nodes.

use crate::fixed_point::FixedPoint;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{collections::HashMap, fs, path::Path};
use tracing::{info, warn};

/// Normalized validator telemetry (anchored to IPPAN Time)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatorFeatures {
    pub node_id: String,
    pub delta_time_us: i64, // deviation from IPPAN Time median (µs)
    pub latency_ms: FixedPoint,
    pub uptime_pct: FixedPoint,
    pub peer_entropy: FixedPoint,
    pub cpu_usage: Option<FixedPoint>,
    pub memory_usage: Option<FixedPoint>,
    pub network_reliability: Option<FixedPoint>,
}

/// Decision node in a GBDT tree
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecisionNode {
    pub feature: usize,
    pub threshold: FixedPoint,
    pub left: Option<usize>,
    pub right: Option<usize>,
    pub value: Option<FixedPoint>,
}

/// One decision tree
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GBDTTree {
    pub nodes: Vec<DecisionNode>,
}

/// Full deterministic GBDT model
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeterministicGBDT {
    pub trees: Vec<GBDTTree>,
    pub learning_rate: FixedPoint,
}

/// Error types for deterministic GBDT operations
#[derive(Debug, thiserror::Error)]
pub enum DeterministicGBDTError {
    #[error("Failed to load model: {0}")]
    ModelLoadError(String),
    #[error("Invalid model structure: {0}")]
    InvalidModelStructure(String),
    #[error("Invalid node reference in tree {tree} node {node}")]
    InvalidNodeReference { tree: usize, node: usize },
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl DeterministicGBDT {
    // ---------------------------------------------------------------------
    // Loading / saving
    // ---------------------------------------------------------------------

    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self, DeterministicGBDTError> {
        let data = fs::read_to_string(path.as_ref())
            .map_err(|e| DeterministicGBDTError::ModelLoadError(e.to_string()))?;
        let model: Self = serde_json::from_str(&data)
            .map_err(|e| DeterministicGBDTError::ModelLoadError(e.to_string()))?;
        model.validate()?;
        Ok(model)
    }

    pub fn from_binary_file<P: AsRef<Path>>(path: P) -> Result<Self, DeterministicGBDTError> {
        let data = fs::read(path.as_ref())
            .map_err(|e| DeterministicGBDTError::ModelLoadError(e.to_string()))?;
        let model: Self = bincode::deserialize(&data)
            .map_err(|e| DeterministicGBDTError::ModelLoadError(e.to_string()))?;
        model.validate()?;
        Ok(model)
    }

    pub fn save_json<P: AsRef<Path>>(&self, path: P) -> Result<(), DeterministicGBDTError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))?;
        fs::write(path, json)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))?;
        Ok(())
    }

    pub fn save_binary<P: AsRef<Path>>(&self, path: P) -> Result<(), DeterministicGBDTError> {
        let data = bincode::serialize(self)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))?;
        fs::write(path, data)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))?;
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Deterministic inference
    // ---------------------------------------------------------------------

    /// Deterministic prediction using fixed-point arithmetic
    pub fn predict(&self, features: &[FixedPoint]) -> FixedPoint {
        let mut score = FixedPoint::zero();

        for tree in &self.trees {
            let mut node_idx = 0usize;
            loop {
                if node_idx >= tree.nodes.len() {
                    warn!("Invalid node index {}", node_idx);
                    break;
                }
                let node = &tree.nodes[node_idx];

                if let Some(value) = node.value {
                    score = score + value;
                    break;
                }

                if node.feature >= features.len() {
                    warn!(
                        "Feature index {} out of bounds (len={})",
                        node.feature,
                        features.len()
                    );
                    break;
                }

                let feat_val = features[node.feature];
                node_idx = if feat_val <= node.threshold {
                    node.left.unwrap_or(node_idx)
                } else {
                    node.right.unwrap_or(node_idx)
                };
            }
        }

        score * self.learning_rate
    }

    /// Deterministic model certificate hash (anchors to HashTimer)
    pub fn model_hash(&self, round_hash_timer: &str) -> Result<String, DeterministicGBDTError> {
        let serialized = serde_json::to_vec(self)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))?;
        let mut hasher = Sha3_256::new();
        hasher.update(&serialized);
        hasher.update(round_hash_timer.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Validate structural correctness
    pub fn validate(&self) -> Result<(), DeterministicGBDTError> {
        if self.trees.is_empty() {
            return Err(DeterministicGBDTError::InvalidModelStructure(
                "Model has no trees".into(),
            ));
        }

        for (t_idx, tree) in self.trees.iter().enumerate() {
            if tree.nodes.is_empty() {
                return Err(DeterministicGBDTError::InvalidModelStructure(format!(
                    "Tree {t_idx} has no nodes"
                )));
            }

            for (n_idx, node) in tree.nodes.iter().enumerate() {
                if let Some(left) = node.left {
                    if left >= tree.nodes.len() {
                        return Err(DeterministicGBDTError::InvalidNodeReference {
                            tree: t_idx,
                            node: n_idx,
                        });
                    }
                }
                if let Some(right) = node.right {
                    if right >= tree.nodes.len() {
                        return Err(DeterministicGBDTError::InvalidNodeReference {
                            tree: t_idx,
                            node: n_idx,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Feature normalization & scoring
// ---------------------------------------------------------------------

/// Normalize raw telemetry using IPPAN Time median.
/// Input: map (node_id → (local_time_us, latency_ms, uptime, entropy))
pub fn normalize_features(
    telemetry: &HashMap<String, (i64, f64, f64, f64)>,
    ippan_time_median: i64,
) -> Vec<ValidatorFeatures> {
    telemetry
        .iter()
        .map(|(node_id, (local_time_us, latency, uptime, entropy))| {
            let delta_time_us = local_time_us - ippan_time_median;
            ValidatorFeatures {
                node_id: node_id.clone(),
                delta_time_us,
                latency_ms: FixedPoint::from_f64(*latency),
                uptime_pct: FixedPoint::from_f64(*uptime),
                peer_entropy: FixedPoint::from_f64(*entropy),
                cpu_usage: None,
                memory_usage: None,
                network_reliability: None,
            }
        })
        .collect()
}

/// Compute deterministic validator scores
pub fn compute_scores(
    model: &DeterministicGBDT,
    features: &[ValidatorFeatures],
    round_hash_timer: &str,
) -> Result<HashMap<String, FixedPoint>, DeterministicGBDTError> {
    let mut scores = HashMap::new();

    for v in features {
        let feature_vector = [
            FixedPoint::from_integer(v.delta_time_us),
            v.latency_ms,
            v.uptime_pct,
            v.peer_entropy,
        ];
        let score = model.predict(&feature_vector);
        scores.insert(v.node_id.clone(), score);
    }

    let cert = model.model_hash(round_hash_timer)?;
    info!("Deterministic GBDT certificate: {}", cert);
    Ok(scores)
}

fn sample_test_model() -> DeterministicGBDT {
    let tree = GBDTTree {
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
                value: Some(FixedPoint::from_ratio(1, 10)),
            },
            DecisionNode {
                feature: 0,
                threshold: FixedPoint::zero(),
                left: None,
                right: None,
                value: Some(FixedPoint::from_ratio(2, 10)),
            },
        ],
    };

    DeterministicGBDT {
        trees: vec![tree],
        learning_rate: FixedPoint::from_ratio(1, 10),
    }
}

/// Helper for integration tests and documentation examples.
pub fn create_test_model() -> DeterministicGBDT {
    sample_test_model()
}

impl DeterministicGBDT {
    /// Construct a deterministic test model useful for examples and benchmarks.
    pub fn create_test_model() -> Self {
        sample_test_model()
    }
}

// ---------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_model() -> DeterministicGBDT {
        let tree = GBDTTree {
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
                    value: Some(FixedPoint::from_ratio(1, 10)),
                },
                DecisionNode {
                    feature: 0,
                    threshold: FixedPoint::zero(),
                    left: None,
                    right: None,
                    value: Some(FixedPoint::from_ratio(2, 10)),
                },
            ],
        };
        DeterministicGBDT {
            trees: vec![tree],
            learning_rate: FixedPoint::from_ratio(1, 10),
        }
    }

    #[test]
    fn test_prediction_determinism() {
        let model = create_test_model();
        let features = [
            FixedPoint::from_integer(1),
            FixedPoint::from_integer(2),
            FixedPoint::from_integer(3),
            FixedPoint::from_integer(4),
        ];
        let r1 = model.predict(&features);
        let r2 = model.predict(&features);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_model_hash_consistency() {
        let model = create_test_model();
        let h1 = model.model_hash("round1").unwrap();
        let h2 = model.model_hash("round1").unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_normalization_and_scores() {
        let mut telemetry = HashMap::new();
        telemetry.insert("node1".into(), (100_000, 1.2, 99.9, 0.42));
        telemetry.insert("node2".into(), (100_080, 0.9, 99.8, 0.38));
        let median = 100_050;
        let features = normalize_features(&telemetry, median);
        let model = create_test_model();
        let scores = compute_scores(&model, &features, "round_hash").unwrap();
        assert_eq!(scores.len(), 2);
        assert!(scores.contains_key("node1"));
    }

    #[test]
    fn test_model_validation() {
        let valid = create_test_model();
        assert!(valid.validate().is_ok());
        let invalid = DeterministicGBDT {
            trees: vec![],
            learning_rate: FixedPoint::from_ratio(1, 10),
        };
        assert!(invalid.validate().is_err());
    }
}
