//! IPPAN AI Registry — Governance-Controlled Model Publication
//!
//! Provides deterministic types and logic for registering,
//! approving, activating, and revoking AI models on-chain.
//!
//! Each entry includes cryptographic verification, round-based
//! activation, and governance thresholds for decentralized control.
//!
//! ## Features
//!
//! - **Model Registry**: Register, approve, and manage AI models
//! - **Governance**: Proposal creation and voting system
//! - **Security**: Authentication, rate limiting, and validation
//! - **Fee Management**: Configurable fee structures for operations
//! - **Activation Management**: Schedule model activations by round
//! - **Storage**: Persistent storage with Sled database backend
//!
//! ## Example
//! ```rust,no_run
//! use ippan_ai_registry::{ModelRegistry, RegistryStorage, RegistryConfig};
//! use ai_core::types::{ModelId, ModelMetadata};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let storage = RegistryStorage::new(None)?;
//!     let config = RegistryConfig::default();
//!     let mut registry = ModelRegistry::new(storage, config);
//!
//!     let model_id = ModelId {
//!         name: "my_model".to_string(),
//!         version: "1.0.0".to_string(),
//!         hash: "abc123".to_string(),
//!     };
//!
//!     // Register a model...
//!
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};

pub mod errors;
pub mod types;
pub mod storage;
pub mod registry;
pub mod security;
pub mod governance;
pub mod fees;
pub mod activation;
pub mod proposal;

// Re-export commonly used types
pub use errors::{RegistryError, Result};
pub use types::*;
pub use storage::RegistryStorage;
pub use registry::ModelRegistry;
pub use security::{SecurityManager, SecurityConfig, AuthToken, UserPermissions, SecurityStats};
pub use governance::GovernanceManager;
pub use fees::{FeeManager, FeeStats, FeeCalculation};
pub use activation::ActivationManager;
pub use proposal::{ProposalManager, AiModelProposal, ProposalStatus};

// API module (optional)
#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "api")]
pub use api::{ApiState, create_router, ApiResponse};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
        assert!(!NAME.is_empty());
    }

    #[test]
    fn test_types_creation() {
        let config = RegistryConfig::default();
        assert!(config.min_registration_fee > 0);
        assert!(config.max_model_size > 0);
    }
}
