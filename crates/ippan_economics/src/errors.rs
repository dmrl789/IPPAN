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
    InvalidEmissionParams(String),

    #[error("Mathematical overflow in emission calculation")]
    MathematicalOverflow,

    #[error("Invalid halving interval: {0}")]
    InvalidHalvingInterval(u64),

    #[error("Emission calculation failed: {0}")]
    CalculationFailed(String),
}

/// Errors that can occur in reward distribution
#[derive(Error, Debug)]
pub enum DistributionError {
    #[error("No validators in round: {0}")]
    NoValidators(u64),

    #[error("Invalid validator participation: {0}")]
    InvalidParticipation(String),

    #[error("Distribution calculation failed: {0}")]
    CalculationFailed(String),

    #[error("Invalid weight factor: {0}")]
    InvalidWeightFactor(String),

    #[error("Reward distribution overflow")]
    DistributionOverflow,
}

/// Errors that can occur in supply tracking
#[derive(Error, Debug)]
pub enum SupplyError {
    #[error("Supply verification failed: expected={expected}, actual={actual}")]
    VerificationFailed { expected: u64, actual: u64 },

    #[error("Invalid supply state: {0}")]
    InvalidSupplyState(String),

    #[error("Supply tracking error: {0}")]
    TrackingError(String),
}

/// Errors that can occur in governance parameter updates
#[derive(Error, Debug)]
pub enum GovernanceError {
    #[error("Invalid parameter value: {param}={value}, reason: {reason}")]
    InvalidParameter { param: String, value: String, reason: String },

    #[error("Governance update failed: {0}")]
    UpdateFailed(String),

    #[error("Insufficient voting power: required={required}, actual={actual}")]
    InsufficientVotingPower { required: u64, actual: u64 },

    #[error("Governance error: {0}")]
    GovernanceError(String),
}

/// Main error type for the economics crate
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

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("General economics error: {0}")]
    General(String),
}

impl From<String> for EconomicsError {
    fn from(msg: String) -> Self {
        Self::General(msg)
    }
}

impl From<&str> for EconomicsError {
    fn from(msg: &str) -> Self {
        Self::General(msg.to_string())
    }
}