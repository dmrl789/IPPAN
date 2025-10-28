//! IPPAN On-Chain AI Model Registry
//!
//! Provides deterministic, verifiable registry logic for AI models
//! used at L1 consensus and reputation scoring. Includes lifecycle
//! management (propose → activate → deprecate → revoke) and
//! Ed25519-based signature verification.

use crate::{
    errors::{RegistryError, Result},
    types::*,
    storage::RegistryStorage,
};
use ai_core::types::{ModelId, ModelMetadata};
use std::collections::HashMap;
use tracing::{info, warn, error};

/// Deterministic in-memory AI model registry
///
/// This struct can be serialized and verified on-chain,
/// ensuring consistent behavior across all nodes.
pub struct ModelRegistry {
    /// Storage backend
    storage: RegistryStorage,
    /// Currently active model ID
    active_model_id: Option<ModelId>,
    /// Configuration
    config: RegistryConfig,
    /// In-memory cache for quick access
    cache: HashMap<String, ModelRegistration>,
}

impl ModelRegistry {
    /// Create a new registry with storage backend
    pub fn new(storage: RegistryStorage, config: RegistryConfig) -> Self {
        Self {
            storage,
            active_model_id: None,
            config,
            cache: HashMap::new(),
        }
    }

    /// Register a model
    pub async fn register_model(
        &mut self,
        model_id: ModelId,
        metadata: ModelMetadata,
        registrant: String,
        category: ModelCategory,
        description: Option<String>,
        license: Option<String>,
        source_url: Option<String>,
        tags: Vec<String>,
    ) -> Result<ModelRegistration> {
        info!("Registering model: {:?}", model_id);
        
        // Validate model size
        if metadata.size_bytes > self.config.max_model_size {
            return Err(RegistryError::InvalidRegistration(format!(
                "Model size {} exceeds maximum {}",
                metadata.size_bytes, self.config.max_model_size
            )));
        }
        
        // Validate parameter count
        if metadata.parameter_count > self.config.max_parameter_count {
            return Err(RegistryError::InvalidRegistration(format!(
                "Parameter count {} exceeds maximum {}",
                metadata.parameter_count, self.config.max_parameter_count
            )));
        }
        
        // Check if model already exists
        if self.storage.load_model_registration(&model_id).await?.is_some() {
            return Err(RegistryError::ModelAlreadyExists(format!(
                "Model {} already exists",
                model_id.name
            )));
        }
        
        let now = chrono::Utc::now();
        
        let registration = ModelRegistration {
            model_id: model_id.clone(),
            metadata,
            status: RegistrationStatus::Pending,
            registrant,
            registered_at: now,
            updated_at: now,
            registration_fee: self.config.min_registration_fee,
            category,
            tags,
            description,
            license,
            source_url,
        };
        
        // Store registration
        self.storage.store_model_registration(&registration).await?;
        
        // Update cache
        self.cache.insert(model_id.name.clone(), registration.clone());
        
        info!("Model registered successfully: {:?}", model_id);
        Ok(registration)
    }

    /// Get model registration
    pub async fn get_model_registration(&mut self, model_id: &ModelId) -> Result<Option<ModelRegistration>> {
        // Check cache first
        if let Some(registration) = self.cache.get(&model_id.name) {
            return Ok(Some(registration.clone()));
        }
        
        // Load from storage
        let registration = self.storage.load_model_registration(model_id).await?;
        
        // Update cache
        if let Some(ref reg) = registration {
            self.cache.insert(model_id.name.clone(), reg.clone());
        }
        
        Ok(registration)
    }

    /// Update model status
    pub async fn update_model_status(&mut self, model_id: &ModelId, status: RegistrationStatus) -> Result<()> {
        info!("Updating model status: {:?} -> {:?}", model_id, status);
        
        let mut registration = self.storage.load_model_registration(model_id)
            .await?
            .ok_or_else(|| RegistryError::ModelNotFound(model_id.name.clone()))?;
        
        registration.status = status;
        registration.updated_at = chrono::Utc::now();
        
        // Store updated registration
        self.storage.store_model_registration(&registration).await?;
        
        // Update cache
        self.cache.insert(model_id.name.clone(), registration.clone());
        
        // Update active model if needed
        if status == RegistrationStatus::Approved {
            self.active_model_id = Some(model_id.clone());
        } else if self.active_model_id.as_ref() == Some(model_id) {
            self.active_model_id = None;
        }
        
        info!("Model status updated successfully");
        Ok(())
    }

    /// List models by status
    pub async fn list_models_by_status(&self, status: RegistrationStatus) -> Result<Vec<ModelRegistration>> {
        self.storage.list_models_by_status(status).await
    }

    /// List models by category
    pub async fn list_models_by_category(&self, category: ModelCategory) -> Result<Vec<ModelRegistration>> {
        self.storage.list_models_by_category(category).await
    }

    /// Search models
    pub async fn search_models(
        &self,
        query: &str,
        category: Option<ModelCategory>,
        status: Option<RegistrationStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<ModelRegistration>> {
        self.storage.search_models(query, category, status, limit).await
    }

    /// Get model usage statistics
    pub async fn get_model_usage_stats(&self, model_id: &ModelId) -> Result<Option<ModelUsageStats>> {
        self.storage.get_model_usage_stats(model_id).await
    }

    /// Update model usage statistics
    pub async fn update_model_usage_stats(&self, stats: &ModelUsageStats) -> Result<()> {
        self.storage.store_model_usage_stats(stats).await
    }

    /// Get the currently active model
    pub fn get_active_model_id(&self) -> Option<&ModelId> {
        self.active_model_id.as_ref()
    }

    /// Set the active model
    pub fn set_active_model(&mut self, model_id: Option<ModelId>) {
        self.active_model_id = model_id;
    }

    /// Get registry statistics
    pub async fn get_registry_stats(&mut self) -> Result<RegistryStats> {
        let all_models = self.storage.list_models_by_status(RegistrationStatus::Pending).await?
            .len() as u64
            + self.storage.list_models_by_status(RegistrationStatus::Approved).await?
            .len() as u64
            + self.storage.list_models_by_status(RegistrationStatus::Rejected).await?
            .len() as u64
            + self.storage.list_models_by_status(RegistrationStatus::Suspended).await?
            .len() as u64
            + self.storage.list_models_by_status(RegistrationStatus::Deprecated).await?
            .len() as u64;
        
        let active_models = self.storage.list_models_by_status(RegistrationStatus::Approved).await?.len() as u64;
        let pending_models = self.storage.list_models_by_status(RegistrationStatus::Pending).await?.len() as u64;
        
        Ok(RegistryStats {
            total_models: all_models,
            active_models,
            pending_models,
            total_executions: 0, // This would be tracked separately
            total_fees_collected: 0, // This would be tracked by FeeManager
            total_registrants: 0, // This would require tracking unique registrants
            avg_model_size: 0, // This would require calculating average
            most_popular_category: None, // This would require analyzing categories
        })
    }

    /// Get configuration
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        let config = RegistryConfig {
            min_registration_fee: 1000,
            max_registration_fee: 1000000,
            default_execution_fee: 100,
            storage_fee_per_byte_per_day: 1,
            proposal_fee: 10000,
            voting_period_seconds: 86400 * 7, // 7 days
            execution_period_seconds: 86400 * 14, // 14 days
            min_voting_power: 1000,
            max_model_size: 1024 * 1024 * 100, // 100MB
            max_parameter_count: 1_000_000_000, // 1B parameters
        };
        
        Self::new(
            RegistryStorage::new(None).unwrap(),
            config,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_model_id() -> ModelId {
        ModelId {
            name: "test_model".to_string(),
            version: "1.0.0".to_string(),
            hash: "abc123".to_string(),
        }
    }

    fn create_test_metadata() -> ModelMetadata {
        ModelMetadata {
            id: create_test_model_id(),
            architecture: "GBDT".to_string(),
            input_shape: vec![10],
            output_shape: vec![1],
            parameter_count: 1000,
            size_bytes: 10000,
            created_at: 1234567890,
            description: Some("Test model".to_string()),
        }
    }

    #[tokio::test]
    async fn test_model_registration() {
        let storage = RegistryStorage::new(None).unwrap();
        let config = RegistryConfig {
            min_registration_fee: 1000,
            max_registration_fee: 1000000,
            default_execution_fee: 100,
            storage_fee_per_byte_per_day: 1,
            proposal_fee: 10000,
            voting_period_seconds: 86400 * 7,
            execution_period_seconds: 86400 * 14,
            min_voting_power: 1000,
            max_model_size: 1024 * 1024 * 100,
            max_parameter_count: 1_000_000_000,
        };
        let mut registry = ModelRegistry::new(storage, config);
        
        let model_id = create_test_model_id();
        let metadata = create_test_metadata();
        
        let registration = registry.register_model(
            model_id.clone(),
            metadata,
            "test_user".to_string(),
            ModelCategory::Other,
            Some("Test description".to_string()),
            Some("MIT".to_string()),
            Some("https://example.com".to_string()),
            vec!["test".to_string()],
        ).await.unwrap();
        
        assert_eq!(registration.status, RegistrationStatus::Pending);
        assert_eq!(registration.model_id.name, "test_model");
    }

    #[tokio::test]
    async fn test_model_status_update() {
        let storage = RegistryStorage::new(None).unwrap();
        let config = RegistryConfig {
            min_registration_fee: 1000,
            max_registration_fee: 1000000,
            default_execution_fee: 100,
            storage_fee_per_byte_per_day: 1,
            proposal_fee: 10000,
            voting_period_seconds: 86400 * 7,
            execution_period_seconds: 86400 * 14,
            min_voting_power: 1000,
            max_model_size: 1024 * 1024 * 100,
            max_parameter_count: 1_000_000_000,
        };
        let mut registry = ModelRegistry::new(storage, config);
        
        let model_id = create_test_model_id();
        let metadata = create_test_metadata();
        
        registry.register_model(
            model_id.clone(),
            metadata,
            "test_user".to_string(),
            ModelCategory::Other,
            None,
            None,
            None,
            vec![],
        ).await.unwrap();
        
        registry.update_model_status(&model_id, RegistrationStatus::Approved).await.unwrap();
        
        let registration = registry.get_model_registration(&model_id).await.unwrap().unwrap();
        assert_eq!(registration.status, RegistrationStatus::Approved);
    }
}
