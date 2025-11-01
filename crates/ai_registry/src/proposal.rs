use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI model proposal for governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelProposal {
    /// Unique proposal ID
    pub proposal_id: String,
    /// Model identifier
    pub model_id: String,
    /// Model version
    pub version: u32,
    /// URL or path to the model file
    pub model_url: String,
    /// SHA-256 hash of the model
    pub model_hash: [u8; 32],
    /// Ed25519 signature of the model hash
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// Public key that signed the model
    pub signer_pubkey: [u8; 32],
    /// Round when the model should be activated
    pub activation_round: u64,
    /// Proposal description
    pub description: String,
    /// Proposer address
    pub proposer: [u8; 32],
    /// Creation timestamp
    pub created_at: u64,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Status of a governance proposal
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Proposal is pending review
    Pending,
    /// Proposal is under voting
    Voting,
    /// Proposal has been approved
    Approved,
    /// Proposal has been rejected
    Rejected,
    /// Proposal has been executed
    Executed,
}

/// Governance proposal manager
pub struct ProposalManager {
    /// Active proposals
    proposals: HashMap<String, (AiModelProposal, ProposalStatus)>,
    /// Voting threshold (percentage of stake required)
    voting_threshold: f64,
    /// Minimum stake required to propose
    min_proposal_stake: u64,
}

impl ProposalManager {
    /// Create a new proposal manager
    pub fn new(voting_threshold: f64, min_proposal_stake: u64) -> Self {
        Self {
            proposals: HashMap::new(),
            voting_threshold,
            min_proposal_stake,
        }
    }

    /// Submit a new AI model proposal
    pub fn submit_proposal(
        &mut self,
        proposal: AiModelProposal,
        proposer_stake: u64,
    ) -> Result<()> {
        // Check minimum stake requirement
        if proposer_stake < self.min_proposal_stake {
            return Err(anyhow::anyhow!(
                "Insufficient stake to propose: {} < {}",
                proposer_stake,
                self.min_proposal_stake
            ));
        }

        // Validate proposal
        self.validate_proposal(&proposal)?;

        // Check for duplicate proposal ID
        if self.proposals.contains_key(&proposal.proposal_id) {
            return Err(anyhow::anyhow!(
                "Proposal ID {} already exists",
                proposal.proposal_id
            ));
        }

        // Add proposal as pending
        self.proposals.insert(
            proposal.proposal_id.clone(),
            (proposal, ProposalStatus::Pending),
        );

        Ok(())
    }

    /// Start voting on a proposal
    pub fn start_voting(&mut self, proposal_id: &str) -> Result<()> {
        if let Some((_, status)) = self.proposals.get_mut(proposal_id) {
            if *status == ProposalStatus::Pending {
                *status = ProposalStatus::Voting;
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Proposal {} is not in pending status",
                    proposal_id
                ))
            }
        } else {
            Err(anyhow::anyhow!("Proposal {} not found", proposal_id))
        }
    }

    /// Vote on a proposal
    pub fn vote(
        &mut self,
        proposal_id: &str,
        _voter: [u8; 32],
        _stake: u64,
        _approve: bool,
    ) -> Result<()> {
        if let Some((_proposal, status)) = self.proposals.get_mut(proposal_id) {
            if *status != ProposalStatus::Voting {
                return Err(anyhow::anyhow!(
                    "Proposal {} is not in voting status",
                    proposal_id
                ));
            }

            // In a real implementation, you would track votes here
            // For now, we'll just simulate the voting process
            Ok(())
        } else {
            Err(anyhow::anyhow!("Proposal {} not found", proposal_id))
        }
    }

    /// Execute a proposal (create registry entry)
    pub fn execute_proposal(
        &mut self,
        proposal_id: &str,
    ) -> Result<crate::types::ModelRegistration> {
        use chrono::{DateTime, Utc};
        use ippan_ai_core::types::{ModelId, ModelMetadata};

        if let Some((proposal, status)) = self.proposals.get_mut(proposal_id) {
            if *status != ProposalStatus::Approved {
                return Err(anyhow::anyhow!("Proposal {} is not approved", proposal_id));
            }

            // Convert timestamp to DateTime
            let timestamp = DateTime::from_timestamp(proposal.created_at as i64, 0)
                .unwrap_or_else(|| Utc::now());

            // Create ModelId from proposal
            let model_id = ModelId {
                name: proposal.model_id.clone(),
                version: proposal.version.to_string(),
                hash: hex::encode(&proposal.model_hash),
            };

            // Create ModelMetadata
            let metadata = ModelMetadata {
                id: model_id.clone(),
                name: proposal.model_id.clone(),
                version: proposal.version.to_string(),
                description: proposal.description.clone(),
                author: String::new(),
                license: String::new(),
                tags: Vec::new(),
                created_at: timestamp.timestamp() as u64,
                updated_at: timestamp.timestamp() as u64,
                architecture: String::from("gbdt"),
                input_shape: Vec::new(),
                output_shape: Vec::new(),
                size_bytes: 0,
                parameter_count: 0,
            };

            // Create registry entry
            let entry = crate::types::ModelRegistration {
                model_id,
                metadata,
                status: crate::types::RegistrationStatus::Pending,
                registrant: hex::encode(proposal.proposer),
                registered_at: timestamp,
                updated_at: timestamp,
                registration_fee: 0, // Placeholder - should be set by caller
                category: crate::types::ModelCategory::default(),
                tags: Vec::new(),
                description: Some(proposal.description.clone()),
                license: None,
                source_url: Some(proposal.model_url.clone()),
            };

            *status = ProposalStatus::Executed;
            Ok(entry)
        } else {
            Err(anyhow::anyhow!("Proposal {} not found", proposal_id))
        }
    }

    /// Get a proposal by ID
    pub fn get_proposal(&self, proposal_id: &str) -> Option<&(AiModelProposal, ProposalStatus)> {
        self.proposals.get(proposal_id)
    }

    /// Get all proposals
    pub fn get_all_proposals(&self) -> &HashMap<String, (AiModelProposal, ProposalStatus)> {
        &self.proposals
    }

    /// Get proposals by status
    pub fn get_proposals_by_status(
        &self,
        status: ProposalStatus,
    ) -> Vec<&(AiModelProposal, ProposalStatus)> {
        self.proposals
            .values()
            .filter(|(_, s)| *s == status)
            .collect()
    }

    /// Validate a proposal
    fn validate_proposal(&self, proposal: &AiModelProposal) -> Result<()> {
        // Validate signature
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        let verifying_key = VerifyingKey::from_bytes(&proposal.signer_pubkey)
            .map_err(|e| anyhow::anyhow!("Invalid public key: {}", e))?;
        let signature = Signature::from_bytes(&proposal.signature);

        if verifying_key
            .verify(&proposal.model_hash, &signature)
            .is_err()
        {
            return Err(anyhow::anyhow!(
                "Invalid signature for proposal {}",
                proposal.proposal_id
            ));
        }

        // Validate activation round is in the future
        let current_round = 0; // In real implementation, get from consensus
        if proposal.activation_round <= current_round {
            return Err(anyhow::anyhow!("Activation round must be in the future"));
        }

        Ok(())
    }
}

impl Default for ProposalManager {
    fn default() -> Self {
        Self::new(0.67, 1000000) // 67% threshold, 1M minimum stake
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

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
            proposal_id: "proposal_1".to_string(),
            model_id: "test_model".to_string(),
            version: 1,
            model_url: "https://example.com/model.json".to_string(),
            model_hash: hash_bytes,
            signature: signature.to_bytes(),
            signer_pubkey: pubkey,
            activation_round: 100,
            description: "Test model proposal".to_string(),
            proposer: [1u8; 32],
            created_at: 1234567890,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_proposal_submission() {
        let mut manager = ProposalManager::new(0.67, 1000000);
        let proposal = create_test_proposal();

        assert!(manager.submit_proposal(proposal, 2000000).is_ok());
        assert!(manager.get_proposal("proposal_1").is_some());
    }

    #[test]
    fn test_insufficient_stake() {
        let mut manager = ProposalManager::new(0.67, 1000000);
        let proposal = create_test_proposal();

        assert!(manager.submit_proposal(proposal, 500000).is_err());
    }

    #[test]
    fn test_duplicate_proposal() {
        let mut manager = ProposalManager::new(0.67, 1000000);
        let proposal1 = create_test_proposal();
        let mut proposal2 = create_test_proposal();
        proposal2.proposal_id = "proposal_1".to_string(); // Same ID

        manager.submit_proposal(proposal1, 2000000).unwrap();
        assert!(manager.submit_proposal(proposal2, 2000000).is_err());
    }
}
