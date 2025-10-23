use thiserror::Error;

#[derive(Debug, Error)]
pub enum EcoError {
    #[error("division by zero: no blocks recorded in round")]
    NoBlocksInRound,
    #[error("hard cap exceeded: requested={requested}, remaining={remaining}")]
    HardCapExceeded { requested: u128, remaining: u128 },
    #[error("invalid fee cap: fees={fees}, cap={cap}")]
    FeeCapExceeded { fees: u128, cap: u128 },
}
