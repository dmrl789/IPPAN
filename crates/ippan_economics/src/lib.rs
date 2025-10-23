//! IPPAN Economics Module
//!
//! Comprehensive economic modeling for IPPAN blockchain:
//! - Emission schedules with halving
//! - Validator reward distribution
//! - Fee recycling and burning mechanisms
//! - Supply projection and fairness analysis

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Micro-IPN: smallest unit of IPPAN token (1 IPN = 100,000,000 μIPN)
pub type MicroIPN = u128;

/// Conversion constant: 1 IPN = 10^8 μIPN
pub const MICRO_PER_IPN: u128 = 100_000_000;

/// Validator identifier (e.g. @validator1.ipn)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

impl std::fmt::Display for ValidatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validator role in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Block proposer (higher reward)
    Proposer,
    /// Block verifier (lower reward)
    Verifier,
}

/// Participation record for a validator in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participation {
    pub role: Role,
    pub blocks: u32,
}

/// Set of validator participations in a round
pub type ParticipationSet = HashMap<ValidatorId, Participation>;

/// Reward payouts for validators
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

/// Global economics parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicsParams {
    /// Initial emission per round (μIPN)
    pub initial_emission: MicroIPN,
    /// Rounds between halvings
    pub halving_interval_rounds: u64,
    /// Supply cap (μIPN)
    pub supply_cap: MicroIPN,
    /// Proposer reward share (basis points, 2000 = 20%)
    pub proposer_share_bps: u16,
    /// Verifier reward share (basis points, 8000 = 80%)
    pub verifier_share_bps: u16,
    /// Minimum fee per transaction (μIPN)
    pub min_fee: MicroIPN,
    /// Burn rate for unused emission (basis points)
    pub burn_rate_bps: u16,
}

impl Default for EconomicsParams {
    fn default() -> Self {
        Self {
            initial_emission: 50_000_000, // 0.5 IPN per round
            halving_interval_rounds: 315_000_000, // ~2 years at 200ms rounds
            supply_cap: 21_000_000 * MICRO_PER_IPN, // 21M IPN
            proposer_share_bps: 2000,
            verifier_share_bps: 8000,
            min_fee: 1000,
            burn_rate_bps: 10000, // 100% of unused emission
        }
    }
}

/// Compute emission for a specific round using halving schedule
pub fn emission_for_round(round: u64, params: &EconomicsParams) -> MicroIPN {
    if round == 0 {
        return 0;
    }
    let halvings = (round / params.halving_interval_rounds) as u32;
    if halvings >= 64 {
        return 0;
    }
    params.initial_emission >> halvings
}

/// Distribute rewards for a round among validators
///
/// Returns (payouts, emission_distributed, fees_distributed)
pub fn distribute_round(
    emission_available: MicroIPN,
    fees_collected: MicroIPN,
    participants: &ParticipationSet,
    params: &EconomicsParams,
) -> Result<(Payouts, MicroIPN, MicroIPN), EconomicsError> {
    if participants.is_empty() {
        return Ok((HashMap::new(), 0, 0));
    }

    let total_pool = emission_available.saturating_add(fees_collected);
    
    // Count proposers and verifiers
    let mut _proposer_count = 0u32;
    let mut _verifier_count = 0u32;
    let mut total_proposer_blocks = 0u32;
    let mut total_verifier_blocks = 0u32;

    for participation in participants.values() {
        match participation.role {
            Role::Proposer => {
                _proposer_count += 1;
                total_proposer_blocks += participation.blocks;
            }
            Role::Verifier => {
                _verifier_count += 1;
                total_verifier_blocks += participation.blocks;
            }
        }
    }

    // Split pool according to configured shares
    let proposer_pool = (total_pool * params.proposer_share_bps as u128) / 10_000;
    let verifier_pool = total_pool.saturating_sub(proposer_pool);

    let mut payouts = HashMap::new();
    let mut total_distributed = 0u128;

    // Distribute to proposers (weighted by blocks)
    if total_proposer_blocks > 0 {
        for (vid, participation) in participants.iter() {
            if participation.role == Role::Proposer {
                let share = (proposer_pool * participation.blocks as u128) / total_proposer_blocks as u128;
                *payouts.entry(vid.clone()).or_insert(0) += share;
                total_distributed = total_distributed.saturating_add(share);
            }
        }
    }

    // Distribute to verifiers (weighted by blocks)
    if total_verifier_blocks > 0 {
        for (vid, participation) in participants.iter() {
            if participation.role == Role::Verifier {
                let share = (verifier_pool * participation.blocks as u128) / total_verifier_blocks as u128;
                *payouts.entry(vid.clone()).or_insert(0) += share;
                total_distributed = total_distributed.saturating_add(share);
            }
        }
    }

    let emission_distributed = emission_available.min(total_distributed);
    let fees_distributed = total_distributed.saturating_sub(emission_distributed);

    Ok((payouts, emission_distributed, fees_distributed))
}

/// Calculate burned emission from unused allocation
pub fn epoch_auto_burn(emission_available: MicroIPN, emission_used: MicroIPN) -> MicroIPN {
    emission_available.saturating_sub(emission_used)
}

/// Project total supply after N rounds
pub fn project_supply(rounds: u64, params: &EconomicsParams) -> MicroIPN {
    if rounds == 0 {
        return 0;
    }

    let mut total = 0u128;
    let mut current_halving = 0u32;

    loop {
        let emission = if current_halving >= 64 {
            0
        } else {
            params.initial_emission >> current_halving
        };
        
        if emission == 0 {
            break;
        }

        let halving_start = (current_halving as u64) * params.halving_interval_rounds + 1;
        let halving_end = ((current_halving + 1) as u64) * params.halving_interval_rounds;

        if halving_start > rounds {
            break;
        }

        let effective_end = halving_end.min(rounds);
        let rounds_in_period = (effective_end - halving_start + 1) as u128;
        total = total.saturating_add(emission.saturating_mul(rounds_in_period));

        current_halving += 1;
    }

    total.min(params.supply_cap)
}

/// Economics-related errors
#[derive(Debug, thiserror::Error)]
pub enum EconomicsError {
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Arithmetic overflow")]
    Overflow,
    #[error("Supply cap exceeded")]
    SupplyCapExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emission_halving() {
        let params = EconomicsParams {
            initial_emission: 100_000_000,
            halving_interval_rounds: 1000,
            ..Default::default()
        };
        
        assert_eq!(emission_for_round(500, &params), 100_000_000);
        assert_eq!(emission_for_round(1000, &params), 50_000_000);
        assert_eq!(emission_for_round(2000, &params), 25_000_000);
    }

    #[test]
    fn test_distribution() {
        let params = EconomicsParams::default();
        let mut participants = ParticipationSet::new();
        
        participants.insert(
            ValidatorId("v1".to_string()),
            Participation { role: Role::Proposer, blocks: 2 },
        );
        participants.insert(
            ValidatorId("v2".to_string()),
            Participation { role: Role::Verifier, blocks: 1 },
        );

        let (payouts, distributed, _) = distribute_round(100_000, 0, &participants, &params).unwrap();
        
        assert!(payouts.len() == 2);
        assert!(distributed <= 100_000);
    }

    #[test]
    fn test_supply_projection() {
        let params = EconomicsParams {
            initial_emission: 100_000,
            halving_interval_rounds: 1000,
            supply_cap: 1_000_000_000,
            ..Default::default()
        };

        let supply_1k = project_supply(1000, &params);
        let supply_2k = project_supply(2000, &params);
        
        assert!(supply_1k < supply_2k);
        assert!(supply_2k <= params.supply_cap);
    }

    #[test]
    fn test_burn_tracking() {
        let burned = epoch_auto_burn(100_000, 80_000);
        assert_eq!(burned, 20_000);
    }
}
