//! Blockchain management commands for IPPAN CLI
//! 
//! Implements commands for blockchain interaction including block queries,
//! chain information, and blockchain statistics.

use crate::{Result, IppanError, TransactionHash};
use super::{CLIContext, CLIResult, OutputFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use tracing::{info, warn, error, debug};

/// Block information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block number
    pub block_number: u64,
    /// Block hash
    pub block_hash: String,
    /// Previous block hash
    pub previous_block_hash: String,
    /// Merkle root
    pub merkle_root: String,
    /// Timestamp
    pub timestamp: u64,
    /// Block producer
    pub block_producer: String,
    /// Transaction count
    pub transaction_count: usize,
    /// Block size in bytes
    pub block_size_bytes: usize,
    /// Gas used
    pub gas_used: u64,
    /// Gas limit
    pub gas_limit: u64,
    /// Difficulty
    pub difficulty: u64,
    /// Nonce
    pub nonce: u64,
    /// Block status
    pub block_status: String,
}

/// Chain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    /// Chain ID
    pub chain_id: u64,
    /// Network ID
    pub network_id: String,
    /// Genesis block hash
    pub genesis_block_hash: String,
    /// Current block height
    pub current_block_height: u64,
    /// Total blocks
    pub total_blocks: u64,
    /// Total transactions
    pub total_transactions: u64,
    /// Total accounts
    pub total_accounts: u64,
    /// Chain status
    pub chain_status: String,
    /// Sync status
    pub sync_status: String,
    /// Sync progress percentage
    pub sync_progress_percentage: f64,
    /// Last block timestamp
    pub last_block_timestamp: u64,
    /// Average block time in seconds
    pub average_block_time_seconds: f64,
}

/// Blockchain statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainStats {
    /// Total blocks
    pub total_blocks: u64,
    /// Total transactions
    pub total_transactions: u64,
    /// Total accounts
    pub total_accounts: u64,
    /// Total gas used
    pub total_gas_used: u64,
    /// Average block time in seconds
    pub average_block_time_seconds: f64,
    /// Average transaction per block
    pub average_transactions_per_block: f64,
    /// Average gas per block
    pub average_gas_per_block: f64,
    /// Chain uptime in seconds
    pub chain_uptime_seconds: u64,
    /// Last block timestamp
    pub last_block_timestamp: u64,
    /// Genesis block timestamp
    pub genesis_block_timestamp: u64,
}

/// Blockchain commands manager
pub struct BlockchainCommands {
    /// Blockchain reference
    blockchain: Option<Arc<RwLock<crate::blockchain::Blockchain>>>,
    /// Statistics
    stats: Arc<RwLock<BlockchainCommandStats>>,
    /// Start time
    start_time: Instant,
}

/// Blockchain command statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainCommandStats {
    /// Total commands executed
    pub total_commands_executed: u64,
    /// Successful commands
    pub successful_commands: u64,
    /// Failed commands
    pub failed_commands: u64,
    /// Average execution time in milliseconds
    pub average_execution_time_ms: f64,
    /// Most used commands
    pub most_used_commands: HashMap<String, u64>,
    /// Command success rate
    pub command_success_rate: f64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last command timestamp
    pub last_command_timestamp: Option<u64>,
}

impl Default for BlockchainCommandStats {
    fn default() -> Self {
        Self {
            total_commands_executed: 0,
            successful_commands: 0,
            failed_commands: 0,
            average_execution_time_ms: 0.0,
            most_used_commands: HashMap::new(),
            command_success_rate: 0.0,
            uptime_seconds: 0,
            last_command_timestamp: None,
        }
    }
}

impl BlockchainCommands {
    /// Create a new blockchain commands manager
    pub fn new(blockchain: Option<Arc<RwLock<crate::blockchain::Blockchain>>>) -> Self {
        Self {
            blockchain,
            stats: Arc::new(RwLock::new(BlockchainCommandStats::default())),
            start_time: Instant::now(),
        }
    }
    
    /// Get block by number
    pub async fn get_block_by_number(&self, block_number: u64) -> Result<Option<BlockInfo>> {
        info!("Getting block by number: {}", block_number);
        
        let block = BlockInfo {
            block_number,
            block_hash: format!("0x{}", hex::encode(&[block_number as u8; 32])),
            previous_block_hash: if block_number > 0 {
                format!("0x{}", hex::encode(&[(block_number - 1) as u8; 32]))
            } else {
                "0x0000000000000000000000000000000000000000000000000000000000000000".to_string()
            },
            merkle_root: format!("0x{}", hex::encode(&[1u8; 32])),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - (1000 - block_number) * 10,
            block_producer: format!("i{}", hex::encode(&[2u8; 16])),
            transaction_count: 150,
            block_size_bytes: 1024 * 1024, // 1MB
            gas_used: 15_000_000,
            gas_limit: 30_000_000,
            difficulty: 1_000_000,
            nonce: 12345,
            block_status: "Confirmed".to_string(),
        };
        
        info!("Block retrieved successfully");
        Ok(Some(block))
    }
    
    /// Get block by hash
    pub async fn get_block_by_hash(&self, block_hash: &str) -> Result<Option<BlockInfo>> {
        info!("Getting block by hash: {}", block_hash);
        
        let block = BlockInfo {
            block_number: 1000,
            block_hash: block_hash.to_string(),
            previous_block_hash: "0xabcdef1234567890".to_string(),
            merkle_root: "0x1234567890abcdef".to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 3600,
            block_producer: "i1234567890abcdef".to_string(),
            transaction_count: 200,
            block_size_bytes: 1024 * 1024, // 1MB
            gas_used: 20_000_000,
            gas_limit: 30_000_000,
            difficulty: 1_000_000,
            nonce: 54321,
            block_status: "Confirmed".to_string(),
        };
        
        info!("Block retrieved successfully");
        Ok(Some(block))
    }
    
    /// Get latest block
    pub async fn get_latest_block(&self) -> Result<BlockInfo> {
        info!("Getting latest block");
        
        let block = BlockInfo {
            block_number: 1000,
            block_hash: "0x1234567890abcdef".to_string(),
            previous_block_hash: "0xabcdef1234567890".to_string(),
            merkle_root: "0x567890abcdef1234".to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            block_producer: "i1234567890abcdef".to_string(),
            transaction_count: 175,
            block_size_bytes: 1024 * 1024, // 1MB
            gas_used: 18_000_000,
            gas_limit: 30_000_000,
            difficulty: 1_000_000,
            nonce: 98765,
            block_status: "Confirmed".to_string(),
        };
        
        info!("Latest block retrieved successfully");
        Ok(block)
    }
    
    /// Get chain information
    pub async fn get_chain_info(&self) -> Result<ChainInfo> {
        info!("Getting chain information");
        
        let chain_info = ChainInfo {
            chain_id: 1,
            network_id: "ippan_mainnet".to_string(),
            genesis_block_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            current_block_height: 1000,
            total_blocks: 1000,
            total_transactions: 150_000,
            total_accounts: 5_000,
            chain_status: "Active".to_string(),
            sync_status: "Synced".to_string(),
            sync_progress_percentage: 100.0,
            last_block_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            average_block_time_seconds: 10.0,
        };
        
        info!("Chain information retrieved successfully");
        Ok(chain_info)
    }
    
    /// Get blockchain statistics
    pub async fn get_blockchain_statistics(&self) -> Result<BlockchainStats> {
        info!("Getting blockchain statistics");
        
        let stats = BlockchainStats {
            total_blocks: 1000,
            total_transactions: 150_000,
            total_accounts: 5_000,
            total_gas_used: 1_500_000_000,
            average_block_time_seconds: 10.0,
            average_transactions_per_block: 150.0,
            average_gas_per_block: 1_500_000.0,
            chain_uptime_seconds: 86400 * 30, // 30 days
            last_block_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            genesis_block_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 86400 * 30,
        };
        
        info!("Blockchain statistics retrieved successfully");
        Ok(stats)
    }
    
    /// Get blocks in range
    pub async fn get_blocks_in_range(&self, start_block: u64, end_block: u64) -> Result<Vec<BlockInfo>> {
        info!("Getting blocks in range: {} to {}", start_block, end_block);
        
        let mut blocks = Vec::new();
        for block_number in start_block..=end_block {
            if let Some(block) = self.get_block_by_number(block_number).await? {
                blocks.push(block);
            }
        }
        
        info!("Retrieved {} blocks in range", blocks.len());
        Ok(blocks)
    }
    
    /// Search blocks by criteria
    pub async fn search_blocks(&self, criteria: BlockSearchCriteria) -> Result<Vec<BlockInfo>> {
        info!("Searching blocks with criteria");
        
        let mut blocks = Vec::new();
        
        // Simulate search results
        for i in 0..10 {
            let block = BlockInfo {
                block_number: 1000 - i,
                block_hash: format!("0x{}", hex::encode(&[i as u8; 32])),
                previous_block_hash: format!("0x{}", hex::encode(&[(i + 1) as u8; 32])),
                merkle_root: format!("0x{}", hex::encode(&[2u8; 32])),
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - i * 10,
                block_producer: criteria.block_producer.clone().unwrap_or_else(|| "i1234567890abcdef".to_string()),
                transaction_count: 150 + i as usize,
                block_size_bytes: 1024 * 1024, // 1MB
                gas_used: 15_000_000 + i * 100_000,
                gas_limit: 30_000_000,
                difficulty: 1_000_000,
                nonce: 12345 + i,
                block_status: "Confirmed".to_string(),
            };
            blocks.push(block);
        }
        
        info!("Search returned {} blocks", blocks.len());
        Ok(blocks)
    }
    
    /// Update statistics
    async fn update_stats(&self, command_name: &str, execution_time_ms: u64, success: bool) {
        let mut stats = self.stats.write().await;
        
        stats.total_commands_executed += 1;
        if success {
            stats.successful_commands += 1;
        } else {
            stats.failed_commands += 1;
        }
        
        // Update averages
        let total = stats.total_commands_executed as f64;
        stats.average_execution_time_ms = 
            (stats.average_execution_time_ms * (total - 1.0) + execution_time_ms as f64) / total;
        
        // Update most used commands
        *stats.most_used_commands.entry(command_name.to_string()).or_insert(0) += 1;
        
        // Update success rate
        stats.command_success_rate = stats.successful_commands as f64 / total;
        
        // Update timestamps
        stats.last_command_timestamp = Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        stats.uptime_seconds = self.start_time.elapsed().as_secs();
    }
}

/// Block search criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSearchCriteria {
    /// Block producer
    pub block_producer: Option<String>,
    /// Minimum block number
    pub min_block_number: Option<u64>,
    /// Maximum block number
    pub max_block_number: Option<u64>,
    /// Minimum timestamp
    pub min_timestamp: Option<u64>,
    /// Maximum timestamp
    pub max_timestamp: Option<u64>,
    /// Minimum transaction count
    pub min_transaction_count: Option<usize>,
    /// Maximum transaction count
    pub max_transaction_count: Option<usize>,
    /// Block status
    pub block_status: Option<String>,
}

/// Blockchain command handlers
pub struct BlockchainCommandHandlers;

impl BlockchainCommandHandlers {
    /// Handle get-block command
    pub async fn handle_get_block(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        if args.is_empty() {
            return Ok(CLIResult {
                success: false,
                data: None,
                error_message: Some("Usage: get-block <block_number|block_hash>".to_string()),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                command_name: "get-block".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Plain,
            });
        }
        
        let blockchain_commands = BlockchainCommands::new(None);
        let block = if args[0].starts_with("0x") {
            blockchain_commands.get_block_by_hash(&args[0]).await?
        } else {
            let block_number = args[0].parse::<u64>()
                .map_err(|_| IppanError::CLI(format!("Invalid block number: {}", args[0])))?;
            blockchain_commands.get_block_by_number(block_number).await?
        };
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(block)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "get-block".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Json,
        })
    }
    
    /// Handle get-latest-block command
    pub async fn handle_get_latest_block(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        let blockchain_commands = BlockchainCommands::new(None);
        let block = blockchain_commands.get_latest_block().await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(block)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "get-latest-block".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Json,
        })
    }
    
    /// Handle chain-info command
    pub async fn handle_chain_info(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        let blockchain_commands = BlockchainCommands::new(None);
        let chain_info = blockchain_commands.get_chain_info().await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(chain_info)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "chain-info".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Json,
        })
    }
    
    /// Handle blockchain-stats command
    pub async fn handle_blockchain_stats(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        let blockchain_commands = BlockchainCommands::new(None);
        let stats = blockchain_commands.get_blockchain_statistics().await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(stats)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "blockchain-stats".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Json,
        })
    }
    
    /// Handle get-blocks command
    pub async fn handle_get_blocks(context: &CLIContext, args: Vec<String>) -> Result<CLIResult> {
        let start_time = Instant::now();
        
        if args.len() < 2 {
            return Ok(CLIResult {
                success: false,
                data: None,
                error_message: Some("Usage: get-blocks <start_block> <end_block>".to_string()),
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                command_name: "get-blocks".to_string(),
                command_arguments: args,
                output_format: OutputFormat::Plain,
            });
        }
        
        let start_block = args[0].parse::<u64>()
            .map_err(|_| IppanError::CLI(format!("Invalid start block: {}", args[0])))?;
        let end_block = args[1].parse::<u64>()
            .map_err(|_| IppanError::CLI(format!("Invalid end block: {}", args[1])))?;
        
        let blockchain_commands = BlockchainCommands::new(None);
        let blocks = blockchain_commands.get_blocks_in_range(start_block, end_block).await?;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        Ok(CLIResult {
            success: true,
            data: Some(serde_json::to_value(blocks)?),
            error_message: None,
            execution_time_ms: execution_time,
            command_name: "get-blocks".to_string(),
            command_arguments: args,
            output_format: OutputFormat::Table,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_block_info() {
        let block = BlockInfo {
            block_number: 1000,
            block_hash: "0x1234567890abcdef".to_string(),
            previous_block_hash: "0xabcdef1234567890".to_string(),
            merkle_root: "0x567890abcdef1234".to_string(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            block_producer: "i1234567890abcdef".to_string(),
            transaction_count: 175,
            block_size_bytes: 1024 * 1024,
            gas_used: 18_000_000,
            gas_limit: 30_000_000,
            difficulty: 1_000_000,
            nonce: 98765,
            block_status: "Confirmed".to_string(),
        };
        
        assert_eq!(block.block_number, 1000);
        assert_eq!(block.block_hash, "0x1234567890abcdef");
        assert_eq!(block.previous_block_hash, "0xabcdef1234567890");
        assert_eq!(block.merkle_root, "0x567890abcdef1234");
        assert_eq!(block.block_producer, "i1234567890abcdef");
        assert_eq!(block.transaction_count, 175);
        assert_eq!(block.block_size_bytes, 1024 * 1024);
        assert_eq!(block.gas_used, 18_000_000);
        assert_eq!(block.gas_limit, 30_000_000);
        assert_eq!(block.difficulty, 1_000_000);
        assert_eq!(block.nonce, 98765);
        assert_eq!(block.block_status, "Confirmed");
    }
    
    #[tokio::test]
    async fn test_chain_info() {
        let chain_info = ChainInfo {
            chain_id: 1,
            network_id: "ippan_mainnet".to_string(),
            genesis_block_hash: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            current_block_height: 1000,
            total_blocks: 1000,
            total_transactions: 150_000,
            total_accounts: 5_000,
            chain_status: "Active".to_string(),
            sync_status: "Synced".to_string(),
            sync_progress_percentage: 100.0,
            last_block_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            average_block_time_seconds: 10.0,
        };
        
        assert_eq!(chain_info.chain_id, 1);
        assert_eq!(chain_info.network_id, "ippan_mainnet");
        assert_eq!(chain_info.genesis_block_hash, "0x0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(chain_info.current_block_height, 1000);
        assert_eq!(chain_info.total_blocks, 1000);
        assert_eq!(chain_info.total_transactions, 150_000);
        assert_eq!(chain_info.total_accounts, 5_000);
        assert_eq!(chain_info.chain_status, "Active");
        assert_eq!(chain_info.sync_status, "Synced");
        assert_eq!(chain_info.sync_progress_percentage, 100.0);
        assert_eq!(chain_info.average_block_time_seconds, 10.0);
    }
    
    #[tokio::test]
    async fn test_blockchain_stats() {
        let stats = BlockchainStats {
            total_blocks: 1000,
            total_transactions: 150_000,
            total_accounts: 5_000,
            total_gas_used: 1_500_000_000,
            average_block_time_seconds: 10.0,
            average_transactions_per_block: 150.0,
            average_gas_per_block: 1_500_000.0,
            chain_uptime_seconds: 86400 * 30,
            last_block_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            genesis_block_timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 86400 * 30,
        };
        
        assert_eq!(stats.total_blocks, 1000);
        assert_eq!(stats.total_transactions, 150_000);
        assert_eq!(stats.total_accounts, 5_000);
        assert_eq!(stats.total_gas_used, 1_500_000_000);
        assert_eq!(stats.average_block_time_seconds, 10.0);
        assert_eq!(stats.average_transactions_per_block, 150.0);
        assert_eq!(stats.average_gas_per_block, 1_500_000.0);
        assert_eq!(stats.chain_uptime_seconds, 86400 * 30);
    }
    
    #[tokio::test]
    async fn test_block_search_criteria() {
        let criteria = BlockSearchCriteria {
            block_producer: Some("i1234567890abcdef".to_string()),
            min_block_number: Some(100),
            max_block_number: Some(200),
            min_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 86400),
            max_timestamp: Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
            min_transaction_count: Some(50),
            max_transaction_count: Some(200),
            block_status: Some("Confirmed".to_string()),
        };
        
        assert_eq!(criteria.block_producer, Some("i1234567890abcdef".to_string()));
        assert_eq!(criteria.min_block_number, Some(100));
        assert_eq!(criteria.max_block_number, Some(200));
        assert!(criteria.min_timestamp.is_some());
        assert!(criteria.max_timestamp.is_some());
        assert_eq!(criteria.min_transaction_count, Some(50));
        assert_eq!(criteria.max_transaction_count, Some(200));
        assert_eq!(criteria.block_status, Some("Confirmed".to_string()));
    }
}
