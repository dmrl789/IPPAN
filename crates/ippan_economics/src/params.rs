use crate::types::RoundIndex;
use ippan_types::MicroIPN;
use serde::{Deserialize, Serialize};

/// Static/On-chain configurable parameters controlling emission & distribution.
///
/// These fields are designed to be serialized into chain config storage and
/// adjustable only via on-chain governance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomicsParams {
    /// Genesis hard cap in μIPN (21_000_000 IPN default)
    pub hard_cap_micro: MicroIPN,
    /// Initial reward per round in μIPN (R0). Default: 0.0001 IPN = 100 μIPN
    pub initial_round_reward_micro: MicroIPN,
    /// Halving interval length in rounds (≈ 2 years).
    pub halving_interval_rounds: RoundIndex,
    /// Max share of fees per round as numerator/denominator (e.g. 1/10 = 10%)
    pub fee_cap_numer: u32,
    pub fee_cap_denom: u32,
    /// Role weights (scaled by 1_000 to avoid floats). 1000 = 1.0x
    pub weight_proposer_milli: u32, // e.g. 1200 = 1.2x
    pub weight_verifier_milli: u32, // e.g. 1000 = 1.0x
}

impl Default for EconomicsParams {
    fn default() -> Self {
        // Defaults per PRD example
        // Hard cap: 21_000_000 IPN
        let hard_cap_micro = 21_000_000u128 * 1_000_000u128;

        // R0: 0.0001 IPN -> 100 μIPN
        let initial_round_reward_micro = 100u128;

        // Halving every ~2y. With 5 rounds/s (200ms): 5 * 31_536_000 * 2 = 315_360_000 rounds.
        let halving_interval_rounds = 315_360_000u64;

        Self {
            hard_cap_micro,
            initial_round_reward_micro,
            halving_interval_rounds,
            fee_cap_numer: 1,
            fee_cap_denom: 10,
            weight_proposer_milli: 1200,
            weight_verifier_milli: 1000,
        }
    }
}

impl EconomicsParams {
    /// Returns the fee cap multiplier as a fraction (numer/denom).
    pub fn fee_cap_fraction(&self) -> (u32, u32) {
        (self.fee_cap_numer, self.fee_cap_denom)
    }

    /// Role multiplier in milli (1000 = 1.0x)
    pub fn role_weight_milli(&self, proposer: bool) -> u32 {
        if proposer {
            self.weight_proposer_milli
        } else {
            self.weight_verifier_milli
        }
    }
}
