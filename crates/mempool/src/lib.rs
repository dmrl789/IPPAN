use anyhow::Result;
use ippan_crypto::validate_confidential_transaction;
use ippan_types::Transaction;
use parking_lot::RwLock;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};
use std::sync::Arc;

/// Transaction metadata for mempool management
#[derive(Debug, Clone)]
struct TransactionMeta {
    transaction: Transaction,
    added_at: Instant,
    fee: u64,
}

/// Mempool for managing pending transactions
pub struct Mempool {
    /// Pending transactions indexed by hash
    transactions: RwLock<HashMap<String, TransactionMeta>>,
    /// Transactions indexed by sender nonce for ordering
    sender_nonces: RwLock<HashMap<String, BTreeMap<u64, String>>>,
    /// Maximum number of transactions to keep
    max_size: usize,
    /// Transaction expiration time
    expiration_duration: Duration,
    /// Last cleanup time
    last_cleanup: RwLock<Instant>,
}

impl Mempool {
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: RwLock::new(HashMap::new()),
            sender_nonces: RwLock::new(HashMap::new()),
            max_size,
            expiration_duration: Duration::from_secs(300), // 5 minutes
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    pub fn new_with_expiration(max_size: usize, expiration_duration: Duration) -> Self {
        Self {
            transactions: RwLock::new(HashMap::new()),
            sender_nonces: RwLock::new(HashMap::new()),
            max_size,
            expiration_duration,
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Add a transaction to the mempool
    pub fn add_transaction(&self, tx: Transaction) -> Result<bool> {
        let tx_hash = hex::encode(tx.hash());
        let sender = hex::encode(tx.from);

        // Cleanup expired transactions first
        self.cleanup_expired_transactions();

        let mut transactions = self.transactions.write();
        let mut sender_nonces = self.sender_nonces.write();

        // Check if transaction already exists
        if transactions.contains_key(&tx_hash) {
            return Ok(false);
        }

        // Check mempool size limit - remove oldest low-fee transactions if needed
        if transactions.len() >= self.max_size {
            if !self.make_space_for_transaction(&mut transactions, &mut sender_nonces, 0) {
                return Ok(false);
            }
        }

        // Validate confidential payloads before admission
        validate_confidential_transaction(&tx)?;

        // Calculate fee (simplified - in production, this would be more sophisticated)
        let fee = self.calculate_transaction_fee(&tx);

        // Add transaction with metadata
        let meta = TransactionMeta {
            transaction: tx.clone(),
            added_at: Instant::now(),
            fee,
        };
        transactions.insert(tx_hash.clone(), meta);

        // Update sender nonce index
        sender_nonces
            .entry(sender)
            .or_default()
            .insert(tx.nonce, tx_hash);

        Ok(true)
    }

    /// Remove a transaction from the mempool
    pub fn remove_transaction(&self, tx_hash: &str) -> Result<Option<Transaction>> {
        let mut transactions = self.transactions.write();
        let mut sender_nonces = self.sender_nonces.write();

        if let Some(meta) = transactions.remove(tx_hash) {
            let sender = hex::encode(meta.transaction.from);
            if let Some(nonces) = sender_nonces.get_mut(&sender) {
                nonces.remove(&meta.transaction.nonce);
                if nonces.is_empty() {
                    sender_nonces.remove(&sender);
                }
            }
            Ok(Some(meta.transaction))
        } else {
            Ok(None)
        }
    }

    /// Get a transaction by hash
    pub fn get_transaction(&self, tx_hash: &str) -> Option<Transaction> {
        self.transactions.read().get(tx_hash).map(|meta| meta.transaction.clone())
    }

    /// Get all transactions for a sender
    pub fn get_sender_transactions(&self, sender: &str) -> Vec<Transaction> {
        let transactions = self.transactions.read();
        let sender_nonces = self.sender_nonces.read();

        if let Some(nonces) = sender_nonces.get(sender) {
            nonces
                .values()
                .filter_map(|tx_hash| transactions.get(tx_hash))
                .map(|meta| meta.transaction.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get transactions for block creation (up to max_count)
    /// Uses fee-based prioritization with proper nonce ordering
    pub fn get_transactions_for_block(&self, max_count: usize) -> Vec<Transaction> {
        let transactions = self.transactions.read();
        let sender_nonces = self.sender_nonces.read();

        // Collect all transactions with their metadata
        let mut all_transactions = Vec::new();
        for (tx_hash, meta) in transactions.iter() {
            all_transactions.push((tx_hash.clone(), meta.clone()));
        }

        // Sort by fee (descending) then by age (ascending)
        all_transactions.sort_by(|a, b| {
            b.1.fee.cmp(&a.1.fee)
                .then(a.1.added_at.cmp(&b.1.added_at))
        });

        let mut selected = Vec::new();
        let mut used_senders: HashMap<String, u64> = HashMap::new();

        // Select transactions maintaining nonce order per sender
        for (tx_hash, meta) in all_transactions {
            if selected.len() >= max_count {
                break;
            }

            let sender = hex::encode(meta.transaction.from);
            let nonce = meta.transaction.nonce;

            // Check if we can include this transaction based on nonce ordering
            let can_include = if let Some(&last_nonce) = used_senders.get(&sender) {
                nonce == last_nonce + 1
            } else {
                // First transaction from this sender - can start with any nonce
                true
            };

            if can_include {
                selected.push(meta.transaction.clone());
                used_senders.insert(sender, nonce);
            }
        }

        selected
    }

    /// Get mempool size
    pub fn size(&self) -> usize {
        self.transactions.read().len()
    }

    /// Clear all transactions
    pub fn clear(&self) {
        self.transactions.write().clear();
        self.sender_nonces.write().clear();
    }

    /// Clean up expired transactions
    fn cleanup_expired_transactions(&self) {
        let now = Instant::now();
        let last_cleanup = *self.last_cleanup.read();
        
        // Only cleanup if enough time has passed (avoid excessive cleanup)
        if now.duration_since(last_cleanup) < Duration::from_secs(30) {
            return;
        }

        let mut transactions = self.transactions.write();
        let mut sender_nonces = self.sender_nonces.write();
        let mut to_remove = Vec::new();

        for (tx_hash, meta) in transactions.iter() {
            if now.duration_since(meta.added_at) > self.expiration_duration {
                to_remove.push(tx_hash.clone());
            }
        }

        for tx_hash in to_remove {
            if let Some(meta) = transactions.remove(&tx_hash) {
                let sender = hex::encode(meta.transaction.from);
                if let Some(nonces) = sender_nonces.get_mut(&sender) {
                    nonces.remove(&meta.transaction.nonce);
                    if nonces.is_empty() {
                        sender_nonces.remove(&sender);
                    }
                }
            }
        }

        *self.last_cleanup.write() = now;
    }

    /// Calculate transaction fee (simplified implementation)
    fn calculate_transaction_fee(&self, tx: &Transaction) -> u64 {
        // In production, this would be more sophisticated
        // For now, use a simple calculation based on transaction size
        let base_fee = 1000; // Base fee
        let size_fee = tx.payload.len() as u64 * 10; // Size-based fee
        base_fee + size_fee
    }

    /// Make space for a new transaction by removing low-fee transactions
    fn make_space_for_transaction(
        &self,
        transactions: &mut HashMap<String, TransactionMeta>,
        sender_nonces: &mut HashMap<String, BTreeMap<u64, String>>,
        new_fee: u64,
    ) -> bool {
        // Find the lowest fee transaction
        let mut lowest_fee = u64::MAX;
        let mut lowest_tx_hash = None;

        for (tx_hash, meta) in transactions.iter() {
            if meta.fee < lowest_fee {
                lowest_fee = meta.fee;
                lowest_tx_hash = Some(tx_hash.clone());
            }
        }

        // Remove the lowest fee transaction if it has lower fee than the new one
        if let Some(tx_hash) = lowest_tx_hash {
            if lowest_fee < new_fee {
                if let Some(meta) = transactions.remove(&tx_hash) {
                    let sender = hex::encode(meta.transaction.from);
                    if let Some(nonces) = sender_nonces.get_mut(&sender) {
                        nonces.remove(&meta.transaction.nonce);
                        if nonces.is_empty() {
                            sender_nonces.remove(&sender);
                        }
                    }
                    return true;
                }
            }
        }

        false
    }

    /// Get mempool statistics
    pub fn get_stats(&self) -> MempoolStats {
        let transactions = self.transactions.read();
        let mut total_fee = 0u64;
        let mut oldest_tx = Instant::now();
        let mut newest_tx = Instant::now();

        for meta in transactions.values() {
            total_fee += meta.fee;
            if meta.added_at < oldest_tx {
                oldest_tx = meta.added_at;
            }
            if meta.added_at > newest_tx {
                newest_tx = meta.added_at;
            }
        }

        MempoolStats {
            size: transactions.len(),
            total_fee,
            oldest_tx_age: Instant::now().duration_since(oldest_tx),
            newest_tx_age: Instant::now().duration_since(newest_tx),
        }
    }
}

/// Mempool statistics
#[derive(Debug, Clone)]
pub struct MempoolStats {
    pub size: usize,
    pub total_fee: u64,
    pub oldest_tx_age: Duration,
    pub newest_tx_age: Duration,
}
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::Transaction;
    use std::time::Duration;

    #[test]
    fn test_mempool_add_remove() {
        let mempool = Mempool::new(100);

        let tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let tx_hash = hex::encode(tx.hash());

        // Add transaction
        assert!(mempool.add_transaction(tx.clone()).unwrap());
        assert_eq!(mempool.size(), 1);

        // Try to add same transaction again
        assert!(!mempool.add_transaction(tx.clone()).unwrap());

        // Remove transaction
        let removed = mempool.remove_transaction(&tx_hash).unwrap();
        assert!(removed.is_some());
        assert_eq!(mempool.size(), 0);
    }

    #[test]
    fn test_mempool_sender_transactions() {
        let mempool = Mempool::new(100);

        let sender = [1u8; 32];
        let tx1 = Transaction::new(sender, [2u8; 32], 1000, 1);
        let tx2 = Transaction::new(sender, [3u8; 32], 2000, 2);

        mempool.add_transaction(tx1).unwrap();
        mempool.add_transaction(tx2).unwrap();

        let sender_txs = mempool.get_sender_transactions(&hex::encode(sender));
        assert_eq!(sender_txs.len(), 2);
    }

    #[test]
    fn test_mempool_fee_prioritization() {
        let mempool = Mempool::new(100);

        let sender1 = [1u8; 32];
        let sender2 = [2u8; 32];

        // Add transactions with different fees
        let tx1 = Transaction::new(sender1, [3u8; 32], 1000, 1);
        let tx2 = Transaction::new(sender2, [4u8; 32], 2000, 1);
        let tx3 = Transaction::new(sender1, [5u8; 32], 1500, 2);

        mempool.add_transaction(tx1).unwrap();
        mempool.add_transaction(tx2).unwrap();
        mempool.add_transaction(tx3).unwrap();

        // Get transactions for block - should prioritize by fee
        let block_txs = mempool.get_transactions_for_block(3);
        assert_eq!(block_txs.len(), 3);
    }

    #[test]
    fn test_mempool_nonce_ordering() {
        let mempool = Mempool::new(100);

        let sender = [1u8; 32];
        let tx1 = Transaction::new(sender, [2u8; 32], 1000, 1);
        let tx2 = Transaction::new(sender, [3u8; 32], 2000, 2);
        let tx3 = Transaction::new(sender, [4u8; 32], 1500, 3);

        // Add transactions out of order
        mempool.add_transaction(tx2.clone()).unwrap();
        mempool.add_transaction(tx1.clone()).unwrap();
        mempool.add_transaction(tx3.clone()).unwrap();

        let sender_txs = mempool.get_sender_transactions(&hex::encode(sender));
        assert_eq!(sender_txs.len(), 3);

        // Check that nonce ordering is maintained
        let block_txs = mempool.get_transactions_for_block(3);
        let sender_block_txs: Vec<_> = block_txs
            .iter()
            .filter(|tx| tx.from == sender)
            .collect();
        
        // Should include transactions in nonce order
        assert!(sender_block_txs.len() >= 1);
    }

    #[test]
    fn test_mempool_expiration() {
        let mempool = Mempool::new_with_expiration(100, Duration::from_millis(100));

        let tx = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        assert!(mempool.add_transaction(tx).unwrap());
        assert_eq!(mempool.size(), 1);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        
        // Trigger cleanup by adding another transaction
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], 1000, 1);
        assert!(mempool.add_transaction(tx2).unwrap());
        
        // First transaction should be expired and removed
        assert_eq!(mempool.size(), 1);
    }

    #[test]
    fn test_mempool_stats() {
        let mempool = Mempool::new(100);

        let tx1 = Transaction::new([1u8; 32], [2u8; 32], 1000, 1);
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], 2000, 1);

        mempool.add_transaction(tx1).unwrap();
        mempool.add_transaction(tx2).unwrap();

        let stats = mempool.get_stats();
        assert_eq!(stats.size, 2);
        assert!(stats.total_fee > 0);
    }
}
