//! Deterministic round reward distribution for IPPAN BlockDAG.
//! Ensures fair proportional allocation under the DAG-Fair model,
//! using weighted participation and fee caps defined in `EconomicsParams`.

use crate::{EcoError, EconomicsParams};
use crate::types::{MicroIPN, ParticipationSet, Payouts, Role};

/// Distribute a round’s total emission (μIPN) and fees across validators proportionally,
/// according to their recorded participation (`blocks`) and role (proposer/verifier).
///
/// - `emission_micro`: total emission allocated for the round (after halving and cap)
/// - `fees_micro`: total transaction fees collected during the round
/// - `parts`: per-validator participation map
///
/// Returns `(payouts_map, emission_paid, fees_paid)`.
///
/// If `parts` is empty, this returns an empty set with `(0, 0)`.
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    parts: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Payouts, MicroIPN, MicroIPN), EcoError> {
    // --- Fee cap enforcement ---
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

    // --- Weighted total calculation ---
    let mut total_weighted: u128 = 0;
    for p in parts.values() {
        let weight = role_weight(params, p.role) as u128;
        total_weighted = total_weighted.saturating_add(weight.saturating_mul(p.blocks as u128));
    }

    // --- No participation → no payout ---
    if total_weighted == 0 {
        return Ok((Payouts::default(), 0, 0));
    }

    // --- Pool to distribute ---
    let pool = emission_micro.saturating_add(fees_micro);

    // --- Proportional payout allocation ---
    let mut payouts = Payouts::default();
    let mut distributed: u128 = 0;

    for (vid, p) in parts.iter() {
        let weight = role_weight(params, p.role) as u128;
        let share = weight.saturating_mul(p.blocks as u128);
        let amt = pool.saturating_mul(share) / total_weighted;

        if amt > 0 {
            payouts.insert(vid.clone(), amt);
            distributed = distributed.saturating_add(amt);
        }
    }

    // Slight rounding remainder (due to integer division) is ignored or can be auto-burned.
    Ok((payouts, emission_micro, fees_micro))
}

/// Return milli-weight for each role (1000 = 1.0x)
#[inline]
fn role_weight(params: &EconomicsParams, role: Role) -> u32 {
    match role {
        Role::Proposer => params.weight_proposer_milli,
        Role::Verifier => params.weight_verifier_milli,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Participation, ValidatorId};

    #[test]
    fn test_distribute_round_proportional() {
        let params = EconomicsParams::default();

        let mut parts = ParticipationSet::new();
        parts.insert(
            ValidatorId("alice".into()),
            Participation { role: Role::Proposer, blocks: 5 },
        );
        parts.insert(
            ValidatorId("bob".into()),
            Participation { role: Role::Verifier, blocks: 10 },
        );

        let emission = 1_000_000u128; // 1 IPN
        let fees = 100_000u128; // 0.1 IPN
        let (payouts, e_paid, f_paid) = distribute_round(emission, fees, &parts, &params).unwrap();

        assert_eq!(e_paid, emission);
        assert_eq!(f_paid, fees);
        assert_eq!(payouts.len(), 2);

        // Proposer gets higher weight → higher reward ratio
        let a = payouts.get(&ValidatorId("alice".into())).copied().unwrap_or(0);
        let b = payouts.get(&ValidatorId("bob".into())).copied().unwrap_or(0);
        assert!(a < b); // 5 vs 10 blocks, even with 1.2x weight
    }

    #[test]
    fn test_fee_cap_enforced() {
        let params = EconomicsParams::default();
        let mut parts = ParticipationSet::new();
        parts.insert(
            ValidatorId("alice".into()),
            Participation { role: Role::Verifier, blocks: 10 },
        );
        let result = distribute_round(1_000_000, 200_000, &parts, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_participation_yields_zero() {
        let params = EconomicsParams::default();
        let parts = ParticipationSet::new();
        let (payouts, e, f) = distribute_round(1000, 100, &parts, &params).unwrap();
        assert!(payouts.is_empty());
        assert_eq!(e, 0);
        assert_eq!(f, 0);
    }
}
