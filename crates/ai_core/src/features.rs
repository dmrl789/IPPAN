//! Feature extraction and normalization for AI models
//!
//! All operations use integer arithmetic for determinism
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
    /// Maximum expected stake
    pub max_stake: u64,
    /// Maximum expected age in rounds
    pub max_age_rounds: u64,
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            scale: 10000,
            max_blocks_proposed: 100000,
            max_latency_us: 1_000_000,       // 1 second
            max_stake: 100_000_000_000_000,  // 1M IPN (with 8 decimals)
            max_age_rounds: 10_000_000,      // ~231 days at 200ms/round
        }
    }
}

/// Extract features from validator telemetry
///
/// Features (all scaled to [0, scale]):
/// 0. Proposal rate (blocks_proposed / rounds_active)
/// 1. Verification rate (blocks_verified / rounds_active)
/// 2. Average latency (normalized, inverted)
/// 3. Slash penalty (scale - slash_count * 1000)
/// 4. Stake weight (normalized)
/// 5. Longevity (normalized age)
pub fn extract_features(telemetry: &ValidatorTelemetry, config: &FeatureConfig) -> FeatureVector {
    let scale = config.scale;

    // Feature 0: Proposal rate
    let proposal_rate = if telemetry.rounds_active > 0 {
        let rate = (telemetry.blocks_proposed * scale as u64) / telemetry.rounds_active;
        let normalized = (rate * scale as u64) / config.max_blocks_proposed;
        normalized.min(scale as u64) as i64
    } else {
        0
    };

    // Feature 1: Verification rate
    let verification_rate = if telemetry.rounds_active > 0 {
        let rate = (telemetry.blocks_verified * scale as u64) / telemetry.rounds_active;
        let normalized = (rate * scale as u64) / config.max_blocks_proposed;
        normalized.min(scale as u64) as i64
    } else {
        0
    };

    // Feature 2: Latency (inverted - lower is better)
    let latency_score = if telemetry.avg_latency_us > 0 {
        let normalized = (telemetry.avg_latency_us * scale as u64) / config.max_latency_us;
        (scale - normalized.min(scale as u64) as i64).max(0)
    } else {
        scale
    };

    // Feature 3: Slash penalty (scale - count * 1000)
    let slash_penalty = (scale - (telemetry.slash_count as i64 * 1000)).max(0);

    // Feature 4: Stake weight
    let stake_weight = {
        let normalized = (telemetry.stake * scale as u64) / config.max_stake;
        normalized.min(scale as u64) as i64
    };

    // Feature 5: Longevity
    let longevity = {
        let normalized = (telemetry.age_rounds * scale as u64) / config.max_age_rounds;
        normalized.min(scale as u64) as i64
    };

    vec![
        proposal_rate,
        verification_rate,
        latency_score,
        slash_penalty,
        stake_weight,
        longevity,
    ]
}

/// Normalize raw feature values to scaled integers
///
/// # Arguments
/// * `raw_values` - Raw feature values
/// * `min_values` - Minimum value for each feature
/// * `max_values` - Maximum value for each feature
/// * `scale` - Output scale
pub fn normalize_features(
    raw_values: &[i64],
    min_values: &[i64],
    max_values: &[i64],
    scale: i64,
) -> FeatureVector {
    raw_values
        .iter()
        .zip(min_values.iter())
        .zip(max_values.iter())
        .map(|((&val, &min), &max)| {
            if max == min {
                return scale / 2; // Middle value if no range
            }
            let normalized = ((val - min) * scale) / (max - min);
            normalized.clamp(0, scale)
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
        let features = extract_features(&telemetry, &config);

        assert_eq!(features.len(), 6);
        assert_eq!(features[0], 0);
        assert_eq!(features[1], 0);
        assert_eq!(features[2], config.scale);
        assert_eq!(features[3], config.scale);
        assert_eq!(features[4], 0);
        assert_eq!(features[5], 0);
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
        let features = extract_features(&telemetry, &config);

        assert_eq!(features.len(), 6);
        assert!(features[0] > 0);
        assert!(features[1] > 0);
        assert!(features[2] > 8000);
        assert_eq!(features[3], config.scale);
        assert!(features[4] > 0);
        assert!(features[5] > 0);
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
        let features = extract_features(&telemetry, &config);

        assert!(features[3] < config.scale);
        assert_eq!(features[3], config.scale - 5000);
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
        let features = extract_features(&telemetry, &config);

        assert!(features[0] <= config.scale);
        assert!(features[1] <= config.scale);
        assert!(features[2] >= 0);
        assert!(features[3] >= 0);
        assert!(features[4] <= config.scale);
        assert!(features[5] <= config.scale);
    }

    #[test]
    fn test_normalize_features() {
        let raw = vec![50, 100, 150];
        let min = vec![0, 0, 0];
        let max = vec![100, 200, 200];
        let scale = 10000;

        let normalized = normalize_features(&raw, &min, &max, scale);

        assert_eq!(normalized, vec![5000, 5000, 7500]);
    }

    #[test]
    fn test_normalize_features_clamps() {
        let raw = vec![-10, 0, 150];
        let min = vec![0, 0, 0];
        let max = vec![100, 100, 100];
        let scale = 10000;

        let normalized = normalize_features(&raw, &min, &max, scale);

        assert_eq!(normalized[0], 0);
        assert_eq!(normalized[1], 0);
        assert_eq!(normalized[2], 10000);
    }

    #[test]
    fn test_normalize_features_same_min_max() {
        let raw = vec![50];
        let min = vec![100];
        let max = vec![100];
        let scale = 10000;

        let normalized = normalize_features(&raw, &min, &max, scale);
        assert_eq!(normalized[0], scale / 2);
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

        let f1 = extract_features(&telemetry, &config);
        let f2 = extract_features(&telemetry, &config);
        let f3 = extract_features(&telemetry, &config);

        assert_eq!(f1, f2);
        assert_eq!(f2, f3);
    }
}
