//! Gradient Boosted Decision Tree (GBDT) trainer
//!
//! Implements deterministic GBDT training with fixed-point arithmetic
//! and exact-greedy CART splits.

use anyhow::Result;
use ippan_ai_core::gbdt::{GBDTModel, ModelMetadata, SecurityConstraints, Tree};
use std::collections::HashMap;

use crate::cart::{CartBuilder, TreeConfig};
use crate::dataset::Dataset;

/// GBDT training configuration
#[derive(Clone, Debug)]
pub struct GbdtConfig {
    pub num_trees: usize,
    pub max_depth: usize,
    pub min_samples_leaf: usize,
    pub learning_rate: i64, // Fixed-point, e.g., 100_000 = 0.1
    pub quant_step: i64,
    pub scale: i32,
}

impl Default for GbdtConfig {
    fn default() -> Self {
        Self {
            num_trees: 64,
            max_depth: 6,
            min_samples_leaf: 32,
            learning_rate: 100_000, // 0.1 in fixed-point
            quant_step: 1000,
            scale: 10_000,
        }
    }
}

/// GBDT trainer
pub struct GbdtTrainer {
    config: GbdtConfig,
}

impl GbdtTrainer {
    pub fn new(config: GbdtConfig) -> Self {
        Self { config }
    }

    /// Train a GBDT model on the given dataset
    pub fn train(&self, dataset: &Dataset) -> Result<GBDTModel> {
        let n_samples = dataset.len();
        let feature_count = dataset.feature_count;

        // Initialize predictions with zeros (bias will be calculated)
        let mut predictions = vec![0i64; n_samples];

        // Calculate initial bias (mean of targets)
        let bias = self.calculate_bias(&dataset.targets);

        // Initialize predictions with bias
        for pred in &mut predictions {
            *pred = bias as i64;
        }

        // Train trees
        let mut trees = Vec::with_capacity(self.config.num_trees);

        for tree_idx in 0..self.config.num_trees {
            tracing::info!("Training tree {}/{}", tree_idx + 1, self.config.num_trees);

            // Calculate gradients and hessians
            let (gradients, hessians) = self.calculate_gradients_hessians(&dataset.targets, &predictions);

            // Build tree
            let tree_config = TreeConfig {
                max_depth: self.config.max_depth,
                min_samples_leaf: self.config.min_samples_leaf,
                quant_step: self.config.quant_step,
            };

            let builder = CartBuilder::new(&dataset.features, &gradients, &hessians, tree_config);
            let tree = builder.build();

            // Update predictions with tree output scaled by learning rate
            self.update_predictions(&tree, &dataset.features, &mut predictions);

            trees.push(tree);
        }

        // Create metadata
        let metadata = ModelMetadata {
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
            feature_count,
            tree_count: trees.len(),
            max_depth: self.config.max_depth,
            model_hash: GBDTModel::calculate_model_hash(&trees, bias, self.config.scale)?,
            training_data_hash: String::new(),
            performance_metrics: HashMap::new(),
        };

        // Create model
        let model = GBDTModel {
            trees,
            bias,
            scale: self.config.scale,
            metadata,
            feature_normalization: None,
            security_constraints: SecurityConstraints::default(),
            metrics: Default::default(),
            evaluation_cache: HashMap::new(),
        };

        Ok(model)
    }

    /// Calculate initial bias (mean of targets)
    fn calculate_bias(&self, targets: &[i64]) -> i32 {
        if targets.is_empty() {
            return 0;
        }

        let sum: i128 = targets.iter().map(|&t| t as i128).sum();
        let mean = (sum / targets.len() as i128) as i64;

        mean.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }

    /// Calculate gradients and hessians for regression (MSE loss)
    /// gradient = prediction - target
    /// hessian = 1 (constant for MSE)
    fn calculate_gradients_hessians(&self, targets: &[i64], predictions: &[i64]) -> (Vec<i64>, Vec<i64>) {
        let n = targets.len();
        let mut gradients = Vec::with_capacity(n);
        let mut hessians = Vec::with_capacity(n);

        for i in 0..n {
            // Gradient of MSE: 2 * (pred - target)
            // We use (pred - target) and scale later
            let grad = predictions[i].saturating_sub(targets[i]);
            gradients.push(grad);

            // Hessian is constant 1 for MSE, scaled to match gradients
            hessians.push(1000);
        }

        (gradients, hessians)
    }

    /// Update predictions with tree output
    fn update_predictions(&self, tree: &Tree, features: &[Vec<i64>], predictions: &mut [i64]) {
        for (i, feature_vec) in features.iter().enumerate() {
            let tree_value = self.evaluate_tree(tree, feature_vec);
            
            // Apply learning rate: value * learning_rate / 1_000_000
            let scaled_value = ((tree_value as i128 * self.config.learning_rate as i128) / 1_000_000) as i64;
            
            predictions[i] = predictions[i].saturating_add(scaled_value);
        }
    }

    /// Evaluate a single tree on a feature vector
    fn evaluate_tree(&self, tree: &Tree, features: &[i64]) -> i64 {
        let mut idx = 0usize;

        loop {
            if idx >= tree.nodes.len() {
                return 0;
            }

            let node = &tree.nodes[idx];

            if let Some(value) = node.value {
                return value as i64;
            }

            let feature_idx = node.feature_index as usize;
            if feature_idx >= features.len() {
                return 0;
            }

            idx = if features[feature_idx] <= node.threshold {
                node.left as usize
            } else {
                node.right as usize
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_dataset() -> Dataset {
        Dataset {
            features: vec![
                vec![100_000, 200_000],
                vec![200_000, 300_000],
                vec![300_000, 400_000],
                vec![400_000, 500_000],
            ],
            targets: vec![1_000_000, 2_000_000, 3_000_000, 4_000_000],
            feature_count: 2,
        }
    }

    #[test]
    fn test_train_simple_model() -> Result<()> {
        let dataset = create_simple_dataset();
        let config = GbdtConfig {
            num_trees: 4,
            max_depth: 2,
            min_samples_leaf: 1,
            learning_rate: 100_000,
            quant_step: 50_000,
            scale: 10_000,
        };

        let trainer = GbdtTrainer::new(config);
        let model = trainer.train(&dataset)?;

        assert_eq!(model.trees.len(), 4);
        assert_eq!(model.metadata.feature_count, 2);
        assert_eq!(model.scale, 10_000);

        Ok(())
    }

    #[test]
    fn test_bias_calculation() {
        let trainer = GbdtTrainer::new(GbdtConfig::default());
        let targets = vec![1_000_000, 2_000_000, 3_000_000];
        let bias = trainer.calculate_bias(&targets);
        
        // Mean should be 2_000_000
        assert_eq!(bias, 2_000_000);
    }

    #[test]
    fn test_determinism() -> Result<()> {
        let dataset = create_simple_dataset();
        let config = GbdtConfig {
            num_trees: 2,
            max_depth: 2,
            min_samples_leaf: 1,
            learning_rate: 100_000,
            quant_step: 50_000,
            scale: 10_000,
        };

        let trainer = GbdtTrainer::new(config.clone());
        let model1 = trainer.train(&dataset)?;
        
        let trainer2 = GbdtTrainer::new(config);
        let model2 = trainer2.train(&dataset)?;

        // Models should be identical
        assert_eq!(model1.bias, model2.bias);
        assert_eq!(model1.trees.len(), model2.trees.len());
        
        for (t1, t2) in model1.trees.iter().zip(model2.trees.iter()) {
            assert_eq!(t1.nodes.len(), t2.nodes.len());
        }

        Ok(())
    }
}
