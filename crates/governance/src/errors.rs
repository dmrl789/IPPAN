//! Error types for the Governance module

use thiserror::Error;

/// Errors that can occur in the Governance module
#[derive(Error, Debug)]
pub enum GovernanceError {
    /// Proposal not found
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),
    
    /// Invalid proposal
    #[error("Invalid proposal: {0}")]
    InvalidProposal(String),
    
    /// Voting period expired
    #[error("Voting period expired for proposal: {0}")]
    VotingPeriodExpired(String),
    
    /// Insufficient voting power
    #[error("Insufficient voting power: {0}")]
    InsufficientVotingPower(String),
    
    /// Already voted
    #[error("Already voted on proposal: {0}")]
    AlreadyVoted(String),
    
    /// Proposal not executable
    #[error("Proposal not executable: {0}")]
    ProposalNotExecutable(String),
    
    /// Treasury insufficient funds
    #[error("Treasury insufficient funds: {0}")]
    TreasuryInsufficientFunds(String),
    
    /// Invalid delegation
    #[error("Invalid delegation: {0}")]
    InvalidDelegation(String),
    
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Database error
    #[error("Database error: {0}")]
    Database(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for Governance operations
pub type Result<T> = std::result::Result<T, GovernanceError>;