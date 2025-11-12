//! Deterministic GBDT Scoring for Validator Selection
//!
//! This module integrates the deterministic GBDT model from ai_core
//! into the consensus_dlc validator selection mechanism.
//!
//! # Overview
//!
//! - Extracts features from validator telemetry in a deterministic way
//! - Uses ai_core's integer-only GBDT evaluation
//! - Converts scores to selection weights
//! - Falls back to default PoA weights if no model is available
//!
//! # Feature Schema
//!
//! The feature vector contains 7 deterministic i64 features (at SCALE = 10000):
//!
//! | Index | Feature            | Description                                    | Range        |
//! |-------|--------------------|------------------------------------------------|--------------|
//! | 0     | uptime_ms          | Uptime in milliseconds (normalized)            | [0, 10000]   |
//! | 1     | missed_rounds      | Number of missed rounds (inverted & clamped)   | [0, 10000]   |
//! | 2     | response_ms_p50    | Median response time in ms (inverted)          | [0, 10000]   |
//! | 3     | stake_i64_scaled   | Stake amount (normalized to scale)             | [0, 10000]   |
//! | 4     | slash_count        | Slash events (inverted penalty)                | [0, 10000]   |
//! | 5     | last_24h_blocks    | Blocks in last 24h (normalized)                | [0, 10000]   |
//! | 6     | age_rounds         | Validator age in rounds (normalized)           | [0, 10000]   |
//!
//! All features are scaled to [0, SCALE] using deterministic integer arithmetic.

use crate::dgbdt::ValidatorMetrics;
use crate::error::{DlcError, Result};
use serde::{Deserialize, Serialize};

/// Feature vector scale constant (matches ai_core::features::FeatureConfig::default)
pub const SCALE: i64 = 10_000;

/// Number of features in the D-GBDT model
pub const FEATURE_LEN: usize = 7;

/// Minimum selection weight to ensure liveness
pub const MIN_WEIGHT: i64 = 1;

/// Maximum selection weight cap
pub const MAX_WEIGHT: i64 = SCALE * 100; // 1,000,000

/// Deterministic feature schema for documentation and validation
pub const FEATURE_SCHEMA: [&str; FEATURE_LEN] = [
    "uptime_ms",
    "missed_rounds",
    "response_ms_p50",
    "stake_i64_scaled",
    "slash_count",
    "last_24h_blocks",
    "age_rounds",
];

/// Validator telemetry snapshot for deterministic feature extraction
///
/// This struct captures the essential telemetry data needed to score
/// a validator. All fields use integer types to ensure determinism.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ValidatorSnapshot {
    /// Validator identifier
    pub validator_id: String,
    /// Uptime in milliseconds
    pub uptime_ms: u64,
    /// Number of missed rounds
    pub missed_rounds: u64,
    /// Median response time in milliseconds (p50)
    pub response_ms_p50: u64,
    /// Stake amount in atomic units (scaled to i64 range)
    pub stake_i64_scaled: u64,
    /// Number of slash events
    pub slash_count: u32,
    /// Blocks proposed in last 24 hours
    pub last_24h_blocks: u64,
    /// Validator age in rounds
    pub age_rounds: u64,
}

impl ValidatorSnapshot {
    /// Convert from existing ValidatorMetrics (legacy compatibility)
    pub fn from_metrics(validator_id: String, metrics: &ValidatorMetrics) -> Self {
        // Convert f64 metrics to deterministic integers
        let uptime_ms = ((metrics.uptime * 86400.0 * 1000.0) as u64).min(86400000);
        let missed_rounds = (metrics.rounds_active / 10).saturating_sub(metrics.blocks_proposed);
        let response_ms_p50 = ((metrics.latency * 1000.0) as u64).min(10000);
        let stake_i64_scaled = (metrics.stake.atomic() / 1_000_000) as u64;
        let slash_count = 0; // Not tracked in legacy metrics
        let last_24h_blocks = metrics.blocks_proposed;
        let age_rounds = metrics.rounds_active;

        Self {
            validator_id,
            uptime_ms,
            missed_rounds,
            response_ms_p50,
            stake_i64_scaled,
            slash_count,
            last_24h_blocks,
            age_rounds,
        }
    }
}

/// Deterministic feature extraction from validator snapshot
///
/// Converts raw telemetry into a normalized feature vector suitable for
/// D-GBDT evaluation. All operations use integer arithmetic.
///
/// # Arguments
///
/// * `snapshot` - Validator telemetry snapshot
///
/// # Returns
///
/// A fixed-size feature vector of length FEATURE_LEN with values in [0, SCALE]
pub fn extract_features(snapshot: &ValidatorSnapshot) -> Vec<i64> {
    // Normalization constants (chosen to match typical validator behavior)
    const MAX_UPTIME_MS: u64 = 86_400_000; // 24 hours
    const MAX_MISSED_ROUNDS: u64 = 1000;
    const MAX_RESPONSE_MS: u64 = 5000; // 5 seconds
    const MAX_STAKE_SCALED: u64 = 1_000_000_000; // 1B units
    const MAX_SLASH_COUNT: u32 = 10;
    const MAX_BLOCKS_24H: u64 = 500;
    const MAX_AGE_ROUNDS: u64 = 100_000;

    // Feature 0: Uptime (higher is better)
    let uptime = snapshot.uptime_ms
        .saturating_mul(SCALE as u64)
        .checked_div(MAX_UPTIME_MS)
        .unwrap_or(SCALE as u64)
        .min(SCALE as u64) as i64;

    // Feature 1: Missed rounds (inverted - fewer misses is better)
    let missed_penalty = snapshot.missed_rounds
        .saturating_mul(SCALE as u64)
        .checked_div(MAX_MISSED_ROUNDS)
        .unwrap_or(SCALE as u64)
        .min(SCALE as u64) as i64;
    let missed_score = (SCALE - missed_penalty).max(0);

    // Feature 2: Response time (inverted - faster is better)
    let response_penalty = snapshot.response_ms_p50
        .saturating_mul(SCALE as u64)
        .checked_div(MAX_RESPONSE_MS)
        .unwrap_or(SCALE as u64)
        .min(SCALE as u64) as i64;
    let response_score = (SCALE - response_penalty).max(0);

    // Feature 3: Stake weight
    let stake_weight = snapshot.stake_i64_scaled
        .saturating_mul(SCALE as u64)
        .checked_div(MAX_STAKE_SCALED)
        .unwrap_or(0)
        .min(SCALE as u64) as i64;

    // Feature 4: Slash count (inverted - fewer slashes is better)
    let slash_penalty = (snapshot.slash_count.min(MAX_SLASH_COUNT) as i64 * 1000)
        .min(SCALE);
    let slash_score = (SCALE - slash_penalty).max(0);

    // Feature 5: Recent block production
    let blocks_score = snapshot.last_24h_blocks
        .saturating_mul(SCALE as u64)
        .checked_div(MAX_BLOCKS_24H)
        .unwrap_or(0)
        .min(SCALE as u64) as i64;

    // Feature 6: Validator longevity/age
    let age_score = snapshot.age_rounds
        .saturating_mul(SCALE as u64)
        .checked_div(MAX_AGE_ROUNDS)
        .unwrap_or(0)
        .min(SCALE as u64) as i64;

    vec![
        uptime,
        missed_score,
        response_score,
        stake_weight,
        slash_score,
        blocks_score,
        age_score,
    ]
}

/// Score a validator using the D-GBDT model
///
/// # Arguments
///
/// * `snapshot` - Validator telemetry snapshot
/// * `model` - Optional GBDT model (from ai_registry)
///
/// # Returns
///
/// Fixed-point score in range [0, SCALE*10] representing validator quality.
/// Returns a default score if model is None (fail-closed for liveness).
pub fn score_validator(
    snapshot: &ValidatorSnapshot,
    model: Option<&ippan_ai_core::gbdt::GBDTModel>,
) -> Result<i64> {
    // Extract features
    let features = extract_features(snapshot);

    // Validate feature vector length
    if features.len() != FEATURE_LEN {
        return Err(DlcError::Model(format!(
            "Feature vector length mismatch: expected {}, got {}",
            FEATURE_LEN,
            features.len()
        )));
    }

    // Use model if available, otherwise use default heuristic
    let score = match model {
        Some(m) => {
            // Use ai_core's deterministic eval_gbdt
            let result = ippan_ai_core::gbdt::eval_gbdt(m, &features);
            // Scale to our range using safe arithmetic
            // result is in model's scale, we want it in SCALE
            let result_i64 = result as i64;
            let scale_i64 = m.scale as i64;
            
            // Avoid overflow: (result * SCALE) / m.scale
            // Rewrite as: result / (m.scale / SCALE) to maintain precision
            if scale_i64 >= SCALE {
                result_i64 / (scale_i64 / SCALE)
            } else {
                result_i64.saturating_mul(SCALE / scale_i64)
            }
        }
        None => {
            // Default PoA-style scoring: weighted average of features
            // This preserves liveness even without a model
            let weights = [25, 15, 15, 10, 20, 10, 5]; // Sum to 100
            let mut weighted_sum = 0i64;
            for (feat, weight) in features.iter().zip(weights.iter()) {
                weighted_sum = weighted_sum.saturating_add(feat.saturating_mul(*weight));
            }
            weighted_sum / 100 // Average
        }
    };

    Ok(score)
}

/// Convert a score to a selection weight
///
/// Clamps the score to a reasonable range [MIN_WEIGHT, MAX_WEIGHT]
/// to ensure fair validator selection and prevent extreme weights.
///
/// # Arguments
///
/// * `score` - Raw score from score_validator
///
/// # Returns
///
/// Weight suitable for probabilistic or deterministic validator selection
pub fn score_to_weight(score: i64) -> i64 {
    score.clamp(MIN_WEIGHT, MAX_WEIGHT)
}

/// Score multiple validators and return sorted list
///
/// # Arguments
///
/// * `snapshots` - List of validator snapshots
/// * `model` - Optional GBDT model
///
/// # Returns
///
/// Vector of (validator_id, score, weight) tuples, sorted by score descending
pub fn score_validators(
    snapshots: &[ValidatorSnapshot],
    model: Option<&ippan_ai_core::gbdt::GBDTModel>,
) -> Result<Vec<(String, i64, i64)>> {
    let mut results = Vec::new();

    for snapshot in snapshots {
        let score = score_validator(snapshot, model)?;
        let weight = score_to_weight(score);
        results.push((snapshot.validator_id.clone(), score, weight));
    }

    // Sort by score descending, then by validator_id for determinism
    results.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.cmp(&b.0))
    });

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_schema_length() {
        assert_eq!(FEATURE_SCHEMA.len(), FEATURE_LEN);
    }

    #[test]
    fn test_extract_features_default() {
        let snapshot = ValidatorSnapshot::default();
        let features = extract_features(&snapshot);

        assert_eq!(features.len(), FEATURE_LEN);
        // Default snapshot should have mostly zero features
        // except inverted ones which should be max
        assert_eq!(features[1], SCALE); // missed_rounds inverted
        assert_eq!(features[2], SCALE); // response_time inverted
        assert_eq!(features[4], SCALE); // slash_count inverted
    }

    #[test]
    fn test_extract_features_perfect_validator() {
        let snapshot = ValidatorSnapshot {
            validator_id: "perfect".to_string(),
            uptime_ms: 86_400_000, // 24h
            missed_rounds: 0,
            response_ms_p50: 100, // 100ms
            stake_i64_scaled: 1_000_000_000,
            slash_count: 0,
            last_24h_blocks: 500,
            age_rounds: 100_000,
        };

        let features = extract_features(&snapshot);

        assert_eq!(features.len(), FEATURE_LEN);
        assert_eq!(features[0], SCALE); // max uptime
        assert_eq!(features[1], SCALE); // no missed rounds
        assert_eq!(features[4], SCALE); // no slashes
        assert!(features[2] > SCALE * 9 / 10); // good response time
    }

    #[test]
    fn test_extract_features_deterministic() {
        let snapshot = ValidatorSnapshot {
            validator_id: "test".to_string(),
            uptime_ms: 50_000_000,
            missed_rounds: 10,
            response_ms_p50: 500,
            stake_i64_scaled: 100_000_000,
            slash_count: 1,
            last_24h_blocks: 100,
            age_rounds: 10_000,
        };

        let features1 = extract_features(&snapshot);
        let features2 = extract_features(&snapshot);

        assert_eq!(features1, features2, "Feature extraction must be deterministic");
    }

    #[test]
    fn test_score_validator_without_model() {
        let snapshot = ValidatorSnapshot {
            validator_id: "test".to_string(),
            uptime_ms: 86_400_000,
            missed_rounds: 0,
            response_ms_p50: 100,
            stake_i64_scaled: 500_000_000,
            slash_count: 0,
            last_24h_blocks: 250,
            age_rounds: 50_000,
        };

        let score = score_validator(&snapshot, None).unwrap();
        assert!(score > 0, "Score should be positive for good validator");
        assert!(score <= SCALE * 10, "Score should not exceed maximum");
    }

    #[test]
    fn test_score_validator_deterministic() {
        let snapshot = ValidatorSnapshot {
            validator_id: "test".to_string(),
            uptime_ms: 43_200_000, // 12h
            missed_rounds: 5,
            response_ms_p50: 200,
            stake_i64_scaled: 250_000_000,
            slash_count: 0,
            last_24h_blocks: 100,
            age_rounds: 20_000,
        };

        let score1 = score_validator(&snapshot, None).unwrap();
        let score2 = score_validator(&snapshot, None).unwrap();

        assert_eq!(score1, score2, "Scoring must be deterministic");
    }

    #[test]
    fn test_score_to_weight() {
        assert_eq!(score_to_weight(0), MIN_WEIGHT);
        assert_eq!(score_to_weight(-100), MIN_WEIGHT);
        assert_eq!(score_to_weight(5000), 5000);
        assert_eq!(score_to_weight(MAX_WEIGHT + 1000), MAX_WEIGHT);
    }

    #[test]
    fn test_score_validators_ordering() {
        let snapshots = vec![
            ValidatorSnapshot {
                validator_id: "good".to_string(),
                uptime_ms: 86_400_000,
                missed_rounds: 0,
                response_ms_p50: 100,
                stake_i64_scaled: 1_000_000_000,
                slash_count: 0,
                last_24h_blocks: 500,
                age_rounds: 100_000,
            },
            ValidatorSnapshot {
                validator_id: "bad".to_string(),
                uptime_ms: 10_000_000,
                missed_rounds: 100,
                response_ms_p50: 2000,
                stake_i64_scaled: 10_000_000,
                slash_count: 5,
                last_24h_blocks: 10,
                age_rounds: 1_000,
            },
            ValidatorSnapshot {
                validator_id: "medium".to_string(),
                uptime_ms: 43_200_000,
                missed_rounds: 10,
                response_ms_p50: 300,
                stake_i64_scaled: 500_000_000,
                slash_count: 0,
                last_24h_blocks: 200,
                age_rounds: 50_000,
            },
        ];

        let results = score_validators(&snapshots, None).unwrap();

        assert_eq!(results.len(), 3);
        // Good validator should rank first
        assert_eq!(results[0].0, "good");
        assert!(results[0].1 > results[1].1, "Good score > medium score");
        assert!(results[1].1 > results[2].1, "Medium score > bad score");
    }

    #[test]
    fn test_validator_snapshot_from_metrics() {
        use ippan_types::Amount;

        let metrics = ValidatorMetrics::new(
            0.99,
            0.1,
            1.0,
            100,
            500,
            Amount::from_micro_ipn(10_000_000),
            1000,
        );

        let snapshot = ValidatorSnapshot::from_metrics("test".to_string(), &metrics);

        assert_eq!(snapshot.validator_id, "test");
        assert!(snapshot.uptime_ms > 0);
        assert_eq!(snapshot.age_rounds, 1000);
    }
}
