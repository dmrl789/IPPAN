//! Deterministic Gradient-Boosted Decision Tree (D-GBDT) fairness engine
//!
//! This module implements a deterministic machine learning model for validator
//! selection and reputation scoring using integer-only arithmetic.

use crate::error::{DlcError, Result};
use ippan_types::currency::denominations;
use ippan_types::Amount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validator metrics used for fairness scoring (deterministic, scaled integers)
/// All percentage/ratio fields are scaled by 10000 (e.g., 10000 = 100%)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidatorMetrics {
    /// Uptime percentage scaled (0-10000 = 0%-100%)
    pub uptime: i64,
    /// Average latency scaled (scaled by 10000)
    pub latency: i64,
    /// Honesty score scaled (0-10000 = 0%-100%)
    pub honesty: i64,
    /// Number of blocks proposed
    pub blocks_proposed: u64,
    /// Number of blocks verified
    pub blocks_verified: u64,
    /// Stake amount
    pub stake: Amount,
    /// Time active in rounds
    pub rounds_active: u64,
}

impl Default for ValidatorMetrics {
    fn default() -> Self {
        Self {
            uptime: 10000,  // 100%
            latency: 0,
            honesty: 10000, // 100%
            blocks_proposed: 0,
            blocks_verified: 0,
            stake: Amount::zero(),
            rounds_active: 0,
        }
    }
}

impl ValidatorMetrics {
    /// Create new validator metrics (scaled integers)
    pub fn new(
        uptime: i64,
        latency: i64,
        honesty: i64,
        blocks_proposed: u64,
        blocks_verified: u64,
        stake: Amount,
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

    // from_floats() removed - use new() with scaled i64 values directly

    /// Update metrics with new data (scaled integer inputs)
    pub fn update(&mut self, uptime_delta: i64, latency_sample: i64, proposed: u64, verified: u64) {
        // Exponential moving average for uptime (integer math)
        self.uptime = (9000 * self.uptime + 1000 * uptime_delta) / 10000;

        // Exponential moving average for latency (integer math)
        self.latency = (9000 * self.latency + 1000 * latency_sample) / 10000;

        self.blocks_proposed += proposed;
        self.blocks_verified += verified;
        self.rounds_active += 1;
    }

    /// Normalize metrics to 0-10000 range (integer arithmetic)
    pub fn to_normalized(&self) -> NormalizedMetrics {
        NormalizedMetrics {
            uptime: self.uptime, // Already scaled
            latency_inv: (10000 - self.latency.min(10000)).max(0), // Invert latency
            honesty: self.honesty, // Already scaled
            proposal_rate: if self.rounds_active > 0 {
                (self.blocks_proposed as i64 * 10000) / self.rounds_active as i64
            } else {
                0
            },
            verification_rate: if self.rounds_active > 0 {
                (self.blocks_verified as i64 * 10000) / self.rounds_active as i64
            } else {
                0
            },
            stake_weight: {
                let stake_micro = self.stake.atomic() / denominations::MICRO_IPN;
                (stake_micro / 1_000_000u128).min(10_000u128) as i64
            },
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

/// Fairness model using ensemble of decision trees (deterministic, integer-only)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FairnessModel {
    /// Feature weights for linear combination (scaled integers, sum to 100 for percentage)
    pub weights: Vec<i64>,
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
        // Default weights (integers summing to 100): uptime, latency, honesty, proposal rate, verification rate, stake
        let weights = vec![25, 15, 25, 15, 15, 5]; // Sum = 100

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
        let weights = vec![25, 15, 25, 15, 15, 5]; // Sum = 100

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

    // score() removed - use score_deterministic() for integer-only arithmetic

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

        // Apply weights (linear combination with integer arithmetic)
        let mut weighted_score = 0i64;
        for (i, &feature) in features.iter().enumerate() {
            if i < self.weights.len() {
                // Integer multiplication: weights sum to 100, so divide by 100
                weighted_score += (feature * self.weights[i]) / 100;
            }
        }

        // Combine tree predictions and weighted features
        score = (score + weighted_score) / 2;

        // Clamp to valid range [0, scale]
        score.max(0).min(self.scale)
    }

    /// Train or update the model with new data (placeholder for future ML training)
    #[deprecated(note = "Training interface not implemented - model is pre-trained")]
    pub fn update(&mut self, _training_data: &[(ValidatorMetrics, i64)]) {
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

/// Validator ranking result (deterministic integer scoring)
#[derive(Debug, Clone)]
pub struct ValidatorRanking {
    pub validator_id: String,
    pub score: i64, // Scaled integer score
    pub rank: usize,
}

/// Rank multiple validators using the fairness model (deterministic integer scoring)
pub fn rank_validators(
    model: &FairnessModel,
    validators: HashMap<String, ValidatorMetrics>,
) -> Vec<ValidatorRanking> {
    let mut rankings: Vec<(String, i64)> = validators
        .into_iter()
        .map(|(id, metrics)| (id, model.score_deterministic(&metrics)))
        .collect();

    // Sort by score (descending), then by ID for deterministic tie-breaking
    rankings.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.cmp(&b.0))
    });

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
#[allow(deprecated)]
mod tests {
    use super::*;
    use ippan_types::Amount;

    #[test]
    fn test_validator_metrics() {
        let metrics = ValidatorMetrics::default();
        assert_eq!(metrics.uptime, 10000); // 100% scaled
        assert_eq!(metrics.honesty, 10000); // 100% scaled
    }

    #[test]
    fn test_metrics_normalization() {
        let metrics = ValidatorMetrics::new(
            9500,  // 0.95 * 10000
            1000,  // 0.1 * 10000
            10000, // 1.0 * 10000
            100,
            500,
            Amount::from_micro_ipn(10_000_000),
            1000,
        );
        let normalized = metrics.to_normalized();

        assert_eq!(normalized.uptime, 9500);
        assert!(normalized.latency_inv > 8000);
    }

    #[test]
    fn test_fairness_model_scoring() {
        let model = FairnessModel::new_default();
        let metrics = ValidatorMetrics::default();

        let score = model.score_deterministic(&metrics);
        assert!(score >= 0 && score <= 10000); // Score is scaled 0-10000
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
            ValidatorMetrics::new(
                9900,  // 0.99 * 10000
                500,   // 0.05 * 10000
                10000, // 1.0 * 10000
                100,
                500,
                Amount::from_micro_ipn(10_000_000),
                100,
            ),
        );
        validators.insert(
            "val2".to_string(),
            ValidatorMetrics::new(
                9500,  // 0.95 * 10000
                1500,  // 0.15 * 10000
                9800,  // 0.98 * 10000
                80,
                400,
                Amount::from_micro_ipn(5_000_000),
                100,
            ),
        );

        let rankings = rank_validators(&model, validators);
        assert_eq!(rankings.len(), 2);
        assert_eq!(rankings[0].rank, 1);
    }

    #[test]
    fn test_deterministic_scoring() {
        let model = FairnessModel::new_production();
        let metrics = ValidatorMetrics::new(
            9900,  // 0.99 * 10000
            1000,  // 0.1 * 10000
            10000, // 1.0 * 10000
            100,
            500,
            Amount::from_micro_ipn(10_000_000),
            100,
        );

        // Score should be deterministic
        let score1 = model.score_deterministic(&metrics);
        let score2 = model.score_deterministic(&metrics);

        assert_eq!(score1, score2);
    }
}
