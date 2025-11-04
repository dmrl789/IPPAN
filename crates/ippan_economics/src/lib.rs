//! # IPPAN Economics - DAG-Fair Emission Framework
//!
//! This crate implements the deterministic, round-based token economics for IPPAN,
//! providing fair emission distribution across the BlockDAG structure.
//!
//! ## Core Concepts
//!
//! - **Round-based emission**: Rewards are computed per round, not per block.
//! - **DAG-Fair distribution**: Proportional rewards based on validator participation.
//! - **Deterministic halving**: Bitcoin-style halving schedule with round precision.
//! - **Supply integrity**: Hard-capped 21M IPN with automatic burn of excess.
//!
//! ## Key Components
//!
//! - [`EmissionEngine`]: Core emission calculation and distribution logic
//! - [`RoundRewards`]: Per-round reward computation and validator distribution
//! - [`SupplyTracker`]: Total supply monitoring and integrity verification
//! - [`GovernanceParameters`]: On-chain configurable emission parameters

pub mod distribution;
pub mod emission;
pub mod errors;
pub mod params;
pub mod supply;
pub mod types;

pub use distribution::RoundRewards;
pub use emission::EmissionEngine;
pub use errors::*;
pub use params::EconomicsParams;
pub use supply::{projected_supply, rounds_until_supply_cap, scheduled_round_reward, SupplyTracker};
pub use types::*;

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::{
        DistributionError, EmissionEngine, EmissionError, EmissionParams, RewardAmount, RoundIndex,
        RoundRewards, SupplyTracker, ValidatorReward,
    };
}
