//! Integer-only GBDT (Gradient Boosted Decision Tree) evaluator
//!
//! This module provides deterministic, reproducible evaluation of GBDT models
//! using only integer arithmetic. No floating point operations are used.
use serde::{Deserialize, Serialize};

/// A decision tree node (internal or leaf)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Node {
    /// Feature index to compare (for internal nodes)
    pub feature_index: u16,
    /// Threshold value for comparison (integer, scaled)
    pub threshold: i64,
    /// Index of left child node
    pub left: u16,
    /// Index of right child node
    pub right: u16,
    /// Leaf value (None for internal nodes, Some for leaves)
    pub value: Option<i32>,
}

/// A single decision tree
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tree {
    /// Nodes in breadth-first or depth-first order
    pub nodes: Vec<Node>,
}

/// Complete GBDT model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GBDTModel {
    /// Collection of trees
    pub trees: Vec<Tree>,
    /// Base bias/intercept value (scaled integer)
    pub bias: i32,
    /// Output scale factor (e.g., 10000 for 4 decimal places)
    pub scale: i32,
}

/// Evaluate a single tree with integer features
fn eval_tree(tree: &Tree, features: &[i64]) -> i32 {
    let mut idx = 0usize;
    loop {
        if idx >= tree.nodes.len() {
            return 0; // Safety: invalid tree structure
        }
        let node = &tree.nodes[idx];
        if let Some(value) = node.value {
            return value;
        }
        let feature_idx = node.feature_index as usize;
        if feature_idx >= features.len() {
            return 0; // Safety: feature index out of bounds
        }
        let feature_value = features[feature_idx];
        idx = if feature_value <= node.threshold {
            node.left as usize
        } else {
            node.right as usize
        };
    }
}

/// Evaluate GBDT model with integer features
///
/// Returns a clamped scaled integer in [0, scale].
pub fn eval_gbdt(model: &GBDTModel, features: &[i64]) -> i32 {
    let mut sum: i32 = model.bias;
    for tree in &model.trees {
        let tree_value = eval_tree(tree, features);
        sum = sum.saturating_add(tree_value);
    }
    sum.clamp(0, model.scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_tree() -> Tree {
        Tree {
            nodes: vec![
                Node { feature_index: 0, threshold: 50, left: 1, right: 2, value: None },
                Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(10) },
                Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(20) },
            ],
        }
    }

    #[test]
    fn test_eval_tree_branches() {
        let tree = create_simple_tree();
        assert_eq!(eval_tree(&tree, &vec![30]), 10);
        assert_eq!(eval_tree(&tree, &vec![60]), 20);
        assert_eq!(eval_tree(&tree, &vec![50]), 10);
    }

    #[test]
    fn test_eval_gbdt_basic() {
        let model = GBDTModel { trees: vec![create_simple_tree()], bias: 0, scale: 100 };
        assert_eq!(eval_gbdt(&model, &vec![30]), 10);
        assert_eq!(eval_gbdt(&model, &vec![60]), 20);
    }

    #[test]
    fn test_eval_gbdt_with_bias() {
        let model = GBDTModel { trees: vec![create_simple_tree()], bias: 5, scale: 100 };
        assert_eq!(eval_gbdt(&model, &vec![30]), 15);
        assert_eq!(eval_gbdt(&model, &vec![60]), 25);
    }

    #[test]
    fn test_eval_gbdt_multiple_trees() {
        let model = GBDTModel { trees: vec![create_simple_tree(), create_simple_tree()], bias: 0, scale: 100 };
        assert_eq!(eval_gbdt(&model, &vec![30]), 20);
        assert_eq!(eval_gbdt(&model, &vec![60]), 40);
    }

    #[test]
    fn test_eval_gbdt_clamping() {
        let model = GBDTModel {
            trees: vec![Tree { nodes: vec![Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(200) }] }],
            bias: 0,
            scale: 100,
        };
        assert_eq!(eval_gbdt(&model, &vec![0]), 100);
    }

    #[test]
    fn test_eval_gbdt_negative() {
        let model = GBDTModel {
            trees: vec![Tree { nodes: vec![Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(-50) }] }],
            bias: -100,
            scale: 100,
        };
        assert_eq!(eval_gbdt(&model, &vec![0]), 0);
    }

    #[test]
    fn test_multi_level_tree() {
        let tree = Tree {
            nodes: vec![
                Node { feature_index: 0, threshold: 50, left: 1, right: 2, value: None },
                Node { feature_index: 1, threshold: 30, left: 3, right: 4, value: None },
                Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(100) },
                Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(10) },
                Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(50) },
            ],
        };
        assert_eq!(eval_tree(&tree, &vec![40, 20]), 10);
        assert_eq!(eval_tree(&tree, &vec![40, 40]), 50);
        assert_eq!(eval_tree(&tree, &vec![60, 0]), 100);
    }

    #[test]
    fn test_determinism() {
        let model = GBDTModel { trees: vec![create_simple_tree()], bias: 3, scale: 100 };
        let f = vec![45];
        assert_eq!(eval_gbdt(&model, &f), eval_gbdt(&model, &f));
        assert_eq!(eval_gbdt(&model, &f), eval_gbdt(&model, &f));
    }
}
