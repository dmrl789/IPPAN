//! Round execution and finalization with DAG-Fair emission integration
//!
//! Integrates deterministic emission, halving, supply-cap enforcement,
//! and fair validator reward distribution into the consensus layer.

use crate::fees::FeeCollector;
use ippan_economics::{
    distribute_round, emission_for_round_capped, EconomicsParams, Participation, ParticipationSet,
    Role, MICRO_PER_IPN,
};
use ippan_treasury::{RewardSink, AccountLedger};
use ippan_types::{ChainState, MicroIPN, RoundId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Result of a finalized consensus round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundExecutionResult {
    pub round: RoundId,
    pub emission_micro: MicroIPN,
    pub fees_collected_micro: MicroIPN,
    pub total_participants: usize,
    pub total_payouts: MicroIPN,
    pub state_root: [u8; 32],
}

/// RoundExecutor coordinates consensus, emission, and reward distribution.
pub struct RoundExecutor {
    economics_params: EconomicsParams,
    reward_sink: RewardSink,
    fee_collector: FeeCollector,
    account_ledger: Box<dyn AccountLedger>,
}

impl RoundExecutor {
    /// Create a new round executor.
    pub fn new(economics_params: EconomicsParams, account_ledger: Box<dyn AccountLedger>) -> Self {
        Self {
            economics_params,
            reward_sink: RewardSink::new(),
            fee_collector: FeeCollector::new(),
            account_ledger,
        }
    }

    /// Execute a single consensus round with DAG-Fair emission.
    pub fn execute_round(
        &mut self,
        round: RoundId,
        chain_state: &mut ChainState,
        participants: ParticipationSet,
        fees_micro: MicroIPN,
    ) -> Result<RoundExecutionResult> {
        // Collect transaction fees for this round
        self.fee_collector.collect_round_fees(round, fees_micro)?;

        // Calculate emission for this round (enforcing supply cap)
        let issued = chain_state.total_issued_micro();
        let emission_micro = emission_for_round_capped(round, issued, &self.economics_params)?;

        // Distribute rewards proportionally to participants
        let (payouts, emission_paid, fees_capped) =
            distribute_round(emission_micro, fees_micro, &participants, &self.economics_params)?;

        // Credit rewards to the treasury sink
        self.reward_sink.credit_round_payouts(round, &payouts)?;

        // Update chain state
        chain_state.update_after_round(
            round,
            emission_paid,
            self.calculate_state_root(round, &payouts),
            self.get_current_timestamp(),
        );

        // Apply settlements
        self.reward_sink
            .settle_to_accounts(self.account_ledger.as_mut())?;
        self.reward_sink.clear_settled_payouts(round);

        // Build execution result
        let result = RoundExecutionResult {
            round,
            emission_micro: emission_paid,
            fees_collected_micro: fees_capped,
            total_participants: participants.len(),
            total_payouts: payouts.values().sum(),
            state_root: chain_state.state_root(),
        };

        info!(
            target: "round_executor",
            "Round {} executed → emission={} μIPN (≈ {:.6} IPN), {} participants, total payouts={}",
            round,
            emission_paid,
            (emission_paid as f64) / (MICRO_PER_IPN as f64),
            participants.len(),
            result.total_payouts
        );

        Ok(result)
    }

    /// Get current economics parameters.
    pub fn get_economics_params(&self) -> &EconomicsParams {
        &self.economics_params
    }

    /// Update parameters via governance.
    pub fn update_economics_params(&mut self, params: EconomicsParams) {
        self.economics_params = params;
        info!(target: "round_executor", "Economics parameters updated via governance");
    }

    /// Internal: calculate deterministic round state root (includes payouts).
    fn calculate_state_root(&self, round: RoundId, payouts: &HashMap<[u8; 32], MicroIPN>) -> [u8; 32] {
        use blake3::Hasher as Blake3;
        let mut hasher = Blake3::new();
        hasher.update(&round.to_be_bytes());
        let mut entries: Vec<_> = payouts.iter().collect();
        entries.sort_by_key(|(vid, _)| *vid);
        for (vid, amt) in entries {
            hasher.update(vid);
            hasher.update(&amt.to_be_bytes());
        }
        let digest = hasher.finalize();
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(digest.as_bytes());
        state_root
    }

    /// Internal: current timestamp (seconds since UNIX epoch).
    fn get_current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Create participation set from validator tuples.
/// Format: `(stake, id, blocks_proposed, reputation)`
pub fn create_participation_set(
    validators: &[(u64, [u8; 32], u64, f64)],
    proposer_id: [u8; 32],
) -> ParticipationSet {
    let mut parts = Vec::new();
    for (stake, id, blocks_proposed, reputation) in validators {
        let role = if *id == proposer_id {
            Role::Proposer
        } else {
            Role::Verifier
        };
        parts.push(Participation {
            validator_id: *id,
            role,
            blocks_proposed: *blocks_proposed as u32,
            blocks_verified: 0,
            reputation_score: *reputation,
            stake_weight: *stake,
        });
    }
    parts
}

/// Create participation set including both proposer and verifier roles.
pub fn create_full_participation_set(
    validators: &[(u64, [u8; 32], u32, u32, f64)],
    proposer_id: [u8; 32],
) -> ParticipationSet {
    let mut parts = Vec::new();
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
        parts.push(Participation {
            validator_id: *id,
            role,
            blocks_proposed: *blocks_proposed,
            blocks_verified: *blocks_verified,
            reputation_score: *reputation,
            stake_weight: *stake,
        });
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_treasury::MockAccountLedger;

    #[test]
    fn test_round_executor_creation() {
        let params = EconomicsParams::default();
        let ledger = Box::new(MockAccountLedger::new());
        let executor = RoundExecutor::new(params, ledger);
        assert!(executor.get_economics_params().initial_round_reward_micro > 0);
    }

    #[test]
    fn test_create_participation_set() {
        let validators = vec![
            (1000, [1u8; 32], 1, 1.0),
            (2000, [2u8; 32], 0, 1.2),
        ];
        let proposer_id = [1u8; 32];
        let parts = create_participation_set(&validators, proposer_id);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].role, Role::Proposer);
        assert_eq!(parts[1].role, Role::Verifier);
    }

    #[test]
    fn test_create_full_participation_set() {
        let validators = vec![
            (1000, [1u8; 32], 1, 2, 1.0),
            (2000, [2u8; 32], 0, 3, 1.2),
        ];
        let proposer_id = [1u8; 32];
        let parts = create_full_participation_set(&validators, proposer_id);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].role, Role::Both);
        assert_eq!(parts[1].role, Role::Verifier);
    }

    #[test]
    fn test_round_execution() {
        let params = EconomicsParams::default();
        let ledger = Box::new(MockAccountLedger::new());
        let mut executor = RoundExecutor::new(params, ledger);
        let mut state = ChainState::new();
        let participants = create_participation_set(
            &[(1000, [1u8; 32], 1, 1.0)],
            [1u8; 32],
        );
        let result = executor.execute_round(1, &mut state, participants, 1000).unwrap();
        assert_eq!(result.round, 1);
        assert!(result.emission_micro > 0);
        assert!(result.total_participants > 0);
    }
}
