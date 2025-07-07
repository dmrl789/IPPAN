use crate::{Result, NodeId};
use super::{
    StorageConfig, FileMetadata, StorageStats, StorageResult, StorageError,
    StorageEncryption, ShardManager, ShardConfig, ProofOfStorage,
};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use tokio::sync::RwLock;

/// Storage orchestrator that manages all storage operations
pub struct StorageOrchestrator {
    /// Storage configuration
    config: StorageConfig,
    /// Encryption manager
    encryption: StorageEncryption,
    /// Shard manager
    shard_manager: ShardManager,
    /// Storage directory
    storage_path: PathBuf,
    /// File metadata cache
    file_metadata: RwLock<HashMap<[u8; 32], FileMetadata>>,
    /// Storage statistics
    stats: RwLock<StorageStats>,
}

impl StorageOrchestrator {
    /// Create a new storage orchestrator
    pub async fn new(config: StorageConfig) -> Result<Self> {
        // Create storage directory if it doesn't exist
        let storage_path = PathBuf::from(&config.storage_path);
        fs::create_dir_all(&storage_path)
            .map_err(|e| StorageError::IoError(e))?;

        // Generate or load encryption key
        let master_key = Self::get_or_generate_master_key(&storage_path).await?;
        let encryption = StorageEncryption::new(&master_key)?;

        // Create shard manager
        let shard_config = ShardConfig::default();
        let shard_manager = ShardManager::new(shard_config);

        // Initialize storage statistics
        let stats = StorageStats::new(0, config.max_capacity, 0, 0);

        Ok(Self {
            config,
            encryption,
            shard_manager,
            storage_path,
            file_metadata: RwLock::new(HashMap::new()),
            stats: RwLock::new(stats),
        })
    }

    /// Store a file with encryption and sharding
    pub async fn store_file(&self, file_data: &[u8], mime_type: &str) -> Result<StorageResult> {
        // Calculate file hash
        let file_hash = self.calculate_file_hash(file_data);
        
        // Check if file already exists
        if self.file_exists(&file_hash).await? {
            return Ok(StorageResult::Stored {
                file_hash,
                shards: self.get_file_metadata(&file_hash).await?.shards,
            });
        }

        // Check storage capacity
        self.check_capacity(file_data.len() as u64).await?;

        // Encrypt file data
        let encrypted_data = if self.config.enable_encryption {
            self.encryption.encrypt_file(file_data, 1024 * 1024)? // 1MB chunks
        } else {
            // Store as single chunk if encryption is disabled
            vec![super::encryption::EncryptedData {
                data: file_data.to_vec(),
                nonce: [0; 12],
                tag: [0; 16],
            }]
        };

        // Combine encrypted chunks
        let mut combined_encrypted = Vec::new();
        for chunk in &encrypted_data {
            combined_encrypted.extend(&chunk.data);
            combined_encrypted.extend(&chunk.nonce);
            combined_encrypted.extend(&chunk.tag);
        }

        // Split into shards
        let shards = self.shard_manager.split_file(&combined_encrypted, &file_hash)?;

        // Store shards locally
        self.store_shards_locally(&shards).await?;

        // Create file metadata
        let metadata = FileMetadata {
            hash: file_hash,
            size: file_data.len() as u64,
            mime_type: mime_type.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            shards: shards.iter().map(|s| super::ShardInfo {
                id: s.id,
                hash: s.hash,
                nodes: vec![], // Will be populated by DHT
                size: s.size,
            }).collect(),
            merkle_root: self.generate_merkle_root(file_data)?,
        };

        // Store metadata
        self.store_metadata(&metadata).await?;

        // Update statistics
        self.update_stats(file_data.len() as u64, shards.len() as u64).await?;

        Ok(StorageResult::Stored {
            file_hash,
            shards: metadata.shards,
        })
    }

    /// Retrieve a file
    pub async fn retrieve_file(&self, file_hash: &[u8; 32]) -> Result<StorageResult> {
        // Get file metadata
        let metadata = self.get_file_metadata(file_hash).await?;

        // Retrieve shards
        let shards = self.retrieve_shards_locally(file_hash).await?;
        
        if shards.is_empty() {
            return Err(StorageError::FileNotFound(*file_hash).into());
        }

        // Reconstruct file
        let combined_encrypted = self.shard_manager.reconstruct_file(&shards)?;

        // Parse encrypted chunks
        let encrypted_chunks = self.parse_encrypted_chunks(&combined_encrypted)?;

        // Decrypt file
        let file_data = if self.config.enable_encryption {
            self.encryption.decrypt_file(&encrypted_chunks)?
        } else {
            // If encryption is disabled, data is stored directly
            combined_encrypted
        };

        Ok(StorageResult::Retrieved {
            data: file_data,
            metadata,
        })
    }

    /// Delete a file
    pub async fn delete_file(&self, file_hash: &[u8; 32]) -> Result<StorageResult> {
        // Get file metadata
        let metadata = self.get_file_metadata(file_hash).await?;

        // Delete shards locally
        self.delete_shards_locally(file_hash).await?;

        // Remove metadata
        self.remove_metadata(file_hash).await?;

        // Update statistics
        self.update_stats_deletion(metadata.size, metadata.shards.len() as u64).await?;

        Ok(StorageResult::Deleted { file_hash: *file_hash })
    }

    /// Verify storage proof
    pub async fn verify_storage_proof(&self, file_hash: &[u8; 32]) -> Result<StorageResult> {
        let metadata = self.get_file_metadata(file_hash).await?;
        
        // Create proof of storage
        let mut proof = ProofOfStorage::new(*file_hash, 1024);
        
        // Retrieve file data for proof generation
        let file_data = match self.retrieve_file(file_hash).await? {
            StorageResult::Retrieved { data, .. } => data,
            _ => return Err(StorageError::FileNotFound(*file_hash).into()),
        };

        // Build Merkle tree
        let merkle_root = proof.build_tree(&file_data)?;
        
        // Verify against stored merkle root
        let is_valid = merkle_root == metadata.merkle_root;

        Ok(StorageResult::ProofVerified {
            file_hash: *file_hash,
            valid: is_valid,
        })
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageStats {
        self.stats.read().await.clone()
    }

    /// Check if file exists
    async fn file_exists(&self, file_hash: &[u8; 32]) -> Result<bool> {
        Ok(self.file_metadata.read().await.contains_key(file_hash))
    }

    /// Get file metadata
    async fn get_file_metadata(&self, file_hash: &[u8; 32]) -> Result<FileMetadata> {
        self.file_metadata.read().await
            .get(file_hash)
            .cloned()
            .ok_or_else(|| StorageError::FileNotFound(*file_hash).into())
    }

    /// Store file metadata
    async fn store_metadata(&self, metadata: &FileMetadata) {
        self.file_metadata.write().await.insert(metadata.hash, metadata.clone());
    }

    /// Remove file metadata
    async fn remove_metadata(&self, file_hash: &[u8; 32]) {
        self.file_metadata.write().await.remove(file_hash);
    }

    /// Calculate file hash
    fn calculate_file_hash(&self, data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Generate Merkle root for file
    fn generate_merkle_root(&self, file_data: &[u8]) -> Result<[u8; 32]> {
        let mut proof = ProofOfStorage::new([0; 32], 1024);
        proof.build_tree(file_data)
    }

    /// Check storage capacity
    async fn check_capacity(&self, needed_bytes: u64) -> Result<()> {
        let stats = self.stats.read().await;
        if stats.used_bytes + needed_bytes > stats.capacity_bytes {
            return Err(StorageError::InsufficientSpace {
                needed: needed_bytes,
                available: stats.capacity_bytes - stats.used_bytes,
            }.into());
        }
        Ok(())
    }

    /// Update storage statistics
    async fn update_stats(&self, file_size: u64, shard_count: u64) -> Result<()> {
        let mut stats = self.stats.write().await;
        stats.used_bytes += file_size;
        stats.file_count += 1;
        stats.shard_count += shard_count;
        stats.utilization_percent = (stats.used_bytes as f64 / stats.capacity_bytes as f64) * 100.0;
        Ok(())
    }

    /// Update statistics after deletion
    async fn update_stats_deletion(&self, file_size: u64, shard_count: u64) -> Result<()> {
        let mut stats = self.stats.write().await;
        stats.used_bytes = stats.used_bytes.saturating_sub(file_size);
        stats.file_count = stats.file_count.saturating_sub(1);
        stats.shard_count = stats.shard_count.saturating_sub(shard_count);
        stats.utilization_percent = (stats.used_bytes as f64 / stats.capacity_bytes as f64) * 100.0;
        Ok(())
    }

    /// Store shards locally
    async fn store_shards_locally(&self, shards: &[super::StorageShard]) -> Result<()> {
        for shard in shards {
            let shard_path = self.storage_path.join(format!("shard_{}.dat", shard.id));
            fs::write(&shard_path, &shard.data)
                .map_err(|e| StorageError::IoError(e))?;
        }
        Ok(())
    }

    /// Retrieve shards locally
    async fn retrieve_shards_locally(&self, file_hash: &[u8; 32]) -> Result<Vec<super::StorageShard>> {
        let metadata = self.get_file_metadata(file_hash).await?;
        let mut shards = Vec::new();

        for shard_info in &metadata.shards {
            let shard_path = self.storage_path.join(format!("shard_{}.dat", shard_info.id));
            if shard_path.exists() {
                let data = fs::read(&shard_path)
                    .map_err(|e| StorageError::IoError(e))?;
                
                shards.push(super::StorageShard {
                    id: shard_info.id,
                    file_hash: *file_hash,
                    data,
                    hash: shard_info.hash,
                    size: shard_info.size,
                    total_shards: metadata.shards.len() as u32,
                    parity_data: None,
                });
            }
        }

        Ok(shards)
    }

    /// Delete shards locally
    async fn delete_shards_locally(&self, file_hash: &[u8; 32]) -> Result<()> {
        let metadata = self.get_file_metadata(file_hash).await?;
        
        for shard_info in &metadata.shards {
            let shard_path = self.storage_path.join(format!("shard_{}.dat", shard_info.id));
            if shard_path.exists() {
                fs::remove_file(&shard_path)
                    .map_err(|e| StorageError::IoError(e))?;
            }
        }
        Ok(())
    }

    /// Parse encrypted chunks from combined data
    fn parse_encrypted_chunks(&self, combined_data: &[u8]) -> Result<Vec<super::encryption::EncryptedData>> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < combined_data.len() {
            if offset + 28 > combined_data.len() { // 12 + 16 = 28 bytes for nonce + tag
                break;
            }

            // Extract nonce and tag
            let nonce = combined_data[offset..offset + 12].try_into().unwrap();
            let tag = combined_data[offset + 12..offset + 28].try_into().unwrap();
            offset += 28;

            // Find data length (this is a simplified approach)
            let data_end = combined_data.len() - 28;
            let data = combined_data[offset..data_end].to_vec();
            offset = data_end;

            chunks.push(super::encryption::EncryptedData {
                data,
                nonce,
                tag,
            });
        }

        Ok(chunks)
    }

    /// Get or generate master encryption key
    async fn get_or_generate_master_key(storage_path: &PathBuf) -> Result<[u8; 32]> {
        let key_path = storage_path.join("master.key");
        
        if key_path.exists() {
            let key_data = fs::read(&key_path)
                .map_err(|e| StorageError::IoError(e))?;
            if key_data.len() == 32 {
                return Ok(key_data.try_into().unwrap());
            }
        }

        // Generate new key
        let master_key = StorageEncryption::generate_master_key();
        fs::write(&key_path, &master_key)
            .map_err(|e| StorageError::IoError(e))?;
        
        Ok(master_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_storage_and_retrieval() {
        let config = StorageConfig::default();
        let orchestrator = StorageOrchestrator::new(config).await.unwrap();
        
        let test_data = b"Test file data for storage and retrieval";
        let mime_type = "text/plain";
        
        // Store file
        let store_result = orchestrator.store_file(test_data, mime_type).await.unwrap();
        match store_result {
            StorageResult::Stored { file_hash, .. } => {
                // Retrieve file
                let retrieve_result = orchestrator.retrieve_file(&file_hash).await.unwrap();
                match retrieve_result {
                    StorageResult::Retrieved { data, .. } => {
                        assert_eq!(test_data, data.as_slice());
                    }
                    _ => panic!("Expected Retrieved result"),
                }
            }
            _ => panic!("Expected Stored result"),
        }
    }

    #[tokio::test]
    async fn test_storage_statistics() {
        let config = StorageConfig::default();
        let orchestrator = StorageOrchestrator::new(config).await.unwrap();
        
        let initial_stats = orchestrator.get_stats().await;
        assert_eq!(initial_stats.file_count, 0);
        assert_eq!(initial_stats.used_bytes, 0);
        
        let test_data = b"Test data for statistics";
        orchestrator.store_file(test_data, "text/plain").await.unwrap();
        
        let updated_stats = orchestrator.get_stats().await;
        assert_eq!(updated_stats.file_count, 1);
        assert!(updated_stats.used_bytes > 0);
    }
}
