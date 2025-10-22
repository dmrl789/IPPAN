use anyhow::Result;
use ippan_ai_registry::{proposal::AiModelProposal, registry::ModelRegistryEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI model governance manager
pub struct AiModelGovernance {
    /// Proposal manager for AI models
    proposal_manager: ippan_ai_registry::proposal::ProposalManager,
    /// Model registry
    model_registry: ippan_ai_registry::registry::ModelRegistry,
    /// Activation manager
    activation_manager: ippan_ai_registry::activation::ActivationManager,
}

impl AiModelGovernance {
    /// Create a new AI model governance manager
    pub fn new(
        voting_threshold: f64,
        min_proposal_stake: u64,
    ) -> Self {
        Self {
            proposal_manager: ippan_ai_registry::proposal::ProposalManager::new(
                voting_threshold,
                min_proposal_stake,
            ),
            model_registry: ippan_ai_registry::registry::ModelRegistry::new(),
            activation_manager: ippan_ai_registry::activation::ActivationManager::new(),
        }
    }

    /// Submit a new AI model proposal
    pub fn submit_model_proposal(
        &mut self,
        proposal: AiModelProposal,
        proposer_stake: u64,
    ) -> Result<()> {
        self.proposal_manager.submit_proposal(proposal, proposer_stake)
    }

    /// Start voting on a proposal
    pub fn start_voting(&mut self, proposal_id: &str) -> Result<()> {
        self.proposal_manager.start_voting(proposal_id)
    }

    /// Vote on a proposal
    pub fn vote(
        &mut self,
        proposal_id: &str,
        voter: [u8; 32],
        stake: u64,
        approve: bool,
    ) -> Result<()> {
        self.proposal_manager.vote(proposal_id, voter, stake, approve)
    }

    /// Execute an approved proposal
    pub fn execute_proposal(&mut self, proposal_id: &str) -> Result<()> {
        let registry_entry = self.proposal_manager.execute_proposal(proposal_id)?;
        self.model_registry.register_model(registry_entry)?;
        Ok(())
    }

    /// Process model activations for the current round
    pub fn process_round(&mut self, round: u64) -> Result<Vec<String>> {
        let activated_models = self.activation_manager.process_round(round)?;
        
        for model_id in &activated_models {
            self.model_registry.activate_model(model_id, round)?;
        }
        
        Ok(activated_models)
    }

    /// Get the model registry
    pub fn get_model_registry(&self) -> &ippan_ai_registry::registry::ModelRegistry {
        &self.model_registry
    }

    /// Get the proposal manager
    pub fn get_proposal_manager(&self) -> &ippan_ai_registry::proposal::ProposalManager {
        &self.proposal_manager
    }

    /// Get the activation manager
    pub fn get_activation_manager(&self) -> &ippan_ai_registry::activation::ActivationManager {
        &self.activation_manager
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
    if proposal.proposal_id.is_empty() {
        return Err(anyhow::anyhow!("Proposal ID cannot be empty"));
    }
    
    if proposal.model_id.is_empty() {
        return Err(anyhow::anyhow!("Model ID cannot be empty"));
    }
    
    if proposal.model_url.is_empty() {
        return Err(anyhow::anyhow!("Model URL cannot be empty"));
    }
    
    if proposal.description.is_empty() {
        return Err(anyhow::anyhow!("Description cannot be empty"));
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
            proposal_id: "test_proposal".to_string(),
            model_id: "test_model".to_string(),
            version: 1,
            model_url: "https://example.com/model.json".to_string(),
            model_hash: hash_bytes,
            signature: signature.to_bytes(),
            signer_pubkey: pubkey,
            activation_round: 100,
            description: "Test model".to_string(),
            proposer: [1u8; 32],
            created_at: 1234567890,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_governance_workflow() {
        let mut governance = AiModelGovernance::new(0.67, 1000000);
        let proposal = create_test_proposal();
        
        // Submit proposal
        assert!(governance.submit_model_proposal(proposal, 2000000).is_ok());
        
        // Start voting
        assert!(governance.start_voting("test_proposal").is_ok());
        
        // Vote
        assert!(governance.vote("test_proposal", [2u8; 32], 1000000, true).is_ok());
        
        // Execute proposal
        assert!(governance.execute_proposal("test_proposal").is_ok());
        
        // Check that model is registered
        let registry = governance.get_model_registry();
        assert!(registry.get_model("test_model").is_some());
    }

    #[test]
    fn test_json_parsing() {
        let proposal = create_test_proposal();
        let json = serde_json::to_string(&proposal).unwrap();
        let parsed = parse_json_proposal(&json).unwrap();
        
        assert_eq!(parsed.proposal_id, proposal.proposal_id);
        assert_eq!(parsed.model_id, proposal.model_id);
    }

    #[test]
    fn test_yaml_parsing() {
        let proposal = create_test_proposal();
        let yaml = serde_yaml::to_string(&proposal).unwrap();
        let parsed = parse_yaml_proposal(&yaml).unwrap();
        
        assert_eq!(parsed.proposal_id, proposal.proposal_id);
        assert_eq!(parsed.model_id, proposal.model_id);
    }

    #[test]
    fn test_proposal_validation() {
        let mut proposal = create_test_proposal();
        assert!(validate_proposal_format(&proposal).is_ok());
        
        proposal.proposal_id = String::new();
        assert!(validate_proposal_format(&proposal).is_err());
    }
}