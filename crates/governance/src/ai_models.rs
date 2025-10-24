use anyhow::Result;
use ippan_ai_registry::{AiModelProposal, ModelRegistryEntry};
use std::collections::HashMap;

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
    pub fn submit_proposal(&mut self, proposal: AiModelProposal) -> Result<()> {
        let model_id = proposal.model_id.clone();
        self.active_proposals.insert(model_id, proposal);
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

    /// Reject a proposal
    pub fn reject_proposal(&mut self, model_id: &str) -> Result<()> {
        self.active_proposals.remove(model_id);
        Ok(())
    }

    /// Get all registered models
    pub fn get_registered_models(&self) -> &HashMap<String, ModelRegistryEntry> {
        &self.model_registry
    }

    /// Get a specific registered model
    pub fn get_registered_model(&self, model_id: &str) -> Option<&ModelRegistryEntry> {
        self.model_registry.get(model_id)
    }

    /// Check if a model is registered
    pub fn is_model_registered(&self, model_id: &str) -> bool {
        self.model_registry.contains_key(model_id)
    }

    /// Get the number of active proposals
    pub fn active_proposal_count(&self) -> usize {
        self.active_proposals.len()
    }

    /// Get the number of registered models
    pub fn registered_model_count(&self) -> usize {
        self.model_registry.len()
    }
}

impl Default for AiModelGovernance {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_ai_registry::{AiModelProposal, ModelRegistryEntry};

    fn create_test_proposal(model_id: &str) -> AiModelProposal {
        AiModelProposal {
            model_id: model_id.to_string(),
            model_hash: "test_hash".to_string(),
            version: "1.0.0".to_string(),
            activation_round: 100,
            signature_foundation: "test_sig".to_string(),
            proposer_pubkey: "test_pubkey".to_string(),
            threshold_bps: 6600,
            rationale: "Test proposal".to_string(),
            model_url: "https://example.com/model".to_string(),
        }
    }

    #[test]
    fn test_governance_creation() {
        let governance = AiModelGovernance::new();
        assert_eq!(governance.active_proposal_count(), 0);
        assert_eq!(governance.registered_model_count(), 0);
    }

    #[test]
    fn test_proposal_submission() {
        let mut governance = AiModelGovernance::new();
        let proposal = create_test_proposal("test_model");
        
        governance.submit_proposal(proposal).unwrap();
        assert_eq!(governance.active_proposal_count(), 1);
        assert!(governance.get_proposal("test_model").is_some());
    }

    #[test]
    fn test_proposal_approval() {
        let mut governance = AiModelGovernance::new();
        let proposal = create_test_proposal("test_model");
        
        governance.submit_proposal(proposal).unwrap();
        governance.approve_proposal("test_model", 200).unwrap();
        
        assert_eq!(governance.active_proposal_count(), 0);
        assert_eq!(governance.registered_model_count(), 1);
        assert!(governance.is_model_registered("test_model"));
    }

    #[test]
    fn test_proposal_rejection() {
        let mut governance = AiModelGovernance::new();
        let proposal = create_test_proposal("test_model");
        
        governance.submit_proposal(proposal).unwrap();
        governance.reject_proposal("test_model").unwrap();
        
        assert_eq!(governance.active_proposal_count(), 0);
        assert_eq!(governance.registered_model_count(), 0);
        assert!(!governance.is_model_registered("test_model"));
    }

    #[test]
    fn test_duplicate_proposal() {
        let mut governance = AiModelGovernance::new();
        let proposal1 = create_test_proposal("test_model");
        let proposal2 = create_test_proposal("test_model");
        
        governance.submit_proposal(proposal1).unwrap();
        governance.submit_proposal(proposal2).unwrap();
        
        // Second proposal should overwrite the first
        assert_eq!(governance.active_proposal_count(), 1);
    }

    #[test]
    fn test_nonexistent_proposal_operations() {
        let mut governance = AiModelGovernance::new();
        
        // These should not panic
        governance.approve_proposal("nonexistent", 100).unwrap();
        governance.reject_proposal("nonexistent").unwrap();
        
        assert_eq!(governance.active_proposal_count(), 0);
        assert_eq!(governance.registered_model_count(), 0);
    }

    #[test]
    fn test_proposal_format_validation() {
        let mut governance = AiModelGovernance::new();
        let mut proposal = create_test_proposal("test_model");
        
        // Valid proposal should be accepted
        governance.submit_proposal(proposal.clone()).unwrap();
        assert_eq!(governance.active_proposal_count(), 1);
        
        // Invalid proposal (empty model_id) should be rejected
        proposal.model_id = String::new();
        assert!(governance.submit_proposal(proposal).is_err());
    }
}
