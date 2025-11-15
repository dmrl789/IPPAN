//! IPPAN Consensus Engine — Deterministic Learning Consensus (DLC)
//!
//! Implements DLC: time-anchored, AI-driven, voting-free consensus using:
//! - HashTimer™ for deterministic temporal finality (100-250ms windows)
//! - BlockDAG for parallel block processing and ordering
//! - D-GBDT for AI-driven fairness and validator selection
//! - Shadow verifiers (3-5) for redundant validation
//! - 10 IPN validator bonding mechanism
//! - No BFT, no voting, no quorums — pure deterministic consensus

use anyhow::Result;
use blake3::Hasher as Blake3;
use ippan_crypto::{validate_confidential_block, validate_confidential_transaction};
use ippan_l1_fees::FeePolicy;
use ippan_mempool::Mempool;
use ippan_storage::Storage;
use ippan_types::{
    Block, BlockId, IppanTimeMicros, RoundCertificate, RoundFinalizationRecord, RoundId,
    RoundWindow, Transaction,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::mpsc,
    time::{interval, sleep},
};
use tracing::{error, info, warn};

// ---------------------------------------------------------------------
// Submodules
// ---------------------------------------------------------------------

// Core DLC modules
pub mod bonding;
pub mod dgbdt;
pub mod dlc;
pub mod dlc_integration;
pub mod hashtimer_integration;
pub mod payments;
pub mod shadow_verifier;

// Economic and emission modules
pub mod emission;
pub mod emission_tracker;
pub mod fees;

// AI and selection modules
// Legacy l1_ai_consensus removed - using DLC fairness model instead

// Telemetry and metrics
pub mod metrics;
pub mod model_reload;
pub mod reputation;
pub mod telemetry;

// DAG and ordering
pub mod ordering;
pub mod parallel_dag;

// Round management
pub mod round;
pub mod round_executor;

// ---------------------------------------------------------------------
// Public re-exports
// ---------------------------------------------------------------------

// DLC Core
pub use bonding::{BondingManager, ValidatorBond, MIN_BOND_AMOUNT, VALIDATOR_BOND_AMOUNT};
pub use dgbdt::{DGBDTEngine, ValidatorMetrics, VerifierSelection};
pub use dlc::{DLCConfig, DLCConsensus, DLCRoundState};
pub use dlc_integration::{dlc_config_from_poa, DLCIntegratedConsensus};
pub use hashtimer_integration::{
    derive_selection_seed, generate_block_hashtimer, generate_round_hashtimer, should_close_round,
    verify_temporal_ordering,
};
pub use shadow_verifier::{ShadowVerifier, ShadowVerifierSet, VerificationResult};

// Emissions and fees
pub use emission::{
    calculate_fee_recycling, calculate_round_emission, calculate_round_reward,
    distribute_dag_fair_rewards, distribute_round_reward, projected_supply, round_reward,
    rounds_until_cap, DAGEmissionParams, FeeRecyclingParams, RoundDistribution, RoundEmission,
    ValidatorContribution, ValidatorParticipation, ValidatorReward, ValidatorRole,
};
pub use emission_tracker::{
    EmissionStatistics, EmissionTracker, ValidatorContribution as TrackerValidatorContribution,
};
pub use fees::{classify_transaction, validate_fee, FeeCapConfig, FeeCollector, FeeError, TxKind};
pub use ippan_economics::{EmissionEngine, EmissionParams, RewardAmount, RoundIndex, RoundRewards};
pub use ordering::order_round;
pub use parallel_dag::{
    DagError, DagSnapshot, InsertionOutcome, ParallelDag, ParallelDagConfig, ParallelDagEngine,
    ValidationResult,
};
pub use reputation::{
    apply_reputation_weight, calculate_reputation, ReputationScore, ValidatorTelemetry,
    DEFAULT_REPUTATION,
};
use round::RoundConsensus;
pub use round_executor::{
    create_full_participation_set, create_participation_set, RoundExecutionResult, RoundExecutor,
};

// ---------------------------------------------------------------------
// Errors and Configs
// ---------------------------------------------------------------------

#[derive(thiserror::Error, Debug)]
pub enum ConsensusError {
    #[error("Consensus engine not running")]
    NotRunning,
    #[error("Invalid proposer for current slot")]
    InvalidProposer,
    #[error("Block validation failed")]
    InvalidBlock,
    #[error("Storage error: {0}")]
    Storage(#[from] anyhow::Error),
}

/// Validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub id: [u8; 32],
    pub address: [u8; 32],
    pub stake: u64,
    pub is_active: bool,
}

/// Proof-of-Authority consensus configuration
#[derive(Debug, Clone)]
pub struct PoAConfig {
    pub slot_duration_ms: u64,
    pub validators: Vec<Validator>,
    pub max_transactions_per_block: usize,
    pub block_reward: u64,
    pub finalization_interval_ms: u64,
    pub enable_ai_reputation: bool,
    pub enable_fee_caps: bool,
    pub enable_dag_fair_emission: bool,
}

impl Default for PoAConfig {
    fn default() -> Self {
        Self {
            slot_duration_ms: 100,
            validators: vec![],
            max_transactions_per_block: 1000,
            block_reward: 10,
            finalization_interval_ms: 200,
            enable_ai_reputation: false,
            enable_fee_caps: true,
            enable_dag_fair_emission: true,
        }
    }
}

/// Snapshot of consensus state
#[derive(Debug, Clone)]
pub struct ConsensusState {
    pub current_slot: u64,
    pub current_proposer: Option<[u8; 32]>,
    pub is_proposing: bool,
    pub validator_count: usize,
    pub latest_block_height: u64,
    pub current_round: RoundId,
}

#[derive(Debug)]
pub struct RoundTracker {
    current_round: RoundId,
    round_start: Instant,
    round_start_time: IppanTimeMicros,
    previous_round_blocks: Vec<BlockId>,
    current_round_blocks: Vec<BlockId>,
}

// ---------------------------------------------------------------------
// Main Consensus Engine
// ---------------------------------------------------------------------

pub struct PoAConsensus {
    pub config: PoAConfig,
    pub storage: Arc<dyn Storage + Send + Sync>,
    pub validator_id: [u8; 32],
    pub is_running: Arc<RwLock<bool>>,
    pub current_slot: Arc<RwLock<u64>>,
    pub tx_sender: mpsc::UnboundedSender<Transaction>,
    pub mempool: Arc<Mempool>,
    pub round_tracker: Arc<RwLock<RoundTracker>>,
    pub finalization_interval: Duration,
    pub round_consensus: Arc<RwLock<RoundConsensus>>,
    pub fee_collector: Arc<RwLock<FeeCollector>>,
    pub dgbdt_engine: Arc<RwLock<DGBDTEngine>>,
    pub emission_tracker: Arc<RwLock<EmissionTracker>>,
    pub telemetry_manager: Arc<telemetry::TelemetryManager>,
    pub model_reloader: Option<Arc<model_reload::ModelReloader>>,
    pub metrics: Arc<metrics::ConsensusMetrics>,
    pub payment_engine: Arc<payments::PaymentApplier>,
}

impl PoAConsensus {
    /// Create a new PoA consensus engine
    pub fn new(
        config: PoAConfig,
        storage: Arc<dyn Storage + Send + Sync>,
        validator_id: [u8; 32],
    ) -> Self {
        let (tx_sender, _rx) = mpsc::unbounded_channel();
        let latest_height = storage.get_latest_height().unwrap_or(0);

        let previous_round_blocks = if latest_height == 0 {
            vec![]
        } else if let Ok(Some(block)) = storage.get_block_by_height(latest_height) {
            vec![block.hash()]
        } else {
            vec![]
        };

        let tracker = RoundTracker {
            current_round: latest_height.saturating_add(1),
            round_start: Instant::now(),
            round_start_time: IppanTimeMicros::now(),
            previous_round_blocks,
            current_round_blocks: Vec::new(),
        };

        let emission_params = ippan_economics::EmissionParams::default();
        let audit_interval = 6_048_000; // ~1 week at 100ms
        let emission_tracker = EmissionTracker::new(emission_params, audit_interval);

        let dgbdt_engine = DGBDTEngine::new();

        let telemetry_manager = Arc::new(telemetry::TelemetryManager::new(storage.clone()));
        // Load existing telemetry from storage
        let _ = telemetry_manager.load_from_storage();

        let metrics = Arc::new(metrics::ConsensusMetrics::new());

        Self {
            config: config.clone(),
            storage: storage.clone(),
            validator_id,
            is_running: Arc::new(RwLock::new(false)),
            current_slot: Arc::new(RwLock::new(0)),
            tx_sender,
            mempool: Arc::new(Mempool::new(10_000)),
            round_tracker: Arc::new(RwLock::new(tracker)),
            finalization_interval: Duration::from_millis(
                config.finalization_interval_ms.clamp(100, 250),
            ),
            round_consensus: Arc::new(RwLock::new(RoundConsensus::new())),
            fee_collector: Arc::new(RwLock::new(FeeCollector::new())),
            dgbdt_engine: Arc::new(RwLock::new(dgbdt_engine)),
            emission_tracker: Arc::new(RwLock::new(emission_tracker)),
            telemetry_manager,
            model_reloader: None,
            metrics,
            payment_engine: Arc::new(payments::PaymentApplier::new(
                FeePolicy::default(),
                payments::TREASURY_ACCOUNT,
            )),
        }
    }

    pub fn get_tx_sender(&self) -> mpsc::UnboundedSender<Transaction> {
        self.tx_sender.clone()
    }

    pub fn mempool(&self) -> Arc<Mempool> {
        self.mempool.clone()
    }

    // -----------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------

    pub async fn start(&mut self) -> Result<()> {
        *self.is_running.write() = true;
        info!("Starting PoA consensus engine");

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        *self.current_slot.write() = now / self.config.slot_duration_ms;

        let (
            is_running,
            current_slot,
            config,
            storage,
            mempool,
            round_tracker,
            round_consensus,
            finalization_interval,
            validator_id,
            fee_collector,
            telemetry_manager,
            metrics,
            dgbdt_engine,
            payment_engine,
        ) = (
            self.is_running.clone(),
            self.current_slot.clone(),
            self.config.clone(),
            self.storage.clone(),
            self.mempool.clone(),
            self.round_tracker.clone(),
            self.round_consensus.clone(),
            self.finalization_interval,
            self.validator_id,
            self.fee_collector.clone(),
            self.telemetry_manager.clone(),
            self.metrics.clone(),
            self.dgbdt_engine.clone(),
            self.payment_engine.clone(),
        );

        let mut ticker = interval(Duration::from_millis(config.slot_duration_ms));
        tokio::spawn(async move {
            loop {
                if !*is_running.read() {
                    break;
                }

                if let Err(e) = Self::finalize_round_if_ready(
                    &storage,
                    &round_tracker,
                    finalization_interval,
                    &config,
                    &fee_collector,
                    &payment_engine,
                    &metrics,
                ) {
                    error!("Round finalization error: {e}");
                }

                let slot = *current_slot.read();
                let maybe_proposer = Self::select_proposer(
                    &config,
                    &round_consensus,
                    slot,
                    &dgbdt_engine,
                    &telemetry_manager,
                    &metrics,
                );

                if let Some(proposer) = maybe_proposer {
                    if proposer == validator_id {
                        if let Err(e) = Self::propose_block(
                            &storage,
                            &mempool,
                            &config,
                            &round_tracker,
                            slot,
                            validator_id,
                            &telemetry_manager,
                            &metrics,
                        )
                        .await
                        {
                            error!("Block proposal failed: {e}");
                        }
                    }
                }

                *current_slot.write() = slot + 1;
                ticker.tick().await;
            }
        });

        info!("PoA consensus engine started");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        *self.is_running.write() = false;
        sleep(Duration::from_millis(100)).await;
        info!("PoA consensus engine stopped");
        Ok(())
    }

    // -----------------------------------------------------------------
    // Proposer selection
    // -----------------------------------------------------------------

    fn select_proposer(
        config: &PoAConfig,
        _round_consensus: &Arc<RwLock<RoundConsensus>>,
        slot: u64,
        dgbdt_engine: &Arc<RwLock<DGBDTEngine>>,
        telemetry_manager: &Arc<telemetry::TelemetryManager>,
        metrics: &Arc<metrics::ConsensusMetrics>,
    ) -> Option<[u8; 32]> {
        let active: Vec<_> = config
            .validators
            .iter()
            .filter(|v| v.is_active)
            .map(|v| v.id)
            .collect();
        if active.is_empty() {
            return None;
        }

        if config.enable_ai_reputation {
            metrics.record_ai_selection_attempt();
            let start = Instant::now();

            // Build stake map for telemetry defaults
            let stakes: HashMap<[u8; 32], u64> =
                config.validators.iter().map(|v| (v.id, v.stake)).collect();

            // Load real telemetry from storage (with defaults for missing validators)
            let all_telemetry = telemetry_manager.get_all_telemetry_with_defaults(&active, &stakes);

            // Convert telemetry to ValidatorMetrics for DGBDT
            let mut validator_metrics: HashMap<[u8; 32], dgbdt::ValidatorMetrics> = HashMap::new();

            for validator in &config.validators {
                if !validator.is_active {
                    continue;
                }

                let telemetry = all_telemetry.get(&validator.id).unwrap();

                validator_metrics.insert(
                    validator.id,
                    dgbdt::ValidatorMetrics {
                        blocks_proposed: telemetry.blocks_proposed,
                        blocks_verified: telemetry.blocks_verified,
                        rounds_active: telemetry.rounds_active,
                        avg_latency_us: telemetry.avg_latency_us,
                        uptime_percentage: (telemetry.uptime_percentage_scaled * 1_000_000) / 10000,
                        slash_count: telemetry.slash_count,
                        recent_performance: (telemetry.recent_performance_scaled * 1_000_000)
                            / 10000,
                        network_contribution: (telemetry.network_contribution_scaled as i64
                            * 1_000_000)
                            / 10000,
                        stake_amount: validator.stake,
                    },
                );
            }

            // Use DGBDT engine for fair selection
            let engine = dgbdt_engine.read();
            match engine.select_verifiers(slot, &validator_metrics, 0, 0) {
                Ok(selection) => {
                    let latency_us = start.elapsed().as_micros() as u64;

                    // Record metrics
                    let reputation_scores: HashMap<[u8; 32], i32> = selection
                        .selection_scores
                        .iter()
                        .map(|(id, score)| (*id, *score))
                        .collect();
                    metrics.record_reputation_scores(&reputation_scores);
                    metrics.record_ai_selection_success(8000, latency_us); // Default confidence
                    metrics.record_validator_selected(&selection.primary);

                    info!(
                        "DGBDT selected validator: {} (latency: {}µs)",
                        hex::encode(selection.primary),
                        latency_us
                    );
                    Some(selection.primary)
                }
                Err(e) => {
                    metrics.record_ai_selection_fallback();
                    warn!("DGBDT selection failed: {e}, falling back to round-robin");
                    Self::fallback_proposer(&active, slot)
                }
            }
        } else {
            Self::fallback_proposer(&active, slot)
        }
    }

    fn fallback_proposer(validators: &[[u8; 32]], slot: u64) -> Option<[u8; 32]> {
        if validators.is_empty() {
            None
        } else {
            Some(validators[(slot % validators.len() as u64) as usize])
        }
    }

    // -----------------------------------------------------------------
    // Core round logic
    // -----------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    async fn propose_block(
        storage: &Arc<dyn Storage + Send + Sync>,
        mempool: &Arc<Mempool>,
        config: &PoAConfig,
        tracker: &Arc<RwLock<RoundTracker>>,
        _slot: u64,
        proposer: [u8; 32],
        telemetry_manager: &Arc<telemetry::TelemetryManager>,
        metrics: &Arc<metrics::ConsensusMetrics>,
    ) -> Result<()> {
        let txs = mempool.get_transactions_for_block(config.max_transactions_per_block);
        if txs.is_empty() {
            return Ok(());
        }

        let height = storage.get_latest_height()?;
        let parents = {
            let t = tracker.read();
            if !t.previous_round_blocks.is_empty() {
                t.previous_round_blocks.clone()
            } else if height == 0 {
                vec![]
            } else {
                vec![storage
                    .get_block_by_height(height)?
                    .ok_or_else(|| anyhow::anyhow!("Previous block not found"))?
                    .hash()]
            }
        };

        let block = Block::new(parents, txs, height + 1, proposer);
        if !block.is_valid() {
            return Err(anyhow::anyhow!("Invalid block"));
        }
        validate_confidential_block(&block.transactions)?;

        storage.store_block(block.clone())?;
        for tx in &block.transactions {
            let _ = mempool.remove_transaction(&hex::encode(tx.hash()));
        }

        tracker.write().current_round_blocks.push(block.header.id);

        // Record telemetry for block proposal
        let _ = telemetry_manager.record_block_proposal(&proposer);

        // Record metrics
        metrics.record_block_proposed();

        info!(
            "Proposed block {} at height {} ({} txs)",
            hex::encode(block.hash()),
            block.header.round,
            block.transactions.len()
        );
        Ok(())
    }

    fn finalize_round_if_ready(
        storage: &Arc<dyn Storage + Send + Sync>,
        tracker: &Arc<RwLock<RoundTracker>>,
        interval: Duration,
        config: &PoAConfig,
        fee_collector: &Arc<RwLock<FeeCollector>>,
        payment_engine: &Arc<payments::PaymentApplier>,
        metrics: &Arc<metrics::ConsensusMetrics>,
    ) -> Result<()> {
        let (round_id, block_ids, start, end) = {
            let mut t = tracker.write();
            if t.round_start.elapsed() < interval {
                return Ok(());
            }
            if t.current_round_blocks.is_empty() {
                t.round_start = Instant::now();
                t.round_start_time = IppanTimeMicros::now();
                return Ok(());
            }
            let id = t.current_round;
            let blocks = t.current_round_blocks.clone();
            t.previous_round_blocks = blocks.clone();
            t.current_round += 1;
            t.current_round_blocks.clear();
            let ws = t.round_start_time;
            t.round_start = Instant::now();
            t.round_start_time = IppanTimeMicros::now();
            (id, blocks, ws, t.round_start_time)
        };

        let blocks: Vec<_> = block_ids
            .iter()
            .filter_map(|id| storage.get_block(id).ok().flatten())
            .collect();
        if blocks.is_empty() {
            return Ok(());
        }

        let mut map = HashMap::new();
        for b in &blocks {
            map.insert(b.header.id, b.clone());
        }

        let mut conflicts = Vec::new();
        let ordered = order_round(
            round_id,
            &blocks,
            |bid| {
                map.get(bid)
                    .map(|b| b.header.parent_ids.clone())
                    .unwrap_or_default()
            },
            |bid| {
                map.get(bid)
                    .map(|b| b.header.payload_ids.clone())
                    .unwrap_or_default()
            },
            |_| true,
            |txid| conflicts.push(*txid),
        );

        let mut tx_lookup = HashMap::new();
        for block in &blocks {
            for tx in &block.transactions {
                tx_lookup.insert(tx.hash(), (tx.clone(), block.header.creator));
            }
        }

        let mut payment_stats = payments::PaymentRoundStats::new(round_id);
        for tx_id in &ordered {
            if let Some((tx, proposer)) = tx_lookup.get(tx_id) {
                match payment_engine.apply(storage, tx, proposer) {
                    Ok(split) => payment_stats.record_success(tx, *proposer, split),
                    Err(err) => {
                        payment_stats.record_failure(&err);
                        warn!(
                            "Round {}: failed to apply payment {} due to {:?}",
                            round_id,
                            hex::encode(tx_id),
                            err.kind()
                        );
                    }
                }
            }
        }

        if payment_stats.total_fees > 0 {
            let mut collector = fee_collector.write();
            collector.collect(ippan_types::Amount::from_atomic(payment_stats.total_fees));
        }

        let cert = RoundCertificate {
            round: round_id,
            block_ids: block_ids.clone(),
            agg_sig: Self::aggregate_round_signature(round_id, &block_ids),
        };

        let prev_root = storage
            .get_latest_round_finalization()?
            .map(|r| r.state_root)
            .unwrap_or([0u8; 32]);

        let mut hasher = Blake3::new();
        hasher.update(&round_id.to_be_bytes());
        hasher.update(&prev_root);
        for id in &block_ids {
            hasher.update(id);
        }
        for tx in &ordered {
            hasher.update(tx);
        }
        for c in &conflicts {
            hasher.update(c);
        }

        let digest = hasher.finalize();
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(digest.as_bytes());

        let window = RoundWindow {
            id: round_id,
            start_us: start,
            end_us: end,
        };
        let record = RoundFinalizationRecord {
            round: round_id,
            window,
            ordered_tx_ids: ordered,
            fork_drops: conflicts,
            state_root,
            proof: cert.clone(),
            total_fees_atomic: (payment_stats.total_fees > 0).then_some(payment_stats.total_fees),
            treasury_fees_atomic: (payment_stats.treasury_total > 0)
                .then_some(payment_stats.treasury_total),
            applied_payments: (payment_stats.applied > 0).then_some(payment_stats.applied as u64),
            rejected_payments: (payment_stats.rejected > 0)
                .then_some(payment_stats.rejected as u64),
        };
        storage.store_round_finalization(record)?;
        info!(
            "Finalized round {} -> state root {}",
            round_id,
            hex::encode(state_root)
        );
        if payment_stats.applied > 0 || payment_stats.rejected > 0 {
            info!(
                target: "fees",
                round = round_id,
                applied = payment_stats.applied,
                rejected = payment_stats.rejected,
                total_fees = payment_stats.total_fees,
                treasury = payment_stats.treasury_total,
                "Applied L1 payment fees"
            );
        }
        metrics.record_fee_stats(&payment_stats);

        // DAG-Fair Emission
        if config.enable_dag_fair_emission {
            let params = EmissionParams::default();
            let engine = EmissionEngine::with_params(params);
            let _ = engine.calculate_round_reward(round_id).unwrap_or(0);
        }
        Ok(())
    }

    fn aggregate_round_signature(round: RoundId, blocks: &[BlockId]) -> Vec<u8> {
        let mut h = Blake3::new();
        h.update(&round.to_be_bytes());
        for id in blocks {
            h.update(id);
        }
        h.finalize().as_bytes()[..32].to_vec()
    }

    pub fn get_state(&self) -> ConsensusState {
        let slot = *self.current_slot.read();
        let proposer = Self::select_proposer(
            &self.config,
            &self.round_consensus,
            slot,
            &self.dgbdt_engine,
            &self.telemetry_manager,
            &self.metrics,
        );

        ConsensusState {
            current_slot: slot,
            current_proposer: proposer,
            is_proposing: proposer == Some(self.validator_id),
            validator_count: self.config.validators.len(),
            latest_block_height: self.storage.get_latest_height().unwrap_or(0),
            current_round: self.round_tracker.read().current_round,
        }
    }

    /// Get metrics in Prometheus format
    pub fn get_metrics_prometheus(&self) -> String {
        self.metrics.export_prometheus()
    }

    /// Get metrics object for programmatic access
    pub fn get_metrics(&self) -> Arc<metrics::ConsensusMetrics> {
        self.metrics.clone()
    }

    pub fn add_validator(&mut self, v: Validator) {
        self.config.validators.push(v.clone());
        info!("Added validator {}", hex::encode(v.id));
    }

    pub fn remove_validator(&mut self, id: &[u8; 32]) {
        self.config.validators.retain(|v| v.id != *id);
        info!("Removed validator {}", hex::encode(id));
    }

    /// Update DGBDT model weights (for adaptive learning)
    pub fn update_dgbdt_weights(&self, factor: &str, new_weight: i64) {
        self.dgbdt_engine.write().update_weights(factor, new_weight);
        info!("DGBDT model weights updated: {} = {}", factor, new_weight);
    }
}

// ---------------------------------------------------------------------
// ConsensusEngine Trait
// ---------------------------------------------------------------------

#[allow(async_fn_in_trait)]
pub trait ConsensusEngine {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn propose_block(&self, transactions: Vec<Transaction>) -> Result<Block>;
    async fn validate_block(&self, block: &Block) -> Result<bool>;
    fn get_state(&self) -> ConsensusState;
}

impl ConsensusEngine for PoAConsensus {
    async fn start(&mut self) -> Result<()> {
        PoAConsensus::start(self).await
    }
    async fn stop(&mut self) -> Result<()> {
        PoAConsensus::stop(self).await
    }
    async fn propose_block(&self, txs: Vec<Transaction>) -> Result<Block> {
        let h = self.storage.get_latest_height()?;
        let prev = if h == 0 {
            [0u8; 32]
        } else {
            self.storage
                .get_block_by_height(h)?
                .ok_or_else(|| anyhow::anyhow!("Missing previous block"))?
                .hash()
        };
        Ok(Block::new(
            if h == 0 { vec![] } else { vec![prev] },
            txs,
            h + 1,
            self.validator_id,
        ))
    }
    async fn validate_block(&self, block: &Block) -> Result<bool> {
        if block
            .transactions
            .iter()
            .any(|tx| validate_confidential_transaction(tx).is_err())
        {
            return Ok(false);
        }
        Ok(true)
    }
    fn get_state(&self) -> ConsensusState {
        PoAConsensus::get_state(self)
    }
}

#[cfg(test)]
mod tests;
