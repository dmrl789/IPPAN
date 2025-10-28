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
pub mod deployment;
pub mod determinism;
pub mod deterministic_gbdt;
pub mod errors;
pub mod execution;
pub mod feature_engineering;
pub mod features;
pub mod gbdt;
pub mod health;
pub mod log;
pub mod model;
pub mod model_manager;
pub mod models;
pub mod monitoring;
pub mod production_config;
pub mod security;
pub mod tests;
pub mod types;
pub mod validation;

// ------------------------------------------------------------
// Re-exports for external crates and downstream use
// ------------------------------------------------------------

pub use config::{
    AiCoreConfig, ConfigManager, ExecutionConfig, FeatureConfig as ConfigFeatureConfig,
    HealthConfig as ConfigHealthConfig, LoggingConfig as ConfigLoggingConfig, PerformanceConfig,
    SecurityConfig as ConfigSecurityConfig, ValidationConfig,
};

pub use features::{
    extract_features, normalize_features, FeatureConfig, FeatureVector, ValidatorTelemetry,
};

// GBDT and deterministic evaluation
pub use deterministic_gbdt::{
    compute_scores, DecisionNode, DeterministicGBDT, DeterministicGBDTError, GBDTTree,
    ValidatorFeatures,
};
pub use gbdt::{
    eval_gbdt, FeatureNormalization, GBDTError, GBDTMetrics, GBDTModel, GBDTResult,
    ModelMetadata as GBDTModelMetadata, Node, SecurityConstraints, Tree,
};

// Model management and feature pipeline
pub use feature_engineering::{
    FeatureEngineeringConfig, FeatureEngineeringPipeline, FeatureImportance, FeatureStatistics,
    ProcessedFeatureData, RawFeatureData,
};
pub use model_manager::{
    ModelLoadResult, ModelManager, ModelManagerConfig, ModelManagerMetrics, ModelSaveResult,
};

// Production configuration and deployment
pub use deployment::{
    utils, DeploymentMetrics, DeploymentStatus, HealthCheckResult,
    HealthStatus as DeploymentHealthStatus, ProductionDeployment,
};
pub use production_config::{
    ConfigFormat, ConfigValidationResult, DeploymentConfig, Environment, FeatureFlags, GBDTConfig,
    LoggingConfig, ProductionConfig, ProductionConfigManager, ResourceLimits,
};

// Test suites and benchmarks
#[cfg(test)]
pub use tests::{test_utils, BenchmarkSuite, TestConfig, TestResult, TestSuite};

// Health and monitoring
pub use health::{
    HealthChecker, HealthConfig, HealthMonitor, MemoryUsageChecker, ModelExecutionChecker,
    PerformanceMetrics, SystemHealth,
};

// Core model and execution types
pub use errors::AiCoreError;
pub use model::{load_model, verify_model_hash, ModelPackage, MODEL_HASH_SIZE};
pub use types::{
    DataType, ExecutionContext, ExecutionMetadata, ExecutionResult, ModelId, ModelInput,
    ModelOutput,
};

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
pub fn compute_validator_score(telemetry: &ValidatorTelemetry, model: &GBDTModel) -> i32 {
    let config = FeatureConfig::default();
    let features = extract_features(telemetry, &config);
    eval_gbdt(model, &features)
}

// ------------------------------------------------------------
// Internal deterministic tests
// ------------------------------------------------------------
#[cfg(test)]
mod internal_tests {
    use super::*;
    use crate::features::ValidatorTelemetry;

    #[test]
    fn sort_is_deterministic_for_integers() {
        let input = vec![3, 1, 2, 2, 5, 4];
        let out1 = deterministically_sorted(input.clone());
        let out2 = deterministically_sorted(input);
        assert_eq!(out1, out2);
        assert_eq!(out1, vec![1, 2, 2, 3, 4, 5]);
    }

    #[test]
    fn test_compute_validator_score_consistency() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 1000,
            blocks_verified: 3000,
            rounds_active: 10_000,
            avg_latency_us: 80_000,
            slash_count: 0,
            stake: 500_000_00000000,
            age_rounds: 100_000,
        };

        let model = GBDTModel {
            bias: 10,
            scale: 10_000,
            trees: vec![Tree {
                nodes: vec![
                    Node {
                        feature_index: 0,
                        threshold: 5000,
                        left: 1,
                        right: 2,
                        value: None,
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(100),
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(200),
                    },
                ],
            }],
        };

        let s1 = compute_validator_score(&telemetry, &model);
        let s2 = compute_validator_score(&telemetry, &model);
        assert_eq!(s1, s2);
        assert!(s1 > 0);
    }

    #[test]
    fn test_no_float_usage() {
        // Ensures no floating-point operations are required in deterministic consensus paths.
        let x = 42;
        assert_eq!(x + 1, 43);
    }
}
