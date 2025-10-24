//! DAG-Fair Emission System Demo
//!
//! This example demonstrates the complete integration of the DAG-Fair Emission system
//! into the IPPAN blockchain, showing how emission, distribution, and governance work together.

use ippan_consensus::{RoundExecutor, create_participation_set};
use ippan_economics_core::{EconomicsParams, EconomicsParameterManager};
use ippan_governance::ParameterManager;
use ippan_treasury::{InMemoryAccountLedger, RewardSink, FeeCollector};
use ippan_types::{ChainState, MicroIPN, RoundId};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 IPPAN DAG-Fair Emission System Demo");
    println!("=====================================\n");

    // 1. Initialize the system with default parameters
    let economics_params = EconomicsParams::default();
    let account_ledger = Box::new(InMemoryAccountLedger::new());
    let mut executor = RoundExecutor::new(economics_params.clone(), account_ledger);
    let mut chain_state = ChainState::new();

    println!("📊 Initial Economics Parameters:");
    println!("  • Initial round reward: {} micro-IPN ({:.6} IPN)", 
        economics_params.initial_round_reward_micro,
        economics_params.initial_round_reward_micro as f64 / 100_000_000.0
    );
    println!("  • Halving interval: {} rounds", economics_params.halving_interval_rounds);
    println!("  • Max supply: {} micro-IPN ({:.6} IPN)", 
        economics_params.max_supply_micro,
        economics_params.max_supply_micro as f64 / 100_000_000.0
    );
    println!("  • Fee cap: {}/{} ({:.1}%)", 
        economics_params.fee_cap_numer,
        economics_params.fee_cap_denom,
        (economics_params.fee_cap_numer as f64 / economics_params.fee_cap_denom as f64) * 100.0
    );
    println!("  • Proposer weight: {} bps ({:.1}%)", 
        economics_params.proposer_weight_bps,
        economics_params.proposer_weight_bps as f64 / 100.0
    );
    println!("  • Verifier weight: {} bps ({:.1}%)", 
        economics_params.verifier_weight_bps,
        economics_params.verifier_weight_bps as f64 / 100.0
    );
    println!();

    // 2. Simulate 10 rounds of activity
    println!("🔄 Simulating 10 rounds of blockchain activity...\n");

    for round in 1..=10 {
        // Create participants for this round
        let participants = create_demo_participants(round);
        let fees = generate_demo_fees(round);
        
        // Execute the round
        let result = executor.execute_round(round, &mut chain_state, participants, fees)?;
        
        println!("Round {} Results:", round);
        println!("  • Emission: {} micro-IPN ({:.6} IPN)", 
            result.emission_micro,
            result.emission_micro as f64 / 100_000_000.0
        );
        println!("  • Fees collected: {} micro-IPN", result.fees_collected_micro);
        println!("  • Participants: {}", result.total_participants);
        println!("  • Total payouts: {} micro-IPN", result.total_payouts);
        println!("  • Total issued: {} micro-IPN ({:.6} IPN)", 
            chain_state.total_issued_micro(),
            chain_state.total_issued_micro() as f64 / 100_000_000.0
        );
        println!("  • Remaining cap: {} micro-IPN", 
            chain_state.remaining_cap(economics_params.max_supply_micro)
        );
        println!();
    }

    // 3. Demonstrate governance parameter update
    println!("🏛️  Demonstrating governance parameter update...\n");
    
    let mut param_manager = ParameterManager::new();
    let initial_emission = param_manager.get_economics_params().initial_round_reward_micro;
    
    // Create a proposal to double the emission rate
    let proposal = ippan_governance::parameters::create_parameter_proposal(
        "economics.initial_round_reward_micro",
        serde_json::Value::Number(serde_json::Number::from(20_000_000)), // Double
        serde_json::Value::Number(serde_json::Number::from(10_000_000)), // Current
        "Double emission rate for demonstration".to_string(),
        [1u8; 32],
        7 * 24 * 3600, // 7 days
    );
    
    println!("Proposal created: {}", proposal.proposal_id);
    println!("  • Parameter: {}", proposal.parameter_name);
    println!("  • Current value: {}", proposal.current_value);
    println!("  • New value: {}", proposal.new_value);
    println!("  • Justification: {}", proposal.justification);
    println!();
    
    // Submit and execute the proposal
    param_manager.submit_parameter_change(proposal.clone())?;
    param_manager.execute_parameter_change(&proposal.proposal_id)?;
    
    let updated_emission = param_manager.get_economics_params().initial_round_reward_micro;
    println!("✅ Parameter updated successfully!");
    println!("  • Old emission rate: {} micro-IPN", initial_emission);
    println!("  • New emission rate: {} micro-IPN", updated_emission);
    println!();

    // 4. Show treasury statistics
    println!("💰 Treasury Statistics:");
    let sink = executor.get_reward_sink();
    let stats = sink.get_statistics();
    println!("  • Total rounds processed: {}", stats.total_rounds);
    println!("  • Total validators: {}", stats.total_validators);
    println!("  • Total distributed: {} micro-IPN ({:.6} IPN)", 
        stats.total_distributed_micro,
        stats.total_distributed_micro as f64 / 100_000_000.0
    );
    println!("  • Average per round: {} micro-IPN", stats.average_per_round);
    println!();

    // 5. Show fee collection statistics
    println!("💸 Fee Collection Statistics:");
    let collector = executor.get_fee_collector();
    let fee_stats = collector.get_statistics();
    println!("  • Total fees collected: {} micro-IPN", fee_stats.total_collected_micro);
    println!("  • Rounds with fees: {}", fee_stats.total_rounds);
    println!("  • Average fees per round: {} micro-IPN", fee_stats.average_per_round);
    println!("  • Highest round fees: {} micro-IPN", fee_stats.highest_round);
    println!("  • Lowest round fees: {} micro-IPN", fee_stats.lowest_round);
    println!();

    // 6. Show account balances
    println!("👥 Account Balances:");
    let account_ledger = executor.get_account_ledger();
    let balances = account_ledger.get_all_balances()?;
    
    for (validator_id, balance) in balances {
        println!("  • Validator {}: {} micro-IPN ({:.6} IPN)", 
            hex::encode(validator_id),
            balance,
            balance as f64 / 100_000_000.0
        );
    }
    println!();

    // 7. Demonstrate supply cap enforcement
    println!("🛡️  Supply Cap Enforcement Test:");
    let test_params = EconomicsParams {
        initial_round_reward_micro: 1_000_000,
        halving_interval_rounds: 1000,
        max_supply_micro: 5_000_000, // Very low cap for testing
        ..Default::default()
    };
    
    let mut test_state = ChainState::with_initial(4_000_000, 0, 0);
    let emission = ippan_economics_core::emission_for_round_capped(1, test_state.total_issued_micro(), &test_params)?;
    
    println!("  • Current supply: {} micro-IPN", test_state.total_issued_micro());
    println!("  • Max supply: {} micro-IPN", test_params.max_supply_micro);
    println!("  • Emission for round 1: {} micro-IPN", emission);
    println!("  • Would exceed cap: {}", test_state.would_exceed_cap(emission, test_params.max_supply_micro));
    println!();

    println!("✅ Demo completed successfully!");
    println!("\nThe DAG-Fair Emission system is now fully integrated into IPPAN, providing:");
    println!("  • Deterministic, capped emission with halving");
    println!("  • Fair reward distribution based on participation and stake");
    println!("  • Fee capping and recycling mechanisms");
    println!("  • Governance-controlled parameters");
    println!("  • Verifiable supply tracking and enforcement");

    Ok(())
}

/// Create demo participants for a round
fn create_demo_participants(round: RoundId) -> Vec<ippan_economics_core::Participation> {
    let mut participants = Vec::new();
    
    // Create 3-5 validators per round
    let num_validators = 3 + (round % 3) as usize;
    
    for i in 0..num_validators {
        let validator_id = [i as u8; 32];
        let stake = 1000 + (i as u128 * 500);
        let blocks_proposed = if i == 0 { 1 } else { 0 };
        let blocks_verified = (i + 1) as u32;
        let reputation = 0.8 + (i as f64 * 0.1);
        
        participants.push(ippan_economics_core::Participation {
            validator_id,
            role: if i == 0 {
                ippan_economics_core::Role::Proposer
            } else {
                ippan_economics_core::Role::Verifier
            },
            blocks_proposed,
            blocks_verified,
            reputation_score: reputation,
            stake_weight: stake,
        });
    }
    
    participants
}

/// Generate demo fees for a round
fn generate_demo_fees(round: RoundId) -> MicroIPN {
    // Generate fees between 0 and 5% of base emission
    let base_emission = 10_000_000;
    let fee_percentage = (round % 6) as u128; // 0-5%
    (base_emission * fee_percentage) / 100
}