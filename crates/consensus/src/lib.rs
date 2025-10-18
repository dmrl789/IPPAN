use anyhow::Result;
use blake3::Hasher as Blake3;
use ippan_crypto::{validate_confidential_block, validate_confidential_transaction};
use ippan_mempool::Mempool;
use ippan_storage::{Account, Storage};
use ippan_types::IppanTimeMicros;
use ippan_types::{
    Block, BlockId, RoundCertificate, RoundFinalizationRecord, RoundId, RoundWindow, Transaction,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};

pub mod ordering;
pub use ordering::order_round;

/// Consensus errors
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

/// PoA consensus configuration
#[derive(Debug, Clone)]
pub struct PoAConfig {
    pub slot_duration_ms: u64,
    pub validators: Vec<Validator>,
    pub max_transactions_per_block: usize,
    pub block_reward: u64,
    pub finalization_interval_ms: u64,
}

impl Default for PoAConfig {
    fn default() -> Self {
        Self {
            slot_duration_ms: 100,
            validators: vec![],
            max_transactions_per_block: 1000,
            block_reward: 10,
            finalization_interval_ms: 200,
        }
    }
}

/// Consensus state
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

/// PoA consensus engine
pub struct PoAConsensus {
    config: PoAConfig,
    storage: Arc<dyn Storage + Send + Sync>,
    validator_id: [u8; 32],
    is_running: Arc<RwLock<bool>>,
    current_slot: Arc<RwLock<u64>>,
    tx_receiver: Option<mpsc::UnboundedReceiver<Transaction>>,
    tx_sender: mpsc::UnboundedSender<Transaction>,
    mempool: Arc<Mempool>,
    round_tracker: Arc<RwLock<RoundTracker>>,
    finalization_interval: Duration,
}

impl PoAConsensus {
    /// Create a new PoA consensus engine
    pub fn new(
        config: PoAConfig,
        storage: Arc<dyn Storage + Send + Sync>,
        validator_id: [u8; 32],
    ) -> Self {
        let (tx_sender, tx_receiver) = mpsc::unbounded_channel();

        let latest_height = storage.get_latest_height().unwrap_or(0);
        let previous_round_blocks = if latest_height == 0 {
            Vec::new()
        } else if let Ok(Some(block)) = storage.get_block_by_height(latest_height) {
            vec![block.hash()]
        } else {
            Vec::new()
        };

        let round_tracker = RoundTracker {
            current_round: latest_height.saturating_add(1),
            round_start: Instant::now(),
            round_start_time: IppanTimeMicros::now(),
            previous_round_blocks,
            current_round_blocks: Vec::new(),
        };

        let finalization_interval =
            Duration::from_millis(config.finalization_interval_ms.clamp(100, 250));

        Self {
            config,
            storage,
            validator_id,
            is_running: Arc::new(RwLock::new(false)),
            current_slot: Arc::new(RwLock::new(0)),
            tx_receiver: Some(tx_receiver),
            tx_sender,
            mempool: Arc::new(Mempool::new(10000)), // 10k transaction limit
            round_tracker: Arc::new(RwLock::new(round_tracker)),
            finalization_interval,
        }
    }

    /// Get the transaction sender for submitting transactions
    pub fn get_tx_sender(&self) -> mpsc::UnboundedSender<Transaction> {
        self.tx_sender.clone()
    }

    /// Get a handle to the current mempool
    pub fn mempool(&self) -> Arc<Mempool> {
        self.mempool.clone()
    }

    /// Start the consensus engine
    pub async fn start(&mut self) -> Result<()> {
        *self.is_running.write() = true;
        info!("Starting PoA consensus engine");

        // Initialize current slot based on time
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let initial_slot = current_time / self.config.slot_duration_ms;
        *self.current_slot.write() = initial_slot;

        // Start the consensus loop
        {
            let is_running = self.is_running.clone();
            let current_slot = self.current_slot.clone();
            let config = self.config.clone();
            let storage = self.storage.clone();
            let validator_id = self.validator_id;
            let mempool = self.mempool.clone();
            let mut tx_receiver = self.tx_receiver.take().unwrap();
            let round_tracker = self.round_tracker.clone();
            let finalization_interval = self.finalization_interval;

            tokio::spawn(async move {
                let mut slot_interval = interval(Duration::from_millis(config.slot_duration_ms));

                loop {
                    if !*is_running.read() {
                        break;
                    }

                    // Process incoming transactions
                    while let Ok(tx) = tx_receiver.try_recv() {
                        if let Err(e) = mempool.add_transaction(tx) {
                            warn!("Failed to add transaction to mempool: {}", e);
                        }
                    }

                    if let Err(e) = Self::finalize_round_if_ready(
                        &storage,
                        &round_tracker,
                        finalization_interval,
                    ) {
                        error!("Failed to finalize round: {}", e);
                    }

                    // Check if it's our turn to propose
                    let slot_number = *current_slot.read();
                    let proposer = Self::get_proposer_for_slot(&config.validators, slot_number);

                    if let Some(proposer_id) = proposer {
                        if proposer_id == validator_id {
                            // It's our turn to propose a block
                            if let Err(e) = Self::propose_block(
                                &storage,
                                &mempool,
                                &config,
                                &round_tracker,
                                slot_number,
                                validator_id,
                            )
                            .await
                            {
                                error!("Failed to propose block: {}", e);
                            }
                        }
                    }

                    // Update current slot
                    *current_slot.write() = slot_number + 1;

                    slot_interval.tick().await;
                }
            });
        }

        info!("PoA consensus engine started");
        Ok(())
    }

    /// Stop the consensus engine
    pub async fn stop(&mut self) -> Result<()> {
        *self.is_running.write() = false;
        info!("Stopping PoA consensus engine");

        // Wait a bit for the consensus loop to finish
        sleep(Duration::from_millis(100)).await;

        info!("PoA consensus engine stopped");
        Ok(())
    }

    /// Get current consensus state
    pub fn get_state(&self) -> ConsensusState {
        let current_slot = *self.current_slot.read();
        let proposer = Self::get_proposer_for_slot(&self.config.validators, current_slot);
        let is_proposing = proposer == Some(self.validator_id);
        let current_round = self.round_tracker.read().current_round;

        ConsensusState {
            current_slot,
            current_proposer: proposer,
            is_proposing,
            validator_count: self.config.validators.len(),
            latest_block_height: self.storage.get_latest_height().unwrap_or(0),
            current_round,
        }
    }

    /// Get the proposer for a given slot
    fn get_proposer_for_slot(validators: &[Validator], slot: u64) -> Option<[u8; 32]> {
        if validators.is_empty() {
            return None;
        }

        let active_validators: Vec<&Validator> =
            validators.iter().filter(|v| v.is_active).collect();

        if active_validators.is_empty() {
            return None;
        }

        let proposer_index = (slot % active_validators.len() as u64) as usize;
        Some(active_validators[proposer_index].id)
    }

    /// Propose a new block
    async fn propose_block(
        storage: &Arc<dyn Storage + Send + Sync>,
        mempool: &Arc<Mempool>,
        config: &PoAConfig,
        round_tracker: &Arc<RwLock<RoundTracker>>,
        _slot: u64,
        proposer_id: [u8; 32],
    ) -> Result<()> {
        // Get transactions from mempool
        let block_transactions =
            mempool.get_transactions_for_block(config.max_transactions_per_block);

        if block_transactions.is_empty() {
            return Ok(());
        }

        // Get previous block hash
        let latest_height = storage.get_latest_height()?;
        let parent_ids = {
            let tracker = round_tracker.read();
            if !tracker.previous_round_blocks.is_empty() {
                tracker.previous_round_blocks.clone()
            } else if latest_height == 0 {
                Vec::new()
            } else {
                let prev_block = storage
                    .get_block_by_height(latest_height)?
                    .ok_or_else(|| anyhow::anyhow!("Previous block not found"))?;
                vec![prev_block.hash()]
            }
        };

        // Create new block
        let block = Block::new(
            parent_ids,
            block_transactions,
            latest_height + 1,
            proposer_id,
        );

        // Validate block
        if !block.is_valid() {
            return Err(anyhow::anyhow!("Invalid block"));
        }

        validate_confidential_block(&block)
            .map_err(|err| anyhow::anyhow!("invalid confidential transaction: {err}"))?;

        // Store block
        storage.store_block(block.clone())?;

        {
            let mut tracker = round_tracker.write();
            tracker.current_round_blocks.push(block.header.id);
        }

        let final_round = round_tracker.read().current_round;

        let storage_clone = storage.clone();
        let transactions = block.transactions.clone();
        let block_reward = config.block_reward;
        tokio::spawn(async move {
            if let Err(e) =
                Self::process_transactions(&storage_clone, &transactions, block_reward, proposer_id)
                    .await
            {
                error!("Failed to process block transactions: {}", e);
            }
        });

        info!(
            "Proposed and stored block {} at height {} (round {}) with {} transactions",
            hex::encode(block.hash()),
            block.header.round,
            final_round,
            block.transactions.len()
        );

        Ok(())
    }

    fn finalize_round_if_ready(
        storage: &Arc<dyn Storage + Send + Sync>,
        round_tracker: &Arc<RwLock<RoundTracker>>,
        finalization_interval: Duration,
    ) -> Result<()> {
        let (round_id, block_ids, window_start, window_end) = {
            let mut tracker = round_tracker.write();
            if tracker.round_start.elapsed() < finalization_interval {
                return Ok(());
            }

            if tracker.current_round_blocks.is_empty() {
                tracker.round_start = Instant::now();
                tracker.round_start_time = IppanTimeMicros::now();
                return Ok(());
            }

            let round_id = tracker.current_round;
            let block_ids = tracker.current_round_blocks.clone();
            let window_start = tracker.round_start_time;
            tracker.previous_round_blocks = block_ids.clone();
            tracker.current_round += 1;
            tracker.current_round_blocks.clear();
            tracker.round_start = Instant::now();
            let window_end = IppanTimeMicros::now();
            tracker.round_start_time = window_end;
            (round_id, block_ids, window_start, window_end)
        };

        let mut blocks = Vec::new();
        for block_id in &block_ids {
            if let Some(block) = storage.get_block(block_id)? {
                blocks.push(block);
            }
        }

        if blocks.is_empty() {
            return Ok(());
        }

        let mut block_map = HashMap::new();
        for block in &blocks {
            block_map.insert(block.header.id, block.clone());
        }

        let mut conflicts = Vec::new();
        let ordered = order_round(
            round_id,
            &blocks,
            |block_id| {
                block_map
                    .get(block_id)
                    .map(|block| block.header.parent_ids.clone())
                    .unwrap_or_default()
            },
            |block_id| {
                block_map
                    .get(block_id)
                    .map(|block| block.header.payload_ids.clone())
                    .unwrap_or_default()
            },
            |_| true,
            |tx_id| conflicts.push(*tx_id),
        );

        info!(
            "Finalized round {} with {} blocks, {} ordered transactions, {} conflicts",
            round_id,
            block_ids.len(),
            ordered.len(),
            conflicts.len()
        );

        for conflict in &conflicts {
            warn!(
                "Conflict detected in round {} for transaction {}",
                round_id,
                hex::encode(conflict)
            );
        }

        let certificate = RoundCertificate {
            round: round_id,
            block_ids: block_ids.clone(),
            agg_sig: Self::aggregate_round_signature(round_id, &block_ids),
        };

        let previous_state_root = storage
            .get_latest_round_finalization()?
            .map(|record| record.state_root)
            .unwrap_or([0u8; 32]);

        let mut state_hasher = Blake3::new();
        state_hasher.update(&round_id.to_be_bytes());
        state_hasher.update(&previous_state_root);
        for block_id in &block_ids {
            state_hasher.update(block_id);
        }
        for tx_id in &ordered {
            state_hasher.update(tx_id);
        }
        for conflict in &conflicts {
            state_hasher.update(conflict);
        }
        let state_digest = state_hasher.finalize();
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(state_digest.as_bytes());

        let window = RoundWindow {
            id: round_id,
            start_us: window_start,
            end_us: window_end,
        };

        let finalization = RoundFinalizationRecord {
            round: round_id,
            window,
            ordered_tx_ids: ordered.clone(),
            fork_drops: conflicts.clone(),
            state_root,
            proof: certificate.clone(),
        };

        storage.store_round_finalization(finalization.clone())?;

        info!(
            "Round {} certificate aggregated {} blocks; state root {}",
            round_id,
            certificate.block_ids.len(),
            hex::encode(state_root)
        );

        Ok(())
    }

    fn aggregate_round_signature(round_id: RoundId, block_ids: &[BlockId]) -> Vec<u8> {
        let mut hasher = Blake3::new();
        hasher.update(&round_id.to_be_bytes());
        for block_id in block_ids {
            hasher.update(block_id);
        }
        hasher.finalize().as_bytes()[..32].to_vec()
    }

    /// Apply transactions and block rewards using the consensus configuration
    pub async fn apply_block_state(
        &self,
        transactions: &[Transaction],
        proposer_id: [u8; 32],
    ) -> Result<()> {
        Self::apply_transactions_to_storage(
            &self.storage,
            transactions,
            self.config.block_reward,
            proposer_id,
        )
        .await
    }

    /// Process transactions and update account balances
    async fn process_transactions(
        storage: &Arc<dyn Storage + Send + Sync>,
        transactions: &[Transaction],
        block_reward: u64,
        proposer_id: [u8; 32],
    ) -> Result<()> {
        Self::apply_transactions_to_storage(storage, transactions, block_reward, proposer_id).await
    }

    async fn apply_transactions_to_storage(
        storage: &Arc<dyn Storage + Send + Sync>,
        transactions: &[Transaction],
        block_reward: u64,
        proposer_id: [u8; 32],
    ) -> Result<()> {
        // Award block reward to proposer
        if let Some(mut proposer_account) = storage.get_account(&proposer_id)? {
            proposer_account.balance += block_reward;
            storage.update_account(proposer_account)?;
        } else {
            // Create new account for proposer
            let proposer_account = Account {
                address: proposer_id,
                balance: block_reward,
                nonce: 0,
            };
            storage.update_account(proposer_account)?;
        }

        // Process each transaction
        for tx in transactions {
            // Get sender account
            if let Some(mut sender_account) = storage.get_account(&tx.from)? {
                // Check balance and nonce
                if sender_account.balance >= tx.amount && sender_account.nonce == tx.nonce {
                    sender_account.balance -= tx.amount;
                    sender_account.nonce += 1;
                    storage.update_account(sender_account)?;

                    // Update receiver account
                    if let Some(mut receiver_account) = storage.get_account(&tx.to)? {
                        receiver_account.balance += tx.amount;
                        storage.update_account(receiver_account)?;
                    } else {
                        // Create new account for receiver
                        let receiver_account = Account {
                            address: tx.to,
                            balance: tx.amount,
                            nonce: 0,
                        };
                        storage.update_account(receiver_account)?;
                    }
                } else {
                    warn!("Invalid transaction: insufficient balance or nonce");
                }
            } else {
                warn!("Transaction from unknown account");
            }
        }

        Ok(())
    }

    /// Validate a proposed block
    pub async fn validate_block(&self, block: &Block) -> Result<bool> {
        if !*self.is_running.read() {
            return Err(ConsensusError::NotRunning.into());
        }

        // Basic block validation
        if !block.is_valid() {
            return Ok(false);
        }

        if validate_confidential_block(block).is_err() {
            return Ok(false);
        }

        // Check if the proposer is valid for this slot
        let expected_proposer =
            Self::get_proposer_for_slot(&self.config.validators, block.header.round);
        if expected_proposer != Some(block.header.creator) {
            return Ok(false);
        }

        // Check if block height is correct
        let latest_height = self.storage.get_latest_height()?;
        if block.header.round != latest_height + 1 {
            return Ok(false);
        }

        // Validate transactions
        for tx in &block.transactions {
            if !tx.is_valid() {
                return Ok(false);
            }
            if validate_confidential_transaction(tx).is_err() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Add a validator
    pub fn add_validator(&mut self, validator: Validator) {
        let validator_id = validator.id;
        self.config.validators.push(validator);
        info!("Added validator: {}", hex::encode(validator_id));
    }

    /// Remove a validator
    pub fn remove_validator(&mut self, validator_id: &[u8; 32]) {
        self.config.validators.retain(|v| v.id != *validator_id);
        info!("Removed validator: {}", hex::encode(validator_id));
    }

    /// Get validator list
    pub fn get_validators(&self) -> &[Validator] {
        &self.config.validators
    }
}

/// Legacy consensus engine trait for compatibility
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

    async fn propose_block(&self, transactions: Vec<Transaction>) -> Result<Block> {
        // This is a simplified version for the trait
        let latest_height = self.storage.get_latest_height()?;
        let prev_hash = if latest_height == 0 {
            [0u8; 32]
        } else {
            let prev_block = self
                .storage
                .get_block_by_height(latest_height)?
                .ok_or_else(|| anyhow::anyhow!("Previous block not found"))?;
            prev_block.hash()
        };

        Ok(Block::new(
            if latest_height == 0 {
                Vec::new()
            } else {
                vec![prev_hash]
            },
            transactions,
            latest_height + 1,
            self.validator_id,
        ))
    }

    async fn validate_block(&self, block: &Block) -> Result<bool> {
        PoAConsensus::validate_block(self, block).await
    }

    fn get_state(&self) -> ConsensusState {
        PoAConsensus::get_state(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use ippan_storage::{Account, MemoryStorage, SledStorage, Storage};
    use ippan_types::{ippan_time_init, ippan_time_now};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_poa_consensus() {
        let storage = Arc::new(MemoryStorage::new());
        let config = PoAConfig::default();
        let validator_id = [1u8; 32];

        let mut consensus = PoAConsensus::new(config, storage, validator_id);

        // Test starting and stopping
        consensus.start().await.unwrap();
        assert!(*consensus.is_running.read());

        consensus.stop().await.unwrap();
        assert!(!*consensus.is_running.read());
    }

    #[test]
    fn test_proposer_selection() {
        let validators = vec![
            Validator {
                id: [1u8; 32],
                address: [1u8; 32],
                stake: 1000,
                is_active: true,
            },
            Validator {
                id: [2u8; 32],
                address: [2u8; 32],
                stake: 2000,
                is_active: true,
            },
        ];

        // Test proposer rotation
        assert_eq!(
            PoAConsensus::get_proposer_for_slot(&validators, 0),
            Some([1u8; 32])
        );
        assert_eq!(
            PoAConsensus::get_proposer_for_slot(&validators, 1),
            Some([2u8; 32])
        );
        assert_eq!(
            PoAConsensus::get_proposer_for_slot(&validators, 2),
            Some([1u8; 32])
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_multi_node_block_production_with_transactions() {
        ippan_time_init();

        let temp_dir = tempdir().unwrap();
        let sled_storage = Arc::new(SledStorage::new(temp_dir.path()).unwrap());
        sled_storage.initialize().unwrap();
        let storage: Arc<dyn Storage + Send + Sync> = sled_storage.clone();

        let secret_one = [1u8; 32];
        let secret_two = [2u8; 32];
        let validator_one = SigningKey::from_bytes(&secret_one)
            .verifying_key()
            .to_bytes();
        let validator_two = SigningKey::from_bytes(&secret_two)
            .verifying_key()
            .to_bytes();

        storage
            .update_account(Account {
                address: validator_one,
                balance: 1_000_000,
                nonce: 0,
            })
            .unwrap();
        storage
            .update_account(Account {
                address: validator_two,
                balance: 1_000_000,
                nonce: 0,
            })
            .unwrap();

        let validators = vec![
            Validator {
                id: validator_one,
                address: validator_one,
                stake: 1_000,
                is_active: true,
            },
            Validator {
                id: validator_two,
                address: validator_two,
                stake: 1_000,
                is_active: true,
            },
        ];

        let config = PoAConfig {
            slot_duration_ms: 50,
            validators: validators.clone(),
            max_transactions_per_block: 10,
            block_reward: 5,
            finalization_interval_ms: 150,
        };

        let node_one = PoAConsensus::new(config.clone(), storage.clone(), validator_one);
        let node_two = PoAConsensus::new(config, storage.clone(), validator_two);

        let mempool_one = node_one.mempool();
        let mempool_two = node_two.mempool();

        let transfers_per_side = 20u64;
        for nonce in 0..transfers_per_side {
            let mut tx = Transaction::new(validator_one, validator_two, 10, nonce);
            tx.sign(&secret_one).unwrap();
            let _ = mempool_one.add_transaction(tx).unwrap();
        }
        for nonce in 0..transfers_per_side {
            let mut tx = Transaction::new(validator_two, validator_one, 10, nonce);
            tx.sign(&secret_two).unwrap();
            let _ = mempool_two.add_transaction(tx).unwrap();
        }

        let mut slot = 0u64;
        let mut produced_blocks = 0u64;
        loop {
            let empty1 = mempool_one.size() == 0;
            let empty2 = mempool_two.size() == 0;
            if empty1 && empty2 {
                break;
            }

            let proposer = PoAConsensus::get_proposer_for_slot(&validators, slot).unwrap();
            if proposer == validator_one {
                PoAConsensus::propose_block(
                    &storage,
                    &mempool_one,
                    &node_one.config,
                    &node_one.round_tracker,
                    slot,
                    validator_one,
                )
                .await
                .unwrap();
            } else {
                PoAConsensus::propose_block(
                    &storage,
                    &mempool_two,
                    &node_two.config,
                    &node_two.round_tracker,
                    slot,
                    validator_two,
                )
                .await
                .unwrap();
            }

            slot += 1;
            produced_blocks += 1;

            if produced_blocks > 64 {
                panic!("Exceeded expected block production while draining mempools");
            }
        }

        let expected_nonce = transfers_per_side;
        assert_eq!(mempool_one.size(), 0);
        assert_eq!(mempool_two.size(), 0);

        let latest_height = storage.get_latest_height().unwrap();
        assert!(latest_height >= 4);

        let mut total_transactions = 0usize;
        let mut proposer_one_blocks = 0usize;
        let mut proposer_two_blocks = 0usize;
        let mut block_summary = Vec::new();

        for height in 1..=latest_height {
            if let Some(block) = storage.get_block_by_height(height).unwrap() {
                total_transactions += block.transactions.len();
                if !block.transactions.is_empty() {
                    block_summary.push((
                        height,
                        block.transactions.len(),
                        hex::encode(block.header.creator),
                    ));
                }
                if block.header.creator == validator_one {
                    proposer_one_blocks += 1;
                } else if block.header.creator == validator_two {
                    proposer_two_blocks += 1;
                }
            }
        }

        let total_expected = (transfers_per_side * 2) as usize;
        assert_eq!(
            total_transactions, total_expected,
            "non_empty_blocks: {block_summary:?}"
        );
        assert!(proposer_one_blocks >= 2);
        assert!(proposer_two_blocks >= 2);

        let (account_one, account_two) = {
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let account_one = storage.get_account(&validator_one).unwrap().unwrap();
                let account_two = storage.get_account(&validator_two).unwrap().unwrap();

                if account_one.nonce == expected_nonce && account_two.nonce == expected_nonce {
                    break (account_one, account_two);
                }

                if Instant::now() >= deadline {
                    panic!(
                        "Timed out waiting for accounts to reach expected nonces: first={}, second={}",
                        account_one.nonce, account_two.nonce
                    );
                }

                sleep(Duration::from_millis(10)).await;
            }
        };

        let reward = 5u64;
        assert_eq!(
            account_one.balance,
            1_000_000 + reward * proposer_one_blocks as u64
        );
        assert_eq!(
            account_two.balance,
            1_000_000 + reward * proposer_two_blocks as u64
        );

        {
            let mut tracker = node_one.round_tracker.write();
            tracker.current_round_blocks = (1..=latest_height)
                .filter_map(|height| storage.get_block_by_height(height).unwrap())
                .map(|block| block.header.id)
                .collect();
            tracker.current_round = latest_height.saturating_add(1);
            tracker.round_start = Instant::now() - node_one.finalization_interval;
            let interval_us = node_one.finalization_interval.as_micros() as u64;
            let now_us = ippan_time_now();
            tracker.round_start_time = IppanTimeMicros(now_us.saturating_sub(interval_us));
        }

        PoAConsensus::finalize_round_if_ready(
            &storage,
            &node_one.round_tracker,
            node_one.finalization_interval,
        )
        .unwrap();

        let round_record = storage
            .get_latest_round_finalization()
            .unwrap()
            .expect("finalization record stored");
        assert_eq!(round_record.proof.block_ids.len(), latest_height as usize);
    }
}
