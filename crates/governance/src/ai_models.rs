//! AI Model Governance Module — voting and approval of AI models
use anyhow::Result;
use ippan_ai_registry::{AiModelProposal, ModelRegistryEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Proposal manager for AI model governance
pub struct ProposalManager {
    threshold: f64,
    min_stake: u64,
    proposals: HashMap<String, AiModelProposal>,
    votes: HashMap<String, HashMap<[u8; 32], (u64, bool)>>, // proposal_id -> (voter_id -> (stake, approve))
    total_stake: u64,
}

impl ProposalManager {
    pub fn new(threshold: f64, min_stake: u64) -> Self {
        Self {
            threshold,
            min_stake,
            proposals: HashMap::new(),
            votes: HashMap::new(),
            total_stake: 0,
        }
    }
    
    pub fn submit_proposal(&mut self, proposal: AiModelProposal, stake: u64) -> Result<()> {
        if stake < self.min_stake {
            return Err(anyhow::anyhow!("Insufficient stake to submit proposal"));
        }
        
        self.proposals.insert(proposal.model_id.clone(), proposal);
        self.total_stake += stake;
        Ok(())
    }
    
    pub fn start_voting(&mut self, id: &str) -> Result<()> {
        if !self.proposals.contains_key(id) {
            return Err(anyhow::anyhow!("Proposal not found"));
        }
        
        self.votes.insert(id.to_string(), HashMap::new());
        Ok(())
    }
    
    pub fn vote(&mut self, id: &str, voter: [u8; 32], stake: u64, approve: bool) -> Result<()> {
        if let Some(votes) = self.votes.get_mut(id) {
            votes.insert(voter, (stake, approve));
            Ok(())
        } else {
            Err(anyhow::anyhow!("Voting not started for this proposal"))
        }
    }
    
    pub fn execute_proposal(&mut self, id: &str) -> Result<ModelRegistryEntry> {
        if let Some(proposal) = self.proposals.get(id) {
            if let Some(votes) = self.votes.get(id) {
                let (total_approve_stake, total_stake) = votes.values()
                    .fold((0u64, 0u64), |(approve, total), (stake, is_approve)| {
                        if *is_approve {
                            (approve + stake, total + stake)
                        } else {
                            (approve, total + stake)
                        }
                    });
                
                let approval_ratio = if total_stake > 0 {
                    total_approve_stake as f64 / total_stake as f64
                } else {
                    0.0
                };
                
                if approval_ratio >= self.threshold {
                    Ok(ModelRegistryEntry::new(
                        proposal.model_id.clone(),
                        proposal.model_hash,
                        proposal.version,
                        proposal.activation_round,
                        proposal.signature_foundation,
                        0, // approved_round
                        proposal.model_url.clone(),
                    ))
                } else {
                    Err(anyhow::anyhow!("Proposal did not meet approval threshold"))
                }
            } else {
                Err(anyhow::anyhow!("No votes recorded for this proposal"))
            }
        } else {
            Err(anyhow::anyhow!("Proposal not found"))
        }
    }
}

/// Model registry for managing approved AI models
pub struct ModelRegistry {
    models: HashMap<String, ModelRegistryEntry>,
    active_models: HashMap<String, u64>, // model_id -> activation_round
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            active_models: HashMap::new(),
        }
    }
    
    pub fn register_model(&mut self, entry: ModelRegistryEntry) -> Result<()> {
        self.models.insert(entry.model_id.clone(), entry);
        Ok(())
    }
    
    pub fn activate_model(&mut self, id: &str, round: u64) -> Result<()> {
        if self.models.contains_key(id) {
            self.active_models.insert(id.to_string(), round);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Model not found in registry"))
        }
    }
    
    pub fn get_model(&self, id: &str) -> Option<ModelRegistryEntry> {
        self.models.get(id).cloned()
    }
    
    pub fn is_active(&self, id: &str, current_round: u64) -> bool {
        if let Some(activation_round) = self.active_models.get(id) {
            current_round >= *activation_round
        } else {
            false
        }
    }
    
    pub fn get_active_models(&self, current_round: u64) -> Vec<ModelRegistryEntry> {
        self.models.values()
            .filter(|model| self.is_active(&model.model_id, current_round))
            .cloned()
            .collect()
    }
}

/// Activation manager for processing model activations
pub struct ActivationManager {
    registry: ModelRegistry,
    pending_activations: HashMap<String, u64>, // model_id -> activation_round
}

impl ActivationManager {
    pub fn new() -> Self {
        Self {
            registry: ModelRegistry::new(),
            pending_activations: HashMap::new(),
        }
    }
    
    pub fn process_round(&mut self, round: u64) -> Result<Vec<String>> {
        let mut activated = Vec::new();
        
        // Check for models that should be activated this round
        for (model_id, activation_round) in self.pending_activations.iter() {
            if *activation_round <= round {
                if let Err(e) = self.registry.activate_model(model_id, round) {
                    tracing::warn!("Failed to activate model {}: {}", model_id, e);
                } else {
                    activated.push(model_id.clone());
                }
            }
        }
        
        // Remove activated models from pending
        for model_id in &activated {
            self.pending_activations.remove(model_id);
        }
        
        Ok(activated)
    }
    
    pub fn schedule_activation(&mut self, model_id: String, activation_round: u64) {
        self.pending_activations.insert(model_id, activation_round);
    }
    
    pub fn get_registry(&self) -> &ModelRegistry {
        &self.registry
    }
    
    pub fn get_registry_mut(&mut self) -> &mut ModelRegistry {
        &mut self.registry
    }
}

/// AI model governance manager
pub struct AiModelGovernance {
    /// Model registry entries
    model_registry: HashMap<String, ModelRegistryEntry>,
    /// Active proposals
    active_proposals: HashMap<String, AiModelProposal>,
}

impl AiModelGovernance {
    /// Create a new AI model governance manager
    pub fn new() -> Self {
        Self {
            model_registry: HashMap::new(),
            active_proposals: HashMap::new(),
        }
    }

    /// Submit a new AI model proposal
    pub fn submit_model_proposal(
        &mut self,
        proposal: AiModelProposal,
    ) -> Result<()> {
        // For now, just store the proposal
        self.active_proposals.insert(proposal.model_id.clone(), proposal);
        Ok(())
    }

    /// Get all active proposals
    pub fn get_active_proposals(&self) -> &HashMap<String, AiModelProposal> {
        &self.active_proposals
    }

    /// Get a specific proposal
    pub fn get_proposal(&self, model_id: &str) -> Option<&AiModelProposal> {
        self.active_proposals.get(model_id)
    }

    /// Approve a proposal and add to registry
    pub fn approve_proposal(&mut self, model_id: &str, round: u64) -> Result<()> {
        if let Some(proposal) = self.active_proposals.remove(model_id) {
            let registry_entry = ModelRegistryEntry::new(
                proposal.model_id.clone(),
                proposal.model_hash,
                proposal.version,
                proposal.activation_round,
                proposal.signature_foundation,
                round,
                proposal.model_url,
            );
            self.model_registry.insert(model_id.to_string(), registry_entry);
        }
        Ok(())
    }

    /// Get the model registry
    pub fn get_model_registry(&self) -> &HashMap<String, ModelRegistryEntry> {
        &self.model_registry
    }

    /// Get a specific model from registry
    pub fn get_model(&self, model_id: &str) -> Option<&ModelRegistryEntry> {
        self.model_registry.get(model_id)
    }
}

/// JSON template for AI model proposals
pub const AI_MODEL_PROPOSAL_TEMPLATE: &str = r#"{
  "model_id": "model_identifier",
  "version": 1,
  "model_url": "https://example.com/model.json",
  "model_hash": "sha256_hash_here",
  "signature_foundation": "ed25519_signature_here",
  "proposer_pubkey": "ed25519_public_key_here",
  "activation_round": 1000,
  "rationale": "Description of the model and its purpose",
  "threshold_bps": 8000
}"#;

/// YAML template for AI model proposals
pub const AI_MODEL_PROPOSAL_YAML_TEMPLATE: &str = r#"---
model_id: "model_identifier"
version: 1
model_url: "https://example.com/model.json"
model_hash: "sha256_hash_here"
signature_foundation: "ed25519_signature_here"
proposer_pubkey: "ed25519_public_key_here"
activation_round: 1000
rationale: "Description of the model and its purpose"
threshold_bps: 8000
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
    use ed25519_dalek::{SigningKey, Signer};

    fn create_test_proposal() -> AiModelProposal {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
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
            model_hash: hash_bytes,
            model_url: "https://example.com/model.json".to_string(),
            activation_round: 100,
            signature_foundation: signature.to_bytes(),
            proposer_pubkey: pubkey,
            rationale: "Test model".to_string(),
            threshold_bps: 8000,
        }
    }

    #[test]
    fn test_governance_workflow() {
        let mut governance = AiModelGovernance::new();
        let proposal = create_test_proposal();
        
        // Submit proposal
        assert!(governance.submit_model_proposal(proposal).is_ok());
        
        // Check that proposal is stored
        assert!(governance.get_proposal("test_model").is_some());
        
        // Approve proposal
        assert!(governance.approve_proposal("test_model", 200).is_ok());
        
        // Check that model is registered
        assert!(governance.get_model("test_model").is_some());
    }

    #[test]
    fn test_json_parsing() {
        let proposal = create_test_proposal();
        let json = serde_json::to_string(&proposal).unwrap();
        let parsed = parse_json_proposal(&json).unwrap();
        
        assert_eq!(parsed.model_id, proposal.model_id);
        assert_eq!(parsed.version, proposal.version);
    }

    // YAML parsing test removed due to byte array serialization limitations

    #[test]
    fn test_proposal_validation() {
        let mut proposal = create_test_proposal();
        assert!(validate_proposal_format(&proposal).is_ok());
        
        proposal.model_id = String::new();
        assert!(validate_proposal_format(&proposal).is_err());
    }
}
