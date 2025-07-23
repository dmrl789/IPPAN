//! Storage subsystem for IPPAN
//!
//! Handles encrypted, sharded storage, proofs, and orchestration.

use crate::config::StorageConfig;
use crate::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Serialize;

pub mod distributed;
pub mod encryption;
pub mod shards;
pub mod proofs;
pub mod orchestrator;

use distributed::DistributedStorage;
use encryption::{EncryptionManager, EncryptedData};
use shards::ShardManager;
use proofs::ProofManager;
use orchestrator::StorageOrchestrator;

/// Storage manager
pub struct StorageManager {
    pub config: StorageConfig,
    pub distributed_storage: Arc<RwLock<DistributedStorage>>,
    pub encryption_manager: Arc<RwLock<EncryptionManager>>,
    pub shard_manager: Arc<RwLock<ShardManager>>,
    pub proof_manager: Arc<RwLock<ProofManager>>,
    pub orchestrator: Arc<RwLock<StorageOrchestrator>>,
    running: bool,
}

impl StorageManager {
    /// Create a new storage manager
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let distributed_storage = Arc::new(RwLock::new(
            DistributedStorage::new(
                config.replication_factor.try_into().unwrap(),
                config.shard_size.try_into().unwrap()
            ).await?
        ));
        
        let encryption_manager = Arc::new(RwLock::new(
            EncryptionManager::new(30)? // Default key rotation interval
        ));
        
        let shard_manager = Arc::new(RwLock::new(
            ShardManager::new(
                shards::PlacementStrategy::HashBased,
                config.replication_factor.try_into().unwrap(),
                config.shard_size.try_into().unwrap(),
            )
        ));
        
        let proof_manager = Arc::new(RwLock::new(
            ProofManager::new(300, 0.8) // Default challenge interval and threshold
        ));
        
        let orchestrator = Arc::new(RwLock::new(
            StorageOrchestrator::new(4).await? // Default worker count
        ));
        
        Ok(Self {
            config,
            distributed_storage,
            encryption_manager,
            shard_manager,
            proof_manager,
            orchestrator,
            running: false,
        })
    }

    /// Start the storage subsystem
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting storage subsystem");
        self.running = true;
        
        // Start all components
        {
            let mut distributed = self.distributed_storage.write().await;
            distributed.start().await?;
        }
        
        {
            let mut encryption = self.encryption_manager.write().await;
            encryption.start().await?;
        }
        
        {
            let mut shards = self.shard_manager.write().await;
            shards.start().await?;
        }
        
        {
            let mut proofs = self.proof_manager.write().await;
            proofs.start().await?;
        }
        
        {
            let mut orchestrator = self.orchestrator.write().await;
            orchestrator.start().await?;
        }
        
        log::info!("Storage subsystem started successfully");
        Ok(())
    }

    /// Stop the storage subsystem
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping storage subsystem");
        self.running = false;
        
        // Stop all components
        {
            let mut distributed = self.distributed_storage.write().await;
            distributed.stop().await?;
        }
        
        {
            let mut encryption = self.encryption_manager.write().await;
            encryption.stop().await?;
        }
        
        {
            let mut shards = self.shard_manager.write().await;
            shards.stop().await?;
        }
        
        {
            let mut proofs = self.proof_manager.write().await;
            proofs.stop().await?;
        }
        
        {
            let mut orchestrator = self.orchestrator.write().await;
            orchestrator.stop().await?;
        }
        
        log::info!("Storage subsystem stopped successfully");
        Ok(())
    }

    /// Store a file
    pub async fn store_file(
        &self,
        file_id: &str,
        name: &str,
        data: &[u8],
        mime_type: &str,
        replication_factor: u32,
    ) -> Result<String> {
        let data_to_store = if self.config.enable_encryption {
            // Generate encryption key if needed
            let encryption_manager = self.encryption_manager.read().await;
            let key_id = format!("key_{}", file_id);
            encryption_manager.generate_key(&key_id, encryption::EncryptionAlgorithm::Aes256Gcm).await?;
            
            // Encrypt data
            let encrypted_data = encryption_manager.encrypt_data(data, &key_id).await?;
            encrypted_data.data
        } else {
            // Store data without encryption
            data.to_vec()
        };
        
        // Create shards
        let shard_manager = self.shard_manager.read().await;
        let shards = shard_manager.create_shards(file_id, data.len() as u64, &data_to_store).await?;
        
        // Store in distributed storage
        let distributed = self.distributed_storage.read().await;
        let store_request = distributed::StoreFileRequest {
            file_id: file_id.to_string(),
            name: name.to_string(),
            data: data_to_store,
            mime_type: mime_type.to_string(),
            replication_factor,
            encryption_key_id: if self.config.enable_encryption { Some(format!("key_{}", file_id)) } else { None },
        };
        
        let stored_file_id = distributed.store_file(store_request).await?;
        
        log::info!("Stored file: {} with {} shards", stored_file_id, shards.len());
        Ok(stored_file_id)
    }

    /// Retrieve a file
    pub async fn retrieve_file(&self, file_id: &str) -> Result<Vec<u8>> {
        // Retrieve from distributed storage
        let distributed = self.distributed_storage.read().await;
        let retrieve_request = distributed::RetrieveFileRequest {
            file_id: file_id.to_string(),
            shard_index: None,
        };
        
        let encrypted_data = distributed.retrieve_file(retrieve_request).await?;
        
        if self.config.enable_encryption {
            // TODO: Get encryption key ID from metadata
            let key_id = format!("key_{}", file_id);
            
            // Decrypt data
            let encryption_manager = self.encryption_manager.read().await;
            let decrypted_data = encryption_manager.decrypt_data(&EncryptedData {
                key_id: key_id.clone(),
                nonce: Vec::new(), // TODO: Get from metadata
                data: encrypted_data,
                tag: Vec::new(), // TODO: Get from metadata
                encrypted_at: chrono::Utc::now(),
            }).await?;
            
            log::info!("Retrieved file: {} ({} bytes)", file_id, decrypted_data.len());
            Ok(decrypted_data)
        } else {
            // Return data without decryption
            log::info!("Retrieved file: {} ({} bytes)", file_id, encrypted_data.len());
            Ok(encrypted_data)
        }
    }

    /// Delete a file
    pub async fn delete_file(&self, file_id: &str) -> Result<()> {
        // Delete from distributed storage
        let distributed = self.distributed_storage.read().await;
        let delete_request = distributed::DeleteFileRequest {
            file_id: file_id.to_string(),
        };
        
        distributed.delete_file(delete_request).await?;
        
        // TODO: Revoke encryption key
        let encryption_manager = self.encryption_manager.read().await;
        let key_id = format!("key_{}", file_id);
        encryption_manager.revoke_key(&key_id).await?;
        
        log::info!("Deleted file: {}", file_id);
        Ok(())
    }

    /// Get storage statistics
    pub async fn get_storage_stats(&self) -> StorageStats {
        let distributed_stats = {
            let distributed = self.distributed_storage.read().await;
            distributed.get_storage_stats().await
        };
        
        let encryption_stats = {
            let encryption = self.encryption_manager.read().await;
            encryption.get_encryption_stats().await
        };
        
        let shard_stats = {
            let shards = self.shard_manager.read().await;
            shards.get_shard_stats().await
        };
        
        let proof_stats = {
            let proofs = self.proof_manager.read().await;
            proofs.get_proof_stats().await
        };
        
        let orchestrator_stats = {
            let orchestrator = self.orchestrator.read().await;
            orchestrator.get_orchestrator_stats().await
        };
        
        StorageStats {
            distributed: distributed_stats,
            encryption: encryption_stats,
            shards: shard_stats,
            proofs: proof_stats,
            orchestrator: orchestrator_stats,
            running: self.running,
        }
    }
}

/// Storage statistics
#[derive(Debug, Serialize)]
pub struct StorageStats {
    pub distributed: distributed::StorageStats,
    pub encryption: encryption::EncryptionStats,
    pub shards: shards::ShardStats,
    pub proofs: proofs::ProofStats,
    pub orchestrator: orchestrator::OrchestratorStats,
    pub running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_manager_creation() {
        let config = StorageConfig {
            db_path: "test_db".to_string().into(),
            max_storage_size: 1024 * 1024 * 1024,
            shard_size: 1024 * 1024,
            replication_factor: 3,
            enable_encryption: true,
            proof_interval: 300,
        };
        
        let manager = StorageManager::new(config).await.unwrap();
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_storage_manager_start_stop() {
        let config = StorageConfig {
            db_path: "test_db".to_string().into(),
            max_storage_size: 1024 * 1024 * 1024,
            shard_size: 1024 * 1024,
            replication_factor: 3,
            enable_encryption: true,
            proof_interval: 300,
        };
        
        let mut manager = StorageManager::new(config).await.unwrap();
        
        manager.start().await.unwrap();
        assert!(manager.running);
        
        manager.stop().await.unwrap();
        assert!(!manager.running);
    }

    #[tokio::test]
    async fn test_file_storage_retrieval() {
        let config = StorageConfig {
            db_path: "test_db".to_string().into(),
            max_storage_size: 1024 * 1024 * 1024,
            shard_size: 1024 * 1024,
            replication_factor: 3,
            enable_encryption: false, // Disable encryption for this test
            proof_interval: 300,
        };
        
        let mut manager = StorageManager::new(config).await.unwrap();
        manager.start().await.unwrap();
        
        // Register some storage nodes in distributed storage
        {
            let distributed = manager.distributed_storage.read().await;
            let node1 = distributed::StorageNode {
                node_id: "node1".to_string(),
                address: "127.0.0.1:8080".to_string(),
                capacity: 1024 * 1024 * 1024,
                used_storage: 0,
                status: distributed::NodeStatus::Online,
                last_heartbeat: chrono::Utc::now(),
                score: 1.0,
            };
            
            let node2 = distributed::StorageNode {
                node_id: "node2".to_string(),
                address: "127.0.0.1:8081".to_string(),
                capacity: 1024 * 1024 * 1024,
                used_storage: 0,
                status: distributed::NodeStatus::Online,
                last_heartbeat: chrono::Utc::now(),
                score: 1.0,
            };
            
            let node3 = distributed::StorageNode {
                node_id: "node3".to_string(),
                address: "127.0.0.1:8082".to_string(),
                capacity: 1024 * 1024 * 1024,
                used_storage: 0,
                status: distributed::NodeStatus::Online,
                last_heartbeat: chrono::Utc::now(),
                score: 1.0,
            };
            
            distributed.register_node(node1).await.unwrap();
            distributed.register_node(node2).await.unwrap();
            distributed.register_node(node3).await.unwrap();
        }
        
        // Register some storage nodes in shard manager
        {
            let shards = manager.shard_manager.read().await;
            let node1 = shards::StorageNodeInfo {
                node_id: "node1".to_string(),
                address: "127.0.0.1:8080".to_string(),
                available_capacity: 1024 * 1024 * 1024,
                used_capacity: 0,
                status: shards::NodeStatus::Online,
                location: Some("US".to_string()),
                load_score: 0.1,
                last_heartbeat: chrono::Utc::now(),
            };
            
            let node2 = shards::StorageNodeInfo {
                node_id: "node2".to_string(),
                address: "127.0.0.1:8081".to_string(),
                available_capacity: 1024 * 1024 * 1024,
                used_capacity: 0,
                status: shards::NodeStatus::Online,
                location: Some("EU".to_string()),
                load_score: 0.2,
                last_heartbeat: chrono::Utc::now(),
            };
            
            let node3 = shards::StorageNodeInfo {
                node_id: "node3".to_string(),
                address: "127.0.0.1:8082".to_string(),
                available_capacity: 1024 * 1024 * 1024,
                used_capacity: 0,
                status: shards::NodeStatus::Online,
                location: Some("ASIA".to_string()),
                load_score: 0.3,
                last_heartbeat: chrono::Utc::now(),
            };
            
            shards.register_storage_node(node1).await.unwrap();
            shards.register_storage_node(node2).await.unwrap();
            shards.register_storage_node(node3).await.unwrap();
        }
        
        // Debug: Check stats
        let stats = manager.get_storage_stats().await;
        println!("Debug: Distributed nodes: {}, Shard nodes: {}", stats.distributed.total_nodes, stats.shards.total_nodes);
        
        // Store file
        let data = b"Hello, World! This is a test file.";
        let file_id = manager.store_file(
            "test_file",
            "test.txt",
            data,
            "text/plain",
            3,
        ).await.unwrap();
        
        assert_eq!(file_id, "test_file");
        
        // Retrieve file
        let retrieved_data = manager.retrieve_file("test_file").await.unwrap();
        assert_eq!(retrieved_data, data);
    }
}
