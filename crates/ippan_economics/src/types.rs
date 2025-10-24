//! Core types for IPPAN economics

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Round index (HashTimer-based)
pub type RoundIndex = u64;

/// Reward amount in micro-IPN (1 IPN = 10^8 micro-IPN)
pub type RewardAmount = u64;

/// Validator address
pub type ValidatorId = String;

/// Emission parameters that can be configured via governance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmissionParams {
    /// Initial reward per round (in micro-IPN)
    pub initial_round_reward: RewardAmount,
    /// Halving interval in rounds (≈2 years at 10 rounds/second)
    pub halving_interval: RoundIndex,
    /// Total supply cap (in micro-IPN)
    pub total_supply_cap: RewardAmount,
    /// Fee cap as fraction of round reward (0.1 = 10%)
    pub fee_cap_fraction: Decimal,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            initial_round_reward: 10_000, // 0.0001 IPN = 10,000 micro-IPN
            halving_interval: 630_000_000, // ≈2 years at 10 rounds/second
            total_supply_cap: 2_100_000_000_000, // 21M IPN = 2.1T micro-IPN
            fee_cap_fraction: Decimal::new(1, 1), // 0.1 = 10%
        }
    }
}

/// Validator role and participation weight
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ValidatorRole {
    /// Block proposer (1.2x weight)
    Proposer,
    /// Block verifier (1.0x weight)
    Verifier,
    /// Observer (0x weight)
    Observer,
}

impl ValidatorRole {
    /// Get the weight multiplier for this role
    pub fn weight_multiplier(self) -> Decimal {
        match self {
            ValidatorRole::Proposer => Decimal::new(12, 1), // 1.2
            ValidatorRole::Verifier => Decimal::new(10, 1),  // 1.0
            ValidatorRole::Observer => Decimal::ZERO,        // 0.0
        }
    }
}

/// Validator participation in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorParticipation {
    /// Validator ID
    pub validator_id: ValidatorId,
    /// Role in this round
    pub role: ValidatorRole,
    /// Number of micro-blocks contributed
    pub blocks_contributed: u32,
    /// Uptime score (0.0 to 1.0)
    pub uptime_score: Decimal,
}

/// Round reward distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    /// Round index
    pub round_index: RoundIndex,
    /// Total reward pool for this round
    pub total_reward: RewardAmount,
    /// Number of micro-blocks in this round
    pub blocks_in_round: u32,
    /// Individual validator rewards
    pub validator_rewards: HashMap<ValidatorId, ValidatorReward>,
    /// Fees collected in this round
    pub fees_collected: RewardAmount,
    /// Excess burned (if any)
    pub excess_burned: RewardAmount,
}

/// Individual validator reward
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorReward {
    /// Base round emission share
    pub round_emission: RewardAmount,
    /// Transaction fees share
    pub transaction_fees: RewardAmount,
    /// AI service commissions share
    pub ai_commissions: RewardAmount,
    /// Network reward dividend share
    pub network_dividend: RewardAmount,
    /// Total reward for this validator
    pub total_reward: RewardAmount,
    /// Weight factor applied
    pub weight_factor: Decimal,
}

/// Supply tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyInfo {
    /// Current total supply (in micro-IPN)
    pub total_supply: RewardAmount,
    /// Supply cap (in micro-IPN)
    pub supply_cap: RewardAmount,
    /// Remaining supply to be emitted
    pub remaining_supply: RewardAmount,
    /// Percentage of total supply emitted
    pub emission_percentage: Decimal,
    /// Current round index
    pub current_round: RoundIndex,
    /// Next halving round
    pub next_halving_round: RoundIndex,
}

/// Reward composition breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardComposition {
    /// Round emission (60%)
    pub round_emission: RewardAmount,
    /// Transaction fees (25%)
    pub transaction_fees: RewardAmount,
    /// AI service commissions (10%)
    pub ai_commissions: RewardAmount,
    /// Network reward dividend (5%)
    pub network_dividend: RewardAmount,
}

impl RewardComposition {
    /// Create a new reward composition with the specified total
    pub fn new(total_reward: RewardAmount) -> Self {
        let round_emission = (total_reward * 60) / 100;
        let transaction_fees = (total_reward * 25) / 100;
        let ai_commissions = (total_reward * 10) / 100;
        let network_dividend = (total_reward * 5) / 100;

        Self {
            round_emission,
            transaction_fees,
            ai_commissions,
            network_dividend,
        }
    }

    /// Create a new reward composition using actual collected fees
    pub fn new_with_fees(round_reward: RewardAmount, actual_fees: RewardAmount) -> Self {
        // Use actual collected fees instead of minting from emission
        let transaction_fees = actual_fees;
        
        // Calculate remaining reward after using actual fees
        let remaining_reward = round_reward.saturating_sub(transaction_fees);
        
        // Distribute remaining reward among other components
        let round_emission = (remaining_reward * 60) / 100;
        let ai_commissions = (remaining_reward * 10) / 100;
        let network_dividend = (remaining_reward * 5) / 100;

        Self {
            round_emission,
            transaction_fees,
            ai_commissions,
            network_dividend,
        }
    }

    /// Get the total reward
    pub fn total(&self) -> RewardAmount {
        self.round_emission + self.transaction_fees + self.ai_commissions + self.network_dividend
    }
}

/// Emission curve data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionCurvePoint {
    /// Round index
    pub round: RoundIndex,
    /// Reward per round (in micro-IPN)
    pub reward_per_round: RewardAmount,
    /// Annual issuance (in micro-IPN)
    pub annual_issuance: RewardAmount,
    /// Cumulative supply (in micro-IPN)
    pub cumulative_supply: RewardAmount,
    /// Halving epoch (0, 1, 2, ...)
    pub halving_epoch: u32,
}
