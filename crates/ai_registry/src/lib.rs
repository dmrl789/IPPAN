/// On-chain AI model registry types and verification
///
/// This crate provides types for registering and activating AI models
/// on the IPPAN blockchain through governance.
use serde::{Deserialize, Serialize};
use serde_bytes;

/// Status of a model in the registry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    /// Proposed but not yet approved
    Proposed,
    /// Approved and waiting for activation
    Approved,
    /// Active and being used
    Active,
    /// Deprecated (still valid but not recommended)
    Deprecated,
    /// Revoked (no longer valid)
    Revoked,
}

/// On-chain model registry entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRegistryEntry {
    /// Unique model identifier
    pub model_id: String,
    /// SHA-256 hash of the model structure and weights
    #[serde(with = "serde_bytes")]
    pub hash_sha256: [u8; 32],
    /// Model version number
    pub version: u32,
    /// Round when this model becomes active
    pub activation_round: u64,
    /// Ed25519 signature from foundation or governance
    #[serde(with = "serde_bytes")]
    pub signature: [u8; 64],
    /// Current status of the model
    pub status: ModelStatus,
    /// Round when this entry was created
    pub created_round: u64,
    /// Round when last updated
    pub updated_round: u64,
    /// URL or IPFS hash for model download
    pub model_url: String,
}

impl ModelRegistryEntry {
    /// Create a new registry entry
    pub fn new(
        model_id: String,
        hash_sha256: [u8; 32],
        version: u32,
        activation_round: u64,
        signature: [u8; 64],
        created_round: u64,
        model_url: String,
    ) -> Self {
        Self {
            model_id,
            hash_sha256,
            version,
            activation_round,
            signature,
            status: ModelStatus::Proposed,
            created_round,
            updated_round: created_round,
            model_url,
        }
    }

    /// Check if model is active at a given round
    pub fn is_active_at(&self, round: u64) -> bool {
        self.status == ModelStatus::Active && round >= self.activation_round
    }

    /// Approve the model
    pub fn approve(&mut self, current_round: u64) {
        self.status = ModelStatus::Approved;
        self.updated_round = current_round;
    }

    /// Activate the model
    pub fn activate(&mut self, current_round: u64) {
        self.status = ModelStatus::Active;
        self.updated_round = current_round;
    }

    /// Deprecate the model
    pub fn deprecate(&mut self, current_round: u64) {
        self.status = ModelStatus::Deprecated;
        self.updated_round = current_round;
    }

    /// Revoke the model
    pub fn revoke(&mut self, current_round: u64) {
        self.status = ModelStatus::Revoked;
        self.updated_round = current_round;
    }
}

/// Governance proposal for AI model registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelProposal {
    /// Model identifier
    pub model_id: String,
    /// Model version
    pub version: u32,
    /// SHA-256 hash of model
    #[serde(with = "serde_bytes")]
    pub model_hash: [u8; 32],
    /// URL for model download/verification
    pub model_url: String,
    /// Round when model should activate (if approved)
    pub activation_round: u64,
    /// Ed25519 signature from proposer
    #[serde(with = "serde_bytes")]
    pub signature_foundation: [u8; 64],
    /// Public key of foundation/proposer
    #[serde(with = "serde_bytes")]
    pub proposer_pubkey: [u8; 32],
    /// Rationale for the proposal
    pub rationale: String,
    /// Voting threshold required (basis points, 10000 = 100%)
    pub threshold_bps: u16,
}

/// Governance proposal errors
#[derive(thiserror::Error, Debug)]
pub enum ProposalError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Activation round must be in the future")]
    ActivationInPast,
    #[error("Model ID already exists")]
    ModelExists,
    #[error("Invalid model hash")]
    InvalidHash,
    #[error("Invalid threshold: {0}")]
    InvalidThreshold(u16),
}

/// Verify proposal signature
pub fn verify_proposal_signature(proposal: &AiModelProposal) -> Result<(), ProposalError> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    // Create message to verify
    let mut message = Vec::new();
    message.extend_from_slice(proposal.model_id.as_bytes());
    message.extend_from_slice(&proposal.version.to_be_bytes());
    message.extend_from_slice(&proposal.model_hash);
    message.extend_from_slice(proposal.model_url.as_bytes());
    message.extend_from_slice(&proposal.activation_round.to_be_bytes());

    let verifying_key = VerifyingKey::from_bytes(&proposal.proposer_pubkey)
        .map_err(|_| ProposalError::InvalidSignature)?;

    let signature = Signature::from_bytes(&proposal.signature_foundation);

    verifying_key
        .verify(&message, &signature)
        .map_err(|_| ProposalError::InvalidSignature)
}

/// Validate proposal before submission
pub fn validate_proposal(
    proposal: &AiModelProposal,
    current_round: u64,
) -> Result<(), ProposalError> {
    // Check activation round is in future
    if proposal.activation_round <= current_round {
        return Err(ProposalError::ActivationInPast);
    }

    // Check threshold is valid
    if proposal.threshold_bps > 10000 {
        return Err(ProposalError::InvalidThreshold(proposal.threshold_bps));
    }

    // Verify signature
    verify_proposal_signature(proposal)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn test_model_registry_entry_creation() {
        let entry = ModelRegistryEntry::new(
            "test_model".to_string(),
            [1u8; 32],
            1,
            1000,
            [0u8; 64],
            100,
            "https://example.com/model.json".to_string(),
        );

        assert_eq!(entry.model_id, "test_model");
        assert_eq!(entry.status, ModelStatus::Proposed);
        assert_eq!(entry.created_round, 100);
    }

    #[test]
    fn test_model_is_active_at() {
        let mut entry = ModelRegistryEntry::new(
            "test".to_string(),
            [1u8; 32],
            1,
            1000,
            [0u8; 64],
            100,
            "url".to_string(),
        );

        entry.activate(1000);

        assert!(!entry.is_active_at(999));
        assert!(entry.is_active_at(1000));
        assert!(entry.is_active_at(1001));
    }

    #[test]
    fn test_model_status_transitions() {
        let mut entry = ModelRegistryEntry::new(
            "test".to_string(),
            [1u8; 32],
            1,
            1000,
            [0u8; 64],
            100,
            "url".to_string(),
        );

        assert_eq!(entry.status, ModelStatus::Proposed);

        entry.approve(200);
        assert_eq!(entry.status, ModelStatus::Approved);
        assert_eq!(entry.updated_round, 200);

        entry.activate(300);
        assert_eq!(entry.status, ModelStatus::Active);
        assert_eq!(entry.updated_round, 300);

        entry.deprecate(400);
        assert_eq!(entry.status, ModelStatus::Deprecated);

        entry.revoke(500);
        assert_eq!(entry.status, ModelStatus::Revoked);
    }

    #[test]
    fn test_verify_proposal_signature() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let proposer_pubkey = signing_key.verifying_key().to_bytes();

        let model_id = "test_model";
        let version = 1u32;
        let model_hash = [2u8; 32];
        let model_url = "https://example.com/model.json";
        let activation_round = 1000u64;

        // Create message
        let mut message = Vec::new();
        message.extend_from_slice(model_id.as_bytes());
        message.extend_from_slice(&version.to_be_bytes());
        message.extend_from_slice(&model_hash);
        message.extend_from_slice(model_url.as_bytes());
        message.extend_from_slice(&activation_round.to_be_bytes());

        let signature = signing_key.sign(&message);

        let proposal = AiModelProposal {
            model_id: model_id.to_string(),
            version,
            model_hash,
            model_url: model_url.to_string(),
            activation_round,
            signature_foundation: signature.to_bytes(),
            proposer_pubkey,
            rationale: "Test proposal".to_string(),
            threshold_bps: 6667,
        };

        assert!(verify_proposal_signature(&proposal).is_ok());
    }

    #[test]
    fn test_verify_proposal_signature_invalid() {
        let proposal = AiModelProposal {
            model_id: "test".to_string(),
            version: 1,
            model_hash: [2u8; 32],
            model_url: "url".to_string(),
            activation_round: 1000,
            signature_foundation: [0u8; 64], // Invalid signature
            proposer_pubkey: [1u8; 32],
            rationale: "Test".to_string(),
            threshold_bps: 6667,
        };

        assert!(verify_proposal_signature(&proposal).is_err());
    }

    #[test]
    fn test_validate_proposal_activation_in_past() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let proposer_pubkey = signing_key.verifying_key().to_bytes();

        let proposal = AiModelProposal {
            model_id: "test".to_string(),
            version: 1,
            model_hash: [2u8; 32],
            model_url: "url".to_string(),
            activation_round: 100,
            signature_foundation: [0u8; 64],
            proposer_pubkey,
            rationale: "Test".to_string(),
            threshold_bps: 6667,
        };

        let result = validate_proposal(&proposal, 200);
        assert!(matches!(result, Err(ProposalError::ActivationInPast)));
    }

    #[test]
    fn test_validate_proposal_invalid_threshold() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let proposer_pubkey = signing_key.verifying_key().to_bytes();

        let proposal = AiModelProposal {
            model_id: "test".to_string(),
            version: 1,
            model_hash: [2u8; 32],
            model_url: "url".to_string(),
            activation_round: 1000,
            signature_foundation: [0u8; 64],
            proposer_pubkey,
            rationale: "Test".to_string(),
            threshold_bps: 10001, // > 100%
        };

        let result = validate_proposal(&proposal, 100);
        assert!(matches!(result, Err(ProposalError::InvalidThreshold(_))));
    }
}
