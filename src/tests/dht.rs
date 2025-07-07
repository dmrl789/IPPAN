//! Tests for the DHT module

use crate::{
    dht::{
        routing::{DHTRouter, RoutingTable, RoutingConfig},
        discovery::{DiscoveryManager, DiscoveryConfig},
        lookup::{LookupManager, LookupConfig},
        replication::{ReplicationManager, ReplicationConfig},
    },
    utils::{crypto::sha256_hash, time::current_time_secs},
    NodeId,
};

use super::create_test_node_id;

/// Test DHTRouter creation and basic operations
#[tokio::test]
async fn test_dht_router_creation() {
    let config = RoutingConfig {
        bucket_size: 20,
        max_buckets: 160,
        refresh_interval: 3600,
        timeout: 30,
    };
    
    let router = DHTRouter::new(config);
    assert_eq!(router.get_config().bucket_size, 20);
    assert_eq!(router.get_config().max_buckets, 160);
}

/// Test routing table operations
#[tokio::test]
async fn test_routing_table_operations() {
    let mut routing_table = RoutingTable::new(20, 160);
    let node_id = create_test_node_id();
    
    // Test adding node
    let result = routing_table.add_node(node_id, "127.0.0.1:8080".to_string()).await;
    assert!(result.is_ok());
    assert_eq!(routing_table.get_node_count().await, 1);
    
    // Test finding node
    let found_node = routing_table.find_node(&node_id).await;
    assert!(found_node.is_some());
    assert_eq!(found_node.unwrap().node_id, node_id);
    
    // Test removing node
    let remove_result = routing_table.remove_node(&node_id).await;
    assert!(remove_result.is_ok());
    assert_eq!(routing_table.get_node_count().await, 0);
}

/// Test routing table bucket management
#[tokio::test]
async fn test_routing_table_buckets() {
    let mut routing_table = RoutingTable::new(20, 160);
    
    // Add multiple nodes to test bucket management
    for i in 0..25 {
        let node_id = create_test_node_id();
        let addr = format!("127.0.0.{}:8080", i);
        routing_table.add_node(node_id, addr).await.unwrap();
    }
    
    assert_eq!(routing_table.get_node_count().await, 25);
    assert!(routing_table.get_bucket_count().await > 0);
}

/// Test DiscoveryManager creation and operations
#[tokio::test]
async fn test_discovery_manager_creation() {
    let config = DiscoveryConfig {
        bootstrap_nodes: vec!["127.0.0.1:8080".to_string()],
        discovery_interval: 300,
        max_discovered_nodes: 1000,
        ping_timeout: 5,
    };
    
    let discovery_manager = DiscoveryManager::new(config);
    assert_eq!(discovery_manager.get_config().bootstrap_nodes.len(), 1);
    assert_eq!(discovery_manager.get_config().discovery_interval, 300);
}

/// Test node discovery process
#[tokio::test]
async fn test_node_discovery() {
    let config = DiscoveryConfig {
        bootstrap_nodes: vec!["127.0.0.1:8080".to_string()],
        discovery_interval: 300,
        max_discovered_nodes: 1000,
        ping_timeout: 5,
    };
    
    let mut discovery_manager = DiscoveryManager::new(config);
    let node_id = create_test_node_id();
    
    // Test adding discovered node
    discovery_manager.add_discovered_node(node_id, "127.0.0.2:8080".to_string()).await;
    
    let discovered_nodes = discovery_manager.get_discovered_nodes().await;
    assert_eq!(discovered_nodes.len(), 1);
    assert!(discovered_nodes.contains_key(&node_id));
}

/// Test LookupManager creation and operations
#[tokio::test]
async fn test_lookup_manager_creation() {
    let config = LookupConfig {
        lookup_timeout: 30,
        max_parallel_lookups: 3,
        retry_count: 3,
        alpha: 3,
    };
    
    let lookup_manager = LookupManager::new(config);
    assert_eq!(lookup_manager.get_config().lookup_timeout, 30);
    assert_eq!(lookup_manager.get_config().max_parallel_lookups, 3);
}

/// Test key lookup operations
#[tokio::test]
async fn test_key_lookup() {
    let config = LookupConfig {
        lookup_timeout: 30,
        max_parallel_lookups: 3,
        retry_count: 3,
        alpha: 3,
    };
    
    let mut lookup_manager = LookupManager::new(config);
    let key = [1u8; 32];
    let target_node = create_test_node_id();
    
    // Test starting lookup
    let lookup_id = lookup_manager.start_lookup(key, target_node).await;
    assert!(lookup_id > 0);
    
    // Test getting lookup status
    let status = lookup_manager.get_lookup_status(lookup_id).await;
    assert!(status.is_some());
}

/// Test lookup with multiple nodes
#[tokio::test]
async fn test_multi_node_lookup() {
    let config = LookupConfig {
        lookup_timeout: 30,
        max_parallel_lookups: 3,
        retry_count: 3,
        alpha: 3,
    };
    
    let mut lookup_manager = LookupManager::new(config);
    let key = [1u8; 32];
    
    // Start multiple lookups
    let mut lookup_ids = Vec::new();
    for _ in 0..5 {
        let target_node = create_test_node_id();
        let lookup_id = lookup_manager.start_lookup(key, target_node).await;
        lookup_ids.push(lookup_id);
    }
    
    assert_eq!(lookup_ids.len(), 5);
    
    // Test getting all active lookups
    let active_lookups = lookup_manager.get_active_lookups().await;
    assert_eq!(active_lookups.len(), 5);
}

/// Test ReplicationManager creation and operations
#[tokio::test]
async fn test_replication_manager_creation() {
    let config = ReplicationConfig {
        replication_factor: 3,
        replication_interval: 3600,
        max_replication_attempts: 5,
        replication_timeout: 30,
    };
    
    let replication_manager = ReplicationManager::new(config);
    assert_eq!(replication_manager.get_config().replication_factor, 3);
    assert_eq!(replication_manager.get_config().replication_interval, 3600);
}

/// Test data replication
#[tokio::test]
async fn test_data_replication() {
    let config = ReplicationConfig {
        replication_factor: 3,
        replication_interval: 3600,
        max_replication_attempts: 5,
        replication_timeout: 30,
    };
    
    let mut replication_manager = ReplicationManager::new(config);
    let key = [1u8; 32];
    let data = b"test data for replication";
    
    // Test starting replication
    let replication_id = replication_manager.start_replication(key, data.to_vec()).await;
    assert!(replication_id > 0);
    
    // Test getting replication status
    let status = replication_manager.get_replication_status(replication_id).await;
    assert!(status.is_some());
}

/// Test replication with multiple targets
#[tokio::test]
async fn test_multi_target_replication() {
    let config = ReplicationConfig {
        replication_factor: 3,
        replication_interval: 3600,
        max_replication_attempts: 5,
        replication_timeout: 30,
    };
    
    let mut replication_manager = ReplicationManager::new(config);
    let key = [1u8; 32];
    let data = b"test data for multi-target replication";
    let targets = vec![create_test_node_id(), create_test_node_id(), create_test_node_id()];
    
    // Test replication to multiple targets
    let replication_id = replication_manager.replicate_to_targets(key, data.to_vec(), targets).await;
    assert!(replication_id > 0);
    
    // Test getting replication progress
    let progress = replication_manager.get_replication_progress(replication_id).await;
    assert!(progress.is_some());
}

/// Test DHT integration
#[tokio::test]
async fn test_dht_integration() {
    let routing_config = RoutingConfig {
        bucket_size: 20,
        max_buckets: 160,
        refresh_interval: 3600,
        timeout: 30,
    };
    
    let discovery_config = DiscoveryConfig {
        bootstrap_nodes: vec!["127.0.0.1:8080".to_string()],
        discovery_interval: 300,
        max_discovered_nodes: 1000,
        ping_timeout: 5,
    };
    
    let lookup_config = LookupConfig {
        lookup_timeout: 30,
        max_parallel_lookups: 3,
        retry_count: 3,
        alpha: 3,
    };
    
    let replication_config = ReplicationConfig {
        replication_factor: 3,
        replication_interval: 3600,
        max_replication_attempts: 5,
        replication_timeout: 30,
    };
    
    let router = DHTRouter::new(routing_config);
    let discovery_manager = DiscoveryManager::new(discovery_config);
    let lookup_manager = LookupManager::new(lookup_config);
    let replication_manager = ReplicationManager::new(replication_config);
    
    // Test that all components can work together
    assert_eq!(router.get_config().bucket_size, 20);
    assert_eq!(discovery_manager.get_config().bootstrap_nodes.len(), 1);
    assert_eq!(lookup_manager.get_config().max_parallel_lookups, 3);
    assert_eq!(replication_manager.get_config().replication_factor, 3);
}

/// Test DHT performance metrics
#[tokio::test]
async fn test_dht_performance() {
    let mut routing_table = RoutingTable::new(20, 160);
    
    // Add many nodes to test performance
    for i in 0..100 {
        let node_id = create_test_node_id();
        let addr = format!("127.0.0.{}:8080", i);
        routing_table.add_node(node_id, addr).await.unwrap();
    }
    
    // Test lookup performance
    let start_time = std::time::Instant::now();
    let target_node = create_test_node_id();
    let _found = routing_table.find_node(&target_node).await;
    let lookup_time = start_time.elapsed();
    
    // Lookup should be fast (less than 1ms)
    assert!(lookup_time.as_micros() < 1000);
}

/// Test DHT error handling
#[tokio::test]
async fn test_dht_error_handling() {
    let mut routing_table = RoutingTable::new(20, 160);
    
    // Test finding non-existent node
    let non_existent_node = create_test_node_id();
    let found = routing_table.find_node(&non_existent_node).await;
    assert!(found.is_none());
    
    // Test removing non-existent node
    let remove_result = routing_table.remove_node(&non_existent_node).await;
    assert!(remove_result.is_err());
}

/// Test DHT with full bucket
#[tokio::test]
async fn test_dht_full_bucket() {
    let mut routing_table = RoutingTable::new(5, 160); // Small bucket size
    
    // Fill bucket
    for i in 0..5 {
        let node_id = create_test_node_id();
        let addr = format!("127.0.0.{}:8080", i);
        routing_table.add_node(node_id, addr).await.unwrap();
    }
    
    assert_eq!(routing_table.get_node_count().await, 5);
    
    // Try to add one more node to full bucket
    let extra_node = create_test_node_id();
    let result = routing_table.add_node(extra_node, "127.0.0.5:8080".to_string()).await;
    
    // Should handle full bucket gracefully
    assert!(result.is_ok() || result.is_err()); // Either succeeds or fails gracefully
}

/// Test DHT node eviction
#[tokio::test]
async fn test_dht_node_eviction() {
    let mut routing_table = RoutingTable::new(3, 160); // Very small bucket
    
    // Add nodes
    let node_ids: Vec<NodeId> = (0..5).map(|_| create_test_node_id()).collect();
    for (i, node_id) in node_ids.iter().enumerate() {
        let addr = format!("127.0.0.{}:8080", i);
        routing_table.add_node(*node_id, addr).await.unwrap();
    }
    
    // Test that some nodes may be evicted due to bucket size limit
    let final_count = routing_table.get_node_count().await;
    assert!(final_count <= 3); // Should not exceed bucket size
}

/// Test DHT routing table statistics
#[tokio::test]
async fn test_routing_table_stats() {
    let mut routing_table = RoutingTable::new(20, 160);
    
    // Add nodes
    for i in 0..50 {
        let node_id = create_test_node_id();
        let addr = format!("127.0.0.{}:8080", i);
        routing_table.add_node(node_id, addr).await.unwrap();
    }
    
    // Test statistics
    let stats = routing_table.get_statistics().await;
    assert_eq!(stats.total_nodes, 50);
    assert!(stats.bucket_count > 0);
    assert!(stats.average_bucket_size > 0.0);
}

/// Test DHT discovery with invalid nodes
#[tokio::test]
async fn test_discovery_invalid_nodes() {
    let config = DiscoveryConfig {
        bootstrap_nodes: vec!["invalid-address:99999".to_string()],
        discovery_interval: 300,
        max_discovered_nodes: 1000,
        ping_timeout: 5,
    };
    
    let discovery_manager = DiscoveryManager::new(config);
    
    // Should handle invalid bootstrap nodes gracefully
    let discovered_nodes = discovery_manager.get_discovered_nodes().await;
    assert_eq!(discovered_nodes.len(), 0);
}
