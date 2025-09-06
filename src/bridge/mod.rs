//! Bridge module for L2 integration
//! 
//! This module provides the bridge functionality for connecting L2 networks
//! to IPPAN L1, including the L2 registry and verifier interfaces.

pub mod registry;

pub use registry::{L2Registry, L2RegistryConfig, L2RegistryEntry, L2RegistryStats};

/// Re-export types for convenience
pub use crate::crosschain::types::{
    L2CommitTx, L2ExitTx, L2Params, ProofType, DataAvailabilityMode,
    AnchorEvent, ExitStatus, L2ExitRecord, L2ValidationError,
};
