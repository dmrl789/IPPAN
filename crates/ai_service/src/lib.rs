//! IPPAN AI Service Layer
//!
//! High-level AI service providing:
//! - LLM integration for natural language processing
//! - Smart contract analysis and generation
//! - Blockchain analytics and insights
//! - AI-powered transaction optimization
//! - Intelligent monitoring and alerting

#[cfg(feature = "analytics")]
pub mod analytics;
pub mod errors;
pub mod llm;
#[cfg(feature = "analytics")]
pub mod monitoring;
#[cfg(feature = "analytics")]
pub mod optimization;
pub mod service;
pub mod smart_contracts;
pub mod types;
pub mod health;
pub mod metrics;
pub mod config;

pub use errors::AIServiceError;

#[cfg(feature = "analytics")]
pub use monitoring::{
    ServiceMonitor,
    MonitoringService,
    ServiceStatus,
    ServiceMetrics,
    ServiceMetricsSnapshot,
    ServiceHealthReport,
    MonitoringConfig,
    AlertHandler,
    ServiceAlert,
    AlertType,
    AlertSeverity,
    ConsoleAlertHandler,
    FileAlertHandler,
    MonitoringStatistics,
};

pub use service::AIService;
pub use types::*;

/// Re-export core AI functionality
pub use ippan_ai_core::{
    compute_validator_score, extract_features, FeatureConfig, GBDTModel, ValidatorTelemetry,
};

/// Re-export registry functionality
pub use ippan_ai_registry::{validate_proposal, AiModelProposal, ModelRegistryEntry, ModelStatus};

/// AI Service version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
