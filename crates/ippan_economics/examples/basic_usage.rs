//! Basic usage example for IPPAN Economics crate
//! 
//! This example demonstrates how to use the economics crate for:
//! - Computing round-based emission with halving
//! - Distributing rewards across validators
//! - Enforcing fee caps and hard caps

use ippan_economics::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("IPPAN Economics - Basic Usage Example\n");

    // 1. Create default economics parameters
    let params = EconomicsParams::default();
    println!("Economics Parameters:");
    println!("  Hard Cap: {} μIPN ({} IPN)", 
        params.hard_cap_micro, 
        params.hard_cap_micro / MICRO_PER_IPN);
    println!("  Initial Round Reward: {} μIPN ({} IPN)", 
        params.initial_round_reward_micro,
        params.initial_round_reward_micro as f64 / MICRO_PER_IPN as f64);
    println!("  Halving Interval: {} rounds", params.halving_interval_rounds);
    println!("  Fee Cap: {}/{} ({}%)", 
        params.fee_cap_numer, 
        params.fee_cap_denom,
        (params.fee_cap_numer as f64 / params.fee_cap_denom as f64) * 100.0);
    println!();

    // 2. Demonstrate emission calculation for different rounds
    println!("Emission Calculation Examples:");
    for round in [0, 1, 100, 1000, params.halving_interval_rounds, params.halving_interval_rounds + 1] {
        let emission = emission_for_round(round, &params);
        println!("  Round {}: {} μIPN ({} IPN)", 
            round, 
            emission,
            emission as f64 / MICRO_PER_IPN as f64);
    }
    println!();

    // 3. Create a participation set for a round
    let mut participation = HashMap::new();
    participation.insert(
        ValidatorId("alice".to_string()),
        Participation { role: Role::Proposer, blocks: 5 },
    );
    participation.insert(
        ValidatorId("bob".to_string()),
        Participation { role: Role::Verifier, blocks: 10 },
    );
    participation.insert(
        ValidatorId("charlie".to_string()),
        Participation { role: Role::Verifier, blocks: 3 },
    );

    println!("Participation Set:");
    for (validator, part) in &participation {
        let weight = match part.role {
            Role::Proposer => params.weight_proposer_milli,
            Role::Verifier => params.weight_verifier_milli,
        };
        println!("  {}: {:?} with {} blocks (weight: {})", 
            validator.0, part.role, part.blocks, weight);
    }
    println!();

    // 4. Distribute rewards for a round
    let round_emission = 1_000_000; // 1 IPN in μIPN
    let fees_collected = 50_000;    // 0.05 IPN in μIPN

    println!("Round Settlement:");
    println!("  Emission: {} μIPN ({} IPN)", 
        round_emission, 
        round_emission as f64 / MICRO_PER_IPN as f64);
    println!("  Fees: {} μIPN ({} IPN)", 
        fees_collected, 
        fees_collected as f64 / MICRO_PER_IPN as f64);

    let (payouts, emission_paid, fees_paid) = distribute_round(
        round_emission,
        fees_collected,
        &participation,
        &params,
    )?;

    println!("  Total Pool: {} μIPN ({} IPN)", 
        emission_paid + fees_paid,
        (emission_paid + fees_paid) as f64 / MICRO_PER_IPN as f64);
    println!();

    println!("Payouts:");
    for (validator, amount) in &payouts {
        println!("  {}: {} μIPN ({} IPN)", 
            validator.0, 
            amount,
            *amount as f64 / MICRO_PER_IPN as f64);
    }
    println!();

    // 5. Demonstrate hard cap enforcement
    println!("Hard Cap Enforcement:");
    let almost_at_cap = params.hard_cap_micro - 1000;
    let emission_at_cap = emission_for_round_capped(0, almost_at_cap, &params)?;
    println!("  Remaining cap: {} μIPN", params.hard_cap_micro - almost_at_cap);
    println!("  Emission allowed: {} μIPN", emission_at_cap);

    // 6. Demonstrate epoch verification
    println!("\nEpoch Verification:");
    let expected_emission = sum_emission_over_rounds(0, 9, |r| emission_for_round(r, &params));
    let actual_minted = expected_emission + 1000; // Simulate over-minting
    let burn_amount = epoch_auto_burn(expected_emission, actual_minted);
    println!("  Expected emission (rounds 0-9): {} μIPN", expected_emission);
    println!("  Actual minted: {} μIPN", actual_minted);
    println!("  Auto-burn required: {} μIPN", burn_amount);

    Ok(())
}