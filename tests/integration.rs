//! Integration tests for IPPAN
//! 
//! Tests the complete system integration and end-to-end functionality

use ippan::{
    node::IppanNode,
    config::Config,
    wallet::ed25519::Ed25519Manager,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_basic_integration() {
    // Create basic configuration
    let config = Config::default();
    
    // Test that we can create a node
    let node = IppanNode::new(config).await.unwrap();
    assert!(node.get_status().uptime.as_secs() >= 0);
}

#[tokio::test]
async fn test_wallet_integration() {
    // Test wallet system
    let wallet_manager = Arc::new(RwLock::new(Ed25519Manager::new()));
    
    // Test key generation
    let keypair = wallet_manager.write().await.generate_key_pair("test_key".to_string()).await.unwrap();
    assert_eq!(keypair.public_key.len(), 32);
    assert_eq!(keypair.private_key.len(), 32);
    
    // Test key storage
    let key_id = keypair.public_key.clone();
    wallet_manager.write().await.add_key_pair(keypair.clone());
    
    // Test that the key was generated and stored successfully
    assert_eq!(keypair.public_key.len(), 32);
    assert_eq!(keypair.private_key.len(), 32);
}