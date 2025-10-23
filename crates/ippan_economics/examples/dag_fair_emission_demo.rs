//! Demonstration of IPPAN's DAG-Fair Emission Framework
//!
//! This example shows how the round-based emission system works with
//! multiple validators participating in parallel block creation.

use ippan_economics::prelude::*;
use ippan_economics::{ValidatorParticipation, ValidatorRole};
use ippan_economics::governance::Vote;
use rust_decimal::Decimal;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    // Initialize logging (simplified for demo)
    println!("Initializing IPPAN Economics Demo...");

    println!("🪙 IPPAN DAG-Fair Emission Framework Demo");
    println!("==========================================\n");

    // 1. Create emission engine with default parameters
    let mut emission_engine = EmissionEngine::new();
    println!("📊 Initial Emission Parameters:");
    println!("   • Initial round reward: {} micro-IPN", emission_engine.params().initial_round_reward);
    println!("   • Halving interval: {} rounds", emission_engine.params().halving_interval);
    println!("   • Total supply cap: {} micro-IPN", emission_engine.params().total_supply_cap);
    println!("   • Fee cap fraction: {}%", emission_engine.params().fee_cap_fraction * Decimal::from(100));
    println!();

    // 2. Demonstrate emission calculation for different rounds
    println!("🔢 Emission Calculation Examples:");
    for round in [1, 100, 1000, 630_000_000, 1_260_000_000] {
        let reward = emission_engine.calculate_round_reward(round)?;
        let halving_epoch = round / emission_engine.params().halving_interval;
        println!("   • Round {}: {} micro-IPN (halving epoch {})", round, reward, halving_epoch);
    }
    println!();

    // 3. Simulate a round with multiple validators
    println!("🎯 Round Reward Distribution Simulation:");
    let round_index = 1000;
    let round_reward = emission_engine.calculate_round_reward(round_index)?;
    
    // Create validator participations
    let participations = vec![
        ValidatorParticipation {
            validator_id: "validator_1".to_string(),
            role: ValidatorRole::Proposer,
            blocks_contributed: 15,
            uptime_score: Decimal::new(95, 2), // 0.95
        },
        ValidatorParticipation {
            validator_id: "validator_2".to_string(),
            role: ValidatorRole::Verifier,
            blocks_contributed: 12,
            uptime_score: Decimal::new(98, 2), // 0.98
        },
        ValidatorParticipation {
            validator_id: "validator_3".to_string(),
            role: ValidatorRole::Verifier,
            blocks_contributed: 8,
            uptime_score: Decimal::new(92, 2), // 0.92
        },
        ValidatorParticipation {
            validator_id: "validator_4".to_string(),
            role: ValidatorRole::Observer,
            blocks_contributed: 0,
            uptime_score: Decimal::new(100, 2), // 1.0
        },
    ];

    // Distribute rewards
    let round_rewards = RoundRewards::new(emission_engine.params().clone());
    let distribution = round_rewards.distribute_round_rewards(
        round_index,
        round_reward,
        participations,
        500, // 500 micro-IPN in fees
    )?;

    println!("   • Round {} total reward: {} micro-IPN", distribution.round_index, distribution.total_reward);
    println!("   • Blocks in round: {}", distribution.blocks_in_round);
    println!("   • Fees collected: {} micro-IPN", distribution.fees_collected);
    println!("   • Excess burned: {} micro-IPN", distribution.excess_burned);
    println!();

    println!("   📋 Validator Rewards:");
    for (validator_id, reward) in &distribution.validator_rewards {
        println!("   • {}: {} micro-IPN total", validator_id, reward.total_reward);
        println!("     - Round emission: {} micro-IPN", reward.round_emission);
        println!("     - Transaction fees: {} micro-IPN", reward.transaction_fees);
        println!("     - AI commissions: {} micro-IPN", reward.ai_commissions);
        println!("     - Network dividend: {} micro-IPN", reward.network_dividend);
        println!("     - Weight factor: {}", reward.weight_factor);
        println!();
    }

    // 4. Demonstrate supply tracking
    println!("📈 Supply Tracking Simulation:");
    let mut supply_tracker = SupplyTracker::new(emission_engine.params().total_supply_cap);
    
    // Simulate several rounds of emission
    for round in 1..=10 {
        let round_reward = emission_engine.calculate_round_reward(round)?;
        supply_tracker.record_emission(round, round_reward)?;
        
        if round % 5 == 0 {
            let supply_info = supply_tracker.get_supply_info();
            println!("   • After round {}: {} micro-IPN emitted ({:.2}% of cap)",
                round, supply_info.total_supply, supply_info.emission_percentage);
        }
    }
    println!();

    // 5. Demonstrate governance
    println!("🗳️  Governance Simulation:");
    let mut governance = GovernanceParams::new(emission_engine.params().clone());
    
    // Set up validators with voting power
    governance.set_validator_power("validator_1".to_string(), 100);
    governance.set_validator_power("validator_2".to_string(), 80);
    governance.set_validator_power("validator_3".to_string(), 60);
    
    // Create a proposal to increase initial round reward
    let mut new_params = emission_engine.params().clone();
    new_params.initial_round_reward = 15_000; // Increase from 10,000 to 15,000
    
    let proposal_id = governance.create_proposal(
        "validator_1".to_string(),
        new_params,
        100, // 10 seconds voting period
        "Increase initial round reward to improve validator incentives".to_string(),
        1000,
    )?;
    
    println!("   • Created proposal {} to increase initial round reward", proposal_id);
    
    // Simulate voting
    governance.vote_on_proposal(proposal_id, "validator_1".to_string(), Vote::Approve, 1001)?;
    governance.vote_on_proposal(proposal_id, "validator_2".to_string(), Vote::Approve, 1002)?;
    governance.vote_on_proposal(proposal_id, "validator_3".to_string(), Vote::Reject, 1003)?;
    
    println!("   • Validators voted on proposal");
    
    // Process results
    let approved = governance.process_proposal_results(proposal_id)?;
    if approved {
        governance.execute_proposal(proposal_id)?;
        println!("   ✅ Proposal approved and executed!");
        println!("   • New initial round reward: {} micro-IPN", 
            governance.emission_params().initial_round_reward);
    } else {
        println!("   ❌ Proposal rejected");
    }
    println!();

    // 6. Generate emission curve
    println!("📊 Emission Curve (first 1000 rounds):");
    let curve = emission_engine.generate_emission_curve(1, 1000, 100)?;
    
    for point in curve.iter().take(5) {
        println!("   • Round {}: {} micro-IPN/round, {} micro-IPN annual",
            point.round, point.reward_per_round, point.annual_issuance);
    }
    println!("   ... (showing first 5 points)");
    println!();

    // 7. Supply audit
    println!("🔍 Supply Audit:");
    let audit_result = supply_tracker.audit_supply();
    println!("   • Supply healthy: {}", audit_result.is_healthy);
    println!("   • Total emissions: {} micro-IPN", audit_result.total_emissions);
    println!("   • Total burns: {} micro-IPN", audit_result.total_burns);
    println!("   • Net supply: {} micro-IPN", audit_result.net_supply);
    
    if !audit_result.issues.is_empty() {
        println!("   ⚠️  Issues:");
        for issue in &audit_result.issues {
            println!("     - {}", issue);
        }
    }
    
    if !audit_result.warnings.is_empty() {
        println!("   ⚠️  Warnings:");
        for warning in &audit_result.warnings {
            println!("     - {}", warning);
        }
    }
    println!();

    println!("✅ DAG-Fair Emission Framework Demo Complete!");
    println!("\nKey Features Demonstrated:");
    println!("• Round-based emission calculation with halving");
    println!("• Proportional reward distribution based on participation");
    println!("• Supply cap enforcement and integrity verification");
    println!("• Governance-controlled parameter updates");
    println!("• Comprehensive audit and monitoring capabilities");

    Ok(())
}