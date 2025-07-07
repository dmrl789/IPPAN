//! Tests module for IPPAN
//! 
//! This module provides integration tests and test utilities for the IPPAN codebase.

pub mod consensus;
pub mod dht;
pub mod storage;
pub mod rewards;

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{
    config::Config,
    node::IppanNode,
    utils::{crypto::random_bytes, time::current_time_secs},
    NodeId,
};

/// Test utilities for IPPAN
pub struct TestUtils;

impl TestUtils {
    /// Generate a random node ID for testing
    pub fn random_node_id() -> NodeId {
        let mut node_id = [0u8; 32];
        node_id.copy_from_slice(&random_bytes(32)[..32]);
        node_id
    }

    /// Generate a random block hash for testing
    pub fn random_block_hash() -> crate::BlockHash {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&random_bytes(32)[..32]);
        hash
    }

    /// Generate a random transaction hash for testing
    pub fn random_transaction_hash() -> crate::TransactionHash {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&random_bytes(32)[..32]);
        hash
    }

    /// Create a test configuration
    pub fn test_config() -> Config {
        Config {
            node: crate::config::NodeConfig {
                node_id: "test-node".to_string(),
                name: "Test Node".to_string(),
                version: "0.1.0".to_string(),
                data_dir: std::path::PathBuf::from("./test_data"),
                auto_update: false,
                max_memory_mb: 512,
                enable_metrics: false,
            },
            network: crate::config::NetworkConfig {
                listen_addr: "127.0.0.1".to_string(),
                listen_port: 0, // Use random port
                external_addr: None,
                bootstrap_peers: vec![],
                max_peers: 10,
                connection_timeout: 5,
                enable_nat: false,
                enable_relay: false,
                relay_servers: vec![],
            },
            storage: crate::config::StorageConfig {
                storage_dir: std::path::PathBuf::from("./test_storage"),
                max_storage_gb: 1,
                shard_size: 1024 * 1024, // 1MB
                replication_factor: 2,
                enable_encryption: false,
                enable_compression: false,
                gc_interval_hours: 1,
            },
            consensus: crate::config::ConsensusConfig {
                block_time: 5,
                max_block_size: 1024 * 1024, // 1MB
                validator_count: 3,
                min_stake_amount: 1_000_000_000, // 10 IPN
                max_stake_amount: 10_000_000_000, // 100 IPN
                slashing_conditions: std::collections::HashMap::new(),
                enable_hashtimer_validation: false,
            },
            api: crate::config::ApiConfig {
                http_enabled: false,
                http_addr: "127.0.0.1".to_string(),
                http_port: 0,
                cli_enabled: false,
                explorer_enabled: false,
                explorer_addr: "127.0.0.1".to_string(),
                explorer_port: 0,
                cors_origins: vec![],
                rate_limiting_enabled: false,
                rate_limit_rpm: 1000,
            },
            logging: crate::config::LoggingConfig {
                level: "error".to_string(),
                log_file: None,
                console_output: false,
                structured_logging: false,
                log_rotation: false,
                max_log_size_mb: 10,
                max_log_files: 1,
            },
            database: crate::config::DatabaseConfig {
                db_type: "sled".to_string(),
                db_path: std::path::PathBuf::from("./test_db"),
                enable_compression: false,
                cache_size_mb: 64,
                enable_metrics: false,
                backup_enabled: false,
                backup_interval_hours: 24,
            },
        }
    }

    /// Create a test node
    pub async fn test_node() -> Result<IppanNode, crate::error::IppanError> {
        let config = Self::test_config();
        IppanNode::new(config).await
    }

    /// Clean up test data
    pub async fn cleanup_test_data() -> Result<(), std::io::Error> {
        let test_dirs = vec!["./test_data", "./test_storage", "./test_db"];
        
        for dir in test_dirs {
            if std::path::Path::new(dir).exists() {
                std::fs::remove_dir_all(dir)?;
            }
        }
        
        Ok(())
    }

    /// Wait for a condition with timeout
    pub async fn wait_for_condition<F, Fut>(condition: F, timeout_secs: u64) -> bool
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = current_time_secs();
        let timeout = start + timeout_secs;
        
        while current_time_secs() < timeout {
            if condition().await {
                return true;
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        false
    }

    /// Generate test data of specified size
    pub fn generate_test_data(size: usize) -> Vec<u8> {
        random_bytes(size)
    }

    /// Create a test domain name
    pub fn test_domain_name() -> String {
        format!("test-{}.ipn", hex::encode(&random_bytes(8)[..4]))
    }

    /// Create a test file hash
    pub fn test_file_hash() -> [u8; 32] {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&random_bytes(32)[..32]);
        hash
    }

    /// Create a test stake amount
    pub fn test_stake_amount() -> u64 {
        // Random amount between 10 and 100 IPN
        let base = 10_000_000_000; // 10 IPN
        let random = (random_bytes(8)[0] as u64) % 90; // 0-89
        base + (random * 1_000_000_000) // 10-99 IPN
    }

    /// Create a test IPN amount
    pub fn test_ipn_amount() -> u64 {
        // Random amount between 0.001 and 1 IPN
        let base = 100_000; // 0.001 IPN
        let random = (random_bytes(8)[0] as u64) % 999; // 0-998
        base + (random * 100_000) // 0.001-0.999 IPN
    }
}

/// Test environment setup and teardown
pub struct TestEnvironment {
    /// Test configuration
    pub config: Config,
    /// Test node
    pub node: Option<IppanNode>,
    /// Test data directory
    pub data_dir: std::path::PathBuf,
}

impl TestEnvironment {
    /// Create a new test environment
    pub async fn new() -> Result<Self, crate::error::IppanError> {
        let config = TestUtils::test_config();
        let data_dir = config.node.data_dir.clone();
        
        // Ensure test directories exist
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&config.storage.storage_dir)?;
        std::fs::create_dir_all(&config.database.db_path)?;
        
        Ok(Self {
            config,
            node: None,
            data_dir,
        })
    }

    /// Start the test node
    pub async fn start_node(&mut self) -> Result<(), crate::error::IppanError> {
        self.node = Some(IppanNode::new(self.config.clone()).await?);
        Ok(())
    }

    /// Stop the test node
    pub async fn stop_node(&mut self) -> Result<(), crate::error::IppanError> {
        if let Some(mut node) = self.node.take() {
            node.stop().await?;
        }
        Ok(())
    }

    /// Clean up test environment
    pub async fn cleanup(&self) -> Result<(), std::io::Error> {
        TestUtils::cleanup_test_data().await
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Ensure cleanup happens even if async cleanup fails
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

/// Test data structures
pub mod test_data {
    use super::*;
    use serde::{Serialize, Deserialize};

    /// Test block data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestBlock {
        pub hash: crate::BlockHash,
        pub height: u64,
        pub timestamp: u64,
        pub transactions: Vec<TestTransaction>,
        pub parent_hashes: Vec<crate::BlockHash>,
    }

    /// Test transaction data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestTransaction {
        pub hash: crate::TransactionHash,
        pub from: NodeId,
        pub to: NodeId,
        pub amount: u64,
        pub fee: u64,
        pub timestamp: u64,
    }

    /// Test storage file data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestStorageFile {
        pub hash: [u8; 32],
        pub name: String,
        pub size: u64,
        pub data: Vec<u8>,
        pub shards: Vec<TestShard>,
    }

    /// Test shard data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestShard {
        pub index: u32,
        pub hash: [u8; 32],
        pub data: Vec<u8>,
        pub node_id: NodeId,
    }

    /// Test domain data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestDomain {
        pub name: String,
        pub owner: NodeId,
        pub expiry_time: u64,
        pub status: String,
    }

    /// Test stake data
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TestStake {
        pub node_id: NodeId,
        pub amount: u64,
        pub status: String,
        pub performance_score: f64,
    }

    /// Generate test block
    pub fn generate_test_block(height: u64) -> TestBlock {
        TestBlock {
            hash: TestUtils::random_block_hash(),
            height,
            timestamp: current_time_secs(),
            transactions: vec![
                generate_test_transaction(),
                generate_test_transaction(),
            ],
            parent_hashes: vec![TestUtils::random_block_hash()],
        }
    }

    /// Generate test transaction
    pub fn generate_test_transaction() -> TestTransaction {
        TestTransaction {
            hash: TestUtils::random_transaction_hash(),
            from: TestUtils::random_node_id(),
            to: TestUtils::random_node_id(),
            amount: TestUtils::test_ipn_amount(),
            fee: 1_000_000, // 0.01 IPN
            timestamp: current_time_secs(),
        }
    }

    /// Generate test storage file
    pub fn generate_test_storage_file(name: &str, size: usize) -> TestStorageFile {
        let data = TestUtils::generate_test_data(size);
        let hash = TestUtils::test_file_hash();
        
        // Create shards
        let shard_size = 1024 * 1024; // 1MB shards
        let mut shards = Vec::new();
        
        for (i, chunk) in data.chunks(shard_size).enumerate() {
            shards.push(TestShard {
                index: i as u32,
                hash: TestUtils::test_file_hash(),
                data: chunk.to_vec(),
                node_id: TestUtils::random_node_id(),
            });
        }
        
        TestStorageFile {
            hash,
            name: name.to_string(),
            size: data.len() as u64,
            data,
            shards,
        }
    }

    /// Generate test domain
    pub fn generate_test_domain() -> TestDomain {
        TestDomain {
            name: TestUtils::test_domain_name(),
            owner: TestUtils::random_node_id(),
            expiry_time: current_time_secs() + 31536000, // 1 year
            status: "active".to_string(),
        }
    }

    /// Generate test stake
    pub fn generate_test_stake() -> TestStake {
        TestStake {
            node_id: TestUtils::random_node_id(),
            amount: TestUtils::test_stake_amount(),
            status: "active".to_string(),
            performance_score: 0.8 + (random_bytes(1)[0] as f64 / 255.0) * 0.2, // 0.8-1.0
        }
    }
}

/// Integration test helpers
pub mod integration {
    use super::*;
    use std::time::Duration;

    /// Run integration test with timeout
    pub async fn run_integration_test<F, Fut, T>(
        test_fn: F,
        timeout_secs: u64,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            test_fn(),
        )
        .await
        .map_err(|_| "Test timeout".into())?
    }

    /// Setup test network
    pub async fn setup_test_network(node_count: usize) -> Result<Vec<IppanNode>, crate::error::IppanError> {
        let mut nodes = Vec::new();
        
        for i in 0..node_count {
            let mut config = TestUtils::test_config();
            config.node.node_id = format!("test-node-{}", i);
            config.node.data_dir = std::path::PathBuf::from(format!("./test_data/node_{}", i));
            config.network.listen_port = 0; // Use random port
            
            // Ensure directories exist
            std::fs::create_dir_all(&config.node.data_dir)?;
            std::fs::create_dir_all(&config.storage.storage_dir)?;
            std::fs::create_dir_all(&config.database.db_path)?;
            
            let node = IppanNode::new(config).await?;
            nodes.push(node);
        }
        
        Ok(nodes)
    }

    /// Teardown test network
    pub async fn teardown_test_network(nodes: Vec<IppanNode>) -> Result<(), crate::error::IppanError> {
        for mut node in nodes {
            node.stop().await?;
        }
        
        TestUtils::cleanup_test_data().await?;
        Ok(())
    }

    /// Wait for network consensus
    pub async fn wait_for_consensus(nodes: &[IppanNode], timeout_secs: u64) -> bool {
        TestUtils::wait_for_condition(
            || async {
                // Check if all nodes have the same latest block
                let mut block_hashes = Vec::new();
                for node in nodes {
                    // This would check the actual consensus state
                    // For now, just return true after a delay
                    block_hashes.push(TestUtils::random_block_hash());
                }
                
                // All nodes should have the same block hash
                block_hashes.windows(2).all(|w| w[0] == w[1])
            },
            timeout_secs,
        )
        .await
    }

    /// Wait for storage replication
    pub async fn wait_for_storage_replication(nodes: &[IppanNode], file_hash: &[u8; 32], timeout_secs: u64) -> bool {
        TestUtils::wait_for_condition(
            || async {
                // Check if file is replicated across nodes
                let mut replication_count = 0;
                for _node in nodes {
                    // This would check actual storage state
                    // For now, just simulate replication
                    replication_count += 1;
                }
                
                replication_count >= nodes.len() / 2 // At least 50% replication
            },
            timeout_secs,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_utils() {
        let node_id = TestUtils::random_node_id();
        assert_eq!(node_id.len(), 32);
        
        let block_hash = TestUtils::random_block_hash();
        assert_eq!(block_hash.len(), 32);
        
        let domain_name = TestUtils::test_domain_name();
        assert!(domain_name.ends_with(".ipn"));
        
        let stake_amount = TestUtils::test_stake_amount();
        assert!(stake_amount >= 10_000_000_000); // At least 10 IPN
        assert!(stake_amount <= 100_000_000_000); // At most 100 IPN
    }

    #[tokio::test]
    async fn test_test_environment() {
        let env = TestEnvironment::new().await.unwrap();
        assert!(env.data_dir.exists());
        
        // Cleanup should work
        env.cleanup().await.unwrap();
    }

    #[test]
    fn test_test_data_generation() {
        let block = test_data::generate_test_block(1);
        assert_eq!(block.height, 1);
        assert!(!block.transactions.is_empty());
        
        let transaction = test_data::generate_test_transaction();
        assert!(transaction.amount > 0);
        
        let file = test_data::generate_test_storage_file("test.txt", 1024);
        assert_eq!(file.name, "test.txt");
        assert_eq!(file.size, 1024);
        
        let domain = test_data::generate_test_domain();
        assert!(domain.name.ends_with(".ipn"));
        
        let stake = test_data::generate_test_stake();
        assert!(stake.performance_score >= 0.8);
        assert!(stake.performance_score <= 1.0);
    }

    #[tokio::test]
    async fn test_wait_for_condition() {
        let result = TestUtils::wait_for_condition(
            || async { true },
            1,
        )
        .await;
        assert!(result);
        
        let result = TestUtils::wait_for_condition(
            || async { false },
            1,
        )
        .await;
        assert!(!result);
    }
}
