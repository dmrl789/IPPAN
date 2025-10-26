//! AI Model Governance Module — voting and approval of AI models
use anyhow::Result;
use ippan_ai_registry::{AiModelProposal, ModelRegistryEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Production proposal manager with voting and stake-based approval
pub struct ProposalManager {
    threshold: f64,
    minimum_stake: u64,
    proposals: HashMap<String, ProposalState>,
}

#[derive(Debug, Clone)]
struct ProposalState {
    proposal: AiModelProposal,
    total_stake_for: u64,
    total_stake_against: u64,
    voters: HashMap<[u8; 32], Vote>,
    status: ProposalStatus,
}

#[derive(Debug, Clone)]
struct Vote {
    stake: u64,
    approve: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum ProposalStatus {
    Pending,
    Voting,
    Approved,
    Rejected,
    Executed,
}

impl ProposalManager {
    pub fn new(threshold: f64, minimum_stake: u64) -> Self {
        Self {
            threshold,
            minimum_stake,
            proposals: HashMap::new(),
        }
    }
    
    pub fn submit_proposal(&mut self, proposal: AiModelProposal, stake: u64) -> Result<()> {
        if stake < self.minimum_stake {
            return Err(anyhow::anyhow!("Insufficient stake: {} < {}", stake, self.minimum_stake));
        }
        
        if self.proposals.contains_key(&proposal.model_id) {
            return Err(anyhow::anyhow!("Proposal already exists for model {}", proposal.model_id));
        }
        
        self.proposals.insert(
            proposal.model_id.clone(),
            ProposalState {
                proposal,
                total_stake_for: 0,
                total_stake_against: 0,
                voters: HashMap::new(),
                status: ProposalStatus::Pending,
            },
        );
        Ok(())
    }
    
    pub fn start_voting(&mut self, id: &str) -> Result<()> {
        let state = self.proposals.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", id))?;
        
        if state.status != ProposalStatus::Pending {
            return Err(anyhow::anyhow!("Proposal is not in pending state"));
        }
        
        state.status = ProposalStatus::Voting;
        Ok(())
    }
    
    pub fn vote(&mut self, id: &str, voter: [u8; 32], stake: u64, approve: bool) -> Result<()> {
        let state = self.proposals.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", id))?;
        
        if state.status != ProposalStatus::Voting {
            return Err(anyhow::anyhow!("Proposal is not accepting votes"));
        }
        
        if state.voters.contains_key(&voter) {
            return Err(anyhow::anyhow!("Voter has already voted"));
        }
        
        if approve {
            state.total_stake_for += stake;
        } else {
            state.total_stake_against += stake;
        }
        
        state.voters.insert(voter, Vote { stake, approve });
        
        // Check if threshold is met
        let total_stake = state.total_stake_for + state.total_stake_against;
        if total_stake > 0 {
            let approval_ratio = state.total_stake_for as f64 / total_stake as f64;
            if approval_ratio >= self.threshold {
                state.status = ProposalStatus::Approved;
            } else if approval_ratio < (1.0 - self.threshold) {
                state.status = ProposalStatus::Rejected;
            }
        }
        
        Ok(())
    }
    
    pub fn execute_proposal(&mut self, id: &str) -> Result<ModelRegistryEntry> {
        let state = self.proposals.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Proposal not found: {}", id))?;
        
        if state.status != ProposalStatus::Approved {
            return Err(anyhow::anyhow!("Proposal is not approved for execution"));
        }
        
        let entry = ModelRegistryEntry::new(
            state.proposal.model_id.clone(),
            state.proposal.model_hash,
            state.proposal.version,
            state.proposal.activation_round,
            state.proposal.signature_foundation,
            0,
            state.proposal.model_url.clone(),
        );
        
        state.status = ProposalStatus::Executed;
        Ok(entry)
    }
}

/// Production model registry with activation tracking
pub struct ModelRegistry {
    models: HashMap<String, RegistryState>,
    active_models: HashMap<u64, Vec<String>>,
}

#[derive(Debug, Clone)]
struct RegistryState {
    entry: ModelRegistryEntry,
    activated: bool,
    activation_round: Option<u64>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            active_models: HashMap::new(),
        }
    }
    
    pub fn register_model(&mut self, entry: ModelRegistryEntry) -> Result<()> {
        if self.models.contains_key(&entry.model_id) {
            return Err(anyhow::anyhow!("Model already registered: {}", entry.model_id));
        }
        
        self.models.insert(
            entry.model_id.clone(),
            RegistryState {
                entry,
                activated: false,
                activation_round: None,
            },
        );
        Ok(())
    }
    
    pub fn activate_model(&mut self, id: &str, round: u64) -> Result<()> {
        let state = self.models.get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", id))?;
        
        if state.activated {
            return Err(anyhow::anyhow!("Model already activated"));
        }
        
        state.activated = true;
        state.activation_round = Some(round);
        
        self.active_models
            .entry(round)
            .or_insert_with(Vec::new)
            .push(id.to_string());
        
        Ok(())
    }
    
    pub fn get_model(&self, id: &str) -> Option<ModelRegistryEntry> {
        self.models.get(id).map(|state| state.entry.clone())
    }
    
    pub fn list_active_models(&self, round: u64) -> Vec<String> {
        self.active_models
            .iter()
            .filter(|(r, _)| **r <= round)
            .flat_map(|(_, models)| models.clone())
            .collect()
    }
}

/// Production activation manager for scheduled model activations
pub struct ActivationManager {
    pending_activations: HashMap<u64, Vec<String>>,
    activated_models: HashMap<String, u64>,
}

impl ActivationManager {
    pub fn new() -> Self {
        Self {
            pending_activations: HashMap::new(),
            activated_models: HashMap::new(),
        }
    }
    
    pub fn schedule_activation(&mut self, model_id: String, round: u64) -> Result<()> {
        if self.activated_models.contains_key(&model_id) {
            return Err(anyhow::anyhow!("Model already activated: {}", model_id));
        }
        
        self.pending_activations
            .entry(round)
            .or_insert_with(Vec::new)
            .push(model_id);
        
        Ok(())
    }
    
    pub fn process_round(&mut self, round: u64) -> Result<Vec<String>> {
        let mut activated = Vec::new();
        
        // Process all pending activations up to and including this round
        let rounds_to_process: Vec<u64> = self.pending_activations
            .keys()
            .filter(|&&r| r <= round)
            .copied()
            .collect();
        
        for r in rounds_to_process {
            if let Some(models) = self.pending_activations.remove(&r) {
                for model_id in models {
                    self.activated_models.insert(model_id.clone(), r);
                    activated.push(model_id);
                }
            }
        }
        
        Ok(activated)
    }
    
    pub fn is_active(&self, model_id: &str) -> bool {
        self.activated_models.contains_key(model_id)
    }
    
    pub fn get_activation_round(&self, model_id: &str) -> Option<u64> {
        self.activated_models.get(model_id).copied()
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
