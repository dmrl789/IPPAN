//! Core types for IPPAN economics
//!
//! Provides deterministic definitions for validator roles, emissions,
//! round-based rewards, and supply tracking across the network.
//!
//! All values are integer-based (micro-IPN precision) for deterministic
//! reproducibility across validator nodes.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Round index (HashTimer-based deterministic timestamp)
pub type RoundIndex = u64;

/// Reward amount in micro-IPN (1 IPN = 10^8 micro-IPN)
pub type RewardAmount = u64;

/// Micro-IPN type alias for consistency with other crates
pub type MicroIPN = u128;

/// Validator identifier
///
/// Can represent:
/// - Ed25519 public key (hex-encoded 64-character string)
/// - Human-readable handle (e.g., `@alice.ipn`)
/// - Registry alias (short internal identifier)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

impl ValidatorId {
    /// Create a new `ValidatorId`
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True if this is a human-readable handle (starts with '@')
    pub fn is_handle(&self) -> bool {
        self.0.starts_with('@')
    }

    /// True if this looks like a valid Ed25519 public key (hex-encoded)
    pub fn is_public_key(&self) -> bool {
        self.0.len() == 64 && self.0.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// True if this is a registry alias (neither handle nor public key)
    pub fn is_alias(&self) -> bool {
        !self.is_handle() && !self.is_public_key()
    }
}

impl fmt::Display for ValidatorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Validator → reward mapping (in micro-IPN)
pub type Payouts = HashMap<ValidatorId, u128>;

/// Emission parameters configurable via governance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmissionParams {
    /// Initial reward per round (micro-IPN)
    pub initial_round_reward_micro: RewardAmount,
    /// Halving interval in rounds (~2 years @ 10 rounds/sec)
    pub halving_interval_rounds: RoundIndex,
    /// Total supply cap (micro-IPN)
    pub max_supply_micro: RewardAmount,
    /// Fee cap as fraction of round reward (e.g., 0.1 = 10%)
    pub fee_cap_fraction: Decimal,
    /// Proposer reward weight (basis points)
    pub proposer_weight_bps: u32,
    /// Verifier reward weight (basis points)
    pub verifier_weight_bps: u32,
}

impl Default for EmissionParams {
    fn default() -> Self {
        Self {
            initial_round_reward_micro: 10_000,   // 0.0001 IPN per round
            halving_interval_rounds: 630_000_000, // ≈ 2 years @ 10 rps
            max_supply_micro: 2_100_000_000_000,  // 21 M IPN
            fee_cap_fraction: Decimal::new(1, 1), // 0.1 = 10%
            proposer_weight_bps: 2000,            // 20 %
            verifier_weight_bps: 8000,            // 80 %
        }
    }
}

/// Role of a validator during a round
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorRole {
    /// Proposer (selected to emit DAG event / block)
    Proposer,
    /// Verifier (confirms other blocks)
    #[default]
    Verifier,
    /// Passive observer (no block contribution)
    Observer,
}

impl ValidatorRole {
    /// Returns deterministic weight multiplier for reward allocation
    pub fn weight_multiplier(self) -> Decimal {
        match self {
            ValidatorRole::Proposer => Decimal::new(12, 1), // 1.2×
            ValidatorRole::Verifier => Decimal::new(10, 1), // 1.0×
            ValidatorRole::Observer => Decimal::ZERO,       // 0.0×
        }
    }
}

/// Validator participation metrics in a given round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorParticipation {
    pub validator_id: ValidatorId,
    pub role: ValidatorRole,
    pub blocks_contributed: u32,
    pub uptime_score: Decimal, // 0.0–1.0
}

/// Per-round reward distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundRewardDistribution {
    pub round_index: RoundIndex,
    pub total_reward: RewardAmount,
    pub blocks_in_round: u32,
    pub validator_rewards: HashMap<ValidatorId, ValidatorReward>,
    pub fees_collected: RewardAmount,
    pub excess_burned: RewardAmount,
}

/// Detailed breakdown of a validator’s reward
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorReward {
    pub round_emission: RewardAmount,
    pub transaction_fees: RewardAmount,
    pub ai_commissions: RewardAmount,
    pub network_dividend: RewardAmount,
    pub total_reward: RewardAmount,
    pub weight_factor: Decimal,
}

/// Total token supply tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyInfo {
    pub total_supply: RewardAmount,
    pub supply_cap: RewardAmount,
    pub remaining_supply: RewardAmount,
    pub emission_percentage: Decimal,
    pub current_round: RoundIndex,
    pub next_halving_round: RoundIndex,
}

/// Breakdown of per-round reward composition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardComposition {
    pub round_emission: RewardAmount,   // 60 %
    pub transaction_fees: RewardAmount, // 25 %
    pub ai_commissions: RewardAmount,   // 10 %
    pub network_dividend: RewardAmount, // 5 %
}

impl RewardComposition {
    /// Deterministic 60 / 25 / 10 / 5 split
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

    /// Adjust distribution using actual collected fees
    pub fn new_with_fees(round_reward: RewardAmount, actual_fees: RewardAmount) -> Self {
        let transaction_fees = actual_fees;
        let remaining = round_reward.saturating_sub(transaction_fees);
        let round_emission = (remaining * 60) / 100;
        let ai_commissions = (remaining * 10) / 100;
        let network_dividend = (remaining * 5) / 100;
        Self {
            round_emission,
            transaction_fees,
            ai_commissions,
            network_dividend,
        }
    }

    /// Compute total reward sum
    pub fn total(&self) -> RewardAmount {
        self.round_emission + self.transaction_fees + self.ai_commissions + self.network_dividend
    }
}

/// Emission curve analytics (for dashboards, charts, and simulations)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmissionCurvePoint {
    pub round: RoundIndex,
    pub reward_per_round: RewardAmount,
    pub annual_issuance: RewardAmount,
    pub cumulative_supply: RewardAmount,
    pub halving_epoch: u32,
}
