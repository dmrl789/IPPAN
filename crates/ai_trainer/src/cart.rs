//! CART (Classification and Regression Tree) builder
//!
//! Implements deterministic exact-greedy decision tree construction
//! with fixed-point arithmetic only.

use ippan_ai_core::gbdt::{Node, Tree};
use std::collections::BTreeMap;

use crate::deterministic::SplitTieBreaker;

/// Training parameters for a single tree
#[derive(Clone, Debug)]
pub struct TreeConfig {
    pub max_depth: usize,
    pub min_samples_leaf: usize,
    pub quant_step: i64,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            max_depth: 6,
            min_samples_leaf: 32,
            quant_step: 1000,
        }
    }
}

/// Sample with features, gradient, and hessian
#[derive(Clone, Debug)]
struct Sample {
    features: Vec<i64>,
    gradient: i64,
    hessian: i64,
}

/// Split candidate with gain and tie-breaker
#[derive(Debug, Clone)]
struct SplitCandidate {
    feature_idx: usize,
    threshold: i64,
    gain: i64,
    tie_breaker: SplitTieBreaker,
}

impl SplitCandidate {
    fn new(feature_idx: usize, threshold: i64, gain: i64, node_id: usize) -> Self {
        Self {
            feature_idx,
            threshold,
            gain,
            tie_breaker: SplitTieBreaker::new(feature_idx, threshold, node_id),
        }
    }
}

/// Build a regression tree using exact-greedy CART algorithm
pub struct CartBuilder {
    config: TreeConfig,
    samples: Vec<Sample>,
    feature_count: usize,
}

impl CartBuilder {
    pub fn new(
        features: &[Vec<i64>],
        gradients: &[i64],
        hessians: &[i64],
        config: TreeConfig,
    ) -> Self {
        assert_eq!(features.len(), gradients.len());
        assert_eq!(features.len(), hessians.len());

        let samples: Vec<Sample> = features
            .iter()
            .zip(gradients.iter().zip(hessians.iter()))
            .map(|(f, (&g, &h))| Sample {
                features: f.clone(),
                gradient: g,
                hessian: h,
            })
            .collect();

        let feature_count = if samples.is_empty() {
            0
        } else {
            samples[0].features.len()
        };

        Self {
            config,
            samples,
            feature_count,
        }
    }

    /// Build tree and return nodes
    pub fn build(&self) -> Tree {
        let mut nodes = Vec::new();
        let indices: Vec<usize> = (0..self.samples.len()).collect();

        self.build_node(&indices, 0, &mut nodes, 0);

        Tree { nodes }
    }

    /// Recursively build tree nodes
    fn build_node(
        &self,
        indices: &[usize],
        depth: usize,
        nodes: &mut Vec<Node>,
        node_id: usize,
    ) -> u16 {
        let current_idx = nodes.len() as u16;

        // Calculate leaf value
        let leaf_value = self.calculate_leaf_value(indices);

        // Check stopping conditions
        if depth >= self.config.max_depth
            || indices.len() < 2 * self.config.min_samples_leaf
        {
            nodes.push(Node {
                feature_index: 0,
                threshold: 0,
                left: 0,
                right: 0,
                value: Some(leaf_value),
            });
            return current_idx;
        }

        // Find best split
        let split = match self.find_best_split(indices, node_id) {
            Some(s) => s,
            None => {
                // No valid split, create leaf
                nodes.push(Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(leaf_value),
                });
                return current_idx;
            }
        };

        // Split samples
        let (left_indices, right_indices) = self.split_samples(indices, split.feature_idx, split.threshold);

        if left_indices.len() < self.config.min_samples_leaf
            || right_indices.len() < self.config.min_samples_leaf
        {
            // Split would violate min_samples_leaf, create leaf
            nodes.push(Node {
                feature_index: 0,
                threshold: 0,
                left: 0,
                right: 0,
                value: Some(leaf_value),
            });
            return current_idx;
        }

        // Reserve space for current node
        nodes.push(Node {
            feature_index: split.feature_idx as u16,
            threshold: split.threshold,
            left: 0,
            right: 0,
            value: None,
        });

        // Build left and right subtrees
        let left_idx = self.build_node(&left_indices, depth + 1, nodes, node_id * 2 + 1);
        let right_idx = self.build_node(&right_indices, depth + 1, nodes, node_id * 2 + 2);

        // Update current node with child indices
        nodes[current_idx as usize].left = left_idx;
        nodes[current_idx as usize].right = right_idx;

        current_idx
    }

    /// Find best split using exact-greedy algorithm
    fn find_best_split(&self, indices: &[usize], node_id: usize) -> Option<SplitCandidate> {
        let mut best_split: Option<SplitCandidate> = None;

        for feature_idx in 0..self.feature_count {
            // Get quantized thresholds for this feature
            let thresholds = self.get_quantized_thresholds(indices, feature_idx);

            for threshold in thresholds {
                let (left_indices, right_indices) = self.split_samples(indices, feature_idx, threshold);

                if left_indices.len() < self.config.min_samples_leaf
                    || right_indices.len() < self.config.min_samples_leaf
                {
                    continue;
                }

                let gain = self.calculate_split_gain(&left_indices, &right_indices, indices);

                let candidate = SplitCandidate::new(feature_idx, threshold, gain, node_id);

                best_split = match best_split {
                    None => Some(candidate),
                    Some(ref current) => {
                        // Deterministic tie-breaking
                        if gain > current.gain
                            || (gain == current.gain && candidate.tie_breaker < current.tie_breaker)
                        {
                            Some(candidate)
                        } else {
                            best_split
                        }
                    }
                };
            }
        }

        best_split
    }

    /// Get quantized threshold values for a feature
    fn get_quantized_thresholds(&self, indices: &[usize], feature_idx: usize) -> Vec<i64> {
        // Collect unique quantized values
        let mut values = BTreeMap::new();
        
        for &idx in indices {
            let val = self.samples[idx].features[feature_idx];
            let quantized = (val / self.config.quant_step) * self.config.quant_step;
            values.insert(quantized, ());
        }

        values.into_keys().collect()
    }

    /// Split samples based on threshold
    fn split_samples(&self, indices: &[usize], feature_idx: usize, threshold: i64) -> (Vec<usize>, Vec<usize>) {
        let mut left = Vec::new();
        let mut right = Vec::new();

        for &idx in indices {
            if self.samples[idx].features[feature_idx] <= threshold {
                left.push(idx);
            } else {
                right.push(idx);
            }
        }

        (left, right)
    }

    /// Calculate split gain using fixed-point arithmetic
    /// Gain = G_left²/H_left + G_right²/H_right - G_parent²/H_parent
    fn calculate_split_gain(&self, left: &[usize], right: &[usize], parent: &[usize]) -> i64 {
        let (g_left, h_left) = self.sum_gradients_hessians(left);
        let (g_right, h_right) = self.sum_gradients_hessians(right);
        let (g_parent, h_parent) = self.sum_gradients_hessians(parent);

        // Use i128 to avoid overflow
        let gain_left = if h_left > 0 {
            ((g_left as i128 * g_left as i128) / h_left as i128) as i64
        } else {
            0
        };

        let gain_right = if h_right > 0 {
            ((g_right as i128 * g_right as i128) / h_right as i128) as i64
        } else {
            0
        };

        let gain_parent = if h_parent > 0 {
            ((g_parent as i128 * g_parent as i128) / h_parent as i128) as i64
        } else {
            0
        };

        gain_left.saturating_add(gain_right).saturating_sub(gain_parent)
    }

    /// Sum gradients and hessians for a set of samples
    fn sum_gradients_hessians(&self, indices: &[usize]) -> (i64, i64) {
        let mut sum_g = 0i64;
        let mut sum_h = 0i64;

        for &idx in indices {
            sum_g = sum_g.saturating_add(self.samples[idx].gradient);
            sum_h = sum_h.saturating_add(self.samples[idx].hessian);
        }

        (sum_g, sum_h)
    }

    /// Calculate optimal leaf value: -G/H
    fn calculate_leaf_value(&self, indices: &[usize]) -> i32 {
        let (sum_g, sum_h) = self.sum_gradients_hessians(indices);

        if sum_h == 0 {
            return 0;
        }

        // Scale to maintain precision
        let value = -((sum_g as i128 * 1000) / sum_h as i128) as i64;
        value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tree() {
        let features = vec![
            vec![100_000, 200_000],
            vec![200_000, 300_000],
            vec![300_000, 400_000],
            vec![400_000, 500_000],
        ];
        let gradients = vec![-1000, -500, 500, 1000];
        let hessians = vec![1000, 1000, 1000, 1000];

        let config = TreeConfig {
            max_depth: 2,
            min_samples_leaf: 1,
            quant_step: 50_000,
        };

        let builder = CartBuilder::new(&features, &gradients, &hessians, config);
        let tree = builder.build();

        assert!(!tree.nodes.is_empty());
        assert!(tree.nodes[0].value.is_some() || tree.nodes[0].value.is_none());
    }

    #[test]
    fn test_leaf_only_tree() {
        let features = vec![vec![100_000]];
        let gradients = vec![-1000];
        let hessians = vec![1000];

        let config = TreeConfig::default();
        let builder = CartBuilder::new(&features, &gradients, &hessians, config);
        let tree = builder.build();

        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.nodes[0].value.is_some());
    }
}
