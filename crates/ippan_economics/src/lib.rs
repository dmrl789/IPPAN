//! # IPPAN Economics - DAG-Fair Emission Framework
//!
//! This crate implements the deterministic round-based token economics for IPPAN,
//! providing fair emission distribution across the BlockDAG structure.
//!
//! ## Core Concepts
//!
//! - **Round-based emission**: Rewards are computed per round, not per block
//! - **DAG-Fair distribution**: Proportional rewards based on validator participation
//! - **Deterministic halving**: Bitcoin-style halving schedule with round precision
//! - **Supply integrity**: Hard-capped 21M IPN with automatic burn of excess
//!
//! ## Key Components
//!
//! - [`EmissionEngine`]: Core emission calculation and distribution logic
//! - [`RoundRewards`]: Per-round reward computation and validator distribution
//! - [`SupplyTracker`]: Total supply monitoring and integrity verification
//! - [`GovernanceParams`]: On-chain configurable emission parameters

pub mod emission;
pub mod distribution;
pub mod supply;
pub mod governance;
pub mod types;
pub mod errors;

pub use emission::EmissionEngine;
pub use distribution::RoundRewards;
pub use supply::SupplyTracker;
pub use governance::GovernanceParams;
pub use types::*;
pub use errors::*;

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::{
        EmissionEngine, RoundRewards, SupplyTracker, GovernanceParams,
        EmissionParams, RoundIndex, RewardAmount, ValidatorReward,
        EmissionError, DistributionError,
    };
}
