//! Error types for IPPAN economics

use thiserror::Error;

/// Errors that can occur in emission calculations
#[derive(Error, Debug)]
pub enum EmissionError {
    #[error("Invalid round index: {0}")]
    InvalidRoundIndex(u64),

    #[error("Invalid reward amount: {0}")]
    InvalidRewardAmount(u64),

    #[error("Supply cap exceeded: current={current}, cap={cap}")]
    SupplyCapExceeded { current: u64, cap: u64 },

    #[error("Invalid emission parameters: {0}")]
    InvalidParameters(String),

    #[error("Calculation overflow: {0}")]
    CalculationOverflow(String),
}

/// Errors that can occur in reward distribution
#[derive(Error, Debug)]
pub enum DistributionError {
    #[error("No validators in round {0}")]
    NoValidators(u64),

    #[error("Invalid participation data: {0}")]
    InvalidParticipation(String),

    #[error("Calculation failed: {0}")]
    CalculationFailed(String),

    #[error("Invalid weight calculation: {0}")]
    InvalidWeight(String),
}

/// Errors that can occur in supply tracking
#[derive(Error, Debug)]
pub enum SupplyError {
    #[error("Supply verification failed: {0}")]
    VerificationFailed(String),

    #[error("Invalid supply data: {0}")]
    InvalidSupplyData(String),

    #[error("Supply cap violation: {0}")]
    SupplyCapViolation(String),
}

/// Errors that can occur in governance
#[derive(Error, Debug)]
pub enum GovernanceError {
    #[error("Insufficient voting power: required={required}, actual={actual}")]
    InsufficientVotingPower { required: u64, actual: u64 },

    #[error("Invalid parameter: {param}={value}, reason: {reason}")]
    InvalidParameter { param: String, value: String, reason: String },

    #[error("Governance error: {0}")]
    GovernanceError(String),
}

/// General economics error type
#[derive(Error, Debug)]
pub enum EconomicsError {
    #[error("Emission error: {0}")]
    Emission(#[from] EmissionError),

    #[error("Distribution error: {0}")]
    Distribution(#[from] DistributionError),

    #[error("Supply error: {0}")]
    Supply(#[from] SupplyError),

    #[error("Governance error: {0}")]
    Governance(#[from] GovernanceError),
}
