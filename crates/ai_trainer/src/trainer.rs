//! Gradient Boosted Decision Tree (GBDT) trainer
//!
//! Implements deterministic GBDT training with fixed-point arithmetic
//! and exact-greedy CART splits that target the runtime `ippan-ai-core`
//! integer model format.

use anyhow::Result;
use ippan_ai_core::gbdt::{Model, Tree, SCALE};

use crate::cart::{CartBuilder, TreeConfig};
use crate::dataset::Dataset;

/// Training configuration for deterministic GBDT.
#[derive(Clone, Debug)]
pub struct TrainingParams {
    pub tree_count: usize,
    pub max_depth: usize,
    pub min_samples_leaf: usize,
    pub learning_rate_micro: i64,
    pub quantization_step: i64,
}

impl Default for TrainingParams {
    fn default() -> Self {
        Self {
            tree_count: 32,
            max_depth: 4,
            min_samples_leaf: 8,
            learning_rate_micro: 100_000, // 0.1
            quantization_step: 10_000,
        }
    }
}

/// Deterministic trainer that produces integer-only runtime models.
pub struct GbdtTrainer {
    config: TrainingParams,
}

impl GbdtTrainer {
    pub fn new(config: TrainingParams) -> Self {
        Self { config }
    }

    /// Train a deterministic model from a dataset.
    pub fn train(&self, dataset: &Dataset) -> Result<Model> {
        let n_samples = dataset.len();
        let mut predictions = vec![0i64; n_samples];

        let bias = self.calculate_bias(&dataset.targets);
        for pred in &mut predictions {
            *pred = bias;
        }

        let mut trees = Vec::with_capacity(self.config.tree_count);

        for tree_idx in 0..self.config.tree_count {
            tracing::info!(training_tree = tree_idx + 1, total = self.config.tree_count);

            let (gradients, hessians) =
                self.calculate_gradients_hessians(&dataset.targets, &predictions);

            let tree_config = TreeConfig {
                max_depth: self.config.max_depth,
                min_samples_leaf: self.config.min_samples_leaf,
                quant_step: self.config.quantization_step,
            };

            let builder = CartBuilder::new(&dataset.features, &gradients, &hessians, tree_config);
            let nodes = builder.build();

            self.update_predictions(&nodes, &dataset.features, &mut predictions);

            trees.push(Tree::new(nodes, self.config.learning_rate_micro));
        }

        Ok(Model::new(trees, bias))
    }

    fn calculate_bias(&self, targets: &[i64]) -> i64 {
        if targets.is_empty() {
            return 0;
        }

        let sum: i128 = targets.iter().map(|&t| t as i128).sum();
        (sum / targets.len() as i128) as i64
    }

    fn calculate_gradients_hessians(
        &self,
        targets: &[i64],
        predictions: &[i64],
    ) -> (Vec<i64>, Vec<i64>) {
        let mut gradients = Vec::with_capacity(targets.len());
        let mut hessians = Vec::with_capacity(targets.len());

        for i in 0..targets.len() {
            gradients.push(predictions[i].saturating_sub(targets[i]));
            hessians.push(1); // unit hessian for MSE
        }

        (gradients, hessians)
    }

    fn update_predictions(
        &self,
        nodes: &[ippan_ai_core::gbdt::tree::Node],
        features: &[Vec<i64>],
        predictions: &mut [i64],
    ) {
        for (idx, feature_vec) in features.iter().enumerate() {
            let value = self.evaluate_nodes(nodes, feature_vec);
            let scaled =
                ((value as i128 * self.config.learning_rate_micro as i128) / SCALE as i128) as i64;
            predictions[idx] = predictions[idx].saturating_add(scaled);
        }
    }

    fn evaluate_nodes(&self, nodes: &[ippan_ai_core::gbdt::tree::Node], features: &[i64]) -> i64 {
        if nodes.is_empty() {
            return 0;
        }

        let mut idx = 0usize;
        loop {
            if idx >= nodes.len() {
                return 0;
            }

            let node = &nodes[idx];
            if let Some(value) = node.leaf {
                return value;
            }

            let feature_idx = node.feature_idx as usize;
            if feature_idx >= features.len() {
                return 0;
            }

            idx = if features[feature_idx] <= node.threshold {
                node.left.max(0) as usize
            } else {
                node.right.max(0) as usize
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dataset() -> Dataset {
        Dataset {
            features: vec![
                vec![100, 10, 5, 0, 500],
                vec![200, 15, 6, 1, 500],
                vec![300, 20, 7, 1, 600],
                vec![400, 25, 8, 2, 600],
            ],
            targets: vec![1_000, 1_500, 2_000, 2_500],
            feature_count: 5,
            feature_names: FEATURE_COLUMNS.iter().map(|s| s.to_string()).collect(),
        }
    }

    use crate::dataset::FEATURE_COLUMNS;

    #[test]
    fn test_bias_calculation() {
        let dataset = sample_dataset();
        let trainer = GbdtTrainer::new(TrainingParams::default());
        let bias = trainer.calculate_bias(&dataset.targets);
        assert_eq!(bias, 1_750);
    }

    #[test]
    fn test_training_determinism() -> Result<()> {
        let dataset = sample_dataset();
        let params = TrainingParams {
            tree_count: 3,
            max_depth: 2,
            min_samples_leaf: 1,
            learning_rate_micro: 200_000,
            quantization_step: 10,
        };

        let trainer = GbdtTrainer::new(params.clone());
        let model_a = trainer.train(&dataset)?;
        let trainer2 = GbdtTrainer::new(params);
        let model_b = trainer2.train(&dataset)?;

        assert_eq!(model_a, model_b);
        Ok(())
    }
}
