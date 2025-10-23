//! DAG-Fair Emission Integration Test
//!
//! Simulates 1000 rounds of consensus with emission, fee distribution,
//! and validates total supply constraints, fairness, and halving behavior.

use ippan_consensus::{
    distribute_round, emission_for_round_capped, finalize_round, Participation, ParticipationSet,
    Role, MICRO_PER_IPN,
};
use ippan_governance::parameters::EconomicsParams;
use ippan_storage::{ChainState, MemoryStorage, Storage};
use ippan_treasury::reward_pool::{RewardSink, ValidatorId};
use parking_lot::RwLock;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::sync::Arc;

/// Create a test validator ID
fn test_validator(id: u8) -> ValidatorId {
    let mut vid = [0u8; 32];
    vid[0] = id;
    vid
}

/// Generate random participants for a round
fn generate_participants(
    round: u64,
    validator_count: usize,
    rng: &mut StdRng,
) -> ParticipationSet {
    let mut participants = Vec::new();

    // Select one proposer
    let proposer_id = rng.gen_range(0..validator_count) as u8;
    participants.push(Participation {
        validator_id: test_validator(proposer_id),
        role: Role::Proposer,
        weight: rng.gen_range(50..200),
    });

    // Select 3-5 verifiers (excluding proposer)
    let verifier_count = rng.gen_range(3..=5);
    for _ in 0..verifier_count {
        let mut verifier_id = rng.gen_range(0..validator_count) as u8;
        while verifier_id == proposer_id {
            verifier_id = rng.gen_range(0..validator_count) as u8;
        }
        participants.push(Participation {
            validator_id: test_validator(verifier_id),
            role: Role::Verifier,
            weight: rng.gen_range(50..200),
        });
    }

    participants
}

#[test]
fn test_emission_integration_1000_rounds() {
    let mut rng = StdRng::seed_from_u64(42);
    let economics = EconomicsParams::default();
    let storage = Arc::new(MemoryStorage::new());
    let reward_sink = Arc::new(RwLock::new(RewardSink::new()));

    let validator_count = 10;
    let rounds = 1_000;

    let mut total_emitted = 0u128;
    let mut total_fees = 0u128;

    for round in 1..=rounds {
        let participants = generate_participants(round, validator_count, &mut rng);
        let fees = rng.gen_range(0..5_000) as u128;

        finalize_round(
            round,
            &storage,
            participants,
            fees,
            &economics,
            &reward_sink,
        )
        .unwrap();

        // Track totals
        let chain_state = storage.get_chain_state().unwrap();
        total_emitted = chain_state.total_issued_micro();
        total_fees += fees;
    }

    // Validate results
    let chain_state = storage.get_chain_state().unwrap();
    
    println!("✅ Integration Test Results:");
    println!("  Rounds simulated: {}", rounds);
    println!("  Total emitted: {} μIPN ({:.6} IPN)", total_emitted, total_emitted as f64 / MICRO_PER_IPN as f64);
    println!("  Total fees collected: {} μIPN", total_fees);
    println!("  Supply cap: {} μIPN", economics.supply_cap_micro);
    println!("  Chain state round: {}", chain_state.last_updated_round);

    // Assertions
    assert_eq!(chain_state.last_updated_round, rounds);
    assert!(total_emitted > 0, "Emission should be positive");
    assert!(
        total_emitted <= economics.supply_cap_micro,
        "Emission should not exceed cap"
    );

    // Verify rewards were distributed
    let all_totals = reward_sink.read().get_all_validator_totals();
    assert!(!all_totals.is_empty(), "Should have distributed rewards");

    let total_distributed: u128 = all_totals.values().sum();
    println!("  Total distributed to validators: {} μIPN", total_distributed);
    assert!(total_distributed > 0, "Should have distributed rewards");
}

#[test]
fn test_emission_halving_behavior() {
    let mut economics = EconomicsParams::default();
    economics.halving_interval_rounds = 100; // Halving every 100 rounds for testing
    economics.initial_round_reward_micro = 10_000;

    let storage = Arc::new(MemoryStorage::new());
    let reward_sink = Arc::new(RwLock::new(RewardSink::new()));

    // Test first epoch (rounds 1-99)
    let emission_1 = emission_for_round_capped(1, 0, &economics).unwrap();
    assert_eq!(emission_1, 10_000, "First epoch should have full reward");

    // Test at halving boundary
    let emission_100 = emission_for_round_capped(100, 0, &economics).unwrap();
    assert_eq!(emission_100, 5_000, "Second epoch should have half reward");

    // Test second epoch
    let emission_150 = emission_for_round_capped(150, 0, &economics).unwrap();
    assert_eq!(emission_150, 5_000, "Second epoch should maintain half reward");

    // Test third epoch
    let emission_200 = emission_for_round_capped(200, 0, &economics).unwrap();
    assert_eq!(emission_200, 2_500, "Third epoch should have quarter reward");

    println!("✅ Halving behavior verified:");
    println!("  Round 1: {} μIPN", emission_1);
    println!("  Round 100: {} μIPN", emission_100);
    println!("  Round 150: {} μIPN", emission_150);
    println!("  Round 200: {} μIPN", emission_200);
}

#[test]
fn test_supply_cap_enforcement() {
    let mut economics = EconomicsParams::default();
    economics.supply_cap_micro = 50_000; // Low cap for testing
    economics.initial_round_reward_micro = 10_000;

    let storage = Arc::new(MemoryStorage::new());
    let reward_sink = Arc::new(RwLock::new(RewardSink::new()));

    let mut participants = vec![Participation {
        validator_id: test_validator(1),
        role: Role::Proposer,
        weight: 100,
    }];

    // Emit until cap is reached
    for round in 1..=10 {
        finalize_round(round, &storage, participants.clone(), 0, &economics, &reward_sink).unwrap();
    }

    let chain_state = storage.get_chain_state().unwrap();
    
    println!("✅ Supply cap enforcement:");
    println!("  Supply cap: {} μIPN", economics.supply_cap_micro);
    println!("  Total issued: {} μIPN", chain_state.total_issued_micro());
    
    assert!(
        chain_state.total_issued_micro() <= economics.supply_cap_micro,
        "Should not exceed supply cap"
    );
}

#[test]
fn test_fee_cap_enforcement() {
    let mut economics = EconomicsParams::default();
    economics.fee_cap_numer = 1;
    economics.fee_cap_denom = 10; // 10% cap
    economics.initial_round_reward_micro = 10_000;

    let participants = vec![
        Participation {
            validator_id: test_validator(1),
            role: Role::Proposer,
            weight: 100,
        },
        Participation {
            validator_id: test_validator(2),
            role: Role::Verifier,
            weight: 100,
        },
    ];

    // Try to distribute with excessive fees
    let excessive_fees = 50_000u128; // Much higher than 10% of emission
    let (payouts, emission, capped_fees) =
        distribute_round(10_000, excessive_fees, &participants, &economics).unwrap();

    println!("✅ Fee cap enforcement:");
    println!("  Emission: {} μIPN", emission);
    println!("  Attempted fees: {} μIPN", excessive_fees);
    println!("  Capped fees: {} μIPN", capped_fees);
    println!("  Fee cap: {}%", (economics.fee_cap_numer * 100) / economics.fee_cap_denom);

    // Fees should be capped to 10% of emission = 1_000
    assert_eq!(capped_fees, 1_000, "Fees should be capped to 10% of emission");

    let total_distributed: u128 = payouts.values().sum();
    assert_eq!(
        total_distributed, 11_000,
        "Total should be emission + capped fees"
    );
}

#[test]
fn test_reward_distribution_fairness() {
    let economics = EconomicsParams::default();
    let storage = Arc::new(MemoryStorage::new());
    let reward_sink = Arc::new(RwLock::new(RewardSink::new()));

    // Run 100 rounds with fixed participants
    let participants = vec![
        Participation {
            validator_id: test_validator(1),
            role: Role::Proposer,
            weight: 100,
        },
        Participation {
            validator_id: test_validator(2),
            role: Role::Verifier,
            weight: 100,
        },
        Participation {
            validator_id: test_validator(3),
            role: Role::Verifier,
            weight: 100,
        },
    ];

    for round in 1..=100 {
        finalize_round(
            round,
            &storage,
            participants.clone(),
            1_000,
            &economics,
            &reward_sink,
        )
        .unwrap();
    }

    let all_totals = reward_sink.read().get_all_validator_totals();
    
    println!("✅ Reward distribution fairness:");
    for (vid, total) in &all_totals {
        println!("  Validator {:?}: {} μIPN", vid[0], total);
    }

    // Proposer should get ~20% (2000 bps)
    // Each verifier should get ~40% (8000 bps / 2 verifiers)
    let proposer_total = all_totals.get(&test_validator(1)).unwrap();
    let verifier1_total = all_totals.get(&test_validator(2)).unwrap();
    let verifier2_total = all_totals.get(&test_validator(3)).unwrap();

    // Verifiers should have approximately equal shares
    let diff = (*verifier1_total as i128 - *verifier2_total as i128).abs();
    assert!(
        diff < 100,
        "Verifiers with equal weight should receive similar rewards"
    );

    // Proposer should have less than verifiers (20% vs 40% each)
    assert!(
        proposer_total < verifier1_total,
        "Proposer should receive less than individual verifiers"
    );
}

#[test]
fn test_zero_emission_after_supply_cap() {
    let mut economics = EconomicsParams::default();
    economics.supply_cap_micro = 10_000;
    economics.initial_round_reward_micro = 5_000;

    let storage = Arc::new(MemoryStorage::new());
    let reward_sink = Arc::new(RwLock::new(RewardSink::new()));

    let participants = vec![Participation {
        validator_id: test_validator(1),
        role: Role::Proposer,
        weight: 100,
    }];

    // Round 1: Should emit 5_000
    finalize_round(1, &storage, participants.clone(), 0, &economics, &reward_sink).unwrap();
    let state1 = storage.get_chain_state().unwrap();
    assert_eq!(state1.total_issued_micro(), 5_000);

    // Round 2: Should emit 5_000 (reaching cap)
    finalize_round(2, &storage, participants.clone(), 0, &economics, &reward_sink).unwrap();
    let state2 = storage.get_chain_state().unwrap();
    assert_eq!(state2.total_issued_micro(), 10_000);

    // Round 3: Should emit 0 (cap reached)
    finalize_round(3, &storage, participants.clone(), 0, &economics, &reward_sink).unwrap();
    let state3 = storage.get_chain_state().unwrap();
    assert_eq!(state3.total_issued_micro(), 10_000, "Should not exceed cap");

    println!("✅ Zero emission after cap:");
    println!("  Round 1: {} μIPN", state1.total_issued_micro());
    println!("  Round 2: {} μIPN", state2.total_issued_micro());
    println!("  Round 3: {} μIPN (capped)", state3.total_issued_micro());
}
