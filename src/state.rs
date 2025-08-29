use crate::block::Block;
use crate::crypto::Hash;
use crate::error::{Error, Result};
use crate::transaction::Transaction;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
}

impl AccountState {
    pub fn new() -> Self {
        Self {
            balance: 0,
            nonce: 0,
        }
    }

    pub fn with_balance(balance: u64) -> Self {
        Self {
            balance,
            nonce: 0,
        }
    }
}

impl Default for AccountState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub accounts: HashMap<Hash, AccountState>,
    pub state_root: Hash,
    pub round_id: u64,
    pub timestamp_us: u64,
}

pub struct StateManager {
    accounts: Arc<DashMap<Hash, AccountState>>,
    state_root: Arc<RwLock<Hash>>,
    last_snapshot_round: Arc<RwLock<u64>>,
    snapshot_interval: u64, // Create snapshot every N rounds
    snapshot_path: Option<String>,
}

impl StateManager {
    pub fn new(snapshot_interval: u64) -> Self {
        Self {
            accounts: Arc::new(DashMap::new()),
            state_root: Arc::new(RwLock::new([0u8; 32])),
            last_snapshot_round: Arc::new(RwLock::new(0)),
            snapshot_interval,
            snapshot_path: None,
        }
    }

    pub fn with_snapshot_path(mut self, path: String) -> Self {
        self.snapshot_path = Some(path);
        self
    }

    pub async fn get_account_state(&self, account_pub: &Hash) -> Option<AccountState> {
        self.accounts.get(account_pub).map(|state| state.clone())
    }

    pub async fn get_balance(&self, account_pub: &Hash) -> u64 {
        self.accounts
            .get(account_pub)
            .map(|state| state.balance)
            .unwrap_or(0)
    }

    pub async fn get_nonce(&self, account_pub: &Hash) -> u64 {
        self.accounts
            .get(account_pub)
            .map(|state| state.nonce)
            .unwrap_or(0)
    }

    pub async fn set_balance(&self, account_pub: &Hash, balance: u64) {
        let mut state = self.accounts.entry(*account_pub).or_insert_with(AccountState::new);
        state.balance = balance;
    }

    pub async fn set_nonce(&self, account_pub: &Hash, nonce: u64) {
        let mut state = self.accounts.entry(*account_pub).or_insert_with(AccountState::new);
        state.nonce = nonce;
    }

    pub async fn apply_transaction(&self, tx: &Transaction) -> Result<bool, Error> {
        let from_pub_hash = crate::crypto::hash(&tx.from_pub);
        let to_pub_hash = crate::crypto::hash(&tx.to_addr);

        // Get current account states
        let from_balance = self.get_balance(&from_pub_hash).await;
        let from_nonce = self.get_nonce(&from_pub_hash).await;

        // Validate transaction
        if tx.nonce != from_nonce + 1 {
            return Err(Error::State(format!(
                "Invalid nonce: expected {}, got {}",
                from_nonce + 1,
                tx.nonce
            )));
        }

        if tx.amount > from_balance {
            return Err(Error::State(format!(
                "Insufficient balance: {} (required: {})",
                from_balance,
                tx.amount
            )));
        }

        // Apply transaction
        let new_from_balance = from_balance - tx.amount;
        let to_balance = self.get_balance(&to_pub_hash).await;
        let new_to_balance = to_balance + tx.amount;

        // Update accounts atomically
        self.set_balance(&from_pub_hash, new_from_balance).await;
        self.set_nonce(&from_pub_hash, tx.nonce).await;
        self.set_balance(&to_pub_hash, new_to_balance).await;

        debug!(
            "Applied transaction: {} -> {} (amount: {}, nonce: {})",
            hex::encode(&from_pub_hash[..8]),
            hex::encode(&to_pub_hash[..8]),
            tx.amount,
            tx.nonce
        );

        Ok(true)
    }

    pub async fn apply_block(&self, block: &Block, transactions: &[Transaction]) -> Result<usize, Error> {
        let mut applied_count = 0;

        // Sort transactions by HashTimer for deterministic ordering
        let mut tx_with_timers: Vec<_> = transactions
            .iter()
            .enumerate()
            .map(|(i, tx)| {
                let sort_key = tx.get_sort_key()?;
                Ok((sort_key, i))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        tx_with_timers.sort_by_key(|(sort_key, _)| *sort_key);

        // Apply transactions in sorted order
        for (_, tx_index) in tx_with_timers {
            let tx = &transactions[tx_index];
            let applied = self.apply_transaction(tx).await?;
            if applied {
                applied_count += 1;
            }
        }

        // Update state root
        self.update_state_root().await;

        info!(
            "Applied block with {} transactions (block_id: {})",
            applied_count,
            hex::encode(&block.compute_id()?[..8])
        );

        Ok(applied_count)
    }

    pub async fn apply_finalized_round(&self, blocks: &[&Block], transactions: &[Transaction]) -> Result<usize, Error> {
        let mut total_applied = 0;

        // Sort blocks by HashTimer for deterministic ordering
        let mut block_with_timers: Vec<_> = blocks
            .iter()
            .enumerate()
            .map(|(i, block)| {
                let sort_key = block.get_sort_key()?;
                Ok((sort_key, i))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        block_with_timers.sort_by_key(|(sort_key, _)| *sort_key);

        // Apply blocks in sorted order
        for (_, block_index) in block_with_timers {
            let block = blocks[block_index];
            
            // Find transactions for this block
            let block_txs: Vec<&Transaction> = transactions
                .iter()
                .filter(|tx| {
                    if let Ok(tx_id) = tx.compute_id() {
                        block.transactions.contains(&tx_id)
                    } else {
                        false
                    }
                })
                .collect();

            let applied = self.apply_block(block, &block_txs).await?;
            total_applied += applied;
        }

        info!("Applied finalized round with {} total transactions", total_applied);
        Ok(total_applied)
    }

    async fn update_state_root(&self) {
        // Simple state root computation - hash of all account states
        let mut accounts_vec: Vec<_> = self.accounts.iter().collect();
        accounts_vec.sort_by_key(|(pub_hash, _)| **pub_hash);

        let mut hasher = crate::crypto::blake3_hash(&[]);
        for (pub_hash, state) in accounts_vec {
            let mut data = Vec::new();
            data.extend_from_slice(pub_hash);
            data.extend_from_slice(&state.balance.to_le_bytes());
            data.extend_from_slice(&state.nonce.to_le_bytes());
            hasher = crate::crypto::blake3_hash(&data);
        }

        *self.state_root.write().await = hasher;
    }

    pub async fn get_state_root(&self) -> Hash {
        *self.state_root.read().await
    }

    pub async fn create_snapshot(&self, round_id: u64, timestamp_us: u64) -> Result<StateSnapshot, Error> {
        let accounts: HashMap<Hash, AccountState> = self
            .accounts
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();

        let state_root = self.get_state_root().await;

        let snapshot = StateSnapshot {
            accounts,
            state_root,
            round_id,
            timestamp_us,
        };

        // Save to disk if path is configured
        if let Some(ref path) = self.snapshot_path {
            self.save_snapshot_to_disk(&snapshot, path).await?;
        }

        *self.last_snapshot_round.write().await = round_id;
        info!("Created state snapshot for round {}", round_id);

        Ok(snapshot)
    }

    async fn save_snapshot_to_disk(&self, snapshot: &StateSnapshot, path: &str) -> Result<(), Error> {
        let data = bincode::serialize(snapshot)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        tokio::fs::write(path, data).await
            .map_err(|e| Error::Io(e))?;

        Ok(())
    }

    pub async fn load_snapshot_from_disk(&self, path: &str) -> Result<StateSnapshot, Error> {
        let data = tokio::fs::read(path).await
            .map_err(|e| Error::Io(e))?;

        let snapshot: StateSnapshot = bincode::deserialize(&data)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Restore state from snapshot
        self.accounts.clear();
        for (pub_hash, state) in &snapshot.accounts {
            self.accounts.insert(*pub_hash, state.clone());
        }

        *self.state_root.write().await = snapshot.state_root;
        *self.last_snapshot_round.write().await = snapshot.round_id;

        info!("Loaded state snapshot from round {}", snapshot.round_id);
        Ok(snapshot)
    }

    pub async fn should_create_snapshot(&self, round_id: u64) -> bool {
        let last_snapshot = *self.last_snapshot_round.read().await;
        round_id >= last_snapshot + self.snapshot_interval
    }

    pub async fn get_account_count(&self) -> usize {
        self.accounts.len()
    }

    pub async fn get_total_balance(&self) -> u64 {
        self.accounts.iter().map(|entry| entry.value().balance).sum()
    }

    pub async fn clear(&self) {
        self.accounts.clear();
        *self.state_root.write().await = [0u8; 32];
        *self.last_snapshot_round.write().await = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::transaction::Transaction;
    use crate::time::IppanTime;
    use crate::block::Block;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_account_state_creation() {
        let state = StateManager::new(10);
        
        let account_pub = [1u8; 32];
        state.set_balance(&account_pub, 1000).await;
        state.set_nonce(&account_pub, 5).await;
        
        assert_eq!(state.get_balance(&account_pub).await, 1000);
        assert_eq!(state.get_nonce(&account_pub).await, 5);
    }

    #[tokio::test]
    async fn test_transaction_application() {
        let state = StateManager::new(10);
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        // Set initial balance
        let from_pub_hash = crate::crypto::hash(&keypair.public_key);
        state.set_balance(&from_pub_hash, 1000).await;
        state.set_nonce(&from_pub_hash, 0).await;
        
        // Create and apply transaction
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            500,
            1,
            ippan_time,
        ).unwrap();
        
        let applied = state.apply_transaction(&tx).await.unwrap();
        assert!(applied);
        
        // Check balances
        let to_pub_hash = crate::crypto::hash(&recipient.public_key);
        assert_eq!(state.get_balance(&from_pub_hash).await, 500);
        assert_eq!(state.get_balance(&to_pub_hash).await, 500);
        assert_eq!(state.get_nonce(&from_pub_hash).await, 1);
    }

    #[tokio::test]
    async fn test_block_application() {
        let state = StateManager::new(10);
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        // Set initial balance
        let from_pub_hash = crate::crypto::hash(&keypair.public_key);
        state.set_balance(&from_pub_hash, 1000).await;
        state.set_nonce(&from_pub_hash, 0).await;
        
        // Create transaction
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            500,
            1,
            ippan_time,
        ).unwrap();
        
        // Create block
        let block = Block::new(
            vec![],
            1,
            1234567890,
            [1u8; 32],
            vec![tx.clone()],
        ).unwrap();
        
        // Apply block
        let applied = state.apply_block(&block, &[tx]).await.unwrap();
        assert_eq!(applied, 1);
        
        // Check state root was updated
        let state_root = state.get_state_root().await;
        assert_ne!(state_root, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_snapshot_creation() {
        let state = StateManager::new(5);
        let account_pub = [1u8; 32];
        state.set_balance(&account_pub, 1000).await;
        
        let snapshot = state.create_snapshot(1, 1234567890).await.unwrap();
        assert_eq!(snapshot.round_id, 1);
        assert_eq!(snapshot.accounts.len(), 1);
        assert_eq!(snapshot.accounts[&account_pub].balance, 1000);
    }

    #[tokio::test]
    async fn test_insufficient_balance() {
        let state = StateManager::new(10);
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        // Set insufficient balance
        let from_pub_hash = crate::crypto::hash(&keypair.public_key);
        state.set_balance(&from_pub_hash, 100).await;
        state.set_nonce(&from_pub_hash, 0).await;
        
        // Try to send more than balance
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            500,
            1,
            ippan_time,
        ).unwrap();
        
        let result = state.apply_transaction(&tx).await;
        assert!(result.is_err());
    }
}
