use anyhow::Result;
use ed25519_dalek::Verifier;
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
    /// Voting threshold as scaled integer (0-10000 = 0%-100%)
    #[allow(dead_code)]
    voting_threshold: i64,
    /// Minimum stake required to propose
    min_proposal_stake: u64,
    /// Base registration fee (in micro-IPN)
    base_registration_fee: u64,
    /// Fee per MB of model size (in micro-IPN)
    fee_per_mb: u64,
}

impl ProposalManager {
    /// Create a new proposal manager
    /// voting_threshold: scaled by 10000 (e.g., 6667 = 66.67%)
    pub fn new(voting_threshold: i64, min_proposal_stake: u64) -> Self {
        Self {
            proposals: HashMap::new(),
            voting_threshold,
            min_proposal_stake,
            base_registration_fee: 1_000_000, // 1 IPN base fee
            fee_per_mb: 100_000,              // 0.1 IPN per MB
        }
    }

    /// Create with custom fee parameters
    /// voting_threshold: scaled by 10000 (e.g., 6667 = 66.67%)
    pub fn with_fees(
        voting_threshold: i64,
        min_proposal_stake: u64,
        base_registration_fee: u64,
        fee_per_mb: u64,
    ) -> Self {
        Self {
            proposals: HashMap::new(),
            voting_threshold,
            min_proposal_stake,
            base_registration_fee,
            fee_per_mb,
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

        // First, check status and clone needed data
        let (proposal_data, model_size) = {
            if let Some((proposal, status)) = self.proposals.get(proposal_id) {
                if *status != ProposalStatus::Approved {
                    return Err(anyhow::anyhow!("Proposal {} is not approved", proposal_id));
                }

                // Clone proposal data we need
                let data = (
                    proposal.clone(),
                    0u64, // size_bytes - will be calculated from model data in production
                );
                (data.0, data.1)
            } else {
                return Err(anyhow::anyhow!("Proposal {} not found", proposal_id));
            }
        };

        // Calculate fee (no borrow conflict now)
        let registration_fee = self.calculate_registration_fee(model_size);

        // Convert timestamp to DateTime
        let timestamp =
            DateTime::from_timestamp(proposal_data.created_at as i64, 0).unwrap_or_else(Utc::now);

        // Create ModelId from proposal
        let model_id = ModelId {
            name: proposal_data.model_id.clone(),
            version: proposal_data.version.to_string(),
            hash: hex::encode(proposal_data.model_hash),
        };

        // Create ModelMetadata
        let metadata = ModelMetadata {
            id: model_id.clone(),
            name: proposal_data.model_id.clone(),
            version: proposal_data.version.to_string(),
            description: proposal_data.description.clone(),
            author: String::new(),
            license: String::new(),
            tags: Vec::new(),
            created_at: timestamp.timestamp() as u64,
            updated_at: timestamp.timestamp() as u64,
            architecture: String::from("gbdt"),
            input_shape: Vec::new(),
            output_shape: Vec::new(),
            size_bytes: model_size,
            parameter_count: 0,
        };

        // Create registry entry
        let entry = crate::types::ModelRegistration {
            model_id,
            metadata,
            status: crate::types::RegistrationStatus::Pending,
            registrant: hex::encode(proposal_data.proposer),
            registered_at: timestamp,
            updated_at: timestamp,
            registration_fee,
            category: crate::types::ModelCategory::default(),
            tags: Vec::new(),
            description: Some(proposal_data.description.clone()),
            license: None,
            source_url: Some(proposal_data.model_url.clone()),
        };

        // Update status
        if let Some((_, status)) = self.proposals.get_mut(proposal_id) {
            *status = ProposalStatus::Executed;
        }

        Ok(entry)
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
        use ed25519_dalek::{Signature, VerifyingKey};
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
        Self::new(0.67, 1_000_000) // 67% threshold, 1M minimum stake
    }
}

impl ProposalManager {
    /// Calculate registration fee based on model size
    fn calculate_registration_fee(&self, model_size_bytes: u64) -> u64 {
        // Base fee + size-based fee
        let size_mb = model_size_bytes.div_ceil(1_000_000); // Round up to nearest MB
        let size_fee = size_mb * self.fee_per_mb;

        self.base_registration_fee.saturating_add(size_fee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
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

    #[test]
    fn test_registration_fee_calculation() {
        // Test with default fees: base=1M µIPN (1 IPN), per_mb=100K µIPN (0.1 IPN)
        let manager = ProposalManager::default();

        // Small model (< 1MB): should be base fee only
        assert_eq!(
            manager.calculate_registration_fee(500_000),
            1_000_000 + 100_000
        );

        // 1MB model: base + 1MB
        assert_eq!(
            manager.calculate_registration_fee(1_000_000),
            1_000_000 + 100_000
        );

        // 10MB model: base + 10MB
        assert_eq!(
            manager.calculate_registration_fee(10_000_000),
            1_000_000 + 1_000_000
        );

        // 100MB model: base + 100MB
        assert_eq!(
            manager.calculate_registration_fee(100_000_000),
            1_000_000 + 10_000_000
        );
    }

    #[test]
    fn test_custom_fee_parameters() {
        // Custom fees: base=2M, per_mb=200K
        let manager = ProposalManager::with_fees(0.67, 1_000_000, 2_000_000, 200_000);

        assert_eq!(
            manager.calculate_registration_fee(1_000_000),
            2_000_000 + 200_000
        );
        assert_eq!(
            manager.calculate_registration_fee(10_000_000),
            2_000_000 + 2_000_000
        );
    }
}
