//! Model metadata and verification structures
//!
//! Provides deterministic model packaging, integrity verification, and
//! SHA-based hashing for L1-deployed AI models (GBDT, linear, etc.)

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MODEL_HASH_SIZE: usize = 32;

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
        return Err(ModelError::InvalidStructure("Feature count cannot be zero".into()));
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
    let json = serde_json::to_string(model).expect("Model serialization failed");
    let hash = blake3::hash(json.as_bytes());
    let mut result = [0u8; MODEL_HASH_SIZE];
    result.copy_from_slice(&hash.as_bytes()[..MODEL_HASH_SIZE]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gbdt::{GBDTModel, Node, Tree};

    fn create_test_model() -> GBDTModel {
        GBDTModel {
            trees: vec![Tree {
                nodes: vec![
                    Node { feature_index: 0, threshold: 50, left: 1, right: 2, value: None },
                    Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(10) },
                    Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(20) },
                ],
            }],
            bias: 0,
            scale: 100,
        }
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
