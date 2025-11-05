//! Error types for DLC consensus

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DlcError {
    #[error("Block validation failed: {0}")]
    BlockValidation(String),

    #[error("Invalid verifier set: {0}")]
    InvalidVerifierSet(String),

    #[error("DAG operation failed: {0}")]
    DagOperation(String),

    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Invalid block structure: {0}")]
    InvalidBlock(String),

    #[error("Consensus round failed: {0}")]
    ConsensusRound(String),

    #[error("Reputation update failed: {0}")]
    ReputationUpdate(String),

    #[error("Emission calculation failed: {0}")]
    EmissionCalculation(String),

    #[error("Invalid bond: {0}")]
    InvalidBond(String),

    #[error("Validator not found: {0}")]
    ValidatorNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DlcError>;
