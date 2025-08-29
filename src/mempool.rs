use crate::crypto::Hash;
use crate::transaction::Transaction;
use crate::error::{Error, Result};
use dashmap::DashMap;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

const MAX_MEMPOOL_SIZE: usize = 1_000_000; // 1M transactions
const MAX_MEMPOOL_SIZE_PER_SHARD: usize = 100_000; // 100K transactions per shard

#[derive(Debug, Clone)]
pub struct MempoolEntry {
    pub transaction: Transaction,
    pub received_at: std::time::Instant,
    pub sort_key: (Hash, Hash), // (hash_timer, tx_id)
}

impl PartialEq for MempoolEntry {
    fn eq(&self, other: &Self) -> bool {
        self.sort_key == other.sort_key
    }
}

impl Eq for MempoolEntry {}

impl PartialOrd for MempoolEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MempoolEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for max-heap (smallest hash_timer first)
        self.sort_key.cmp(&other.sort_key).reverse()
    }
}

pub struct ShardMempool {
    priority_queue: Arc<RwLock<BinaryHeap<MempoolEntry>>>,
    account_nonces: Arc<DashMap<Hash, u64>>, // account_pub -> latest_nonce
    size: Arc<RwLock<usize>>,
}

impl ShardMempool {
    pub fn new() -> Self {
        Self {
            priority_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            account_nonces: Arc::new(DashMap::new()),
            size: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn add_transaction(&self, tx: Transaction) -> Result<bool, Error> {
        // Check if we're at capacity
        let current_size = *self.size.read().await;
        if current_size >= MAX_MEMPOOL_SIZE_PER_SHARD {
            warn!("Shard mempool at capacity, dropping transaction");
            return Ok(false);
        }

        // Get account public key hash for nonce tracking
        let account_pub_hash = crate::crypto::hash(&tx.from_pub);
        
        // Check nonce
        let latest_nonce = self.account_nonces.get(&account_pub_hash).map(|v| *v).unwrap_or(0);
        if tx.nonce <= latest_nonce {
            debug!("Transaction nonce too low: {} <= {}", tx.nonce, latest_nonce);
            return Ok(false);
        }

        // Create mempool entry
        let sort_key = tx.get_sort_key()?;
        let entry = MempoolEntry {
            transaction: tx,
            received_at: std::time::Instant::now(),
            sort_key,
        };

        // Add to priority queue
        {
            let mut queue = self.priority_queue.write().await;
            queue.push(entry);
        }

        // Update size
        {
            let mut size = self.size.write().await;
            *size += 1;
        }

        Ok(true)
    }

    pub async fn get_next_transactions(&self, count: usize) -> Vec<Transaction> {
        let mut transactions = Vec::new();
        let mut queue = self.priority_queue.write().await;
        let mut size = self.size.write().await;

        for _ in 0..count {
            if let Some(entry) = queue.pop() {
                transactions.push(entry.transaction);
                *size -= 1;
            } else {
                break;
            }
        }

        transactions
    }

    pub async fn remove_transaction(&self, tx_id: &Hash) -> bool {
        // This is a simplified implementation
        // In a real implementation, you'd need a more efficient way to remove specific transactions
        let mut queue = self.priority_queue.write().await;
        let mut size = self.size.write().await;
        
        let mut temp_queue = BinaryHeap::new();
        let mut found = false;
        
        while let Some(entry) = queue.pop() {
            if entry.transaction.compute_id().unwrap_or([0u8; 32]) == *tx_id {
                found = true;
                *size -= 1;
                break;
            } else {
                temp_queue.push(entry);
            }
        }
        
        // Restore remaining entries
        while let Some(entry) = temp_queue.pop() {
            queue.push(entry);
        }
        
        found
    }

    pub async fn update_account_nonce(&self, account_pub: &Hash, nonce: u64) {
        self.account_nonces.insert(*account_pub, nonce);
    }

    pub async fn get_size(&self) -> usize {
        *self.size.read().await
    }

    pub async fn clear(&self) {
        let mut queue = self.priority_queue.write().await;
        queue.clear();
        
        let mut size = self.size.write().await;
        *size = 0;
    }
}

pub struct Mempool {
    shards: Vec<ShardMempool>,
    shard_count: usize,
    total_size: Arc<RwLock<usize>>,
}

impl Mempool {
    pub fn new(shard_count: usize) -> Self {
        let shards: Vec<ShardMempool> = (0..shard_count)
            .map(|_| ShardMempool::new())
            .collect();

        Self {
            shards,
            shard_count,
            total_size: Arc::new(RwLock::new(0)),
        }
    }

    fn get_shard_for_transaction(&self, tx: &Transaction) -> usize {
        // Simple sharding based on sender's public key
        let account_hash = crate::crypto::hash(&tx.from_pub);
        (account_hash[0] as usize) % self.shard_count
    }

    pub async fn add_transaction(&self, tx: Transaction) -> Result<bool, Error> {
        // Verify transaction first
        tx.verify()?;

        // Check total mempool size
        let total_size = *self.total_size.read().await;
        if total_size >= MAX_MEMPOOL_SIZE {
            warn!("Global mempool at capacity, dropping transaction");
            return Ok(false);
        }

        // Get appropriate shard
        let shard_index = self.get_shard_for_transaction(&tx);
        let shard = &self.shards[shard_index];

        // Add to shard
        let added = shard.add_transaction(tx).await?;
        
        if added {
            let mut total_size = self.total_size.write().await;
            *total_size += 1;
        }

        Ok(added)
    }

    pub async fn get_transactions_for_block(&self, target_size: usize) -> Vec<Transaction> {
        let mut all_transactions = Vec::new();
        let mut remaining_size = target_size;

        // Collect transactions from all shards in round-robin fashion
        let mut shard_index = 0;
        while remaining_size > 0 && all_transactions.len() < target_size {
            let shard = &self.shards[shard_index];
            let batch_size = std::cmp::min(remaining_size, 100); // Batch size per shard
            
            let transactions = shard.get_next_transactions(batch_size).await;
            if transactions.is_empty() {
                break;
            }
            
            all_transactions.extend(transactions);
            remaining_size = remaining_size.saturating_sub(batch_size);
            
            shard_index = (shard_index + 1) % self.shard_count;
        }

        // Update total size
        let removed_count = all_transactions.len();
        if removed_count > 0 {
            let mut total_size = self.total_size.write().await;
            *total_size = total_size.saturating_sub(removed_count);
        }

        all_transactions
    }

    pub async fn remove_transaction(&self, tx_id: &Hash) -> bool {
        // Try to remove from all shards (inefficient but simple)
        for shard in &self.shards {
            if shard.remove_transaction(tx_id).await {
                let mut total_size = self.total_size.write().await;
                *total_size = total_size.saturating_sub(1);
                return true;
            }
        }
        false
    }

    pub async fn update_account_nonce(&self, account_pub: &Hash, nonce: u64) {
        // Update nonce in all shards (for simplicity)
        for shard in &self.shards {
            shard.update_account_nonce(account_pub, nonce).await;
        }
    }

    pub async fn get_total_size(&self) -> usize {
        *self.total_size.read().await
    }

    pub async fn get_shard_sizes(&self) -> Vec<usize> {
        let mut sizes = Vec::new();
        for shard in &self.shards {
            sizes.push(shard.get_size().await);
        }
        sizes
    }

    pub async fn clear(&self) {
        for shard in &self.shards {
            shard.clear().await;
        }
        
        let mut total_size = self.total_size.write().await;
        *total_size = 0;
    }

    pub fn get_shard_count(&self) -> usize {
        self.shard_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::KeyPair;
    use crate::time::IppanTime;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_mempool_creation() {
        let mempool = Mempool::new(4);
        assert_eq!(mempool.get_shard_count(), 4);
        assert_eq!(mempool.get_total_size().await, 0);
    }

    #[tokio::test]
    async fn test_transaction_addition() {
        let mempool = Mempool::new(2);
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        let added = mempool.add_transaction(tx).await.unwrap();
        assert!(added);
        assert_eq!(mempool.get_total_size().await, 1);
    }

    #[tokio::test]
    async fn test_transaction_retrieval() {
        let mempool = Mempool::new(1);
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        let tx = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            ippan_time,
        ).unwrap();
        
        mempool.add_transaction(tx).await.unwrap();
        
        let transactions = mempool.get_transactions_for_block(10).await;
        assert_eq!(transactions.len(), 1);
        assert_eq!(mempool.get_total_size().await, 0);
    }

    #[tokio::test]
    async fn test_nonce_validation() {
        let mempool = Mempool::new(1);
        let keypair = KeyPair::generate();
        let recipient = KeyPair::generate();
        let ippan_time = Arc::new(IppanTime::new());
        
        // Add transaction with nonce 1
        let tx1 = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            1,
            Arc::clone(&ippan_time),
        ).unwrap();
        
        mempool.add_transaction(tx1).await.unwrap();
        
        // Try to add transaction with nonce 0 (should be rejected)
        let tx2 = Transaction::new(
            &keypair,
            recipient.public_key,
            1000,
            0,
            ippan_time,
        ).unwrap();
        
        let added = mempool.add_transaction(tx2).await.unwrap();
        assert!(!added);
    }
}
