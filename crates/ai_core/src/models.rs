//! Model management and loading
//!
//! Provides registration, validation, and dynamic loading of AI models from
//! local storage, remote URLs, IPFS, or blockchain gateways.
//! Supports integrity verification via BLAKE3 hash comparison.

use crate::{
    errors::{AiCoreError, Result},
    types::*,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::collections::HashMap;
use std::fs;
use tracing::{info, warn};
#[cfg(not(feature = "remote_loading"))]
use tracing::error;

/// Optional dependency for remote loading (activated via `remote_loading` feature)
#[cfg(feature = "remote_loading")]
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

/// Enumerates supported model source types
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

        let model_id = metadata.id.clone();
        self.registry.available.insert(model_id.clone(), metadata);
        self.registry.sources.insert(model_id, source);

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
                fs::read(&source.location).map_err(|e| AiCoreError::Io(e.to_string()))
            }
            SourceType::Remote => self.load_from_url(&source.location).await,
            SourceType::Ipfs => self.load_from_ipfs(&source.location).await,
            SourceType::Blockchain => self.load_from_blockchain(&source.location).await,
        }
    }

    /// Load model data from HTTP(S) URL
    async fn load_from_url(&self, url: &str) -> Result<Vec<u8>> {
        info!("Loading model from URL: {}", url);

        // Inline formats for testing or offline deployment
        if let Some(data) = url.strip_prefix("data:application/octet-stream;base64,") {
            let bytes = STANDARD
                .decode(data)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64 data: {}", e)))?;
            return Ok(bytes);
        }

        if let Some(hex_data) = url.strip_prefix("hex://") {
            let bytes = hex::decode(hex_data)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid hex payload: {}", e)))?;
            return Ok(bytes);
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AiCoreError::InvalidParameters(
                "Invalid URL: must start with http://, https://, data:, or hex://".to_string(),
            ));
        }

        #[cfg(feature = "remote_loading")]
        {
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .map_err(|e| {
                    AiCoreError::ExecutionFailed(format!("HTTP client init failed: {}", e))
                })?;

            let response =
                client.get(url).send().await.map_err(|e| {
                    AiCoreError::ExecutionFailed(format!("HTTP request failed: {}", e))
                })?;

            if !response.status().is_success() {
                return Err(AiCoreError::ExecutionFailed(format!(
                    "HTTP error: {}",
                    response.status()
                )));
            }

            let data = response
                .bytes()
                .await
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Read failed: {}", e)))?
                .to_vec();

            info!("Model loaded from URL successfully ({} bytes)", data.len());
            Ok(data)
        }

        #[cfg(not(feature = "remote_loading"))]
        {
            error!("Remote HTTP model loading not enabled (requires remote_loading feature)");
            Err(AiCoreError::ExecutionFailed(
                "Remote HTTP loading not enabled".to_string(),
            ))
        }
    }

    /// Load model data from IPFS (via gateways or embedded base64)
    async fn load_from_ipfs(&self, hash: &str) -> Result<Vec<u8>> {
        info!("Loading model from IPFS: {}", hash);

        if let Some(b64) = hash.strip_prefix("base64:") {
            let bytes = STANDARD
                .decode(b64)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64: {}", e)))?;
            return Ok(bytes);
        }

        if !hash.starts_with("Qm") && !hash.starts_with("bafy") {
            return Err(AiCoreError::InvalidParameters(
                "Invalid IPFS hash format (must start with Qm or bafy)".to_string(),
            ));
        }

        #[cfg(feature = "remote_loading")]
        {
            let gateways = vec![
                format!("https://ipfs.io/ipfs/{}", hash),
                format!("https://gateway.pinata.cloud/ipfs/{}", hash),
                format!("https://cloudflare-ipfs.com/ipfs/{}", hash),
                format!("https://dweb.link/ipfs/{}", hash),
            ];

            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .map_err(|e| {
                    AiCoreError::ExecutionFailed(format!("HTTP client init failed: {}", e))
                })?;

            for gateway_url in &gateways {
                info!("Trying IPFS gateway: {}", gateway_url);
                match client.get(gateway_url).send().await {
                    Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                        Ok(data) => {
                            info!(
                                "Model loaded from IPFS via {} ({} bytes)",
                                gateway_url,
                                data.len()
                            );
                            return Ok(data.to_vec());
                        }
                        Err(e) => warn!("Failed to read from {}: {}", gateway_url, e),
                    },
                    Ok(resp) => warn!("Gateway {} returned {}", gateway_url, resp.status()),
                    Err(e) => warn!("Failed to connect to {}: {}", gateway_url, e),
                }
            }

            Err(AiCoreError::ExecutionFailed(
                "Failed to load model from IPFS gateways".to_string(),
            ))
        }

        #[cfg(not(feature = "remote_loading"))]
        {
            error!("IPFS client not available (requires remote_loading feature)");
            Err(AiCoreError::ExecutionFailed(
                "IPFS client not available".to_string(),
            ))
        }
    }

    /// Load model data from blockchain storage (via gateway or embedded payload)
    async fn load_from_blockchain(&self, storage_key: &str) -> Result<Vec<u8>> {
        info!("Loading model from blockchain storage: {}", storage_key);

        if let Some(b64) = storage_key.strip_prefix("base64:") {
            let bytes = STANDARD
                .decode(b64)
                .map_err(|e| AiCoreError::ExecutionFailed(format!("Invalid base64: {}", e)))?;
            return Ok(bytes);
        }

        if storage_key.is_empty() {
            return Err(AiCoreError::InvalidParameters(
                "Empty storage key".to_string(),
            ));
        }

        if let Ok(base) = std::env::var("IPPAN_GATEWAY_URL") {
            let url = format!("{}/models/{}", base.trim_end_matches('/'), storage_key);
            if let Ok(bytes) = self.load_from_url(&url).await {
                return Ok(bytes);
            }
        }

        Err(AiCoreError::ExecutionFailed(format!(
            "Blockchain model loading requires RPC or IPPAN_GATEWAY_URL (key: {})",
            storage_key
        )))
    }

    /// Validate model metadata structure
    fn validate_metadata(&self, metadata: &ModelMetadata) -> Result<()> {
        if metadata.id.name.is_empty()
            || metadata.id.version.is_empty()
            || metadata.id.hash.is_empty()
        {
            return Err(AiCoreError::InvalidParameters(
                "Model ID fields cannot be empty".to_string(),
            ));
        }
        if metadata.input_shape.is_empty() {
            return Err(AiCoreError::InvalidParameters(
                "Input shape cannot be empty".to_string(),
            ));
        }
        if metadata.output_shape.is_empty() {
            return Err(AiCoreError::InvalidParameters(
                "Output shape cannot be empty".to_string(),
            ));
        }
        if metadata.parameter_count == 0 || metadata.size_bytes == 0 {
            return Err(AiCoreError::InvalidParameters(
                "Parameter count and size must be nonzero".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate integrity of loaded model data (size and hash)
    fn validate_model_data(&self, data: &[u8], metadata: &ModelMetadata) -> Result<()> {
        if data.len() as u64 != metadata.size_bytes {
            return Err(AiCoreError::ValidationFailed(
                "Model data size mismatch".to_string(),
            ));
        }

        let computed_hash = blake3::hash(data).to_hex().to_string();
        if computed_hash != metadata.id.hash {
            return Err(AiCoreError::ValidationFailed(
                "Model hash verification failed".to_string(),
            ));
        }

        Ok(())
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            available: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    pub fn list_models(&self) -> Vec<&ModelId> {
        self.available.keys().collect()
    }

    pub fn get_model_metadata(&self, model_id: &ModelId) -> Option<&ModelMetadata> {
        self.available.get(model_id)
    }

    pub fn get_model_source(&self, model_id: &ModelId) -> Option<&ModelSource> {
        self.sources.get(model_id)
    }
}
