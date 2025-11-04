//! Round execution and finalization with DAG-Fair emission integration
//!
//! Integrates deterministic emission, halving, supply-cap enforcement,
//! and fair validator reward distribution into the consensus layer.

use crate::fees::FeeCollector;
use anyhow::Result;
use ippan_economics::{
    EmissionEngine, EmissionParams, Payouts, RoundRewards, ValidatorId, ValidatorParticipation,
    ValidatorRole,
};
use ippan_treasury::{AccountLedger, RewardSink};
use ippan_types::{ChainState, MicroIPN, RoundId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Participation data for a validator in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participation {
    pub role: ValidatorRole,
    pub blocks: u32,
}

/// Participation set mapping validator ID to participation data
pub type ParticipationSet = HashMap<ValidatorId, Participation>;

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
    emission_engine: EmissionEngine,
    round_rewards: RoundRewards,
    reward_sink: RewardSink,
    fee_collector: FeeCollector,
    account_ledger: Box<dyn AccountLedger>,
}

impl RoundExecutor {
    /// Create a new round executor.
    pub fn new(economics_params: EmissionParams, account_ledger: Box<dyn AccountLedger>) -> Self {
        let emission_engine = EmissionEngine::with_params(economics_params.clone());
        let round_rewards = RoundRewards::new(economics_params);
        Self {
            emission_engine,
            round_rewards,
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
        // Collect fees into economics-aware collector: convert Amount from types
        self.fee_collector
            .collect(ippan_types::Amount::from_micro_ipn(fees_micro as u64));

        // Calculate emission for this round (enforcing supply cap)
        let _issued = chain_state.total_issued_micro();
        let emission_micro = self.emission_engine.calculate_round_reward(round)?;

        // Convert participants to ValidatorParticipation format
        let participations =
            self.convert_participants_to_validator_participations(participants.clone())?;

        // Distribute rewards proportionally to participants
        let distribution = self.round_rewards.distribute_round_rewards(
            round,
            emission_micro,
            participations,
            fees_micro as u64,
        )?;

        // Convert distribution to payouts format
        let payouts = self.convert_distribution_to_payouts(&distribution)?;

        // Convert Payouts to the expected format for treasury
        let treasury_payouts: HashMap<[u8; 32], u128> = payouts
            .iter()
            .filter_map(|(validator_id, amount)| {
                let id_bytes = validator_id.0.as_bytes();
                if id_bytes.len() >= 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&id_bytes[..32]);
                    Some((key, *amount))
                } else {
                    None
                }
            })
            .collect();

        // Credit rewards to the treasury sink
        self.reward_sink
            .credit_round_payouts(round, &treasury_payouts)?;

        // Update chain state
        chain_state.update_after_round(
            round,
            distribution.total_reward as u128,
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
            emission_micro: distribution.total_reward as u128,
            fees_collected_micro: distribution.fees_collected as u128,
            total_participants: participants.len(),
            total_payouts: payouts.values().sum(),
            state_root: chain_state.state_root(),
        };

        info!(
            target: "round_executor",
            "Round {} executed → emission={} μIPN (≈ {:.6} IPN), {} participants, total payouts={}",
            round,
            result.emission_micro,
            (result.emission_micro as f64) / 1_000_000.0, // Convert to IPN
            participants.len(),
            result.total_payouts
        );

        Ok(result)
    }

    /// Get current economics parameters.
    pub fn get_economics_params(&self) -> EmissionParams {
        // Note: params field is private, so we return a copy
        // This should be fixed by making params public or adding a getter method
        EmissionParams::default() // Placeholder
    }

    /// Update parameters via governance.
    pub fn update_economics_params(&mut self, params: EmissionParams) {
        self.emission_engine = EmissionEngine::with_params(params.clone());
        self.round_rewards = RoundRewards::new(params);
        info!(target: "round_executor", "Economics parameters updated via governance");
    }

    /// Internal: calculate deterministic round state root (includes payouts).
    fn calculate_state_root(&self, round: RoundId, payouts: &Payouts) -> [u8; 32] {
        use blake3::Hasher as Blake3;
        let mut hasher = Blake3::new();
        hasher.update(&round.to_be_bytes());
        let mut entries: Vec<_> = payouts.iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.0.cmp(&b.0));
        for (vid, amt) in entries {
            hasher.update(vid.0.as_bytes());
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
    let mut parts: ParticipationSet = HashMap::new();
    for (_stake, id, blocks_proposed, _reputation) in validators {
        let role = if *id == proposer_id {
            ValidatorRole::Proposer
        } else {
            ValidatorRole::Verifier
        };
        let vid = ValidatorId(hex::encode(id));
        parts.insert(
            vid,
            Participation {
                role,
                blocks: *blocks_proposed as u32,
            },
        );
    }
    parts
}

/// Create participation set including both proposer and verifier roles.
pub fn create_full_participation_set(
    validators: &[(u64, [u8; 32], u32, u32, f64)],
    proposer_id: [u8; 32],
) -> ParticipationSet {
    let mut parts: ParticipationSet = HashMap::new();
    for (_stake, id, blocks_proposed, blocks_verified, _reputation) in validators {
        // Economics Role enum does not support Both; choose Proposer if proposer else Verifier
        let role = if *id == proposer_id {
            ValidatorRole::Proposer
        } else {
            ValidatorRole::Verifier
        };
        let total_blocks = (*blocks_proposed as u64 + *blocks_verified as u64) as u32;
        let vid = ValidatorId(hex::encode(id));
        parts.insert(
            vid,
            Participation {
                role,
                blocks: total_blocks,
            },
        );
    }
    parts
}

impl RoundExecutor {
    /// Convert ParticipationSet to Vec<ValidatorParticipation>
    fn convert_participants_to_validator_participations(
        &self,
        participants: ParticipationSet,
    ) -> Result<Vec<ValidatorParticipation>> {
        let mut participations = Vec::new();
        for (validator_id, participation) in participants {
            let participation = ValidatorParticipation {
                validator_id,
                role: participation.role,
                blocks_contributed: participation.blocks,
                uptime_score: rust_decimal::Decimal::new(100, 2), // Default to 1.0
            };
            participations.push(participation);
        }
        Ok(participations)
    }

    /// Convert RoundRewardDistribution to Payouts
    fn convert_distribution_to_payouts(
        &self,
        distribution: &ippan_economics::RoundRewardDistribution,
    ) -> Result<Payouts> {
        let mut payouts = HashMap::new();
        for (validator_id, reward) in &distribution.validator_rewards {
            payouts.insert(validator_id.clone(), reward.total_reward as u128);
        }
        Ok(payouts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_treasury::MockAccountLedger;

    #[test]
    fn test_round_executor_creation() {
        let params = EmissionParams::default();
        let ledger = Box::new(MockAccountLedger::new());
        let executor = RoundExecutor::new(params, ledger);
        assert!(executor.emission_engine.params().initial_round_reward_micro > 0);
    }

    #[test]
    fn test_create_participation_set() {
        let validators = vec![(1000, [1u8; 32], 1, 1.0), (2000, [2u8; 32], 0, 1.2)];
        let proposer_id = [1u8; 32];
        let parts = create_participation_set(&validators, proposer_id);
        assert_eq!(parts.len(), 2);
        // Check that the proposer exists in the map
        let proposer_vid = ValidatorId(hex::encode([1u8; 32]));
        assert!(parts.contains_key(&proposer_vid));
        assert!(matches!(parts.get(&proposer_vid).unwrap().role, ValidatorRole::Proposer));
    }

    #[test]
    fn test_create_full_participation_set() {
        let validators = vec![(1000, [1u8; 32], 1, 2, 1.0), (2000, [2u8; 32], 0, 3, 1.2)];
        let proposer_id = [1u8; 32];
        let parts = create_full_participation_set(&validators, proposer_id);
        assert_eq!(parts.len(), 2);
        // Check that the proposer exists in the map
        let proposer_vid = ValidatorId(hex::encode([1u8; 32]));
        assert!(parts.contains_key(&proposer_vid));
        assert!(matches!(parts.get(&proposer_vid).unwrap().role, ValidatorRole::Proposer));
    }

    #[test]
    fn test_round_execution() {
        let params = EmissionParams::default();
        let ledger = Box::new(MockAccountLedger::new());
        let mut executor = RoundExecutor::new(params, ledger);
        let mut state = ChainState::new();
        let participants = create_participation_set(&[(1000, [1u8; 32], 1, 1.0)], [1u8; 32]);
        let result = executor
            .execute_round(1, &mut state, participants, 1000)
            .unwrap();
        assert_eq!(result.round, 1);
        assert!(result.emission_micro > 0);
        assert!(result.total_participants > 0);
    }
}
