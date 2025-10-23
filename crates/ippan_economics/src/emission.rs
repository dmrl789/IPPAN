use crate::types::{MicroIPN, RoundIndex};
use crate::{EcoError, EconomicsParams};

/// Compute the deterministic per-round emission R(t).
/// R(t) = R0 / 2^{ floor( t / T_h ) }
pub fn emission_for_round(round: RoundIndex, p: &EconomicsParams) -> MicroIPN {
    if p.halving_interval_rounds == 0 {
        return p.initial_round_reward_micro; // guard; though config should never set 0
    }
    let halvings = round / p.halving_interval_rounds;
    // Shift right by number of halvings (integer division by 2^n)
    p.initial_round_reward_micro >> (halvings as u32)
}

/// Clamp emission to remaining supply under the hard cap.
/// Returns (allowed_emission, remaining_after).
pub fn clamp_to_cap(
    requested: MicroIPN,
    already_issued: MicroIPN,
    p: &EconomicsParams,
) -> (MicroIPN, MicroIPN) {
    let remaining = p.hard_cap_micro.saturating_sub(already_issued);
    let allowed = requested.min(remaining);
    (allowed, remaining.saturating_sub(allowed))
}

/// Compute per-round emission, enforcing hard cap.
/// Returns the emission actually allowed. Errors only if cap is fully exhausted.
pub fn emission_for_round_capped(
    round: RoundIndex,
    already_issued: MicroIPN,
    p: &EconomicsParams,
) -> Result<MicroIPN, EcoError> {
    let raw = emission_for_round(round, p);
    let (allowed, _remaining_after) = clamp_to_cap(raw, already_issued, p);
    if allowed == 0 {
        return Err(EcoError::HardCapExceeded {
            requested: raw,
            remaining: 0,
        });
    }
    Ok(allowed)
}
