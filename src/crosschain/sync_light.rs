use crate::Result;
use crate::consensus::hashtimer::HashTimer;
use crate::crosschain::{LightSyncData, AnchorHeader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Light sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSyncConfig {
    /// Enable light sync
    pub enabled: bool,
    /// Maximum data size for light sync (bytes)
    pub max_data_size: usize,
    /// Cache size for light sync data
    pub cache_size: usize,
    /// Compression enabled
    pub compression_enabled: bool,
    /// Include ZK proofs in light sync
    pub include_zk_proofs: bool,
    /// Include anchor headers in light sync
    pub include_anchor_headers: bool,
}

impl Default for LightSyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_data_size: 1024 * 1024, // 1MB
            cache_size: 1000,
            compression_enabled: true,
            include_zk_proofs: true,
            include_anchor_headers: true,
        }
    }
}

/// Light sync client for ultra-light clients
pub struct LightSyncClient {
    /// Configuration
    config: LightSyncConfig,
    /// Cache for light sync data
    cache: Arc<RwLock<HashMap<u64, LightSyncData>>>,
    /// Statistics
    stats: Arc<RwLock<LightSyncStats>>,
}

/// Light sync statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSyncStats {
    /// Total sync requests
    pub total_sync_requests: u64,
    /// Successful sync requests
    pub successful_sync_requests: u64,
    /// Failed sync requests
    pub failed_sync_requests: u64,
    /// Average sync data size (bytes)
    pub average_sync_data_size: u64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Total data transferred (bytes)
    pub total_data_transferred: u64,
}

impl LightSyncClient {
    /// Create a new light sync client
    pub async fn new(config: LightSyncConfig) -> Result<Self> {
        Ok(Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(LightSyncStats {
                total_sync_requests: 0,
                successful_sync_requests: 0,
                failed_sync_requests: 0,
                average_sync_data_size: 0,
                cache_hit_rate: 0.0,
                total_data_transferred: 0,
            })),
        })
    }

    /// Get light sync data for a specific round
    pub async fn get_sync_data(&self, round: u64) -> Result<Option<LightSyncData>> {
        if !self.config.enabled {
            return Err(crate::error::IppanError::FeatureDisabled("Light sync is disabled".to_string()));
        }

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_data) = cache.get(&round) {
                self.update_stats(true, cached_data).await;
                return Ok(Some(cached_data.clone()));
            }
        }

        // Generate light sync data
        let sync_data = self.generate_light_sync_data(round).await?;
        
        // Cache the data
        {
            let mut cache = self.cache.write().await;
            
            // Trim cache if it exceeds the limit
            if cache.len() >= self.config.cache_size {
                let oldest_round = cache.keys().min().copied();
                if let Some(old_round) = oldest_round {
                    cache.remove(&old_round);
                }
            }
            
            cache.insert(round, sync_data.clone());
        }
        
        self.update_stats(true, &sync_data).await;
        Ok(Some(sync_data))
    }

    /// Generate light sync data for a round
    async fn generate_light_sync_data(&self, round: u64) -> Result<LightSyncData> {
        // In a real implementation, this would query the blockchain state
        // For now, we'll generate mock data
        
        let hashtimer = HashTimer::new([0u8; 32], [0u8; 32]);
        let merkle_root = self.generate_merkle_root(round).await?;
        
        let zk_proof = if self.config.include_zk_proofs {
            Some(self.generate_zk_proof(round).await?)
        } else {
            None
        };
        
        let anchor_headers = if self.config.include_anchor_headers {
            self.generate_anchor_headers(round).await?
        } else {
            Vec::new()
        };
        
        let sync_data = LightSyncData {
            round,
            hashtimer,
            merkle_root,
            zk_proof,
            anchor_headers,
        };
        
        // Compress if enabled
        if self.config.compression_enabled {
            // In a real implementation, this would compress the data
            debug!("Light sync data would be compressed for round {}", round);
        }
        
        info!("Generated light sync data for round {} (size: {} bytes)", 
              round, self.calculate_data_size(&sync_data));
        
        Ok(sync_data)
    }

    /// Generate Merkle root for a round
    async fn generate_merkle_root(&self, round: u64) -> Result<String> {
        // In a real implementation, this would compute the actual Merkle root
        // For now, we'll generate a mock root
        let mock_root = format!("merkle_root_round_{}", round);
        Ok(mock_root)
    }

    /// Generate ZK proof for a round
    async fn generate_zk_proof(&self, round: u64) -> Result<Vec<u8>> {
        // In a real implementation, this would generate the actual ZK proof
        // For now, we'll generate mock proof data
        let mock_proof = format!("zk_proof_round_{}", round).into_bytes();
        Ok(mock_proof)
    }

    /// Generate anchor headers for a round
    async fn generate_anchor_headers(&self, round: u64) -> Result<Vec<AnchorHeader>> {
        // In a real implementation, this would query actual anchor data
        // For now, we'll generate mock anchor headers
        let mock_anchors = vec![
            AnchorHeader {
                chain_id: "starknet".to_string(),
                state_root: format!("0xstarknet_state_root_{}", round),
                timestamp: chrono::Utc::now().timestamp() as u64,
                round,
            },
            AnchorHeader {
                chain_id: "rollupX".to_string(),
                state_root: format!("0xrollupX_state_root_{}", round),
                timestamp: chrono::Utc::now().timestamp() as u64,
                round,
            },
        ];
        
        Ok(mock_anchors)
    }

    /// Calculate data size
    fn calculate_data_size(&self, sync_data: &LightSyncData) -> usize {
        let mut size = 0;
        
        // Round number
        size += 8;
        
        // HashTimer size (approximate)
        size += 32;
        
        // Merkle root
        size += sync_data.merkle_root.len();
        
        // ZK proof
        if let Some(ref zk_proof) = sync_data.zk_proof {
            size += zk_proof.len();
        }
        
        // Anchor headers
        for header in &sync_data.anchor_headers {
            size += header.chain_id.len();
            size += header.state_root.len();
            size += 8; // timestamp
            size += 8; // round
        }
        
        size
    }

    /// Update statistics
    async fn update_stats(&self, success: bool, sync_data: &LightSyncData) {
        let mut stats = self.stats.write().await;
        
        stats.total_sync_requests += 1;
        if success {
            stats.successful_sync_requests += 1;
        } else {
            stats.failed_sync_requests += 1;
        }
        
        let data_size = self.calculate_data_size(sync_data);
        stats.total_data_transferred += data_size as u64;
        
        // Update average data size
        let total_size = stats.average_sync_data_size * (stats.total_sync_requests - 1) + data_size as u64;
        stats.average_sync_data_size = total_size / stats.total_sync_requests;
        
        // Update cache hit rate
        let cache_hits = stats.successful_sync_requests;
        stats.cache_hit_rate = cache_hits as f64 / stats.total_sync_requests as f64;
    }

    /// Get light sync statistics
    pub async fn get_stats(&self) -> Result<LightSyncStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Light sync cache cleared");
        Ok(())
    }

    /// Get cache size
    pub async fn get_cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Export light sync data to JSON
    pub async fn export_sync_data_json(&self, round: u64) -> Result<String> {
        if let Some(sync_data) = self.get_sync_data(round).await? {
            Ok(serde_json::to_string_pretty(&sync_data)?)
        } else {
            Err(crate::error::IppanError::NotFound(
                format!("Light sync data not found for round {}", round)
            ))
        }
    }

    /// Export light sync data to binary format
    pub async fn export_sync_data_binary(&self, round: u64) -> Result<Vec<u8>> {
        if let Some(sync_data) = self.get_sync_data(round).await? {
            Ok(bincode::serialize(&sync_data)?)
        } else {
            Err(crate::error::IppanError::NotFound(
                format!("Light sync data not found for round {}", round)
            ))
        }
    }

    /// Import light sync data from binary format
    pub async fn import_sync_data_binary(&self, round: u64, data: &[u8]) -> Result<()> {
        let sync_data: LightSyncData = bincode::deserialize(data)?;
        
        // Cache the imported data
        {
            let mut cache = self.cache.write().await;
            
            // Trim cache if it exceeds the limit
            if cache.len() >= self.config.cache_size {
                let oldest_round = cache.keys().min().copied();
                if let Some(old_round) = oldest_round {
                    cache.remove(&old_round);
                }
            }
            
            cache.insert(round, sync_data);
        }
        
        info!("Imported light sync data for round {}", round);
        Ok(())
    }

    /// Get light sync data for a range of rounds
    pub async fn get_sync_data_range(&self, start_round: u64, end_round: u64) -> Result<Vec<LightSyncData>> {
        let mut sync_data_list = Vec::new();
        
        for round in start_round..=end_round {
            if let Some(sync_data) = self.get_sync_data(round).await? {
                sync_data_list.push(sync_data);
            }
        }
        
        Ok(sync_data_list)
    }

    /// Validate light sync data
    pub fn validate_sync_data(&self, sync_data: &LightSyncData) -> Result<bool> {
        // Check data size
        let data_size = self.calculate_data_size(sync_data);
        if data_size > self.config.max_data_size {
            return Err(crate::error::IppanError::Validation(
                format!("Light sync data size ({}) exceeds maximum allowed size ({})", 
                       data_size, self.config.max_data_size)
            ));
        }
        
        // Validate Merkle root format
        if sync_data.merkle_root.is_empty() {
            return Err(crate::error::IppanError::Validation(
                "Merkle root cannot be empty".to_string()
            ));
        }
        
        // Validate anchor headers
        for header in &sync_data.anchor_headers {
            if header.chain_id.is_empty() {
                return Err(crate::error::IppanError::Validation(
                    "Anchor header chain ID cannot be empty".to_string()
                ));
            }
            
            if header.state_root.is_empty() {
                return Err(crate::error::IppanError::Validation(
                    "Anchor header state root cannot be empty".to_string()
                ));
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_light_sync_client_creation() {
        let config = LightSyncConfig::default();
        let client = LightSyncClient::new(config).await.unwrap();
        
        let stats = client.get_stats().await.unwrap();
        assert_eq!(stats.total_sync_requests, 0);
    }

    #[tokio::test]
    async fn test_light_sync_data_generation() {
        let config = LightSyncConfig::default();
        let client = LightSyncClient::new(config).await.unwrap();
        
        let sync_data = client.get_sync_data(12345).await.unwrap();
        assert!(sync_data.is_some());
        
        let data = sync_data.unwrap();
        assert_eq!(data.round, 12345);
        assert!(!data.merkle_root.is_empty());
        assert!(data.zk_proof.is_some());
        assert!(!data.anchor_headers.is_empty());
    }

    #[tokio::test]
    async fn test_light_sync_data_validation() {
        let config = LightSyncConfig::default();
        let client = LightSyncClient::new(config).await.unwrap();
        
        let sync_data = client.get_sync_data(12345).await.unwrap().unwrap();
        let is_valid = client.validate_sync_data(&sync_data).unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_light_sync_cache() {
        let config = LightSyncConfig::default();
        let client = LightSyncClient::new(config).await.unwrap();
        
        // First request should generate data
        let _ = client.get_sync_data(12345).await.unwrap();
        
        // Second request should use cache
        let _ = client.get_sync_data(12345).await.unwrap();
        
        let cache_size = client.get_cache_size().await;
        assert_eq!(cache_size, 1);
    }

    #[tokio::test]
    async fn test_light_sync_export_import() {
        let config = LightSyncConfig::default();
        let client = LightSyncClient::new(config).await.unwrap();
        
        let sync_data = client.get_sync_data(12345).await.unwrap().unwrap();
        let binary_data = bincode::serialize(&sync_data).unwrap();
        
        // Import the data
        client.import_sync_data_binary(12346, &binary_data).await.unwrap();
        
        // Verify it was imported
        let imported_data = client.get_sync_data(12346).await.unwrap();
        assert!(imported_data.is_some());
    }
} 