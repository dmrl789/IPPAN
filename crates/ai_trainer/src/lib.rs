//! IPPAN AI Trainer - Deterministic offline GBDT trainer
//!
//! Provides tools for training Gradient Boosted Decision Tree models
//! with full determinism and reproducibility across platforms.

pub mod cart;
pub mod dataset;
pub mod deterministic;
pub mod errors;
pub mod trainer;

use ippan_ai_core::gbdt::Model;
use std::path::Path;

pub use dataset::{Dataset, FeatureStats, FEATURE_COLUMNS};
pub use deterministic::{LcgRng, SplitTieBreaker};
pub use errors::TrainerError;
pub use trainer::{GbdtTrainer, TrainingParams};

/// Train a deterministic model directly from a CSV file using the provided parameters.
pub fn train_model_from_csv(path: &Path, params: TrainingParams) -> Result<Model, TrainerError> {
    let dataset = Dataset::from_csv(path).map_err(|err| TrainerError::Dataset(err.to_string()))?;
    let trainer = GbdtTrainer::new(params);
    trainer
        .train(&dataset)
        .map_err(|err| TrainerError::Training(err.to_string()))
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
