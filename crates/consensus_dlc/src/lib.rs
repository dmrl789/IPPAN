//! Deterministic Learning Consensus (DLC)
//!
//! A production-ready consensus engine combining HashTimer™, BlockDAG,
//! and Deterministic Gradient-Boosted Decision Trees (D-GBDT) for fair
//! and verifiable validator selection.
//!
//! # Overview
//!
//! DLC provides a comprehensive consensus mechanism with:
//! - **HashTimer**: Deterministic time-based ordering
//! - **BlockDAG**: Parallel block production with DAG structure
//! - **D-GBDT**: Fair validator selection using machine learning
//! - **Reputation**: Validator behavior tracking and scoring
//! - **Bonding**: Stake-based security with slashing
//! - **Emission**: Controlled token distribution and rewards
//!
//! # Example
//!
//! ```no_run
//! use ippan_consensus_dlc::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize consensus components
//!     let mut dag = dag::BlockDAG::new();
//!     let fairness_model = dgbdt::FairnessModel::new_production();
//!     
//!     // Process a consensus round
//!     let result = process_round(
//!         &mut dag,
//!         &fairness_model,
//!         1, // round number
//!     ).await?;
//!     
//!     println!("Processed round 1: {} blocks finalized", result.blocks_finalized);
//!     Ok(())
//! }
//! ```

pub mod bond;
pub mod dag;
pub mod dgbdt;
pub mod emission;
pub mod error;
pub mod hashtimer;
pub mod reputation;
pub mod verifier;

#[cfg(test)]
mod tests;

use bond::BondManager;
use dag::BlockDAG;
use dgbdt::FairnessModel;
use emission::{EmissionSchedule, RewardDistributor};
use error::{DlcError, Result};
use hashtimer::HashTimer;
use reputation::ReputationDB;
use verifier::{ValidatorSetManager, VerifiedBlock, VerifierSet};

use ippan_types::Amount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Initialize the DLC consensus engine
pub fn init_dlc() {
    tracing::info!("Initializing Deterministic Learning Consensus (DLC)...");
    tracing::info!("  - HashTimer: Deterministic time-based ordering");
    tracing::info!("  - BlockDAG: Parallel block production");
    tracing::info!("  - D-GBDT: Fair validator selection");
    tracing::info!("  - Reputation: Validator behavior tracking");
    tracing::info!("  - Bonding: Stake-based security");
    tracing::info!("  - Emission: Controlled token distribution");
}

/// DLC consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlcConfig {
    /// Number of validators to select per round
    pub validators_per_round: usize,
    /// Minimum stake required to be a validator
    pub min_validator_stake: Amount,
    /// Unstaking lock duration in rounds
    pub unstaking_lock_rounds: u64,
    /// Minimum reputation to participate
    pub min_reputation: i64,
    /// Enable slashing for malicious behavior
    pub enable_slashing: bool,
}

impl Default for DlcConfig {
    fn default() -> Self {
        Self {
            validators_per_round: 21,
            min_validator_stake: bond::MIN_VALIDATOR_BOND,
            unstaking_lock_rounds: 1_440, // ~1 day at 1 block/min
            min_reputation: 5000,
            enable_slashing: true,
        }
    }
}

/// DLC consensus state
pub struct DlcConsensus {
    /// Block DAG
    pub dag: BlockDAG,
    /// Validator set manager
    pub validators: ValidatorSetManager,
    /// Reputation database
    pub reputation: ReputationDB,
    /// Bond manager
    pub bonds: BondManager,
    /// Emission schedule
    pub emission: EmissionSchedule,
    /// Reward distributor
    pub rewards: RewardDistributor,
    /// Configuration
    pub config: DlcConfig,
    /// Current round number
    pub current_round: u64,
}

impl DlcConsensus {
    /// Create a new DLC consensus instance
    pub fn new(config: DlcConfig) -> Self {
        let model = FairnessModel::new_production();

        Self {
            dag: BlockDAG::new(),
            validators: ValidatorSetManager::new(model, config.validators_per_round),
            reputation: ReputationDB::default(),
            bonds: BondManager::new(config.unstaking_lock_rounds),
            emission: EmissionSchedule::default(),
            rewards: RewardDistributor::default(),
            config,
            current_round: 0,
        }
    }

    /// Register a new validator
    pub fn register_validator(
        &mut self,
        validator_id: String,
        stake: Amount,
        metrics: dgbdt::ValidatorMetrics,
    ) -> Result<()> {
        if stake < self.config.min_validator_stake {
            return Err(DlcError::InvalidBond(format!(
                "Stake {} is below minimum {}",
                stake, self.config.min_validator_stake
            )));
        }

        // Create bond
        self.bonds.create_bond(validator_id.clone(), stake)?;

        // Register in validator set
        self.validators
            .register_validator(validator_id.clone(), metrics)?;

        // Initialize reputation
        self.reputation.initialize_validator(validator_id.clone())?;

        tracing::info!("Registered validator {} with stake {}", validator_id, stake);

        Ok(())
    }

    /// Process a consensus round
    pub async fn process_round(&mut self) -> Result<RoundResult> {
        self.current_round += 1;
        let round_time = HashTimer::for_round(self.current_round);

        // Select verifiers for this round
        let seed = round_time.hash.clone();
        let verifier_set = self.validators.select_for_round(seed, self.current_round)?;

        tracing::debug!(
            "Round {}: Selected {} verifiers, primary: {}",
            self.current_round,
            verifier_set.size(),
            verifier_set.primary
        );

        // Collect and verify blocks
        let pending = self.dag.pending();
        let blocks_to_process = verifier_set.collect_blocks(pending);

        let mut verified_blocks = Vec::new();
        for block in blocks_to_process {
            match verifier_set.validate(&block) {
                Ok(()) => {
                    let block_clone = block.clone();
                    let verified =
                        VerifiedBlock::new(block_clone.clone(), verifier_set.all_verifiers());
                    verified_blocks.push(verified);

                    // Insert into DAG if we have not already ingested this block
                    if !self.dag.blocks.contains_key(&block_clone.id) {
                        self.dag.insert(block_clone)?;
                    }
                }
                Err(e) => {
                    tracing::warn!("Block validation failed: {}", e);
                    // Penalize proposer
                    if self.config.enable_slashing {
                        let _ = self
                            .reputation
                            .penalize_invalid_proposal(&block.proposer, self.current_round);
                    }
                }
            }
        }

        // Finalize blocks
        let finalized_ids = self.dag.finalize_round(round_time.clone());

        // Calculate and distribute rewards
        let block_reward = self.emission.calculate_block_reward(self.current_round);

        if !verified_blocks.is_empty() && block_reward > 0 {
            for verified in &verified_blocks {
                let distribution = self.rewards.distribute_block_reward(
                    block_reward,
                    &verified.block.proposer,
                    &verified.verified_by,
                )?;

                // Update reputation for participants
                let _ = self
                    .reputation
                    .reward_proposal(&verified.block.proposer, self.current_round);

                for verifier in &verified.verified_by {
                    let _ = self
                        .reputation
                        .reward_verification(verifier, self.current_round);
                }

                tracing::debug!("Distributed rewards: {:?}", distribution);
            }
        }

        // Update emission schedule
        self.emission
            .update(self.current_round, verified_blocks.len() as u64)?;

        Ok(RoundResult {
            round: self.current_round,
            blocks_processed: verified_blocks.len(),
            blocks_finalized: finalized_ids.len(),
            verifiers: verifier_set.size(),
            block_reward,
        })
    }

    /// Get consensus statistics
    pub fn stats(&self) -> ConsensusStats {
        ConsensusStats {
            current_round: self.current_round,
            dag_stats: self.dag.stats(),
            reputation_stats: self.reputation.stats(),
            bond_stats: self.bonds.stats(),
            emission_stats: self.emission.stats(),
            reward_stats: self.rewards.stats(),
        }
    }
}

/// Result of processing a consensus round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundResult {
    pub round: u64,
    pub blocks_processed: usize,
    pub blocks_finalized: usize,
    pub verifiers: usize,
    pub block_reward: u64,
}

/// Comprehensive consensus statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    pub current_round: u64,
    pub dag_stats: dag::DagStats,
    pub reputation_stats: reputation::ReputationStats,
    pub bond_stats: bond::BondStats,
    pub emission_stats: emission::EmissionStats,
    pub reward_stats: emission::DistributorStats,
}

/// Process one consensus round (simplified interface)
pub async fn process_round(
    dag: &mut BlockDAG,
    fairness: &FairnessModel,
    round: u64,
) -> Result<RoundResult> {
    let round_time = HashTimer::for_round(round);

    // Create a minimal validator set for demonstration
    let mut validators = HashMap::new();
    validators.insert("validator1".to_string(), dgbdt::ValidatorMetrics::default());

    let verifier_set = VerifierSet::select(
        fairness,
        &validators,
        round_time.hash.clone(),
        round,
        validators.len(),
    )?;

    let pending = dag.pending();
    let blocks = verifier_set.collect_blocks(pending);

    let mut processed = 0;
    for block in blocks {
        if verifier_set.validate(&block).is_ok() {
            dag.insert(block)?;
            processed += 1;
        }
    }

    let finalized_ids = dag.finalize_round(round_time);

    tracing::info!("DLC round {} finalized", round);

    Ok(RoundResult {
        round,
        blocks_processed: processed,
        blocks_finalized: finalized_ids.len(),
        verifiers: verifier_set.size(),
        block_reward: emission::BLOCK_REWARD,
    })
}
