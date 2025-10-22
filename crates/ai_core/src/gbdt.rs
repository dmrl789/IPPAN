/// Integer-only GBDT (Gradient Boosted Decision Tree) evaluator
///
/// This module provides deterministic, reproducible evaluation of GBDT models
/// using only integer arithmetic. No floating point operations are used.
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
///
/// # Arguments
/// * `tree` - Tree to evaluate
/// * `features` - Feature vector (scaled integers)
///
/// # Returns
/// Leaf value for this tree
fn eval_tree(tree: &Tree, features: &[i64]) -> i32 {
    let mut idx = 0usize;

    loop {
        if idx >= tree.nodes.len() {
            // Safety: invalid tree structure
            return 0;
        }

        let node = &tree.nodes[idx];

        // Check if this is a leaf node
        if let Some(value) = node.value {
            return value;
        }

        // Internal node: compare feature
        let feature_idx = node.feature_index as usize;
        if feature_idx >= features.len() {
            // Safety: feature index out of bounds
            return 0;
        }

        let feature_value = features[feature_idx];

        // Navigate left or right based on threshold
        idx = if feature_value <= node.threshold {
            node.left as usize
        } else {
            node.right as usize
        };
    }
}

/// Evaluate GBDT model with integer features
///
/// # Arguments
/// * `model` - GBDT model to evaluate
/// * `features` - Feature vector (scaled integers)
///
/// # Returns
/// Model output (scaled integer, clamped to [0, scale])
///
/// # Example
/// ```
/// use ippan_ai_core::gbdt::{eval_gbdt, GBDTModel, Tree, Node};
///
/// let model = GBDTModel {
///     trees: vec![
///         Tree {
///             nodes: vec![
///                 Node { feature_index: 0, threshold: 50, left: 1, right: 2, value: None },
///                 Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(10) },
///                 Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(20) },
///             ],
///         },
///     ],
///     bias: 0,
///     scale: 100,
/// };
///
/// let features = vec![30]; // feature[0] = 30
/// let score = eval_gbdt(&model, &features);
/// assert_eq!(score, 10); // Takes left branch (30 <= 50)
/// ```
pub fn eval_gbdt(model: &GBDTModel, features: &[i64]) -> i32 {
    let mut sum: i32 = model.bias;

    for tree in &model.trees {
        let tree_value = eval_tree(tree, features);
        sum = sum.saturating_add(tree_value);
    }

    // Clamp to valid range [0, scale]
    sum.clamp(0, model.scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_tree() -> Tree {
        Tree {
            nodes: vec![
                // Root: if feature[0] <= 50 go left, else right
                Node {
                    feature_index: 0,
                    threshold: 50,
                    left: 1,
                    right: 2,
                    value: None,
                },
                // Left leaf: return 10
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(10),
                },
                // Right leaf: return 20
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(20),
                },
            ],
        }
    }

    #[test]
    fn test_eval_tree_left_branch() {
        let tree = create_simple_tree();
        let features = vec![30];
        assert_eq!(eval_tree(&tree, &features), 10);
    }

    #[test]
    fn test_eval_tree_right_branch() {
        let tree = create_simple_tree();
        let features = vec![60];
        assert_eq!(eval_tree(&tree, &features), 20);
    }

    #[test]
    fn test_eval_tree_threshold_boundary() {
        let tree = create_simple_tree();
        let features = vec![50];
        assert_eq!(eval_tree(&tree, &features), 10); // <= 50 goes left
    }

    #[test]
    fn test_eval_gbdt_single_tree() {
        let model = GBDTModel {
            trees: vec![create_simple_tree()],
            bias: 0,
            scale: 100,
        };

        assert_eq!(eval_gbdt(&model, &vec![30]), 10);
        assert_eq!(eval_gbdt(&model, &vec![60]), 20);
    }

    #[test]
    fn test_eval_gbdt_with_bias() {
        let model = GBDTModel {
            trees: vec![create_simple_tree()],
            bias: 5,
            scale: 100,
        };

        assert_eq!(eval_gbdt(&model, &vec![30]), 15); // 5 + 10
        assert_eq!(eval_gbdt(&model, &vec![60]), 25); // 5 + 20
    }

    #[test]
    fn test_eval_gbdt_multiple_trees() {
        let model = GBDTModel {
            trees: vec![create_simple_tree(), create_simple_tree()],
            bias: 0,
            scale: 100,
        };

        assert_eq!(eval_gbdt(&model, &vec![30]), 20); // 10 + 10
        assert_eq!(eval_gbdt(&model, &vec![60]), 40); // 20 + 20
    }

    #[test]
    fn test_eval_gbdt_clamps_to_scale() {
        let model = GBDTModel {
            trees: vec![Tree {
                nodes: vec![Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(200),
                }],
            }],
            bias: 0,
            scale: 100,
        };

        assert_eq!(eval_gbdt(&model, &vec![0]), 100); // Clamped to scale
    }

    #[test]
    fn test_eval_gbdt_clamps_negative_to_zero() {
        let model = GBDTModel {
            trees: vec![Tree {
                nodes: vec![Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(-50),
                }],
            }],
            bias: -100,
            scale: 100,
        };

        assert_eq!(eval_gbdt(&model, &vec![0]), 0); // Clamped to 0
    }

    #[test]
    fn test_eval_tree_invalid_feature_index() {
        let tree = create_simple_tree();
        let features = vec![]; // Empty features
        assert_eq!(eval_tree(&tree, &features), 0); // Returns 0 for safety
    }

    #[test]
    fn test_multi_level_tree() {
        let tree = Tree {
            nodes: vec![
                // Root: feature[0] <= 50
                Node {
                    feature_index: 0,
                    threshold: 50,
                    left: 1,
                    right: 2,
                    value: None,
                },
                // Left branch: feature[1] <= 30
                Node {
                    feature_index: 1,
                    threshold: 30,
                    left: 3,
                    right: 4,
                    value: None,
                },
                // Right leaf
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(100),
                },
                // Left-left leaf
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(10),
                },
                // Left-right leaf
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(50),
                },
            ],
        };

        assert_eq!(eval_tree(&tree, &vec![40, 20]), 10); // Left-left
        assert_eq!(eval_tree(&tree, &vec![40, 40]), 50); // Left-right
        assert_eq!(eval_tree(&tree, &vec![60, 0]), 100); // Right
    }
}
