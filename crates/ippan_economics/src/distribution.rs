use crate::{EcoError, EconomicsParams};
use crate::types::{MicroIPN, ParticipationSet, Payouts, Role};

/// Distribute a round's emission across validators proportionally to their
/// recorded micro-blocks, weighted by role (proposer/verifier).
///
/// - `emission_micro`: the round's emission (μIPN), *after* hard-cap clamping
/// - `fees_micro`: total fees collected in the round (μIPN); must obey fee cap
/// - `parts`: per-validator participation (role + block counts)
///
/// Returns (payouts_map, emission_paid, fees_paid).
///
/// NOTE: If `parts` is empty or sums to zero, this pays nothing and returns Ok(empty).
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    parts: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Payouts, MicroIPN, MicroIPN), EcoError> {
    // Enforce fee cap
    let (numer, denom) = params.fee_cap_fraction();
    let fee_cap = emission_micro
        .saturating_mul(numer as u128)
        / (denom as u128).max(1);
    if fees_micro > fee_cap {
        return Err(EcoError::FeeCapExceeded {
            fees: fees_micro,
            cap: fee_cap,
        });
    }

    // Compute total "weighted blocks" across all validators
    let mut total_weighted: u128 = 0;
    for p in parts.values() {
        let w = role_weight(params, p.role) as u128;
        total_weighted = total_weighted.saturating_add(w.saturating_mul(p.blocks as u128));
    }

    // If no participation, pay nothing (silent success)
    if total_weighted == 0 {
        return Ok((Payouts::default(), 0, 0));
    }

    // Pool to distribute: emission + fees
    let pool = emission_micro.saturating_add(fees_micro);

    // Proportional allocation
    let mut payouts = Payouts::default();
    let mut distributed: u128 = 0;

    for (vid, p) in parts.iter() {
        let w = role_weight(params, p.role) as u128;
        let share_num = w.saturating_mul(p.blocks as u128);

        // floor division — keep remainder for burn/treasury if desired
        let amt = pool.saturating_mul(share_num) / total_weighted;

        if amt > 0 {
            payouts.insert(vid.clone(), amt);
            distributed = distributed.saturating_add(amt);
        }
    }

    // `distributed` may be slightly less than `pool` due to integer division.
    // The difference can be credited to a "rounding sink" (auto-burn or treasury).
    // Here we simply return the amounts actually paid.
    Ok((payouts, emission_micro, fees_micro))
}

/// Helper: convert role to milli weight (1000 = 1.0x)
#[inline]
fn role_weight(params: &EconomicsParams, role: Role) -> u32 {
    match role {
        Role::Proposer => params.weight_proposer_milli,
        Role::Verifier => params.weight_verifier_milli,
    }
}