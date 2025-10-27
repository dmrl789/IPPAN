//! Deterministic Gradient-Boosted Decision Tree inference
//! Anchored to IPPAN Time and HashTimer
//!
//! Ensures that every node in the network computes identical
//! predictions and validator rankings within a round.

use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{collections::HashMap, fs, path::Path};

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

impl DeterministicGBDT {
    /// Load model from JSON file (shared by all validators)
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&data)?)
    }

    /// Load model from binary file encoded with bincode
    pub fn from_bin_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let bytes = fs::read(path)?;
        Ok(bincode::deserialize(&bytes)?)
    }

    /// Load model inferring format from file extension (json|bin)
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let p = path.as_ref();
        match p.extension().and_then(|s| s.to_str()) {
            Some("json") => Self::from_json_file(p),
            Some("bin") => Self::from_bin_file(p),
            _ => Self::from_json_file(p),
        }
    }

    /// Deterministic prediction using fixed-point math
    pub fn predict(&self, features: &[f64]) -> f64 {
        let mut score_fp: i64 = 0;
        for tree in &self.trees {
            let mut node_idx = 0;
            loop {
                let node = &tree.nodes[node_idx];
                if let Some(value) = node.value {
                    score_fp += (value * FP_PRECISION) as i64;
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
        let x = vec![
            v.delta_time_us as f64,
            v.latency_ms,
            v.uptime_pct,
            v.peer_entropy,
        ];
        let y = model.predict(&x);
        scores.insert(v.node_id.clone(), y);
    }

    // Generate reproducible hash certificate
    let cert = model.model_hash(round_hash_timer);
    tracing::info!("Deterministic GBDT certificate: {}", cert);
    scores
}
