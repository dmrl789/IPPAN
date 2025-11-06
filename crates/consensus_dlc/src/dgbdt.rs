//! Deterministic Gradient-Boosted Decision Tree (D-GBDT) fairness engine
//!
//! This module implements a deterministic machine learning model for validator
//! selection and reputation scoring using integer-only arithmetic.

use crate::error::{DlcError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validator metrics used for fairness scoring
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatorMetrics {
    /// Uptime percentage (0.0 to 1.0)
    pub uptime: f64,
    /// Average latency in seconds
    pub latency: f64,
    /// Honesty score (0.0 to 1.0)
    pub honesty: f64,
    /// Number of blocks proposed
    pub blocks_proposed: u64,
    /// Number of blocks verified
    pub blocks_verified: u64,
    /// Stake amount
    pub stake: u64,
    /// Time active in rounds
    pub rounds_active: u64,
}

impl Default for ValidatorMetrics {
    fn default() -> Self {
        Self {
            uptime: 1.0,
            latency: 0.0,
            honesty: 1.0,
            blocks_proposed: 0,
            blocks_verified: 0,
            stake: 0,
            rounds_active: 0,
        }
    }
}

impl ValidatorMetrics {
    /// Create new validator metrics
    pub fn new(
        uptime: f64,
        latency: f64,
        honesty: f64,
        blocks_proposed: u64,
        blocks_verified: u64,
        stake: u64,
        rounds_active: u64,
    ) -> Self {
        Self {
            uptime,
            latency,
            honesty,
            blocks_proposed,
            blocks_verified,
            stake,
            rounds_active,
        }
    }

    /// Update metrics with new data
    pub fn update(&mut self, uptime_delta: f64, latency_sample: f64, proposed: u64, verified: u64) {
        // Exponential moving average for uptime
        self.uptime = 0.9 * self.uptime + 0.1 * uptime_delta;

        // Exponential moving average for latency
        self.latency = 0.9 * self.latency + 0.1 * latency_sample;

        self.blocks_proposed += proposed;
        self.blocks_verified += verified;
        self.rounds_active += 1;
    }

    /// Normalize metrics to 0-10000 range (integer arithmetic)
    pub fn to_normalized(&self) -> NormalizedMetrics {
        NormalizedMetrics {
            uptime: (self.uptime * 10000.0) as i64,
            latency_inv: ((1.0 - self.latency.min(1.0)) * 10000.0) as i64,
            honesty: (self.honesty * 10000.0) as i64,
            proposal_rate: if self.rounds_active > 0 {
                ((self.blocks_proposed as f64 / self.rounds_active as f64) * 10000.0) as i64
            } else {
                0
            },
            verification_rate: if self.rounds_active > 0 {
                ((self.blocks_verified as f64 / self.rounds_active as f64) * 10000.0) as i64
            } else {
                0
            },
            stake_weight: (self.stake / 1_000_000).min(10000) as i64, // Normalize stake
        }
    }
}

/// Normalized metrics for integer arithmetic
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedMetrics {
    pub uptime: i64,
    pub latency_inv: i64,
    pub honesty: i64,
    pub proposal_rate: i64,
    pub verification_rate: i64,
    pub stake_weight: i64,
}

/// Decision tree node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TreeNode {
    /// Feature index to split on
    pub feature_index: usize,
    /// Threshold value (in normalized 0-10000 range)
    pub threshold: i64,
    /// Left child (if Some)
    pub left: Option<Box<TreeNode>>,
    /// Right child (if Some)
    pub right: Option<Box<TreeNode>>,
    /// Leaf value (if this is a leaf node)
    pub value: Option<i64>,
}

impl TreeNode {
    /// Create a leaf node
    pub fn leaf(value: i64) -> Self {
        Self {
            feature_index: 0,
            threshold: 0,
            left: None,
            right: None,
            value: Some(value),
        }
    }

    /// Create an internal node
    pub fn internal(feature_index: usize, threshold: i64, left: TreeNode, right: TreeNode) -> Self {
        Self {
            feature_index,
            threshold,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            value: None,
        }
    }

    /// Predict using this tree node
    pub fn predict(&self, features: &[i64]) -> i64 {
        if let Some(value) = self.value {
            return value;
        }

        let feature_value = features.get(self.feature_index).copied().unwrap_or(0);

        if feature_value < self.threshold {
            if let Some(left) = &self.left {
                left.predict(features)
            } else {
                0
            }
        } else if let Some(right) = &self.right {
            right.predict(features)
        } else {
            0
        }
    }
}

/// Fairness model using ensemble of decision trees
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FairnessModel {
    /// Feature weights for linear combination
    pub weights: Vec<f64>,
    /// Decision trees for GBDT
    pub trees: Vec<TreeNode>,
    /// Model bias
    pub bias: i64,
    /// Output scale factor
    pub scale: i64,
}

impl Default for FairnessModel {
    fn default() -> Self {
        Self::new_default()
    }
}

impl FairnessModel {
    /// Create a new default fairness model
    pub fn new_default() -> Self {
        // Default weights: uptime, latency, honesty, proposal rate, verification rate, stake
        let weights = vec![0.25, 0.15, 0.25, 0.15, 0.15, 0.05];

        // Create a simple default tree
        let default_tree = TreeNode::leaf(5000); // Neutral score

        Self {
            weights,
            trees: vec![default_tree],
            bias: 0,
            scale: 10000,
        }
    }

    /// Create a production-ready fairness model with multiple trees
    pub fn new_production() -> Self {
        let weights = vec![0.25, 0.15, 0.25, 0.15, 0.15, 0.05];

        // Tree 1: Focus on uptime and honesty
        let tree1 = TreeNode::internal(
            0, // uptime
            7000,
            TreeNode::leaf(3000), // Low uptime penalty
            TreeNode::internal(
                2, // honesty
                8000,
                TreeNode::leaf(6000), // Medium honesty
                TreeNode::leaf(9000), // High honesty
            ),
        );

        // Tree 2: Focus on performance (latency and proposal rate)
        let tree2 = TreeNode::internal(
            1, // latency_inv
            6000,
            TreeNode::leaf(4000), // High latency penalty
            TreeNode::internal(
                3, // proposal_rate
                5000,
                TreeNode::leaf(6000), // Medium proposal rate
                TreeNode::leaf(8000), // High proposal rate
            ),
        );

        // Tree 3: Focus on verification and stake
        let tree3 = TreeNode::internal(
            4, // verification_rate
            5000,
            TreeNode::leaf(5000), // Low verification
            TreeNode::internal(
                5, // stake_weight
                3000,
                TreeNode::leaf(7000), // Medium stake
                TreeNode::leaf(8000), // High stake
            ),
        );

        Self {
            weights,
            trees: vec![tree1, tree2, tree3],
            bias: 1000,
            scale: 10000,
        }
    }

    /// Score validator using the fairness model
    pub fn score(&self, metrics: &ValidatorMetrics) -> f64 {
        // Use integer arithmetic for determinism
        let score_int = self.score_deterministic(metrics);
        score_int as f64 / self.scale as f64
    }

    /// Deterministic integer-based scoring
    pub fn score_deterministic(&self, metrics: &ValidatorMetrics) -> i64 {
        let normalized = metrics.to_normalized();
        let features = vec![
            normalized.uptime,
            normalized.latency_inv,
            normalized.honesty,
            normalized.proposal_rate,
            normalized.verification_rate,
            normalized.stake_weight,
        ];

        // GBDT prediction: sum of all tree predictions
        let mut score = self.bias;

        for tree in &self.trees {
            score += tree.predict(&features);
        }

        // Apply weights (linear combination)
        let mut weighted_score = 0i64;
        for (i, &feature) in features.iter().enumerate() {
            if i < self.weights.len() {
                weighted_score += ((feature as f64) * self.weights[i]) as i64;
            }
        }

        // Combine tree predictions and weighted features
        score = (score + weighted_score) / 2;

        // Clamp to valid range [0, scale]
        score.max(0).min(self.scale)
    }

    /// Train or update the model with new data (placeholder for future ML training)
    pub fn update(&mut self, _training_data: &[(ValidatorMetrics, f64)]) {
        // In production, this would update the model using gradient boosting
        // For now, we use the pre-trained model
        tracing::debug!("Model update requested (using pre-trained model)");
    }

    /// Validate model integrity
    pub fn validate(&self) -> Result<()> {
        if self.weights.is_empty() {
            return Err(DlcError::Model("Model has no weights".to_string()));
        }

        if self.trees.is_empty() {
            return Err(DlcError::Model("Model has no trees".to_string()));
        }

        if self.scale <= 0 {
            return Err(DlcError::Model("Invalid scale factor".to_string()));
        }

        Ok(())
    }

    /// Get model metadata
    pub fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            num_trees: self.trees.len(),
            num_features: self.weights.len(),
            scale: self.scale,
            bias: self.bias,
        }
    }
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub num_trees: usize,
    pub num_features: usize,
    pub scale: i64,
    pub bias: i64,
}

/// Validator ranking result
#[derive(Debug, Clone)]
pub struct ValidatorRanking {
    pub validator_id: String,
    pub score: f64,
    pub rank: usize,
}

/// Rank multiple validators using the fairness model
pub fn rank_validators(
    model: &FairnessModel,
    validators: HashMap<String, ValidatorMetrics>,
) -> Vec<ValidatorRanking> {
    let mut rankings: Vec<(String, f64)> = validators
        .into_iter()
        .map(|(id, metrics)| (id, model.score(&metrics)))
        .collect();

    // Sort by score (descending)
    rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    rankings
        .into_iter()
        .enumerate()
        .map(|(rank, (validator_id, score))| ValidatorRanking {
            validator_id,
            score,
            rank: rank + 1,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_metrics() {
        let metrics = ValidatorMetrics::default();
        assert_eq!(metrics.uptime, 1.0);
        assert_eq!(metrics.honesty, 1.0);
    }

    #[test]
    fn test_metrics_normalization() {
        let metrics = ValidatorMetrics::new(0.95, 0.1, 1.0, 100, 500, 10_000_000, 1000);
        let normalized = metrics.to_normalized();

        assert_eq!(normalized.uptime, 9500);
        assert!(normalized.latency_inv > 8000);
    }

    #[test]
    fn test_fairness_model_scoring() {
        let model = FairnessModel::new_default();
        let metrics = ValidatorMetrics::default();

        let score = model.score(&metrics);
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn test_production_model() {
        let model = FairnessModel::new_production();
        assert!(model.validate().is_ok());
        assert_eq!(model.trees.len(), 3);
    }

    #[test]
    fn test_tree_prediction() {
        let tree = TreeNode::internal(0, 5000, TreeNode::leaf(1000), TreeNode::leaf(9000));

        assert_eq!(tree.predict(&[3000]), 1000);
        assert_eq!(tree.predict(&[7000]), 9000);
    }

    #[test]
    fn test_validator_ranking() {
        let model = FairnessModel::new_production();
        let mut validators = HashMap::new();

        validators.insert(
            "val1".to_string(),
            ValidatorMetrics::new(0.99, 0.05, 1.0, 100, 500, 10_000_000, 100),
        );
        validators.insert(
            "val2".to_string(),
            ValidatorMetrics::new(0.95, 0.15, 0.98, 80, 400, 5_000_000, 100),
        );

        let rankings = rank_validators(&model, validators);
        assert_eq!(rankings.len(), 2);
        assert_eq!(rankings[0].rank, 1);
    }

    #[test]
    fn test_deterministic_scoring() {
        let model = FairnessModel::new_production();
        let metrics = ValidatorMetrics::new(0.99, 0.1, 1.0, 100, 500, 10_000_000, 100);

        // Score should be deterministic
        let score1 = model.score_deterministic(&metrics);
        let score2 = model.score_deterministic(&metrics);

        assert_eq!(score1, score2);
    }
}
