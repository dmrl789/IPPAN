use anyhow::Result;
use serde::{Deserialize, Serialize};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use std::collections::HashMap;

/// Status of a model in the registry
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelStatus {
    /// Model is proposed but not yet activated
    Proposed,
    /// Model is active and can be used for consensus
    Active,
    /// Model is deprecated and will be deactivated
    Deprecated,
    /// Model is deactivated and cannot be used
    Deactivated,
}

/// Entry in the AI model registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryEntry {
    /// Unique model identifier
    pub model_id: String,
    /// SHA-256 hash of the model
    pub hash_sha256: [u8; 32],
    /// Model version
    pub version: u32,
    /// Round when the model becomes active
    pub activation_round: u64,
    /// Round when the model becomes deactivated (if any)
    pub deactivation_round: Option<u64>,
    /// Ed25519 signature of the model
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// Public key that signed the model
    pub signer_pubkey: [u8; 32],
    /// Current status of the model
    pub status: ModelStatus,
    /// Creation timestamp
    pub created_at: u64,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// On-chain AI model registry
pub struct ModelRegistry {
    /// Registered models indexed by model_id
    models: HashMap<String, ModelRegistryEntry>,
    /// Models indexed by activation round for efficient lookup
    activation_index: HashMap<u64, Vec<String>>,
    /// Current active model ID
    active_model_id: Option<String>,
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            activation_index: HashMap::new(),
            active_model_id: None,
        }
    }

    /// Register a new model
    pub fn register_model(&mut self, entry: ModelRegistryEntry) -> Result<()> {
        // Validate the entry
        self.validate_entry(&entry)?;
        
        let model_id = entry.model_id.clone();
        let activation_round = entry.activation_round;
        
        // Add to models map
        self.models.insert(model_id.clone(), entry);
        
        // Add to activation index
        self.activation_index
            .entry(activation_round)
            .or_insert_with(Vec::new)
            .push(model_id);
        
        Ok(())
    }

    /// Get a model by ID
    pub fn get_model(&self, model_id: &str) -> Option<&ModelRegistryEntry> {
        self.models.get(model_id)
    }

    /// Get all models
    pub fn get_all_models(&self) -> &HashMap<String, ModelRegistryEntry> {
        &self.models
    }

    /// Get models that should be activated at a given round
    pub fn get_models_for_activation(&self, round: u64) -> Vec<&ModelRegistryEntry> {
        self.activation_index
            .get(&round)
            .map(|model_ids| {
                model_ids
                    .iter()
                    .filter_map(|id| self.models.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Activate a model
    pub fn activate_model(&mut self, model_id: &str, round: u64) -> Result<()> {
        if let Some(entry) = self.models.get_mut(model_id) {
            if entry.activation_round <= round {
                entry.status = ModelStatus::Active;
                self.active_model_id = Some(model_id.to_string());
                Ok(())
            } else {
                Err(anyhow::anyhow!("Model not ready for activation at round {}", round))
            }
        } else {
            Err(anyhow::anyhow!("Model {} not found", model_id))
        }
    }

    /// Deprecate a model
    pub fn deprecate_model(&mut self, model_id: &str) -> Result<()> {
        if let Some(entry) = self.models.get_mut(model_id) {
            entry.status = ModelStatus::Deprecated;
            if self.active_model_id.as_ref() == Some(&model_id.to_string()) {
                self.active_model_id = None;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Model {} not found", model_id))
        }
    }

    /// Deactivate a model
    pub fn deactivate_model(&mut self, model_id: &str) -> Result<()> {
        if let Some(entry) = self.models.get_mut(model_id) {
            entry.status = ModelStatus::Deactivated;
            if self.active_model_id.as_ref() == Some(&model_id.to_string()) {
                self.active_model_id = None;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Model {} not found", model_id))
        }
    }

    /// Get the currently active model
    pub fn get_active_model(&self) -> Option<&ModelRegistryEntry> {
        self.active_model_id
            .as_ref()
            .and_then(|id| self.models.get(id))
    }

    /// Get models by status
    pub fn get_models_by_status(&self, status: ModelStatus) -> Vec<&ModelRegistryEntry> {
        self.models
            .values()
            .filter(|entry| entry.status == status)
            .collect()
    }

    /// Validate a registry entry
    fn validate_entry(&self, entry: &ModelRegistryEntry) -> Result<()> {
        // Check if model ID is unique
        if self.models.contains_key(&entry.model_id) {
            return Err(anyhow::anyhow!("Model ID {} already exists", entry.model_id));
        }

        // Validate signature
        use ed25519_dalek::{VerifyingKey, Signature};
        let verifying_key = VerifyingKey::from_bytes(&entry.signer_pubkey)
            .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;
        let signature = Signature::from_bytes(&entry.signature);
        
        if verifying_key.verify(&entry.hash_sha256, &signature).is_err() {
            return Err(anyhow::anyhow!("Invalid signature for model {}", entry.model_id));
        }

        Ok(())
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, Verifier};

    fn create_test_entry() -> ModelRegistryEntry {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey = signing_key.verifying_key().to_bytes();
        
        let model_data = b"test_model_data";
        let mut hasher = blake3::Hasher::new();
        hasher.update(model_data);
        let hash = hasher.finalize();
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(hash.as_bytes());
        
        let signature = signing_key.sign(&hash_bytes);
        
        ModelRegistryEntry {
            model_id: "test_model".to_string(),
            hash_sha256: hash_bytes,
            version: 1,
            activation_round: 100,
            deactivation_round: None,
            signature: signature.to_bytes(),
            signer_pubkey: pubkey,
            status: ModelStatus::Proposed,
            created_at: 1234567890,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_model_registration() {
        let mut registry = ModelRegistry::new();
        let entry = create_test_entry();
        
        assert!(registry.register_model(entry).is_ok());
        assert!(registry.get_model("test_model").is_some());
    }

    #[test]
    fn test_model_activation() {
        let mut registry = ModelRegistry::new();
        let entry = create_test_entry();
        
        registry.register_model(entry).unwrap();
        assert!(registry.activate_model("test_model", 100).is_ok());
        
        let active_model = registry.get_active_model();
        assert!(active_model.is_some());
        assert_eq!(active_model.unwrap().status, ModelStatus::Active);
    }

    #[test]
    fn test_duplicate_model_id() {
        let mut registry = ModelRegistry::new();
        let entry1 = create_test_entry();
        let mut entry2 = create_test_entry();
        entry2.model_id = "test_model".to_string(); // Same ID
        
        registry.register_model(entry1).unwrap();
        assert!(registry.register_model(entry2).is_err());
    }

    #[test]
    fn test_models_for_activation() {
        let mut registry = ModelRegistry::new();
        let mut entry = create_test_entry();
        entry.activation_round = 50;
        
        registry.register_model(entry).unwrap();
        
        let models = registry.get_models_for_activation(50);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "test_model");
    }
}