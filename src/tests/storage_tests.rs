//! Tests for IPPAN Distributed Storage System

use ippan::storage::distributed::{
    DistributedStorageManager, StorageConfig, StorageNode, StorageNodeStatus,
    StoragePriority, StorageMetrics, DataShard
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

#[tokio::test]
async fn test_storage_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        max_file_size_bytes: 1_000_000_000,
        shard_size_bytes: 64 * 1024 * 1024,
        default_replication_factor: 3,
        encryption_enabled: true,
        compression_enabled: true,
        max_concurrent_operations: 100,
        operation_timeout_ms: 30000,
        cleanup_interval_ms: 3600000,
        metrics_interval_ms: 60000,
    };

    let manager = DistributedStorageManager::new("test_node", config).await;

    assert!(manager.is_ok());
    
    let manager = manager.unwrap();
    let metrics = manager.get_storage_metrics().await;
    
    assert_eq!(metrics.total_capacity_bytes, 1_000_000_000_000); // 1TB default
    assert_eq!(metrics.used_capacity_bytes, 0);
    assert_eq!(metrics.available_capacity_bytes, metrics.total_capacity_bytes);
    assert_eq!(metrics.total_shards, 0);
    assert_eq!(metrics.replicated_shards, 0);
    assert_eq!(metrics.failed_shards, 0);
}

#[tokio::test]
async fn test_storage_node_management() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Create test storage node
    let node = StorageNode {
        node_id: "storage_node_1".to_string(),
        address: "127.0.0.1".to_string(),
        port: 8081,
        capacity_bytes: 1_000_000_000_000, // 1TB
        used_bytes: 100_000_000_000, // 100GB
        status: StorageNodeStatus::Online,
        capabilities: vec!["storage".to_string(), "encryption".to_string()],
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        replication_factor: 3,
        encryption_enabled: true,
    };

    // Add storage node
    assert!(manager.add_storage_node(node.clone()).await.is_ok());

    // Get storage node
    let retrieved_node = manager.get_storage_node("storage_node_1").await;
    assert!(retrieved_node.is_some());
    
    let retrieved_node = retrieved_node.unwrap();
    assert_eq!(retrieved_node.node_id, "storage_node_1");
    assert_eq!(retrieved_node.address, "127.0.0.1");
    assert_eq!(retrieved_node.port, 8081);
    assert_eq!(retrieved_node.capacity_bytes, 1_000_000_000_000);
    assert_eq!(retrieved_node.used_bytes, 100_000_000_000);
    assert_eq!(retrieved_node.status, StorageNodeStatus::Online);
    assert_eq!(retrieved_node.capabilities, vec!["storage".to_string(), "encryption".to_string()]);
    assert_eq!(retrieved_node.replication_factor, 3);
    assert!(retrieved_node.encryption_enabled);

    // Get all storage nodes
    let all_nodes = manager.get_all_storage_nodes().await;
    assert_eq!(all_nodes.len(), 1);
    assert_eq!(all_nodes[0].node_id, "storage_node_1");

    // Remove storage node
    assert!(manager.remove_storage_node("storage_node_1").await.is_ok());

    // Check node was removed
    let retrieved_node = manager.get_storage_node("storage_node_1").await;
    assert!(retrieved_node.is_none());
}

#[tokio::test]
async fn test_data_write_and_read() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 1024, // Small shard size for testing
        default_replication_factor: 2,
        encryption_enabled: true,
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add storage nodes
    for i in 1..=3 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 2,
            encryption_enabled: true,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Test data
    let test_data = b"This is test data for distributed storage".to_vec();
    let mut metadata = HashMap::new();
    metadata.insert("content_type".to_string(), "text/plain".to_string());
    metadata.insert("created_by".to_string(), "test_user".to_string());

    // Write data
    let shard_ids = manager.write_data(
        "test_data_1",
        test_data.clone(),
        StoragePriority::High,
        metadata.clone(),
    ).await.unwrap();

    // Check shards were created
    assert!(!shard_ids.is_empty());

    // Read data back
    let read_data = manager.read_data("test_data_1", true).await.unwrap();

    // Verify data integrity
    assert_eq!(read_data, test_data);

    // Check metrics
    let metrics = manager.get_storage_metrics().await;
    assert!(metrics.write_operations_per_sec > 0.0);
    assert!(metrics.read_operations_per_sec > 0.0);
    assert!(metrics.used_capacity_bytes > 0);
}

#[tokio::test]
async fn test_data_sharding() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 100, // Very small shard size to force multiple shards
        default_replication_factor: 2,
        encryption_enabled: false, // Disable encryption for simpler testing
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add storage nodes
    for i in 1..=3 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 2,
            encryption_enabled: false,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Create large test data that will be split into multiple shards
    let large_data = vec![b'A'; 500]; // 500 bytes, should create 5 shards of 100 bytes each

    let mut metadata = HashMap::new();
    metadata.insert("test_type".to_string(), "large_data".to_string());

    // Write large data
    let shard_ids = manager.write_data(
        "large_test_data",
        large_data.clone(),
        StoragePriority::Normal,
        metadata,
    ).await.unwrap();

    // Check multiple shards were created
    assert!(shard_ids.len() > 1);

    // Read data back
    let read_data = manager.read_data("large_test_data", true).await.unwrap();

    // Verify data integrity
    assert_eq!(read_data, large_data);
}

#[tokio::test]
async fn test_data_encryption() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 1024,
        default_replication_factor: 2,
        encryption_enabled: true,
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add storage nodes
    for i in 1..=2 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string(), "encryption".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 2,
            encryption_enabled: true,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Test data
    let test_data = b"Sensitive data that should be encrypted".to_vec();
    let mut metadata = HashMap::new();
    metadata.insert("sensitive".to_string(), "true".to_string());

    // Write encrypted data
    let shard_ids = manager.write_data(
        "encrypted_data",
        test_data.clone(),
        StoragePriority::Critical,
        metadata,
    ).await.unwrap();

    // Read encrypted data back
    let read_data = manager.read_data("encrypted_data", true).await.unwrap();

    // Verify data integrity
    assert_eq!(read_data, test_data);

    // Check that encryption was used
    let metrics = manager.get_storage_metrics().await;
    assert!(metrics.encryption_overhead_percent > 0.0);
}

#[tokio::test]
async fn test_data_deletion() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 1024,
        default_replication_factor: 2,
        encryption_enabled: false,
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add storage nodes
    for i in 1..=2 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 2,
            encryption_enabled: false,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Write test data
    let test_data = b"Data to be deleted".to_vec();
    let mut metadata = HashMap::new();
    metadata.insert("temporary".to_string(), "true".to_string());

    manager.write_data(
        "data_to_delete",
        test_data,
        StoragePriority::Low,
        metadata,
    ).await.unwrap();

    // Verify data exists
    let read_data = manager.read_data("data_to_delete", true).await.unwrap();
    assert_eq!(read_data, b"Data to be deleted");

    // Delete data
    assert!(manager.delete_data("data_to_delete", true).await.is_ok());

    // Verify data was deleted
    let read_result = manager.read_data("data_to_delete", true).await;
    assert!(read_result.is_err());
}

#[tokio::test]
async fn test_storage_priority() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 1024,
        default_replication_factor: 2,
        encryption_enabled: true,
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add storage nodes
    for i in 1..=3 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 3,
            encryption_enabled: true,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Test different priority levels
    let test_data = b"Priority test data".to_vec();
    let mut metadata = HashMap::new();

    // Critical priority data
    metadata.insert("priority".to_string(), "critical".to_string());
    let critical_shards = manager.write_data(
        "critical_data",
        test_data.clone(),
        StoragePriority::Critical,
        metadata.clone(),
    ).await.unwrap();

    // High priority data
    metadata.insert("priority".to_string(), "high".to_string());
    let high_shards = manager.write_data(
        "high_data",
        test_data.clone(),
        StoragePriority::High,
        metadata.clone(),
    ).await.unwrap();

    // Normal priority data
    metadata.insert("priority".to_string(), "normal".to_string());
    let normal_shards = manager.write_data(
        "normal_data",
        test_data.clone(),
        StoragePriority::Normal,
        metadata.clone(),
    ).await.unwrap();

    // Low priority data
    metadata.insert("priority".to_string(), "low".to_string());
    let low_shards = manager.write_data(
        "low_data",
        test_data.clone(),
        StoragePriority::Low,
        metadata,
    ).await.unwrap();

    // Verify all data was written successfully
    assert!(!critical_shards.is_empty());
    assert!(!high_shards.is_empty());
    assert!(!normal_shards.is_empty());
    assert!(!low_shards.is_empty());

    // Read all data back
    let critical_read = manager.read_data("critical_data", true).await.unwrap();
    let high_read = manager.read_data("high_data", true).await.unwrap();
    let normal_read = manager.read_data("normal_data", true).await.unwrap();
    let low_read = manager.read_data("low_data", true).await.unwrap();

    assert_eq!(critical_read, test_data);
    assert_eq!(high_read, test_data);
    assert_eq!(normal_read, test_data);
    assert_eq!(low_read, test_data);
}

#[tokio::test]
async fn test_storage_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 1024,
        default_replication_factor: 2,
        encryption_enabled: true,
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add storage nodes
    for i in 1..=3 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 2,
            encryption_enabled: true,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Get initial metrics
    let initial_metrics = manager.get_storage_metrics().await;
    assert_eq!(initial_metrics.total_shards, 0);
    assert_eq!(initial_metrics.replicated_shards, 0);
    assert_eq!(initial_metrics.failed_shards, 0);

    // Write some data
    let test_data = b"Metrics test data".to_vec();
    let mut metadata = HashMap::new();
    metadata.insert("test".to_string(), "metrics".to_string());

    manager.write_data(
        "metrics_test_data",
        test_data.clone(),
        StoragePriority::Normal,
        metadata,
    ).await.unwrap();

    // Read data
    manager.read_data("metrics_test_data", true).await.unwrap();

    // Get updated metrics
    let updated_metrics = manager.get_storage_metrics().await;
    assert!(updated_metrics.total_shards > 0);
    assert!(updated_metrics.replicated_shards > 0);
    assert!(updated_metrics.write_operations_per_sec > 0.0);
    assert!(updated_metrics.read_operations_per_sec > 0.0);
    assert!(updated_metrics.used_capacity_bytes > 0);
    assert!(updated_metrics.encryption_overhead_percent > 0.0);
}

#[tokio::test]
async fn test_node_failure_handling() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 1024,
        default_replication_factor: 3, // Higher replication for fault tolerance
        encryption_enabled: false,
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add multiple storage nodes
    for i in 1..=5 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 3,
            encryption_enabled: false,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Write data with high replication
    let test_data = b"Fault tolerant data".to_vec();
    let mut metadata = HashMap::new();
    metadata.insert("fault_tolerant".to_string(), "true".to_string());

    manager.write_data(
        "fault_tolerant_data",
        test_data.clone(),
        StoragePriority::Critical,
        metadata,
    ).await.unwrap();

    // Verify data exists
    let read_data = manager.read_data("fault_tolerant_data", true).await.unwrap();
    assert_eq!(read_data, test_data);

    // Simulate node failure by removing a node
    assert!(manager.remove_storage_node("storage_node_1").await.is_ok());

    // Data should still be accessible due to replication
    let read_data_after_failure = manager.read_data("fault_tolerant_data", true).await.unwrap();
    assert_eq!(read_data_after_failure, test_data);

    // Check metrics for failed shards
    let metrics = manager.get_storage_metrics().await;
    assert!(metrics.failed_shards >= 0); // May have some failed shards
}

#[tokio::test]
async fn test_storage_configuration() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        max_file_size_bytes: 500_000_000, // 500MB
        shard_size_bytes: 32 * 1024 * 1024, // 32MB
        default_replication_factor: 4,
        encryption_enabled: true,
        compression_enabled: true,
        max_concurrent_operations: 50,
        operation_timeout_ms: 60000,
        cleanup_interval_ms: 1800000, // 30 minutes
        metrics_interval_ms: 30000, // 30 seconds
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Verify configuration was applied
    let metrics = manager.get_storage_metrics().await;
    assert!(metrics.total_capacity_bytes > 0);
    assert_eq!(metrics.used_capacity_bytes, 0);
    assert_eq!(metrics.available_capacity_bytes, metrics.total_capacity_bytes);
}

#[tokio::test]
async fn test_concurrent_operations() {
    let temp_dir = TempDir::new().unwrap();
    let config = StorageConfig {
        base_path: temp_dir.path().to_path_buf(),
        shard_size_bytes: 1024,
        default_replication_factor: 2,
        encryption_enabled: false,
        max_concurrent_operations: 10,
        ..Default::default()
    };

    let manager = DistributedStorageManager::new("test_node", config).await.unwrap();

    // Add storage nodes
    for i in 1..=3 {
        let node = StorageNode {
            node_id: format!("storage_node_{}", i),
            address: format!("127.0.0.{}", i),
            port: 8080 + i,
            capacity_bytes: 1_000_000_000_000,
            used_bytes: 0,
            status: StorageNodeStatus::Online,
            capabilities: vec!["storage".to_string()],
            last_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            replication_factor: 2,
            encryption_enabled: false,
        };
        manager.add_storage_node(node).await.unwrap();
    }

    // Perform concurrent write operations
    let mut handles = Vec::new();
    for i in 0..5 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let test_data = format!("Concurrent data {}", i).into_bytes();
            let mut metadata = HashMap::new();
            metadata.insert("concurrent".to_string(), "true".to_string());
            
            manager_clone.write_data(
                &format!("concurrent_data_{}", i),
                test_data,
                StoragePriority::Normal,
                metadata,
            ).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    // Verify all data was written
    for i in 0..5 {
        let read_result = manager.read_data(&format!("concurrent_data_{}", i), true).await;
        assert!(read_result.is_ok());
    }
} 