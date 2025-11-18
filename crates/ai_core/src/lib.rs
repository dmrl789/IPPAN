//! IPPAN deterministic AI core.
//!
//! This crate bundles the fixed-point math primitives, integer-only Gradient
//! Boosted Decision Trees (GBDT), telemetry feature extraction, and supporting
//! utilities required to run deterministic AI models inside consensus code.

pub mod config;
pub mod deployment;
pub mod determinism;
pub mod deterministic_gbdt;
pub mod errors;
pub mod execution;
pub mod feature_engineering;
pub mod features;
pub mod fixed;
pub mod fixed_point;
pub mod gbdt;
pub mod gbdt_legacy;
pub mod health;
pub mod log;
pub mod model;
pub mod model_hash;
pub mod model_manager;
pub mod models;
pub mod monitoring;
pub mod production_config;
pub mod security;
pub mod serde_canon;
pub mod serialization;
pub mod tests;
pub mod types;
pub mod validation;

// Re-exports -----------------------------------------------------------------

pub use crate::deterministic_gbdt::{
    compute_scores, normalize_features as deterministic_normalize_features, DecisionNode,
    DeterministicGBDT, GBDTTree, ValidatorFeatures,
};
pub use crate::errors::{AiCoreError, Result as AiCoreResult};
pub use crate::features::{extract_features, FeatureConfig, FeatureVector, ValidatorTelemetry};
pub use crate::fixed::{hash_fixed, hash_fixed_slice, Fixed, SCALE as FIXED_SCALE};
pub use crate::gbdt::{
    model_hash, Model as DeterministicModel, ModelError as DeterministicModelError,
    Node as DeterministicNode, Tree as DeterministicTree, SCALE as DGBDT_SCALE,
};
pub use crate::gbdt_legacy::{
    eval_gbdt, FeatureNormalization, GBDTError, GBDTMetrics, GBDTModel,
    ModelMetadata as LegacyGbdtMetadata, Node as LegacyNode, SecurityConstraints,
    Tree as LegacyTree,
};
pub use crate::model::{
    load_model, verify_model_hash, Model as LegacyModel, ModelMetadata, ModelPackage,
    MODEL_HASH_SIZE,
};
pub use crate::model_hash::model_hash_hex;
pub use crate::serialization::{canonical_model_json, load_model_from_path};
pub use crate::types::{
    DataType, ExecutionContext, ExecutionMetadata, ExecutionResult, ModelId, ModelInput,
    ModelMetadata as TypesModelMetadata, ModelOutput,
};

/// AI Core crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Deterministically sort a vector for reproducible hashing/validation.
pub fn deterministically_sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

/// Compute a validator reputation score using the legacy integer GBDT model.
///
/// This helper stitches together telemetry feature extraction with the legacy
/// `GBDTModel` evaluator so downstream crates can consume a single function.
pub fn compute_validator_score(telemetry: &ValidatorTelemetry, model: &GBDTModel) -> i32 {
    let config = FeatureConfig::default();
    let features = extract_features(telemetry, &config);
    eval_gbdt(model, &features)
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn test_deterministic_sort() {
        let sorted = deterministically_sorted(vec![3, 1, 2]);
        assert_eq!(sorted, vec![1, 2, 3]);
    }

    #[test]
    fn test_compute_validator_score_consistent() {
        use crate::gbdt_legacy::{Node, Tree};

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
            0,
            10_000,
            1,
        )
        .expect("valid model");

        let score_a = compute_validator_score(&telemetry, &model);
        let score_b = compute_validator_score(&telemetry, &model);
        assert_eq!(score_a, score_b);
    }
}
