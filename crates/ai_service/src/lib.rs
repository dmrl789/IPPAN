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
pub mod config;
pub mod errors;
pub mod health;
pub mod llm;
pub mod metrics;
#[cfg(feature = "analytics")]
pub mod monitoring;
#[cfg(feature = "analytics")]
pub mod optimization;
pub mod service;
pub mod smart_contracts;
pub mod types;

pub use errors::AIServiceError;

#[cfg(feature = "analytics")]
pub use monitoring::{
    AlertHandler, AlertSeverity, AlertType, ConsoleAlertHandler, FileAlertHandler,
    MonitoringService, MonitoringStatistics, ServiceAlert, ServiceHealthReport, ServiceMetrics,
    ServiceMetricsSnapshot, ServiceMonitor, ServiceStatus,
};

pub use config::{ConfigManager, Environment};
pub use health::{CheckResult, CheckStatus, HealthResponse, HealthStatus};
pub use metrics::{
    JsonExporter, MetricsCollector, MetricsExporter, MetricsSnapshot, PrometheusExporter,
};
pub use service::AIService;
pub use types::{
    AIServiceConfig, AlertStatus, AnalyticsConfig, AnalyticsInsight, ContractAnalysisMetadata,
    ContractAnalysisType, ContractIssue, DataPoint, DifficultyLevel, InsightType, LLMConfig,
    LLMRequest, LLMResponse, LLMUsage, MonitoringAlert, OptimizationConstraints, OptimizationGoal,
    OptimizationSuggestion, SeverityLevel, SmartContractAnalysisRequest,
    SmartContractAnalysisResponse, TransactionData, TransactionOptimizationRequest,
    TransactionOptimizationResponse,
};

/// Re-export core AI functionality
pub use ippan_ai_core::{
    compute_validator_score, extract_features, FeatureConfig, GBDTModel, ValidatorTelemetry,
};

/// Re-export registry functionality
pub use ippan_ai_registry::AiModelProposal;

/// AI Service version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
