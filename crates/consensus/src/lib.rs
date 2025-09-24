use anyhow::Result;
use ippan_storage::{Account, Storage};
use ippan_types::{ippan_time_now, Block, HashTimer, IppanTimeMicros, Transaction};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

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
}

impl Default for PoAConfig {
    fn default() -> Self {
        Self {
            slot_duration_ms: 1000, // 1 second slots
            validators: vec![],
            max_transactions_per_block: 1000,
            block_reward: 10,
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
    mempool: Arc<RwLock<Vec<Transaction>>>,
}

impl PoAConsensus {
    /// Create a new PoA consensus engine
    pub fn new(
        config: PoAConfig,
        storage: Arc<dyn Storage + Send + Sync>,
        validator_id: [u8; 32],
    ) -> Self {
        let (tx_sender, tx_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            storage,
            validator_id,
            is_running: Arc::new(RwLock::new(false)),
            current_slot: Arc::new(RwLock::new(0)),
            tx_receiver: Some(tx_receiver),
            tx_sender,
            mempool: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get the transaction sender for submitting transactions
    pub fn get_tx_sender(&self) -> mpsc::UnboundedSender<Transaction> {
        self.tx_sender.clone()
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

            tokio::spawn(async move {
                let mut slot_interval = interval(Duration::from_millis(config.slot_duration_ms));

                loop {
                    if !*is_running.read() {
                        break;
                    }

                    // Process incoming transactions
                    while let Ok(tx) = tx_receiver.try_recv() {
                        mempool.write().push(tx);
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
            })
        };

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
        let is_proposing = proposer.map_or(false, |p| p == self.validator_id);

        ConsensusState {
            current_slot,
            current_proposer: proposer,
            is_proposing,
            validator_count: self.config.validators.len(),
            latest_block_height: self.storage.get_latest_height().unwrap_or(0),
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
        mempool: &Arc<RwLock<Vec<Transaction>>>,
        config: &PoAConfig,
        _slot: u64,
        proposer_id: [u8; 32],
    ) -> Result<()> {
        // Get transactions from mempool
        let block_transactions = {
            let mut transactions = mempool.write();
            let tx_count = transactions.len().min(config.max_transactions_per_block);
            transactions.drain(..tx_count).collect::<Vec<_>>()
        };

        // Get previous block hash
        let latest_height = storage.get_latest_height()?;
        let prev_hash = if latest_height == 0 {
            [0u8; 32] // Genesis block
        } else {
            let prev_block = storage
                .get_block_by_height(latest_height)?
                .ok_or_else(|| anyhow::anyhow!("Previous block not found"))?;
            prev_block.hash()
        };

        // Create new block
        let block = Block::new(
            prev_hash,
            block_transactions,
            latest_height + 1,
            proposer_id,
        );

        // Validate block
        if !block.is_valid() {
            return Err(anyhow::anyhow!("Invalid block"));
        }

        // Store block
        storage.store_block(block.clone())?;

        // Process transactions and update accounts
        Self::process_transactions(
            storage,
            &block.transactions,
            config.block_reward,
            proposer_id,
        )
        .await?;

        info!(
            "Proposed and stored block {} at height {} with {} transactions",
            hex::encode(block.hash()),
            block.header.round_id,
            block.transactions.len()
        );

        Ok(())
    }

    /// Process transactions and update account balances
    async fn process_transactions(
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

        // Check if the proposer is valid for this slot
        let expected_proposer =
            Self::get_proposer_for_slot(&self.config.validators, block.header.round_id);
        if expected_proposer != Some(block.header.proposer_id) {
            return Ok(false);
        }

        // Check if block height is correct
        let latest_height = self.storage.get_latest_height()?;
        if block.header.round_id != latest_height + 1 {
            return Ok(false);
        }

        // Validate transactions
        for tx in &block.transactions {
            if !tx.is_valid() {
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
            prev_hash,
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
    use ippan_storage::MemoryStorage;
    use std::sync::Arc;

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
}
