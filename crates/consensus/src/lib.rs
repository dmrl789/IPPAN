//! IPPAN Consensus Engine — Parallel PoA + DAG-Fair Emission
//!
//! Implements Proof-of-Authority + AI Reputation-weighted consensus
//! for the IPPAN L1. Integrates deterministic BlockDAG ordering,
//! fee capping, emission schedules, and round-level finalization.

use anyhow::Result;
use blake3::Hasher as Blake3;
use ippan_crypto::{validate_confidential_block, validate_confidential_transaction};
use ippan_mempool::Mempool;
use ippan_storage::Storage;
use ippan_types::{
    IppanTimeMicros, Block, BlockId, RoundCertificate, RoundFinalizationRecord,
    RoundId, RoundWindow, Transaction,
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

pub mod ordering;
pub mod parallel_dag;
pub mod reputation;
pub mod emission;
pub mod emission_tracker;
pub mod fees;
pub mod round;

// ---------------------------------------------------------------------
// Public re-exports
// ---------------------------------------------------------------------

pub use emission::{
    DAGEmissionParams, ValidatorRole, ValidatorParticipation,
    RoundEmission, ValidatorReward,
    calculate_round_reward, calculate_round_emission, distribute_dag_fair_rewards,
    calculate_fee_recycling, FeeRecyclingParams, projected_supply,
};
pub use emission_tracker::{EmissionStatistics, EmissionTracker};
pub use fees::{classify_transaction, validate_fee, FeeCapConfig, FeeCollector, FeeError, TxKind};
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
    pub emission_tracker: Arc<RwLock<EmissionTracker>>,
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

        let emission_params = DAGEmissionParams::default();
        let audit_interval = 6_048_000; // ~1 week at 100ms
        let emission_tracker = EmissionTracker::new(emission_params.clone(), audit_interval);

        Self {
            config: config.clone(),
            storage: storage.clone(),
            validator_id,
            is_running: Arc::new(RwLock::new(false)),
            current_slot: Arc::new(RwLock::new(0)),
            tx_sender,
            mempool: Arc::new(Mempool::new(10_000)),
            round_tracker: Arc::new(RwLock::new(tracker)),
            finalization_interval: Duration::from_millis(config.finalization_interval_ms.clamp(100, 250)),
            round_consensus: Arc::new(RwLock::new(RoundConsensus::new())),
            fee_collector: Arc::new(RwLock::new(FeeCollector::new())),
            emission_tracker: Arc::new(RwLock::new(emission_tracker)),
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

        let (is_running, current_slot, config, storage, mempool, round_tracker, round_consensus, finalization_interval, validator_id, fee_collector) =
            (
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
            );

        let mut ticker = interval(Duration::from_millis(config.slot_duration_ms));
        tokio::spawn(async move {
            loop {
                if !*is_running.read() {
                    break;
                }

                if let Err(e) =
                    Self::finalize_round_if_ready(&storage, &round_tracker, finalization_interval, &config, &fee_collector)
                {
                    error!("Round finalization error: {e}");
                }

                let slot = *current_slot.read();
                if let Some(proposer) = Self::select_proposer(&config, &round_consensus, slot) {
                    if proposer == validator_id {
                        if let Err(e) =
                            Self::propose_block(&storage, &mempool, &config, &round_tracker, slot, validator_id)
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
    // Core logic
    // -----------------------------------------------------------------

    fn select_proposer(
        config: &PoAConfig,
        round_consensus: &Arc<RwLock<RoundConsensus>>,
        slot: u64,
    ) -> Option<[u8; 32]> {
        let active: Vec<_> = config.validators.iter().filter(|v| v.is_active).map(|v| v.id).collect();
        if active.is_empty() {
            return None;
        }

        if config.enable_ai_reputation {
            let weights: HashMap<[u8; 32], u64> =
                config.validators.iter().map(|v| (v.id, v.stake)).collect();
            match round_consensus.write().select_validators(&active, &weights) {
                Ok(sel) => Some(sel.proposer),
                Err(e) => {
                    warn!("AI selection failed: {e}");
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

    async fn propose_block(
        storage: &Arc<dyn Storage + Send + Sync>,
        mempool: &Arc<Mempool>,
        config: &PoAConfig,
        tracker: &Arc<RwLock<RoundTracker>>,
        _slot: u64,
        proposer: [u8; 32],
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
        validate_confidential_block(&block)?;

        storage.store_block(block.clone())?;
        for tx in &block.transactions {
            let _ = mempool.remove_transaction(&hex::encode(tx.hash()));
        }

        {
            let mut t = tracker.write();
            t.current_round_blocks.push(block.header.id);
        }

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

        let blocks: Vec<_> = block_ids.iter().filter_map(|id| storage.get_block(id).ok().flatten()).collect();
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
            |bid| map.get(bid).map(|b| b.header.parent_ids.clone()).unwrap_or_default(),
            |bid| map.get(bid).map(|b| b.header.payload_ids.clone()).unwrap_or_default(),
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
        info!("Finalized round {} -> state root {}", round_id, hex::encode(state_root));

        // -----------------------------------------------------------------
        // DAG-Fair Emission + Reward Distribution
        // -----------------------------------------------------------------
        if config.enable_dag_fair_emission {
            use crate::emission::EmissionParams;

            let params = EmissionParams::default();
            let issued_micro = storage.get_total_issued_micro().unwrap_or(0);
            let remaining_cap = params.supply_cap.saturating_sub(issued_micro);
            let planned_emission = round_reward(round_id, &params);
            let emission_micro = planned_emission.min(remaining_cap);

            // Fee recycling (weekly by default)
            let mut recycled_fees: u128 = 0;
            {
                let mut fc = fee_collector.write();
                let fr = FeeRecyclingParams::default();
                if fc.should_recycle(round_id, fr.rounds_per_week) {
                    recycled_fees = fc.recycle(round_id, fr.recycle_bps);
                }
            }

            // Determine distribution pools
            let proposer_pool = (emission_micro * params.proposer_bps as u128) / 10_000;
            let verifier_pool = emission_micro.saturating_sub(proposer_pool);

            // Aggregate blocks by creator for proposer weighting
            let mut blocks_by_creator: HashMap<[u8; 32], u128> = HashMap::new();
            for b in &blocks {
                *blocks_by_creator.entry(b.header.creator).or_insert(0) += 1;
            }
            let total_blocks_in_round: u128 = blocks_by_creator.values().sum();

            // Active validators are verifiers
            let verifiers: Vec<[u8; 32]> = config
                .validators
                .iter()
                .filter(|v| v.is_active)
                .map(|v| v.id)
                .collect();
            let verifier_count = verifiers.len() as u128;

            // Credit proposer pool proportionally to blocks produced
            let mut distributed_proposer_total: u128 = 0;
            let mut sorted_creators: Vec<[u8; 32]> = blocks_by_creator.keys().copied().collect();
            sorted_creators.sort(); // deterministic tie-breaking
            for (i, creator) in sorted_creators.iter().enumerate() {
                let weight = *blocks_by_creator.get(creator).unwrap_or(&0);
                let share = if total_blocks_in_round > 0 {
                    (proposer_pool * weight) / total_blocks_in_round
                } else {
                    0
                };
                if share > 0 {
                    let _ = storage.credit_account_micro(creator, share);
                }
                distributed_proposer_total = distributed_proposer_total.saturating_add(share);
                // On last creator, add rounding remainder
                if i + 1 == sorted_creators.len() {
                    let remainder = proposer_pool.saturating_sub(distributed_proposer_total);
                    if remainder > 0 {
                        let _ = storage.credit_account_micro(creator, remainder);
                        distributed_proposer_total = distributed_proposer_total.saturating_add(remainder);
                    }
                }
            }

            // Distribute verifier pool equally among active verifiers
            let mut distributed_verifier_total: u128 = 0;
            if verifier_count > 0 {
                let per_verifier = verifier_pool / verifier_count;
                for vid in &verifiers {
                    if per_verifier > 0 {
                        let _ = storage.credit_account_micro(vid, per_verifier);
                    }
                    distributed_verifier_total = distributed_verifier_total.saturating_add(per_verifier);
                }
                // Remainder due to integer division goes to the lowest-sorted validator deterministically
                let remainder = verifier_pool.saturating_sub(distributed_verifier_total);
                if remainder > 0 {
                    let mut sorted = verifiers.clone();
                    sorted.sort();
                    if let Some(first) = sorted.first() {
                        let _ = storage.credit_account_micro(first, remainder);
                        distributed_verifier_total = distributed_verifier_total.saturating_add(remainder);
                    }
                }

                // Distribute recycled fees equally among verifiers
                if recycled_fees > 0 {
                    let per = recycled_fees / verifier_count;
                    for vid in &verifiers {
                        if per > 0 {
                            let _ = storage.credit_account_micro(vid, per);
                        }
                    }
                    let leftover = recycled_fees.saturating_sub(per * verifier_count);
                    if leftover > 0 {
                        let mut sorted = verifiers.clone();
                        sorted.sort();
                        if let Some(first) = sorted.first() {
                            let _ = storage.credit_account_micro(first, leftover);
                        }
                    }
                }
            }

            // Update total issued supply by the emission amount actually paid
            let emission_paid = distributed_proposer_total.saturating_add(distributed_verifier_total);
            if emission_paid > 0 {
                let _ = storage.add_total_issued_micro(emission_paid);
                info!(
                    target: "emission",
                    "Round {} → emitted {} µIPN (proposers {} + verifiers {}), fees_recycled {} µIPN",
                    round_id,
                    emission_paid,
                    distributed_proposer_total,
                    distributed_verifier_total,
                    recycled_fees
                );
            }
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
        let proposer = Self::select_proposer(&self.config, &self.round_consensus, slot);
        ConsensusState {
            current_slot: slot,
            current_proposer: proposer,
            is_proposing: proposer == Some(self.validator_id),
            validator_count: self.config.validators.len(),
            latest_block_height: self.storage.get_latest_height().unwrap_or(0),
            current_round: self.round_tracker.read().current_round,
        }
    }

    pub fn add_validator(&mut self, v: Validator) {
        self.config.validators.push(v.clone());
        info!("Added validator {}", hex::encode(v.id));
    }

    pub fn remove_validator(&mut self, id: &[u8; 32]) {
        self.config.validators.retain(|v| v.id != *id);
        info!("Removed validator {}", hex::encode(id));
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
        Ok(Block::new(if h == 0 { vec![] } else { vec![prev] }, txs, h + 1, self.validator_id))
    }
    async fn validate_block(&self, block: &Block) -> Result<bool> {
        if block.transactions.iter().any(|tx| validate_confidential_transaction(tx).is_err()) {
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
