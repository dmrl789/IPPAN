//! Integration tests for IPPAN DAG-Fair Emission Framework

use ippan_economics::prelude::*;
use ippan_economics::{ValidatorParticipation, ValidatorRole};
use ippan_economics::governance::Vote;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[test]
fn test_complete_emission_cycle() {
    // Test a complete emission cycle from genesis to halving
    let mut emission_engine = EmissionEngine::new();
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
    let emission_engine = EmissionEngine::new();
    let halving_interval = emission_engine.params().halving_interval;
    
    // Test rewards before and after halving
    let before_halving = emission_engine.calculate_round_reward(halving_interval - 1).unwrap();
    let after_halving = emission_engine.calculate_round_reward(halving_interval).unwrap();
    
    assert_eq!(after_halving, before_halving / 2);
    
    // Test second halving
    let second_halving = emission_engine.calculate_round_reward(halving_interval * 2).unwrap();
    assert_eq!(second_halving, before_halving / 4);
}

#[test]
fn test_reward_distribution_fairness() {
    let params = EmissionParams::default();
    let round_rewards = RoundRewards::new(params);
    
    // Create validators with different participation levels
    let participations = vec![
        ValidatorParticipation {
            validator_id: "high_participant".to_string(),
            role: ValidatorRole::Proposer,
            blocks_contributed: 20,
            uptime_score: Decimal::new(100, 2), // 1.0
        },
        ValidatorParticipation {
            validator_id: "medium_participant".to_string(),
            role: ValidatorRole::Verifier,
            blocks_contributed: 10,
            uptime_score: Decimal::new(90, 2), // 0.9
        },
        ValidatorParticipation {
            validator_id: "low_participant".to_string(),
            role: ValidatorRole::Verifier,
            blocks_contributed: 5,
            uptime_score: Decimal::new(80, 2), // 0.8
        },
        ValidatorParticipation {
            validator_id: "observer".to_string(),
            role: ValidatorRole::Observer,
            blocks_contributed: 0,
            uptime_score: Decimal::new(100, 2), // 1.0
        },
    ];
    
    let distribution = round_rewards.distribute_round_rewards(
        1,
        10_000,
        participations,
        1_000,
    ).unwrap();
    
    // High participant should get most rewards
    let high_reward = distribution.validator_rewards.get("high_participant").unwrap();
    let medium_reward = distribution.validator_rewards.get("medium_participant").unwrap();
    let low_reward = distribution.validator_rewards.get("low_participant").unwrap();
    let observer_reward = distribution.validator_rewards.get("observer").unwrap();
    
    assert!(high_reward.total_reward > medium_reward.total_reward);
    assert!(medium_reward.total_reward > low_reward.total_reward);
    assert_eq!(observer_reward.total_reward, 0); // Observer gets nothing
}

#[test]
fn test_supply_cap_enforcement() {
    let mut supply_tracker = SupplyTracker::new(100_000); // Small cap for testing
    
    // Try to emit more than the cap
    supply_tracker.record_emission(1, 150_000).unwrap();
    
    let supply_info = supply_tracker.get_supply_info();
    assert_eq!(supply_info.total_supply, 100_000); // Should be capped
    assert_eq!(supply_info.remaining_supply, 0);
}

#[test]
fn test_governance_proposal_lifecycle() {
    let mut governance = GovernanceParams::new(EmissionParams::default());
    
    // Set up validators
    governance.set_validator_power("validator1".to_string(), 100);
    governance.set_validator_power("validator2".to_string(), 80);
    governance.set_validator_power("validator3".to_string(), 60);
    
    // Create proposal
    let mut new_params = EmissionParams::default();
    new_params.initial_round_reward = 20_000;
    
    let proposal_id = governance.create_proposal(
        "validator1".to_string(),
        new_params,
        100,
        "Test proposal".to_string(),
        1000,
    ).unwrap();
    
    // Vote on proposal
    governance.vote_on_proposal(proposal_id, "validator1".to_string(), Vote::Approve, 1001).unwrap();
    governance.vote_on_proposal(proposal_id, "validator2".to_string(), Vote::Approve, 1002).unwrap();
    governance.vote_on_proposal(proposal_id, "validator3".to_string(), Vote::Reject, 1003).unwrap();
    
    // Process results
    let approved = governance.process_proposal_results(proposal_id).unwrap();
    assert!(approved); // Should be approved with 180/240 votes (75%)
    
    // Execute proposal
    governance.execute_proposal(proposal_id).unwrap();
    assert_eq!(governance.emission_params().initial_round_reward, 20_000);
}

#[test]
fn test_fee_cap_enforcement() {
    let mut params = EmissionParams::default();
    params.fee_cap_fraction = Decimal::new(1, 1); // 10%
    
    let round_rewards = RoundRewards::new(params);
    
    // Test fee cap
    let capped_fees = round_rewards.apply_fee_cap(5_000, 10_000);
    assert_eq!(capped_fees, 1_000); // 10% of 10,000
    
    // Test when fees are already under cap
    let capped_fees_low = round_rewards.apply_fee_cap(500, 10_000);
    assert_eq!(capped_fees_low, 500); // Should remain unchanged
}

#[test]
fn test_emission_curve_generation() {
    let emission_engine = EmissionEngine::new();
    let curve = emission_engine.generate_emission_curve(1, 100, 10).unwrap();
    
    assert_eq!(curve.len(), 10);
    assert_eq!(curve[0].round, 1);
    assert_eq!(curve[0].reward_per_round, 10_000);
    
    // Check that rewards decrease over time (due to halving)
    for i in 1..curve.len() {
        assert!(curve[i].reward_per_round <= curve[i-1].reward_per_round);
    }
}

#[test]
fn test_supply_audit() {
    let mut supply_tracker = SupplyTracker::new(1_000_000);
    
    // Add some emissions and burns
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
    
    // Test that rewards never go below 1 micro-IPN
    for round in 1..=1_000_000 {
        let reward = emission_engine.calculate_round_reward(round).unwrap();
        assert!(reward >= 1, "Reward at round {} was {}", round, reward);
    }
}

#[test]
fn test_overflow_protection() {
    let mut emission_engine = EmissionEngine::new();
    
    // Test with very large round numbers
    let large_round = u64::MAX;
    let result = emission_engine.calculate_round_reward(large_round);
    
    // Should either succeed or fail gracefully with overflow error
    match result {
        Ok(reward) => assert!(reward >= 1),
        Err(EmissionError::MathematicalOverflow) => {}, // Expected for very large rounds
        Err(e) => panic!("Unexpected error: {:?}", e),
    }
}

#[test]
fn test_round_reward_distribution_components() {
    let params = EmissionParams::default();
    let round_rewards = RoundRewards::new(params);
    
    let participations = vec![
        ValidatorParticipation {
            validator_id: "validator1".to_string(),
            role: ValidatorRole::Proposer,
            blocks_contributed: 10,
            uptime_score: Decimal::ONE,
        },
    ];
    
    let distribution = round_rewards.distribute_round_rewards(
        1,
        10_000,
        participations,
        1_000,
    ).unwrap();
    
    let validator_reward = distribution.validator_rewards.get("validator1").unwrap();
    
    // Check that all components are present
    assert!(validator_reward.round_emission > 0);
    assert!(validator_reward.transaction_fees > 0);
    assert!(validator_reward.ai_commissions > 0);
    assert!(validator_reward.network_dividend > 0);
    
    // Check that total is sum of components
    let calculated_total = validator_reward.round_emission
        + validator_reward.transaction_fees
        + validator_reward.ai_commissions
        + validator_reward.network_dividend;
    
    assert_eq!(validator_reward.total_reward, calculated_total);
}