//! Fairness-based reward weighting using D-GBDT model v1
//!
//! This module computes deterministic reward weights based on validator fairness scores.
//! Weights are normalized to preserve total payout while allowing ±20% variation.

/// Fixed-point scale factor (1e6)
pub const SCALE: i64 = 1_000_000;

/// Minimum reward multiplier (0.8x)
pub const MIN_MULT: i64 = 800_000;

/// Maximum reward multiplier (1.2x)
pub const MAX_MULT: i64 = 1_200_000;

/// Convert a fairness score to a reward multiplier
///
/// Maps score from [0..SCALE] to [MIN_MULT..MAX_MULT] linearly.
/// Uses i128 for intermediate calculations to avoid overflow.
fn score_to_multiplier(score: i64) -> i64 {
    // Clamp score to valid range
    let clamped_score = score.clamp(0, SCALE);

    // Linear mapping: mult = MIN_MULT + (score * (MAX_MULT - MIN_MULT)) / SCALE
    let range = (MAX_MULT - MIN_MULT) as i128;
    let scaled = (clamped_score as i128) * range;
    let mult = MIN_MULT as i128 + (scaled / SCALE as i128);

    mult as i64
}

/// Compute normalized reward weights for validators
///
/// # Arguments
/// * `scores` - Fairness scores for each validator (i64 fixed-point in [0..SCALE])
/// * `validator_ids` - Stable validator identifiers for tie-breaking
///
/// # Returns
/// Vector of normalized weights where sum(weights) == SCALE exactly
///
/// # Algorithm
/// 1. Convert scores to multipliers
/// 2. Normalize so sum equals SCALE
/// 3. Distribute any remainder deterministically by validator ID
pub fn compute_reward_weights(scores: &[i64], validator_ids: &[String]) -> Vec<i64> {
    if scores.is_empty() || validator_ids.is_empty() {
        return Vec::new();
    }

    if scores.len() != validator_ids.len() {
        return Vec::new();
    }

    // Step 1: Convert scores to multipliers
    let multipliers: Vec<i64> = scores
        .iter()
        .map(|&score| score_to_multiplier(score))
        .collect();

    // Step 2: Compute raw weights (equal baseline, scaled by multiplier)
    // For simplicity, use multiplier directly as raw weight
    let raw_weights: Vec<i64> = multipliers;

    // Step 3: Compute sum of raw weights
    let sum_raw: i128 = raw_weights.iter().map(|&w| w as i128).sum();

    if sum_raw == 0 {
        // If all weights are zero, return equal weights
        let equal_weight = SCALE / raw_weights.len() as i64;
        let remainder = SCALE % raw_weights.len() as i64;
        let mut weights = vec![equal_weight; raw_weights.len()];
        // Distribute remainder to first validators
        for i in 0..(remainder as usize) {
            if i < weights.len() {
                weights[i] += 1;
            }
        }
        return weights;
    }

    // Step 4: Normalize to SCALE
    let mut exact_weights: Vec<i64> = Vec::with_capacity(raw_weights.len());
    let mut remainders: Vec<(usize, i128)> = Vec::with_capacity(raw_weights.len());

    for (i, &raw) in raw_weights.iter().enumerate() {
        let exact = ((raw as i128) * SCALE as i128) / sum_raw;
        let rem = ((raw as i128) * SCALE as i128) % sum_raw;
        exact_weights.push(exact as i64);
        remainders.push((i, rem));
    }

    // Step 5: Distribute leftover deterministically
    let sum_exact: i64 = exact_weights.iter().sum();
    let leftover = SCALE - sum_exact;

    if leftover > 0 {
        // Sort by remainder (descending), then by validator ID (ascending) for tie-break
        remainders.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| validator_ids[a.0].cmp(&validator_ids[b.0]))
        });

        // Distribute +1 to validators with largest remainders
        for i in 0..(leftover as usize) {
            if i < remainders.len() {
                let idx = remainders[i].0;
                exact_weights[idx] += 1;
            }
        }
    }

    exact_weights
}

/// Distribute a total reward amount using weights
///
/// # Arguments
/// * `total_reward` - Total reward to distribute (u64)
/// * `weights` - Normalized weights (sum should equal SCALE)
/// * `validator_ids` - Validator identifiers for remainder distribution
///
/// # Returns
/// Vector of (validator_id, reward_amount) tuples
///
/// # Algorithm
/// 1. Compute floor shares: (total_reward * weight_i) / SCALE
/// 2. Distribute remainder deterministically by validator ID
pub fn distribute_by_weights(
    total_reward: u64,
    weights: &[i64],
    validator_ids: &[String],
) -> Vec<(String, u64)> {
    if weights.is_empty() || validator_ids.is_empty() || weights.len() != validator_ids.len() {
        return Vec::new();
    }

    if total_reward == 0 {
        return validator_ids.iter().map(|id| (id.clone(), 0)).collect();
    }

    // Step 1: Compute floor shares
    let mut shares: Vec<(usize, u64, u64)> = Vec::with_capacity(weights.len());
    let mut total_distributed: u64 = 0;

    for (i, &weight) in weights.iter().enumerate() {
        let share = ((total_reward as u128) * (weight as u128)) / SCALE as u128;
        let remainder = ((total_reward as u128) * (weight as u128)) % SCALE as u128;
        let share_u64 = share as u64;
        total_distributed = total_distributed.saturating_add(share_u64);
        shares.push((i, share_u64, remainder as u64));
    }

    // Step 2: Distribute leftover deterministically
    let leftover = total_reward.saturating_sub(total_distributed);

    if leftover > 0 {
        // Sort by remainder (descending), then by validator ID (ascending)
        shares.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then_with(|| validator_ids[a.0].cmp(&validator_ids[b.0]))
        });

        // Distribute +1 to validators with largest remainders
        for i in 0..(leftover as usize) {
            if i < shares.len() {
                shares[i].1 += 1;
            }
        }

        // Re-sort by original index to preserve order
        shares.sort_by_key(|&(idx, _, _)| idx);
    }

    // Build result vector
    shares
        .into_iter()
        .map(|(idx, amount, _)| (validator_ids[idx].clone(), amount))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_sum_to_scale_and_respects_caps() {
        let scores = vec![0, SCALE / 4, SCALE / 2, 3 * SCALE / 4, SCALE];
        let validator_ids = vec![
            "val0".to_string(),
            "val1".to_string(),
            "val2".to_string(),
            "val3".to_string(),
            "val4".to_string(),
        ];

        let weights = compute_reward_weights(&scores, &validator_ids);

        // Assert sum equals SCALE
        let sum: i64 = weights.iter().sum();
        assert_eq!(sum, SCALE, "Weights must sum to SCALE");

        // Assert all weights are non-negative
        for (i, &w) in weights.iter().enumerate() {
            assert!(w >= 0, "Weight {i} must be non-negative, got {w}");
        }

        // Assert relative ordering roughly matches score ordering
        // Higher scores should generally produce higher weights
        for i in 0..(scores.len() - 1) {
            if scores[i] < scores[i + 1] {
                // Weights should be monotonic for this simple set
                // (allowing for normalization effects)
                assert!(
                    weights[i] <= weights[i + 1] + 1, // Allow small rounding differences
                    "Weight ordering should match score ordering: weights[{}]={}, weights[{}]={}",
                    i,
                    weights[i],
                    i + 1,
                    weights[i + 1]
                );
            }
        }
    }

    #[test]
    fn remainder_distribution_is_deterministic() {
        let scores = vec![SCALE / 3, SCALE / 3, SCALE / 3]; // Will cause remainders
        let validator_ids = vec!["val1".to_string(), "val2".to_string(), "val3".to_string()];

        let weights1 = compute_reward_weights(&scores, &validator_ids);
        let weights2 = compute_reward_weights(&scores, &validator_ids);

        assert_eq!(weights1, weights2, "Weights must be deterministic");
        assert_eq!(weights1.iter().sum::<i64>(), SCALE);
    }

    #[test]
    fn payout_preserves_total() {
        let total_reward = 1_000_000u64;
        let weights = vec![200_000, 300_000, 500_000]; // Sum = SCALE
        let validator_ids = vec!["val1".to_string(), "val2".to_string(), "val3".to_string()];

        let payouts = distribute_by_weights(total_reward, &weights, &validator_ids);

        let sum_payouts: u64 = payouts.iter().map(|(_, amount)| amount).sum();
        assert_eq!(
            sum_payouts, total_reward,
            "Sum of payouts must equal total reward"
        );
    }

    #[test]
    fn score_to_multiplier_respects_caps() {
        assert_eq!(score_to_multiplier(0), MIN_MULT);
        assert_eq!(score_to_multiplier(SCALE), MAX_MULT);
        assert_eq!(score_to_multiplier(SCALE / 2), (MIN_MULT + MAX_MULT) / 2);

        // Test clamping
        assert_eq!(score_to_multiplier(-100), MIN_MULT);
        assert_eq!(score_to_multiplier(SCALE * 2), MAX_MULT);
    }

    #[test]
    fn empty_inputs_return_empty() {
        assert!(compute_reward_weights(&[], &[]).is_empty());
        assert!(distribute_by_weights(1000, &[], &[]).is_empty());
    }

    #[test]
    fn mismatched_lengths_return_empty() {
        let scores = vec![SCALE / 2];
        let ids = vec!["val1".to_string(), "val2".to_string()];
        assert!(compute_reward_weights(&scores, &ids).is_empty());
    }
}
