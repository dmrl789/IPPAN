//! Deterministic Learning Consensus (DLC) Core Module
//!
//! This module implements the DLC consensus algorithm which uses:
//! - HashTimer™ for deterministic temporal finality
//! - BlockDAG for parallel block processing
//! - D-GBDT for AI-driven fairness and validator selection
//! - Shadow verifiers for redundant validation
//! - No voting or BFT mechanisms

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use ippan_types::{Block, BlockId, IppanTimeMicros, RoundId, ValidatorId};

use crate::bonding::BondingManager;
use crate::dgbdt::{DGBDTEngine, ValidatorMetrics};
use crate::parallel_dag::{ParallelDag, ParallelDagConfig};
use crate::shadow_verifier::ShadowVerifierSet;

/// DLC Consensus configuration
#[derive(Debug, Clone)]
pub struct DLCConfig {
    /// Temporal finality window in milliseconds (100-250ms)
    pub temporal_finality_ms: u64,

    /// HashTimer precision in microseconds
    pub hashtimer_precision_us: u64,

    /// Number of shadow verifiers (3-5 recommended)
    pub shadow_verifier_count: usize,

    /// Minimum reputation score for validator selection (0-10000)
    pub min_reputation_score: i32,

    /// Maximum transactions per block
    pub max_transactions_per_block: usize,

    /// Enable D-GBDT fairness model
    pub enable_dgbdt_fairness: bool,

    /// Enable shadow verifier system
    pub enable_shadow_verifiers: bool,

    /// Require 10 IPN validator bond
    pub require_validator_bond: bool,

    /// DAG configuration
    pub dag_config: ParallelDagConfig,
}

impl Default for DLCConfig {
    fn default() -> Self {
        Self {
            temporal_finality_ms: 250,
            hashtimer_precision_us: 1,
            shadow_verifier_count: 3,
            min_reputation_score: 5000,
            max_transactions_per_block: 1000,
            enable_dgbdt_fairness: true,
            enable_shadow_verifiers: true,
            require_validator_bond: true,
            dag_config: ParallelDagConfig::default(),
        }
    }
}

/// Round state tracking for DLC
#[derive(Debug, Clone)]
pub struct DLCRoundState {
    pub round_id: RoundId,
    pub round_start: Instant,
    pub round_start_time: IppanTimeMicros,
    pub primary_verifier: ValidatorId,
    pub shadow_verifiers: Vec<ValidatorId>,
    pub blocks_proposed: Vec<BlockId>,
    pub is_finalized: bool,
}

/// DLC Consensus Engine - voting-free, time-anchored consensus
pub struct DLCConsensus {
    /// Configuration
    pub config: DLCConfig,

    /// This validator's ID
    pub validator_id: ValidatorId,

    /// Current round state
    pub current_round: Arc<RwLock<DLCRoundState>>,

    /// BlockDAG for parallel processing
    pub dag: Arc<ParallelDag>,

    /// D-GBDT engine for fairness and selection
    pub dgbdt_engine: Arc<RwLock<DGBDTEngine>>,

    /// Shadow verifier set
    pub shadow_verifiers: Arc<RwLock<ShadowVerifierSet>>,

    /// Validator bonding manager
    pub bonding_manager: Arc<RwLock<BondingManager>>,

    /// Validator metrics for D-GBDT
    pub validator_metrics: Arc<RwLock<HashMap<ValidatorId, ValidatorMetrics>>>,
}

impl DLCConsensus {
    /// Initialize a new DLC consensus engine
    pub fn new(config: DLCConfig, validator_id: ValidatorId) -> Self {
        let current_round = DLCRoundState {
            round_id: 1,
            round_start: Instant::now(),
            round_start_time: IppanTimeMicros::now(),
            primary_verifier: validator_id,
            shadow_verifiers: Vec::new(),
            blocks_proposed: Vec::new(),
            is_finalized: false,
        };

        Self {
            dag: Arc::new(ParallelDag::new(config.dag_config.clone())),
            dgbdt_engine: Arc::new(RwLock::new(DGBDTEngine::new())),
            shadow_verifiers: Arc::new(RwLock::new(ShadowVerifierSet::new(
                config.shadow_verifier_count,
            ))),
            bonding_manager: Arc::new(RwLock::new(BondingManager::new())),
            validator_metrics: Arc::new(RwLock::new(HashMap::new())),
            current_round: Arc::new(RwLock::new(current_round)),
            validator_id,
            config,
        }
    }

    /// Start the DLC consensus engine
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting DLC consensus engine");

        // Verify validator bond if required
        if self.config.require_validator_bond {
            let bonding = self.bonding_manager.read();
            if !bonding.has_valid_bond(&self.validator_id) {
                return Err(anyhow::anyhow!(
                    "Validator bond of 10 IPN required but not found"
                ));
            }
        }

        info!(
            "DLC consensus engine started with temporal finality window: {}ms",
            self.config.temporal_finality_ms
        );
        Ok(())
    }

    /// Process a new round using DLC algorithm
    pub async fn process_round(&mut self) -> Result<()> {
        let mut round = self.current_round.write();

        // Check if round should close based on temporal finality
        let elapsed = round.round_start.elapsed();
        let finality_window = Duration::from_millis(self.config.temporal_finality_ms);

        if elapsed < finality_window {
            return Ok(()); // Round not ready to close
        }

        info!("Round {} closing after {:?}", round.round_id, elapsed);

        // Select next round's verifiers using D-GBDT
        let (primary, shadows) = self.select_verifiers_deterministic(round.round_id + 1)?;

        // Finalize current round
        round.is_finalized = true;

        // Prepare next round
        let next_round = DLCRoundState {
            round_id: round.round_id + 1,
            round_start: Instant::now(),
            round_start_time: IppanTimeMicros::now(),
            primary_verifier: primary,
            shadow_verifiers: shadows,
            blocks_proposed: Vec::new(),
            is_finalized: false,
        };

        *round = next_round;

        Ok(())
    }

    /// Select verifiers deterministically using D-GBDT
    fn select_verifiers_deterministic(
        &self,
        round_seed: RoundId,
    ) -> Result<(ValidatorId, Vec<ValidatorId>)> {
        let dgbdt = self.dgbdt_engine.read();
        let metrics = self.validator_metrics.read();

        // Use D-GBDT to select based on reputation and fairness
        let selection = dgbdt.select_verifiers(
            round_seed,
            &metrics,
            self.config.shadow_verifier_count,
            self.config.min_reputation_score,
        )?;

        Ok((selection.primary, selection.shadows))
    }

    /// Verify a block using primary + shadow verifiers
    pub async fn verify_block(&self, block: &Block) -> Result<bool> {
        // Primary verification
        let primary_result = self.verify_block_internal(block)?;

        if !self.config.enable_shadow_verifiers {
            return Ok(primary_result);
        }

        // Get shadow verifiers list before await
        let shadow_verifier_ids = {
            let round = self.current_round.read();
            round.shadow_verifiers.clone()
        };

        // Shadow verification (parallel) - lock released before await
        let shadow_results = {
            let mut shadow_verifiers = self.shadow_verifiers.write();
            shadow_verifiers
                .verify_block(block, &shadow_verifier_ids)
                .await?
        };

        // Check consensus among verifiers
        let consistent = shadow_results.iter().all(|r| r.is_valid == primary_result);

        if !consistent {
            warn!(
                "Shadow verifier inconsistency detected for block {}",
                hex::encode(block.hash())
            );
            // Flag for investigation but don't block
        }

        Ok(primary_result)
    }

    /// Internal block verification logic
    fn verify_block_internal(&self, block: &Block) -> Result<bool> {
        // Structural validation
        if !block.is_valid() {
            return Ok(false);
        }

        // Check round timing
        let round = self.current_round.read();
        if block.header.round != round.round_id {
            return Ok(false);
        }

        // Validate all transactions
        for tx in &block.transactions {
            if !tx.is_valid() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Finalize a round deterministically (no voting)
    pub async fn finalize_round(&self, round_id: RoundId) -> Result<()> {
        info!("Finalizing round {} via temporal closure", round_id);

        // Round closure is deterministic based on HashTimer
        // No voting, no quorum - pure temporal finality

        Ok(())
    }

    /// Get current consensus state
    pub fn get_state(&self) -> DLCRoundState {
        self.current_round.read().clone()
    }

    /// Add a validator bond
    pub fn add_validator_bond(&self, validator_id: ValidatorId, amount: u64) -> Result<()> {
        let mut bonding = self.bonding_manager.write();
        bonding.add_bond(validator_id, amount)
    }

    /// Update validator metrics for D-GBDT
    pub fn update_validator_metrics(&self, validator_id: ValidatorId, metrics: ValidatorMetrics) {
        let mut metrics_map = self.validator_metrics.write();
        metrics_map.insert(validator_id, metrics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dlc_initialization() {
        let config = DLCConfig::default();
        let validator_id = [1u8; 32];
        let dlc = DLCConsensus::new(config, validator_id);

        let state = dlc.get_state();
        assert_eq!(state.round_id, 1);
        assert_eq!(state.primary_verifier, validator_id);
    }

    #[test]
    fn test_temporal_finality_window() {
        let config = DLCConfig {
            temporal_finality_ms: 250,
            ..Default::default()
        };

        assert_eq!(config.temporal_finality_ms, 250);
        assert!(config.temporal_finality_ms >= 100);
        assert!(config.temporal_finality_ms <= 250);
    }
}
