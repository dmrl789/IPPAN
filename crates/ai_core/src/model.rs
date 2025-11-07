//! Model metadata and verification structures
//!
//! Provides deterministic model packaging, integrity verification, and
//! SHA-based hashing for L1-deployed AI models (GBDT, linear, etc.)

use crate::serialization::canonical_json_string;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const MODEL_HASH_SIZE: usize = 32;

/// High-level in-memory model representation used by logging and
/// deterministic evaluation helpers. Wraps a GBDT structure with
/// additional metadata required by L1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Model {
    /// Semantic model version
    pub version: u32,
    /// Number of expected features
    pub feature_count: usize,
    /// Base bias/intercept value
    pub bias: i32,
    /// Output scale (for fixed-point representation)
    pub scale: i32,
    /// Trees composing the model
    pub trees: Vec<crate::gbdt::Tree>,
}

impl Model {
    /// Construct a new model instance
    pub fn new(
        version: u32,
        feature_count: usize,
        bias: i32,
        scale: i32,
        trees: Vec<crate::gbdt::Tree>,
    ) -> Self {
        Self {
            version,
            feature_count,
            bias,
            scale,
            trees,
        }
    }

    /// Validate structural invariants of this model
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.feature_count == 0 {
            anyhow::bail!("Feature count cannot be zero");
        }
        if self.scale <= 0 {
            anyhow::bail!("Scale must be positive");
        }
        if self.trees.is_empty() {
            anyhow::bail!("Model must contain at least one tree");
        }
        // Basic node presence checks
        for (idx, tree) in self.trees.iter().enumerate() {
            if tree.nodes.is_empty() {
                anyhow::bail!("Tree {idx} is empty");
            }
        }
        Ok(())
    }
}

/// Metadata for an AI model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelMetadata {
    /// Model identifier
    pub model_id: String,
    /// Model version
    pub version: u32,
    /// Model type (e.g., "gbdt", "linear")
    pub model_type: String,
    /// SHA-256 hash of the model structure
    pub hash_sha256: [u8; MODEL_HASH_SIZE],
    /// Feature count expected by the model
    pub feature_count: usize,
    /// Output scale (for fixed-point representation)
    pub output_scale: i32,
    /// Minimum output value
    pub output_min: i32,
    /// Maximum output value
    pub output_max: i32,
}

/// Full model package including metadata and weights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackage {
    /// Model metadata
    pub metadata: ModelMetadata,
    /// Model structure (GBDT trees, etc.)
    pub model: crate::gbdt::GBDTModel,
}

/// Model loading errors
#[derive(thiserror::Error, Debug)]
pub enum ModelError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Model hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("Invalid model structure: {0}")]
    InvalidStructure(String),
}

/// Load model from JSON file
pub fn load_model(path: impl AsRef<Path>) -> Result<ModelPackage, ModelError> {
    let content = std::fs::read_to_string(path)?;
    let package: ModelPackage = serde_json::from_str(&content)?;

    // Structural validation
    if package.model.trees.is_empty() {
        return Err(ModelError::InvalidStructure("No trees in model".into()));
    }
    if package.metadata.feature_count == 0 {
        return Err(ModelError::InvalidStructure(
            "Feature count cannot be zero".into(),
        ));
    }

    Ok(package)
}

/// Verify model hash matches the computed hash
pub fn verify_model_hash(package: &ModelPackage) -> Result<(), ModelError> {
    let computed = compute_model_hash(&package.model);

    if computed != package.metadata.hash_sha256 {
        return Err(ModelError::HashMismatch {
            expected: hex::encode(package.metadata.hash_sha256),
            actual: hex::encode(computed),
        });
    }

    Ok(())
}

/// Compute deterministic hash of model structure
fn compute_model_hash(model: &crate::gbdt::GBDTModel) -> [u8; MODEL_HASH_SIZE] {
    #[derive(Serialize)]
    struct HashableMetadata<'a> {
        version: &'a str,
        created_at: u64,
        feature_count: usize,
        tree_count: usize,
        max_depth: usize,
        training_data_hash: &'a str,
        performance_metrics: &'a std::collections::HashMap<String, f64>,
    }

    #[derive(Serialize)]
    struct HashableModel<'a> {
        trees: &'a [crate::gbdt::Tree],
        bias: i32,
        scale: i32,
        metadata: HashableMetadata<'a>,
        feature_normalization: &'a Option<crate::gbdt::FeatureNormalization>,
        security_constraints: &'a crate::gbdt::SecurityConstraints,
    }

    let hashable = HashableModel {
        trees: &model.trees,
        bias: model.bias,
        scale: model.scale,
        metadata: HashableMetadata {
            version: &model.metadata.version,
            created_at: model.metadata.created_at,
            feature_count: model.metadata.feature_count,
            tree_count: model.metadata.tree_count,
            max_depth: model.metadata.max_depth,
            training_data_hash: &model.metadata.training_data_hash,
            performance_metrics: &model.metadata.performance_metrics,
        },
        feature_normalization: &model.feature_normalization,
        security_constraints: &model.security_constraints,
    };

    let json = canonical_json_string(&hashable).expect("Model serialization failed");
    let hash = Sha256::digest(json.as_bytes());
    let mut result = [0u8; MODEL_HASH_SIZE];
    result.copy_from_slice(&hash[..MODEL_HASH_SIZE]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbdt::{GBDTModel, Node, Tree};

    fn create_test_model() -> GBDTModel {
        GBDTModel::new(
            vec![Tree {
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
            }],
            0,
            100,
            1,
        )
        .expect("valid test model")
    }

    #[test]
    fn test_compute_model_hash() {
        let model = create_test_model();
        let hash = super::compute_model_hash(&model);
        assert_ne!(hash, [0u8; MODEL_HASH_SIZE]);
    }

    #[test]
    fn test_verify_model_hash_success() {
        let model = create_test_model();
        let hash = super::compute_model_hash(&model);
        let package = ModelPackage {
            metadata: ModelMetadata {
                model_id: "test".into(),
                version: 1,
                model_type: "gbdt".into(),
                hash_sha256: hash,
                feature_count: 1,
                output_scale: 100,
                output_min: 0,
                output_max: 100,
            },
            model,
        };
        assert!(verify_model_hash(&package).is_ok());
    }

    #[test]
    fn test_verify_model_hash_mismatch() {
        let model = create_test_model();
        let package = ModelPackage {
            metadata: ModelMetadata {
                model_id: "test".into(),
                version: 1,
                model_type: "gbdt".into(),
                hash_sha256: [1u8; MODEL_HASH_SIZE],
                feature_count: 1,
                output_scale: 100,
                output_min: 0,
                output_max: 100,
            },
            model,
        };
        assert!(verify_model_hash(&package).is_err());
    }

    #[test]
    fn test_load_model_structure() {
        let model = create_test_model();
        let metadata = ModelMetadata {
            model_id: "test".into(),
            version: 1,
            model_type: "gbdt".into(),
            hash_sha256: super::compute_model_hash(&model),
            feature_count: 1,
            output_scale: 100,
            output_min: 0,
            output_max: 100,
        };
        let package = ModelPackage { metadata, model };
        let json = serde_json::to_string(&package).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), json).unwrap();
        let loaded = load_model(tmp.path()).unwrap();
        assert_eq!(loaded.metadata.model_id, "test");
    }
}
