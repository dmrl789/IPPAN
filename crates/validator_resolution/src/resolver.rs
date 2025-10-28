//! Validator ID resolver implementation

use crate::errors::*;
use crate::types::*;
use ippan_economics::ValidatorId;
use ippan_l1_handle_anchors::L1HandleAnchorStorage;
use ippan_l2_handle_registry::{Handle, L2HandleRegistry, PublicKey as L2PublicKey};
use std::sync::Arc;
use tokio::time::Duration;

/// Validator ID resolver
///
/// Resolves ValidatorId to public keys using multiple resolution methods:
/// 1. Direct public key (no resolution needed)
/// 2. L2 handle registry lookup
/// 3. L1 ownership anchor lookup
/// 4. Registry alias lookup
#[derive(Debug)]
pub struct ValidatorResolver {
    l2_registry: Arc<L2HandleRegistry>,
    l1_anchors: Arc<L1HandleAnchorStorage>,
    cache: Arc<parking_lot::RwLock<std::collections::HashMap<ValidatorId, ResolvedValidator>>>,
    cache_ttl: Duration,
}

impl ValidatorResolver {
    /// Create a new validator resolver
    pub fn new(l2_registry: Arc<L2HandleRegistry>, l1_anchors: Arc<L1HandleAnchorStorage>) -> Self {
        Self {
            l2_registry,
            l1_anchors,
            cache: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
            cache_ttl: Duration::from_secs(300),
        }
    }

    /// Resolve a ValidatorId to a ResolvedValidator
    pub async fn resolve(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        if let Some(cached) = self.get_from_cache(id) {
            if self.is_cache_valid(&cached) {
                return Ok(cached);
            }
        }

        let resolved = match self.resolve_method(id) {
            ResolutionMethod::Direct => self.resolve_direct(id).await?,
            ResolutionMethod::L2HandleRegistry => self.resolve_via_l2_handle(id).await?,
            ResolutionMethod::L1OwnershipAnchor => self.resolve_via_l1_anchor(id).await?,
            ResolutionMethod::RegistryAlias => self.resolve_via_alias(id).await?,
        };

        self.store_in_cache(id, &resolved);
        Ok(resolved)
    }

    /// Resolve multiple ValidatorIds in parallel
    pub async fn resolve_batch(
        &self,
        ids: &[ValidatorId],
    ) -> std::collections::HashMap<ValidatorId, Result<ResolvedValidator>> {
        let mut results = std::collections::HashMap::new();
        let mut futures = Vec::new();

        for id in ids {
            let resolver = self.clone();
            let id_clone = id.clone();
            let future = async move {
                let id_result = id_clone.clone();
                let result = resolver.resolve(&id_clone).await;
                (id_result, result)
            };
            futures.push(future);
        }

        let batch_results = futures::future::join_all(futures).await;
        for (id, result) in batch_results {
            results.insert(id, result);
        }

        results
    }

    /// Determine resolution method for a ValidatorId
    fn resolve_method(&self, id: &ValidatorId) -> ResolutionMethod {
        if id.is_public_key() {
            ResolutionMethod::Direct
        } else if id.is_handle() {
            ResolutionMethod::L2HandleRegistry
        } else {
            ResolutionMethod::RegistryAlias
        }
    }

    /// Direct resolution (public key)
    async fn resolve_direct(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let public_key = self.parse_public_key(id.as_str())?;
        Ok(ResolvedValidator::new(id.clone(), public_key, ResolutionMethod::Direct))
    }

    /// Resolve via L2 handle registry (async-safe)
    async fn resolve_via_l2_handle(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let handle = Handle::new(id.as_str());
        let registry = self.l2_registry.clone();
        let handle_clone = handle.clone();

        let l2_public_key = tokio::task::spawn_blocking(move || registry.resolve(&handle_clone))
            .await
            .map_err(|_| ValidatorResolutionError::ResolutionTimeout)??;

        let public_key = *l2_public_key.as_bytes();
        let metadata = self.get_handle_metadata(&handle).await.ok();

        Ok(ResolvedValidator::with_metadata(
            id.clone(),
            public_key,
            ResolutionMethod::L2HandleRegistry,
            metadata.unwrap_or_else(|| ValidatorMetadata {
                handle: Some(id.as_str().to_string()),
                created_at: None,
                updated_at: None,
                status: None,
                custom: std::collections::HashMap::new(),
            }),
        ))
    }

    /// Resolve via L1 ownership anchor
    async fn resolve_via_l1_anchor(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let anchor = self.l1_anchors.get_anchor_by_handle(id.as_str())?;
        Ok(ResolvedValidator::new(
            id.clone(),
            anchor.owner,
            ResolutionMethod::L1OwnershipAnchor,
        ))
    }

    /// Registry alias placeholder
    async fn resolve_via_alias(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        Err(ValidatorResolutionError::InvalidFormat {
            id: id.as_str().to_string(),
        })
    }

    /// Retrieve handle metadata
    async fn get_handle_metadata(&self, handle: &Handle) -> Result<ValidatorMetadata> {
        let registry = self.l2_registry.clone();
        let handle_clone = handle.clone();

        let metadata = tokio::task::spawn_blocking(move || registry.get_metadata(&handle_clone))
            .await
            .map_err(|_| ValidatorResolutionError::ResolutionTimeout)??;

        Ok(ValidatorMetadata {
            handle: Some(handle.as_str().to_string()),
            created_at: Some(metadata.created_at),
            updated_at: Some(metadata.updated_at),
            status: Some(format!("{:?}", metadata.status)),
            custom: metadata.metadata,
        })
    }

    /// Parse public key from hex string
    fn parse_public_key(&self, hex_str: &str) -> Result<[u8; 32]> {
        if hex_str.len() != 64 {
            return Err(ValidatorResolutionError::InvalidPublicKey);
        }
        let mut key = [0u8; 32];
        hex::decode_to_slice(hex_str, &mut key)
            .map_err(|_| ValidatorResolutionError::InvalidPublicKey)?;
        Ok(key)
    }

    /// Cache helpers
    fn get_from_cache(&self, id: &ValidatorId) -> Option<ResolvedValidator> {
        self.cache.read().get(id).cloned()
    }

    fn store_in_cache(&self, id: &ValidatorId, resolved: &ResolvedValidator) {
        self.cache.write().insert(id.clone(), resolved.clone());
    }

    fn is_cache_valid(&self, _resolved: &ResolvedValidator) -> bool {
        true // TTL logic can be added later
    }

    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }
}

impl Clone for ValidatorResolver {
    fn clone(&self) -> Self {
        Self {
            l2_registry: self.l2_registry.clone(),
            l1_anchors: self.l1_anchors.clone(),
            cache: self.cache.clone(),
            cache_ttl: self.cache_ttl,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ippan_l2_handle_registry::HandleRegistration;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_direct_resolution() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let resolver = ValidatorResolver::new(l2_registry, l1_anchors);

        let id = ValidatorId::new(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        let resolved = resolver.resolve(&id).await.unwrap();

        assert_eq!(resolved.resolution_method, ResolutionMethod::Direct);
        assert_eq!(resolved.public_key_bytes()[0], 0x01);
    }

    #[tokio::test]
    async fn test_handle_resolution() {
        let l2_registry = Arc::new(L2HandleRegistry::new());
        let l1_anchors = Arc::new(L1HandleAnchorStorage::new());
        let resolver = ValidatorResolver::new(l2_registry.clone(), l1_anchors);

        let handle = "@test.ipn";
        let owner = [1u8; 32];
        let registration = HandleRegistration {
            handle: Handle::new(handle),
            owner: L2PublicKey::new(owner),
            signature: vec![1, 2, 3],
            metadata: HashMap::new(),
            expires_at: None,
        };
        l2_registry.register(registration).unwrap();

        let id = ValidatorId::new(handle);
        let resolved = resolver.resolve(&id).await.unwrap();

        assert_eq!(resolved.resolution_method, ResolutionMethod::L2HandleRegistry);
        assert_eq!(resolved.public_key_bytes(), &owner);
    }
}
