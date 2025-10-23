use anyhow::Result;
use ippan_ai_registry::{AiModelProposal, ModelRegistryEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI model governance manager
pub struct AiModelGovernance {
    /// Model registry entries
    model_registry: HashMap<String, ModelRegistryEntry>,
    /// Active proposals
    active_proposals: HashMap<String, AiModelProposal>,
    /// Voting threshold (0.0 to 1.0)
    voting_threshold: f64,
    /// Minimum stake required for proposals
    min_proposal_stake: u64,
}

impl AiModelGovernance {
    /// Create a new AI model governance manager
    pub fn new(
        voting_threshold: f64,
        min_proposal_stake: u64,
    ) -> Self {
        Self {
            model_registry: HashMap::new(),
            active_proposals: HashMap::new(),
            voting_threshold,
            min_proposal_stake,
        }
    }

    /// Submit a new AI model proposal
    pub fn submit_model_proposal(
        &mut self,
        proposal: AiModelProposal,
        proposer_stake: u64,
    ) -> Result<()> {
        if proposer_stake < self.min_proposal_stake {
            return Err(anyhow::anyhow!("Insufficient stake for proposal"));
        }
        
        self.active_proposals.insert(proposal.model_id.clone(), proposal);
        Ok(())
    }

    /// Get a model from the registry
    pub fn get_model(&self, model_id: &str) -> Option<&ModelRegistryEntry> {
        self.model_registry.get(model_id)
    }

    /// Get all active proposals
    pub fn get_active_proposals(&self) -> &HashMap<String, AiModelProposal> {
        &self.active_proposals
    }

    /// Process model activations for the current round
    pub fn process_round(&mut self, round: u64) -> Result<Vec<String>> {
        let mut activated_models = Vec::new();
        
        for (model_id, entry) in &mut self.model_registry {
            if entry.activation_round == round && entry.status == ippan_ai_registry::ModelStatus::Approved {
                entry.activate(round);
                activated_models.push(model_id.clone());
            }
        }
        
        Ok(activated_models)
    }
}

/// JSON template for AI model proposals
pub const AI_MODEL_PROPOSAL_TEMPLATE: &str = r#"{
  "proposal_id": "unique_proposal_id",
  "model_id": "model_identifier",
  "version": 1,
  "model_url": "https://example.com/model.json",
  "model_hash": "sha256_hash_here",
  "signature": "ed25519_signature_here",
  "signer_pubkey": "ed25519_public_key_here",
  "activation_round": 1000,
  "description": "Description of the model and its purpose",
  "proposer": "proposer_address_here",
  "created_at": 1234567890,
  "metadata": {
    "category": "reputation_scoring",
    "performance_metrics": "accuracy: 0.95, precision: 0.93",
    "training_data": "historical_validator_data"
  }
}"#;

/// YAML template for AI model proposals
pub const AI_MODEL_PROPOSAL_YAML_TEMPLATE: &str = r#"---
proposal_id: "unique_proposal_id"
model_id: "model_identifier"
version: 1
model_url: "https://example.com/model.json"
model_hash: "sha256_hash_here"
signature: "ed25519_signature_here"
signer_pubkey: "ed25519_public_key_here"
activation_round: 1000
description: "Description of the model and its purpose"
proposer: "proposer_address_here"
created_at: 1234567890
metadata:
  category: "reputation_scoring"
  performance_metrics: "accuracy: 0.95, precision: 0.93"
  training_data: "historical_validator_data"
"#;

/// Parse a JSON proposal
pub fn parse_json_proposal(json: &str) -> Result<AiModelProposal> {
    let proposal: AiModelProposal = serde_json::from_str(json)?;
    Ok(proposal)
}

/// Parse a YAML proposal
pub fn parse_yaml_proposal(yaml: &str) -> Result<AiModelProposal> {
    let proposal: AiModelProposal = serde_yaml::from_str(yaml)?;
    Ok(proposal)
}

/// Validate a proposal format
pub fn validate_proposal_format(proposal: &AiModelProposal) -> Result<()> {
    if proposal.model_id.is_empty() {
        return Err(anyhow::anyhow!("Model ID cannot be empty"));
    }
    
    if proposal.model_url.is_empty() {
        return Err(anyhow::anyhow!("Model URL cannot be empty"));
    }
    
    if proposal.rationale.is_empty() {
        return Err(anyhow::anyhow!("Rationale cannot be empty"));
    }
    
    if proposal.activation_round == 0 {
        return Err(anyhow::anyhow!("Activation round must be greater than 0"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn create_test_proposal() -> AiModelProposal {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey = signing_key.verifying_key().to_bytes();
        
        let model_data = b"test_model_data";
        let mut hasher = blake3::Hasher::new();
        hasher.update(model_data);
        let hash = hasher.finalize();
        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(hash.as_bytes());
        
        let signature = signing_key.sign(&hash_bytes);
        
        AiModelProposal {
            model_id: "test_model".to_string(),
            version: 1,
            model_url: "https://example.com/model.json".to_string(),
            model_hash: hash_bytes,
            signature_foundation: signature.to_bytes(),
            proposer_pubkey: pubkey,
            activation_round: 100,
            rationale: "Test model".to_string(),
            threshold_bps: 8000,
        }
    }

    #[test]
    fn test_governance_workflow() {
        let mut governance = AiModelGovernance::new(0.67, 1000000);
        let proposal = create_test_proposal();
        
        // Submit proposal
        assert!(governance.submit_model_proposal(proposal, 2000000).is_ok());
        
        // Check that proposal is active
        assert!(governance.get_active_proposals().contains_key("test_model"));
    }

    #[test]
    fn test_json_parsing() {
        let proposal = create_test_proposal();
        let json = serde_json::to_string(&proposal).unwrap();
        let parsed = parse_json_proposal(&json).unwrap();
        
        assert_eq!(parsed.model_id, proposal.model_id);
        assert_eq!(parsed.version, proposal.version);
    }

    #[test]
    fn test_yaml_parsing() {
        let proposal = create_test_proposal();
        let yaml = serde_yaml::to_string(&proposal).unwrap();
        let parsed = parse_yaml_proposal(&yaml).unwrap();
        
        assert_eq!(parsed.model_id, proposal.model_id);
        assert_eq!(parsed.version, proposal.version);
    }

    #[test]
    fn test_proposal_validation() {
        let mut proposal = create_test_proposal();
        assert!(validate_proposal_format(&proposal).is_ok());
        
        proposal.model_id = String::new();
        assert!(validate_proposal_format(&proposal).is_err());
    }
}