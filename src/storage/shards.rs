use crate::Result;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// A storage shard containing a portion of a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageShard {
    /// Shard ID
    pub id: u32,
    /// File hash this shard belongs to
    pub file_hash: [u8; 32],
    /// Shard data
    pub data: Vec<u8>,
    /// Shard hash
    pub hash: [u8; 32],
    /// Shard size in bytes
    pub size: u64,
    /// Total number of shards for this file
    pub total_shards: u32,
    /// Reed-Solomon parity data (if using erasure coding)
    pub parity_data: Option<Vec<u8>>,
}

/// Shard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// Number of data shards
    pub data_shards: usize,
    /// Number of parity shards (for erasure coding)
    pub parity_shards: usize,
    /// Maximum shard size in bytes
    pub max_shard_size: usize,
    /// Whether to use erasure coding
    pub use_erasure_coding: bool,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            data_shards: 16,
            parity_shards: 4,
            max_shard_size: 1024 * 1024, // 1MB
            use_erasure_coding: true,
        }
    }
}

/// Shard manager for splitting and reconstructing files
pub struct ShardManager {
    config: ShardConfig,
}

impl ShardManager {
    /// Create a new shard manager
    pub fn new(config: ShardConfig) -> Self {
        Self { config }
    }

    /// Split a file into shards
    pub fn split_file(&self, file_data: &[u8], file_hash: &[u8; 32]) -> Result<Vec<StorageShard>> {
        let total_shards = self.config.data_shards + self.config.parity_shards;
        let mut shards = Vec::new();

        // Calculate shard size
        let shard_size = (file_data.len() + self.config.data_shards - 1) / self.config.data_shards;
        let shard_size = shard_size.min(self.config.max_shard_size);

        // Create data shards
        for i in 0..self.config.data_shards {
            let start = i * shard_size;
            let end = (start + shard_size).min(file_data.len());
            
            if start >= file_data.len() {
                break;
            }

            let shard_data = file_data[start..end].to_vec();
            let shard_hash = self.calculate_shard_hash(&shard_data, file_hash, i);
            
            let shard = StorageShard {
                id: i as u32,
                file_hash: *file_hash,
                data: shard_data,
                hash: shard_hash,
                size: (end - start) as u64,
                total_shards: total_shards as u32,
                parity_data: None,
            };
            
            shards.push(shard);
        }

        // Add parity shards if erasure coding is enabled
        if self.config.use_erasure_coding && self.config.parity_shards > 0 {
            let parity_shards = self.generate_parity_shards(&shards, file_hash)?;
            shards.extend(parity_shards);
        }

        Ok(shards)
    }

    /// Reconstruct a file from shards
    pub fn reconstruct_file(&self, shards: &[StorageShard]) -> Result<Vec<u8>> {
        if shards.is_empty() {
            return Err(crate::IppanError::Storage("No shards provided".to_string()));
        }

        let file_hash = shards[0].file_hash;
        let total_shards = shards[0].total_shards;

        // Verify all shards belong to the same file
        for shard in shards {
            if shard.file_hash != file_hash {
                return Err(crate::IppanError::Storage("Shards from different files".to_string()));
            }
        }

        // Sort shards by ID
        let mut sorted_shards = shards.to_vec();
        sorted_shards.sort_by_key(|s| s.id);

        // If we have enough data shards, reconstruct directly
        let data_shards: Vec<_> = sorted_shards
            .iter()
            .filter(|s| s.id < self.config.data_shards as u32)
            .collect();

        if data_shards.len() == self.config.data_shards {
            return self.reconstruct_from_data_shards(&data_shards);
        }

        // If erasure coding is enabled, try to reconstruct with parity
        if self.config.use_erasure_coding {
            return self.reconstruct_with_parity(&sorted_shards);
        }

        Err(crate::IppanError::Storage("Insufficient shards for reconstruction".to_string()))
    }

    /// Calculate shard hash
    fn calculate_shard_hash(&self, shard_data: &[u8], file_hash: &[u8; 32], shard_id: usize) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(file_hash);
        hasher.update(&shard_id.to_le_bytes());
        hasher.update(shard_data);
        hasher.finalize().into()
    }

    /// Generate parity shards using Reed-Solomon erasure coding
    fn generate_parity_shards(&self, data_shards: &[StorageShard], file_hash: &[u8; 32]) -> Result<Vec<StorageShard>> {
        // TODO: Implement Reed-Solomon erasure coding
        // For now, create simple parity shards by XORing data shards
        
        let mut parity_shards = Vec::new();
        
        for i in 0..self.config.parity_shards {
            let parity_id = (data_shards.len() + i) as u32;
            let mut parity_data = Vec::new();
            
            // Find the maximum shard size
            let max_size = data_shards.iter().map(|s| s.data.len()).max().unwrap_or(0);
            
            for byte_pos in 0..max_size {
                let mut parity_byte = 0u8;
                for shard in data_shards {
                    if byte_pos < shard.data.len() {
                        parity_byte ^= shard.data[byte_pos];
                    }
                }
                parity_data.push(parity_byte);
            }
            
            let parity_hash = self.calculate_shard_hash(&parity_data, file_hash, parity_id as usize);
            
            let parity_shard = StorageShard {
                id: parity_id,
                file_hash: *file_hash,
                data: parity_data,
                hash: parity_hash,
                size: parity_data.len() as u64,
                total_shards: (data_shards.len() + self.config.parity_shards) as u32,
                parity_data: None,
            };
            
            parity_shards.push(parity_shard);
        }
        
        Ok(parity_shards)
    }

    /// Reconstruct file from data shards only
    fn reconstruct_from_data_shards(&self, data_shards: &[&StorageShard]) -> Result<Vec<u8>> {
        let mut file_data = Vec::new();
        
        for shard in data_shards {
            file_data.extend(&shard.data);
        }
        
        Ok(file_data)
    }

    /// Reconstruct file using parity shards
    fn reconstruct_with_parity(&self, shards: &[StorageShard]) -> Result<Vec<u8>> {
        let total_shards = shards[0].total_shards;
        let mut reconstructed_data = Vec::new();
        
        // Reconstruct data from shards
        for i in 0..shards[0].data.len() {
            let mut byte = 0u8;
            for shard in shards {
                byte ^= shard.data[i];
            }
            reconstructed_data.push(byte);
        }
        
        Ok(reconstructed_data)
    }

    /// Verify shard integrity
    pub fn verify_shard(&self, shard: &StorageShard) -> bool {
        let expected_hash = self.calculate_shard_hash(&shard.data, &shard.file_hash, shard.id as usize);
        shard.hash == expected_hash
    }

    /// Get shard distribution map
    pub fn get_shard_distribution(&self, shards: &[StorageShard]) -> HashMap<u32, Vec<[u8; 32]>> {
        let mut distribution = HashMap::new();
        
        for shard in shards {
            distribution.entry(shard.id).or_insert_with(Vec::new);
            // TODO: Add node IDs that store this shard
        }
        
        distribution
    }

    /// Create parity shards for redundancy
    fn create_parity_shards(&self, data_shards: &[StorageShard], parity_count: usize) -> Result<Vec<StorageShard>> {
        let mut parity_shards = Vec::new();
        
        // Simple XOR-based parity (not as robust as Reed-Solomon)
        for i in 0..parity_count {
            let mut parity_data = Vec::new();
            
            // Calculate parity for each byte position
            for byte_pos in 0..data_shards[0].data.len() {
                let mut parity_byte = 0u8;
                for shard in data_shards {
                    parity_byte ^= shard.data[byte_pos];
                }
                parity_data.push(parity_byte);
            }
            
            let parity_hash = self.calculate_shard_hash(&parity_data, &data_shards[0].file_hash, data_shards.len() + i);
            
            parity_shards.push(StorageShard {
                id: (data_shards.len() + i) as u32,
                data: parity_data.clone(),
                hash: parity_hash,
                size: parity_data.len() as u64,
                total_shards: (data_shards.len() + parity_count) as u32,
                file_hash: data_shards[0].file_hash,
                parity_data: None,
            });
        }
        
        Ok(parity_shards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_sharding() {
        let config = ShardConfig::default();
        let manager = ShardManager::new(config);
        
        let file_data = b"This is a test file that will be split into multiple shards for distributed storage across the IPPAN network.";
        let file_hash = [1u8; 32];
        
        let shards = manager.split_file(file_data, &file_hash).unwrap();
        
        assert_eq!(shards.len(), 20); // 16 data + 4 parity shards
        
        // Verify all shards
        for shard in &shards {
            assert!(manager.verify_shard(shard));
        }
    }

    #[test]
    fn test_file_reconstruction() {
        let config = ShardConfig::default();
        let manager = ShardManager::new(config);
        
        let file_data = b"Test file for reconstruction";
        let file_hash = [2u8; 32];
        
        let all_shards = manager.split_file(file_data, &file_hash).unwrap();
        
        // Reconstruct using only data shards
        let data_shards: Vec<_> = all_shards.iter()
            .filter(|s| s.id < 16)
            .collect();
        
        let reconstructed = manager.reconstruct_file(&data_shards).unwrap();
        assert_eq!(file_data, reconstructed.as_slice());
    }
}
