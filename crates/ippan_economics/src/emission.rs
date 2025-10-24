//! Emission and supply logic for the IPPAN DAG-Fair economics system.
//!
//! Handles deterministic per-round emission, halving schedule, and supply-cap enforcement.

use crate::{EcoError, EconomicsParams};
use crate::types::{MicroIPN, RoundIndex};

/// Compute deterministic per-round emission `R(t)`
///
/// Formula:  
/// `R(t) = R0 / 2^{ floor( t / T_h ) }`
///
/// where:
/// - `R0` = initial_round_reward_micro
/// - `T_h` = halving_interval_rounds
pub fn emission_for_round(round: RoundIndex, p: &EconomicsParams) -> MicroIPN {
    if round == 0 {
        return 0;
    }
    if p.halving_interval_rounds == 0 {
        return p.initial_round_reward_micro; // guard for invalid config
    }

    let halvings = round / p.halving_interval_rounds;
    if halvings >= 64 {
        return 0; // stop after 64 halvings
    }

    p.initial_round_reward_micro >> (halvings as u32)
}

/// Clamp emission to remaining supply under the hard cap.
/// Returns `(allowed_emission, remaining_after)`.
pub fn clamp_to_cap(
    requested: MicroIPN,
    already_issued: MicroIPN,
    p: &EconomicsParams,
) -> (MicroIPN, MicroIPN) {
    let remaining = p.hard_cap_micro.saturating_sub(already_issued);
    let allowed = requested.min(remaining);
    (allowed, remaining.saturating_sub(allowed))
}

/// Compute per-round emission with hard-cap enforcement.
/// Returns the emission actually allowed, or `EcoError::HardCapExceeded` if fully capped.
pub fn emission_for_round_capped(
    round: RoundIndex,
    already_issued: MicroIPN,
    p: &EconomicsParams,
) -> Result<MicroIPN, EcoError> {
    let raw = emission_for_round(round, p);
    let (allowed, _) = clamp_to_cap(raw, already_issued, p);

    if allowed == 0 {
        return Err(EcoError::HardCapExceeded {
            requested: raw,
            remaining: 0,
        });
    }

    Ok(allowed)
}

/// Sum total emission across a range of rounds `[start, end]` (inclusive).
pub fn sum_emission_over_rounds<F>(start: RoundIndex, end: RoundIndex, f: F) -> MicroIPN
where
    F: Fn(RoundIndex) -> MicroIPN,
{
    let mut total = 0u128;
    for r in start..=end {
        total = total.saturating_add(f(r));
    }
    total
}

/// Automatic burn logic for epoch reconciliation.
/// If more was produced than expected, burn the excess.
pub fn epoch_auto_burn(expected: MicroIPN, actual: MicroIPN) -> MicroIPN {
    expected.saturating_sub(actual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EconomicsParams;

    #[test]
    fn test_emission_halving() {
        let params = EconomicsParams {
            initial_round_reward_micro: 1000,
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
            initial_round_reward_micro: 1000,
            halving_interval_rounds: 100,
            hard_cap_micro: 5000,
            ..Default::default()
        };

        // Within cap
        assert_eq!(emission_for_round_capped(1, 0, &params).unwrap(), 1000);

        // Near cap (remaining < emission)
        assert_eq!(emission_for_round_capped(1, 4500, &params).unwrap(), 500);

        // Fully capped
        assert!(emission_for_round_capped(1, 5000, &params).is_err());
    }

    #[test]
    fn test_sum_emission_over_rounds() {
        let params = EconomicsParams {
            initial_round_reward_micro: 100,
            halving_interval_rounds: 100,
            ..Default::default()
        };

        let sum = sum_emission_over_rounds(1, 3, |r| emission_for_round(r, &params));
        assert_eq!(sum, 300);
    }

    #[test]
    fn test_epoch_auto_burn() {
        assert_eq!(epoch_auto_burn(1000, 1000), 0);
        assert_eq!(epoch_auto_burn(1000, 1200), 0);
        assert_eq!(epoch_auto_burn(1200, 1000), 200);
    }

    #[test]
    fn test_clamp_to_cap_behavior() {
        let params = EconomicsParams {
            hard_cap_micro: 10_000,
            ..Default::default()
        };
        let (allowed, remaining) = clamp_to_cap(2000, 9000, &params);
        assert_eq!(allowed, 1000);
        assert_eq!(remaining, 0);
    }
}
