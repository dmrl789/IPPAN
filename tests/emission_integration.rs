//! Comprehensive integration tests for the DAG-Fair Emission system

use ippan_consensus::{RoundExecutor, create_participation_set};
use ippan_economics::{EconomicsParams, EconomicsParameterManager, distribute_round, emission_for_round_capped};
use ippan_governance::ParameterManager;
use ippan_treasury::{MockAccountLedger, RewardSink, FeeCollector};
use ippan_types::{ChainState, MicroIPN, RoundId};
use std::collections::HashMap;

/// Test the complete emission flow with 1000 rounds
#[test]
fn test_emission_integration_1000_rounds() {
    let params = EconomicsParams {
        initial_round_reward_micro: 1_000_000, // 0.01 IPN per round
        halving_interval_rounds: 100, // Halve every 100 rounds for testing
        max_supply_micro: 100_000_000, // 1 IPN total cap for testing
        fee_cap_numer: 1,
        fee_cap_denom: 10, // 10% fee cap
        proposer_weight_bps: 2000, // 20% proposer
        verifier_weight_bps: 8000, // 80% verifier
        fee_recycling_bps: 10000, // 100% fee recycling
    };

    let account_ledger = Box::new(MockAccountLedger::new());
    let mut executor = RoundExecutor::new(params.clone(), account_ledger);
    let mut chain_state = ChainState::new();

    let mut total_emission = 0u128;
    let mut total_fees = 0u128;
    let mut round_results = Vec::new();

    // Simulate 1000 rounds with random validators
    for round in 1..=1000 {
        // Create random participants for this round
        let participants = create_test_participants(round);
        let fees = generate_random_fees(round);
        
        let result = executor.execute_round(round, &mut chain_state, participants, fees).unwrap();
        
        total_emission += result.emission_micro;
        total_fees += result.fees_collected_micro;
        round_results.push(result);
        
        // Verify supply cap is not exceeded
        assert!(chain_state.total_issued_micro() <= params.max_supply_micro);
        
        // Log every 100 rounds
        if round % 100 == 0 {
            println!(
                "Round {}: {} micro-IPN emitted, {} total issued, {} remaining cap",
                round,
                result.emission_micro,
                chain_state.total_issued_micro(),
                chain_state.remaining_cap(params.max_supply_micro)
            );
        }
    }

    // Verify final state
    assert_eq!(chain_state.current_round(), 1000);
    assert!(total_emission > 0);
    assert!(total_fees > 0);
    
    // Verify supply cap enforcement
    assert!(chain_state.total_issued_micro() <= params.max_supply_micro);
    
    // Verify halving behavior
    let early_rounds: MicroIPN = round_results[0..100].iter().map(|r| r.emission_micro).sum();
    let late_rounds: MicroIPN = round_results[900..1000].iter().map(|r| r.emission_micro).sum();
    assert!(late_rounds < early_rounds, "Halving should reduce emission over time");
    
    println!(
        "Integration test completed: {} total emission, {} total fees, {} final supply",
        total_emission,
        total_fees,
        chain_state.total_issued_micro()
    );
}

/// Test governance parameter updates
#[test]
fn test_governance_economics_update() {
    let mut param_manager = ParameterManager::new();
    let initial_params = param_manager.get_economics_params().clone();
    
    // Create a parameter change proposal
    let proposal = ippan_governance::parameters::create_parameter_proposal(
        "economics.initial_round_reward_micro",
        serde_json::Value::Number(serde_json::Number::from(20_000_000)), // Double the emission
        serde_json::Value::Number(serde_json::Number::from(10_000_000)),
        "Double emission rate for testing".to_string(),
        [1u8; 32],
        7 * 24 * 3600, // 7 days
    );
    
    // Submit and execute the proposal
    param_manager.submit_parameter_change(proposal.clone()).unwrap();
    param_manager.execute_parameter_change(&proposal.proposal_id).unwrap();
    
    // Verify the parameter was updated
    let updated_params = param_manager.get_economics_params();
    assert_eq!(updated_params.initial_round_reward_micro, 20_000_000);
    assert_ne!(updated_params.initial_round_reward_micro, initial_params.initial_round_reward_micro);
}

/// Test fee capping and recycling
#[test]
fn test_fee_capping_and_recycling() {
    let params = EconomicsParams {
        initial_round_reward_micro: 1_000_000,
        halving_interval_rounds: 1000,
        max_supply_micro: 10_000_000,
        fee_cap_numer: 1,
        fee_cap_denom: 10, // 10% fee cap
        proposer_weight_bps: 2000,
        verifier_weight_bps: 8000,
        fee_recycling_bps: 10000, // 100% recycling
    };
    
    let participants = create_test_participants(1);
    let high_fees = 500_000; // 50% of emission (should be capped)
    
    let (payouts, emission, fees_capped) = distribute_round(
        1_000_000, // emission
        high_fees, // fees
        &participants,
        &params,
    ).unwrap();
    
    // Verify fee cap was applied
    assert!(fees_capped > 0, "Fees should have been capped");
    
    // Verify total rewards include recycled fees
    let total_rewards: MicroIPN = payouts.values().sum();
    assert!(total_rewards > 1_000_000, "Total rewards should include recycled fees");
    
    println!(
        "Fee capping test: {} emission, {} fees, {} capped, {} total rewards",
        1_000_000,
        high_fees,
        fees_capped,
        total_rewards
    );
}

/// Test supply cap enforcement
#[test]
fn test_supply_cap_enforcement() {
    let params = EconomicsParams {
        initial_round_reward_micro: 1_000_000,
        halving_interval_rounds: 1000,
        max_supply_micro: 5_000_000, // Very low cap for testing
        fee_cap_numer: 1,
        fee_cap_denom: 10,
        proposer_weight_bps: 2000,
        verifier_weight_bps: 8000,
        fee_recycling_bps: 10000,
    };
    
    let mut chain_state = ChainState::with_initial(4_000_000, 0, 0); // Start near cap
    
    // First round should emit normally
    let emission1 = emission_for_round_capped(1, chain_state.total_issued_micro(), &params).unwrap();
    assert_eq!(emission1, 1_000_000);
    
    chain_state.add_issued_micro(emission1);
    
    // Second round should be capped
    let emission2 = emission_for_round_capped(2, chain_state.total_issued_micro(), &params).unwrap();
    assert_eq!(emission2, 0); // Should be capped to 0
    
    // Verify we're at the cap
    assert_eq!(chain_state.total_issued_micro(), 5_000_000);
    assert_eq!(chain_state.remaining_cap(params.max_supply_micro), 0);
}

/// Test reward distribution fairness
#[test]
fn test_reward_distribution_fairness() {
    let params = EconomicsParams::default();
    
    // Create participants with different stakes and contributions
    let participants = vec![
        ippan_economics::Participation {
            validator_id: [1u8; 32],
            role: ippan_economics::Role::Proposer,
            blocks_proposed: 1,
            blocks_verified: 0,
            reputation_score: 1.0,
            stake_weight: 1000,
        },
        ippan_economics::Participation {
            validator_id: [2u8; 32],
            role: ippan_economics::Role::Verifier,
            blocks_proposed: 0,
            blocks_verified: 2,
            reputation_score: 1.2,
            stake_weight: 2000,
        },
        ippan_economics::Participation {
            validator_id: [3u8; 32],
            role: ippan_economics::Role::Both,
            blocks_proposed: 1,
            blocks_verified: 1,
            reputation_score: 1.5,
            stake_weight: 1500,
        },
    ];
    
    let (payouts, emission, _) = distribute_round(1_000_000, 0, &participants, &params).unwrap();
    
    // Verify all participants received rewards
    assert_eq!(payouts.len(), 3);
    assert!(payouts.contains_key(&[1u8; 32]));
    assert!(payouts.contains_key(&[2u8; 32]));
    assert!(payouts.contains_key(&[3u8; 32]));
    
    // Verify total rewards equal emission
    let total_payouts: MicroIPN = payouts.values().sum();
    assert_eq!(total_payouts, emission);
    
    // Verify higher stake/contribution gets more rewards
    let validator2_reward = payouts.get(&[2u8; 32]).unwrap();
    let validator1_reward = payouts.get(&[1u8; 32]).unwrap();
    assert!(validator2_reward > validator1_reward, "Higher stake should get more rewards");
    
    println!(
        "Fairness test: Validator 1: {}, Validator 2: {}, Validator 3: {}",
        validator1_reward,
        validator2_reward,
        payouts.get(&[3u8; 32]).unwrap()
    );
}

/// Test treasury operations
#[test]
fn test_treasury_operations() {
    let mut sink = RewardSink::new();
    let mut payouts1 = HashMap::new();
    payouts1.insert([1u8; 32], 1000);
    payouts1.insert([2u8; 32], 2000);
    
    let mut payouts2 = HashMap::new();
    payouts2.insert([1u8; 32], 500);
    payouts2.insert([3u8; 32], 1500);
    
    // Credit payouts
    sink.credit_round_payouts(1, &payouts1).unwrap();
    sink.credit_round_payouts(2, &payouts2).unwrap();
    
    // Verify totals
    assert_eq!(sink.get_total_distributed(), 5000);
    assert_eq!(sink.validator_total(&[1u8; 32]), 1500);
    assert_eq!(sink.validator_total(&[2u8; 32]), 2000);
    assert_eq!(sink.validator_total(&[3u8; 32]), 1500);
    
    // Test statistics
    let stats = sink.get_statistics();
    assert_eq!(stats.total_rounds, 2);
    assert_eq!(stats.total_validators, 3);
    assert_eq!(stats.total_distributed_micro, 5000);
    assert_eq!(stats.average_per_round, 2500);
}

/// Test fee collection
#[test]
fn test_fee_collection() {
    let mut collector = FeeCollector::new();
    
    collector.collect_round_fees(1, 1000).unwrap();
    collector.collect_round_fees(2, 2000).unwrap();
    collector.collect_round_fees(3, 500).unwrap();
    
    assert_eq!(collector.get_total_collected(), 3500);
    assert_eq!(collector.get_round_fees(1), 1000);
    assert_eq!(collector.get_round_fees(2), 2000);
    assert_eq!(collector.get_round_fees(3), 500);
    
    let stats = collector.get_statistics();
    assert_eq!(stats.total_rounds, 3);
    assert_eq!(stats.average_per_round, 1166); // 3500 / 3
    assert_eq!(stats.highest_round, 2000);
    assert_eq!(stats.lowest_round, 500);
}

/// Helper function to create test participants
fn create_test_participants(round: RoundId) -> Vec<ippan_economics::Participation> {
    let mut participants = Vec::new();
    
    // Create 3-5 random validators per round
    let num_validators = 3 + (round % 3) as usize;
    
    for i in 0..num_validators {
        let validator_id = [i as u8; 32];
        let stake = 1000 + (i as u128 * 500);
        let blocks_proposed = if i == 0 { 1 } else { 0 }; // First validator is proposer
        let blocks_verified = (i + 1) as u32;
        let reputation = 0.8 + (i as f64 * 0.1);
        
        participants.push(ippan_economics::Participation {
            validator_id,
            role: if i == 0 {
                ippan_economics::Role::Proposer
            } else {
                ippan_economics::Role::Verifier
            },
            blocks_proposed,
            blocks_verified,
            reputation_score: reputation,
            stake_weight: stake,
        });
    }
    
    participants
}

/// Helper function to generate random fees
fn generate_random_fees(round: RoundId) -> MicroIPN {
    // Generate fees between 0 and 10% of base emission
    let base_emission = 1_000_000;
    let fee_percentage = (round % 11) as u128; // 0-10%
    (base_emission * fee_percentage) / 100
}