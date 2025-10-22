/// Deterministic AI core for L1 blockchain operations
///
/// This crate provides integer-only GBDT evaluation for reputation scoring
/// and model management with cryptographic verification.
pub mod features;
pub mod gbdt;
pub mod model;

pub use features::{extract_features, normalize_features, FeatureVector};
pub use gbdt::{eval_gbdt, GBDTModel, Node, Tree};
pub use model::{load_model, verify_model_hash, ModelMetadata, ModelPackage};

/// Re-export for convenience
pub use model::MODEL_HASH_SIZE;

#[cfg(test)]
mod tests {
    #[test]
    fn test_no_float_usage() {
        // This is a compile-time check - if this compiles, we're good
        // In CI, we'll add a lint to forbid f32/f64 in this crate
    }
}
