//! Core types for IPPAN economics

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Round index (HashTimer-based)
pub type RoundIndex = u64;

/// Reward amount in micro-IPN (1 IPN = 10^8 micro-IPN)
pub type RewardAmount = u64;

/// Validator identifier
///
/// Can be one of:
/// - Ed25519 public key (hex-encoded 64-char string)
/// - Human-readable handle (e.g., `@alice.ipn`)
/// - Registry alias (custom short ID)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

impl ValidatorId {
    /// Create a new ValidatorId from string
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as &str
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this is a human-readable handle (starts with '@')
    pub fn is_handle(&self) -> bool {
        self.0.starts_with('@')
    }

    /// Check if this is a valid hex-encoded public key
    pub fn is_public_key(&self) -> bool {
        self.0.len() == 64 && self.0.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Check if this is a registry alias (not handle or pubkey)
    pub fn is_alias(&self) -> bool {
        !self.is_handle() && !self.is_public_key()
    }
}

impl std::fmt::Display for ValidatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validator rewards mapping (validator → micro-IPN)
pub type Payouts = HashMap<ValidatorId, u128>;

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
            halving_interval: 630_000_000, // ≈2 years at 10 rps
            total_supply_cap: 2_100_000_000_000, // 21 M IPN = 2.1 T micro-IPN
            fee_cap_fraction: Decimal::new(1, 1), // 0.1 = 10%
        }
    }
}

/// Participation role within a round
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorRole {
    /// Validator proposing a block or DAG event (1.2×)
    Proposer,
    /// Validator verifying others’ blocks (1.0×)
    #[default]
    Verifier,
    /// Observer (non-participating)
    Observer,
}

impl ValidatorRole {
    /// Get weight multiplier for this role
    pub fn weight_multiplier(self) -> Decimal {
        match self {
            ValidatorRole::Proposer => Decimal::new(12, 1), // 1.2
            ValidatorRole::Verifier => Decimal::new(10, 1), // 1.0
            ValidatorRole::Observer => Decimal::ZERO,       // 0.0
        }
    }
}

/// Validator participation data for a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorParticipation {
    pub validator_id: ValidatorId,
    pub role: ValidatorRole,
    pub blocks_contributed: u32,
    pub uptime_score: Decimal, // 0.0–1.0
}

/// Round reward distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    pub round_index: RoundIndex,
    pub total_reward: RewardAmount,
    pub blocks_in_round: u32,
    pub validator_rewards: HashMap<ValidatorId, ValidatorReward>,
    pub fees_collected: RewardAmount,
    pub excess_burned: RewardAmount,
}

/// Individual validator reward breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorReward {
    pub round_emission: RewardAmount,
    pub transaction_fees: RewardAmount,
    pub ai_commissions: RewardAmount,
    pub network_dividend: RewardAmount,
    pub total_reward: RewardAmount,
    pub weight_factor: Decimal,
}

/// Supply tracking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyInfo {
    pub total_supply: RewardAmount,
    pub supply_cap: RewardAmount,
    pub remaining_supply: RewardAmount,
    pub emission_percentage: Decimal,
    pub current_round: RoundIndex,
    pub next_halving_round: RoundIndex,
}

/// Reward composition percentages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardComposition {
    pub round_emission: RewardAmount,   // 60%
    pub transaction_fees: RewardAmount, // 25%
    pub ai_commissions: RewardAmount,   // 10%
    pub network_dividend: RewardAmount, // 5%
}

impl RewardComposition {
    /// Create new reward composition (using fixed 60/25/10/5 distribution)
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

    /// Create composition using actual collected fees (dynamic adjustment)
    pub fn new_with_fees(round_reward: RewardAmount, actual_fees: RewardAmount) -> Self {
        let transaction_fees = actual_fees;
        let remaining_reward = round_reward.saturating_sub(transaction_fees);
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

    /// Compute total reward
    pub fn total(&self) -> RewardAmount {
        self.round_emission
            + self.transaction_fees
            + self.ai_commissions
            + self.network_dividend
    }
}

/// Emission curve data point for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionCurvePoint {
    pub round: RoundIndex,
    pub reward_per_round: RewardAmount,
    pub annual_issuance: RewardAmount,
    pub cumulative_supply: RewardAmount,
    pub halving_epoch: u32,
}
