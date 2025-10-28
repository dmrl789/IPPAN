//! Integration tests for IPPAN DAG-Fair Emission Framework

use ippan_economics::prelude::*;
use ippan_economics::{ValidatorId, ValidatorParticipation, ValidatorRole};
use rust_decimal::Decimal;

#[test]
fn test_complete_emission_cycle() {
    // Test a complete emission cycle from genesis to halving
    let emission_engine = EmissionEngine::new();
    let mut supply_tracker = SupplyTracker::new(emission_engine.params().total_supply_cap);

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
    let mut params = EmissionParams::default();
    params.halving_interval = 10; // Small interval for testing
    let emission_engine = EmissionEngine::with_params(params);

    let before_halving = emission_engine.calculate_round_reward(10).unwrap();
    let after_halving = emission_engine.calculate_round_reward(11).unwrap();

    assert_eq!(after_halving, before_halving / 2);

    let second_halving = emission_engine.calculate_round_reward(21).unwrap();
    assert_eq!(second_halving, before_halving / 4);
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
            uptime_score: Decimal::new(100, 2), // 1.0
        },
        ValidatorParticipation {
            validator_id: ValidatorId::new("medium_participant"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 10,
            uptime_score: Decimal::new(90, 2), // 0.9
        },
        ValidatorParticipation {
            validator_id: ValidatorId::new("low_participant"),
            role: ValidatorRole::Verifier,
            blocks_contributed: 5,
            uptime_score: Decimal::new(80, 2), // 0.8
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

    let high_reward = distribution.validator_rewards.get(&ValidatorId::new("high_participant")).unwrap();
    let medium_reward = distribution.validator_rewards.get(&ValidatorId::new("medium_participant")).unwrap();
    let low_reward = distribution.validator_rewards.get(&ValidatorId::new("low_participant")).unwrap();
    let observer_reward = distribution.validator_rewards.get(&ValidatorId::new("observer")).unwrap();

    assert!(high_reward.total_reward > medium_reward.total_reward);
    assert!(medium_reward.total_reward > low_reward.total_reward);
    assert_eq!(observer_reward.total_reward, 0);
}

#[test]
fn test_supply_cap_enforcement() {
    let mut supply_tracker = SupplyTracker::new(100_000);

    supply_tracker.record_emission(1, 150_000).unwrap();

    let supply_info = supply_tracker.get_supply_info();
    assert_eq!(supply_info.total_supply, 100_000);
    assert_eq!(supply_info.remaining_supply, 0);
}

#[test]
fn test_fee_cap_enforcement() {
    let mut params = EmissionParams::default();
    params.fee_cap_fraction = Decimal::new(1, 1); // 10%

    let round_rewards = RoundRewards::new(params);

    let capped_fees = round_rewards.apply_fee_cap(5_000, 10_000);
    assert_eq!(capped_fees, 1_000);

    let capped_fees_low = round_rewards.apply_fee_cap(500, 10_000);
    assert_eq!(capped_fees_low, 500);
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

    let audit_result = supply_tracker.audit_supply();
    assert!(audit_result.is_healthy);
    assert_eq!(audit_result.total_emissions, 150_000);
    assert_eq!(audit_result.total_burns, 10_000);
    assert_eq!(audit_result.net_supply, 140_000);
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
    let mut emission_engine = EmissionEngine::new();

    let large_round = u64::MAX;
    let result = emission_engine.calculate_round_reward(large_round);

    match result {
        Ok(_) => {}, // Reward can be 0 or positive due to halving
        Err(EmissionError::CalculationOverflow(_)) => {}, // Expected for very large rounds
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_round_reward_distribution_components() {
    let params = EmissionParams::default();
    let round_rewards = RoundRewards::new(params);

    let participations = vec![
        ValidatorParticipation {
            validator_id: ValidatorId::new("validator1"),
            role: ValidatorRole::Proposer,
            blocks_contributed: 10,
            uptime_score: Decimal::ONE,
        },
    ];

    let distribution = round_rewards
        .distribute_round_rewards(1, 10_000, participations, 1_000)
        .unwrap();

    let validator_reward = distribution
        .validator_rewards
        .get(&ValidatorId::new("validator1"))
        .unwrap();

    assert!(validator_reward.round_emission > 0);
    assert!(validator_reward.transaction_fees > 0);
    assert!(validator_reward.ai_commissions > 0);
    assert!(validator_reward.network_dividend > 0);

    let calculated_total = validator_reward.round_emission
        + validator_reward.transaction_fees
        + validator_reward.ai_commissions
        + validator_reward.network_dividend;

    assert_eq!(validator_reward.total_reward, calculated_total);
}
