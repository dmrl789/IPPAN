//! Model management and loading
//!
//! Provides registration, validation, and dynamic loading of AI models from
//! local storage, remote URLs, IPFS, or blockchain gateways.
//! Supports integrity verification via BLAKE3 hash comparison.

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use std::collections::HashMap;
use std::fs;
use tracing::{info, warn, error};
use reqwest::Client;

/// Model manager for loading and managing AI models
pub struct ModelManager {
    /// Loaded models currently in memory
    models: HashMap<ModelId, LoadedModel>,
    /// Registry of all known models and their sources
    registry: ModelRegistry,
}

/// Represents a loaded model instance with associated metadata and usage stats
pub struct LoadedModel {
    pub metadata: ModelMetadata,
    pub data: Vec<u8>,
    pub loaded_at: u64,
    pub access_count: u64,
}

/// Registry for tracking available models and their sources
pub struct ModelRegistry {
    available: HashMap<ModelId, ModelMetadata>,
    sources: HashMap<ModelId, ModelSource>,
}

/// Describes a model source (location and type)
#[derive(Debug, Clone)]
pub struct ModelSource {
    pub location: String,
    pub source_type: SourceType,
    pub last_updated: u64,
}

/// Enumerates the supported source types
#[derive(Debug, Clone)]
pub enum SourceType {
    Local,
    Remote,
    Ipfs,
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
        self.validate_metadata(&metadata)?;

        self.registry
            .available
            .insert(metadata.id.clone(), metadata.clone());
        self.registry
            .sources
            .insert(metadata.id.clone(), source);

        info!("Model registered successfully");
        Ok(())
    }

    /// Load a model from the registry (local, remote, IPFS, or blockchain)
    pub async fn load_model(&mut self, model_id: &ModelId) -> Result<()> {
        info!("Loading model: {:?}", model_id);

        if self.models.contains_key(model_id) {
            info!("Model already loaded");
            return Ok(());
        }

        let metadata = self
            .registry
            .available
            .get(model_id)
            .ok_or_else(|| AiCoreError::ExecutionFailed("Model not found in registry".to_string()))?
            .clone();

        let source = self
            .registry
            .sources
            .get(model_id)
            .ok_or_else(|| AiCoreError::ExecutionFailed("Model source not found".to_string()))?
            .clone();

        let model_data = self.load_model_data(&source).await?;
        self.validate_model_data(&model_data, &metadata)?;

        let loaded_model = LoadedModel {
            metadata,
            data: model_data,
            loaded_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 0,
        };

        self.models.insert(model_id.clone(), loaded_model);
        info!("Model loaded successfully");
        Ok(())
    }

    /// Unload a model from memory
    pub fn unload_model(&mut self, model_id: &ModelId) -> Result<()> {
        info!("Unloading model: {:?}", model_id);
        if self.models.remove(model_id).is_some() {
            info!("Model unloaded successfully");
            Ok(())
        } else {
            Err(AiCoreError::ExecutionFailed("Model not loaded".to_string()))
        }
    }

    /// Retrieve model metadata
    pub fn get_model_metadata(&self, model_id: &ModelId) -> Option<&ModelMetadata> {
        self.models.get(model_id).map(|m| &m.metadata)
    }

    pub fn loaded_model_count(&self) -> usize {
        self.models.len()
    }

    pub fn available_model_count(&self) -> usize {
        self.registry.available.len()
    }

    /// Core loader: read model data from any supported source type
    async fn load_model_data(&self, source: &ModelSource) -> Result<Vec<u8>> {
        match source.source_type {
            SourceType::Local => {
                fs::read(&source.location).map_err(AiCoreError::Io)
            }
            SourceType::Remote => self.load_from_url(&source.location).await,
            SourceType::Ipfs => self.load_from_ipfs(&source.location).await,
            SourceType::Blockchain => self.load_from_blockchain(&source.location).await,
        }
    }

    /// Load model data from HTTP(S) URL
    async fn load_from_url(&self, url: &str) -> Result<Vec<u8>> {
        // Support embedded test payloads
        if let Some(data) = url.strip_prefix("data:application/octet-stream;base64,") {
            let bytes = base64::decode(data)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64 data URL: {e}")))?;
            return Ok(bytes);
        }

        if let Some(hex_data) = url.strip_prefix("hex://") {
            let bytes = hex::decode(hex_data)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid hex payload: {e}")))?;
            return Ok(bytes);
        }

        // Production HTTP fetch
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AiCoreError::ExecutionFailed(format!("Failed to build HTTP client: {e}")))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| AiCoreError::ExecutionFailed(format!("HTTP request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(AiCoreError::ExecutionFailed(format!(
                "Remote fetch failed with status {}", resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AiCoreError::ExecutionFailed(format!("Failed to read response body: {e}")))?;
        Ok(bytes.to_vec())
    }

    /// Load model data from IPFS (using public gateways or embedded base64)
    async fn load_from_ipfs(&self, hash: &str) -> Result<Vec<u8>> {
        if let Some(b64) = hash.strip_prefix("base64:") {
            let bytes = base64::decode(b64)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64 for IPFS: {e}")))?;
            return Ok(bytes);
        }

        // Attempt fetch from public gateways
        let url = format!("https://ipfs.io/ipfs/{hash}");
        match self.load_from_url(&url).await {
            Ok(bytes) => Ok(bytes),
            Err(err) => {
                let alt_url = format!("https://cloudflare-ipfs.com/ipfs/{hash}");
                self.load_from_url(&alt_url).await.map_err(|_| err)
            }
        }
    }

    /// Load model data from blockchain storage (via gateway or embedded payload)
    async fn load_from_blockchain(&self, storage_key: &str) -> Result<Vec<u8>> {
        if let Some(b64) = storage_key.strip_prefix("base64:") {
            let bytes = base64::decode(b64)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64 for chain: {e}")))?;
            return Ok(bytes);
        }

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

    /// Validate model metadata structure
    fn validate_metadata(&self, metadata: &ModelMetadata) -> Result<()> {
        if metadata.id.name.is_empty() {
            return Err(AiCoreError::InvalidParameters("Model name cannot be empty".to_string()));
        }
        if metadata.id.version.is_empty() {
            return Err(AiCoreError::InvalidParameters("Model version cannot be empty".to_string()));
        }
        if metadata.id.hash.is_empty() {
            return Err(AiCoreError::InvalidParameters("Model hash cannot be empty".to_string()));
        }
        if metadata.input_shape.is_empty() {
            return Err(AiCoreError::InvalidParameters("Input shape cannot be empty".to_string()));
        }
        if metadata.output_shape.is_empty() {
            return Err(AiCoreError::InvalidParameters("Output shape cannot be empty".to_string()));
        }
        if metadata.parameter_count == 0 {
            return Err(AiCoreError::InvalidParameters("Parameter count cannot be zero".to_string()));
        }
        if metadata.size_bytes == 0 {
            return Err(AiCoreError::InvalidParameters("Model size cannot be zero".to_string()));
        }
        Ok(())
    }

    /// Validate integrity of loaded model data (size and hash)
    fn validate_model_data(&self, data: &[u8], metadata: &ModelMetadata) -> Result<()> {
        if data.len() as u64 != metadata.size_bytes {
            return Err(AiCoreError::ValidationFailed(
                "Model data size mismatch".to_string()
            ));
        }

        let computed_hash = blake3::hash(data).to_hex();
        if computed_hash.as_str() != metadata.id.hash.as_str() {
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

    /// List all registered model IDs
    pub fn list_models(&self) -> Vec<&ModelId> {
        self.available.keys().collect()
    }

    /// Get metadata for a registered model
    pub fn get_model_metadata(&self, model_id: &ModelId) -> Option<&ModelMetadata> {
        self.available.get(model_id)
    }

    /// Get source info for a registered model
    pub fn get_model_source(&self, model_id: &ModelId) -> Option<&ModelSource> {
        self.sources.get(model_id)
    }
}
