//! IPPAN On-Chain AI Model Registry
//!
//! Provides deterministic, verifiable registry logic for AI models
//! used at L1 consensus and reputation scoring. Includes lifecycle
//! management (propose → activate → deprecate → revoke) and
//! Ed25519-based signature verification.

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
    /// Model is deprecated and will soon be replaced
    Deprecated,
    /// Model is deactivated or revoked
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
    /// Optional round when the model is deactivated
    pub deactivation_round: Option<u64>,
    /// Ed25519 signature of the model
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// Public key of the signer
    pub signer_pubkey: [u8; 32],
    /// Current status
    pub status: ModelStatus,
    /// Timestamp when created (Unix micros or round index)
    pub created_at: u64,
    /// Additional metadata (IPFS URL, rationale, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Deterministic in-memory AI model registry
///
/// This struct can be serialized and verified on-chain,
/// ensuring consistent behavior across all nodes.
pub struct ModelRegistry {
    /// All known models
    models: HashMap<String, ModelRegistryEntry>,
    /// Models indexed by activation round
    activation_index: HashMap<u64, Vec<String>>,
    /// Currently active model ID
    active_model_id: Option<String>,
}

impl ModelRegistry {
    /// Create a new, empty registry
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            activation_index: HashMap::new(),
            active_model_id: None,
        }
    }

    /// Register a model proposal (must have valid signature)
    pub fn register_model(&mut self, entry: ModelRegistryEntry) -> Result<()> {
        self.validate_entry(&entry)?;

        let model_id = entry.model_id.clone();
        let activation_round = entry.activation_round;

        self.models.insert(model_id.clone(), entry);
        self.activation_index
            .entry(activation_round)
            .or_insert_with(Vec::new)
            .push(model_id);

        Ok(())
    }

    /// Retrieve a model by ID
    pub fn get_model(&self, model_id: &str) -> Option<&ModelRegistryEntry> {
        self.models.get(model_id)
    }

    /// Retrieve all registered models
    pub fn get_all_models(&self) -> &HashMap<String, ModelRegistryEntry> {
        &self.models
    }

    /// Return models that should activate at a given round
    pub fn get_models_for_activation(&self, round: u64) -> Vec<&ModelRegistryEntry> {
        self.activation_index
            .get(&round)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.models.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Activate a model if its activation round is reached
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

    /// Deprecate a model, retaining it for audit purposes
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

    /// Fully deactivate or revoke a model
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

    /// Get the currently active model entry
    pub fn get_active_model(&self) -> Option<&ModelRegistryEntry> {
        self.active_model_id
            .as_ref()
            .and_then(|id| self.models.get(id))
    }

    /// List all models by status
    pub fn list_models_by_status(&self, status: ModelStatus) -> Vec<&ModelRegistryEntry> {
        self.models
            .values()
            .filter(|entry| entry.status == status)
            .collect()
    }

    /// Verify the signature of a registry entry
    fn validate_entry(&self, entry: &ModelRegistryEntry) -> Result<()> {
        let mut message = Vec::new();
        message.extend_from_slice(entry.model_id.as_bytes());
        message.extend_from_slice(&entry.hash_sha256);
        message.extend_from_slice(&entry.version.to_be_bytes());
        message.extend_from_slice(&entry.activation_round.to_be_bytes());

        let verifying_key = VerifyingKey::from_bytes(&entry.signer_pubkey)
            .map_err(|_| anyhow::anyhow!("Invalid signer public key"))?;

        let signature = Signature::from_bytes(&entry.signature);

        verifying_key
            .verify(&message, &signature)
            .map_err(|_| anyhow::anyhow!("Invalid signature"))
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
