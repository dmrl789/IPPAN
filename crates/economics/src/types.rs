//! Core types for the DAG-Fair Emission system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Micro-IPN unit (1 IPN = 10^8 micro-IPN)
pub type MicroIPN = u128;

/// Validator identifier
pub type ValidatorId = [u8; 32];

/// Round identifier
pub type RoundId = u64;

/// Participation role in a round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Block proposer
    Proposer,
    /// Block verifier
    Verifier,
    /// Both proposer and verifier
    Both,
}

/// Participation record for a validator in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participation {
    pub validator_id: ValidatorId,
    pub role: Role,
    pub blocks_proposed: u32,
    pub blocks_verified: u32,
    pub reputation_score: f64,
    pub stake_weight: u128,
}

/// Set of all participants in a round
pub type ParticipationSet = Vec<Participation>;

/// Payouts map: validator_id -> micro-IPN amount
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

/// Economics parameters that can be adjusted via governance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EconomicsParams {
    /// Initial round reward in micro-IPN
    pub initial_round_reward_micro: MicroIPN,
    /// Halving interval in rounds
    pub halving_interval_rounds: u64,
    /// Maximum total supply in micro-IPN (21M IPN = 2,100,000,000,000,000 micro-IPN)
    pub max_supply_micro: MicroIPN,
    /// Fee cap numerator (e.g., 1 for 1/10 = 10% max fees)
    pub fee_cap_numer: u64,
    /// Fee cap denominator (e.g., 10 for 1/10 = 10% max fees)
    pub fee_cap_denom: u64,
    /// Proposer reward weight (basis points, e.g., 2000 = 20%)
    pub proposer_weight_bps: u16,
    /// Verifier reward weight (basis points, e.g., 8000 = 80%)
    pub verifier_weight_bps: u16,
    /// Fee recycling percentage (basis points, e.g., 10000 = 100%)
    pub fee_recycling_bps: u16,
}

impl Default for EconomicsParams {
    fn default() -> Self {
        Self {
            // ~50 IPN/day at 200ms rounds with finalization
            initial_round_reward_micro: 10_000_000, // 0.1 IPN per round
            // Halving every ~2 years at 200ms rounds
            halving_interval_rounds: 315_000_000,
            // 21 million IPN cap
            max_supply_micro: 2_100_000_000_000_000,
            // 10% fee cap
            fee_cap_numer: 1,
            fee_cap_denom: 10,
            // 20% proposer, 80% verifier
            proposer_weight_bps: 2000,
            verifier_weight_bps: 8000,
            // 100% fee recycling
            fee_recycling_bps: 10000,
        }
    }
}

/// Constants for the emission system
pub const MICRO_PER_IPN: u128 = 100_000_000; // 10^8

/// Emission calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionResult {
    pub round: RoundId,
    pub emission_micro: MicroIPN,
    pub total_issued_micro: MicroIPN,
    pub remaining_cap_micro: MicroIPN,
    pub halving_epoch: u32,
}

/// Distribution calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    pub round: RoundId,
    pub total_emission_micro: MicroIPN,
    pub total_fees_micro: MicroIPN,
    pub fee_cap_applied_micro: MicroIPN,
    pub net_emission_micro: MicroIPN,
    pub payouts: Payouts,
    pub proposer_rewards_micro: MicroIPN,
    pub verifier_rewards_micro: MicroIPN,
}

#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_economics_params_default() {
        let params = EconomicsParams::default();
        assert_eq!(params.initial_round_reward_micro, 10_000_000);
        assert_eq!(params.halving_interval_rounds, 315_000_000);
        assert_eq!(params.max_supply_micro, 2_100_000_000_000_000);
        assert_eq!(params.fee_cap_numer, 1);
        assert_eq!(params.fee_cap_denom, 10);
        assert_eq!(params.proposer_weight_bps, 2000);
        assert_eq!(params.verifier_weight_bps, 8000);
        assert_eq!(params.fee_recycling_bps, 10000);
    }

    #[test]
    fn test_participation_creation() {
        let participation = Participation {
            validator_id: [1u8; 32],
            role: Role::Proposer,
            blocks_proposed: 1,
            blocks_verified: 0,
            reputation_score: 1.0,
            stake_weight: 1000,
        };

        assert_eq!(participation.validator_id, [1u8; 32]);
        assert_eq!(participation.role, Role::Proposer);
        assert_eq!(participation.blocks_proposed, 1);
    }
}
