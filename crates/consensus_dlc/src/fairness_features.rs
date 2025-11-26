//! Feature extraction for fairness model v1
//!
//! Maps validator metrics to the 7 features required by the deterministic fairness model:
//! 1. uptime_ratio_7d
//! 2. validated_blocks_7d
//! 3. missed_blocks_7d
//! 4. avg_latency_ms
//! 5. slashing_events_90d
//! 6. stake_normalized
//! 7. peer_reports_quality
//!
//! All features are fixed-point i64 scaled by SCALE (1_000_000).

use crate::dgbdt::ValidatorMetrics;
use ippan_types::Amount;

/// Fixed-point scale factor (1e6)
pub const SCALE: i64 = 1_000_000;

/// Extract 7 features for the fairness model v1
///
/// Features are in the order expected by the model:
/// 1. uptime_ratio_7d: [0..SCALE] (0-100%)
/// 2. validated_blocks_7d: count * SCALE
/// 3. missed_blocks_7d: count * SCALE
/// 4. avg_latency_ms: latency_ms * SCALE
/// 5. slashing_events_90d: events * SCALE
/// 6. stake_normalized: [0..SCALE] (normalized by max stake)
/// 7. peer_reports_quality: [0..SCALE] (0-100%)
pub fn features_for_validator(metrics: &ValidatorMetrics, max_stake: Amount) -> [i64; 7] {
    // 1. uptime_ratio_7d: Use uptime field (already scaled 0-10000), convert to SCALE
    let uptime_ratio_7d = if metrics.uptime > 0 {
        // Convert from 0-10000 scale to 0-SCALE scale
        (metrics.uptime as i64 * SCALE) / 10_000
    } else {
        SCALE / 2 // Default to neutral (50%) if no data
    };

    // 2. validated_blocks_7d: Use blocks_verified (approximate for 7d window)
    // For now, use blocks_verified as proxy (scaled by SCALE)
    let validated_blocks_7d = (metrics.blocks_verified as i64).saturating_mul(SCALE);

    // 3. missed_blocks_7d: Estimate from rounds_active and blocks_proposed
    // missed = expected - proposed, where expected = rounds_active
    let expected_blocks = metrics.rounds_active as i64;
    let proposed_blocks = metrics.blocks_proposed as i64;
    let missed_blocks_7d = expected_blocks
        .saturating_sub(proposed_blocks)
        .max(0)
        .saturating_mul(SCALE);

    // 4. avg_latency_ms: Use latency field (already scaled), convert to ms * SCALE
    // latency is in 0-10000 scale, convert to approximate ms
    // Assuming latency is in 0-10000 scale where 10000 = 100% = some max latency
    // For now, treat it as relative and scale: latency_ms ≈ (latency * 100) / 10000 * SCALE
    let avg_latency_ms = if metrics.latency > 0 {
        // Convert from 0-10000 scale to approximate ms, then scale
        // Rough approximation: 10000 = 1000ms, so latency_ms = (latency * 1000) / 10000
        let latency_ms = (metrics.latency * 1000) / 10_000;
        latency_ms.saturating_mul(SCALE)
    } else {
        0
    };

    // 5. slashing_events_90d: Not available in current metrics, default to 0
    let slashing_events_90d = 0;

    // 6. stake_normalized: Normalize stake by max_stake, clamped to [0..SCALE]
    let stake_normalized = if max_stake.atomic() > 0 {
        let stake_micro = metrics.stake.atomic();
        let max_micro = max_stake.atomic();
        // Compute (stake * SCALE) / max_stake, clamped
        let normalized = (stake_micro * SCALE as u128)
            .checked_div(max_micro)
            .unwrap_or(0);
        (normalized.min(SCALE as u128)) as i64
    } else {
        SCALE // If all have zero stake, treat as equal (100%)
    };

    // 7. peer_reports_quality: Use honesty field as proxy (already scaled 0-10000)
    // Convert from 0-10000 scale to 0-SCALE scale
    let peer_reports_quality = if metrics.honesty > 0 {
        (metrics.honesty as i64 * SCALE) / 10_000
    } else {
        SCALE / 2 // Default to neutral (50%) if no data
    };

    [
        uptime_ratio_7d,
        validated_blocks_7d,
        missed_blocks_7d,
        avg_latency_ms,
        slashing_events_90d,
        stake_normalized,
        peer_reports_quality,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::Amount;

    #[test]
    fn test_features_are_scaled() {
        let metrics = ValidatorMetrics::default();
        let max_stake = Amount::from_micro_ipn(10_000_000);
        let features = features_for_validator(&metrics, max_stake);

        // All features should be in reasonable range (0 to SCALE or reasonable multiples)
        for (i, &feat) in features.iter().enumerate() {
            assert!(
                feat >= 0,
                "Feature {} should be non-negative, got {}",
                i,
                feat
            );
            // Features can exceed SCALE for counts (validated_blocks, missed_blocks, latency_ms)
            // but ratios should be in [0..SCALE]
            if i == 0 || i == 5 || i == 6 {
                // uptime_ratio, stake_normalized, peer_reports_quality should be <= SCALE
                assert!(
                    feat <= SCALE,
                    "Feature {} (ratio) should be <= SCALE, got {}",
                    i,
                    feat
                );
            }
        }
    }

    #[test]
    fn test_features_deterministic() {
        let metrics = ValidatorMetrics::default();
        let max_stake = Amount::from_micro_ipn(10_000_000);

        let features1 = features_for_validator(&metrics, max_stake);
        let features2 = features_for_validator(&metrics, max_stake);

        assert_eq!(features1, features2, "Features should be deterministic");
    }
}
