//! Deterministic Gradient-Boosted Decision Tree (GBDT) inference
//! Anchored to IPPAN Time and HashTimer for consensus-safe validator scoring.
//!
//! Ensures identical predictions, rankings, and hashes across all validator nodes.

use crate::fixed::Fixed;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{collections::HashMap, fs, path::Path};
use tracing::{info, warn};

/// Normalized validator telemetry (anchored to IPPAN Time)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatorFeatures {
    pub node_id: String,
    pub delta_time_us: i64, // deviation from IPPAN Time median (µs)
    #[cfg(feature = "deterministic_math")]
    pub latency_ms: Fixed,
    #[cfg(not(feature = "deterministic_math"))]
    pub latency_ms: f64,
    #[cfg(feature = "deterministic_math")]
    pub uptime_pct: Fixed,
    #[cfg(not(feature = "deterministic_math"))]
    pub uptime_pct: f64,
    #[cfg(feature = "deterministic_math")]
    pub peer_entropy: Fixed,
    #[cfg(not(feature = "deterministic_math"))]
    pub peer_entropy: f64,
    #[cfg(feature = "deterministic_math")]
    pub cpu_usage: Option<Fixed>,
    #[cfg(not(feature = "deterministic_math"))]
    pub cpu_usage: Option<f64>,
    #[cfg(feature = "deterministic_math")]
    pub memory_usage: Option<Fixed>,
    #[cfg(not(feature = "deterministic_math"))]
    pub memory_usage: Option<f64>,
    #[cfg(feature = "deterministic_math")]
    pub network_reliability: Option<Fixed>,
    #[cfg(not(feature = "deterministic_math"))]
    pub network_reliability: Option<f64>,
}

/// Decision node in a GBDT tree
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecisionNode {
    pub feature: usize,
    #[cfg(feature = "deterministic_math")]
    pub threshold: Fixed,
    #[cfg(not(feature = "deterministic_math"))]
    pub threshold: f64,
    pub left: Option<usize>,
    pub right: Option<usize>,
    #[cfg(feature = "deterministic_math")]
    pub value: Option<Fixed>,
    #[cfg(not(feature = "deterministic_math"))]
    pub value: Option<f64>,
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
    #[cfg(feature = "deterministic_math")]
    pub learning_rate: Fixed,
    #[cfg(not(feature = "deterministic_math"))]
    pub learning_rate: f64,
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
    #[cfg(feature = "deterministic_math")]
    pub fn predict(&self, features: &[Fixed]) -> Fixed {
        let mut score = Fixed::ZERO;

        for tree in &self.trees {
            let mut node_idx = 0usize;
            loop {
                if node_idx >= tree.nodes.len() {
                    warn!("Invalid node index {}", node_idx);
                    break;
                }
                let node = &tree.nodes[node_idx];

                if let Some(value) = node.value {
                    score += value;
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

    /// Deterministic prediction using fixed-point arithmetic (fallback to f64)
    #[cfg(not(feature = "deterministic_math"))]
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut score_fp: i64 = 0;
        const FP_PRECISION: f64 = 1_000_000.0;

        for tree in &self.trees {
            let mut node_idx = 0usize;
            loop {
                if node_idx >= tree.nodes.len() {
                    warn!("Invalid node index {}", node_idx);
                    break;
                }
                let node = &tree.nodes[node_idx];

                if let Some(value) = node.value {
                    score_fp += (value * FP_PRECISION) as i64;
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

        (score_fp as f64 / FP_PRECISION) * self.learning_rate
    }

    /// Deterministic model certificate hash (anchors to HashTimer)
    pub fn model_hash(&self, round_hash_timer: &str) -> String {
        let serialized = serde_json::to_string(self).unwrap();
        let mut hasher = Sha3_256::new();
        hasher.update(serialized.as_bytes());
        hasher.update(round_hash_timer.as_bytes());
        format!("{:x}", hasher.finalize())
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
#[cfg(feature = "deterministic_math")]
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
                latency_ms: Fixed::from_f64(*latency),
                uptime_pct: Fixed::from_f64(*uptime),
                peer_entropy: Fixed::from_f64(*entropy),
                cpu_usage: None,
                memory_usage: None,
                network_reliability: None,
            }
        })
        .collect()
}

/// Normalize raw telemetry using IPPAN Time median (fallback to f64).
/// Input: map (node_id → (local_time_us, latency_ms, uptime, entropy))
#[cfg(not(feature = "deterministic_math"))]
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
                latency_ms: *latency,
                uptime_pct: *uptime,
                peer_entropy: *entropy,
                cpu_usage: None,
                memory_usage: None,
                network_reliability: None,
            }
        })
        .collect()
}

/// Compute deterministic validator scores
#[cfg(feature = "deterministic_math")]
pub fn compute_scores(
    model: &DeterministicGBDT,
    features: &[ValidatorFeatures],
    round_hash_timer: &str,
) -> HashMap<String, Fixed> {
    let mut scores = HashMap::new();

    for v in features {
        let feature_vector = vec![
            Fixed::from_int(v.delta_time_us),
            v.latency_ms,
            v.uptime_pct,
            v.peer_entropy,
        ];
        let score = model.predict(&feature_vector);
        scores.insert(v.node_id.clone(), score);
    }

    let cert = model.model_hash(round_hash_timer);
    info!("Deterministic GBDT certificate: {}", cert);
    scores
}

/// Compute deterministic validator scores (fallback to f64)
#[cfg(not(feature = "deterministic_math"))]
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

    let cert = model.model_hash(round_hash_timer);
    info!("Deterministic GBDT certificate: {}", cert);
    scores
}

#[cfg(feature = "deterministic_math")]
fn sample_test_model() -> DeterministicGBDT {
    let tree = GBDTTree {
        nodes: vec![
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: Some(1),
                right: Some(2),
                value: None,
            },
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: None,
                right: None,
                value: Some(Fixed::from_f64(0.1)),
            },
            DecisionNode {
                feature: 0,
                threshold: Fixed::ZERO,
                left: None,
                right: None,
                value: Some(Fixed::from_f64(0.2)),
            },
        ],
    };

    DeterministicGBDT {
        trees: vec![tree],
        learning_rate: Fixed::from_f64(0.1),
    }
}

#[cfg(not(feature = "deterministic_math"))]
fn sample_test_model() -> DeterministicGBDT {
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

    #[cfg(feature = "deterministic_math")]
    fn create_test_model() -> DeterministicGBDT {
        let tree = GBDTTree {
            nodes: vec![
                DecisionNode {
                    feature: 0,
                    threshold: Fixed::ZERO,
                    left: Some(1),
                    right: Some(2),
                    value: None,
                },
                DecisionNode {
                    feature: 0,
                    threshold: Fixed::ZERO,
                    left: None,
                    right: None,
                    value: Some(Fixed::from_f64(0.1)),
                },
                DecisionNode {
                    feature: 0,
                    threshold: Fixed::ZERO,
                    left: None,
                    right: None,
                    value: Some(Fixed::from_f64(0.2)),
                },
            ],
        };
        DeterministicGBDT {
            trees: vec![tree],
            learning_rate: Fixed::from_f64(0.1),
        }
    }

    #[cfg(not(feature = "deterministic_math"))]
    fn create_test_model() -> DeterministicGBDT {
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

    #[test]
    #[cfg(feature = "deterministic_math")]
    fn test_prediction_determinism() {
        let model = create_test_model();
        let features = vec![Fixed::from_int(1), Fixed::from_int(2), Fixed::from_int(3), Fixed::from_int(4)];
        let r1 = model.predict(&features);
        let r2 = model.predict(&features);
        assert_eq!(r1, r2);
    }

    #[test]
    #[cfg(not(feature = "deterministic_math"))]
    fn test_prediction_determinism() {
        let model = create_test_model();
        let features = vec![1.0, 2.0, 3.0, 4.0];
        let r1 = model.predict(&features);
        let r2 = model.predict(&features);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_model_hash_consistency() {
        let model = create_test_model();
        let h1 = model.model_hash("round1");
        let h2 = model.model_hash("round1");
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
        let scores = compute_scores(&model, &features, "round_hash");
        assert_eq!(scores.len(), 2);
        assert!(scores.contains_key("node1"));
    }

    #[test]
    #[cfg(feature = "deterministic_math")]
    fn test_model_validation() {
        let valid = create_test_model();
        assert!(valid.validate().is_ok());
        let invalid = DeterministicGBDT {
            trees: vec![],
            learning_rate: Fixed::from_f64(0.1),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    #[cfg(not(feature = "deterministic_math"))]
    fn test_model_validation() {
        let valid = create_test_model();
        assert!(valid.validate().is_ok());
        let invalid = DeterministicGBDT {
            trees: vec![],
            learning_rate: 0.1,
        };
        assert!(invalid.validate().is_err());
    }
}
