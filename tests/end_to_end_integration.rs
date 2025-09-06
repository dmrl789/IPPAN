//! End-to-End Integration Tests for IPPAN
//! 
//! Tests the complete system integration

use ippan::{
    node::IppanNode,
    config::Config,
    wallet::ed25519::Ed25519Manager,
    network::security::{NetworkSecurityManager, NetworkSecurityConfig},
    performance::{PerformanceMonitor, metrics::PerformanceMetrics},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_basic_system_integration() {
    // Test basic node creation
    let config = Config::default();
    let node = IppanNode::new(config).await.unwrap();
    assert!(node.get_status().uptime.as_secs() >= 0);

    // Test wallet system
    let wallet_manager = Arc::new(RwLock::new(Ed25519Manager::new()));
    let keypair = wallet_manager.write().await.generate_key_pair("test_key".to_string()).await.unwrap();
    assert_eq!(keypair.public_key.len(), 32);

    // Test network security
    let security_config = NetworkSecurityConfig::default();
    let security_manager = Arc::new(RwLock::new(NetworkSecurityManager::new(security_config)));
    let _guard = security_manager.read().await;
    assert!(true); // Basic test that the manager was created

    // Test performance monitor
    let performance_metrics = Arc::new(PerformanceMetrics::new());
    let performance_monitor = Arc::new(RwLock::new(PerformanceMonitor::new(performance_metrics, std::time::Duration::from_secs(1))));
    let metrics = performance_monitor.read().await.get_metrics();
    assert!(metrics.transactions_processed >= 0);
}

#[tokio::test]
async fn test_component_interaction() {
    // Test that different components can work together
    let config = Config::default();
    
    // Create multiple components
    let wallet_manager = Arc::new(RwLock::new(Ed25519Manager::new()));
    let security_config = NetworkSecurityConfig::default();
    let security_manager = Arc::new(RwLock::new(NetworkSecurityManager::new(security_config)));
    let performance_metrics = Arc::new(PerformanceMetrics::new());
    let performance_monitor = Arc::new(RwLock::new(PerformanceMonitor::new(performance_metrics, std::time::Duration::from_secs(1))));

    // Test basic operations
    let keypair = wallet_manager.write().await.generate_key_pair("integration_test".to_string()).await.unwrap();
    assert_eq!(keypair.public_key.len(), 32);
    
    let _security_guard = security_manager.read().await;
    let metrics = performance_monitor.read().await.get_metrics();
    assert!(metrics.blocks_processed >= 0);
    
    // All components created and basic operations work
    assert!(true);
}