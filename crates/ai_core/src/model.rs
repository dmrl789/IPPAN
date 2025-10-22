use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use std::collections::HashMap;

/// A single decision tree node in the GBDT model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Feature index to compare against
    pub feature_index: usize,
    /// Threshold value for comparison
    pub threshold: i64,
    /// Left child node index (if not a leaf)
    pub left: u32,
    /// Right child node index (if not a leaf)
    pub right: u32,
    /// Leaf value (if this is a leaf node)
    pub value: Option<i32>,
}

/// A single decision tree in the GBDT model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    /// Nodes in the tree (indexed by position)
    pub nodes: Vec<Node>,
}

/// A complete GBDT model for reputation scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model version for compatibility
    pub version: u32,
    /// Number of input features expected
    pub feature_count: usize,
    /// Base bias added to all predictions
    pub bias: i32,
    /// Maximum possible score (for clamping)
    pub scale: i32,
    /// Decision trees in the ensemble
    pub trees: Vec<Tree>,
    /// Model metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Model package containing the model and verification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPackage {
    /// The actual model
    pub model: Model,
    /// SHA-256 hash of the model for verification
    #[serde(with = "serde_bytes")]
    pub hash_sha256: [u8; 32],
    /// Ed25519 signature of the model hash
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// Public key that signed the model
    #[serde(with = "serde_bytes")]
    pub signer_pubkey: [u8; 32],
    /// Creation timestamp
    pub created_at: u64,
}

impl Model {
    /// Create a new model with the given parameters
    pub fn new(
        version: u32,
        feature_count: usize,
        bias: i32,
        scale: i32,
        trees: Vec<Tree>,
    ) -> Self {
        Self {
            version,
            feature_count,
            bias,
            scale,
            trees,
            metadata: HashMap::new(),
        }
    }

    /// Compute the SHA-256 hash of the model
    pub fn compute_hash(&self) -> [u8; 32] {
        let serialized = serde_json::to_vec(self).expect("Failed to serialize model");
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }

    /// Validate the model structure
    pub fn validate(&self) -> Result<()> {
        if self.trees.is_empty() {
            return Err(anyhow::anyhow!("Model has no trees"));
        }

        if self.feature_count == 0 {
            return Err(anyhow::anyhow!("Model has no features"));
        }

        for (tree_idx, tree) in self.trees.iter().enumerate() {
            if tree.nodes.is_empty() {
                return Err(anyhow::anyhow!("Tree {} has no nodes", tree_idx));
            }

            for (node_idx, node) in tree.nodes.iter().enumerate() {
                // Check if this is a leaf node
                if node.value.is_some() {
                    // Leaf node - left and right should be 0
                    if node.left != 0 || node.right != 0 {
                        return Err(anyhow::anyhow!(
                            "Leaf node {} in tree {} has non-zero children",
                            node_idx, tree_idx
                        ));
                    }
                } else {
                    // Internal node - must have valid children
                    if node.left as usize >= tree.nodes.len() {
                        return Err(anyhow::anyhow!(
                            "Node {} in tree {} has invalid left child {}",
                            node_idx, tree_idx, node.left
                        ));
                    }
                    if node.right as usize >= tree.nodes.len() {
                        return Err(anyhow::anyhow!(
                            "Node {} in tree {} has invalid right child {}",
                            node_idx, tree_idx, node.right
                        ));
                    }
                    if node.feature_index >= self.feature_count {
                        return Err(anyhow::anyhow!(
                            "Node {} in tree {} has invalid feature index {}",
                            node_idx, tree_idx, node.feature_index
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

impl ModelPackage {
    /// Create a new model package
    pub fn new(model: Model, signature: [u8; 64], signer_pubkey: [u8; 32]) -> Self {
        let hash_sha256 = model.compute_hash();
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            model,
            hash_sha256,
            signature,
            signer_pubkey,
            created_at,
        }
    }

    /// Verify the model package signature
    pub fn verify_signature(&self) -> Result<bool> {
        use ed25519_dalek::{VerifyingKey, Signature, Verifier};
        
        let verifying_key = VerifyingKey::from_bytes(&self.signer_pubkey)
            .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;
        
        let signature = Signature::from_bytes(&self.signature);
        
        Ok(verifying_key.verify(&self.hash_sha256, &signature).is_ok())
    }

    /// Load a model package from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        let package: ModelPackage = serde_json::from_str(json)?;
        package.model.validate()?;
        Ok(package)
    }

    /// Serialize the model package to JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_model() -> Model {
        Model::new(
            1,
            3,
            100,
            10000,
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
                        value: Some(1000),
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(500),
                    },
                ],
            }],
        )
    }

    #[test]
    fn test_model_validation() {
        let model = create_test_model();
        assert!(model.validate().is_ok());
    }

    #[test]
    fn test_model_hash() {
        let model1 = create_test_model();
        let model2 = create_test_model();
        
        let hash1 = model1.compute_hash();
        let hash2 = model2.compute_hash();
        
        // Same model should produce same hash
        assert_eq!(hash1, hash2);
        
        // Hash should be 32 bytes
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_invalid_model_empty_trees() {
        let model = Model::new(1, 3, 100, 10000, vec![]);
        assert!(model.validate().is_err());
    }

    #[test]
    fn test_invalid_model_invalid_feature_index() {
        let model = Model::new(
            1,
            3,
            100,
            10000,
            vec![Tree {
                nodes: vec![Node {
                    feature_index: 5, // Invalid: only 3 features (0, 1, 2)
                    threshold: 50,
                    left: 1,
                    right: 2,
                    value: None,
                }],
            }],
        );
        assert!(model.validate().is_err());
    }
}