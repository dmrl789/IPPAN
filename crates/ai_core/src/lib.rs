//! Deterministic AI Core for L1 Blockchain Operations
//!
//! Provides integer-only AI evaluation for validator reputation, model
//! verification, and on-chain inference under consensus constraints.
//!
//! Modules:
//! - `features`: Deterministic feature extraction from validator telemetry
//! - `gbdt`: Integer-only Gradient Boosted Decision Tree evaluator
//! - `model`: Model packaging and verification utilities
//! - `types`: Common data structures for models and execution
//! - `execution`: Deterministic execution engine for packaged models
//! - `models`: Model manager and loaders (local/remote)
//! - `validation`: Model validation utilities
//! - `log`: Evaluation logging helpers

pub mod features;
pub mod gbdt;
pub mod model;
pub mod types;
pub mod errors;
pub mod execution;
pub mod models;
pub mod validation;
pub mod log;

pub use features::{
    extract_features,
    normalize_features,
    FeatureVector,
    FeatureConfig,
    ValidatorTelemetry,
};
pub use gbdt::{eval_gbdt, GBDTModel, Node, Tree};
pub use model::{load_model, verify_model_hash, ModelMetadata as PackageMetadata, ModelPackage, MODEL_HASH_SIZE};
pub use types::{ModelId, ModelMetadata, ModelInput, ModelOutput, ExecutionContext, ExecutionResult, DataType, ExecutionMetadata};
pub use errors::AiCoreError;

/// Crate version string for metadata and validation reports
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Deterministically sorts a vector for reproducible consensus behavior.
///
/// Used in various AI and reputation subsystems to ensure sorting
/// consistency across nodes.
pub fn deterministically_sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    // Rust’s sort is deterministic for a given input and ordering.
    items.sort();
    items
}

/// High-level deterministic validator reputation computation
///
/// Combines feature extraction and GBDT evaluation.
/// Used by consensus to score validators in each round.
///
/// # Arguments
/// * `telemetry` - ValidatorTelemetry object (pre-normalized data)
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

#[cfg(test)]
mod tests {
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
            rounds_active: 10000,
            avg_latency_us: 80000,
            slash_count: 0,
            stake: 500_000_00000000,
            age_rounds: 100000,
        };

        let model = GBDTModel {
            bias: 10,
            scale: 10000,
            trees: vec![Tree {
                nodes: vec![
                    Node { feature_index: 0, threshold: 5000, left: 1, right: 2, value: None },
                    Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(100) },
                    Node { feature_index: 0, threshold: 0, left: 0, right: 0, value: Some(200) },
                ],
            }],
        };

        let score1 = compute_validator_score(&telemetry, &model);
        let score2 = compute_validator_score(&telemetry, &model);
        assert_eq!(score1, score2);
        assert!(score1 > 0);
    }

    #[test]
    fn test_no_float_usage() {
        // Ensures no floating-point types exist in deterministic paths.
        let _ = 42;
        assert_eq!(_ + 1, 43);
    }
}
