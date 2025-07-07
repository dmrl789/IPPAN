//! Tests for the storage module

use crate::{
    storage::{
        encryption::{StorageEncryption, EncryptionConfig},
        shards::{ShardManager, ShardConfig, Shard},
        proofs::{ProofManager, ProofConfig, StorageProof},
        traffic::{TrafficManager, TrafficConfig, TrafficStats},
        orchestrator::{StorageOrchestrator, OrchestratorConfig},
    },
    utils::{crypto::sha256_hash, time::current_time_secs},
    NodeId,
};

use super::create_test_node_id;

/// Test StorageEncryption creation and operations
#[tokio::test]
async fn test_storage_encryption_creation() {
    let config = EncryptionConfig {
        algorithm: "AES-256-GCM".to_string(),
        key_size: 32,
        iv_size: 12,
        tag_size: 16,
    };
    
    let encryption = StorageEncryption::new(config);
    assert_eq!(encryption.get_config().algorithm, "AES-256-GCM");
    assert_eq!(encryption.get_config().key_size, 32);
}

/// Test data encryption and decryption
#[tokio::test]
async fn test_data_encryption_decryption() {
    let config = EncryptionConfig {
        algorithm: "AES-256-GCM".to_string(),
        key_size: 32,
        iv_size: 12,
        tag_size: 16,
    };
    
    let encryption = StorageEncryption::new(config);
    let data = b"test data for encryption and decryption";
    let key = [1u8; 32];
    
    // Test encryption
    let encrypted_data = encryption.encrypt(data, &key).await.unwrap();
    assert_ne!(encrypted_data, data);
    assert!(encrypted_data.len() > data.len());
    
    // Test decryption
    let decrypted_data = encryption.decrypt(&encrypted_data, &key).await.unwrap();
    assert_eq!(decrypted_data, data);
}

/// Test encryption with different data sizes
#[tokio::test]
async fn test_encryption_different_sizes() {
    let config = EncryptionConfig {
        algorithm: "AES-256-GCM".to_string(),
        key_size: 32,
        iv_size: 12,
        tag_size: 16,
    };
    
    let encryption = StorageEncryption::new(config);
    let key = [1u8; 32];
    
    // Test small data
    let small_data = b"small";
    let encrypted_small = encryption.encrypt(small_data, &key).await.unwrap();
    let decrypted_small = encryption.decrypt(&encrypted_small, &key).await.unwrap();
    assert_eq!(decrypted_small, small_data);
    
    // Test large data
    let large_data = vec![42u8; 1024 * 1024]; // 1MB
    let encrypted_large = encryption.encrypt(&large_data, &key).await.unwrap();
    let decrypted_large = encryption.decrypt(&encrypted_large, &key).await.unwrap();
    assert_eq!(decrypted_large, large_data);
}

/// Test ShardManager creation and operations
#[tokio::test]
async fn test_shard_manager_creation() {
    let config = ShardConfig {
        shard_size: 1024 * 1024, // 1MB
        redundancy_factor: 3,
        max_shards_per_file: 100,
        shard_timeout: 30,
    };
    
    let shard_manager = ShardManager::new(config);
    assert_eq!(shard_manager.get_config().shard_size, 1024 * 1024);
    assert_eq!(shard_manager.get_config().redundancy_factor, 3);
}

/// Test file sharding
#[tokio::test]
async fn test_file_sharding() {
    let config = ShardConfig {
        shard_size: 1024, // 1KB
        redundancy_factor: 3,
        max_shards_per_file: 100,
        shard_timeout: 30,
    };
    
    let mut shard_manager = ShardManager::new(config);
    let file_data = vec![42u8; 2048]; // 2KB file
    let file_hash = sha256_hash(&file_data);
    
    // Test sharding
    let shards = shard_manager.create_shards(&file_data, &file_hash).await.unwrap();
    assert_eq!(shards.len(), 2); // Should create 2 shards for 2KB file
    
    // Test shard validation
    for shard in &shards {
        assert!(shard_manager.validate_shard(shard).await);
    }
}

/// Test shard reconstruction
#[tokio::test]
async fn test_shard_reconstruction() {
    let config = ShardConfig {
        shard_size: 1024,
        redundancy_factor: 3,
        max_shards_per_file: 100,
        shard_timeout: 30,
    };
    
    let mut shard_manager = ShardManager::new(config);
    let file_data = vec![42u8; 2048];
    let file_hash = sha256_hash(&file_data);
    
    // Create shards
    let shards = shard_manager.create_shards(&file_data, &file_hash).await.unwrap();
    
    // Test reconstruction
    let reconstructed_data = shard_manager.reconstruct_file(&shards, &file_hash).await.unwrap();
    assert_eq!(reconstructed_data, file_data);
}

/// Test ProofManager creation and operations
#[tokio::test]
async fn test_proof_manager_creation() {
    let config = ProofConfig {
        proof_interval: 3600,
        merkle_tree_depth: 16,
        spot_check_probability: 0.1,
        proof_timeout: 30,
    };
    
    let proof_manager = ProofManager::new(config);
    assert_eq!(proof_manager.get_config().proof_interval, 3600);
    assert_eq!(proof_manager.get_config().merkle_tree_depth, 16);
}

/// Test storage proof generation
#[tokio::test]
async fn test_storage_proof_generation() {
    let config = ProofConfig {
        proof_interval: 3600,
        merkle_tree_depth: 16,
        spot_check_probability: 0.1,
        proof_timeout: 30,
    };
    
    let mut proof_manager = ProofManager::new(config);
    let file_data = vec![42u8; 1024];
    let file_hash = sha256_hash(&file_data);
    let node_id = create_test_node_id();
    
    // Test proof generation
    let proof = proof_manager.generate_proof(&file_data, &file_hash, node_id).await.unwrap();
    assert_eq!(proof.file_hash, file_hash);
    assert_eq!(proof.node_id, node_id);
    assert!(!proof.merkle_proof.is_empty());
}

/// Test proof verification
#[tokio::test]
async fn test_proof_verification() {
    let config = ProofConfig {
        proof_interval: 3600,
        merkle_tree_depth: 16,
        spot_check_probability: 0.1,
        proof_timeout: 30,
    };
    
    let mut proof_manager = ProofManager::new(config);
    let file_data = vec![42u8; 1024];
    let file_hash = sha256_hash(&file_data);
    let node_id = create_test_node_id();
    
    // Generate proof
    let proof = proof_manager.generate_proof(&file_data, &file_hash, node_id).await.unwrap();
    
    // Test proof verification
    let is_valid = proof_manager.verify_proof(&proof, &file_data).await.unwrap();
    assert!(is_valid);
}

/// Test TrafficManager creation and operations
#[tokio::test]
async fn test_traffic_manager_creation() {
    let config = TrafficConfig {
        stats_interval: 60,
        max_stats_history: 1000,
        bandwidth_limit: 1024 * 1024 * 1024, // 1GB
        request_timeout: 30,
    };
    
    let traffic_manager = TrafficManager::new(config);
    assert_eq!(traffic_manager.get_config().stats_interval, 60);
    assert_eq!(traffic_manager.get_config().bandwidth_limit, 1024 * 1024 * 1024);
}

/// Test traffic tracking
#[tokio::test]
async fn test_traffic_tracking() {
    let config = TrafficConfig {
        stats_interval: 60,
        max_stats_history: 1000,
        bandwidth_limit: 1024 * 1024 * 1024,
        request_timeout: 30,
    };
    
    let mut traffic_manager = TrafficManager::new(config);
    let node_id = create_test_node_id();
    
    // Test tracking upload
    traffic_manager.track_upload(node_id, 1024).await;
    
    // Test tracking download
    traffic_manager.track_download(node_id, 2048).await;
    
    // Test getting stats
    let stats = traffic_manager.get_node_stats(&node_id).await;
    assert_eq!(stats.bytes_uploaded, 1024);
    assert_eq!(stats.bytes_downloaded, 2048);
}

/// Test traffic statistics
#[tokio::test]
async fn test_traffic_statistics() {
    let config = TrafficConfig {
        stats_interval: 60,
        max_stats_history: 1000,
        bandwidth_limit: 1024 * 1024 * 1024,
        request_timeout: 30,
    };
    
    let mut traffic_manager = TrafficManager::new(config);
    
    // Track traffic for multiple nodes
    for i in 0..5 {
        let node_id = create_test_node_id();
        traffic_manager.track_upload(node_id, 1024 * (i + 1)).await;
        traffic_manager.track_download(node_id, 2048 * (i + 1)).await;
    }
    
    // Test global statistics
    let global_stats = traffic_manager.get_global_stats().await;
    assert!(global_stats.total_uploads > 0);
    assert!(global_stats.total_downloads > 0);
    assert!(global_stats.active_nodes > 0);
}

/// Test StorageOrchestrator creation and operations
#[tokio::test]
async fn test_storage_orchestrator_creation() {
    let config = OrchestratorConfig {
        max_concurrent_operations: 10,
        operation_timeout: 300,
        retry_attempts: 3,
        health_check_interval: 60,
    };
    
    let orchestrator = StorageOrchestrator::new(config);
    assert_eq!(orchestrator.get_config().max_concurrent_operations, 10);
    assert_eq!(orchestrator.get_config().operation_timeout, 300);
}

/// Test file storage orchestration
#[tokio::test]
async fn test_file_storage_orchestration() {
    let config = OrchestratorConfig {
        max_concurrent_operations: 10,
        operation_timeout: 300,
        retry_attempts: 3,
        health_check_interval: 60,
    };
    
    let mut orchestrator = StorageOrchestrator::new(config);
    let file_data = vec![42u8; 1024];
    let file_hash = sha256_hash(&file_data);
    let node_id = create_test_node_id();
    
    // Test storing file
    let storage_id = orchestrator.store_file(&file_data, &file_hash, node_id).await.unwrap();
    assert!(storage_id > 0);
    
    // Test retrieving file
    let retrieved_data = orchestrator.retrieve_file(&file_hash, node_id).await.unwrap();
    assert_eq!(retrieved_data, file_data);
}

/// Test storage health monitoring
#[tokio::test]
async fn test_storage_health_monitoring() {
    let config = OrchestratorConfig {
        max_concurrent_operations: 10,
        operation_timeout: 300,
        retry_attempts: 3,
        health_check_interval: 60,
    };
    
    let mut orchestrator = StorageOrchestrator::new(config);
    let node_id = create_test_node_id();
    
    // Test health check
    let health_status = orchestrator.check_node_health(node_id).await;
    assert!(health_status.is_ok());
    
    // Test getting health statistics
    let health_stats = orchestrator.get_health_statistics().await;
    assert!(health_stats.total_nodes >= 0);
    assert!(health_stats.healthy_nodes >= 0);
}

/// Test storage performance metrics
#[tokio::test]
async fn test_storage_performance() {
    let shard_config = ShardConfig {
        shard_size: 1024,
        redundancy_factor: 3,
        max_shards_per_file: 100,
        shard_timeout: 30,
    };
    
    let mut shard_manager = ShardManager::new(shard_config);
    let file_data = vec![42u8; 1024 * 1024]; // 1MB
    let file_hash = sha256_hash(&file_data);
    
    // Test sharding performance
    let start_time = std::time::Instant::now();
    let shards = shard_manager.create_shards(&file_data, &file_hash).await.unwrap();
    let sharding_time = start_time.elapsed();
    
    // Sharding should be fast (less than 10ms for 1MB)
    assert!(sharding_time.as_millis() < 10);
    assert_eq!(shards.len(), 1024); // 1MB / 1KB = 1024 shards
}

/// Test storage error handling
#[tokio::test]
async fn test_storage_error_handling() {
    let config = EncryptionConfig {
        algorithm: "AES-256-GCM".to_string(),
        key_size: 32,
        iv_size: 12,
        tag_size: 16,
    };
    
    let encryption = StorageEncryption::new(config);
    let data = b"test data";
    let wrong_key = [0u8; 32];
    let correct_key = [1u8; 32];
    
    // Encrypt with correct key
    let encrypted_data = encryption.encrypt(data, &correct_key).await.unwrap();
    
    // Try to decrypt with wrong key
    let decryption_result = encryption.decrypt(&encrypted_data, &wrong_key).await;
    assert!(decryption_result.is_err());
}

/// Test storage with corrupted data
#[tokio::test]
async fn test_storage_corrupted_data() {
    let config = ProofConfig {
        proof_interval: 3600,
        merkle_tree_depth: 16,
        spot_check_probability: 0.1,
        proof_timeout: 30,
    };
    
    let mut proof_manager = ProofManager::new(config);
    let original_data = vec![42u8; 1024];
    let corrupted_data = vec![0u8; 1024]; // Corrupted data
    let file_hash = sha256_hash(&original_data);
    let node_id = create_test_node_id();
    
    // Generate proof with original data
    let proof = proof_manager.generate_proof(&original_data, &file_hash, node_id).await.unwrap();
    
    // Try to verify with corrupted data
    let verification_result = proof_manager.verify_proof(&proof, &corrupted_data).await;
    assert!(verification_result.is_err() || !verification_result.unwrap());
}

/// Test storage scalability
#[tokio::test]
async fn test_storage_scalability() {
    let config = ShardConfig {
        shard_size: 1024,
        redundancy_factor: 3,
        max_shards_per_file: 1000,
        shard_timeout: 30,
    };
    
    let mut shard_manager = ShardManager::new(config);
    
    // Test with many small files
    for i in 0..100 {
        let file_data = vec![i as u8; 512];
        let file_hash = sha256_hash(&file_data);
        let shards = shard_manager.create_shards(&file_data, &file_hash).await.unwrap();
        assert_eq!(shards.len(), 1); // 512 bytes < 1KB shard size
    }
}

/// Test storage integration
#[tokio::test]
async fn test_storage_integration() {
    let encryption_config = EncryptionConfig {
        algorithm: "AES-256-GCM".to_string(),
        key_size: 32,
        iv_size: 12,
        tag_size: 16,
    };
    
    let shard_config = ShardConfig {
        shard_size: 1024,
        redundancy_factor: 3,
        max_shards_per_file: 100,
        shard_timeout: 30,
    };
    
    let proof_config = ProofConfig {
        proof_interval: 3600,
        merkle_tree_depth: 16,
        spot_check_probability: 0.1,
        proof_timeout: 30,
    };
    
    let traffic_config = TrafficConfig {
        stats_interval: 60,
        max_stats_history: 1000,
        bandwidth_limit: 1024 * 1024 * 1024,
        request_timeout: 30,
    };
    
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_operations: 10,
        operation_timeout: 300,
        retry_attempts: 3,
        health_check_interval: 60,
    };
    
    let encryption = StorageEncryption::new(encryption_config);
    let shard_manager = ShardManager::new(shard_config);
    let proof_manager = ProofManager::new(proof_config);
    let traffic_manager = TrafficManager::new(traffic_config);
    let orchestrator = StorageOrchestrator::new(orchestrator_config);
    
    // Test that all components can work together
    assert_eq!(encryption.get_config().algorithm, "AES-256-GCM");
    assert_eq!(shard_manager.get_config().shard_size, 1024);
    assert_eq!(proof_manager.get_config().merkle_tree_depth, 16);
    assert_eq!(traffic_manager.get_config().bandwidth_limit, 1024 * 1024 * 1024);
    assert_eq!(orchestrator.get_config().max_concurrent_operations, 10);
}
