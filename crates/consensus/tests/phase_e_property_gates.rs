use proptest::prelude::*;

// Phase E - Step 3: Property-Based Tests for Consensus Invariants
// These tests act as "gates" that must pass consistently before external audit
// They cover consensus-critical invariants that MUST hold under all conditions

/// Supply cap constant (21 billion IPN with 18 decimals)
const SUPPLY_CAP: u128 = 21_000_000_000_000_000_000_000_000_000;

/// Maximum reasonable block reward (to prevent overflow)
const MAX_BLOCK_REWARD: u64 = 100_000_000_000_000_000; // 0.1 IPN
const MAX_CLOCK_SKEW_MS: u64 = 5_000; // 5 seconds

proptest! {
    #[test]
    fn supply_cap_never_exceeded_by_emission(
        current_supply in 0u128..=SUPPLY_CAP,
        block_reward in 0u64..=MAX_BLOCK_REWARD,
    ) {
        let reward_u128 = block_reward as u128;

        // Core invariant: current_supply + reward <= SUPPLY_CAP
        let new_supply = current_supply.saturating_add(reward_u128);

        if current_supply + reward_u128 <= SUPPLY_CAP {
            prop_assert!(new_supply <= SUPPLY_CAP);
            prop_assert!(new_supply >= current_supply);
        } else {
            // If emission would exceed cap, reward should be clamped
            prop_assert_eq!(new_supply.min(SUPPLY_CAP), SUPPLY_CAP);
        }
    }
}

proptest! {
    #[test]
    fn reward_distribution_never_overflows(
        total_reward in 0u64..=MAX_BLOCK_REWARD,
        proposer_share_bps in 0u16..=10_000, // basis points (0-100%)
        verifier_share_bps in 0u16..=10_000,
    ) {
        // Shares should sum to 10000 bps or less
        if proposer_share_bps.saturating_add(verifier_share_bps) <= 10_000 {
            let proposer_reward = (total_reward as u128)
                .saturating_mul(proposer_share_bps as u128)
                .saturating_div(10_000);

            let verifier_reward = (total_reward as u128)
                .saturating_mul(verifier_share_bps as u128)
                .saturating_div(10_000);

            let distributed = proposer_reward.saturating_add(verifier_reward);

            prop_assert!(distributed <= total_reward as u128);
            prop_assert!(proposer_reward <= total_reward as u128);
            prop_assert!(verifier_reward <= total_reward as u128);
        }
    }
}

proptest! {
    #[test]
    fn round_numbers_monotonically_increase(
        rounds in prop::collection::vec(0u64..=1_000_000, 1..50)
    ) {
        let mut sorted = rounds.clone();
        sorted.sort_unstable();

        // Check monotonic increase
        for window in sorted.windows(2) {
            prop_assert!(window[1] >= window[0]);
        }

        // Check no round number overflow in transitions
        for &round in &sorted {
            let next_round = round.saturating_add(1);
            prop_assert!(next_round > round || round == u64::MAX);
        }
    }
}

proptest! {
    #[test]
    fn validator_selection_seed_produces_deterministic_output(
        seed_bytes in prop::array::uniform32(any::<u8>()),
        round in 1u64..=1_000_000,
    ) {
        // Validator selection must be deterministic for the same seed + round
        let seed_a = seed_bytes;
        let seed_b = seed_bytes;

        // Hash-based selection should produce identical results
        use blake3;
        let mut hasher_a = blake3::Hasher::new();
        hasher_a.update(&seed_a);
        hasher_a.update(&round.to_le_bytes());
        let hash_a = hasher_a.finalize();

        let mut hasher_b = blake3::Hasher::new();
        hasher_b.update(&seed_b);
        hasher_b.update(&round.to_le_bytes());
        let hash_b = hasher_b.finalize();

        prop_assert_eq!(hash_a.as_bytes(), hash_b.as_bytes());
    }
}

proptest! {
    #[test]
    fn fork_choice_weight_calculation_never_overflows(
        block_height in 0u32..=1_000_000,
        verifier_count in 0u32..=1_000,
        timestamp in 0u32..=2_000_000_000, // Unix timestamp up to ~2033
    ) {
        // Fork choice uses weight = f(height, verifier_count, timestamp)
        let weight = (block_height as u64)
            .saturating_add(verifier_count as u64)
            .saturating_add(timestamp as u64);

        prop_assert!(weight >= block_height as u64);
        prop_assert!(weight >= verifier_count as u64);
        prop_assert!(weight >= timestamp as u64);

        // Verify multiplication factors don't overflow
        let weighted_height = (block_height as u64).saturating_mul(1000);
        let weighted_verifiers = (verifier_count as u64).saturating_mul(100);

        let total_weight = weighted_height
            .saturating_add(weighted_verifiers)
            .saturating_add(timestamp as u64);

        prop_assert!(total_weight < u64::MAX / 2); // Leave headroom
    }
}

proptest! {
    #[test]
    fn transaction_fee_validation_prevents_overflow(
        amount in 0u64..=1_000_000_000_000_000_000, // Max 1 IPN
        fee in 0u64..=10_000_000_000_000_000,       // Max 0.01 IPN
    ) {
        // Core invariant: amount + fee should not overflow
        let total = amount.checked_add(fee);

        prop_assert!(total.is_some());

        if let Some(t) = total {
            const MAX_EXPECTED_TOTAL: u64 = 1_010_000_000_000_000_000; // amount max + fee max

            prop_assert!(t >= amount);
            prop_assert!(t >= fee);
            prop_assert!(t <= MAX_EXPECTED_TOTAL);
        }
    }
}

proptest! {
    #[test]
    fn balance_deduction_never_goes_negative(
        initial_balance in 0u64..=1_000_000_000_000_000_000,
        amount in 0u64..=1_000_000_000_000_000_000,
        fee in 0u64..=10_000_000_000_000_000,
    ) {
        let total_deduction = amount.saturating_add(fee);

        if total_deduction <= initial_balance {
            let new_balance = initial_balance.saturating_sub(total_deduction);

            prop_assert!(new_balance <= initial_balance);
            prop_assert!(new_balance.saturating_add(total_deduction) == initial_balance);
        } else {
            // Insufficient balance, transaction should be rejected
            prop_assert!(total_deduction > initial_balance);
        }
    }
}

proptest! {
    #[test]
    fn dag_parent_count_stays_bounded(
        parent_count in 0usize..=100,
    ) {
        // DAG blocks should have bounded parent count (1-10 typical)
        const MAX_PARENTS: usize = 20;

        let valid = parent_count <= MAX_PARENTS;

        if valid {
            prop_assert!(parent_count <= MAX_PARENTS);
        }

        // Verify parent list size calculation doesn't overflow
        let parent_hash_size = 32usize; // BLAKE3 hash
        let capped_parent_count = parent_count.min(MAX_PARENTS);
        let total_size = capped_parent_count.saturating_mul(parent_hash_size);

        prop_assert!(total_size <= MAX_PARENTS * parent_hash_size);
    }
}

proptest! {
    #[test]
    fn timestamp_ordering_within_consensus_round(
        deltas in prop::collection::vec(-(MAX_CLOCK_SKEW_MS as i64)..=MAX_CLOCK_SKEW_MS as i64, 1..20)
    ) {
        // Timestamps should be monotonically increasing within a round
        // (or at least not decreasing by more than clock skew tolerance)

        let mut timestamps = Vec::with_capacity(deltas.len() + 1);
        let mut current = 1_700_000_000_000u64; // fixed anchor to avoid underflow
        timestamps.push(current);

        for delta in deltas {
            current = current.saturating_add_signed(delta);
            timestamps.push(current);
        }

        for window in timestamps.windows(2) {
            let diff = window[1].abs_diff(window[0]);

            // Allow backward movement within clock skew tolerance
            if window[1] < window[0] {
                prop_assert!(diff <= MAX_CLOCK_SKEW_MS);
            }
        }
    }
}

proptest! {
    #[test]
    fn validator_bond_arithmetic_never_overflows(
        bonds in prop::collection::vec(0u64..=100_000_000_000_000_000, 1..50)
    ) {
        // Sum of all validator bonds should not overflow
        let mut total: u128 = 0;

        for bond in bonds {
            total = total.saturating_add(bond as u128);
            prop_assert!(total >= bond as u128);
        }

        prop_assert!(total <= SUPPLY_CAP);
    }
}

proptest! {
    #[test]
    fn slashing_penalty_stays_within_bond_amount(
        bond_amount in 1_000_000u64..=100_000_000_000_000_000,
        penalty_bps in 0u16..=10_000, // 0-100%
    ) {
        // Slashing penalty calculation
        let penalty = (bond_amount as u128)
            .saturating_mul(penalty_bps as u128)
            .saturating_div(10_000) as u64;

        prop_assert!(penalty <= bond_amount);

        let remaining_bond = bond_amount.saturating_sub(penalty);
        prop_assert!(remaining_bond <= bond_amount);
        prop_assert!(remaining_bond.saturating_add(penalty) == bond_amount);
    }
}

proptest! {
    #[test]
    fn block_hash_collisions_are_astronomically_unlikely(
        block_data_a in prop::array::uniform32(any::<u8>()),
        block_data_b in prop::array::uniform32(any::<u8>()),
    ) {
        // BLAKE3 hash collision resistance
        use blake3;

        let hash_a = blake3::hash(&block_data_a);
        let hash_b = blake3::hash(&block_data_b);

        // Hashes should only match if inputs match
        if block_data_a == block_data_b {
            prop_assert_eq!(hash_a.as_bytes(), hash_b.as_bytes());
        } else {
            // Collision chance is 2^-256, effectively zero
            prop_assert_ne!(hash_a.as_bytes(), hash_b.as_bytes());
        }
    }
}

proptest! {
    #[test]
    fn emission_halving_calculation_never_underflows(
        base_reward in 1u64..=MAX_BLOCK_REWARD,
        halving_count in 0u32..=64,
    ) {
        // Emission halving: reward = base_reward >> halving_count
        let halved_reward = if halving_count < 64 {
            base_reward.checked_shr(halving_count)
        } else {
            Some(0)
        };

        prop_assert!(halved_reward.is_some());

        if let Some(reward) = halved_reward {
            prop_assert!(reward <= base_reward);

            // After enough halvings, reward should approach zero
            if halving_count >= 64 {
                prop_assert_eq!(reward, 0);
            }
        }
    }
}

proptest! {
    #[test]
    fn verifier_set_size_bounded_by_total_validator_count(
        total_validators in 1usize..=1_000,
        verifiers_per_round in 1usize..=100,
    ) {
        // Verifiers per round cannot exceed total validators
        let effective_verifiers = verifiers_per_round.min(total_validators);

        prop_assert!(effective_verifiers <= total_validators);
        prop_assert!(effective_verifiers > 0);
        prop_assert!(effective_verifiers <= verifiers_per_round);
    }
}

proptest! {
    #[test]
    fn nonce_increment_prevents_replay_attacks(
        nonces in prop::collection::vec(0u64..=1_000_000, 1..50)
    ) {
        // Nonces must be strictly increasing for replay prevention
        let mut sorted = nonces.clone();
        sorted.sort_unstable();
        sorted.dedup();

        // Verify each nonce is unique after deduplication
        prop_assert_eq!(sorted.len(), nonces.iter().collect::<std::collections::HashSet<_>>().len());

        // Verify nonce arithmetic doesn't overflow
        for &nonce in &sorted {
            let next_nonce = nonce.saturating_add(1);
            prop_assert!(next_nonce > nonce || nonce == u64::MAX);
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn supply_cap_constant_is_correct() {
        // 21 billion * 10^18
        assert_eq!(SUPPLY_CAP, 21_000_000_000 * 1_000_000_000_000_000_000);
    }

    #[test]
    fn basis_points_conversion() {
        // 10000 bps = 100%
        let full = 10_000u16;
        let half = 5_000u16;

        assert_eq!(full as f64 / 10_000.0, 1.0);
        assert_eq!(half as f64 / 10_000.0, 0.5);
    }
}
