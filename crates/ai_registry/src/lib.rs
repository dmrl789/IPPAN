//! AI Registry module for IPPAN blockchain
//! 
//! This module provides model registration, governance, and fee management
//! capabilities for the IPPAN blockchain. It manages the lifecycle of AI models
//! and ensures proper governance and economic incentives.

pub mod registry;
pub mod governance;
pub mod fees;
pub mod storage;
pub mod api;
pub mod types;
pub mod errors;

pub use registry::*;
pub use governance::*;
pub use fees::*;
pub use storage::*;
pub use api::*;
pub use types::*;
pub use errors::*;

/// Version of the AI Registry module
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Feature flags for the AI Registry module
pub mod features {
    /// Enable persistent storage
    pub const PERSISTENT: &str = "persistent";
    /// Enable API endpoints
    pub const API: &str = "api";
}