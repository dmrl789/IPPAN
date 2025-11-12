use crate::errors::EconomicsError;
use crate::types::{EconomicsParams, EmissionResult, MicroIPN, RoundId};

const MAX_HALVING_EPOCH: u32 = 63;

/// Compute the per-round emission without considering the global supply cap.
pub fn emission_for_round(round: RoundId, params: &EconomicsParams) -> MicroIPN {
    if round == 0 {
        return 0;
    }

    let interval = params.halving_interval_rounds.max(1);
    let epoch = ((round.saturating_sub(1)) / interval) as u32;
    let bounded_epoch = epoch.min(MAX_HALVING_EPOCH);

    params.initial_round_reward_micro >> bounded_epoch
}

/// Compute the emission for a round, respecting the total supply cap and previously issued supply.
pub fn emission_for_round_capped(
    round: RoundId,
    total_issued: MicroIPN,
    params: &EconomicsParams,
) -> Result<MicroIPN, EconomicsError> {
    if total_issued > params.max_supply_micro {
        return Err(EconomicsError::SupplyCapExceeded {
            cap: params.max_supply_micro,
            issued: total_issued,
        });
    }

    let remaining = params.max_supply_micro - total_issued;
    if remaining == 0 {
        return Ok(0);
    }

    let emission = emission_for_round(round, params);
    Ok(emission.min(remaining))
}

/// Project the total supply emitted after `round` rounds (inclusive), ignoring the current issued amount.
pub fn project_total_supply(round: RoundId, params: &EconomicsParams) -> MicroIPN {
    if round == 0 {
        return 0;
    }

    let mut total: MicroIPN = 0;
    for r in 1..=round {
        total = total.saturating_add(emission_for_round(r, params));
    }
    total
}

/// Provide a detailed view of the emission state for a specific round.
pub fn get_emission_details(
    round: RoundId,
    total_issued: MicroIPN,
    params: &EconomicsParams,
) -> Result<EmissionResult, EconomicsError> {
    let emission = emission_for_round_capped(round, total_issued, params)?;
    let total_after =
        total_issued
            .checked_add(emission)
            .ok_or(EconomicsError::CalculationOverflow(
                "total_issued_micro overflow",
            ))?;

    let remaining = params.max_supply_micro.saturating_sub(total_after);
    let interval = params.halving_interval_rounds.max(1);
    let halving_epoch = if round == 0 {
        0
    } else {
        ((round.saturating_sub(1)) / interval) as u32
    };

    Ok(EmissionResult {
        round,
        emission_micro: emission,
        total_issued_micro: total_after,
        remaining_cap_micro: remaining,
        halving_epoch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EconomicsParams;

    #[test]
    fn emission_respects_halving_schedule() {
        let mut params = EconomicsParams::default();
        params.initial_round_reward_micro = 128;
        params.halving_interval_rounds = 2;

        assert_eq!(emission_for_round(0, &params), 0);
        assert_eq!(emission_for_round(1, &params), 128);
        assert_eq!(emission_for_round(2, &params), 128);
        assert_eq!(emission_for_round(3, &params), 64);
        assert_eq!(emission_for_round(4, &params), 64);
        assert_eq!(emission_for_round(5, &params), 32);
    }

    #[test]
    fn capped_emission_honors_remaining_supply() {
        let mut params = EconomicsParams::default();
        params.initial_round_reward_micro = 1_000;
        params.max_supply_micro = 10_000;

        let emission = emission_for_round_capped(1, 5_000, &params).unwrap();
        assert_eq!(emission, 1_000);

        // Remaining supply smaller than emission → clamp
        let emission = emission_for_round_capped(2, 9_500, &params).unwrap();
        assert_eq!(emission, 500);

        // Remaining supply exhausted
        let emission = emission_for_round_capped(3, 10_000, &params).unwrap();
        assert_eq!(emission, 0);
    }

    #[test]
    fn capped_emission_errors_when_supply_exceeded() {
        let params = EconomicsParams::default();
        let err = emission_for_round_capped(1, params.max_supply_micro + 1, &params)
            .expect_err("should error when supply exceeded");

        match err {
            EconomicsError::SupplyCapExceeded { cap, issued } => {
                assert_eq!(cap, params.max_supply_micro);
                assert_eq!(issued, params.max_supply_micro + 1);
            }
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn project_total_supply_matches_manual_summation() {
        let mut params = EconomicsParams::default();
        params.initial_round_reward_micro = 100;
        params.halving_interval_rounds = 3;

        // Manual summation for first 6 rounds: 100 + 100 + 100 + 50 + 50 + 50 = 450
        assert_eq!(project_total_supply(0, &params), 0);
        assert_eq!(project_total_supply(6, &params), 450);
    }

    #[test]
    fn emission_details_reports_remaining_supply() {
        let mut params = EconomicsParams::default();
        params.initial_round_reward_micro = 1_000;
        params.halving_interval_rounds = 4;
        params.max_supply_micro = 5_000;

        let result = get_emission_details(3, 2_000, &params).unwrap();
        assert_eq!(result.round, 3);
        assert_eq!(result.emission_micro, 1_000);
        assert_eq!(result.total_issued_micro, 3_000);
        assert_eq!(result.remaining_cap_micro, 2_000);
        assert_eq!(result.halving_epoch, 0);

        let result = get_emission_details(5, 4_500, &params).unwrap();
        assert_eq!(result.emission_micro, 500); // clamped to remaining supply
        assert_eq!(result.total_issued_micro, 5_000);
        assert_eq!(result.remaining_cap_micro, 0);
        assert_eq!(result.halving_epoch, 1);
    }
}
