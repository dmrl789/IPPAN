//! Emission calculation functions for DAG-Fair system

use crate::types::{EconomicsParams, MicroIPN};

/// Calculate emission for a specific round using halving schedule
pub fn emission_for_round(round: u64, params: &EconomicsParams) -> MicroIPN {
    if round == 0 {
        return 0;
    }
    
    let halvings = (round / params.halving_interval_rounds) as u32;
    if halvings >= 64 {
        return 0;
    }
    
    params.initial_reward_micro >> halvings
}

/// Calculate emission for a round with supply cap enforcement
pub fn emission_for_round_capped(
    round: u64,
    current_supply: MicroIPN,
    params: &EconomicsParams,
) -> Result<MicroIPN, &'static str> {
    let base_emission = emission_for_round(round, params);
    let remaining_cap = params.hard_cap_micro.saturating_sub(current_supply);
    
    // If we're at or over the cap, return 0 emission
    if remaining_cap == 0 {
        return Ok(0);
    }
    
    Ok(base_emission.min(remaining_cap))
}

/// Sum emission over a range of rounds
pub fn sum_emission_over_rounds<F>(
    start_round: u64,
    end_round: u64,
    emission_fn: F,
) -> MicroIPN
where
    F: Fn(u64) -> MicroIPN,
{
    let mut total = 0u128;
    for round in start_round..=end_round {
        total = total.saturating_add(emission_fn(round));
    }
    total
}

/// Calculate automatic burn amount for epoch reconciliation
pub fn epoch_auto_burn(expected: MicroIPN, actual: MicroIPN) -> MicroIPN {
    expected.saturating_sub(actual)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_halving() {
        let params = EconomicsParams {
            initial_reward_micro: 1000,
            halving_interval_rounds: 100,
            ..Default::default()
        };
        
        assert_eq!(emission_for_round(0, &params), 0);
        assert_eq!(emission_for_round(50, &params), 1000);
        assert_eq!(emission_for_round(100, &params), 500);
        assert_eq!(emission_for_round(200, &params), 250);
    }

    #[test]
    fn test_emission_capped() {
        let params = EconomicsParams {
            initial_reward_micro: 1000,
            halving_interval_rounds: 100,
            hard_cap_micro: 5000,
            ..Default::default()
        };
        
        // Within cap
        assert_eq!(emission_for_round_capped(1, 0, &params).unwrap(), 1000);
        
        // At cap - should return 0, not error
        assert_eq!(emission_for_round_capped(1, 5000, &params).unwrap(), 0);
        
        // Over cap - should return 0, not error
        assert_eq!(emission_for_round_capped(1, 6000, &params).unwrap(), 0);
    }

    #[test]
    fn test_sum_emission() {
        let params = EconomicsParams {
            initial_reward_micro: 1000,
            halving_interval_rounds: 100,
            ..Default::default()
        };
        
        let sum = sum_emission_over_rounds(1, 3, |r| emission_for_round(r, &params));
        assert_eq!(sum, 3000);
    }
}
