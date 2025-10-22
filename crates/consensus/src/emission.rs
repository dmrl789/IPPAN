use ippan_types::RoundId;

/// Parameters controlling the deterministic round-based emission schedule.
#[derive(Debug, Clone, Copy)]
pub struct EmissionParams {
    /// Initial emission per round in the base unit (e.g., µIPN).
    pub r0: u128,
    /// Number of rounds after which the emission halves.
    pub halving_rounds: u64,
    /// Proposer bonus in basis points (1/100th of a percent).
    /// For 20%, set to 2000 bps.
    pub proposer_bonus_bps: u16,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            // Start value chosen to be small and test-friendly. Projects can
            // override at runtime/config without changing the deterministic math.
            r0: 10_000,               // 10_000 base units per round
            halving_rounds: 100,      // halve every 100 rounds by default
            proposer_bonus_bps: 2000, // 20%
        }
    }
}

/// Compute the deterministic per-round emission R(t) using a simple halving
/// schedule: R(t) = R0 >> floor(t / halving_rounds).
///
/// The function is integer-only and deterministic across architectures.
pub fn round_reward(round: RoundId, params: &EmissionParams) -> u128 {
    if params.halving_rounds == 0 {
        return params.r0;
    }

    // Compute number of completed halving epochs for this round.
    let halvings = round / params.halving_rounds;

    // Clamp the shift to avoid undefined behavior for very large values.
    let shift = u32::try_from(halvings.min(127)).unwrap_or(127);
    params.r0.saturating_shr(shift)
}

/// Distribution result for a given round's emission pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Distribution {
    /// Amount allocated to the proposer.
    pub proposer: u128,
    /// Amount allocated to each verifier (equal split).
    pub per_verifier: u128,
    /// Number of verifiers assumed in the split.
    pub verifier_count: usize,
    /// Remainder left after integer division (can be recycled to the reward pool).
    pub remainder: u128,
}

/// Split a pool between the proposer (bonus) and verifiers (equal remainder).
///
/// - `pool`: total emission for the round or per-block slice
/// - `verifier_count`: number of parties that share the verifier portion
/// - `params.proposer_bonus_bps`: proposer share in basis points
pub fn split_proposer_and_verifiers(
    pool: u128,
    verifier_count: usize,
    params: &EmissionParams,
) -> Distribution {
    let proposer = (pool.saturating_mul(params.proposer_bonus_bps as u128)) / 10_000u128;
    let remaining = pool.saturating_sub(proposer);
    let count = verifier_count.max(1) as u128; // avoid division by zero
    let per_verifier = remaining / count;
    let remainder = remaining % count;

    Distribution {
        proposer,
        per_verifier,
        verifier_count: verifier_count.max(1),
        remainder,
    }
}

/// Divide a round pool across `block_count` blocks equally (floor), returning
/// the per-block slice and the leftover remainder (which can be added to a
/// cumulative pool and recycled periodically).
pub fn per_block_slice(pool: u128, block_count: usize) -> (u128, u128) {
    let count = block_count.max(1) as u128;
    let per_block = pool / count;
    let remainder = pool % count;
    (per_block, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_reward_halving() {
        let params = EmissionParams {
            r0: 1_024, // power of two to make halving exact
            halving_rounds: 10,
            proposer_bonus_bps: 2000,
        };

        assert_eq!(round_reward(0, &params), 1_024);
        assert_eq!(round_reward(9, &params), 1_024);
        assert_eq!(round_reward(10, &params), 512);
        assert_eq!(round_reward(19, &params), 512);
        assert_eq!(round_reward(20, &params), 256);
    }

    #[test]
    fn test_split_distribution_integrity() {
        let params = EmissionParams::default();
        let pool = 10_000u128;
        let dist = split_proposer_and_verifiers(pool, 5, &params);
        // 20% to proposer = 2_000
        assert_eq!(dist.proposer, 2_000);
        // Remaining 8_000 split among 5 verifiers => 1_600 each, remainder 0
        assert_eq!(dist.per_verifier, 1_600);
        assert_eq!(dist.remainder, 0);
        assert_eq!(dist.verifier_count, 5);
        // Sum check (without remainder since it's 0 here)
        assert_eq!(
            dist.proposer + dist.per_verifier * dist.verifier_count as u128,
            pool
        );
    }

    #[test]
    fn test_split_with_remainder() {
        let params = EmissionParams {
            proposer_bonus_bps: 2500,
            ..Default::default()
        }; // 25%
        let pool = 10_003u128;
        let dist = split_proposer_and_verifiers(pool, 3, &params);
        // Proposer gets floor(25% of 10003) = floor(2500.75) = 2500
        assert_eq!(dist.proposer, 2_500);
        // Remaining 7_503 / 3 = 2_501 each, remainder 0
        assert_eq!(dist.per_verifier, 2_501);
        assert_eq!(dist.remainder, 0);
    }

    #[test]
    fn test_per_block_slice() {
        let (per_block, rem) = per_block_slice(1_001, 10);
        assert_eq!(per_block, 100);
        assert_eq!(rem, 1);
    }
}
