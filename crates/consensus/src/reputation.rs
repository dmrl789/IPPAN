/// Reputation-based validator selection using L1 AI
///
/// This module integrates deterministic GBDT evaluation into the consensus
/// validator selection process when the ai_l1 feature is enabled.

#[cfg(feature = "ai_l1")]
use ippan_ai_core::{
    eval_gbdt,
    features::{extract_features, FeatureConfig, ValidatorTelemetry as AiTelemetry},
    GBDTModel,
};

/// Validator reputation score (scaled 0-10000)
pub type ReputationScore = i32;

/// Default reputation score when AI is disabled or data unavailable
pub const DEFAULT_REPUTATION: ReputationScore = 5000;

/// Validator telemetry for reputation calculation
/// (Re-exported from ai_core when feature is enabled, otherwise defined here)
#[cfg(not(feature = "ai_l1"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidatorTelemetry {
    pub blocks_proposed: u64,
    pub blocks_verified: u64,
    pub rounds_active: u64,
    pub avg_latency_us: u64,
    pub slash_count: u32,
    pub stake: u64,
    pub age_rounds: u64,
}

#[cfg(feature = "ai_l1")]
pub type ValidatorTelemetry = AiTelemetry;

/// Calculate reputation score for a validator
///
/// # Arguments
/// * `telemetry` - Validator performance metrics
/// * `model` - GBDT model (only used when ai_l1 feature is enabled)
///
/// # Returns
/// Reputation score in range [0, 10000]
#[cfg(feature = "ai_l1")]
pub fn calculate_reputation(
    telemetry: &ValidatorTelemetry,
    model: Option<&GBDTModel>,
) -> ReputationScore {
    match model {
        Some(m) => {
            let config = FeatureConfig::default();
            let features = extract_features(telemetry, &config);
            eval_gbdt(m, &features)
        }
        None => DEFAULT_REPUTATION,
    }
}

/// Calculate reputation score for a validator (AI disabled)
#[cfg(not(feature = "ai_l1"))]
pub fn calculate_reputation(
    _telemetry: &ValidatorTelemetry,
    _model: Option<&()>,
) -> ReputationScore {
    DEFAULT_REPUTATION
}

/// Apply reputation weighting to validator stake
///
/// # Arguments
/// * `base_stake` - Validator's staked amount
/// * `reputation` - Reputation score [0, 10000]
///
/// # Returns
/// Weighted stake for validator selection
pub fn apply_reputation_weight(base_stake: u64, reputation: ReputationScore) -> u64 {
    // Reputation is in range [0, 10000], representing [0%, 100%]
    // Weight = base_stake * (reputation / 10000)
    // To avoid precision loss, we use: (base_stake * reputation) / 10000

    let reputation_u64 = reputation.max(0) as u64;
    base_stake.saturating_mul(reputation_u64) / 10000
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_default_reputation() {
        assert_eq!(DEFAULT_REPUTATION, 5000);
    }

    #[test]
    fn test_apply_reputation_weight_full() {
        let stake = 100_000u64;
        let reputation = 10000; // 100%
        assert_eq!(apply_reputation_weight(stake, reputation), 100_000);
    }

    #[test]
    fn test_apply_reputation_weight_half() {
        let stake = 100_000u64;
        let reputation = 5000; // 50%
        assert_eq!(apply_reputation_weight(stake, reputation), 50_000);
    }

    #[test]
    fn test_apply_reputation_weight_zero() {
        let stake = 100_000u64;
        let reputation = 0;
        assert_eq!(apply_reputation_weight(stake, reputation), 0);
    }

    #[test]
    fn test_apply_reputation_weight_negative_clamped() {
        let stake = 100_000u64;
        let reputation = -1000; // Should be clamped to 0
        assert_eq!(apply_reputation_weight(stake, reputation), 0);
    }

    #[test]
    fn test_calculate_reputation_no_model() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 100,
            blocks_verified: 500,
            rounds_active: 1000,
            avg_latency_us: 100_000,
            slash_count: 0,
            stake: 100_000_00000000,
            age_rounds: 10_000,
        };

        #[cfg(feature = "ai_l1")]
        let score = calculate_reputation(&telemetry, None);
        #[cfg(not(feature = "ai_l1"))]
        let score = calculate_reputation(&telemetry, None);

        assert_eq!(score, DEFAULT_REPUTATION);
    }

    #[cfg(feature = "ai_l1")]
    #[test]
    fn test_calculate_reputation_with_model() {
        use ippan_ai_core::gbdt::{GBDTModel, Node, Tree};

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
                        value: Some(3000),
                    },
                    Node {
                        feature_index: 0,
                        threshold: 0,
                        left: 0,
                        right: 0,
                        value: Some(7000),
                    },
                ],
            }],
            0,
            10000,
            6,
        )
        .expect("valid test model");

        let telemetry = ValidatorTelemetry {
            blocks_proposed: 100,
            blocks_verified: 500,
            rounds_active: 1000,
            avg_latency_us: 100_000,
            slash_count: 0,
            stake: 100_000_00000000,
            age_rounds: 10_000,
        };

        let score = calculate_reputation(&telemetry, Some(&model));
        assert!(score >= 0);
        assert!(score <= 10000);
    }

    #[test]
    fn test_apply_reputation_weight_large_stake() {
        let stake = u64::MAX / 2;
        let reputation = 5000; // 50%
        let weighted = apply_reputation_weight(stake, reputation);
        assert!(weighted > 0);
        assert!(weighted <= stake);
    }
}
