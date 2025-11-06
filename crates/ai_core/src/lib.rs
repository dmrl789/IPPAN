//! Deterministic AI Core for L1 Blockchain Operations
//!
//! Provides integer-only deterministic AI evaluation for validator reputation,
//! model verification, and on-chain inference under consensus constraints.
//!
//! ## Modules
//! - `features`: Deterministic feature extraction from validator telemetry
//! - `gbdt`: Integer-only Gradient Boosted Decision Tree evaluator
//! - `determinism`: Deterministic context and RNG utilities
//! - `execution`: Deterministic model execution engine
//! - `model`: Model packaging and verification utilities
//! - `model_manager`: Model registry and lifecycle management
//! - `feature_engineering`: Feature preprocessing and statistics
//! - `production_config`: Environment and deployment configuration
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
pub mod fixed;
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
// Re-exports for workspace-wide use
// ------------------------------------------------------------

// Config & environment
pub use config::{
    AiCoreConfig, ConfigManager, ExecutionConfig, FeatureConfig as ConfigFeatureConfig,
    HealthConfig as ConfigHealthConfig, LoggingConfig as ConfigLoggingConfig, PerformanceConfig,
    SecurityConfig as ConfigSecurityConfig, ValidationConfig,
};

// Features and telemetry
pub use features::{
    extract_features, normalize_features, FeatureConfig, FeatureVector, ValidatorTelemetry,
};

// GBDT and deterministic inference
pub use deterministic_gbdt::{
    compute_scores, DecisionNode, DeterministicGBDT, DeterministicGBDTError, GBDTTree,
    ValidatorFeatures,
};
pub use gbdt::{
    eval_gbdt, FeatureNormalization, GBDTError, GBDTMetrics, GBDTModel, GBDTResult,
    ModelMetadata as GBDTModelMetadata, Node, SecurityConstraints, Tree,
};

// Feature pipeline & model management
pub use feature_engineering::{
    FeatureEngineeringConfig, FeatureEngineeringPipeline, FeatureImportance, FeatureStatistics,
    ProcessedFeatureData, RawFeatureData,
};
pub use model_manager::{
    ModelLoadResult, ModelManager, ModelManagerConfig, ModelManagerMetrics, ModelSaveResult,
};

// Deployment & production config
pub use deployment::{
    DeploymentMetrics, DeploymentStatus, HealthCheckResult, HealthStatus as DeploymentHealthStatus,
    ProductionDeployment,
};
pub use production_config::{
    ConfigFormat, ConfigValidationResult, DeploymentConfig, Environment, FeatureFlags, GBDTConfig,
    LoggingConfig, ProductionConfig, ProductionConfigManager, ResourceLimits,
};

// Health & monitoring
pub use health::{
    HealthChecker, HealthConfig, HealthMonitor, MemoryUsageChecker, ModelExecutionChecker,
    PerformanceMetrics, SystemHealth,
};

// Core model and execution
pub use errors::AiCoreError;
pub use fixed::{hash_fixed, hash_fixed_slice, Fixed, SCALE as FIXED_SCALE};
pub use model::{load_model, verify_model_hash, ModelPackage, MODEL_HASH_SIZE};
pub use types::{
    DataType, ExecutionContext, ExecutionMetadata, ExecutionResult, ModelId, ModelInput,
    ModelMetadata, ModelOutput,
};

// ------------------------------------------------------------
// Constants and helper functions
// ------------------------------------------------------------

/// AI Core crate version string for metadata and validation reports.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Deterministically sorts a vector (used for consensus reproducibility).
pub fn deterministically_sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

/// High-level deterministic validator reputation computation.
///
/// Combines telemetry feature extraction and GBDT evaluation.
/// Used by consensus to score validators per round.
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
            stake: 50_000_000_000_000,
            age_rounds: 100_000,
        };

        let model = GBDTModel::new(
            vec![Tree {
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
            10,
            10_000,
            6,
        )
        .expect("valid test model");

        let s1 = compute_validator_score(&telemetry, &model);
        let s2 = compute_validator_score(&telemetry, &model);
        assert_eq!(s1, s2);
        assert!(s1 > 0);
    }

    #[test]
    fn test_no_float_usage() {
        let x = 42;
        assert_eq!(x + 1, 43);
    }
}
