//! Model management and loading

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use std::collections::HashMap;
use std::fs;
use tracing::{info, warn, error};

/// Model manager for loading and managing AI models
pub struct ModelManager {
    /// Loaded models
    models: HashMap<ModelId, LoadedModel>,
    /// Model registry
    registry: ModelRegistry,
}

/// Loaded model with execution state
pub struct LoadedModel {
    /// Model metadata
    pub metadata: ModelMetadata,
    /// Model data
    pub data: Vec<u8>,
    /// Load timestamp
    pub loaded_at: u64,
    /// Access count
    pub access_count: u64,
}

/// Model registry for tracking available models
pub struct ModelRegistry {
    /// Available models
    available: HashMap<ModelId, ModelMetadata>,
    /// Model sources
    sources: HashMap<ModelId, ModelSource>,
}

/// Model source information
#[derive(Debug, Clone)]
pub struct ModelSource {
    /// Source URL or path
    pub location: String,
    /// Source type
    pub source_type: SourceType,
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Source type for models
#[derive(Debug, Clone)]
pub enum SourceType {
    /// Local file system
    Local,
    /// Remote URL
    Remote,
    /// IPFS hash
    Ipfs,
    /// Blockchain storage
    Blockchain,
}

impl ModelManager {
    /// Create a new model manager
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            registry: ModelRegistry::new(),
        }
    }

    /// Register a model in the registry
    pub fn register_model(&mut self, metadata: ModelMetadata, source: ModelSource) -> Result<()> {
        info!("Registering model: {:?}", metadata.id);
        
        // Validate metadata
        self.validate_metadata(&metadata)?;
        
        // Clone model_id before moving metadata
        let model_id = metadata.id.clone();
        
        // Add to registry
        self.registry.available.insert(model_id.clone(), metadata);
        self.registry.sources.insert(model_id, source);
        
        info!("Model registered successfully");
        Ok(())
    }

    /// Load a model from the registry
    pub async fn load_model(&mut self, model_id: &ModelId) -> Result<()> {
        info!("Loading model: {:?}", model_id);
        
        // Check if model is already loaded
        if self.models.contains_key(model_id) {
            info!("Model already loaded");
            return Ok(());
        }
        
        // Get model metadata from registry
        let metadata = self.registry.available
            .get(model_id)
            .ok_or_else(|| AiCoreError::ExecutionFailed("Model not found in registry".to_string()))?
            .clone();
        
        // Get model source
        let source = self.registry.sources
            .get(model_id)
            .ok_or_else(|| AiCoreError::ExecutionFailed("Model source not found".to_string()))?
            .clone();
        
        // Load model data
        let model_data: Vec<u8> = self.load_model_data(&source).await?;
        
        // Validate model data
        self.validate_model_data(&model_data, &metadata)?;
        
        // Create loaded model
        let loaded_model = LoadedModel {
            metadata,
            data: model_data,
            loaded_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 0,
        };
        
        // Store loaded model
        self.models.insert(model_id.clone(), loaded_model);
        
        info!("Model loaded successfully");
        Ok(())
    }

    /// Unload a model
    pub fn unload_model(&mut self, model_id: &ModelId) -> Result<()> {
        info!("Unloading model: {:?}", model_id);
        
        if self.models.remove(model_id).is_some() {
            info!("Model unloaded successfully");
            Ok(())
        } else {
            Err(AiCoreError::ExecutionFailed("Model not loaded".to_string()))
        }
    }

    /// Get model metadata
    pub fn get_model_metadata(&self, model_id: &ModelId) -> Option<&ModelMetadata> {
        self.models.get(model_id).map(|m| &m.metadata)
    }

    /// Get loaded model count
    pub fn loaded_model_count(&self) -> usize {
        self.models.len()
    }

    /// Get available model count
    pub fn available_model_count(&self) -> usize {
        self.registry.available.len()
    }

    /// Load model data from source
    async fn load_model_data(&self, source: &ModelSource) -> Result<Vec<u8>> {
        match source.source_type {
            SourceType::Local => {
                // Load from local file system
                fs::read(&source.location)
                    .map_err(AiCoreError::Io)
            },
            SourceType::Remote => {
                // Load from remote URL
                self.load_from_url(&source.location).await
            },
            SourceType::Ipfs => {
                // Load from IPFS
                self.load_from_ipfs(&source.location).await
            },
            SourceType::Blockchain => {
                // Load from blockchain storage
                self.load_from_blockchain(&source.location).await
            },
        }
    }

    /// Load model data from URL
    async fn load_from_url(&self, url: &str) -> Result<Vec<u8>> {
        info!("Loading model from URL: {}", url);
        
        // Support data: URLs for offline tests
        if let Some(data) = url.strip_prefix("data:application/octet-stream;base64,") {
            let bytes = base64::decode(data)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64 data URL: {}", e)))?;
            return Ok(bytes);
        }

        // Support hex-encoded payloads using hex:// prefix
        if let Some(hex_data) = url.strip_prefix("hex://") {
            let bytes = hex::decode(hex_data)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid hex payload: {}", e)))?;
            return Ok(bytes);
        }
        
        // Validate HTTP/HTTPS URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AiCoreError::InvalidParameters(
                "Invalid URL: must start with http://, https://, data:, or hex://".to_string()
            ));
        }
        
        // Create HTTP client (requires remote_loading feature)
        #[cfg(feature = "remote_loading")]
        {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
                .build()
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Failed to create HTTP client: {}", e)))?;
            
            // Fetch model data
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Failed to fetch model: {}", e)))?;
            
            // Check response status
            if !response.status().is_success() {
                return Err(AiCoreError::ExecutionFailed(
                    format!("HTTP error: {}", response.status())
                ));
            }
            
            // Read response body
            let data = response
                .bytes()
                .await
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Failed to read model data: {}", e)))?
                .to_vec();
            
            info!("Model loaded from URL successfully, size: {} bytes", data.len());
            Ok(data)
        }
        
        #[cfg(not(feature = "remote_loading"))]
        {
            error!("Remote HTTP model loading not enabled (requires remote_loading feature)");
            Err(AiCoreError::ExecutionFailed("Remote HTTP loading not enabled".to_string()))
        }
    }

    /// Load model data from IPFS
    async fn load_from_ipfs(&self, hash: &str) -> Result<Vec<u8>> {
        info!("Loading model from IPFS: {}", hash);
        
        // Allow embedding as base64:<payload> for testing
        if let Some(b64) = hash.strip_prefix("base64:") {
            let bytes = base64::decode(b64)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64 for IPFS: {}", e)))?;
            return Ok(bytes);
        }
        
        // Validate IPFS hash format
        if !hash.starts_with("Qm") && !hash.starts_with("bafy") {
            return Err(AiCoreError::InvalidParameters(
                "Invalid IPFS hash format (must start with Qm or bafy, or use base64: prefix)".to_string()
            ));
        }
        
        // Try multiple IPFS gateways for redundancy (requires remote_loading feature)
        #[cfg(feature = "remote_loading")]
        {
            let gateways = vec![
                format!("https://ipfs.io/ipfs/{}", hash),
                format!("https://gateway.pinata.cloud/ipfs/{}", hash),
                format!("https://cloudflare-ipfs.com/ipfs/{}", hash),
                format!("https://dweb.link/ipfs/{}", hash),
            ];
            
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
                .build()
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Failed to create HTTP client: {}", e)))?;
            
            // Try each gateway until one succeeds
            for gateway_url in &gateways {
                info!("Trying IPFS gateway: {}", gateway_url);
                
                match client.get(gateway_url).send().await {
                    Ok(response) if response.status().is_success() => {
                        match response.bytes().await {
                            Ok(data) => {
                                let vec_data = data.to_vec();
                                info!("Model loaded from IPFS successfully via {}, size: {} bytes", 
                                    gateway_url, vec_data.len());
                                return Ok(vec_data);
                            }
                            Err(e) => {
                                warn!("Failed to read data from {}: {}", gateway_url, e);
                                continue;
                            }
                        }
                    }
                    Ok(response) => {
                        warn!("Gateway {} returned error: {}", gateway_url, response.status());
                        continue;
                    }
                    Err(e) => {
                        warn!("Failed to connect to gateway {}: {}", gateway_url, e);
                        continue;
                    }
                }
            }
            
            Err(AiCoreError::ExecutionFailed(
                "Failed to load model from all IPFS gateways".to_string()
            ))
        }
        
        #[cfg(not(feature = "remote_loading"))]
        {
            error!("IPFS client not available (requires remote_loading feature)");
            Err(AiCoreError::ExecutionFailed("IPFS client not available".to_string()))
        }
    }

    /// Load model data from blockchain storage
    async fn load_from_blockchain(&self, storage_key: &str) -> Result<Vec<u8>> {
        info!("Loading model from blockchain storage: {}", storage_key);
        
        // Support embedded format base64:<payload> for testing
        if let Some(b64) = storage_key.strip_prefix("base64:") {
            let bytes = base64::decode(b64)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64 for chain: {}", e)))?;
            return Ok(bytes);
        }
        
        // Validate storage key format
        if storage_key.is_empty() {
            return Err(AiCoreError::InvalidParameters(
                "Empty storage key".to_string()
            ));
        }
        
        // In a production implementation, this would:
        // 1. Connect to the blockchain RPC
        // 2. Query the storage contract/location
        // 3. Retrieve the model data (possibly chunked)
        // 4. Verify integrity with on-chain hash
        // 5. Reassemble and return the data
        
        // Parse storage key format: "contract:key" or just "key"
        let parts: Vec<&str> = storage_key.split(':').collect();
        let (contract_id, key) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("default", storage_key)
        };
        
        info!("Blockchain storage contract: {}, key: {}", contract_id, key);
        
        // Example implementation skeleton for future blockchain RPC integration:
        // let rpc_url = std::env::var("BLOCKCHAIN_RPC_URL")
        //     .unwrap_or_else(|_| "http://localhost:8545".to_string());
        // 
        // let client = BlockchainClient::new(&rpc_url)?;
        // let storage_contract = client.get_contract(contract_id)?;
        // let data = storage_contract.call("getModelData", &[key]).await?;
        
        // For now, return an error indicating this needs blockchain integration
        Err(AiCoreError::ExecutionFailed(
            format!("Blockchain model loading requires blockchain RPC configuration. \
                    Storage key: {}:{}. Use base64: prefix for testing.", contract_id, key)
        ))
    }

    /// Validate model metadata
    fn validate_metadata(&self, metadata: &ModelMetadata) -> Result<()> {
        // Check required fields
        if metadata.id.name.is_empty() {
            return Err(AiCoreError::InvalidParameters("Model name cannot be empty".to_string()));
        }
        
        if metadata.id.version.is_empty() {
            return Err(AiCoreError::InvalidParameters("Model version cannot be empty".to_string()));
        }
        
        if metadata.id.hash.is_empty() {
            return Err(AiCoreError::InvalidParameters("Model hash cannot be empty".to_string()));
        }
        
        // Check shapes
        if metadata.input_shape.is_empty() {
            return Err(AiCoreError::InvalidParameters("Input shape cannot be empty".to_string()));
        }
        
        if metadata.output_shape.is_empty() {
            return Err(AiCoreError::InvalidParameters("Output shape cannot be empty".to_string()));
        }
        
        // Check parameter count
        if metadata.parameter_count == 0 {
            return Err(AiCoreError::InvalidParameters("Parameter count cannot be zero".to_string()));
        }
        
        // Check size
        if metadata.size_bytes == 0 {
            return Err(AiCoreError::InvalidParameters("Model size cannot be zero".to_string()));
        }
        
        Ok(())
    }

    /// Validate model data
    fn validate_model_data(&self, data: &[u8], metadata: &ModelMetadata) -> Result<()> {
        // Check data size matches metadata
        if data.len() as u64 != metadata.size_bytes {
            return Err(AiCoreError::ValidationFailed(
                "Model data size mismatch".to_string()
            ));
        }

        // Verify model hash
        let computed_hash = blake3::hash(data).to_hex().to_string();
        if computed_hash != metadata.id.hash {
            return Err(AiCoreError::ValidationFailed(
                "Model hash verification failed".to_string()
            ));
        }

        Ok(())
    }
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// List available models
    pub fn list_models(&self) -> Vec<&ModelId> {
        self.available.keys().collect()
    }

    /// Get model metadata
    pub fn get_model_metadata(&self, model_id: &ModelId) -> Option<&ModelMetadata> {
        self.available.get(model_id)
    }

    /// Get model source
    pub fn get_model_source(&self, model_id: &ModelId) -> Option<&ModelSource> {
        self.sources.get(model_id)
    }
}