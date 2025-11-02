use crate::types::{MicroIPN, RoundId, ValidatorId};
use thiserror::Error;

/// Errors that can occur while computing emissions and reward distribution.
#[derive(Debug, Error)]
pub enum EconomicsError {
    #[error("invalid economics parameter: {0}")]
    InvalidParameter(&'static str),

    #[error("round {0} is not valid in the current context")]
    InvalidRound(RoundId),

    #[error("total issued supply {issued} exceeds hard cap {cap}")]
    SupplyCapExceeded { cap: MicroIPN, issued: MicroIPN },

    #[error("no validators available for reward distribution")]
    NoParticipants,

    #[error("duplicate validator entry detected while distributing rewards")]
    DuplicateValidator(ValidatorId),

    #[error("invalid participation entry: {0}")]
    InvalidParticipation(String),

    #[error("arithmetic overflow while performing economics calculation: {0}")]
    CalculationOverflow(&'static str),
}
