//! IPPAN Economics Module
//!
//! Implements the DAG-Fair Emission system with:
//! - Deterministic round-based emission with hard supply cap
//! - Fee capping and recycling mechanisms
//! - Governance-controlled parameters
//! - Verifiable reward distribution

pub mod distribution;
pub mod emission;
pub mod errors;
pub mod parameters;
pub mod types;

// Re-export distribution functions
pub use distribution::*;
// Re-export emission functions
pub use emission::*;
// Re-export error types
pub use errors::*;
// Re-export commonly used parameter types
pub use parameters::{EconomicsParameterManager, EconomicsParameterProposal};
// Re-export commonly used types
pub use types::{
    DistributionResult, EconomicsParams, EmissionResult, MicroIPN, Participation, ParticipationSet,
    Payouts, Role, RoundId, ValidatorId, MICRO_PER_IPN, REPUTATION_SCORE_MAX, REPUTATION_SCORE_MIN,
    REPUTATION_SCORE_SCALE,
};

/// Module version for API introspection
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
