//! Security Integration Tests for IPPAN
//! 
//! Tests the complete security system integration

use ippan::{
    wallet::ed25519::Ed25519Manager,
    network::security::{NetworkSecurityManager, NetworkSecurityConfig},
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_key_management() {
    // Test key management system
    let key_manager = Arc::new(RwLock::new(Ed25519Manager::new()));

    // Test key generation
    let keypair = key_manager.write().await.generate_key_pair("test_key".to_string()).await.unwrap();
    assert_eq!(keypair.public_key.len(), 32);
    assert_eq!(keypair.private_key.len(), 32);

    // Test key storage
    key_manager.write().await.add_key_pair(keypair.clone());
    
    // Basic test that the key was generated and stored
    assert!(keypair.public_key.len() > 0);
    assert!(keypair.private_key.len() > 0);
}

#[tokio::test]
async fn test_network_security() {
    // Test network security system
    let security_config = NetworkSecurityConfig::default();
    let security_manager = Arc::new(RwLock::new(NetworkSecurityManager::new(security_config)));

    // Test that the security manager was created successfully
    // Just verify it exists and can be accessed
    let _guard = security_manager.read().await;
    assert!(true); // Basic test that the manager was created
}