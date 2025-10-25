//! Model management and loading

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use std::collections::HashMap;
use tracing::{info, warn, error};
use reqwest::Client;

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
        
        // Add to registry
        self.registry.available.insert(metadata.id.clone(), metadata);
        self.registry.sources.insert(metadata.id.clone(), source);
        
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
        let model_data = self.load_model_data(&source).await?;
        
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
                tokio::fs::read(&source.location)
                    .await
                    .map_err(|e| AiCoreError::Io(e))
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
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AiCoreError::ExecutionFailed(format!("failed to build http client: {e}")))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| AiCoreError::ExecutionFailed(format!("http request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(AiCoreError::ExecutionFailed(format!(
                "remote fetch failed with status {}", resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AiCoreError::ExecutionFailed(format!("failed to read response body: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// Load model data from IPFS
    async fn load_from_ipfs(&self, hash: &str) -> Result<Vec<u8>> {
        // Best-effort: attempt public IPFS gateway. Hash may include CID prefix.
        let url = format!("https://ipfs.io/ipfs/{hash}");
        match self.load_from_url(&url).await {
            Ok(bytes) => Ok(bytes),
            Err(err) => {
                // Try cloudflare gateway as fallback
                let alt_url = format!("https://cloudflare-ipfs.com/ipfs/{hash}");
                self.load_from_url(&alt_url).await.map_err(|_| err)
            }
        }
    }

    /// Load model data from blockchain storage
    async fn load_from_blockchain(&self, storage_key: &str) -> Result<Vec<u8>> {
        // Minimal production implementation: attempt gateway endpoint if configured via env
        // IPPAN_GATEWAY_URL like http://host:8081/api
        if let Ok(base) = std::env::var("IPPAN_GATEWAY_URL") {
            let url = format!("{}/models/{}", base.trim_end_matches('/'), storage_key);
            if let Ok(bytes) = self.load_from_url(&url).await {
                return Ok(bytes);
            }
        }
        Err(AiCoreError::ExecutionFailed(
            "Blockchain storage loading not available (set IPPAN_GATEWAY_URL)".to_string(),
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
        let computed_hash = blake3::hash(data).to_hex();
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