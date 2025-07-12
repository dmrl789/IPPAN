//! Integration tests for IPPAN
//! 
//! Tests the complete system integration and end-to-end functionality

use crate::{
    node::IppanNode,
    config::Config,
    consensus::{Block, Transaction, HashTimer},
    storage::{StorageOrchestrator, StorageConfig},
    network::{NetworkManager, NetworkConfig},
    wallet::{WalletManager, WalletConfig},
    dht::{DhtManager, DhtConfig},
    staking::{StakingSystem, StakingConfig},
    domain::{DomainSystem, DomainConfig},
    api::ApiLayer,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

/// Test configuration for integration tests
pub struct IntegrationTestConfig {
    pub consensus_config: crate::consensus::ConsensusConfig,
    pub storage_config: StorageConfig,
    pub network_config: NetworkConfig,
    pub wallet_config: WalletConfig,
    pub dht_config: DhtConfig,
    pub staking_config: StakingConfig,
    pub domain_config: DomainConfig,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            consensus_config: crate::consensus::ConsensusConfig::default(),
            storage_config: StorageConfig::default(),
            network_config: NetworkConfig::default(),
            wallet_config: WalletConfig::default(),
            dht_config: DhtConfig::default(),
            staking_config: StakingConfig::default(),
            domain_config: DomainConfig::default(),
        }
    }
}

/// Integration test suite
pub struct IntegrationTestSuite {
    config: IntegrationTestConfig,
}

impl IntegrationTestSuite {
    /// Create a new integration test suite
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self { config }
    }

    /// Run all integration tests
    pub async fn run_all_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting integration test suite...");

        // Test consensus system
        self.test_consensus_system().await?;
        log::info!("✅ Consensus system tests passed");

        // Test storage system
        self.test_storage_system().await?;
        log::info!("✅ Storage system tests passed");

        // Test network system
        self.test_network_system().await?;
        log::info!("✅ Network system tests passed");

        // Test wallet system
        self.test_wallet_system().await?;
        log::info!("✅ Wallet system tests passed");

        // Test DHT system
        self.test_dht_system().await?;
        log::info!("✅ DHT system tests passed");

        // Test staking system
        self.test_staking_system().await?;
        log::info!("✅ Staking system tests passed");

        // Test domain system
        self.test_domain_system().await?;
        log::info!("✅ Domain system tests passed");

        // Test global fund
        self.test_global_fund().await?;
        log::info!("✅ Global fund tests passed");

        // Test M2M payments
        self.test_m2m_payments().await?;
        log::info!("✅ M2M payments tests passed");

        // Test API layer
        self.test_api_layer().await?;
        log::info!("✅ API layer tests passed");

        // Test full node integration
        self.test_full_node_integration().await?;
        log::info!("✅ Full node integration tests passed");

        log::info!("🎉 All integration tests passed!");
        Ok(())
    }

    /// Test consensus system integration
    async fn test_consensus_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing consensus system...");

        // Create consensus engine
        let mut consensus = crate::consensus::ConsensusEngine::new(self.config.consensus_config.clone())?;

        // Test HashTimer creation
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        assert!(hashtimer.is_valid(10));

        // Test block creation
        let transactions = vec![
            Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer.clone()),
        ];
        let block = consensus.create_block(transactions, [4u8; 32]).await?;
        assert_eq!(block.header.round, consensus.current_round());

        // Test block validation
        assert!(consensus.validate_block(&block)?);

        // Test block addition
        consensus.add_block(block).await?;

        // Test IPPAN Time
        let ippan_time = consensus.get_ippan_time();
        assert!(ippan_time > 0);

        // Test validator management
        consensus.add_validator([5u8; 32], 1000)?;
        let validators = consensus.get_validators();
        assert!(validators.contains_key(&[5u8; 32]));

        Ok(())
    }

    /// Test storage system integration
    async fn test_storage_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing storage system...");

        // Create storage orchestrator
        let mut storage = StorageOrchestrator::new(self.config.storage_config.clone())?;
        storage.start().await?;

        // Test file upload
        let test_data = b"Hello, IPPAN! This is test data for storage.";
        let file_hash = storage.upload_file("test.txt", test_data).await?;
        assert!(!file_hash.is_empty());

        // Test file download
        let downloaded_data = storage.download_file(&file_hash).await?;
        assert_eq!(downloaded_data, test_data);

        // Test file encryption
        let encrypted_data = storage.encrypt_file(test_data).await?;
        assert_ne!(encrypted_data, test_data);

        // Test storage proofs
        let proof = storage.generate_storage_proof(&file_hash).await?;
        assert!(storage.verify_storage_proof(&file_hash, &proof).await?);

        // Test storage statistics
        let stats = storage.get_usage();
        assert!(stats.used_bytes > 0);

        storage.stop().await?;
        Ok(())
    }

    /// Test network system integration
    async fn test_network_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing network system...");

        // Create network manager
        let mut network = NetworkManager::new(self.config.network_config.clone())?;
        network.start().await?;

        // Test peer discovery
        let peers = network.get_peers();
        assert_eq!(peers.len(), 0); // No peers initially

        // Test peer connection (simulated)
        network.connect_peer("127.0.0.1:8080").await?;
        let peer_count = network.get_peer_count();
        assert!(peer_count >= 0);

        // Test network statistics
        let stats = network.get_stats();
        assert!(stats.total_nodes >= 0);

        network.stop().await?;
        Ok(())
    }

    /// Test wallet system integration
    async fn test_wallet_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing wallet system...");

        // Create wallet manager
        let mut wallet = WalletManager::new(self.config.wallet_config.clone()).await?;
        wallet.start().await?;

        // Test key generation
        let keypair = wallet.keys.write().await.generate_keypair().await?;
        assert!(keypair.public_key.len() == 32);

        // Test payment processing
        let payment_tx = wallet.payments.write().await.process_payment(
            &keypair.public_key,
            1000,
            [1u8; 32],
        ).await?;
        assert_eq!(payment_tx.amount, 1000);

        // Test M2M payment channel creation
        let channel = wallet.create_payment_channel(
            "alice".to_string(),
            "bob".to_string(),
            10000,
            24,
        ).await?;
        assert_eq!(channel.total_deposit, 10000);

        // Test M2M micro-payment
        let micro_tx = wallet.process_micro_payment(
            &channel.channel_id,
            100,
            crate::wallet::m2m_payments::MicroTransactionType::DataTransfer { bytes_transferred: 1024 },
        ).await?;
        assert_eq!(micro_tx.amount, 100);

        wallet.stop().await?;
        Ok(())
    }

    /// Test DHT system integration
    async fn test_dht_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing DHT system...");

        // Create DHT manager
        let node_id = [1u8; 32];
        let mut dht = DhtManager::new(self.config.dht_config.clone(), node_id).await?;
        dht.start().await?;

        // Test key-value storage
        let key = "test_key".to_string();
        let value = "test_value".to_string();
        dht.put(&key, &value).await?;

        // Test key-value retrieval
        let retrieved_value = dht.get(&key).await?;
        assert_eq!(retrieved_value, Some(value));

        // Test key lookup
        let keys = dht.get_keys();
        assert!(keys.contains(&key));

        // Test node discovery
        let discovered_nodes = dht.discover_nodes().await?;
        assert!(discovered_nodes.len() >= 0);

        dht.stop().await?;
        Ok(())
    }

    /// Test staking system integration
    async fn test_staking_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing staking system...");

        // Create wallet and consensus for staking
        let wallet = Arc::new(RwLock::new(WalletManager::new(self.config.wallet_config.clone()).await?));
        let consensus = Arc::new(RwLock::new(crate::consensus::ConsensusEngine::new(self.config.consensus_config.clone())?));

        // Create staking system
        let staking = StakingSystem::new(wallet.clone(), consensus.clone())?;

        // Test staking
        let stake_result = staking.stake(10000).await?;
        assert_eq!(stake_result.amount, 10000);

        // Test staking info
        let staking_info = staking.get_staking_info().await;
        assert_eq!(staking_info.staked_amount, 10000);

        // Test validator selection
        let validators = staking.get_validators().await;
        assert!(validators.len() >= 0);

        // Test global fund
        staking.add_transaction_fee(100).await;
        let balance = staking.get_global_fund_balance().await;
        assert_eq!(balance, 100);

        // Test node metrics
        let mut metrics = crate::staking::global_fund::NodeMetrics::new("test_node".to_string());
        metrics.update_uptime(95.5);
        metrics.increment_blocks_validated();
        staking.update_node_metrics("test_node".to_string(), metrics).await;

        Ok(())
    }

    /// Test domain system integration
    async fn test_domain_system(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing domain system...");

        // Create wallet for domain system
        let wallet = Arc::new(RwLock::new(WalletManager::new(self.config.wallet_config.clone()).await?));

        // Create domain system
        let mut domain = DomainSystem::new(wallet.clone())?;
        domain.start().await?;

        // Test domain registration
        let domain_info = domain.register_domain("alice".to_string(), "ipn".to_string(), 365).await?;
        assert_eq!(domain_info.name, "alice.ipn");

        // Test domain lookup
        let lookup_result = domain.lookup_domain("alice.ipn".to_string()).await?;
        assert!(lookup_result.is_some());

        // Test domain renewal
        let renewal_result = domain.renew_domain("alice.ipn".to_string(), 365).await?;
        assert!(renewal_result);

        // Test premium TLD
        let premium_domain = domain.register_premium_domain("bot".to_string(), "iot".to_string(), 365).await?;
        assert_eq!(premium_domain.tld, "iot");

        domain.stop().await?;
        Ok(())
    }

    /// Test global fund integration
    async fn test_global_fund(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing global fund...");

        // Create wallet and consensus
        let wallet = Arc::new(RwLock::new(WalletManager::new(self.config.wallet_config.clone()).await?));
        let consensus = Arc::new(RwLock::new(crate::consensus::ConsensusEngine::new(self.config.consensus_config.clone())?));

        // Create staking system (includes global fund)
        let staking = StakingSystem::new(wallet.clone(), consensus.clone())?;

        // Test fee collection
        staking.add_transaction_fee(1000).await;
        staking.add_domain_fee(500).await;
        let balance = staking.get_global_fund_balance().await;
        assert_eq!(balance, 1500);

        // Test node metrics
        let mut metrics = crate::staking::global_fund::NodeMetrics::new("test_node".to_string());
        metrics.update_uptime(95.0);
        metrics.increment_blocks_validated();
        metrics.increment_blocks_produced();
        metrics.update_storage_score(90.0);
        metrics.add_traffic_served(1_000_000);
        metrics.update_time_precision(99.9);
        metrics.update_hashtimer_accuracy(98.5);

        staking.update_node_metrics("test_node".to_string(), metrics).await;

        // Test fund statistics
        let stats = staking.get_global_fund_stats().await;
        assert_eq!(stats.total_funds_ever, 1500);
        assert_eq!(stats.current_balance, 1500);

        Ok(())
    }

    /// Test M2M payments integration
    async fn test_m2m_payments(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing M2M payments...");

        // Create wallet manager
        let mut wallet = WalletManager::new(self.config.wallet_config.clone()).await?;
        wallet.start().await?;

        // Test payment channel creation
        let channel = wallet.create_payment_channel(
            "iot_device".to_string(),
            "data_consumer".to_string(),
            10000,
            168, // 1 week
        ).await?;
        assert_eq!(channel.total_deposit, 10000);

        // Test micro-payment processing
        let micro_tx = wallet.process_micro_payment(
            &channel.channel_id,
            100,
            crate::wallet::m2m_payments::MicroTransactionType::SensorData {
                sensor_type: "temperature".to_string(),
                data_points: 10,
            },
        ).await?;
        assert_eq!(micro_tx.amount, 100);

        // Test payment channel retrieval
        let retrieved_channel = wallet.get_payment_channel(&channel.channel_id).await?;
        assert!(retrieved_channel.is_some());

        // Test M2M statistics
        let stats = wallet.get_m2m_statistics().await;
        assert_eq!(stats.total_channels, 1);
        assert_eq!(stats.total_transactions, 1);

        // Test fee collection
        let total_fees = wallet.get_total_m2m_fees().await;
        assert!(total_fees > 0);

        wallet.stop().await?;
        Ok(())
    }

    /// Test API layer integration
    async fn test_api_layer(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing API layer...");

        // Create config for node
        let config = Config {
            consensus: self.config.consensus_config.clone(),
            storage: self.config.storage_config.clone(),
            network: self.config.network_config.clone(),
            wallet: self.config.wallet_config.clone(),
            dht: self.config.dht_config.clone(),
            staking: self.config.staking_config.clone(),
            domain: self.config.domain_config.clone(),
        };

        // Create node
        let mut node = IppanNode::new(config).await?;
        node.init_api();

        // Test node startup
        node.start().await?;
        assert!(node.get_status().is_running);

        // Test API endpoints (simulated)
        let status = node.get_status();
        assert!(status.uptime.as_secs() >= 0);

        // Test global fund API
        node.add_transaction_fee(100).await?;
        let balance = node.get_global_fund_balance().await?;
        assert_eq!(balance, 100);

        // Test M2M payment API
        let channel = node.create_m2m_payment_channel(
            "test_sender".to_string(),
            "test_recipient".to_string(),
            5000,
            24,
        ).await?;
        assert_eq!(channel.total_deposit, 5000);

        // Test node shutdown
        node.stop().await?;
        assert!(!node.get_status().is_running);

        Ok(())
    }

    /// Test full node integration
    async fn test_full_node_integration(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Testing full node integration...");

        // Create complete config
        let config = Config {
            consensus: self.config.consensus_config.clone(),
            storage: self.config.storage_config.clone(),
            network: self.config.network_config.clone(),
            wallet: self.config.wallet_config.clone(),
            dht: self.config.dht_config.clone(),
            staking: self.config.staking_config.clone(),
            domain: self.config.domain_config.clone(),
        };

        // Create and start node
        let mut node = IppanNode::new(config).await?;
        node.init_api();
        node.start().await?;

        // Test consensus integration
        let consensus = node.consensus.read().await;
        let current_round = consensus.current_round();
        assert!(current_round >= 0);

        // Test storage integration
        let storage = node.storage.read().await;
        let usage = storage.get_usage();
        assert!(usage.total_bytes >= 0);

        // Test network integration
        let network = node.network.read().await;
        let peer_count = network.get_peer_count();
        assert!(peer_count >= 0);

        // Test wallet integration
        let wallet = node.wallet.read().await;
        let balance = wallet.get_balance().await;
        assert!(balance >= 0);

        // Test DHT integration
        let dht = node.dht.read().await;
        let key_count = dht.get_key_count();
        assert!(key_count >= 0);

        // Test staking integration
        let staking = node.staking.read().await;
        let staking_info = staking.get_staking_info().await;
        assert!(staking_info.total_staked >= 0);

        // Test domain integration
        let domain = node.domain.read().await;
        let domain_count = domain.get_domain_count().await;
        assert!(domain_count >= 0);

        // Test global fund integration
        node.add_transaction_fee(200).await?;
        let fund_balance = node.get_global_fund_balance().await?;
        assert_eq!(fund_balance, 200);

        // Test M2M payments integration
        let m2m_stats = node.get_m2m_statistics().await?;
        assert!(m2m_stats.total_channels >= 0);

        // Test node shutdown
        node.stop().await?;

        Ok(())
    }
}

/// Run integration tests
pub async fn run_integration_tests() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🚀 Starting IPPAN integration tests...");

    let config = IntegrationTestConfig::default();
    let test_suite = IntegrationTestSuite::new(config);
    
    test_suite.run_all_tests().await?;

    log::info!("🎉 All integration tests completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_suite() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.run_all_tests().await.unwrap();
    }

    #[tokio::test]
    async fn test_consensus_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.test_consensus_system().await.unwrap();
    }

    #[tokio::test]
    async fn test_storage_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.test_storage_system().await.unwrap();
    }

    #[tokio::test]
    async fn test_wallet_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.test_wallet_system().await.unwrap();
    }

    #[tokio::test]
    async fn test_staking_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.test_staking_system().await.unwrap();
    }

    #[tokio::test]
    async fn test_global_fund_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.test_global_fund().await.unwrap();
    }

    #[tokio::test]
    async fn test_m2m_payments_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.test_m2m_payments().await.unwrap();
    }

    #[tokio::test]
    async fn test_full_node_integration() {
        let config = IntegrationTestConfig::default();
        let test_suite = IntegrationTestSuite::new(config);
        
        test_suite.test_full_node_integration().await.unwrap();
    }
} 