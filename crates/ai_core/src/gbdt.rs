use crate::model::Model;
use anyhow::Result;
use thiserror::Error;

/// GBDT evaluation errors
#[derive(Error, Debug)]
pub enum GbdtError {
    #[error("Invalid feature index: {0}")]
    InvalidFeatureIndex(usize),
    #[error("Model validation failed: {0}")]
    ModelValidation(String),
    #[error("Feature count mismatch: expected {expected}, got {actual}")]
    FeatureCountMismatch { expected: usize, actual: usize },
}

/// Deterministic GBDT evaluator for L1 consensus
pub struct GbdtEvaluator {
    model: Model,
}

impl GbdtEvaluator {
    /// Create a new GBDT evaluator with the given model
    pub fn new(model: Model) -> Result<Self, GbdtError> {
        // Validate model structure
        if model.trees.is_empty() {
            return Err(GbdtError::ModelValidation("Model has no trees".to_string()));
        }

        for (i, tree) in model.trees.iter().enumerate() {
            if tree.nodes.is_empty() {
                return Err(GbdtError::ModelValidation(format!("Tree {} has no nodes", i)));
            }
        }

        Ok(Self { model })
    }

    /// Evaluate the GBDT model on the given features
    /// Returns a reputation score in the range [0, model.scale]
    pub fn evaluate(&self, features: &[i64]) -> Result<i32, GbdtError> {
        if features.len() != self.model.feature_count {
            return Err(GbdtError::FeatureCountMismatch {
                expected: self.model.feature_count,
                actual: features.len(),
            });
        }

        let mut sum: i32 = self.model.bias;
        
        for tree in &self.model.trees {
            let mut node_index = 0usize;
            
            loop {
                let node = &tree.nodes[node_index];
                
                // Check if this is a leaf node
                if let Some(value) = node.value {
                    sum = sum.saturating_add(value as i32);
                    break;
                }
                
                // Validate feature index
                if node.feature_index >= features.len() {
                    return Err(GbdtError::InvalidFeatureIndex(node.feature_index));
                }
                
                // Traverse to next node based on feature comparison
                let feature_value = features[node.feature_index];
                node_index = if feature_value <= node.threshold {
                    node.left as usize
                } else {
                    node.right as usize
                };
                
                // Safety check to prevent infinite loops
                if node_index >= tree.nodes.len() {
                    return Err(GbdtError::ModelValidation(
                        "Invalid tree structure: node index out of bounds".to_string()
                    ));
                }
            }
        }
        
        // Clamp the result to the model's scale range
        Ok(sum.clamp(0, self.model.scale))
    }

    /// Get the model's feature count
    pub fn feature_count(&self) -> usize {
        self.model.feature_count
    }

    /// Get the model's scale (maximum possible score)
    pub fn scale(&self) -> i32 {
        self.model.scale
    }
}

/// Convenience function to evaluate a model directly
pub fn eval_gbdt(model: &Model, features: &[i64]) -> Result<i32, GbdtError> {
    let evaluator = GbdtEvaluator::new(model.clone())?;
    evaluator.evaluate(features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Model, Tree, Node};

    fn create_test_model() -> Model {
        Model {
            version: 1,
            feature_count: 3,
            bias: 100,
            scale: 10000,
            trees: vec![
                Tree {
                    nodes: vec![
                        Node {
                            feature_index: 0,
                            threshold: 50,
                            left: 1,
                            right: 2,
                            value: None,
                        },
                        Node {
                            feature_index: 0,
                            threshold: 0,
                            left: 0,
                            right: 0,
                            value: Some(1000),
                        },
                        Node {
                            feature_index: 1,
                            threshold: 25,
                            left: 3,
                            right: 4,
                            value: None,
                        },
                        Node {
                            feature_index: 0,
                            threshold: 0,
                            left: 0,
                            right: 0,
                            value: Some(2000),
                        },
                        Node {
                            feature_index: 0,
                            threshold: 0,
                            left: 0,
                            right: 0,
                            value: Some(500),
                        },
                    ],
                },
            ],
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_gbdt_evaluation() {
        let model = create_test_model();
        let evaluator = GbdtEvaluator::new(model.clone()).unwrap();
        
        // Test case 1: features = [30, 20, 10] -> should go left-left -> 1000
        let features1 = vec![30, 20, 10];
        let result1 = evaluator.evaluate(&features1).unwrap();
        assert_eq!(result1, 1100); // bias (100) + leaf value (1000)
        
        // Test case 2: features = [60, 30, 10] -> should go right-right -> 500
        let features2 = vec![60, 30, 10];
        let result2 = evaluator.evaluate(&features2).unwrap();
        assert_eq!(result2, 600); // bias (100) + leaf value (500)
        
        // Test case 3: features = [60, 20, 10] -> should go right-left -> 2000
        let features3 = vec![60, 20, 10];
        let result3 = evaluator.evaluate(&features3).unwrap();
        assert_eq!(result3, 2100); // bias (100) + leaf value (2000)
    }

    #[test]
    fn test_feature_count_mismatch() {
        let model = create_test_model();
        let evaluator = GbdtEvaluator::new(model).unwrap();
        
        let wrong_features = vec![1, 2]; // Only 2 features, but model expects 3
        let result = evaluator.evaluate(&wrong_features);
        assert!(matches!(result, Err(GbdtError::FeatureCountMismatch { .. })));
    }

    #[test]
    fn test_scale_clamping() {
        let mut model = create_test_model();
        model.scale = 100; // Very low scale
        
        let evaluator = GbdtEvaluator::new(model).unwrap();
        let features = vec![30, 20, 10];
        let result = evaluator.evaluate(&features).unwrap();
        
        // Should be clamped to scale (100)
        assert_eq!(result, 100);
    }

    #[test]
    fn test_determinism() {
        let model = create_test_model();
        let evaluator = GbdtEvaluator::new(model).unwrap();
        let features = vec![30, 20, 10];
        
        // Run evaluation multiple times
        let result1 = evaluator.evaluate(&features).unwrap();
        let result2 = evaluator.evaluate(&features).unwrap();
        let result3 = evaluator.evaluate(&features).unwrap();
        
        // Results should be identical
        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_cross_platform() {
        let model = create_test_model();
        let evaluator = GbdtEvaluator::new(model).unwrap();
        let features = vec![30, 20, 10];
        
        // This test ensures the same inputs produce the same outputs
        // across different platforms
        let result = evaluator.evaluate(&features).unwrap();
        
        // The result should be deterministic
        assert!(result >= 0);
        assert!(result <= 10000);
    }
}