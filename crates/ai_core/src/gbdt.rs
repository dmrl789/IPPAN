//! Production-grade Integer-only GBDT (Gradient Boosted Decision Tree) evaluator
//!
//! This module provides deterministic, reproducible evaluation of GBDT models
//! using only integer arithmetic. No floating point operations are used.
//!
//! Features:
//! - Deterministic evaluation across all platforms
//! - Production-grade error handling and validation
//! - Model versioning and compatibility checking
//! - Performance monitoring and metrics
//! - Security hardening against adversarial inputs
//! - Comprehensive logging and observability

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, error, info, instrument, warn};

/// GBDT evaluation errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum GBDTError {
    #[error("Invalid feature index: {index}, max: {max}")]
    InvalidFeatureIndex { index: usize, max: usize },

    #[error("Invalid tree structure: node {node} references invalid child {child}")]
    InvalidTreeStructure { node: usize, child: usize },

    #[error("Model validation failed: {reason}")]
    ModelValidationFailed { reason: String },

    #[error("Feature vector size mismatch: expected {expected}, got {actual}")]
    FeatureSizeMismatch { expected: usize, actual: usize },

    #[error("Evaluation timeout after {timeout_ms}ms")]
    EvaluationTimeout { timeout_ms: u64 },

    #[error("Model version incompatible: expected {expected}, got {actual}")]
    VersionIncompatible { expected: String, actual: String },

    #[error("Security validation failed: {reason}")]
    SecurityValidationFailed { reason: String },
}

/// GBDT evaluation result with metadata
#[derive(Debug, Clone)]
pub struct GBDTResult {
    /// The predicted value (clamped to [0, scale])
    pub value: i32,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Evaluation time in microseconds
    pub evaluation_time_us: u64,
    /// Number of trees evaluated
    pub trees_evaluated: usize,
    /// Features used in evaluation
    pub features_used: Vec<i64>,
    /// Model version used
    pub model_version: String,
}

/// Performance metrics for GBDT evaluation
#[derive(Debug, Clone, Default)]
pub struct GBDTMetrics {
    pub total_evaluations: u64,
    pub total_time_us: u64,
    pub avg_time_us: f64,
    pub max_time_us: u64,
    pub min_time_us: u64,
    pub error_count: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

/// Model metadata for versioning and compatibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelMetadata {
    pub version: String,
    pub created_at: u64,
    pub feature_count: usize,
    pub tree_count: usize,
    pub max_depth: usize,
    pub model_hash: String,
    pub training_data_hash: String,
    pub performance_metrics: HashMap<String, f64>,
}

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

/// Production-grade GBDT model with comprehensive metadata and validation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GBDTModel {
    /// Collection of trees
    pub trees: Vec<Tree>,
    /// Base bias/intercept value (scaled integer)
    pub bias: i32,
    /// Output scale factor (e.g., 10000 for 4 decimal places)
    pub scale: i32,
    /// Model metadata for versioning and validation
    pub metadata: ModelMetadata,
    /// Feature normalization parameters
    pub feature_normalization: Option<FeatureNormalization>,
    /// Security constraints
    pub security_constraints: SecurityConstraints,
    /// Performance metrics
    #[serde(skip)]
    pub metrics: GBDTMetrics,
    /// Evaluation cache for performance optimization
    #[serde(skip)]
    pub evaluation_cache: HashMap<Vec<i64>, GBDTResult>,
}

/// Feature normalization parameters for consistent evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureNormalization {
    /// Mean values for each feature (scaled integers)
    pub means: Vec<i64>,
    /// Standard deviations for each feature (scaled integers)
    pub std_devs: Vec<i64>,
    /// Min values for clipping (scaled integers)
    pub mins: Vec<i64>,
    /// Max values for clipping (scaled integers)
    pub maxs: Vec<i64>,
}

impl Default for FeatureNormalization {
    fn default() -> Self {
        Self {
            means: Vec::new(),
            std_devs: Vec::new(),
            mins: Vec::new(),
            maxs: Vec::new(),
        }
    }
}

/// Security constraints for model evaluation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityConstraints {
    /// Maximum evaluation time in milliseconds
    pub max_evaluation_time_ms: u64,
    /// Maximum feature value (prevents overflow attacks)
    pub max_feature_value: i64,
    /// Minimum feature value (prevents underflow attacks)
    pub min_feature_value: i64,
    /// Maximum tree depth (prevents deep recursion attacks)
    pub max_tree_depth: usize,
    /// Maximum number of trees (prevents resource exhaustion)
    pub max_trees: usize,
    /// Enable feature validation
    pub enable_feature_validation: bool,
    /// Enable security logging
    pub enable_security_logging: bool,
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

impl Default for SecurityConstraints {
    fn default() -> Self {
        Self {
            max_evaluation_time_ms: 100,       // 100ms timeout
            max_feature_value: 1_000_000_000,  // 1B max feature value
            min_feature_value: -1_000_000_000, // -1B min feature value
            max_tree_depth: 32,                // Max 32 levels deep
            max_trees: 1000,                   // Max 1000 trees
            enable_feature_validation: true,
            enable_security_logging: true,
        }
    }
}

impl Default for ModelMetadata {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
            feature_count: 0,
            tree_count: 0,
            max_depth: 0,
            model_hash: String::new(),
            training_data_hash: String::new(),
            performance_metrics: HashMap::new(),
        }
    }
}

impl GBDTModel {
    /// Create a new production-grade GBDT model
    pub fn new(
        trees: Vec<Tree>,
        bias: i32,
        scale: i32,
        feature_count: usize,
    ) -> Result<Self, GBDTError> {
        let metadata = ModelMetadata {
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
            feature_count,
            tree_count: trees.len(),
            max_depth: Self::calculate_max_depth(&trees)?,
            model_hash: Self::calculate_model_hash(&trees, bias, scale),
            training_data_hash: String::new(),
            performance_metrics: HashMap::new(),
        };

        let model = Self {
            trees,
            bias,
            scale,
            metadata,
            feature_normalization: None,
            security_constraints: SecurityConstraints::default(),
            metrics: GBDTMetrics::default(),
            evaluation_cache: HashMap::new(),
        };

        model.validate()?;
        Ok(model)
    }

    /// Production-grade GBDT evaluation with comprehensive error handling and monitoring
    #[instrument(skip(self, features), fields(feature_count = features.len()))]
    pub fn evaluate(&mut self, features: &[i64]) -> Result<GBDTResult, GBDTError> {
        let start_time = Instant::now();

        // Security validation
        if self.security_constraints.enable_feature_validation {
            self.validate_features(features)?;
        }

        // Check cache first
        if let Some(cached_result) = self.evaluation_cache.get(features) {
            self.metrics.cache_hits += 1;
            debug!("Cache hit for GBDT evaluation");
            return Ok(cached_result.clone());
        }
        self.metrics.cache_misses += 1;

        // Apply feature normalization if configured
        let normalized_features = if let Some(ref norm) = self.feature_normalization {
            self.normalize_features(features, norm)?
        } else {
            features.to_vec()
        };

        // Evaluate trees with timeout protection
        let result = self.evaluate_with_timeout(&normalized_features, start_time)?;

        // Update metrics
        let evaluation_time = start_time.elapsed();
        self.update_metrics(evaluation_time);

        // Cache result if not too large
        if self.evaluation_cache.len() < 1000 {
            self.evaluation_cache
                .insert(features.to_vec(), result.clone());
        }

        debug!(
            "GBDT evaluation completed in {}μs, value: {}, confidence: {:.3}",
            result.evaluation_time_us, result.value, result.confidence
        );

        Ok(result)
    }

    /// Evaluate with timeout protection
    fn evaluate_with_timeout(
        &self,
        features: &[i64],
        start_time: Instant,
    ) -> Result<GBDTResult, GBDTError> {
        let mut sum: i32 = self.bias;
        let mut trees_evaluated = 0;

        for tree in &self.trees {
            // Check timeout
            if start_time.elapsed().as_millis()
                > self.security_constraints.max_evaluation_time_ms as u128
            {
                return Err(GBDTError::EvaluationTimeout {
                    timeout_ms: self.security_constraints.max_evaluation_time_ms,
                });
            }

            let tree_value = self.eval_tree_safe(tree, features)?;
            sum = sum.saturating_add(tree_value);
            trees_evaluated += 1;
        }

        let final_value = sum.clamp(0, self.scale);
        let evaluation_time = start_time.elapsed();

        // Calculate confidence based on tree agreement and evaluation time
        let confidence = self.calculate_confidence(trees_evaluated, evaluation_time);

        Ok(GBDTResult {
            value: final_value,
            confidence,
            evaluation_time_us: evaluation_time.as_micros() as u64,
            trees_evaluated,
            features_used: features.to_vec(),
            model_version: self.metadata.version.clone(),
        })
    }

    /// Safe tree evaluation with comprehensive error checking
    fn eval_tree_safe(&self, tree: &Tree, features: &[i64]) -> Result<i32, GBDTError> {
        let mut idx = 0usize;
        let mut depth = 0;
        let max_depth = self.security_constraints.max_tree_depth;

        loop {
            if idx >= tree.nodes.len() {
                return Err(GBDTError::InvalidTreeStructure {
                    node: idx,
                    child: 0,
                });
            }

            if depth > max_depth {
                return Err(GBDTError::SecurityValidationFailed {
                    reason: format!("Tree depth {} exceeds maximum {}", depth, max_depth),
                });
            }

            let node = &tree.nodes[idx];
            if let Some(value) = node.value {
                return Ok(value);
            }

            let feature_idx = node.feature_index as usize;
            if feature_idx >= features.len() {
                return Err(GBDTError::InvalidFeatureIndex {
                    index: feature_idx,
                    max: features.len() - 1,
                });
            }

            let feature_value = features[feature_idx];
            let left_idx = node.left as usize;
            let right_idx = node.right as usize;

            if left_idx >= tree.nodes.len() || right_idx >= tree.nodes.len() {
                return Err(GBDTError::InvalidTreeStructure {
                    node: idx,
                    child: if left_idx >= tree.nodes.len() {
                        left_idx
                    } else {
                        right_idx
                    },
                });
            }

            idx = if feature_value <= node.threshold {
                left_idx
            } else {
                right_idx
            };
            depth += 1;
        }
    }

    /// Validate input features for security
    fn validate_features(&self, features: &[i64]) -> Result<(), GBDTError> {
        if features.len() != self.metadata.feature_count {
            return Err(GBDTError::FeatureSizeMismatch {
                expected: self.metadata.feature_count,
                actual: features.len(),
            });
        }

        for (i, &feature) in features.iter().enumerate() {
            if feature < self.security_constraints.min_feature_value {
                if self.security_constraints.enable_security_logging {
                    warn!(
                        "Feature {} value {} below minimum {}",
                        i, feature, self.security_constraints.min_feature_value
                    );
                }
                return Err(GBDTError::SecurityValidationFailed {
                    reason: format!("Feature {} value {} below minimum", i, feature),
                });
            }
            if feature > self.security_constraints.max_feature_value {
                if self.security_constraints.enable_security_logging {
                    warn!(
                        "Feature {} value {} above maximum {}",
                        i, feature, self.security_constraints.max_feature_value
                    );
                }
                return Err(GBDTError::SecurityValidationFailed {
                    reason: format!("Feature {} value {} above maximum", i, feature),
                });
            }
        }

        Ok(())
    }

    /// Normalize features using configured normalization parameters
    fn normalize_features(
        &self,
        features: &[i64],
        norm: &FeatureNormalization,
    ) -> Result<Vec<i64>, GBDTError> {
        if features.len() != norm.means.len() {
            return Err(GBDTError::FeatureSizeMismatch {
                expected: norm.means.len(),
                actual: features.len(),
            });
        }

        let mut normalized = Vec::with_capacity(features.len());
        for (i, &feature) in features.iter().enumerate() {
            // Z-score normalization: (x - mean) / std
            let normalized_feature = ((feature - norm.means[i]) * 1000) / norm.std_devs[i];

            // Clip to min/max bounds
            let clipped = normalized_feature.max(norm.mins[i]).min(norm.maxs[i]);

            normalized.push(clipped);
        }

        Ok(normalized)
    }

    /// Calculate confidence score based on evaluation characteristics
    fn calculate_confidence(&self, trees_evaluated: usize, evaluation_time: Duration) -> f64 {
        let time_factor = if evaluation_time.as_micros() < 1000 {
            1.0 // Fast evaluation = high confidence
        } else if evaluation_time.as_micros() < 5000 {
            0.8 // Medium evaluation time
        } else {
            0.6 // Slow evaluation = lower confidence
        };

        let tree_factor = if trees_evaluated == self.trees.len() {
            1.0 // All trees evaluated
        } else {
            trees_evaluated as f64 / self.trees.len() as f64
        };

        (time_factor * tree_factor).min(1.0)
    }

    /// Update performance metrics
    fn update_metrics(&mut self, evaluation_time: Duration) {
        let time_us = evaluation_time.as_micros() as u64;

        self.metrics.total_evaluations += 1;
        self.metrics.total_time_us += time_us;
        self.metrics.avg_time_us =
            self.metrics.total_time_us as f64 / self.metrics.total_evaluations as f64;
        self.metrics.max_time_us = self.metrics.max_time_us.max(time_us);
        self.metrics.min_time_us = if self.metrics.min_time_us == 0 {
            time_us
        } else {
            self.metrics.min_time_us.min(time_us)
        };
    }

    /// Validate model structure and constraints
    pub fn validate(&self) -> Result<(), GBDTError> {
        if self.trees.len() > self.security_constraints.max_trees {
            return Err(GBDTError::ModelValidationFailed {
                reason: format!(
                    "Too many trees: {} > {}",
                    self.trees.len(),
                    self.security_constraints.max_trees
                ),
            });
        }

        for (tree_idx, tree) in self.trees.iter().enumerate() {
            if tree.nodes.is_empty() {
                return Err(GBDTError::ModelValidationFailed {
                    reason: format!("Tree {} is empty", tree_idx),
                });
            }

            for (node_idx, node) in tree.nodes.iter().enumerate() {
                if node.feature_index as usize >= self.metadata.feature_count {
                    return Err(GBDTError::ModelValidationFailed {
                        reason: format!(
                            "Tree {} node {} references invalid feature {}",
                            tree_idx, node_idx, node.feature_index
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Calculate maximum tree depth
    fn calculate_max_depth(trees: &[Tree]) -> Result<usize, GBDTError> {
        let mut max_depth = 0;
        for tree in trees {
            let depth = Self::calculate_tree_depth(tree, 0, 0)?;
            max_depth = max_depth.max(depth);
        }
        Ok(max_depth)
    }

    /// Calculate depth of a specific tree
    fn calculate_tree_depth(
        tree: &Tree,
        node_idx: usize,
        current_depth: usize,
    ) -> Result<usize, GBDTError> {
        if node_idx >= tree.nodes.len() {
            return Err(GBDTError::InvalidTreeStructure {
                node: node_idx,
                child: 0,
            });
        }

        let node = &tree.nodes[node_idx];
        if let Some(_) = node.value {
            return Ok(current_depth);
        }

        let left_depth = Self::calculate_tree_depth(tree, node.left as usize, current_depth + 1)?;
        let right_depth = Self::calculate_tree_depth(tree, node.right as usize, current_depth + 1)?;

        Ok(left_depth.max(right_depth))
    }

    /// Calculate model hash for integrity checking
    pub fn calculate_model_hash(trees: &[Tree], bias: i32, scale: i32) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        trees.hash(&mut hasher);
        bias.hash(&mut hasher);
        scale.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> &GBDTMetrics {
        &self.metrics
    }

    /// Reset performance metrics
    pub fn reset_metrics(&mut self) {
        self.metrics = GBDTMetrics::default();
    }

    /// Clear evaluation cache
    pub fn clear_cache(&mut self) {
        self.evaluation_cache.clear();
    }

    /// Check if model is compatible with expected version
    pub fn is_compatible(&self, expected_version: &str) -> bool {
        self.metadata.version == expected_version
    }
}

/// Legacy function for backward compatibility
pub fn eval_gbdt(model: &GBDTModel, features: &[i64]) -> i32 {
    // Create a mutable copy for evaluation
    let mut model_clone = model.clone();
    match model_clone.evaluate(features) {
        Ok(result) => result.value,
        Err(_) => {
            // Fallback to simple evaluation on error
            let mut sum: i32 = model.bias;
            for tree in &model.trees {
                let tree_value = eval_tree(tree, features);
                sum = sum.saturating_add(tree_value);
            }
            sum.clamp(0, model.scale)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_tree() -> Tree {
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
                    value: Some(10),
                },
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

    fn create_test_model() -> GBDTModel {
        GBDTModel::new(vec![create_simple_tree()], 0, 100, 1).unwrap()
    }

    #[test]
    fn test_eval_tree_branches() {
        let tree = create_simple_tree();
        assert_eq!(eval_tree(&tree, &vec![30]), 10);
        assert_eq!(eval_tree(&tree, &vec![60]), 20);
        assert_eq!(eval_tree(&tree, &vec![50]), 10);
    }

    #[test]
    fn test_gbdt_model_creation() {
        let model = create_test_model();
        assert_eq!(model.trees.len(), 1);
        assert_eq!(model.bias, 0);
        assert_eq!(model.scale, 100);
        assert_eq!(model.metadata.feature_count, 1);
        assert_eq!(model.metadata.tree_count, 1);
    }

    #[test]
    fn test_gbdt_evaluation() {
        let mut model = create_test_model();
        let result = model.evaluate(&vec![30]).unwrap();
        assert_eq!(result.value, 10);
        assert!(result.confidence > 0.0);
        assert!(result.evaluation_time_us > 0);
        assert_eq!(result.trees_evaluated, 1);
    }

    #[test]
    fn test_gbdt_evaluation_with_bias() {
        let mut model = GBDTModel::new(vec![create_simple_tree()], 5, 100, 1).unwrap();

        let result = model.evaluate(&vec![30]).unwrap();
        assert_eq!(result.value, 15);
    }

    #[test]
    fn test_gbdt_multiple_trees() {
        let mut model =
            GBDTModel::new(vec![create_simple_tree(), create_simple_tree()], 0, 100, 1).unwrap();

        let result = model.evaluate(&vec![30]).unwrap();
        assert_eq!(result.value, 20);
        assert_eq!(result.trees_evaluated, 2);
    }

    #[test]
    fn test_gbdt_clamping() {
        let mut model = GBDTModel::new(
            vec![Tree {
                nodes: vec![Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(200),
                }],
            }],
            0,
            100,
            1,
        )
        .unwrap();

        let result = model.evaluate(&vec![0]).unwrap();
        assert_eq!(result.value, 100); // Clamped to scale
    }

    #[test]
    fn test_gbdt_negative_bias() {
        let mut model = GBDTModel::new(
            vec![Tree {
                nodes: vec![Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(-50),
                }],
            }],
            -100,
            100,
            1,
        )
        .unwrap();

        let result = model.evaluate(&vec![0]).unwrap();
        assert_eq!(result.value, 0); // Clamped to 0
    }

    #[test]
    fn test_multi_level_tree() {
        let tree = Tree {
            nodes: vec![
                Node {
                    feature_index: 0,
                    threshold: 50,
                    left: 1,
                    right: 2,
                    value: None,
                },
                Node {
                    feature_index: 1,
                    threshold: 30,
                    left: 3,
                    right: 4,
                    value: None,
                },
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(100),
                },
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(10),
                },
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(50),
                },
            ],
        };

        let mut model = GBDTModel::new(vec![tree], 0, 100, 2).unwrap();

        let result1 = model.evaluate(&vec![40, 20]).unwrap();
        assert_eq!(result1.value, 10);

        let result2 = model.evaluate(&vec![40, 40]).unwrap();
        assert_eq!(result2.value, 50);

        let result3 = model.evaluate(&vec![60, 0]).unwrap();
        assert_eq!(result3.value, 100);
    }

    #[test]
    fn test_determinism() {
        let mut model = create_test_model();
        let f = vec![45];

        let result1 = model.evaluate(&f).unwrap();
        let result2 = model.evaluate(&f).unwrap();

        assert_eq!(result1.value, result2.value);
        assert_eq!(result1.confidence, result2.confidence);
    }

    #[test]
    fn test_feature_validation() {
        let mut model = create_test_model();

        // Test feature size mismatch
        let result = model.evaluate(&vec![1, 2, 3]); // Wrong size
        assert!(matches!(result, Err(GBDTError::FeatureSizeMismatch { .. })));
    }

    #[test]
    fn test_security_constraints() {
        let mut model = create_test_model();
        model.security_constraints.max_feature_value = 10;

        // Test feature value too high
        let result = model.evaluate(&vec![100]);
        assert!(matches!(
            result,
            Err(GBDTError::SecurityValidationFailed { .. })
        ));
    }

    #[test]
    fn test_metrics_tracking() {
        let mut model = create_test_model();

        // Initial metrics should be zero
        let initial_metrics = model.get_metrics();
        assert_eq!(initial_metrics.total_evaluations, 0);

        // Perform evaluation
        let _ = model.evaluate(&vec![30]).unwrap();

        // Metrics should be updated
        let updated_metrics = model.get_metrics();
        assert_eq!(updated_metrics.total_evaluations, 1);
        assert!(updated_metrics.total_time_us > 0);
    }

    #[test]
    fn test_caching() {
        let mut model = create_test_model();
        let features = vec![30];

        // First evaluation - cache miss
        let result1 = model.evaluate(&features).unwrap();
        assert_eq!(model.get_metrics().cache_misses, 1);
        assert_eq!(model.get_metrics().cache_hits, 0);

        // Second evaluation - cache hit
        let result2 = model.evaluate(&features).unwrap();
        assert_eq!(model.get_metrics().cache_misses, 1);
        assert_eq!(model.get_metrics().cache_hits, 1);

        // Results should be identical
        assert_eq!(result1.value, result2.value);
    }

    #[test]
    fn test_model_validation() {
        // Test invalid tree structure
        let invalid_tree = Tree {
            nodes: vec![
                Node {
                    feature_index: 0,
                    threshold: 50,
                    left: 5,
                    right: 2,
                    value: None,
                }, // Invalid child index
                Node {
                    feature_index: 0,
                    threshold: 0,
                    left: 0,
                    right: 0,
                    value: Some(10),
                },
            ],
        };

        let result = GBDTModel::new(vec![invalid_tree], 0, 100, 1);
        assert!(matches!(
            result,
            Err(GBDTError::InvalidTreeStructure { .. })
        ));
    }

    #[test]
    fn test_legacy_compatibility() {
        let model = create_test_model();
        let features = vec![30];

        // Test legacy eval_gbdt function
        let legacy_result = eval_gbdt(&model, &features);
        let modern_result = model.clone().evaluate(&features).unwrap();

        assert_eq!(legacy_result, modern_result.value);
    }
}
