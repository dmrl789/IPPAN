//! Model registry implementation

use crate::{
    errors::{RegistryError, Result},
    types::*,
    storage::RegistryStorage,
};
use ai_core::types::{ModelId, ModelMetadata};
use std::collections::HashMap;
use tracing::{info, warn, error};
use chrono::Utc;

/// AI Model Registry
pub struct ModelRegistry {
    /// Storage backend
    storage: RegistryStorage,
    /// Configuration
    config: RegistryConfig,
    /// Statistics
    stats: RegistryStats,
    /// Model cache
    model_cache: HashMap<ModelId, ModelRegistration>,
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new(storage: RegistryStorage, config: RegistryConfig) -> Self {
        Self {
            storage,
            config,
            stats: RegistryStats::default(),
            model_cache: HashMap::new(),
        }
    }

    /// Register a new model
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
        
        // Check if model already exists
        if self.model_cache.contains_key(&model_id) {
            return Err(RegistryError::ModelAlreadyExists(model_id.name.clone()));
        }
        
        // Validate model metadata
        self.validate_model_metadata(&metadata)?;
        
        // Calculate registration fee
        let registration_fee = self.calculate_registration_fee(&metadata)?;
        
        // Create registration record
        let registration = ModelRegistration {
            model_id: model_id.clone(),
            metadata,
            status: RegistrationStatus::Pending,
            registrant,
            registered_at: Utc::now(),
            updated_at: Utc::now(),
            registration_fee,
            category,
            tags,
            description,
            license,
            source_url,
        };
        
        // Store registration
        self.storage.store_model_registration(&registration).await?;
        
        // Update cache
        self.model_cache.insert(model_id.clone(), registration.clone());
        
        // Update statistics
        self.update_stats();
        
        info!("Model registered successfully: {:?}", model_id);
        Ok(registration)
    }

    /// Get model registration
    pub async fn get_model_registration(&mut self, model_id: &ModelId) -> Result<Option<ModelRegistration>> {
        // Check cache first
        if let Some(registration) = self.model_cache.get(model_id) {
            return Ok(Some(registration.clone()));
        }
        
        // Load from storage
        let registration = self.storage.load_model_registration(model_id).await?;
        
        // Update cache
        if let Some(ref reg) = registration {
            self.model_cache.insert(model_id.clone(), reg.clone());
        }
        
        Ok(registration)
    }

    /// Update model registration status
    pub async fn update_model_status(
        &mut self,
        model_id: &ModelId,
        status: RegistrationStatus,
    ) -> Result<()> {
        info!("Updating model status: {:?} -> {:?}", model_id, status);
        
        // Get current registration
        let mut registration = self.get_model_registration(model_id).await?
            .ok_or_else(|| RegistryError::ModelNotFound(model_id.name.clone()))?;
        
        // Update status
        registration.status = status;
        registration.updated_at = Utc::now();
        
        // Store updated registration
        self.storage.store_model_registration(&registration).await?;
        
        // Update cache
        self.model_cache.insert(model_id.clone(), registration);
        
        // Update statistics
        self.update_stats();
        
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
    pub async fn update_model_usage_stats(
        &mut self,
        model_id: &ModelId,
        execution_successful: bool,
        execution_time_us: u64,
        user: &str,
    ) -> Result<()> {
        let mut stats = self.get_model_usage_stats(model_id).await?
            .unwrap_or_else(|| ModelUsageStats {
                model_id: model_id.clone(),
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
                total_fees_collected: 0,
                last_execution: None,
                avg_execution_time_us: 0,
                unique_users: 0,
            });
        
        // Update statistics
        stats.total_executions += 1;
        if execution_successful {
            stats.successful_executions += 1;
        } else {
            stats.failed_executions += 1;
        }
        
        // Update average execution time
        stats.avg_execution_time_us = 
            (stats.avg_execution_time_us * (stats.total_executions - 1) + execution_time_us) 
            / stats.total_executions;
        
        stats.last_execution = Some(Utc::now());
        
        // Store updated statistics
        self.storage.store_model_usage_stats(&stats).await?;
        
        Ok(())
    }

    /// Get registry statistics
    pub async fn get_registry_stats(&mut self) -> Result<RegistryStats> {
        // Update statistics from storage
        self.update_stats();
        Ok(self.stats.clone())
    }

    /// Validate model metadata
    fn validate_model_metadata(&self, metadata: &ModelMetadata) -> Result<()> {
        // Check model size
        if metadata.size_bytes > self.config.max_model_size {
            return Err(RegistryError::InvalidRegistration(
                format!("Model size {} exceeds maximum allowed size {}", 
                    metadata.size_bytes, self.config.max_model_size)
            ));
        }
        
        // Check parameter count
        if metadata.parameter_count > self.config.max_parameter_count {
            return Err(RegistryError::InvalidRegistration(
                format!("Parameter count {} exceeds maximum allowed count {}", 
                    metadata.parameter_count, self.config.max_parameter_count)
            ));
        }
        
        // Check required fields
        if metadata.id.name.is_empty() {
            return Err(RegistryError::InvalidRegistration("Model name cannot be empty".to_string()));
        }
        
        if metadata.id.version.is_empty() {
            return Err(RegistryError::InvalidRegistration("Model version cannot be empty".to_string()));
        }
        
        if metadata.id.hash.is_empty() {
            return Err(RegistryError::InvalidRegistration("Model hash cannot be empty".to_string()));
        }
        
        Ok(())
    }

    /// Calculate registration fee
    fn calculate_registration_fee(&self, metadata: &ModelMetadata) -> Result<u64> {
        // Base fee
        let mut fee = self.config.min_registration_fee;
        
        // Add fee based on model size
        let size_fee = (metadata.size_bytes as f64 * 0.001) as u64; // 0.001 per byte
        fee += size_fee;
        
        // Add fee based on parameter count
        let param_fee = (metadata.parameter_count as f64 * 0.01) as u64; // 0.01 per parameter
        fee += param_fee;
        
        // Ensure fee is within bounds
        fee = fee.max(self.config.min_registration_fee);
        fee = fee.min(self.config.max_registration_fee);
        
        Ok(fee)
    }

    /// Update registry statistics
    fn update_stats(&mut self) {
        // This is a simplified implementation
        // In a real implementation, this would query the storage for actual statistics
        self.stats.total_models = self.model_cache.len() as u64;
        self.stats.active_models = self.model_cache.values()
            .filter(|r| r.status == RegistrationStatus::Approved)
            .count() as u64;
        self.stats.pending_models = self.model_cache.values()
            .filter(|r| r.status == RegistrationStatus::Pending)
            .count() as u64;
    }
}

impl Default for RegistryStats {
    fn default() -> Self {
        Self {
            total_models: 0,
            active_models: 0,
            pending_models: 0,
            total_executions: 0,
            total_fees_collected: 0,
            total_registrants: 0,
            avg_model_size: 0,
            most_popular_category: None,
        }
    }
}