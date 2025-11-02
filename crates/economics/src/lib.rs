//! IPPAN Economics Module
//!
//! Implements the DAG-Fair Emission system with:
//! - Deterministic round-based emission with hard supply cap
//! - Fee capping and recycling mechanisms
//! - Governance-controlled parameters
//! - Verifiable reward distribution

pub mod distribution;
pub mod emission;
pub mod parameters;
pub mod types;

// Re-export commonly used types explicitly
pub use parameters::{EconomicsParameterManager, EconomicsParameterProposal};
pub use types::{
    DistributionResult, EconomicsParams, EmissionResult, MicroIPN, Participation, ParticipationSet,
    Payouts, Role, RoundId, ValidatorId, MICRO_PER_IPN,
};

/// Module version for API introspection
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
