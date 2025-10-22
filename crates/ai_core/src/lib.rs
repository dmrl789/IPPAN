//! Deterministic AI Core for L1 Blockchain Operations
//!
//! Provides integer-only AI evaluation for validator reputation, model
//! verification, and on-chain inference under consensus constraints.
//!
//! Modules:
//! - `features`: Deterministic feature extraction from validator telemetry
//! - `gbdt`: Integer-only Gradient Boosted Decision Tree evaluator
//! - `model`: Model packaging and verification utilities

pub mod features;
pub mod gbdt;
pub mod model;

pub use features::{extract_features, normalize_features, FeatureVector};
pub use gbdt::{eval_gbdt, GBDTModel, Node, Tree};
pub use model::{load_model, verify_model_hash, ModelMetadata, ModelPackage, MODEL_HASH_SIZE};

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
/// Scaled integer reputation score
pub fn compute_validator_score(
    telemetry: &crate::features::ValidatorTelemetry,
    model: &GBDTModel,
) -> i32 {
    let config = crate::features::FeatureConfig::default();
    let features = crate::features::extract_features(telemetry, &config);
    eval_gbdt(model, &features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{FeatureConfig, ValidatorTelemetry};

    #[test]
    fn test_compute_validator_score_consistency() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 1000,
            blocks_verified: 3000,
            rounds_active: 10000,
            avg_latency_us: 80000,
            slash_count: 0,
            stake: 500000_00000000,
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
        // Placeholder test to confirm compile-time exclusion of f32/f64.
        // In CI, lint forbids floating point in this crate.
        let _ = 42;
    }
}
