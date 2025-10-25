//! DAG-Fair Emission calculation with hard supply cap enforcement

use crate::types::{EconomicsParams, EmissionResult, MicroIPN, RoundId, MICRO_PER_IPN};
use anyhow::Result;
use tracing::{debug, info, warn};

/// Calculate emission for a round with hard supply cap enforcement
pub fn emission_for_round_capped(
    round: RoundId,
    current_issued_micro: MicroIPN,
    params: &EconomicsParams,
) -> Result<MicroIPN> {
    if round == 0 {
        return Ok(0);
    }

    // Calculate base emission for this round
    let base_emission = calculate_base_emission(round, params);
    
    // Calculate halving epoch
    let halving_epoch = (round - 1) / params.halving_interval_rounds;
    
    // Check if we've hit the supply cap
    let remaining_cap = params.max_supply_micro.saturating_sub(current_issued_micro);
    
    if remaining_cap == 0 {
        debug!(
            target: "emission",
            "Round {}: Supply cap reached, no emission",
            round
        );
        return Ok(0);
    }
    
    // Cap emission to remaining supply
    let capped_emission = base_emission.min(remaining_cap);
    
    if capped_emission != base_emission {
        warn!(
            target: "emission",
            "Round {}: Emission capped from {} to {} micro-IPN (supply cap)",
            round,
            base_emission,
            capped_emission
        );
    }
    
    info!(
        target: "emission",
        "Round {}: Emitting {} micro-IPN (≈ {:.6} IPN), halving epoch {}",
        round,
        capped_emission,
        (capped_emission as f64) / (MICRO_PER_IPN as f64),
        halving_epoch
    );
    
    Ok(capped_emission)
}

/// Calculate base emission for a round (before supply cap)
fn calculate_base_emission(round: RoundId, params: &EconomicsParams) -> MicroIPN {
    if round == 0 {
        return 0;
    }
    
    let halving_epoch = (round - 1) / params.halving_interval_rounds;
    
    // Prevent overflow with too many halvings
    if halving_epoch >= 64 {
        return 0;
    }
    
    // Apply halving: emission = initial / (2^halving_epoch)
    params.initial_round_reward_micro >> halving_epoch
}

/// Project total supply after a given number of rounds
pub fn project_total_supply(rounds: RoundId, params: &EconomicsParams) -> MicroIPN {
    if rounds == 0 {
        return 0;
    }
    
    let mut total = 0u128;
    let mut current_round = 1;
    
    while current_round <= rounds {
        let emission = calculate_base_emission(current_round, params);
        if emission == 0 {
            break;
        }
        
        total = total.saturating_add(emission);
        current_round += 1;
    }
    
    total.min(params.max_supply_micro)
}

/// Calculate remaining supply cap
pub fn calculate_remaining_cap(
    current_issued_micro: MicroIPN,
    params: &EconomicsParams,
) -> MicroIPN {
    params.max_supply_micro.saturating_sub(current_issued_micro)
}

/// Get emission details for a round
pub fn get_emission_details(
    round: RoundId,
    current_issued_micro: MicroIPN,
    params: &EconomicsParams,
) -> Result<EmissionResult> {
    let emission_micro = emission_for_round_capped(round, current_issued_micro, params)?;
    let total_issued_micro = current_issued_micro.saturating_add(emission_micro);
    let remaining_cap_micro = calculate_remaining_cap(total_issued_micro, params);
    let halving_epoch = if round == 0 { 0u32 } else { ((round - 1) / params.halving_interval_rounds) as u32 };
    
    Ok(EmissionResult {
        round,
        emission_micro,
        total_issued_micro,
        remaining_cap_micro,
        halving_epoch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_halving() {
        let params = EconomicsParams {
            initial_round_reward_micro: 1_000_000,
            halving_interval_rounds: 1000,
            ..Default::default()
        };
        
        // First halving epoch
        assert_eq!(emission_for_round_capped(999, 0, &params).unwrap(), 1_000_000);
        assert_eq!(emission_for_round_capped(1000, 0, &params).unwrap(), 500_000);
        assert_eq!(emission_for_round_capped(1999, 0, &params).unwrap(), 500_000);
        
        // Second halving epoch
        assert_eq!(emission_for_round_capped(2000, 0, &params).unwrap(), 250_000);
        assert_eq!(emission_for_round_capped(2999, 0, &params).unwrap(), 250_000);
    }

    #[test]
    fn test_supply_cap_enforcement() {
        let params = EconomicsParams {
            initial_round_reward_micro: 1_000_000,
            halving_interval_rounds: 1000,
            max_supply_micro: 5_000_000, // Very low cap for testing
            ..Default::default()
        };
        
        // Should be capped at remaining supply
        let emission = emission_for_round_capped(1, 4_000_000, &params).unwrap();
        assert_eq!(emission, 1_000_000); // Normal emission
        
        let emission = emission_for_round_capped(2, 4_500_000, &params).unwrap();
        assert_eq!(emission, 500_000); // Capped to remaining supply
        
        let emission = emission_for_round_capped(3, 5_000_000, &params).unwrap();
        assert_eq!(emission, 0); // Cap reached
    }

    #[test]
    fn test_zero_round() {
        let params = EconomicsParams::default();
        assert_eq!(emission_for_round_capped(0, 0, &params).unwrap(), 0);
    }

    #[test]
    fn test_projected_supply() {
        let params = EconomicsParams {
            initial_round_reward_micro: 1_000_000,
            halving_interval_rounds: 1000,
            max_supply_micro: 10_000_000,
            ..Default::default()
        };
        
        let supply_1000 = project_total_supply(1000, &params);
        assert_eq!(supply_1000, 1_000_000 * 1000);
        
        let supply_2000 = project_total_supply(2000, &params);
        assert_eq!(supply_2000, 1_000_000 * 1000 + 500_000 * 1000);
    }

    #[test]
    fn test_emission_details() {
        let params = EconomicsParams::default();
        let details = get_emission_details(1000, 0, &params).unwrap();
        
        assert_eq!(details.round, 1000);
        assert!(details.emission_micro > 0);
        assert_eq!(details.total_issued_micro, details.emission_micro);
        assert!(details.remaining_cap_micro > 0);
        assert_eq!(details.halving_epoch, 0);
    }
}