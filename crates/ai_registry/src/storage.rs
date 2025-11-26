//! Storage implementation for AI Registry

use crate::{
    errors::{RegistryError, Result},
    types::*,
};
use ippan_ai_core::types::ModelId;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Storage backend for AI Registry
pub struct RegistryStorage {
    /// Database connection (placeholder)
    db: Option<sled::Db>,
    /// In-memory cache protected by an async-aware lock
    cache: RwLock<HashMap<String, Vec<u8>>>,
}

impl RegistryStorage {
    /// Create a new storage backend
    pub fn new(db_path: Option<&str>) -> Result<Self> {
        let db = if let Some(path) = db_path {
            Some(sled::open(path).map_err(|e| RegistryError::Database(e.to_string()))?)
        } else {
            None
        };

        Ok(Self {
            db,
            cache: RwLock::new(HashMap::new()),
        })
    }

    /// Store model registration
    pub async fn store_model_registration(&self, registration: &ModelRegistration) -> Result<()> {
        let key = format!("model_registration:{}", registration.model_id.name);
        let data = bincode::serialize(registration)
            .map_err(|e| RegistryError::Internal(format!("Serialization error: {e}")))?;

        if let Some(ref db) = self.db {
            db.insert(key.as_bytes(), data.as_slice())
                .map_err(|e| RegistryError::Database(e.to_string()))?;
        } else {
            // Use in-memory cache
            self.cache.write().await.insert(key, data);
        }

        Ok(())
    }

    /// Load model registration
    pub async fn load_model_registration(
        &self,
        model_id: &ModelId,
    ) -> Result<Option<ModelRegistration>> {
        let key = format!("model_registration:{}", model_id.name);

        let data = if let Some(ref db) = self.db {
            db.get(key.as_bytes())
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map(|v| v.to_vec())
        } else {
            self.cache.read().await.get(&key).cloned()
        };

        if let Some(data) = data {
            let registration = bincode::deserialize(&data)
                .map_err(|e| RegistryError::Internal(format!("Serialization error: {e}")))?;
            Ok(Some(registration))
        } else {
            Ok(None)
        }
    }

    /// List models by status
    pub async fn list_models_by_status(
        &self,
        status: RegistrationStatus,
    ) -> Result<Vec<ModelRegistration>> {
        // This is a simplified implementation
        // In a real implementation, this would query the database efficiently
        let mut models = Vec::new();

        if let Some(ref db) = self.db {
            for item in db.iter() {
                let (key, value) = item.map_err(|e| RegistryError::Database(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);

                if key_str.starts_with("model_registration:") {
                    if let Ok(registration) = bincode::deserialize::<ModelRegistration>(&value) {
                        if registration.status == status {
                            models.push(registration);
                        }
                    }
                }
            }
        } else {
            // Use in-memory cache
            let cache = self.cache.read().await;
            for (key, value) in cache.iter() {
                if key.starts_with("model_registration:") {
                    if let Ok(registration) = bincode::deserialize::<ModelRegistration>(value) {
                        if registration.status == status {
                            models.push(registration);
                        }
                    }
                }
            }
        }

        Ok(models)
    }

    /// List models by category
    pub async fn list_models_by_category(
        &self,
        category: ModelCategory,
    ) -> Result<Vec<ModelRegistration>> {
        let mut models = Vec::new();

        if let Some(ref db) = self.db {
            for item in db.iter() {
                let (key, value) = item.map_err(|e| RegistryError::Database(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);

                if key_str.starts_with("model_registration:") {
                    if let Ok(registration) = bincode::deserialize::<ModelRegistration>(&value) {
                        if registration.category == category {
                            models.push(registration);
                        }
                    }
                }
            }
        } else {
            // Use in-memory cache
            let cache = self.cache.read().await;
            for (key, value) in cache.iter() {
                if key.starts_with("model_registration:") {
                    if let Ok(registration) = bincode::deserialize::<ModelRegistration>(value) {
                        if registration.category == category {
                            models.push(registration);
                        }
                    }
                }
            }
        }

        Ok(models)
    }

    /// Search models
    pub async fn search_models(
        &self,
        query: &str,
        category: Option<ModelCategory>,
        status: Option<RegistrationStatus>,
        limit: Option<usize>,
    ) -> Result<Vec<ModelRegistration>> {
        let mut models = Vec::new();
        let query_lower = query.to_lowercase();

        if let Some(ref db) = self.db {
            for item in db.iter() {
                let (key, value) = item.map_err(|e| RegistryError::Database(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);

                if key_str.starts_with("model_registration:") {
                    if let Ok(registration) = bincode::deserialize::<ModelRegistration>(&value) {
                        // Check query match
                        let matches_query = registration
                            .model_id
                            .name
                            .to_lowercase()
                            .contains(&query_lower)
                            || registration
                                .model_id
                                .version
                                .to_lowercase()
                                .contains(&query_lower)
                            || registration
                                .description
                                .as_ref()
                                .is_some_and(|d| d.to_lowercase().contains(&query_lower))
                            || registration
                                .tags
                                .iter()
                                .any(|tag| tag.to_lowercase().contains(&query_lower));

                        // Check category filter
                        let matches_category = category
                            .as_ref()
                            .is_none_or(|c| &registration.category == c);

                        // Check status filter
                        let matches_status =
                            status.as_ref().is_none_or(|s| &registration.status == s);

                        if matches_query && matches_category && matches_status {
                            models.push(registration);
                        }
                    }
                }
            }
        } else {
            // Use in-memory cache
            let cache = self.cache.read().await;
            for (key, value) in cache.iter() {
                if key.starts_with("model_registration:") {
                    if let Ok(registration) = bincode::deserialize::<ModelRegistration>(value) {
                        // Check query match
                        let matches_query = registration
                            .model_id
                            .name
                            .to_lowercase()
                            .contains(&query_lower)
                            || registration
                                .model_id
                                .version
                                .to_lowercase()
                                .contains(&query_lower)
                            || registration
                                .description
                                .as_ref()
                                .is_some_and(|d| d.to_lowercase().contains(&query_lower))
                            || registration
                                .tags
                                .iter()
                                .any(|tag| tag.to_lowercase().contains(&query_lower));

                        // Check category filter
                        let matches_category = category
                            .as_ref()
                            .is_none_or(|c| &registration.category == c);

                        // Check status filter
                        let matches_status =
                            status.as_ref().is_none_or(|s| &registration.status == s);

                        if matches_query && matches_category && matches_status {
                            models.push(registration);
                        }
                    }
                }
            }
        }

        // Apply limit
        if let Some(limit) = limit {
            models.truncate(limit);
        }

        Ok(models)
    }

    /// Store governance proposal
    pub async fn store_governance_proposal(&self, proposal: &GovernanceProposal) -> Result<()> {
        let key = format!("governance_proposal:{}", proposal.id);
        let data = bincode::serialize(proposal)
            .map_err(|e| RegistryError::Internal(format!("Serialization error: {e}")))?;

        if let Some(ref db) = self.db {
            db.insert(key.as_bytes(), data.as_slice())
                .map_err(|e| RegistryError::Database(e.to_string()))?;
        } else {
            self.cache.write().await.insert(key, data);
        }

        Ok(())
    }

    /// Load governance proposal
    pub async fn load_governance_proposal(
        &self,
        proposal_id: &str,
    ) -> Result<Option<GovernanceProposal>> {
        let key = format!("governance_proposal:{proposal_id}");

        let data = if let Some(ref db) = self.db {
            db.get(key.as_bytes())
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map(|v| v.to_vec())
        } else {
            self.cache.read().await.get(&key).cloned()
        };

        if let Some(data) = data {
            let proposal = bincode::deserialize(&data)
                .map_err(|e| RegistryError::Internal(format!("Serialization error: {e}")))?;
            Ok(Some(proposal))
        } else {
            Ok(None)
        }
    }

    /// List active proposals
    pub async fn list_active_proposals(&self) -> Result<Vec<GovernanceProposal>> {
        let mut proposals = Vec::new();

        if let Some(ref db) = self.db {
            for item in db.iter() {
                let (key, value) = item.map_err(|e| RegistryError::Database(e.to_string()))?;
                let key_str = String::from_utf8_lossy(&key);

                if key_str.starts_with("governance_proposal:") {
                    if let Ok(proposal) = bincode::deserialize::<GovernanceProposal>(&value) {
                        if proposal.status == ProposalStatus::Active {
                            proposals.push(proposal);
                        }
                    }
                }
            }
        } else {
            // Use in-memory cache
            let cache = self.cache.read().await;
            for (key, value) in cache.iter() {
                if key.starts_with("governance_proposal:") {
                    if let Ok(proposal) = bincode::deserialize::<GovernanceProposal>(value) {
                        if proposal.status == ProposalStatus::Active {
                            proposals.push(proposal);
                        }
                    }
                }
            }
        }

        Ok(proposals)
    }

    /// Store model usage statistics
    pub async fn store_model_usage_stats(&self, stats: &ModelUsageStats) -> Result<()> {
        let key = format!("model_usage_stats:{}", stats.model_id.name);
        let data = bincode::serialize(stats)
            .map_err(|e| RegistryError::Internal(format!("Serialization error: {e}")))?;

        if let Some(ref db) = self.db {
            db.insert(key.as_bytes(), data.as_slice())
                .map_err(|e| RegistryError::Database(e.to_string()))?;
        } else {
            self.cache.write().await.insert(key, data);
        }

        Ok(())
    }

    /// Get model usage statistics
    pub async fn get_model_usage_stats(
        &self,
        model_id: &ModelId,
    ) -> Result<Option<ModelUsageStats>> {
        let key = format!("model_usage_stats:{}", model_id.name);

        let data = if let Some(ref db) = self.db {
            db.get(key.as_bytes())
                .map_err(|e| RegistryError::Database(e.to_string()))?
                .map(|v| v.to_vec())
        } else {
            self.cache.read().await.get(&key).cloned()
        };

        if let Some(data) = data {
            let stats = bincode::deserialize(&data)
                .map_err(|e| RegistryError::Internal(format!("Serialization error: {e}")))?;
            Ok(Some(stats))
        } else {
            Ok(None)
        }
    }

    /// Record fee collection
    pub async fn record_fee_collection(
        &self,
        fee_type: FeeType,
        model_id: Option<&ModelId>,
        user: &str,
        amount: u64,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let key = format!(
            "fee_collection:{}:{}:{}",
            chrono::Utc::now().timestamp(),
            fee_type as u8,
            user
        );

        let fee_record = FeeRecord {
            fee_type,
            model_id: model_id.cloned(),
            user: user.to_string(),
            amount,
            timestamp: chrono::Utc::now(),
            metadata,
        };

        let data = bincode::serialize(&fee_record)
            .map_err(|e| RegistryError::Internal(format!("Serialization error: {e}")))?;

        if let Some(ref db) = self.db {
            db.insert(key.as_bytes(), data.as_slice())
                .map_err(|e| RegistryError::Database(e.to_string()))?;
        } else {
            self.cache.write().await.insert(key, data);
        }

        Ok(())
    }
}

/// Fee collection record
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeeRecord {
    /// Fee type
    pub fee_type: FeeType,
    /// Model ID (if applicable)
    pub model_id: Option<ModelId>,
    /// User who paid the fee
    pub user: String,
    /// Fee amount
    pub amount: u64,
    /// Collection timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, String>>,
}
