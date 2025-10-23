//! Core types for IPPAN Economics

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Micro-IPN unit (1 IPN = 10^8 micro-IPN)
pub const MICRO_PER_IPN: u128 = 100_000_000;

/// Validator identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidatorId(pub String);

/// Validator role in a round
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Proposer,
    Verifier,
}

/// Validator participation in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participation {
    pub role: Role,
    pub blocks: u32,
}

/// Set of validators participating in a round
pub type ParticipationSet = HashMap<ValidatorId, Participation>;

/// Economics parameters for the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicsParams {
    /// Initial reward per round (in micro-IPN)
    pub initial_reward_micro: u128,
    /// Halving interval in rounds
    pub halving_interval_rounds: u64,
    /// Hard supply cap (in micro-IPN)
    pub hard_cap_micro: u128,
    /// Proposer reward percentage (basis points)
    pub proposer_bps: u16,
    /// Verifier reward percentage (basis points)
    pub verifier_bps: u16,
}

impl Default for EconomicsParams {
    fn default() -> Self {
        Self {
            // ~50 IPN/day at 100ms rounds
            initial_reward_micro: 10_000 * MICRO_PER_IPN / 1_000_000, // 0.1 micro-IPN
            // Halving every ~2 years at 200ms rounds
            halving_interval_rounds: 315_000_000,
            // 21 million IPN
            hard_cap_micro: 21_000_000 * MICRO_PER_IPN,
            proposer_bps: 2000, // 20%
            verifier_bps: 8000, // 80%
        }
    }
}

/// Type alias for micro-IPN amounts
pub type MicroIPN = u128;
