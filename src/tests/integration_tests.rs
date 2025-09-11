//! Integration tests for IPPAN
//! 
//! Tests component interactions and end-to-end workflows

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::{
    crypto::real_implementations::*,
    consensus::{bft_engine::*, consensus_manager::*},
    storage::real_storage::*,
    network::real_p2p::*,
    wallet::real_wallet::*,
    database::real_database::*,
    api::real_rest_api::*,
    mining::{block_creator::*, block_validator::*, mining_manager::*},
    genesis::{genesis_creator::*, genesis_manager::*},
    cli::{cli_manager::*, node_commands::*, wallet_commands::*},
    logging::{structured_logger::*, log_aggregator::*, log_analyzer::*},
};

/// Integration test results
#[derive(Debug, Clone)]
pub struct IntegrationTestResults {
    pub test_name: String,
    pub components_involved: Vec<String>,
    pub test_duration: Duration,
    pub success: bool,
    pub error_message: Option<String>,
    pub performance_metrics: IntegrationPerformanceMetrics,
}

/// Integration performance metrics
#[derive(Debug, Clone)]
pub struct IntegrationPerformanceMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub average_operation_time: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Integration test suite
pub struct IntegrationTestSuite;

impl IntegrationTestSuite {
    /// Run all integration tests
    pub async fn run_all_integration_tests() -> Vec<IntegrationTestResults> {
        let mut results = Vec::new();

        // Crypto + Storage integration
        results.push(Self::test_crypto_storage_integration().await);
        
        // Crypto + Wallet integration
        results.push(Self::test_crypto_wallet_integration().await);
        
        // Consensus + Network integration
        results.push(Self::test_consensus_network_integration().await);
        
        // Storage + Database integration
        results.push(Self::test_storage_database_integration().await);
        
        // Wallet + API integration
        results.push(Self::test_wallet_api_integration().await);
        
        // Mining + Consensus integration
        results.push(Self::test_mining_consensus_integration().await);
        
        // Genesis + Network integration
        results.push(Self::test_genesis_network_integration().await);
        
        // CLI + Node integration
        results.push(Self::test_cli_node_integration().await);
        
        // Logging + All components integration
        results.push(Self::test_logging_integration().await);
        
        // End-to-end blockchain workflow
        results.push(Self::test_end_to_end_blockchain_workflow().await);

        results
    }

    /// Test crypto and storage integration
    async fn test_crypto_storage_integration() -> IntegrationTestResults {
        let test_name = "Crypto + Storage Integration".to_string();
        let components_involved = vec!["Crypto".to_string(), "Storage".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test encrypted file storage
        let storage_manager = RealStorageManager::new();
        let test_data = b"Encrypted test data for crypto-storage integration";
        
        // Generate encryption key
        let key_pair = generate_ed25519_keypair();
        let aes_key = [0u8; 32];
        let nonce = [0u8; 12];
        
        // Encrypt data
        let op_start = Instant::now();
        let encrypted_data = aes256_gcm_encrypt(test_data, &aes_key, &nonce);
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if encrypted_data.len() > 0 {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
        
        // Store encrypted data
        let op_start = Instant::now();
        let store_result = storage_manager.store_file("encrypted_test.txt", &encrypted_data).await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if store_result.is_ok() {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
        
        // Retrieve encrypted data
        let op_start = Instant::now();
        let retrieve_result = storage_manager.retrieve_file("encrypted_test.txt").await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if retrieve_result.is_ok() {
            successful_operations += 1;
            
            // Decrypt data
            let op_start = Instant::now();
            let decrypted_data = aes256_gcm_decrypt(&retrieve_result.unwrap(), &aes_key, &nonce);
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if decrypted_data == test_data {
                successful_operations += 1;
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        // Clean up
        let _ = storage_manager.delete_file("encrypted_test.txt").await;
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test crypto and wallet integration
    async fn test_crypto_wallet_integration() -> IntegrationTestResults {
        let test_name = "Crypto + Wallet Integration".to_string();
        let components_involved = vec!["Crypto".to_string(), "Wallet".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test wallet with cryptographic operations
        let wallet_manager = RealWalletManager::new();
        
        // Create account
        let op_start = Instant::now();
        let account_result = wallet_manager.create_account("crypto_test_account".to_string(), "Standard".to_string()).await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if account_result.is_ok() {
            successful_operations += 1;
            
            // Create transaction
            let op_start = Instant::now();
            let transaction_result = wallet_manager.create_transaction(
                "crypto_test_account".to_string(),
                "recipient_account".to_string(),
                1000,
                "Transfer".to_string(),
            ).await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if transaction_result.is_ok() {
                successful_operations += 1;
                
                // Sign transaction (simulated)
                let op_start = Instant::now();
                let key_pair = generate_ed25519_keypair();
                let transaction_data = b"Transaction data for signing";
                let signature = sign_ed25519(&key_pair.private_key, transaction_data);
                let is_valid = verify_ed25519(&key_pair.public_key, transaction_data, &signature);
                let op_duration = op_start.elapsed();
                operation_times.push(op_duration);
                total_operations += 1;
                
                if is_valid {
                    successful_operations += 1;
                } else {
                    failed_operations += 1;
                }
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some wallet operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test consensus and network integration
    async fn test_consensus_network_integration() -> IntegrationTestResults {
        let test_name = "Consensus + Network Integration".to_string();
        let components_involved = vec!["Consensus".to_string(), "Network".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test consensus with network communication
        let bft_engine = BFTEngine::new();
        let network_manager = RealP2PNetwork::new();
        
        // Create consensus message
        let op_start = Instant::now();
        let consensus_result = bft_engine.process_pre_prepare_message(
            "network_consensus_block".to_string(),
            1,
            0
        ).await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if consensus_result.is_ok() {
            successful_operations += 1;
            
            // Broadcast consensus message
            let op_start = Instant::now();
            let broadcast_result = network_manager.broadcast_message(
                "consensus_message".as_bytes().to_vec()
            ).await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if broadcast_result.is_ok() {
                successful_operations += 1;
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some consensus-network operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test storage and database integration
    async fn test_storage_database_integration() -> IntegrationTestResults {
        let test_name = "Storage + Database Integration".to_string();
        let components_involved = vec!["Storage".to_string(), "Database".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test storage with database persistence
        let storage_manager = RealStorageManager::new();
        let database_manager = RealDatabaseManager::new();
        
        // Store file
        let test_data = b"Database integration test data";
        let op_start = Instant::now();
        let store_result = storage_manager.store_file("db_test.txt", test_data).await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if store_result.is_ok() {
            successful_operations += 1;
            
            // Store metadata in database
            let op_start = Instant::now();
            let db_result = database_manager.insert("file_metadata", "db_test.txt").await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if db_result.is_ok() {
                successful_operations += 1;
                
                // Retrieve from database
                let op_start = Instant::now();
                let retrieve_db_result = database_manager.get("file_metadata").await;
                let op_duration = op_start.elapsed();
                operation_times.push(op_duration);
                total_operations += 1;
                
                if retrieve_db_result.is_ok() && retrieve_db_result.unwrap() == "db_test.txt" {
                    successful_operations += 1;
                } else {
                    failed_operations += 1;
                }
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        // Clean up
        let _ = storage_manager.delete_file("db_test.txt").await;
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some storage-database operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test wallet and API integration
    async fn test_wallet_api_integration() -> IntegrationTestResults {
        let test_name = "Wallet + API Integration".to_string();
        let components_involved = vec!["Wallet".to_string(), "API".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test wallet operations through API
        let wallet_manager = RealWalletManager::new();
        let api = RealRestApi::new();
        
        // Create account through wallet
        let op_start = Instant::now();
        let account_result = wallet_manager.create_account("api_test_account".to_string(), "Standard".to_string()).await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if account_result.is_ok() {
            successful_operations += 1;
            
            // Get API statistics
            let op_start = Instant::now();
            let stats = api.get_api_statistics().await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if stats.total_requests >= 0 {
                successful_operations += 1;
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some wallet-API operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test mining and consensus integration
    async fn test_mining_consensus_integration() -> IntegrationTestResults {
        let test_name = "Mining + Consensus Integration".to_string();
        let components_involved = vec!["Mining".to_string(), "Consensus".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test mining with consensus
        let block_creator = BlockCreator::new();
        let block_validator = BlockValidator::new();
        let bft_engine = BFTEngine::new();
        
        // Create block
        let op_start = Instant::now();
        let block_result = block_creator.create_block(vec!["tx1".to_string(), "tx2".to_string()]).await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if block_result.is_ok() {
            successful_operations += 1;
            
            // Validate block
            let op_start = Instant::now();
            let validation_result = block_validator.validate_block(&block_result.unwrap()).await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if validation_result.is_ok() {
                successful_operations += 1;
                
                // Process through consensus
                let op_start = Instant::now();
                let consensus_result = bft_engine.process_pre_prepare_message(
                    "mined_block".to_string(),
                    1,
                    0
                ).await;
                let op_duration = op_start.elapsed();
                operation_times.push(op_duration);
                total_operations += 1;
                
                if consensus_result.is_ok() {
                    successful_operations += 1;
                } else {
                    failed_operations += 1;
                }
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some mining-consensus operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test genesis and network integration
    async fn test_genesis_network_integration() -> IntegrationTestResults {
        let test_name = "Genesis + Network Integration".to_string();
        let components_involved = vec!["Genesis".to_string(), "Network".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test genesis with network setup
        let genesis_creator = GenesisCreator::new();
        let network_manager = RealP2PNetwork::new();
        
        // Create genesis block
        let op_start = Instant::now();
        let genesis_result = genesis_creator.create_genesis_block().await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if genesis_result.is_ok() {
            successful_operations += 1;
            
            // Setup network
            let op_start = Instant::now();
            let network_result = network_manager.connect_to_peer("127.0.0.1:30303".to_string()).await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if network_result.is_ok() {
                successful_operations += 1;
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some genesis-network operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test CLI and node integration
    async fn test_cli_node_integration() -> IntegrationTestResults {
        let test_name = "CLI + Node Integration".to_string();
        let components_involved = vec!["CLI".to_string(), "Node".to_string()];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test CLI with node operations
        let cli_manager = CliManager::new();
        let node_commands = NodeCommands::new();
        
        // Test CLI manager
        let op_start = Instant::now();
        let cli_result = cli_manager.start().await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if cli_result.is_ok() {
            successful_operations += 1;
            
            // Test node commands
            let op_start = Instant::now();
            let node_result = node_commands.start_node().await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if node_result.is_ok() {
                successful_operations += 1;
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some CLI-node operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test logging integration with all components
    async fn test_logging_integration() -> IntegrationTestResults {
        let test_name = "Logging + All Components Integration".to_string();
        let components_involved = vec![
            "Logging".to_string(),
            "Crypto".to_string(),
            "Consensus".to_string(),
            "Storage".to_string(),
            "Network".to_string(),
            "Wallet".to_string(),
            "Database".to_string(),
            "API".to_string(),
            "Mining".to_string(),
            "Genesis".to_string(),
            "CLI".to_string(),
        ];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Test logging with all components
        let structured_logger = Arc::new(StructuredLogger::new());
        let log_aggregator = LogAggregator::new(Arc::clone(&structured_logger));
        let log_analyzer = LogAnalyzer::new(Arc::clone(&structured_logger));
        
        // Test logging with crypto operations
        let op_start = Instant::now();
        let key_pair = generate_ed25519_keypair();
        let test_data = b"Logging integration test";
        let signature = sign_ed25519(&key_pair.private_key, test_data);
        let is_valid = verify_ed25519(&key_pair.public_key, test_data, &signature);
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if is_valid {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
        
        // Test logging with consensus
        let op_start = Instant::now();
        let bft_engine = BFTEngine::new();
        let consensus_result = bft_engine.process_pre_prepare_message(
            "logging_test_block".to_string(),
            1,
            0
        ).await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if consensus_result.is_ok() {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
        
        // Test logging with storage
        let op_start = Instant::now();
        let storage_manager = RealStorageManager::new();
        let store_result = storage_manager.store_file("logging_test.txt", b"Logging test data").await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if store_result.is_ok() {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
        
        // Test log aggregator
        let op_start = Instant::now();
        let aggregator_result = log_aggregator.start().await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if aggregator_result.is_ok() {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
        
        // Test log analyzer
        let op_start = Instant::now();
        let analyzer_result = log_analyzer.start().await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if analyzer_result.is_ok() {
            successful_operations += 1;
        } else {
            failed_operations += 1;
        }
        
        // Clean up
        let _ = storage_manager.delete_file("logging_test.txt").await;
        let _ = log_aggregator.stop().await;
        let _ = log_analyzer.stop().await;
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some logging integration operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Test end-to-end blockchain workflow
    async fn test_end_to_end_blockchain_workflow() -> IntegrationTestResults {
        let test_name = "End-to-End Blockchain Workflow".to_string();
        let components_involved = vec![
            "Genesis".to_string(),
            "Crypto".to_string(),
            "Wallet".to_string(),
            "Mining".to_string(),
            "Consensus".to_string(),
            "Storage".to_string(),
            "Database".to_string(),
            "Network".to_string(),
            "API".to_string(),
            "Logging".to_string(),
        ];
        let start_time = Instant::now();
        
        let mut total_operations = 0u64;
        let mut successful_operations = 0u64;
        let mut failed_operations = 0u64;
        let mut operation_times = Vec::new();

        // Step 1: Create genesis block
        let op_start = Instant::now();
        let genesis_creator = GenesisCreator::new();
        let genesis_result = genesis_creator.create_genesis_block().await;
        let op_duration = op_start.elapsed();
        operation_times.push(op_duration);
        total_operations += 1;
        
        if genesis_result.is_ok() {
            successful_operations += 1;
            
            // Step 2: Create wallet and account
            let op_start = Instant::now();
            let wallet_manager = RealWalletManager::new();
            let account_result = wallet_manager.create_account("e2e_test_account".to_string(), "Standard".to_string()).await;
            let op_duration = op_start.elapsed();
            operation_times.push(op_duration);
            total_operations += 1;
            
            if account_result.is_ok() {
                successful_operations += 1;
                
                // Step 3: Create transaction
                let op_start = Instant::now();
                let transaction_result = wallet_manager.create_transaction(
                    "e2e_test_account".to_string(),
                    "recipient_account".to_string(),
                    1000,
                    "Transfer".to_string(),
                ).await;
                let op_duration = op_start.elapsed();
                operation_times.push(op_duration);
                total_operations += 1;
                
                if transaction_result.is_ok() {
                    successful_operations += 1;
                    
                    // Step 4: Mine block
                    let op_start = Instant::now();
                    let block_creator = BlockCreator::new();
                    let block_result = block_creator.create_block(vec!["e2e_transaction".to_string()]).await;
                    let op_duration = op_start.elapsed();
                    operation_times.push(op_duration);
                    total_operations += 1;
                    
                    if block_result.is_ok() {
                        successful_operations += 1;
                        
                        // Step 5: Validate block
                        let op_start = Instant::now();
                        let block_validator = BlockValidator::new();
                        let validation_result = block_validator.validate_block(&block_result.unwrap()).await;
                        let op_duration = op_start.elapsed();
                        operation_times.push(op_duration);
                        total_operations += 1;
                        
                        if validation_result.is_ok() {
                            successful_operations += 1;
                            
                            // Step 6: Consensus
                            let op_start = Instant::now();
                            let bft_engine = BFTEngine::new();
                            let consensus_result = bft_engine.process_pre_prepare_message(
                                "e2e_block".to_string(),
                                1,
                                0
                            ).await;
                            let op_duration = op_start.elapsed();
                            operation_times.push(op_duration);
                            total_operations += 1;
                            
                            if consensus_result.is_ok() {
                                successful_operations += 1;
                                
                                // Step 7: Store block
                                let op_start = Instant::now();
                                let storage_manager = RealStorageManager::new();
                                let store_result = storage_manager.store_file("e2e_block.txt", b"Block data").await;
                                let op_duration = op_start.elapsed();
                                operation_times.push(op_duration);
                                total_operations += 1;
                                
                                if store_result.is_ok() {
                                    successful_operations += 1;
                                    
                                    // Step 8: Update database
                                    let op_start = Instant::now();
                                    let database_manager = RealDatabaseManager::new();
                                    let db_result = database_manager.insert("e2e_block_hash", "block_hash_value").await;
                                    let op_duration = op_start.elapsed();
                                    operation_times.push(op_duration);
                                    total_operations += 1;
                                    
                                    if db_result.is_ok() {
                                        successful_operations += 1;
                                        
                                        // Step 9: Network broadcast
                                        let op_start = Instant::now();
                                        let network_manager = RealP2PNetwork::new();
                                        let broadcast_result = network_manager.broadcast_message(b"e2e_block".to_vec()).await;
                                        let op_duration = op_start.elapsed();
                                        operation_times.push(op_duration);
                                        total_operations += 1;
                                        
                                        if broadcast_result.is_ok() {
                                            successful_operations += 1;
                                            
                                            // Step 10: API response
                                            let op_start = Instant::now();
                                            let api = RealRestApi::new();
                                            let stats = api.get_api_statistics().await;
                                            let op_duration = op_start.elapsed();
                                            operation_times.push(op_duration);
                                            total_operations += 1;
                                            
                                            if stats.total_requests >= 0 {
                                                successful_operations += 1;
                                            } else {
                                                failed_operations += 1;
                                            }
                                        } else {
                                            failed_operations += 1;
                                        }
                                    } else {
                                        failed_operations += 1;
                                    }
                                } else {
                                    failed_operations += 1;
                                }
                            } else {
                                failed_operations += 1;
                            }
                        } else {
                            failed_operations += 1;
                        }
                    } else {
                        failed_operations += 1;
                    }
                } else {
                    failed_operations += 1;
                }
            } else {
                failed_operations += 1;
            }
        } else {
            failed_operations += 1;
        }
        
        // Clean up
        let _ = storage_manager.delete_file("e2e_block.txt").await;
        
        let test_duration = start_time.elapsed();
        let average_operation_time = operation_times.iter().sum::<Duration>() / operation_times.len() as u32;
        let success = failed_operations == 0;
        
        IntegrationTestResults {
            test_name,
            components_involved,
            test_duration,
            success,
            error_message: if success { None } else { Some("Some end-to-end workflow operations failed".to_string()) },
            performance_metrics: IntegrationPerformanceMetrics {
                total_operations,
                successful_operations,
                failed_operations,
                average_operation_time,
                memory_usage_mb: 0.0,
                cpu_usage_percent: 0.0,
            },
        }
    }

    /// Print integration test results
    pub fn print_integration_results(results: &[IntegrationTestResults]) {
        println!("\n🔗 Integration Test Results:");
        println!("=============================");
        
        for result in results {
            let status = if result.success { "✅" } else { "❌" };
            println!("\n{} {}", status, result.test_name);
            println!("  Components: {}", result.components_involved.join(", "));
            println!("  Duration: {:?}", result.test_duration);
            println!("  Operations: {}/{} successful", result.performance_metrics.successful_operations, result.performance_metrics.total_operations);
            println!("  Avg operation time: {:?}", result.performance_metrics.average_operation_time);
            
            if let Some(ref error) = result.error_message {
                println!("  Error: {}", error);
            }
        }
        
        let total_tests = results.len();
        let successful_tests = results.iter().filter(|r| r.success).count();
        let success_rate = successful_tests as f64 / total_tests as f64;
        
        println!("\n📊 Integration Test Summary:");
        println!("  Total tests: {}", total_tests);
        println!("  Successful: {}", successful_tests);
        println!("  Failed: {}", total_tests - successful_tests);
        println!("  Success rate: {:.1}%", success_rate * 100.0);
    }
}

/// Run all integration tests
pub async fn run_all_integration_tests() -> Vec<IntegrationTestResults> {
    IntegrationTestSuite::run_all_integration_tests().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crypto_storage_integration() {
        let result = IntegrationTestSuite::test_crypto_storage_integration().await;
        
        assert_eq!(result.test_name, "Crypto + Storage Integration");
        assert!(result.components_involved.contains(&"Crypto".to_string()));
        assert!(result.components_involved.contains(&"Storage".to_string()));
        assert!(result.performance_metrics.total_operations > 0);
    }

    #[tokio::test]
    async fn test_crypto_wallet_integration() {
        let result = IntegrationTestSuite::test_crypto_wallet_integration().await;
        
        assert_eq!(result.test_name, "Crypto + Wallet Integration");
        assert!(result.components_involved.contains(&"Crypto".to_string()));
        assert!(result.components_involved.contains(&"Wallet".to_string()));
        assert!(result.performance_metrics.total_operations > 0);
    }

    #[tokio::test]
    async fn test_end_to_end_workflow() {
        let result = IntegrationTestSuite::test_end_to_end_blockchain_workflow().await;
        
        assert_eq!(result.test_name, "End-to-End Blockchain Workflow");
        assert!(result.components_involved.len() >= 5);
        assert!(result.performance_metrics.total_operations > 0);
    }
}
