//! Integration tests for DAG-Fair Emission System
//!
//! These tests verify the complete emission system behavior including:
//! - Round-based reward calculation
//! - Validator contribution weighting
//! - Supply cap enforcement
//! - Emission tracking and auditing
//! - Long-term emission schedule accuracy

use ippan_consensus::{
    distribute_round_reward, projected_supply, round_reward, rounds_until_cap, EmissionParams,
    EmissionTracker, TrackerValidatorContribution, ValidatorContribution,
};

#[test]
fn test_emission_schedule_accuracy() {
    let params = EmissionParams::default();

    // Test initial emission rate
    let r1 = round_reward(1, &params);
    assert_eq!(r1, 10_000, "Initial reward should be 10,000 µIPN");

    // Test first halving (at ~2 years, 630M rounds)
    let first_halving = params.halving_interval_rounds;
    let r_before = round_reward(first_halving - 1, &params);
    let r_after = round_reward(first_halving, &params);
    assert_eq!(r_before, 10_000);
    assert_eq!(r_after, 5_000, "Reward should halve at halving_rounds");

    // Test second halving
    let second_halving = params.halving_interval_rounds * 2;
    let r2_after = round_reward(second_halving, &params);
    assert_eq!(r2_after, 2_500, "Reward should halve again");
}

#[test]
fn test_supply_convergence() {
    let params = EmissionParams::default();

    // Calculate supply at various time points
    let year1_rounds = 315_360_000u64; // 1 year at 10 rounds/sec
    let year2_rounds = year1_rounds * 2;
    let year10_rounds = year1_rounds * 10;

    let s1 = projected_supply(year1_rounds, &params);
    let s2 = projected_supply(year2_rounds, &params);
    let s10 = projected_supply(year10_rounds, &params);

    // Verify monotonic increase
    assert!(s1 < s2);
    assert!(s2 < s10);

    // Verify convergence toward cap
    assert!(s10 < params.max_supply_micro as u128);

    // Year 1 should emit roughly 31,536 IPN
    let expected_year1 = 3_153_600_000_000_u128;
    let tolerance = expected_year1 / 100; // 1% tolerance
    assert!(
        s1 > expected_year1 - tolerance && s1 < expected_year1 + tolerance,
        "Year 1 emission: expected ~{}, got {}",
        expected_year1,
        s1
    );
}

#[test]
fn test_supply_cap_never_exceeded() {
    let params = EmissionParams::default();

    // Test at extreme round numbers
    let extreme_rounds = vec![
        1_000_000_000u64,   // 1 billion rounds
        10_000_000_000u64,  // 10 billion rounds
        100_000_000_000u64, // 100 billion rounds
        u64::MAX / 1000,    // Near maximum
    ];

    for rounds in extreme_rounds {
        let supply = projected_supply(rounds, &params);
        assert!(
            supply <= params.max_supply_micro as u128,
            "Supply {} exceeds cap {} at round {}",
            supply,
            params.max_supply_micro as u128,
            rounds
        );
    }
}

#[test]
fn test_validator_contribution_weighting() {
    let params = EmissionParams::default();

    // High performer: many blocks, high reputation, perfect uptime
    let high_performer = ValidatorContribution {
        validator_id: [1u8; 32],
        blocks_proposed: 100,
        blocks_verified: 200,
        reputation_score: 10000, // 100%
        uptime_factor: 10000,    // 100%
    };

    // Average performer: moderate blocks, good reputation, good uptime
    let avg_performer = ValidatorContribution {
        validator_id: [2u8; 32],
        blocks_proposed: 50,
        blocks_verified: 100,
        reputation_score: 8000, // 80%
        uptime_factor: 9000,    // 90%
    };

    // Low performer: few blocks, lower reputation, reduced uptime
    let low_performer = ValidatorContribution {
        validator_id: [3u8; 32],
        blocks_proposed: 10,
        blocks_verified: 20,
        reputation_score: 5000, // 50%
        uptime_factor: 7000,    // 70%
    };

    let high_score = high_performer.weighted_score(&params);
    let avg_score = avg_performer.weighted_score(&params);
    let low_score = low_performer.weighted_score(&params);

    // Verify ordering
    assert!(high_score > avg_score);
    assert!(avg_score > low_score);

    // High performer should get significantly more
    assert!(high_score > low_score * 5);
}

#[test]
fn test_fair_distribution_among_equals() {
    let params = EmissionParams::default();

    // Four validators with identical contributions
    let contributions: Vec<ValidatorContribution> = (1..=4)
        .map(|i| ValidatorContribution {
            validator_id: [i as u8; 32],
            blocks_proposed: 10,
            blocks_verified: 20,
            reputation_score: 10000,
            uptime_factor: 10000,
        })
        .collect();

    let distribution = distribute_round_reward(100, &params, &contributions, 1_000, 500, 10_000);

    // All validators should receive approximately equal rewards
    let rewards: Vec<u128> = distribution.validator_rewards.values().copied().collect();

    let avg_reward: u128 = rewards.iter().sum::<u128>() / rewards.len() as u128;
    let tolerance = avg_reward / 100; // 1% tolerance for rounding

    for &reward in &rewards {
        assert!(
            (reward as i128 - avg_reward as i128).abs() <= tolerance as i128,
            "Reward {} differs too much from average {}",
            reward,
            avg_reward
        );
    }
}

#[test]
fn test_proposer_bonus() {
    let params = EmissionParams::default();

    // Validator A: Only proposes
    let proposer_only = ValidatorContribution {
        validator_id: [1u8; 32],
        blocks_proposed: 10,
        blocks_verified: 0,
        reputation_score: 10000,
        uptime_factor: 10000,
    };

    // Validator B: Only verifies (same total work)
    let verifier_only = ValidatorContribution {
        validator_id: [2u8; 32],
        blocks_proposed: 0,
        blocks_verified: 10,
        reputation_score: 10000,
        uptime_factor: 10000,
    };

    let contributions = vec![proposer_only.clone(), verifier_only.clone()];
    let distribution = distribute_round_reward(100, &params, &contributions, 0, 0, 0);

    let proposer_reward = distribution.validator_rewards[&[1u8; 32]];
    let verifier_reward = distribution.validator_rewards[&[2u8; 32]];

    // Proposer should get more due to 1.2× weight
    assert!(proposer_reward > verifier_reward);

    // Ratio should be approximately 1.2:1
    let ratio = proposer_reward as f64 / verifier_reward as f64;
    assert!(
        (ratio - 1.2).abs() < 0.05,
        "Proposer/verifier ratio {} should be ~1.2",
        ratio
    );
}

#[test]
fn test_reputation_impact_on_rewards() {
    let params = EmissionParams::default();

    // High reputation validator
    let high_rep = ValidatorContribution {
        validator_id: [1u8; 32],
        blocks_proposed: 10,
        blocks_verified: 10,
        reputation_score: 10000, // 100%
        uptime_factor: 10000,
    };

    // Low reputation validator (same work)
    let low_rep = ValidatorContribution {
        validator_id: [2u8; 32],
        blocks_proposed: 10,
        blocks_verified: 10,
        reputation_score: 5000, // 50%
        uptime_factor: 10000,
    };

    let contributions = vec![high_rep, low_rep];
    let distribution = distribute_round_reward(100, &params, &contributions, 0, 0, 0);

    let high_reward = distribution.validator_rewards[&[1u8; 32]];
    let low_reward = distribution.validator_rewards[&[2u8; 32]];

    // High reputation should get approximately double
    let ratio = high_reward as f64 / low_reward as f64;
    assert!(
        (ratio - 2.0).abs() < 0.1,
        "Reputation impact ratio {} should be ~2.0",
        ratio
    );
}

#[test]
fn test_emission_tracker_integration() {
    let params = EmissionParams::default();
    let mut tracker = EmissionTracker::new(params.clone(), 1000);

    // Simulate 1000 rounds of operation
    for round in 1..=1000 {
        let contributions = vec![
            TrackerValidatorContribution {
                validator_id: [1u8; 32],
                blocks_proposed: 5,
                blocks_verified: 10,
                reputation_score: 10000.0,
            },
            TrackerValidatorContribution {
                validator_id: [2u8; 32],
                blocks_proposed: 3,
                blocks_verified: 8,
                reputation_score: 9000.0,
            },
        ];

        let result = tracker.process_round(round, &contributions, 100, 50);
        assert!(result.is_ok(), "Round {} failed: {:?}", round, result);
    }

    // Verify state consistency
    assert_eq!(tracker.last_round, 1000);
    assert!(tracker.cumulative_supply > 0);
    assert!(tracker.verify_consistency().is_ok());

    // Check statistics
    let stats = tracker.get_statistics();
    assert_eq!(stats.current_round, 1000);
    assert_eq!(stats.active_validators, 2);
    assert!(stats.cumulative_supply < params.max_supply_micro as u128);
}

#[test]
fn test_emission_with_varying_participation() {
    let params = EmissionParams::default();
    let mut tracker = EmissionTracker::new(params.clone(), 1000);

    // Simulate varying participation patterns
    for round in 1..=100 {
        let contributions = if round % 3 == 0 {
            // High participation round
            vec![
                TrackerValidatorContribution {
                    validator_id: [1u8; 32],
                    blocks_proposed: 10,
                    blocks_verified: 20,
                    reputation_score: 10000.0,
                },
                TrackerValidatorContribution {
                    validator_id: [2u8; 32],
                    blocks_proposed: 8,
                    blocks_verified: 15,
                    reputation_score: 9500.0,
                },
                TrackerValidatorContribution {
                    validator_id: [3u8; 32],
                    blocks_proposed: 5,
                    blocks_verified: 12,
                    reputation_score: 9000.0,
                },
            ]
        } else if round % 2 == 0 {
            // Medium participation
            vec![TrackerValidatorContribution {
                validator_id: [1u8; 32],
                blocks_proposed: 5,
                blocks_verified: 10,
                reputation_score: 10000.0,
            }]
        } else {
            // Low/no participation
            vec![]
        };

        let result =
            tracker.process_round(round, &contributions, 50 * contributions.len() as u128, 25);
        assert!(result.is_ok());
    }

    // Verify system handled varying participation
    assert!(tracker.empty_rounds > 0);
    assert!(tracker.verify_consistency().is_ok());
}

#[test]
fn test_long_term_emission_projection() {
    let params = EmissionParams::default();

    // Project 10 years of emission
    let years = 10;
    let rounds_per_year = 315_360_000u64;
    let total_rounds = rounds_per_year * years;

    let supply = projected_supply(total_rounds, &params);

    // After 10 years, should have emitted some supply but not all
    // With default params: r0=10,000 µIPN, halving every 630M rounds (~2 years)
    // Year 1-2: 10,000 × 630M = 6.3T µIPN
    // Year 3-4: 5,000 × 630M = 3.15T µIPN
    // Year 5-6: 2,500 × 630M = 1.575T µIPN
    // Year 7-8: 1,250 × 630M = 787.5B µIPN
    // Year 9-10: 625 × 630M = 393.75B µIPN
    // Total ~10 years: ~12.2T µIPN = 122 IPN (very small relative to 21M cap)
    assert!(supply > 0, "Should have emitted some supply");
    assert!(
        supply < params.max_supply_micro as u128,
        "Should not exceed cap"
    );

    // Calculate emission rate decay
    let year1 = projected_supply(rounds_per_year, &params);
    let year10 = supply;
    let year10_incremental = year10.saturating_sub(projected_supply(rounds_per_year * 9, &params));

    // Year 10 emission should be less than year 1 (due to halvings)
    // Only check if year 10 incremental is non-zero
    if year10_incremental > 0 {
        assert!(
            year10_incremental < year1,
            "Later years should emit less due to halvings"
        );
    }
}

#[test]
fn test_rounds_to_supply_cap() {
    // Use params where cap is achievable
    // Max theoretical supply = 2 × r0 × halving_rounds
    // With r0=100_000, halving_rounds=10_000: max = 2B
    // So use cap = 1.5B (75% of theoretical max)
    let params = EmissionParams {
        initial_round_reward_micro: 100_000,
        halving_interval_rounds: 10_000,
        max_supply_micro: 1_500_000_000, // 1.5B µIPN (achievable)
        ..Default::default()
    };

    let cap_round = rounds_until_cap(&params);

    assert!(cap_round > 0, "Should take non-zero rounds to reach cap");

    let supply_at_cap = projected_supply(cap_round, &params);
    let supply_before = projected_supply(cap_round.saturating_sub(1), &params);

    // Should be at or above cap (within one round's reward)
    assert!(
        supply_at_cap >= params.max_supply_micro as u128
            || (params.max_supply_micro as u128 - supply_at_cap)
                <= params.initial_round_reward_micro as u128,
        "Supply at cap round {} should be >= cap {}",
        supply_at_cap,
        params.max_supply_micro
    );

    // Previous round should be below cap
    assert!(
        supply_before < params.max_supply_micro as u128,
        "Supply before cap {} should be < cap {}",
        supply_before,
        params.max_supply_micro
    );
}

#[test]
fn test_component_distribution() {
    let params = EmissionParams::default();

    let contributions = vec![ValidatorContribution {
        validator_id: [1u8; 32],
        blocks_proposed: 10,
        blocks_verified: 10,
        reputation_score: 10000,
        uptime_factor: 10000,
    }];

    let distribution = distribute_round_reward(
        100,
        &params,
        &contributions,
        10_000,  // transaction fees
        5_000,   // AI commissions
        100_000, // network pool
    );

    // Verify all components contributed
    assert!(distribution.total_base_emission > 0);
    assert_eq!(distribution.transaction_fees, 10_000);
    assert_eq!(distribution.ai_commissions, 5_000);
    assert!(distribution.network_dividend > 0);

    // Total distributed should be sum of all components (proportionally)
    assert!(distribution.total_distributed > distribution.total_base_emission);
}

#[test]
fn test_zero_reward_after_many_halvings() {
    let params = EmissionParams {
        initial_round_reward_micro: 1000,
        halving_interval_rounds: 10,
        ..Default::default()
    };

    // After 64 halvings, reward should be 0
    let far_future = 65 * params.halving_interval_rounds;
    let reward = round_reward(far_future, &params);

    assert_eq!(reward, 0, "Reward should be 0 after 64+ halvings");
}

#[test]
fn test_distribution_consistency() {
    let params = EmissionParams::default();

    let contributions = vec![
        ValidatorContribution {
            validator_id: [1u8; 32],
            blocks_proposed: 10,
            blocks_verified: 20,
            reputation_score: 10000,
            uptime_factor: 10000,
        },
        ValidatorContribution {
            validator_id: [2u8; 32],
            blocks_proposed: 5,
            blocks_verified: 15,
            reputation_score: 8000,
            uptime_factor: 9000,
        },
    ];

    // Run same distribution multiple times
    let dist1 = distribute_round_reward(100, &params, &contributions, 1000, 500, 10_000);
    let dist2 = distribute_round_reward(100, &params, &contributions, 1000, 500, 10_000);

    // Results should be deterministic
    assert_eq!(dist1.total_distributed, dist2.total_distributed);
    assert_eq!(
        dist1.validator_rewards[&[1u8; 32]],
        dist2.validator_rewards[&[1u8; 32]]
    );
    assert_eq!(
        dist1.validator_rewards[&[2u8; 32]],
        dist2.validator_rewards[&[2u8; 32]]
    );
}

#[test]
fn test_audit_trail_creation() {
    let params = EmissionParams::default();
    let mut tracker = EmissionTracker::new(params.clone(), 100); // Audit every 100 rounds

    let contributions = vec![TrackerValidatorContribution {
        validator_id: [1u8; 32],
        blocks_proposed: 5,
        blocks_verified: 10,
        reputation_score: 10000.0,
    }];

    // Process 250 rounds (should create 2 audit checkpoints)
    for round in 1..=250 {
        tracker
            .process_round(round, &contributions, 100, 50)
            .unwrap();
    }

    assert!(
        tracker.audit_history.len() >= 2,
        "Should have at least 2 audit checkpoints"
    );

    // Verify audit records are sequential (end of one should be before start of next)
    for window in tracker.audit_history.windows(2) {
        assert!(
            window[0].end_round <= window[1].start_round,
            "Audit records should be sequential: record 0 ends at {}, record 1 starts at {}",
            window[0].end_round,
            window[1].start_round
        );
    }
}

#[test]
fn test_emission_params_validation() {
    let valid_params = EmissionParams::default();
    // Note: EmissionParams from ippan_economics doesn't have a validate() method
    // so we check basic invariants manually
    assert!(valid_params.initial_round_reward_micro > 0);
    assert!(valid_params.halving_interval_rounds > 0);
    assert!(valid_params.max_supply_micro > 0);

    // Test that proposer and verifier weights sum to 10000
    assert_eq!(
        valid_params.proposer_weight_bps + valid_params.verifier_weight_bps,
        10000
    );
}
