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

pub use distribution::*;
pub use emission::*;
pub use parameters::*;
pub use types::*;

/// Module version for API introspection
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
