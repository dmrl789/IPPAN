//! Deterministic Gradient-Boosted Decision Tree (GBDT) inference
//! Anchored to IPPAN Time and HashTimer for consensus-safe validator scoring.
//!
//! Ensures identical predictions, rankings, and hashes across all validator nodes.

use crate::fixed::Fixed;
use crate::serialization::canonical_json_string;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha3::{Digest, Sha3_256};
use std::{collections::HashMap, fs, path::Path};
use tracing::{info, warn};

fn deserialize_fixed<'de, D>(deserializer: D) -> Result<Fixed, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    if deserializer.is_human_readable() {
        let value = Value::deserialize(deserializer)?;
        value_to_fixed(&value).map_err(D::Error::custom)
    } else {
        Fixed::deserialize(deserializer).map_err(D::Error::custom)
    }
}

fn deserialize_option_fixed<'de, D>(deserializer: D) -> Result<Option<Fixed>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    if deserializer.is_human_readable() {
        let value = Option::<Value>::deserialize(deserializer)?;
        match value {
            Some(val) => value_to_fixed(&val).map(Some).map_err(D::Error::custom),
            None => Ok(None),
        }
    } else {
        Option::<Fixed>::deserialize(deserializer).map_err(D::Error::custom)
    }
}

fn value_to_fixed(value: &Value) -> Result<Fixed, String> {
    match value {
        Value::Number(num) => {
            if let Some(int) = num.as_i64() {
                Ok(Fixed::from_micro(int))
            } else if let Some(uint) = num.as_u64() {
                if uint > i64::MAX as u64 {
                    Err(format!("numeric value {num} exceeds i64 range"))
                } else {
                    Ok(Fixed::from_micro(uint as i64))
                }
            } else {
                Fixed::from_decimal_str(&num.to_string())
                    .ok_or_else(|| format!("unable to parse decimal {num}"))
            }
        }
        Value::String(s) => {
            Fixed::from_decimal_str(s).ok_or_else(|| format!("unable to parse decimal string {s}"))
        }
        _ => Err(format!("expected number, found {value}")),
    }
}

/// Normalized validator telemetry (anchored to IPPAN Time)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatorFeatures {
    pub node_id: String,
    pub delta_time_us: i64, // deviation from IPPAN Time median (µs)
    #[serde(deserialize_with = "deserialize_fixed")]
    pub latency_ms: Fixed,
    #[serde(deserialize_with = "deserialize_fixed")]
    pub uptime_pct: Fixed,
    #[serde(deserialize_with = "deserialize_fixed")]
    pub peer_entropy: Fixed,
    #[serde(default, deserialize_with = "deserialize_option_fixed")]
    pub cpu_usage: Option<Fixed>,
    #[serde(default, deserialize_with = "deserialize_option_fixed")]
    pub memory_usage: Option<Fixed>,
    #[serde(default, deserialize_with = "deserialize_option_fixed")]
    pub network_reliability: Option<Fixed>,
}

/// Decision node in a GBDT tree
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecisionNode {
    pub feature: usize,
    #[serde(deserialize_with = "deserialize_fixed")]
    pub threshold: Fixed,
    pub left: Option<usize>,
    pub right: Option<usize>,
    #[serde(default, deserialize_with = "deserialize_option_fixed")]
    pub value: Option<Fixed>,
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
    #[serde(deserialize_with = "deserialize_fixed")]
    pub learning_rate: Fixed,
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

pub fn normalize_features(
    telemetry: &HashMap<String, (i64, Fixed, Fixed, Fixed)>,
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

    if let Ok(cert) = model.model_hash(round_hash_timer) {
        info!("Deterministic GBDT certificate: {}", cert);
    }

    scores
}

// ---------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------

#[cfg(any(test, feature = "enable-tests"))]
impl DeterministicGBDT {
    /// Creates a deterministic test model for use in integration tests and examples.
    pub fn create_test_model() -> Self {
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
            learning_rate: Fixed::from_f64(0.1),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_model_hash_consistency_fixed() {
        let model = DeterministicGBDT {
            trees: vec![GBDTTree {
                nodes: vec![DecisionNode {
                    feature: 0,
                    threshold: Fixed::ZERO,
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

    #[test]
    fn test_json_integers_are_treated_as_micro_units() {
        let data = json!({
            "node_id": "validator-alpha",
            "delta_time_us": 0,
            "latency_ms": 1_500, // 1.5 ms expressed in micro-units
            "uptime_pct": 999_000, // 99.9% expressed in micro-units
            "peer_entropy": 500_000,
            "cpu_usage": 250_000,
        });

        let features: ValidatorFeatures =
            serde_json::from_value(data).expect("valid features json");

        assert_eq!(features.latency_ms, Fixed::from_micro(1_500));
        assert_eq!(features.uptime_pct, Fixed::from_micro(999_000));
        assert_eq!(features.peer_entropy, Fixed::from_micro(500_000));
        assert_eq!(
            features.cpu_usage.expect("cpu_usage should be present"),
            Fixed::from_micro(250_000)
        );
    }
}
