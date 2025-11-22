//! Consensus-facing deterministic fairness scoring API.
//!
//! This module exposes a minimal, stable surface for consensus code to apply
//! deterministic GBDT models to validator feature vectors using fixed-point
//! arithmetic only. All scaling is performed with integer math; no floating
//! point operations are used.

use crate::errors::{AiCoreError, Result};
use crate::gbdt::{Model as DgbdtModel, SCALE as DGBDT_SCALE};
use serde::{Deserialize, Serialize};

/// Fixed-point normalized validator features (0-10_000 scale per field).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatorFeatureVector {
    pub uptime: i64,
    pub latency_inverse: i64,
    pub honesty: i64,
    pub proposal_rate: i64,
    pub verification_rate: i64,
    pub stake_weight: i64,
}

impl ValidatorFeatureVector {
    /// Create a new normalized feature vector.
    pub fn new(
        uptime: i64,
        latency_inverse: i64,
        honesty: i64,
        proposal_rate: i64,
        verification_rate: i64,
        stake_weight: i64,
    ) -> Self {
        Self {
            uptime,
            latency_inverse,
            honesty,
            proposal_rate,
            verification_rate,
            stake_weight,
        }
    }

    /// Clamp all features into the expected 0-10_000 range.
    pub fn clamped(&self) -> Self {
        Self {
            uptime: self.uptime.clamp(0, 10_000),
            latency_inverse: self.latency_inverse.clamp(0, 10_000),
            honesty: self.honesty.clamp(0, 10_000),
            proposal_rate: self.proposal_rate.clamp(0, 10_000),
            verification_rate: self.verification_rate.clamp(0, 10_000),
            stake_weight: self.stake_weight.clamp(0, 10_000),
        }
    }

    /// Convert to a raw array for deterministic scoring.
    pub fn as_array(&self) -> [i64; 6] {
        [
            self.uptime,
            self.latency_inverse,
            self.honesty,
            self.proposal_rate,
            self.verification_rate,
            self.stake_weight,
        ]
    }

    /// Scale normalized features into the D-GBDT feature scale.
    pub fn scaled(&self, scale: i64) -> [i64; 6] {
        self.clamped()
            .as_array()
            .map(|value| value.saturating_mul(scale))
    }
}

/// Deterministic GBDT wrapper with consensus-friendly scoring helpers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeterministicFairnessModel {
    model: DgbdtModel,
}

impl DeterministicFairnessModel {
    /// Normalize 0-10_000 features into the GBDT scale (1e6).
    pub const FEATURE_SCALE: i64 = DGBDT_SCALE / 10_000;
    /// Maximum consensus score exposed to consensus callers.
    pub const MAX_SCORE: i64 = 10_000;

    /// Wrap an already validated deterministic model.
    pub fn from_model(model: DgbdtModel) -> Self {
        Self { model }
    }

    /// Load a deterministic model from disk using the shared loader.
    pub fn from_path(path: &std::path::Path) -> Result<Self> {
        let model = crate::load_model_from_path(path)?;
        Ok(Self::from_model(model))
    }

    /// Apply the model to a normalized feature vector and return a
    /// consensus-friendly score in the `[0, 10_000]` range.
    pub fn score(&self, normalized: &ValidatorFeatureVector) -> i64 {
        let scale = Self::feature_scale();
        let scaled = normalized.scaled(scale);
        let raw = self.model.score(&scaled);
        Self::quantize(raw)
    }

    /// Compute the canonical BLAKE3 hash for the wrapped model.
    pub fn hash_hex(&self) -> Result<String> {
        self.model.hash_hex().map_err(AiCoreError::from)
    }

    /// Access the underlying deterministic model (read-only).
    pub fn model(&self) -> &DgbdtModel {
        &self.model
    }

    /// Internal: determine feature scale with a safe non-zero default.
    const fn feature_scale() -> i64 {
        if Self::FEATURE_SCALE == 0 {
            1
        } else {
            Self::FEATURE_SCALE
        }
    }

    /// Internal: quantize raw model output into consensus score range.
    fn quantize(raw: i64) -> i64 {
        let divisor = Self::feature_scale();
        let normalized = raw / divisor;
        normalized.clamp(0, Self::MAX_SCORE)
    }
}
