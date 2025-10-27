//! Deterministic AI Core for L1 Blockchain Operations
//!
//! Provides integer-only AI evaluation for validator reputation, model
//! verification, and on-chain inference under consensus constraints.
//!
//! Modules:
//! - `features`: Deterministic feature extraction from validator telemetry
//! - `gbdt`: Integer-only Gradient Boosted Decision Tree evaluator
//! - `deterministic_gbdt`: Deterministic, consensus-safe GBDT evaluator
//! - `model`: Model packaging and verification utilities
//! - `model_manager`: Model registry and lifecycle management
//! - `types`: Common data structures for models and execution
//! - `execution`: Deterministic execution engine for packaged models
//! - `feature_engineering`: Feature preprocessing and statistical profiling
//! - `production_config`: Environment and deployment configuration management
//! - `deployment`: Production deployment orchestration and monitoring
//! - `validation`: Model validation and benchmarking
//! - `health`: Runtime health and performance monitoring
//! - `security`: Model integrity and constraint enforcement
//! - `log`: Evaluation and audit logging utilities
//! - `tests`: Deterministic test harness and benchmarking

pub mod config;
pub mod errors;
pub mod features;
pub mod gbdt;
pub mod deterministic_gbdt;
pub mod health;
pub mod model;
pub mod model_manager;
pub mod feature_engineering;
pub mod types;
pub mod execution;
pub mod models;
pub mod validation;
pub mod determinism;
pub mod log;
pub mod production_config;
pub mod deployment;
pub mod tests;
pub mod monitoring;
pub mod security;

// ------------------------------------------------------------
// Re-exports for external crates and downstream use
// ------------------------------------------------------------

pub use config::{
    AiCoreConfig,
    ConfigManager,
    HealthConfig as ConfigHealthConfig,
    ExecutionConfig,
    LoggingConfig as ConfigLoggingConfig,
    SecurityConfig as ConfigSecurityConfig,
    PerformanceConfig,
    FeatureConfig as ConfigFeatureConfig,
    ValidationConfig,
};

pub use features::{
    extract_features,
    normalize_features,
    FeatureVector,
    FeatureConfig,
    ValidatorTelemetry,
};

// GBDT and deterministic evaluation
pub use gbdt::{
    eval_gbdt,
    GBDTModel,
    Node,
    Tree,
    GBDTError,
    GBDTResult,
    GBDTMetrics,
    ModelMetadata as GBDTModelMetadata,
    SecurityConstraints,
    FeatureNormalization,
};
pub use deterministic_gbdt::{
    DeterministicGBDT,
    GBDTTree,
    DecisionNode,
    ValidatorFeatures,
    compute_scores,
};

// Model management and feature pipeline
pub use model_manager::{
    ModelManager,
    ModelManagerConfig,
    ModelManagerMetrics,
    ModelLoadResult,
    ModelSaveResult,
};
pub use feature_engineering::{
    FeatureEngineeringPipeline,
    FeatureEngineeringConfig,
    RawFeatureData,
    ProcessedFeatureData,
    FeatureStatistics,
    FeatureImportance,
};

// Production configuration and deployment
pub use production_config::{
    ProductionConfig,
    ProductionConfigManager,
    Environment,
    GBDTConfig,
    ResourceLimits,
    FeatureFlags,
    DeploymentConfig,
    LoggingConfig,
    ConfigFormat,
    ConfigValidationResult,
};
pub use deployment::{
    ProductionDeployment,
    DeploymentStatus,
    HealthCheckResult,
    HealthStatus as DeploymentHealthStatus,
    DeploymentMetrics,
    utils,
};

// Test suites and benchmarks
pub use tests::{
    TestSuite,
    TestConfig,
    TestResult,
    BenchmarkSuite,
    test_utils,
};

// Health and monitoring
pub use health::{
    HealthMonitor,
    HealthConfig,
    HealthStatus,
    SystemHealth,
    PerformanceMetrics,
    HealthChecker,
    MemoryUsageChecker,
    ModelExecutionChecker,
};

// Core model and execution types
pub use model::{
    load_model,
    verify_model_hash,
    ModelMetadata,
    ModelPackage,
    MODEL_HASH_SIZE,
};
pub use types::{
    ModelId,
    ModelInput,
    ModelOutput,
    ExecutionContext,
    ExecutionResult,
    DataType,
    ExecutionMetadata,
};
pub use errors::AiCoreError;

// ------------------------------------------------------------
// Constants and helper functions
// ------------------------------------------------------------

/// AI Core version — crate version string for metadata and validation reports
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Deterministically sorts a vector for reproducible consensus behavior.
///
/// Used in AI and reputation subsystems to ensure sorting
/// consistency across validator nodes.
pub fn deterministically_sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    // Rust’s sort is deterministic for the same input and ordering.
    items.sort();
    items
}

/// High-level deterministic validator reputation computation.
///
/// Combines feature extraction and GBDT evaluation.
/// Used by consensus to score validators in each round.
///
/// # Arguments
/// * `telemetry` - ValidatorTelemetry (normalized telemetry data)
/// * `model` - Loaded GBDT model package
///
/// # Returns
/// Scaled integer reputation score (0–10000)
pub fn compute_validator_score(
    telemetry: &ValidatorTelemetry,
    model: &GBDTModel,
) -> i32 {
    let config = FeatureConfig::default();
    let features = extract_features(telemetry, &config);
    eval_gbdt(model, &features)
}
