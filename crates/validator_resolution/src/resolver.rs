//! Validator ID resolver implementation
//!
//! Resolves human-readable handles or public keys to deterministic validator identities.
//!
//! Sources used in resolution:
//! 1. Direct Ed25519 public key (no lookup needed)
//! 2. L2 Handle Registry (@handle.ipn)
//! 3. L1 Ownership Anchor (chain-anchored ownership records)
//! 4. Registry alias (reserved internal identifiers)

use crate::errors::*;
use crate::types::{ResolvedValidator, ResolutionMethod, ValidatorMetadata};
use ippan_economics::ValidatorId;
use ippan_l1_handle_anchors::L1HandleAnchorStorage;
use ippan_l2_handle_registry::{Handle, L2HandleRegistry, PublicKey as L2PublicKey};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Validator ID resolver
///
/// Provides unified resolution of validator identities across
/// L1 and L2 namespaces. Used by consensus and governance modules.
#[derive(Debug)]
pub struct ValidatorResolver {
    l2_registry: Arc<L2HandleRegistry>,
    l1_anchors: Arc<L1HandleAnchorStorage>,
    cache: Arc<parking_lot::RwLock<std::collections::HashMap<ValidatorId, CachedEntry>>>,
    cache_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedEntry {
    value: ResolvedValidator,
    inserted_at: Instant,
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

    /// Resolve a single `ValidatorId` into a public key and metadata
    pub async fn resolve(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        if let Some(cached_entry) = self.get_from_cache(id) {
            if self.is_cache_valid(&cached_entry) {
                return Ok(cached_entry.value.clone());
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
            let fut = async move {
                let result = resolver.resolve(&id_clone).await;
                (id_clone, result)
            };
            futures.push(fut);
        }

        let batch_results = futures::future::join_all(futures).await;
        for (id, result) in batch_results {
            results.insert(id, result);
        }

        results
    }

    /// Determine resolution method for a given ID
    fn resolve_method(&self, id: &ValidatorId) -> ResolutionMethod {
        if id.is_public_key() {
            ResolutionMethod::Direct
        } else if id.is_handle() {
            ResolutionMethod::L2HandleRegistry
        } else {
            ResolutionMethod::RegistryAlias
        }
    }

    /// Direct resolution for Ed25519 public keys
    async fn resolve_direct(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let public_key = self.parse_public_key(id.as_str())?;
        Ok(ResolvedValidator::new(id.clone(), public_key, ResolutionMethod::Direct))
    }

    /// Resolve handle via L2 Handle Registry
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

    /// Resolve validator ownership via L1 anchors
    async fn resolve_via_l1_anchor(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        let anchor = self.l1_anchors.get_anchor_by_handle(id.as_str())?;
        Ok(ResolvedValidator::new(
            id.clone(),
            anchor.owner,
            ResolutionMethod::L1OwnershipAnchor,
        ))
    }

    /// Registry alias fallback (reserved internal identifiers)
    async fn resolve_via_alias(&self, id: &ValidatorId) -> Result<ResolvedValidator> {
        Err(ValidatorResolutionError::InvalidFormat {
            id: id.as_str().to_string(),
        })
    }

    /// Retrieve handle metadata from the L2 registry
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

    // -------------------------------------------------------------------------
    // Cache Management
    // -------------------------------------------------------------------------
    fn get_from_cache(&self, id: &ValidatorId) -> Option<CachedEntry> {
        self.cache.read().get(id).cloned()
    }

    fn store_in_cache(&self, id: &ValidatorId, resolved: &ResolvedValidator) {
        let entry = CachedEntry {
            value: resolved.clone(),
            inserted_at: Instant::now(),
        };
        self.cache.write().insert(id.clone(), entry);
    }

    fn is_cache_valid(&self, entry: &CachedEntry) -> bool {
        entry.inserted_at.elapsed() < self.cache_ttl
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------
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
