use ippan_common::{Transaction, Result, crypto::Hash, PublicKeyBytes};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// Mempool entry with priority ordering
#[derive(Debug, Clone)]
struct MempoolEntry {
    tx: Transaction,
    sort_key: (Hash, Hash), // (hashtimer, tx_id)
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
        // Higher priority (lower hashtimer) comes first
        self.sort_key.cmp(&other.sort_key).reverse()
    }
}

/// Sharded mempool implementation
pub struct Mempool {
    shards: Vec<Arc<RwLock<BinaryHeap<MempoolEntry>>>>,
    account_nonces: DashMap<PublicKeyBytes, u64>,
    shard_count: usize,
    max_size_per_shard: usize,
}

impl Mempool {
    pub fn new(shards: usize) -> Self {
        let mut shard_queues = Vec::with_capacity(shards);
        for _ in 0..shards {
            shard_queues.push(Arc::new(RwLock::new(BinaryHeap::new())));
        }

        Self {
            shards: shard_queues,
            account_nonces: DashMap::new(),
            shard_count: shards,
            max_size_per_shard: 1_000_000, // 1M transactions per shard for high-TPS
        }
    }

    /// Add a transaction to the mempool
    pub async fn add_transaction(&mut self, tx: Transaction) -> Result<bool> {
        // Verify transaction signature
        if !tx.verify()? {
            return Ok(false);
        }

        // Check nonce
        let current_nonce = self.account_nonces
            .get(&tx.from_pub)
            .map(|n| *n)
            .unwrap_or(0);

        if tx.nonce <= current_nonce {
            return Ok(false);
        }

        // Get sort key for priority ordering
        let sort_key = tx.get_sort_key()?;
        
        // Determine shard based on sender's public key
        let shard_index = self.get_shard_index(&tx.from_pub);
        let shard = &self.shards[shard_index];

        // Check if shard is full
        {
            let queue = shard.read();
            if queue.len() >= self.max_size_per_shard {
                return Ok(false);
            }
        }

        // Add transaction to shard
        {
            let mut queue = shard.write();
            let entry = MempoolEntry { tx: tx.clone(), sort_key };
            queue.push(entry);
        }

        // Update account nonce
        self.account_nonces.insert(tx.from_pub, tx.nonce);

        Ok(true)
    }

    /// Get transactions for block building
    pub async fn get_transactions_for_block(&self, max_count: usize) -> Vec<Transaction> {
        let mut transactions = Vec::new();
        let mut shard_indices: Vec<usize> = (0..self.shard_count).collect();

        // Round-robin through shards
        while transactions.len() < max_count && !shard_indices.is_empty() {
            for &shard_idx in &shard_indices {
                if transactions.len() >= max_count {
                    break;
                }

                let shard = &self.shards[shard_idx];
                let mut queue = shard.write();
                
                if let Some(entry) = queue.pop() {
                    transactions.push(entry.tx);
                }
            }

            // Remove empty shards
            shard_indices.retain(|&idx| {
                let shard = &self.shards[idx];
                let queue = shard.read();
                !queue.is_empty()
            });
        }

        transactions
    }

    /// Get mempool size
    pub fn size(&self) -> usize {
        self.shards.iter()
            .map(|shard| shard.read().len())
            .sum()
    }

    /// Get shard index for a public key
    fn get_shard_index(&self, pub_key: &PublicKeyBytes) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash as _, Hasher};
        
        let mut hasher = DefaultHasher::new();
        pub_key.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }

    /// Remove transactions that are no longer valid
    pub async fn cleanup(&mut self) {
        for shard in &self.shards {
            let mut queue = shard.write();
            let mut valid_entries = Vec::new();

            while let Some(entry) = queue.pop() {
                // Check if transaction is still valid (synchronously)
                if self.is_transaction_valid_sync(&entry.tx) {
                    valid_entries.push(entry);
                }
            }

            // Re-add valid transactions
            for entry in valid_entries {
                queue.push(entry);
            }
        }
    }

    /// Check if a transaction is still valid (synchronous version)
    fn is_transaction_valid_sync(&self, tx: &Transaction) -> bool {
        // Check if signature is still valid
        if let Err(_) = tx.verify() {
            return false;
        }

        // Check if nonce is still valid
        let current_nonce = self.account_nonces
            .get(&tx.from_pub)
            .map(|n| *n)
            .unwrap_or(0);

        tx.nonce > current_nonce
    }

    /// Get statistics about the mempool
    pub fn get_stats(&self) -> MempoolStats {
        let mut total_size = 0;
        let mut shard_sizes = Vec::new();

        for shard in &self.shards {
            let size = shard.read().len();
            total_size += size;
            shard_sizes.push(size);
        }

        MempoolStats {
            total_transactions: total_size,
            shard_sizes,
            account_count: self.account_nonces.len(),
        }
    }
}

#[derive(Debug)]
pub struct MempoolStats {
    pub total_transactions: usize,
    pub shard_sizes: Vec<usize>,
    pub account_count: usize,
}
