//! Unit tests for IPPAN subsystems
//! 
//! Tests individual components in isolation

use crate::{
    consensus::{Block, Transaction, HashTimer, ConsensusEngine, ConsensusConfig},
    storage::{StorageOrchestrator, StorageConfig, StorageUsage},
    network::{NetworkManager, NetworkConfig, NetworkStats},
    wallet::{WalletManager, WalletConfig},
    dht::{DhtManager, DhtConfig},
    staking::{StakingSystem, StakingConfig, StakingInfo},
    domain::{DomainSystem, DomainConfig, DomainInfo},
    api::{ApiLayer, HttpServer},
    utils::crypto,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unit test suite for consensus system
pub mod consensus_tests {
    use super::*;

    #[tokio::test]
    async fn test_hashtimer_creation() {
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        
        assert!(hashtimer.is_valid(10));
        assert!(hashtimer.ippan_time_ns > 0);
    }

    #[tokio::test]
    async fn test_block_creation() {
        let config = ConsensusConfig::default();
        let mut consensus = ConsensusEngine::new(config).unwrap();
        
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        
        let transactions = vec![
            Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer.clone()),
        ];
        
        let block = consensus.create_block(transactions, [4u8; 32]).await.unwrap();
        
        assert_eq!(block.header.round, consensus.current_round());
        assert_eq!(block.transactions.len(), 1);
        assert_eq!(block.header.validator_id, [4u8; 32]);
    }

    #[tokio::test]
    async fn test_block_validation() {
        let config = ConsensusConfig::default();
        let consensus = ConsensusEngine::new(config).unwrap();
        
        let hashtimer = HashTimer::with_ippan_time(
            [0u8; 32],
            [1u8; 32],
            1234567890,
        );
        
        let transactions = vec![
            Transaction::new([2u8; 32], 1000, [3u8; 32], hashtimer.clone()),
        ];
        
        let block = Block::new(
            1,
            transactions,
            [4u8; 32],
            hashtimer,
        );
        
        assert!(consensus.validate_block(&block).unwrap());
    }

    #[tokio::test]
    async fn test_validator_management() {
        let config = ConsensusConfig::default();
        let mut consensus = ConsensusEngine::new(config).unwrap();
        
        // Add validator
        consensus.add_validator([1u8; 32], 1000).unwrap();
        
        let validators = consensus.get_validators();
        assert!(validators.contains_key(&[1u8; 32]));
        assert_eq!(validators[&[1u8; 32]], 1000);
        
        // Remove validator
        consensus.remove_validator(&[1u8; 32]).unwrap();
        let validators = consensus.get_validators();
        assert!(!validators.contains_key(&[1u8; 32]));
    }

    #[tokio::test]
    async fn test_ippan_time() {
        let config = ConsensusConfig::default();
        let consensus = ConsensusEngine::new(config).unwrap();
        
        let ippan_time = consensus.get_ippan_time();
        assert!(ippan_time > 0);
        
        let time_stats = consensus.get_time_stats();
        assert!(time_stats.total_samples >= 0);
    }
}

/// Unit test suite for storage system
pub mod storage_tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_orchestrator_creation() {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        
        let usage = storage.get_usage();
        assert!(usage.total_bytes >= 0);
        assert!(usage.used_bytes >= 0);
    }

    #[tokio::test]
    async fn test_file_upload_download() {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().await.unwrap();
        
        let test_data = b"Hello, IPPAN! This is test data.";
        let file_hash = storage.upload_file("test.txt", test_data).await.unwrap();
        
        assert!(!file_hash.is_empty());
        
        let downloaded_data = storage.download_file(&file_hash).await.unwrap();
        assert_eq!(downloaded_data, test_data);
        
        storage.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_file_encryption() {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        
        let test_data = b"Secret data that needs encryption.";
        let encrypted_data = storage.encrypt_file(test_data).await.unwrap();
        
        assert_ne!(encrypted_data, test_data);
        assert!(encrypted_data.len() > test_data.len());
    }

    #[tokio::test]
    async fn test_storage_proofs() {
        let config = StorageConfig::default();
        let mut storage = StorageOrchestrator::new(config).unwrap();
        storage.start().await.unwrap();
        
        let test_data = b"Data for storage proof testing.";
        let file_hash = storage.upload_file("proof_test.txt", test_data).await.unwrap();
        
        let proof = storage.generate_storage_proof(&file_hash).await.unwrap();
        assert!(storage.verify_storage_proof(&file_hash, &proof).await.unwrap());
        
        storage.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_storage_statistics() {
        let config = StorageConfig::default();
        let storage = StorageOrchestrator::new(config).unwrap();
        
        let usage = storage.get_usage();
        assert!(usage.total_bytes >= 0);
        assert!(usage.used_bytes >= 0);
        assert!(usage.available_bytes >= 0);
    }
}

/// Unit test suite for network system
pub mod network_tests {
    use super::*;

    #[tokio::test]
    async fn test_network_manager_creation() {
        let config = NetworkConfig::default();
        let network = NetworkManager::new(config).unwrap();
        
        let stats = network.get_stats();
        assert!(stats.total_nodes >= 0);
        assert!(stats.active_nodes >= 0);
    }

    #[tokio::test]
    async fn test_peer_management() {
        let config = NetworkConfig::default();
        let mut network = NetworkManager::new(config).unwrap();
        network.start().await.unwrap();
        
        let initial_peers = network.get_peers();
        assert_eq!(initial_peers.len(), 0);
        
        // Test peer connection (simulated)
        network.connect_peer("127.0.0.1:8080").await.unwrap();
        
        let peer_count = network.get_peer_count();
        assert!(peer_count >= 0);
        
        network.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_network_statistics() {
        let config = NetworkConfig::default();
        let network = NetworkManager::new(config).unwrap();
        
        let stats = network.get_stats();
        assert!(stats.total_nodes >= 0);
        assert!(stats.active_nodes >= 0);
        assert!(stats.peer_count >= 0);
    }
}

/// Unit test suite for wallet system
pub mod wallet_tests {
    use super::*;

    #[tokio::test]
    async fn test_wallet_creation() {
        let config = WalletConfig::default();
        let wallet = WalletManager::new(config).await.unwrap();
        
        assert!(!wallet.running);
    }

    #[tokio::test]
    async fn test_wallet_lifecycle() {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).await.unwrap();
        
        wallet.start().await.unwrap();
        assert!(wallet.running);
        
        wallet.stop().await.unwrap();
        assert!(!wallet.running);
    }

    #[tokio::test]
    async fn test_key_management() {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).await.unwrap();
        wallet.start().await.unwrap();
        
        let keypair = wallet.keys.write().await.generate_keypair().await.unwrap();
        assert_eq!(keypair.public_key.len(), 32);
        assert_eq!(keypair.private_key.len(), 64);
    }

    #[tokio::test]
    async fn test_payment_processing() {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).await.unwrap();
        wallet.start().await.unwrap();
        
        let keypair = wallet.keys.write().await.generate_keypair().await.unwrap();
        
        let payment_tx = wallet.payments.write().await.process_payment(
            &keypair.public_key,
            1000,
            [1u8; 32],
        ).await.unwrap();
        
        assert_eq!(payment_tx.amount, 1000);
        assert_eq!(payment_tx.to_address, [1u8; 32]);
        
        wallet.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_m2m_payment_channels() {
        let config = WalletConfig::default();
        let mut wallet = WalletManager::new(config).await.unwrap();
        wallet.start().await.unwrap();
        
        let channel = wallet.create_payment_channel(
            "alice".to_string(),
            "bob".to_string(),
            10000,
            24,
        ).await.unwrap();
        
        assert_eq!(channel.sender, "alice");
        assert_eq!(channel.recipient, "bob");
        assert_eq!(channel.total_deposit, 10000);
        assert_eq!(channel.available_balance, 10000);
        
        wallet.stop().await.unwrap();
    }
}

/// Unit test suite for DHT system
pub mod dht_tests {
    use super::*;

    #[tokio::test]
    async fn test_dht_creation() {
        let config = DhtConfig::default();
        let node_id = [1u8; 32];
        let dht = DhtManager::new(config, node_id).await.unwrap();
        
        assert_eq!(dht.node_id, node_id);
    }

    #[tokio::test]
    async fn test_dht_lifecycle() {
        let config = DhtConfig::default();
        let node_id = [1u8; 32];
        let mut dht = DhtManager::new(config, node_id).await.unwrap();
        
        dht.start().await.unwrap();
        assert!(dht.is_running);
        
        dht.stop().await.unwrap();
        assert!(!dht.is_running);
    }

    #[tokio::test]
    async fn test_key_value_operations() {
        let config = DhtConfig::default();
        let node_id = [1u8; 32];
        let mut dht = DhtManager::new(config, node_id).await.unwrap();
        dht.start().await.unwrap();
        
        let key = "test_key".to_string();
        let value = "test_value".to_string();
        
        dht.put(&key, &value).await.unwrap();
        
        let retrieved_value = dht.get(&key).await.unwrap();
        assert_eq!(retrieved_value, Some(value));
        
        let keys = dht.get_keys();
        assert!(keys.contains(&key));
        
        dht.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_node_discovery() {
        let config = DhtConfig::default();
        let node_id = [1u8; 32];
        let mut dht = DhtManager::new(config, node_id).await.unwrap();
        dht.start().await.unwrap();
        
        let discovered_nodes = dht.discover_nodes().await.unwrap();
        assert!(discovered_nodes.len() >= 0);
        
        dht.stop().await.unwrap();
    }
}

/// Unit test suite for staking system
pub mod staking_tests {
    use super::*;

    #[tokio::test]
    async fn test_staking_system_creation() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(ConsensusConfig::default()).unwrap()));
        
        let staking = StakingSystem::new(wallet, consensus).unwrap();
        
        assert_eq!(staking.min_stake, 10_000_000);
        assert_eq!(staking.max_stake, 100_000_000);
    }

    #[tokio::test]
    async fn test_staking_operations() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(ConsensusConfig::default()).unwrap()));
        
        let staking = StakingSystem::new(wallet, consensus).unwrap();
        
        // Test staking
        let stake_result = staking.stake(10000).await.unwrap();
        assert_eq!(stake_result.amount, 10000);
        
        // Test staking info
        let staking_info = staking.get_staking_info().await;
        assert_eq!(staking_info.staked_amount, 10000);
        assert_eq!(staking_info.min_stake, 10_000_000);
        assert_eq!(staking_info.max_stake, 100_000_000);
    }

    #[tokio::test]
    async fn test_validator_management() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(ConsensusConfig::default()).unwrap()));
        
        let staking = StakingSystem::new(wallet, consensus).unwrap();
        
        let validators = staking.get_validators().await;
        assert!(validators.len() >= 0);
    }

    #[tokio::test]
    async fn test_global_fund_operations() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let consensus = Arc::new(RwLock::new(ConsensusEngine::new(ConsensusConfig::default()).unwrap()));
        
        let staking = StakingSystem::new(wallet, consensus).unwrap();
        
        // Test fee collection
        staking.add_transaction_fee(1000).await;
        staking.add_domain_fee(500).await;
        
        let balance = staking.get_global_fund_balance().await;
        assert_eq!(balance, 1500);
        
        // Test fund statistics
        let stats = staking.get_global_fund_stats().await;
        assert_eq!(stats.total_funds_ever, 1500);
        assert_eq!(stats.current_balance, 1500);
    }
}

/// Unit test suite for domain system
pub mod domain_tests {
    use super::*;

    #[tokio::test]
    async fn test_domain_system_creation() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let domain = DomainSystem::new(wallet).unwrap();
        
        assert!(!domain.is_running);
    }

    #[tokio::test]
    async fn test_domain_lifecycle() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let mut domain = DomainSystem::new(wallet).unwrap();
        
        domain.start().await.unwrap();
        assert!(domain.is_running);
        
        domain.stop().await.unwrap();
        assert!(!domain.is_running);
    }

    #[tokio::test]
    async fn test_domain_registration() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let mut domain = DomainSystem::new(wallet).unwrap();
        domain.start().await.unwrap();
        
        let domain_info = domain.register_domain("alice".to_string(), "ipn".to_string(), 365).await.unwrap();
        
        assert_eq!(domain_info.name, "alice.ipn");
        assert_eq!(domain_info.tld, "ipn");
        assert_eq!(domain_info.duration_days, 365);
        
        domain.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_domain_lookup() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let mut domain = DomainSystem::new(wallet).unwrap();
        domain.start().await.unwrap();
        
        // Register a domain first
        domain.register_domain("bob".to_string(), "ipn".to_string(), 365).await.unwrap();
        
        // Look up the domain
        let lookup_result = domain.lookup_domain("bob.ipn".to_string()).await.unwrap();
        assert!(lookup_result.is_some());
        
        domain.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_domain_renewal() {
        let wallet = Arc::new(RwLock::new(WalletManager::new(WalletConfig::default()).await.unwrap()));
        let mut domain = DomainSystem::new(wallet).unwrap();
        domain.start().await.unwrap();
        
        // Register a domain first
        domain.register_domain("charlie".to_string(), "ipn".to_string(), 365).await.unwrap();
        
        // Renew the domain
        let renewal_result = domain.renew_domain("charlie.ipn".to_string(), 365).await.unwrap();
        assert!(renewal_result);
        
        domain.stop().await.unwrap();
    }
}

/// Unit test suite for API system
pub mod api_tests {
    use super::*;

    #[tokio::test]
    async fn test_api_layer_creation() {
        let config = Config::default();
        let node = Arc::new(RwLock::new(IppanNode::new(config).await.unwrap()));
        let api = ApiLayer::new(node);
        
        assert!(api.http_server.is_some());
        assert!(api.cli.is_some());
        assert!(api.explorer.is_some());
    }

    #[tokio::test]
    async fn test_http_server_creation() {
        let config = Config::default();
        let node = Arc::new(RwLock::new(IppanNode::new(config).await.unwrap()));
        let server = HttpServer::new(node);
        
        assert_eq!(server.bind_addr, "127.0.0.1:8080");
    }
}

/// Unit test suite for utility functions
pub mod utility_tests {
    use super::*;

    #[test]
    fn test_crypto_functions() {
        // Test node ID generation
        let node_id = crypto::generate_node_id();
        assert_eq!(node_id.len(), 32);
        
        // Test hash generation
        let data = b"test data";
        let hash = crypto::hash(data);
        assert_eq!(hash.len(), 32);
        
        // Test signature generation and verification
        let keypair = crypto::generate_keypair();
        let message = b"test message";
        let signature = crypto::sign(message, &keypair.private_key);
        assert!(crypto::verify(message, &signature, &keypair.public_key));
    }

    #[test]
    fn test_time_utilities() {
        let now = std::time::SystemTime::now();
        let timestamp = crate::utils::time::get_timestamp();
        assert!(timestamp > 0);
        
        let duration = crate::utils::time::get_duration_since(now);
        assert!(duration.as_secs() >= 0);
    }
}

/// Run all unit tests
pub async fn run_unit_tests() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("🧪 Starting IPPAN unit tests...");

    // Run consensus tests
    consensus_tests::test_hashtimer_creation().await;
    consensus_tests::test_block_creation().await;
    consensus_tests::test_block_validation().await;
    consensus_tests::test_validator_management().await;
    consensus_tests::test_ippan_time().await;

    // Run storage tests
    storage_tests::test_storage_orchestrator_creation().await;
    storage_tests::test_file_upload_download().await;
    storage_tests::test_file_encryption().await;
    storage_tests::test_storage_proofs().await;
    storage_tests::test_storage_statistics().await;

    // Run network tests
    network_tests::test_network_manager_creation().await;
    network_tests::test_peer_management().await;
    network_tests::test_network_statistics().await;

    // Run wallet tests
    wallet_tests::test_wallet_creation().await;
    wallet_tests::test_wallet_lifecycle().await;
    wallet_tests::test_key_management().await;
    wallet_tests::test_payment_processing().await;
    wallet_tests::test_m2m_payment_channels().await;

    // Run DHT tests
    dht_tests::test_dht_creation().await;
    dht_tests::test_dht_lifecycle().await;
    dht_tests::test_key_value_operations().await;
    dht_tests::test_node_discovery().await;

    // Run staking tests
    staking_tests::test_staking_system_creation().await;
    staking_tests::test_staking_operations().await;
    staking_tests::test_validator_management().await;
    staking_tests::test_global_fund_operations().await;

    // Run domain tests
    domain_tests::test_domain_system_creation().await;
    domain_tests::test_domain_lifecycle().await;
    domain_tests::test_domain_registration().await;
    domain_tests::test_domain_lookup().await;
    domain_tests::test_domain_renewal().await;

    // Run API tests
    api_tests::test_api_layer_creation().await;
    api_tests::test_http_server_creation().await;

    // Run utility tests
    utility_tests::test_crypto_functions();
    utility_tests::test_time_utilities();

    log::info!("🎉 All unit tests completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all_unit_tests() {
        run_unit_tests().await.unwrap();
    }
} 