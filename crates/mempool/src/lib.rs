use anyhow::Result;
use ippan_types::Transaction;
use parking_lot::RwLock;
use std::collections::{HashMap, BTreeMap};

/// Mempool for managing pending transactions
pub struct Mempool {
    /// Pending transactions indexed by hash
    transactions: RwLock<HashMap<String, Transaction>>,
    /// Transactions indexed by sender nonce for ordering
    sender_nonces: RwLock<HashMap<String, BTreeMap<u64, String>>>,
    /// Maximum number of transactions to keep
    max_size: usize,
}

impl Mempool {
    pub fn new(max_size: usize) -> Self {
        Self {
            transactions: RwLock::new(HashMap::new()),
            sender_nonces: RwLock::new(HashMap::new()),
            max_size,
        }
    }

    /// Add a transaction to the mempool
    pub fn add_transaction(&self, tx: Transaction) -> Result<bool> {
        let tx_hash = hex::encode(tx.hash());
        let sender = hex::encode(tx.from);
        
        let mut transactions = self.transactions.write();
        let mut sender_nonces = self.sender_nonces.write();
        
        // Check if transaction already exists
        if transactions.contains_key(&tx_hash) {
            return Ok(false);
        }
        
        // Check mempool size limit
        if transactions.len() >= self.max_size {
            return Ok(false);
        }
        
        // Add transaction
        transactions.insert(tx_hash.clone(), tx.clone());
        
        // Update sender nonce index
        sender_nonces
            .entry(sender)
            .or_insert_with(BTreeMap::new)
            .insert(tx.nonce, tx_hash);
        
        Ok(true)
    }

    /// Remove a transaction from the mempool
    pub fn remove_transaction(&self, tx_hash: &str) -> Result<Option<Transaction>> {
        let mut transactions = self.transactions.write();
        let mut sender_nonces = self.sender_nonces.write();
        
        if let Some(tx) = transactions.remove(tx_hash) {
            let sender = hex::encode(tx.from);
            if let Some(nonces) = sender_nonces.get_mut(&sender) {
                nonces.remove(&tx.nonce);
                if nonces.is_empty() {
                    sender_nonces.remove(&sender);
                }
            }
            Ok(Some(tx))
        } else {
            Ok(None)
        }
    }

    /// Get a transaction by hash
    pub fn get_transaction(&self, tx_hash: &str) -> Option<Transaction> {
        self.transactions.read().get(tx_hash).cloned()
    }

    /// Get all transactions for a sender
    pub fn get_sender_transactions(&self, sender: &str) -> Vec<Transaction> {
        let transactions = self.transactions.read();
        let sender_nonces = self.sender_nonces.read();
        
        if let Some(nonces) = sender_nonces.get(sender) {
            nonces.values()
                .filter_map(|tx_hash| transactions.get(tx_hash))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get transactions for block creation (up to max_count)
    pub fn get_transactions_for_block(&self, max_count: usize) -> Vec<Transaction> {
        let transactions = self.transactions.read();
        let sender_nonces = self.sender_nonces.read();
        
        let mut selected = Vec::new();
        let mut used_senders = HashMap::new();
        
        // Select transactions in nonce order for each sender
        for (sender, nonces) in sender_nonces.iter() {
            let mut sender_txs = Vec::new();
            for (nonce, tx_hash) in nonces.iter() {
                if let Some(tx) = transactions.get(tx_hash) {
                    sender_txs.push((*nonce, tx.clone()));
                }
            }
            
            // Sort by nonce and take the first few
            sender_txs.sort_by_key(|(nonce, _)| *nonce);
            for (nonce, tx) in sender_txs {
                if selected.len() >= max_count {
                    break;
                }
                if let Some(last_nonce) = used_senders.get(sender) {
                    if nonce == *last_nonce + 1 {
                        selected.push(tx);
                        used_senders.insert(sender.clone(), nonce);
                    }
                } else if nonce == 0 {
                    selected.push(tx);
                    used_senders.insert(sender.clone(), nonce);
                }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_types::Transaction;

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
}
