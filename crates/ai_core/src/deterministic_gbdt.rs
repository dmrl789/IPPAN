//! Deterministic Gradient-Boosted Decision Tree (GBDT) inference
//! Anchored to IPPAN Time and HashTimer for consensus-safe validator scoring.
//!
//! Ensures identical predictions, rankings, and hashes across all validator nodes.

#[cfg(feature = "deterministic_math")]
use crate::fixed::Fixed;
use crate::serialization::canonical_json_string;
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
        let json = canonical_json_string(self)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))?;
        fs::write(path, json)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))?;
        Ok(())
    }

    pub fn to_canonical_json(&self) -> Result<String, DeterministicGBDTError> {
        canonical_json_string(self)
            .map_err(|e| DeterministicGBDTError::SerializationError(e.to_string()))
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

    #[cfg(not(feature = "deterministic_math"))]
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut score = 0.0;

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

    /// Deterministic model certificate hash (anchors to HashTimer)
    pub fn model_hash(&self, round_hash_timer: &str) -> Result<String, DeterministicGBDTError> {
        let serialized = self.to_canonical_json()?;
        let mut hasher = Sha3_256::new();
        hasher.update(serialized.as_bytes());
        hasher.update(round_hash_timer.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

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

    let cert = model.model_hash(round_hash_timer).unwrap();
    info!("Deterministic GBDT certificate: {}", cert);
    scores
}

#[cfg(not(feature = "deterministic_math"))]
pub fn compute_scores(
    model: &DeterministicGBDT,
    features: &[ValidatorFeatures],
    round_hash_timer: &str,
) -> Result<HashMap<String, f64>, DeterministicGBDTError> {
    let mut scores = HashMap::new();

    for v in features {
        let feature_vector = [
            v.delta_time_us as f64,
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

// ---------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------

#[cfg(any(test, feature = "enable-tests"))]
impl DeterministicGBDT {
    /// Creates a deterministic test model for use in integration tests and examples.
    pub fn create_test_model() -> Self {
        #[cfg(feature = "deterministic_math")]
        {
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
                        value: Some(Fixed::from_f64(-0.05)),
                    },
                ],
            };

            Self {
                trees: vec![tree],
                learning_rate: Fixed::from_f64(1.0),
            }
        }

        #[cfg(not(feature = "deterministic_math"))]
        {
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
                        value: Some(-0.05),
                    },
                ],
            };

            Self {
                trees: vec![tree],
                learning_rate: 0.1,
            }
        }
    }
}

#[cfg(any(test, feature = "enable-tests"))]
pub fn create_test_model() -> DeterministicGBDT {
    DeterministicGBDT::create_test_model()
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

#[cfg(all(test, feature = "deterministic_math"))]
mod tests {
    use super::*;

    #[test]
    fn test_model_hash_consistency() {
        let model = DeterministicGBDT {
            trees: vec![GBDTTree {
                nodes: vec![DecisionNode {
                    feature: 0,
                    threshold: Fixed::from_f64(0.0),
                    left: None,
                    right: None,
                    value: Some(Fixed::from_f64(0.1)),
                }],
            }],
            learning_rate: Fixed::from_f64(0.1),
        };
        let h1 = model.model_hash("round1").unwrap();
        let h2 = model.model_hash("round1").unwrap();
        assert_eq!(h1, h2);
    }
}
