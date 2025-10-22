//! Governance module for IPPAN blockchain
//! 
//! This module provides governance capabilities for the IPPAN blockchain,
//! including voting, proposals, and decision making processes.

pub mod voting;
pub mod proposals;
pub mod delegation;
pub mod treasury;
pub mod types;
pub mod errors;

pub use voting::*;
pub use proposals::*;
pub use delegation::*;
pub use treasury::*;
pub use types::*;
pub use errors::*;

/// Version of the Governance module
pub const VERSION: &str = env!("CARGO_PKG_VERSION");