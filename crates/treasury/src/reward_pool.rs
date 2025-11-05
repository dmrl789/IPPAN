//! Reward Pool Module
//!
//! Manages the accumulation and distribution of rewards from emission and fees
//! integrated with the DAG-Fair emission system.

use crate::account_ledger::AccountLedger;
use anyhow::Result;
use ippan_types::{MicroIPN, ValidatorId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use tracing::{debug, info};

/// Payouts map from validator ID to amount
pub type Payouts = HashMap<ValidatorId, u128>;

/// In-memory staging of payouts; in production this maps to persistent state storage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RewardSink {
    /// round_id → (validator → micro-IPN)
    pub rounds: HashMap<u64, Payouts>,
    /// Total rewards distributed across all rounds
    pub total_distributed_micro: MicroIPN,
}

impl RewardSink {
    /// Create a new reward sink
    pub fn new() -> Self {
        Self {
            rounds: HashMap::new(),
            total_distributed_micro: 0,
        }
    }

    /// Credit payouts for a finalized round
    pub fn credit_round_payouts(&mut self, round: u64, payouts: &Payouts) -> Result<()> {
        if payouts.is_empty() {
            debug!(target: "treasury", "Round {}: No payouts to credit", round);
            return Ok(());
        }

        let round_total: MicroIPN = payouts.values().sum::<u128>();
        self.total_distributed_micro = self.total_distributed_micro.saturating_add(round_total);
        self.rounds.insert(round, payouts.clone());

        info!(
            target: "treasury",
            "Round {}: Credited {} μIPN across {} validators",
            round,
            round_total,
            payouts.len()
        );

        Ok(())
    }

    /// Retrieve total reward accrued by a validator across all rounds
    pub fn validator_total(&self, vid: &ValidatorId) -> MicroIPN {
        self.rounds
            .values()
            .flat_map(|p| p.get(vid))
            .copied()
            .sum::<u128>()
    }

    /// Get payouts for a specific round
    pub fn get_round_payouts(&self, round: u64) -> Option<&Payouts> {
        self.rounds.get(&round)
    }

    /// Get all rounds with payouts
    pub fn get_rounds(&self) -> Vec<u64> {
        self.rounds.keys().copied().collect()
    }

    /// Get total distributed rewards
    pub fn get_total_distributed(&self) -> MicroIPN {
        self.total_distributed_micro
    }

    /// Flush payouts into on-chain accounts
    pub fn settle_to_accounts(&self, accounts: &mut dyn AccountLedger) -> Result<()> {
        let mut total_settled = 0u128;
        let mut rounds_settled = 0;

        for (round, payouts) in &self.rounds {
            for (vid, amount) in payouts {
                accounts.credit_validator(vid, *amount)?;
                total_settled = total_settled.saturating_add(*amount);

                debug!(
                    target: "treasury",
                    "Settled {} μIPN to validator {:?} for round {}",
                    amount, vid, round
                );
            }
            rounds_settled += 1;
        }

        info!(
            target: "treasury",
            "Settled {} μIPN across {} rounds to accounts",
            total_settled,
            rounds_settled
        );

        Ok(())
    }

    /// Clear settled payouts (after successful account settlement)
    pub fn clear_settled_payouts(&mut self, up_to_round: u64) {
        let rounds_to_remove: Vec<u64> = self
            .rounds
            .keys()
            .filter(|&&round| round <= up_to_round)
            .copied()
            .collect();

        for round in rounds_to_remove {
            self.rounds.remove(&round);
        }

        debug!(
            target: "treasury",
            "Cleared settled payouts up to round {}",
            up_to_round
        );
    }

    /// Get statistics about the reward pool
    pub fn get_statistics(&self) -> RewardPoolStatistics {
        let total_rounds = self.rounds.len();
        let total_validators: usize = self
            .rounds
            .values()
            .flat_map(|p| p.keys())
            .collect::<HashSet<_>>()
            .len();

        RewardPoolStatistics {
            total_rounds,
            total_validators,
            total_distributed_micro: self.total_distributed_micro,
            average_per_round: if total_rounds > 0 {
                self.total_distributed_micro / total_rounds as u128
            } else {
                0
            },
        }
    }
}

/// Summary statistics about the reward pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPoolStatistics {
    pub total_rounds: usize,
    pub total_validators: usize,
    pub total_distributed_micro: MicroIPN,
    pub average_per_round: MicroIPN,
}

/// Reward pool manager coordinating between emission and account updates
pub struct RewardPoolManager {
    sink: RewardSink,
    account_ledger: Box<dyn AccountLedger>,
}

impl RewardPoolManager {
    /// Create a new reward pool manager
    pub fn new(account_ledger: Box<dyn AccountLedger>) -> Self {
        Self {
            sink: RewardSink::new(),
            account_ledger,
        }
    }

    /// Process a round’s rewards
    pub fn process_round_rewards(&mut self, round: u64, payouts: &Payouts) -> Result<()> {
        self.sink.credit_round_payouts(round, payouts)?;
        self.sink.settle_to_accounts(self.account_ledger.as_mut())?;
        self.sink.clear_settled_payouts(round);
        Ok(())
    }

    /// Inspect reward sink
    pub fn get_sink(&self) -> &RewardSink {
        &self.sink
    }

    /// Mutable access to reward sink
    pub fn get_sink_mut(&mut self) -> &mut RewardSink {
        &mut self.sink
    }

    /// Get account ledger for queries
    pub fn get_account_ledger(&self) -> &dyn AccountLedger {
        self.account_ledger.as_ref()
    }

    /// Get account ledger for updates
    pub fn get_account_ledger_mut(&mut self) -> &mut dyn AccountLedger {
        self.account_ledger.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account_ledger::MockAccountLedger;

    /// Helper function to create a ValidatorId from a string for testing
    fn test_validator_id(s: &str) -> ValidatorId {
        let hash = blake3::hash(s.as_bytes());
        *hash.as_bytes()
    }

    #[test]
    fn test_reward_sink_creation() {
        let sink = RewardSink::new();
        assert_eq!(sink.get_total_distributed(), 0);
        assert!(sink.get_rounds().is_empty());
    }

    #[test]
    fn test_credit_round_payouts() {
        let mut sink = RewardSink::new();
        let mut payouts: Payouts = HashMap::new();
        let vid1 = test_validator_id("validator1");
        let vid2 = test_validator_id("validator2");
        payouts.insert(vid1, 1000);
        payouts.insert(vid2, 2000);

        sink.credit_round_payouts(1, &payouts).unwrap();

        assert_eq!(sink.get_total_distributed(), 3000);
        assert_eq!(sink.get_rounds().len(), 1);
        assert_eq!(sink.validator_total(&vid1), 1000);
        assert_eq!(sink.validator_total(&vid2), 2000);
    }

    #[test]
    fn test_multiple_rounds() {
        let mut sink = RewardSink::new();
        let vid1 = test_validator_id("validator1");
        let vid2 = test_validator_id("validator2");

        let mut payouts1: Payouts = HashMap::new();
        payouts1.insert(vid1, 1000);
        sink.credit_round_payouts(1, &payouts1).unwrap();

        let mut payouts2: Payouts = HashMap::new();
        payouts2.insert(vid1, 500);
        payouts2.insert(vid2, 1500);
        sink.credit_round_payouts(2, &payouts2).unwrap();

        assert_eq!(sink.get_total_distributed(), 3000);
        assert_eq!(sink.get_rounds().len(), 2);
        assert_eq!(sink.validator_total(&vid1), 1500);
        assert_eq!(sink.validator_total(&vid2), 1500);
    }

    #[test]
    fn test_statistics() {
        let mut sink = RewardSink::new();
        let vid1 = test_validator_id("validator1");
        let vid2 = test_validator_id("validator2");

        let mut payouts1: Payouts = HashMap::new();
        payouts1.insert(vid1, 1000);
        payouts1.insert(vid2, 2000);
        sink.credit_round_payouts(1, &payouts1).unwrap();

        let mut payouts2: Payouts = HashMap::new();
        payouts2.insert(vid1, 500);
        sink.credit_round_payouts(2, &payouts2).unwrap();

        let stats = sink.get_statistics();
        assert_eq!(stats.total_rounds, 2);
        assert_eq!(stats.total_validators, 2);
        assert_eq!(stats.total_distributed_micro, 3500);
        assert_eq!(stats.average_per_round, 1750);
    }

    #[test]
    fn test_reward_pool_manager() {
        let account_ledger = Box::new(MockAccountLedger::new());
        let mut manager = RewardPoolManager::new(account_ledger);

        let mut payouts: Payouts = HashMap::new();
        let vid1 = test_validator_id("validator1");
        payouts.insert(vid1, 1000);

        manager.process_round_rewards(1, &payouts).unwrap();

        let sink = manager.get_sink();
        assert_eq!(sink.get_total_distributed(), 1000);
    }
}
