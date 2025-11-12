//! Validator scoring modules for consensus
//!
//! This module provides different scoring mechanisms for validator selection.

#[cfg(feature = "d_gbdt")]
pub mod d_gbdt;

#[cfg(feature = "d_gbdt")]
pub use d_gbdt::{
    extract_features, score_to_weight, score_validator, score_validators, ValidatorSnapshot,
    FEATURE_LEN, FEATURE_SCHEMA, MAX_WEIGHT, MIN_WEIGHT, SCALE,
};
