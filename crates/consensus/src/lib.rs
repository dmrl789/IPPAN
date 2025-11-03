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
pub mod dlc;
pub mod dgbdt;
pub mod shadow_verifier;
pub mod bonding;
pub mod hashtimer_integration;
pub mod dlc_integration;

// Economic and emission modules
pub mod emission;
pub mod emission_tracker;
pub mod fees;

// AI and selection modules
pub mod l1_ai_consensus;

// Telemetry and metrics
pub mod metrics;
pub mod model_reload;
pub mod telemetry;
pub mod reputation;

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
pub use dlc::{DLCConfig, DLCConsensus, DLCRoundState};
pub use dgbdt::{DGBDTEngine, ValidatorMetrics, VerifierSelection};
pub use shadow_verifier::{ShadowVerifier, ShadowVerifierSet, VerificationResult};
pub use bonding::{BondingManager, ValidatorBond, VALIDATOR_BOND_AMOUNT, MIN_BOND_AMOUNT};
pub use hashtimer_integration::{
    generate_round_hashtimer, generate_block_hashtimer, verify_temporal_ordering,
    should_close_round, derive_selection_seed,
};
pub use dlc_integration::{DLCIntegratedConsensus, dlc_config_from_poa};

// Emissions and fees
pub use emission::{
    calculate_fee_recycling, calculate_round_emission, calculate_round_reward,
    distribute_dag_fair_rewards, DAGEmissionParams, FeeRecyclingParams, RoundEmission,
    ValidatorParticipation, ValidatorReward, ValidatorRole,
};
pub use emission_tracker::{EmissionStatistics, EmissionTracker};
pub use fees::{classify_transaction, validate_fee, FeeCapConfig, FeeCollector, FeeError, TxKind};
pub use ippan_economics::{EmissionEngine, EmissionParams, RewardAmount, RoundIndex, RoundRewards};
#[cfg(feature = "ai_l1")]
pub use l1_ai_consensus::{
    FeeOptimizationResult, L1AIConfig, L1AIConsensus, NetworkHealthReport, NetworkState,
    ValidatorCandidate, ValidatorSelectionResult,
};
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
struct RoundTracker {
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
    #[cfg(feature = "ai_l1")]
    pub l1_ai_consensus: Arc<RwLock<L1AIConsensus>>,
    pub emission_tracker: Arc<RwLock<EmissionTracker>>,
    pub telemetry_manager: Arc<telemetry::TelemetryManager>,
    #[cfg(feature = "ai_l1")]
    pub model_reloader: Option<Arc<model_reload::ModelReloader>>,
    pub metrics: Arc<metrics::ConsensusMetrics>,
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

        #[cfg(feature = "ai_l1")]
        let l1_ai_consensus = {
            let ai_config = L1AIConfig::default();
            L1AIConsensus::new(ai_config)
        };

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
            #[cfg(feature = "ai_l1")]
            l1_ai_consensus: Arc::new(RwLock::new(l1_ai_consensus)),
            emission_tracker: Arc::new(RwLock::new(emission_tracker)),
            telemetry_manager,
            #[cfg(feature = "ai_l1")]
            model_reloader: None,
            metrics,
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
        );

        #[cfg(feature = "ai_l1")]
        let l1_ai_consensus = self.l1_ai_consensus.clone();

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
                ) {
                    error!("Round finalization error: {e}");
                }

                let slot = *current_slot.read();
                #[cfg(feature = "ai_l1")]
                let maybe_proposer = Self::select_proposer(
                    &config,
                    &round_consensus,
                    slot,
                    &l1_ai_consensus,
                    &telemetry_manager,
                    &metrics,
                );
                #[cfg(not(feature = "ai_l1"))]
                let maybe_proposer = Self::select_proposer_no_ai(&config, &round_consensus, slot);

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

    #[cfg(feature = "ai_l1")]
    fn select_proposer(
        config: &PoAConfig,
        _round_consensus: &Arc<RwLock<RoundConsensus>>,
        slot: u64,
        l1_ai: &Arc<RwLock<L1AIConsensus>>,
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

            let candidates: Vec<ValidatorCandidate> = config
                .validators
                .iter()
                .filter(|v| v.is_active)
                .map(|v| {
                    let telemetry = all_telemetry.get(&v.id).unwrap();

                    // Calculate reputation score from telemetry
                    let reputation_score = Self::calculate_reputation_from_telemetry(telemetry);

                    ValidatorCandidate {
                        id: v.id,
                        stake: v.stake,
                        reputation_score,
                        uptime_percentage: telemetry.uptime_percentage,
                        recent_performance: telemetry.recent_performance,
                        network_contribution: telemetry.network_contribution,
                    }
                })
                .collect();

            // Record reputation scores
            let reputation_scores: HashMap<[u8; 32], i32> = candidates
                .iter()
                .map(|c| (c.id, c.reputation_score))
                .collect();
            metrics.record_reputation_scores(&reputation_scores);

            let network_state = NetworkState {
                congestion_level: 0.3,
                avg_block_time_ms: 200.0,
                active_validators: active.len(),
                total_stake: config.validators.iter().map(|v| v.stake).sum(),
                current_round: slot,
                recent_tx_volume: 1000,
            };

            match l1_ai.read().select_validator(&candidates, &network_state) {
                Ok(result) => {
                    let latency_us = start.elapsed().as_micros() as u64;
                    metrics.record_ai_selection_success(result.confidence_score, latency_us);
                    metrics.record_validator_selected(&result.selected_validator);

                    info!(
                        "L1 AI selected validator: {} (confidence: {:.2}, latency: {}µs)",
                        hex::encode(result.selected_validator),
                        result.confidence_score,
                        latency_us
                    );
                    Some(result.selected_validator)
                }
                Err(e) => {
                    metrics.record_ai_selection_fallback();
                    warn!("L1 AI selection failed: {e}, falling back to simple selection");
                    Self::fallback_proposer(&active, slot)
                }
            }
        } else {
            Self::fallback_proposer(&active, slot)
        }
    }

    /// Calculate reputation score (0-10000) from telemetry data
    #[cfg(feature = "ai_l1")]
    fn calculate_reputation_from_telemetry(telemetry: &ippan_storage::ValidatorTelemetry) -> i32 {
        // Calculate based on various factors
        let proposal_rate = if telemetry.rounds_active > 0 {
            (telemetry.blocks_proposed * 10000 / telemetry.rounds_active).min(10000) as i32
        } else {
            5000
        };

        let verification_rate = if telemetry.rounds_active > 0 {
            (telemetry.blocks_verified * 1000 / telemetry.rounds_active).min(10000) as i32
        } else {
            5000
        };

        let latency_score =
            ((200_000 - telemetry.avg_latency_us.min(200_000)) * 10000 / 200_000) as i32;
        let slash_penalty = 10000 - (telemetry.slash_count * 1000).min(10000) as i32;
        let uptime_score = (telemetry.uptime_percentage * 100.0) as i32;
        let performance_score = (telemetry.recent_performance * 10000.0) as i32;

        // Weighted average
        let score = (proposal_rate * 25
            + verification_rate * 20
            + latency_score * 15
            + slash_penalty * 20
            + uptime_score * 10
            + performance_score * 10)
            / 100;

        score.clamp(0, 10000)
    }

    #[cfg(not(feature = "ai_l1"))]
    fn select_proposer_no_ai(
        config: &PoAConfig,
        _round_consensus: &Arc<RwLock<RoundConsensus>>,
        slot: u64,
    ) -> Option<[u8; 32]> {
        let active: Vec<_> = config
            .validators
            .iter()
            .filter(|v| v.is_active)
            .map(|v| v.id)
            .collect();
        if active.is_empty() {
            None
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
        };
        storage.store_round_finalization(record)?;
        info!(
            "Finalized round {} -> state root {}",
            round_id,
            hex::encode(state_root)
        );

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
        #[cfg(feature = "ai_l1")]
        let proposer = Self::select_proposer(
            &self.config,
            &self.round_consensus,
            slot,
            &self.l1_ai_consensus,
            &self.telemetry_manager,
            &self.metrics,
        );
        #[cfg(not(feature = "ai_l1"))]
        let proposer = Self::select_proposer_no_ai(&self.config, &self.round_consensus, slot);

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

    #[cfg(feature = "ai_l1")]
    pub fn load_ai_models(
        &self,
        validator_model: Option<ippan_ai_core::GBDTModel>,
        fee_model: Option<ippan_ai_core::GBDTModel>,
        health_model: Option<ippan_ai_core::GBDTModel>,
        ordering_model: Option<ippan_ai_core::GBDTModel>,
    ) -> Result<(), String> {
        self.l1_ai_consensus.write().load_models(
            validator_model,
            fee_model,
            health_model,
            ordering_model,
        )
    }

    #[cfg(feature = "ai_l1")]
    pub fn get_ai_config(&self) -> L1AIConfig {
        self.l1_ai_consensus.read().config.clone()
    }

    #[cfg(feature = "ai_l1")]
    pub fn update_ai_config(&self, config: L1AIConfig) {
        self.l1_ai_consensus.write().config = config;
        info!("L1 AI consensus configuration updated");
    }

    /// Enable hot-reload for GBDT models
    ///
    /// Starts a background watcher that monitors model files and reloads them when changed.
    /// Models are checked every `check_interval`.
    #[cfg(feature = "ai_l1")]
    pub fn enable_model_hot_reload(
        &mut self,
        validator_model_path: Option<std::path::PathBuf>,
        fee_model_path: Option<std::path::PathBuf>,
        health_model_path: Option<std::path::PathBuf>,
        ordering_model_path: Option<std::path::PathBuf>,
        check_interval: Duration,
    ) -> Result<()> {
        let l1_ai = self.l1_ai_consensus.clone();

        let reloader = Arc::new(model_reload::ModelReloader::new(move |update| {
            match update {
                model_reload::ModelUpdate::Validator(model) => {
                    l1_ai.write().validator_selection_model = Some(model);
                    info!("Hot-reloaded validator selection model");
                }
                model_reload::ModelUpdate::Fee(model) => {
                    l1_ai.write().fee_optimization_model = Some(model);
                    info!("Hot-reloaded fee optimization model");
                }
                model_reload::ModelUpdate::Health(model) => {
                    l1_ai.write().network_health_model = Some(model);
                    info!("Hot-reloaded network health model");
                }
                model_reload::ModelUpdate::Ordering(model) => {
                    l1_ai.write().block_ordering_model = Some(model);
                    info!("Hot-reloaded block ordering model");
                }
            }
            Ok(())
        }));

        // Configure paths
        let mut reloader_mut = Arc::try_unwrap(reloader).unwrap_or_else(|arc| (*arc).clone());

        if let Some(path) = validator_model_path {
            reloader_mut.set_validator_model_path(path);
        }
        if let Some(path) = fee_model_path {
            reloader_mut.set_fee_model_path(path);
        }
        if let Some(path) = health_model_path {
            reloader_mut.set_health_model_path(path);
        }
        if let Some(path) = ordering_model_path {
            reloader_mut.set_ordering_model_path(path);
        }

        let reloader = Arc::new(reloader_mut);

        // Start the watcher
        reloader.clone().start_watcher(check_interval);

        self.model_reloader = Some(reloader);

        info!(
            "Model hot-reload enabled with check interval: {:?}",
            check_interval
        );
        Ok(())
    }

    /// Manually trigger a reload of all models
    #[cfg(feature = "ai_l1")]
    pub async fn reload_models_now(&self) -> Result<()> {
        if let Some(reloader) = &self.model_reloader {
            reloader.force_reload_all().await?;
            info!("Manually triggered model reload");
        } else {
            warn!("Model hot-reload is not enabled");
        }
        Ok(())
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
