//! Transaction selector for IPPAN
//! 
//! Implements intelligent transaction selection for block creation including
//! prioritization, fee optimization, and gas management.

use crate::{Result, IppanError, TransactionHash};
use crate::database::{DatabaseManager, StoredTransaction};
use crate::consensus::bft_engine::BFTTransaction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::cmp::Ordering;
use tracing::{info, warn, error, debug};

/// Transaction selection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSelectionConfig {
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Maximum block size in bytes
    pub max_block_size_bytes: usize,
    /// Maximum gas per block
    pub max_gas_per_block: u64,
    /// Minimum transaction fee
    pub min_transaction_fee: u64,
    /// Enable fee-based prioritization
    pub enable_fee_prioritization: bool,
    /// Enable time-based prioritization
    pub enable_time_prioritization: bool,
    /// Enable gas price prioritization
    pub enable_gas_price_prioritization: bool,
    /// Transaction selection algorithm
    pub selection_algorithm: SelectionAlgorithm,
    /// Maximum transaction age in seconds
    pub max_transaction_age_seconds: u64,
    /// Enable transaction replacement
    pub enable_transaction_replacement: bool,
    /// Replacement fee multiplier
    pub replacement_fee_multiplier: f64,
}

impl Default for TransactionSelectionConfig {
    fn default() -> Self {
        Self {
            max_transactions_per_block: 1000,
            max_block_size_bytes: 1024 * 1024, // 1MB
            max_gas_per_block: 10000000, // 10M gas
            min_transaction_fee: 100,
            enable_fee_prioritization: true,
            enable_time_prioritization: true,
            enable_gas_price_prioritization: true,
            selection_algorithm: SelectionAlgorithm::FeeBased,
            max_transaction_age_seconds: 3600, // 1 hour
            enable_transaction_replacement: true,
            replacement_fee_multiplier: 1.1,
        }
    }
}

/// Transaction selection algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub enum SelectionAlgorithm {
    /// Fee-based selection (highest fee first)
    FeeBased,
    /// Time-based selection (oldest first)
    TimeBased,
    /// Gas price-based selection (highest gas price first)
    GasPriceBased,
    /// Hybrid selection (combination of factors)
    Hybrid,
    /// Random selection
    Random,
}

/// Transaction priority score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPriority {
    /// Transaction hash
    pub hash: TransactionHash,
    /// Priority score (higher is better)
    pub score: f64,
    /// Fee per gas
    pub fee_per_gas: f64,
    /// Transaction age in seconds
    pub age_seconds: u64,
    /// Gas used
    pub gas_used: u64,
    /// Transaction size in bytes
    pub size_bytes: usize,
}

impl PartialEq for TransactionPriority {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for TransactionPriority {}

impl PartialOrd for TransactionPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TransactionPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher score is better
        other.score.partial_cmp(&self.score).unwrap_or(Ordering::Equal)
    }
}

/// Transaction selection result
#[derive(Debug, Clone)]
pub struct TransactionSelectionResult {
    /// Selected transactions
    pub selected_transactions: Vec<BFTTransaction>,
    /// Total transactions considered
    pub total_transactions_considered: usize,
    /// Total gas used
    pub total_gas_used: u64,
    /// Total block size in bytes
    pub total_block_size_bytes: usize,
    /// Total fees collected
    pub total_fees_collected: u64,
    /// Selection time in milliseconds
    pub selection_time_ms: u64,
    /// Selection algorithm used
    pub algorithm_used: SelectionAlgorithm,
    /// Replaced transactions
    pub replaced_transactions: Vec<TransactionHash>,
}

/// Transaction selection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSelectionStats {
    /// Total selections performed
    pub total_selections: u64,
    /// Total transactions selected
    pub total_transactions_selected: u64,
    /// Total transactions considered
    pub total_transactions_considered: u64,
    /// Average selection time in milliseconds
    pub average_selection_time_ms: f64,
    /// Average transactions per selection
    pub average_transactions_per_selection: f64,
    /// Average gas used per selection
    pub average_gas_used_per_selection: f64,
    /// Average fees collected per selection
    pub average_fees_collected_per_selection: f64,
    /// Selection efficiency (selected/considered ratio)
    pub selection_efficiency: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last selection timestamp
    pub last_selection: Option<u64>,
}

/// Transaction selector
pub struct TransactionSelector {
    /// Configuration
    config: TransactionSelectionConfig,
    /// Database manager
    database: Arc<DatabaseManager>,
    /// Statistics
    stats: Arc<RwLock<TransactionSelectionStats>>,
    /// Is running
    is_running: Arc<RwLock<bool>>,
    /// Start time
    start_time: Instant,
}

impl TransactionSelector {
    /// Create a new transaction selector
    pub fn new(config: TransactionSelectionConfig, database: Arc<DatabaseManager>) -> Self {
        let stats = TransactionSelectionStats {
            total_selections: 0,
            total_transactions_selected: 0,
            total_transactions_considered: 0,
            average_selection_time_ms: 0.0,
            average_transactions_per_selection: 0.0,
            average_gas_used_per_selection: 0.0,
            average_fees_collected_per_selection: 0.0,
            selection_efficiency: 0.0,
            uptime_seconds: 0,
            last_selection: None,
        };
        
        Self {
            config,
            database,
            stats: Arc::new(RwLock::new(stats)),
            is_running: Arc::new(RwLock::new(false)),
            start_time: Instant::now(),
        }
    }
    
    /// Start the transaction selector
    pub async fn start(&self) -> Result<()> {
        info!("Starting transaction selector");
        
        let mut is_running = self.is_running.write().await;
        *is_running = true;
        drop(is_running);
        
        info!("Transaction selector started successfully");
        Ok(())
    }
    
    /// Stop the transaction selector
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping transaction selector");
        
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        info!("Transaction selector stopped");
        Ok(())
    }
    
    /// Select transactions for block creation
    pub async fn select_transactions(&self) -> Result<TransactionSelectionResult> {
        let start_time = Instant::now();
        
        debug!("Selecting transactions for block creation");
        
        // Get pending transactions
        let pending_transactions = self.get_pending_transactions().await?;
        
        if pending_transactions.is_empty() {
            return Ok(TransactionSelectionResult {
                selected_transactions: vec![],
                total_transactions_considered: 0,
                total_gas_used: 0,
                total_block_size_bytes: 0,
                total_fees_collected: 0,
                selection_time_ms: start_time.elapsed().as_millis() as u64,
                algorithm_used: self.config.selection_algorithm.clone(),
                replaced_transactions: vec![],
            });
        }
        
        // Calculate priorities
        let priorities = self.calculate_transaction_priorities(&pending_transactions).await?;
        
        // Select transactions based on algorithm
        let selected_transactions = match self.config.selection_algorithm {
            SelectionAlgorithm::FeeBased => self.select_by_fee(&priorities).await?,
            SelectionAlgorithm::TimeBased => self.select_by_time(&priorities).await?,
            SelectionAlgorithm::GasPriceBased => self.select_by_gas_price(&priorities).await?,
            SelectionAlgorithm::Hybrid => self.select_hybrid(&priorities).await?,
            SelectionAlgorithm::Random => self.select_random(&priorities).await?,
        };
        
        // Calculate totals
        let total_gas_used: u64 = 21000; // TODO: Implement proper gas calculation
        let total_block_size_bytes = 1024; // TODO: Implement proper block size calculation
        let total_fees_collected: u64 = 1000; // TODO: Implement proper fee calculation
        
        let selection_time = start_time.elapsed().as_millis() as u64;
        
        // Update statistics
        self.update_stats(
            selection_time,
            selected_transactions.len(),
            pending_transactions.len(),
            total_gas_used,
            total_fees_collected,
        ).await;
        
        let result = TransactionSelectionResult {
            selected_transactions,
            total_transactions_considered: pending_transactions.len(),
            total_gas_used,
            total_block_size_bytes,
            total_fees_collected,
            selection_time_ms: selection_time,
            algorithm_used: self.config.selection_algorithm.clone(),
            replaced_transactions: vec![], // TODO: Implement transaction replacement
        };
        
        debug!("Selected {} transactions in {}ms", result.selected_transactions.len(), selection_time);
        Ok(result)
    }
    
    /// Get pending transactions from database
    async fn get_pending_transactions(&self) -> Result<Vec<StoredTransaction>> {
        // In a real implementation, this would query the database for pending transactions
        // For now, return empty vector as placeholder
        debug!("Getting pending transactions from database (placeholder)");
        Ok(vec![])
    }
    
    /// Calculate transaction priorities
    async fn calculate_transaction_priorities(
        &self,
        transactions: &[StoredTransaction],
    ) -> Result<Vec<TransactionPriority>> {
        let mut priorities = Vec::new();
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        for transaction in transactions {
            // Calculate age
            let age_seconds = current_time - transaction.timestamp;
            
            // Skip old transactions
            if age_seconds > self.config.max_transaction_age_seconds {
                continue;
            }
            
            // Calculate gas used
            let gas_used = transaction.gas_used.unwrap_or(21000);
            
            // Calculate fee per gas
            let fee_per_gas = transaction.fee as f64 / gas_used as f64;
            
            // Calculate priority score
            let score = self.calculate_priority_score(transaction, fee_per_gas, age_seconds);
            
            let priority = TransactionPriority {
                hash: transaction.hash,
                score,
                fee_per_gas,
                age_seconds,
                gas_used,
                size_bytes: 1024, // TODO: Implement proper transaction size calculation
            };
            
            priorities.push(priority);
        }
        
        Ok(priorities)
    }
    
    /// Calculate priority score for a transaction
    fn calculate_priority_score(
        &self,
        transaction: &StoredTransaction,
        fee_per_gas: f64,
        age_seconds: u64,
    ) -> f64 {
        let mut score = 0.0;
        
        // Fee component
        if self.config.enable_fee_prioritization {
            score += fee_per_gas * 0.4;
        }
        
        // Time component
        if self.config.enable_time_prioritization {
            score += (age_seconds as f64 / 3600.0) * 0.3; // Older transactions get higher score
        }
        
        // Gas price component
        if self.config.enable_gas_price_prioritization {
            score += fee_per_gas * 0.3;
        }
        
        score
    }
    
    /// Select transactions by fee
    async fn select_by_fee(&self, priorities: &[TransactionPriority]) -> Result<Vec<BFTTransaction>> {
        let mut heap = BinaryHeap::from(priorities.to_vec());
        let mut selected = Vec::new();
        let mut total_gas = 0;
        let mut total_size = 0;
        
        while let Some(priority) = heap.pop() {
            if selected.len() >= self.config.max_transactions_per_block {
                break;
            }
            
            if total_gas + priority.gas_used > self.config.max_gas_per_block {
                break;
            }
            
            if total_size + priority.size_bytes > self.config.max_block_size_bytes {
                break;
            }
            
            // Convert to BFT transaction
            let bft_transaction = self.convert_to_bft_transaction(priority.hash).await?;
            selected.push(bft_transaction);
            total_gas += priority.gas_used;
            total_size += priority.size_bytes;
        }
        
        Ok(selected)
    }
    
    /// Select transactions by time
    async fn select_by_time(&self, priorities: &[TransactionPriority]) -> Result<Vec<BFTTransaction>> {
        let mut sorted_priorities = priorities.to_vec();
        sorted_priorities.sort_by(|a, b| b.age_seconds.cmp(&a.age_seconds)); // Oldest first
        
        let mut selected = Vec::new();
        let mut total_gas = 0;
        let mut total_size = 0;
        
        for priority in sorted_priorities {
            if selected.len() >= self.config.max_transactions_per_block {
                break;
            }
            
            if total_gas + priority.gas_used > self.config.max_gas_per_block {
                break;
            }
            
            if total_size + priority.size_bytes > self.config.max_block_size_bytes {
                break;
            }
            
            let bft_transaction = self.convert_to_bft_transaction(priority.hash).await?;
            selected.push(bft_transaction);
            total_gas += priority.gas_used;
            total_size += priority.size_bytes;
        }
        
        Ok(selected)
    }
    
    /// Select transactions by gas price
    async fn select_by_gas_price(&self, priorities: &[TransactionPriority]) -> Result<Vec<BFTTransaction>> {
        let mut sorted_priorities = priorities.to_vec();
        sorted_priorities.sort_by(|a, b| b.fee_per_gas.partial_cmp(&a.fee_per_gas).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut selected = Vec::new();
        let mut total_gas = 0;
        let mut total_size = 0;
        
        for priority in sorted_priorities {
            if selected.len() >= self.config.max_transactions_per_block {
                break;
            }
            
            if total_gas + priority.gas_used > self.config.max_gas_per_block {
                break;
            }
            
            if total_size + priority.size_bytes > self.config.max_block_size_bytes {
                break;
            }
            
            let bft_transaction = self.convert_to_bft_transaction(priority.hash).await?;
            selected.push(bft_transaction);
            total_gas += priority.gas_used;
            total_size += priority.size_bytes;
        }
        
        Ok(selected)
    }
    
    /// Select transactions using hybrid algorithm
    async fn select_hybrid(&self, priorities: &[TransactionPriority]) -> Result<Vec<BFTTransaction>> {
        // Use a combination of fee and time
        let mut heap = BinaryHeap::from(priorities.to_vec());
        let mut selected = Vec::new();
        let mut total_gas = 0;
        let mut total_size = 0;
        
        while let Some(priority) = heap.pop() {
            if selected.len() >= self.config.max_transactions_per_block {
                break;
            }
            
            if total_gas + priority.gas_used > self.config.max_gas_per_block {
                break;
            }
            
            if total_size + priority.size_bytes > self.config.max_block_size_bytes {
                break;
            }
            
            let bft_transaction = self.convert_to_bft_transaction(priority.hash).await?;
            selected.push(bft_transaction);
            total_gas += priority.gas_used;
            total_size += priority.size_bytes;
        }
        
        Ok(selected)
    }
    
    /// Select transactions randomly
    async fn select_random(&self, priorities: &[TransactionPriority]) -> Result<Vec<BFTTransaction>> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        
        let mut rng = thread_rng();
        let mut shuffled = priorities.to_vec();
        shuffled.shuffle(&mut rng);
        
        let mut selected = Vec::new();
        let mut total_gas = 0;
        let mut total_size = 0;
        
        for priority in shuffled {
            if selected.len() >= self.config.max_transactions_per_block {
                break;
            }
            
            if total_gas + priority.gas_used > self.config.max_gas_per_block {
                break;
            }
            
            if total_size + priority.size_bytes > self.config.max_block_size_bytes {
                break;
            }
            
            let bft_transaction = self.convert_to_bft_transaction(priority.hash).await?;
            selected.push(bft_transaction);
            total_gas += priority.gas_used;
            total_size += priority.size_bytes;
        }
        
        Ok(selected)
    }
    
    /// Convert stored transaction to BFT transaction
    async fn convert_to_bft_transaction(&self, hash: TransactionHash) -> Result<BFTTransaction> {
        // In a real implementation, this would fetch the transaction from the database
        // For now, create a placeholder
        Ok(BFTTransaction {
            hash,
            data: vec![],
            sender: [0u8; 32], // Placeholder sender
            from: "placeholder_sender".to_string(),
            to: "placeholder_receiver".to_string(),
            amount: 0,
            fee: 0,
            gas_used: 21000,
            gas_price: 1,
            nonce: 1,
            signature: [0u8; 64],
        })
    }
    
    /// Update statistics
    async fn update_stats(
        &self,
        selection_time_ms: u64,
        transactions_selected: usize,
        transactions_considered: usize,
        total_gas_used: u64,
        total_fees_collected: u64,
    ) {
        let mut stats = self.stats.write().await;
        
        stats.total_selections += 1;
        stats.total_transactions_selected += transactions_selected as u64;
        stats.total_transactions_considered += transactions_considered as u64;
        
        // Update averages
        let total = stats.total_selections as f64;
        stats.average_selection_time_ms = 
            (stats.average_selection_time_ms * (total - 1.0) + selection_time_ms as f64) / total;
        stats.average_transactions_per_selection = 
            (stats.average_transactions_per_selection * (total - 1.0) + transactions_selected as f64) / total;
        stats.average_gas_used_per_selection = 
            (stats.average_gas_used_per_selection * (total - 1.0) + total_gas_used as f64) / total;
        stats.average_fees_collected_per_selection = 
            (stats.average_fees_collected_per_selection * (total - 1.0) + total_fees_collected as f64) / total;
        
        // Update efficiency
        if stats.total_transactions_considered > 0 {
            stats.selection_efficiency = stats.total_transactions_selected as f64 / stats.total_transactions_considered as f64;
        }
        
        // Update timestamps
        stats.last_selection = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
    
    /// Get transaction selection statistics
    pub async fn get_stats(&self) -> Result<TransactionSelectionStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_transaction_selection_config() {
        let config = TransactionSelectionConfig::default();
        assert_eq!(config.max_transactions_per_block, 1000);
        assert_eq!(config.max_block_size_bytes, 1024 * 1024);
        assert_eq!(config.max_gas_per_block, 10000000);
        assert!(config.enable_fee_prioritization);
        assert_eq!(config.selection_algorithm, SelectionAlgorithm::FeeBased);
    }
    
    #[tokio::test]
    async fn test_transaction_priority() {
        let priority = TransactionPriority {
            hash: [1u8; 32],
            score: 0.95,
            fee_per_gas: 10.5,
            age_seconds: 300,
            gas_used: 21000,
            size_bytes: 256,
        };
        
        assert_eq!(priority.score, 0.95);
        assert_eq!(priority.fee_per_gas, 10.5);
        assert_eq!(priority.age_seconds, 300);
        assert_eq!(priority.gas_used, 21000);
    }
    
    #[tokio::test]
    async fn test_transaction_selection_result() {
        let result = TransactionSelectionResult {
            selected_transactions: vec![],
            total_transactions_considered: 100,
            total_gas_used: 1000000,
            total_block_size_bytes: 51200,
            total_fees_collected: 5000,
            selection_time_ms: 25,
            algorithm_used: SelectionAlgorithm::FeeBased,
            replaced_transactions: vec![],
        };
        
        assert_eq!(result.total_transactions_considered, 100);
        assert_eq!(result.total_gas_used, 1000000);
        assert_eq!(result.total_block_size_bytes, 51200);
        assert_eq!(result.total_fees_collected, 5000);
        assert_eq!(result.selection_time_ms, 25);
    }
    
    #[tokio::test]
    async fn test_transaction_selection_stats() {
        let stats = TransactionSelectionStats {
            total_selections: 100,
            total_transactions_selected: 5000,
            total_transactions_considered: 10000,
            average_selection_time_ms: 15.5,
            average_transactions_per_selection: 50.0,
            average_gas_used_per_selection: 500000.0,
            average_fees_collected_per_selection: 2500.0,
            selection_efficiency: 0.5,
            uptime_seconds: 3600,
            last_selection: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
        };
        
        assert_eq!(stats.total_selections, 100);
        assert_eq!(stats.total_transactions_selected, 5000);
        assert_eq!(stats.total_transactions_considered, 10000);
        assert_eq!(stats.selection_efficiency, 0.5);
    }
}
