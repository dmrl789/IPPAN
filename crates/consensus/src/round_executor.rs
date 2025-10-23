//! Round execution and finalization with DAG-Fair emission integration

use crate::fees::FeeCollector;
use ippan_economics_core::{
    distribute_round, emission_for_round_capped, EconomicsParams, Participation, ParticipationSet,
    Role, MICRO_PER_IPN,
};
use ippan_treasury::{RewardSink, AccountLedger};
use ippan_types::{ChainState, MicroIPN, RoundId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Round execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundExecutionResult {
    pub round: RoundId,
    pub emission_micro: MicroIPN,
    pub fees_collected_micro: MicroIPN,
    pub total_participants: usize,
    pub total_payouts: MicroIPN,
    pub state_root: [u8; 32],
}

/// Round executor that coordinates consensus, emission, and distribution
pub struct RoundExecutor {
    economics_params: EconomicsParams,
    reward_sink: RewardSink,
    fee_collector: FeeCollector,
    account_ledger: Box<dyn AccountLedger>,
}

impl RoundExecutor {
    /// Create a new round executor
    pub fn new(
        economics_params: EconomicsParams,
        account_ledger: Box<dyn AccountLedger>,
    ) -> Self {
        Self {
            economics_params,
            reward_sink: RewardSink::new(),
            fee_collector: FeeCollector::new(),
            account_ledger,
        }
    }

    /// Execute a round with DAG-Fair emission and distribution
    pub fn execute_round(
        &mut self,
        round: RoundId,
        chain_state: &mut ChainState,
        participants: ParticipationSet,
        fees_micro: MicroIPN,
    ) -> Result<RoundExecutionResult> {
        // Validate participation set
        ippan_economics::validate_participation_set(&participants)?;

        // Collect fees for this round
        self.fee_collector.collect_round_fees(round, fees_micro)?;

        // Calculate emission for this round (with supply cap enforcement)
        let current_issued = chain_state.total_issued_micro();
        let emission_micro = emission_for_round_capped(round, current_issued, &self.economics_params)?;

        // Distribute rewards fairly among participants
        let (payouts, emission_paid, fees_capped) = distribute_round(
            emission_micro,
            fees_micro,
            &participants,
            &self.economics_params,
        )?;

        // Credit payouts to reward sink
        self.reward_sink.credit_round_payouts(round, &payouts)?;

        // Update chain state
        chain_state.update_after_round(
            round,
            emission_paid,
            self.calculate_state_root(round, &payouts),
            self.get_current_timestamp(),
        );

        // Settle rewards to accounts
        self.reward_sink.settle_to_accounts(self.account_ledger.as_mut())?;

        // Clear settled payouts
        self.reward_sink.clear_settled_payouts(round);

        let result = RoundExecutionResult {
            round,
            emission_micro: emission_paid,
            fees_collected_micro: fees_micro,
            total_participants: participants.len(),
            total_payouts: payouts.values().sum(),
            state_root: chain_state.state_root(),
        };

        info!(
            target: "round_executor",
            "Round {} executed: {} micro-IPN emitted, {} participants, {} total payouts",
            round,
            emission_paid,
            participants.len(),
            result.total_payouts
        );

        Ok(result)
    }

    /// Get current economics parameters
    pub fn get_economics_params(&self) -> &EconomicsParams {
        &self.economics_params
    }

    /// Update economics parameters (via governance)
    pub fn update_economics_params(&mut self, params: EconomicsParams) {
        self.economics_params = params;
        info!(target: "round_executor", "Economics parameters updated via governance");
    }

    /// Get reward sink for inspection
    pub fn get_reward_sink(&self) -> &RewardSink {
        &self.reward_sink
    }

    /// Get fee collector for inspection
    pub fn get_fee_collector(&self) -> &FeeCollector {
        &self.fee_collector
    }

    /// Get account ledger
    pub fn get_account_ledger(&self) -> &dyn AccountLedger {
        self.account_ledger.as_ref()
    }

    /// Calculate state root for the round
    fn calculate_state_root(&self, round: RoundId, payouts: &HashMap<[u8; 32], MicroIPN>) -> [u8; 32] {
        use blake3::Hasher as Blake3;
        
        let mut hasher = Blake3::new();
        hasher.update(&round.to_be_bytes());
        
        // Include payouts in state root calculation
        let mut payout_entries: Vec<_> = payouts.iter().collect();
        payout_entries.sort_by_key(|(vid, _)| *vid);
        
        for (validator_id, amount) in payout_entries {
            hasher.update(validator_id);
            hasher.update(&amount.to_be_bytes());
        }
        
        let digest = hasher.finalize();
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(digest.as_bytes());
        state_root
    }

    /// Get current timestamp
    fn get_current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Helper function to create participation set from validator data
pub fn create_participation_set(
    validators: &[(u64, [u8; 32], u64, f64)], // (stake, id, blocks_proposed, reputation)
    proposer_id: [u8; 32],
) -> ParticipationSet {
    let mut participants = Vec::new();
    
    for (stake, id, blocks_proposed, reputation) in validators {
        let role = if *id == proposer_id {
            Role::Proposer
        } else {
            Role::Verifier
        };
        
        participants.push(Participation {
            validator_id: *id,
            role,
            blocks_proposed: *blocks_proposed as u32,
            blocks_verified: 0, // Simplified for this example
            reputation_score: *reputation,
            stake_weight: *stake,
        });
    }
    
    participants
}

/// Helper function to create participation set with both proposer and verifier roles
pub fn create_full_participation_set(
    validators: &[(u64, [u8; 32], u32, u32, f64)], // (stake, id, blocks_proposed, blocks_verified, reputation)
    proposer_id: [u8; 32],
) -> ParticipationSet {
    let mut participants = Vec::new();
    
    for (stake, id, blocks_proposed, blocks_verified, reputation) in validators {
        let role = if *id == proposer_id {
            if *blocks_verified > 0 {
                Role::Both
            } else {
                Role::Proposer
            }
        } else {
            Role::Verifier
        };
        
        participants.push(Participation {
            validator_id: *id,
            role,
            blocks_proposed: *blocks_proposed,
            blocks_verified: *blocks_verified,
            reputation_score: *reputation,
            stake_weight: *stake,
        });
    }
    
    participants
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_treasury::MockAccountLedger;

    #[test]
    fn test_round_executor_creation() {
        let params = EconomicsParams::default();
        let account_ledger = Box::new(MockAccountLedger::new());
        let executor = RoundExecutor::new(params, account_ledger);
        
        assert_eq!(executor.get_economics_params().initial_round_reward_micro, 10_000_000);
    }

    #[test]
    fn test_create_participation_set() {
        let validators = vec![
            (1000, [1u8; 32], 1, 1.0),
            (2000, [2u8; 32], 0, 1.2),
        ];
        let proposer_id = [1u8; 32];
        
        let participants = create_participation_set(&validators, proposer_id);
        
        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].role, Role::Proposer);
        assert_eq!(participants[1].role, Role::Verifier);
    }

    #[test]
    fn test_create_full_participation_set() {
        let validators = vec![
            (1000, [1u8; 32], 1, 2, 1.0),
            (2000, [2u8; 32], 0, 3, 1.2),
        ];
        let proposer_id = [1u8; 32];
        
        let participants = create_full_participation_set(&validators, proposer_id);
        
        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].role, Role::Both);
        assert_eq!(participants[1].role, Role::Verifier);
    }

    #[test]
    fn test_round_execution() {
        let params = EconomicsParams::default();
        let account_ledger = Box::new(MockAccountLedger::new());
        let mut executor = RoundExecutor::new(params, account_ledger);
        
        let mut chain_state = ChainState::new();
        let participants = create_participation_set(
            &[(1000, [1u8; 32], 1, 1.0)],
            [1u8; 32],
        );
        
        let result = executor.execute_round(1, &mut chain_state, participants, 1000).unwrap();
        
        assert_eq!(result.round, 1);
        assert!(result.emission_micro > 0);
        assert_eq!(result.fees_collected_micro, 1000);
        assert_eq!(result.total_participants, 1);
    }
}