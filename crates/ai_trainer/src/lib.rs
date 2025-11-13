//! IPPAN AI Trainer - Deterministic offline GBDT trainer
//!
//! Provides tools for training Gradient Boosted Decision Tree models
//! with full determinism and reproducibility across platforms.

pub mod cart;
pub mod dataset;
pub mod deterministic;
pub mod trainer;

pub use cart::{CartBuilder, TreeConfig};
pub use dataset::Dataset;
pub use deterministic::{LcgRng, SplitTieBreaker};
pub use trainer::{GbdtConfig, GbdtTrainer};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
