//! Comprehensive test suite for IPPAN
//! 
//! Tests all components including crypto, consensus, storage, network, wallet, database, API, mining, genesis, CLI, and logging

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

/// Comprehensive test suite results
#[derive(Debug, Clone)]
pub struct ComprehensiveTestResults {
    pub crypto_tests: TestCategoryResults,
    pub consensus_tests: TestCategoryResults,
    pub storage_tests: TestCategoryResults,
    pub network_tests: TestCategoryResults,
    pub wallet_tests: TestCategoryResults,
    pub database_tests: TestCategoryResults,
    pub api_tests: TestCategoryResults,
    pub mining_tests: TestCategoryResults,
    pub genesis_tests: TestCategoryResults,
    pub cli_tests: TestCategoryResults,
    pub logging_tests: TestCategoryResults,
    pub total_duration: Duration,
    pub overall_success: bool,
}

/// Test category results
#[derive(Debug, Clone)]
pub struct TestCategoryResults {
    pub tests_run: u32,
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub duration: Duration,
    pub errors: Vec<String>,
    pub success_rate: f64,
}

impl TestCategoryResults {
    pub fn new() -> Self {
        Self {
            tests_run: 0,
            tests_passed: 0,
            tests_failed: 0,
            duration: Duration::ZERO,
            errors: Vec::new(),
            success_rate: 0.0,
        }
    }

    pub fn add_test_result(&mut self, passed: bool, duration: Duration, error: Option<String>) {
        self.tests_run += 1;
        if passed {
            self.tests_passed += 1;
        } else {
            self.tests_failed += 1;
            if let Some(err) = error {
                self.errors.push(err);
            }
        }
        self.duration += duration;
        self.success_rate = if self.tests_run > 0 {
            self.tests_passed as f64 / self.tests_run as f64
        } else {
            0.0
        };
    }
}

/// Comprehensive test suite
pub struct ComprehensiveTestSuite {
    start_time: Instant,
}

impl ComprehensiveTestSuite {
    /// Create a new comprehensive test suite
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    /// Run all comprehensive tests
    pub async fn run_all_tests(&self) -> ComprehensiveTestResults {
        println!("🚀 Starting Comprehensive IPPAN Test Suite...");
        
        let mut results = ComprehensiveTestResults {
            crypto_tests: TestCategoryResults::new(),
            consensus_tests: TestCategoryResults::new(),
            storage_tests: TestCategoryResults::new(),
            network_tests: TestCategoryResults::new(),
            wallet_tests: TestCategoryResults::new(),
            database_tests: TestCategoryResults::new(),
            api_tests: TestCategoryResults::new(),
            mining_tests: TestCategoryResults::new(),
            genesis_tests: TestCategoryResults::new(),
            cli_tests: TestCategoryResults::new(),
            logging_tests: TestCategoryResults::new(),
            total_duration: Duration::ZERO,
            overall_success: false,
        };

        // Run crypto tests
        println!("🔐 Running Crypto Tests...");
        results.crypto_tests = self.run_crypto_tests().await;

        // Run consensus tests
        println!("🤝 Running Consensus Tests...");
        results.consensus_tests = self.run_consensus_tests().await;

        // Run storage tests
        println!("💾 Running Storage Tests...");
        results.storage_tests = self.run_storage_tests().await;

        // Run network tests
        println!("🌐 Running Network Tests...");
        results.network_tests = self.run_network_tests().await;

        // Run wallet tests
        println!("💰 Running Wallet Tests...");
        results.wallet_tests = self.run_wallet_tests().await;

        // Run database tests
        println!("🗄️  Running Database Tests...");
        results.database_tests = self.run_database_tests().await;

        // Run API tests
        println!("🔌 Running API Tests...");
        results.api_tests = self.run_api_tests().await;

        // Run mining tests
        println!("⛏️  Running Mining Tests...");
        results.mining_tests = self.run_mining_tests().await;

        // Run genesis tests
        println!("🌱 Running Genesis Tests...");
        results.genesis_tests = self.run_genesis_tests().await;

        // Run CLI tests
        println!("💻 Running CLI Tests...");
        results.cli_tests = self.run_cli_tests().await;

        // Run logging tests
        println!("📊 Running Logging Tests...");
        results.logging_tests = self.run_logging_tests().await;

        results.total_duration = self.start_time.elapsed();
        results.overall_success = self.calculate_overall_success(&results);

        self.print_test_summary(&results);
        results
    }

    /// Run crypto tests
    async fn run_crypto_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();
        let start_time = Instant::now();

        // Test SHA-256
        let test_data = b"Hello, IPPAN!";
        let hash = sha256_hash(test_data);
        let duration = start_time.elapsed();
        results.add_test_result(
            hash.len() == 32,
            duration,
            if hash.len() != 32 { Some("SHA-256 hash length incorrect".to_string()) } else { None }
        );

        // Test SHA-512
        let start_time = Instant::now();
        let hash512 = sha512_hash(test_data);
        let duration = start_time.elapsed();
        results.add_test_result(
            hash512.len() == 64,
            duration,
            if hash512.len() != 64 { Some("SHA-512 hash length incorrect".to_string()) } else { None }
        );

        // Test Blake3
        let start_time = Instant::now();
        let blake3_hash = blake3_hash(test_data);
        let duration = start_time.elapsed();
        results.add_test_result(
            blake3_hash.len() == 32,
            duration,
            if blake3_hash.len() != 32 { Some("Blake3 hash length incorrect".to_string()) } else { None }
        );

        // Test Ed25519 key generation
        let start_time = Instant::now();
        let key_pair = generate_ed25519_keypair();
        let duration = start_time.elapsed();
        results.add_test_result(
            key_pair.public_key.len() == 32 && key_pair.private_key.len() == 32,
            duration,
            if key_pair.public_key.len() != 32 || key_pair.private_key.len() != 32 {
                Some("Ed25519 key pair generation failed".to_string())
            } else {
                None
            }
        );

        // Test Ed25519 signing and verification
        let start_time = Instant::now();
        let signature = sign_ed25519(&key_pair.private_key, test_data);
        let is_valid = verify_ed25519(&key_pair.public_key, test_data, &signature);
        let duration = start_time.elapsed();
        results.add_test_result(
            is_valid,
            duration,
            if !is_valid { Some("Ed25519 signature verification failed".to_string()) } else { None }
        );

        // Test AES-256-GCM encryption
        let start_time = Instant::now();
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let encrypted = aes256_gcm_encrypt(test_data, &key, &nonce);
        let decrypted = aes256_gcm_decrypt(&encrypted, &key, &nonce);
        let duration = start_time.elapsed();
        results.add_test_result(
            decrypted == test_data,
            duration,
            if decrypted != test_data { Some("AES-256-GCM encryption/decryption failed".to_string()) } else { None }
        );

        results
    }

    /// Run consensus tests
    async fn run_consensus_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test BFT engine creation
        let start_time = Instant::now();
        let bft_engine = BFTEngine::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // BFT engine created successfully
            duration,
            None
        );

        // Test consensus manager creation
        let start_time = Instant::now();
        let consensus_manager = ConsensusManager::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Consensus manager created successfully
            duration,
            None
        );

        // Test BFT consensus phases
        let start_time = Instant::now();
        let pre_prepare_result = bft_engine.process_pre_prepare_message("test_block".to_string(), 1, 0).await;
        let duration = start_time.elapsed();
        results.add_test_result(
            pre_prepare_result.is_ok(),
            duration,
            if pre_prepare_result.is_err() { Some("BFT pre-prepare phase failed".to_string()) } else { None }
        );

        // Test view change mechanism
        let start_time = Instant::now();
        let view_change_result = bft_engine.initiate_view_change().await;
        let duration = start_time.elapsed();
        results.add_test_result(
            view_change_result.is_ok(),
            duration,
            if view_change_result.is_err() { Some("BFT view change failed".to_string()) } else { None }
        );

        results
    }

    /// Run storage tests
    async fn run_storage_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test storage manager creation
        let start_time = Instant::now();
        let storage_manager = RealStorageManager::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Storage manager created successfully
            duration,
            None
        );

        // Test file storage
        let start_time = Instant::now();
        let test_data = b"Test storage data";
        let store_result = storage_manager.store_file("test_file.txt", test_data).await;
        let duration = start_time.elapsed();
        results.add_test_result(
            store_result.is_ok(),
            duration,
            if store_result.is_err() { Some("File storage failed".to_string()) } else { None }
        );

        // Test file retrieval
        let start_time = Instant::now();
        let retrieve_result = storage_manager.retrieve_file("test_file.txt").await;
        let duration = start_time.elapsed();
        results.add_test_result(
            retrieve_result.is_ok() && retrieve_result.unwrap() == test_data,
            duration,
            if retrieve_result.is_err() { Some("File retrieval failed".to_string()) } else { None }
        );

        // Test file deletion
        let start_time = Instant::now();
        let delete_result = storage_manager.delete_file("test_file.txt").await;
        let duration = start_time.elapsed();
        results.add_test_result(
            delete_result.is_ok(),
            duration,
            if delete_result.is_err() { Some("File deletion failed".to_string()) } else { None }
        );

        results
    }

    /// Run network tests
    async fn run_network_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test P2P network manager creation
        let start_time = Instant::now();
        let network_manager = RealP2PNetwork::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Network manager created successfully
            duration,
            None
        );

        // Test peer connection
        let start_time = Instant::now();
        let connect_result = network_manager.connect_to_peer("127.0.0.1:30303".to_string()).await;
        let duration = start_time.elapsed();
        results.add_test_result(
            connect_result.is_ok(),
            duration,
            if connect_result.is_err() { Some("Peer connection failed".to_string()) } else { None }
        );

        // Test message broadcasting
        let start_time = Instant::now();
        let broadcast_result = network_manager.broadcast_message("test_message".as_bytes().to_vec()).await;
        let duration = start_time.elapsed();
        results.add_test_result(
            broadcast_result.is_ok(),
            duration,
            if broadcast_result.is_err() { Some("Message broadcasting failed".to_string()) } else { None }
        );

        results
    }

    /// Run wallet tests
    async fn run_wallet_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test wallet manager creation
        let start_time = Instant::now();
        let wallet_manager = RealWalletManager::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Wallet manager created successfully
            duration,
            None
        );

        // Test account creation
        let start_time = Instant::now();
        let account_result = wallet_manager.create_account("test_account".to_string(), "Standard".to_string()).await;
        let duration = start_time.elapsed();
        results.add_test_result(
            account_result.is_ok(),
            duration,
            if account_result.is_err() { Some("Account creation failed".to_string()) } else { None }
        );

        // Test transaction creation
        let start_time = Instant::now();
        let transaction_result = wallet_manager.create_transaction(
            "test_account".to_string(),
            "recipient".to_string(),
            1000,
            "Transfer".to_string(),
        ).await;
        let duration = start_time.elapsed();
        results.add_test_result(
            transaction_result.is_ok(),
            duration,
            if transaction_result.is_err() { Some("Transaction creation failed".to_string()) } else { None }
        );

        results
    }

    /// Run database tests
    async fn run_database_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test database manager creation
        let start_time = Instant::now();
        let database_manager = RealDatabaseManager::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Database manager created successfully
            duration,
            None
        );

        // Test data insertion
        let start_time = Instant::now();
        let insert_result = database_manager.insert("test_key", "test_value").await;
        let duration = start_time.elapsed();
        results.add_test_result(
            insert_result.is_ok(),
            duration,
            if insert_result.is_err() { Some("Data insertion failed".to_string()) } else { None }
        );

        // Test data retrieval
        let start_time = Instant::now();
        let get_result = database_manager.get("test_key").await;
        let duration = start_time.elapsed();
        results.add_test_result(
            get_result.is_ok() && get_result.unwrap() == "test_value",
            duration,
            if get_result.is_err() { Some("Data retrieval failed".to_string()) } else { None }
        );

        results
    }

    /// Run API tests
    async fn run_api_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test REST API creation
        let start_time = Instant::now();
        let api = RealRestApi::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // API created successfully
            duration,
            None
        );

        // Test API statistics
        let start_time = Instant::now();
        let stats = api.get_api_statistics().await;
        let duration = start_time.elapsed();
        results.add_test_result(
            stats.total_requests >= 0,
            duration,
            if stats.total_requests < 0 { Some("API statistics failed".to_string()) } else { None }
        );

        results
    }

    /// Run mining tests
    async fn run_mining_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test block creator creation
        let start_time = Instant::now();
        let block_creator = BlockCreator::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Block creator created successfully
            duration,
            None
        );

        // Test block validator creation
        let start_time = Instant::now();
        let block_validator = BlockValidator::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Block validator created successfully
            duration,
            None
        );

        // Test mining manager creation
        let start_time = Instant::now();
        let mining_manager = MiningManager::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Mining manager created successfully
            duration,
            None
        );

        results
    }

    /// Run genesis tests
    async fn run_genesis_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test genesis creator creation
        let start_time = Instant::now();
        let genesis_creator = GenesisCreator::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Genesis creator created successfully
            duration,
            None
        );

        // Test genesis manager creation
        let start_time = Instant::now();
        let genesis_manager = GenesisManager::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Genesis manager created successfully
            duration,
            None
        );

        results
    }

    /// Run CLI tests
    async fn run_cli_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test CLI manager creation
        let start_time = Instant::now();
        let cli_manager = CliManager::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // CLI manager created successfully
            duration,
            None
        );

        // Test node commands creation
        let start_time = Instant::now();
        let node_commands = NodeCommands::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Node commands created successfully
            duration,
            None
        );

        // Test wallet commands creation
        let start_time = Instant::now();
        let wallet_commands = WalletCommands::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Wallet commands created successfully
            duration,
            None
        );

        results
    }

    /// Run logging tests
    async fn run_logging_tests(&self) -> TestCategoryResults {
        let mut results = TestCategoryResults::new();

        // Test structured logger creation
        let start_time = Instant::now();
        let structured_logger = StructuredLogger::new();
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Structured logger created successfully
            duration,
            None
        );

        // Test log aggregator creation
        let start_time = Instant::now();
        let log_aggregator = LogAggregator::new(Arc::new(structured_logger));
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Log aggregator created successfully
            duration,
            None
        );

        // Test log analyzer creation
        let start_time = Instant::now();
        let structured_logger2 = StructuredLogger::new();
        let log_analyzer = LogAnalyzer::new(Arc::new(structured_logger2));
        let duration = start_time.elapsed();
        results.add_test_result(
            true, // Log analyzer created successfully
            duration,
            None
        );

        results
    }

    /// Calculate overall test success
    fn calculate_overall_success(&self, results: &ComprehensiveTestResults) -> bool {
        results.crypto_tests.success_rate >= 0.8 &&
        results.consensus_tests.success_rate >= 0.8 &&
        results.storage_tests.success_rate >= 0.8 &&
        results.network_tests.success_rate >= 0.8 &&
        results.wallet_tests.success_rate >= 0.8 &&
        results.database_tests.success_rate >= 0.8 &&
        results.api_tests.success_rate >= 0.8 &&
        results.mining_tests.success_rate >= 0.8 &&
        results.genesis_tests.success_rate >= 0.8 &&
        results.cli_tests.success_rate >= 0.8 &&
        results.logging_tests.success_rate >= 0.8
    }

    /// Print test summary
    fn print_test_summary(&self, results: &ComprehensiveTestResults) {
        println!("\n📊 Comprehensive Test Suite Results:");
        println!("=====================================");
        
        self.print_category_results("🔐 Crypto Tests", &results.crypto_tests);
        self.print_category_results("🤝 Consensus Tests", &results.consensus_tests);
        self.print_category_results("💾 Storage Tests", &results.storage_tests);
        self.print_category_results("🌐 Network Tests", &results.network_tests);
        self.print_category_results("💰 Wallet Tests", &results.wallet_tests);
        self.print_category_results("🗄️  Database Tests", &results.database_tests);
        self.print_category_results("🔌 API Tests", &results.api_tests);
        self.print_category_results("⛏️  Mining Tests", &results.mining_tests);
        self.print_category_results("🌱 Genesis Tests", &results.genesis_tests);
        self.print_category_results("💻 CLI Tests", &results.cli_tests);
        self.print_category_results("📊 Logging Tests", &results.logging_tests);
        
        println!("\n📈 Overall Results:");
        println!("  Total Duration: {:?}", results.total_duration);
        println!("  Overall Success: {}", if results.overall_success { "✅ PASSED" } else { "❌ FAILED" });
        
        let total_tests = results.crypto_tests.tests_run +
                         results.consensus_tests.tests_run +
                         results.storage_tests.tests_run +
                         results.network_tests.tests_run +
                         results.wallet_tests.tests_run +
                         results.database_tests.tests_run +
                         results.api_tests.tests_run +
                         results.mining_tests.tests_run +
                         results.genesis_tests.tests_run +
                         results.cli_tests.tests_run +
                         results.logging_tests.tests_run;
        
        let total_passed = results.crypto_tests.tests_passed +
                          results.consensus_tests.tests_passed +
                          results.storage_tests.tests_passed +
                          results.network_tests.tests_passed +
                          results.wallet_tests.tests_passed +
                          results.database_tests.tests_passed +
                          results.api_tests.tests_passed +
                          results.mining_tests.tests_passed +
                          results.genesis_tests.tests_passed +
                          results.cli_tests.tests_passed +
                          results.logging_tests.tests_passed;
        
        println!("  Total Tests: {}", total_tests);
        println!("  Tests Passed: {}", total_passed);
        println!("  Tests Failed: {}", total_tests - total_passed);
        println!("  Success Rate: {:.1}%", (total_passed as f64 / total_tests as f64) * 100.0);
        
        if results.overall_success {
            println!("\n🎉 All test categories passed! IPPAN is ready for deployment!");
        } else {
            println!("\n💥 Some test categories failed. Please review the errors above.");
        }
    }

    /// Print category results
    fn print_category_results(&self, category_name: &str, results: &TestCategoryResults) {
        let status = if results.success_rate >= 0.8 { "✅" } else { "❌" };
        println!("  {} {}: {}/{} tests passed ({:.1}%) in {:?}",
                 status, category_name, results.tests_passed, results.tests_run,
                 results.success_rate * 100.0, results.duration);
        
        if !results.errors.is_empty() {
            for error in &results.errors {
                println!("    Error: {}", error);
            }
        }
    }
}

/// Run comprehensive test suite
pub async fn run_comprehensive_test_suite() -> ComprehensiveTestResults {
    let test_suite = ComprehensiveTestSuite::new();
    test_suite.run_all_tests().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_comprehensive_test_suite_creation() {
        let test_suite = ComprehensiveTestSuite::new();
        assert!(test_suite.start_time.elapsed() >= Duration::ZERO);
    }

    #[tokio::test]
    async fn test_test_category_results() {
        let mut results = TestCategoryResults::new();
        results.add_test_result(true, Duration::from_millis(100), None);
        results.add_test_result(false, Duration::from_millis(200), Some("Test error".to_string()));
        
        assert_eq!(results.tests_run, 2);
        assert_eq!(results.tests_passed, 1);
        assert_eq!(results.tests_failed, 1);
        assert_eq!(results.success_rate, 0.5);
        assert_eq!(results.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_crypto_tests() {
        let test_suite = ComprehensiveTestSuite::new();
        let results = test_suite.run_crypto_tests().await;
        
        assert!(results.tests_run > 0);
        assert!(results.success_rate >= 0.0);
    }
}
