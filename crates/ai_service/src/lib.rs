//! IPPAN AI Service Layer
//!
//! High-level AI service providing:
//! - LLM integration for natural language processing
//! - Smart contract analysis and generation
//! - Blockchain analytics and insights
//! - AI-powered transaction optimization
//! - Intelligent monitoring and alerting

pub mod llm;
#[cfg(feature = "analytics")]
pub mod analytics;
pub mod smart_contracts;
#[cfg(feature = "analytics")]
pub mod monitoring;
#[cfg(feature = "analytics")]
pub mod optimization;
pub mod errors;
pub mod types;
pub mod service;

pub use service::AIService;
pub use errors::AIServiceError;
pub use types::*;

/// Re-export core AI functionality
pub use ippan_ai_core::{
    compute_validator_score,
    extract_features,
    ValidatorTelemetry,
    FeatureConfig,
    GBDTModel,
};

/// Re-export registry functionality
pub use ippan_ai_registry::{
    ModelRegistryEntry,
    ModelStatus,
    AiModelProposal,
    validate_proposal,
};

/// Re-export governance functionality
// Note: Removed to avoid circular dependencies
// pub use ippan_governance::*;

/// AI Service version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");