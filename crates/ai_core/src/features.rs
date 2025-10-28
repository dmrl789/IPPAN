//! Feature extraction and normalization for AI models
//!
//! Deterministic, integer-based feature extraction used by the validator
//! reputation engine and D-GBDT model inference in IPPAN.

use serde::{Deserialize, Serialize};

/// Feature vector (scaled integers)
pub type FeatureVector = Vec<i64>;

/// Telemetry data for feature extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorTelemetry {
    /// Total blocks proposed by this validator
    pub blocks_proposed: u64,
    /// Total blocks verified by this validator
    pub blocks_verified: u64,
    /// Number of rounds this validator has been active
    pub rounds_active: u64,
    /// Average block proposal latency (microseconds)
    pub avg_latency_us: u64,
    /// Number of slashing events
    pub slash_count: u32,
    /// Current stake amount
    pub stake: u64,
    /// Age of validator (rounds since registration)
    pub age_rounds: u64,
}

/// Feature extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Scale factor for normalization (e.g., 10000 for 4 decimals)
    pub scale: i64,
    /// Maximum expected blocks proposed (for normalization)
    pub max_blocks_proposed: u64,
    /// Maximum expected latency in microseconds
    pub max_latency_us: u64,
    /// Maximum expected stake (in atomic units, 8 decimals)
    pub max_stake: u64,
    /// Maximum expected age in rounds
    pub max_age_rounds: u64,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            scale: 10000,
            max_blocks_proposed: 100_000,
            max_latency_us: 1_000_000,       // 1 second
            max_stake: 100_000_000_000_000,  // 1M IPN (with 8 decimals)
            max_age_rounds: 10_000_000,      // ~231 days at 200ms/round
        }
    }
}

/// Extract deterministic features from validator telemetry
///
/// Features (all scaled to [0, scale]):
/// 0. Proposal rate (blocks_proposed / rounds_active)
/// 1. Verification rate (blocks_verified / rounds_active)
/// 2. Latency score (inverted)
/// 3. Slash penalty (scale - count * 1000)
/// 4. Stake weight (normalized)
/// 5. Longevity (normalized age)
pub fn extract_features(telemetry: &ValidatorTelemetry, config: &FeatureConfig) -> FeatureVector {
    let scale = config.scale;

    // 0. Proposal rate
    let proposal_rate = if telemetry.rounds_active > 0 {
        let rate = (telemetry.blocks_proposed * scale as u64) / telemetry.rounds_active;
        ((rate * scale as u64) / config.max_blocks_proposed).min(scale as u64) as i64
    } else {
        0
    };

    // 1. Verification rate
    let verification_rate = if telemetry.rounds_active > 0 {
        let rate = (telemetry.blocks_verified * scale as u64) / telemetry.rounds_active;
        ((rate * scale as u64) / config.max_blocks_proposed).min(scale as u64) as i64
    } else {
        0
    };

    // 2. Latency score (inverted)
    let latency_score = if telemetry.avg_latency_us > 0 {
        let normalized = (telemetry.avg_latency_us * scale as u64) / config.max_latency_us;
        (scale - normalized.min(scale as u64) as i64).max(0)
    } else {
        scale
    };

    // 3. Slash penalty
    let slash_penalty = (scale - (telemetry.slash_count as i64 * 1000)).max(0);

    // 4. Stake weight
    let stake_weight = ((telemetry.stake * scale as u64) / config.max_stake)
        .min(scale as u64) as i64;

    // 5. Longevity
    let longevity = ((telemetry.age_rounds * scale as u64) / config.max_age_rounds)
        .min(scale as u64) as i64;

    vec![
        proposal_rate,
        verification_rate,
        latency_score,
        slash_penalty,
        stake_weight,
        longevity,
    ]
}

/// Normalize arbitrary raw features deterministically to [0, scale]
pub fn normalize_features(
    raw_values: &[i64],
    min_values: &[i64],
    max_values: &[i64],
    scale: i64,
) -> FeatureVector {
    raw_values
        .iter()
        .zip(min_values)
        .zip(max_values)
        .map(|((&val, &min), &max)| {
            if max == min {
                scale / 2
            } else {
                ((val - min) * scale / (max - min)).clamp(0, scale)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_features_all_zero() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 0,
            blocks_verified: 0,
            rounds_active: 0,
            avg_latency_us: 0,
            slash_count: 0,
            stake: 0,
            age_rounds: 0,
        };
        let config = FeatureConfig::default();
        let f = extract_features(&telemetry, &config);
        assert_eq!(f, vec![0, 0, config.scale, config.scale, 0, 0]);
    }

    #[test]
    fn test_extract_features_good_validator() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 1000,
            blocks_verified: 5000,
            rounds_active: 10000,
            avg_latency_us: 100000,
            slash_count: 0,
            stake: 100000_00000000,
            age_rounds: 1000000,
        };
        let config = FeatureConfig::default();
        let f = extract_features(&telemetry, &config);
        assert_eq!(f.len(), 6);
        assert!(f[0] > 0 && f[1] > 0 && f[2] > 8000);
        assert_eq!(f[3], config.scale);
    }

    #[test]
    fn test_extract_features_slashed_validator() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 1000,
            blocks_verified: 1000,
            rounds_active: 10000,
            avg_latency_us: 100000,
            slash_count: 5,
            stake: 100000_00000000,
            age_rounds: 1000000,
        };
        let config = FeatureConfig::default();
        let f = extract_features(&telemetry, &config);
        assert_eq!(f[3], config.scale - 5000);
    }

    #[test]
    fn test_extract_features_clamping() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 1_000_000,
            blocks_verified: 1_000_000,
            rounds_active: 1,
            avg_latency_us: 5_000_000,
            slash_count: 100,
            stake: 10_000_000_00000000,
            age_rounds: 100_000_000,
        };
        let config = FeatureConfig::default();
        let f = extract_features(&telemetry, &config);
        for v in f {
            assert!(v >= 0 && v <= config.scale);
        }
    }

    #[test]
    fn test_normalize_features() {
        let n = normalize_features(&[50, 100, 150], &[0, 0, 0], &[100, 200, 200], 10000);
        assert_eq!(n, vec![5000, 5000, 7500]);
    }

    #[test]
    fn test_normalize_features_clamps() {
        let n = normalize_features(&[-10, 0, 150], &[0, 0, 0], &[100, 100, 100], 10000);
        assert_eq!(n, vec![0, 0, 10000]);
    }

    #[test]
    fn test_normalize_features_same_min_max() {
        let n = normalize_features(&[50], &[100], &[100], 10000);
        assert_eq!(n, vec![5000]);
    }

    #[test]
    fn test_feature_determinism() {
        let telemetry = ValidatorTelemetry {
            blocks_proposed: 1234,
            blocks_verified: 5678,
            rounds_active: 10000,
            avg_latency_us: 123456,
            slash_count: 2,
            stake: 123456_00000000,
            age_rounds: 2000000,
        };
        let config = FeatureConfig::default();
        assert_eq!(
            extract_features(&telemetry, &config),
            extract_features(&telemetry, &config)
        );
    }
}
