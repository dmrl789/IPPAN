//! Reward Pool Module
//!
//! Manages the accumulation and distribution of rewards from emission and fees.

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Validator identifier (32-byte address)
pub type ValidatorId = [u8; 32];

/// Micro-IPN (10^-8 IPN)
pub type MicroIPN = u128;

/// Mapping of validator to reward amount
pub type Payouts = HashMap<ValidatorId, MicroIPN>;

/// In-memory staging of payouts; in production this maps to state storage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RewardSink {
    /// round_id -> (validator -> micro-IPN)
    pub rounds: HashMap<u64, Payouts>,
    /// Total rewards credited per validator across all rounds
    pub validator_totals: HashMap<ValidatorId, MicroIPN>,
}

impl RewardSink {
    /// Create a new reward sink
    pub fn new() -> Self {
        Self::default()
    }

    /// Credit payouts for a finalized round
    pub fn credit_round_payouts(&mut self, round: u64, payouts: &Payouts) -> Result<()> {
        // Store round-specific payouts
        self.rounds.insert(round, payouts.clone());

        // Update validator totals
        for (vid, amount) in payouts {
            *self.validator_totals.entry(*vid).or_insert(0) += amount;
        }

        tracing::debug!(
            "Credited rewards for round {} to {} validators",
            round,
            payouts.len()
        );

        Ok(())
    }

    /// Retrieve total reward accrued by a validator across all rounds
    pub fn validator_total(&self, vid: &ValidatorId) -> MicroIPN {
        self.validator_totals.get(vid).copied().unwrap_or(0)
    }

    /// Get payouts for a specific round
    pub fn get_round_payouts(&self, round: u64) -> Option<&Payouts> {
        self.rounds.get(&round)
    }

    /// Get all validator totals
    pub fn get_all_validator_totals(&self) -> &HashMap<ValidatorId, MicroIPN> {
        &self.validator_totals
    }

    /// Flush payouts into on-chain accounts
    pub fn settle_to_accounts(&self, accounts: &mut dyn AccountLedger) -> Result<()> {
        for (round, payouts) in &self.rounds {
            for (vid, amount) in payouts {
                accounts.credit_validator(vid, *amount)?;
                tracing::debug!(
                    "Settled {} μIPN to {:?} for round {}",
                    amount,
                    hex::encode(vid),
                    round
                );
            }
        }
        Ok(())
    }

    /// Clear all historical round data (keep totals)
    pub fn clear_history(&mut self) {
        self.rounds.clear();
    }

    /// Get total number of rounds recorded
    pub fn round_count(&self) -> usize {
        self.rounds.len()
    }
}

/// Interface expected from account ledger / wallet subsystem
pub trait AccountLedger {
    fn credit_validator(&mut self, vid: &ValidatorId, micro_amount: MicroIPN) -> Result<()>;
    fn get_balance(&self, vid: &ValidatorId) -> Result<MicroIPN>;
}

/// Thread-safe wrapper for RewardSink
#[derive(Clone)]
pub struct SharedRewardSink {
    inner: Arc<RwLock<RewardSink>>,
}

impl SharedRewardSink {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RewardSink::new())),
        }
    }

    pub fn credit_round_payouts(&self, round: u64, payouts: &Payouts) -> Result<()> {
        self.inner.write().credit_round_payouts(round, payouts)
    }

    pub fn validator_total(&self, vid: &ValidatorId) -> MicroIPN {
        self.inner.read().validator_total(vid)
    }

    pub fn get_round_payouts(&self, round: u64) -> Option<Payouts> {
        self.inner.read().get_round_payouts(round).cloned()
    }

    pub fn get_all_validator_totals(&self) -> HashMap<ValidatorId, MicroIPN> {
        self.inner.read().get_all_validator_totals().clone()
    }
}

impl Default for SharedRewardSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reward_sink_basic() {
        let mut sink = RewardSink::new();
        let mut payouts = HashMap::new();
        payouts.insert([1u8; 32], 1000);
        payouts.insert([2u8; 32], 2000);

        sink.credit_round_payouts(1, &payouts).unwrap();

        assert_eq!(sink.validator_total(&[1u8; 32]), 1000);
        assert_eq!(sink.validator_total(&[2u8; 32]), 2000);
        assert_eq!(sink.round_count(), 1);
    }

    #[test]
    fn test_multiple_rounds() {
        let mut sink = RewardSink::new();
        let vid = [1u8; 32];

        // Round 1
        let mut payouts1 = HashMap::new();
        payouts1.insert(vid, 1000);
        sink.credit_round_payouts(1, &payouts1).unwrap();

        // Round 2
        let mut payouts2 = HashMap::new();
        payouts2.insert(vid, 1500);
        sink.credit_round_payouts(2, &payouts2).unwrap();

        assert_eq!(sink.validator_total(&vid), 2500);
        assert_eq!(sink.round_count(), 2);
    }

    #[test]
    fn test_shared_reward_sink() {
        let sink = SharedRewardSink::new();
        let mut payouts = HashMap::new();
        payouts.insert([1u8; 32], 5000);

        sink.credit_round_payouts(10, &payouts).unwrap();

        assert_eq!(sink.validator_total(&[1u8; 32]), 5000);
        let retrieved = sink.get_round_payouts(10);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().get(&[1u8; 32]), Some(&5000));
    }
}
