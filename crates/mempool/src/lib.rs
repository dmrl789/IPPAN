//! # IPPAN Mempool
//!
//! Production-grade transaction mempool for the IPPAN blockchain.
//!
//! ## Features
//! - **Fee-based prioritization**
//! - **Nonce ordering**
//! - **Automatic expiration**
//! - **Size limits**
//! - **Thread-safe**
//! - **Confidential transaction support**

use anyhow::Result;
use ippan_crypto::validate_confidential_transaction;
use ippan_types::Transaction;
use parking_lot::RwLock;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::time::{Duration, Instant};

/// Transaction metadata
#[derive(Debug, Clone)]
struct TransactionMeta {
    transaction: Transaction,
    added_at: Instant,
    fee: u64,
}

/// Candidate for block inclusion
#[derive(Clone, Debug, PartialEq, Eq)]
struct BlockCandidate {
    fee: u64,
    added_at: Instant,
    sender: String,
    nonce: u64,
    tx_hash: String,
}

impl Ord for BlockCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fee
            .cmp(&other.fee)
            .then_with(|| other.added_at.cmp(&self.added_at))
            .then_with(|| self.tx_hash.cmp(&other.tx_hash))
    }
}

impl PartialOrd for BlockCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Thread-safe, fee-prioritized, nonce-ordered mempool
pub struct Mempool {
    transactions: RwLock<HashMap<String, TransactionMeta>>,
    sender_nonces: RwLock<HashMap<String, BTreeMap<u64, String>>>,
    max_size: usize,
    expiration_duration: Duration,
    last_cleanup: RwLock<Instant>,
}

impl Mempool {
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: RwLock::new(HashMap::new()),
            sender_nonces: RwLock::new(HashMap::new()),
            max_size,
            expiration_duration: Duration::from_secs(300),
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

    /// Add transaction to mempool
    pub fn add_transaction(&self, tx: Transaction) -> Result<bool> {
        let tx_hash = hex::encode(tx.hash());
        let sender = hex::encode(tx.from);

        self.cleanup_expired_transactions();

        let mut transactions = self.transactions.write();
        let mut sender_nonces = self.sender_nonces.write();

        if transactions.contains_key(&tx_hash) {
            return Ok(false);
        }

        // Ensure space before adding new transaction
        if transactions.len() >= self.max_size
            && !self.make_space_for_transaction(&mut transactions, &mut sender_nonces, 0)
        {
            return Ok(false);
        }

        // Lightweight confidential validation
        if tx.visibility == ippan_types::TransactionVisibility::Confidential {
            validate_confidential_transaction(&tx)?;
        }

        let fee = self.calculate_transaction_fee(&tx);
        const MAX_FEE_PER_TX: u64 = 10_000_000;
        if fee > MAX_FEE_PER_TX {
            return Ok(false);
        }

        let meta = TransactionMeta {
            transaction: tx.clone(),
            added_at: Instant::now(),
            fee,
        };

        transactions.insert(tx_hash.clone(), meta);
        sender_nonces
            .entry(sender)
            .or_default()
            .insert(tx.nonce, tx_hash);

        Ok(true)
    }

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

    pub fn get_transaction(&self, tx_hash: &str) -> Option<Transaction> {
        self.transactions
            .read()
            .get(tx_hash)
            .map(|meta| meta.transaction.clone())
    }

    pub fn get_sender_transactions(&self, sender: &str) -> Vec<Transaction> {
        let transactions = self.transactions.read();
        let sender_nonces = self.sender_nonces.read();

        if let Some(nonces) = sender_nonces.get(sender) {
            nonces
                .values()
                .filter_map(|h| transactions.get(h))
                .map(|meta| meta.transaction.clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Fee-prioritized selection for block building
    pub fn get_transactions_for_block(&self, max_count: usize) -> Vec<Transaction> {
        if max_count == 0 {
            return Vec::new();
        }

        let transactions = self.transactions.read();
        let sender_nonces = self.sender_nonces.read();
        let mut heap = BinaryHeap::new();
        let mut per_sender_entries = HashMap::new();
        let mut next_index = HashMap::new();

        for (sender, nonces) in sender_nonces.iter() {
            let entries: Vec<(u64, String)> = nonces.iter().map(|(&n, h)| (n, h.clone())).collect();

            if let Some((nonce, tx_hash)) = entries.first() {
                if let Some(meta) = transactions.get(tx_hash) {
                    heap.push(BlockCandidate {
                        fee: meta.fee,
                        added_at: meta.added_at,
                        sender: sender.clone(),
                        nonce: *nonce,
                        tx_hash: tx_hash.clone(),
                    });
                    per_sender_entries.insert(sender.clone(), entries);
                    next_index.insert(sender.clone(), 1);
                }
            }
        }

        let mut selected = Vec::new();
        let mut last_nonce: HashMap<String, u64> = HashMap::new();

        while selected.len() < max_count {
            let Some(candidate) = heap.pop() else { break };
            if let Some(&last) = last_nonce.get(&candidate.sender) {
                if candidate.nonce != last + 1 {
                    continue;
                }
            }

            let Some(meta) = transactions.get(&candidate.tx_hash) else {
                continue;
            };
            selected.push(meta.transaction.clone());
            last_nonce.insert(candidate.sender.clone(), candidate.nonce);

            if let Some(entries) = per_sender_entries.get(&candidate.sender) {
                if let Some(next_idx) = next_index.get_mut(&candidate.sender) {
                    let mut idx = *next_idx;
                    while idx < entries.len() {
                        let (next_nonce, next_hash) = &entries[idx];
                        if *next_nonce == candidate.nonce + 1 {
                            if let Some(next_meta) = transactions.get(next_hash) {
                                heap.push(BlockCandidate {
                                    fee: next_meta.fee,
                                    added_at: next_meta.added_at,
                                    sender: candidate.sender.clone(),
                                    nonce: *next_nonce,
                                    tx_hash: next_hash.clone(),
                                });
                                idx += 1;
                            }
                            break;
                        } else if *next_nonce <= candidate.nonce {
                            idx += 1;
                        } else {
                            break;
                        }
                    }
                    *next_idx = idx;
                }
            }
        }

        selected
    }

    pub fn size(&self) -> usize {
        self.transactions.read().len()
    }

    pub fn clear(&self) {
        self.transactions.write().clear();
        self.sender_nonces.write().clear();
    }

    fn cleanup_expired_transactions(&self) {
        let now = Instant::now();
        let last_cleanup = *self.last_cleanup.read();
        let since = now.duration_since(last_cleanup);

        if since < Duration::from_secs(30) && since < self.expiration_duration {
            return;
        }

        let mut transactions = self.transactions.write();
        let mut sender_nonces = self.sender_nonces.write();
        let mut to_remove = Vec::new();

        for (hash, meta) in transactions.iter() {
            if now.duration_since(meta.added_at) > self.expiration_duration {
                to_remove.push(hash.clone());
            }
        }

        for hash in to_remove {
            if let Some(meta) = transactions.remove(&hash) {
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

    fn calculate_transaction_fee(&self, tx: &Transaction) -> u64 {
        let base_fee = 1000;
        let mut size = 32 * 3
            + 8 * 2
            + 64
            + std::mem::size_of_val(&tx.hashtimer.timestamp_us)
            + tx.hashtimer.entropy.len()
            + std::mem::size_of_val(&tx.timestamp.0);
        size += tx.topics.iter().map(|t| t.len()).sum::<usize>();

        if let Some(env) = &tx.confidential {
            size += env.enc_algo.len() + env.iv.len() + env.ciphertext.len();
            size += env
                .access_keys
                .iter()
                .map(|k| k.recipient_pub.len() + k.enc_key.len())
                .sum::<usize>();
        }
        if let Some(proof) = &tx.zk_proof {
            size += proof.proof.len();
            size += proof
                .public_inputs
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>();
        }

        base_fee + (size as u64 * 10)
    }

    fn make_space_for_transaction(
        &self,
        transactions: &mut HashMap<String, TransactionMeta>,
        sender_nonces: &mut HashMap<String, BTreeMap<u64, String>>,
        new_fee: u64,
    ) -> bool {
        let mut lowest_fee = u64::MAX;
        let mut lowest_hash = None;

        for (hash, meta) in transactions.iter() {
            if meta.fee < lowest_fee {
                lowest_fee = meta.fee;
                lowest_hash = Some(hash.clone());
            }
        }

        if let Some(hash) = lowest_hash {
            if lowest_fee < new_fee {
                if let Some(meta) = transactions.remove(&hash) {
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

    /// Collect mempool diagnostics
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
#[cfg(all(test, feature = "enable-tests"))]
mod tests {
    use super::*;
    use ippan_types::{Amount, Transaction};
    use std::time::Duration;

    #[test]
    fn test_mempool_add_remove() {
        let mempool = Mempool::new(100);
        let tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        let tx_hash = hex::encode(tx.hash());
        assert!(mempool.add_transaction(tx.clone()).unwrap());
        assert_eq!(mempool.size(), 1);
        assert!(!mempool.add_transaction(tx.clone()).unwrap());
        let removed = mempool.remove_transaction(&tx_hash).unwrap();
        assert!(removed.is_some());
        assert_eq!(mempool.size(), 0);
    }

    #[test]
    fn test_mempool_sender_transactions() {
        let mempool = Mempool::new(100);
        let sender = [1u8; 32];
        let tx1 = Transaction::new(sender, [2u8; 32], Amount::from_atomic(1000), 1);
        let tx2 = Transaction::new(sender, [3u8; 32], Amount::from_atomic(2000), 2);
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
        let tx1 = Transaction::new(sender1, [3u8; 32], Amount::from_atomic(1000), 1);
        let tx2 = Transaction::new(sender2, [4u8; 32], Amount::from_atomic(2000), 1);
        let tx3 = Transaction::new(sender1, [5u8; 32], Amount::from_atomic(1500), 2);
        mempool.add_transaction(tx1).unwrap();
        mempool.add_transaction(tx2).unwrap();
        mempool.add_transaction(tx3).unwrap();
        let block_txs = mempool.get_transactions_for_block(3);
        assert_eq!(block_txs.len(), 3);
    }

    #[test]
    fn test_mempool_nonce_ordering() {
        let mempool = Mempool::new(100);
        let sender = [1u8; 32];
        let tx1 = Transaction::new(sender, [2u8; 32], Amount::from_atomic(1000), 1);
        let tx2 = Transaction::new(sender, [3u8; 32], Amount::from_atomic(2000), 2);
        let tx3 = Transaction::new(sender, [4u8; 32], Amount::from_atomic(1500), 3);
        mempool.add_transaction(tx2.clone()).unwrap();
        mempool.add_transaction(tx1.clone()).unwrap();
        mempool.add_transaction(tx3.clone()).unwrap();
        let block_txs = mempool.get_transactions_for_block(3);
        let nonces: Vec<_> = block_txs
            .iter()
            .filter(|tx| tx.from == sender)
            .map(|tx| tx.nonce)
            .collect();
        assert_eq!(nonces, vec![1, 2, 3]);
    }

    #[test]
    fn test_mempool_expiration() {
        let mempool = Mempool::new_with_expiration(100, Duration::from_millis(100));
        let tx = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        assert!(mempool.add_transaction(tx).unwrap());
        assert_eq!(mempool.size(), 1);
        std::thread::sleep(Duration::from_millis(150));
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], Amount::from_atomic(1000), 1);
        assert!(mempool.add_transaction(tx2).unwrap());
        assert_eq!(mempool.size(), 1);
    }

    #[test]
    fn test_mempool_stats() {
        let mempool = Mempool::new(100);
        let tx1 = Transaction::new([1u8; 32], [2u8; 32], Amount::from_atomic(1000), 1);
        let tx2 = Transaction::new([3u8; 32], [4u8; 32], Amount::from_atomic(2000), 1);
        mempool.add_transaction(tx1).unwrap();
        mempool.add_transaction(tx2).unwrap();
        let stats = mempool.get_stats();
        assert_eq!(stats.size, 2);
        assert!(stats.total_fee > 0);
    }
}
