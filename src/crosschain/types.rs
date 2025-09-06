//! L2 transaction types and related structures for IPPAN
//! 
//! This module defines the core types for L2 commitments and exits,
//! enabling arbitrary smart contracts on L2s that post succinct proofs to L1.

use serde::{Serialize, Deserialize};
use thiserror::Error;

/// L2 proof types supported by IPPAN
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ProofType {
    /// Zero-knowledge proof using Groth16
    ZkGroth16,
    /// Optimistic rollup with challenge window
    Optimistic,
    /// External attestation (off-chain verification)
    External,
}

/// Data availability modes for L2 commits
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DataAvailabilityMode {
    /// Store data inline in the L1 transaction
    Inline,
    /// Only store hash, data available externally
    External,
}

/// L2 commit transaction for posting state updates
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L2CommitTx {
    /// L2 identifier (e.g., "rollup-eth-zk1", "appchain-xyz")
    pub l2_id: String,
    /// L2 epoch/batch number (must be monotonically increasing)
    pub epoch: u64,
    /// State root after applying this batch
    pub state_root: [u8; 32],
    /// Data availability hash (blob/content hash)
    pub da_hash: [u8; 32],
    /// Proof type for this commit
    pub proof_type: ProofType,
    /// Proof bytes or commitment data
    pub proof: Vec<u8>,
    /// Optional inline data (only if DA mode is Inline)
    pub inline_data: Option<Vec<u8>>,
}

/// L2 exit transaction for withdrawing assets to L1
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L2ExitTx {
    /// L2 identifier
    pub l2_id: String,
    /// L2 epoch this exit is based on
    pub epoch: u64,
    /// Proof of inclusion in the L2 state
    pub proof_of_inclusion: Vec<u8>,
    /// Recipient account on L1
    pub account: [u8; 32],
    /// Amount or asset payload
    pub amount: u128,
    /// Nonce to prevent replay attacks
    pub nonce: u64,
}

/// L2 parameters for registration and configuration
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct L2Params {
    /// Proof type for this L2
    pub proof_type: ProofType,
    /// Data availability mode
    pub da_mode: DataAvailabilityMode,
    /// Challenge window in milliseconds (for optimistic rollups)
    pub challenge_window_ms: u64,
    /// Maximum commit size in bytes
    pub max_commit_size: usize,
    /// Minimum time between epochs in milliseconds
    pub min_epoch_gap_ms: u64,
}

/// L2 anchor event emitted when a commit is accepted
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnchorEvent {
    /// L2 identifier
    pub l2_id: String,
    /// L2 epoch
    pub epoch: u64,
    /// State root after applying batch
    pub state_root: [u8; 32],
    /// Data availability hash
    pub da_hash: [u8; 32],
    /// Timestamp when committed
    pub committed_at: u64,
}

/// L2 exit status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ExitStatus {
    /// Exit is pending verification
    Pending,
    /// Exit has been finalized
    Finalized,
    /// Exit was rejected
    Rejected,
    /// Exit is in challenge window (optimistic only)
    ChallengeWindow,
}

/// L2 exit record
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct L2ExitRecord {
    /// Exit transaction
    pub exit: L2ExitTx,
    /// Current status
    pub status: ExitStatus,
    /// Timestamp when submitted
    pub submitted_at: u64,
    /// Timestamp when finalized (if applicable)
    pub finalized_at: Option<u64>,
    /// Rejection reason (if applicable)
    pub rejection_reason: Option<String>,
}

/// Validation errors for L2 transactions
#[derive(Error, Debug)]
pub enum L2ValidationError {
    #[error("Invalid L2 ID format")]
    InvalidL2Id,
    #[error("Epoch must be monotonically increasing")]
    EpochRegression,
    #[error("Proof size exceeds maximum allowed")]
    ProofTooLarge,
    #[error("Rate limit exceeded for L2 ID")]
    RateLimitExceeded,
    #[error("Invalid proof format")]
    InvalidProof,
    #[error("Challenge window not elapsed")]
    ChallengeWindowOpen,
    #[error("L2 not registered")]
    L2NotRegistered,
    #[error("Invalid state root format")]
    InvalidStateRoot,
    #[error("Invalid DA hash format")]
    InvalidDaHash,
}

impl L2CommitTx {
    /// Validate the L2 commit transaction
    pub fn validate(&self, max_commit_size: usize) -> Result<(), L2ValidationError> {
        // Validate L2 ID format
        if self.l2_id.is_empty() || self.l2_id.len() > 64 {
            return Err(L2ValidationError::InvalidL2Id);
        }
        
        // Validate proof size
        if self.proof.len() > max_commit_size {
            return Err(L2ValidationError::ProofTooLarge);
        }
        
        // Validate inline data size if present
        if let Some(ref data) = self.inline_data {
            if data.len() > max_commit_size {
                return Err(L2ValidationError::ProofTooLarge);
            }
        }
        
        Ok(())
    }
}

impl L2ExitTx {
    /// Validate the L2 exit transaction
    pub fn validate(&self) -> Result<(), L2ValidationError> {
        // Validate L2 ID format
        if self.l2_id.is_empty() || self.l2_id.len() > 64 {
            return Err(L2ValidationError::InvalidL2Id);
        }
        
        // Validate proof of inclusion
        if self.proof_of_inclusion.is_empty() {
            return Err(L2ValidationError::InvalidProof);
        }
        
        // Validate amount
        if self.amount == 0 {
            return Err(L2ValidationError::InvalidProof);
        }
        
        Ok(())
    }
}

impl L2Params {
    /// Create default parameters for a proof type
    pub fn default_for(proof_type: ProofType) -> Self {
        match proof_type {
            ProofType::ZkGroth16 => Self {
                proof_type,
                da_mode: DataAvailabilityMode::External,
                challenge_window_ms: 0, // No challenge window for ZK
                max_commit_size: 16384, // 16 KB
                min_epoch_gap_ms: 250,  // 250ms minimum
            },
            ProofType::Optimistic => Self {
                proof_type,
                da_mode: DataAvailabilityMode::External,
                challenge_window_ms: 60000, // 1 minute challenge window
                max_commit_size: 16384,
                min_epoch_gap_ms: 250,
            },
            ProofType::External => Self {
                proof_type,
                da_mode: DataAvailabilityMode::External,
                challenge_window_ms: 0,
                max_commit_size: 16384,
                min_epoch_gap_ms: 250,
            },
        }
    }
}
