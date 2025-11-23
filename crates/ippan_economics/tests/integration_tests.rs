//! Integration tests for IPPAN DAG-Fair Emission Framework
//!
//! These tests validate the full deterministic emission lifecycle, halving
//! logic, validator fairness, and supply tracking used across the economics
//! module of IPPAN.

use ippan_economics::prelude::*;
use ippan_economics::{projected_supply, ValidatorId, ValidatorParticipation, ValidatorRole};
use rust_decimal::Decimal;

#[test]
fn test_complete_emission_cycle() {
    let mut emission_engine = EmissionEngine::new();
    let mut supply_tracker = SupplyTracker::new(emission_engine.params().max_supply_micro);

    // Simulate 1000 rounds
    for round in 1..=1000 {
        let round_reward = emission_engine.calculate_round_reward(round).unwrap();
        supply_tracker.record_emission(round, round_reward).unwrap();
        emission_engine.advance_round(round).unwrap();
    }

    let supply_info = supply_tracker.get_supply_info();
    assert!(supply_info.total_supply > 0);
    assert!(supply_info.total_supply <= supply_info.supply_cap);
    assert_eq!(supply_info.current_round, 1000);
}

#[test]
fn test_halving_schedule() {
    let params = EmissionParams {
        halving_interval_rounds: 10, // small interval for test
        ..Default::default()
    };
    let emission_engine = EmissionEngine::with_params(params);

    let before_halving = emission_engine.calculate_round_reward(10).unwrap();
    let after_halving = emission_engine.calculate_round_reward(11).unwrap();

    assert_eq!(after_halving, before_halving / 2);

    let second_halving = emission_engine.calculate_round_reward(21).unwrap();
    assert_eq!(second_halving, before_halving / 4);
}

#[test]
fn test_emission_projection_alignment() {
    let params = EmissionParams {
        initial_round_reward_micro: 1_000,
        halving_interval_rounds: 4,
        max_supply_micro: 100_000,
        ..Default::default()
    };

    let mut emission_engine = EmissionEngine::with_params(params.clone());

    for round in 1..=8 {
        emission_engine.advance_round(round).unwrap();
    }

    let supply_info = emission_engine.get_supply_info();
    assert_eq!(supply_info.current_round, 8);
    assert_eq!(supply_info.total_supply, 6_000);

    let projected = projected_supply(8, &params);
    assert_eq!(u128::from(supply_info.total_supply), projected);
    assert_eq!(emission_engine.total_supply(), supply_info.total_supply);
}

#[test]
fn test_reward_distribution_fairness() {
    let params = EmissionParams::default();
    let round_rewards = RoundRewards::new(params);

    let participations = vec![
        ValidatorParticipation {
            validator_id: ValidatorId::new("high_participant"),
            role: ValidatorRole::Proposer,
            blocks_contributed: 20,
            uptime_score: Decimal::new(100, 2),
        },
        ValidatorParticipation {
            validator_id: ValidatorId::new("medium_participant"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 10,
            uptime_score: Decimal::new(90, 2),
        },
        ValidatorParticipation {
            validator_id: ValidatorId::new("low_participant"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 5,
            uptime_score: Decimal::new(80, 2),
        },
        ValidatorParticipation {
            validator_id: ValidatorId::new("observer"),
            role: ValidatorRole::Observer,
            blocks_contributed: 0,
            uptime_score: Decimal::new(100, 2),
        },
    ];

    let distribution = round_rewards
        .distribute_round_rewards(1, 10_000, participations, 1_000)
        .unwrap();

    let high = distribution
        .validator_rewards
        .get(&ValidatorId::new("high_participant"))
        .unwrap();
    let medium = distribution
        .validator_rewards
        .get(&ValidatorId::new("medium_participant"))
        .unwrap();
    let low = distribution
        .validator_rewards
        .get(&ValidatorId::new("low_participant"))
        .unwrap();
    let observer = distribution
        .validator_rewards
        .get(&ValidatorId::new("observer"))
        .unwrap();

    assert!(high.total_reward > medium.total_reward);
    assert!(medium.total_reward > low.total_reward);
    assert_eq!(observer.total_reward, 0);
}

#[test]
fn test_validator_uptime_impacts_rewards() {
    let round_rewards = RoundRewards::new(EmissionParams::default());

    let participations = vec![
        ValidatorParticipation {
            validator_id: ValidatorId::new("uptime_high"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 12,
            uptime_score: Decimal::new(95, 2), // 0.95
        },
        ValidatorParticipation {
            validator_id: ValidatorId::new("uptime_low"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 12,
            uptime_score: Decimal::new(65, 2), // 0.65
        },
    ];

    let distribution = round_rewards
        .distribute_round_rewards(7, 12_000, participations, 0)
        .unwrap();

    let high = distribution
        .validator_rewards
        .get(&ValidatorId::new("uptime_high"))
        .unwrap();
    let low = distribution
        .validator_rewards
        .get(&ValidatorId::new("uptime_low"))
        .unwrap();

    assert!(high.weight_factor > low.weight_factor);
    assert!(high.total_reward > low.total_reward);
}

#[test]
fn test_supply_cap_enforcement() {
    let mut supply_tracker = SupplyTracker::new(100_000);

    supply_tracker.record_emission(1, 150_000).unwrap();

    let info = supply_tracker.get_supply_info();
    assert_eq!(info.total_supply, 100_000);
    assert_eq!(info.remaining_supply, 0);
}

#[test]
fn test_fee_cap_enforcement() {
    let params = EmissionParams {
        fee_cap_fraction: Decimal::new(1, 1), // 10%
        ..Default::default()
    };

    let round_rewards = RoundRewards::new(params);
    let capped_high = round_rewards.apply_fee_cap(5_000, 10_000);
    assert_eq!(capped_high, 1_000);

    let capped_low = round_rewards.apply_fee_cap(500, 10_000);
    assert_eq!(capped_low, 500);
}

#[test]
fn test_emission_curve_generation() {
    let emission_engine = EmissionEngine::new();
    let curve = emission_engine.generate_emission_curve(1, 100, 10).unwrap();

    assert_eq!(curve.len(), 10);
    assert_eq!(curve[0].round, 1);
    assert_eq!(curve[0].reward_per_round, 10_000);

    for i in 1..curve.len() {
        assert!(curve[i].reward_per_round <= curve[i - 1].reward_per_round);
    }
}

#[test]
fn test_supply_audit() {
    let mut supply_tracker = SupplyTracker::new(1_000_000);

    supply_tracker.record_emission(1, 100_000).unwrap();
    supply_tracker.record_emission(2, 50_000).unwrap();
    supply_tracker.record_burn(2, 10_000).unwrap();

    let audit = supply_tracker.audit_supply();
    assert!(audit.is_healthy);
    assert_eq!(audit.total_emissions, 150_000);
    assert_eq!(audit.total_burns, 10_000);
    assert_eq!(audit.net_supply, 140_000);
}

#[test]
fn test_mathematical_precision() {
    let emission_engine = EmissionEngine::new();

    for round in 1..=1_000_000 {
        let reward = emission_engine.calculate_round_reward(round).unwrap();
        assert!(reward >= 1, "Reward at round {} was {}", round, reward);
    }
}

#[test]
fn test_overflow_protection() {
    let emission_engine = EmissionEngine::new();

    let large_round = u64::MAX;
    let result = emission_engine.calculate_round_reward(large_round);

    match result {
        Ok(_) => {}
        Err(EmissionError::CalculationOverflow(_)) => {}
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_round_reward_distribution_components() {
    let params = EmissionParams::default();
    let round_rewards = RoundRewards::new(params);

    let participations = vec![ValidatorParticipation {
        validator_id: ValidatorId::new("validator1"),
        role: ValidatorRole::Proposer,
        blocks_contributed: 10,
        uptime_score: Decimal::ONE,
    }];

    let distribution = round_rewards
        .distribute_round_rewards(1, 10_000, participations, 1_000)
        .unwrap();

    let reward = distribution
        .validator_rewards
        .get(&ValidatorId::new("validator1"))
        .unwrap();

    // Check that all direct components are present and dividends are routed to the pool
    assert!(reward.round_emission > 0);
    assert!(reward.transaction_fees > 0);
    assert!(reward.ai_commissions > 0);
    assert_eq!(reward.network_dividend, 0);
    assert!(distribution.network_pool_allocation > 0);

    let total = reward.round_emission + reward.transaction_fees + reward.ai_commissions;

    assert_eq!(reward.total_reward, total);
    assert_eq!(distribution.total_reward, total);
}

#[test]
fn test_network_pool_allocation_matches_dividend_policy() {
    let round_rewards = RoundRewards::new(EmissionParams::default());

    let participations = vec![
        ValidatorParticipation {
            validator_id: ValidatorId::new("validator_alpha"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 10,
            uptime_score: Decimal::ONE,
        },
        ValidatorParticipation {
            validator_id: ValidatorId::new("validator_beta"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 10,
            uptime_score: Decimal::ONE,
        },
    ];

    let round_reward = 10_000;
    let distribution = round_rewards
        .distribute_round_rewards(9, round_reward, participations, 0)
        .unwrap();

    let expected_treasury = (round_reward * 5) / 100;
    assert_eq!(distribution.network_pool_allocation, expected_treasury);
}

#[test]
fn test_no_burn_fee_and_emission_accounting() {
    let round_rewards = RoundRewards::new(EmissionParams::default());

    let participations = vec![ValidatorParticipation {
        validator_id: ValidatorId::new("validator_unique"),
        role: ValidatorRole::Proposer,
        blocks_contributed: 10,
        uptime_score: Decimal::ONE,
    }];

    let round_reward = 10_000;
    let fees_collected = 2_000;

    let distribution = round_rewards
        .distribute_round_rewards(3, round_reward, participations, fees_collected)
        .unwrap();

    let distributed: RewardAmount = distribution
        .validator_rewards
        .values()
        .map(|reward| reward.total_reward)
        .sum();

    assert_eq!(
        distributed + distribution.network_pool_allocation,
        round_reward + fees_collected
    );
}
