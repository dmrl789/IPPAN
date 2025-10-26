//! IPPAN AI Registry — Governance-Controlled Model Publication
//!
//! Provides deterministic types and logic for registering,
//! approving, activating, and revoking AI models on-chain.
//!
//! Each entry includes cryptographic verification, round-based
//! activation, and governance thresholds for decentralized control.

use serde::{Deserialize, Serialize};
use serde_bytes;

pub mod errors;
pub mod security;

pub use security::{
    SecurityManager,
    SecurityConfig,
    AuthToken,
    UserPermissions,
    RateLimiter,
    SecurityStats,
};

/// Status of a model in the registry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    Proposed,
    Approved,
    Active,
    Deprecated,
    Revoked,
}

/// On-chain model registry entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelRegistryEntry {
    /// Unique model identifier (e.g. "gbdt_v1")
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
    /// Current model status
    pub status: ModelStatus,
    /// Round when this entry was created
    pub created_round: u64,
    /// Round when last updated
    pub updated_round: u64,
    /// IPFS hash or HTTPS URL for model download
    pub model_url: String,
}

impl ModelRegistryEntry {
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

    pub fn is_active_at(&self, round: u64) -> bool {
        self.status == ModelStatus::Active && round >= self.activation_round
    }

    pub fn approve(&mut self, round: u64) {
        self.status = ModelStatus::Approved;
        self.updated_round = round;
    }

    pub fn activate(&mut self, round: u64) {
        self.status = ModelStatus::Active;
        self.updated_round = round;
    }

    pub fn deprecate(&mut self, round: u64) {
        self.status = ModelStatus::Deprecated;
        self.updated_round = round;
    }

    pub fn revoke(&mut self, round: u64) {
        self.status = ModelStatus::Revoked;
        self.updated_round = round;
    }
}

/// Governance proposal for AI model registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelProposal {
    pub model_id: String,
    pub version: u32,
    #[serde(with = "serde_bytes")]
    pub model_hash: [u8; 32],
    pub model_url: String,
    pub activation_round: u64,
    #[serde(with = "serde_bytes")]
    pub signature_foundation: [u8; 64],
    #[serde(with = "serde_bytes")]
    pub proposer_pubkey: [u8; 32],
    pub rationale: String,
    /// Required approval threshold (bps; 10000 = 100%)
    pub threshold_bps: u16,
}

/// Errors encountered during proposal validation
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

/// Verify the signature of a model proposal
pub fn verify_proposal_signature(proposal: &AiModelProposal) -> Result<(), ProposalError> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

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
    if proposal.activation_round <= current_round {
        return Err(ProposalError::ActivationInPast);
    }
    if proposal.threshold_bps > 10000 {
        return Err(ProposalError::InvalidThreshold(proposal.threshold_bps));
    }
    verify_proposal_signature(proposal)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn test_entry_lifecycle() {
        let mut entry = ModelRegistryEntry::new(
            "validator_model".to_string(),
            [1u8; 32],
            1,
            1000,
            [0u8; 64],
            100,
            "ipfs://model".to_string(),
        );

        assert_eq!(entry.status, ModelStatus::Proposed);
        entry.approve(200);
        assert_eq!(entry.status, ModelStatus::Approved);
        entry.activate(300);
        assert!(entry.is_active_at(300));
        entry.deprecate(400);
        assert_eq!(entry.status, ModelStatus::Deprecated);
        entry.revoke(500);
        assert_eq!(entry.status, ModelStatus::Revoked);
    }

    #[test]
    fn test_signature_verification() {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let proposer_pubkey = signing_key.verifying_key().to_bytes();
        let model_id = "test";
        let version = 1u32;
        let hash = [9u8; 32];
        let url = "https://ippan.org/model.json";
        let activation_round = 500u64;

        let mut msg = Vec::new();
        msg.extend_from_slice(model_id.as_bytes());
        msg.extend_from_slice(&version.to_be_bytes());
        msg.extend_from_slice(&hash);
        msg.extend_from_slice(url.as_bytes());
        msg.extend_from_slice(&activation_round.to_be_bytes());

        let sig = signing_key.sign(&msg);

        let proposal = AiModelProposal {
            model_id: model_id.to_string(),
            version,
            model_hash: hash,
            model_url: url.to_string(),
            activation_round,
            signature_foundation: sig.to_bytes(),
            proposer_pubkey,
            rationale: "Test proposal".into(),
            threshold_bps: 8000,
        };

        assert!(verify_proposal_signature(&proposal).is_ok());
    }

    #[test]
    fn test_invalid_threshold_and_round() {
        let proposal = AiModelProposal {
            model_id: "m".into(),
            version: 1,
            model_hash: [1u8; 32],
            model_url: "url".into(),
            activation_round: 5,
            signature_foundation: [0u8; 64],
            proposer_pubkey: [1u8; 32],
            rationale: "r".into(),
            threshold_bps: 12000,
        };

        let r1 = validate_proposal(&proposal, 10);
        assert!(matches!(r1, Err(ProposalError::ActivationInPast)));

        let mut p2 = proposal.clone();
        p2.activation_round = 100;
        let r2 = validate_proposal(&p2, 10);
        assert!(matches!(r2, Err(ProposalError::InvalidThreshold(_))));
    }
}
