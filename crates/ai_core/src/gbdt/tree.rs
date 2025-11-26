//! Decision tree structures for GBDT inference
//!
//! Provides deterministic integer-only tree nodes and traversal.
//! All values are fixed-point integers at SCALE precision.

use serde::{Deserialize, Serialize};

/// A decision tree node (internal or leaf)
///
/// For internal nodes:
/// - `feature >= 0`: index into feature vector
/// - `left` and `right` point to child node indices
/// - `leaf` is `None`
///
/// For leaf nodes:
/// - `feature == -1` indicates this is a leaf
/// - `leaf` contains the prediction value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Node {
    /// Node ID (for reference, not used in traversal)
    pub id: i32,

    /// Left child index (-1 for leaf nodes)
    pub left: i32,

    /// Right child index (-1 for leaf nodes)
    pub right: i32,

    /// Feature index to split on (-1 for leaf nodes)
    #[serde(rename = "feature_idx", alias = "feature")]
    pub feature_idx: i32,

    /// Threshold value for split (fixed-point integer)
    pub threshold: i64,

    /// Leaf value (Some for leaf nodes, None for internal nodes)
    pub leaf: Option<i64>,
}

impl Node {
    /// Create a new internal (split) node
    pub fn internal(id: i32, feature_idx: i32, threshold: i64, left: i32, right: i32) -> Self {
        Self {
            id,
            left,
            right,
            feature_idx,
            threshold,
            leaf: None,
        }
    }

    /// Create a new leaf node
    pub fn leaf(id: i32, value: i64) -> Self {
        Self {
            id,
            left: -1,
            right: -1,
            feature_idx: -1,
            threshold: 0,
            leaf: Some(value),
        }
    }

    /// Check if this node is a leaf
    pub fn is_leaf(&self) -> bool {
        self.feature_idx == -1 || self.leaf.is_some()
    }

    /// Get the leaf value if this is a leaf node
    pub fn leaf_value(&self) -> Option<i64> {
        self.leaf
    }
}

/// A single decision tree with integer-only nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Tree {
    /// Tree nodes (node 0 is the root)
    pub nodes: Vec<Node>,

    /// Tree weight for ensemble aggregation (fixed-point integer)
    pub weight: i64,
}

impl Tree {
    /// Create a new tree with the given nodes and weight
    pub fn new(nodes: Vec<Node>, weight: i64) -> Self {
        Self { nodes, weight }
    }

    /// Evaluate this tree on a feature vector
    ///
    /// Uses FixedOrd-style comparison (integer <= comparison)
    pub fn evaluate(&self, features: &[i64]) -> i64 {
        if self.nodes.is_empty() {
            return 0;
        }

        let mut idx = 0usize;

        loop {
            if idx >= self.nodes.len() {
                return 0; // Invalid tree structure
            }

            let node = &self.nodes[idx];

            // Check if leaf
            if node.is_leaf() {
                return node.leaf_value().unwrap_or(0);
            }

            // Internal node - compare feature value
            let feature_idx = node.feature_idx as usize;
            if feature_idx >= features.len() {
                return 0; // Invalid feature index
            }

            let feature_value = features[feature_idx];

            // FixedOrd comparison: go left if feature <= threshold
            idx = if feature_value <= node.threshold {
                if node.left < 0 || node.left as usize >= self.nodes.len() {
                    return 0;
                }
                node.left as usize
            } else {
                if node.right < 0 || node.right as usize >= self.nodes.len() {
                    return 0;
                }
                node.right as usize
            };
        }
    }

    /// Get the root node
    pub fn root(&self) -> Option<&Node> {
        self.nodes.first()
    }

    /// Validate tree structure
    pub fn validate(&self) -> Result<(), String> {
        if self.nodes.is_empty() {
            return Err("Tree has no nodes".to_string());
        }

        // Validate each node
        for (i, node) in self.nodes.iter().enumerate() {
            if !node.is_leaf() {
                // Check left child
                if node.left < 0 || node.left as usize >= self.nodes.len() {
                    return Err(format!("Node {} has invalid left child: {}", i, node.left));
                }

                // Check right child
                if node.right < 0 || node.right as usize >= self.nodes.len() {
                    return Err(format!(
                        "Node {} has invalid right child: {}",
                        i, node.right
                    ));
                }

                // Check feature index is valid (>= 0 for internal nodes)
                if node.feature_idx < 0 {
                    return Err(format!(
                        "Internal node {} has invalid feature index: {}",
                        i, node.feature_idx
                    ));
                }
            } else {
                // Leaf node must have a leaf value
                if node.leaf.is_none() {
                    return Err(format!("Leaf node {i} has no leaf value"));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let internal = Node::internal(0, 3, 12345, 1, 2);
        assert_eq!(internal.id, 0);
        assert_eq!(internal.feature_idx, 3);
        assert_eq!(internal.threshold, 12345);
        assert_eq!(internal.left, 1);
        assert_eq!(internal.right, 2);
        assert!(!internal.is_leaf());

        let leaf = Node::leaf(1, -234);
        assert_eq!(leaf.id, 1);
        assert_eq!(leaf.feature_idx, -1);
        assert!(leaf.is_leaf());
        assert_eq!(leaf.leaf_value(), Some(-234));
    }

    #[test]
    fn test_tree_evaluation() {
        // Simple tree: if feature[0] <= 50, return 100, else return 200
        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 50, 1, 2),
                Node::leaf(1, 100),
                Node::leaf(2, 200),
            ],
            1_000_000,
        );

        assert_eq!(tree.evaluate(&[30]), 100);
        assert_eq!(tree.evaluate(&[50]), 100); // Equal goes left
        assert_eq!(tree.evaluate(&[60]), 200);
    }

    #[test]
    fn test_tree_validation() {
        let valid_tree = Tree::new(
            vec![
                Node::internal(0, 0, 50, 1, 2),
                Node::leaf(1, 100),
                Node::leaf(2, 200),
            ],
            1_000_000,
        );
        assert!(valid_tree.validate().is_ok());

        // Invalid tree: left child out of bounds
        let invalid_tree = Tree::new(
            vec![
                Node::internal(0, 0, 50, 5, 2), // left=5 is invalid
                Node::leaf(1, 100),
                Node::leaf(2, 200),
            ],
            1_000_000,
        );
        assert!(invalid_tree.validate().is_err());
    }

    #[test]
    fn test_deterministic_traversal() {
        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 50, 1, 2),
                Node::leaf(1, 100),
                Node::leaf(2, 200),
            ],
            1_000_000,
        );

        let features = vec![30, 40, 50];

        // Same input should always give same output
        let result1 = tree.evaluate(&features);
        let result2 = tree.evaluate(&features);
        let result3 = tree.evaluate(&features);

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }
}
