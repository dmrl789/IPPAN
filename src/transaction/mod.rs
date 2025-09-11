//! Transaction processing system for IPPAN
//! 
//! Handles transaction validation, mempool management, and block assembly

use crate::{Result, IppanError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use sha2::{Digest, Sha256};

/// Transaction hash type
pub type TransactionHash = [u8; 32];

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// Payment transaction
    Payment {
        from: String,
        to: String,
        amount: u64,
        fee: u64,
    },
    /// Staking transaction
    Staking {
        validator: String,
        amount: u64,
        duration: u64,
    },
    /// Unstaking transaction
    Unstaking {
        validator: String,
        amount: u64,
    },
    /// Storage transaction
    Storage {
        data_hash: String,
        size: u64,
        duration: u64,
    },
    /// DNS zone update
    DnsZoneUpdate {
        domain: String,
        records: Vec<DnsRecord>,
    },
}

/// DNS record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

/// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: TransactionHash,
    pub tx_type: TransactionType,
    pub nonce: u64,
    pub timestamp: u64,
    pub signature: String,
    pub sender: String,
}

/// Transaction pool entry
#[derive(Debug, Clone)]
pub struct TransactionPoolEntry {
    pub transaction: Transaction,
    pub received_at: SystemTime,
    pub priority: u64,
    pub size: usize,
}

/// Mempool for managing pending transactions
#[derive(Debug)]
pub struct Mempool {
    /// Pending transactions by hash
    transactions: Arc<RwLock<HashMap<TransactionHash, TransactionPoolEntry>>>,
    /// Transactions by sender (for nonce management)
    sender_transactions: Arc<RwLock<HashMap<String, VecDeque<TransactionHash>>>>,
    /// Priority queue for block assembly
    priority_queue: Arc<RwLock<VecDeque<TransactionHash>>>,
    /// Maximum mempool size
    max_size: usize,
    /// Transaction timeout
    timeout: Duration,
}

impl Mempool {
    /// Create a new mempool
    pub fn new(max_size: usize, timeout: Duration) -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            sender_transactions: Arc::new(RwLock::new(HashMap::new())),
            priority_queue: Arc::new(RwLock::new(VecDeque::new())),
            max_size,
            timeout,
        }
    }

    /// Add a transaction to the mempool
    pub async fn add_transaction(&self, transaction: Transaction) -> Result<bool> {
        let hash = transaction.hash;
        let sender = transaction.sender.clone();
        
        // Check if transaction already exists
        {
            let transactions = self.transactions.read().await;
            if transactions.contains_key(&hash) {
                return Ok(false); // Already exists
            }
        }

        // Check mempool size
        {
            let transactions = self.transactions.read().await;
            if transactions.len() >= self.max_size {
                // Remove oldest transaction
                self.remove_oldest_transaction().await?;
            }
        }

        // Calculate priority based on fee and timestamp
        let priority = self.calculate_priority(&transaction);
        let size = 1024; // TODO: Implement proper transaction size calculation
        
        let entry = TransactionPoolEntry {
            transaction: transaction.clone(),
            received_at: SystemTime::now(),
            priority,
            size,
        };

        // Add to transactions
        {
            let mut transactions = self.transactions.write().await;
            transactions.insert(hash, entry);
        }

        // Add to sender transactions for nonce management
        {
            let mut sender_transactions = self.sender_transactions.write().await;
            sender_transactions.entry(sender).or_insert_with(VecDeque::new).push_back(hash);
        }

        // Add to priority queue
        {
            let mut priority_queue = self.priority_queue.write().await;
            self.insert_by_priority(&mut priority_queue, hash, priority);
        }

        log::debug!("Added transaction {} to mempool", hex::encode(hash));
        Ok(true)
    }

    /// Remove a transaction from the mempool
    pub async fn remove_transaction(&self, hash: &TransactionHash) -> Result<Option<Transaction>> {
        let transaction = {
            let mut transactions = self.transactions.write().await;
            transactions.remove(hash).map(|entry| entry.transaction)
        };

        if let Some(ref tx) = transaction {
            // Remove from sender transactions
            {
                let mut sender_transactions = self.sender_transactions.write().await;
                if let Some(sender_txs) = sender_transactions.get_mut(&tx.sender) {
                    sender_txs.retain(|&h| h != *hash);
                    if sender_txs.is_empty() {
                        sender_transactions.remove(&tx.sender);
                    }
                }
            }

            // Remove from priority queue
            {
                let mut priority_queue = self.priority_queue.write().await;
                priority_queue.retain(|&h| h != *hash);
            }
        }

        Ok(transaction)
    }

    /// Get transactions for block assembly
    pub async fn get_transactions_for_block(&self, max_size: usize) -> Result<Vec<Transaction>> {
        let mut selected_transactions = Vec::new();
        let mut current_size = 0;
        let mut priority_queue = self.priority_queue.write().await;
        
        while let Some(hash) = priority_queue.pop_front() {
            let transaction_entry = {
                let transactions = self.transactions.read().await;
                transactions.get(&hash).cloned()
            };

            if let Some(entry) = transaction_entry {
                // Check if transaction is still valid (not expired)
                if let Ok(elapsed) = entry.received_at.elapsed() {
                    if elapsed < self.timeout {
                        if current_size + entry.size <= max_size {
                            selected_transactions.push(entry.transaction);
                            current_size += entry.size;
                        } else {
                            // Put back in queue if it doesn't fit
                            priority_queue.push_front(hash);
                        }
                    }
                } else {
                    // Remove expired transaction
                    self.remove_transaction(&hash).await?;
                }
            }
        }

        Ok(selected_transactions)
    }

    /// Get mempool statistics
    pub async fn get_stats(&self) -> MempoolStats {
        let transactions = self.transactions.read().await;
        let sender_transactions = self.sender_transactions.read().await;
        
        let total_transactions = transactions.len();
        let total_senders = sender_transactions.len();
        let total_size: usize = transactions.values().map(|entry| entry.size).sum();
        
        let mut fee_distribution = HashMap::new();
        for entry in transactions.values() {
            let fee = match &entry.transaction.tx_type {
                TransactionType::Payment { fee, .. } => *fee,
                TransactionType::Staking { .. } => 1000, // Default staking fee
                TransactionType::Unstaking { .. } => 1000, // Default unstaking fee
                TransactionType::Storage { .. } => 500, // Default storage fee
                TransactionType::DnsZoneUpdate { .. } => 200, // Default DNS fee
            };
            *fee_distribution.entry(fee).or_insert(0) += 1;
        }

        MempoolStats {
            total_transactions,
            total_senders,
            total_size,
            fee_distribution,
        }
    }

    /// Calculate transaction priority
    fn calculate_priority(&self, transaction: &Transaction) -> u64 {
        let fee = match &transaction.tx_type {
            TransactionType::Payment { fee, .. } => *fee,
            TransactionType::Staking { .. } => 1000,
            TransactionType::Unstaking { .. } => 1000,
            TransactionType::Storage { .. } => 500,
            TransactionType::DnsZoneUpdate { .. } => 200,
        };

        // Higher fee = higher priority
        // Earlier timestamp = higher priority
        let timestamp_priority = u64::MAX - transaction.timestamp;
        fee * 1000 + timestamp_priority / 1000
    }

    /// Insert transaction by priority in the queue
    fn insert_by_priority(&self, queue: &mut VecDeque<TransactionHash>, hash: TransactionHash, priority: u64) {
        let mut insert_pos = 0;
        for (i, &existing_hash) in queue.iter().enumerate() {
            // This is a simplified implementation - in reality we'd need to look up priorities
            // For now, we'll just insert at the end
            insert_pos = i + 1;
        }
        queue.insert(insert_pos, hash);
    }

    /// Remove the oldest transaction
    async fn remove_oldest_transaction(&self) -> Result<()> {
        let oldest_hash = {
            let transactions = self.transactions.read().await;
            let mut oldest_time = SystemTime::now();
            let mut oldest_hash = None;
            
            for (hash, entry) in transactions.iter() {
                if entry.received_at < oldest_time {
                    oldest_time = entry.received_at;
                    oldest_hash = Some(*hash);
                }
            }
            oldest_hash
        };

        if let Some(hash) = oldest_hash {
            self.remove_transaction(&hash).await?;
        }

        Ok(())
    }

    /// Clean up expired transactions
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let expired_hashes = {
            let transactions = self.transactions.read().await;
            let mut expired = Vec::new();
            
            for (hash, entry) in transactions.iter() {
                if let Ok(elapsed) = entry.received_at.elapsed() {
                    if elapsed >= self.timeout {
                        expired.push(*hash);
                    }
                }
            }
            expired
        };

        let mut removed_count = 0;
        for hash in expired_hashes {
            if self.remove_transaction(&hash).await?.is_some() {
                removed_count += 1;
            }
        }

        log::debug!("Cleaned up {} expired transactions", removed_count);
        Ok(removed_count)
    }
}

/// Transaction processor
#[derive(Debug)]
pub struct TransactionProcessor {
    mempool: Arc<Mempool>,
    /// Account balances (simplified)
    balances: Arc<RwLock<HashMap<String, u64>>>,
    /// Account nonces
    nonces: Arc<RwLock<HashMap<String, u64>>>,
}

impl TransactionProcessor {
    /// Create a new transaction processor
    pub fn new(mempool: Arc<Mempool>) -> Self {
        Self {
            mempool,
            balances: Arc::new(RwLock::new(HashMap::new())),
            nonces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Process a new transaction
    pub async fn process_transaction(&self, transaction: Transaction) -> Result<bool> {
        // Validate transaction
        if !self.validate_transaction(&transaction).await? {
            return Ok(false);
        }

        // Add to mempool
        self.mempool.add_transaction(transaction).await
    }

    /// Validate a transaction
    async fn validate_transaction(&self, transaction: &Transaction) -> Result<bool> {
        // Check nonce
        {
            let nonces = self.nonces.read().await;
            let expected_nonce = nonces.get(&transaction.sender).copied().unwrap_or(0);
            if transaction.nonce != expected_nonce {
                log::warn!("Invalid nonce for {}: expected {}, got {}", 
                          transaction.sender, expected_nonce, transaction.nonce);
                return Ok(false);
            }
        }

        // Check balance for payment transactions
        if let TransactionType::Payment { from, amount, fee, .. } = &transaction.tx_type {
            if from != &transaction.sender {
                return Ok(false);
            }

            let balances = self.balances.read().await;
            let balance = balances.get(from).copied().unwrap_or(0);
            if balance < amount + fee {
                log::warn!("Insufficient balance for {}: need {}, have {}", 
                          from, amount + fee, balance);
                return Ok(false);
            }
        }

        // TODO: Add signature validation
        // TODO: Add more sophisticated validation

        Ok(true)
    }

    /// Execute transactions (for block processing)
    pub async fn execute_transactions(&self, transactions: Vec<Transaction>) -> Result<()> {
        let mut balances = self.balances.write().await;
        let mut nonces = self.nonces.write().await;

        for transaction in transactions {
            match &transaction.tx_type {
                TransactionType::Payment { from, to, amount, fee } => {
                    // Update balances
                    let from_balance = balances.get(from).copied().unwrap_or(0);
                    let to_balance = balances.get(to).copied().unwrap_or(0);
                    
                    if from_balance >= amount + fee {
                        balances.insert(from.clone(), from_balance - amount - fee);
                        balances.insert(to.clone(), to_balance + amount);
                        
                        // Update nonce
                        let current_nonce = nonces.get(from).copied().unwrap_or(0);
                        nonces.insert(from.clone(), current_nonce + 1);
                    }
                }
                TransactionType::Staking { validator, amount, duration: _ } => {
                    // Handle staking logic
                    let balance = balances.get(&transaction.sender).copied().unwrap_or(0);
                    if balance >= *amount {
                        balances.insert(transaction.sender.clone(), balance - amount);
                        
                        // Update nonce
                        let current_nonce = nonces.get(&transaction.sender).copied().unwrap_or(0);
                        nonces.insert(transaction.sender.clone(), current_nonce + 1);
                    }
                }
                TransactionType::Unstaking { validator, amount } => {
                    // Handle unstaking logic
                    let balance = balances.get(&transaction.sender).copied().unwrap_or(0);
                    balances.insert(transaction.sender.clone(), balance + amount);
                    
                    // Update nonce
                    let current_nonce = nonces.get(&transaction.sender).copied().unwrap_or(0);
                    nonces.insert(transaction.sender.clone(), current_nonce + 1);
                }
                TransactionType::Storage { .. } => {
                    // Handle storage logic
                    let current_nonce = nonces.get(&transaction.sender).copied().unwrap_or(0);
                    nonces.insert(transaction.sender.clone(), current_nonce + 1);
                }
                TransactionType::DnsZoneUpdate { .. } => {
                    // Handle DNS logic
                    let current_nonce = nonces.get(&transaction.sender).copied().unwrap_or(0);
                    nonces.insert(transaction.sender.clone(), current_nonce + 1);
                }
            }
        }

        Ok(())
    }

    /// Get account balance
    pub async fn get_balance(&self, account: &str) -> u64 {
        let balances = self.balances.read().await;
        balances.get(account).copied().unwrap_or(0)
    }

    /// Set account balance (for testing)
    pub async fn set_balance(&self, account: &str, balance: u64) {
        let mut balances = self.balances.write().await;
        balances.insert(account.to_string(), balance);
    }

    /// Get account nonce
    pub async fn get_nonce(&self, account: &str) -> u64 {
        let nonces = self.nonces.read().await;
        nonces.get(account).copied().unwrap_or(0)
    }
}

/// Mempool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolStats {
    pub total_transactions: usize,
    pub total_senders: usize,
    pub total_size: usize,
    pub fee_distribution: HashMap<u64, usize>,
}

/// Create a transaction hash
pub fn create_transaction_hash(transaction: &Transaction) -> Result<TransactionHash> {
    let mut hasher = Sha256::new();
    
    // Hash the transaction data (excluding the hash field itself)
    // Serialize the transaction for hashing
    let tx_data = bincode::serialize(&Transaction {
        hash: [0u8; 32], // Will be set after hashing
        tx_type: transaction.tx_type.clone(),
        nonce: transaction.nonce,
        timestamp: transaction.timestamp,
        signature: transaction.signature.clone(),
        sender: transaction.sender.clone(),
    }).map_err(|e| IppanError::Serialization(format!("Failed to serialize transaction: {}", e)))?;
    
    hasher.update(&tx_data);
    let result = hasher.finalize();
    
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    Ok(hash)
}

/// Create a new transaction
pub fn create_transaction(
    tx_type: TransactionType,
    nonce: u64,
    sender: String,
    signature: String,
) -> Result<Transaction> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut transaction = Transaction {
        hash: [0u8; 32], // Will be set after creation
        tx_type,
        nonce,
        timestamp,
        signature,
        sender,
    };

    // Set the hash
    transaction.hash = create_transaction_hash(&transaction)?;
    Ok(transaction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_mempool_operations() {
        let mempool = Arc::new(Mempool::new(1000, Duration::from_secs(300)));
        
        // Create a test transaction
        let transaction = create_transaction(
            TransactionType::Payment {
                from: "alice".to_string(),
                to: "bob".to_string(),
                amount: 1000,
                fee: 10,
            },
            0,
            "alice".to_string(),
            "test_signature".to_string(),
        ).unwrap();

        // Add transaction
        let added = mempool.add_transaction(transaction.clone()).await.unwrap();
        assert!(added);

        // Try to add same transaction again
        let added_again = mempool.add_transaction(transaction.clone()).await.unwrap();
        assert!(!added_again);

        // Get stats
        let stats = mempool.get_stats().await;
        assert_eq!(stats.total_transactions, 1);
        assert_eq!(stats.total_senders, 1);

        // Remove transaction
        let removed = mempool.remove_transaction(&transaction.hash).await.unwrap();
        assert!(removed.is_some());

        // Check stats after removal
        let stats = mempool.get_stats().await;
        assert_eq!(stats.total_transactions, 0);
    }

    #[tokio::test]
    async fn test_transaction_processor() {
        let mempool = Arc::new(Mempool::new(1000, Duration::from_secs(300)));
        let processor = TransactionProcessor::new(mempool);

        // Set initial balance
        processor.set_balance("alice", 10000).await;

        // Create a valid payment transaction
        let transaction = create_transaction(
            TransactionType::Payment {
                from: "alice".to_string(),
                to: "bob".to_string(),
                amount: 1000,
                fee: 10,
            },
            0,
            "alice".to_string(),
            "test_signature".to_string(),
        ).unwrap();

        // Process transaction
        let processed = processor.process_transaction(transaction.clone()).await.unwrap();
        assert!(processed);

        // Check nonce was incremented
        let nonce = processor.get_nonce("alice").await;
        assert_eq!(nonce, 0); // Nonce is incremented during execution, not processing

        // Execute transaction
        processor.execute_transactions(vec![transaction]).await.unwrap();

        // Check balances
        let alice_balance = processor.get_balance("alice").await;
        let bob_balance = processor.get_balance("bob").await;
        
        assert_eq!(alice_balance, 8990); // 10000 - 1000 - 10
        assert_eq!(bob_balance, 1000);

        // Check nonce was incremented
        let nonce = processor.get_nonce("alice").await;
        assert_eq!(nonce, 1);
    }

    #[tokio::test]
    async fn test_transaction_validation() {
        let mempool = Arc::new(Mempool::new(1000, Duration::from_secs(300)));
        let processor = TransactionProcessor::new(mempool);

        // Set initial balance
        processor.set_balance("alice", 1000).await;

        // Create transaction with insufficient balance
        let transaction = create_transaction(
            TransactionType::Payment {
                from: "alice".to_string(),
                to: "bob".to_string(),
                amount: 2000, // More than balance
                fee: 10,
            },
            0,
            "alice".to_string(),
            "test_signature".to_string(),
        ).unwrap();

        // Process transaction - should fail validation
        let processed = processor.process_transaction(transaction).await.unwrap();
        assert!(!processed);
    }

    #[tokio::test]
    async fn test_transaction_hash_consistency() {
        let transaction1 = create_transaction(
            TransactionType::Payment {
                from: "alice".to_string(),
                to: "bob".to_string(),
                amount: 1000,
                fee: 10,
            },
            0,
            "alice".to_string(),
            "test_signature".to_string(),
        ).unwrap();

        let transaction2 = create_transaction(
            TransactionType::Payment {
                from: "alice".to_string(),
                to: "bob".to_string(),
                amount: 1000,
                fee: 10,
            },
            0,
            "alice".to_string(),
            "test_signature".to_string(),
        ).unwrap();

        // Same transaction data should produce same hash
        assert_eq!(transaction1.hash, transaction2.hash);
    }
}
