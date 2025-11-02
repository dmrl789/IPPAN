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
    // Tests temporarily disabled - awaiting implementation of emission functions
    // use super::*;
    // use crate::types::{EconomicsParams, Participation, Role};
}
