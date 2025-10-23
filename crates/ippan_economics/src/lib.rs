//! IPPAN Economics crate
//! 
//! Provides deterministic DAG-Fair emission primitives used by tests
//! and higher-level consensus components.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type MicroIPN = u128;

/// Number of micro-IPN per IPN
pub const MICRO_PER_IPN: MicroIPN = 100_000_000; // 10^8

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Proposer,
    Verifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participation {
    pub role: Role,
    pub blocks: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParticipationSet(pub HashMap<ValidatorId, Participation>);

impl ParticipationSet {
    pub fn insert(
        &mut self,
        validator_id: ValidatorId,
        participation: Participation,
    ) -> Option<Participation> {
        self.0.insert(validator_id, participation)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicsParams {
    /// Initial per-round emission in micro-IPN
    pub base_emission_micro: MicroIPN,
    /// Halving interval in rounds
    pub halving_interval_rounds: u64,
    /// Hard cap of supply in micro-IPN
    pub hard_cap_micro: MicroIPN,
    /// Proposer pool share, basis points (10000 = 100%)
    pub proposer_bps: u16,
    /// Verifier pool share, basis points (should sum to 10000 with proposer_bps)
    pub verifier_bps: u16,
}

impl Default for EconomicsParams {
    fn default() -> Self {
        Self {
            base_emission_micro: 10_000, // arbitrary small default for tests
            halving_interval_rounds: 315_000_000,
            hard_cap_micro: 21_000_000u128 * MICRO_PER_IPN,
            proposer_bps: 2000,
            verifier_bps: 8000,
        }
    }
}

/// Compute the raw emission for a given round (without cap),
/// including round 0 as the first round at full emission.
pub fn emission_for_round(round: u64, params: &EconomicsParams) -> MicroIPN {
    let halvings = if params.halving_interval_rounds == 0 {
        0
    } else {
        (round / params.halving_interval_rounds) as u32
    };
    if halvings >= 64 {
        return 0;
    }
    params.base_emission_micro >> halvings
}

/// Compute emission for a round but cap to remaining supply.
/// Returns Some(value) unless the cap has been fully reached.
pub fn emission_for_round_capped(
    round: u64,
    total_issued_so_far: MicroIPN,
    params: &EconomicsParams,
) -> Option<MicroIPN> {
    if total_issued_so_far >= params.hard_cap_micro {
        return None;
    }
    let remaining = params.hard_cap_micro - total_issued_so_far;
    let raw = emission_for_round(round, params);
    Some(raw.min(remaining))
}

/// Sum emission over an inclusive round range using a provided function.
pub fn sum_emission_over_rounds<F>(start_round: u64, end_round: u64, f: F) -> MicroIPN
where
    F: Fn(u64) -> MicroIPN,
{
    if end_round < start_round {
        return 0;
    }
    let mut total: MicroIPN = 0;
    let mut r = start_round;
    while r <= end_round {
        total = total.saturating_add(f(r));
        r = r.saturating_add(1);
    }
    total
}

/// Compute automatic burn due to rounding or over-estimation.
pub fn epoch_auto_burn(expected: MicroIPN, actual: MicroIPN) -> MicroIPN {
    expected.saturating_sub(actual)
}

/// Distribute a round's emission among participants.
/// Returns (payouts, emission_paid, fees_paid).
/// - payouts are only from emission, fees distribution is not modeled.
pub fn distribute_round(
    emission_micro: MicroIPN,
    fees_micro: MicroIPN,
    parts: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Vec<(ValidatorId, MicroIPN)>, MicroIPN, MicroIPN), &'static str> {
    if emission_micro == 0 || parts.0.is_empty() {
        return Ok((Vec::new(), 0, 0));
    }

    let proposer_pool = (emission_micro * params.proposer_bps as MicroIPN) / 10_000u128;
    let verifier_pool = emission_micro.saturating_sub(proposer_pool);

    let mut total_proposer_blocks: u128 = 0;
    let mut total_verifier_blocks: u128 = 0;
    for p in parts.0.values() {
        match p.role {
            Role::Proposer => total_proposer_blocks = total_proposer_blocks.saturating_add(p.blocks as u128),
            Role::Verifier => total_verifier_blocks = total_verifier_blocks.saturating_add(p.blocks as u128),
        }
    }

    let mut payouts: Vec<(ValidatorId, MicroIPN)> = Vec::with_capacity(parts.0.len());
    let mut paid_total: MicroIPN = 0;

    for (vid, p) in parts.0.iter() {
        let share = match p.role {
            Role::Proposer => {
                if total_proposer_blocks == 0 {
                    0
                } else {
                    (proposer_pool * (p.blocks as MicroIPN)) / total_proposer_blocks
                }
            }
            Role::Verifier => {
                if total_verifier_blocks == 0 {
                    0
                } else {
                    (verifier_pool * (p.blocks as MicroIPN)) / total_verifier_blocks
                }
            }
        };
        if share > 0 {
            payouts.push((vid.clone(), share));
            paid_total = paid_total.saturating_add(share);
        }
    }

    // Fees are ignored in payouts in this simplified model, but we return them as paid.
    Ok((payouts, paid_total, fees_micro))
}
