//! GBDT Model with deterministic inference
//!
//! Implements a fixed-point-only GBDT model with:
//! - Canonical JSON serialization
//! - Blake3 model hashing
//! - Integer-only inference
//! - Deterministic across all platforms

use super::tree::Tree;
use crate::serde_canon::{hash_canonical_hex, to_canonical_json, CanonicalError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

/// GBDT Model errors
#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Model validation failed: {0}")]
    ValidationFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Canonical serialization error: {0}")]
    CanonicalError(#[from] CanonicalError),

    #[error("Invalid model format: {0}")]
    InvalidFormat(String),
}

/// Default scale factor for fixed-point arithmetic (1e6)
pub const SCALE: i64 = 1_000_000;

/// GBDT Model with integer-only representation
///
/// All values are fixed-point integers scaled by `scale` (default 1e6).
/// Model format uses canonical JSON with sorted keys for deterministic hashing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Model {
    /// Model format version (always 1 for now)
    pub version: i32,

    /// Fixed-point scale factor (typically 1_000_000 for micro precision)
    pub scale: i64,

    /// Decision trees in the ensemble
    pub trees: Vec<Tree>,

    /// Bias term (fixed-point integer)
    pub bias: i64,

    /// Post-processing scale factor (typically same as scale)
    pub post_scale: i64,
}

impl Model {
    /// Create a new GBDT model
    pub fn new(trees: Vec<Tree>, bias: i64) -> Self {
        Self {
            version: 1,
            scale: SCALE,
            trees,
            bias,
            post_scale: SCALE,
        }
    }

    /// Create a model with custom scale factors
    pub fn with_scale(trees: Vec<Tree>, bias: i64, scale: i64, post_scale: i64) -> Self {
        Self {
            version: 1,
            scale,
            trees,
            bias,
            post_scale,
        }
    }

    /// Validate model structure
    pub fn validate(&self) -> Result<(), ModelError> {
        // Check version
        if self.version != 1 {
            return Err(ModelError::ValidationFailed(format!(
                "Unsupported model version: {}",
                self.version
            )));
        }

        // Check scale is positive
        if self.scale <= 0 {
            return Err(ModelError::ValidationFailed(format!(
                "Invalid scale: {}",
                self.scale
            )));
        }

        // Check post_scale is positive
        if self.post_scale <= 0 {
            return Err(ModelError::ValidationFailed(format!(
                "Invalid post_scale: {}",
                self.post_scale
            )));
        }

        // Validate each tree
        for (i, tree) in self.trees.iter().enumerate() {
            tree.validate().map_err(|e| {
                ModelError::ValidationFailed(format!("Tree {} validation failed: {}", i, e))
            })?;
        }

        Ok(())
    }

    /// Perform deterministic inference on a feature vector
    ///
    /// Returns a fixed-point integer score scaled by `post_scale`.
    ///
    /// Algorithm:
    /// 1. For each tree, traverse to find leaf value
    /// 2. Accumulate: leaf_value * tree_weight / scale
    /// 3. Add bias
    /// 4. Quantize to post_scale
    pub fn score(&self, features: &[i64]) -> i64 {
        let mut sum = self.bias;

        for tree in &self.trees {
            let leaf_value = tree.evaluate(features);

            // Accumulate: (leaf_value * tree_weight) / scale
            // Use checked operations to prevent overflow
            let weighted = leaf_value.checked_mul(tree.weight).unwrap_or(0);

            let contribution = weighted / self.scale;
            sum = sum.saturating_add(contribution);
        }

        // Quantize to post_scale (already at correct scale)
        sum
    }

    /// Serialize model to canonical JSON (sorted keys, no whitespace)
    pub fn to_canonical_json(&self) -> Result<String, ModelError> {
        Ok(to_canonical_json(self)?)
    }

    /// Compute Blake3 hash of canonical JSON representation
    pub fn hash(&self) -> Result<[u8; 32], ModelError> {
        let json = self.to_canonical_json()?;
        let hash = blake3::hash(json.as_bytes());
        Ok(*hash.as_bytes())
    }

    /// Compute model hash as hex string
    pub fn hash_hex(&self) -> Result<String, ModelError> {
        Ok(hash_canonical_hex(self)?)
    }

    /// Save model to JSON file with canonical serialization
    pub fn save_json<P: AsRef<Path>>(&self, path: P) -> Result<(), ModelError> {
        let json = self.to_canonical_json()?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load model from JSON file
    pub fn load_json<P: AsRef<Path>>(path: P) -> Result<Self, ModelError> {
        let json = fs::read_to_string(path)?;
        let model: Model = serde_json::from_str(&json)?;
        model.validate()?;
        Ok(model)
    }

    /// Get number of trees in the model
    pub fn num_trees(&self) -> usize {
        self.trees.len()
    }
}

/// Compute model hash from raw model data
pub fn model_hash(
    trees: &[Tree],
    bias: i64,
    scale: i64,
    post_scale: i64,
) -> Result<String, ModelError> {
    let model = Model {
        version: 1,
        scale,
        trees: trees.to_vec(),
        bias,
        post_scale,
    };
    model.hash_hex()
}

/// Compute model hash as hex string (convenience function)
pub fn model_hash_hex(model: &Model) -> String {
    model.hash_hex().unwrap_or_else(|_| String::from("error"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbdt::tree::Node;

    fn create_test_model() -> Model {
        // Two simple trees
        let tree1 = Tree::new(
            vec![
                Node::internal(0, 0, 50 * SCALE, 1, 2),
                Node::leaf(1, 100 * SCALE),
                Node::leaf(2, 200 * SCALE),
            ],
            SCALE,
        );

        let tree2 = Tree::new(
            vec![
                Node::internal(0, 1, 30 * SCALE, 1, 2),
                Node::leaf(1, -50 * SCALE),
                Node::leaf(2, 50 * SCALE),
            ],
            SCALE,
        );

        Model::new(vec![tree1, tree2], 0)
    }

    #[test]
    fn test_model_creation() {
        let model = create_test_model();
        assert_eq!(model.version, 1);
        assert_eq!(model.scale, SCALE);
        assert_eq!(model.num_trees(), 2);
        assert!(model.validate().is_ok());
    }

    #[test]
    fn test_model_inference() {
        let model = create_test_model();

        // features[0] = 30 (< 50), features[1] = 20 (< 30)
        // Tree 1: goes left -> 100 * SCALE
        // Tree 2: goes left -> -50 * SCALE
        // Sum: (100 + (-50)) * SCALE = 50 * SCALE
        let features = vec![30 * SCALE, 20 * SCALE];
        let score = model.score(&features);

        // Expected: tree1(100M) * 1M / 1M + tree2(-50M) * 1M / 1M + 0 = 50M
        assert_eq!(score, 50 * SCALE);
    }

    #[test]
    fn test_deterministic_inference() {
        let model = create_test_model();
        let features = vec![30 * SCALE, 20 * SCALE];

        // Same input should always give same output
        let score1 = model.score(&features);
        let score2 = model.score(&features);
        let score3 = model.score(&features);

        assert_eq!(score1, score2);
        assert_eq!(score2, score3);
    }

    #[test]
    fn test_canonical_json() {
        let model = create_test_model();
        let json = model.to_canonical_json().unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());

        // Should have expected fields
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"scale\""));
        assert!(json.contains("\"trees\""));
        assert!(json.contains("\"bias\""));

        // Should be compact (no whitespace)
        assert!(!json.contains('\n'));
    }

    #[test]
    fn test_hash_deterministic() {
        let model1 = create_test_model();
        let model2 = create_test_model();

        let hash1 = model1.hash_hex().unwrap();
        let hash2 = model2.hash_hex().unwrap();

        // Same model should have same hash
        assert_eq!(hash1, hash2);

        // Hash should be 64 hex chars (32 bytes)
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_hash_changes_with_model() {
        let model1 = create_test_model();

        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 50 * SCALE, 1, 2),
                Node::leaf(1, 999 * SCALE), // Different value
                Node::leaf(2, 200 * SCALE),
            ],
            SCALE,
        );
        let model2 = Model::new(vec![tree], 0);

        let hash1 = model1.hash_hex().unwrap();
        let hash2 = model2.hash_hex().unwrap();

        // Different models should have different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_save_load_json() {
        use tempfile::NamedTempFile;

        let model = create_test_model();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Save
        model.save_json(path).unwrap();

        // Load
        let loaded = Model::load_json(path).unwrap();

        // Should be identical
        assert_eq!(model, loaded);

        // Hash should match
        assert_eq!(model.hash_hex().unwrap(), loaded.hash_hex().unwrap());
    }

    #[test]
    fn test_model_validation() {
        // Valid model
        let valid = create_test_model();
        assert!(valid.validate().is_ok());

        // Invalid scale
        let mut invalid = create_test_model();
        invalid.scale = 0;
        assert!(invalid.validate().is_err());

        // Invalid version
        let mut invalid = create_test_model();
        invalid.version = 999;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_inference_exact_integers() {
        // Create a simple model with known integer outputs
        let tree = Tree::new(
            vec![
                Node::internal(0, 0, 0, 1, 2),
                Node::leaf(1, -1000),
                Node::leaf(2, 2000),
            ],
            SCALE,
        );

        let model = Model::new(vec![tree], 500);

        // Test with feature < 0 (goes left)
        let score1 = model.score(&[-100]);
        assert_eq!(score1, -1000 + 500); // -1000 * 1M / 1M + 500 = -500

        // Test with feature > 0 (goes right)
        let score2 = model.score(&[100]);
        assert_eq!(score2, 2000 + 500); // 2000 * 1M / 1M + 500 = 2500
    }
}
